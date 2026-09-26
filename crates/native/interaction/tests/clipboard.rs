//! The clipboard and the paste tool through the session, over the native
//! document: the web's `Clipboard`, its commands and `PasteTool`, one rule
//! per test (docs/adr/0056). The drawing is the traces' `objects.kcad`:
//! lines 1–3, the closed area 4, the point 5, a line on a locked layer (6)
//! and one on a hidden layer (7). Expected values are worked out by hand.

mod common;

use common::{Bench, E, N, rel};
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::clipboard::{self, Clipboard};
use kentos_interaction::paste::Paste;
use kentos_interaction::{Level, Vec2};

const OBJECTS: &str = include_str!("../../../../fixtures/interaction/v1/objects.kcad");

/// The objects drawing with nothing running and snapping off; `selected` selected.
fn bench(selected: &[u32]) -> Bench {
    let mut b = Bench::on(OBJECTS);
    b.draft.snap = false;
    b.selection.set(selected.iter().map(|s| Slot(*s)));
    b
}

fn copy(b: &mut Bench) -> Clipboard {
    let mut board = Clipboard::new();
    b.run(|_, cx| clipboard::copy(&mut board, cx));
    board
}

/// `edit.paste`: the paste tool with the clipboard's objects, as the desktop runs it.
fn paste_tool(b: &mut Bench, board: &Clipboard) {
    b.session
        .run(Box::new(Paste::new(board.items().to_vec(), board.base())));
    b.run(|s, cx| s.activate(cx));
}

fn line(b: &Bench, slot: u32) -> [[f64; 2]; 2] {
    let Some(Entity::Line(l)) = b.doc.get(Slot(slot)) else {
        panic!("line {slot}");
    };
    [rel(l.a), rel(l.b)]
}

fn ids(b: &Bench) -> Vec<u32> {
    b.doc.entities().map(|e| e.base().id).collect()
}

/// A world point as east and north differences from (E, N).
fn at(p: Vec2) -> [f64; 2] {
    [p.x - E, p.y - N]
}

#[test]
fn copy_puts_the_selection_aside_in_its_order_with_its_lower_left_corner() {
    let mut b = bench(&[4, 1]);
    let before = b.doc.revision();
    let board = copy(&mut b);
    let order: Vec<&str> = board.items().iter().map(Entity::kind).collect();
    assert_eq!(
        order,
        ["polygon", "line"],
        "in the order they were selected"
    );
    // Line 1 runs from (−24, −12); the area reaches up to 14: the box's lower left is (−24, −12).
    assert_eq!(board.base(), Vec2::new(E - 24.0, N - 12.0));
    assert!(
        board.items().iter().all(|e| e.base().id == 0),
        "copies have no slot"
    );
    assert_eq!(b.last_text(), Some("2 nesne panoya kopyalandı."));
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.doc.revision(), before, "copying is no edit");
    assert!(!b.doc.can_undo());
    // With nothing selected the command is off: nothing said, nothing kept.
    let mut empty = bench(&[]);
    let board = copy(&mut empty);
    assert!(board.is_empty() && empty.log.is_empty());
}

#[test]
fn cut_leaves_locked_objects_selected_and_is_one_undo_step() {
    let mut b = bench(&[6, 2]);
    let mut board = Clipboard::new();
    let n = b.run(|_, cx| clipboard::cut(&mut board, cx));
    assert_eq!(n, 1);
    assert_eq!(
        b.said(0),
        [
            (
                Level::Warn,
                "1 nesne kilitli katmanda olduğu için kesilmedi."
            ),
            (Level::Success, "1 nesne panoya kesildi."),
        ]
    );
    assert_eq!(ids(&b), [1, 3, 4, 5, 6, 7]);
    assert_eq!(b.selected(), [6], "the locked one stays, selected");
    assert_eq!(board.len(), 1);
    assert_eq!(board.base(), Vec2::new(E, N + 4.0));
    assert_eq!(b.doc.undo().as_deref(), Some("Kes"));
    assert_eq!(ids(&b), [1, 2, 3, 4, 5, 6, 7], "back in its place");
    // All locked: a warning, and the clipboard keeps what it had.
    let mut locked = bench(&[6]);
    let n = locked.run(|_, cx| clipboard::cut(&mut board, cx));
    assert_eq!(n, 0);
    assert_eq!(locked.last_level(), Some(Level::Warn));
    assert_eq!(board.len(), 1);
    assert_eq!(board.items()[0].kind(), "line");
    assert!(!locked.doc.can_undo());
}

#[test]
fn the_paste_tool_places_the_base_point_where_clicked() {
    let mut b = bench(&[1]);
    let board = copy(&mut b);
    paste_tool(&mut b, &board);
    assert_eq!(b.session.tool_id(), "paste");
    assert_eq!(
        b.session.prompt().text(),
        "Yapıştır: yerleştirme noktasını belirtin ya da koordinat yazın"
    );
    assert!(b.options().is_empty());
    // The ghost follows the cursor by the base point, dashed.
    b.move_to(-4.0, -18.0);
    let preview = b.session.preview(&kentos_interaction::Format::default());
    let ghost = &preview.expect("a preview").strokes[0];
    assert_eq!(ghost.dash, Some([4.0, 3.0]));
    let ghost: Vec<[f64; 2]> = ghost.pts.iter().map(|p| at(*p)).collect();
    assert_eq!(ghost, [[-4.0, -18.0], [12.0, -18.0]]);
    let before = b.log.len();
    b.click(-4.0, -18.0);
    assert_eq!(line(&b, 8), [[-4.0, -18.0], [12.0, -18.0]]);
    assert_eq!(b.said(before), [(Level::Success, "1 nesne yapıştırıldı.")]);
    assert_eq!(b.selected(), [8], "the pasted objects are selected");
    assert_eq!(b.session.tool_id(), "select", "the tool leaves");
    assert_eq!(b.doc.undo().as_deref(), Some("Yapıştır"));
    assert_eq!(ids(&b), [1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn pasted_objects_are_new_every_time() {
    let mut b = bench(&[2, 3]);
    let board = copy(&mut b);
    let originals: Vec<_> = [2, 3].iter().map(|s| b.doc.uid(Slot(*s))).collect();
    for at in [[-20.0, -16.0], [4.0, -16.0]] {
        paste_tool(&mut b, &board);
        b.click(at[0], at[1]);
    }
    assert_eq!(ids(&b), [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    let pasted: Vec<_> = (8..=11).map(|s| b.doc.uid(Slot(s))).collect();
    for (i, uid) in pasted.iter().enumerate() {
        assert!(uid.is_some());
        assert!(!originals.contains(uid), "a paste is a new object");
        assert!(!pasted[i + 1..].contains(uid), "each paste is its own");
    }
    // The copies keep their layer and attributes; they were placed from (0, 4), the box's corner.
    assert_eq!(line(&b, 8), [[-20.0, -16.0], [4.0, -4.0]]);
    assert_eq!(line(&b, 11), [[4.0, -4.0], [20.0, -16.0]]);
    assert_eq!(
        b.doc.get(Slot(8)).map(|e| e.base().layer_id.as_str()),
        Some("cizim")
    );
}

#[test]
fn a_typed_point_is_measured_from_the_base_point() {
    let mut b = bench(&[1]);
    let board = copy(&mut b);
    paste_tool(&mut b, &board);
    assert!(b.type_text("@10,0"));
    // Exactly: the base (−24, −12) moved 10 east.
    let Some(Entity::Line(l)) = b.doc.get(Slot(8)) else {
        panic!("pasted");
    };
    assert_eq!(
        (l.a.x, l.a.y, l.b.x, l.b.y),
        (E - 14.0, N - 12.0, E + 2.0, N - 12.0)
    );
    assert_eq!(b.session.tool_id(), "select");
    // A coordinate is where the base point goes; text that is no point is refused.
    paste_tool(&mut b, &board);
    assert!(!b.type_text("K"));
    assert!(b.type_text(&format!("{},{}", E + 1.0, N + 2.0)));
    assert_eq!(line(&b, 9), [[1.0, 2.0], [17.0, 2.0]]);
}

#[test]
fn enter_and_esc_leave_without_pasting() {
    let mut b = bench(&[1]);
    let board = copy(&mut b);
    paste_tool(&mut b, &board);
    b.move_to(0.0, 0.0);
    b.confirm();
    assert_eq!(b.session.tool_id(), "select");
    paste_tool(&mut b, &board);
    assert!(!b.run(|s, cx| s.cancel(cx)), "Esc: nothing to step back to");
    assert!(!b.session.is_running());
    assert_eq!(b.doc.len(), 7);
    assert!(!b.doc.can_undo());
    // Ctrl+Z while it runs undoes the drawing: the tool takes no steps of its own.
    paste_tool(&mut b, &board);
    assert!(!b.undo_step());
    assert_eq!(b.points(), 0);
}

#[test]
fn objects_go_to_the_active_layer_when_theirs_is_locked_or_gone() {
    // Copied from the locked line 6: its layer is locked, so the copy goes to the active one.
    let mut b = bench(&[6]);
    let board = copy(&mut b);
    paste_tool(&mut b, &board);
    b.click(0.0, 0.0);
    let active = b.doc.layers().active().to_owned();
    assert_eq!(
        b.doc.get(Slot(8)).map(|e| e.base().layer_id.clone()),
        Some(active.clone())
    );
    // A layer the drawing does not have (copied from another drawing): the active one too.
    let mut other = board.items().to_vec();
    other[0].base_mut().layer_id = "baska-cizimden".into();
    let mut moved = Clipboard::new();
    moved.set(other, None);
    assert_eq!(moved.base(), Vec2::new(0.0, 0.0), "no box: the origin");
    let slots = b.run(|_, cx| clipboard::paste(moved.items(), 0.0, 0.0, cx));
    assert_eq!(slots, [Slot(9)]);
    assert_eq!(
        b.doc.get(Slot(9)).map(|e| e.base().layer_id.clone()),
        Some(active)
    );
}

#[test]
fn a_locked_active_layer_refuses_what_would_go_there() {
    // Line 1 is on Çizim, the active layer; the area 4 on Parsel.
    let mut b = bench(&[1, 4]);
    let board = copy(&mut b);
    assert_eq!(b.doc.layers().active(), "cizim");
    b.doc.toggle_layer_locked("cizim");
    paste_tool(&mut b, &board);
    let before = b.log.len();
    b.click(0.0, 0.0);
    assert_eq!(
        b.said(before),
        [(Level::Warn, "“Çizim” katmanı kilitli; yapıştırılamadı.")]
    );
    assert_eq!(b.doc.len(), 7, "nothing pasted, not even the area");
    assert_eq!(
        b.session.tool_id(),
        "select",
        "the tool leaves all the same"
    );
    assert_eq!(b.selected(), [1, 4], "the selection stays");
    // Objects whose own layers are open still go, onto them.
    b.selection.set([Slot(4)]);
    let board = copy(&mut b);
    let slots = b.run(|_, cx| clipboard::paste(board.items(), 1.0, 0.0, cx));
    assert_eq!(slots, [Slot(8)]);
    assert_eq!(
        b.doc.get(Slot(8)).map(|e| e.base().layer_id.as_str()),
        Some("parsel")
    );
}

#[test]
fn paste_in_place_puts_copies_where_the_objects_were() {
    let mut b = bench(&[5, 2]);
    let board = copy(&mut b);
    let slots = b.run(|_, cx| clipboard::paste_in_place(&board, cx));
    assert_eq!(slots, [Slot(8), Slot(9)]);
    assert_eq!(b.selected(), [8, 9]);
    // Every field the same but the slot: the point with its attributes, the line.
    let mut point = b.doc.get(Slot(5)).cloned().expect("the point");
    point.base_mut().id = 8;
    assert_eq!(b.doc.get(Slot(8)), Some(&point));
    assert_eq!(line(&b, 9), line(&b, 2));
    assert_ne!(b.doc.uid(Slot(9)), b.doc.uid(Slot(2)));
    assert_eq!(b.doc.undo().as_deref(), Some("Yapıştır"));
}

#[test]
fn the_paste_tool_is_not_repeated() {
    let mut b = bench(&[1]);
    b.start("line");
    b.run(|s, _| s.exit());
    let board = copy(&mut b);
    paste_tool(&mut b, &board);
    assert_eq!(b.session.last(), Some("line"), "Yapıştır is not remembered");
    // It snaps, as the web's does.
    b.draft.snap = true;
    let p = b.snapped(-7.9, -12.1);
    assert_eq!(p.world, Vec2::new(E - 8.0, N - 12.0), "line 1's end");
}
