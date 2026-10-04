//! Plane, ellipsoid and ground (docs/adr/0171) against
//! `fixtures/geodesy/v1/ground.json` (`scripts/fixtures/ground_cases.py`:
//! PROJ and GeographicLib's C library through pyproj, and mpmath; no KentOS
//! code): point and line scales and height factors within 1e-10 (PROJ's
//! factors are numerical derivatives), lengths within 1e-6 m, areas within
//! 1e-6 m² for every 100 m of their perimeter and four times the floor of
//! GeographicLib's polygon of their vertices (`areaNoise`: how far it may
//! move for one ulp in each of its coordinates), a measure's scale as its
//! values allow; none where the reference has none. The ops the web calls
//! run the same cases through the call table, and their answers are frozen
//! in `ground-answers.json`, which the web's WASM must give bit for bit.

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::api::run_named;
use kentos_geometry_core::crs::System;
use kentos_geometry_core::crs::ground::{
    GroundMeasures, ground_measures, line_factors, line_scale, point_scale,
};
use kentos_geometry_core::crs::measure::Ring;
use kentos_geometry_core::geodesy::{Tm, tm_scale};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/geodesy/v1/ground.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.ground");
    assert_eq!(file["version"], 1);
    file
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn system(v: &Value) -> System {
    System::from_json(&Json::parse(&v.to_string()).expect("JSON")).expect("a system")
}

fn point(v: &Value) -> Vec2 {
    Vec2::new(num(&v[0]), num(&v[1]))
}

fn xy(v: &Value) -> Value {
    json!({"x": v[0], "y": v[1]})
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
                .map(point)
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
                    .map(xy)
                    .collect();
                json!({"pts": pts, "bulges": r.get("bulges").cloned().unwrap_or(Value::Null)})
            })
            .collect(),
    )
}

fn op(name: &str, args: Value) -> Value {
    serde_json::from_str(&run_named(name, &args.to_string()).expect("the op runs"))
        .expect("op JSON")
}

fn close(got: Option<f64>, want: &Value, tolerance: f64, what: &str) {
    match (got, want.as_f64()) {
        (None, None) => {}
        (Some(g), Some(w)) => assert!((g - w).abs() <= tolerance, "{what}: {g} ≠ {w}"),
        (g, _) => panic!("{what}: {g:?} ≠ {want}"),
    }
}

#[test]
fn the_scale_is_the_scale_factor_on_the_central_meridian() {
    for (k0, lat) in [(1.0, 39.0), (0.9996, 41.5), (0.9999, 36.25)] {
        let tm = Tm {
            central_meridian: 33.0,
            scale_factor: k0,
            false_easting: 500_000.0,
            false_northing: 0.0,
            semi_major: 6_378_137.0,
            inverse_flattening: 298.257_222_101,
        };
        let k = tm_scale(&tm, lat, 33.0).expect("a scale");
        assert!((k - k0).abs() <= 1e-15, "{k} at {lat}°");
        // The grid is symmetric about its central meridian.
        assert_eq!(tm_scale(&tm, lat, 34.2), tm_scale(&tm, lat, 31.8));
    }
}

#[test]
fn point_scales_are_projs() {
    let file = fixture();
    let cases = file["scales"].as_array().expect("cases");
    assert!(cases.len() >= 15);
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let s = system(&case["system"]);
        let got = point_scale(&s, point(&case["point"]));
        close(got, &case["expect"], 1e-10, name);
        let by_op = op("crsPointScale", json!([case["system"], xy(&case["point"])]));
        assert_eq!(by_op.as_f64(), got, "{name}: the op");
    }
}

#[test]
fn line_factors_are_simpsons_and_eulers() {
    let file = fixture();
    let cases = file["lines"].as_array().expect("cases");
    assert!(cases.len() >= 10);
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let s = system(&case["system"]);
        let (a, b) = (point(&case["a"]), point(&case["b"]));
        let height = case["height"].as_f64();
        let got = line_factors(&s, a, b, height);
        let by_op = op(
            "crsLineFactors",
            json!([
                case["system"],
                xy(&case["a"]),
                xy(&case["b"]),
                case["height"]
            ]),
        );
        let Some(want) = case["expect"].as_object() else {
            assert!(got.is_err(), "{name}: {got:?}");
            assert_eq!(by_op["why"], "unreachable", "{name}: the op");
            continue;
        };
        let got = got.unwrap_or_else(|_| panic!("{name}: unreachable"));
        close(
            Some(got.ellipsoid_length),
            &want["ellipsoidLength"],
            1e-6,
            &format!("{name}: geodesic"),
        );
        let missing = Value::Null;
        close(
            got.scale,
            want.get("scale").unwrap_or(&missing),
            1e-10,
            &format!("{name}: scale"),
        );
        assert_eq!(got.scale, line_scale(&s, a, b), "{name}: Simpson's");
        close(
            got.height_factor,
            want.get("heightFactor").unwrap_or(&missing),
            1e-10,
            &format!("{name}: height factor"),
        );
        assert_eq!(
            by_op["ellipsoidLength"].as_f64(),
            Some(got.ellipsoid_length)
        );
        assert_eq!(by_op["scale"].as_f64(), got.scale, "{name}: the op");
        assert_eq!(
            by_op["heightFactor"].as_f64(),
            got.height_factor,
            "{name}: the op"
        );
    }
}

#[test]
fn paths_and_areas_measure_in_the_plane_on_the_ellipsoid_and_on_the_ground() {
    let file = fixture();
    let cases = file["measures"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    for case in cases {
        let name = case["name"].as_str().expect("name");
        let s = system(&case["system"]);
        let closed = case["closed"].as_bool().expect("closed");
        let height = case["height"].as_f64();
        let got = ground_measures(&s, &rings(&case["rings"]), closed, height);
        let by_op = op(
            "crsGroundMeasures",
            json!([
                case["system"],
                rings_json(&case["rings"]),
                closed,
                case["height"]
            ]),
        );
        let Some(want) = case["expect"].as_object() else {
            assert_eq!(case["why"], "unreachable");
            assert!(got.is_err(), "{name}: {got:?}");
            assert_eq!(by_op["why"], "unreachable", "{name}: the op");
            continue;
        };
        let m: GroundMeasures = got.unwrap_or_else(|_| panic!("{name}: unreachable"));
        let perimeter = num(&want["ellipsoidLength"]);
        let area =
            1e-6 * (perimeter / 100.0).max(1.0) + 4.0 * case["areaNoise"].as_f64().unwrap_or(0.0);
        // A path's scale is its lengths' ratio, an area's its areas' ratio's
        // square root: their relative tolerance, or half of it.
        let scale = 1e-10
            + want
                .get("ellipsoidArea")
                .map_or_else(|| 1e-6 / perimeter.max(1e-6), |a| area / (2.0 * num(a)));
        let missing = Value::Null;
        for (key, value, tolerance) in [
            ("planeLength", m.plane_length, 1e-6),
            ("planeArea", m.plane_area, area),
            ("ellipsoidLength", Some(m.ellipsoid_length), 1e-6),
            ("ellipsoidArea", m.ellipsoid_area, area),
            ("groundLength", m.ground_length, 1e-6),
            ("groundArea", m.ground_area, area),
            ("scale", m.scale, scale),
            ("heightFactor", m.height_factor, 1e-10),
        ] {
            close(
                value,
                want.get(key).unwrap_or(&missing),
                tolerance,
                &format!("{name}: {key}"),
            );
            assert_eq!(by_op[key].as_f64(), value, "{name}: the op's {key}");
        }
    }
}

/// Every case's op and arguments, as the web calls them.
fn calls(file: &Value) -> Vec<(&'static str, Value)> {
    let mut out = Vec::new();
    for c in file["scales"].as_array().expect("scales") {
        out.push(("crsPointScale", json!([c["system"], xy(&c["point"])])));
    }
    for c in file["lines"].as_array().expect("lines") {
        out.push((
            "crsLineFactors",
            json!([c["system"], xy(&c["a"]), xy(&c["b"]), c["height"]]),
        ));
    }
    for c in file["measures"].as_array().expect("measures") {
        out.push((
            "crsGroundMeasures",
            json!([
                c["system"],
                rings_json(&c["rings"]),
                c["closed"],
                c["height"]
            ]),
        ));
    }
    out
}

/// The ops' answers to every case, frozen in
/// `fixtures/geodesy/v1/ground-answers.json`: the web's WASM must give the
/// same bits (`apps/web/src/wasm/crsGround.wasm.test.ts`; geodesics through
/// the libm copy of GeographicLib, docs/adr/0171). `KENTOS_WRITE_GROUND=1`
/// writes them anew; read the difference.
#[test]
fn the_answers_are_frozen_for_every_target() {
    let file = fixture();
    let answers: Vec<Value> = calls(&file)
        .into_iter()
        .map(|(name, args)| {
            let answer: Value =
                serde_json::from_str(&run_named(name, &args.to_string()).expect("the op runs"))
                    .expect("op JSON");
            json!({"op": name, "args": args, "answer": answer})
        })
        .collect();
    let path = format!(
        "{}/../../../fixtures/geodesy/v1/ground-answers.json",
        env!("CARGO_MANIFEST_DIR")
    );
    if std::env::var_os("KENTOS_WRITE_GROUND").is_some() {
        let text = serde_json::to_string_pretty(&json!({
            "format": "kentos.ground-answers",
            "version": 1,
            "answers": answers,
        }))
        .expect("JSON");
        std::fs::write(&path, text + "\n").expect("writes the answers");
        return;
    }
    let frozen: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("the answers' file"))
            .expect("answers JSON");
    assert_eq!(frozen["format"], "kentos.ground-answers");
    let frozen = frozen["answers"].as_array().expect("answers");
    assert_eq!(frozen.len(), answers.len(), "a case without its answer");
    for (want, got) in frozen.iter().zip(&answers) {
        assert_eq!(
            want, got,
            "the answer changed: KENTOS_WRITE_GROUND=1 writes it anew"
        );
    }
}
