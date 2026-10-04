//! Lengths and areas in the second system's plane (docs/adr/0167 §2)
//! against PROJ in `fixtures/geodesy/v1/measure.json`
//! (`scripts/fixtures/crs_measure_cases.py`: pyproj and the ADR's rule, no
//! KentOS code): lengths within 1e-6 m, areas within 1e-6 m² for every 100 m
//! of their perimeter, no measure in
//! a geographic system or the Pseudo-Mercator, and the arcs cut into as
//! many pieces as the rule says. The op the web calls runs the same cases
//! through the call table.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::api::run_named;
use kentos_geometry_core::crs::System;
use kentos_geometry_core::crs::measure::{Ring, arc_pieces, plane_measures};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/geodesy/v1/measure.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.crs-measure");
    assert_eq!(file["version"], 1);
    file
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn system(v: &Value) -> System {
    System::from_json(&Json::parse(&v.to_string()).expect("JSON")).expect("a system")
}

fn rings(v: &Value) -> Vec<Ring> {
    v.as_array()
        .expect("rings")
        .iter()
        .map(|r| Ring {
            pts: r["pts"]
                .as_array()
                .expect("points")
                .iter()
                .map(|p| Vec2::new(num(&p[0]), num(&p[1])))
                .collect(),
            bulges: r["bulges"].as_array().map(|b| b.iter().map(num).collect()),
        })
        .collect()
}

/// The rings as the op takes them: points as `{x, y}`.
fn rings_json(v: &Value) -> Value {
    Value::Array(
        v.as_array()
            .expect("rings")
            .iter()
            .map(|r| {
                let pts: Vec<Value> = r["pts"]
                    .as_array()
                    .expect("points")
                    .iter()
                    .map(|p| json!({"x": p[0], "y": p[1]}))
                    .collect();
                json!({"pts": pts, "bulges": r.get("bulges").cloned().unwrap_or(Value::Null)})
            })
            .collect(),
    )
}

#[test]
fn paths_and_areas_measure_in_the_second_plane_as_proj_takes_them() {
    let file = fixture();
    let cases = file["measure"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let (from, to) = (system(&case["from"]), system(&case["to"]));
        let closed = case["closed"].as_bool().expect("closed");
        let got = plane_measures(&from, &to, &rings(&case["rings"]), closed);
        let args = json!([case["from"], case["to"], rings_json(&case["rings"]), closed]);
        let op: Value = serde_json::from_str(
            &run_named("crsPlaneMeasures", &args.to_string()).expect("the op runs"),
        )
        .expect("op JSON");
        match case["expect"].as_object() {
            None => {
                let why = case["why"].as_str().expect("why");
                assert_eq!(got.map_err(|e| e.as_str()), Err(why), "{name}");
                assert_eq!(op["why"], why, "{name}: the op");
            }
            Some(want) => {
                let m = got.unwrap_or_else(|e| panic!("{name}: {e:?}"));
                let length = num(&want["length"]);
                assert!(
                    (m.length - length).abs() <= 1e-6,
                    "{name}: length {} ≠ {length}",
                    m.length
                );
                // Points agree with PROJ's within nanometres: an area moves by its
                // perimeter times that, so 1e-6 m² for every 100 m of it.
                let area = want.get("area").map_or(0.0, num);
                assert!(
                    (m.area - area).abs() <= 1e-6 * (length / 100.0).max(1.0),
                    "{name}: area {} ≠ {area}",
                    m.area
                );
                assert_eq!(
                    (num(&op["length"]), num(&op["area"])),
                    (m.length, m.area),
                    "{name}: the op"
                );
            }
        }
    }
}

#[test]
fn arcs_are_cut_into_the_rules_pieces() {
    let file = fixture();
    for piece in file["pieces"].as_array().expect("pieces") {
        let a = Vec2::new(num(&piece["a"][0]), num(&piece["a"][1]));
        let b = Vec2::new(num(&piece["b"][0]), num(&piece["b"][1]));
        let bulge = num(&piece["bulge"]);
        let points = arc_pieces(a, b, bulge);
        assert_eq!(
            points.len() as u64,
            piece["points"].as_u64().expect("points"),
            "bulge {bulge}"
        );
        assert_eq!(
            (points[0], points[points.len() - 1]),
            (a, b),
            "the ends are the vertices"
        );
    }
}
