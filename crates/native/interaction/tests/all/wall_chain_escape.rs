//! Esc on the wall chain (docs/adr/3001): the segments already drawn are kept as Enter keeps them and the tool stays
//! for the next chain, as Revit's first Esc ends a chain and AutoCAD keeps what was drawn; the next Esc leaves. Before
//! this, Esc dropped every segment without a word. The expected walls are the points the hand gave, never a tool run.

use crate::common::{Bench, rel};
use kentos_commands::{DrawingUnit, Entity, Workspace};
use kentos_interaction::Level;

fn walls() -> Bench {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.srid = 0;
    settings.workspace = Some(Workspace::Cad);
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b
}

fn wall_points(b: &Bench) -> Vec<[f64; 2]> {
    let Entity::Wall(w) = b.newest() else {
        panic!("a semantic wall")
    };
    w.wall.pts.iter().copied().map(rel).collect()
}

#[test]
fn escape_keeps_the_walls_drawn_and_stays_for_the_next_chain() {
    let mut b = walls();
    b.click(0.0, 0.0);
    b.click(6.0, 0.0);
    b.click(6.0, 4.0);
    b.move_to(0.0, 4.0);
    assert_eq!(b.doc.len(), 0, "nothing written while drawing");
    let before = b.log.len();
    assert!(b.run(|s, cx| s.cancel(cx)), "the first Esc keeps the tool");
    assert_eq!(b.doc.len(), 1);
    assert_eq!(wall_points(&b), [[0.0, 0.0], [6.0, 0.0], [6.0, 4.0]], "the two segments drawn, not the rubber band");
    assert_eq!(b.session.tool_id(), "wall");
    assert_eq!(b.points(), 0, "the next click starts a new chain");
    let said = b.said(before);
    assert!(
        said.iter().any(|(level, text)| *level == Level::Info && *text == kentos_interaction::path::esc_kept(2)),
        "Esc says what it kept: {said:?}"
    );
    assert!(!b.run(|s, cx| s.cancel(cx)), "the next Esc leaves");
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.len(), 1);
    assert!(b.doc.undo().is_some());
    assert_eq!(b.doc.len(), 0, "one undo takes the chain back");
}

#[test]
fn escape_before_a_segment_has_nothing_to_keep() {
    let mut b = walls();
    b.click(0.0, 0.0);
    b.move_to(3.0, 0.0);
    assert!(!b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.doc.len(), 0);
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn escape_leaves_a_question_before_it_ends_the_chain() {
    let mut b = walls();
    b.click(0.0, 0.0);
    b.click(4.0, 0.0);
    assert!(b.type_text("S"), "S asks the wall style's name");
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.doc.len(), 0, "the question is left, the chain is still drawn");
    assert_eq!(b.points(), 2);
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.doc.len(), 1);
    assert_eq!(wall_points(&b), [[0.0, 0.0], [4.0, 0.0]]);
    assert_eq!(b.session.tool_id(), "wall");
}

#[test]
fn enter_still_writes_and_stays_as_before() {
    let mut b = walls();
    b.click(0.0, 0.0);
    b.click(5.0, 0.0);
    b.confirm();
    assert_eq!(b.doc.len(), 1);
    assert_eq!(wall_points(&b), [[0.0, 0.0], [5.0, 0.0]]);
    assert_eq!(b.session.tool_id(), "wall");
    assert!(!b.run(|s, cx| s.cancel(cx)), "an empty chain: Esc leaves at once");
    assert_eq!(b.session.tool_id(), "select");
}
