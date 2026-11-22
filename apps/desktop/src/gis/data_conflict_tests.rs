//! The municipal report's 26th item over a GeoPackage table (docs/adr/9148; the same write path as PostGIS's): the
//! 22nd parcel (kullanim “Park”, alan 625) is taken and its kullanim made “Okul” while someone else writes the table.
//! Another field changed there is merged in one save; the same field to another value asks, field by field, with the
//! three values, and nothing is written until chosen; Vazgeç keeps the draft; the table's value chosen stays in the
//! table, the user's chosen is written; a choice met by yet another change is checked again; a row deleted in the table
//! can be written again as a new row.

use kentos_data::Value;
use kentos_native_application::data_write;

use crate::app::{App, Dialog};
use crate::data_edits::Event;
use crate::data_layers_tests::{node, parcels};
use crate::files_testing::{app_with_drawing, drive};

/// The app with the parcels' table (40 rows) bound and being edited, its 22nd row taken and its kullanim made “Okul”:
/// the app, the layer's id and the file's address.
fn taken(name: &str) -> (App, String, String) {
    crate::data_layers::register();
    let address = parcels(name, 40).display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
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
    (app, id, address)
}

/// Someone else writes the 22nd row, or deletes it (`values` none).
fn others(address: &str, values: Option<Vec<(&str, Value)>>) {
    let source = kentos_data::registry().source(address).expect("the source");
    let mut tx = source.begin().expect("a transaction");
    match values {
        Some(values) => tx
            .update(
                "parseller",
                &[kentos_data::Change {
                    fid: 22,
                    geometry: None,
                    values: values.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
                }],
            )
            .expect("changed"),
        None => tx.delete("parseller", &[22]).expect("deleted"),
    }
    tx.commit().expect("written");
}

/// The table's kullanim and alan of row `fid`, read apart from the app.
fn table(address: &str, fid: i64) -> Option<(String, f64)> {
    let source = kentos_data::registry().source(address).expect("the source");
    let schema = source.schema("parseller").expect("its schema");
    let mut tx = source.begin().expect("a transaction");
    let rows = tx.rows("parseller", &[fid]).expect("read");
    let _ = tx.rollback();
    let f = rows.first()?;
    let at = |n: &str| f.values.get(schema.index(n).expect("a field")).cloned();
    let kullanim = match at("kullanim") {
        Some(Value::Text(t)) => t,
        other => panic!("{other:?}"),
    };
    let alan = match at("alan") {
        Some(Value::Real(r)) => r,
        other => panic!("{other:?}"),
    };
    Some((kullanim, alan))
}

/// Değişiklikleri kaydet, driven to its end.
fn save(app: &mut App, id: &str) {
    let task = app.data_edits_event(Event::Save(vec![id.to_owned()]));
    drive(app, task);
}

fn asking(app: &App) -> bool {
    app.dialog == Some(Dialog::Gis(crate::gis::GisDialogKind::DataConflict))
}

fn pending(app: &App, id: &str) -> usize {
    let c = app.document.as_ref().expect("a drawing").model.data_changes(id);
    c.added.len() + c.changed.len() + c.deleted.len()
}

#[test]
fn another_field_changed_in_the_table_is_merged_in_one_save() {
    let (mut app, id, address) = taken("cakisma-ayri-alan");
    others(&address, Some(vec![("alan", Value::Real(700.0))]));
    save(&mut app, &id);
    assert!(!asking(&app), "nothing to ask: different fields");
    assert_eq!(table(&address, 22), Some(("Okul".into(), 700.0)), "both changes in the table");
    assert_eq!(pending(&app, &id), 0);
}

#[test]
fn the_same_field_asks_with_its_three_values_and_writes_nothing_until_chosen() {
    let (mut app, id, address) = taken("cakisma-ayni-alan");
    others(&address, Some(vec![("kullanim", Value::Text("Park2".into()))]));
    save(&mut app, &id);
    assert!(asking(&app));
    let c = app.gis.data_edits.conflict.as_ref().expect("the clash");
    assert_eq!(c.rows.len(), 1);
    assert_eq!(c.rows[0].key, 22);
    let f = &c.rows[0].fields;
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(
        (f[0].field.as_str(), f[0].first.as_deref(), f[0].mine.as_deref(), f[0].theirs.as_deref()),
        ("kullanim", Some("Park"), Some("Okul"), Some("Park2"))
    );
    assert!(!c.ready(), "nothing chosen for the user");
    // Kaydet before a choice: nothing happens, the window stays.
    let task = app.data_edits_event(Event::Settle);
    drive(&mut app, task);
    assert!(asking(&app) && app.gis.data_edits.conflict.is_some());
    assert_eq!(table(&address, 22), Some(("Park2".into(), 625.0)), "the table as the other left it");
    // Vazgeç: the draft kept.
    let _ = app.data_edits_event(Event::KeepConflict);
    assert!(!asking(&app));
    assert_eq!(pending(&app, &id), 1, "the change waits");
    // Asked again; the table's value chosen: it stays, nothing waits.
    save(&mut app, &id);
    assert!(asking(&app));
    let _ = app.data_edits_event(Event::Choose(0, 0, false));
    let task = app.data_edits_event(Event::Settle);
    drive(&mut app, task);
    assert!(!asking(&app));
    assert_eq!(table(&address, 22), Some(("Park2".into(), 625.0)));
    assert_eq!(pending(&app, &id), 0);
}

#[test]
fn the_users_value_chosen_is_written_and_a_choice_met_by_another_change_is_checked_again() {
    let (mut app, id, address) = taken("cakisma-benimki");
    others(&address, Some(vec![("kullanim", Value::Text("Park2".into())), ("alan", Value::Real(650.0))]));
    save(&mut app, &id);
    let _ = app.data_edits_event(Event::Choose(0, 0, true));
    // Before it is saved, the row changes once more.
    others(&address, Some(vec![("kullanim", Value::Text("Park3".into()))]));
    let task = app.data_edits_event(Event::Settle);
    drive(&mut app, task);
    assert!(asking(&app), "checked again: the table moved on since the choice");
    let c = app.gis.data_edits.conflict.as_ref().expect("the clash");
    assert_eq!(c.rows[0].fields[0].theirs.as_deref(), Some("Park3"));
    assert_eq!(table(&address, 22), Some(("Park3".into(), 650.0)), "nothing written meanwhile");
    // Mine again: written, the table's other change kept.
    let _ = app.data_edits_event(Event::Choose(0, 0, true));
    let task = app.data_edits_event(Event::Settle);
    drive(&mut app, task);
    assert!(!asking(&app));
    assert_eq!(table(&address, 22), Some(("Okul".into(), 650.0)));
    assert_eq!(pending(&app, &id), 0);
}

#[test]
fn a_row_deleted_in_the_table_can_be_written_again_as_a_new_row() {
    let (mut app, id, address) = taken("cakisma-silinmis");
    others(&address, None);
    save(&mut app, &id);
    assert!(asking(&app));
    let c = app.gis.data_edits.conflict.as_ref().expect("the clash");
    assert!(c.rows[0].gone() && c.rows[0].mine.is_some());
    let _ = app.data_edits_event(Event::Choose(0, 0, true));
    let task = app.data_edits_event(Event::Settle);
    drive(&mut app, task);
    assert!(!asking(&app));
    assert_eq!(table(&address, 22), None, "the old key stays deleted");
    assert_eq!(table(&address, 41), Some(("Okul".into(), 625.0)), "written as a new row");
    assert_eq!(pending(&app, &id), 0);
}

/// The window's picture (`.run/gui-evidence/kullanim-9148/`): the same field clashing, its three values and the choice.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9148", "01-alan-cakismasi", || {
        let (mut app, id, address) = taken("cakisma-resim");
        others(&address, Some(vec![("kullanim", Value::Text("Park2".into()))]));
        save(&mut app, &id);
        let _ = app.data_edits_event(Event::Choose(0, 0, false));
        app
    });
}
