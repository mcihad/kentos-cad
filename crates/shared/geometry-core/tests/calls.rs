//! The frozen call fixtures (`fixtures/geometry/v1/calls-*.json`, recorded
//! from the TypeScript core, docs/adr/0008) through the core's call table,
//! natively. The WASM build runs the same files through the app's path
//! (`apps/web/src/wasm/calls.wasm.test.ts`).

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use std::path::PathBuf;

use kentos_geometry_core::api::run_named;
use serde_json::Value;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/geometry/v1")
}

/// Numbers within |a − e| ≤ abs + rel·max(|a|, |e|); strings (and the "#NaN"
/// / "#Inf" markers), booleans, nulls, lengths and keys exactly.
fn same(actual: &Value, expected: &Value, abs: f64, rel: f64, path: &str) -> Result<(), String> {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => {
            let (a, e) = (
                a.as_f64().unwrap_or(f64::NAN),
                e.as_f64().unwrap_or(f64::NAN),
            );
            let d = (a - e).abs();
            if d <= abs + rel * a.abs().max(e.abs()) {
                Ok(())
            } else {
                Err(format!("{path}: {a} ≠ {e} (fark {d})"))
            }
        }
        (Value::Array(a), Value::Array(e)) => {
            if a.len() != e.len() {
                return Err(format!("{path}: {} öğe ≠ {} öğe", a.len(), e.len()));
            }
            for (i, (x, y)) in a.iter().zip(e).enumerate() {
                same(x, y, abs, rel, &format!("{path}[{i}]"))?;
            }
            Ok(())
        }
        (Value::Object(a), Value::Object(e)) => {
            for k in a.keys().chain(e.keys()) {
                match (a.get(k), e.get(k)) {
                    (Some(x), Some(y)) => same(x, y, abs, rel, &format!("{path}.{k}"))?,
                    (None, _) => return Err(format!("{path}.{k}: eksik")),
                    (_, None) => return Err(format!("{path}.{k}: fazla")),
                }
            }
            Ok(())
        }
        _ if actual == expected => Ok(()),
        _ => Err(format!("{path}: {actual} ≠ {expected}")),
    }
}

/// A number as the reference wrote it: whole numbers without a fraction, others in their shortest form.
fn js_number(v: f64) -> String {
    if v == 0.0 {
        "0".to_string()
    } else if v.fract() == 0.0 && v.abs() < 1e21 {
        format!("{v:.0}")
    } else {
        format!("{v}")
    }
}

fn js_text(v: &Value) -> String {
    match v {
        Value::Number(n) => js_number(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(a) => format!("[{}]", a.iter().map(js_text).collect::<Vec<_>>().join(",")),
        other => serde_json::to_string(other).unwrap(),
    }
}

/// `KENTOS_WRITE_CALLS=1`: a case the core now answers otherwise has its `expect` written anew in place, the rest of
/// the file byte for byte (one case a line). Only for a decision of the Rust core's own, said in an ADR: the robust
/// slice of docs/adr/0008 (the fill triangulation of polygons whose rings cross or touch, docs/adr/9228).
fn rewrite_expects(path: &std::path::Path, answers: &[(String, String, Value)]) {
    let text = std::fs::read_to_string(path).expect("fixture file");
    let mut out = String::with_capacity(text.len());
    let mut written = 0;
    for line in text.split_inclusive('\n') {
        let found = answers.iter().find(|(f, name, _)| {
            let (name, f) = (
                serde_json::to_string(name).unwrap(),
                serde_json::to_string(f).unwrap(),
            );
            let head = format!("{{\"name\":{name},\"fn\":{f},");
            line.trim_start().starts_with(&head)
        });
        match found.and_then(|(_, _, got)| line.rfind("\"expect\":").map(|at| (at, got))) {
            Some((at, got)) => {
                let tail = &line[at..];
                let end = tail.rfind('}').expect("a case ends its line");
                out.push_str(&line[..at]);
                out.push_str(&format!("\"expect\":{}", js_text(got)));
                out.push_str(&tail[end..]);
                written += 1;
            }
            None => out.push_str(line),
        }
    }
    assert_eq!(
        written,
        answers.len(),
        "{}: every answer written",
        path.display()
    );
    std::fs::write(path, out).expect("fixture written");
    println!("{}: {written} case(s) written", path.display());
}

#[test]
fn every_call_fixture_matches_the_typescript_reference() {
    let writing = std::env::var_os("KENTOS_WRITE_CALLS").is_some();
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir())
        .expect("fixture directory")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("calls-") && n.ends_with(".json"))
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no call fixtures");
    let mut failures = Vec::new();
    let mut count = 0;
    for path in &files {
        let file: Value =
            serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
                .expect("fixture JSON");
        assert_eq!(
            file["format"],
            "kentos.geometry-calls",
            "{}",
            path.display()
        );
        assert_eq!(file["version"], 1);
        assert_eq!(file["crs"]["unit"], "metre");
        let abs = file["tolerance"]["abs"].as_f64().unwrap();
        let rel = file["tolerance"]["rel"].as_f64().unwrap();
        let mut answers = Vec::new();
        for c in file["cases"].as_array().expect("cases") {
            count += 1;
            let label = format!(
                "{}: {}",
                c["fn"].as_str().unwrap(),
                c["name"].as_str().unwrap()
            );
            let args = serde_json::to_string(&c["args"]).unwrap();
            // A case may carry a wider, documented bound (sin/cos-sensitive operations).
            let (abs, rel) = match c.get("tol") {
                Some(t) => (t["abs"].as_f64().unwrap(), t["rel"].as_f64().unwrap()),
                None => (abs, rel),
            };
            let got = run_named(c["fn"].as_str().unwrap(), &args).and_then(|out| {
                serde_json::from_str::<Value>(&out).map_err(|e| format!("çıktı JSON değil: {e}"))
            });
            match got {
                Ok(v) => {
                    if let Err(e) = same(&v, &c["expect"], abs, rel, &label) {
                        if writing {
                            let (f, name) =
                                (c["fn"].as_str().unwrap(), c["name"].as_str().unwrap());
                            answers.push((f.to_string(), name.to_string(), v));
                        } else {
                            failures.push(e);
                        }
                    }
                }
                Err(e) => failures.push(format!("{label}: {e}")),
            }
        }
        if !answers.is_empty() {
            rewrite_expects(path, &answers);
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {count} call(s) differ:\n{}",
        failures.len(),
        failures
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Every number within `bound` of the independent reference's decimal text.
fn within(actual: &Value, expected: &Value, bound: f64, path: &str) -> Result<(), String> {
    match (actual, expected) {
        (Value::Number(a), Value::String(e)) => {
            let (a, e) = (
                a.as_f64().unwrap_or(f64::NAN),
                e.parse::<f64>()
                    .map_err(|_| format!("{path}: sayı değil"))?,
            );
            let err = (a - e).abs();
            if err <= bound {
                Ok(())
            } else {
                Err(format!("{path}: hata {err} > {bound}"))
            }
        }
        (Value::Array(a), Value::Array(e)) if a.len() == e.len() => {
            for (i, (x, y)) in a.iter().zip(e).enumerate() {
                within(x, y, bound, &format!("{path}[{i}]"))?;
            }
            Ok(())
        }
        (Value::Object(a), Value::Object(e)) => {
            for (k, y) in e {
                within(
                    a.get(k).unwrap_or(&Value::Null),
                    y,
                    bound,
                    &format!("{path}.{k}"),
                )?;
            }
            Ok(())
        }
        _ if actual == expected => Ok(()),
        _ => Err(format!("{path}: {actual} ≠ {expected}")),
    }
}

/// Accuracy against the independent reference (`reference-calls.json`,
/// Python fractions and 60-digit roots; CLAUDE.md §23.4).
#[test]
fn operations_stay_within_the_independent_reference_bounds() {
    let file: Value = serde_json::from_str(
        &std::fs::read_to_string(dir().join("reference-calls.json")).expect("reference file"),
    )
    .expect("reference JSON");
    assert_eq!(file["format"], "kentos.geometry-call-reference");
    let mut failures = Vec::new();
    for c in file["cases"].as_array().expect("cases") {
        let label = format!(
            "{}: {}",
            c["fn"].as_str().unwrap(),
            c["name"].as_str().unwrap()
        );
        let bound: f64 = c["bound"].as_str().unwrap().parse().unwrap();
        let out = run_named(
            c["fn"].as_str().unwrap(),
            &serde_json::to_string(&c["args"]).unwrap(),
        )
        .and_then(|o| serde_json::from_str::<Value>(&o).map_err(|e| e.to_string()));
        match out {
            Ok(v) => {
                if let Err(e) = within(&v, &c["expect"], bound, &label) {
                    failures.push(e);
                }
            }
            Err(e) => failures.push(format!("{label}: {e}")),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
