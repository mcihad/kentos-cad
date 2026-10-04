//! The golden cases shared with the TypeScript core (`fixtures/geometry/v1`,
//! checked on the TS side by `apps/web/src/model/geom/golden.test.ts`): the Rust port
//! must reproduce every recorded result within the file's tolerance.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::bulge::{bulge_arc, bulge_path_length, bulge_ring_area};
use kentos_geometry_core::geometry::{point_in_polygon, signed_area};
use kentos_geometry_core::measure::{Ring, polygon_area, polygon_perimeter};
use serde_json::{Value, json};

const FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/geometry/v1/cases.json"
);

fn vec2(v: &Value) -> Vec2 {
    Vec2::new(v["x"].as_f64().expect("x"), v["y"].as_f64().expect("y"))
}

fn pts(v: &Value) -> Vec<Vec2> {
    v.as_array().expect("pts").iter().map(vec2).collect()
}

fn bulges(v: &Value) -> Option<Vec<f64>> {
    v.as_array()
        .map(|a| a.iter().map(|b| b.as_f64().expect("bulge")).collect())
}

fn ring(v: &Value) -> Ring {
    Ring {
        pts: pts(&v["pts"]),
        bulges: bulges(&v["bulges"]),
    }
}

fn rings(v: &Value) -> Vec<Ring> {
    v.as_array().expect("holes").iter().map(ring).collect()
}

/// What the Rust core computes for a case, in the fixture's JSON shape.
fn run(op: &str, input: &Value) -> Value {
    match op {
        "bulgeArc" => match bulge_arc(
            vec2(&input["a"]),
            vec2(&input["b"]),
            input["bulge"].as_f64().unwrap(),
        ) {
            Some(a) => {
                json!({ "c": { "x": a.c.x, "y": a.c.y }, "r": a.r, "a0": a.a0, "sweep": a.sweep })
            }
            None => Value::Null,
        },
        "bulgePathLength" => json!(bulge_path_length(
            &pts(&input["pts"]),
            bulges(&input["bulges"]).as_deref(),
            input["closed"].as_bool().unwrap()
        )),
        "bulgeRingArea" => json!(bulge_ring_area(
            &pts(&input["pts"]),
            bulges(&input["bulges"]).as_deref()
        )),
        "signedArea" => json!(signed_area(&pts(&input["pts"]))),
        "pointInPolygon" => json!(point_in_polygon(vec2(&input["p"]), &pts(&input["pts"]))),
        "polygonArea" => json!(polygon_area(
            &ring(&input["outer"]),
            &rings(&input["holes"])
        )),
        "polygonPerimeter" => json!(polygon_perimeter(
            &ring(&input["outer"]),
            &rings(&input["holes"])
        )),
        other => panic!("unknown golden op {other}"),
    }
}

/// Numbers within |a − e| ≤ abs + rel·max(|a|, |e|); everything else exactly.
fn close(actual: &Value, expected: &Value, abs: f64, rel: f64, path: &str) -> Result<(), String> {
    match (actual, expected) {
        (Value::Number(a), Value::Number(e)) => {
            let (a, e) = (a.as_f64().unwrap(), e.as_f64().unwrap());
            let d = (a - e).abs();
            if d <= abs + rel * a.abs().max(e.abs()) {
                Ok(())
            } else {
                Err(format!("{path}: {a} ≠ {e} (fark {d})"))
            }
        }
        (Value::Object(a), Value::Object(e)) => {
            for (k, ev) in e {
                close(
                    a.get(k).unwrap_or(&Value::Null),
                    ev,
                    abs,
                    rel,
                    &format!("{path}.{k}"),
                )?;
            }
            Ok(())
        }
        _ if actual == expected => Ok(()),
        _ => Err(format!("{path}: {actual} ≠ {expected}")),
    }
}

#[test]
fn golden_cases_match_the_typescript_reference() {
    let file: Value = serde_json::from_str(&std::fs::read_to_string(FILE).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.geometry-fixtures");
    assert_eq!(file["version"], 1);
    // The tolerance is written for projected metres; other systems need their own file and bound.
    assert_eq!(file["crs"]["kind"], "projected");
    assert_eq!(file["crs"]["unit"], "metre");
    let abs = file["tolerance"]["abs"].as_f64().unwrap();
    let rel = file["tolerance"]["rel"].as_f64().unwrap();
    let cases = file["cases"].as_array().expect("cases");
    assert!(!cases.is_empty());
    let failures: Vec<String> = cases
        .iter()
        .filter_map(|c| {
            let name = c["name"].as_str().unwrap();
            close(
                &run(c["op"].as_str().unwrap(), &c["input"]),
                &c["expected"],
                abs,
                rel,
                name,
            )
            .err()
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} golden case(s) differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// Accuracy against the independent exact reference (`fixtures/geometry/v1/reference.json`,
/// Python fractions and a 60-digit π): within each case's bound, as the TypeScript core is.
#[test]
fn geometry_stays_within_the_independent_reference_bounds() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../../fixtures/geometry/v1/reference.json"
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("reference file"))
        .expect("reference JSON");
    assert_eq!(file["format"], "kentos.geometry-reference");
    let num = |v: &Value| {
        v.as_str()
            .expect("decimal text")
            .parse::<f64>()
            .expect("number")
    };
    let ring_of = |v: &Value| Ring {
        pts: v["pts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| Vec2::new(num(&p[0]), num(&p[1])))
            .collect(),
        bulges: v["bulges"].as_array().map(|b| b.iter().map(num).collect()),
    };
    for c in file["cases"].as_array().unwrap() {
        let outer = ring_of(&c["input"]["outer"]);
        let holes: Vec<Ring> = c["input"]["holes"]
            .as_array()
            .unwrap()
            .iter()
            .map(ring_of)
            .collect();
        let got = match c["op"].as_str().unwrap() {
            "polygonArea" => polygon_area(&outer, &holes),
            "polygonPerimeter" => polygon_perimeter(&outer, &holes),
            other => panic!("unknown op {other}"),
        };
        let err = (got - num(&c["expected"])).abs();
        assert!(
            err <= num(&c["bound"]),
            "{}: error {err} over bound {}",
            c["name"],
            c["bound"]
        );
    }
}
