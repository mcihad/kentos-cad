//! Yazı through the session, over the native document: the web's words, the
//! field it asks the host for, one undo step, the locked layer said at the
//! click, what it remembers (the web's `TextTool`, docs/adr/0060).
//! Expected values are worked out by hand, not taken from a run.

mod common;

use common::{Bench, rel};
use kentos_contracts::Entity;
use kentos_interaction::{Level, ViewChange};

const TOOLS: &str = include_str!("../../../../fixtures/interaction/v1/tools.kcad");

#[test]
fn a_click_asks_for_the_field_and_enter_adds_the_text() {
    let mut b = Bench::new("text");
    assert_eq!(
        b.session.prompt().text(),
        "Yazı: yazının başlangıcına tıklayın [Yükseklik (Y): 2.5 mm / Açı (A): 0°]"
    );
    b.click(4.0, 2.0);
    // The field opens where the text starts: 2.5 mm at 1:1000 is 2.5 m.
    let Some(ViewChange::Text(field)) = b.views.last().copied() else {
        panic!("a text field asked for: {:?}", b.views);
    };
    assert_eq!(rel(kentos_contracts::Vec2 { x: field.at.x, y: field.at.y }), [4.0, 2.0]);
    assert_eq!((field.height, field.rotation), (2.5, 0.0));
    assert_eq!(
        b.session.prompt().text(),
        "Yazı: yazıyı tıkladığınız yere yazın; Enter ekler, Esc vazgeçer"
    );
    b.run(|s, cx| s.text_typed(Some("  Ada 101  "), cx));
    assert_eq!(b.last_text(), Some("Yazı eklendi: “Ada 101”"));
    let Entity::Text(t) = b.newest() else {
        panic!("a text");
    };
    assert_eq!((t.text.as_str(), t.height, t.rotation), ("Ada 101", 2.5, 0.0));
    assert_eq!(rel(t.p), [4.0, 2.0]);
    // Back to where the next text starts; Ctrl+Z takes this one back.
    assert!(b.session.prompt().text().starts_with("Yazı: yazının başlangıcına"));
    assert!(b.undo_step());
    assert!(!b.doc.entities().any(|e| matches!(e, Entity::Text(_))));
}

#[test]
fn esc_or_nothing_typed_writes_nothing_and_the_tool_waits() {
    let mut b = Bench::new("text");
    let count = b.doc.entities().count();
    b.click(0.0, 0.0);
    b.run(|s, cx| s.text_typed(None, cx));
    b.click(1.0, 0.0);
    b.run(|s, cx| s.text_typed(Some("   "), cx));
    assert_eq!(b.doc.entities().count(), count);
    assert!(b.session.is_running());
    assert!(b.session.prompt().text().starts_with("Yazı: yazının başlangıcına"));
}

#[test]
fn height_and_angle_are_typed_or_shown_and_kept() {
    let mut b = Bench::new("text");
    assert!(b.type_text("Y"));
    assert_eq!(
        b.session.prompt().text(),
        "Yazı: kâğıt üzerindeki yazı yüksekliğini mm olarak yazın"
    );
    // A decimal comma is a point; zero is not a height.
    assert!(!b.type_text("0"));
    assert!(b.type_text("3,5"));
    assert!(b.type_text("A"));
    assert_eq!(
        b.session.prompt().text(),
        "Yazı: açıyı yazın (derece) ya da doğrultu için iki noktaya tıklayın"
    );
    // Two clicks along an edge pointing left: turned around, kept readable.
    b.click(10.0, 0.0);
    assert_eq!(
        b.session.prompt().text(),
        "Yazı: doğrultunun ikinci noktasına tıklayın"
    );
    b.click(0.0, -10.0);
    assert_eq!(
        b.session.prompt().text(),
        "Yazı: yazının başlangıcına tıklayın [Yükseklik (Y): 3.5 mm / Açı (A): 45°]"
    );
    // Kept for the next run.
    b.confirm();
    assert!(!b.session.is_running());
    b.start("text");
    assert!(b.session.prompt().text().ends_with("[Yükseklik (Y): 3.5 mm / Açı (A): 45°]"));
}

#[test]
fn a_locked_active_layer_is_said_at_the_click_and_no_field_opens() {
    let mut b = Bench::on(TOOLS);
    b.draft.snap = false;
    assert!(b.doc.set_active_layer("kilitli"), "the locked layer");
    b.start("text");
    b.click(0.0, 0.0);
    assert!(b.views.is_empty(), "no field: {:?}", b.views);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.last_text(),
        Some("“Kilitli katman” katmanı kilitli. Kilidi Katmanlar panelinden açın ya da başka bir katmanı etkinleştirin.")
    );
    assert!(b.session.prompt().text().starts_with("Yazı: yazının başlangıcına"));
}

#[test]
fn enter_leaves_where_the_text_starts_and_steps_back_from_a_question() {
    let mut b = Bench::new("text");
    assert!(b.type_text("Y"));
    b.confirm();
    assert!(b.session.is_running());
    assert!(b.session.prompt().text().starts_with("Yazı: yazının başlangıcına"));
    b.confirm();
    assert!(!b.session.is_running());
}
