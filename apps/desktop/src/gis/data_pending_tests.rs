//! Katman değişiklikleri (docs/adr/9123): a project's save and its data layers' writes told apart. With a local layer
//! and two data layers changed, the counts the status bar and the window show follow what a second reader of the
//! sources sees: the project saved with its rows waiting, one layer written, the project opened again with the rest
//! back from this device, one layer let go, all written; a write its source refused keeps the rows and its reason,
//! and goes on the next try. A form's Kaydet on a data layer says the value waits.

use std::path::Path;

use kentos_native_application::data_write;

use crate::app::{App, Message, Picker};
use crate::data_edits::Event;
use crate::data_journal::Journals;
use crate::data_layers_tests::{node, parcels};
use crate::data_pending::Event as Pending;
use crate::files_testing::{app_with_drawing, drive, scratch};

/// A row's `kullanim` as a reader of its own reads it: its own connection to the file, not the program's.
fn second_reader(path: &Path, fid: i64) -> String {
    let gp = kentos_geopackage::GeoPackage::open(path, false).expect("the file");
    gp.connection()
        .query_row("SELECT kullanim FROM parseller WHERE fid = ?1", [fid], |r| r.get::<_, String>(0))
        .expect("the row")
}

/// Row `fid` of the data layer taken and its `kullanim` changed on this device.
fn change(app: &mut App, layer: &kentos_commands::LayerNode, fid: i64, value: &str) {
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let rows = data_write::rows_to_take(layer, &settings, &[fid], &|| false).expect("read");
    let doc = app.document.as_mut().expect("a drawing");
    let taken = doc.model.take_data_rows(&layer.id, rows).expect("taken")[0].expect("the row");
    let slot = doc.model.slot_of(taken).expect("its object");
    let mut e = doc.model.get(slot).cloned().expect("the row");
    e.base_mut().attrs.insert("kullanim".into(), value.into());
    assert!(doc.model.update(slot, e));
}

/// The cell's words, none while nothing waits.
fn cell(app: &App) -> Option<String> {
    app.data_pending_words(0).map(|(w, _)| w)
}

/// A drawing with two data layers on two GeoPackages, both being edited, and the journals in a scratch folder.
fn two_layers(tag: &str) -> (App, std::path::PathBuf, std::path::PathBuf, std::path::PathBuf, Journals) {
    crate::data_layers::register();
    let (a, b) = (parcels(&format!("{tag}-a"), 6), parcels(&format!("{tag}-b"), 6));
    let dir = scratch(tag);
    let journals = Journals::at(&dir.join("veri-oturumlari"));
    let mut app = app_with_drawing();
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.workspace = Some(kentos_commands::Workspace::Gis);
    doc.model.set_settings(settings);
    // A CBS project's data is its tables (docs/adr/9196 §2): the sample's drawing objects go, its layers stay.
    doc.model.set_layer_locked("bina", false);
    let uids: Vec<String> = doc.model.entities_with_uids().map(|(u, _)| u.to_string()).collect();
    let gone = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.entities.delete",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "uids": uids }),
    )
    .expect("an answer");
    assert_eq!(gone["status"], "completed", "{gone}");
    app.gis.data_journal = journals.clone();
    let mut second = crate::data_layers::describe(&b.display().to_string(), "parseller").expect("described");
    second.name = "parseller-b".into();
    app.add_data_layers(vec![crate::data_layers::describe(&a.display().to_string(), "parseller").expect("described"), second])
        .expect("added");
    for name in ["parseller", "parseller-b"] {
        let id = node(&app, name).expect("the layer").id.clone();
        let _ = app.data_edits_event(Event::Start(id));
    }
    (app, a, b, dir.join("proje.kcad"), journals)
}

#[test]
fn the_counts_follow_what_a_second_reader_of_the_sources_sees() {
    let (mut app, a, b, file, journals) = two_layers("bekleyen");
    let la = node(&app, "parseller").expect("a").clone();
    let lb = node(&app, "parseller-b").expect("b").clone();
    // A local layer's change, the project's own, and a row changed on each data layer.
    let doc = app.document.as_mut().expect("a drawing");
    doc.model.add_layer_at(kentos_domain::NewLayer::layer("Notlar"), None, Some(0), Some("Katman ekle"), false).expect("a layer");
    change(&mut app, &la, 1, "Okul");
    change(&mut app, &lb, 2, "Pazar");
    assert_eq!(cell(&app).as_deref(), Some("Kaynağa yazılmadı: 2 katmanda 2 satır"));

    // Kaydet: the project is written, the rows are not; the second reader sees both sources as they were.
    app.picker = Picker::File(file.clone());
    let task = app.update(Message::Run("file.save"));
    drive(&mut app, task);
    assert!(app.document.as_ref().is_some_and(|d| !d.dirty()), "the project saved");
    assert_eq!(cell(&app).as_deref(), Some("Kaynağa yazılmadı: 2 katmanda 2 satır"), "its rows still wait");
    assert_eq!((second_reader(&a, 1), second_reader(&b, 2)), ("Konut".to_owned(), "Park".to_owned()));
    let said: Vec<String> = app.log.lines().map(|l| l.text.clone()).collect();
    assert!(
        said.iter().any(|t| t.contains("Proje kaydedildi; katmanlardaki 2 satır kaynağa yazılmadı (parseller-b: 1, parseller: 1)")),
        "{said:#?}"
    );

    // One layer written from the window: one row waits; the reader sees that one row.
    let _ = app.data_pending_event(Pending::Open(Some(la.id.clone())));
    let task = app.data_pending_event(Pending::Save(vec![la.id.clone()]));
    drive(&mut app, task);
    assert_eq!(cell(&app).as_deref(), Some("Kaynağa yazılmadı: 1 katmanda 1 satır"));
    assert_eq!((second_reader(&a, 1), second_reader(&b, 2)), ("Okul".to_owned(), "Park".to_owned()));

    // Opened again from its file: the waiting row comes back from this device, counted as before.
    let mut again = app_with_drawing();
    again.gis.data_journal = journals.clone();
    let doc = crate::document::Document::read(&file).expect("the file reads");
    let _ = again.update(Message::Opened(Some(Ok(Box::new(doc)))));
    assert_eq!(cell(&again).as_deref(), Some("Kaynağa yazılmadı: 1 katmanda 1 satır"));
    let waiting = again.data_waiting();
    assert_eq!((waiting[0].name.as_str(), waiting[0].changed), ("parseller-b", 1));

    // Let go from the window, at the second press: nothing waits, the source as it was.
    let _ = again.data_pending_event(Pending::Rollback(lb.id.clone()));
    assert!(cell(&again).is_some(), "the first press only asks");
    let _ = again.data_pending_event(Pending::RollbackConfirmed(lb.id.clone()));
    assert_eq!(cell(&again), None);
    assert_eq!(second_reader(&b, 2), "Park");
}

#[test]
fn hepsini_kaynaga_yaz_writes_every_layer_and_a_refused_write_keeps_its_rows_and_reason() {
    use std::os::unix::fs::PermissionsExt;
    let (mut app, a, b, _, _) = two_layers("bekleyen-hata");
    let la = node(&app, "parseller").expect("a").clone();
    let lb = node(&app, "parseller-b").expect("b").clone();
    change(&mut app, &la, 3, "Cami");
    change(&mut app, &lb, 4, "Okul");

    // The first source cannot be written: its rows stay, the cell turns to the danger's words and says why.
    std::fs::set_permissions(&a, std::fs::Permissions::from_mode(0o444)).expect("read only");
    let task = app.data_pending_event(Pending::Save(Vec::new()));
    drive(&mut app, task);
    let _ = std::fs::set_permissions(&a, std::fs::Permissions::from_mode(0o644));
    let (words, failed) = app.data_pending_words(0).expect("rows wait");
    assert_eq!((words.as_str(), failed), ("Kaynağa yazılamadı: 1 katmanda 1 satır", true));
    let waiting = app.data_waiting();
    assert!(waiting[0].failed.as_deref().is_some_and(|w| w.contains("hiçbir şey yazılmadı")), "{waiting:?}");
    assert_eq!((second_reader(&a, 3), second_reader(&b, 4)), ("Konut".to_owned(), "Okul".to_owned()));

    // Tried again once it can be written: written, the reason gone, nothing waits.
    let task = app.data_pending_event(Pending::Save(Vec::new()));
    drive(&mut app, task);
    assert_eq!(cell(&app), None);
    assert!(app.gis.data_edits.failed_of(&la.id).is_none());
    assert_eq!(second_reader(&a, 3), "Cami");
}

#[test]
fn a_value_saved_on_a_data_layer_record_is_said_to_wait() {
    let (mut app, _, _, _, _) = two_layers("bekleyen-form");
    let la = node(&app, "parseller").expect("a").clone();
    change(&mut app, &la, 5, "Okul");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let uid = doc.entities_with_uids().find(|(_, e)| e.base().layer_id == la.id).map(|(u, _)| u.to_string()).expect("the row");
    let words = app.waiting_words_of(&uid).expect("it waits");
    assert!(words.starts_with("Değer “parseller” katmanında bekliyor, kaynağa yazılmadı: 1 değişen satır."), "{words}");
}

/// Pictures (docs/adr/9123): the status bar's cell with two layers waiting, then Katman değişiklikleri on the first
/// layer, light at 1440×900 and dark at 1100×760.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib data_pending_tests::screens -- --ignored --nocapture"]
fn screens() {
    let prepare = |open: bool| {
        move || {
            let (mut app, _, _, _, _) = two_layers("bekleyen-resim");
            let la = node(&app, "parseller").expect("a").clone();
            let lb = node(&app, "parseller-b").expect("b").clone();
            change(&mut app, &la, 1, "Okul");
            change(&mut app, &la, 2, "Pazar");
            change(&mut app, &lb, 3, "Cami");
            if open {
                let _ = app.data_pending_event(Pending::Open(Some(la.id.clone())));
            }
            app
        }
    };
    crate::files_testing::usage_shots("9123", "01-durum-hucresi", prepare(false));
    crate::files_testing::usage_shots("9123", "02-katman-degisiklikleri", prepare(true));
}
