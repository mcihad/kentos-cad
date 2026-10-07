//! Plan yolu çizimi (docs/adr/0198) against the independent reference in
//! `fixtures/plan-road/v1/cases.json` (`scripts/fixtures/plan_road_cases.py`,
//! written from the ADR; no KentOS code): a road's areas from its axis, the
//! inner corners of an area rounded, two lines closed into a median; through
//! the op table as the web calls them (`apps/web/src/tools/planRoad.wasm.test.ts`).
//! A ring is compared from whichever of its vertices matches the
//! reference's first.

use kentos_geometry_core::api::run_named;
use kentos_geometry_core::jsmath::js_max;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/plan-road/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn run(name: &str, args: Value) -> Value {
    serde_json::from_str(
        &run_named(name, &args.to_string()).unwrap_or_else(|e| panic!("{name}: {e}")),
    )
    .expect("JSON")
}

fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * js_max(1.0, b.abs())
}

fn xy(p: &Value) -> (f64, f64) {
    (p["x"].as_f64().expect("x"), p["y"].as_f64().expect("y"))
}

/// A ring's vertices and bulges (0 for a straight edge).
fn ring(r: &Value) -> Vec<((f64, f64), f64)> {
    let pts = r["pts"].as_array().expect("pts");
    let bulges = r["bulges"].as_array();
    pts.iter()
        .enumerate()
        .map(|(i, p)| {
            let b = bulges
                .and_then(|b| b.get(i))
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            (xy(p), b)
        })
        .collect()
}

/// The two rings are the same from some vertex on.
fn same_ring(got: &Value, want: &Value, what: &str) {
    let (g, w) = (ring(got), ring(want));
    assert_eq!(g.len(), w.len(), "{what}: {got} ≠ {want}");
    let n = g.len();
    let matches = |k: usize| {
        (0..n).all(|i| {
            let ((gx, gy), gb) = g[(i + k) % n];
            let ((wx, wy), wb) = w[i];
            close(gx, wx) && close(gy, wy) && (gb - wb).abs() <= 1e-9
        })
    };
    assert!((0..n).any(matches), "{what}: {got} ≠ {want}");
}

fn same_area(got: &Value, want: &Value, what: &str) {
    same_ring(&got["outer"], &want["outer"], &format!("{what} outer"));
    let (g, w) = (
        got["holes"].as_array().expect("holes"),
        want["holes"].as_array().expect("holes"),
    );
    assert_eq!(g.len(), w.len(), "{what} holes");
    for (i, (gh, wh)) in g.iter().zip(w).enumerate() {
        same_ring(gh, wh, &format!("{what} hole {i}"));
    }
}

#[test]
fn roads_are_the_references_areas() {
    for c in cases()["roads"].as_array().expect("roads") {
        let name = c["name"].as_str().expect("name");
        let got = run(
            "roadParts",
            json!([c["axis"], c["width"], c["kerb"], c["median"]]),
        );
        let want = &c["want"];
        same_area(&got["road"], &want["road"], &format!("{name} road"));
        for part in ["carriageway", "median"] {
            assert_eq!(got[part].is_null(), want[part].is_null(), "{name} {part}");
            if !want[part].is_null() {
                same_area(&got[part], &want[part], &format!("{name} {part}"));
            }
        }
    }
}

#[test]
fn inner_corners_round_as_the_reference_rounds_them() {
    for c in cases()["corners"].as_array().expect("corners") {
        let name = c["name"].as_str().expect("name");
        let got = run("roundInnerCorners", json!([c["area"], c["radius"]]));
        same_area(&got["area"], &c["want"]["area"], name);
        assert_eq!(got["done"], c["want"]["done"], "{name} done");
        assert_eq!(got["skipped"], c["want"]["skipped"], "{name} skipped");
    }
}

#[test]
fn two_lines_close_into_the_references_median() {
    for c in cases()["medians"].as_array().expect("medians") {
        let name = c["name"].as_str().expect("name");
        let got = run("medianRing", json!([c["first"], c["second"], c["round"]]));
        same_ring(&got, &c["want"], name);
    }
}
