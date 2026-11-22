//! The municipal report's 31st item (docs/adr/9151): a data layer's table whose source cannot be read lists the edit
//! session's rows still: the new one, the changed one by its key with its local value and the value it had, the
//! deleted one by its key; the line says they wait unverified. Read again, the source's rows come back with the taken
//! one over them; saved, nothing waits.

use std::sync::Arc;

use kentos_native_application::data_write;

use super::{pending_lines, pending_words};
use crate::data_edits::Event;
use crate::data_layers_tests::{answer_pages, node, parcels};
use crate::features::data::DataEvent;
use crate::files_testing::{app_with_drawing, drive};

fn point(key: i64, durum: &str) -> kentos_commands::Entity {
    serde_json::from_value(serde_json::json!({
        "kind": "point", "id": key, "layerId": "saha", "p": { "x": 0.0, "y": 0.0 }, "attrs": { "durum": durum, "not": "n" }
    }))
    .expect("a point")
}

#[test]
fn a_changed_row_shows_its_local_and_its_first_value_and_a_deleted_one_its_key() {
    let uid = kentos_domain::Uuid::from_bytes([7; 16]);
    let changes = kentos_domain::DataChanges {
        added: Vec::new(),
        changed: vec![kentos_domain::DataChange {
            key: 1,
            uid,
            now: Arc::new(point(1, "KentOS-guncel")),
            row: Arc::new(point(1, "acik")),
        }],
        deleted: vec![5],
        originals: [(5, (kentos_domain::Uuid::from_bytes([8; 16]), Arc::new(point(5, "izleme"))))].into(),
    };
    let lines = pending_lines(&changes, &["durum".to_owned(), "not".to_owned()]);
    assert_eq!(lines.len(), 2);
    assert_eq!((lines[0].key, lines[0].deleted), (1, false));
    assert_eq!(lines[0].cells, ["KentOS-guncel (önce: acik)", "n"]);
    assert_eq!((lines[1].key, lines[1].deleted), (5, true));
    assert_eq!(lines[1].cells, ["izleme", "n"]);
    assert_eq!(
        pending_words(1, 1, 0),
        "Bekleyen değişiklikler: 1 yeni, 1 değişen satır; kaynak okunamadığı için doğrulanmadı"
    );
}

#[test]
fn the_table_of_an_unread_source_lists_the_sessions_rows_and_a_save_leaves_none() {
    crate::data_layers::register();
    let path = parcels("bekleyen", 30);
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    let layer = node(&app, "parseller").expect("the layer").clone();
    let id = layer.id.clone();
    let _ = app.data_edits_event(Event::Start(id.clone()));
    // The 22nd row taken and its kullanim made “Okul”.
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let rows = data_write::rows_to_take(&layer, &settings, &[22], &|| false).expect("read");
    let doc = app.document.as_mut().expect("a drawing");
    let uid = doc.model.take_data_rows(&id, rows).expect("taken")[0].expect("22");
    let slot = doc.model.slot_of(uid).expect("its object");
    let mut e = doc.model.get(slot).cloned().expect("22");
    e.base_mut().attrs.insert("kullanim".into(), "Okul".into());
    assert!(doc.model.update(slot, e));
    // Offline: the table's question comes back refused.
    app.gis.features.layer = Some(id.clone());
    app.command_expanded = true;
    app.bottom_tab = crate::bottom::BottomTab::Table;
    let _ = app.data_table_event(DataEvent::Range(0, 12));
    let _ = app.data_table_tasks();
    for (generation, page, _) in app.gis.features.data.pending() {
        let _ = app.data_table_event(DataEvent::Answered(generation, page, Err("Kaynak okunamadı: bağlantı yok".into())));
    }
    assert!(app.gis.features.data.error().is_some(), "the source unread");
    // What the table lists: the changed row, its local value and the value it had.
    let changes = app.document.as_ref().expect("a drawing").model.data_changes(&id);
    let fields: Vec<String> = layer.fields.iter().map(|f| f.name.clone()).collect();
    let lines = pending_lines(&changes, &fields);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].key, 22);
    let at = fields.iter().position(|f| f == "kullanim").expect("kullanim");
    assert_eq!(lines[0].cells[at], "Okul (önce: Park)");
    // Back: read again, the taken row over the source's; saved, nothing waits.
    let _ = app.data_table_event(DataEvent::Apply);
    answer_pages(&mut app);
    assert!(app.gis.features.data.error().is_none());
    let task = app.data_edits_event(Event::Save(vec![id.clone()]));
    drive(&mut app, task);
    let left = app.document.as_ref().expect("a drawing").model.data_changes(&id);
    assert!(left.is_empty(), "nothing waits after the save");
    let _ = std::fs::remove_file(&path);
}

/// The picture (`.run/gui-evidence/kullanim-9151/`): the table of an unread source listing a new row and the changed
/// 22nd row with its value before.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    // A file of its own each picture: a source once opened stays open by its address.
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    crate::files_testing::usage_shots("9151", "01-bekleyen-degisiklikler", || {
        crate::data_layers::register();
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = parcels(&format!("bekleyen-resim-{n}"), 30);
        let mut app = app_with_drawing();
        app.add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described")])
            .expect("added");
        let layer = node(&app, "parseller").expect("the layer").clone();
        let id = layer.id.clone();
        let _ = app.data_edits_event(Event::Start(id.clone()));
        let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
        let rows = data_write::rows_to_take(&layer, &settings, &[22], &|| false).expect("read");
        let doc = app.document.as_mut().expect("a drawing");
        let uid = doc.model.take_data_rows(&id, rows).expect("taken")[0].expect("22");
        let slot = doc.model.slot_of(uid).expect("its object");
        let mut e = doc.model.get(slot).cloned().expect("22");
        e.base_mut().attrs.insert("kullanim".into(), "Okul".into());
        assert!(doc.model.update(slot, e));
        let mut new = doc.model.get(slot).cloned().expect("a shape");
        new.base_mut().attrs.insert("kullanim".into(), "Yeni".into());
        new.base_mut().id = 0;
        doc.model.add(new).expect("a new row");
        // Unreadable for sure, an open connection too: the file emptied.
        std::fs::write(&path, b"").expect("emptied");
        app.gis.features.layer = Some(id);
        app.command_expanded = true;
        app.bottom_tab = crate::bottom::BottomTab::Table;
        let _ = app.data_table_event(DataEvent::Range(0, 12));
        answer_pages(&mut app);
        app
    });
}
