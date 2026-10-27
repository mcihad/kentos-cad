//! Mutfak dizisi (docs/adr/0706): the start, the end and the side clicked write a kitchen run in one undo step — its
//! cabinets, sink, cooktop, countertop and filler — with KentOS's library blocks it brings; the preview is the
//! command's plan and writes nothing. Evye and Ocak start Otomatik; a run too short is said, never written.

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};

fn bench() -> Bench {
    let mut b = Bench::new("casework");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.start("casework");
    b
}

/// The inserts written, by their blocks' names, in the drawing's order.
fn inserted(b: &Bench) -> Vec<String> {
    b.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Insert(i) => b.doc.block(i.block).map(|d| d.name.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn the_start_the_end_and_the_side_write_the_run_in_one_step() {
    let mut b = bench();
    b.click(0., 0.);
    b.click(3.6, 0.);
    b.move_to(1.8, -1.0);
    let before = b.doc.to_snapshot_v2();
    let preview = b.session.preview(&Default::default()).expect("the plan drawn");
    assert!(preview.strokes.len() > 12, "the cabinets drawn: {}", preview.strokes.len());
    let tag = preview.tag.expect("the run in words").lines.join(" | ");
    assert!(tag.contains("5 modül"), "{tag}");
    assert!(tag.contains("evye 2. modül") && tag.contains("ocak 4. modül"), "{tag}");
    assert_eq!(b.doc.to_snapshot_v2(), before, "the preview writes nothing");
    b.click(1.8, -1.0);
    assert_eq!(
        inserted(&b),
        ["Alt dolap", "Alt dolap", "Alt dolap", "Alt dolap", "Alt dolap", "Evye", "Ocak"]
    );
    // The countertop on the pointer's side: below the line walked east.
    let top = b
        .doc
        .entities()
        .find_map(|e| match e {
            Entity::Polygon(p) => Some(p.pts.clone()),
            _ => None,
        })
        .expect("the countertop");
    assert!(top.iter().all(|p| p.y <= N + 1e-9 && p.y >= N - 0.6 - 1e-9), "{top:?}");
    assert!(top.iter().any(|p| (p.x - (E + 3.6)).abs() < 1e-9));
    // 0.90 + 4 × 0.60 = 3.30: a 0.30 m filler at the end.
    assert!(b.doc.entities().any(|e| matches!(e, Entity::Polyline(p) if p.closed)));
    assert!(b.doc.block_named("Alt dolap").is_some());
    assert_eq!(b.doc.undo().as_deref(), Some("Mutfak dizisi"));
    assert_eq!(b.doc.entities().count(), 0, "one undo takes the whole run back");
    assert!(b.doc.block_named("Alt dolap").is_none(), "and the blocks it brought");
}

#[test]
fn a_typed_length_enter_and_the_values_of_the_settings() {
    let mut b = bench();
    assert!(b.run(|s, cx| s.configure("M", "45 cm", cx)));
    assert!(b.run(|s, cx| s.configure("U", "yes", cx)));
    assert!(b.run(|s, cx| s.configure("E", "none", cx)));
    assert!(b.run(|s, cx| s.configure("O", "1", cx)));
    assert!(b.run(|s, cx| s.configure("Y", "left", cx)));
    assert!(!b.run(|s, cx| s.configure("M", "50 cm", cx)), "not one of the cabinet's widths");
    b.click(0., 0.);
    b.move_to(2.0, 0.);
    assert!(b.type_text("1.8"));
    b.move_to(1.0, -1.0);
    b.confirm();
    // Four 45 cm modules, the cooktop in the first, a wall cabinet over each; on the left of the line, as Yan says.
    assert_eq!(
        inserted(&b),
        [
            "Alt dolap", "Alt dolap", "Alt dolap", "Alt dolap", "Ocak", "Üst dolap", "Üst dolap", "Üst dolap",
            "Üst dolap"
        ]
    );
    let top = b
        .doc
        .entities()
        .find_map(|e| match e {
            Entity::Polygon(p) => Some(p.pts.clone()),
            _ => None,
        })
        .expect("the countertop");
    assert!(top.iter().all(|p| p.y >= N - 1e-9), "on the line's left: {top:?}");
    assert!(top.iter().any(|p| (p.x - (E + 1.8)).abs() < 1e-9), "the typed length: {top:?}");
    assert_eq!(b.memory.casework_module, 0.45, "the module stays for the next run");
}

#[test]
fn a_run_shorter_than_a_module_is_said_and_never_written() {
    let mut b = bench();
    b.click(0., 0.);
    b.click(0.2, 0.);
    b.move_to(0.1, -1.0);
    let preview = b.session.preview(&Default::default()).expect("the tag");
    assert!(preview.tag.expect("why").lines.join(" ").contains("modüle"));
    let before = b.doc.to_snapshot_v2();
    b.click(0.1, -1.0);
    assert_eq!(b.doc.to_snapshot_v2(), before, "nothing written");
    assert!(b.undo_step(), "the end taken back");
    assert_eq!(b.points(), 1);
}
