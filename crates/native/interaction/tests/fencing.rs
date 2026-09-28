//! Buda and Uzat's Çit method and Ötele's İki yana and Kaynağı sil through the
//! session, over the native document (docs/adr/0140, phase 3): the fence
//! drawn and previewed, everything it crosses cut or extended in one undo
//! step, an object cut again where an earlier crossing left it, a locked
//! layer left out, nothing crossed said plainly, and Ötele's two copies and
//! deleted source. Points are east and north differences from (E, N);
//! expected values are worked out by hand.

mod common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::{Level, MarkerShape, Tone};

const EDITS: &str = include_str!("../../../../fixtures/interaction/v1/edits.kcad");

fn near(have: f64, want: f64) -> bool {
    (have - want).abs() < 1e-9
}

fn preview(b: &Bench) -> kentos_interaction::Preview {
    b.session.preview(&Default::default()).expect("a preview")
}

/// A line's ends, east and north differences from (E, N).
fn ends(b: &Bench, slot: Slot) -> [[f64; 2]; 2] {
    let Some(Entity::Line(l)) = b.doc.get(slot) else {
        panic!("a line at {slot:?}: {:?}", b.doc.get(slot));
    };
    [rel(l.a), rel(l.b)]
}

/// Every line's ends, sorted, for what has no fixed slots.
fn all_lines(b: &Bench) -> Vec<[[f64; 2]; 2]> {
    let mut out: Vec<_> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Line(l) => Some([rel(l.a), rel(l.b)]),
            _ => None,
        })
        .collect();
    out.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    out
}

/// Three vertical roads (x = 0, 5, 10) crossed by two streets (y = ±2)
/// reaching to `right`.
fn grid(b: &mut Bench, right: f64) -> Vec<Slot> {
    let mut slots = Vec::new();
    for x in [0.0, 5.0, 10.0] {
        slots.push(b.add_line("cizim", [x, -5.0], [x, 5.0]));
    }
    for y in [-2.0, 2.0] {
        slots.push(b.add_line("cizim", [-3.0, y], [right, y]));
    }
    slots
}

// ── Buda: Çit ───────────────────────────────────────────────────────────────

#[test]
fn a_fence_trims_everything_it_crosses_in_one_step() {
    let mut b = Bench::new("trim");
    let roads = grid(&mut b, 13.0);
    assert!(b.type_text("C"), "the ribbon's letter");
    assert_eq!(
        b.session.prompt().text(),
        "Buda: çitin ilk noktasını belirtin [Tıklama (K)]"
    );
    b.click(-1.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Buda: çitin sonraki noktasını belirtin [Tıklama (K)]"
    );
    // The fence follows the cursor, dashed; a cross marks each crossing, the
    // tag counts what it crosses.
    b.move_to(12.0, 0.0);
    let p = preview(&b);
    assert!(p.strokes.iter().any(|s| s.dash.is_some()));
    assert_eq!(
        p.markers
            .iter()
            .filter(|m| matches!(m.shape, MarkerShape::Cross(_)) && m.tone == Tone::Danger)
            .count(),
        3
    );
    assert_eq!(p.tag.expect("a tag").lines, ["3 nesne"]);
    b.click(12.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Buda: çitin sonraki noktasını belirtin [Tıklama (K) / Uygula (Enter)]"
    );
    let before = b.doc.len();
    assert_eq!(before, 5, "nothing is written yet");
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.last_text(), Some("Çit: 3 nesne budandı."));
    // Each road lost the part between the streets: two pieces each.
    assert_eq!(b.doc.len(), before + 3);
    for (road, x) in roads.iter().zip([0.0, 5.0, 10.0]) {
        assert_eq!(
            ends(&b, *road),
            [[x, -5.0], [x, -2.0]],
            "the first piece in its place"
        );
    }
    let lines = all_lines(&b);
    assert!(lines.contains(&[[5.0, 2.0], [5.0, 5.0]]));
    // The streets were not crossed: whole.
    assert_eq!(ends(&b, roads[3]), [[-3.0, -2.0], [13.0, -2.0]]);
    // The tool stays for another fence; one undo step took it all.
    assert!(b.session.is_running());
    assert_eq!(b.doc.undo().as_deref(), Some("Buda"));
    assert_eq!(b.doc.len(), before);
    assert_eq!(ends(&b, roads[1]), [[5.0, -5.0], [5.0, 5.0]]);
}

#[test]
fn an_object_is_cut_again_where_an_earlier_crossing_left_it() {
    let mut b = Bench::new("trim");
    let road = b.add_line("cizim", [5.0, -5.0], [5.0, 5.0]);
    for y in [-2.0, 2.0] {
        b.add_line("cizim", [-3.0, y], [7.0, y]);
    }
    assert!(b.type_text("C"));
    // Across the road below the streets, up round their ends, back across above them.
    for (e, n) in [(4.0, -3.5), (8.0, -3.5), (8.0, 3.5), (4.0, 3.5)] {
        b.click(e, n);
    }
    b.move_to(4.0, 3.5);
    assert_eq!(preview(&b).tag.expect("a tag").lines, ["1 nesne"]);
    b.confirm();
    // The lower end went first; the piece left was cut again at the upper end.
    assert_eq!(ends(&b, road), [[5.0, -2.0], [5.0, 2.0]]);
    assert_eq!(b.doc.len(), 3, "no piece was added");
    assert_eq!(b.last_text(), Some("Çit: 1 nesne budandı."));
    assert_eq!(b.doc.undo().as_deref(), Some("Buda"));
    assert_eq!(ends(&b, road), [[5.0, -5.0], [5.0, 5.0]]);
}

#[test]
fn a_crossing_on_a_part_already_taken_away_is_let_go() {
    let mut b = Bench::new("trim");
    let road = b.add_line("cizim", [5.0, -5.0], [5.0, 5.0]);
    for y in [-2.0, 2.0] {
        b.add_line("cizim", [-3.0, y], [13.0, y]);
    }
    assert!(b.type_text("C"));
    // Twice across the road between the streets, going round its right side.
    for (e, n) in [(4.0, -1.0), (6.0, -1.0), (6.0, 1.0), (4.0, 1.0)] {
        b.click(e, n);
    }
    b.confirm();
    // Two crossings, one cut: the second was in what the first took away.
    assert_eq!(b.last_text(), Some("Çit: 1 nesne budandı."));
    assert_eq!(ends(&b, road), [[5.0, -5.0], [5.0, -2.0]]);
    assert_eq!(all_lines(&b).len(), 4);
}

#[test]
fn a_fence_that_crosses_nothing_or_only_a_locked_line_says_so() {
    let mut b = Bench::on(EDITS);
    b.draft.snap = false;
    b.start("trim");
    assert!(b.type_text("C"));
    // The locked line 13 is at x = 8, y from −18 to −12.
    b.click(5.0, -15.0);
    b.click(11.0, -15.0);
    let rev = b.doc.revision();
    b.move_to(11.0, -15.0);
    assert_eq!(preview(&b).tag.expect("a tag").lines, ["Kesişen nesne yok"]);
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("Çit hiçbir nesneyi kesmiyor; çiti budanacak nesnelerin üstünden geçirin.")
    );
    assert_eq!(b.doc.revision(), rev);
    // The fence stays as it was, to amend; Esc drops it. With one point, Enter asks for the second.
    assert_eq!(b.points(), 0);
    assert!(b.run(|s, cx| s.cancel(cx)));
    b.click(5.0, -15.0);
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .expect("said")
            .starts_with("Çit için en az iki nokta")
    );
}

#[test]
fn a_fence_leaves_a_boundaryless_crossing_and_says_why() {
    let mut b = Bench::new("trim");
    // One long line and nothing else: there is no boundary to trim at.
    b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    assert!(b.type_text("C"));
    b.click(5.0, -2.0);
    b.click(5.0, 2.0);
    let rev = b.doc.revision();
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .expect("said")
            .starts_with("Çit hiçbir nesneyi budayamadı")
    );
    assert_eq!(b.doc.revision(), rev);
}

#[test]
fn esc_and_k_leave_the_fence_for_clicking_and_ctrl_z_takes_a_point_back() {
    let mut b = Bench::new("trim");
    grid(&mut b, 13.0);
    assert!(b.type_text("C"));
    b.click(-1.0, 0.0);
    b.click(3.0, 0.0);
    assert!(b.undo_step(), "the last fence point goes");
    b.click(3.0, 1.0);
    // Esc drops the fence's points, then the method.
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert_eq!(b.points(), 0);
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Buda: çitin ilk noktasını")
    );
    assert!(b.run(|s, cx| s.cancel(cx)));
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Buda: silinecek parçaya tıklayın")
    );
    // K goes back too, and Enter with no point leaves the tool.
    assert!(b.type_text("C"));
    assert!(b.type_text("K"));
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Buda: silinecek parçaya")
    );
    assert!(b.type_text("C"));
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
    // The click way still works.
    let mut b = Bench::new("trim");
    grid(&mut b, 13.0);
    let before = b.doc.len();
    b.click(0.0, 0.0);
    assert_eq!(b.doc.len(), before + 1);
}

// ── Uzat: Çit ───────────────────────────────────────────────────────────────

#[test]
fn a_fence_extends_the_end_nearest_each_crossing_in_one_step() {
    let mut b = Bench::new("extend");
    let wall = b.add_line("cizim", [10.0, -10.0], [10.0, 10.0]);
    let short = b.add_line("cizim", [0.0, 0.0], [6.0, 0.0]);
    let long = b.add_line("cizim", [0.0, 3.0], [8.0, 3.0]);
    assert!(b.type_text("C"));
    assert_eq!(
        b.session.prompt().text(),
        "Uzat: çitin ilk noktasını belirtin [Tıklama (K)]"
    );
    b.click(5.0, -1.0);
    b.click(5.0, 4.0);
    // A plus marks each end to be reached.
    b.move_to(5.0, 4.0);
    let p = preview(&b);
    assert_eq!(
        p.markers
            .iter()
            .filter(|m| matches!(m.shape, MarkerShape::Plus(_)) && m.tone == Tone::Accent)
            .count(),
        2
    );
    b.confirm();
    assert_eq!(b.last_text(), Some("Çit: 2 nesne uzatıldı."));
    assert_eq!(ends(&b, short), [[0.0, 0.0], [10.0, 0.0]]);
    assert_eq!(ends(&b, long), [[0.0, 3.0], [10.0, 3.0]]);
    assert_eq!(ends(&b, wall), [[10.0, -10.0], [10.0, 10.0]]);
    assert_eq!(b.doc.undo().as_deref(), Some("Uzat"));
    assert_eq!(ends(&b, short), [[0.0, 0.0], [6.0, 0.0]]);
    assert_eq!(ends(&b, long), [[0.0, 3.0], [8.0, 3.0]]);
}

#[test]
fn an_end_is_extended_once_however_often_the_fence_crosses_near_it() {
    let mut b = Bench::new("extend");
    b.add_line("cizim", [10.0, -10.0], [10.0, 10.0]);
    b.add_line("cizim", [14.0, -10.0], [14.0, 10.0]);
    let line = b.add_line("cizim", [0.0, 0.0], [6.0, 0.0]);
    assert!(b.type_text("C"));
    // A zigzag over the line at x = 5 and at x = 5.5.
    for (e, n) in [(5.0, -1.0), (5.0, 1.0), (5.5, 1.0), (5.5, -1.0)] {
        b.click(e, n);
    }
    b.confirm();
    // The first boundary, not the second.
    assert_eq!(ends(&b, line), [[0.0, 0.0], [10.0, 0.0]]);
    assert_eq!(b.last_text(), Some("Çit: 1 nesne uzatıldı."));
}

// ── Ötele: İki yana, Kaynağı sil ────────────────────────────────────────────

#[test]
fn both_sides_make_two_copies_and_the_source_can_go_in_the_same_step() {
    let mut b = Bench::new("offset");
    let road = b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    assert!(b.type_text("I"));
    assert!(b.session.prompt().text().contains("İki yana (I): açık"));
    assert!(b.memory.offset_both);
    b.click(5.0, 0.0);
    b.move_to(5.0, 3.0);
    // Both copies dashed, the tag says both sides.
    let p = preview(&b);
    assert_eq!(p.strokes.iter().filter(|s| s.dash.is_some()).count(), 2);
    assert_eq!(p.tag.expect("a tag").lines, ["Mesafe 1.000 m", "İki yana"]);
    b.click(5.0, 3.0);
    assert_eq!(
        b.last_text(),
        Some("1.000 m iki yana ötelenmiş 2 kopya eklendi.")
    );
    assert_eq!(b.doc.len(), 3);
    let lines = all_lines(&b);
    assert!(lines.contains(&[[0.0, 1.0], [10.0, 1.0]]));
    assert!(lines.contains(&[[0.0, -1.0], [10.0, -1.0]]));
    assert_eq!(ends(&b, road), [[0.0, 0.0], [10.0, 0.0]]);
    assert_eq!(b.doc.undo().as_deref(), Some("Ötele"));
    assert_eq!(b.doc.len(), 1);

    // Kaynağı sil with both sides: the source goes in the same step.
    assert!(b.type_text("S"));
    b.click(5.0, 0.0);
    b.click(5.0, -3.0);
    assert_eq!(
        b.last_text(),
        Some("1.000 m iki yana ötelenmiş 2 kopya eklendi; kaynak silindi.")
    );
    assert!(b.doc.get(road).is_none());
    assert_eq!(b.doc.len(), 2);
    assert_eq!(b.doc.undo().as_deref(), Some("Ötele"));
    assert_eq!(b.doc.len(), 1);
    assert!(b.doc.get(road).is_some());
}

#[test]
fn deleting_the_source_alone_moves_the_line() {
    let mut b = Bench::new("offset");
    let road = b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    assert!(b.type_text("S"));
    assert!(b.type_text("2.5"));
    b.click(5.0, 0.0);
    b.click(5.0, 4.0);
    assert_eq!(
        b.last_text(),
        Some("2.500 m ötelenmiş kopya eklendi; kaynak silindi.")
    );
    assert!(b.doc.get(road).is_none());
    assert_eq!(all_lines(&b), [[[0.0, 2.5], [10.0, 2.5]]]);
    assert!(b.memory.offset_erase);
    // Both settings are kept: another run starts with the same prompt.
    b.confirm();
    b.start("offset");
    assert!(b.session.prompt().text().contains("Kaynağı sil (S): açık"));
    // S again turns it off.
    assert!(b.type_text("s"));
    assert!(
        b.session
            .prompt()
            .text()
            .ends_with("İki yana (I) / Kaynağı sil (S)]")
    );
}

#[test]
fn both_sides_of_a_circle_are_inside_and_outside() {
    let mut b = Bench::new("offset");
    b.add_line("cizim", [-30.0, -30.0], [-29.0, -30.0]);
    b.doc
        .add(Entity::Circle(kentos_contracts::CircleEntity {
            base: common::base("cizim"),
            c: kentos_contracts::Vec2 {
                x: common::E,
                y: common::N,
            },
            r: 5.0,
        }))
        .expect("a slot");
    assert!(b.type_text("I"));
    b.click(5.0, 0.0);
    // Far outside: the far side is still the inside.
    b.click(9.0, 0.0);
    let mut radii: Vec<f64> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Circle(c) => Some(c.r),
            _ => None,
        })
        .collect();
    radii.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
    assert_eq!(radii.len(), 3);
    assert!(
        near(radii[0], 4.0) && near(radii[1], 5.0) && near(radii[2], 6.0),
        "{radii:?}"
    );
}

#[test]
fn a_side_point_on_the_object_gives_only_the_one_copy_it_can() {
    let mut b = Bench::new("offset");
    b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    assert!(b.type_text("I"));
    b.click(5.0, 0.0);
    let before = b.log.len();
    // The point is on the object: it has no far side, so the second copy is said not made.
    b.click(5.0, 0.0);
    let said = b.said(before);
    assert_eq!(said.len(), 2, "{said:?}");
    assert_eq!(said[0].0, Level::Warn);
    assert!(said[0].1.starts_with("Nokta nesnenin üzerinde"));
    assert_eq!(
        said[1],
        (Level::Success, "1.000 m ötelenmiş kopya eklendi.")
    );
    assert_eq!(b.doc.len(), 2);
}
