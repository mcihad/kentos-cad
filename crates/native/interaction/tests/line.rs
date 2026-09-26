//! The line tool through the session, over the native document: the
//! behaviour of the web's `LineTool` that the interaction traces
//! (`line-chain`) and ADR 0018 fix, one rule per test (docs/adr/0027).

mod common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_interaction::Level;

/// The newest object as a line: its ends relative to (E, N).
fn newest(b: &Bench) -> [[f64; 2]; 2] {
    let Entity::Line(line) = b.newest() else {
        panic!("a line, not {:?}", b.newest());
    };
    [rel(line.a), rel(line.b)]
}

#[test]
fn prompts_are_the_web_s_text_and_options() {
    let mut b = Bench::new("line");
    assert_eq!(b.session.prompt().text(), "Çizgi: ilk noktayı belirtin");
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Çizgi: sonraki noktayı belirtin [Bitir (Enter)]"
    );
    b.click(10.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Çizgi: sonraki noktayı belirtin [Geri (G) / Bitir (Enter)]"
    );
    b.click(10.0, 10.0);
    assert_eq!(
        b.session.prompt().text(),
        "Çizgi: sonraki noktayı belirtin [Geri (G) / Kapat (K) / Bitir (Enter)]"
    );
    assert_eq!(b.options(), ["G", "K", "Enter"]);
}

#[test]
fn every_segment_is_its_own_object_and_undo_step() {
    let mut b = Bench::new("line");
    b.click(0.0, 0.0);
    assert_eq!(b.doc.len(), 0, "one point draws nothing");
    b.move_to(20.0, 0.0);
    assert!(b.type_text("12"), "a distance towards the cursor");
    assert!(b.type_text("@0,8"));
    assert_eq!((b.points(), b.doc.len()), (3, 2));
    assert_eq!(newest(&b), [[12.0, 0.0], [12.0, 8.0]]);
    let before = b.log.len();
    b.confirm();
    assert_eq!(
        b.said(before),
        [(Level::Success, "2 çizgi eklendi.")],
        "the web's message"
    );
    assert_eq!(
        (b.session.tool_id(), b.points()),
        ("line", 0),
        "ready for the next chain"
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
    assert_eq!(b.doc.len(), 1, "one step took one line back");
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
    assert_eq!(b.doc.len(), 0);
}

#[test]
fn a_second_point_on_the_same_place_adds_nothing() {
    let mut b = Bench::new("line");
    b.click(0.0, 0.0);
    b.click(0.0, 0.0);
    assert_eq!((b.points(), b.doc.len()), (1, 0));
}

#[test]
fn geri_takes_the_last_line_back_as_an_undo() {
    let mut b = Bench::new("line");
    for (de, dn) in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)] {
        b.click(de, dn);
    }
    assert!(b.type_text("g"), "G is Geri");
    assert_eq!((b.points(), b.doc.len()), (2, 1));
    assert!(
        b.doc.can_redo(),
        "an undo: nothing changed since, so the line can be redone"
    );
    // After an unrelated change the line is removed instead: a later redo cannot bring the undone step back.
    b.click(10.0, 10.0);
    b.doc.toggle_layer_visible("cizim");
    assert!(b.type_text("G"));
    assert_eq!((b.points(), b.doc.len()), (2, 1));
    assert!(!b.doc.can_redo());
    assert_eq!(b.doc.undo().as_deref(), Some("Sil"));
    // With no line left, G is not an option.
    let mut first = Bench::new("line");
    first.click(0.0, 0.0);
    assert!(!first.type_text("G"));
}

#[test]
fn ctrl_z_takes_the_newest_step_first() {
    let mut b = Bench::new("line");
    for (de, dn) in [(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)] {
        b.click(de, dn);
    }
    assert!(b.undo_step(), "the chain's last line");
    assert_eq!((b.points(), b.doc.len()), (2, 1));
    assert!(b.undo_step());
    assert_eq!((b.points(), b.doc.len()), (1, 0));
    assert!(
        b.undo_step(),
        "the draft's first point: the chain starts over"
    );
    assert_eq!(b.points(), 0);
    assert!(
        !b.undo_step(),
        "nothing pending: the drawing is undone instead"
    );
    assert_eq!(b.session.tool_id(), "line");
}

#[test]
fn kapat_draws_back_to_the_first_point_and_ends_the_chain() {
    let mut b = Bench::new("line");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert!(!b.type_text("K"), "two points: nothing to close");
    b.click(10.0, 10.0);
    let before = b.log.len();
    assert!(b.type_text("k"));
    assert_eq!((b.points(), b.doc.len()), (0, 3));
    assert_eq!(newest(&b), [[10.0, 10.0], [0.0, 0.0]]);
    assert_eq!(b.said(before), [(Level::Success, "3 çizgi eklendi.")]);
    assert_eq!(b.session.tool_id(), "line");
}

#[test]
fn a_confirm_with_one_point_writes_nothing_and_with_none_leaves() {
    let mut b = Bench::new("line");
    b.click(0.0, 0.0);
    let before = b.log.len();
    b.confirm();
    assert!(b.said(before).is_empty(), "no line, no message");
    assert_eq!(
        (b.points(), b.doc.len(), b.session.tool_id()),
        (0, 0, "line")
    );
    b.confirm();
    assert!(!b.session.is_running());
    assert_eq!(b.session.last(), Some("line"), "Enter repeats it");
}

#[test]
fn a_locked_layer_takes_no_line_and_says_how_to_fix_it() {
    let mut b = Bench::new("line");
    b.doc.toggle_layer_locked("cizim");
    b.click(0.0, 0.0);
    b.click(5.0, 0.0);
    assert_eq!((b.points(), b.doc.len()), (1, 0), "the point is not taken");
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some(
            "“Çizim” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
        )
    );
}

#[test]
fn a_hidden_layer_takes_each_line_with_a_warning() {
    let mut b = Bench::new("line");
    b.doc.toggle_layer_visible("cizim");
    b.click(0.0, 0.0);
    let before = b.log.len();
    b.click(5.0, 0.0);
    assert_eq!(b.doc.len(), 1);
    assert_eq!(
        b.said(before),
        [
            (Level::Info, "  Y 487005.000  X 4420000.000"),
            (
                Level::Warn,
                "“Çizim” katmanı gizli; çizilen nesne görünmeyecek."
            ),
        ]
    );
}

#[test]
fn shift_turns_ortho_on_for_a_point() {
    let mut b = Bench::new("line");
    b.click(0.0, 0.0);
    let mut p = Bench::pointer(10.0, 3.0);
    p.shift = true;
    b.run(|s, cx| s.pointer_down(&p, cx));
    assert_eq!(newest(&b), [[0.0, 0.0], [10.0, 0.0]]);
}

#[test]
fn the_preview_follows_the_cursor_and_measures() {
    let mut b = Bench::new("line");
    let format = kentos_interaction::Format::default();
    b.move_to(3.0, 3.0);
    let preview = b.session.preview(&format).expect("a tool runs");
    assert_eq!(preview.path.len(), 1, "the cursor alone");
    assert!(preview.tag.is_none());
    b.click(0.0, 0.0);
    b.move_to(12.0, 0.0);
    let preview = b.session.preview(&format).expect("a tool runs");
    assert_eq!(preview.path.len(), 2);
    assert!(preview.ring.is_none());
    let tag = preview.tag.expect("a measurement beside the cursor");
    assert_eq!(tag.lines, ["12.000 m", "Semt 100.0000 g"]);
    assert_eq!(b.doc.len(), 0);
}
