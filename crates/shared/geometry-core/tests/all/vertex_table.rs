//! Köşe tablosu's rows and writes (docs/adr/0172 §7) against the independent
//! reference in `fixtures/vertex-table/v1/cases.json`
//! (`scripts/fixtures/vertex_table_cases.py`, no KentOS code): the
//! operations, called by name as the web calls them through WASM, give every
//! object's rows and every write's paths or refusal. Coordinates and
//! elevations are copied and compared bit for bit; chords, radii, bulges and
//! the least radius come from arithmetic, within 1e-12 (relative).

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

/// Whether the value at `path` is worked out (compared within 1e-12) rather than copied.
fn computed(path: &str) -> bool {
    path.contains(".bulges")
        || path.ends_with(".chord")
        || path.ends_with(".radius")
        || path.ends_with(".least")
}

/// Computed numbers within 1e-12 (relative), copied ones exactly; a missing field is null.
fn same(a: &Value, e: &Value, path: &str) -> Result<(), String> {
    match (a, e) {
        (Value::Number(x), Value::Number(y)) => {
            let (x, y) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            let close = if computed(path) {
                (x - y).abs() <= 1e-12 * y.abs().max(1.0)
            } else {
                x == y
            };
            if close {
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
        "{}/../../../fixtures/vertex-table/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.vertex-table");
    assert_eq!(file["version"], 1);
    file
}

fn call(op: &str, args: Value) -> Result<Value, String> {
    run_named(op, &args.to_string()).map(|text| serde_json::from_str(&text).expect("answer JSON"))
}

#[test]
fn the_rows_are_the_references() {
    let file = fixture();
    let rows = file["rows"].as_array().expect("rows");
    assert_eq!(rows.len(), 5);
    for r in rows {
        let name = r["shape"].as_str().unwrap();
        let shape = &file["shapes"][name];
        let got = call("vertexTableRows", json!([shape["paths"]])).expect("rows");
        same(&got, &r["expect"], name).unwrap();
    }
}

#[test]
fn every_write_gives_the_references_paths_or_refusal() {
    let file = fixture();
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 45, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let shape = &file["shapes"][c["shape"].as_str().unwrap()];
        let (kind, paths, op) = (&shape["kind"], &shape["paths"], &c["op"]);
        let (name_of, args) = match op["kind"].as_str().unwrap() {
            "move" => (
                "vertexTableMove",
                json!([kind, paths, op["path"], op["index"], op["to"]]),
            ),
            "z" => (
                "vertexTableZ",
                json!([kind, paths, op["path"], op["index"], op["z"]]),
            ),
            "radius" => (
                "vertexTableRadius",
                json!([
                    kind,
                    paths,
                    op["path"],
                    op["index"],
                    op["radius"],
                    op["slack"]
                ]),
            ),
            "insert" => (
                "vertexTableInsert",
                json!([kind, paths, op["path"], op["after"], op["at"], op["z"]]),
            ),
            "remove" => ("vertexTableRemove", json!([kind, paths, op["at"]])),
            other => panic!("{name}: unknown write {other}"),
        };
        match call(name_of, args) {
            Ok(got) => {
                if let Err(e) = same(&got, &c["expect"], name) {
                    failures.push(e);
                }
            }
            Err(e) => failures.push(format!("{name}: {e}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} writes differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

#[test]
fn a_radius_given_back_is_the_bulge_it_came_from() {
    use kentos_geometry_core::ops::elevation::Elevated;
    use kentos_geometry_core::ops::vertex_table::{Kind, radius_of, rows, with_radius};
    use kentos_geometry_core::vec2::Vec2;
    // Every bulge from a sliver to almost a full circle, either way round:
    // the radius the table shows, typed back, gives the same edge.
    for b in [
        0.01, 0.2, 0.5, 0.9, 0.999, 1.0, 1.001, 1.5, 3.0, 20.0, -0.3, -1.0, -2.5,
    ] {
        let path = Elevated {
            pts: vec![
                Vec2::new(486500.0, 4420100.0),
                Vec2::new(486537.5, 4420122.25),
            ],
            bulges: Some(vec![b, 0.0]),
            closed: false,
            zs: vec![None, None],
        };
        let row = &rows(std::slice::from_ref(&path))[0];
        let r = radius_of(row.chord.unwrap(), b).unwrap();
        assert_eq!(row.radius, Some(r));
        let back = with_radius(Kind::Polyline, &[path], 0, 0, Some(r), 0.0).unwrap();
        let got = back[0].bulges.as_ref().unwrap()[0];
        assert!(
            (got - b).abs() <= 1e-9 * b.abs().max(1.0),
            "bulge {b}: back {got}"
        );
    }
}
