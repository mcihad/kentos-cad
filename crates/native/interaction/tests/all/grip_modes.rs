//! Grip editing's modes and several hot grips (docs/adr/0280) over the selection traces' drawing
//! (fixtures/interaction/v1/objects.kcad): line 1 from (−24, −12) to (−8, −12), the 12 × 10 m area 4 from
//! (−24, 4), point 5 at (20, −8). Expected values are worked out by hand.

use crate::common;

use common::{Bench, rel};
use kentos_commands::Entity;
use kentos_domain::Slot;

const OBJECTS: &str = include_str!("../../../../../fixtures/interaction/v1/objects.kcad");

fn bench(slots: &[u32]) -> Bench {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.selection
        .set(slots.iter().map(|s| Slot(*s)).collect::<Vec<_>>());
    b
}

fn pts(b: &Bench, slot: u32) -> Vec<[f64; 2]> {
    match b.doc.get(Slot(slot)) {
        Some(Entity::Line(l)) => vec![rel(l.a), rel(l.b)],
        Some(Entity::Polygon(p)) => p.pts.iter().map(|&q| rel(q)).collect(),
        Some(Entity::Point(p)) => vec![rel(p.p)],
        other => panic!("a line, an area or a point: {other:?}"),
    }
}

/// The pointer moves there, then clicks: as the desktop gives a click.
fn place(b: &mut Bench, de: f64, dn: f64) {
    b.move_to(de, dn);
    b.click(de, dn);
}

fn shift_click(b: &mut Bench, de: f64, dn: f64) {
    b.shift = true;
    place(b, de, dn);
    b.shift = false;
}

/// Shift makes two of the area's corners hot; a click on one starts the edit; the next click moves both by the
/// same distance, in one step.
#[test]
fn two_hot_corners_stretch_together() {
    let mut b = bench(&[4]);
    shift_click(&mut b, -12.0, 4.0);
    shift_click(&mut b, -12.0, 14.0);
    assert_eq!(
        b.last_text(),
        Some("2 tutamaç sıcak; düzenlemek için birine tıklayın (Shift ile ekleyip çıkarın).")
    );
    assert_eq!(b.session.hot_grips().len(), 2);
    assert!(!b.session.grip_active(), "Shift only heats");
    place(&mut b, -12.0, 4.0);
    assert!(b.session.grip_active());
    place(&mut b, -10.0, 4.0);
    assert_eq!(
        pts(&b, 4),
        [[-24.0, 4.0], [-10.0, 4.0], [-10.0, 14.0], [-24.0, 14.0]]
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Tutamaçla düzenle"));
    assert_eq!(
        pts(&b, 4),
        [[-24.0, 4.0], [-12.0, 4.0], [-12.0, 14.0], [-24.0, 14.0]]
    );
}

/// Shift on a hot grip cools it; a click elsewhere cools them all.
#[test]
fn shift_again_cools_a_grip_and_a_click_away_all() {
    let mut b = bench(&[4]);
    shift_click(&mut b, -12.0, 4.0);
    shift_click(&mut b, -24.0, 4.0);
    shift_click(&mut b, -12.0, 4.0);
    assert_eq!(b.session.hot_grips(), [(Slot(4), 0)]);
    place(&mut b, 40.0, 40.0);
    assert!(b.session.hot_grips().is_empty());
}

/// An edge's middle among the hot grips moves its edge: its two vertices.
#[test]
fn an_edge_middle_among_hot_grips_moves_its_edge() {
    let mut b = bench(&[4]);
    // The top edge's middle and the bottom-left corner.
    shift_click(&mut b, -18.0, 14.0);
    shift_click(&mut b, -24.0, 4.0);
    place(&mut b, -18.0, 14.0);
    place(&mut b, -18.0, 16.0);
    assert_eq!(
        pts(&b, 4),
        [[-24.0, 6.0], [-12.0, 4.0], [-12.0, 16.0], [-24.0, 16.0]]
    );
}

/// Taşı (T): the grip is the base point; the selection moves to the click, in one step “Taşı”.
#[test]
fn move_mode_moves_the_selection_from_the_grip() {
    let mut b = bench(&[1, 5]);
    place(&mut b, -8.0, -12.0);
    assert!(b.type_text("T"));
    assert_eq!(b.last_text(), Some("Tutamaç: Taşı."));
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Tutamaç · Taşı: hedef noktayı")
    );
    place(&mut b, -6.0, -10.0);
    assert_eq!(pts(&b, 1), [[-22.0, -10.0], [-6.0, -10.0]]);
    assert_eq!(pts(&b, 5), [[22.0, -6.0]]);
    assert_eq!(b.last_text(), Some("2 nesne taşındı."));
    assert!(!b.session.grip_active());
    assert_eq!(b.doc.undo().as_deref(), Some("Taşı"));
}

/// Taban noktası (B): another base point than the grip.
#[test]
fn a_base_point_moves_from_where_it_was_given() {
    let mut b = bench(&[1]);
    place(&mut b, -8.0, -12.0);
    b.type_text("T");
    b.type_text("B");
    place(&mut b, 0.0, 0.0);
    assert!(b.session.grip_active(), "the base is not the placement");
    place(&mut b, 1.0, 2.0);
    assert_eq!(pts(&b, 1), [[-23.0, -10.0], [-7.0, -10.0]]);
}

/// Döndür (D) with Referans (R) typed: the selection turns about the grip to the typed angle from the reference,
/// both in the project's way (docs/adr/0291, 0294): this drawing is a CBS project, so semts in grads.
#[test]
fn rotate_mode_takes_a_reference_and_a_typed_angle() {
    let mut b = bench(&[1]);
    place(&mut b, -24.0, -12.0);
    b.type_text("D");
    b.type_text("R");
    assert!(b.type_text("100"));
    assert_eq!(b.last_text(), Some("Referans açı 100.0000g."));
    // To semt 200g (south) from 100g (east): a quarter turn clockwise about (−24, −12).
    assert!(b.type_text("200"));
    let got = pts(&b, 1);
    assert_eq!(got[0], [-24.0, -12.0]);
    assert!(
        (got[1][0] + 24.0).abs() < 1e-9 && (got[1][1] + 28.0).abs() < 1e-9,
        "{got:?}"
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Döndür"));
    // Without a reference the typed number is a turn: 100g clockwise, the same quarter turn.
    place(&mut b, -24.0, -12.0);
    b.type_text("D");
    assert!(b.type_text("100"));
    let got = pts(&b, 1);
    assert!(
        (got[1][0] + 24.0).abs() < 1e-9 && (got[1][1] + 28.0).abs() < 1e-9,
        "{got:?}"
    );
}

/// Ölçekle (L) with a typed factor; with a reference the typed number is the new length.
#[test]
fn scale_mode_takes_a_factor_and_a_reference_length() {
    let mut b = bench(&[1]);
    place(&mut b, -24.0, -12.0);
    b.type_text("L");
    assert!(b.type_text("2"));
    assert_eq!(pts(&b, 1), [[-24.0, -12.0], [8.0, -12.0]]);
    place(&mut b, -24.0, -12.0);
    b.type_text("L");
    b.type_text("R");
    // A reference of two points: the line's own length, then 16 m as the new length.
    place(&mut b, -24.0, -12.0);
    place(&mut b, 8.0, -12.0);
    assert!(b.type_text("16"));
    assert_eq!(pts(&b, 1), [[-24.0, -12.0], [-8.0, -12.0]]);
}

/// Kopya (K): each placement adds copies and the grip stays hot; Esc ends.
#[test]
fn copy_mode_places_copies_until_esc() {
    let mut b = bench(&[1]);
    let before = b.doc.len();
    place(&mut b, -8.0, -12.0);
    b.type_text("T");
    b.type_text("K");
    assert_eq!(b.last_text(), Some("Kopya açık."));
    place(&mut b, -8.0, -8.0);
    place(&mut b, -8.0, -4.0);
    assert!(b.session.grip_active(), "the grip stays hot");
    assert_eq!(b.doc.len(), before + 2);
    assert_eq!(
        pts(&b, 1),
        [[-24.0, -12.0], [-8.0, -12.0]],
        "the original stays"
    );
    b.run(|s, cx| s.cancel(cx));
    assert!(!b.session.grip_active());
    assert_eq!(b.doc.len(), before + 2);
}

/// Esnet with Kopya: a new object with its grip moved; the original stays.
#[test]
fn stretch_with_copy_adds_the_stretched_object() {
    let mut b = bench(&[1]);
    let before = b.doc.len();
    place(&mut b, -8.0, -12.0);
    b.type_text("K");
    place(&mut b, -8.0, -6.0);
    assert_eq!(b.doc.len(), before + 1);
    assert_eq!(pts(&b, 1), [[-24.0, -12.0], [-8.0, -12.0]]);
    assert_eq!(b.last_text(), Some("1 nesne kopyalandı (Esnet)."));
}

/// Aynala (A): the selection mirrors in place in the line from the grip.
#[test]
fn mirror_mode_mirrors_in_the_line_from_the_grip() {
    let mut b = bench(&[1]);
    place(&mut b, -8.0, -12.0);
    b.type_text("A");
    place(&mut b, -8.0, 0.0);
    assert_eq!(pts(&b, 1), [[8.0, -12.0], [-8.0, -12.0]]);
    assert_eq!(b.doc.undo().as_deref(), Some("Aynala"));
}
