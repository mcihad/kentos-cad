//! Merdiven from its height (docs/adr/0704): one click and Enter draw a stair whose risers come from the height and
//! the highest riser, its last flight ending at its last riser however far the second point is; one undo takes it
//! back. Tıklanan yerde keeps the clicked point; the preview is the command's plan and writes nothing.

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};

fn bench() -> Bench {
    let mut b = Bench::new("stair");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.start("stair");
    b
}

fn stair(b: &Bench) -> kentos_commands::StairFields {
    match b.newest() {
        Entity::Stair(s) => s.stair.clone(),
        other => panic!("a stair, not {other:?}"),
    }
}

#[test]
fn a_click_and_enter_draw_the_height_s_risers_ending_at_the_last() {
    let mut b = bench();
    b.click(0., 0.);
    b.move_to(0., 9.);
    let before = b.doc.to_snapshot_v2();
    let preview = b.session.preview(&Default::default()).expect("the plan drawn");
    assert!(preview.strokes.len() > 18, "the risers drawn: {}", preview.strokes.len());
    assert_eq!(b.doc.to_snapshot_v2(), before, "the preview writes nothing");
    b.click(0., 9.);
    b.confirm();
    let s = stair(&b);
    assert_eq!(s.risers, 18);
    assert!((s.riser_height - 3.0 / 18.0).abs() < 1e-12);
    assert_eq!(s.cut, Some(6), "1.10 m over 0.167 m risers: the sixth");
    let end = s.pts.last().copied().expect("its end");
    assert!((end.x - E).abs() < 1e-9 && (end.y - (N + 17.0 * 0.28)).abs() < 1e-9, "{end:?}");
    assert_eq!(b.doc.undo().as_deref(), Some("Merdiven"));
    assert!(!b.doc.entities().any(|e| matches!(e, Entity::Stair(_))));
}

#[test]
fn a_given_count_and_the_clicked_end_are_kept() {
    let mut b = bench();
    assert!(b.run(|session, cx| session.configure("S", "20", cx)));
    assert!(b.run(|session, cx| session.configure("U", "Tıklanan yerde biter", cx)));
    b.click(0., 0.);
    b.click(6., 0.);
    b.confirm();
    let s = stair(&b);
    assert_eq!(s.risers, 20);
    assert!((s.riser_height - 0.15).abs() < 1e-12);
    let end = s.pts.last().copied().expect("its end");
    assert!((end.x - (E + 6.0)).abs() < 1e-9, "the clicked end kept: {end:?}");
}

#[test]
fn risers_ending_before_the_last_flight_say_why_and_write_nothing() {
    let mut b = bench();
    b.click(0., 0.);
    b.click(8., 0.);
    b.move_to(8., 3.);
    let tag = b.session.preview(&Default::default()).and_then(|p| p.tag).expect("the tag");
    assert!(tag.lines.iter().any(|l| l.contains("son kola varmadan")), "{:?}", tag.lines);
    b.click(8., 3.);
    b.confirm();
    assert!(!b.doc.entities().any(|e| matches!(e, Entity::Stair(_))));
}
