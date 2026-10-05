//! A group template's members (docs/adr/0176 §5) against the independent
//! reference in `fixtures/template-members/v1/cases.json`
//! (`scripts/fixtures/template_member_cases.py`, exact fractions, no KentOS
//! code): the operations, called by name as the web calls them through WASM,
//! give every case's parallels or refusal and every centroid; numbers within
//! 1e-9 m, a missing field null.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

/// Numbers within 1e-9 (a missing field is null), everything else exactly.
fn same(a: &Value, e: &Value, path: &str) -> Result<(), String> {
    match (a, e) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            if (x - y).abs() <= 1e-9 {
                Ok(())
            } else {
                Err(format!("{path}: {x} ≠ {y}"))
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            if x.len() != y.len() {
                return Err(format!("{path}: {} öğe ≠ {} öğe", x.len(), y.len()));
            }
            for (i, (p, q)) in x.iter().zip(y).enumerate() {
                same(p, q, &format!("{path}[{i}]"))?;
            }
            Ok(())
        }
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                same(
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                    &format!("{path}.{k}"),
                )?;
            }
            Ok(())
        }
        _ if a == e => Ok(()),
        _ => Err(format!("{path}: {a} ≠ {e}")),
    }
}

fn run(name: &str, args: Value) -> Value {
    let out = run_named(name, &args.to_string()).unwrap_or_else(|e| panic!("{name}: {e}"));
    serde_json::from_str(&out).expect("the operation's JSON")
}

#[test]
fn every_member_is_made_as_the_reference_makes_it() {
    let path = format!(
        "{}/../../../fixtures/template-members/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.template-member-cases");
    assert_eq!(file["version"], 1);
    let mut wrong = Vec::new();
    let offsets = file["offsets"].as_array().expect("offsets");
    assert!(offsets.len() >= 20, "{} offset cases", offsets.len());
    for c in offsets {
        let got = run(
            "templateMemberOffsets",
            json!([c["entity"], c["distance"], c["side"]]),
        );
        if let Err(e) = same(&got, &c["expected"], "") {
            wrong.push(format!("{}: {e}", c["name"]));
        }
    }
    let centroids = file["centroids"].as_array().expect("centroids");
    assert!(centroids.len() >= 6, "{} centroid cases", centroids.len());
    for c in centroids {
        let got = run("templateMemberCentroid", json!([c["entity"]]));
        if let Err(e) = same(&got, &c["expected"], "") {
            wrong.push(format!("{}: {e}", c["name"]));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
