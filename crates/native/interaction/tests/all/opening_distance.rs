//! Açıklığın geçici ölçüleri (docs/adr/0701): a selected door shows its distances along its wall's face; a click on a
//! number opens its text field, the value typed moves the door with its reference fixed, in one undo step; Esc writes
//! nothing.

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};
use kentos_interaction::ViewChange;

/// A 10 m wall along the east, 20 cm, a 1 m door centred at 5 m; the door selected, the pointer above the wall.
fn door() -> (Bench, kentos_domain::Slot) {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.start("wall");
    b.click(0., 0.);
    b.click(10., 0.);
    b.confirm();
    b.confirm();
    b.start("door");
    assert!(b.run(|session, cx| session.configure("E", "1 m", cx)));
    b.click(5., 0.2);
    b.confirm();
    let door = kentos_domain::Slot(b.newest().base().id);
    assert!(matches!(b.doc.get(door), Some(Entity::Opening(_))));
    b.selection.set([door]);
    b.move_to(5., 1.);
    (b, door)
}

fn at(b: &Bench, door: kentos_domain::Slot) -> f64 {
    match b.doc.get(door) {
        Some(Entity::Opening(o)) => o.opening.at,
        _ => panic!("the door"),
    }
}

#[test]
fn a_selected_door_shows_both_distances_and_writes_nothing() {
    let (mut b, _) = door();
    let before = b.doc.to_snapshot_v2();
    let numbers = b.session.temporary_numbers();
    assert_eq!(numbers.len(), 2, "one before and one after the door");
    let preview = b.session.preview(&Default::default()).expect("the dimensions");
    assert_eq!(preview.texts.len(), 2);
    assert!(preview.texts.iter().all(|t| t.text.contains("4.5")), "{:?}", preview.texts);
    // Above the wall: the left face's dimensions stand above it.
    assert!(numbers.iter().all(|p| p.y > N + 0.1));
    b.move_to(5., -1.);
    assert!(b.session.temporary_numbers().iter().all(|p| p.y < N - 0.1), "the right face's below");
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn a_number_typed_moves_the_door_from_its_reference_and_one_undo_takes_it_back() {
    let (mut b, door) = door();
    let before = b.doc.to_snapshot_v2();
    let start = b.session.temporary_numbers()[0];
    b.views.clear();
    b.click(start.x - E, start.y - N);
    assert!(b.session.is_running(), "the number's text field");
    assert!(
        b.views
            .iter()
            .any(|v| matches!(v, ViewChange::Text(f) if f.initial.as_deref().is_some_and(|t| t.contains("4.5")))),
        "{:?}",
        b.views
    );
    assert_eq!(b.doc.to_snapshot_v2(), before, "the field writes nothing");
    b.run(|session, cx| session.text_typed(Some("1 m"), cx));
    assert!(!b.session.is_running());
    assert!((at(&b, door) - 1.5).abs() < 1e-9, "the start jamb 1 m from the wall's end");
    assert_eq!(b.selection.ids(), [door], "the door stays selected");
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
}

#[test]
fn escape_and_a_bad_value_write_nothing() {
    let (mut b, door) = door();
    let before = b.doc.to_snapshot_v2();
    let end = b.session.temporary_numbers()[1];
    b.click(end.x - E, end.y - N);
    b.run(|session, cx| session.text_typed(None, cx));
    assert!(!b.session.is_running());
    assert_eq!(b.doc.to_snapshot_v2(), before);
    let end = b.session.temporary_numbers()[1];
    b.click(end.x - E, end.y - N);
    b.views.clear();
    b.run(|session, cx| session.text_typed(Some("kapı"), cx));
    assert!(b.session.is_running(), "the field opens again for the value");
    assert!(b.views.iter().any(|v| matches!(v, ViewChange::Text(_))));
    b.run(|session, cx| session.text_typed(Some("20 m"), cx));
    assert!(!b.session.is_running());
    assert_eq!(b.doc.to_snapshot_v2(), before, "off its wall: refused, said");
    assert!((at(&b, door) - 5.).abs() < 1e-9);
}
