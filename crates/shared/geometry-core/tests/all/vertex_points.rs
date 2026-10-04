//! Köşelere nokta (docs/adr/0152 §5) against the independent reference in
//! `fixtures/vertex-points/v1/cases.json` (`scripts/fixtures/vertex_points_cases.py`,
//! no KentOS code): the operation, called by name as the web calls it
//! through WASM, gives every case's points (place, elevation, name), the
//! places passed over and the next name; numbers within 1e-9 m.

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

#[test]
fn every_case_is_placed_as_the_reference_places_it() {
    let path = format!(
        "{}/../../../fixtures/vertex-points/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.vertex-points-fixtures");
    assert_eq!(file["version"], 1);
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 40, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let args = json!([c["objects"], c["existing"], c["first"]]).to_string();
        let got: Value = match run_named("vertexPoints", &args) {
            Ok(text) => serde_json::from_str(&text).expect("answer JSON"),
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        if let Err(e) = same(&got, &c["expected"], name) {
            failures.push(e);
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}
