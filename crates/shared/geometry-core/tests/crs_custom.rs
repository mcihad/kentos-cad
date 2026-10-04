//! The project's coordinate systems (docs/adr/0168 §1–§3) against PROJ in
//! `fixtures/geodesy/v1/custom.json` (`scripts/fixtures/crs_custom_cases.py`:
//! pyproj pipelines written from the ADR's rules, no KentOS code): a
//! transverse Mercator grid of any origin, a datum of the project's in
//! either rotation convention, local systems, the project's datum choices.
//! Grid points within 1e-6 m, latitudes and longitudes within 1e-11 degrees;
//! the accuracy, what the values rest on and whether an EPSG operation of
//! ED50 was used, exactly; a point with no value says why. The op the web
//! calls is run on the same cases through the call table.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::api::run_named;
use kentos_geometry_core::crs::{Choice, System, transform_in};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/geodesy/v1/custom.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.crs-custom");
    assert_eq!(file["version"], 1);
    file
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn read<T: FromJson>(v: &Value) -> T {
    let parsed = Json::parse(&v.to_string()).expect("the core reads the JSON");
    T::from_json(&parsed).unwrap_or_else(|e| panic!("{e}: {v}"))
}

#[test]
fn points_move_through_the_projects_systems_as_proj_moves_them() {
    let file = fixture();
    let cases = file["transforms"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let (from, to): (System, System) = (read(&case["from"]), read(&case["to"]));
        let choices: Vec<Choice> = read(&case["choices"]);
        let geographic = case["to"]["kind"] == "geographic";
        let tol = if geographic { 1e-11 } else { 1e-6 };
        for (p, want) in case["points"]
            .as_array()
            .expect("points")
            .iter()
            .zip(case["expect"].as_array().expect("expect"))
        {
            let p = Vec2::new(num(&p[0]), num(&p[1]));
            let got = transform_in(&from, &to, p, &choices);
            let args = json!([case["from"], case["to"], {"x": p.x, "y": p.y}, case["choices"]]);
            let out: Value = serde_json::from_str(
                &run_named("crsTransformIn", &args.to_string()).expect("the op runs"),
            )
            .expect("op JSON");
            if let Some(error) = want["error"].as_str() {
                assert_eq!(got.map_err(|e| e.as_str()), Err(error), "{name}");
                assert_eq!(out["error"], error, "{name}: the op");
                continue;
            }
            let got = got.unwrap_or_else(|e| panic!("{name}: {e:?}"));
            let w = Vec2::new(num(&want["point"][0]), num(&want["point"][1]));
            assert!(
                (got.point.x - w.x).abs() <= tol && (got.point.y - w.y).abs() <= tol,
                "{name}: ({}, {}) ≠ ({}, {}), off by ({:e}, {:e})",
                got.point.x,
                got.point.y,
                w.x,
                w.y,
                got.point.x - w.x,
                got.point.y - w.y
            );
            assert_eq!(got.accuracy, want["accuracy"].as_f64(), "{name}: accuracy");
            assert_eq!(got.via, want["via"].as_str().expect("via"), "{name}: via");
            assert_eq!(
                got.unofficial,
                want["unofficial"].as_bool().expect("unofficial"),
                "{name}: unofficial"
            );
            assert_eq!(
                (num(&out["point"]["x"]), num(&out["point"]["y"])),
                (got.point.x, got.point.y),
                "{name}: the op"
            );
            assert_eq!(
                out["accuracy"].as_f64(),
                got.accuracy,
                "{name}: the op's accuracy"
            );
        }
    }
}

/// The value with every object's fields in name order.
fn sorted(v: Json) -> Json {
    match v {
        Json::Obj(mut fields) => {
            fields.sort_by(|a, b| a.0.cmp(&b.0));
            Json::Obj(fields.into_iter().map(|(k, v)| (k, sorted(v))).collect())
        }
        Json::Arr(items) => Json::Arr(items.into_iter().map(sorted).collect()),
        v => v,
    }
}

#[test]
fn a_system_reads_back_as_it_was_written() {
    let file = fixture();
    for case in file["transforms"].as_array().expect("cases") {
        for side in ["from", "to"] {
            let s: System = read(&case[side]);
            // Compared as the core reads JSON (500000 and 500000.0 are one
            // number), the fields in any order (serde_json sorts them).
            let written = sorted(
                Json::parse(&kentos_geometry_core::api::json::to_string(&s))
                    .expect("the core's JSON"),
            );
            let given = sorted(Json::parse(&case[side].to_string()).expect("the fixture's JSON"));
            assert_eq!(written, given, "{}: {side}", case["name"]);
        }
    }
}
