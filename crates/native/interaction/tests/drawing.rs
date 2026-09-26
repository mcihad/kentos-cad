//! The ellipse, spline, construction line, ray, parallel line,
//! perpendicular, donut, revision cloud, spot elevation and divide tools
//! through the session, over the native document: what the interaction
//! traces do not fix, one rule per test (docs/adr/0057): the web's words,
//! one undo step, the locked and hidden layer answers of the product
//! commands said once, and what the tools remember. Expected values are
//! worked out by hand, not taken from a run.

mod common;

use common::{Bench, rel};
use kentos_contracts::{Entity, HatchPatternType};
use kentos_interaction::Level;

const OBJECTS: &str = include_str!("../../../../fixtures/interaction/v1/objects.kcad");
const TOOLS: &str = include_str!("../../../../fixtures/interaction/v1/tools.kcad");

/// `tool` running on `drawing` with `layer` active, snapping off.
fn on(drawing: &str, layer: &str, tool: &str) -> Bench {
    let mut b = Bench::on(drawing);
    b.draft.snap = false;
    assert!(b.doc.set_active_layer(layer), "{layer} is a layer");
    b.start(tool);
    b
}

const LOCKED: &str = "“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.";
const HIDDEN: &str = "“Gizli katman” katmanı gizli; çizilen nesne görünmeyecek.";

// ── Elips ───────────────────────────────────────────────────────────────────

#[test]
fn the_ellipse_says_what_the_web_says_and_is_one_step() {
    let mut b = Bench::new("ellipse");
    assert_eq!(
        b.session.prompt().text(),
        "Elips: bir eksenin ilk ucunu belirtin [Merkez (M) / Yay (Y): kapalı]"
    );
    b.click(-20.0, 10.0);
    assert_eq!(
        b.session.prompt().text(),
        "Elips: eksenin diğer ucunu belirtin"
    );
    b.click(0.0, 10.0);
    assert_eq!(
        b.session.prompt().text(),
        "Elips: diğer yarı ekseni gösterin ya da uzunluğunu yazın [Döndürme (D)]"
    );
    assert!(b.type_text("4"));
    assert_eq!(
        b.last_text(),
        Some("Elips eklendi: 10.000 × 4.000 m (yarı eksenler)")
    );
    let Entity::Ellipse(e) = b.newest() else {
        panic!("an ellipse");
    };
    assert_eq!(rel(e.c), [-10.0, 10.0]);
    assert_eq!([e.major.x, e.major.y], [10.0, 0.0]);
    assert_eq!((e.ratio, e.t0, e.t1), (0.4, 0.0, 0.0));
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
}

#[test]
fn an_elliptic_arc_asks_for_its_angles_and_zero_axes_warn() {
    let mut b = Bench::new("ellipse");
    assert!(b.type_text("Y"));
    assert_eq!(
        b.session.prompt().text(),
        "Elips: Eliptik yay: bir eksenin ilk ucunu belirtin [Merkez (M) / Yay (Y): açık]"
    );
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    // A typed zero is not taken (the web's `n > 0`); the centre shown is.
    let before = b.log.len();
    assert!(b.type_text("0"));
    assert!(b.said(before).is_empty());
    b.click(5.0, 0.0);
    assert_eq!(b.last_text(), Some("Eksenler sıfır uzunlukta olamaz."));
    assert!(b.type_text("2.5"));
    assert_eq!(
        b.session.prompt().text(),
        "Elips: başlangıç açısını gösterin ya da yazın (büyük eksenden, derece)"
    );
    assert!(b.type_text("30"));
    assert!(b.type_text("30"));
    assert_eq!(b.last_text(), Some("Bitiş açısı başlangıçla aynı olamaz."));
    assert!(b.type_text("180"));
    assert_eq!(
        b.last_text(),
        Some("Eliptik yay eklendi: 5.000 × 2.500 m (yarı eksenler)")
    );
    assert_eq!(b.doc.len(), 1);
}

// ── Eğri ────────────────────────────────────────────────────────────────────

#[test]
fn a_spline_says_its_points_and_length_and_one_point_writes_nothing() {
    let mut b = Bench::new("spline");
    b.click(0.0, 0.0);
    b.confirm();
    assert_eq!((b.doc.len(), b.points()), (0, 0), "one point is no curve");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    b.confirm();
    // Two fit points: the curve is their straight line.
    assert_eq!(b.last_text(), Some("Eğri eklendi: 2 nokta, 10.000 m"));
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
}

// ── Yardımcı çizgi ve ışın ─────────────────────────────────────────────────

#[test]
fn the_typed_angle_stays_for_the_next_construction_lines() {
    let mut b = Bench::new("xline");
    assert!(b.type_text("A"));
    assert_eq!(
        b.session.prompt().text(),
        "Yardımcı çizgi: açıyı yazın (derece, doğudan saat yönünün tersine)"
    );
    assert!(b.type_text("12.34567"));
    assert_eq!(
        b.session.prompt().text(),
        "Yardımcı çizgi: geçeceği noktayı belirtin (12.3457°) [Bitir (Enter)]"
    );
    assert!((b.memory.xline_angle - 12.34567).abs() < 1e-12);
    b.click(0.0, 0.0);
    assert_eq!(b.last_text(), Some("Yardımcı çizgi eklendi."));
    let Entity::Xline(x) = b.newest() else {
        panic!("a construction line");
    };
    let a = 12.34567_f64.to_radians();
    assert!((x.dir.x - a.cos()).abs() < 1e-12 && (x.dir.y - a.sin()).abs() < 1e-12);
}

#[test]
fn a_ray_needs_a_direction() {
    let mut b = Bench::new("ray");
    b.click(0.0, 0.0);
    b.click(0.0, 0.0);
    assert_eq!(b.doc.len(), 0, "the start again gives no direction");
    b.click(0.0, 5.0);
    assert_eq!(b.last_text(), Some("Işın eklendi."));
    assert_eq!(b.doc.len(), 1);
}

// ── Paralel çizgi ───────────────────────────────────────────────────────────

#[test]
fn a_parallel_line_is_one_step_and_says_its_width() {
    let mut b = Bench::new("parallel");
    assert_eq!(
        b.session.prompt().text(),
        "Paralel çizgi: eksenin ilk noktasını gösterin [Sol (S): 5.000 m / Sağ (A): 5.000 m / Eksen (E): çizilir / Alan olarak (U): hayır]"
    );
    b.click(-10.0, 0.0);
    b.click(10.0, 0.0);
    b.confirm();
    assert_eq!(b.doc.len(), 3);
    assert_eq!(
        b.last_text(),
        Some("Paralel çizgi: 3 nesne, genişlik 10.000 m.")
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Paralel çizgi"));
    assert_eq!(b.doc.len(), 0, "the three objects in one step");
}

#[test]
fn the_corridor_as_an_area_says_its_area() {
    let mut b = Bench::new("parallel");
    assert!(b.type_text("E"));
    assert!(b.type_text("U"));
    b.click(-10.0, 0.0);
    b.click(10.0, 0.0);
    b.confirm();
    assert_eq!(b.doc.len(), 1, "the corridor alone: the axis is not drawn");
    assert_eq!(
        b.last_text(),
        Some("Paralel çizgi: 1 nesne, genişlik 10.000 m, alan 200.00 m².")
    );
}

#[test]
fn a_parallel_line_on_a_locked_layer_says_so_once() {
    let mut b = on(OBJECTS, "kilitli", "parallel");
    b.click(-10.0, 0.0);
    b.click(10.0, 0.0);
    let before = b.log.len();
    b.confirm();
    // The web said it three times (each side and the axis); the command once.
    assert_eq!(b.said(before), vec![(Level::Warn, LOCKED)]);
    assert_eq!(b.doc.len(), 7);
}

#[test]
fn nothing_to_draw_is_said() {
    let mut b = Bench::new("parallel");
    for (key, value) in [("S", "0"), ("A", "0")] {
        assert!(b.type_text(key));
        assert!(b.type_text(value));
    }
    assert!(b.type_text("E"));
    b.click(-10.0, 0.0);
    b.click(10.0, 0.0);
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Sol ve sağ mesafe sıfır ve eksen çizilmiyor: çizilecek bir şey yok.")
    );
    assert_eq!(b.doc.len(), 0);
    let bad = b.type_text("S") && b.type_text("-1");
    assert!(bad);
    assert_eq!(
        b.last_text(),
        Some("Mesafe sıfır ya da pozitif olmalı; karşı taraf için diğer seçeneği kullanın.")
    );
}

// ── Dik in, dik çık ─────────────────────────────────────────────────────────

#[test]
fn a_perpendicular_is_named_after_its_tool() {
    let mut b = on(TOOLS, "cizim", "perpIn");
    b.click(-20.0, -12.0);
    b.click(-16.0, -4.0);
    assert_eq!(b.last_text(), Some("Dik inildi: dik boy 8.000 m."));
    assert_eq!(b.doc.undo().as_deref(), Some("Dik in"));
    b.start("perpOut");
    b.click(-22.0, -12.0);
    assert!(b.type_text("4"));
    assert!(b.type_text("0"));
    assert_eq!(b.last_text(), Some("Dik boy sıfır olamaz."));
    assert!(b.type_text("2.5"));
    assert_eq!(
        b.last_text(),
        Some("Dik çıkıldı: dik ayak 4.000 m, dik boy 2.500 m.")
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Dik çık"));
}

#[test]
fn a_perpendicular_on_a_locked_layer_says_only_why() {
    let mut b = on(OBJECTS, "kilitli", "perpIn");
    b.click(-20.0, -12.0);
    let before = b.log.len();
    b.click(-16.0, -4.0);
    // The web added “Nokta zaten hattın üzerinde.” to the lock's message.
    assert_eq!(b.said(before), vec![(Level::Warn, LOCKED)]);
}

#[test]
fn dik_cik_takes_straight_edges_only() {
    let mut b = on(TOOLS, "cizim", "perpOut");
    b.click(16.0, -4.0);
    assert_eq!(
        b.last_text(),
        Some("Düz bir kenara tıklayın: çizgi ya da çoklu çizgi.")
    );
    assert_eq!(b.options(), Vec::<&str>::new(), "no reference");
}

// ── Halka ───────────────────────────────────────────────────────────────────

#[test]
fn a_donut_is_a_solid_hatch_with_its_hole() {
    let mut b = Bench::new("donut");
    b.click(0.0, 0.0);
    let Entity::Hatch(h) = b.newest() else {
        panic!("a hatch");
    };
    assert_eq!(h.pattern.kind, HatchPatternType::Solid);
    assert_eq!(
        (h.ring.len(), h.holes.as_ref().map(Vec::len)),
        (96, Some(1))
    );
    assert!(
        (rel(h.ring[0])[0] - 0.5).abs() < 1e-12,
        "outer radius 0.5 m"
    );
    // A bigger hole keeps the ring's width: the outer diameter follows.
    assert!(b.type_text("I"));
    assert!(b.type_text("2"));
    assert_eq!((b.memory.donut_inner, b.memory.donut_outer), (2.0, 2.5));
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
}

#[test]
fn a_donut_on_a_hidden_layer_warns_and_is_drawn() {
    let mut b = on(OBJECTS, "gizli", "donut");
    let before = b.log.len();
    b.click(0.0, 0.0);
    assert_eq!(b.said(before).last(), Some(&(Level::Warn, HIDDEN)));
    assert_eq!(b.doc.len(), 8);
}

// ── Revizyon bulutu ─────────────────────────────────────────────────────────

#[test]
fn a_cloud_says_its_arcs_and_a_refused_one_only_why() {
    let mut b = Bench::new("revcloud");
    b.click(-8.0, -4.0);
    b.click(8.0, 4.0);
    // 8 mm at 1:1000 is 8 m: two arcs on the 16 m sides, one on the 8 m ones.
    assert_eq!(b.last_text(), Some("Revizyon bulutu eklendi: 6 yay."));
    let Entity::Polygon(p) = b.newest() else {
        panic!("a closed area");
    };
    assert_eq!(p.bulges.as_deref(), Some(&[0.5; 6][..]));
    let mut b = on(OBJECTS, "kilitli", "revcloud");
    b.click(-8.0, -4.0);
    let before = b.log.len();
    b.click(8.0, 4.0);
    // The corner's echo, then the lock's message; the web added “Bulut için
    // alanı olan …” to it.
    assert_eq!(
        b.said(before),
        vec![
            (Level::Info, "  Y 487008.000  X 4420004.000"),
            (Level::Warn, LOCKED)
        ]
    );
}

// ── Kot noktası ─────────────────────────────────────────────────────────────

#[test]
fn a_spot_elevation_goes_on_its_layer_with_its_elevation() {
    let mut b = on(TOOLS, "cizim", "spot");
    b.click(-4.0, -4.0);
    assert_eq!(
        b.session.prompt().text(),
        "Kot noktası: kot değerini yazın (m)"
    );
    assert!(b.type_text("12.3456"));
    let Entity::Point(p) = b.newest() else {
        panic!("a point");
    };
    assert_eq!(p.base.layer_id, "kot");
    assert_eq!(p.z, Some(12.3456));
    assert_eq!(p.base.label.as_deref(), Some("12.35"));
    assert_eq!(
        p.base.attrs.get("Tür").map(String::as_str),
        Some("Kot noktası")
    );
    assert_eq!(
        p.base.attrs.get("Z (m)").map(String::as_str),
        Some("12.346")
    );
    // A drawing without the spot elevations' layer says so.
    let mut b = on(OBJECTS, "cizim", "spot");
    b.click(0.0, 0.0);
    assert!(b.type_text("1"));
    assert_eq!(
        b.last_text(),
        Some("“kot” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin.")
    );
    assert_eq!(b.doc.len(), 7);
}

// ── Böl ─────────────────────────────────────────────────────────────────────

#[test]
fn divide_is_one_step_and_the_locked_active_layer_says_why() {
    let mut b = on(TOOLS, "cizim", "divide");
    b.click(-18.0, 10.0);
    assert!(b.type_text("5"));
    assert_eq!(b.last_text(), Some("4 nokta kondu."));
    assert_eq!(b.doc.undo().as_deref(), Some("Böl"));
    assert_eq!(b.doc.len(), 4);
    assert!(b.type_text("A"));
    b.click(-18.0, 10.0);
    assert!(b.type_text("25"));
    assert_eq!(
        b.last_text(),
        Some("Aralık nesne boyundan uzun; nokta konmadı.")
    );
    assert!(b.type_text("P"), "back to parts");
    let mut b = on(OBJECTS, "kilitli", "divide");
    b.click(-16.0, -12.0);
    let before = b.log.len();
    b.confirm();
    assert_eq!(b.said(before), vec![(Level::Warn, LOCKED)]);
}
