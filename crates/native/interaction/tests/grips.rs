//! Grips through the session, while no command runs: the web's `SelectTool`
//! (`apps/web/src/tools/SelectTool.ts`, docs/adr/0068) over the selection
//! traces' drawing (fixtures/interaction/v1/objects.kcad): line 1 from
//! (−24, −12) to (−8, −12), the 12 × 10 m area 4 from (−24, 4), point 5 at
//! (20, −8), line 6 on the locked layer from (4, −16) to (16, −16). Expected
//! values are worked out by hand.

mod common;

use std::collections::BTreeMap;

use common::{Bench, E, N, rel};
use kentos_contracts::{CircleEntity, Entity, EntityBase, Vec2 as Wire};
use kentos_domain::Slot;
use kentos_interaction::{Format, GripSet, Level, Vec2};

const OBJECTS: &str = include_str!("../../../../fixtures/interaction/v1/objects.kcad");

const PROMPT: &str = "Tutamaç: yeni konumu belirtin ya da koordinat yazın (Esc: vazgeç)";

/// The drawing with `slots` selected, snapping off.
fn bench(slots: &[u32]) -> Bench {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.selection
        .set(slots.iter().map(|s| Slot(*s)).collect::<Vec<_>>());
    b
}

fn ends(b: &Bench, slot: u32) -> Vec<[f64; 2]> {
    match b.doc.get(Slot(slot)) {
        Some(Entity::Line(l)) => vec![rel(l.a), rel(l.b)],
        Some(Entity::Polygon(p)) => p.pts.iter().map(|&q| rel(q)).collect(),
        _ => panic!("a line or an area"),
    }
}

#[test]
fn a_dragged_grip_moves_the_end_in_one_step() {
    let mut b = bench(&[1]);
    b.drag([-8.0, -12.0], [-8.0, -6.0]);
    assert_eq!(ends(&b, 1), [[-24.0, -12.0], [-8.0, -6.0]]);
    assert_eq!(b.selected(), [1], "the selection stays");
    assert!(!b.session.grip_active());
    assert_eq!(b.doc.undo().as_deref(), Some("Tutamaçla düzenle"));
    assert_eq!(ends(&b, 1), [[-24.0, -12.0], [-8.0, -12.0]]);
}

#[test]
fn a_clicked_grip_is_hot_until_the_next_click_places_it() {
    let mut b = bench(&[1]);
    b.click(-24.0, -12.0);
    assert!(b.session.grip_active());
    assert_eq!(b.session.prompt().text(), PROMPT);
    assert_eq!(b.session.active_grip(), Some((Slot(1), 0)));
    // The preview: the line as it would be, dashed; the distance beside the pointer.
    b.move_to(-24.0, -4.0);
    let preview = b
        .session
        .preview(&Format::default())
        .expect("a grip's preview");
    assert_eq!(preview.tag.expect("a tag").lines, ["8.000 m"]);
    assert!(preview.strokes.iter().any(|s| s.dash == Some([4.0, 3.0])));
    b.click(-24.0, -4.0);
    assert_eq!(ends(&b, 1), [[-24.0, -4.0], [-8.0, -12.0]]);
    assert!(!b.session.grip_active());
    assert_eq!(b.session.prompt().text(), "Komut");
}

#[test]
fn a_typed_point_places_the_grip_from_where_it_was() {
    let mut b = bench(&[1]);
    b.click(-8.0, -12.0);
    assert!(b.type_text("@0,4"));
    assert_eq!(ends(&b, 1), [[-24.0, -12.0], [-8.0, -8.0]]);
    // With no grip, typed text is not the select tool's.
    assert!(!b.type_text("@0,4"));
}

#[test]
fn enter_places_the_grip_at_the_pointer_and_esc_leaves_it() {
    let mut b = bench(&[1]);
    b.click(-8.0, -12.0);
    b.move_to(0.0, 0.0);
    assert!(b.run(|s, cx| s.cancel(cx)), "Esc is the grip's");
    assert_eq!(ends(&b, 1), [[-24.0, -12.0], [-8.0, -12.0]]);
    assert_eq!(b.selected(), [1]);
    b.click(-8.0, -12.0);
    b.move_to(-8.0, -8.0);
    assert!(b.session.confirms());
    b.confirm();
    assert_eq!(ends(&b, 1), [[-24.0, -12.0], [-8.0, -8.0]]);
    // With no grip, Enter is not the select tool's: the last command repeats.
    assert!(!b.session.confirms());
}

#[test]
fn a_mid_grip_adds_a_corner_and_a_short_segment_hides_it() {
    let mut b = bench(&[4]);
    b.drag([-18.0, 4.0], [-18.0, 0.0]);
    assert_eq!(
        ends(&b, 4),
        [[-24.0, 4.0], [-18.0, 0.0], [-12.0, 4.0], [-12.0, 14.0], [-24.0, 14.0]]
    );
    b.spatial.sync(&b.doc);
    let sets = b.spatial.grips(&[Slot(4)]);
    let set = &sets[0];
    assert_eq!(set.vertices, 5);
    let first = set
        .segments
        .iter()
        .position(|s| *s == Some(0))
        .expect("the first segment's mid grip");
    // 7.2 m long at 8 px per metre: 58 px, shown.
    assert!(set.shown(first, &common::Camera));
    // A segment shorter than 28 px on the area offers no mid grip: 2 m is 16 px.
    let (a, c) = (Vec2::new(E, N), Vec2::new(E + 2.0, N));
    let short = GripSet {
        slot: Slot(9),
        points: vec![a, c, Vec2::new(E + 1.0, N), Vec2::new(E + 1.0, N)],
        segments: vec![None, None, Some(0), Some(1)],
        vertices: 2,
    };
    assert!(short.shown(0, &common::Camera), "a vertex always shows");
    assert!(!short.shown(2, &common::Camera));
}

#[test]
fn a_locked_objects_grips_are_not_taken() {
    let mut b = bench(&[6]);
    b.drag([4.0, -16.0], [7.0, -10.0]);
    assert_eq!(ends(&b, 6), [[4.0, -16.0], [16.0, -16.0]]);
    // The drag was a selection box, with nothing inside it.
    assert_eq!(b.selected(), Vec::<u32>::new());
}

#[test]
fn a_grip_that_would_break_the_shape_is_left_where_it_was() {
    let mut b = bench(&[]);
    let circle = b
        .doc
        .add(Entity::Circle(CircleEntity {
            base: EntityBase {
                id: 0,
                layer_id: "cizim".to_owned(),
                color: None,
                attrs: BTreeMap::new(),
                label: None,
                symbol: None,
            },
            c: Wire { x: E, y: N + 20.0 },
            r: 4.0,
        }))
        .expect("a slot");
    let undo_before = b.doc.can_undo();
    b.selection.set([circle]);
    // A quadrant onto the centre: no circle is left.
    b.drag([4.0, 20.0], [0.0, 20.0]);
    let Some(Entity::Circle(c)) = b.doc.get(circle) else {
        panic!("the circle");
    };
    assert_eq!(c.r, 4.0);
    assert_eq!(
        b.last_text(),
        Some("Bu konum geçersiz bir şekil oluşturuyor; tutamaç yerinde bırakıldı.")
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.can_undo(), undo_before, "no step");
}

#[test]
fn a_grip_snaps_from_where_it_was() {
    let mut b = bench(&[1]);
    b.draft.snap = true;
    b.click(-8.0, -12.0);
    // Line 2's end at (0, 4) is within the snap aperture of the pointer.
    b.move_to(0.3, 4.2);
    b.click(0.3, 4.2);
    assert_eq!(ends(&b, 1), [[-24.0, -12.0], [0.0, 4.0]]);
}
