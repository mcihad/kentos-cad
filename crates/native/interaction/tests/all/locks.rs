//! The digitizing locks through the session (docs/adr/0166 §1–§2, §6), on
//! the polyline tool over the empty drawing (a project not asked its type:
//! CBS, grads): a locked length and a typed direction hold the next point,
//! one-shot locks go once it is placed and kept ones follow, Sapma turns
//! from the last edge, Esc lets the locks go before anything else.

use crate::common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_interaction::{Level, LockPick, NO_LOCK_EDGE, Toward};

/// The newest object's points, relative to (E, N).
fn points(b: &Bench) -> Vec<[f64; 2]> {
    let Entity::Polyline(path) = b.newest() else {
        panic!("a polyline, not {:?}", b.newest());
    };
    path.pts.iter().map(|p| rel(*p)).collect()
}

fn near(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).abs() < 1e-6 && (a[1] - b[1]).abs() < 1e-6
}

#[test]
fn a_locked_length_holds_the_next_point_and_goes_once_it_is_placed() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert!(b.run(|s, cx| s.lock_length(12.0, cx)));
    assert_eq!(b.last_text(), Some("Kilit: Uzunluk 12.000 m."));
    // Towards the cursor (3, 4): 12 m along (0.6, 0.8).
    b.click(30.0, 40.0);
    assert!(!b.locks.any(), "a one-shot lock goes with the point");
    b.click(7.2, 20.0);
    b.confirm();
    let pts = points(&b);
    assert!(near(pts[1], [7.2, 9.6]), "{pts:?}");
    assert!(near(pts[2], [7.2, 20.0]), "the next point is free: {pts:?}");
}

#[test]
fn a_typed_angle_locks_the_direction_one_way() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    // A CBS project's semt in grads: 100 is east.
    assert!(b.type_text("<100"));
    assert_eq!(b.locks.toward, Some(Toward::Angle(100.0)));
    assert_eq!(b.last_text(), Some("Kilit: Semt 100.0000 g."));
    b.move_to(20.0, 7.0);
    // A bare distance follows the locked direction (the cursor's place on it).
    assert!(b.type_text("15"));
    b.confirm();
    assert!(near(points(&b)[1], [15.0, 0.0]), "{:?}", points(&b));
}

#[test]
fn a_cursor_behind_a_one_way_direction_gives_the_reference() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert!(b.type_text("<100"));
    b.click(-20.0, 3.0);
    assert_eq!(b.points(), 1, "no edge of no length");
    assert!(b.locks.any(), "the lock waits for its point");
}

#[test]
fn kept_locks_follow_the_reference() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.run(|s, cx| s.keep_locks(true, cx));
    assert!(b.run(|s, cx| s.lock_length(5.0, cx)));
    b.click(0.0, 30.0);
    b.click(30.0, 5.0);
    assert_eq!(b.locks.length, Some(5.0), "kept");
    b.confirm();
    let pts = points(&b);
    assert!(
        near(pts[1], [0.0, 5.0]) && near(pts[2], [5.0, 5.0]),
        "{pts:?}"
    );
    // The command waits for a new path: no reference, so no lock; Kalıcı stays with the command.
    assert!(!b.locks.any() && b.locks.keep);
    b.start("line");
    assert!(!b.locks.keep, "a new command keeps nothing");
}

#[test]
fn a_deflection_turns_from_the_last_edge() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    let before = b.log.len();
    assert!(!b.run(|s, cx| s.lock_toward(Toward::Deflection(100.0), cx)));
    assert_eq!(
        b.said(before),
        [(
            Level::Warn,
            "Sapma için önce bir kenar çizin: sapma önceki kenarın doğrultusundan ölçülür."
        )]
    );
    b.click(10.0, 0.0);
    // A CBS project turns clockwise: 100 grads right of east is south.
    assert!(b.run(|s, cx| s.lock_toward(Toward::Deflection(100.0), cx)));
    b.click(13.0, -20.0);
    b.confirm();
    assert!(near(points(&b)[2], [10.0, -20.0]), "{:?}", points(&b));
}

#[test]
fn esc_lets_the_locks_go_first() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert!(b.run(|s, cx| s.lock_length(12.0, cx)));
    assert!(b.run(|s, cx| s.cancel(cx)), "the command stays");
    assert!(!b.locks.any());
    assert_eq!(b.last_text(), Some("Kilitler kaldırıldı."));
    assert_eq!(b.points(), 1);
}

#[test]
fn no_lock_before_the_first_point() {
    let mut b = Bench::new("polyline");
    let before = b.log.len();
    assert!(!b.run(|s, cx| s.lock_length(12.0, cx)));
    assert!(b.type_text("<100"), "lock text is understood, and refused");
    assert_eq!(
        b.said(before),
        [
            (
                Level::Warn,
                "Kilit için önce bir nokta verin: uzunluk ve doğrultu son noktadan ölçülür."
            ),
            (
                Level::Warn,
                "Kilit için önce bir nokta verin: uzunluk ve doğrultu son noktadan ölçülür."
            )
        ]
    );
    assert!(!b.locks.any());
}

#[test]
fn a_new_command_starts_with_nothing_locked() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.run(|s, cx| s.keep_locks(true, cx));
    assert!(b.run(|s, cx| s.lock_length(12.0, cx)));
    b.start("line");
    assert!(!b.locks.any() && !b.locks.keep);
}

/// A locked direction's unit vector, within 1e-12.
fn along(toward: Option<Toward>, u: [f64; 2]) -> bool {
    match toward {
        Some(Toward::Parallel(v) | Toward::Perpendicular(v)) => {
            (v.x - u[0]).abs() < 1e-12 && (v.y - u[1]).abs() < 1e-12
        }
        _ => false,
    }
}

#[test]
fn nesneye_paralel_waits_for_an_edge_and_takes_its_direction() {
    let mut b = Bench::new("polyline");
    b.add_line("cizim", [-20.0, -10.0], [-4.0, 2.0]);
    b.click(-10.0, -15.0);
    assert!(b.run(|s, cx| s.pick_lock_edge(LockPick::Parallel, cx)));
    let prompt = b.run(|s, _| s.prompt());
    assert_eq!(prompt.step, "paralel kilidi için kenarı ya da yayı seçin");
    // Nothing under the press: said, and the wait goes on; nothing reaches the tool.
    let before = b.log.len();
    b.click(-25.0, -2.0);
    assert_eq!(b.said(before), [(Level::Warn, NO_LOCK_EDGE)]);
    assert_eq!(b.points(), 1);
    assert_eq!(b.run(|s, _| s.lock_pick()), Some(LockPick::Parallel));
    // The slanted line's direction, (16, 12) / 20; its edge kept for the drawing.
    b.click(-12.0, -4.0);
    assert!(along(b.locks.toward, [0.8, 0.6]), "{:?}", b.locks.toward);
    assert!(b.locks.edge.is_some());
    assert_eq!(b.points(), 1, "the press picked, it placed nothing");
    assert_eq!(b.run(|s, _| s.lock_pick()), None);
    // Both ways: the next point on the parallel through the first one.
    b.click(5.0, -3.0);
    assert!(!b.locks.any() && b.locks.edge.is_none(), "one-shot");
    b.confirm();
    let pts = points(&b);
    assert!(near(pts[1], [5.36, -3.48]), "{pts:?}");
}

#[test]
fn nesneye_dik_on_an_arc_is_square_to_its_tangent() {
    let mut b = Bench::new("polyline");
    // The upper half of a circle round (10, −10), radius 6.
    b.add_arc("cizim", [10.0, -10.0], 6.0, 0.0, std::f64::consts::PI);
    b.click(-10.0, -15.0);
    assert!(b.run(|s, cx| s.pick_lock_edge(LockPick::Perpendicular, cx)));
    // Its top: the tangent runs west (counter-clockwise), square to it is north–south.
    b.click(10.0, -4.0);
    assert!(along(b.locks.toward, [-1.0, 0.0]), "{:?}", b.locks.toward);
    let format = b.run(|_, cx| cx.format());
    assert_eq!(b.locks.words(&format), ["Dik"]);
    b.click(-10.0, -30.0);
    b.confirm();
    assert!(near(points(&b)[1], [-10.0, -30.0]), "{:?}", points(&b));
}

#[test]
fn esc_leaves_an_awaited_edge_before_the_locks() {
    let mut b = Bench::new("polyline");
    b.add_line("cizim", [-20.0, -10.0], [-4.0, 2.0]);
    b.click(-10.0, -15.0);
    assert!(b.run(|s, cx| s.lock_length(5.0, cx)));
    assert!(b.run(|s, cx| s.pick_lock_edge(LockPick::Parallel, cx)));
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.run(|s, _| s.lock_pick()), None);
    assert_eq!(b.locks.length, Some(5.0), "the locks stay for the next Esc");
    assert_eq!(b.run(|s, _| s.prompt()).step, "sonraki noktayı belirtin");
}

#[test]
fn no_edge_pick_before_the_first_point() {
    let mut b = Bench::new("polyline");
    let before = b.log.len();
    assert!(!b.run(|s, cx| s.pick_lock_edge(LockPick::Parallel, cx)));
    assert_eq!(
        b.said(before),
        [(
            Level::Warn,
            "Kilit için önce bir nokta verin: uzunluk ve doğrultu son noktadan ölçülür."
        )]
    );
}

/// The newest area's corners, relative to (E, N).
fn corners(b: &Bench) -> Vec<[f64; 2]> {
    let Entity::Polygon(area) = b.newest() else {
        panic!("an area, not {:?}", b.newest());
    };
    area.pts.iter().map(|p| rel(*p)).collect()
}

#[test]
fn dik_aci_squares_each_edge_after_the_first_and_dik_kapat_closes_square() {
    let mut b = Bench::new("polygon");
    b.draft.right_angle = true;
    b.click(-10.0, -10.0);
    // The first edge is free.
    b.click(6.0, -2.0);
    // The second square to it: the click goes onto the square line, (4, 2).
    b.click(10.0, 5.0);
    assert_eq!(b.points(), 3);
    assert!(b.options().contains(&"D"), "{:?}", b.options());
    // Dik kapat: the fourth corner where the two square lines meet; the area is written.
    assert!(b.type_text("D"));
    assert_eq!(b.points(), 0);
    let pts = corners(&b);
    let want = [[-10.0, -10.0], [6.0, -2.0], [4.0, 2.0], [-12.0, -6.0]];
    assert_eq!(pts.len(), 4, "{pts:?}");
    assert!(pts.iter().zip(want).all(|(p, w)| near(*p, w)), "{pts:?}");
}

#[test]
fn dik_kapat_says_why_when_the_first_and_last_edges_run_parallel() {
    let mut b = Bench::new("polygon");
    b.draft.right_angle = true;
    b.click(-15.0, 10.0);
    b.click(-5.0, 10.0);
    b.click(-3.0, 14.0);
    b.click(-20.0, 12.0);
    assert_eq!(b.points(), 4);
    let before = b.log.len();
    assert!(b.type_text("D"));
    assert_eq!(
        b.said(before),
        [(Level::Warn, kentos_interaction::NO_SQUARE_CLOSE)]
    );
    assert_eq!(b.points(), 4, "the corners stay");
    b.confirm();
    let pts = corners(&b);
    let want = [[-15.0, 10.0], [-5.0, 10.0], [-5.0, 14.0], [-20.0, 14.0]];
    assert!(pts.iter().zip(want).all(|(p, w)| near(*p, w)), "{pts:?}");
}

#[test]
fn a_direction_locked_by_hand_comes_before_dik_aci() {
    let mut b = Bench::new("polyline");
    b.draft.right_angle = true;
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    // Square to the first edge would be north–south; a semt of 50 grads wins.
    assert!(b.type_text("<50"));
    b.click(20.0, 9.0);
    b.confirm();
    let p = points(&b)[2];
    // Along 50 grads (north-east), the cursor projected: (10, 0) + t·(√½, √½).
    assert!((p[0] - 10.0 - (p[1])).abs() < 1e-9 && p[1] > 0.0, "{p:?}");
}

#[test]
fn polyline_has_no_dik_kapat() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.click(5.0, 0.0);
    b.click(5.0, 5.0);
    assert!(!b.options().contains(&"D"), "{:?}", b.options());
}

#[test]
fn a_reference_point_measures_typed_points_and_goes_with_the_corner() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert!(b.run(|s, cx| s.ask_reference(cx)));
    assert_eq!(b.run(|s, _| s.prompt()).step, "referans noktasını belirtin");
    // The press is the reference, not a corner.
    b.click(10.0, 5.0);
    assert_eq!(b.points(), 1);
    assert_eq!(
        b.last_text(),
        Some("Referans noktası: Y 487010.000  X 4420005.000")
    );
    // Relative input from the reference; the corner placed, the reference goes.
    assert!(b.type_text("@0,4"));
    assert_eq!(b.points(), 2);
    assert_eq!(b.run(|s, _| s.reference()), None, "one-shot");
    assert!(b.type_text("@5,0"));
    // Yapım kipi: presses and typed points renew the reference.
    assert!(b.run(|s, cx| s.set_construction(true, cx)));
    b.click(20.0, 0.0);
    assert!(b.type_text("@0,-3"));
    assert_eq!(b.points(), 3);
    // Off: the next point is a corner, measured from the reference.
    assert!(b.run(|s, cx| s.set_construction(false, cx)));
    assert!(b.type_text("@5,0"));
    b.confirm();
    let pts = points(&b);
    let want = [[0.0, 0.0], [10.0, 9.0], [15.0, 9.0], [25.0, -3.0]];
    assert_eq!(pts.len(), 4, "{pts:?}");
    assert!(pts.iter().zip(want).all(|(p, w)| near(*p, w)), "{pts:?}");
}

#[test]
fn esc_steps_back_through_the_reference() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert!(b.run(|s, cx| s.ask_reference(cx)));
    assert!(
        b.run(|s, cx| s.cancel(cx)),
        "the wait goes, the command stays"
    );
    assert_eq!(b.run(|s, _| s.prompt()).step, "sonraki noktayı belirtin");
    assert!(b.run(|s, cx| s.set_construction(true, cx)));
    b.click(4.0, 4.0);
    assert!(b.run(|s, cx| s.cancel(cx)), "Yapım kipi goes");
    assert!(!b.run(|s, _| s.construction()));
    assert!(
        b.run(|s, _| s.reference()).is_some(),
        "its point stays for the next corner"
    );
    assert!(
        b.run(|s, cx| s.cancel(cx)),
        "then the reference with the locks"
    );
    assert_eq!(b.run(|s, _| s.reference()), None);
    assert_eq!(b.points(), 1);
}

#[test]
fn no_reference_without_a_command_that_takes_points() {
    let mut b = Bench::new("polyline");
    b.run(|s, _| s.exit());
    let before = b.log.len();
    assert!(!b.run(|s, cx| s.ask_reference(cx)));
    assert!(!b.run(|s, cx| s.set_construction(true, cx)));
    assert_eq!(
        b.said(before),
        [
            (Level::Warn, kentos_interaction::NO_REFERENCE_TOOL),
            (Level::Warn, kentos_interaction::NO_REFERENCE_TOOL)
        ]
    );
}
