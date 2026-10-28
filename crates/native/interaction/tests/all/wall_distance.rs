//! Duvarın geçici ölçüleri (docs/adr/0710): a selected wall shows the clear distance to the nearest parallel wall on
//! each side; a click on a number opens its text field, the value typed moves the wall square to itself (the walls at
//! its corners following, the wall across fixed), in one undo step; Esc writes nothing. A many-edged wall is measured
//! on the edge the pointer is nearest.

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};
use kentos_domain::Slot;
use kentos_interaction::ViewChange;

fn bench() -> Bench {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b
}

/// A wall drawn with the Wall tool through `pts`, its slot.
fn wall(b: &mut Bench, pts: &[(f64, f64)]) -> Slot {
    b.start("wall");
    for &(x, y) in pts {
        b.click(x, y);
    }
    b.confirm();
    b.confirm();
    Slot(b.newest().base().id)
}

fn pts(b: &Bench, s: Slot) -> Vec<(f64, f64)> {
    match b.doc.get(s) {
        Some(Entity::Wall(w)) => w.wall.pts.iter().map(|p| (p.x - E, p.y - N)).collect(),
        _ => panic!("a wall"),
    }
}

/// A room of four walls round 0..5 × 0..4 (east, then the others), the east wall selected, the pointer inside it.
fn room() -> (Bench, [Slot; 4]) {
    let mut b = bench();
    let south = wall(&mut b, &[(0., 0.), (5., 0.)]);
    let east = wall(&mut b, &[(5., 0.), (5., 4.)]);
    let north = wall(&mut b, &[(5., 4.), (0., 4.)]);
    let west = wall(&mut b, &[(0., 4.), (0., 0.)]);
    b.selection.set([east]);
    b.move_to(4.5, 2.);
    (b, [south, east, north, west])
}

#[test]
fn a_selected_wall_shows_the_gap_to_the_wall_across_and_writes_nothing() {
    let (b, _) = room();
    let before = b.doc.to_snapshot_v2();
    let numbers = b.session.temporary_numbers();
    assert_eq!(numbers.len(), 1, "inside only: nothing east of the room");
    assert!(b.session.temporary_length().is_some(), "and the edge's length (docs/adr/0728)");
    let preview = b
        .session
        .preview(&Default::default())
        .expect("the dimension");
    let width = 5.0 - kentos_commands::DEFAULT_WALL_WIDTH;
    assert!(
        preview
            .texts
            .iter()
            .any(|t| t.text.contains(&format!("{width:.3}"))),
        "{width:.3} m clear: {:?}",
        preview.texts
    );
    assert!(
        (numbers[0].x - (E + 2.5)).abs() < 1e-6,
        "across the room's middle"
    );
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn a_number_typed_moves_the_wall_and_its_corners_follow_in_one_step() {
    let (mut b, [south, east, north, _]) = room();
    let before = b.doc.to_snapshot_v2();
    let at = b.session.temporary_numbers()[0];
    b.views.clear();
    b.click(at.x - E, at.y - N);
    assert!(b.session.is_running(), "the number's text field");
    assert!(
        b.views.iter().any(|v| matches!(v, ViewChange::Text(_))),
        "{:?}",
        b.views
    );
    assert_eq!(b.doc.to_snapshot_v2(), before, "the field writes nothing");
    let width = kentos_commands::DEFAULT_WALL_WIDTH;
    // 30 cm wider: the east wall goes 30 cm east.
    let typed = format!("{}", 5.0 - width + 0.3);
    b.run(|session, cx| session.text_typed(Some(&typed), cx));
    assert!(!b.session.is_running());
    let close = |a: (f64, f64), x: f64, y: f64| (a.0 - x).abs() < 1e-9 && (a.1 - y).abs() < 1e-9;
    let e = pts(&b, east);
    assert!(close(e[0], 5.3, 0.) && close(e[1], 5.3, 4.), "{e:?}");
    assert!(
        close(pts(&b, south)[1], 5.3, 0.),
        "the south wall's end follows"
    );
    assert!(
        close(pts(&b, north)[0], 5.3, 4.),
        "the north wall's start follows"
    );
    assert_eq!(b.selection.ids(), [east], "the wall stays selected");
    assert_eq!(b.doc.undo().as_deref(), Some("Duvarı düzenle"));
    assert_eq!(
        b.doc.to_snapshot_v2().entities,
        before.entities,
        "one undo takes all back"
    );
}

#[test]
fn escape_and_a_bad_value_write_nothing() {
    let (mut b, _) = room();
    let before = b.doc.to_snapshot_v2();
    let at = b.session.temporary_numbers()[0];
    b.click(at.x - E, at.y - N);
    b.run(|session, cx| session.text_typed(None, cx));
    assert!(!b.session.is_running());
    let at = b.session.temporary_numbers()[0];
    b.click(at.x - E, at.y - N);
    b.run(|session, cx| session.text_typed(Some("geniş"), cx));
    assert!(
        b.session.is_running(),
        "a value that does not read keeps the field"
    );
    b.run(|session, cx| session.text_typed(None, cx));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn a_many_edged_wall_is_measured_on_the_edge_the_pointer_is_by() {
    let mut b = bench();
    let u = wall(&mut b, &[(0., 0.), (6., 0.), (6., 4.)]);
    let across = wall(&mut b, &[(0., 3.), (4., 3.)]);
    let _ = across;
    b.selection.set([u]);
    b.move_to(3., 0.5);
    let near_first = b.session.temporary_numbers();
    assert_eq!(near_first.len(), 1, "the first edge: the wall 3 m north");
    assert!((near_first[0].y - (N + 1.5)).abs() < 1e-6, "{near_first:?}");
    b.move_to(5.5, 2.);
    let second = b.session.temporary_numbers();
    assert!(
        second.is_empty() || second.iter().all(|p| (p.y - (N + 1.5)).abs() > 1e-6),
        "the second edge: {second:?}"
    );
}
