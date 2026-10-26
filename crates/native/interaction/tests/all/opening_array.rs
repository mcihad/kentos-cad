use crate::common::{Bench, E};
use kentos_commands::Entity;
use kentos_domain::Slot;

fn set(b: &mut Bench, k: &str, v: &str) -> bool {
    b.run(|s, cx| s.configure(k, v, cx))
}
fn source() -> Bench {
    let mut b = Bench::new("wall");
    b.click(0., 0.);
    b.click(20., 0.);
    b.confirm();
    b.confirm();
    b.start("door");
    assert!(set(&mut b, "E", "1 m"));
    b.click(1., 0.2);
    b.confirm();
    let slot = Slot(b.newest().base().id);
    b.selection.set([slot]);
    b.start("openingArray");
    b
}
#[test]
fn settings_preview_is_read_only_and_one_confirmation_writes_the_whole_numbered_array() {
    let mut b = source();
    let before = b.doc.to_snapshot_v2();
    let source = b.newest().clone();
    let settings = b.session.settings(&Default::default());
    assert_eq!(settings[0].value, format!("#{} · Kapı", source.base().id));
    for (k, v) in [
        ("M", "Aralıkla çoğalt"),
        ("R", "Net aralık"),
        ("Y", "Sona doğru"),
        ("A", "3"),
        ("U", "1 m"),
        ("N", "on"),
        ("Ö", "K"),
        ("İ", "2"),
        ("D", "2"),
    ] {
        assert!(set(&mut b, k, v));
    }
    let preview = b.session.preview(&Default::default()).unwrap();
    assert!(preview.strokes.len() > 3);
    assert!(
        preview
            .tag
            .unwrap()
            .lines
            .join(" ")
            .contains("3 yeni açıklık")
    );
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.confirm();
    assert_eq!(b.doc.len(), 5);
    assert_eq!(b.doc.get(Slot(source.base().id)), Some(&source));
    let added: Vec<_> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Opening(o) if o.base.id != source.base().id => Some(o),
            _ => None,
        })
        .collect();
    for (i, o) in added.iter().enumerate() {
        assert!((o.opening.at - (3. + i as f64 * 2.)).abs() < 1e-10);
        assert_eq!(
            o.opening.number.as_deref(),
            Some(format!("K{}", 2 + i * 2).as_str())
        );
        assert!((o.opening.place[0].x - (E + 2.5 + i as f64 * 2.)).abs() < 1e-9);
    }
    let after = b.doc.to_snapshot_v2();
    assert_eq!(
        b.doc.journal().last().unwrap().command.as_deref(),
        Some("cad.openings.edit")
    );
    assert!(b.doc.undo().is_some());
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
    assert!(b.doc.redo().is_some());
    assert_eq!(b.doc.to_snapshot_v2().entities, after.entities);
}
#[test]
fn rejected_settings_and_overlapping_fit_never_write_a_partial_array() {
    let mut b = source();
    let before = b.doc.to_snapshot_v2();
    for (k, v) in [
        ("A", "0"),
        ("A", "10001"),
        ("D", "0"),
        ("U", "-2 m"),
        ("U", "NaN"),
    ] {
        assert!(!set(&mut b, k, v));
    }
    assert!(set(&mut b, "M", "fit"));
    assert!(set(&mut b, "B", "0 m"));
    assert!(set(&mut b, "S", "0 m"));
    // Source spans 0.5..1.5 m; first fitted copy spans 0..1 m.
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    let p = b.session.preview(&Default::default()).unwrap();
    assert_eq!(p.tag_tone, kentos_interaction::Tone::Danger);
    assert!(set(&mut b, "B", "2 m"));
    assert!(set(&mut b, "S", "1 m"));
    assert!(set(&mut b, "A", "1"));
    b.confirm();
    assert_eq!(b.doc.len(), 3);
    let Entity::Opening(o) = b.newest() else {
        panic!()
    };
    assert_eq!(o.opening.at, 10.5);
}
#[test]
fn stale_preview_requires_explicit_confirmation_again_after_refresh() {
    let mut b = source();
    let slot = b
        .doc
        .entities()
        .find(|e| matches!(e, Entity::Wall(_)))
        .map(|e| Slot(e.base().id))
        .unwrap();
    let mut wall = b.doc.get(slot).unwrap().clone();
    wall.base_mut().label = Some("revision test".into());
    // Use the existing document update path to invalidate the preview's revision.
    b.doc.update_many(vec![(slot, wall)], "external edit");
    let changed = b.doc.to_snapshot_v2();
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2(), changed);
    b.confirm();
    assert_eq!(b.doc.len(), 5);
}
#[test]
fn picking_source_and_cancelling_or_clearing_preview_does_not_edit_the_drawing() {
    let mut b = source();
    let before = b.doc.to_snapshot_v2();
    assert!(b.undo_step());
    assert_eq!(b.points(), 0);
    b.click(1., 0.2);
    assert_eq!(b.points(), 1);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}
