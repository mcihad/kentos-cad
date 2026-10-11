//! Hat boyunca kilometre, Km yaz (docs/adr/0189), and Km yaz by a measured
//! route's measures (docs/adr/0224 §6, `measureCases`) against the
//! independent reference in `fixtures/stationing/v1/cases.json`
//! (`scripts/fixtures/stationing_cases.py`, 50 digits, no KentOS code),
//! through the op table as the web calls it
//! (`apps/web/src/tools/stationing.wasm.test.ts`).

use kentos_geometry_core::api::run_named;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/stationing/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn near(got: &Value, want: &Value, tol: f64, what: &str) {
    let (g, w) = (got.as_f64().expect(what), want.as_f64().expect(what));
    assert!((g - w).abs() <= tol, "{what}: {g} ≠ {w} (±{tol})");
}

fn point(got: &Value, want: &Value, what: &str) {
    near(&got["x"], &want[0], 1e-7, what);
    near(&got["y"], &want[1], 1e-7, what);
}

/// The core's answer for a case against the reference's.
fn compare(name: &str, out: &str, want: &Value) {
    let got: Value = serde_json::from_str(out).expect("JSON");
    if let Some(problem) = want.get("problem") {
        assert_eq!(&got["problem"], problem, "{name}");
        return;
    }
    let got = &got["stationing"];
    let stations = got["stations"].as_array().expect("stations");
    let expected = want["stations"].as_array().expect("stations");
    assert_eq!(stations.len(), expected.len(), "{name}: stations");
    for (g, w) in stations.iter().zip(expected) {
        assert_eq!(g["text"], w["text"], "{name}");
        near(&g["s"], &w["s"], 1e-7, name);
        point(&g["point"], &w["point"], name);
        point(&g["tangent"], &w["tangent"], name);
    }
    for key in ["texts", "ticks", "sections", "points"] {
        let (g, w) = (
            got[key].as_array().expect(key),
            want[key].as_array().expect(key),
        );
        assert_eq!(g.len(), w.len(), "{name}: {key}");
        for (g, w) in g.iter().zip(w) {
            match key {
                "texts" => {
                    point(&g["p"], &w["p"], name);
                    near(&g["rotation"], &w["rotation"], 1e-9, name);
                    assert_eq!(g["align"], w["align"], "{name}");
                    assert_eq!(g["text"], w["text"], "{name}");
                }
                "points" => {
                    point(&g["p"], &w["p"], name);
                    assert_eq!(g["km"], w["km"], "{name}");
                }
                _ => {
                    point(&g["a"], &w["a"], name);
                    point(&g["b"], &w["b"], name);
                    assert_eq!(g["km"], w["km"], "{name}");
                }
            }
        }
    }
}

#[test]
fn the_stations_of_a_route_and_what_is_written_at_them() {
    for c in cases()["cases"].as_array().expect("cases") {
        let name = c["name"].as_str().expect("a name");
        let out = run_named(
            "stationing",
            &json!([c["shape"], c["rules"], c["look"]]).to_string(),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        compare(name, &out, &c["expect"]);
    }
}

#[test]
fn the_stations_of_a_measured_route_by_its_measures() {
    let all = cases();
    let measured = all["measureCases"].as_array().expect("measure cases");
    assert!(!measured.is_empty(), "no measure cases");
    for c in measured {
        let name = c["name"].as_str().expect("a name");
        let out = run_named(
            "stationingMeasure",
            &json!([c["route"], c["rules"], c["look"]]).to_string(),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        compare(name, &out, &c["expect"]);
    }
}
