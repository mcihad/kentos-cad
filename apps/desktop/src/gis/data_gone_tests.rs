//! The municipal report's 32nd item over a GeoPackage table (docs/adr/9152): rows 22 and 23 taken and changed, row 22
//! deleted by someone else. The first save writes neither; the window says row 22 is gone with its two ways and how
//! many rows its choice writes; the table lists row 22 among its rows meanwhile. Its draft let go, row 23 is written
//! alone and row 22 is neither written again nor brought back; nothing waits after.

use kentos_data::Value;
use kentos_native_application::data_write;

use crate::app::{App, Dialog};
use crate::data_edits::Event;
use crate::data_layers_tests::{node, parcels};
use crate::files_testing::{app_with_drawing, drive};

/// The parcels' table being edited, rows 22 and 23 taken with kullanim “Okul” and “Pazar”: the app, the layer, the file.
fn two_taken(name: &str) -> (App, String, String) {
    crate::data_layers::register();
    let address = parcels(name, 30).display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    let layer = node(&app, "parseller").expect("the layer").clone();
    let id = layer.id.clone();
    let _ = app.data_edits_event(Event::Start(id.clone()));
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let rows = data_write::rows_to_take(&layer, &settings, &[22, 23], &|| false).expect("read");
    let doc = app.document.as_mut().expect("a drawing");
    let uids = doc.model.take_data_rows(&id, rows).expect("taken");
    for (uid, value) in uids.into_iter().zip(["Okul", "Pazar"]) {
        let slot = doc.model.slot_of(uid.expect("taken")).expect("its object");
        let mut e = doc.model.get(slot).cloned().expect("a row");
        e.base_mut().attrs.insert("kullanim".into(), value.into());
        assert!(doc.model.update(slot, e));
    }
    (app, id, address)
}

fn kullanim(address: &str, fid: i64) -> Option<String> {
    let source = kentos_data::registry().source(address).expect("the source");
    let schema = source.schema("parseller").expect("its schema");
    let mut tx = source.begin().expect("a transaction");
    let rows = tx.rows("parseller", &[fid]).expect("read");
    let _ = tx.rollback();
    match rows.first()?.values.get(schema.index("kullanim")?)? {
        Value::Text(t) => Some(t.clone()),
        _ => None,
    }
}

fn save(app: &mut App, id: &str) {
    let task = app.data_edits_event(Event::Save(vec![id.to_owned()]));
    drive(app, task);
}

#[test]
fn a_rows_draft_let_go_leaves_the_sound_row_to_be_written_alone() {
    let (mut app, id, address) = two_taken("silinen-taslak");
    let source = kentos_data::registry().source(&address).expect("the source");
    let mut tx = source.begin().expect("a transaction");
    tx.delete("parseller", &[22]).expect("deleted");
    tx.commit().expect("written");
    // The first save writes neither.
    save(&mut app, &id);
    assert_eq!(app.dialog, Some(Dialog::Gis(crate::gis::GisDialogKind::DataConflict)));
    assert_eq!(kullanim(&address, 23).as_deref(), Some("Konut"), "row 23 not written yet");
    let c = app.gis.data_edits.conflict.clone().expect("the clash");
    assert_eq!(c.rows.len(), 1);
    assert!(c.rows[0].gone() && c.rows[0].key == 22);
    assert_eq!(app.data_conflict_count(&c), "Seçilenleri kaydet 2 satırı tek işlemde yazar.");
    // The table lists row 22 meanwhile.
    assert!(app.gis.data_edits.gone.get(&id).is_some_and(|k| k.contains(&22)));
    // Its draft let go: the count says so; row 23 written alone, row 22 not again.
    let _ = app.data_edits_event(Event::Choose(0, 0, false));
    let c = app.gis.data_edits.conflict.clone().expect("the clash");
    assert_eq!(
        app.data_conflict_count(&c),
        "Seçilenleri kaydet 1 satırı tek işlemde yazar; 1 satırın taslağı bırakılır."
    );
    let task = app.data_edits_event(Event::Settle);
    drive(&mut app, task);
    assert_eq!(app.dialog, None);
    assert_eq!(kullanim(&address, 23).as_deref(), Some("Pazar"));
    assert_eq!(kullanim(&address, 22), None, "no row 22 written again");
    assert_eq!(kullanim(&address, 31), None, "nor as a new row");
    let left = app.document.as_ref().expect("a drawing").model.data_changes(&id);
    assert!(left.is_empty(), "nothing waits");
    assert!(app.gis.data_edits.gone.get(&id).is_none_or(|k| k.is_empty()));
}

#[test]
fn a_taken_rows_untouched_field_shows_the_sources_value_now() {
    let row = |kullanim: &str, alan: &str| -> kentos_commands::Entity {
        serde_json::from_value(serde_json::json!({
            "kind": "point", "id": 1, "layerId": "p", "p": { "x": 0.0, "y": 0.0 },
            "attrs": { "kullanim": kullanim, "alan": alan }
        }))
        .expect("a point")
    };
    let (now, taken) = (row("Okul", "625"), row("Park", "625"));
    let page: std::collections::BTreeMap<String, String> =
        [("kullanim".to_owned(), "Park".to_owned()), ("alan".to_owned(), "700".to_owned())].into();
    use crate::features::pending_rows::cell_value;
    assert_eq!(cell_value(Some((&now, &taken)), &page, "kullanim"), "Okul", "changed here: the draft's");
    assert_eq!(cell_value(Some((&now, &taken)), &page, "alan"), "700", "not changed here: the source's now");
    assert_eq!(cell_value(None, &page, "alan"), "700");
}

#[test]
fn letting_a_layers_changes_go_says_how_many_rows() {
    let (mut app, id, _) = two_taken("geri-al-sayi");
    let _ = app.data_edits_event(Event::Rollback(vec![id.clone()]));
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains("parseller: 2 değişen satır"), "{said}");
}

/// The pictures (`.run/gui-evidence/kullanim-9152/`): the window with row 22 gone, its draft let go and the count; the
/// table listing row 22 among its rows after Vazgeç.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let gone = || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (mut app, id, address) = two_taken(&format!("silinen-resim-{n}"));
        let source = kentos_data::registry().source(&address).expect("the source");
        let mut tx = source.begin().expect("a transaction");
        tx.delete("parseller", &[22]).expect("deleted");
        tx.commit().expect("written");
        save(&mut app, &id);
        (app, id)
    };
    crate::files_testing::usage_shots("9152", "01-silinen-satir-penceresi", || {
        let (mut app, _) = gone();
        let _ = app.data_edits_event(Event::Choose(0, 0, false));
        app
    });
    crate::files_testing::usage_shots("9152", "02-silinen-satir-tabloda", || {
        let (mut app, id) = gone();
        let _ = app.data_edits_event(Event::KeepConflict);
        app.gis.features.layer = Some(id);
        app.command_expanded = true;
        app.bottom_tab = crate::bottom::BottomTab::Table;
        let _ = app.data_table_event(crate::features::data::DataEvent::Range(0, 12));
        crate::data_layers_tests::answer_pages(&mut app);
        app
    });
}
