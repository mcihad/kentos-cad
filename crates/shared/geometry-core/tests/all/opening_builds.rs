//! A door's or window's construction (docs/adr/0772 §3, §5) by hand: a 1.00 m rough opening in a 0.20 m wall, its
//! jambs at x = 1 and x = 2. Its leaf is the rough opening less the fitting gap and the frame each side; its swing zone
//! is that leaf's quarter circle about its hinge; its use zone the light's width, the depth asked, out of the face it
//! opens to. An empty construction draws as none (the reference cases in `openings.rs` hold the old symbols).

use kentos_geometry_core::api::json::{Flat, Json};
use kentos_geometry_core::geom::opening::{self, Spec};
use kentos_geometry_core::jsmath::{js_max, js_min};
use kentos_geometry_core::vec2::Vec2;

const TOL: f64 = 1e-9;

fn place() -> Vec<Vec2> {
    vec![
        Vec2::new(1.0, 0.1),
        Vec2::new(1.0, -0.1),
        Vec2::new(2.0, 0.1),
        Vec2::new(2.0, -0.1),
    ]
}

fn spec(json: &str) -> Spec {
    Spec::read_flat(&Json::parse(json).unwrap()).unwrap()
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (a.x - b.x, a.y - b.y);
    (dx * dx + dy * dy).sqrt()
}

fn area(r: &[Vec2]) -> f64 {
    let mut s = 0.0;
    for i in 0..r.len() {
        let (a, b) = (r[i], r[(i + 1) % r.len()]);
        s += a.x * b.y - b.x * a.y;
    }
    (s / 2.0).abs()
}

#[test]
fn a_leaf_is_the_rough_opening_less_its_gap_and_frame() {
    let s = spec(r#"{"form":"door","frame":0.05,"build":{"fit":0.01,"leafThickness":0.04}}"#);
    let zones = opening::swing_zones(&place(), &s);
    assert_eq!(zones.len(), 1);
    let z = &zones[0];
    let hinge = z[0];
    // Hinge at its start: 1 + 0.01 + 0.05 on one face.
    assert!((hinge.x - 1.06).abs() < TOL, "{hinge:?}");
    assert!((hinge.y.abs() - 0.1).abs() < TOL, "{hinge:?}");
    // Every arc point a leaf (1.00 − 2·0.01 − 2·0.05 = 0.88) from it; the first shut along the wall, the last at 90°.
    for p in &z[1..] {
        assert!((dist(*p, hinge) - 0.88).abs() < 1e-9, "{p:?}");
    }
    let (first, last) = (z[1], z[z.len() - 1]);
    assert!(
        (first.x - 1.94).abs() < TOL && (first.y - hinge.y).abs() < TOL,
        "{first:?}"
    );
    assert!(
        (last.x - 1.06).abs() < TOL && ((last.y - hinge.y).abs() - 0.88).abs() < TOL,
        "{last:?}"
    );
    // It sweeps away from the wall, out of the face its hinge is on.
    assert!((last.y > 0.0) == (hinge.y > 0.0));
}

#[test]
fn two_unequal_leaves_share_the_light() {
    let s = spec(r#"{"form":"door","frame":0.05,"leaves":2,"build":{"split":0.6}}"#);
    let zones = opening::swing_zones(&place(), &s);
    assert_eq!(zones.len(), 2);
    let radius = |z: &Vec<Vec2>| dist(z[1], z[0]);
    // Light 0.90: 0.54 and 0.36.
    assert!((radius(&zones[0]) - 0.54).abs() < TOL);
    assert!((radius(&zones[1]) - 0.36).abs() < TOL);
}

#[test]
fn a_use_zone_stands_out_of_the_face_it_opens_to() {
    for side in ["left", "right"] {
        let s = spec(&format!(
            r#"{{"form":"door","frame":0.05,"side":"{side}","build":{{"fit":0.01,"useZone":1.5}}}}"#
        ));
        let zone = opening::use_zone(&place(), &s).unwrap();
        assert!((area(&zone) - 0.88 * 1.5).abs() < 1e-9, "{side}: {zone:?}");
        let ys: Vec<f64> = zone.iter().map(|p| p.y).collect();
        let (lo, hi) = (
            ys.iter().cloned().fold(f64::MAX, js_min),
            ys.iter().cloned().fold(f64::MIN, js_max),
        );
        // From one face, 1.5 out: never through the wall.
        assert!(((hi - lo) - 1.5).abs() < TOL, "{side}: {ys:?}");
        assert!(lo >= 0.1 - TOL || hi <= -0.1 + TOL, "{side}: {ys:?}");
        // The swing zone opens to the same side.
        let swing = &opening::swing_zones(&place(), &s)[0];
        let far = swing[swing.len() - 1];
        assert!((far.y > 0.0) == (lo >= 0.1 - TOL), "{side}: {far:?}");
        // The use zone is drawn dashed among the pieces.
        let pieces = opening::pieces(&place(), &s);
        assert_eq!(pieces.iter().filter(|m| m.dashed).count(), 1);
    }
}

#[test]
fn only_a_swinging_door_has_swing_zones() {
    for json in [
        r#"{"form":"door","operation":"slide","leaves":2}"#,
        r#"{"form":"door","operation":"fixed"}"#,
        r#"{"form":"window","frame":0.05}"#,
        r#"{"form":"opening"}"#,
    ] {
        assert!(
            opening::swing_zones(&place(), &spec(json)).is_empty(),
            "{json}"
        );
    }
    let window = spec(r#"{"form":"window","frame":0.05,"build":{"useZone":0.6}}"#);
    assert!(opening::use_zone(&place(), &window).is_some());
    let bare = spec(r#"{"form":"opening","build":{"useZone":0.6}}"#);
    assert!(opening::use_zone(&place(), &bare).is_none());
}

#[test]
fn an_empty_construction_draws_as_none() {
    for json in [
        r#"{"form":"door","frame":0.05}"#,
        r#"{"form":"door","frame":0.05,"leaves":2}"#,
        r#"{"form":"window","frame":0.05,"leaves":2}"#,
        r#"{"form":"door","operation":"slide","leaves":2,"frame":0.04}"#,
    ] {
        let mut with: serde_json::Value = serde_json::from_str(json).unwrap();
        with["build"] = serde_json::json!({});
        let a = opening::pieces(&place(), &spec(json));
        let b = opening::pieces(&place(), &spec(&with.to_string()));
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "{json}");
        assert!(a.iter().all(|m| !m.dashed));
    }
}

#[test]
fn a_construction_adds_its_parts() {
    let plain = opening::symbol(&place(), &spec(r#"{"form":"door","frame":0.05}"#)).len();
    let built = opening::symbol(
        &place(),
        &spec(r#"{"form":"door","frame":0.05,"build":{"threshold":0.02,"casing":0.07,"leafThickness":0.04}}"#),
    )
    .len();
    assert!(built > plain, "{plain} → {built}");
    let window = opening::symbol(&place(), &spec(r#"{"form":"window","frame":0.05}"#)).len();
    let mullioned = opening::symbol(
        &place(),
        &spec(r#"{"form":"window","frame":0.05,"build":{"mullions":2,"sillBoard":0.05}}"#),
    )
    .len();
    assert!(mullioned >= window + 3, "{window} → {mullioned}");
}
