//! Nokta editörü's computations (docs/adr/0153 §6) against the independent
//! reference in `fixtures/point-editor/v1/cases.json`
//! (`scripts/fixtures/point_editor_cases.py`, no KentOS code): the
//! operations, called by name as the web calls them through WASM, give every
//! case's natural order, table rows, duplicate groups and moved vertices;
//! numbers within 1e-9 m, everything else exactly.

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

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/point-editor/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.point-editor-fixtures");
    assert_eq!(file["version"], 1);
    file
}

/// Every case of `section`: the operation `op` on the arguments `args`
/// takes from it, against its `expected`.
fn run_section(section: &str, op: &str, least: usize, args: impl Fn(&Value) -> Value) {
    let file = fixture();
    let cases = file[section].as_array().expect("cases");
    assert!(cases.len() >= least, "{section}: {} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let got: Value = match run_named(op, &args(c).to_string()) {
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
        "{section}: {} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

#[test]
fn names_sort_in_the_natural_order() {
    run_section("natural", "naturalOrder", 15, |c| json!([c["names"]]));
}

#[test]
fn the_table_shows_and_sorts_its_rows_as_the_reference() {
    run_section("table", "pointTable", 100, |c| {
        json!([c["rows"], c["query"]])
    });
}

#[test]
fn duplicates_group_and_keep_as_the_reference() {
    run_section("duplicates", "duplicatePoints", 30, |c| {
        json!([c["points"], c["by"], c["tolerance"], c["keep"]])
    });
}

#[test]
fn vertices_follow_a_point_as_the_reference() {
    run_section("follow", "followPoint", 25, |c| {
        json!([c["paths"], c["from"], c["to"], c["setZ"], c["z"]])
    });
}
