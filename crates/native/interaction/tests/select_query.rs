//! Çitle seç, Daireyle seç and İçeren alanı seç through the session, over the
//! native document (docs/adr/0141). The drawing (fixtures/interaction/v1/areas.kcad):
//!
//! | object | what | where (east, north from the origin) |
//! |---|---|---|
//! | 1 | parcel | 0…10 × 0…10 |
//! | 2 | parcel | 6…16 × 4…14, over the first |
//! | 3 | circle, radius 3 | centre (−20, 8) |
//! | 4–7 | four lines | the square −28…−18 × −15…−5 |
//! | 8 | parcel with a 4 × 4 m hole | 18…28 × −15…−5, the hole 21…25 × −12…−8 |
//! | 9 | parcel on the locked layer | −10…−4 × −18…−12 |
//! | 10 | line | (−2, 2) to (12, 2) |
//!
//! Expected values are worked out by hand.

mod common;

use common::Bench;
use kentos_interaction::{Format, Level, Tone};

const AREAS: &str = include_str!("../../../../fixtures/interaction/v1/areas.kcad");

fn bench(tool: &str) -> Bench {
    let mut b = Bench::on(AREAS);
    b.draft.snap = false;
    b.start(tool);
    b
}

fn clicks(b: &mut Bench, pts: &[[f64; 2]]) {
    for &[de, dn] in pts {
        b.click(de, dn);
    }
}

fn select(b: &mut Bench, slots: &[u32]) {
    b.selection.set(
        slots
            .iter()
            .map(|s| kentos_domain::Slot(*s))
            .collect::<Vec<_>>(),
    );
}

fn rel_pt(de: f64, dn: f64) -> kentos_interaction::Vec2 {
    kentos_interaction::Vec2::new(common::E + de, common::N + dn)
}

fn cancel(b: &mut Bench) -> bool {
    b.run(|s, cx| s.cancel(cx))
}

fn preview(b: &Bench) -> kentos_interaction::Preview {
    b.session.preview(&Format::default()).expect("a tool runs")
}

/// Whether the tool has gone back to Seç.
fn left(b: &Bench) -> bool {
    !b.session.is_running()
}

// ── Çitle seç ───────────────────────────────────────────────────────────────

#[test]
fn a_fence_selects_what_it_crosses_and_the_tool_goes_back_to_select() {
    let mut b = bench("selectFence");
    assert_eq!(
        b.session.prompt().text(),
        "Çitle seç: çitin ilk noktasına tıklayın"
    );
    assert!(b.options().is_empty());
    b.click(-5.0, 8.0);
    assert_eq!(
        b.session.prompt().text(),
        "Çitle seç: sonraki noktaya tıklayın [Geri (G) / Bitir (Enter)]"
    );
    assert_eq!(b.options(), ["G", "Enter"]);
    b.click(20.0, 8.0);
    // Along y = 8 the fence meets the sides of both parcels, and nothing else.
    b.confirm();
    assert_eq!(b.selected(), [1, 2]);
    assert_eq!(b.last_text(), Some("Çit 2 nesneyi kesti; seçildi."));
    assert_eq!(b.last_level(), Some(Level::Info));
    assert!(left(&b));
    assert!(!b.doc.can_undo(), "a selection writes nothing");
}

#[test]
fn a_fence_that_crosses_nothing_leaves_the_selection_as_it_was() {
    let mut b = bench("selectFence");
    select(&mut b, &[3]);
    clicks(&mut b, &[[40.0, 40.0], [50.0, 40.0]]);
    b.confirm();
    assert_eq!(b.last_text(), Some("Çit hiçbir nesneyi kesmedi."));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.selected(), [3]);
    assert!(left(&b));
}

#[test]
fn shift_adds_what_the_fence_crosses_to_the_selection() {
    let mut b = bench("selectFence");
    select(&mut b, &[3]);
    clicks(&mut b, &[[-5.0, 8.0], [20.0, 8.0]]);
    b.shift = true;
    b.confirm();
    assert_eq!(b.selected(), [3, 1, 2]);
    assert_eq!(b.last_text(), Some("Çit 2 nesneyi kesti; seçildi."));
}

/// A right click with Shift opens the snap menu: the fence takes the Shift of the last pointer event.
#[test]
fn the_fence_adds_when_shift_was_held_at_the_last_click_or_is_at_enter() {
    let mut b = bench("selectFence");
    select(&mut b, &[3]);
    b.click(-5.0, 8.0);
    b.shift = true;
    b.click(20.0, 8.0);
    // Shift is let go before the fence ends: the last pointer event had it.
    b.shift = false;
    b.confirm();
    assert_eq!(b.selected(), [3, 1, 2]);
    // The pointer moved after Shift was let go: the fence replaces.
    let mut b = bench("selectFence");
    select(&mut b, &[3]);
    b.click(-5.0, 8.0);
    b.shift = true;
    b.click(20.0, 8.0);
    b.shift = false;
    b.move_to(21.0, 8.0);
    b.confirm();
    assert_eq!(b.selected(), [1, 2]);
    // Shift with Enter alone adds.
    let mut b = bench("selectFence");
    select(&mut b, &[3]);
    clicks(&mut b, &[[-5.0, 8.0], [20.0, 8.0]]);
    b.shift = true;
    b.confirm();
    assert_eq!(b.selected(), [3, 1, 2]);
}

#[test]
fn geri_drops_the_last_point_of_the_fence() {
    let mut b = bench("selectFence");
    // Down from (20, 8) the fence would also cross parcel 8 at its top, y = −5.
    clicks(&mut b, &[[-5.0, 8.0], [20.0, 8.0], [20.0, -10.0]]);
    assert_eq!(b.points(), 3);
    assert!(b.type_text("g"));
    assert_eq!(b.points(), 2);
    b.confirm();
    assert_eq!(b.selected(), [1, 2]);
    // The same fence with the third point.
    let mut b = bench("selectFence");
    clicks(&mut b, &[[-5.0, 8.0], [20.0, 8.0], [20.0, -10.0]]);
    b.confirm();
    assert_eq!(b.selected(), [1, 2, 8]);
    assert_eq!(b.last_text(), Some("Çit 3 nesneyi kesti; seçildi."));
    // Ctrl+Z takes a point back too.
    let mut b = bench("selectFence");
    clicks(&mut b, &[[0.0, 0.0], [1.0, 1.0]]);
    assert!(b.undo_step());
    assert_eq!(b.points(), 1);
    assert!(b.undo_step());
    assert!(!b.undo_step(), "the drawing's undo is then the user's");
}

#[test]
fn only_visible_objects_are_crossed_and_locked_ones_are() {
    // The locked layer's parcel is crossed, as a window takes it.
    let mut b = bench("selectFence");
    clicks(&mut b, &[[-12.0, -15.0], [-2.0, -15.0]]);
    b.confirm();
    assert_eq!(b.selected(), [9]);
    // Parcels on a hidden layer are not.
    let mut b = bench("selectFence");
    b.doc.toggle_layer_visible("parsel");
    clicks(&mut b, &[[-5.0, 8.0], [20.0, 8.0]]);
    b.confirm();
    assert!(b.selected().is_empty());
    assert_eq!(b.last_text(), Some("Çit hiçbir nesneyi kesmedi."));
}

#[test]
fn enter_ends_the_fence_with_or_without_points_and_escape_steps_back() {
    // No point at all: it leaves without a word.
    let mut b = bench("selectFence");
    let before = b.log.len();
    b.confirm();
    assert!(left(&b));
    assert_eq!(b.log.len(), before);
    // One point is no fence: Enter says so and the tool waits for the second.
    let mut b = bench("selectFence");
    b.click(5.0, 5.0);
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Çit için en az iki nokta gerekir; ikinci noktayı gösterin.")
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(b.session.is_running());
    assert_eq!(b.points(), 1);
    b.click(20.0, 5.0);
    b.confirm();
    assert_eq!(b.selected(), [1, 2]);
    assert!(left(&b));
    // Esc drops the fence drawn; with none it leaves.
    let mut b = bench("selectFence");
    clicks(&mut b, &[[0.0, 0.0], [5.0, 5.0]]);
    assert!(cancel(&mut b));
    assert_eq!(b.points(), 0);
    assert_eq!(
        b.session.prompt().text(),
        "Çitle seç: çitin ilk noktasına tıklayın"
    );
    assert!(!cancel(&mut b));
    assert!(left(&b));
}

#[test]
fn the_fence_is_a_dashed_path_to_the_cursor_and_takes_typed_points() {
    let mut b = bench("selectFence");
    clicks(&mut b, &[[0.0, 0.0], [10.0, 0.0]]);
    b.move_to(10.0, 6.0);
    let p = preview(&b);
    assert_eq!(p.strokes.len(), 1);
    assert!(p.strokes[0].dash.is_some() && !p.strokes[0].closed);
    assert_eq!(p.strokes[0].pts.len(), 3, "the two points and the cursor");
    assert_eq!(p.markers.len(), 2);
    // A typed point, relative to the last, joins it.
    assert!(b.type_text("@0,6"));
    assert_eq!(b.points(), 3);
    // With no point yet there is nothing to draw.
    let b = bench("selectFence");
    assert!(preview(&b).strokes.is_empty());
}

// ── Daireyle seç ────────────────────────────────────────────────────────────

#[test]
fn a_circle_selects_what_is_wholly_inside_it() {
    let mut b = bench("selectCircle");
    assert_eq!(
        b.session.prompt().text(),
        "Daireyle seç: dairenin merkezine tıklayın [Kesişen (K)]"
    );
    b.click(5.0, 5.0);
    assert_eq!(
        b.session.prompt().text(),
        "Daireyle seç: yarıçapı gösterin ya da yazın [Kesişen (K)]"
    );
    assert!(b.type_text("8"));
    // Parcel 1 (its corners 7.07 m away) and the line (its ends 7.6 m).
    assert_eq!(b.selected(), [1, 10]);
    assert_eq!(b.last_text(), Some("Dairenin içinde 2 nesne; seçildi."));
    assert_eq!(b.last_level(), Some(Level::Info));
    assert!(left(&b));
    assert!(!b.doc.can_undo());
}

#[test]
fn kesisen_also_takes_what_the_circle_touches_and_is_remembered() {
    let mut b = bench("selectCircle");
    assert!(b.type_text("k"));
    assert!(b.memory.circle_crossing);
    assert_eq!(
        b.session.prompt().text(),
        "Daireyle seç: dairenin merkezine tıklayın [Kesişen (K): açık]"
    );
    b.click(5.0, 5.0);
    assert!(b.type_text("8"));
    // Parcel 2 comes too: its left side is 1 m from the centre.
    assert_eq!(b.selected(), [1, 2, 10]);
    assert_eq!(b.last_text(), Some("Daireye dokunan 3 nesne; seçildi."));
    // The switch is the session's: a new run starts with it on, and it turns off again.
    b.start("selectCircle");
    assert!(b.session.prompt().text().ends_with("[Kesişen (K): açık]"));
    assert!(b.type_text("K"));
    assert!(!b.memory.circle_crossing);
    assert!(b.options() == ["K"]);
}

#[test]
fn a_circle_that_holds_nothing_says_so_and_leaves_the_selection() {
    let mut b = bench("selectCircle");
    select(&mut b, &[3]);
    b.click(40.0, 40.0);
    assert!(b.type_text("2"));
    assert_eq!(b.last_text(), Some("Dairede nesne yok."));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.selected(), [3]);
    assert!(left(&b));
}

#[test]
fn the_radius_is_shown_by_a_click_or_a_typed_point_and_shift_adds() {
    let mut b = bench("selectCircle");
    clicks(&mut b, &[[5.0, 5.0], [13.0, 5.0]]);
    assert_eq!(b.selected(), [1, 10]);
    assert!(left(&b));
    // A typed point on the circle: (13, 5) as a full coordinate, east first.
    let mut b = bench("selectCircle");
    b.click(5.0, 5.0);
    assert!(b.type_text("487013,4420005"));
    assert_eq!(b.selected(), [1, 10]);
    // Shift held for the click that shows the radius: joins the selection.
    let mut b = bench("selectCircle");
    select(&mut b, &[3]);
    b.click(5.0, 5.0);
    b.shift = true;
    b.click(13.0, 5.0);
    assert_eq!(b.selected(), [3, 1, 10]);
    // A typed radius takes the Shift of the last click, the centre's.
    let mut b = bench("selectCircle");
    select(&mut b, &[3]);
    b.shift = true;
    b.click(5.0, 5.0);
    b.shift = false;
    assert!(b.type_text("8"));
    assert_eq!(b.selected(), [3, 1, 10]);
}

#[test]
fn a_radius_of_zero_is_refused_and_the_tool_waits() {
    let mut b = bench("selectCircle");
    b.click(5.0, 5.0);
    assert!(b.type_text("0"));
    assert_eq!(b.last_text(), Some("Yarıçap sıfırdan büyük olmalı."));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(b.session.is_running());
    assert_eq!(b.points(), 1);
    // A click on the centre is the same refusal.
    b.click(5.0, 5.0);
    assert!(b.session.is_running());
}

#[test]
fn escape_goes_back_to_the_centre_and_then_leaves() {
    let mut b = bench("selectCircle");
    b.click(5.0, 5.0);
    assert!(cancel(&mut b));
    assert_eq!(b.points(), 0);
    assert!(b.session.prompt().text().contains("merkezine"));
    assert!(!cancel(&mut b));
    assert!(left(&b));
    // Enter leaves too.
    b.start("selectCircle");
    b.confirm();
    assert!(left(&b));
}

#[test]
fn the_circle_is_dashed_and_filled_with_its_radius_beside_the_cursor() {
    let mut b = bench("selectCircle");
    b.click(5.0, 5.0);
    b.move_to(13.0, 5.0);
    let p = preview(&b);
    assert_eq!(p.tag.expect("a tag").lines, ["R 8.000 m"]);
    // The circle is a dashed ring of many points, lightly filled; the radius a short dashed line.
    let ring = p
        .strokes
        .iter()
        .find(|s| s.dash.is_some() && s.pts.len() > 16)
        .expect("the circle");
    assert_eq!(ring.tone, Tone::Accent, "the window's colour");
    assert_eq!(p.areas.len(), 1);
    assert_eq!(p.areas[0].rings[0], ring.pts);
    assert!(p.strokes.iter().any(|s| s.pts.len() == 2));
    assert!(preview(&bench("selectCircle")).strokes.is_empty());
    // Kesişen: the snap colour, as the crossing box is.
    let mut b = bench("selectCircle");
    b.type_text("k");
    b.click(5.0, 5.0);
    b.move_to(13.0, 5.0);
    let p = preview(&b);
    assert_eq!(p.areas[0].fill_tone, Tone::Snap);
    assert!(
        p.strokes
            .iter()
            .any(|s| s.pts.len() > 16 && s.tone == Tone::Snap)
    );
}

#[test]
fn ortho_and_polar_tracking_do_not_bend_a_radius() {
    let mut b = bench("selectCircle");
    b.draft.ortho = true;
    b.click(5.0, 5.0);
    // Ortho would pull (12, 9) to the horizontal, 7 m away: a radius is a distance, 8.062 m.
    b.move_to(12.0, 9.0);
    assert_eq!(preview(&b).tag.expect("a tag").lines, ["R 8.062 m"]);
}

#[test]
fn a_circle_takes_only_visible_objects() {
    let mut b = bench("selectCircle");
    b.doc.toggle_layer_visible("parsel");
    b.type_text("k");
    b.click(5.0, 5.0);
    b.type_text("8");
    // The parcels are hidden: only the line of the Çizim layer is left.
    assert_eq!(b.selected(), [10]);
}

// ── İçeren alanı seç ────────────────────────────────────────────────────────

#[test]
fn a_click_takes_the_smallest_area_and_the_same_place_goes_up_one() {
    let mut b = bench("selectContaining");
    assert_eq!(
        b.session.prompt().text(),
        "İçeren alanı seç: alanın içine tıklayın"
    );
    // Parcels 1 and 2 both hold (8, 6), 100 m² each: the drawing's order.
    b.click(8.0, 6.0);
    assert_eq!(b.selected(), [1]);
    assert_eq!(b.last_text(), Some("Alan seçildi (1/2, 100.00 m²)."));
    assert_eq!(b.last_level(), Some(Level::Info));
    // Within the pick tolerance (0.625 m here) it is the same place.
    b.click(8.3, 6.0);
    assert_eq!(b.selected(), [2]);
    assert_eq!(b.last_text(), Some("Alan seçildi (2/2, 100.00 m²)."));
    // After the last it wraps to the smallest.
    b.click(8.0, 6.0);
    assert_eq!(b.selected(), [1]);
    assert_eq!(b.last_text(), Some("Alan seçildi (1/2, 100.00 m²)."));
    // Elsewhere starts again at the smallest, whichever place the last click was at.
    b.click(8.3, 6.0);
    b.click(2.0, 2.0);
    assert_eq!(b.selected(), [1]);
    assert_eq!(b.last_text(), Some("Alan seçildi (1/1, 100.00 m²)."));
    // The tool stays for more.
    assert!(b.session.is_running());
    assert!(!b.doc.can_undo());
}

#[test]
fn three_levels_go_up_from_the_parcel_to_the_block_to_the_district() {
    let mut b = Bench::new("selectContaining");
    // A parcel of 10 m, a block of 40 m and a district of 100 m, one around the other.
    let square = |lo: f64, hi: f64| [[lo, lo], [hi, lo], [hi, hi], [lo, hi]];
    let district = b.add_path("cizim", &square(-50.0, 50.0), true);
    let parcel = b.add_path("cizim", &square(0.0, 10.0), true);
    let block = b.add_path("cizim", &square(-10.0, 30.0), true);
    let said = |b: &Bench| b.last_text().map(str::to_owned);
    b.click(5.0, 5.0);
    assert_eq!(b.selected(), [parcel.0]);
    assert_eq!(said(&b).as_deref(), Some("Alan seçildi (1/3, 100.00 m²)."));
    b.click(5.0, 5.0);
    assert_eq!(b.selected(), [block.0]);
    assert_eq!(said(&b).as_deref(), Some("Alan seçildi (2/3, 1600.00 m²)."));
    // The label beside the cursor tells where it stands.
    assert_eq!(preview(&b).tag.expect("a tag").lines, ["2/3 · 1600.00 m²"]);
    b.click(5.0, 5.0);
    assert_eq!(b.selected(), [district.0]);
    assert_eq!(
        said(&b).as_deref(),
        Some("Alan seçildi (3/3, 10000.00 m²).")
    );
    b.click(5.0, 5.0);
    assert_eq!(b.selected(), [parcel.0], "after the last, the smallest");
}

#[test]
fn where_no_closed_object_is_it_says_so() {
    let mut b = bench("selectContaining");
    select(&mut b, &[3]);
    b.click(40.0, 40.0);
    assert_eq!(
        b.last_text(),
        Some("Tıklanan noktayı içeren kapalı alan yok.")
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.selected(), [3], "the selection is left as it was");
    // A hole is outside its parcel: the point in it is held by nothing.
    b.click(23.0, -10.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    // In the ring around it: the parcel, its net area.
    b.click(19.0, -10.0);
    assert_eq!(b.selected(), [8]);
    assert_eq!(b.last_text(), Some("Alan seçildi (1/1, 84.00 m²)."));
    // A circle is a closed object too: π · 3².
    b.click(-20.0, 8.0);
    assert_eq!(b.selected(), [3]);
    assert_eq!(b.last_text(), Some("Alan seçildi (1/1, 28.27 m²)."));
}

#[test]
fn a_typed_point_is_a_click_there() {
    let mut b = bench("selectContaining");
    // (2, 2) as a full coordinate, east first.
    assert!(b.type_text("487002,4420002"));
    assert_eq!(b.selected(), [1]);
    assert_eq!(b.last_text(), Some("Alan seçildi (1/1, 100.00 m²)."));
    assert!(!b.type_text("abc"));
}

#[test]
fn shift_adds_the_area_to_the_selection() {
    let mut b = bench("selectContaining");
    select(&mut b, &[3]);
    b.shift = true;
    b.click(2.0, 2.0);
    assert_eq!(b.selected(), [3, 1]);
}

#[test]
fn nothing_is_outlined_and_the_label_stays_at_the_last_click() {
    let mut b = bench("selectContaining");
    // Nothing is outlined as the cursor moves, and there is no label before a click.
    b.move_to(2.0, 2.0);
    assert_eq!(b.selection.hover(), None);
    assert!(preview(&b).tag.is_none());
    b.click(8.0, 6.0);
    let stays = |b: &Bench| {
        let tag = preview(b).tag.expect("a tag");
        (tag.at, tag.lines)
    };
    let label = (rel_pt(8.0, 6.0), vec!["1/2 · 100.00 m²".to_owned()]);
    assert_eq!(stays(&b), label);
    // The cursor goes elsewhere: the label does not follow it.
    b.move_to(40.0, 40.0);
    assert_eq!(stays(&b), label);
    assert_eq!(b.selection.hover(), None);
    // A click where nothing closed is changes nothing: the label and the cycle stay.
    b.click(40.0, 40.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(stays(&b), label);
    b.click(8.3, 6.0);
    assert_eq!(b.last_text(), Some("Alan seçildi (2/2, 100.00 m²)."));
    assert_eq!(stays(&b).1, ["2/2 · 100.00 m²"]);
}

#[test]
fn escape_and_enter_end_the_tool() {
    // Esc.
    let mut b = bench("selectContaining");
    b.click(2.0, 2.0);
    assert!(!cancel(&mut b), "there is nothing to step back to");
    assert!(left(&b));
    assert_eq!(b.selected(), [1], "the selection stays");
    // Enter, or a quick right click, which is the same confirm.
    let mut b = bench("selectContaining");
    b.click(2.0, 2.0);
    b.confirm();
    assert!(left(&b));
    assert_eq!(b.selected(), [1]);
    // Ctrl+Z is the drawing's.
    let mut b = bench("selectContaining");
    assert!(!b.undo_step());
}

#[test]
fn the_three_tools_are_in_the_session() {
    for id in ["selectFence", "selectCircle", "selectContaining"] {
        assert!(kentos_interaction::Session::tools().contains(&id), "{id}");
    }
}
