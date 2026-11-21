//! Editing a data layer on the desktop (docs/adr/9111): a new row first in Öznitelik tablosu, the row's fields after it
//! is drawn, and the pending edits kept with the project on this device — saved, left with Projede tut, opened again,
//! checked against the table when written, and no row written twice.

use kentos_native_application::ExecutionContext;
use kentos_native_application::data_write;

use crate::app::{Dialog, Panel, Picker, Then};
use crate::data_edits::{Answer, Event};
use crate::data_journal::{Journals, file_place};
use crate::data_layers_tests::{node, parcels};
use crate::features::data::DataEvent;
use crate::files_testing::{app_with_drawing, drive, last_said, scratch};

/// A parcel drawn on `layer` as a drawing tool writes it: a new row; its slot.
fn draw(app: &mut crate::app::App, layer: &str, x: f64, y: f64) -> kentos_domain::Slot {
    let p = |dx: f64, dy: f64| serde_json::json!({ "x": x + dx, "y": y + dy });
    let input = serde_json::from_value(serde_json::json!({
        "layerId": layer,
        "objects": [{
            "geometry": { "kind": "polygon", "pts": [p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)] },
            "attrs": { "ada": "300", "kullanim": "Yeni" }
        }]
    }))
    .expect("an input");
    let doc = app.document.as_mut().expect("a drawing");
    let before = doc.model.next_slot();
    let done = kentos_native_application::create::execute(&mut ExecutionContext::new(&mut doc.model), input);
    assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
    (before..doc.model.next_slot())
        .filter_map(|s| u32::try_from(s).ok().map(kentos_domain::Slot))
        .find(|s| doc.model.get(*s).is_some())
        .expect("drawn")
}

/// A save to the table, as its thread does it, handed back as the app's message (its after-message work runs too).
fn save(app: &mut crate::app::App, layer: &str, force: bool) -> bool {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let writes = data_write::writes_of(doc, layer).expect("being edited");
    let outcome = data_write::write_group(std::slice::from_ref(&writes), doc.settings(), force)
        .map(|keys| (vec![writes], keys));
    let written = outcome.is_ok();
    let _ = app.update(crate::data_edits::event(Event::Saved(vec![layer.to_owned()], Box::new(outcome))));
    written
}

#[test]
fn a_new_row_is_listed_first_in_the_table_and_its_fields_open_after_it_is_drawn() {
    crate::data_layers::register();
    let path = parcels("yeni-satir", 12);
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "parseller").expect("described")])
        .expect("added");
    let id = node(&app, "parseller").expect("the layer").id.clone();
    let _ = app.data_edits_event(Event::Start(id.clone()));
    app.docks.close(Panel::Properties);

    // Drawn by a tool: no form on the layer, so the new row is chosen and Öznitelikler shows its fields.
    let slot = draw(&mut app, &id, 486_400.0, 4_420_100.0);
    app.selection.clear();
    app.form_on_create = Some(slot);
    let _ = app.open_form_on_create();
    assert_eq!(app.selection.ids(), vec![slot]);
    assert!(app.docks.is_shown(Panel::Properties), "Öznitelikler shown");

    // Turned off with the forms (Çizimden sonra formu aç): nothing opens.
    let _ = app.settings.choose(&[(crate::forms::ON_CREATE, serde_json::Value::Bool(false))]);
    app.docks.close(Panel::Properties);
    let other = draw(&mut app, &id, 486_420.0, 4_420_100.0);
    app.selection.clear();
    app.form_on_create = Some(other);
    let _ = app.open_form_on_create();
    assert!(app.selection.ids().is_empty() && !app.docks.is_shown(Panel::Properties));

    // The table lists both new rows first, in the drawing's order; pressing one chooses its object.
    let added = app.document.as_ref().expect("a drawing").model.data_changes(&id).added;
    assert_eq!(added.len(), 2);
    let _ = app.data_table_event(DataEvent::PressNew(added[1].0));
    assert_eq!(app.selection.ids(), vec![other]);
    let _ = std::fs::remove_file(&path);
}

#[test]
fn pending_edits_wait_with_the_project_and_come_back_when_it_is_opened() {
    crate::data_layers::register();
    let gpkg = parcels("gunluk", 40);
    let address = gpkg.display().to_string();
    let dir = scratch("veri-gunlugu");
    let journals = Journals::at(&dir.join("veri-oturumlari"));
    let file = dir.join("proje.kcad");
    let mut app = app_with_drawing();
    app.gis.data_journal = journals.clone();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    let layer = node(&app, "parseller").expect("the layer").clone();
    let id = layer.id.clone();
    let _ = app.data_edits_event(Event::Start(id.clone()));

    // A new row drawn; the 22nd row taken and its kullanim changed; the 5th deleted by its key.
    let new = draw(&mut app, &id, 486_400.0, 4_420_100.0);
    let new_uid = app.document.as_ref().expect("a drawing").model.uid(new).expect("its id");
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let rows = data_write::rows_to_take(&layer, &settings, &[22], &|| false).expect("read");
    let doc = app.document.as_mut().expect("a drawing");
    let taken = doc.model.take_data_rows(&id, rows).expect("taken")[0].expect("22");
    let slot = doc.model.slot_of(taken).expect("its object");
    let mut e = doc.model.get(slot).cloned().expect("22");
    e.base_mut().attrs.insert("kullanim".into(), "Okul".into());
    assert!(doc.model.update(slot, e));
    assert_eq!(doc.model.delete_data_rows(&id, &[5]), Ok(1));

    // Saved: the file keeps the new row only; the taken row and the deleted key are kept on this device beside it.
    app.picker = Picker::File(file.clone());
    let task = app.run("file.save");
    drive(&mut app, task);
    assert!(!app.document.as_ref().expect("a drawing").dirty());
    let saved = crate::document::Document::read(&file).expect("the file reads").model.to_snapshot_v2();
    let values: Vec<Option<&str>> = saved
        .entities
        .iter()
        .filter(|e| e.base().layer_id == id)
        .map(|e| e.base().attrs.get("kullanim").map(String::as_str))
        .collect();
    assert_eq!(values, vec![Some("Yeni")], "the new row only: no row of the table in the file");
    let kept = journals.file(&file_place(&file)).expect("a journal's file");
    let journal = std::fs::read_to_string(&kept).expect("the journal");
    assert!(journal.contains("\"key\":22") && journal.contains("Okul") && journal.contains("\"deleted\":[5]"), "{journal}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&kept).expect("its facts").permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "the user's alone");
    }

    // Left with Projede tut: not written, not asked again; the drawing, saved, closes.
    let _ = app.leave(Then::CloseTab);
    assert_eq!(app.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::DataEdits)));
    let _ = app.data_edits_event(Event::Answer(Answer::Keep));
    assert!(app.document.is_none(), "closed");

    // Opened again: everything waiting is back, and the drawing is as saved.
    app.picker = Picker::File(file.clone());
    let task = app.run("file.open");
    drive(&mut app, task);
    let model = &app.document.as_ref().expect("opened").model;
    let c = model.data_changes(&id);
    assert_eq!((c.added.len(), c.changed.len(), c.deleted.len()), (1, 1, 1));
    assert_eq!(c.changed[0].key, 22);
    assert_eq!(c.changed[0].now.base().attrs.get("kullanim").map(String::as_str), Some("Okul"));
    assert_eq!(c.deleted, vec![5]);
    assert!(!model.is_dirty() && !model.can_undo());
    assert!(app.log.lines().any(|l| l.text.contains("Bekleyen veri düzenlemeleri geri geldi")), "{}", last_said(&app));

    // Someone changes the 22nd row meanwhile: the save asks first.
    let source = kentos_data::registry().source(&address).expect("the source");
    let mut tx = source.begin().expect("a transaction");
    tx.update(
        "parseller",
        &[kentos_data::Change {
            fid: 22,
            geometry: None,
            values: vec![("alan".into(), kentos_data::Value::Real(700.0))],
        }],
    )
    .expect("changed");
    tx.commit().expect("written");
    assert!(!save(&mut app, &id, false));
    assert_eq!(app.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::DataConflict)));
    let _ = app.data_edits_event(Event::KeepConflict);

    // Üstüne yaz: written. The journal follows the table: nothing waits, and the file's new row, written now, is named.
    assert!(save(&mut app, &id, true));
    let journal = std::fs::read_to_string(&kept).expect("the journal");
    assert!(journal.contains("\"layers\":[]") && journal.contains(&new_uid.to_string()), "{journal}");

    // Opened again without saving the project: the file's new row is in the table now and does not come back.
    let task = app.run("file.open");
    drive(&mut app, task);
    let model = &app.document.as_ref().expect("opened").model;
    assert_eq!(model.count(&id), 0, "not written twice");
    assert!(!model.editing(&id) && !model.is_dirty());

    // Saved again: nothing waits, so the journal goes.
    app.document.as_mut().expect("opened").model.mark_unsaved();
    let task = app.run("file.save");
    drive(&mut app, task);
    assert!(!kept.exists(), "no journal left");
    let _ = std::fs::remove_file(&gpkg);
    let _ = std::fs::remove_dir_all(&dir);
}
