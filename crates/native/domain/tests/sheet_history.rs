//! Paper snapshots and embedded bytes follow model groups, savepoints and import history (ADR 0590).
use kentos_domain::{
    Document,
    contracts::{DocumentSnapshotV2, ProjectSheets},
};

fn sample() -> Document {
    Document::from_snapshot_v2(
        serde_json::from_str::<DocumentSnapshotV2>(include_str!(
            "../../../../fixtures/kcad/v2/sheets.json"
        ))
        .expect("snapshot"),
    )
    .expect("drawing")
}

fn renamed(book: &ProjectSheets, name: &str) -> ProjectSheets {
    let mut book = book.clone();
    book.book["sheets"][0]["name"] = name.into();
    book
}

#[test]
fn savepoints_keep_the_previous_book_and_its_picture_bytes() {
    let mut doc = sample();
    let original = doc.sheets().unwrap().clone();
    assert!(!original.pictures.is_empty());
    let a = renamed(&original, "A");
    let b = renamed(&original, "B");
    let group = doc.begin_group("Pafta işlemi");
    doc.set_sheets(Some(a.clone())).unwrap();
    let at = doc.group_savepoint().unwrap();
    doc.set_sheets(None).unwrap();
    doc.set_sheets(Some(b)).unwrap();
    doc.rollback_to(at);
    assert_eq!(doc.sheets(), Some(&a));
    doc.end_group(group);
    assert_eq!(doc.undo().as_deref(), Some("Pafta işlemi"));
    assert_eq!(doc.sheets(), Some(&original));
    doc.redo();
    assert_eq!(doc.sheets(), Some(&a));
}

#[test]
fn nested_scopes_join_the_outer_group_and_equal_books_make_no_step() {
    let mut doc = sample();
    let original = doc.sheets().unwrap().clone();
    let revision = doc.revision();
    let group = doc.begin_group("Aynı");
    doc.set_sheets(Some(original.clone())).unwrap();
    doc.end_group(group);
    assert_eq!(doc.revision(), revision);
    assert!(!doc.can_undo());
    let outer = doc.begin_group("Dış");
    let inner = doc.begin_group("İç");
    doc.set_sheets(None).unwrap();
    doc.end_group(inner);
    assert!(doc.is_busy());
    doc.cancel_group(outer);
    assert_eq!(doc.sheets(), Some(&original));
    assert_eq!(doc.revision(), revision);
    assert!(!doc.can_undo());
}

#[test]
fn import_book_history_checks_ownership_before_reverting() {
    let mut doc = sample();
    let original = doc.sheets().unwrap().clone();
    let imported = renamed(&original, "İçe alınan");
    let later = renamed(&original, "Sonraki");
    let _: Result<(), ()> = doc.transact("İçe al", |doc| {
        doc.with_import_metadata("İçe al", |doc| {
            doc.set_sheets(Some(imported.clone())).unwrap();
            Ok(())
        })
    });
    doc.set_sheets(Some(later.clone())).unwrap();
    let revision = doc.revision();
    assert!(doc.sheet_history_problem(false).is_some());
    assert_eq!(doc.undo(), None);
    assert_eq!(doc.revision(), revision);
    assert_eq!(doc.sheets(), Some(&later));
    doc.set_sheets(Some(imported)).unwrap();
    doc.undo();
    assert_eq!(doc.sheets(), Some(&original));
}

#[test]
fn style_only_import_leaves_a_later_independent_book_untouched() {
    let mut doc = sample();
    let later = renamed(doc.sheets().unwrap(), "Bağımsız");
    let _: Result<(), ()> = doc.transact("Stil al", |doc| {
        doc.with_import_metadata("Stil al", |doc| {
            let mut settings = doc.settings().clone();
            settings.line_type_scale = Some(2.0);
            doc.set_settings(settings);
            Ok(())
        })
    });
    doc.set_sheets(Some(later.clone())).unwrap();
    assert_eq!(doc.sheet_history_problem(false), None);
    doc.undo();
    assert_eq!(doc.sheets(), Some(&later));
    assert_ne!(doc.settings().line_type_scale, Some(2.0));
    doc.redo();
    assert_eq!(doc.sheets(), Some(&later));
    assert_eq!(doc.settings().line_type_scale, Some(2.0));
}

#[test]
fn import_inside_group_records_each_book_transition_once() {
    let mut doc = sample();
    let original = doc.sheets().unwrap().clone();
    let imported = renamed(&original, "İçe alınan");
    let final_book = renamed(&original, "Son");
    let group = doc.begin_group("Birleşik");
    let _: Result<(), ()> = doc.transact("İçe al", |doc| {
        doc.with_import_metadata("İçe al", |doc| {
            let mut settings = doc.settings().clone();
            settings.line_type_scale = Some(2.0);
            doc.set_settings(settings);
            doc.set_sheets(Some(imported)).unwrap();
            Ok(())
        })
    });
    doc.set_sheets(Some(final_book.clone())).unwrap();
    doc.end_group(group);
    doc.undo();
    assert_eq!(doc.sheets(), Some(&original));
    doc.redo();
    assert_eq!(doc.sheets(), Some(&final_book));
}
