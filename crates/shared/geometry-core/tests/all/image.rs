//! Resim nesnesi (docs/adr/0192 §5) against the independent reference in
//! `fixtures/image/v1/cases.json` (`scripts/fixtures/image_cases.py`, exact
//! fractions, no KentOS code), through the op table as the web calls it
//! (`apps/web/src/tools/image.wasm.test.ts`): Resim ekle's frame from two
//! points, Resmi kırp's boundary in the picture's own fractions.

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/image/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn near(got: &Value, want: &Value, tol: f64, what: &str) {
    let (g, w) = (got.as_f64().expect(what), want.as_f64().expect(what));
    assert!((g - w).abs() <= tol, "{what}: {g} ≠ {w} (±{tol})");
}

#[test]
fn a_picture_is_placed_by_its_corner_and_a_second_point() {
    for c in cases()["placements"].as_array().expect("placements") {
        let name = c["name"].as_str().expect("a name");
        let out = run_named(
            "imagePlaced",
            &json!([c["p"], c["q"], c["aspect"]]).to_string(),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        let got: Value = serde_json::from_str(&out).expect("JSON");
        if let Some(problem) = c.get("problem") {
            assert_eq!(&got["problem"], problem, "{name}");
            assert!(got["placed"].is_null(), "{name}");
            continue;
        }
        for key in ["width", "height", "rotation"] {
            near(
                &got["placed"][key],
                &c["placed"][key],
                1e-12,
                &format!("{name}: {key}"),
            );
        }
    }
}

#[test]
fn a_boundary_is_cut_to_the_picture_in_its_own_fractions() {
    for c in cases()["clips"].as_array().expect("clips") {
        let name = c["name"].as_str().expect("a name");
        let out = run_named("imageClip", &json!([c["shape"], c["world"]]).to_string())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        let got: Value = serde_json::from_str(&out).expect("JSON");
        if let Some(problem) = c.get("problem") {
            assert_eq!(&got["problem"], problem, "{name}");
            assert!(got["clip"].is_null(), "{name}");
            continue;
        }
        let tol = c["tolerance"].as_f64().unwrap_or(1e-15);
        let (g, w) = (
            got["clip"].as_array().expect(name),
            c["clip"].as_array().expect(name),
        );
        assert_eq!(g.len(), w.len(), "{name}: corners {g:?}");
        for (p, q) in g.iter().zip(w) {
            near(&p["x"], &q["x"], tol, name);
            near(&p["y"], &q["y"], tol, name);
        }
    }
}
