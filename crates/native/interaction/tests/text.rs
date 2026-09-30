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
        "Yazı: yazının başlangıcına tıklayın [Yükseklik (Y): 2.5 mm / Açı (A): 0° / Hiza (H): sol taban / Genişlik (G): 1 / Zemin (Z): kapalı / Artır (R): kapalı]"
    );
    b.click(4.0, 2.0);
    // The field opens where the text starts: 2.5 mm at 1:1000 is 2.5 m.
    let Some(ViewChange::Text(field)) = b.views.last().cloned() else {
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
        "Yazı: yazının başlangıcına tıklayın [Yükseklik (Y): 3.5 mm / Açı (A): 45° / Hiza (H): sol taban / Genişlik (G): 1 / Zemin (Z): kapalı / Artır (R): kapalı]"
    );
    // Kept for the next run.
    b.confirm();
    assert!(!b.session.is_running());
    b.start("text");
    assert!(b.session.prompt().text().contains("[Yükseklik (Y): 3.5 mm / Açı (A): 45° / Hiza"));
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

/// Yazı's options (docs/adr/0145 §6): Hiza from its menu or its name typed
/// together, Genişlik, Zemin and Artır, kept in the session's memory and
/// written with the texts. The web's are apps/web/src/tools/textTool.test.ts.
#[test]
fn hiza_genişlik_zemin_and_artır_are_kept_and_written() {
    let mut b = Bench::new("text");
    assert_eq!(
        b.session.prompt().text(),
        "Yazı: yazının başlangıcına tıklayın [Yükseklik (Y): 2.5 mm / Açı (A): 0° / Hiza (H): sol taban / Genişlik (G): 1 / Zemin (Z): kapalı / Artır (R): kapalı]"
    );
    // Hiza: its name typed together, with or without the Turkish marks.
    assert!(b.type_text("H"));
    assert_eq!(
        b.session.prompt().text(),
        "Yazı: hizayı seçin ya da adını bitişik yazın: sağüst, orta, soltaban … [Hiza (H): sol taban]"
    );
    assert!(b.type_text("sag-ust"));
    assert!(b.session.prompt().text().contains("Hiza (H): sağ üst /"));
    // A word that names none is said, and the step stays.
    assert!(b.type_text("H"));
    assert!(b.type_text("yukarı"));
    assert!(b.last_text().is_some_and(|t| t.starts_with("“yukarı” bir hiza adı değil")));
    assert!(b.session.prompt().text().contains("hizayı seçin"));
    assert!(b.type_text("ORTA"));
    assert!(b.session.prompt().text().contains("Hiza (H): orta /"));
    // The menu: the twelve points row by row, the chosen one checked.
    let choices = b.session.option_choices("H");
    let labels: Vec<&str> = choices.iter().map(|c| c.label).collect();
    assert_eq!(
        labels,
        [
            "Sol üst", "Orta üst", "Sağ üst", "Sol orta", "Orta", "Sağ orta", "Sol alt",
            "Orta alt", "Sağ alt", "Sol taban", "Orta taban", "Sağ taban"
        ]
    );
    let checked: Vec<&str> = choices.iter().filter(|c| c.checked).map(|c| c.label).collect();
    assert_eq!(checked, ["Orta"]);
    assert_eq!((choices[0].icon, choices[9].icon), ("textAlignTopLeft", "textAlignBaselineLeft"));
    assert!(b.session.option_choices("Y").is_empty());
    assert!(b.run(|s, cx| s.choose_option("H", "sağ taban", cx)));
    assert!(b.session.prompt().text().contains("Hiza (H): sağ taban /"));
    assert!(!b.run(|s, cx| s.choose_option("Y", "3", cx)));
    // Genişlik: over 0 and at most 100, else said and asked again.
    assert!(b.type_text("G"));
    assert_eq!(
        b.session.prompt().text(),
        "Yazı: genişlik çarpanını yazın (1: harflerin kendi eni; 0'dan büyük, en çok 100)"
    );
    assert!(b.type_text("0"));
    assert_eq!(
        b.last_text(),
        Some("Genişlik çarpanı 0'dan büyük, en çok 100 olmalı; 0 verildi. Harflerin kendi eni için 1 yazın.")
    );
    assert!(b.type_text("0.8"));
    assert!(b.type_text("Z"));
    assert!(b.type_text("R"));
    assert!(b.session.prompt().text().ends_with(
        "Hiza (H): sağ taban / Genişlik (G): 0.8 / Zemin (Z): açık / Artır (R): açık]"
    ));

    // The field stands as the text will, and the text is written with its extras.
    let write = |b: &mut Bench, at: [f64; 2], text: &str| {
        b.click(at[0], at[1]);
        let Some(ViewChange::Text(field)) = b.views.last().cloned() else {
            panic!("a text field asked for: {:?}", b.views);
        };
        b.run(|s, cx| s.text_typed(Some(text), cx));
        field
    };
    let field = write(&mut b, [10.0, 20.0], "Ada 101");
    assert_eq!(
        (field.align, field.width_factor, field.initial),
        (Some(kentos_contracts::TextAlign::BaselineRight), 0.8, None)
    );
    let Entity::Text(t) = b.newest() else {
        panic!("a text");
    };
    assert_eq!(
        (t.text.as_str(), rel(t.p), t.align, t.width_factor, t.mask),
        ("Ada 101", [10.0, 20.0], Some(kentos_contracts::TextAlign::BaselineRight), Some(0.8), true)
    );
    assert_eq!(b.last_text(), Some("Yazı eklendi: “Ada 101”"));
    // Artır: the last text's number one more; a text with no number comes back as it is.
    assert_eq!(write(&mut b, [10.0, 16.0], "Ada 102").initial.as_deref(), Some("Ada 102"));
    assert_eq!(write(&mut b, [10.0, 12.0], "Yol").initial.as_deref(), Some("Ada 103"));
    assert_eq!(write(&mut b, [10.0, 8.0], "Yol").initial.as_deref(), Some("Yol"));
    assert!(b.type_text("R"));
    assert_eq!(write(&mut b, [10.0, 4.0], "Son").initial, None);
    // Back to the defaults: none of the extras is written.
    for typed in ["H", "soltaban", "G", "1", "Z"] {
        assert!(b.type_text(typed));
    }
    write(&mut b, [0.0, 0.0], "Düz");
    let Entity::Text(t) = b.newest() else {
        panic!("a text");
    };
    assert_eq!(
        (t.text.as_str(), t.align, t.width_factor, t.mask),
        ("Düz", None, None, false)
    );
}

/// Hiza's names read as the web's `textAlignFromName` reads them.
#[test]
fn hiza_names_are_read_together_or_apart_with_or_without_marks() {
    use kentos_contracts::TextAlign::*;
    use kentos_interaction::text::{align_from_name, align_name};
    for (typed, want) in [
        ("sağüst", Some(Some(TopRight))),
        ("SAĞ ÜST", Some(Some(TopRight))),
        ("sag_ust", Some(Some(TopRight))),
        ("Orta", Some(Some(MiddleCenter))),
        ("ortaorta", Some(Some(MiddleCenter))),
        ("ORTA TABAN", Some(Some(BaselineCenter))),
        ("soltaban", Some(None)),
        ("SOLALT", Some(Some(BottomLeft))),
        ("İ", None),
        ("", None),
        ("yukarı", None),
    ] {
        assert_eq!(align_from_name(typed), want, "{typed}");
    }
    assert_eq!(
        (align_name(None), align_name(Some(MiddleCenter)), align_name(Some(TopRight))),
        ("sol taban", "orta", "sağ üst")
    );
}
