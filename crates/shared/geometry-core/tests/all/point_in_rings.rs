//! A point in rings by the exact rule (docs/adr/9401 §9) against the independent reference
//! (fixtures/geometry/v1/point_in_rings.json, scripts/fixtures/point_in_rings_cases.py): a point on the edge two areas
//! share is in exactly one of them, at a drawing's size and at Transverse Mercator's; rings with a hole, of two parts,
//! and a ray through a corner.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geometry::{point_in_polygon, point_in_rings};
use serde_json::Value;

const FILE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/geometry/v1/point_in_rings.json"
);

fn point(v: &Value) -> Vec2 {
    Vec2::new(v["x"].as_f64().expect("x"), v["y"].as_f64().expect("y"))
}

fn ring(v: &Value) -> Vec<Vec2> {
    v.as_array().expect("a ring").iter().map(point).collect()
}

#[test]
fn a_point_on_a_shared_edge_is_in_exactly_one_area() {
    let cases: Value =
        serde_json::from_str(&std::fs::read_to_string(FILE).expect("the cases")).expect("json");
    let edges = cases["sharedEdges"].as_array().expect("shared edges");
    assert!(edges.len() > 200, "{} cases", edges.len());
    for c in edges {
        let (p, left, right) = (point(&c["p"]), ring(&c["left"]), ring(&c["right"]));
        let got = (point_in_polygon(p, &left), point_in_polygon(p, &right));
        let want = (
            c["expected"]["left"].as_bool().expect("left"),
            c["expected"]["right"].as_bool().expect("right"),
        );
        assert_eq!(got, want, "{} at {p:?}", c["name"]);
        assert_ne!(got.0, got.1, "in exactly one: {} at {p:?}", c["name"]);
        // The two areas as the rings of one: a point of the edge they share is inside once.
        assert!(point_in_rings(p, &[left, right]), "{} at {p:?}", c["name"]);
    }
}

#[test]
fn rings_are_taken_together_by_the_even_odd_rule() {
    let cases: Value =
        serde_json::from_str(&std::fs::read_to_string(FILE).expect("the cases")).expect("json");
    let many = cases["rings"].as_array().expect("rings");
    assert!(many.len() >= 17, "{} cases", many.len());
    for c in many {
        let rings: Vec<Vec<Vec2>> = c["rings"]
            .as_array()
            .expect("rings")
            .iter()
            .map(ring)
            .collect();
        let p = point(&c["p"]);
        assert_eq!(
            point_in_rings(p, &rings),
            c["expected"].as_bool().expect("expected"),
            "{} at {p:?}",
            c["name"]
        );
    }
    let none: [Vec<Vec2>; 0] = [];
    assert!(!point_in_rings(Vec2::new(0.0, 0.0), &none));
}
