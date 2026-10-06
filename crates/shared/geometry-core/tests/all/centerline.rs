//! Orta hat (docs/adr/0190) against the independent reference in
//! `fixtures/centerline/v1/cases.json` (`scripts/fixtures/centerline_cases.py`,
//! 50 digits, no KentOS code), through the op table as the web calls it
//! (`apps/web/src/tools/centerline.wasm.test.ts`).

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/centerline/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn near(got: &Value, want: &Value, tol: f64, what: &str) {
    let (g, w) = (got.as_f64().expect(what), want.as_f64().expect(what));
    assert!((g - w).abs() <= tol, "{what}: {g} ≠ {w} (±{tol})");
}

#[test]
fn the_axis_between_two_sides() {
    for c in cases()["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().expect("a name");
        let out = run_named("centerline", &json!([c["a"], c["b"], c["step"]]).to_string())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let got: Value = serde_json::from_str(&out).expect("JSON");
        let want = &c["expect"];
        if let Some(problem) = want.get("problem") {
            assert_eq!(&got["problem"], problem, "{name}");
            continue;
        }
        let axis = &got["centerline"];
        assert_eq!(axis["method"], want["method"], "{name}");
        let (pts, expected) = (
            axis["pts"].as_array().expect("pts"),
            want["pts"].as_array().expect("pts"),
        );
        assert_eq!(pts.len(), expected.len(), "{name}: vertices");
        for (g, w) in pts.iter().zip(expected) {
            near(&g["x"], &w[0], 1e-7, name);
            near(&g["y"], &w[1], 1e-7, name);
        }
        match want["bulges"].as_array() {
            Some(bulges) => {
                let got = axis["bulges"].as_array().expect("bulges");
                assert_eq!(got.len(), bulges.len(), "{name}: bulges");
                for (g, w) in got.iter().zip(bulges) {
                    near(g, w, 1e-12, name);
                }
            }
            None => assert!(axis.get("bulges").is_none(), "{name}: no bulges"),
        }
    }
}
