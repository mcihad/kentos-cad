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

/// An aligned, narrowed, masked text (docs/adr/0145) given new words in
/// Öznitelikler or the text field keeps where it stands, how wide it is and
/// its mask: the step “Değiştir” carries them; a width factor out of its
/// range is refused, and nothing changes.
#[test]
fn a_texts_extras_stay_when_its_words_change() {
    let mut b = Bench::new("text");
    let aligned = |text: &str, width_factor: f64| {
        Entity::Text(kentos_contracts::TextEntity {
            base: common::base("cizim"),
            p: kentos_contracts::Vec2 {
                x: common::E + 3.0,
                y: common::N + 1.0,
            },
            text: text.to_owned(),
            height: 2.5,
            rotation: 30.0,
            align: Some(kentos_contracts::TextAlign::MiddleCenter),
            width_factor: Some(width_factor),
            mask: true,
        })
    };
    let slot = b.doc.add(aligned("Ada 101", 0.8)).expect("a slot");
    let said =
        kentos_interaction::properties::set_geometry(&mut b.doc, slot, &aligned("Ada 102", 0.8));
    assert!(said.is_empty(), "{said:?}");
    let Some(Entity::Text(t)) = b.doc.get(slot) else {
        panic!("a text at {slot:?}");
    };
    assert_eq!(t.text, "Ada 102");
    assert_eq!(rel(t.p), [3.0, 1.0]);
    assert_eq!(
        (t.align, t.width_factor, t.mask),
        (
            Some(kentos_contracts::TextAlign::MiddleCenter),
            Some(0.8),
            true
        )
    );
    // A width factor of 0 is refused, said as the command says it.
    let said =
        kentos_interaction::properties::set_geometry(&mut b.doc, slot, &aligned("Ada 103", 0.0));
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("genişlik çarpanı"), "{said:?}");
    let Some(Entity::Text(t)) = b.doc.get(slot) else {
        panic!("a text at {slot:?}");
    };
    assert_eq!((t.text.as_str(), t.width_factor), ("Ada 102", Some(0.8)));
    // One step, “Değiştir”, takes the words back with their extras.
    assert_eq!(b.doc.undo().as_deref(), Some("Değiştir"));
    let Some(Entity::Text(t)) = b.doc.get(slot) else {
        panic!("a text at {slot:?}");
    };
    assert_eq!((t.text.as_str(), t.mask), ("Ada 101", true));
}
