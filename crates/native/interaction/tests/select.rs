//! Selecting, deleting and snapping over a drawing with objects (docs/adr/0029):
//! the web's `SelectTool`, `EraseTool` and viewport snap rules, one rule per
//! test, on the drawing the selection and snap traces use
//! (`fixtures/interaction/v1/objects.kcad`):
//!
//! | id | what | where (east, north from the view's centre) |
//! |---|---|---|
//! | 1 | line, Çizim | (-24, -12) → (-8, -12) |
//! | 2 | line, Çizim | (0, 4) → (24, 16) |
//! | 3 | line, Çizim | (0, 16) → (16, 4); crosses 2 at (9.6, 8.8) |
//! | 4 | closed area, Parsel | (-24, 4) … (-12, 14) |
//! | 5 | point, Nokta | (20, -8) |
//! | 6 | line, Kilitli katman (locked) | (4, -16) → (16, -16) |
//! | 7 | line, Gizli katman (hidden) | (-4, -4) → (4, -4) |

mod common;

use common::{Bench, E, N, rel};
use kentos_contracts::Entity;
use kentos_interaction::{Level, SnapKind, Spatial, Vec2};

const OBJECTS: &str = include_str!("../../../../fixtures/interaction/v1/objects.kcad");

fn objects() -> Bench {
    Bench::on(OBJECTS)
}

#[test]
fn a_click_picks_one_and_shift_turns_it_over() {
    let mut b = objects();
    b.click(-16.0, -12.3);
    assert_eq!(b.selected(), [1], "within the 5 px pick aperture of line 1");
    b.click(4.0, 6.2);
    assert_eq!(b.selected(), [2], "a click replaces the selection");
    b.shift = true;
    b.click(12.0, 7.2);
    assert_eq!(b.selected(), [2, 3], "Shift adds");
    b.click(4.0, 6.2);
    assert_eq!(b.selected(), [3], "and takes away");
    b.click(26.0, 18.0);
    assert_eq!(b.selected(), [3], "Shift on nothing keeps the selection");
    b.shift = false;
    b.click(26.0, 18.0);
    assert!(b.selected().is_empty(), "a click on nothing clears it");
}

#[test]
fn inside_a_closed_area_picks_it_and_edges_come_first() {
    let mut b = objects();
    b.click(-18.0, 9.0);
    assert_eq!(b.selected(), [4], "its interior");
    b.click(-12.1, 9.0);
    assert_eq!(b.selected(), [4], "and its edge");
}

#[test]
fn a_box_left_to_right_takes_what_is_inside_right_to_left_what_it_touches() {
    let mut b = objects();
    b.drag([-26.0, -14.0], [-6.0, 16.0]);
    assert_eq!(
        b.selected(),
        [1, 4],
        "a window: wholly inside, in document order"
    );
    b.drag([13.0, 11.0], [6.0, 7.0]);
    assert_eq!(b.selected(), [2, 3], "a crossing: 2 and 3 touch it");
    b.drag([6.0, 7.0], [13.0, 11.0]);
    assert!(
        b.selected().is_empty(),
        "the same box as a window holds nothing"
    );
    b.shift = true;
    b.drag([-26.0, -14.0], [-6.0, 16.0]);
    b.drag([13.0, 11.0], [6.0, 7.0]);
    assert_eq!(b.selected(), [1, 4, 2, 3], "Shift adds a box's objects");
}

#[test]
fn a_short_drag_is_a_click() {
    let mut b = objects();
    // 0.375 m is 3 px: under the 4 px that start a box.
    b.drag([-16.0, -12.0], [-15.625, -12.0]);
    assert_eq!(b.selected(), [1]);
}

#[test]
fn hidden_layers_are_never_picked_locked_ones_are() {
    let mut b = objects();
    b.click(10.0, -16.2);
    assert_eq!(b.selected(), [6], "the locked layer's line");
    b.click(0.0, -4.0);
    assert!(
        b.selected().is_empty(),
        "the hidden line is not there to click"
    );
    b.drag([-6.0, -6.0], [6.0, -2.0]);
    assert!(b.selected().is_empty(), "nor to box");
}

#[test]
fn the_pointer_hovers_what_a_click_would_pick() {
    let mut b = objects();
    b.move_to(4.0, 6.2);
    assert_eq!(b.selection.hover().map(|s| s.0), Some(2));
    b.move_to(26.0, 18.0);
    assert_eq!(b.selection.hover(), None);
    b.move_to(4.0, 6.2);
    b.start("polygon");
    assert_eq!(b.selection.hover(), None, "a command takes the pointer");
}

#[test]
fn erase_with_a_selection_deletes_it_in_one_step_and_leaves() {
    let mut b = objects();
    b.click(-16.0, -12.3);
    b.shift = true;
    b.click(4.0, 6.2);
    b.shift = false;
    let uids: Vec<_> = [1, 2]
        .map(|s| b.doc.uid(kentos_domain::Slot(s)).expect("an object"))
        .to_vec();
    let before = b.log.len();
    b.start("erase");
    assert!(!b.session.is_running(), "it leaves");
    assert_eq!(b.session.last(), Some("erase"), "Enter repeats it");
    assert_eq!(b.doc.len(), 5);
    assert!(b.selected().is_empty());
    assert_eq!(b.said(before), [(Level::Success, "2 nesne silindi.")]);
    assert_eq!(b.doc.undo().as_deref(), Some("Sil"), "one step");
    let ids: Vec<u32> = b.doc.entities().map(|e| e.base().id).collect();
    assert_eq!(ids, [1, 2, 3, 4, 5, 6, 7], "back in their places");
    let back: Vec<_> = [1, 2]
        .map(|s| b.doc.uid(kentos_domain::Slot(s)).expect("back"))
        .to_vec();
    assert_eq!(back, uids, "with their ids");
}

#[test]
fn erase_keeps_locked_objects_and_says_so() {
    let mut b = objects();
    b.click(10.0, -16.2);
    b.shift = true;
    b.click(12.0, 7.2);
    b.shift = false;
    let before = b.log.len();
    b.start("erase");
    assert_eq!(
        b.said(before),
        [
            (
                Level::Warn,
                "1 nesne kilitli katmanda olduğu için silinmedi. Silmek için katmanın kilidini Katmanlar panelinden açın."
            ),
            (Level::Success, "1 nesne silindi.")
        ]
    );
    assert_eq!(b.doc.len(), 6);
    assert_eq!(b.selected(), [6], "the locked line stays selected");
    // Alone, the locked line is refused: nothing changes.
    let before = b.log.len();
    b.start("erase");
    assert_eq!(b.doc.len(), 6);
    assert_eq!(b.said(before).len(), 1);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.selected(), [6]);
}

#[test]
fn erase_without_a_selection_deletes_what_is_clicked_until_esc() {
    let mut b = objects();
    b.start("erase");
    assert_eq!(b.session.tool_id(), "erase");
    assert_eq!(b.session.prompt().text(), "Sil: silinecek nesneye tıklayın");
    b.move_to(-18.0, 9.0);
    assert_eq!(b.selection.hover().map(|s| s.0), Some(4), "it hovers");
    b.click(-18.0, 9.0);
    assert_eq!(b.doc.len(), 6, "the closed area went");
    b.click(26.0, 18.0);
    assert_eq!(b.doc.len(), 6, "nothing there");
    assert_eq!(b.session.tool_id(), "erase", "it stays");
    b.session.exit();
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn a_tool_snaps_to_ends_middles_and_crossings_exactly() {
    let mut b = objects();
    b.start("polygon");
    b.move_to(-23.5, -11.6);
    let p = b.snapped(-23.5, -11.6);
    assert_eq!(p.snap.map(|s| s.kind), Some(SnapKind::Endpoint));
    b.click(-23.5, -11.6);
    b.click(-15.6, -11.5);
    b.click(9.2, 8.4);
    let p = b.snapped(9.2, 8.4);
    assert_eq!(p.snap.map(|s| s.kind), Some(SnapKind::Intersection));
    b.confirm();
    let Entity::Polygon(area) = b.newest() else {
        panic!("a closed area");
    };
    let corners: Vec<[f64; 2]> = area.pts.iter().map(|p| rel(*p)).collect();
    assert_eq!(corners[0], [-24.0, -12.0], "line 1's start, exactly");
    assert_eq!(corners[1], [-16.0, -12.0], "line 1's middle, exactly");
    let [x, y] = corners[2];
    assert!(
        (x - 9.6).abs() < 1e-9 && (y - 8.8).abs() < 1e-9,
        "the crossing of 2 and 3: {x}, {y}"
    );
}

#[test]
fn ortho_does_not_move_a_snapped_point_and_hidden_objects_do_not_snap() {
    let mut b = objects();
    b.draft.ortho = true;
    b.start("line");
    b.click(9.2, 8.4);
    // From the crossing, P's corner (-12, 4) is off the axes: ortho would move a free point.
    b.click(-12.4, 4.3);
    let Entity::Line(line) = b.newest() else {
        panic!("a line");
    };
    assert_eq!(rel(line.b), [-12.0, 4.0]);
    // The hidden line's end is not a snap point; the locked line's is.
    assert_eq!(b.snapped(3.7, -3.8).snap, None);
    assert_eq!(
        b.snapped(3.7, -15.8).snap.map(|s| s.kind),
        Some(SnapKind::Endpoint)
    );
    // Snapping off (F3): the point is the pointer's, and ortho applies.
    b.draft.snap = false;
    b.click(-20.25, 0.25);
    let Entity::Line(line) = b.newest() else {
        panic!("a line");
    };
    assert_eq!(rel(line.b), [-20.25, 4.0]);
}

#[test]
fn the_select_and_erase_tools_do_not_snap() {
    let mut b = objects();
    assert_eq!(b.snapped(-23.5, -11.6).snap, None, "idle: the select tool");
    b.start("erase");
    assert_eq!(b.snapped(-23.5, -11.6).snap, None);
}

/// The store follows the document by its journal, never reading every
/// object again, and holds exactly what a store read afresh holds: the same
/// objects in the same order, the same layer flags.
#[test]
fn the_store_follows_the_document_as_a_fresh_one_would() {
    let mut b = objects();
    let check = |b: &mut Bench, what: &str| {
        b.spatial.sync(&b.doc);
        let fresh = Spatial::of(&b.doc);
        let (have, want) = (b.spatial.store(), fresh.store());
        assert_eq!(have.ids(), want.ids(), "{what}: ids");
        for id in want.ids() {
            assert_eq!(have.item_json(id), want.item_json(id), "{what}: {id}");
            let (h, w) = (have.get(id).expect("held"), want.get(id).expect("held"));
            assert_eq!(have.flags(h), want.flags(w), "{what}: flags of {id}");
        }
        assert_eq!(b.spatial.reloads(), 1, "{what}: no reload");
    };
    check(&mut b, "opened");
    b.start("line");
    b.click(-2.0, -18.0);
    b.click(8.0, -18.0);
    check(&mut b, "a line drawn");
    b.session.exit();
    b.click(-16.0, -12.3);
    b.start("erase");
    check(&mut b, "line 1 deleted");
    b.doc.undo();
    check(&mut b, "undone");
    b.doc.redo();
    check(&mut b, "redone");
    b.doc.undo();
    let moved = {
        let mut e = b.doc.get(kentos_domain::Slot(2)).expect("line 2").clone();
        if let Entity::Line(l) = &mut e {
            l.b.x = E + 30.0;
            l.b.y = N + 20.0;
        }
        e
    };
    assert!(b.doc.update(kentos_domain::Slot(2), moved));
    check(&mut b, "line 2 changed");
    let failed: Result<(), ()> = b.doc.transact("Deneme", |doc| {
        doc.remove(&[kentos_domain::Slot(3)]);
        Err(())
    });
    assert!(failed.is_err());
    check(&mut b, "a failed transaction");
    b.doc.toggle_layer_visible("parsel");
    b.doc.toggle_layer_locked("cizim");
    check(&mut b, "layers changed");
    assert_eq!(
        b.spatial.pick(Vec2::new(E - 18.0, N + 9.0), 0.625),
        None,
        "the hidden layer's area is not picked"
    );
}
