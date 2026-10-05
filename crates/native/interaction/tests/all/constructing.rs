//! Daire dilimi, Ara nokta, Kesişim noktası, Açı ölç and Koordinat oku through
//! the session, over the native document (docs/adr/0140, phase 2): what each
//! asks for, what its preview draws, what it writes and in which undo step,
//! the values it keeps, nothing to do said plainly, Esc and the methods.
//! Points are east and north differences from (E, N). Expected values are
//! worked out by hand.

use crate::common;

use common::{Bench, E, N, rel};
use kentos_contracts::{Entity, EntityBase, PointEntity, Vec2 as Wire};
use kentos_interaction::{Level, MarkerShape, Tone};

const EMPTY: &str = include_str!("../../../../../fixtures/interaction/v1/empty.kcad");

fn near(have: f64, want: f64) -> bool {
    (have - want).abs() < 1e-9
}

fn near2(have: [f64; 2], want: [f64; 2]) -> bool {
    near(have[0], want[0]) && near(have[1], want[1])
}

fn preview(b: &Bench) -> kentos_interaction::Preview {
    b.session.preview(&Default::default()).expect("a preview")
}

/// The drawing with degrees as its angle unit.
fn degrees() -> Bench {
    let mut b = Bench::on(&EMPTY.replace("\"angleUnit\":\"grad\"", "\"angleUnit\":\"deg\""));
    b.draft.snap = false;
    b
}

/// The newest object, a point, as east and north differences.
fn newest_point(b: &Bench) -> [f64; 2] {
    let Entity::Point(p) = b.newest() else {
        panic!("a point, not {:?}", b.newest());
    };
    rel(p.p)
}

/// The points on the drawing, as east and north differences, in order.
fn all_points(b: &Bench) -> Vec<[f64; 2]> {
    b.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Point(p) => Some(rel(p.p)),
            _ => None,
        })
        .collect()
}

// ── Daire dilimi ────────────────────────────────────────────────────────────

/// The newest object, a closed area: its ring, relative, and its bulges.
fn slice(b: &Bench) -> (Vec<[f64; 2]>, Vec<f64>) {
    let Entity::Polygon(p) = b.newest() else {
        panic!("a closed area, not {:?}", b.newest());
    };
    (
        p.pts.iter().map(|q| rel(*q)).collect(),
        p.bulges.clone().unwrap_or_default(),
    )
}

#[test]
fn a_slice_takes_the_centre_the_start_and_the_end_direction() {
    let mut b = Bench::new("sector");
    assert_eq!(
        b.session.prompt().text(),
        "Daire dilimi: dilimin merkezini belirtin"
    );
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Daire dilimi: başlangıç noktasını gösterin ya da yarıçapı yazın"
    );
    // The start point gives the radius and the start angle at once.
    b.click(10.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Daire dilimi: bitiş doğrultusunu gösterin ya da açısını yazın (grad)"
    );
    // The slice is drawn as it would be written, its numbers by the cursor.
    b.move_to(0.0, 10.0);
    let p = preview(&b);
    assert_eq!(p.areas.len(), 1);
    assert!(p.areas[0].rings[0].len() > 8, "the arc is tessellated");
    assert_eq!(
        p.tag.expect("a tag").lines,
        ["Açı 100.0000 g", "Yarıçap 10.000 m"]
    );
    let before = b.doc.len();
    b.click(0.0, 10.0);
    assert_eq!(b.doc.len(), before + 1);
    let (ring, bulges) = slice(&b);
    // The centre, the arc's start and end; the arc is the middle edge.
    assert!(
        near2(ring[0], [0.0, 0.0]) && near2(ring[1], [10.0, 0.0]) && near2(ring[2], [0.0, 10.0])
    );
    assert_eq!(bulges.len(), 3);
    assert!(near(bulges[0], 0.0) && near(bulges[2], 0.0));
    // A quarter turn: the bulge is tan(π / 16 · 2) = tan(π / 8).
    assert!(
        near(bulges[1], (std::f64::consts::PI / 8.0).tan()),
        "{bulges:?}"
    );
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(
        b.last_text(),
        Some("Daire dilimi eklendi: r = 10.000 m, açı 100.0000 g")
    );
    // It asks for the next centre; the write is one undo step.
    assert_eq!(
        b.session.prompt().text(),
        "Daire dilimi: dilimin merkezini belirtin"
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
    assert_eq!(b.doc.len(), before);
}

#[test]
fn a_typed_radius_and_typed_angles_are_in_the_project_s_unit() {
    let mut b = Bench::new("sector");
    b.click(2.0, 3.0);
    assert!(b.type_text("8"));
    assert_eq!(
        b.session.prompt().text(),
        "Daire dilimi: başlangıç açısını yazın (grad, doğudan) ya da yönünü gösterin"
    );
    // 100 grads is north; 200 grads is west: a quarter turn counter-clockwise.
    assert!(b.type_text("100"));
    b.move_to(-9.0, 3.0);
    assert!(b.type_text("200"));
    let (ring, _) = slice(&b);
    assert!(near2(ring[0], [2.0, 3.0]));
    assert!(near2(ring[1], [2.0, 11.0]), "{ring:?}");
    assert!(near2(ring[2], [-6.0, 3.0]), "{ring:?}");
    assert_eq!(
        b.last_text(),
        Some("Daire dilimi eklendi: r = 8.000 m, açı 100.0000 g")
    );
}

#[test]
fn the_angles_are_degrees_when_the_project_says_so() {
    let mut b = degrees();
    b.start("sector");
    b.click(0.0, 0.0);
    assert!(b.type_text("5"));
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Daire dilimi: başlangıç açısını yazın (derece,")
    );
    assert!(b.type_text("90"));
    assert!(b.type_text("180"));
    let (ring, _) = slice(&b);
    assert!(
        near2(ring[1], [0.0, 5.0]) && near2(ring[2], [-5.0, 0.0]),
        "{ring:?}"
    );
    assert_eq!(
        b.last_text(),
        Some("Daire dilimi eklendi: r = 5.000 m, açı 90.0000°")
    );
}

#[test]
fn a_slice_may_cross_east() {
    let mut b = Bench::new("sector");
    b.click(0.0, 0.0);
    b.click(4.0, -4.0);
    // From 7/8 of a turn (350 grads) round to 50 grads: 100 grads across east.
    b.click(4.0, 4.0);
    let (ring, bulges) = slice(&b);
    assert!(near2(ring[2], [4.0, 4.0]) && bulges.len() == 3);
    assert_eq!(
        b.last_text(),
        Some("Daire dilimi eklendi: r = 5.657 m, açı 100.0000 g")
    );
}

#[test]
fn a_slice_with_no_sweep_is_refused_and_the_tool_waits() {
    let mut b = Bench::new("sector");
    b.click(0.0, 0.0);
    b.click(6.0, 0.0);
    let before = b.doc.len();
    // The end in the start's own direction, and a whole turn: no slice.
    b.click(9.0, 0.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .expect("said")
            .starts_with("Bitiş doğrultusu başlangıçla aynı")
    );
    assert!(b.type_text("400"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.len(), before);
    assert_eq!(b.points(), 1, "the centre is still there");
    assert!(b.session.prompt().text().contains("bitiş doğrultusunu"));
    // A point on the centre is no start.
    let mut b = Bench::new("sector");
    b.click(1.0, 1.0);
    b.click(1.0, 1.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(b.session.prompt().text().contains("başlangıç noktasını"));
}

#[test]
fn esc_steps_back_a_stage_at_a_time_and_leaves_last() {
    let mut b = Bench::new("sector");
    b.click(0.0, 0.0);
    b.click(6.0, 0.0);
    assert!(b.session.prompt().text().contains("bitiş"));
    assert!(b.run(|s, cx| s.cancel(cx)), "the start is let go");
    assert!(b.session.prompt().text().contains("başlangıç noktasını"));
    assert!(b.run(|s, cx| s.cancel(cx)), "the centre is let go");
    assert!(b.session.prompt().text().contains("merkezini"));
    assert!(
        !b.run(|s, cx| s.cancel(cx)),
        "nothing left: the tool leaves"
    );
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.len(), 0);
}

// ── Ara nokta ───────────────────────────────────────────────────────────────

#[test]
fn equal_parts_show_their_points_and_a_typed_count_writes_them() {
    let mut b = Bench::new("pointsBetween");
    assert_eq!(
        b.session.prompt().text(),
        "Ara nokta: ilk noktayı belirtin [4 parça; Uzaklıkla (U) / Oranla (O)]"
    );
    b.click(0.0, 0.0);
    b.click(12.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ara nokta: parça sayısını yazın (Enter: 4 parça) [Uzaklıkla (U) / Oranla (O)]"
    );
    b.move_to(6.0, 4.0);
    let p = preview(&b);
    // Two rings on the ends, a circle at each of the three points to come.
    assert_eq!(
        p.markers
            .iter()
            .filter(|m| matches!(m.shape, MarkerShape::Ring(7.5)))
            .count(),
        2
    );
    assert_eq!(
        p.markers
            .iter()
            .filter(|m| matches!(m.shape, MarkerShape::Ring(4.5)))
            .count(),
        3
    );
    assert_eq!(p.tag.expect("a tag").lines, ["3 nokta", "aralık 3.000 m"]);
    assert_eq!(b.doc.len(), 0, "nothing is written yet");
    assert!(b.type_text("6"));
    assert_eq!(all_points(&b).len(), 5);
    for (p, x) in all_points(&b).into_iter().zip([2.0, 4.0, 6.0, 8.0, 10.0]) {
        assert!(near2(p, [x, 0.0]), "{p:?} at {x}");
    }
    assert_eq!(b.last_text(), Some("5 nokta kondu."));
    assert_eq!(b.last_level(), Some(Level::Success));
    // One undo step for all of them; the count is kept.
    assert_eq!(b.doc.undo().as_deref(), Some("Ara nokta"));
    assert_eq!(b.doc.len(), 0);
    assert_eq!(b.memory.between_parts, 6);
    // It asks for two more points; Enter now writes at the kept count.
    assert_eq!(b.points(), 0);
    b.click(0.0, 0.0);
    b.click(12.0, 0.0);
    assert!(b.session.prompt().text().contains("(Enter: 6 parça)"));
    b.confirm();
    assert_eq!(all_points(&b).len(), 5);
}

#[test]
fn distances_and_ratios_are_typed_as_lists_and_kept() {
    let mut b = Bench::new("pointsBetween");
    assert!(b.type_text("U"));
    assert_eq!(
        b.session.prompt().text(),
        "Ara nokta: ilk noktayı belirtin [uzaklık yok; Eşit aralık (E) / Oranla (O)]"
    );
    b.click(0.0, 0.0);
    b.click(0.0, 10.0);
    assert_eq!(
        b.session.prompt().text(),
        "Ara nokta: uzaklıkları virgülle yazın (Enter: uzaklık yok) [Eşit aralık (E) / Oranla (O)]"
    );
    // Nothing kept: Enter says so and the points stay.
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.last_text(), Some("Uzaklıkları yazın, ör. 5, 12.5."));
    assert_eq!(b.points(), 2);
    // A distance past the second point is refused.
    assert!(b.type_text("3, 12"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(b.last_text().expect("said").contains("dışında kalıyor"));
    assert_eq!(b.doc.len(), 0);
    assert_eq!(b.points(), 2);
    assert!(b.type_text("2; 3.5 7"));
    let points = all_points(&b);
    assert_eq!(points.len(), 3);
    for (p, y) in points.into_iter().zip([2.0, 3.5, 7.0]) {
        assert!(near2(p, [0.0, y]), "{p:?}");
    }
    // The list is kept: the next pair of points shows it.
    b.click(0.0, 0.0);
    b.click(0.0, 20.0);
    assert!(b.session.prompt().text().contains("(Enter: 2, 3.5, 7 m)"));

    // Oranla: fractions from the first point.
    let mut b = Bench::new("pointsBetween");
    assert!(b.type_text("O"));
    b.click(0.0, 0.0);
    b.click(8.0, 0.0);
    assert!(b.type_text("0.25, 0.5"));
    assert_eq!(all_points(&b).len(), 2);
    assert!(near2(all_points(&b)[0], [2.0, 0.0]) && near2(all_points(&b)[1], [4.0, 0.0]));
    b.click(0.0, 0.0);
    b.click(8.0, 0.0);
    assert!(b.type_text("1.5"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .expect("said")
            .contains("0 ile 1 arasında değil")
    );
    assert_eq!(all_points(&b).len(), 2);
    // E goes back to equal parts.
    assert!(b.type_text("E"));
    assert!(b.session.prompt().text().contains("parça sayısını yazın"));
}

#[test]
fn between_says_what_it_cannot_do() {
    let mut b = Bench::new("pointsBetween");
    b.click(1.0, 1.0);
    // A second point on the first.
    b.click(1.0, 1.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.points(), 1);
    b.click(5.0, 1.0);
    // One part, or too many.
    assert!(b.type_text("1"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(b.type_text("10001"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(!b.type_text("pekçok"), "not a number");
    assert_eq!(b.doc.len(), 0);
    // Esc drops the second point, then the first, then leaves.
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.points(), 1);
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert!(!b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.session.tool_id(), "select");
}

// ── Kesişim noktası ─────────────────────────────────────────────────────────

#[test]
fn two_distances_offer_both_meetings_and_a_click_chooses() {
    let mut b = Bench::new("intersectPoint");
    assert_eq!(
        b.session.prompt().text(),
        "Kesişim noktası: A noktasını belirtin [İki doğrultu (D) / İki doğru (L)]"
    );
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Kesişim noktası: A'dan uzaklığı yazın ya da tıklayın (Enter: 10.000 m)"
    );
    assert!(b.type_text("5"));
    b.click(6.0, 0.0);
    // The distance from B is shown on its circle; both meetings are marked.
    b.move_to(6.0, 5.0);
    let p = preview(&b);
    assert!(p.strokes.len() >= 2, "both circles");
    assert_eq!(
        p.markers.iter().filter(|m| m.tone == Tone::Accent).count(),
        2
    );
    assert!(b.type_text("5"));
    assert_eq!(
        b.session.prompt().text(),
        "Kesişim noktası: istediğiniz kesişime tıklayın [Sağdaki (Enter)]"
    );
    assert_eq!(b.doc.len(), 0);
    // Near the lower one.
    b.click(3.0, -3.0);
    assert!(near2(newest_point(&b), [3.0, -4.0]));
    assert_eq!(
        b.last_text(),
        Some("Kesişim noktası kondu: Y 487003.000  X 4419996.000")
    );
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.doc.undo().as_deref(), Some("Kesişim noktası"));
    assert_eq!(b.doc.len(), 0);
    assert_eq!(b.memory.meeting_distance, 5.0);
}

#[test]
fn enter_takes_the_meeting_right_of_a_towards_b() {
    let mut b = Bench::new("intersectPoint");
    b.click(0.0, 0.0);
    assert!(b.type_text("5"));
    b.click(6.0, 0.0);
    assert!(b.type_text("5"));
    b.confirm();
    // Facing east from A, the right hand is south.
    assert!(near2(newest_point(&b), [3.0, -4.0]));
    // Distances can be clicked on the circle: 3 from A, then 5 from B.
    let mut b = Bench::new("intersectPoint");
    b.click(0.0, 0.0);
    b.click(3.0, 0.0);
    b.click(6.0, 0.0);
    b.click(6.0, 5.0);
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Kesişim noktası: istediğiniz")
    );
    b.confirm();
    // The circles meet at x = 5/3, y = ±√(9 − 25/9); Enter takes the southern one.
    let p = newest_point(&b);
    assert!(near2(p, [5.0 / 3.0, -(56.0f64 / 9.0).sqrt()]), "{p:?}");
}

#[test]
fn touching_circles_have_one_meeting_and_no_meeting_asks_again() {
    let mut b = Bench::new("intersectPoint");
    b.click(0.0, 0.0);
    assert!(b.type_text("3"));
    b.click(6.0, 0.0);
    assert!(b.type_text("3"));
    // One meeting point: written without a choice.
    assert!(near2(newest_point(&b), [3.0, 0.0]));

    let mut b = Bench::new("intersectPoint");
    b.click(0.0, 0.0);
    assert!(b.type_text("1"));
    b.click(6.0, 0.0);
    assert!(b.type_text("1"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .expect("said")
            .starts_with("İki uzaklık kesişmiyor")
    );
    assert_eq!(b.doc.len(), 0);
    // The last distance is asked again; 1 and 5 are 6 apart and just touch.
    assert!(b.session.prompt().text().contains("B'den uzaklığı"));
    assert!(b.type_text("5"));
    assert!(near2(newest_point(&b), [1.0, 0.0]));
    assert_eq!(all_points(&b).len(), 1);
}

#[test]
fn two_bearings_meet_ahead_of_both_points() {
    let mut b = Bench::new("intersectPoint");
    assert!(b.type_text("D"));
    assert_eq!(
        b.session.prompt().text(),
        "Kesişim noktası: A noktasını belirtin [İki uzaklık (U) / İki doğru (L)]"
    );
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Kesişim noktası: A'nın semtini yazın (grad) ya da gösterin"
    );
    // 100 grads is east.
    assert!(b.type_text("100"));
    b.click(10.0, -10.0);
    // North from B; the meeting shows before the direction is given.
    b.move_to(10.0, -4.0);
    let p = preview(&b);
    assert_eq!(
        p.markers.iter().filter(|m| m.tone == Tone::Accent).count(),
        1
    );
    assert!(p.tag.expect("a tag").lines[0].starts_with("Semt "));
    assert!(b.type_text("0"));
    assert!(near2(newest_point(&b), [10.0, 0.0]));
    assert_eq!(
        b.last_text(),
        Some("Kesişim noktası kondu: Y 487010.000  X 4420000.000")
    );

    // The direction can be clicked along: east from A, north from B.
    let mut b = Bench::new("intersectPoint");
    assert!(b.type_text("D"));
    b.click(0.0, 0.0);
    b.click(7.0, 0.0);
    b.click(10.0, -10.0);
    b.click(10.0, -5.0);
    assert!(near2(newest_point(&b), [10.0, 0.0]));
}

#[test]
fn bearings_that_never_meet_ahead_ask_the_second_one_again() {
    let mut b = Bench::new("intersectPoint");
    assert!(b.type_text("D"));
    b.click(0.0, 0.0);
    assert!(b.type_text("100"));
    // B is north of the east line, its bearing north: it meets behind B.
    b.click(10.0, 10.0);
    assert!(b.type_text("0"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .expect("said")
            .starts_with("Doğrultular kesişmiyor")
    );
    assert_eq!(b.doc.len(), 0);
    assert!(b.session.prompt().text().contains("B'nin semtini"));
    // Parallel bearings.
    assert!(b.type_text("100"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.len(), 0);
    // South from B does meet.
    assert!(b.type_text("200"));
    assert!(near2(newest_point(&b), [10.0, 0.0]));
}

#[test]
fn bearings_are_degrees_when_the_project_says_so() {
    let mut b = degrees();
    b.start("intersectPoint");
    assert!(b.type_text("D"));
    b.click(0.0, 0.0);
    assert!(b.session.prompt().text().contains("semtini yazın (derece)"));
    assert!(b.type_text("90"));
    b.click(10.0, -10.0);
    assert!(b.type_text("0"));
    assert!(near2(newest_point(&b), [10.0, 0.0]));
}

#[test]
fn two_lines_meet_where_they_extend_to() {
    let mut b = Bench::new("intersectPoint");
    assert!(b.type_text("L"));
    assert_eq!(
        b.session.prompt().text(),
        "Kesişim noktası: 1. doğrunun ilk noktasını belirtin [İki uzaklık (U) / İki doğrultu (D)]"
    );
    b.click(0.0, 0.0);
    b.click(4.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Kesişim noktası: 2. doğrunun ilk noktasını belirtin"
    );
    b.click(2.0, -3.0);
    b.move_to(2.0, -1.0);
    // The second line runs to the cursor; where they meet is marked.
    let p = preview(&b);
    assert_eq!(
        p.markers.iter().filter(|m| m.tone == Tone::Accent).count(),
        1
    );
    b.click(2.0, -1.0);
    assert!(near2(newest_point(&b), [2.0, 0.0]));
    assert_eq!(b.doc.undo().as_deref(), Some("Kesişim noktası"));
    // Parallel lines do not meet: the fourth point is asked again.
    let mut b = Bench::new("intersectPoint");
    assert!(b.type_text("L"));
    b.click(0.0, 0.0);
    b.click(4.0, 0.0);
    b.click(0.0, 3.0);
    b.click(4.0, 3.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(b.last_text().expect("said").starts_with("Doğrular paralel"));
    assert_eq!(b.doc.len(), 0);
    assert_eq!(b.points(), 3);
}

#[test]
fn intersect_point_steps_back_one_answer_at_a_time() {
    let mut b = Bench::new("intersectPoint");
    b.click(0.0, 0.0);
    assert!(b.type_text("5"));
    b.click(6.0, 0.0);
    assert!(b.run(|s, cx| s.cancel(cx)), "B is let go");
    assert!(b.session.prompt().text().contains("B noktasını"));
    assert!(b.run(|s, cx| s.cancel(cx)), "the distance is let go");
    assert!(b.session.prompt().text().contains("A'dan uzaklığı"));
    assert!(b.run(|s, cx| s.cancel(cx)), "A is let go");
    assert!(!b.run(|s, cx| s.cancel(cx)), "the tool leaves");
    // Letters change the method only while nothing is given.
    let mut b = Bench::new("intersectPoint");
    b.click(0.0, 0.0);
    assert!(!b.type_text("L"));
}

// ── Açı ölç ─────────────────────────────────────────────────────────────────

#[test]
fn the_angle_and_its_explement_are_drawn_live_and_logged() {
    let mut b = Bench::new("measureAngle");
    assert_eq!(
        b.session.prompt().text(),
        "Açı ölç: açının tepe noktasını belirtin"
    );
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Açı ölç: ikinci kolun bir noktasını belirtin"
    );
    b.move_to(0.0, 8.0);
    let p = preview(&b);
    // Both arms and the arc; the angle labelled on the arc and told by the cursor.
    assert_eq!(p.strokes.len(), 3);
    assert_eq!(p.labels.len(), 1);
    assert_eq!(p.labels[0].text, "100.0000 g");
    assert_eq!(
        p.tag.expect("a tag").lines,
        ["Açı 100.0000 g", "Dış açı 300.0000 g"]
    );
    b.click(0.0, 8.0);
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.last_text(), Some("Açı: 100.0000 g, dış açı: 300.0000 g"));
    // It writes nothing and asks again.
    assert_eq!(b.doc.len(), 0);
    assert_eq!(b.points(), 0);
    assert!(b.session.prompt().text().contains("tepe noktasını"));
}

#[test]
fn the_smaller_angle_is_the_angle_whichever_arm_comes_first() {
    let mut b = Bench::new("measureAngle");
    // From north round to east is 270° counter-clockwise, 90° the other way.
    b.click(0.0, 0.0);
    b.click(0.0, 10.0);
    b.click(10.0, 0.0);
    assert_eq!(b.last_text(), Some("Açı: 100.0000 g, dış açı: 300.0000 g"));
    // 60° in degrees.
    let mut b = degrees();
    b.start("measureAngle");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    b.click(5.0, 5.0 * 3f64.sqrt());
    assert_eq!(b.last_text(), Some("Açı: 60.0000°, dış açı: 300.0000°"));
}

#[test]
fn an_arm_on_the_vertex_is_refused_and_esc_steps_back() {
    let mut b = Bench::new("measureAngle");
    b.click(1.0, 1.0);
    b.click(1.0, 1.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.points(), 1);
    b.click(6.0, 1.0);
    assert_eq!(b.points(), 2);
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.points(), 1);
    // Ctrl+Z takes a point back too; with none it undoes the drawing.
    assert!(b.undo_step());
    assert!(!b.undo_step());
    // Enter with nothing leaves.
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
}

// ── Koordinat oku ───────────────────────────────────────────────────────────

fn with_point(b: &mut Bench, at: [f64; 2], z: Option<f64>) {
    let point = Entity::Point(PointEntity {
        base: EntityBase {
            id: 0,
            layer_id: "cizim".to_owned(),
            color: None,
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: None,
        },
        p: Wire {
            x: E + at[0],
            y: N + at[1],
        },
        z,
        parts: None,
    });
    b.doc.add(point).expect("a slot");
}

#[test]
fn every_click_reads_its_point_and_esc_ends() {
    let mut b = Bench::new("crsQuery");
    assert_eq!(
        b.session.prompt().text(),
        "Koordinat oku: okunacak noktaya tıklayın [Esc: bitir]"
    );
    b.click(12.0, 3.5);
    assert_eq!(b.last_level(), Some(Level::Info));
    assert_eq!(b.last_text(), Some("Y=487012.000, X=4420003.500"));
    b.click(-2.0, 0.0);
    assert_eq!(b.last_text(), Some("Y=486998.000, X=4420000.000"));
    assert_eq!(b.doc.len(), 0, "it writes nothing");
    assert!(b.session.is_running(), "it stays for the next click");
    // The cursor carries the numbers, and a ring stays where the last was read.
    b.move_to(1.0, 2.0);
    let p = preview(&b);
    assert_eq!(
        p.tag.expect("a tag").lines,
        ["Y 487001.000", "X 4420002.000"]
    );
    assert_eq!(p.markers.len(), 1);
    assert!(!b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn a_snapped_point_with_an_elevation_says_it() {
    let mut b = Bench::on(EMPTY);
    with_point(&mut b, [5.0, 5.0], Some(12.5));
    with_point(&mut b, [9.0, 9.0], None);
    b.start("crsQuery");
    // Near the first: the snap is on it, its Z follows.
    b.click(5.02, 5.02);
    assert_eq!(b.last_text(), Some("Y=487005.000, X=4420005.000, Z=12.500"));
    b.click(9.02, 9.02);
    assert_eq!(b.last_text(), Some("Y=487009.000, X=4420009.000"));
    // Enter leaves.
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
}

// ── A locked layer ──────────────────────────────────────────────────────────

#[test]
fn a_locked_active_layer_takes_nothing_from_the_drawing_tools() {
    let locked = "“Çizim” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.";
    let mut b = Bench::new("sector");
    b.doc.toggle_layer_locked("cizim");
    b.click(0.0, 0.0);
    b.click(6.0, 0.0);
    b.click(0.0, 6.0);
    assert_eq!(b.doc.len(), 0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.last_text(), Some(locked));

    let mut b = Bench::new("pointsBetween");
    b.doc.toggle_layer_locked("cizim");
    b.click(0.0, 0.0);
    b.click(6.0, 0.0);
    b.confirm();
    assert_eq!(b.doc.len(), 0);
    assert_eq!(b.last_level(), Some(Level::Warn));

    let mut b = Bench::new("intersectPoint");
    b.doc.toggle_layer_locked("cizim");
    assert!(b.type_text("L"));
    for p in [(0.0, 0.0), (4.0, 0.0), (2.0, -3.0), (2.0, -1.0)] {
        b.click(p.0, p.1);
    }
    assert_eq!(b.doc.len(), 0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.last_text(), Some(locked));
}
