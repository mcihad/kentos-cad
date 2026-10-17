//! Denetle through the session (docs/adr/0306): what the audit finds shown first, the broken objects repaired as one
//! undo step, the unused styles removed on asking, a locked layer said. The drawing is the shared cases' setup
//! (`fixtures/commands/v1/cad.drawing.audit.json`): a linked text, a hatch and a dimension whose objects are gone, a
//! text and a dimension naming styles the project lacks, a linked text on the locked layer `kilitli`; unused the
//! layers Boş and Eski yol, the group Eski, the block Eski blok, one text and one dimension style.

use crate::common;

use common::Bench;
use kentos_contracts::Entity;
use kentos_domain::Slot;
use kentos_interaction::Tone;

const CASES: &str = include_str!("../../../../../fixtures/commands/v1/cad.drawing.audit.json");

fn bench() -> Bench {
    let cases: serde_json::Value = serde_json::from_str(CASES).expect("the cases");
    let mut b = Bench::on(&cases["setup"].to_string());
    b.draft.snap = false;
    b
}

#[test]
fn audit_shows_what_it_found_then_repairs_in_one_step() {
    let mut b = bench();
    b.start("audit");
    assert!(
        b.log
            .iter()
            .any(|l| l.text == "Denetle: 6 bozuk nesne (1 kilitli katmanda); Enter ile onarın."),
        "{:?}",
        b.log.iter().map(|l| &l.text).collect::<Vec<_>>()
    );
    assert_eq!(
        b.last_text(),
        Some(
            "Kullanılmayan katman: Boş, Eski / Eski yol; grup: Eski; blok: Eski blok; yazı stili: Eski yazı; ölçü stili: Eski ölçü. Stilleri Y ve Ö ile temizleyin; katman, grup ve blokları Kullanılmayanları temizle kaldırır."
        )
    );
    assert!(
        b.session.prompt().text().starts_with(
            "Denetle: 6 bozuk nesne (1 kilitli katmanda); 6 kullanılmayan kaynak [Denetle (Enter) / Onar (O): açık"
        ),
        "{}",
        b.session.prompt().text()
    );
    assert_eq!(b.options(), ["Enter", "O", "Y", "Ö"]);
    let p = b.session.preview(&Default::default()).expect("a preview");
    assert!(
        p.strokes
            .iter()
            .any(|s| s.tone == Tone::Danger && s.dash.is_some())
    );
    b.confirm();
    assert_eq!(b.last_text(), Some("Denetle: 5 nesne onarıldı."));
    assert_eq!(b.session.tool_id(), "select");
    let Some(Entity::Text(t)) = b.doc.get(Slot(2)) else {
        panic!("text 2");
    };
    assert!(t.label_of.is_none() && t.face.text_style.is_none());
    let Some(Entity::Text(locked)) = b.doc.get(Slot(8)) else {
        panic!("text 8");
    };
    assert!(locked.label_of.is_some(), "the locked layer's text is left");
    assert_eq!(b.doc.undo().as_deref(), Some("Denetle"));
    let Some(Entity::Text(t)) = b.doc.get(Slot(2)) else {
        panic!("text 2");
    };
    assert!(t.label_of.is_some() && t.face.text_style.is_some());
}

#[test]
fn audit_removes_the_unused_styles_on_asking() {
    let mut b = bench();
    b.start("audit");
    assert!(b.type_text("O"));
    assert!(b.type_text("Y"));
    assert!(b.type_text("Ö"));
    assert!(
        b.session
            .prompt()
            .text()
            .contains("Onar (O): kapalı / Yazı stilleri (Y): açık / Ölçü stilleri (Ö): açık"),
        "{}",
        b.session.prompt().text()
    );
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Denetle: 1 yazı stili, 1 ölçü stili temizlendi.")
    );
    assert_eq!(b.doc.settings().text_styles.len(), 1);
    assert_eq!(b.doc.settings().dimension_styles.len(), 1);
    let Some(Entity::Text(t)) = b.doc.get(Slot(2)) else {
        panic!("text 2");
    };
    assert!(t.label_of.is_some(), "Onar off: nothing repaired");
}

#[test]
fn a_sound_drawing_is_said_and_the_tool_leaves() {
    let mut b = Bench::on(include_str!(
        "../../../../../fixtures/interaction/v1/empty.kcad"
    ));
    b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    let rev = b.doc.revision();
    b.start("audit");
    assert_eq!(
        b.last_text(),
        Some("Denetle: bozuk ilişki ya da kayıp stil yok.")
    );
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Denetle: çizim sağlam; kullanılmayan kaynak yok ["),
        "{}",
        b.session.prompt().text()
    );
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Denetle: onarılacak ya da temizlenecek bir şey yok.")
    );
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.revision(), rev);
}
