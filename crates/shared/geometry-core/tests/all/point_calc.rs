//! Nokta hesaplayıcı ekleri (docs/adr/0188) against the independent
//! reference in `fixtures/point-calc/v1/cases.json`
//! (`scripts/fixtures/point_calc_cases.py`: edges and arcs at 50 digits,
//! curves by their own arc length, the km by the display rule; no KentOS
//! code), through the op table as the web calls it
//! (`apps/web/src/tools/pointCalcExtras.wasm.test.ts`).

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/point-calc/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn call(op: &str, args: Value) -> Value {
    let out = run_named(op, &args.to_string()).unwrap_or_else(|e| panic!("{op}: {e}"));
    serde_json::from_str(&out).expect("JSON")
}

fn xy(v: &Value) -> Value {
    json!({ "x": v[0], "y": v[1] })
}

fn near(got: &Value, want: &Value, tol: f64, what: &str) {
    let (g, w) = (got.as_f64().expect(what), want.as_f64().expect(what));
    assert!((g - w).abs() <= tol, "{what}: {g} ≠ {w} (±{tol})");
}

fn point_near(got: &Value, want: &Value, tol: f64, what: &str) {
    if want.is_null() {
        assert!(
            got.is_null() || got.get("x").is_none(),
            "{what}: {got} ≠ none"
        );
        return;
    }
    near(&got["x"], &want[0], tol, what);
    near(&got["y"], &want[1], tol, what);
}

#[test]
fn a_paths_point_at_a_distance_and_an_offset() {
    for c in cases()["stations"].as_array().expect("stations") {
        let name = c["name"].as_str().expect("a name");
        let tol = c["tolerance"].as_f64().expect("a tolerance");
        let got = call(
            "pointCalcStation",
            json!([c["shape"], c["fromEnd"], c["s"], c["offset"]]),
        );
        let want = &c["expect"];
        point_near(&got["point"], &want["point"], tol, name);
        near(&got["length"], &want["length"], tol, name);
    }
}

#[test]
fn where_a_point_stands_against_a_path() {
    for c in cases()["readings"].as_array().expect("readings") {
        let name = c["name"].as_str().expect("a name");
        let tol = c["tolerance"].as_f64().expect("a tolerance");
        let got = call(
            "pointCalcReading",
            json!([c["shape"], c["fromEnd"], xy(&c["at"])]),
        );
        for key in ["s", "offset", "length"] {
            near(&got[key], &c["expect"][key], tol, &format!("{name}: {key}"));
        }
    }
}

#[test]
fn the_km_read_and_written() {
    let f = cases();
    for c in f["km"].as_array().expect("km") {
        let got = call("kmValue", json!([c["text"]]));
        match c["value"].as_f64() {
            Some(v) => near(&got, &json!(v), 1e-9, c["text"].as_str().expect("text")),
            None => assert!(got.is_null(), "{}: {got}", c["text"]),
        }
    }
    for c in f["kmText"].as_array().expect("kmText") {
        let got = call("kmText", json!([c["value"], c["decimals"]]));
        assert_eq!(got, c["text"], "{}", c["value"]);
    }
}

#[test]
fn a_slope_distance_and_the_bisector() {
    let f = cases();
    for c in f["slopes"].as_array().expect("slopes") {
        let got = call("slopeHorizontal", json!([c["s"], c["percent"]]));
        near(
            &got["horizontal"],
            &c["expect"]["horizontal"],
            1e-9,
            "horizontal",
        );
        near(&got["rise"], &c["expect"]["rise"], 1e-9, "rise");
    }
    for c in f["bisectors"].as_array().expect("bisectors") {
        let name = c["name"].as_str().expect("a name");
        let got = call(
            "bisectorPoint",
            json!([xy(&c["k"]), xy(&c["a"]), xy(&c["b"]), c["d"]]),
        );
        point_near(&got, &c["expect"], 1e-7, name);
    }
    for c in f["bisectorClicks"].as_array().expect("bisectorClicks") {
        let name = c["name"].as_str().expect("a name");
        let got = call(
            "bisectorNearest",
            json!([xy(&c["k"]), xy(&c["a"]), xy(&c["b"]), xy(&c["at"])]),
        );
        point_near(&got, &c["expect"], 1e-7, name);
    }
}
