//! The .kstil style file's rules as `fixtures/style/v1/kstil.json` holds them
//! (the web records it, `apps/web/scripts/fixtures/record-kstil.test.ts`, and
//! checks it in `style/kstilFixture.test.ts`): reading and refusing files,
//! symbol checks, SVG cleaning, export and import. The desktop's style
//! manager reads and writes files through the same code.

use std::collections::HashMap;
use std::path::PathBuf;

use kentos_native_style::file::{
    ConflictMode, STYLE_FORMAT, STYLE_VERSION, StyleFile, export_styles, import_styles,
    parse_style_file, sanitize_svg, svg_asset, validate_symbol,
};
use kentos_native_style::library::{Item, Source, StyleLibrary};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/style/v1/kstil.json");
    let text = std::fs::read_to_string(&path).expect("kstil.json");
    serde_json::from_str(&text).expect("kstil.json is JSON")
}

fn library_of(state: &Value) -> StyleLibrary {
    let list = |k: &str| state[k].as_array().cloned().unwrap_or_default();
    let mut lib = StyleLibrary::with_system(
        list("system")
            .into_iter()
            .filter_map(Item::from_value)
            .collect(),
        Vec::new(),
    );
    lib.load(Source::User, &list("user"), &[]);
    lib.load(Source::Project, &list("project"), &[]);
    lib
}

/// Numbers compared as numbers (1 and 1.0 alike), everything else as it is.
fn same(got: &Value, want: &Value) -> bool {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len() && a.iter().all(|(k, x)| b.get(k).is_some_and(|y| same(x, y)))
        }
        _ => got == want,
    }
}

/// Every string equal to a key of `back` replaced by its value.
fn swap(v: &Value, back: &HashMap<String, String>) -> Value {
    match v {
        Value::String(s) => Value::from(back.get(s).cloned().unwrap_or_else(|| s.clone())),
        Value::Array(a) => Value::Array(a.iter().map(|x| swap(x, back)).collect()),
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, x)| (k.clone(), swap(x, back))).collect())
        }
        x => x.clone(),
    }
}

#[test]
fn reads_writes_and_takes_in_files_as_the_web_does() {
    let f = fixture();
    assert_eq!(f["format"], "kentos.style-file-cases");
    assert_eq!(f["version"], 1);
    assert_eq!(f["styleFormat"], STYLE_FORMAT);
    assert_eq!(f["styleVersion"], STYLE_VERSION);
    let mut problems = Vec::new();
    let mut check = |got: Value, want: &Value, what: String| {
        if !same(&got, want) {
            problems.push(format!("{what}:\n  got  {got}\n  want {want}"));
        }
    };

    for c in f["parse"].as_array().unwrap() {
        let got = match parse_style_file(c["text"].as_str().unwrap()) {
            Ok(file) => {
                json!({ "issues": [], "items": file.items, "categories": file.categories.unwrap_or_default() })
            }
            Err(issues) => json!({ "issues": issues }),
        };
        check(got, &c["expect"], format!("reads {}", c["id"]));
    }
    for s in f["sanitize"].as_array().unwrap() {
        check(
            json!(sanitize_svg(s["svg"].as_str().unwrap())),
            &s["expect"],
            format!("cleans {}", s["svg"]),
        );
    }
    for c in f["symbols"].as_array().unwrap() {
        let place = c["where"].as_str().unwrap_or("sembol");
        check(
            json!(validate_symbol(&c["symbol"], place)),
            &c["expect"],
            format!("checks {}", c["id"]),
        );
    }
    let lib = library_of(&f["exportLibrary"]);
    for x in f["exports"].as_array().unwrap() {
        let ids: Vec<&str> = x["ids"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        let file = export_styles(&lib, &ids);
        check(
            json!({ "format": file["format"], "version": file["version"], "items": file["items"] }),
            &x["expect"],
            format!("exports {ids:?}"),
        );
    }
    let file = &f["importFile"];
    let incoming = StyleFile {
        items: file["items"].as_array().cloned().unwrap_or_default(),
        categories: file["categories"].as_array().cloned(),
    };
    for c in f["imports"].as_array().unwrap() {
        let mut lib = library_of(&f["exportLibrary"]);
        let to = if c["to"] == "project" {
            Source::Project
        } else {
            Source::User
        };
        let mode = match c["mode"].as_str().unwrap() {
            "replace" => ConflictMode::Replace,
            "skip" => ConflictMode::Skip,
            _ => ConflictMode::Copy,
        };
        let report = import_styles(&mut lib, &incoming, to, mode);
        let back: HashMap<String, String> = report
            .renamed
            .iter()
            .map(|(old, id)| (id.clone(), format!("{{new:{old}}}")))
            .collect();
        let dump = |s: Source| {
            let (items, categories) = lib.dump(s);
            json!({ "items": items, "categories": categories })
        };
        let renamed: serde_json::Map<String, Value> = report
            .renamed
            .iter()
            .map(|(old, id)| (old.clone(), Value::from(id.clone())))
            .collect();
        let prefixes: serde_json::Map<String, Value> = report
            .renamed
            .iter()
            .map(|(_, id)| (id.clone(), Value::from(id.split('-').next().unwrap_or(""))))
            .collect();
        let got = swap(
            &json!({
                "report": { "added": report.added, "replaced": report.replaced, "skipped": report.skipped, "renamed": renamed },
                "prefixes": prefixes,
                "user": dump(Source::User),
                "project": dump(Source::Project),
            }),
            &back,
        );
        // The prefixes' keys are new ids too.
        let got = {
            let mut g = got;
            if let Some(p) = g.get_mut("prefixes").and_then(Value::as_object_mut) {
                let fixed: serde_json::Map<String, Value> = p
                    .iter()
                    .map(|(k, v)| (back.get(k).cloned().unwrap_or_else(|| k.clone()), v.clone()))
                    .collect();
                *p = fixed;
            }
            g
        };
        check(got, &c["expect"], format!("imports {}", c["id"]));
    }
    for a in f["svgAssets"].as_array().unwrap() {
        let path: Vec<String> = a["path"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|p| p.as_str().map(str::to_owned))
            .collect();
        let mut got = svg_asset(
            a["name"].as_str().unwrap(),
            &path,
            a["svg"].as_str().unwrap(),
            "x",
        );
        if let Some(o) = got.as_object_mut() {
            o.remove("id");
        }
        check(got, &a["expect"], format!("sizes {}", a["name"]));
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
