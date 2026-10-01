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
        "Ölçü: hizalı ölçünün ilk noktasını belirtin [Zemin (Z): kapalı / Doğrusal (D) / Açı (A) / Yarıçap (R) / Çap (Ç) / Koordinat (O) / Yay uzunluğu (U) / Kırıklı yarıçap (I) / Semt (T) / Eğim (E)]"
    );
    b.click(0.0, 0.0);
    // A point is in: the style no longer changes.
    assert_eq!(b.session.prompt().text(), "Ölçü: ikinci ölçü noktasını belirtin");
    assert!(!b.type_text("D"));
    b.click(10.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: ölçü çizgisinin yerini gösterin ya da mesafe yazın [Zemin (Z): kapalı]"
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
        "Ölçü: doğrusal ölçünün (ΔY / ΔX) ilk noktasını belirtin [Zemin (Z): kapalı / Hizalı (H) / Açı (A) / Yarıçap (R) / Çap (Ç) / Koordinat (O) / Yay uzunluğu (U) / Kırıklı yarıçap (I) / Semt (T) / Eğim (E)]"
    );
    b.click(0.0, 0.0);
    b.click(10.0, 5.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: ölçü çizgisinin yerini gösterin ya da mesafe yazın [Yatay ΔY (Y) / Düşey ΔX (X) / Yön (O): imleçten / Açı (A) / Zemin (Z): kapalı]"
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
    assert!(b.session.prompt().text().ends_with("[Yatay ΔY (Y) / Düşey ΔX (X) / Yön (O): düşey / Açı (A) / Zemin (Z): kapalı]"));
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
        "Ölçü: açı ölçüsü için bir kenara, yaya ya da daireye tıklayın [Köşeden (K) / Zemin (Z): kapalı / Hizalı (H) / Doğrusal (D) / Yarıçap (R) / Çap (Ç) / Koordinat (O) / Yay uzunluğu (U) / Kırıklı yarıçap (I) / Semt (T) / Eğim (E)]"
    );
    // Nothing there: what to pick is said.
    b.click(0.0, -30.0);
    assert_eq!(
        b.last_text(),
        Some("Açı için düz bir kenara, yaya ya da daireye tıklayın; köşe noktasından ölçmek için “Köşeden” seçin.")
    );
    b.click(-10.0, 10.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: ikinci kenara tıklayın");
    // The second pick is a straight edge only: not the circle.
    b.click(20.0, -8.0);
    assert_eq!(
        b.last_text(),
        Some("Açının kenarı olarak düz bir çizgiye tıklayın; köşe noktasından ölçmek için “Köşeden” seçin.")
    );
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
        "Ölçü: yayın yerini gösterin ya da yarıçap yazın [Zemin (Z): kapalı]"
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
        "Ölçü: açının köşesini gösterin [Kenarlardan (K) / Zemin (Z): kapalı / Hizalı (H) / Doğrusal (D) / Yarıçap (R) / Çap (Ç) / Koordinat (O) / Yay uzunluğu (U) / Kırıklı yarıçap (I) / Semt (T) / Eğim (E)]"
    );
    b.click(0.0, 0.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: birinci kolun üzerinde bir nokta gösterin");
    b.click(10.0, 0.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: ikinci kolun üzerinde bir nokta gösterin");
    b.click(0.0, 10.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: yayın yerini gösterin ya da yarıçap yazın [Zemin (Z): kapalı]"
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
        "Ölçü: yarıçapı ölçülecek daireye ya da yaya tıklayın [Zemin (Z): kapalı / Hizalı (H) / Doğrusal (D) / Açı (A) / Çap (Ç) / Koordinat (O) / Yay uzunluğu (U) / Kırıklı yarıçap (I) / Semt (T) / Eğim (E)]"
    );
    b.click(-10.0, 10.0);
    assert_eq!(
        b.last_text(),
        Some("Bir daireye, yaya ya da çoklu çizginin yay parçasına tıklayın.")
    );
    b.click(20.0, -8.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: ölçünün doğrultusunu gösterin; daireden dışarı çekince yazı dışarı alınır [Zemin (Z): kapalı]"
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
    assert!(b.session.prompt().text().starts_with("Ölçü: açı ölçüsü için bir kenara"));
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
    assert!(b.session.prompt().text().starts_with("Ölçü: açı ölçüsü için bir kenara"));
}

/// Koordinat (docs/adr/0147 §7): a point, then its line's end (or its
/// length toward the cursor, typed); the axis follows the cursor (further up
/// or down: its Y) unless locked.
#[test]
fn an_ordinate_takes_its_point_then_its_line_s_end() {
    let mut b = Bench::new("dimension");
    assert!(b.type_text("O"));
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: koordinat ölçüsünün noktasını belirtin [Zemin (Z): kapalı / Hizalı (H) / Doğrusal (D) / Açı (A) / Yarıçap (R) / Çap (Ç) / Yay uzunluğu (U) / Kırıklı yarıçap (I) / Semt (T) / Eğim (E)]"
    );
    b.click(10.0, 20.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: çizginin ucunu gösterin ya da uzunluğunu yazın [Y koordinatı (Y) / X koordinatı (X) / Eksen (O): imleçten / Zemin (Z): kapalı]"
    );
    // A typed length goes toward the cursor, as every point tool's: straight up 12 m.
    b.move_to(10.0, 40.0);
    assert!(b.type_text("12"));
    let d = newest(&b);
    assert_eq!((rel(d.a), rel(d.b)), ([10.0, 20.0], [10.0, 32.0]));
    assert_eq!(d.angle, Some(0.0));
    // Up and a little across: its Y, the line jogged over to the end.
    b.click(10.0, 20.0);
    b.click(16.0, 40.0);
    let d = newest(&b);
    assert_eq!((rel(d.a), rel(d.b)), ([10.0, 20.0], [16.0, 40.0]));
    assert_eq!((d.style, d.angle), (Some(DimensionStyle::Ordinate), Some(0.0)));
    assert!(near(d.offset, 0.0) && near(d.height, 2.5), "{d:?}");
    // The point's Y, its east: 487010.
    assert_eq!(b.last_text(), Some("Koordinat ölçüsü eklendi: Y=487010.000"));
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
    // Across, its X; X locks it, Eksen gives it back to the cursor.
    b.click(10.0, 20.0);
    b.click(-10.0, 22.0);
    assert_eq!(newest(&b).angle, Some(90.0));
    b.click(10.0, 20.0);
    assert!(b.type_text("Y"));
    assert!(b.session.prompt().text().ends_with("Eksen (O): Y / Zemin (Z): kapalı]"), "{}", b.session.prompt().text());
    b.click(-10.0, 22.0);
    assert_eq!(newest(&b).angle, Some(0.0), "locked to its Y");
    b.click(10.0, 20.0);
    assert!(b.type_text("O"));
    b.click(-10.0, 22.0);
    assert_eq!(newest(&b).angle, Some(90.0), "the cursor's again");
    // Its end too close to the point (half a metre across: its X, 1.25 m needed): said, nothing written.
    let before = b.doc.entities().count();
    b.click(10.0, 20.0);
    b.click(10.5, 20.3);
    assert_eq!(b.doc.entities().count(), before);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("Çizginin ucu noktaya çok yakın; imleci noktadan eksene dik yönde uzaklaştırın.")
    );
}

/// Yay uzunluğu (docs/adr/0147 §7): an arc, or a path's arc segment, not a
/// circle; with Kısmi two points on it; then where the dimension arc goes,
/// or its distance typed. Ctrl+Z takes the points back, then the arc.
#[test]
fn an_arc_length_picks_an_arc_then_where_its_dimension_arc_goes() {
    let mut b = Bench::new("dimension");
    // A quarter arc of radius 10 about (0, 0), from east to north; a circle beside it.
    b.add_arc("cizim", [0.0, 0.0], 10.0, 0.0, std::f64::consts::FRAC_PI_2);
    b.add_circle("cizim", [40.0, 0.0], 5.0);
    assert!(b.type_text("U"));
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: yay uzunluğu ölçülecek yaya tıklayın [Kısmi (K) / Zemin (Z): kapalı / Hizalı (H) / Doğrusal (D) / Açı (A) / Yarıçap (R) / Çap (Ç) / Koordinat (O) / Kırıklı yarıçap (I) / Semt (T) / Eğim (E)]"
    );
    // A circle has no ends to measure between.
    b.click(45.0, 0.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    b.click(10.0 * std::f64::consts::FRAC_1_SQRT_2, 10.0 * std::f64::consts::FRAC_1_SQRT_2);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: ölçü yayının yerini gösterin ya da uzaklık yazın [Zemin (Z): kapalı]"
    );
    // 3 m out from the arc, typed.
    b.move_to(0.0, 14.0);
    assert!(b.type_text("3"));
    let d = newest(&b);
    assert_eq!(d.style, Some(DimensionStyle::ArcLength));
    assert_eq!(d.c.map(rel), Some([0.0, 0.0]));
    assert!(near(d.offset, 3.0), "{d:?}");
    assert!(near(rel(d.a)[0], 10.0) && near(rel(d.b)[1], 10.0), "{d:?}");
    // π/2 × 10.
    assert_eq!(b.last_text(), Some("Yay uzunluğu ölçüsü eklendi: 15.708"));
    // Kısmi: the arc, two points on it, then the dimension arc's place by the cursor (2 m out).
    assert!(b.type_text("K"));
    assert!(b.session.prompt().text().contains("Bütün yay (K)"));
    b.click(10.0 * std::f64::consts::FRAC_1_SQRT_2, 10.0 * std::f64::consts::FRAC_1_SQRT_2);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: yayın üstünde ölçünün başlangıcını gösterin"
    );
    b.click(0.0, 10.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: yayın üstünde ölçünün sonunu gösterin");
    // Ctrl+Z takes the point back, then the arc.
    assert!(b.undo_step());
    assert_eq!(b.session.prompt().text(), "Ölçü: yayın üstünde ölçünün başlangıcını gösterin");
    b.click(0.0, 10.0);
    b.click(10.0, 0.0);
    b.click(12.0 * std::f64::consts::FRAC_1_SQRT_2, 12.0 * std::f64::consts::FRAC_1_SQRT_2);
    let d = newest(&b);
    assert!(near(d.offset, 2.0), "{d:?}");
    // The whole quarter again, its ends in counter-clockwise order whatever the clicks'.
    assert!(near(rel(d.a)[0], 10.0) && near(rel(d.b)[1], 10.0), "{d:?}");
}

/// Kırıklı yarıçap (docs/adr/0147 §7): a circle or an arc, the centre the
/// line starts from, the point on the arc (put on it), then the jog's place
/// or its distance typed. A point that leaves no room for a jog is refused.
#[test]
fn a_jogged_radius_takes_a_circle_a_centre_shown_a_point_and_its_jog() {
    let mut b = Bench::new("dimension");
    // A circle of radius 300 about (−290, 0): only its east side is in view.
    b.add_circle("cizim", [-290.0, 0.0], 300.0);
    assert!(b.type_text("I"));
    assert!(
        b.session.prompt().text().starts_with("Ölçü: kırıklı yarıçapı ölçülecek daireye ya da yaya tıklayın ["),
        "{}",
        b.session.prompt().text()
    );
    b.click(10.0, 0.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: çizginin başlayacağı merkezi gösterin");
    b.click(-10.0, 4.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: yaydaki noktayı gösterin");
    // Behind the arc, the centre shown leaves no room: refused, asked again.
    b.click(-12.0, 30.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.session.prompt().text(), "Ölçü: yaydaki noktayı gösterin");
    // A point near the arc's east: put on it, (10, 0).
    b.click(10.5, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: kırığın yerini gösterin ya da uzaklığını yazın [Zemin (Z): kapalı]"
    );
    assert!(b.type_text("6"));
    let d = newest(&b);
    assert_eq!(d.style, Some(DimensionStyle::Jogged));
    assert_eq!((rel(d.a), rel(d.b), d.c.map(rel)), ([-290.0, 0.0], [10.0, 0.0], Some([-10.0, 4.0])));
    assert!(near(d.offset, 6.0), "{d:?}");
    assert_eq!(b.last_text(), Some("Kırıklı yarıçap ölçüsü eklendi: R 300.000"));
    // Ctrl+Z: the points first, then the circle.
    b.click(10.0, 0.0);
    b.click(-10.0, 4.0);
    assert!(b.undo_step());
    assert_eq!(b.session.prompt().text(), "Ölçü: çizginin başlayacağı merkezi gösterin");
    assert!(b.undo_step());
    assert!(b.session.prompt().text().starts_with("Ölçü: kırıklı yarıçapı ölçülecek"));
}

/// Semt (docs/adr/0147 §7): two points, or an edge with Kenardan, then the
/// arrow's place or its distance typed (left positive).
#[test]
fn an_azimuth_takes_two_points_or_an_edge_then_its_arrow() {
    let mut b = Bench::new("dimension");
    b.add_line("cizim", [0.0, 0.0], [30.0, 40.0]);
    assert!(b.type_text("T"));
    assert!(b.session.prompt().text().starts_with("Ölçü: semt ölçüsünün başlangıcını gösterin [Kenardan (K) / Zemin (Z): kapalı / Hizalı (H)"));
    b.click(0.0, 0.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: kenarın sonunu gösterin");
    b.click(30.0, 40.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: okun yerini gösterin ya da uzaklık yazın [Zemin (Z): kapalı]");
    // 5 m left of the edge, by the cursor.
    b.click(15.0 - 4.0, 20.0 + 3.0);
    let d = newest(&b);
    assert_eq!(d.style, Some(DimensionStyle::Azimuth));
    assert!(near(d.offset, 5.0), "{d:?}");
    // atan2(30, 40) in grads: 40.9666…
    assert_eq!(b.last_text(), Some("Semt ölçüsü eklendi: t=40.9666 g"));
    // Kenardan: the line clicked gives its two ends; a typed distance places the arrow.
    assert!(b.type_text("K"));
    assert!(b.session.prompt().text().starts_with("Ölçü: semti ölçülecek kenara tıklayın [Noktalardan (K)"));
    b.click(15.0, 20.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: okun yerini gösterin ya da uzaklık yazın [Zemin (Z): kapalı]");
    assert!(b.type_text("-3"));
    let d = newest(&b);
    assert_eq!((rel(d.a), rel(d.b)), ([0.0, 0.0], [30.0, 40.0]));
    assert!(near(d.offset, -3.0));
    // Kenardan off again (the memory as it was).
    assert!(b.type_text("K"));
}

/// Eğim (docs/adr/0147 §7): two points, each point's elevation from the
/// vertex it snapped to, else asked; then the arrow's place.
#[test]
fn a_slope_takes_its_points_elevations_from_their_vertices_or_asks() {
    let mut b = Bench::new("dimension");
    // A point at elevation 105.25; the other end is bare ground.
    b.add_point_z("cizim", [0.0, 0.0], 105.25);
    b.draft.snap = true;
    assert!(b.type_text("E"));
    assert!(b.session.prompt().text().starts_with("Ölçü: eğim ölçüsünün birinci noktasını gösterin [Kenardan (K)"));
    b.click(0.05, 0.05);
    // Snapped to the point: its elevation is taken.
    assert_eq!(b.session.prompt().text(), "Ölçü: ikinci noktayı gösterin");
    b.draft.snap = false;
    b.click(40.0, 0.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: ikinci noktanın kotunu yazın (m)");
    // A click while it asks is refused.
    b.click(20.0, 5.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(!b.type_text("abc"));
    assert!(b.type_text("104.75"));
    assert_eq!(b.session.prompt().text(), "Ölçü: okun yerini gösterin ya da uzaklık yazın [Zemin (Z): kapalı]");
    assert!(b.type_text("1.5"));
    let d = newest(&b);
    assert_eq!(d.style, Some(DimensionStyle::Slope));
    assert_eq!((d.za, d.zb), (Some(105.25), Some(104.75)));
    assert!(near(d.offset, 1.5));
    // 0.5 m over 40 m.
    assert_eq!(b.last_text(), Some("Eğim ölçüsü eklendi: %1.25"));
}

/// Açı from an arc or a circle (docs/adr/0147 §7): Yaydan, the arc's own
/// angle about its centre, its ends counter-clockwise; Daireden, from the
/// point clicked, put on the circle, to a second point. Ctrl+Z takes the
/// second point back, then the circle.
#[test]
fn an_angle_from_an_arc_or_a_circle() {
    let mut b = Bench::new("dimension");
    // A quarter arc of radius 10 about (0, 0), from east to north; a circle of 5 about (40, 0).
    b.add_arc("cizim", [0.0, 0.0], 10.0, 0.0, std::f64::consts::FRAC_PI_2);
    b.add_circle("cizim", [40.0, 0.0], 5.0);
    assert!(b.type_text("A"));
    b.click(10.0 * std::f64::consts::FRAC_1_SQRT_2, 10.0 * std::f64::consts::FRAC_1_SQRT_2);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: yayın yerini gösterin ya da yarıçap yazın [Zemin (Z): kapalı]"
    );
    // The arc's own angle wherever the cursor is; the radius typed, its sign dropped.
    b.move_to(-3.0, -3.0);
    assert!(b.type_text("-6"));
    let d = newest(&b);
    assert_eq!(d.style, Some(DimensionStyle::Angular));
    assert_eq!(d.c.map(rel), Some([0.0, 0.0]));
    assert!(near(rel(d.a)[0], 10.0) && near(rel(d.a)[1], 0.0), "{d:?}");
    assert!(near(rel(d.b)[0], 0.0) && near(rel(d.b)[1], 10.0), "{d:?}");
    assert!(near(d.offset, 6.0), "{d:?}");
    assert_eq!(b.last_text(), Some("Açı ölçüsü eklendi: 100.0000 g"));
    // Daireden: the click is put on the circle (east of its centre), then a second point north of it.
    b.click(45.2, 0.0);
    assert_eq!(b.session.prompt().text(), "Ölçü: açının ikinci noktasını gösterin");
    b.click(40.0, 10.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: yayın yerini gösterin ya da yarıçap yazın [Zemin (Z): kapalı]"
    );
    // Ctrl+Z: the second point, then the circle.
    assert!(b.undo_step());
    assert_eq!(b.session.prompt().text(), "Ölçü: açının ikinci noktasını gösterin");
    assert!(b.undo_step());
    assert!(b.session.prompt().text().starts_with("Ölçü: açı ölçüsü için bir kenara"));
    b.click(45.2, 0.0);
    b.click(40.0, 10.0);
    // Between the two, counter-clockwise from the first.
    b.click(43.0, 3.0);
    let d = newest(&b);
    assert_eq!(d.c.map(rel), Some([40.0, 0.0]));
    assert_eq!((rel(d.a), rel(d.b)), ([45.0, 0.0], [40.0, 10.0]));
    assert!(near(d.offset, 18f64.sqrt()), "{d:?}");
    assert_eq!(b.last_text(), Some("Açı ölçüsü eklendi: 100.0000 g"));
}

/// Zemin (Z, docs/adr/0147 §7): offered while nothing is picked and while
/// the dimension is placed; the dimension is written with its value over the
/// drawing's background, and the choice stays for the next ones.
#[test]
fn zemin_writes_the_value_over_the_background() {
    let mut b = Bench::new("dimension");
    assert!(b.type_text("Z"));
    assert!(
        b.session.prompt().text().contains("[Zemin (Z): açık / Doğrusal (D)"),
        "{}",
        b.session.prompt().text()
    );
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: ölçü çizgisinin yerini gösterin ya da mesafe yazın [Zemin (Z): açık]"
    );
    b.click(5.0, 3.0);
    assert!(newest(&b).mask);
    // Off while the next is placed: written without.
    b.click(0.0, 10.0);
    b.click(10.0, 10.0);
    assert!(b.type_text("Z"));
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: ölçü çizgisinin yerini gösterin ya da mesafe yazın [Zemin (Z): kapalı]"
    );
    b.click(5.0, 13.0);
    assert!(!newest(&b).mask);
}

/// Doğrusal's Açı (A, docs/adr/0147 §7): the measuring direction typed in
/// the project's angle unit, counter-clockwise from east; kept like the
/// Y and X locks.
#[test]
fn linear_measures_along_a_typed_direction() {
    let mut b = Bench::new("dimension");
    assert!(b.type_text("D"));
    b.click(0.0, 0.0);
    b.click(10.0, 10.0);
    assert!(b.type_text("A"));
    assert_eq!(
        b.session.prompt().text(),
        "Ölçü: ölçme doğrultusunu yazın (grad, doğudan saatin tersine)"
    );
    // Not a number: refused, still asked.
    assert!(!b.type_text("Y"));
    assert!(b.session.prompt().text().starts_with("Ölçü: ölçme doğrultusunu"));
    // 50 grads: 45°.
    assert!(b.type_text("50"));
    assert!(
        b.session
            .prompt()
            .text()
            .ends_with("[Yatay ΔY (Y) / Düşey ΔX (X) / Yön (O): 50.0000 g / Açı (A) / Zemin (Z): kapalı]"),
        "{}",
        b.session.prompt().text()
    );
    b.click(0.0, 10.0);
    let d = newest(&b);
    assert_eq!(d.style, Some(DimensionStyle::Linear));
    assert!(d.angle.is_some_and(|a| near(a, 45.0)), "{d:?}");
    // The whole diagonal lies along 45°: 10√2.
    assert_eq!(b.last_text(), Some("Doğrusal ölçü eklendi: 14.142"));
    // Kept: the next linear dimension is measured along it too.
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert!(b.session.prompt().text().contains("Yön (O): 50.0000 g"));
}
