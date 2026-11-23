//! The municipal report's 27th item over a GeoPackage table (docs/adr/9149): with a published form on the parcels'
//! data layer, the 22nd row chosen in the attribute table reaches Formla düzenle. Not being edited, it says so with
//! Düzenlemeyi başlat; being edited, the row is read, taken and its record opened; asked again, the same taken object
//! opens, no second copy. A read-only connection's row opens in its form for reading, its head saying so, nothing
//! taken or written; ten switches between same-named tables write the chosen one alone (docs/adr/9159). On a real
//! PostgreSQL (`KENTOS_DATA_PG`) the same, the read-only binding an account that may only read (its real right).

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

/// `table` of the database at `address` bound as its account's rights say, a published form on it: the app and the
/// layer's id.
fn with_form_on(address: &str, table: &str) -> (App, String) {
    crate::data_layers::register();
    let mut app = app_with_drawing();
    let described = crate::data_layers::describe(address, table).expect("described");
    let name = described.name.clone();
    app.add_data_layers(vec![described]).expect("added");
    let id = node(&app, &name).expect("the layer").id.clone();
    publish_form(&mut app, &id, "parsel-formu");
    (app, id)
}

/// The kullanim of row `fid` in the file, read apart from the app.
fn kullanim(address: &str, fid: i64) -> String {
    kullanim_on(address, "parseller", fid)
}

/// The same in `table`.
fn kullanim_on(address: &str, table: &str, fid: i64) -> String {
    let source = kentos_data::registry().source(address).expect("the source");
    let schema = source.schema(table).expect("its schema");
    let mut tx = source.begin().expect("a transaction");
    let rows = tx.rows(table, &[fid]).expect("read");
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

/// On a real PostgreSQL (the usage report's 27th item): the 22nd row chosen in the table of a database's layer reaches
/// Formla düzenle. Not being edited it says so; being edited the row is read from the server, taken and its record
/// opened, the same object again when asked twice. Through an account that may only read, the binding is read-only by
/// the server's own right: the row opens for reading, its head says so, Kaydet writes nothing.
#[test]
fn on_postgresql_a_row_chosen_in_the_table_opens_its_record_through_the_form() {
    let Some(pg) = crate::pg_testing::pg("PostgreSQL'de formla düzenle") else {
        return;
    };
    crate::data_layers::register();
    let t = crate::pg_testing::table("form");
    crate::data_layers_tests::parcels_in(kentos_data::registry().source(&pg).expect("the database").as_ref(), &t, 40);
    let (mut app, id) = with_form_on(&pg, &t);
    app.gis.features.data.choose_rows(&id, [22].into());
    form_edit(&mut app);
    assert!(app.gis.form_entry.is_none());
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains("düzenlemeye açık değil; 22 anahtarlı kaydı formla düzenlemek için"), "{said}");
    let _ = app.data_edits_event(Event::Start(id.clone()));
    form_edit(&mut app);
    let entry = app.gis.form_entry.as_ref().expect("the form opens");
    let uid = app.document.as_ref().expect("a drawing").model.taken_uid(&id, 22).expect("the row taken");
    assert_eq!(entry.uid_text(), uid.to_string(), "the record is the taken row's");
    app.gis.form_entry = None;
    form_edit(&mut app);
    assert_eq!(app.gis.form_entry.as_ref().expect("the form opens again").uid_text(), uid.to_string());
    assert_eq!(taken(&app, &id), 1, "no second copy of the row");
    // An account that may only read: read-only by the server's right.
    let name = format!("kentos_okur_{}", std::process::id());
    let reader = crate::pg_testing::account(&pg, &name, &t, "SELECT");
    let (mut app, id) = with_form_on(&reader, &t);
    assert!(app.document.as_ref().and_then(|d| d.model.layers().get(&id)).and_then(|n| n.data.as_ref()).is_some_and(|d| d.read_only));
    app.gis.features.data.choose_rows(&id, [22].into());
    form_edit(&mut app);
    let e = app.gis.form_entry.as_ref().expect("the form opens for reading");
    assert!(e.read_only);
    assert_eq!(e.after_value("kullanim").as_deref(), Some("Park"));
    assert!(app.entry_title(e).contains("· fid 22 · salt okunur"), "{}", app.entry_title(e));
    form_event(&mut app, FormEvent::Value("kullanim".into(), "Okul".into()));
    form_event(&mut app, FormEvent::Save);
    assert!(crate::files_testing::last_said(&app).contains("salt okunur bağlantıdan geliyor"));
    assert_eq!(kullanim_on(&pg, &t, 22), "Park", "nothing written");
    assert_eq!(taken(&app, &id), 0);
    crate::pg_testing::drop_table(&pg, &t);
    crate::pg_testing::sql(&pg, &[&format!("DROP OWNED BY {name}"), &format!("DROP ROLE {name}")]);
}

/// On a real PostgreSQL (the usage report's 28th item): two schemas' same-named “parseller” tables, and the first bound
/// once more through an account that may only read. Ten switches of the target: each time the form's head tells the
/// read-only one apart, and each save writes the chosen table's row alone.
#[test]
fn on_postgresql_ten_switches_between_same_named_tables_write_the_chosen_one_alone() {
    let Some(pg) = crate::pg_testing::pg("PostgreSQL'de on geçiş") else {
        return;
    };
    crate::data_layers::register();
    let id = std::process::id();
    let (sa, sb) = (format!("kentos_gecis_a_{id}"), format!("kentos_gecis_b_{id}"));
    crate::pg_testing::sql(&pg, &[&format!("CREATE SCHEMA {sa}"), &format!("CREATE SCHEMA {sb}")]);
    let (ta, tb) = (format!("{sa}.parseller"), format!("{sb}.parseller"));
    let owner = kentos_data::registry().source(&pg).expect("the database");
    crate::data_layers_tests::parcels_in(owner.as_ref(), &ta, 40);
    crate::data_layers_tests::parcels_in(owner.as_ref(), &tb, 40);
    let name = format!("kentos_gecis_okur_{id}");
    let reader = crate::pg_testing::account(&pg, &name, &ta, "SELECT");
    let mut app = app_with_drawing();
    let ids = app
        .add_data_layers(vec![
            crate::data_layers::describe(&pg, &ta).expect("described"),
            crate::data_layers::describe(&pg, &tb).expect("described"),
            crate::data_layers::describe(&reader, &ta).expect("described"),
        ])
        .expect("added");
    let names: Vec<String> = ids
        .iter()
        .map(|i| app.document.as_ref().and_then(|d| d.model.layers().get(i)).map(|n| n.name.clone()).unwrap_or_default())
        .collect();
    assert!(names.iter().all(|n| n == &names[0]), "same-named: {names:?}");
    for (n, i) in ids.iter().enumerate() {
        publish_form(&mut app, i, &format!("parsel-formu-{n}"));
    }
    for i in &ids[..2] {
        let _ = app.data_edits_event(Event::Start(i.clone()));
    }
    let mut now = ["Park".to_owned(), "Park".to_owned()];
    for turn in 0..10 {
        let at = turn % 3;
        let layer = ids[at].clone();
        app.gis.features.data.choose_rows(&layer, [22].into());
        form_edit(&mut app);
        let e = app.gis.form_entry.as_ref().expect("the form opens");
        let title = app.entry_title(e);
        assert_eq!(title.contains("salt okunur"), at == 2, "turn {turn}: {title}");
        let shown = e.after_value("kullanim");
        assert_eq!(shown.as_deref(), Some(now[if at == 1 { 1 } else { 0 }].as_str()), "turn {turn}");
        let value = format!("Deneme-{turn}");
        form_event(&mut app, FormEvent::Value("kullanim".into(), value.clone()));
        form_event(&mut app, FormEvent::Save);
        if at < 2 {
            assert!(app.gis.form_entry.is_none(), "turn {turn}: saved and closed");
            let task = app.data_edits_event(Event::Save(vec![layer.clone()]));
            drive(&mut app, task);
            now[at] = value;
        } else {
            form_event(&mut app, FormEvent::Cancel);
            assert!(app.gis.form_entry.is_none());
        }
        assert_eq!([kullanim_on(&pg, &ta, 22), kullanim_on(&pg, &tb, 22)], now, "turn {turn}");
    }
    assert_eq!(now, ["Deneme-9".to_owned(), "Deneme-7".to_owned()]);
    crate::pg_testing::sql(
        &pg,
        &[
            &format!("DROP OWNED BY {name}"),
            &format!("DROP ROLE {name}"),
            &format!("DROP SCHEMA {sa} CASCADE"),
            &format!("DROP SCHEMA {sb} CASCADE"),
        ],
    );
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
