//! What rules the next point's place (docs/adr/0268 §1), through the session on the polyline tool: a locked
//! coordinate, a locked direction, Dik açı, an object snap's or tracking's point, ortho (Shift turning it over),
//! polar tracking, else the pointer; ortho comes before polar tracking.

use crate::common;

use common::Bench;
use kentos_interaction::{Aid, Pointer};

fn aid(b: &mut Bench, p: Pointer) -> Aid {
    b.run(|s, cx| s.ruling_aid(&p, cx))
}

fn shifted(de: f64, dn: f64) -> Pointer {
    let mut p = Bench::pointer(de, dn);
    p.shift = true;
    p
}

#[test]
fn with_no_point_yet_nothing_rules_and_with_no_command_neither() {
    let mut b = Bench::new("polyline");
    b.draft.ortho = true;
    assert_eq!(aid(&mut b, Bench::pointer(5.0, 3.0)), Aid::Free);
    b.run(|s, _| s.exit());
    assert_eq!(aid(&mut b, Bench::pointer(5.0, 3.0)), Aid::Free);
}

#[test]
fn shift_turns_ortho_over_both_ways() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert_eq!(aid(&mut b, Bench::pointer(20.0, 3.0)), Aid::Free);
    assert_eq!(aid(&mut b, shifted(20.0, 3.0)), Aid::Ortho);
    b.draft.ortho = true;
    assert_eq!(aid(&mut b, Bench::pointer(20.0, 3.0)), Aid::Ortho);
    assert_eq!(aid(&mut b, shifted(20.0, 3.0)), Aid::Free);
}

#[test]
fn ortho_comes_before_polar_tracking() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.draft.polar = Some(45.0);
    // On the 45° ray: polar tracking.
    assert_eq!(aid(&mut b, Bench::pointer(10.0, 10.1)), Aid::Polar);
    // Off every ray: the pointer itself.
    assert_eq!(aid(&mut b, Bench::pointer(10.0, 4.0)), Aid::Free);
    b.draft.ortho = true;
    assert_eq!(aid(&mut b, Bench::pointer(10.0, 10.1)), Aid::Ortho);
    // Shift lets ortho go for the moment: polar tracking again, its step kept.
    assert_eq!(aid(&mut b, shifted(10.0, 10.1)), Aid::Polar);
    b.click(10.0, 10.1);
    b.confirm();
    let kentos_contracts::Entity::Polyline(path) = b.newest() else {
        panic!("a polyline");
    };
    let end = common::rel(path.pts[1]);
    // The northing is the larger: north along the axis.
    assert!(end[0].abs() < 1e-9, "ortho's point, not the ray's: {end:?}");
}

#[test]
fn an_exact_point_comes_before_ortho_and_polar() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.draft.ortho = true;
    b.draft.polar = Some(45.0);
    let mut p = Bench::pointer(7.0, 3.0);
    p.tracked = true;
    assert_eq!(aid(&mut b, p), Aid::Tracking);
}

#[test]
fn locks_come_before_everything_else() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.draft.ortho = true;
    b.draft.polar = Some(45.0);
    // A length alone holds with what rules the direction.
    assert!(b.run(|s, cx| s.lock_length(5.0, cx)));
    assert_eq!(aid(&mut b, Bench::pointer(20.0, 3.0)), Aid::Ortho);
    assert!(b.type_text("<100"));
    let mut p = Bench::pointer(7.0, 3.0);
    p.tracked = true;
    assert_eq!(
        aid(&mut b, p),
        Aid::Direction,
        "a locked direction projects even an exact point"
    );
    assert!(b.run(|s, cx| s.lock_coordinate(false, common::E + 2.0, cx)));
    assert_eq!(aid(&mut b, Bench::pointer(7.0, 3.0)), Aid::Coordinates);
}

#[test]
fn dik_aci_comes_after_a_locked_direction() {
    let mut b = Bench::new("polyline");
    b.draft.right_angle = true;
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert_eq!(aid(&mut b, Bench::pointer(14.0, 6.0)), Aid::RightAngle);
    assert!(b.type_text("<50"));
    assert_eq!(aid(&mut b, Bench::pointer(14.0, 6.0)), Aid::Direction);
}

#[test]
fn every_aid_has_its_trace_name() {
    let names: Vec<&str> = [
        Aid::Free,
        Aid::Coordinates,
        Aid::Direction,
        Aid::RightAngle,
        Aid::Snap,
        Aid::Tracking,
        Aid::Ortho,
        Aid::Polar,
    ]
    .into_iter()
    .map(Aid::key)
    .collect();
    assert_eq!(
        names,
        [
            "free",
            "coordinates",
            "direction",
            "rightAngle",
            "snap",
            "tracking",
            "ortho",
            "polar"
        ]
    );
}
