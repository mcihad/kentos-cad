//! The polyline tool through the session, over the native document: the
//! open form of the closed-area tool (the web's `PathTool` with `closed:
//! false`), one rule per test (docs/adr/0027). What both shapes share (arc
//! options, typed values, Uzunluk) is tested with the closed area
//! (tests/polygon.rs); here is what differs.

mod common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_interaction::Level;

/// The newest object as a polyline: its points relative to (E, N), and its bulges.
fn newest(b: &Bench) -> (Vec<[f64; 2]>, Option<Vec<f64>>) {
    let Entity::Polyline(path) = b.newest() else {
        panic!("a polyline, not {:?}", b.newest());
    };
    (
        path.pts.iter().map(|p| rel(*p)).collect(),
        path.bulges.clone(),
    )
}

#[test]
fn prompts_are_the_web_s_text_and_options() {
    let mut b = Bench::new("polyline");
    assert_eq!(
        b.session.prompt().text(),
        "Çoklu çizgi: ilk noktayı belirtin"
    );
    b.click(0.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Çoklu çizgi: sonraki noktayı belirtin [Yay (Y) / Uzunluk (U) / Geri (G)]"
    );
    b.click(10.0, 0.0);
    assert_eq!(b.options(), ["Y", "U", "G", "Enter"], "two points finish");
    assert!(b.type_text("Y"));
    assert_eq!(
        b.session.prompt().text(),
        "Çoklu çizgi: yayın bitiş noktasını belirtin [Düz (D) / Açı (A) / Merkez (M) / Yarıçap (R) / İkinci nokta (İ) / Doğrultu (T) / Geri (G) / Bitir (Enter)]"
    );
}

#[test]
fn a_confirm_writes_one_polyline_in_one_undo_step() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.move_to(20.0, 0.0);
    assert!(b.type_text("12"));
    assert!(b.type_text("@0,8"));
    assert_eq!(b.doc.len(), 0, "the preview does not touch the drawing");
    let before = b.log.len();
    b.confirm();
    assert_eq!(
        b.said(before),
        [(Level::Success, "Çoklu çizgi eklendi: 20.000 m")]
    );
    assert_eq!(
        newest(&b),
        (vec![[0.0, 0.0], [12.0, 0.0], [12.0, 8.0]], None)
    );
    assert_eq!((b.session.tool_id(), b.points()), ("polyline", 0));
    assert_eq!(b.doc.undo().as_deref(), Some("Ekle"));
    assert_eq!(b.doc.len(), 0);
}

#[test]
fn an_arc_segment_is_kept_with_one_bulge_per_point() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert!(b.type_text("Y"));
    b.click(10.0, 10.0);
    assert!(b.type_text("D"));
    b.click(0.0, 10.0);
    b.confirm();
    let (pts, bulges) = newest(&b);
    assert_eq!(pts.len(), 4);
    let bulges = bulges.expect("an arc");
    assert_eq!(
        bulges.len(),
        4,
        "the document's per-point form, as the web writes it"
    );
    assert!((bulges[1] - 1.0).abs() < 1e-12, "a tangent half circle");
    assert_eq!([bulges[0], bulges[2], bulges[3]], [0.0, 0.0, 0.0]);
}

#[test]
fn the_first_point_again_is_a_point_not_a_close() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    b.click(10.0, 10.0);
    b.click(0.0, 0.0);
    assert_eq!(
        (b.points(), b.doc.len()),
        (4, 0),
        "an open polyline never closes itself"
    );
}

#[test]
fn fewer_than_two_points_warn_and_write_nothing() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.confirm();
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("Çoklu çizgi için en az 2 nokta gerekir.")
    );
    assert_eq!(
        (b.doc.len(), b.points(), b.session.tool_id()),
        (0, 0, "polyline")
    );
}

#[test]
fn ctrl_z_takes_back_the_draft_first_then_hands_over() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    assert!(b.undo_step());
    assert_eq!(b.points(), 1);
    assert!(b.undo_step());
    assert_eq!(b.points(), 0);
    assert!(
        !b.undo_step(),
        "nothing pending: the drawing is undone instead"
    );
}

#[test]
fn a_locked_layer_takes_nothing_and_a_hidden_one_warns() {
    let mut b = Bench::new("polyline");
    b.doc.toggle_layer_locked("cizim");
    b.click(0.0, 0.0);
    b.click(5.0, 0.0);
    b.confirm();
    assert_eq!(b.doc.len(), 0);
    assert_eq!(
        b.last_text(),
        Some(
            "“Çizim” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin."
        )
    );
    let mut h = Bench::new("polyline");
    h.doc.toggle_layer_visible("cizim");
    h.click(0.0, 0.0);
    h.click(5.0, 0.0);
    let before = h.log.len();
    h.confirm();
    assert_eq!(h.doc.len(), 1);
    assert_eq!(
        h.said(before),
        [
            (
                Level::Warn,
                "“Çizim” katmanı gizli; çizilen nesne görünmeyecek."
            ),
            (Level::Success, "Çoklu çizgi eklendi: 5.000 m"),
        ]
    );
}

#[test]
fn the_preview_has_no_area() {
    let mut b = Bench::new("polyline");
    b.click(0.0, 0.0);
    b.click(12.0, 0.0);
    b.move_to(12.0, 8.0);
    let format = kentos_interaction::Format::default();
    let preview = b.session.preview(&format).expect("a tool runs");
    assert_eq!(preview.path.len(), 3);
    assert!(preview.ring.is_none(), "no closing edge");
    let tag = preview.tag.expect("a measurement beside the cursor");
    assert_eq!(tag.lines, ["8.000 m", "Semt 0.0000 g"]);
}
