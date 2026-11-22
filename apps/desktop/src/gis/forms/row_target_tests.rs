//! The municipal report's 27th item over a GeoPackage table (docs/adr/9149): with a published form on the parcels'
//! data layer, the 22nd row chosen in the attribute table reaches Formla düzenle. Not being edited, it says so with
//! Düzenlemeyi başlat; being edited, the row is read, taken and its record opened; asked again, the same taken object
//! opens, no second copy. A read-only connection's row opens in its form for reading, its head saying so, nothing
//! taken or written; ten switches between same-named tables write the chosen one alone (docs/adr/9159).

use crate::app::App;
use crate::data_edits::Event;
use crate::data_layers_tests::{node, parcels};
use crate::files_testing::{app_with_drawing, drive};
use crate::forms::entry::Event as FormEvent;

/// A published form on `layer` with its kullanim, named `id`.
fn publish_form(app: &mut App, layer: &str, id: &str) {
    let body = serde_json::json!({ "revision": 1, "items": [{ "type": "field", "id": "kullanim", "field": "kullanim" }] });
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = serde_json::json!({
        "id": id, "name": "Parsel formu", "layer": layer, "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body, "note": "İlk yayın" }]
    });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms.push(serde_json::from_value(form).expect("a form"));
    doc.model.set_settings(settings);
}

/// The parcels' table bound (read only when asked), a published form on it with its kullanim; the app and the layer's
/// id.
fn with_form(name: &str, read_only: bool) -> (App, String) {
    crate::data_layers::register();
    let address = parcels(name, 40).display().to_string();
    let mut app = app_with_drawing();
    let mut described = crate::data_layers::describe(&address, "parseller").expect("described");
    described.data.read_only = read_only;
    app.add_data_layers(vec![described]).expect("added");
    let id = node(&app, "parseller").expect("the layer").id.clone();
    publish_form(&mut app, &id, "parsel-formu");
    (app, id)
}

/// The kullanim of row `fid` in the file, read apart from the app.
fn kullanim(address: &str, fid: i64) -> String {
    let source = kentos_data::registry().source(address).expect("the source");
    let schema = source.schema("parseller").expect("its schema");
    let mut tx = source.begin().expect("a transaction");
    let rows = tx.rows("parseller", &[fid]).expect("read");
    let _ = tx.rollback();
    match rows.first().and_then(|f| f.values.get(schema.index("kullanim").expect("a field")).cloned()) {
        Some(kentos_data::Value::Text(t)) => t,
        other => panic!("{other:?}"),
    }
}

fn form_event(app: &mut App, event: FormEvent) {
    let task = app.form_entry_event(event);
    drive(app, task);
}

fn form_edit(app: &mut App) {
    let task = app.run("data.formEdit");
    drive(app, task);
}

fn taken(app: &App, id: &str) -> usize {
    app.document.as_ref().expect("a drawing").model.taken_objects(id).len()
}

#[test]
fn a_row_chosen_in_the_table_opens_its_record_through_the_form() {
    let (mut app, id) = with_form("form-satir", false);
    app.gis.features.data.choose_rows(&id, [22].into());
    // Not being edited: said, with the way to open it.
    form_edit(&mut app);
    assert!(app.gis.form_entry.is_none());
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains("düzenlemeye açık değil; 22 anahtarlı kaydı formla düzenlemek için"), "{said}");
    // Being edited: the row read, taken and its record opened.
    let _ = app.data_edits_event(Event::Start(id.clone()));
    form_edit(&mut app);
    let entry = app.gis.form_entry.as_ref().expect("the form opens");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let uid = doc.taken_uid(&id, 22).expect("the row taken");
    assert_eq!(entry.uid_text(), uid.to_string(), "the record is the taken row's");
    assert_eq!(taken(&app, &id), 1);
    // Closed and asked again: the same object, no second copy.
    app.gis.form_entry = None;
    form_edit(&mut app);
    let again = app.gis.form_entry.as_ref().expect("the form opens again");
    assert_eq!(again.uid_text(), uid.to_string());
    assert_eq!(taken(&app, &id), 1, "no second copy of the row");
}

#[test]
fn a_read_only_connections_row_is_shown_in_its_form_and_not_written() {
    let (mut app, id) = with_form("form-salt-okunur", true);
    let address = node(&app, "parseller").and_then(|n| n.data.clone()).expect("a data layer").address;
    app.gis.features.data.choose_rows(&id, [22].into());
    form_edit(&mut app);
    let e = app.gis.form_entry.as_ref().expect("the form opens for reading");
    assert!(e.read_only);
    assert_eq!(e.row, Some(("fid".to_owned(), 22)));
    assert_eq!(e.after_value("kullanim").as_deref(), Some("Park"), "the row as its source holds it");
    let title = app.entry_title(e);
    assert!(title.contains("· fid 22 · salt okunur"), "{title}");
    let why = app.entry_lock_words(e).expect("only shown");
    assert!(why.contains("salt okunur bağlantıdan geliyor: kayıt görüntülenir, değiştirilmez"), "{why}");
    assert_eq!(taken(&app, &id), 0, "nothing taken into the drawing");
    // Kaydet, keyed past its disabled button, writes nothing and says why.
    form_event(&mut app, FormEvent::Value("kullanim".into(), "Okul".into()));
    form_event(&mut app, FormEvent::Save);
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains("salt okunur bağlantıdan geliyor"), "{said}");
    assert_eq!(kullanim(&address, 22), "Park");
    assert_eq!(taken(&app, &id), 0);
    let c = app.document.as_ref().expect("a drawing").model.data_changes(&id);
    assert_eq!(c.added.len() + c.changed.len() + c.deleted.len(), 0, "nothing pending");
}

/// The 28th item's ten switches (docs/adr/9159): two files' same-named “parseller” tables and the first bound once more
/// read-only. The target changes ten times; each time the form's head tells the read-only one apart before Kaydet, and
/// each save writes the chosen target's row alone.
#[test]
fn ten_switches_between_same_named_tables_write_the_chosen_one_alone() {
    crate::data_layers::register();
    let first = parcels("on-gecis-a", 40).display().to_string();
    let second = parcels("on-gecis-b", 40).display().to_string();
    let mut app = app_with_drawing();
    let mut again = crate::data_layers::describe(&first, "parseller").expect("described");
    again.data.read_only = true;
    let ids = app
        .add_data_layers(vec![
            crate::data_layers::describe(&first, "parseller").expect("described"),
            crate::data_layers::describe(&second, "parseller").expect("described"),
            again,
        ])
        .expect("added");
    for (n, id) in ids.iter().enumerate() {
        publish_form(&mut app, id, &format!("parsel-formu-{n}"));
    }
    for id in &ids[..2] {
        let _ = app.data_edits_event(Event::Start(id.clone()));
    }
    let mut now = ["Park".to_owned(), "Park".to_owned()];
    for turn in 0..10 {
        let at = turn % 3;
        let id = ids[at].clone();
        app.gis.features.data.choose_rows(&id, [22].into());
        form_edit(&mut app);
        let e = app.gis.form_entry.as_ref().expect("the form opens");
        let title = app.entry_title(e);
        assert_eq!(title.contains("salt okunur"), at == 2, "turn {turn}: {title}");
        // The read-only binding shows the first file as it is now.
        let shown = e.after_value("kullanim");
        assert_eq!(shown.as_deref(), Some(now[if at == 1 { 1 } else { 0 }].as_str()), "turn {turn}");
        let value = format!("Deneme-{turn}");
        form_event(&mut app, FormEvent::Value("kullanim".into(), value.clone()));
        form_event(&mut app, FormEvent::Save);
        if at < 2 {
            assert!(app.gis.form_entry.is_none(), "turn {turn}: saved and closed");
            let task = app.data_edits_event(Event::Save(vec![id.clone()]));
            drive(&mut app, task);
            now[at] = value;
        } else {
            form_event(&mut app, FormEvent::Cancel);
            assert!(app.gis.form_entry.is_none());
        }
        assert_eq!([kullanim(&first, 22), kullanim(&second, 22)], now, "turn {turn}");
        for id in &ids {
            let c = app.document.as_ref().expect("a drawing").model.data_changes(id);
            assert_eq!(c.added.len() + c.changed.len() + c.deleted.len(), 0, "turn {turn}: nothing left pending");
        }
    }
    assert_eq!(now, ["Deneme-9".to_owned(), "Deneme-7".to_owned()]);
}

/// The picture (`.run/gui-evidence/kullanim-9149/`): the 22nd row chosen in the table, its record open in the form.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9149", "01-tablodan-form", || {
        let (mut app, id) = with_form("form-resim", false);
        let _ = app.data_edits_event(Event::Start(id.clone()));
        app.gis.features.data.choose_rows(&id, [22].into());
        form_edit(&mut app);
        app
    });
}

/// The picture (`.run/gui-evidence/kullanim-9159/`): a read-only connection's 22nd row open in its form for reading.
#[test]
#[ignore = "pictures for the owner"]
fn read_only_screens() {
    crate::files_testing::usage_shots("9159", "01-salt-okunur-form", || {
        let (mut app, id) = with_form("form-salt-okunur-resim", true);
        app.gis.features.data.choose_rows(&id, [22].into());
        form_edit(&mut app);
        app
    });
}
