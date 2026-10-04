//! NTv2 grid shifts (docs/adr/0168 §3–§4) against PROJ in
//! `fixtures/geodesy/v1/ntv2.json` (`scripts/fixtures/ntv2_cases.py`: grids
//! written without KentOS code, PROJ's `hgridshift` on them): what each
//! grid says, points forward and back (latitudes and longitudes within
//! 1e-11 degrees), the project's grid choice between transverse Mercator
//! grids (within 1e-6 m), the reasons for no value and for a refused file,
//! exactly. The op the web calls is run on the transforms too.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::api::run_named;
use kentos_geometry_core::crs::{Choice, System, ntv2, transform_in};
use serde_json::{Value, json};

fn dir() -> String {
    format!(
        "{}/../../../fixtures/geodesy/v1",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn fixture() -> Value {
    let file: Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{}/ntv2.json", dir())).expect("fixture file"),
    )
    .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.ntv2");
    assert_eq!(file["version"], 1);
    file
}

fn bytes(file: &str) -> Vec<u8> {
    std::fs::read(format!("{}/ntv2/{file}", dir())).expect("grid file")
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn read<T: FromJson>(v: &Value) -> T {
    T::from_json(&Json::parse(&v.to_string()).expect("JSON")).unwrap_or_else(|e| panic!("{e}: {v}"))
}

/// Every grid of the reference, read and kept under its id.
fn load(file: &Value) {
    for g in file["grids"].as_array().expect("grids") {
        let grid = ntv2::read(&bytes(g["file"].as_str().expect("file"))).expect("a grid");
        ntv2::register(g["id"].as_str().expect("id"), grid);
    }
}

#[test]
fn grids_say_what_they_are() {
    let file = fixture();
    for g in file["grids"].as_array().expect("grids") {
        let name = g["file"].as_str().expect("file");
        let grid = ntv2::read(&bytes(name)).expect("a grid");
        assert_eq!(
            (grid.from.as_str(), grid.to.as_str()),
            ("ED50", "TUREF"),
            "{name}"
        );
        assert_eq!(
            grid.from_axes,
            (num(&g["fromAxes"][0]), num(&g["fromAxes"][1])),
            "{name}"
        );
        assert_eq!(
            grid.to_axes,
            (num(&g["toAxes"][0]), num(&g["toAxes"][1])),
            "{name}"
        );
        let (subgrids, extent) = grid.summary();
        assert_eq!(
            subgrids as u64,
            g["subgrids"].as_u64().expect("subgrids"),
            "{name}"
        );
        for (got, want) in extent.iter().zip(g["extent"].as_array().expect("extent")) {
            assert!((got - num(want)).abs() < 1e-9, "{name}: {extent:?}");
        }
    }
}

#[test]
fn broken_files_say_why() {
    let file = fixture();
    for b in file["broken"].as_array().expect("broken") {
        let name = b["file"].as_str().expect("file");
        assert_eq!(
            ntv2::read(&bytes(name)).err().map(|e| e.as_str()),
            b["error"].as_str(),
            "{name}"
        );
    }
}

#[test]
fn points_shift_as_proj_shifts_them() {
    let file = fixture();
    let rad = std::f64::consts::PI / 180.0;
    for case in file["shifts"].as_array().expect("shifts") {
        let name = case["name"].as_str().expect("name");
        let grid = ntv2::read(&bytes(case["file"].as_str().expect("file"))).expect("a grid");
        let reverse = case["reverse"].as_bool().expect("reverse");
        for (p, want) in case["points"]
            .as_array()
            .expect("points")
            .iter()
            .zip(case["expect"].as_array().expect("expect"))
        {
            let got = grid
                .apply(num(&p[0]) * rad, num(&p[1]) * rad, reverse)
                .map(|(lam, phi)| {
                    (
                        lam * (180.0 / std::f64::consts::PI),
                        phi * (180.0 / std::f64::consts::PI),
                    )
                });
            match (got, want.as_array()) {
                (None, None) => {}
                (Some((x, y)), Some(w)) => assert!(
                    (x - num(&w[0])).abs() <= 1e-11 && (y - num(&w[1])).abs() <= 1e-11,
                    "{name} {p}: ({x}, {y}) ≠ {want}, off by ({:e}, {:e})",
                    x - num(&w[0]),
                    y - num(&w[1])
                ),
                (got, _) => panic!("{name} {p}: {got:?} ≠ {want}"),
            }
        }
    }
}

#[test]
fn the_projects_grid_choice_moves_points_as_proj_does() {
    let file = fixture();
    load(&file);
    for case in file["transforms"].as_array().expect("transforms") {
        let name = case["name"].as_str().expect("name");
        let (from, to): (System, System) = (read(&case["from"]), read(&case["to"]));
        let choices: Vec<Choice> = read(&case["choices"]);
        for (p, want) in case["points"]
            .as_array()
            .expect("points")
            .iter()
            .zip(case["expect"].as_array().expect("expect"))
        {
            let p = Vec2::new(num(&p[0]), num(&p[1]));
            let got = transform_in(&from, &to, p, &choices);
            let out: Value = serde_json::from_str(
                &run_named(
                    "crsTransformIn",
                    &json!([case["from"], case["to"], {"x": p.x, "y": p.y}, case["choices"]])
                        .to_string(),
                )
                .expect("the op runs"),
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
                (got.point.x - w.x).abs() <= 1e-6 && (got.point.y - w.y).abs() <= 1e-6,
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
            assert!(!got.unofficial, "{name}");
            assert_eq!(
                (num(&out["point"]["x"]), num(&out["point"]["y"])),
                (got.point.x, got.point.y),
                "{name}: the op"
            );
        }
    }
}
