//! The municipal report's 27th item over a GeoPackage table (docs/adr/9149): with a published form on the parcels'
//! data layer, the 22nd row chosen in the attribute table reaches Formla düzenle. Not being edited, it says so with
//! Düzenlemeyi başlat; being edited, the row is read, taken and its record opened; asked again, the same taken object
//! opens, no second copy; a read-only layer says its records are not written.

use crate::app::App;
use crate::data_edits::Event;
use crate::data_layers_tests::{node, parcels};
use crate::files_testing::{app_with_drawing, drive};

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
    let body = serde_json::json!({ "revision": 1, "items": [{ "type": "field", "id": "kullanim", "field": "kullanim" }] });
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = serde_json::json!({
        "id": "parsel-formu", "name": "Parsel formu", "layer": id, "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body, "note": "İlk yayın" }]
    });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms.push(serde_json::from_value(form).expect("a form"));
    doc.model.set_settings(settings);
    (app, id)
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
fn a_read_only_layers_row_is_said_not_written() {
    let (mut app, id) = with_form("form-salt-okunur", true);
    app.gis.features.data.choose_rows(&id, [22].into());
    form_edit(&mut app);
    assert!(app.gis.form_entry.is_none());
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains("salt okunur bağlantıdan geliyor: kayıtları formla yazılmaz"), "{said}");
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
