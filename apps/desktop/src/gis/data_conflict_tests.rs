//! The municipal report's 26th item over a GeoPackage table (docs/adr/9148; the same write path as PostGIS's): the
//! 22nd parcel (kullanim “Park”, alan 625) is taken and its kullanim made “Okul” while someone else writes the table.
//! Another field changed there is merged in one save; the same field to another value asks, field by field, with the
//! three values, and nothing is written until chosen; Vazgeç keeps the draft; the table's value chosen stays in the
//! table, the user's chosen is written; a choice met by yet another change is checked again; a row deleted in the table
//! can be written again as a new row, and only where the connection may add rows (docs/adr/9162). On a real PostgreSQL
//! (`KENTOS_DATA_PG`): two sessions on their own connections, and an account that may not add rows (its real rights).

use kentos_data::Value;
use kentos_native_application::data_write;

use crate::app::{App, Dialog};
use crate::data_edits::Event;
use crate::data_layers_tests::{node, parcels, parcels_in};
use crate::files_testing::{app_with_drawing, drive};
use crate::pg_testing::{Relay, account, pg};

/// The app with the parcels' table (40 rows) bound and being edited, its 22nd row taken and its kullanim made “Okul”:
/// the app, the layer's id and the file's address.
fn taken(name: &str) -> (App, String, String) {
    let address = parcels(name, 40).display().to_string();
    let (app, id) = taken_on(&address, "parseller");
    (app, id, address)
}

/// The same over `table` of the source at `address`: the app and the layer's id.
fn taken_on(address: &str, table: &str) -> (App, String) {
    let (mut app, id) = editing(address, table);
    set(&mut app, &id, 22, "kullanim", "Okul");
    (app, id)
}

/// The app with `table` of the source at `address` bound and being edited: the app and the layer's id.
fn editing(address: &str, table: &str) -> (App, String) {
    crate::data_layers::register();
    let mut app = app_with_drawing();
    let described = crate::data_layers::describe(address, table).expect("described");
    let name = described.name.clone();
    app.add_data_layers(vec![described]).expect("added");
    let id = node(&app, &name).expect("the layer").id.clone();
    let _ = app.data_edits_event(Event::Start(id.clone()));
    (app, id)
}

/// Row `fid` taken (when it is not yet) and its `field` set to `value` on this device.
fn set(app: &mut App, id: &str, fid: i64, field: &str, value: &str) {
    let doc = app.document.as_ref().expect("a drawing");
    let uid = match doc.model.taken_uid(id, fid) {
        Some(uid) => uid,
        None => {
            let layer = doc.model.layers().get(id).cloned().expect("the layer");
            let settings = doc.model.settings().clone();
            let rows =
                data_write::rows_to_take(&layer, &settings, &[fid], &|| false).expect("read");
            let doc = app.document.as_mut().expect("a drawing");
            doc.model.take_data_rows(id, rows).expect("taken")[0].expect("the row")
        }
    };
    let doc = app.document.as_mut().expect("a drawing");
    let slot = doc.model.slot_of(uid).expect("its object");
    let mut e = doc.model.get(slot).cloned().expect("the row");
    e.base_mut().attrs.insert(field.into(), value.into());
    assert!(doc.model.update(slot, e));
}

/// Someone else writes the 22nd row, or deletes it (`values` none).
fn others(address: &str, values: Option<Vec<(&str, Value)>>) {
    others_on(address, "parseller", values);
}

/// The same in `table`.
fn others_on(address: &str, table: &str, values: Option<Vec<(&str, Value)>>) {
    let source = kentos_data::registry().source(address).expect("the source");
    let mut tx = source.begin().expect("a transaction");
    match values {
        Some(values) => tx
            .update(
                table,
                &[kentos_data::Change {
                    fid: 22,
                    geometry: None,
                    values: values.into_iter().map(|(k, v)| (k.to_owned(), v)).collect(),
                }],
            )
            .expect("changed"),
        None => tx.delete(table, &[22]).expect("deleted"),
    }
    tx.commit().expect("written");
}

/// The table's kullanim and alan of row `fid`, read apart from the app.
fn table(address: &str, fid: i64) -> Option<(String, f64)> {
    table_on(address, "parseller", fid)
}

/// The same in `table`.
fn table_on(address: &str, table: &str, fid: i64) -> Option<(String, f64)> {
    let source = kentos_data::registry().source(address).expect("the source");
    let schema = source.schema(table).expect("its schema");
    let mut tx = source.begin().expect("a transaction");
    let rows = tx.rows(table, &[fid]).expect("read");
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
    let c = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .data_changes(id);
    c.added.len() + c.changed.len() + c.deleted.len()
}

#[test]
fn another_field_changed_in_the_table_is_merged_in_one_save() {
    let (mut app, id, address) = taken("cakisma-ayri-alan");
    others(&address, Some(vec![("alan", Value::Real(700.0))]));
    save(&mut app, &id);
    assert!(!asking(&app), "nothing to ask: different fields");
    assert_eq!(
        table(&address, 22),
        Some(("Okul".into(), 700.0)),
        "both changes in the table"
    );
    assert_eq!(pending(&app, &id), 0);
}

#[test]
fn the_same_field_asks_with_its_three_values_and_writes_nothing_until_chosen() {
    let (mut app, id, address) = taken("cakisma-ayni-alan");
    others(
        &address,
        Some(vec![("kullanim", Value::Text("Park2".into()))]),
    );
    save(&mut app, &id);
    assert!(asking(&app));
    let c = app.gis.data_edits.conflict.as_ref().expect("the clash");
    assert_eq!(c.rows.len(), 1);
    assert_eq!(c.rows[0].key, 22);
    let f = &c.rows[0].fields;
    assert_eq!(f.len(), 1, "{f:?}");
    assert_eq!(
        (
            f[0].field.as_str(),
            f[0].first.as_deref(),
            f[0].mine.as_deref(),
            f[0].theirs.as_deref()
        ),
        ("kullanim", Some("Park"), Some("Okul"), Some("Park2"))
    );
    assert!(!c.ready(), "nothing chosen for the user");
    // Kaydet before a choice: nothing happens, the window stays.
    let task = app.data_edits_event(Event::Settle);
    drive(&mut app, task);
    assert!(asking(&app) && app.gis.data_edits.conflict.is_some());
    assert_eq!(
        table(&address, 22),
        Some(("Park2".into(), 625.0)),
        "the table as the other left it"
    );
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
    others(
        &address,
        Some(vec![
            ("kullanim", Value::Text("Park2".into())),
            ("alan", Value::Real(650.0)),
        ]),
    );
    save(&mut app, &id);
    let _ = app.data_edits_event(Event::Choose(0, 0, true));
    // Before it is saved, the row changes once more.
    others(
        &address,
        Some(vec![("kullanim", Value::Text("Park3".into()))]),
    );
    let task = app.data_edits_event(Event::Settle);
    drive(&mut app, task);
    assert!(
        asking(&app),
        "checked again: the table moved on since the choice"
    );
    let c = app.gis.data_edits.conflict.as_ref().expect("the clash");
    assert_eq!(c.rows[0].fields[0].theirs.as_deref(), Some("Park3"));
    assert_eq!(
        table(&address, 22),
        Some(("Park3".into(), 650.0)),
        "nothing written meanwhile"
    );
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
    assert_eq!(
        table(&address, 41),
        Some(("Okul".into(), 625.0)),
        "written as a new row"
    );
    assert_eq!(pending(&app, &id), 0);
}

/// The clash of a deleted row as a connection that may not add rows meets it (docs/adr/9162).
fn without_the_insert_right(app: &mut App) {
    let c = app.gis.data_edits.conflict.take().expect("the clash");
    assert!(
        c.rows.iter().all(|r| r.may_add),
        "a GeoPackage file takes new rows"
    );
    let rows = c
        .rows
        .into_iter()
        .map(|mut r| {
            r.may_add = false;
            r
        })
        .collect();
    app.gis.data_edits.conflict = Some(super::Conflict::of(c.layers, rows));
}

#[test]
fn without_the_insert_right_a_row_deleted_in_the_table_is_let_go() {
    let (mut app, id, address) = taken("cakisma-ekleme-yetkisiz");
    others(&address, None);
    save(&mut app, &id);
    assert!(asking(&app));
    without_the_insert_right(&mut app);
    let c = app.gis.data_edits.conflict.as_ref().expect("the clash");
    assert_eq!(
        c.chosen,
        vec![vec![Some(false)]],
        "its one way, let go, chosen"
    );
    assert!(c.ready());
    // Asking for a new row changes nothing.
    let _ = app.data_edits_event(Event::Choose(0, 0, true));
    let c = app.gis.data_edits.conflict.as_ref().expect("the clash");
    assert_eq!(c.chosen, vec![vec![Some(false)]]);
    let count = app.data_conflict_count(c);
    assert!(count.contains("1 satırın taslağı bırakılır"), "{count}");
    let task = app.data_edits_event(Event::Settle);
    drive(&mut app, task);
    assert!(!asking(&app));
    assert_eq!(table(&address, 22), None, "the old key stays deleted");
    assert_eq!(table(&address, 41), None, "no new row");
    assert_eq!(pending(&app, &id), 0);
}

/// On a real PostgreSQL (the usage report's 26th item): two KentOS sessions edit the 22nd row of one table, each on its
/// own connection (the first through a relay of its own). The second's alan is written first; the first's kullanim is
/// merged with it in one save. Then both change kullanim: the one writing second is asked with the three values and
/// nothing is written until it chooses; its own value chosen is written, the other's alan kept.
#[test]
fn on_postgresql_two_sessions_merge_other_fields_and_ask_on_the_same_one() {
    let Some(pg) = pg("PostgreSQL'de iki oturumlu çakışma") else {
        return;
    };
    crate::data_layers::register();
    let t = crate::pg_testing::table("cakisma");
    parcels_in(
        kentos_data::registry()
            .source(&pg)
            .expect("the database")
            .as_ref(),
        &t,
        40,
    );
    let relay = Relay::over(&pg);
    let (mut first, a) = taken_on(&relay.address, &t);
    let (mut second, b) = editing(&pg, &t);
    set(&mut second, &b, 22, "alan", "700");
    save(&mut second, &b);
    assert_eq!(table_on(&pg, &t, 22), Some(("Park".into(), 700.0)));
    save(&mut first, &a);
    assert!(!asking(&first), "different fields: nothing to ask");
    assert_eq!(
        table_on(&pg, &t, 22),
        Some(("Okul".into(), 700.0)),
        "both changes in one row"
    );
    assert_eq!(pending(&first, &a), 0);
    // The same field: the second writes Pazar first; the first's Okul2 is asked with the three values.
    set(&mut first, &a, 22, "kullanim", "Okul2");
    set(&mut second, &b, 22, "kullanim", "Pazar");
    save(&mut second, &b);
    save(&mut first, &a);
    assert!(asking(&first));
    let c = first.gis.data_edits.conflict.as_ref().expect("the clash");
    let f = &c.rows[0].fields;
    assert_eq!(
        (
            f[0].field.as_str(),
            f[0].first.as_deref(),
            f[0].mine.as_deref(),
            f[0].theirs.as_deref()
        ),
        ("kullanim", Some("Okul"), Some("Okul2"), Some("Pazar"))
    );
    assert_eq!(
        table_on(&pg, &t, 22),
        Some(("Pazar".into(), 700.0)),
        "nothing written before the choice"
    );
    let _ = first.data_edits_event(Event::Choose(0, 0, true));
    let task = first.data_edits_event(Event::Settle);
    drive(&mut first, task);
    assert!(!asking(&first));
    assert_eq!(table_on(&pg, &t, 22), Some(("Okul2".into(), 700.0)));
    assert_eq!((pending(&first, &a), pending(&second, &b)), (0, 0));
    crate::pg_testing::drop_table(&pg, &t);
}

/// On a real PostgreSQL with a second account (the usage report's 32nd item, docs/adr/9162): it may read, change and
/// delete the parcels but not add one. Its session's 22nd row is deleted in the table meanwhile: the window lets the
/// draft go, its one way, and a press for a new row changes nothing; nothing is added. The owner's account may add.
#[test]
fn on_postgresql_an_account_without_the_insert_right_lets_a_deleted_rows_draft_go() {
    let Some(pg) = pg("PostgreSQL'de ekleme yetkisiz hesap") else {
        return;
    };
    crate::data_layers::register();
    let t = crate::pg_testing::table("ekleme");
    let owner = kentos_data::registry().source(&pg).expect("the database");
    parcels_in(owner.as_ref(), &t, 40);
    let name = format!("kentos_ikinci_{}", std::process::id());
    let second = account(&pg, &name, &t, "SELECT, UPDATE, DELETE");
    let theirs = kentos_data::registry()
        .source(&second)
        .expect("the second account");
    let caps = theirs.capabilities(&t).expect("its rights");
    assert!(
        !caps.insert && caps.update_values && caps.delete,
        "{caps:?}"
    );
    assert!(owner.capabilities(&t).expect("the owner's rights").insert);
    let (mut app, id) = taken_on(&second, &t);
    others_on(&pg, &t, None);
    save(&mut app, &id);
    assert!(asking(&app));
    let c = app.gis.data_edits.conflict.as_ref().expect("the clash");
    assert!(
        c.rows[0].gone() && !c.rows[0].may_add,
        "the account's real right: no new row"
    );
    assert_eq!(
        c.chosen,
        vec![vec![Some(false)]],
        "its one way, let go, chosen"
    );
    let _ = app.data_edits_event(Event::Choose(0, 0, true));
    let c = app.gis.data_edits.conflict.as_ref().expect("the clash");
    assert_eq!(
        c.chosen,
        vec![vec![Some(false)]],
        "a new row is not offered"
    );
    let count = app.data_conflict_count(c);
    assert!(count.contains("1 satırın taslağı bırakılır"), "{count}");
    let task = app.data_edits_event(Event::Settle);
    drive(&mut app, task);
    assert!(!asking(&app));
    assert_eq!(table_on(&pg, &t, 22), None, "the old key stays deleted");
    assert_eq!(crate::pg_testing::rows(&pg, &t), 39, "no row added");
    assert_eq!(pending(&app, &id), 0);
    crate::pg_testing::drop_table(&pg, &t);
    crate::pg_testing::sql(
        &pg,
        &[
            &format!("DROP OWNED BY {name}"),
            &format!("DROP ROLE {name}"),
        ],
    );
}

/// The window's picture (`.run/gui-evidence/kullanim-9148/`): the same field clashing, its three values and the choice.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9148", "01-alan-cakismasi", || {
        let (mut app, id, address) = taken("cakisma-resim");
        others(
            &address,
            Some(vec![("kullanim", Value::Text("Park2".into()))]),
        );
        save(&mut app, &id);
        let _ = app.data_edits_event(Event::Choose(0, 0, false));
        app
    });
}

/// The picture (`.run/gui-evidence/kullanim-9162/`): a row deleted in the table, met by a connection that may not add
/// rows: let go, said why.
#[test]
#[ignore = "pictures for the owner"]
fn insert_right_screens() {
    // A file each time: the light and the dark picture each delete its row.
    static TURN: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    crate::files_testing::usage_shots("9162", "01-ekleme-yetkisi-yok", || {
        let n = TURN.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (mut app, id, address) = taken(&format!("cakisma-yetki-resim-{n}"));
        others(&address, None);
        save(&mut app, &id);
        without_the_insert_right(&mut app);
        app
    });
}
