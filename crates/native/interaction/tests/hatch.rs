//! Tarama through the session, over the native document: the web's words,
//! the regions by closed object and by line work, islands, the boundary
//! layer, the patterns, one undo step “Tarama” (the web's `HatchTool`,
//! docs/adr/0062). The drawing is the trace's (fixtures/interaction/v1/hatch.kcad):
//! a 20 × 16 m parcel with an 8 × 6 m building inside, and line work
//! closing two 12 × 16 m faces, a 4 × 4 m closed polyline of another layer
//! inside the left one. Expected values are worked out by hand.

mod common;

use std::collections::BTreeMap;

use common::{Bench, E, N, rel};
use kentos_contracts::{
    Entity, EntityBase, HatchPatternType, PathEntity, Vec2 as Wire,
};
use kentos_interaction::Level;

const HATCH: &str = include_str!("../../../../fixtures/interaction/v1/hatch.kcad");

const PROMPT: &str = "Tarama: taranacak yerin içine tıklayın [Desen (D): Çizgili 45° / Sınır (B): kapalı nesne / Adalar (A): taranmaz]";

fn bench() -> Bench {
    let mut b = Bench::on(HATCH);
    b.start("hatch");
    b
}

/// The newest object's ring's box, relative, its hole count, pattern and area.
fn newest(b: &Bench) -> ([f64; 4], usize, (HatchPatternType, f64, f64), f64) {
    let Entity::Hatch(h) = b.newest() else {
        panic!("a hatch: {:?}", b.newest());
    };
    let xs = h.ring.iter().map(|p| rel(*p));
    let bx = xs.fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, [x, y]| {
        [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)]
    });
    let area = kentos_interaction::measures(b.newest()).0.expect("an area");
    (
        bx,
        h.holes.as_ref().map_or(0, Vec::len),
        (h.pattern.kind, h.pattern.angle, h.pattern.spacing),
        area,
    )
}

fn near(have: f64, want: f64) -> bool {
    (have - want).abs() < 1e-6
}

#[test]
fn a_click_in_a_parcel_leaves_its_building_out() {
    let mut b = bench();
    assert_eq!(b.session.prompt().text(), PROMPT);
    let count = b.doc.entities().count();
    b.click(-12.0, 8.0);
    assert_eq!(b.doc.entities().count(), count + 1);
    let (bx, holes, pattern, area) = newest(&b);
    assert_eq!(bx, [-28.0, -6.0, -8.0, 10.0]);
    assert_eq!(holes, 1);
    // Lines at 45°, 3 paper mm apart: 3 m at 1:1000.
    assert_eq!(pattern, (HatchPatternType::Lines, 45.0, 3.0));
    assert!(near(area, 320.0 - 48.0), "{area}");
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(
        b.last_text(),
        Some("Çizgili tarama eklendi: 272.00 m², 1 ada taranmadı")
    );
    // On the active layer; the tool waits for the next one.
    assert_eq!(b.newest().base().layer_id, "cizim");
    assert!(b.session.is_running());
    assert_eq!(b.session.prompt().text(), PROMPT);
    // Ctrl+Z is the drawing's: one step, “Tarama”.
    assert!(!b.undo_step());
    assert_eq!(b.doc.undo().as_deref(), Some("Tarama"));
    assert_eq!(b.doc.entities().count(), count);
}

#[test]
fn inside_the_building_only_the_building() {
    let mut b = bench();
    b.click(-20.0, 1.0);
    let (bx, holes, _, area) = newest(&b);
    assert_eq!((bx, holes), ([-24.0, -2.0, -16.0, 4.0], 0));
    assert!(near(area, 48.0), "{area}");
    assert_eq!(b.last_text(), Some("Çizgili tarama eklendi: 48.00 m²"));
}

#[test]
fn islands_off_fill_the_whole_parcel() {
    let mut b = bench();
    assert!(b.type_text("a"));
    assert!(b.session.prompt().text().ends_with("/ Adalar (A): taranır]"));
    b.click(-12.0, 8.0);
    let (_, holes, _, area) = newest(&b);
    assert_eq!(holes, 0);
    assert!(near(area, 320.0), "{area}");
    assert_eq!(b.last_text(), Some("Çizgili tarama eklendi: 320.00 m²"));
}

#[test]
fn no_closed_object_around_the_click_is_said() {
    let mut b = bench();
    let count = b.doc.entities().count();
    // Inside the line work: closed by lines, not by an object.
    b.click(8.0, -3.0);
    assert_eq!(b.doc.entities().count(), count);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("Tıklanan noktayı çevreleyen kapalı bir alan, daire ya da kapalı eğri yok. Çizgilerle çevrili yerler için “Sınır: çizgiler” seçin.")
    );
}

#[test]
fn by_lines_the_face_around_the_click_with_its_island() {
    let mut b = bench();
    assert!(b.type_text("B"));
    assert_eq!(
        b.session.prompt().text(),
        "Tarama: taranacak yerin içine tıklayın [Desen (D): Çizgili 45° / Sınır (B): çizgiler / Adalar (A): taranmaz / Sınır katmanı (K): tümü]"
    );
    b.click(8.0, -3.0);
    let (bx, holes, _, area) = newest(&b);
    assert_eq!((bx, holes), ([2.0, -6.0, 14.0, 10.0], 1));
    assert!(near(area, 192.0 - 16.0), "{area}");
    assert_eq!(
        b.last_text(),
        Some("Çizgili tarama eklendi: 176.00 m², 1 ada taranmadı")
    );
    // Outside every face.
    b.click(0.0, 18.0);
    assert_eq!(
        b.last_text(),
        Some("Tıklanan yer çizgilerle kapalı bir bölgenin içinde değil; görünüm dışındaki çizgiler sayılmaz.")
    );
}

#[test]
fn the_boundary_layer_is_picked_by_one_of_its_objects() {
    let mut b = bench();
    // Only with the line work.
    assert!(!b.type_text("K"));
    assert!(b.type_text("B"));
    assert!(b.type_text("K"));
    assert_eq!(
        b.session.prompt().text(),
        "Tarama: sınır olacak katmandan bir nesneye tıklayın [Tüm katmanlar (K)]"
    );
    b.click(0.0, 18.0);
    assert_eq!(
        b.last_text(),
        Some("Sınır katmanını seçmek için bir nesneye tıklayın.")
    );
    // The object under the cursor is highlighted: the left line.
    b.move_to(2.0, 1.0);
    assert_eq!(b.selection.hover().map(|s| s.0), Some(6));
    b.click(2.0, 1.0);
    assert_eq!(b.selection.hover(), None);
    assert!(b.session.prompt().text().ends_with("/ Sınır katmanı (K): Çizim]"));
    // The closed polyline is on Yapılar: no island now.
    let count = b.doc.entities().count();
    b.click(8.0, -3.0);
    assert_eq!(b.doc.entities().count(), count + 1);
    let (_, holes, _, area) = newest(&b);
    assert_eq!(holes, 0);
    assert!(near(area, 192.0), "{area}");
    // K again: every visible layer.
    assert!(b.type_text("K"));
    assert!(b.session.prompt().text().ends_with("/ Sınır katmanı (K): tümü]"));
    // Esc leaves the picking first, then the tool.
    assert!(b.type_text("K"));
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert!(b.session.prompt().text().ends_with("/ Sınır katmanı (K): tümü]"));
    assert!(!b.run(|s, cx| s.cancel(cx)));
    assert!(!b.session.is_running());
}

#[test]
fn patterns_cycle_and_a_solid_one_fills() {
    let mut b = bench();
    for name in ["Çapraz 45°", "Yatay çizgili", "Dolu"] {
        assert!(b.type_text("D"));
        assert!(
            b.session
                .prompt()
                .text()
                .contains(&format!("[Desen (D): {name} /")),
            "{}",
            b.session.prompt().text()
        );
    }
    b.click(-20.0, 1.0);
    let (_, _, pattern, _) = newest(&b);
    assert_eq!(pattern, (HatchPatternType::Solid, 0.0, 3.0));
    assert_eq!(b.last_text(), Some("Dolu tarama eklendi: 48.00 m²"));
    assert!(b.type_text("D"));
    assert_eq!(b.session.prompt().text(), PROMPT);
    // Kept for the next run; the boundary layer is the run's.
    assert!(b.type_text("D"));
    assert!(b.type_text("B"));
    b.confirm();
    assert!(!b.session.is_running());
    b.start("hatch");
    assert_eq!(
        b.session.prompt().text(),
        "Tarama: taranacak yerin içine tıklayın [Desen (D): Çapraz 45° / Sınır (B): çizgiler / Adalar (A): taranmaz / Sınır katmanı (K): tümü]"
    );
}

#[test]
fn a_locked_active_layer_writes_nothing() {
    let mut b = bench();
    assert!(b.doc.set_active_layer("kilitli"), "the locked layer");
    let count = b.doc.entities().count();
    b.click(-12.0, 8.0);
    assert_eq!(b.doc.entities().count(), count);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.")
    );
}

#[test]
fn too_dense_a_pattern_is_refused() {
    let mut b = bench();
    // A 100 km square around everything: 3 m apart, far too many lines.
    let far = |x: f64, y: f64| Wire { x: E + x, y: N + y };
    let square = Entity::Polygon(PathEntity {
        base: EntityBase {
            id: 0,
            layer_id: "cizim".to_owned(),
            color: None,
            attrs: BTreeMap::new(),
            label: None,
            symbol: None,
        },
        pts: vec![
            far(-50_000.0, -50_000.0),
            far(50_000.0, -50_000.0),
            far(50_000.0, 50_000.0),
            far(-50_000.0, 50_000.0),
        ],
        bulges: None,
        holes: None,
    });
    b.doc.add(square).expect("a slot");
    let count = b.doc.entities().count();
    b.click(0.0, 30.0);
    assert_eq!(b.doc.entities().count(), count);
    assert_eq!(
        b.last_text(),
        Some("Desen bu alan için çok sık; çizim ölçeğini büyütün ya da başka bir desen seçin.")
    );
    // Solid has no lines.
    for _ in 0..3 {
        assert!(b.type_text("D"));
    }
    b.click(0.0, 30.0);
    assert_eq!(b.doc.entities().count(), count + 1);
}
