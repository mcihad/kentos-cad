//! The point, circle, arc, rectangle, rotated rectangle and regular polygon
//! tools through the session, over the native document: the behaviour of the
//! web's `PointTool`, `CircleTool`, `ArcTool`, `RectangleTool`,
//! `RotatedRectangleTool` and `RegularPolygonTool` that the interaction
//! traces fix, one rule per test (docs/adr/0032). Expected values are worked
//! out by hand from the geometry, not taken from a run.

mod common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_interaction::{Corners, Level};

const OBJECTS: &str = include_str!("../../../../fixtures/interaction/v1/objects.kcad");

fn near(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-9
}

fn circle(b: &Bench) -> ([f64; 2], f64) {
    let Entity::Circle(c) = b.newest() else {
        panic!("a circle, not {:?}", b.newest());
    };
    (rel(c.c), c.r)
}

fn arc(b: &Bench) -> ([f64; 2], f64, f64, f64) {
    let Entity::Arc(a) = b.newest() else {
        panic!("an arc, not {:?}", b.newest());
    };
    (rel(a.c), a.r, a.a0, a.a1)
}

fn polygon(b: &Bench) -> (Vec<[f64; 2]>, Vec<f64>) {
    let Entity::Polygon(p) = b.newest() else {
        panic!("a closed area, not {:?}", b.newest());
    };
    (
        p.pts.iter().map(|q| rel(*q)).collect(),
        p.bulges.clone().unwrap_or_default(),
    )
}

// ── Nokta ───────────────────────────────────────────────────────────────────

#[test]
fn points_are_written_one_by_one_and_enter_leaves() {
    let mut b = Bench::new("point");
    assert_eq!(b.session.prompt().text(), "Nokta: nokta konumunu belirtin");
    b.click(1.0, 2.0);
    b.click(3.0, 4.0);
    assert!(b.type_text("487005,4420006"), "a typed Y,X");
    assert_eq!(b.doc.len(), 3);
    let Entity::Point(p) = b.newest() else {
        panic!("a point");
    };
    assert_eq!(rel(p.p), [5.0, 6.0]);
    // The echo is the only message: no “eklendi” for a point, as on the web.
    assert_eq!(b.last_level(), Some(Level::Info));
    assert_eq!(b.points(), 0, "the tool never holds a point");
    // Ctrl+Z takes the newest point back; Enter leaves.
    assert!(b.undo_step());
    assert_eq!(b.doc.len(), 2);
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
}

// ── Daire ───────────────────────────────────────────────────────────────────

#[test]
fn the_circle_s_prompts_are_the_web_s() {
    let mut b = Bench::new("circle");
    assert_eq!(
        b.session.prompt().text(),
        "Daire: merkez noktasını belirtin [2 nokta (2N) / 3 nokta (3N) / Teğet-teğet-yarıçap (TTY) / Teğet-teğet-teğet (TTT)]"
    );
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Daire: yarıçapı gösterin ya da yazın [Çap (Ç)]"
    );
    assert!(b.type_text("c"), "C answers for Ç");
    assert_eq!(
        b.session.prompt().text(),
        "Daire: çapı gösterin ya da yazın [Yarıçap (R)]"
    );
}

#[test]
fn centre_and_radius_clicked_typed_or_as_a_diameter() {
    let mut b = Bench::new("circle");
    b.click(0.0, 0.0);
    b.click(3.0, 4.0);
    assert_eq!(circle(&b), ([0.0, 0.0], 5.0));
    assert_eq!(b.last_text(), Some("Daire eklendi: r = 5.000 m"));
    b.click(10.0, 0.0);
    assert!(b.type_text("2.5"));
    assert_eq!(circle(&b), ([10.0, 0.0], 2.5));
    // Çap: a typed diameter is halved, and the choice stays for the next circle.
    b.click(20.0, 0.0);
    assert!(b.type_text("Ç"));
    assert!(b.type_text("8"));
    assert_eq!(circle(&b), ([20.0, 0.0], 4.0));
    b.click(30.0, 0.0);
    b.click(36.0, 0.0);
    assert_eq!(circle(&b).1, 3.0, "still a diameter: 6 m across");
    assert_eq!(b.doc.len(), 4);
}

#[test]
fn two_and_three_points() {
    let mut b = Bench::new("circle");
    assert!(b.type_text("2n"));
    assert_eq!(b.session.prompt().text(), "Daire: çapın ilk ucunu belirtin");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert_eq!(circle(&b), ([5.0, 0.0], 5.0));
    // 2N stays for the next circle until another method is chosen, before its first point.
    assert_eq!(b.session.prompt().text(), "Daire: çapın ilk ucunu belirtin");
    b.click(0.0, 0.0);
    assert!(!b.type_text("3N"), "not after a point");
    b.confirm();
    assert!(b.type_text("3N"));
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    b.click(0.0, 10.0);
    let (c, r) = circle(&b);
    assert_eq!(c, [5.0, 5.0]);
    assert!(near(r, 50f64.sqrt()));
    // Collinear points warn and start over.
    let before = b.doc.len();
    b.click(0.0, 20.0);
    b.click(5.0, 20.0);
    b.click(10.0, 20.0);
    assert_eq!(b.doc.len(), before);
    assert_eq!(
        b.last_text(),
        Some("Üç nokta aynı doğru üzerinde; daire çizilemez.")
    );
    assert_eq!(b.points(), 0);
}

/// The traces' drawing with an L of lines and an empty space: tangent picks
/// take the nearest edge of the object under the pointer.
fn tangents_bench(method: &str) -> Bench {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.start("circle");
    assert!(b.type_text(method));
    b
}

#[test]
fn tangent_tangent_radius_picks_two_objects_then_a_radius() {
    let mut b = tangents_bench("TTY");
    b.draft.snap = true;
    assert!(
        b.snapped(-24.0, -12.0).snap.is_none(),
        "no snaps while objects are picked"
    );
    b.draft.snap = false;
    assert_eq!(
        b.session.prompt().text(),
        "Daire: ilk teğet çizgi, yay ya da daireyi seçin"
    );
    // Nothing under the pointer: a warning, no pick.
    b.click(20.0, 18.0);
    assert_eq!(
        b.last_text(),
        Some("Teğet olunacak bir çizgi, çoklu çizgi, yay ya da daireye tıklayın.")
    );
    // Line 1 (y = −12, from x −24 to −8) and the closed area 4 (its right edge x = −12).
    b.click(-16.0, -12.0);
    b.click(-12.0, 8.0);
    assert_eq!(b.points(), 0, "tangent picks are not points");
    assert_eq!(
        b.session.prompt().text(),
        "Daire: yarıçapı yazın",
        "no radius to offer yet"
    );
    assert!(b.type_text("2"));
    // A circle of radius 2 touching y = −12 and x = −12, near both picks.
    let (c, r) = circle(&b);
    assert_eq!(r, 2.0);
    assert!(near(c[1], -10.0) && near((c[0] + 12.0).abs(), 2.0), "{c:?}");
    // The next one offers the last radius; Enter takes it.
    b.click(-16.0, -12.0);
    b.click(-12.0, 8.0);
    assert_eq!(
        b.session.prompt().text(),
        "Daire: yarıçapı yazın (Enter: 2.000 m)"
    );
    b.confirm();
    assert_eq!(circle(&b).1, 2.0);
}

#[test]
fn three_tangents_make_the_circle_the_picks_are_near() {
    let mut b = tangents_bench("TTT");
    // Line 1 (y = −12), the closed area's left and right edges (x = −24, x = −12).
    b.click(-16.0, -12.0);
    b.click(-24.0, 8.0);
    b.click(-12.0, 8.0);
    let (c, r) = circle(&b);
    // Between two parallels 12 m apart: radius 6, centre on x = −18, 6 m above y = −12.
    assert!(near(r, 6.0), "{r}");
    assert!(near(c[0], -18.0) && near(c[1], -6.0), "{c:?}");
}

// ── Yay ─────────────────────────────────────────────────────────────────────

#[test]
fn three_points_and_the_web_s_message() {
    let mut b = Bench::new("arc");
    assert_eq!(
        b.session.prompt().text(),
        "Yay: başlangıç noktasını belirtin [Merkez (M) / Devam (D)]"
    );
    b.click(10.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Yay: yay üzerinde ikinci bir nokta belirtin [Merkez (M) / Bitiş (B)]"
    );
    b.click(0.0, 10.0);
    b.click(-10.0, 0.0);
    let (c, r, a0, a1) = arc(&b);
    assert_eq!(c, [0.0, 0.0]);
    assert!(near(r, 10.0) && near(a0, 0.0) && near(a1, std::f64::consts::PI));
    assert_eq!(
        b.last_text(),
        Some("Yay eklendi: r = 10.000 m, açı 180.0000°")
    );
}

#[test]
fn start_centre_then_an_angle_or_a_chord() {
    let mut b = Bench::new("arc");
    b.click(10.0, 0.0);
    assert!(b.type_text("M"));
    assert_eq!(b.session.prompt().text(), "Yay: yayın merkezini belirtin");
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Yay: bitiş noktasını belirtin [Açı (A) / Kiriş (U)]"
    );
    assert!(b.type_text("A"));
    assert!(b.type_text("90"), "a typed included angle");
    let (c, r, a0, a1) = arc(&b);
    assert_eq!(c, [0.0, 0.0]);
    assert!(near(r, 10.0) && near(a0, 0.0) && near(a1, std::f64::consts::FRAC_PI_2));
    // The mode resets after an arc.
    assert_eq!(b.options(), ["M", "D"]);
}

#[test]
fn start_end_then_a_radius_and_the_centre_first() {
    let mut b = Bench::new("arc");
    b.click(10.0, 0.0);
    assert!(b.type_text("B"));
    b.click(0.0, 10.0);
    assert_eq!(
        b.session.prompt().text(),
        "Yay: yayın merkezini belirtin [Açı (A) / Yön (Y) / Yarıçap (R)]"
    );
    assert!(b.type_text("R") && b.type_text("10"));
    let (c, r, _, _) = arc(&b);
    assert!(near(r, 10.0));
    assert!(
        near(c[0], 0.0) && near(c[1], 0.0),
        "the minor arc's centre: {c:?}"
    );
    // M first: the centre, then the start, then the end.
    assert!(b.type_text("M"));
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Yay: başlangıç noktasını belirtin"
    );
    b.click(5.0, 0.0);
    b.click(0.0, 5.0);
    let (c, r, a0, a1) = arc(&b);
    assert_eq!(c, [0.0, 0.0]);
    assert!(near(r, 5.0) && near(a0, 0.0) && near(a1, std::f64::consts::FRAC_PI_2));
}

#[test]
fn devam_continues_the_newest_line_tangentially() {
    let mut b = Bench::new("arc");
    let before = b.log.len();
    assert!(b.type_text("D"), "D is always an option before the start");
    assert_eq!(
        b.said(before),
        [(
            Level::Warn,
            "Devam edilecek bir çizgi, yay ya da çoklu çizgi yok."
        )]
    );
    // A line east from (0, 0) to (10, 0), then an arc going on from its end.
    b.start("line");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    b.confirm();
    b.start("arc");
    assert!(b.type_text("D"));
    assert_eq!(b.points(), 1, "the line's end is the start");
    assert_eq!(
        b.session.prompt().text(),
        "Yay: bitiş noktasını belirtin (son nesneye teğet devam)"
    );
    b.click(10.0, 10.0);
    // Leaving east and ending 10 m north: a half turn of radius 5 about (10, 5).
    let (c, r, _, _) = arc(&b);
    assert!(
        near(r, 5.0) && near(c[0], 10.0) && near(c[1], 5.0),
        "{c:?} {r}"
    );
}

#[test]
fn an_arc_that_cannot_be_made_warns_and_keeps_its_points() {
    let mut b = Bench::new("arc");
    b.click(0.0, 0.0);
    b.click(5.0, 0.0);
    b.click(10.0, 0.0);
    assert_eq!(b.doc.len(), 0);
    assert_eq!(
        b.last_text(),
        Some("Bu değerlerle yay oluşmuyor (noktalar aynı doğruda ya da yarıçap kiriş için küçük).")
    );
    assert_eq!(b.points(), 2, "the points stay");
}

// ── Dikdörtgen ──────────────────────────────────────────────────────────────

#[test]
fn two_corners_with_the_options_values_in_the_prompt() {
    let mut b = Bench::new("rectangle");
    assert_eq!(
        b.session.prompt().text(),
        "Dikdörtgen: ilk köşeyi belirtin [Köşe yuvarla (Y): kapalı / Pah (P): kapalı]"
    );
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Dikdörtgen: karşı köşeyi belirtin [Döndür (D): 0° / Boyutlar (B)]"
    );
    b.click(20.0, 10.0);
    let (pts, bulges) = polygon(&b);
    assert_eq!(pts, [[0.0, 0.0], [20.0, 0.0], [20.0, 10.0], [0.0, 10.0]]);
    assert!(bulges.is_empty());
    assert_eq!(b.last_text(), Some("Dikdörtgen eklendi: 20.000 × 10.000 m"));
}

#[test]
fn rounded_and_cut_corners_stay_for_the_next_rectangles() {
    let mut b = Bench::new("rectangle");
    assert!(b.type_text("Y"));
    assert_eq!(
        b.session.prompt().text(),
        "Dikdörtgen: köşe yarıçapını yazın (0: keskin köşe)"
    );
    assert!(b.type_text("2"));
    assert_eq!(b.memory.rect_corners, Corners::Fillet(2.0));
    assert_eq!(
        b.session.prompt().text(),
        "Dikdörtgen: ilk köşeyi belirtin [Köşe yuvarla (Y): 2.000 m / Pah (P): kapalı]"
    );
    b.click(0.0, 0.0);
    b.click(20.0, 10.0);
    let (pts, bulges) = polygon(&b);
    assert_eq!(pts.len(), 8, "two tangent points a corner");
    assert_eq!(
        bulges.iter().filter(|x| **x != 0.0).count(),
        4,
        "a quarter arc a corner"
    );
    // The next run of the tool keeps them (the web's static fields).
    b.start("rectangle");
    assert!(b.type_text("P") && b.type_text("1.5"));
    assert_eq!(b.memory.rect_corners, Corners::Chamfer(1.5));
    b.click(0.0, 20.0);
    b.click(10.0, 30.0);
    let (pts, bulges) = polygon(&b);
    assert_eq!(pts.len(), 8);
    assert!(bulges.iter().all(|x| *x == 0.0));
    assert!(b.type_text("Y") && b.type_text("0"), "0: sharp corners");
    assert_eq!(b.memory.rect_corners, Corners::Sharp);
}

#[test]
fn rotation_and_exact_size() {
    let mut b = Bench::new("rectangle");
    b.click(0.0, 0.0);
    assert!(b.type_text("D") && b.type_text("90"));
    assert_eq!(
        b.session.prompt().text(),
        "Dikdörtgen: karşı köşeyi belirtin [Döndür (D): 90° / Boyutlar (B)]"
    );
    assert!(b.type_text("B"));
    assert!(!b.type_text("20;-10"), "sizes are unsigned");
    assert!(b.type_text("20,10"));
    assert_eq!(
        b.session.prompt().text(),
        "Dikdörtgen: dikdörtgenin hangi yana açılacağını gösterin"
    );
    // Rotated 90°: the length runs north, the width west (towards the click).
    b.click(-5.0, 5.0);
    let (pts, _) = polygon(&b);
    assert!(
        pts.iter().any(|p| near(p[0], 0.0) && near(p[1], 20.0)),
        "{pts:?}"
    );
    assert!(
        pts.iter().any(|p| near(p[0], -10.0) && near(p[1], 20.0)),
        "{pts:?}"
    );
    assert!(near(b.memory.rect_rotation, std::f64::consts::FRAC_PI_2));
}

#[test]
fn a_rotated_rectangle_from_an_edge_and_a_width() {
    let mut b = Bench::new("rectangle3");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Döndürülmüş dikdörtgen: genişliği fareyle yana çekerek gösterin ya da yazın"
    );
    b.move_to(3.0, -2.0);
    assert!(b.type_text("4"), "a typed width goes to the mouse's side");
    let (pts, _) = polygon(&b);
    assert!(pts.iter().any(|p| near(p[1], -4.0)), "{pts:?}");
    assert_eq!(b.last_text(), Some("Dikdörtgen eklendi: 10.000 × 4.000 m"));
}

#[test]
fn a_regular_polygon_by_its_side_count_centre_and_corner() {
    let mut b = Bench::new("regularPolygon");
    assert_eq!(
        b.session.prompt().text(),
        "Düzgün çokgen: merkezi belirtin ya da kenar sayısını yazın [Kenar sayısı (S): 6 / Çember (Ç): köşeler üzerinde / Kenardan (K)]"
    );
    assert!(b.type_text("4"), "a bare number first is the side count");
    assert!(b.type_text("2"), "out of range, but taken as an answer");
    assert_eq!(
        b.last_text(),
        Some("Kenar sayısı 3 ile 1024 arasında bir tam sayı olmalı.")
    );
    b.click(0.0, 0.0);
    b.click(5.0, 0.0);
    let (pts, _) = polygon(&b);
    assert_eq!(pts.len(), 4);
    assert!(
        pts.iter().any(|p| near(p[0], 5.0) && near(p[1], 0.0)),
        "a corner where clicked"
    );
    assert!(
        b.last_text()
            .is_some_and(|t| t.starts_with("4 kenarlı düzgün çokgen eklendi: kenar "))
    );
    assert_eq!(b.memory.polygon_sides, 4, "stays for the next polygons");
}
