//! Sorgu of docs/adr/0141 through the session, over the native document:
//! Mesafe ölç's fixed first point, Alan hesapla's İçine tıkla and Alan olarak
//! çiz, and Dik ayak ölç. The drawing (fixtures/interaction/v1/areas.kcad):
//! parcels 1 and 2 of 10 × 10 m overlapping, a circle, four lines closing a
//! 10 × 10 m square at x −28…−18, y −15…−5, parcel 8 of 10 × 10 m at x 18…28
//! with a 4 × 4 m hole, a square on the locked layer, a line across parcel 1.
//! Expected values are worked out by hand.

mod common;

use common::{Bench, E, N};
use kentos_contracts::Entity;
use kentos_interaction::{Format, Level, MarkerShape, measures};

const AREAS: &str = include_str!("../../../../fixtures/interaction/v1/areas.kcad");

fn bench(tool: &str) -> Bench {
    let mut b = Bench::on(AREAS);
    b.draft.snap = false;
    b.start(tool);
    b
}

/// A bench on the empty drawing: Dik ayak ölç reads against a line of its own.
fn empty(tool: &str) -> Bench {
    Bench::new(tool)
}

fn clicks(b: &mut Bench, pts: &[[f64; 2]]) {
    for &[de, dn] in pts {
        b.click(de, dn);
    }
}

fn preview(b: &Bench) -> kentos_interaction::Preview {
    b.session.preview(&Format::default()).expect("a tool runs")
}

fn tag(b: &Bench) -> Vec<String> {
    preview(b).tag.expect("a tag").lines
}

/// The messages said since `before` (a log length), without the echoes of taken points.
fn said(b: &Bench, before: usize) -> Vec<String> {
    b.said(before)
        .into_iter()
        .map(|(_, text)| text.to_owned())
        .filter(|text| !text.starts_with("  Y "))
        .collect()
}

// ── Mesafe ölç: Sabit ilk nokta ─────────────────────────────────────────────

#[test]
fn a_fixed_first_point_measures_every_point_from_it() {
    let mut b = bench("measure");
    assert_eq!(b.options(), ["S"]);
    assert!(b.type_text("s"), "the switch answers to its letter");
    assert!(b.memory.measure_fixed, "kept in the session's memory");
    assert_eq!(
        b.session.prompt().text(),
        "Mesafe ölç: ilk noktayı belirtin [Sabit ilk nokta (S): açık]"
    );
    b.click(-14.0, 4.0);
    assert_eq!(
        b.session.prompt().text(),
        "Mesafe ölç: sonraki noktayı belirtin [Sabit ilk nokta (S): açık / Geri (G)]"
    );
    let before = b.log.len();
    b.click(-14.0, 10.0);
    // 6 m due north of the first point: semt 0.
    assert_eq!(said(&b, before), ["1: 6.000 m, semt 0.0000 g"]);
    assert_eq!(b.last_level(), Some(Level::Info));
    let before = b.log.len();
    b.click(-8.0, 4.0);
    // Not from the point before (that would be 8.485 m): from the first, 6 m due east.
    assert_eq!(said(&b, before), ["2: 6.000 m, semt 100.0000 g"]);
    let before = b.log.len();
    b.click(-6.0, 12.0);
    // 8 east, 8 north: √128, semt 50 g.
    assert_eq!(said(&b, before), ["3: 11.314 m, semt 50.0000 g"]);
    assert_eq!(b.points(), 4);
    assert_eq!(b.options(), ["S", "G", "Enter"]);
    // Beside the cursor: the distance and semt from the first point, no total.
    b.move_to(-14.0, 16.0);
    assert_eq!(tag(&b), ["12.000 m", "Semt 0.0000 g"]);
    // The preview is rays from the first point, not a chain.
    let p = preview(&b);
    assert!(p.path.is_empty() && p.ring.is_none());
    let first = rel_pt(-14.0, 4.0);
    let rays: Vec<_> = p
        .strokes
        .iter()
        .filter(|s| s.pts.first() == Some(&first))
        .collect();
    assert_eq!(rays.len(), 4, "three rays taken and the cursor's");
    assert_eq!(p.strokes.len(), 4);
    assert_eq!(
        p.labels.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["1", "2", "3"],
        "the rays are numbered as the log numbers them"
    );
    // Enter ends: no total is said, nothing is written.
    let before = b.log.len();
    let revision = b.doc.revision();
    b.confirm();
    assert!(said(&b, before).is_empty(), "{:?}", said(&b, before));
    assert_eq!(b.doc.revision(), revision);
    assert!(
        b.session.is_running() && b.points() == 0,
        "the tool waits for the next run"
    );
    // The switch is remembered from run to run.
    assert!(
        b.session
            .prompt()
            .text()
            .ends_with("[Sabit ilk nokta (S): açık]")
    );
}

fn rel_pt(de: f64, dn: f64) -> kentos_interaction::Vec2 {
    kentos_interaction::Vec2::new(E + de, N + dn)
}

#[test]
fn a_fixed_first_point_takes_typed_points_from_the_first_and_has_no_arcs() {
    let mut b = bench("measure");
    b.type_text("S");
    b.click(-14.0, 4.0);
    // Yay and Uzunluk do not apply to rays.
    assert!(!b.type_text("y") && !b.type_text("U"));
    // A relative point is relative to the first point, twice over.
    let before = b.log.len();
    assert!(b.type_text("@3,4"));
    assert!(b.type_text("@3,4"));
    assert_eq!(
        said(&b, before),
        ["1: 5.000 m, semt 40.9666 g", "2: 5.000 m, semt 40.9666 g"]
    );
    // Geri takes the last ray back.
    assert!(b.type_text("g"));
    assert_eq!(b.points(), 2);
    // S turns it off and starts the run over: a chain again.
    assert!(b.type_text("s"));
    assert!(!b.memory.measure_fixed);
    assert_eq!(b.points(), 0);
    assert_eq!(b.options(), ["S"]);
    b.click(-14.0, 4.0);
    assert_eq!(
        b.options(),
        ["Y", "U", "G"],
        "the chain's own options, none for the switch"
    );
}

#[test]
fn a_chain_is_measured_as_it_always_was_and_the_switch_is_not_offered_mid_run() {
    let mut b = bench("measure");
    clicks(&mut b, &[[-14.0, 4.0], [-14.0, 10.0]]);
    assert_eq!(b.options(), ["Y", "U", "G", "Enter"]);
    assert!(!b.type_text("s"), "not offered now: it would drop the run");
    b.click(-8.0, 10.0);
    b.confirm();
    assert_eq!(b.last_text(), Some("Toplam uzunluk 12.000 m (2 kenar)"));
}

// ── Alan hesapla: İçine tıkla and Alan olarak çiz ───────────────────────────

#[test]
fn a_click_inside_a_region_measures_it_and_nothing_is_written() {
    let mut b = bench("area");
    assert!(b.type_text("i"));
    assert!(b.memory.area_inside);
    assert_eq!(
        b.session.prompt().text(),
        "Alan hesapla: alanı ölçülecek bölgenin içine tıklayın [İçine tıkla (I): açık]"
    );
    // The four lines close a 10 × 10 m square: area and perimeter, as a ring drawn.
    b.move_to(-23.0, -10.0);
    let p = preview(&b);
    assert_eq!(p.areas.len(), 1, "the region under the cursor is filled");
    assert_eq!(p.tag.expect("a tag").lines, ["100.00 m²"]);
    let before = b.log.len();
    b.click(-23.0, -10.0);
    assert_eq!(said(&b, before), ["Alan 100.00 m²   Çevre 40.000 m"]);
    assert_eq!(b.last_level(), Some(Level::Success));
    assert!(!b.doc.can_undo(), "a measurement writes nothing");
    assert_eq!(b.points(), 0, "no corner is taken");
    // Where no closed region is, the tool says so.
    let before = b.log.len();
    b.click(-40.0, 0.0);
    assert_eq!(
        said(&b, before),
        ["Tıklanan noktayı çevreleyen kapalı bölge yok."]
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
    // İçine tıkla off again: corners once more, and the chip is gone with it.
    assert!(b.type_text("I"));
    assert!(!b.memory.area_inside);
    b.click(-14.0, 4.0);
    assert_eq!(b.points(), 1);
}

#[test]
fn islands_are_holes_and_count_in_the_perimeter() {
    let mut b = bench("area");
    b.type_text("i");
    // Parcel 8 with its 4 × 4 m hole: 100 − 16 m², 40 + 16 m round.
    let before = b.log.len();
    b.click(19.0, -10.0);
    assert_eq!(said(&b, before), ["Alan 84.00 m²   Çevre 56.000 m"]);
    // In the hole itself: its own region.
    let before = b.log.len();
    b.click(23.0, -10.0);
    assert_eq!(said(&b, before), ["Alan 16.00 m²   Çevre 16.000 m"]);
}

#[test]
fn alan_olarak_ciz_writes_the_last_measured_region_in_one_named_step() {
    let mut b = bench("area");
    assert_eq!(b.options(), ["I"], "no chip before a measurement");
    b.type_text("i");
    b.click(19.0, -10.0);
    assert_eq!(
        b.session.prompt().text(),
        "Alan hesapla: alanı ölçülecek bölgenin içine tıklayın [İçine tıkla (I): açık / Alan olarak çiz (A)]"
    );
    b.doc.set_active_layer("cizim");
    let count = b.doc.entities().count();
    let before = b.log.len();
    assert!(b.type_text("a"));
    assert_eq!(said(&b, before), ["Alan olarak çizildi: 84.00 m²."]);
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.doc.entities().count(), count + 1);
    let Entity::Polygon(p) = b.newest().clone() else {
        panic!("a closed area");
    };
    assert_eq!(p.base.layer_id, "cizim", "the active layer");
    assert_eq!(
        p.holes.as_ref().map(Vec::len),
        Some(1),
        "the island stays a hole"
    );
    let (area, length) = measures(b.newest());
    assert!((area.expect("an area") - 84.0).abs() < 1e-9);
    assert!((length.expect("a perimeter") - 56.0).abs() < 1e-9);
    // One undo step, named after the chip.
    assert_eq!(b.doc.undo().as_deref(), Some("Alan olarak çiz"));
    assert_eq!(b.doc.entities().count(), count);
    // The chip stays until the next measurement: the next one replaces the region.
    assert!(b.session.prompt().keys().contains(&"A"));
    b.click(-23.0, -10.0);
    assert!(b.type_text("a"));
    assert_eq!(b.last_text(), Some("Alan olarak çizildi: 100.00 m²."));
}

#[test]
fn alan_olarak_ciz_is_refused_as_the_polygon_command_refuses() {
    let mut b = bench("area");
    b.type_text("i");
    b.click(-23.0, -10.0);
    b.doc.set_active_layer("kilitli");
    let count = b.doc.entities().count();
    let steps = b.doc.can_undo();
    b.type_text("a");
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.entities().count(), count, "nothing is written");
    assert_eq!(b.doc.can_undo(), steps, "and no step is left behind");
}

#[test]
fn a_ring_drawn_by_its_corners_can_be_drawn_too_until_the_next_starts() {
    let mut b = bench("area");
    clicks(
        &mut b,
        &[
            [-14.0, 4.0],
            [-8.0, 4.0],
            [-8.0, 10.0],
            [-14.0, 10.0],
            [-14.0, 4.0],
        ],
    );
    assert_eq!(b.last_text(), Some("Alan 36.00 m²   Çevre 24.000 m"));
    assert_eq!(b.options(), ["I", "A"]);
    // Shown dashed while it waits.
    assert_eq!(preview(&b).areas.len(), 1);
    // The next ring's first corner ends it.
    b.click(-14.0, 4.0);
    assert!(!b.session.prompt().keys().contains(&"A"));
    assert!(preview(&b).areas.is_empty());
    b.session.exit();
    // A new run starts without one.
    b.start("area");
    assert_eq!(b.options(), ["I"]);
}

// ── Dik ayak ölç ────────────────────────────────────────────────────────────

/// The reference line of the ADR's worked example: A (0, 0) to B (100, 0), east.
fn line(b: &mut Bench) {
    clicks(b, &[[0.0, 0.0], [100.0, 0.0]]);
}

#[test]
fn a_point_is_read_against_the_line_as_the_adr_works_it() {
    let mut b = empty("stationOffset");
    assert_eq!(
        b.session.prompt().text(),
        "Dik ayak ölç: hattın başına tıklayın (A)"
    );
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Dik ayak ölç: hattın sonuna tıklayın (B)"
    );
    b.click(100.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Dik ayak ölç: ölçülecek noktaya tıklayın [Başka hat (H) / Bitir (Enter)]"
    );
    assert_eq!(b.options(), ["H", "Enter"]);
    // P (30, 5): 30 m along, 5 m to the left of A→B.
    b.move_to(30.0, 5.0);
    assert_eq!(tag(&b), ["Ayak 30.000 m", "Boy \u{2212}5.000 m"]);
    let before = b.log.len();
    b.click(30.0, 5.0);
    assert_eq!(
        said(&b, before),
        ["Dik ayak 30.000 m, dik boy 5.000 m (solda)"]
    );
    assert_eq!(b.last_level(), Some(Level::Info));
    // The right of the line is positive.
    b.move_to(70.0, -2.5);
    assert_eq!(tag(&b), ["Ayak 70.000 m", "Boy +2.500 m"]);
    let before = b.log.len();
    b.click(70.0, -2.5);
    assert_eq!(
        said(&b, before),
        ["Dik ayak 70.000 m, dik boy 2.500 m (sağda)"]
    );
    // Behind A the foot is negative; on the line there is no side and no sign.
    let before = b.log.len();
    b.click(-12.4, 3.1);
    assert_eq!(
        said(&b, before),
        ["Dik ayak -12.400 m, dik boy 3.100 m (solda)"]
    );
    b.move_to(50.0, 0.0002);
    assert_eq!(tag(&b), ["Ayak 50.000 m", "Boy 0.000 m"]);
    let before = b.log.len();
    b.click(50.0, 0.0002);
    assert_eq!(
        said(&b, before),
        ["Dik ayak 50.000 m, dik boy 0.000 m (hat üzerinde)"]
    );
    // Nothing is written, however many points are read.
    assert!(!b.doc.can_undo());
    assert!(b.session.is_running());
}

#[test]
fn a_slanted_line_has_its_own_axes() {
    let mut b = empty("stationOffset");
    // A (10, 10) to B (10, 60), north: the right is east.
    clicks(&mut b, &[[10.0, 10.0], [10.0, 60.0]]);
    let before = b.log.len();
    b.click(14.0, 30.0);
    assert_eq!(
        said(&b, before),
        ["Dik ayak 20.000 m, dik boy 4.000 m (sağda)"]
    );
    // A 3-4-5 slope: B is 30 m east and 40 m north of A. P on the far side at 3 m.
    let mut b = empty("stationOffset");
    clicks(&mut b, &[[0.0, 0.0], [30.0, 40.0]]);
    let before = b.log.len();
    // P = A + 25·u + 3·left(u), u = (0.6, 0.8), left(u) = (−0.8, 0.6): (12.6, 21.8).
    b.click(12.6, 21.8);
    assert_eq!(
        said(&b, before),
        ["Dik ayak 25.000 m, dik boy 3.000 m (solda)"]
    );
}

#[test]
fn the_preview_shows_the_line_its_extension_the_foot_and_the_perpendicular() {
    let mut b = empty("stationOffset");
    // While B is asked for: the line as far as the cursor.
    b.click(0.0, 0.0);
    b.move_to(40.0, 0.0);
    let p = preview(&b);
    assert_eq!(p.strokes.len(), 1);
    assert_eq!(
        p.labels.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["A"]
    );
    b.click(100.0, 0.0);
    b.move_to(30.0, 5.0);
    let p = preview(&b);
    // The dashed extension reaches past both ends; the solid line is A–B.
    let dashed: Vec<_> = p.strokes.iter().filter(|s| s.dash.is_some()).collect();
    let solid: Vec<_> = p.strokes.iter().filter(|s| s.dash.is_none()).collect();
    assert_eq!(dashed.len(), 2, "the extension and the perpendicular");
    assert!(dashed[0].pts[0].x < E && dashed[0].pts[1].x > E + 100.0);
    assert_eq!((solid.len(), solid[0].pts.len()), (1, 2));
    // The perpendicular runs from the foot (30, 0) to the cursor (30, 5).
    assert_eq!(dashed[1].pts, [rel_pt(30.0, 0.0), rel_pt(30.0, 5.0)]);
    // The right-angle mark stands at the foot.
    let corner = p
        .markers
        .iter()
        .find(|m| matches!(m.shape, MarkerShape::RightAngle { .. }))
        .expect("a right angle");
    assert_eq!(corner.at, rel_pt(30.0, 0.0));
    assert_eq!(
        p.labels.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["A", "B"]
    );
    // A point already read stays ringed.
    b.click(30.0, 5.0);
    b.move_to(60.0, -4.0);
    let rings = preview(&b)
        .markers
        .iter()
        .filter(|m| matches!(m.shape, MarkerShape::Ring(r) if (r - 3.5).abs() < 1e-6))
        .count();
    assert_eq!(rings, 1);
}

#[test]
fn escape_steps_back_and_enter_ends() {
    let mut b = empty("stationOffset");
    let cancel = |b: &mut Bench| b.run(|s, cx| s.cancel(cx));
    // From B to A.
    b.click(0.0, 0.0);
    assert_eq!(b.points(), 1);
    assert!(cancel(&mut b));
    assert_eq!(b.points(), 0);
    assert_eq!(
        b.session.prompt().text(),
        "Dik ayak ölç: hattın başına tıklayın (A)"
    );
    // From the points to A: a new line.
    line(&mut b);
    b.click(30.0, 5.0);
    assert!(cancel(&mut b));
    assert_eq!(b.points(), 0);
    assert_eq!(
        b.session.prompt().text(),
        "Dik ayak ölç: hattın başına tıklayın (A)"
    );
    // From A it leaves.
    assert!(!cancel(&mut b));
    assert!(!b.session.is_running());
    // Enter ends; so does a quick right click, which is the same confirm.
    b.start("stationOffset");
    line(&mut b);
    b.confirm();
    assert!(!b.session.is_running());
}

#[test]
fn another_line_asks_for_a_new_start() {
    let mut b = empty("stationOffset");
    line(&mut b);
    b.click(30.0, 5.0);
    assert!(b.type_text("h"), "Başka hat");
    assert_eq!(b.points(), 0);
    // A new line: this one runs north from (0, 0).
    clicks(&mut b, &[[0.0, 0.0], [0.0, 50.0]]);
    let before = b.log.len();
    b.click(4.0, 20.0);
    assert_eq!(
        said(&b, before),
        ["Dik ayak 20.000 m, dik boy 4.000 m (sağda)"]
    );
}

#[test]
fn a_second_point_on_the_first_is_no_line() {
    let mut b = empty("stationOffset");
    b.click(5.0, 5.0);
    b.click(5.0, 5.0);
    assert_eq!(b.points(), 1, "B was not taken");
    assert_eq!(b.last_level(), Some(Level::Warn));
    // Ctrl+Z steps back as Esc does; at A the drawing's undo is the user's.
    assert!(b.undo_step());
    assert!(!b.undo_step());
}

#[test]
fn snaps_apply_and_typed_points_are_taken() {
    // The corners of parcel 1 are the line's ends.
    let mut b = Bench::on(AREAS);
    b.start("stationOffset");
    b.draft.snap = true;
    clicks(&mut b, &[[0.2, 0.1], [9.9, -0.1]]);
    // A (0, 0), B (10, 0): the click on the corner of parcel 2, 4 m along, 4 m up.
    let before = b.log.len();
    b.click(6.05, 3.95);
    assert_eq!(
        said(&b, before),
        ["Dik ayak 6.000 m, dik boy 4.000 m (solda)"]
    );
    // A typed relative point is read too.
    let before = b.log.len();
    assert!(b.type_text("@-3,2"));
    assert_eq!(said(&b, before).len(), 1);
    assert!(!b.doc.can_undo());
}

#[test]
fn the_tool_is_in_the_session() {
    assert!(kentos_interaction::Session::tools().contains(&"stationOffset"));
}
