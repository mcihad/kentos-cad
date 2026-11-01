//! Unfinished radius/distance text is a readonly semantic plan, not a
//! write and not the unrelated size under the mouse. Confirm matches it.
use crate::common::{Bench, E, N};
use kentos_commands::Entity;
use kentos_domain::Slot;
use kentos_interaction::{Format, InputPreview};
fn drawing(tool: &str) -> Bench {
    let mut b = Bench::new(tool);
    b.add_line("cizim", [0., 0.], [2., 0.]);
    b.add_line("cizim", [0., 2.], [0., 0.]);
    b.click(1.8, 0.);
    b.click(0., 1.8);
    b
}
fn draft(b: &mut Bench, text: Option<&str>) -> InputPreview {
    b.run(|s, cx| s.preview_input(text, cx))
}
#[test]
fn two_distances_are_live_readonly_and_click_uses_those_exact_distances() {
    let mut b = drawing("chamfer");
    let before = b.doc.to_snapshot_v2();
    assert_eq!(draft(&mut b, Some("10cm;20cm")), InputPreview::Ready);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(b.memory.chamfer.is_none());
    let p = b.session.preview(&Format::default()).unwrap();
    let tag = p.tag.unwrap().lines.join(" ");
    assert!(tag.contains("0.100") && tag.contains("0.200"));
    let guide_ends: Vec<_> = p
        .strokes
        .iter()
        .filter(|s| s.tone == kentos_interaction::Tone::Snap)
        .map(|s| *s.pts.last().unwrap())
        .collect();
    assert!(
        guide_ends
            .iter()
            .any(|p| (p.x - E - 0.1).abs() < 1e-8 && (p.y - N).abs() < 1e-8)
    );
    assert!(
        guide_ends
            .iter()
            .any(|p| (p.x - E).abs() < 1e-8 && (p.y - N - 0.2).abs() < 1e-8)
    );
    b.move_to(0.7, 0.7);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(0.7, 0.7);
    let Some(Entity::Line(a)) = b.doc.get(Slot(1)) else {
        panic!()
    };
    let Some(Entity::Line(z)) = b.doc.get(Slot(2)) else {
        panic!()
    };
    assert!((a.a.x - E - 0.1).abs() < 1e-8);
    assert!((z.b.y - N - 0.2).abs() < 1e-8);
    assert_eq!(b.doc.len(), 3);
    assert_eq!(b.doc.undo().as_deref(), Some("Pah"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}
#[test]
fn incomplete_input_never_falls_back_to_a_remembered_size_and_cancel_clears_it() {
    let mut b = drawing("chamfer");
    b.memory.chamfer = Some((0.4, 0.4));
    let before = b.doc.to_snapshot_v2();
    assert_eq!(draft(&mut b, Some("10cm;")), InputPreview::Invalid);
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert_eq!(draft(&mut b, None), InputPreview::Off);
    assert_eq!(b.memory.chamfer, Some((0.4, 0.4)));
    assert_eq!(draft(&mut b, Some("10cm;20cm")), InputPreview::Ready);
    b.confirm();
    assert_eq!(b.doc.len(), 3);
    b.doc.undo();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}
#[test]
fn a_radius_and_a_geometrically_impossible_radius_are_not_confused() {
    let mut b = drawing("fillet");
    let before = b.doc.to_snapshot_v2();
    assert_eq!(draft(&mut b, Some("20m")), InputPreview::Invalid);
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert_eq!(draft(&mut b, Some("25cm")), InputPreview::Ready);
    b.confirm();
    let Some(Entity::Arc(a)) = b.doc.get(Slot(3)) else {
        panic!()
    };
    assert!((a.r - 0.25).abs() < 1e-12);
    assert!((a.c.x - E - 0.25).abs() < 1e-8 && (a.c.y - N - 0.25).abs() < 1e-8);
    b.doc.undo();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}
