//! The coordinate transforms (docs/adr/0167) against PROJ in
//! `fixtures/geodesy/v1/transform.json` (`scripts/fixtures/crs_transform_cases.py`:
//! pyproj, the EPSG operations by name, no KentOS code): grid points within
//! 1e-6 m, latitudes and longitudes within 1e-11 degrees, the accuracy and
//! the operations exactly; degrees, minutes and seconds written and read as
//! the reference's exact decimals give them. The ops the web calls are run
//! on the same cases through the call table.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::api::run_named;
use kentos_geometry_core::crs::{System, format_dd, format_dms, parse_angle, transform};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/geodesy/v1/transform.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.crs-transform");
    assert_eq!(file["version"], 1);
    file
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn pt(v: &Value) -> Vec2 {
    Vec2::new(num(&v[0]), num(&v[1]))
}

fn system(v: &Value) -> System {
    let parsed = Json::parse(&v.to_string()).expect("the core reads the JSON");
    System::from_json(&parsed).expect("a system")
}

#[test]
fn points_move_between_the_systems_as_proj_moves_them() {
    let file = fixture();
    let cases = file["transform"].as_array().expect("cases");
    assert!(cases.len() >= 30);
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let (from, to) = (system(&case["from"]), system(&case["to"]));
        let got = transform(&from, &to, pt(&case["p"])).unwrap_or_else(|| panic!("{name}: none"));
        let want = &case["expect"];
        let tol = if case["to"]["kind"] == "geographic" {
            1e-11
        } else {
            1e-6
        };
        let w = pt(&want["point"]);
        assert!(
            (got.point.x - w.x).abs() <= tol && (got.point.y - w.y).abs() <= tol,
            "{name}: ({}, {}) ≠ ({}, {})",
            got.point.x,
            got.point.y,
            w.x,
            w.y
        );
        assert_eq!(
            got.accuracy,
            Some(num(&want["accuracy"])),
            "{name}: accuracy"
        );
        assert_eq!(got.via, want["via"].as_str().expect("via"), "{name}: via");
        let args =
            json!([case["from"], case["to"], {"x": num(&case["p"][0]), "y": num(&case["p"][1])}]);
        let out: Value = serde_json::from_str(
            &run_named("crsTransform", &args.to_string()).expect("the op runs"),
        )
        .expect("op JSON");
        assert_eq!(
            (num(&out["point"]["x"]), num(&out["point"]["y"])),
            (got.point.x, got.point.y),
            "{name}: the op"
        );
    }
}

#[test]
fn angles_are_written_and_read_as_the_rules_say() {
    let file = fixture();
    for case in file["formatting"].as_array().expect("cases") {
        let (deg, latitude) = (
            num(&case["deg"]),
            case["latitude"].as_bool().expect("latitude"),
        );
        let decimals = case["decimals"].as_u64().expect("decimals") as usize;
        if let Some(want) = case["dms"].as_str() {
            assert_eq!(
                format_dms(deg, latitude, decimals),
                want,
                "{deg} {latitude}"
            );
        }
        if let Some(want) = case["dd"].as_str() {
            assert_eq!(format_dd(deg, latitude, decimals), want, "{deg} {latitude}");
        }
    }
    for case in file["parse"].as_array().expect("cases") {
        let text = case["text"].as_str().expect("text");
        let got = parse_angle(text);
        match case["expect"].as_f64() {
            None => assert_eq!(got, None, "{text:?}"),
            Some(want) => {
                let got = got.unwrap_or_else(|| panic!("{text:?}: none"));
                assert!((got - want).abs() <= 1e-12, "{text:?}: {got} ≠ {want}");
            }
        }
    }
}
