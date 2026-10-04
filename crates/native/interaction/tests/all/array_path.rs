//! Yol boyunca dizi through the session, over the native document
//! (docs/adr/0140, phase 3): the objects selected before or after, the path
//! clicked, the copies shown live, the count typed or the spacing (Aralık,
//! A), Hizala (H), one write and undo step, the values kept, Esc stepping
//! back, a locked layer and what does not fit said plainly. Points are east
//! and north differences from (E, N); expected values are worked out by hand.

use crate::common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::Level;

const EMPTY: &str = include_str!("../../../../../fixtures/interaction/v1/empty.kcad");
const EDITS: &str = include_str!("../../../../../fixtures/interaction/v1/edits.kcad");

/// The empty drawing, no command running, snapping off.
fn empty() -> Bench {
    let mut b = Bench::on(EMPTY);
    b.draft.snap = false;
    b
}

fn near(have: f64, want: f64) -> bool {
    (have - want).abs() < 1e-9
}

fn near2(have: [f64; 2], want: [f64; 2]) -> bool {
    near(have[0], want[0]) && near(have[1], want[1])
}

fn preview(b: &Bench) -> kentos_interaction::Preview {
    b.session.preview(&Default::default()).expect("a preview")
}

/// The lines other than `except`, their ends, in the document's order.
fn copies(b: &Bench, except: &[Slot]) -> Vec<[[f64; 2]; 2]> {
    b.doc
        .entities()
        .filter(|e| !except.contains(&Slot(e.base().id)))
        .filter_map(|e| match e {
            Entity::Line(l) => Some([rel(l.a), rel(l.b)]),
            _ => None,
        })
        .collect()
}

/// A short line at (0, 1) to (1, 1), selected, and a 20 m path along the east
/// axis; the tool started with the line selected.
fn bench() -> (Bench, Slot, Slot) {
    let mut b = empty();
    let object = b.add_line("cizim", [0.0, 1.0], [1.0, 1.0]);
    let path = b.add_line("cizim", [0.0, 0.0], [20.0, 0.0]);
    b.selection.set([object]);
    b.start("arrayPath");
    (b, object, path)
}

#[test]
fn the_copies_show_along_the_path_and_enter_writes_them() {
    let (mut b, object, path) = bench();
    assert_eq!(
        b.session.prompt().text(),
        "Yol boyunca dizi: yolu seçin: çizgi, yay, daire ya da çoklu çizgi"
    );
    // Hovering lights the path to be; a click picks it.
    b.move_to(10.0, 0.0);
    assert_eq!(b.selection.hover(), Some(path));
    b.click(10.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Yol boyunca dizi: adedi yazın (Enter: 5 adet) [Aralık (A) / Hizala (H): evet / Uygula (Enter)]"
    );
    b.move_to(12.0, 6.0);
    let p = preview(&b);
    // Four copies as ghosts, the path drawn over them, its start ringed.
    assert_eq!(p.strokes.len(), 5, "{:?}", p.strokes.len());
    assert_eq!(p.markers.len(), 1);
    assert_eq!(p.tag.expect("a tag").lines, ["5 adet"]);
    let before = b.doc.len();
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(
        b.last_text(),
        Some("Yol boyunca dizi: 5 adet, 4 yeni nesne.")
    );
    assert_eq!(b.doc.len(), before + 4);
    let made = copies(&b, &[object, path]);
    assert_eq!(made.len(), 4);
    for (line, x) in made.iter().zip([5.0, 10.0, 15.0, 20.0]) {
        assert!(
            near2(line[0], [x, 1.0]) && near2(line[1], [x + 1.0, 1.0]),
            "{line:?} at {x}"
        );
    }
    // The path is only read; the tool leaves after writing; one undo step.
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.undo().as_deref(), Some("Yol boyunca dizi"));
    assert_eq!(b.doc.len(), before);
}

#[test]
fn a_typed_count_is_kept_and_a_typed_spacing_fills_the_path() {
    let (mut b, object, path) = bench();
    b.click(10.0, 0.0);
    assert!(b.type_text("3"));
    assert_eq!(b.memory.path_count, 3);
    assert!(b.session.prompt().text().contains("(Enter: 3 adet)"));
    b.confirm();
    let made = copies(&b, &[object, path]);
    assert_eq!(made.len(), 2);
    assert!(near2(made[0][0], [10.0, 1.0]) && near2(made[1][0], [20.0, 1.0]));

    // Aralık: 6 m from the start, as many as fit: 0, 6, 12, 18.
    let (mut b, object, path) = bench();
    b.click(10.0, 0.0);
    assert!(b.type_text("A"));
    assert_eq!(
        b.session.prompt().text(),
        "Yol boyunca dizi: aralığı yazın (Enter: 10.000 m) [Adet (N) / Hizala (H): evet / Uygula (Enter)]"
    );
    assert!(b.type_text("6"));
    assert_eq!(b.memory.path_spacing, 6.0);
    b.move_to(15.0, 5.0);
    assert_eq!(
        preview(&b).tag.expect("a tag").lines,
        ["4 adet", "aralık 6.000 m"]
    );
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Yol boyunca dizi: 4 adet, 3 yeni nesne.")
    );
    let made = copies(&b, &[object, path]);
    for (line, x) in made.iter().zip([6.0, 12.0, 18.0]) {
        assert!(near2(line[0], [x, 1.0]), "{line:?}");
    }
    // N goes back to the count.
    let (mut b, ..) = bench();
    assert!(b.type_text("A"));
    b.click(10.0, 0.0);
    assert!(b.type_text("N"));
    assert!(b.session.prompt().text().contains("adedi yazın"));
}

#[test]
fn hizala_turns_the_copies_to_the_path_s_direction() {
    let mut b = empty();
    let object = b.add_line("cizim", [0.0, 1.0], [1.0, 1.0]);
    // East 10 m, then north 10 m.
    let path = b.add_path("cizim", &[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]], false);
    b.selection.set([object]);
    b.start("arrayPath");
    b.click(5.0, 0.0);
    assert!(b.type_text("3"));
    let ghosts = preview(&b).strokes.len();
    assert_eq!(ghosts, 3, "two copies and the path");
    b.confirm();
    let made = copies(&b, &[object, path]);
    assert_eq!(made.len(), 2);
    // The middle place is the corner: the direction is still east there, or already north.
    // The end is north: the copy turned a quarter round the start, then moved to (10, 10).
    assert!(near2(made[1][0], [9.0, 10.0]), "{:?}", made[1]);
    assert!(near2(made[1][1], [9.0, 11.0]), "{:?}", made[1]);
    assert_eq!(b.doc.undo().as_deref(), Some("Yol boyunca dizi"));

    // Hizala off: the copies keep their direction.
    b.selection.set([object]);
    b.start("arrayPath");
    b.click(5.0, 0.0);
    assert!(b.type_text("H"));
    assert!(b.session.prompt().text().contains("Hizala (H): hayır"));
    assert!(b.type_text("3"));
    b.confirm();
    let made = copies(&b, &[object, path]);
    assert!(
        near2(made[1][0], [10.0, 11.0]) && near2(made[1][1], [11.0, 11.0]),
        "{:?}",
        made[1]
    );
    assert!(!b.memory.path_align);
}

#[test]
fn the_objects_can_be_picked_after_the_tool_starts() {
    let mut b = Bench::new("arrayPath");
    let object = b.add_line("cizim", [0.0, 1.0], [1.0, 1.0]);
    let _path = b.add_line("cizim", [0.0, 0.0], [20.0, 0.0]);
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Yol boyunca dizi: nesnelere tıklayın ya da pencereyle seçin")
    );
    b.click(0.5, 1.0);
    assert_eq!(b.selected(), [object.0]);
    b.confirm();
    assert!(b.session.prompt().text().contains("yolu seçin"));
    b.click(10.0, 0.0);
    assert!(b.type_text("2"));
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Yol boyunca dizi: 2 adet, 1 yeni nesne.")
    );
    // With nothing selected, Enter leaves.
    let mut b = Bench::new("arrayPath");
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn the_path_is_a_line_an_arc_a_circle_or_a_polyline_and_not_a_copied_object() {
    let (mut b, object, _) = bench();
    // A click on nothing, and on the object being copied.
    b.click(5.0, 5.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .expect("said")
            .starts_with("Yol olarak bir çizgiye")
    );
    b.click(0.5, 1.0);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .expect("said")
            .starts_with("Yol, kopyalanacak nesnelerden biri olamaz")
    );
    assert!(b.session.prompt().text().contains("yolu seçin"));
    let _ = object;

    // A closed area is no path; a circle is: the copies go round it from its east point.
    let mut b = empty();
    let object = b.add_line("cizim", [0.0, 0.0], [0.0, 1.0]);
    b.doc
        .add(Entity::Circle(kentos_contracts::CircleEntity {
            base: common::base("cizim"),
            c: kentos_contracts::Vec2 {
                x: common::E + 10.0,
                y: common::N,
            },
            r: 10.0,
        }))
        .expect("a slot");
    b.add_path("cizim", &[[30.0, 0.0], [40.0, 0.0], [40.0, 10.0]], true);
    b.selection.set([object]);
    b.start("arrayPath");
    b.click(35.0, 0.0);
    assert_eq!(b.last_level(), Some(Level::Warn), "a closed area");
    b.click(20.0, 0.0);
    assert!(b.session.prompt().text().contains("adedi yazın"));
    assert!(b.type_text("4"));
    // Places at a quarter of the circle, the copies not turned when Hizala is off; count 4: 3 copies.
    assert!(b.type_text("H"));
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Yol boyunca dizi: 4 adet, 3 yeni nesne.")
    );
}

#[test]
fn esc_lets_go_of_the_path_then_leaves() {
    let (mut b, ..) = bench();
    b.click(10.0, 0.0);
    assert!(b.session.prompt().text().contains("adedi yazın"));
    assert!(b.run(|s, cx| s.cancel(cx)), "the path is let go");
    assert!(b.session.prompt().text().contains("yolu seçin"));
    assert!(!b.run(|s, cx| s.cancel(cx)), "the tool leaves");
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn a_spacing_that_does_not_fit_and_a_bad_count_are_said() {
    let (mut b, ..) = bench();
    b.click(10.0, 0.0);
    assert!(b.type_text("1"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(b.type_text("2.5"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(b.type_text("10001"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.memory.path_count, 5, "none of them is kept");
    // A spacing longer than the path: nothing fits, and Enter says so.
    assert!(b.type_text("A"));
    assert!(b.type_text("50"));
    b.move_to(10.0, 4.0);
    assert_eq!(preview(&b).tag.expect("a tag").lines, ["Sığmıyor"]);
    let before = b.doc.len();
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .expect("said")
            .starts_with("50.000 m aralık yola sığmıyor")
    );
    assert_eq!(b.doc.len(), before);
    assert!(b.session.is_running());
    assert!(b.type_text("-3"));
    assert_eq!(b.last_level(), Some(Level::Warn));
    // A number before the path is chosen.
    let (mut b, ..) = bench();
    assert!(b.type_text("4"));
    assert_eq!(b.last_text(), Some("Önce dizinin yolunu seçin."));
    assert!(!b.type_text("pekçok"));
}

#[test]
fn objects_on_a_locked_layer_are_refused() {
    let mut b = Bench::on(EDITS);
    b.draft.snap = false;
    // The locked line 13 as the object, the line 1 as the path.
    b.selection.set([Slot(13)]);
    b.start("arrayPath");
    b.click(-26.0, 12.0);
    let before = b.doc.len();
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.doc.len(), before);
    assert!(b.session.is_running(), "the tool waits for another try");
}
