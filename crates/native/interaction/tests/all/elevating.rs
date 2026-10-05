//! Kot ver through the session, over the native document (docs/adr/0142):
//! the steps and the chips, the three ways (Sabit, Artır, Sıfırla), the words
//! it says, one undo step named “Kot ver”, what it leaves out, and the locked
//! layer. The drawing is the traces' `edits.kcad` (a locked layer `kilitli`)
//! with the objects each test adds north of y = 22 m. The web's tool says the
//! same words (`apps/web/src/tools/elevationTool.test.ts`). Expected values
//! are worked out by hand.

use crate::common;

use common::{Bench, E, N};
use kentos_contracts::{
    CircleEntity, Entity, EntityBase, LineEntity, PathEntity, PointEntity, RingGeometry,
    TextEntity, Vec2,
};
use kentos_domain::Slot;
use kentos_interaction::Level;

const EDITS: &str = include_str!("../../../../../fixtures/interaction/v1/edits.kcad");

fn bench() -> Bench {
    let mut b = Bench::on(EDITS);
    b.draft.snap = false;
    b
}

fn pt(x: f64, y: f64) -> Vec2 {
    Vec2 { x: E + x, y: N + y }
}

fn base(layer: &str) -> EntityBase {
    common::base(layer)
}

fn add(b: &mut Bench, e: Entity) -> Slot {
    b.doc.add(e).expect("a slot")
}

fn zs(v: &[Option<f64>]) -> Option<Vec<Option<f64>>> {
    Some(v.to_vec())
}

fn line(b: &mut Bench, layer: &str, y: f64, za: Option<f64>, zb: Option<f64>) -> Slot {
    add(
        b,
        Entity::Line(LineEntity {
            base: base(layer),
            a: pt(0.0, y),
            b: pt(30.0, y),
            za,
            zb,
        }),
    )
}

fn polyline(b: &mut Bench, y: f64, z: Option<Vec<Option<f64>>>) -> Slot {
    add(
        b,
        Entity::Polyline(PathEntity {
            base: base("cizim"),
            pts: vec![pt(0.0, y), pt(5.0, y), pt(9.0, y)],
            bulges: None,
            holes: None,
            zs: z,
            parts: None,
        }),
    )
}

/// A 10 m square from (0, 34) with a triangular hole.
fn area(b: &mut Bench, z: Option<Vec<Option<f64>>>, hole: Option<Vec<Option<f64>>>) -> Slot {
    add(
        b,
        Entity::Polygon(PathEntity {
            base: base("parsel"),
            pts: vec![pt(0.0, 34.0), pt(10.0, 34.0), pt(10.0, 44.0), pt(0.0, 44.0)],
            bulges: None,
            holes: Some(vec![RingGeometry {
                pts: vec![pt(2.0, 36.0), pt(4.0, 36.0), pt(4.0, 38.0)],
                bulges: None,
                zs: hole,
            }]),
            zs: z,
            parts: None,
        }),
    )
}

fn point(b: &mut Bench, at: [f64; 2], z: Option<f64>) -> Slot {
    add(
        b,
        Entity::Point(PointEntity {
            base: base("cizim"),
            p: pt(at[0], at[1]),
            z,
            parts: None,
        }),
    )
}

/// The objects of the web's scene.
struct Scene {
    line: Slot,
    path: Slot,
    bare: Slot,
    area: Slot,
    spot: Slot,
    spotless: Slot,
}

fn scene(b: &mut Bench) -> Scene {
    Scene {
        line: line(b, "cizim", 22.0, Some(10.0), Some(20.0)),
        path: polyline(b, 27.0, zs(&[Some(1.0), None, Some(3.0)])),
        bare: polyline(b, 30.0, None),
        area: area(
            b,
            zs(&[Some(1.0), Some(2.0), Some(3.0), Some(4.0)]),
            zs(&[Some(5.0), Some(6.0), Some(7.0)]),
        ),
        spot: point(b, [20.0, 40.0], Some(12.5)),
        spotless: point(b, [21.0, 41.0], None),
    }
}

fn select(b: &mut Bench, slots: &[Slot]) {
    b.selection.set(slots.iter().copied());
}

fn line_z(b: &Bench, slot: Slot) -> (Option<f64>, Option<f64>) {
    match b.doc.get(slot) {
        Some(Entity::Line(l)) => (l.za, l.zb),
        other => panic!("a line at {slot:?}: {other:?}"),
    }
}

fn path_z(b: &Bench, slot: Slot) -> Option<Vec<Option<f64>>> {
    match b.doc.get(slot) {
        Some(Entity::Polyline(p) | Entity::Polygon(p)) => p.zs.clone(),
        other => panic!("a path at {slot:?}: {other:?}"),
    }
}

fn hole_z(b: &Bench, slot: Slot) -> Option<Vec<Option<f64>>> {
    match b.doc.get(slot) {
        Some(Entity::Polygon(p)) => p.holes.as_ref().and_then(|h| h[0].zs.clone()),
        other => panic!("an area at {slot:?}: {other:?}"),
    }
}

fn point_z(b: &Bench, slot: Slot) -> Option<f64> {
    match b.doc.get(slot) {
        Some(Entity::Point(p)) => p.z,
        other => panic!("a point at {slot:?}: {other:?}"),
    }
}

fn prompt(b: &Bench) -> String {
    b.session.prompt().text()
}

fn cancel(b: &mut Bench) -> bool {
    b.run(|s, cx| s.cancel(cx))
}

/// The messages said since `before`, as `level: text`.
fn said(b: &Bench, before: usize) -> Vec<String> {
    b.said(before)
        .into_iter()
        .map(|(level, text)| format!("{}: {text}", level.as_str()))
        .collect()
}

const PICKING: &str = "Kot ver: kot verilecek nesneleri seçin [Artır (A) / Sıfırla (S)]";
const NUMBER: &str = "Kot ver: kotu yazın (m) [Artır (A) / Sıfırla (S)]";
const NONE_TAKES: &str = "Seçimde kot alan nesne yok; çizgi, çoklu çizgi, alan ya da nokta seçin.";

// ── The steps ───────────────────────────────────────────────────────────────

#[test]
fn it_asks_for_the_objects_first_and_then_for_the_elevation_with_both_chips_at_both() {
    let mut b = bench();
    let s = scene(&mut b);
    b.start("setElevation");
    assert_eq!(prompt(&b), PICKING);
    assert_eq!(b.options(), ["A", "S"]);
    // A click picks, a confirm ends the picking.
    b.click(1.0, 22.0);
    assert_eq!(b.selected(), [s.line.0]);
    b.confirm();
    assert_eq!(prompt(&b), NUMBER);
    assert_eq!(b.options(), ["A", "S"]);
    // Nothing is written by picking.
    assert_eq!(line_z(&b, s.line), (Some(10.0), Some(20.0)));
}

#[test]
fn a_window_picks_too() {
    let mut b = bench();
    let s = scene(&mut b);
    b.start("setElevation");
    // Left to right around the line and the polyline, holding neither of the others.
    b.drag([-2.0, 20.0], [32.0, 31.0]);
    let mut got = b.selected();
    got.sort();
    assert_eq!(got, [s.line.0, s.path.0, s.bare.0]);
}

#[test]
fn a_selection_made_before_the_tool_started_is_used_at_once() {
    let mut b = bench();
    let s = scene(&mut b);
    select(&mut b, &[s.line]);
    b.start("setElevation");
    assert_eq!(prompt(&b), NUMBER);
}

#[test]
fn artir_is_a_toggle_and_the_number_asked_for_becomes_a_difference() {
    let mut b = bench();
    let s = scene(&mut b);
    select(&mut b, &[s.line]);
    b.start("setElevation");
    assert!(b.type_text("a"));
    assert_eq!(
        prompt(&b),
        "Kot ver: eklenecek farkı yazın (m) [Artır (A): açık / Sıfırla (S)]"
    );
    assert!(b.type_text("A"));
    assert_eq!(prompt(&b), NUMBER);
}

#[test]
fn artir_set_while_picking_is_still_on_at_the_next_step() {
    let mut b = bench();
    let s = scene(&mut b);
    b.start("setElevation");
    assert!(b.type_text("A"));
    assert_eq!(
        prompt(&b),
        "Kot ver: kot verilecek nesneleri seçin [Artır (A): açık / Sıfırla (S)]"
    );
    select(&mut b, &[s.line]);
    b.confirm();
    assert_eq!(
        prompt(&b),
        "Kot ver: eklenecek farkı yazın (m) [Artır (A): açık / Sıfırla (S)]"
    );
}

/// The ribbon's methods each start the tool the way they say: the switch is the run's.
#[test]
fn each_start_is_a_fresh_run_artir_is_not_carried_over() {
    let mut b = bench();
    let s = scene(&mut b);
    select(&mut b, &[s.line]);
    b.start("setElevation");
    assert!(b.type_text("A"));
    b.start("setElevation");
    assert_eq!(prompt(&b), NUMBER);
}

#[test]
fn esc_steps_back_one_step_from_the_number_to_the_picking_and_then_out() {
    let mut b = bench();
    let s = scene(&mut b);
    select(&mut b, &[s.line]);
    b.start("setElevation");
    assert_eq!(prompt(&b), NUMBER);
    assert!(cancel(&mut b), "the tool stays");
    assert_eq!(prompt(&b), PICKING);
    assert_eq!(b.selected(), [s.line.0], "the selection stays");
    assert!(!cancel(&mut b), "from picking it leaves");
    assert!(!b.session.is_running());
}

#[test]
fn enter_with_nothing_selected_leaves_and_at_the_number_step_goes_back_as_esc_does() {
    let mut b = bench();
    let s = scene(&mut b);
    b.start("setElevation");
    b.confirm();
    assert!(!b.session.is_running(), "nothing picked: Enter leaves");
    select(&mut b, &[s.line]);
    b.start("setElevation");
    b.confirm();
    assert!(b.session.is_running(), "at the number step it steps back");
    assert_eq!(prompt(&b), PICKING);
    assert_eq!(b.selected(), [s.line.0], "the selection stays");
}

#[test]
fn a_selection_that_no_longer_takes_an_elevation_goes_back_to_picking() {
    let mut b = bench();
    let s = scene(&mut b);
    select(&mut b, &[s.line]);
    b.start("setElevation");
    assert_eq!(prompt(&b), NUMBER);
    // The line goes while the number is asked for (another hand, an undo).
    assert_eq!(b.doc.remove(&[s.line]), 1);
    let before = b.log.len();
    assert!(b.type_text("5"));
    assert_eq!(said(&b, before), [format!("warn: {NONE_TAKES}")]);
    assert_eq!(prompt(&b), PICKING);
}

#[test]
fn a_number_before_any_object_is_picked_is_not_understood_and_a_computed_point_means_nothing() {
    let mut b = bench();
    let _ = scene(&mut b);
    b.start("setElevation");
    assert!(!b.type_text("100"));
    assert!(
        !b.session.can_calc_point(),
        "only a number means anything here"
    );
    select(&mut b, &[Slot(1)]);
    b.confirm();
    assert!(!b.session.can_calc_point());
}

#[test]
fn what_is_not_a_number_is_not_understood_and_a_comma_reads_as_a_coordinate() {
    let mut b = bench();
    let s = scene(&mut b);
    select(&mut b, &[s.line]);
    b.start("setElevation");
    let rev = b.doc.revision();
    for text in ["yüz", "12,5", "@5,3", "3<4"] {
        assert!(!b.type_text(text), "{text}");
    }
    assert_eq!(b.doc.revision(), rev);
    assert_eq!(line_z(&b, s.line), (Some(10.0), Some(20.0)));
}

// ── Sabit ───────────────────────────────────────────────────────────────────

#[test]
fn sabit_gives_every_vertex_the_number_a_point_its_z_in_one_step_named_kot_ver() {
    let mut b = bench();
    let s = scene(&mut b);
    select(
        &mut b,
        &[s.line, s.path, s.bare, s.area, s.spot, s.spotless],
    );
    b.start("setElevation");
    let before = b.log.len();
    assert!(b.type_text("100.5"));
    assert_eq!(line_z(&b, s.line), (Some(100.5), Some(100.5)));
    assert_eq!(path_z(&b, s.path), zs(&[Some(100.5); 3]));
    assert_eq!(path_z(&b, s.bare), zs(&[Some(100.5); 3]));
    assert_eq!(path_z(&b, s.area), zs(&[Some(100.5); 4]));
    assert_eq!(hole_z(&b, s.area), zs(&[Some(100.5); 3]), "the holes too");
    assert_eq!(point_z(&b, s.spot), Some(100.5));
    assert_eq!(point_z(&b, s.spotless), Some(100.5));
    assert_eq!(said(&b, before), ["info: Kot verildi: 6 nesne."]);
    // Back at the first step, nothing selected, the tool still running.
    assert!(b.selected().is_empty());
    assert_eq!(prompt(&b), PICKING);
    assert!(b.session.is_running());
    // One step, named Kot ver.
    assert_eq!(b.doc.undo().as_deref(), Some("Kot ver"));
    assert_eq!(line_z(&b, s.line), (Some(10.0), Some(20.0)));
    assert_eq!(path_z(&b, s.path), zs(&[Some(1.0), None, Some(3.0)]));
    assert_eq!(path_z(&b, s.bare), None);
    assert_eq!(hole_z(&b, s.area), zs(&[Some(5.0), Some(6.0), Some(7.0)]));
    assert_eq!(point_z(&b, s.spot), Some(12.5));
    assert_eq!(point_z(&b, s.spotless), None);
}

#[test]
fn zero_is_an_elevation_and_so_is_a_negative_one() {
    let mut b = bench();
    let s = scene(&mut b);
    select(&mut b, &[s.line, s.bare]);
    b.start("setElevation");
    assert!(b.type_text("0"));
    assert_eq!(line_z(&b, s.line), (Some(0.0), Some(0.0)));
    assert_eq!(path_z(&b, s.bare), zs(&[Some(0.0); 3]));
    select(&mut b, &[s.line]);
    b.confirm();
    assert!(b.type_text("-4.25"));
    assert_eq!(line_z(&b, s.line), (Some(-4.25), Some(-4.25)));
}

#[test]
fn it_can_be_done_again_on_the_next_objects_without_leaving_the_tool() {
    let mut b = bench();
    let s = scene(&mut b);
    b.start("setElevation");
    select(&mut b, &[s.line]);
    b.confirm();
    assert!(b.type_text("50"));
    select(&mut b, &[s.bare]);
    b.confirm();
    assert!(b.type_text("60"));
    assert_eq!(line_z(&b, s.line), (Some(50.0), Some(50.0)));
    assert_eq!(path_z(&b, s.bare), zs(&[Some(60.0); 3]));
    assert!(b.session.is_running());
    assert_eq!(b.doc.undo().as_deref(), Some("Kot ver"));
    assert_eq!(b.doc.undo().as_deref(), Some("Kot ver"));
}

#[test]
fn everything_but_the_elevations_stays_layer_colour_attributes_label_geometry() {
    let mut b = bench();
    let mut e = PathEntity {
        base: base("parsel"),
        pts: vec![pt(0.0, 22.0), pt(10.0, 22.0), pt(10.0, 32.0), pt(0.0, 32.0)],
        bulges: None,
        holes: None,
        zs: None,
        parts: None,
    };
    e.base.color = Some("#E5484D".into());
    e.base.attrs.insert("Ada".into(), "12".into());
    e.base.label = Some("7".into());
    let slot = add(&mut b, Entity::Polygon(e.clone()));
    select(&mut b, &[slot]);
    b.start("setElevation");
    assert!(b.type_text("3"));
    let Some(Entity::Polygon(now)) = b.doc.get(slot) else {
        panic!("an area");
    };
    let mut want = e;
    want.base.id = slot.0;
    want.zs = zs(&[Some(3.0); 4]);
    assert_eq!(now, &want);
}

// ── Artır ───────────────────────────────────────────────────────────────────

#[test]
fn artir_adds_the_difference_to_each_vertex_that_has_an_elevation() {
    let mut b = bench();
    let s = scene(&mut b);
    select(
        &mut b,
        &[s.line, s.path, s.bare, s.area, s.spot, s.spotless],
    );
    b.start("setElevation");
    assert!(b.type_text("A"));
    let before = b.log.len();
    assert!(b.type_text("2.5"));
    assert_eq!(line_z(&b, s.line), (Some(12.5), Some(22.5)));
    assert_eq!(path_z(&b, s.path), zs(&[Some(3.5), None, Some(5.5)]));
    // No elevation to raise: left as it is.
    assert_eq!(path_z(&b, s.bare), None);
    assert_eq!(
        path_z(&b, s.area),
        zs(&[Some(3.5), Some(4.5), Some(5.5), Some(6.5)])
    );
    assert_eq!(hole_z(&b, s.area), zs(&[Some(7.5), Some(8.5), Some(9.5)]));
    assert_eq!(point_z(&b, s.spot), Some(15.0));
    assert_eq!(point_z(&b, s.spotless), None);
    // The objects raised are the ones counted.
    assert_eq!(
        said(&b, before),
        ["info: Kotlar +2.500 m değişti: 4 nesne."]
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Kot ver"));
}

#[test]
fn a_negative_difference_lowers_and_the_message_shows_its_real_minus_sign() {
    let mut b = bench();
    let s = scene(&mut b);
    select(&mut b, &[s.line]);
    b.start("setElevation");
    assert!(b.type_text("A"));
    assert!(b.type_text("-1"));
    assert_eq!(line_z(&b, s.line), (Some(9.0), Some(19.0)));
    assert_eq!(b.last_text(), Some("Kotlar −1.000 m değişti: 1 nesne."));
    assert_eq!(b.last_level(), Some(Level::Info));
}

#[test]
fn artir_adds_the_sum_as_it_is_not_a_rounded_one() {
    let mut b = bench();
    let l = line(&mut b, "cizim", 22.0, Some(100.1), Some(0.1));
    select(&mut b, &[l]);
    b.start("setElevation");
    assert!(b.type_text("A"));
    assert!(b.type_text("0.2"));
    assert_eq!(line_z(&b, l), (Some(100.1 + 0.2), Some(0.1 + 0.2)));
}

#[test]
fn artir_with_no_elevation_anywhere_says_so_writes_nothing_and_stays_at_the_number() {
    let mut b = bench();
    let s = scene(&mut b);
    select(&mut b, &[s.bare, s.spotless]);
    b.start("setElevation");
    let rev = b.doc.revision();
    assert!(b.type_text("A"));
    assert!(b.type_text("2"));
    assert_eq!(
        b.last_text(),
        Some("Seçili nesnelerin hiçbir köşesinde kot yok; fark eklenecek bir şey bulunamadı.")
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.revision(), rev);
    assert_eq!(b.selected().len(), 2, "the selection is kept");
    assert!(prompt(&b).starts_with("Kot ver: eklenecek farkı yazın (m)"));
}

// ── Sıfırla ─────────────────────────────────────────────────────────────────

#[test]
fn sifirla_writes_at_once_every_vertex_without_an_elevation_not_zero() {
    let mut b = bench();
    let s = scene(&mut b);
    select(&mut b, &[s.line, s.path, s.area, s.spot]);
    b.start("setElevation");
    let before = b.log.len();
    assert!(b.type_text("S"));
    assert_eq!(line_z(&b, s.line), (None, None));
    assert_eq!(path_z(&b, s.path), None);
    assert_eq!(path_z(&b, s.area), None);
    assert_eq!(hole_z(&b, s.area), None);
    assert_eq!(point_z(&b, s.spot), None);
    assert_eq!(said(&b, before), ["info: Kot silindi: 4 nesne."]);
    assert!(b.selected().is_empty());
    assert_eq!(b.doc.undo().as_deref(), Some("Kot ver"));
    assert_eq!(line_z(&b, s.line), (Some(10.0), Some(20.0)));
    assert_eq!(path_z(&b, s.path), zs(&[Some(1.0), None, Some(3.0)]));
}

#[test]
fn sifirla_at_the_picking_step_acts_on_the_objects_picked_so_far() {
    let mut b = bench();
    let s = scene(&mut b);
    b.start("setElevation");
    select(&mut b, &[s.line]);
    assert!(b.type_text("s"));
    assert_eq!(line_z(&b, s.line), (None, None));
    assert_eq!(b.last_text(), Some("Kot silindi: 1 nesne."));
    assert_eq!(prompt(&b), PICKING);
}

#[test]
fn sifirla_at_the_picking_step_with_nothing_picked_asks_for_objects_and_writes_nothing() {
    let mut b = bench();
    let _ = scene(&mut b);
    b.start("setElevation");
    let rev = b.doc.revision();
    assert!(b.type_text("S"));
    assert_eq!(b.last_text(), Some("Önce kotu silinecek nesneleri seçin."));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.revision(), rev);
}

/// The ribbon's Sıfırla with a selection: the tool starts, then the option goes in as if typed.
#[test]
fn the_ribbons_sifirla_with_a_selection_resets_that_selection() {
    let mut b = bench();
    let s = scene(&mut b);
    select(&mut b, &[s.line, s.spot]);
    b.start("setElevation");
    assert!(b.type_text("S"));
    assert_eq!(line_z(&b, s.line), (None, None));
    assert_eq!(point_z(&b, s.spot), None);
}

// ── What is left out ────────────────────────────────────────────────────────

#[test]
fn what_takes_no_elevation_is_counted_on_a_line_of_its_own_after_the_result() {
    let mut b = bench();
    let s = scene(&mut b);
    let circle = add(
        &mut b,
        Entity::Circle(CircleEntity {
            base: base("cizim"),
            c: pt(50.0, 50.0),
            r: 5.0,
        }),
    );
    let text = add(
        &mut b,
        Entity::Text(TextEntity {
            base: base("cizim"),
            p: pt(1.0, 1.0),
            text: "Park".into(),
            height: 2.0,
            rotation: 0.0,
            align: None,
            width_factor: None,
            mask: false,
        }),
    );
    let before_circle = b.doc.get(circle).cloned();
    select(&mut b, &[s.line, circle, text]);
    b.start("setElevation");
    let before = b.log.len();
    assert!(b.type_text("100"));
    assert_eq!(
        said(&b, before),
        [
            "info: Kot verildi: 1 nesne.",
            "warn: 2 nesne kot almaz (yalnız çizgi, çoklu çizgi, alan ve nokta)."
        ]
    );
    assert_eq!(b.doc.get(circle).cloned(), before_circle);
}

#[test]
fn nothing_that_takes_an_elevation_is_said_at_the_start_and_picking_goes_on() {
    let mut b = bench();
    let circle = add(
        &mut b,
        Entity::Circle(CircleEntity {
            base: base("cizim"),
            c: pt(50.0, 50.0),
            r: 5.0,
        }),
    );
    select(&mut b, &[circle]);
    let rev = b.doc.revision();
    b.start("setElevation");
    assert_eq!(b.last_text(), Some(NONE_TAKES));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(prompt(&b), PICKING);
    assert_eq!(b.selected(), [circle.0], "the selection is left as it was");
    // A number is not understood before objects are chosen; Sıfırla with the
    // circle picked finds nothing to write.
    assert!(b.type_text("S"));
    assert_eq!(b.last_text(), Some(NONE_TAKES));
    assert_eq!(b.doc.revision(), rev);
}

#[test]
fn the_same_words_when_the_selection_is_confirmed_with_nothing_that_takes_one() {
    let mut b = bench();
    let circle = add(
        &mut b,
        Entity::Circle(CircleEntity {
            base: base("cizim"),
            c: pt(50.0, 50.0),
            r: 5.0,
        }),
    );
    b.start("setElevation");
    select(&mut b, &[circle]);
    b.confirm();
    assert_eq!(b.last_text(), Some(NONE_TAKES));
    assert_eq!(prompt(&b), PICKING);
}

// ── The locked layer ────────────────────────────────────────────────────────

#[test]
fn objects_on_a_locked_layer_are_left_out_and_counted() {
    let mut b = bench();
    let locked = line(&mut b, "kilitli", 22.0, Some(1.0), Some(2.0));
    let open = line(&mut b, "cizim", 27.0, None, None);
    select(&mut b, &[locked, open]);
    b.start("setElevation");
    let before = b.log.len();
    assert!(b.type_text("10"));
    assert_eq!(line_z(&b, locked), (Some(1.0), Some(2.0)));
    assert_eq!(line_z(&b, open), (Some(10.0), Some(10.0)));
    assert_eq!(
        said(&b, before),
        [
            "info: Kot verildi: 1 nesne.",
            "warn: 1 nesne kilitli katmanda olduğu için atlandı."
        ]
    );
}

#[test]
fn if_only_locked_ones_are_chosen_the_command_refuses_with_its_own_message() {
    let mut b = bench();
    let locked = line(&mut b, "kilitli", 22.0, Some(1.0), Some(2.0));
    select(&mut b, &[locked]);
    b.start("setElevation");
    let rev = b.doc.revision();
    assert!(b.type_text("10"));
    assert_eq!(
        b.last_text(),
        Some(
            "“Kilitli katman” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."
        )
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.revision(), rev, "nothing is written");
    assert_eq!(line_z(&b, locked), (Some(1.0), Some(2.0)));
    // Still at the number: unlock the layer and type it again.
    assert_eq!(prompt(&b), NUMBER);
    assert_eq!(b.selected(), [locked.0]);
}

#[test]
fn sifirla_and_artir_are_refused_the_same_way() {
    let mut b = bench();
    let locked = line(&mut b, "kilitli", 22.0, Some(1.0), Some(2.0));
    select(&mut b, &[locked]);
    b.start("setElevation");
    assert!(b.type_text("S"));
    assert!(b.last_text().is_some_and(|t| t.contains("katmanı kilitli")));
    assert!(b.type_text("A"));
    assert!(b.type_text("1"));
    assert!(b.last_text().is_some_and(|t| t.contains("katmanı kilitli")));
    assert_eq!(line_z(&b, locked), (Some(1.0), Some(2.0)));
}
