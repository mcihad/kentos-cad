//! `file.export.atlas` end to end (docs/adr/0534): the shared cases (`fixtures/atlas/v1/cases.json`,
//! written by `scripts/fixtures/atlas_cases.py` without KentOS code) run through the headless host's protocol. A case
//! names one of the file's drawings; its steps plan, validate or execute the command (or another command, or an undo)
//! against it, and say what came back (`result`: status, error, a subset of the output, the warnings' codes) and what is
//! on disk after (`file`: whether the file is there and how many pages its PDF has).
//!
//! `KENTOS_WRITE_ATLAS=1` also keeps the hundred-page PDF and its record in `fixtures/atlas/v1/out`, for the
//! independent readers (`python3 scripts/fixtures/atlas_cases.py --files`: poppler and GDAL).

use std::path::PathBuf;

use kentos_headless::Session;
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../../fixtures/atlas/v1/cases.json");

fn scratch(name: usize) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("kentos-atlas-cases-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a temporary directory");
    dir
}

/// `want` is in `got`: objects by their keys, arrays element by element and as long, numbers as numbers, `"$any"` takes
/// anything, `{"$near": x, "within": t}` a number within `t` of `x`.
fn within(got: &Value, want: &Value, at: &str, problems: &mut Vec<String>) {
    match (got, want) {
        (_, Value::String(s)) if s == "$any" => {}
        // `{"$near": x, "within": t}`: a number within t of x (another reckoning of the same projection).
        (Value::Number(g), Value::Object(w)) if w.contains_key("$near") => {
            let g = g.as_f64().unwrap_or(f64::NAN);
            let x = w["$near"].as_f64().unwrap_or(f64::NAN);
            let t = w.get("within").and_then(Value::as_f64).unwrap_or(0.0);
            // NaN is never near.
            if (g - x).abs().partial_cmp(&t) != Some(std::cmp::Ordering::Less)
                && (g - x).abs() != t
            {
                problems.push(format!("{at}: {g} (beklenen {x} ± {t})"));
            }
        }
        (Value::Object(g), Value::Object(w)) => {
            for (k, v) in w {
                match g.get(k) {
                    Some(x) => within(x, v, &format!("{at}.{k}"), problems),
                    None => problems.push(format!("{at}.{k}: yok")),
                }
            }
        }
        (Value::Array(g), Value::Array(w)) => {
            if g.len() != w.len() {
                problems.push(format!("{at}: {} öğe (beklenen {})", g.len(), w.len()));
                return;
            }
            for (i, (x, v)) in g.iter().zip(w).enumerate() {
                within(x, v, &format!("{at}[{i}]"), problems);
            }
        }
        (Value::Number(g), Value::Number(w)) => {
            let (g, w) = (
                g.as_f64().unwrap_or(f64::NAN),
                w.as_f64().unwrap_or(f64::NAN),
            );
            if (g - w).abs() > 1e-9 {
                problems.push(format!("{at}: {g} (beklenen {w})"));
            }
        }
        (g, w) => {
            if g != w {
                problems.push(format!("{at}: {g} (beklenen {w})"));
            }
        }
    }
}

/// The input with `$tmp` in its paths and `$name` its captured pins and values.
fn filled(input: &Value, tmp: &str, pins: &std::collections::HashMap<String, Value>) -> Value {
    match input {
        Value::String(s) if s.starts_with("$tmp") => Value::String(s.replacen("$tmp", tmp, 1)),
        Value::String(s) if s.starts_with('$') && pins.contains_key(&s[1..]) => {
            pins[&s[1..]].clone()
        }
        Value::Object(m) => Value::Object(
            m.iter()
                .map(|(k, v)| (k.clone(), filled(v, tmp, pins)))
                .collect(),
        ),
        Value::Array(a) => Value::Array(a.iter().map(|v| filled(v, tmp, pins)).collect()),
        other => other.clone(),
    }
}

/// How many pages a PDF's page tree holds: the `/Type /Page` objects that are not the tree's `/Pages`.
fn pdf_pages(bytes: &[u8]) -> usize {
    let text = String::from_utf8_lossy(bytes);
    text.match_indices("/Type /Page")
        .filter(|(i, _)| !text[i + "/Type /Page".len()..].starts_with('s'))
        .count()
}

#[test]
fn the_shared_cases_pass() {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    assert_eq!(file["command"], "file.export.atlas");
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20, "{} cases", cases.len());
    let mut problems: Vec<String> = Vec::new();
    for (k, case) in cases.iter().enumerate() {
        let name = case["name"].as_str().unwrap_or("?");
        let setup = &file["setups"][case["setup"].as_str().expect("a setup")];
        let mut s = Session::from_bytes(setup["drawing"].to_string().as_bytes())
            .expect("the drawing opens");
        // The sheets go in as a script puts them in: `cad.sheets.edit`'s addSheet.
        for sheet in setup["book"]["sheets"].as_array().into_iter().flatten() {
            let added = s
                .call(json!({ "command": "cad.sheets.edit",
                    "input": { "ops": [{ "op": "addSheet", "sheet": sheet }] } }))
                .expect("answers");
            assert_eq!(added["status"], "completed", "{name}: {added}");
        }
        let dir = scratch(k);
        let tmp = dir.display().to_string();
        let mut pins = std::collections::HashMap::new();
        for (i, step) in case["steps"].as_array().expect("steps").iter().enumerate() {
            let at = format!("{name} › adım {}", i + 1);
            if step["op"] == "undo" {
                assert!(s.undo().is_some(), "{at}: geri alınacak adım yok");
                continue;
            }
            // A file of the fixtures put in the case's folder (a source to bind).
            if step["op"] == "copy" {
                let from = format!(
                    "{}/../../../{}",
                    env!("CARGO_MANIFEST_DIR"),
                    step["from"].as_str().expect("from")
                );
                let to = filled(&step["to"], &tmp, &pins);
                std::fs::copy(&from, to.as_str().expect("to")).expect("copied");
                continue;
            }
            let command = step["command"].as_str().unwrap_or("file.export.atlas");
            let op = step["op"].as_str().unwrap_or("execute");
            let input = filled(&step["input"], &tmp, &pins);
            let answer = s
                .call(json!({ "command": command, "op": op, "input": input }))
                .expect("answers");
            let want = &filled(&step["result"], &tmp, &pins);
            if want["status"] != Value::Null && answer["status"] != want["status"] {
                problems.push(format!(
                    "{at}: durum {} (beklenen {}): {}",
                    answer["status"], want["status"], answer["error"]
                ));
                continue;
            }
            if let Some(e) = want.get("error") {
                within(&answer["error"], e, &format!("{at} error"), &mut problems);
            }
            if let Some(o) = want.get("output") {
                within(&answer["output"], o, &format!("{at} output"), &mut problems);
            }
            if let Some(w) = want.get("warnings") {
                let codes: Vec<&Value> = answer["warnings"]
                    .as_array()
                    .map(|a| a.iter().map(|x| &x["code"]).collect())
                    .unwrap_or_default();
                let wanted: Vec<&Value> =
                    w.as_array().map(|a| a.iter().collect()).unwrap_or_default();
                if codes != wanted {
                    problems.push(format!("{at}: uyarılar {codes:?} (beklenen {wanted:?})"));
                }
            }
            if let Some(a) = step["as"].as_str() {
                pins.insert(a.to_owned(), answer["output"]["pin"].clone());
            }
            // Values an answer gives a later step (`$name`): a JSON pointer into the output each.
            if let Some(take) = step["take"].as_object() {
                for (name, at) in take {
                    let v = answer["output"]
                        .pointer(at.as_str().unwrap_or(""))
                        .cloned()
                        .unwrap_or(Value::Null);
                    pins.insert(name.clone(), v);
                }
            }
            if let Some(f) = step.get("file") {
                let path = dir.join("atlas.pdf");
                if let Some(exists) = f["exists"].as_bool()
                    && path.exists() != exists
                {
                    problems.push(format!(
                        "{at}: dosya {} (beklenen {exists})",
                        if path.exists() { "var" } else { "yok" }
                    ));
                }
                if let Some(n) = f["pdfPages"].as_u64() {
                    let bytes = std::fs::read(&path).unwrap_or_default();
                    if !bytes.starts_with(b"%PDF-") || pdf_pages(&bytes) as u64 != n {
                        problems.push(format!(
                            "{at}: PDF'in {} sayfası var (beklenen {n})",
                            pdf_pages(&bytes)
                        ));
                    }
                }
            }
            // The independent readers' files: the whole atlas, with its record.
            if std::env::var_os("KENTOS_WRITE_ATLAS").is_some()
                && op == "execute"
                && name.starts_with("Plan dosya yazmaz")
                && command == "file.export.atlas"
            {
                let out = format!(
                    "{}/../../../fixtures/atlas/v1/out",
                    env!("CARGO_MANIFEST_DIR")
                );
                std::fs::create_dir_all(&out).expect("a folder");
                std::fs::copy(dir.join("atlas.pdf"), format!("{out}/parsel-atlasi.pdf"))
                    .expect("kept");
                std::fs::write(
                    format!("{out}/parsel-atlasi.pin.json"),
                    serde_json::to_vec_pretty(&answer["output"]["pin"]).expect("json"),
                )
                .expect("kept");
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
    assert!(
        problems.is_empty(),
        "{} sorun:\n{}",
        problems.len(),
        problems.join("\n")
    );
}
