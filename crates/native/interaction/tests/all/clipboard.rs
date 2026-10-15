//! The clipboard and the paste tool through the session, over the native
//! document: the web's `Clipboard`, its commands and `PasteTool`, one rule
//! per test (docs/adr/0056), writing through `cad.entities.copy` and
//! `cad.entities.paste` (docs/adr/0309). The drawing is the traces' `objects.kcad`:
//! lines 1–3, the closed area 4, the point 5, a line on a locked layer (6)
//! and one on a hidden layer (7). Expected values are worked out by hand.

use crate::common;

use common::{Bench, E, N, rel};
use kentos_contracts::{Clip, Entity};
use kentos_domain::Slot;
use kentos_interaction::clipboard::{self, Clipboard};
use kentos_interaction::paste::Paste;
use kentos_interaction::{Level, Vec2};

const OBJECTS: &str = include_str!("../../../../../fixtures/interaction/v1/objects.kcad");

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
        .run(Box::new(Paste::new(board.clip().expect("a copy").clone())));
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
        ["line", "polygon"],
        "in the drawing's order, whatever the selection's"
    );
    // Line 1 runs from (−24, −12); the area reaches up to 14: the box's lower left is (−24, −12).
    assert_eq!(board.base(), Vec2::new(E - 24.0, N - 12.0));
    let numbers: Vec<u32> = board.items().iter().map(|e| e.base().id).collect();
    assert_eq!(
        numbers,
        [1, 2],
        "numbered in the copy, not slots of the drawing"
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

/// The copy with its first layer renamed, as if it came from another drawing.
fn from_elsewhere(board: &Clipboard, name: &str) -> Clip {
    let mut clip = board.clip().expect("a copy").clone();
    let old = clip.layers[0].id.clone();
    clip.layers[0].id = "baska-cizimden".into();
    clip.layers[0].name = name.into();
    clip.layers[0].locked = false;
    for e in &mut clip.objects {
        if e.base().layer_id == old {
            e.base_mut().layer_id = "baska-cizimden".into();
        }
    }
    clip
}

#[test]
fn an_object_goes_to_its_layers_path_and_a_missing_one_is_made() {
    let mut b = bench(&[1]);
    let board = copy(&mut b);
    let clip = from_elsewhere(&board, "Yol ekseni");
    let layers = b.doc.layers().leaves().len();
    let before = b.log.len();
    let slots = b.run(|_, cx| clipboard::paste(&clip, None, None, cx));
    assert_eq!(slots, [Slot(8)]);
    assert_eq!(
        b.said(before),
        [(
            Level::Success,
            "1 nesne yapıştırıldı: 1 katman açıldı (Yol ekseni)."
        )]
    );
    assert_eq!(b.doc.layers().leaves().len(), layers + 1);
    let layer = b
        .doc
        .get(Slot(8))
        .map(|e| e.base().layer_id.clone())
        .expect("pasted");
    assert_eq!(b.doc.layers().path(&layer), "Yol ekseni");
    // The same name again, in other letters: the layer made is the one used.
    let again = from_elsewhere(&board, "YOL EKSENİ");
    let slots = b.run(|_, cx| clipboard::paste(&again, None, None, cx));
    assert_eq!(
        b.doc.get(slots[0]).map(|e| e.base().layer_id.clone()),
        Some(layer)
    );
    assert_eq!(b.doc.layers().leaves().len(), layers + 1);
    // One step takes the object and its layer back.
    assert_eq!(b.doc.undo().as_deref(), Some("Yapıştır"));
    assert_eq!(b.doc.undo().as_deref(), Some("Yapıştır"));
    assert_eq!(b.doc.layers().leaves().len(), layers);
}

#[test]
fn a_locked_layer_refuses_the_whole_paste() {
    // Line 1 is on Çizim; the area 4 on Parsel.
    let mut b = bench(&[1, 4]);
    let board = copy(&mut b);
    b.doc.toggle_layer_locked("cizim");
    paste_tool(&mut b, &board);
    let before = b.log.len();
    b.click(0.0, 0.0);
    assert_eq!(
        b.said(before),
        [(
            Level::Warn,
            "“Çizim” katmanı kilitli; yapıştırılamadı. Kilidi Katmanlar panelinden açın."
        )]
    );
    assert_eq!(b.doc.len(), 7, "nothing pasted, not even the area");
    assert_eq!(
        b.session.tool_id(),
        "select",
        "the tool leaves all the same"
    );
    assert_eq!(b.selected(), [1, 4], "the selection stays");
    // A copy of the locked line 6: its layer is locked, so it is refused too.
    let mut locked = bench(&[6]);
    let board = copy(&mut locked);
    let slots = locked.run(|_, cx| clipboard::paste(board.clip().expect("a copy"), None, None, cx));
    assert!(slots.is_empty());
    assert_eq!(locked.doc.len(), 7);
    // Objects whose layers are open go, onto them, moved by the point given.
    b.selection.set([Slot(4)]);
    let board = copy(&mut b);
    let to = Vec2::new(board.base().x + 1.0, board.base().y);
    let slots = b.run(|_, cx| clipboard::paste(board.clip().expect("a copy"), Some(to), None, cx));
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
    // Every field the same but the slot, in the drawing's order: the line, the point with its attributes.
    assert_eq!(line(&b, 8), line(&b, 2));
    assert_ne!(b.doc.uid(Slot(8)), b.doc.uid(Slot(2)));
    let mut point = b.doc.get(Slot(5)).cloned().expect("the point");
    point.base_mut().id = 9;
    assert_eq!(b.doc.get(Slot(9)), Some(&point));
    assert_eq!(b.doc.undo().as_deref(), Some("Yapıştır"));
}

#[test]
fn a_pasted_object_keeps_its_own_line_weight() {
    // As its colour (docs/adr/0139): 0 is the thinnest line, not “none”.
    let mut b = bench(&[2]);
    let board = copy(&mut b);
    for weight in [Some(0.7), Some(0.0), None] {
        let mut clip = board.clip().expect("a copy").clone();
        clip.objects[0].base_mut().line_weight = weight;
        let mut own = Clipboard::new();
        own.set(clip);
        let slots = b.run(|_, cx| clipboard::paste_in_place(&own, cx));
        let pasted = b.doc.get(slots[0]).expect("pasted");
        assert_eq!(pasted.base().line_weight, weight);
    }
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

/// Yapıştır writes through `cad.entities.paste` in one step “Yapıştır”: a
/// hidden layer that takes some is said once; a copy that does not hold
/// together is refused and nothing is written.
#[test]
fn a_paste_goes_through_the_command_in_one_step() {
    let mut b = bench(&[1, 7]);
    let board = copy(&mut b);
    let count = b.doc.len();
    let before = b.log.len();
    let slots = b.run(|_, cx| clipboard::paste_in_place(&board, cx));
    assert_eq!(slots.len(), 2);
    assert_eq!(
        b.said(before),
        [
            (
                Level::Warn,
                "“Gizli katman” katmanı gizli; yapıştırılan nesneler görünmeyecek."
            ),
            (Level::Success, "2 nesne yapıştırıldı."),
        ]
    );
    assert_eq!(b.doc.len(), count + 2);
    assert_eq!(b.doc.undo().as_deref(), Some("Yapıştır"));
    assert_eq!(b.doc.len(), count);

    // An object on a layer the copy does not hold: refused, nothing written.
    let mut broken = board.clip().expect("a copy").clone();
    broken.objects[0].base_mut().layer_id = "yok".into();
    let mut held = Clipboard::new();
    held.set(broken);
    let revision = b.doc.revision();
    let slots = b.run(|_, cx| clipboard::paste_in_place(&held, cx));
    assert!(slots.is_empty());
    assert_eq!((b.doc.len(), b.doc.revision()), (count, revision));
    assert_eq!(b.last_level(), Some(Level::Warn));
}

#[test]
fn a_paste_as_a_block_makes_one_block_and_places_it() {
    let mut b = bench(&[1, 2]);
    let board = copy(&mut b);
    b.session.run(Box::new(Paste::as_block(
        board.clip().expect("a copy").clone(),
    )));
    b.run(|s, cx| s.activate(cx));
    assert_eq!(
        b.session.prompt().text(),
        "Blok olarak yapıştır: yerleştirme noktasını belirtin ya da koordinat yazın"
    );
    let before = b.log.len();
    b.click(-4.0, -18.0);
    assert_eq!(
        b.said(before),
        [(Level::Success, "2 nesne yapıştırıldı: “Pano” bloğu olarak.")]
    );
    assert_eq!(ids(&b), [1, 2, 3, 4, 5, 6, 7, 8]);
    let Some(Entity::Insert(insert)) = b.doc.get(Slot(8)) else {
        panic!("one insert");
    };
    assert_eq!(at(Vec2::new(insert.p.x, insert.p.y)), [-4.0, -18.0]);
    assert_eq!(b.doc.blocks().len(), 1);
    assert_eq!(b.doc.blocks()[0].name, "Pano");
    // A second time the name is taken.
    b.session.run(Box::new(Paste::as_block(
        board.clip().expect("a copy").clone(),
    )));
    b.run(|s, cx| s.activate(cx));
    b.click(4.0, -18.0);
    let names: Vec<&str> = b.doc.blocks().iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["Pano", "Pano (2)"]);
    assert_eq!(b.doc.undo().as_deref(), Some("Yapıştır"));
    assert_eq!(b.doc.blocks().len(), 1, "the block goes with its insert");
}

#[test]
fn copy_with_a_base_point_hands_the_copy_to_the_host() {
    let mut b = bench(&[1]);
    b.start("copyBase");
    assert_eq!(
        b.session.prompt().text(),
        "Taban noktasıyla kopyala: taban noktasına tıklayın ya da Y,X yazın"
    );
    let revision = b.doc.revision();
    b.click(-20.0, -12.0);
    assert!(!b.session.is_running(), "the tool leaves");
    assert_eq!(b.doc.revision(), revision, "the drawing does not change");
    let clip = b
        .views
        .iter()
        .find_map(|c| match c {
            kentos_interaction::ViewChange::Copied(clip) => Some(clip.clone()),
            _ => None,
        })
        .expect("the copy for the clipboard");
    assert_eq!((clip.base.x - E, clip.base.y - N), (-20.0, -12.0));
    assert_eq!(clip.objects.len(), 1);
    // Pasted at (0, 0): the line moves by what the base point moves.
    let mut board = Clipboard::new();
    board.set(*clip);
    paste_tool(&mut b, &board);
    b.click(0.0, 0.0);
    assert_eq!(line(&b, 8), [[-4.0, 0.0], [12.0, 0.0]]);
}
