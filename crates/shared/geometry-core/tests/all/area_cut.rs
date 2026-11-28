//! Alanla böl (docs/adr/1069) against the independent reference in `fixtures/area-cut/v1/cases.json`
//! (`scripts/fixtures/area_cut_cases.py`: bisection on the clipped area at 50 digits, no KentOS code).

use kentos_geometry_core::ops::area_cut::cut_for_area;
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/area-cut/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

/// A coordinate to 1 nm, or to two units in the last place of a map coordinate's f64, whichever is larger: the
/// reference is exact to 50 digits, the core's f64 rounds its final sum at a large coordinate's ulp (≈ 1 nm at 4.4e6).
fn near(got: f64, want: f64, what: &str) {
    let tol = kentos_geometry_core::jsmath::js_max(2.0 * f64::EPSILON * want.abs(), 1e-9);
    assert!((got - want).abs() <= tol, "{what}: {got} ≠ {want} (±{tol})");
}

#[test]
fn the_cut_leaves_the_target_beside_the_edge() {
    let all = cases();
    let list = all["cases"].as_array().expect("cases");
    assert!(list.len() >= 20, "at least 20 cases");
    for c in list {
        let name = c["name"].as_str().expect("a name");
        let ring: Vec<Vec2> = c["ring"]
            .as_array()
            .expect("a ring")
            .iter()
            .map(|p| Vec2::new(num(&p[0]), num(&p[1])))
            .collect();
        let edge = c["edge"].as_u64().expect("an edge") as usize;
        let got = cut_for_area(&ring, edge, num(&c["target"]));
        let want = &c["expect"];
        if let Some(problem) = want.get("problem") {
            assert_eq!(got.err(), problem.as_str(), "{name}");
            continue;
        }
        let cut = got.unwrap_or_else(|e| panic!("{name}: {e}"));
        near(cut.a.x, num(&want["a"][0]), &format!("{name}: a.x"));
        near(cut.a.y, num(&want["a"][1]), &format!("{name}: a.y"));
        near(cut.b.x, num(&want["b"][0]), &format!("{name}: b.x"));
        near(cut.b.y, num(&want["b"][1]), &format!("{name}: b.y"));
        near(
            cut.distance,
            num(&want["distance"]),
            &format!("{name}: distance"),
        );
        // The pieces' areas as the shoelace gives them: to 1e-9 of the whole area.
        let scale = num(&want["near"]) + num(&want["far"]);
        for (g, w, what) in [
            (cut.near, &want["near"], "near"),
            (cut.far, &want["far"], "far"),
        ] {
            assert!(
                (g - num(w)).abs() <= 1e-9 * kentos_geometry_core::jsmath::js_max(scale, 1.0),
                "{name}: {what} {g} ≠ {w}"
            );
        }
    }
}
