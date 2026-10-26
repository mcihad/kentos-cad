use crate::common::Bench;
use kentos_commands::Entity;
#[test]
fn separator_factory_settings_preview_and_one_confirmation_are_semantic() {
    assert!(kentos_interaction::Session::tools().contains(&"spaceSeparator"));
    let mut b=Bench::new("spaceSeparator");b.memory.polyline_width=2.;b.memory.polyline_width_end=2.;b.start("spaceSeparator");
    let before=b.doc.to_snapshot_v2();b.click(0.,0.);b.move_to(5.,0.);
    let preview=b.session.preview(&Default::default()).unwrap();assert!(!preview.strokes.is_empty());assert!(preview.areas.is_empty());assert!(preview.path.is_empty());
    assert!(b.session.settings(&Default::default()).iter().any(|s|s.key=="L"));
    assert!(!b.run(|s,cx|s.input("E",cx)),"a virtual boundary offered physical width");
    b.click(5.,0.);assert_eq!(b.doc.to_snapshot_v2(),before);b.run(|s,cx|s.confirm(cx));
    assert_eq!(b.session.tool_id(),"select","one Enter did not finish");
    let Entity::Polyline(p)=b.newest()else{panic!()};assert!(p.space_separator.is_some());assert!(p.widths.is_none());
    b.doc.undo().unwrap();assert_eq!(b.doc.to_snapshot_v2(),before);
}
#[test]
fn arc_input_and_escape_use_the_existing_path_grammar_without_writing_early() {
    let mut b=Bench::new("spaceSeparator");let before=b.doc.to_snapshot_v2();
    b.click(0.,0.);b.click(4.,0.);assert!(b.run(|s,cx|s.input("Y",cx)));b.move_to(8.,4.);b.click(8.,4.);
    b.run(|s,cx|s.confirm(cx));let Entity::Polyline(p)=b.newest()else{panic!()};
    assert!(p.space_separator.is_some());assert!(p.bulges.as_ref().unwrap().iter().any(|b|*b!=0.));
    b.doc.undo().unwrap();assert_eq!(b.doc.to_snapshot_v2(),before);
    b.start("spaceSeparator");b.click(0.,0.);b.click(5.,0.);b.run(|s,cx|s.cancel(cx));assert_eq!(b.doc.to_snapshot_v2(),before);
}

#[test]
fn rejected_close_keeps_the_separator_draft_for_retry() {
    let mut b = Bench::new("spaceSeparator");
    b.click(0., 0.);
    b.click(4., 0.);
    b.click(4., 4.);
    assert!(b.doc.set_layer_locked("cizim", true));
    let locked = b.doc.to_snapshot_v2();
    assert!(b.run(|s, cx| s.input("K", cx)));
    assert_eq!(b.doc.to_snapshot_v2(), locked);
    assert_eq!(b.session.tool_id(), "spaceSeparator");
    assert!(b.doc.set_layer_locked("cizim", false));
    let before = b.doc.to_snapshot_v2();
    assert!(b.run(|s, cx| s.input("K", cx)));
    let Entity::Polyline(p) = b.newest() else { panic!() };
    assert!(p.closed);
    assert!(p.space_separator.is_some());
    assert_eq!(p.pts.len(), 3, "refusal discarded the draft vertices");
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2(), before);
}
