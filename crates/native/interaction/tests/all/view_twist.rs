//! Görünümü döndür through the session (docs/adr/0285 §2): the tool asks the host to turn the view so the
//! direction of two points lies horizontal, or by a typed turn in the project's unit and way, UCS'e göre or back
//! north up; it writes nothing and runs over a command without ending it (docs/adr/0283).

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4};

use crate::common;

use common::Bench;
use kentos_interaction::{Session, ViewChange};

fn twist(b: &Bench) -> f64 {
    match b.views.last() {
        Some(ViewChange::Twist(t)) => *t,
        other => panic!("no twist asked: {other:?}"),
    }
}

#[test]
fn two_points_lay_their_direction_horizontal() {
    let mut b = Bench::new("viewTwist");
    assert_eq!(
        b.session.prompt().text(),
        "Görünümü döndür: yatay olacak doğrultunun ilk noktasını verin ya da açı yazın [UCS'e göre (U) / Kuzey yukarı (K) / Esc: bitir]"
    );
    b.click(0.0, 0.0);
    assert_eq!(b.points(), 1);
    b.move_to(10.0, 10.0);
    let preview = b.session.preview(&kentos_interaction::Format::default()).expect("a preview");
    assert_eq!(preview.guides.len(), 1, "the direction to come");
    b.click(10.0, 10.0);
    assert!((twist(&b) - FRAC_PI_4).abs() < 1e-12);
    assert!(!b.session.is_running(), "it ends");
    assert!(b.doc.entities().next().is_none(), "nothing is written");
    assert!(b.last_text().is_some_and(|t| t.starts_with("Görünüm döndürüldü")));
}

#[test]
fn a_typed_turn_is_the_projects_unit_and_way_and_u_and_k_need_no_point() {
    // The traces' drawing: CBS, grad: 100 is a quarter turn clockwise.
    let mut b = Bench::new("viewTwist");
    assert!(b.type_text("100"));
    assert!((twist(&b) + FRAC_PI_2).abs() < 1e-12);
    let mut b = Bench::new("viewTwist");
    assert!(b.type_text("K"));
    assert_eq!(twist(&b), 0.0);
    assert_eq!(b.last_text(), Some("Görünüm: kuzey yukarı"));
    // UCS'e göre: the UCS's x axis horizontal; the world's is north up.
    let mut b = Bench::new("viewTwist");
    let mut s = b.doc.settings().clone();
    s.ucs = Some(kentos_contracts::Ucs::new(
        kentos_contracts::Vec2 { x: 0.0, y: 0.0 },
        0.75,
    ));
    b.doc.set_settings(s);
    assert!(b.type_text("U"));
    assert_eq!(twist(&b), 0.75);
}

#[test]
fn the_same_point_twice_gives_no_direction_and_esc_steps_back() {
    let mut b = Bench::new("viewTwist");
    b.click(2.0, 2.0);
    b.click(2.0, 2.0);
    assert!(b.views.is_empty());
    assert!(b.session.is_running());
    assert!(b.run(|s, cx| s.cancel(cx)), "Esc drops the first point");
    assert_eq!(b.points(), 0);
}

#[test]
fn it_runs_over_a_polyline_and_gives_it_back() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.click(4.0, 0.0);
    assert!(Session::is_transparent("viewTwist"));
    assert!(b.run(|s, cx| s.start_over("viewTwist", cx)));
    assert!(b.type_text("K"));
    assert!(!b.session.nested());
    assert_eq!(b.session.tool_id(), "polyline");
    assert_eq!(b.points(), 2);
}
