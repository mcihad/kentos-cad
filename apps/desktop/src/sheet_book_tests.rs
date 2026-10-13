//! The project's sheet book on the desktop (docs/adr/0251; sheet_book.rs):
//! opened from the drawing's file, written into the drawing as it changes,
//! saved and opened again; a book this device kept before moved into its
//! project and set aside once saved; a book the core does not read kept as
//! it is.

use kentos_interaction::Level;
use kentos_sheet_ui::Message as Sheet;

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{drive, scratch};

const SAMPLE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/kcad/v2/sheets.kcad"
);

fn open(app: &mut App, path: &std::path::Path) {
    let doc = Document::read(path).expect("opens");
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    drive(app, task);
}

fn app_with_store(name: &str) -> (App, kentos_sheet_ui::Store) {
    let (mut app, _) = App::boot(None);
    let store = kentos_sheet_ui::Store::open(&scratch(name)).expect("a store");
    app.sheet_store = Some(store.clone());
    (app, store)
}

/// The key an older build kept the drawing's book under (its project's id, else its file's name).
fn key_of(doc: &kentos_contracts::DocumentSnapshotV2, file: &str) -> String {
    let id = doc
        .project_id
        .map(|p| kentos_domain::Uuid::from_bytes(p.0).to_string());
    kentos_sheet_ui::project_key(None, id.as_deref(), Some(file), "test").id
}

fn sheet(app: &mut App, m: Sheet) {
    let task = app.update(Message::Sheet(m));
    drive(app, task);
}

fn names(app: &App) -> Vec<String> {
    app.sheets
        .book()
        .sheets
        .iter()
        .map(|s| s.name.clone())
        .collect()
}

/// The drawing's book as the drawing holds it, read by the core.
fn kept(app: &App) -> Option<kentos_sheet::model::SheetBook> {
    let doc = app.document.as_ref()?;
    let sheets = doc.model.sheets()?;
    Some(
        kentos_sheet::project::read_project_sheets(sheets)
            .expect("reads")
            .0,
    )
}

#[test]
fn the_book_opens_from_the_file_changes_in_the_drawing_and_is_saved() {
    let (mut app, _store) = app_with_store("pafta-dosyadan");
    let dir = scratch("pafta-dosyadan-cizim");
    let path = dir.join("pafta.kcad");
    std::fs::copy(SAMPLE, &path).expect("copied");
    open(&mut app, &path);
    assert_eq!(names(&app), ["Büro paftası"]);
    let logo = app.sheets.book().assets[0].sha256.clone();
    let doc = app.document.as_ref().expect("a drawing");
    assert!(!doc.model.is_dirty(), "opening is no edit");
    // A change in the sheet mode is in the drawing at once: an edit, not an undo step of the drawing.
    let id = app.sheets.book().sheets[0].id.clone();
    let undo_before = app.document.as_ref().expect("a drawing").model.can_undo();
    sheet(
        &mut app,
        Sheet::RenameSheet(id.clone(), "Ada 101 paftası".into()),
    );
    let doc = app.document.as_ref().expect("a drawing");
    assert!(doc.model.is_dirty());
    assert_eq!(doc.model.can_undo(), undo_before);
    assert_eq!(
        kept(&app).expect("a book").sheets[0].name,
        "Ada 101 paftası"
    );
    // Its picture's bytes go with it.
    let pictures = &app
        .document
        .as_ref()
        .and_then(|d| d.model.sheets())
        .expect("a book")
        .pictures;
    assert_eq!(
        pictures
            .iter()
            .map(|p| p.sha256.as_str())
            .collect::<Vec<_>>(),
        [logo.as_str()]
    );
    // The sheet mode's own undo takes it back, in the drawing too.
    sheet(&mut app, Sheet::Undo);
    assert_eq!(kept(&app).expect("a book").sheets[0].name, "Büro paftası");
    sheet(&mut app, Sheet::Redo);
    // Saved, and opened again in another window: the book is the file's.
    let snapshot = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .to_snapshot_v2();
    crate::document::write(&snapshot, &path).expect("saved");
    let (mut again, _) = app_with_store("pafta-dosyadan-2");
    open(&mut again, &path);
    assert_eq!(names(&again), ["Ada 101 paftası"]);
    // A drawing without a book: an empty one; a new sheet makes the drawing's book.
    let empty = dir.join("bos.kcad");
    let mut bare = snapshot.clone();
    bare.sheets = None;
    crate::document::write(&bare, &empty).expect("saved");
    open(&mut again, &empty);
    assert!(again.sheets.book().sheets.is_empty());
    assert!(kept(&again).is_none());
    let _ = again.update(Message::Sheet(Sheet::NewSheet));
    if again.sheets.questions().is_some() {
        let _ = again.update(Message::Sheet(Sheet::Questions(
            kentos_sheet_ui::questions::QuestionsMessage::Make,
        )));
    }
    assert_eq!(kept(&again).expect("a book").sheets.len(), 1);
}

#[test]
fn a_book_this_device_kept_moves_into_its_project_and_is_set_aside_once_saved() {
    let (mut app, store) = app_with_store("pafta-goc");
    let dir = scratch("pafta-goc-cizim");
    let path = dir.join("ada.kcad");
    // The sample's drawing without its book, and its book where an older build kept it.
    let mut doc = kentos_kcad::decode(&std::fs::read(SAMPLE).expect("sample")).expect("reads");
    let sheets = doc.sheets.take().expect("a book");
    crate::document::write(&doc, &path).expect("saved");
    let (book, pictures) = kentos_sheet::project::read_project_sheets(&sheets).expect("reads");
    for (sha, bytes) in &pictures {
        store.put_asset(sha, bytes);
    }
    let key = key_of(&doc, "ada.kcad");
    store.save_book(&key, &book).expect("kept");
    open(&mut app, &path);
    assert_eq!(names(&app), ["Büro paftası"]);
    assert_eq!(kept(&app), Some(book.clone()));
    let model = &app.document.as_ref().expect("a drawing").model;
    assert!(
        model.is_dirty(),
        "the drawing changed: it is saved with its book"
    );
    assert_eq!(model.sheets().expect("a book").pictures.len(), 1);
    assert!(app.log.said(Level::Info, "bu bilgisayardan çizime alındı"));
    // Not saved yet: the device's copy stays where it was.
    assert_eq!(store.load_book(&key), Some(book.clone()));
    // Saved: set aside, never deleted.
    let revision = app.document.as_ref().expect("a drawing").model.revision();
    let snapshot = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .to_snapshot_v2();
    crate::document::write(&snapshot, &path).expect("saved");
    app.document
        .as_mut()
        .expect("a drawing")
        .model
        .mark_saved(revision);
    let _ = app.update(Message::Swallowed);
    assert_eq!(store.load_book(&key), None);
    let aside = std::fs::read_dir(store.dir().join("kitaplar").join("tasindi"))
        .expect("set aside")
        .count();
    assert_eq!(aside, 1);
    // Opened again: the file's book, nothing moved, nothing said of it.
    let moved = |app: &App| {
        app.log
            .lines()
            .filter(|l| l.text.contains("çizime alındı") || l.text.contains("silinmedi"))
            .count()
    };
    open(&mut app, &path);
    assert_eq!(names(&app), ["Büro paftası"]);
    assert!(!app.document.as_ref().expect("a drawing").model.is_dirty());
    assert_eq!(moved(&app), 1);
}

#[test]
fn a_project_with_a_book_sets_the_devices_other_copy_aside_and_says_so() {
    let (mut app, store) = app_with_store("pafta-iki-kitap");
    let doc = kentos_kcad::decode(&std::fs::read(SAMPLE).expect("sample")).expect("reads");
    let key = key_of(&doc, "sheets.kcad");
    let mut other = kentos_sheet::model::SheetBook::default();
    other.variables.push(kentos_sheet::model::Variable {
        name: "eski".into(),
        label: "Eski".into(),
        kind: kentos_sheet::model::VarKind::Text,
        value: kentos_sheet::model::VarValue::Text("1".into()),
    });
    store.save_book(&key, &other).expect("kept");
    open(&mut app, std::path::Path::new(SAMPLE));
    assert_eq!(names(&app), ["Büro paftası"], "the file's book");
    assert!(!app.document.as_ref().expect("a drawing").model.is_dirty());
    assert_eq!(store.load_book(&key), None);
    assert!(app.log.said(Level::Info, "cihazdaki silinmedi"));
}

#[test]
fn a_book_the_core_does_not_read_is_kept_as_it_is() {
    let (mut app, _store) = app_with_store("pafta-okunamayan");
    let dir = scratch("pafta-okunamayan-cizim");
    let path = dir.join("yeni.kcad");
    let mut doc = kentos_kcad::decode(&std::fs::read(SAMPLE).expect("sample")).expect("reads");
    let mut sheets = doc.sheets.take().expect("a book");
    sheets.book["schema"] = "kentos.sheet/9".into();
    doc.sheets = Some(sheets.clone());
    crate::document::write(&doc, &path).expect("saved");
    open(&mut app, &path);
    assert!(app.log.said(Level::Warn, "pafta kitabı okunamadı"));
    assert!(app.sheets.book().sheets.is_empty());
    let _ = app.update(Message::Sheet(Sheet::NewSheet));
    if app.sheets.questions().is_some() {
        let _ = app.update(Message::Sheet(Sheet::Questions(
            kentos_sheet_ui::questions::QuestionsMessage::Make,
        )));
    }
    let model = &app.document.as_ref().expect("a drawing").model;
    assert_eq!(model.sheets(), Some(&sheets), "kept as it is");
    assert!(!model.is_dirty());
}
