//! A superelevation's axis of rotation and physics against the shared cases
//! (fixtures/road/v1/superelevation-physics.json, docs/adr/0931 §1): written by
//! scripts/fixtures/road_superelevation_physics_cases.py in fractions and at 40 digits, no KentOS code.

use kentos_road::superelevation_physics::{Pivot, Way, axis_lift, friction, relative_gradients, toward_centre};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/superelevation-physics.json");

fn n(v: &Value) -> f64 {
    v.as_f64().expect("a number")
}

fn pivot(v: &Value) -> Pivot {
    match v.as_str().unwrap() {
        "centerline" => Pivot::Centerline,
        "leftEdge" => Pivot::LeftEdge,
        "rightEdge" => Pivot::RightEdge,
        "insideEdge" => Pivot::InsideEdge,
        _ => Pivot::OutsideEdge,
    }
}

fn points(v: &Value) -> Vec<[f64; 2]> {
    v.as_array().map_or_else(Vec::new, |a| a.iter().map(|p| [n(&p[0]), n(&p[1])]).collect())
}

#[test]
fn the_axis_lifts_as_the_reference_says() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    for c in all["lift"].as_array().unwrap() {
        let pair = |k: &str| (n(&c[k][0]), n(&c[k][1]));
        let got = axis_lift(pivot(&c["pivot"]), pair("slopes"), pair("normal"), pair("widths"));
        assert!((got - n(&c["lift"])).abs() < 1e-12, "{}: {got} vs {}", c["name"], c["lift"]);
    }
}

#[test]
fn the_relative_gradients_are_the_references() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    for c in all["gradients"].as_array().unwrap() {
        let w = &c["way"];
        let way = Way {
            left_slope: points(&w["leftSlope"]),
            right_slope: points(&w["rightSlope"]),
            left_width: points(&w["leftWidth"]),
            right_width: points(&w["rightWidth"]),
        };
        let got = relative_gradients(&way, pivot(&c["pivot"]), n(&c["from"]), n(&c["to"]));
        let want = c["rows"].as_array().unwrap();
        assert_eq!(got.len(), want.len(), "{}", c["name"]);
        for (g, r) in got.iter().zip(want) {
            for (a, b, what) in [(g.from, &r["from"], "baş"), (g.to, &r["to"], "son"), (g.left, &r["left"], "sol"), (g.right, &r["right"], "sağ")] {
                assert!((a - n(b)).abs() < 1e-12, "{} {what}: {a} vs {b}", c["name"]);
            }
        }
    }
}

#[test]
fn the_side_friction_is_the_point_mass_balance() {
    let all: Value = serde_json::from_str(CASES).unwrap();
    for c in all["friction"].as_array().unwrap() {
        let got = friction(n(&c["speed"]), n(&c["radius"]), n(&c["e"]));
        assert!((got - n(&c["friction"])).abs() < 1e-12, "{}: {got} vs {}", c["name"], c["friction"]);
    }
    for c in all["toward"].as_array().unwrap() {
        let got = toward_centre(n(&c["left"]), n(&c["right"]), c["leftTurn"].as_bool().unwrap());
        assert!((got - n(&c["e"])).abs() < 1e-15, "{}: {got} vs {}", c["name"], c["e"]);
    }
}
