//! Paralel kaydır (docs/adr/0191) against the independent reference in
//! `fixtures/edge-shift/v1/cases.json` (`scripts/fixtures/edge_shift_cases.py`,
//! exact fractions and 50 digits, no KentOS code), through the op table as
//! the web calls it (`apps/web/src/tools/edgeShift.wasm.test.ts`).

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/edge-shift/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn near(got: &Value, want: &Value, tol: f64, what: &str) {
    let (g, w) = (got.as_f64().expect(what), want.as_f64().expect(what));
    assert!((g - w).abs() <= tol, "{what}: {g} ≠ {w} (±{tol})");
}

fn ring(got: &Value, want: &Value, what: &str) {
    let (g, w) = (got.as_array().expect(what), want.as_array().expect(what));
    assert_eq!(g.len(), w.len(), "{what}: vertices");
    for (p, q) in g.iter().zip(w) {
        near(&p["x"], &q[0], 1e-9, what);
        near(&p["y"], &q[1], 1e-9, what);
    }
}

#[test]
fn an_edge_moves_parallel_and_its_neighbours_follow() {
    for c in cases()["shifts"].as_array().expect("shifts") {
        let name = c["name"].as_str().expect("a name");
        let out = run_named(
            "edgeShift",
            &json!([c["shape"], c["ring"], c["edge"], c["distance"]]).to_string(),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        let got: Value = serde_json::from_str(&out).expect("JSON");
        let want = &c["expect"];
        if let Some(problem) = want.get("problem") {
            assert_eq!(&got["problem"], problem, "{name}");
            continue;
        }
        let e = &got["entity"];
        ring(&e["pts"], &want["pts"], name);
        match want.get("holes") {
            Some(holes) => {
                let got = e["holes"].as_array().expect("holes");
                for (h, w) in got.iter().zip(holes.as_array().expect("holes")) {
                    ring(&h["pts"], w, name);
                }
            }
            None => assert!(
                e.get("holes").is_none_or(Value::is_null),
                "{name}: no holes"
            ),
        }
        assert_eq!(
            e["bulges"],
            want.get("bulges").cloned().unwrap_or(Value::Null),
            "{name}: bulges kept"
        );
        assert_eq!(e["id"], c["shape"]["id"], "{name}: its other fields kept");
        match want.get("area") {
            Some(area) => near(&got["area"], area, 1e-9, name),
            None => assert!(got["area"].is_null(), "{name}: a polyline has no area"),
        }
    }
}

#[test]
fn the_distance_for_a_target_area() {
    for c in cases()["targets"].as_array().expect("targets") {
        let name = c["name"].as_str().expect("a name");
        let out = run_named(
            "edgeShiftForArea",
            &json!([c["shape"], c["ring"], c["edge"], c["target"]]).to_string(),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        let got: Value = serde_json::from_str(&out).expect("JSON");
        let want = &c["expect"];
        match want.get("problem") {
            Some(problem) => assert_eq!(&got["problem"], problem, "{name}"),
            None => near(&got["distance"], &want["distance"], 1e-9, name),
        }
    }
}

#[test]
fn the_edge_under_a_click() {
    let square = json!({"id": 1, "layerId": "a", "attrs": {}, "kind": "polygon",
        "pts": [{"x": 0, "y": 0}, {"x": 10, "y": 0}, {"x": 10, "y": 10}, {"x": 0, "y": 10}],
        "holes": [{"pts": [{"x": 4, "y": 4}, {"x": 6, "y": 4}, {"x": 6, "y": 6}, {"x": 4, "y": 6}]}]});
    let pick = |p: Value| -> Value {
        let out = run_named("edgeShiftPick", &json!([square, p]).to_string()).expect("runs");
        serde_json::from_str(&out).expect("JSON")
    };
    // The right edge, its normal outward.
    let got = pick(json!({"x": 10.2, "y": 3}));
    assert_eq!(got["picked"]["ring"], 0);
    assert_eq!(got["picked"]["edge"], 1);
    near(&got["picked"]["normal"]["x"], &json!(1.0), 1e-15, "outward");
    // The hole's top edge, its normal into the hole (away from the area's inside).
    let got = pick(json!({"x": 5, "y": 6.1}));
    assert_eq!(got["picked"]["ring"], 1);
    assert_eq!(got["picked"]["edge"], 2);
    near(
        &got["picked"]["normal"]["y"],
        &json!(-1.0),
        1e-15,
        "into the hole",
    );
}
