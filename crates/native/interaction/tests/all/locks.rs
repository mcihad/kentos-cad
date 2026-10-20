//! The digitizing locks through the session (docs/adr/0166 §1–§2, §6), on
//! the polyline tool over the empty drawing (a project not asked its type:
//! CBS, grads): a locked length and a typed direction hold the next point,
//! one-shot locks go once it is placed and kept ones follow, Sapma turns
//! from the last edge, Esc lets the locks go before anything else.

use crate::common;

use common::{Bench, rel};
use kentos_commands::Entity;
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

/// The bench's drawing turned to a UCS at (E, N), its x axis 90° from east (docs/adr/0261).
fn turned(b: &mut Bench) {
    let mut settings = b.doc.settings().clone();
    settings.ucs = Some(kentos_commands::Ucs {
        origin: kentos_commands::Vec2 {
            x: common::E,
            y: common::N,
        },
        angle: std::f64::consts::FRAC_PI_2,
    });
    b.doc.set_settings(settings);
}

#[test]
fn a_locked_east_holds_the_point_on_its_line_and_the_cursor_gives_the_north() {
    // docs/adr/0267 §1: one coordinate locked, the other from the cursor.
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert!(b.run(|s, cx| s.lock_coordinate(false, common::E + 10.0, cx)));
    assert!(b.locks.coordinates());
    // A CBS project: the east is Y.
    assert_eq!(b.last_text(), Some("Kilit: Y 487010.000 m."));
    b.click(3.0, 7.0);
    assert!(!b.locks.any(), "a one-shot lock goes with the point");
    b.confirm();
    assert!(near(points(&b)[1], [10.0, 7.0]), "{:?}", points(&b));
}

#[test]
fn both_coordinates_hold_the_point_wherever_the_cursor_is() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert!(b.run(|s, cx| s.lock_coordinate(false, common::E + 4.0, cx)));
    assert!(b.run(|s, cx| s.lock_coordinate(true, common::N - 2.5, cx)));
    b.click(50.0, 50.0);
    b.confirm();
    assert!(near(points(&b)[1], [4.0, -2.5]), "{:?}", points(&b));
}

#[test]
fn a_coordinate_and_a_length_meet_nearest_the_cursor() {
    // From (0, 0), 5 m, the east 3: (3, 4) or (3, −4); the cursor below picks the second.
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert!(b.run(|s, cx| s.lock_length(5.0, cx)));
    assert!(b.run(|s, cx| s.lock_coordinate(false, common::E + 3.0, cx)));
    assert_eq!(
        b.locks.length,
        Some(5.0),
        "a length stays with a coordinate"
    );
    b.click(1.0, -9.0);
    b.confirm();
    assert!(near(points(&b)[1], [3.0, -4.0]), "{:?}", points(&b));
}

#[test]
fn a_length_short_of_the_coordinates_line_holds_no_point() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert!(b.run(|s, cx| s.lock_length(2.0, cx)));
    assert!(b.run(|s, cx| s.lock_coordinate(false, common::E + 3.0, cx)));
    let at = kentos_interaction::Vec2::new(common::E, common::N);
    let cursor = kentos_interaction::Vec2::new(common::E + 5.0, common::N + 5.0);
    let none = b.locks.point(at, cursor, Default::default());
    assert_eq!(
        none, None,
        "2 m from the reference never reaches the east 3"
    );
}

#[test]
fn a_direction_and_a_coordinate_are_never_held_together() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    assert!(b.run(|s, cx| s.lock_coordinate(true, common::N + 6.0, cx)));
    assert!(b.type_text("<100"));
    assert!(!b.locks.coordinates(), "the angle lets the coordinate go");
    assert_eq!(b.locks.toward, Some(Toward::Angle(100.0)));
    assert!(b.run(|s, cx| s.lock_coordinate(false, common::E + 1.0, cx)));
    assert_eq!(b.locks.toward, None, "the coordinate lets the angle go");
}

#[test]
fn in_a_ucs_the_coordinates_are_the_ucs_ones_and_say_so() {
    // docs/adr/0267 §1–§2: the UCS's x axis is north, so its X = 6 is the world's north 6.
    let mut b = Bench::new("polyline");
    turned(&mut b);
    b.click(0.0, 0.0);
    assert!(b.run(|s, cx| s.lock_coordinate(false, 6.0, cx)));
    assert_eq!(b.last_text(), Some("Kilit: Y 6.000 m (UCS)."));
    b.click(-4.0, 1.0);
    b.confirm();
    assert!(near(points(&b)[1], [-4.0, 6.0]), "{:?}", points(&b));
}

#[test]
fn the_locked_coordinates_lines_run_along_the_frames_other_axis() {
    let mut b = Bench::new("polyline");
    turned(&mut b);
    b.click(0.0, 0.0);
    assert!(b.run(|s, cx| s.lock_coordinate(false, 6.0, cx)));
    assert!(b.run(|s, cx| s.lock_coordinate(true, 2.0, cx)));
    let from = kentos_interaction::Vec2::new(common::E, common::N);
    let lines = b.locks.coordinate_lines(from);
    // The UCS's x axis is north: X = 6 is the world's north 6, running east–west; Y = 2 is the world's west 2.
    assert_eq!(lines.len(), 2);
    let (p, u) = lines[0];
    assert!(
        near([p.x - common::E, p.y - common::N], [0.0, 6.0]) && near([u.x, u.y], [-1.0, 0.0]),
        "{lines:?}"
    );
    let (p, u) = lines[1];
    assert!(
        near([p.x - common::E, p.y - common::N], [-2.0, 0.0]) && near([u.x, u.y], [0.0, 1.0]),
        "{lines:?}"
    );
}
