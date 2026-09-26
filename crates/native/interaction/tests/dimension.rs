//! Ölçülendirme through the session, over the native document: the web's
//! words, what each style writes, the typed values, the picks and Ctrl+Z
//! (the web's `DimensionTool` with its fixes of 26 September, docs/adr/0061).
//! Expected values are worked out by hand, not taken from a run.

mod common;

use common::{Bench, rel};
use kentos_contracts::{DimensionEntity, DimensionStyle, Entity};
use kentos_interaction::{DimensionMode, Level};

const TOOLS: &str = include_str!("../../../../fixtures/interaction/v1/tools.kcad");

/// The newest object, a dimension.
fn newest(b: &Bench) -> &DimensionEntity {
    let Entity::Dimension(d) = b.newest() else {
        panic!("a dimension: {:?}", b.newest());
    };
    d
}

fn near(have: f64, want: f64) -> bool {
    (have - want).abs() < 1e-9
}

/// The drawing of the tool traces (docs/adr/0057), the tool running, snapping off.
fn on_tools() -> Bench {
    let mut b = Bench::on(TOOLS);
    b.draft.snap = false;
    b.start("dimension");
    b
}

#[test]
fn aligned_takes_two_points_then_where_the_line_goes() {
    let mut b = Bench::new("dimension");
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: hizalı ölçünün ilk noktasını belirtin [Doğrusal (D) / Açı (A) / Yarıçap (R) / Çap (Ç)]"
    );
    b.click(0.0, 0.0);
    // A point is in: the style no longer changes.
    assert_eq!(b.session.prompt().text(), "Ölçü: ikinci ölçü noktasını belirtin");
    assert!(!b.type_text("D"));
    b.click(10.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: ölçü çizgisinin yerini gösterin ya da mesafe yazın"
    );
    // Three metres left of the measured direction.
    b.click(5.0, 3.0);
    let d = newest(&b);
    assert_eq!((rel(d.a), rel(d.b)), ([0.0, 0.0], [10.0, 0.0]));
    // 2.5 paper mm at 1:1000; aligned is written without a style, as the web does.
    assert!(near(d.offset, 3.0) && near(d.height, 2.5), "{d:?}");
    assert_eq!((d.style, d.angle, d.c, d.text.as_deref()), (None, None, None, None));
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.last_text(), Some("Hizalı ölçü eklendi: 10.000"));
    // The next dimension starts; what was written is the drawing's to undo.
    assert!(b.session.prompt().text().starts_with("Ölçü: hizalı ölçünün ilk noktasını"));
    assert!(!b.undo_step());
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
    assert!(!b.doc.entities().any(|e| matches!(e, Entity::Dimension(_))));
}

#[test]
fn a_typed_distance_is_signed_left_of_the_measured_direction() {
    let mut b = Bench::new("dimension");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    b.move_to(5.0, 3.0);
    // Typed, the side is the sign's, not the cursor's.
    assert!(b.type_text("-4"));
    assert!(near(newest(&b).offset, -4.0));
    assert_eq!(b.last_text(), Some("Hizalı ölçü eklendi: 10.000"));
}

#[test]
fn linear_follows_where_the_line_goes_unless_locked() {
    let mut b = Bench::new("dimension");
    assert!(b.type_text("D"));
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: doğrusal ölçünün (ΔY / ΔX) ilk noktasını belirtin [Hizalı (H) / Açı (A) / Yarıçap (R) / Çap (Ç)]"
    );
    b.click(0.0, 0.0);
    b.click(10.0, 5.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: ölçü çizgisinin yerini gösterin ya da mesafe yazın [Yatay ΔY (Y) / Düşey ΔX (X) / Yön (O): imleçten]"
    );
    // Above the measured points: horizontal, ΔY; the line through the cursor.
    b.click(5.0, 9.0);
    let d = newest(&b);
    assert_eq!((d.style, d.angle), (Some(DimensionStyle::Linear), Some(0.0)));
    assert!(near(d.offset, 9.0), "{d:?}");
    assert_eq!(b.last_text(), Some("Doğrusal ölçü eklendi: 10.000"));
    // Locked to ΔX: the typed distance, whatever the cursor says.
    b.click(0.0, 0.0);
    b.click(10.0, 5.0);
    assert!(b.type_text("X"));
    assert!(b.session.prompt().text().ends_with("[Yatay ΔY (Y) / Düşey ΔX (X) / Yön (O): düşey]"));
    b.move_to(5.0, 9.0);
    assert!(b.type_text("3"));
    let d = newest(&b);
    assert_eq!(d.angle, Some(90.0));
    assert!(near(d.offset, 3.0), "{d:?}");
    assert_eq!(b.last_text(), Some("Doğrusal ölçü eklendi: 5.000"));
    // The lock and the style stay for the next run; Yön gives the cursor back.
    b.confirm();
    assert!(!b.session.is_running());
    assert_eq!(b.memory.dimension_mode, DimensionMode::Linear);
    assert_eq!(b.memory.dimension_lock, Some(90.0));
    b.start("dimension");
    b.click(0.0, 0.0);
    b.click(10.0, 5.0);
    assert!(b.type_text("o"));
    assert_eq!(b.memory.dimension_lock, None);
}

#[test]
fn nothing_measured_is_said_and_the_tool_waits() {
    let mut b = Bench::new("dimension");
    assert!(b.type_text("D"));
    b.click(0.0, 0.0);
    b.click(0.0, 10.0);
    // Horizontal (ΔY) between two points one above the other measures nothing.
    assert!(b.type_text("Y"));
    let count = b.doc.entities().count();
    b.click(5.0, 5.0);
    assert_eq!(b.doc.entities().count(), count);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("Bu yerde ölçü oluşmuyor; ölçülen noktalar çakışıyor ya da yay yarıçapı sıfır.")
    );
    assert_eq!(b.points(), 2);
    // Beside them, with the direction from the cursor: ΔX, 10 m.
    assert!(b.type_text("O"));
    b.click(5.0, 5.0);
    let d = newest(&b);
    assert_eq!(d.angle, Some(90.0));
    assert!(near(d.offset, -5.0), "{d:?}");
    assert_eq!(b.last_text(), Some("Doğrusal ölçü eklendi: 10.000"));
}

#[test]
fn an_angle_between_two_edges_goes_where_the_arc_is_placed() {
    let mut b = on_tools();
    assert!(b.type_text("A"));
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: açı ölçüsü için birinci kenara tıklayın [Köşeden (K) / Hizalı (H) / Doğrusal (D) / Yarıçap (R) / Çap (Ç)]"
    );
    // Nothing there, then a circle: no straight edge.
    for [e, n] in [[0.0, -30.0], [20.0, -8.0]] {
        b.click(e, n);
        assert_eq!(
            b.last_text(),
            Some("Açının kenarı olarak düz bir çizgiye tıklayın; köşe noktasından ölçmek için “Köşeden” seçin.")
        );
    }
    b.click(-10.0, 10.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: ikinci kenara tıklayın");
    // Line 2 runs beside line 1.
    b.click(-16.0, -12.0);
    assert_eq!(b.last_text(), Some("Kenarlar paralel; aralarında açı yok."));
    // The locked layer's polyline is measured too: its side up x = 24.
    b.move_to(24.0, 13.0);
    assert_eq!(b.selection.hover().map(|s| s.0), Some(4));
    b.click(24.0, 14.0);
    assert_eq!(b.selection.hover(), None);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: yayın yerini gösterin ya da yarıçap yazın"
    );
    // Up and left of where the lines cross, (24, 10): the quarter between
    // the side (up) and line 1 (left), each arm as far as it was clicked.
    b.click(20.0, 12.0);
    let d = newest(&b);
    assert_eq!(d.style, Some(DimensionStyle::Angular));
    assert_eq!(d.c.map(rel), Some([24.0, 10.0]));
    assert_eq!((rel(d.a), rel(d.b)), ([24.0, 14.0], [-10.0, 10.0]));
    assert!(near(d.offset, 20f64.sqrt()), "{d:?}");
    // A right angle, in the drawing's grads.
    assert_eq!(b.last_text(), Some("Açı ölçüsü eklendi: 100.0000 g"));
    // On the active layer, not the edges' locked one.
    assert_eq!(d.base.layer_id, "cizim");
}

#[test]
fn an_angle_from_its_vertex_takes_a_typed_radius_without_its_sign() {
    let mut b = Bench::new("dimension");
    assert!(b.type_text("A"));
    assert!(b.type_text("K"));
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: açının köşesini gösterin [Kenarlardan (K) / Hizalı (H) / Doğrusal (D) / Yarıçap (R) / Çap (Ç)]"
    );
    b.click(0.0, 0.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: birinci kolun üzerinde bir nokta gösterin");
    b.click(10.0, 0.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: ikinci kolun üzerinde bir nokta gösterin");
    b.click(0.0, 10.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: yayın yerini gösterin ya da yarıçap yazın"
    );
    // Between the arms, counter-clockwise from the first.
    b.move_to(3.0, 3.0);
    assert!(b.type_text("-6"));
    let d = newest(&b);
    assert_eq!((rel(d.a), rel(d.b)), ([10.0, 0.0], [0.0, 10.0]));
    assert_eq!(d.c.map(rel), Some([0.0, 0.0]));
    assert!(near(d.offset, 6.0), "{d:?}");
    assert_eq!(b.last_text(), Some("Açı ölçüsü eklendi: 100.0000 g"));
    // Outside them, the other way round: the reflex angle.
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    b.click(0.0, 10.0);
    b.click(-4.0, -3.0);
    let d = newest(&b);
    assert_eq!((rel(d.a), rel(d.b)), ([0.0, 10.0], [10.0, 0.0]));
    assert!(near(d.offset, 5.0), "{d:?}");
    assert_eq!(b.last_text(), Some("Açı ölçüsü eklendi: 300.0000 g"));
}

#[test]
fn radius_and_diameter_pick_a_circle_and_take_no_number() {
    let mut b = on_tools();
    assert!(b.type_text("R"));
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: yarıçapı ölçülecek daireye ya da yaya tıklayın [Hizalı (H) / Doğrusal (D) / Açı (A) / Çap (Ç)]"
    );
    b.click(-10.0, 10.0);
    assert_eq!(
        b.last_text(),
        Some("Bir daireye, yaya ya da çoklu çizginin yay parçasına tıklayın.")
    );
    b.click(20.0, -8.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: ölçünün doğrultusunu gösterin; daireden dışarı çekince yazı dışarı alınır"
    );
    let count = b.doc.entities().count();
    b.move_to(16.0, -1.0);
    assert!(!b.type_text("5"));
    assert_eq!(b.doc.entities().count(), count);
    // Up from the centre, three metres out: the leader's end.
    b.click(16.0, -1.0);
    let d = newest(&b);
    assert_eq!(d.style, Some(DimensionStyle::Radius));
    assert_eq!((rel(d.a), rel(d.b)), ([16.0, -8.0], [16.0, -4.0]));
    assert!(near(d.offset, 3.0), "{d:?}");
    assert_eq!(b.last_text(), Some("Yarıçap ölçüsü eklendi: R 4.000"));
    // C is Ç on a keyboard without it.
    assert!(b.type_text("c"));
    assert_eq!(b.memory.dimension_mode, DimensionMode::Diameter);
    assert!(b.session.prompt().text().starts_with("Ölçü: çapı ölçülecek daireye ya da yaya tıklayın"));
    b.click(20.0, -8.0);
    b.click(22.0, -8.0);
    let d = newest(&b);
    assert_eq!(d.style, Some(DimensionStyle::Diameter));
    assert_eq!(rel(d.b), [20.0, -8.0]);
    assert_eq!(b.last_text(), Some("Çap ölçüsü eklendi: Ø 8.000"));
}

#[test]
fn ctrl_z_drops_the_newest_pick_first_and_enter_starts_over() {
    let mut b = on_tools();
    assert!(b.type_text("A"));
    b.click(-10.0, 10.0);
    b.click(24.0, 14.0);
    let count = b.doc.entities().count();
    // The second edge, then the first: the drawing does not change.
    assert!(b.undo_step());
    assert_eq!(b.session.prompt().text(), "Ölçü: ikinci kenara tıklayın");
    assert!(b.undo_step());
    assert!(b.session.prompt().text().starts_with("Ölçü: açı ölçüsü için birinci kenara"));
    assert!(!b.undo_step());
    assert_eq!(b.doc.entities().count(), count);
    // Points: Ctrl+Z starts the dimension over.
    assert!(b.type_text("H"));
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert!(b.undo_step());
    assert_eq!(b.points(), 0);
    // Enter with a pick starts over; with nothing, it leaves.
    assert!(b.type_text("R"));
    b.click(20.0, -8.0);
    b.confirm();
    assert!(b.session.is_running());
    assert!(b.session.prompt().text().starts_with("Ölçü: yarıçapı ölçülecek"));
    b.confirm();
    assert!(!b.session.is_running());
}

#[test]
fn a_locked_active_layer_refuses_and_the_next_dimension_starts() {
    let mut b = on_tools();
    assert!(b.doc.set_active_layer("kilitli"), "the locked layer");
    let count = b.doc.entities().count();
    b.click(0.0, -20.0);
    b.click(10.0, -20.0);
    b.click(5.0, -17.0);
    assert_eq!(b.doc.entities().count(), count);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.")
    );
    assert_eq!(b.points(), 0);
}

#[test]
fn a_typed_point_does_not_pick_an_edge() {
    let mut b = on_tools();
    assert!(b.type_text("A"));
    assert!(!b.type_text("@5,5"));
    assert_eq!(b.points(), 0);
    assert!(b.session.prompt().text().starts_with("Ölçü: açı ölçüsü için birinci kenara"));
}
