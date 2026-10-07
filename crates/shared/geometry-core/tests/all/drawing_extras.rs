//! Çizim ekleri (docs/adr/0197) against the independent reference in
//! `fixtures/drawing-extras/v1/cases.json` (`scripts/fixtures/drawing_extras_cases.py`,
//! written from the ADR; no KentOS code): two circles' or arcs' common
//! tangents and the one two clicks choose, a parallelogram's fourth corner,
//! range rings and their rays; through the op table as the web calls them
//! (`apps/web/src/tools/drawingExtras.wasm.test.ts`).

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/drawing-extras/v1/cases.json",
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

fn near(got: &Value, want: &Value, what: &str) {
    for k in ["x", "y"] {
        let (g, w) = (
            got[k].as_f64().expect("x, y"),
            want[k].as_f64().expect("x, y"),
        );
        let tolerance = 1e-9 * kentos_geometry_core::jsmath::js_max(w.abs(), 1.0);
        assert!((g - w).abs() <= tolerance, "{what}.{k}: {g} ≠ {w}");
    }
}

fn same_tangent(got: &Value, want: &Value, what: &str) {
    assert_eq!(got["kind"], want["kind"], "{what}");
    near(&got["a"], &want["a"], &format!("{what} a"));
    near(&got["b"], &want["b"], &format!("{what} b"));
}

#[test]
fn common_tangents_are_the_references() {
    for c in cases()["tangents"].as_array().expect("tangents") {
        let name = c["name"].as_str().expect("name");
        let got = run("commonTangents", json!([c["first"], c["second"]]));
        let (got, want) = (
            got.as_array().expect("list"),
            c["want"].as_array().expect("list"),
        );
        assert_eq!(got.len(), want.len(), "{name}: {got:?}");
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            same_tangent(g, w, &format!("{name} #{i}"));
        }
    }
}

#[test]
fn the_clicks_choose_the_references_tangent() {
    for c in cases()["choices"].as_array().expect("choices") {
        let name = c["name"].as_str().expect("name");
        let got = run(
            "chosenTangent",
            json!([c["first"], c["second"], c["p1"], c["p2"]]),
        );
        assert_eq!(got.is_null(), c["want"].is_null(), "{name}");
        if !got.is_null() {
            same_tangent(&got, &c["want"], name);
        }
    }
}

#[test]
fn fourth_corners_and_range_rings_are_the_references() {
    let all = cases();
    for c in all["fourth"].as_array().expect("fourth") {
        let name = c["name"].as_str().expect("name");
        near(
            &run("fourthCorner", json!([c["a"], c["b"], c["c"]])),
            &c["want"],
            name,
        );
    }
    for c in all["rings"].as_array().expect("rings") {
        let name = c["name"].as_str().expect("name");
        let got = run(
            "rangeRings",
            json!([c["center"], c["spacing"], c["count"], c["rays"]]),
        );
        let radii = got["radii"].as_array().expect("radii");
        let want = c["want"]["radii"].as_array().expect("radii");
        assert_eq!(radii.len(), want.len(), "{name}");
        for (g, w) in radii.iter().zip(want) {
            let (g, w) = (g.as_f64().expect("radius"), w.as_f64().expect("radius"));
            let tolerance = 1e-9 * kentos_geometry_core::jsmath::js_max(w.abs(), 1.0);
            assert!((g - w).abs() <= tolerance, "{name}: {g} ≠ {w}");
        }
        let rays = got["rays"].as_array().expect("rays");
        let want = c["want"]["rays"].as_array().expect("rays");
        assert_eq!(rays.len(), want.len(), "{name}");
        for (i, (g, w)) in rays.iter().zip(want).enumerate() {
            near(g, w, &format!("{name} ray {i}"));
        }
    }
    // Out of range: none.
    assert!(run("rangeRings", json!([{"x": 0, "y": 0}, 0, 5, 0])).is_null());
    assert!(run("rangeRings", json!([{"x": 0, "y": 0}, 10, 2.5, 0])).is_null());
}
