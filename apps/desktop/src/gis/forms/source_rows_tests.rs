//! The municipal report's 22nd item, its last part (docs/adr/9165): a form on the neighbourhoods' data layer with a
//! related item over the works' data layer, tied by the neighbourhood's code. The works are read from their source:
//! M1 and M2 each list two, M3 none; a work taken into the drawing is listed once, as edited; the rows are shown, not
//! written.

use crate::app::App;
use crate::data_layers_tests::{municipal, node};
use crate::files_testing::{app_with_drawing, drive};

/// The report's two data layers (the neighbourhoods read-only), their relation by the code and a published form on
/// the neighbourhoods with the works listed (`view` a list or a table); the app, the two layers' ids.
fn with_form(tag: &str, view: &str) -> (App, String, String) {
    crate::data_layers::register();
    let address = municipal(tag).display().to_string();
    let mut app = app_with_drawing();
    let mut parents = crate::data_layers::describe(&address, "mahalleler").expect("described");
    parents.data.read_only = true;
    app.add_data_layers(vec![crate::data_layers::describe(&address, "bakim").expect("described"), parents])
        .expect("added");
    let parent = node(&app, "mahalleler").expect("mahalleler").id.clone();
    let child = node(&app, "bakim").expect("bakim").id.clone();
    let body = serde_json::json!({ "revision": 1, "items": [
        { "type": "field", "id": "ad", "field": "ad" },
        { "type": "related", "id": "bakimlar", "label": "Bakımlar", "layer": child, "field": "mahalle", "view": view }
    ] });
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = serde_json::json!({
        "id": "mahalle-formu", "name": "Mahalle formu", "layer": parent, "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body, "note": "İlk yayın" }]
    });
    let relation = serde_json::json!({
        "id": "mahalle-bakim", "name": "Mahalle bakımları", "parent": parent, "child": child,
        "field": "mahalle", "parentKey": "mahalle"
    });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms.push(serde_json::from_value(form).expect("a form"));
    settings.relations.push(serde_json::from_value(relation).expect("a relation"));
    doc.model.set_settings(settings);
    (app, parent, child)
}

/// The neighbourhood `key`'s form opened from the table (read-only: shown from its source), its works read: the
/// related item's ids.
fn works_of(app: &mut App, parent: &str, key: i64) -> Vec<String> {
    app.gis.form_entry = None;
    app.gis.features.data.choose_rows(parent, [key].into());
    let task = app.run("data.formEdit");
    drive(app, task);
    let e = app.gis.form_entry.as_ref().expect("the form opens");
    assert_eq!(e.row.as_ref().map(|(_, k)| *k), Some(key));
    e.related_now("bakimlar")
}

#[test]
fn a_neighbourhoods_form_lists_its_works_read_from_their_source() {
    let (mut app, parent, _) = with_form("form-kaynak-satirlari", "list");
    // M1 (key 1): B01 and B02, keys 1 and 2; M2 (key 2): B03 and B04; M3 (key 3): none.
    assert_eq!(works_of(&mut app, &parent, 1), ["kaynak:1", "kaynak:2"]);
    assert_eq!(works_of(&mut app, &parent, 2), ["kaynak:3", "kaynak:4"]);
    assert!(works_of(&mut app, &parent, 3).is_empty(), "M3 has none");
    let e = app.gis.form_entry.as_ref().expect("the form");
    assert!(e.source_rows_said.is_empty(), "{:?}", e.source_rows_said);
}

#[test]
fn a_work_taken_into_the_drawing_is_listed_once_as_edited_and_the_tables_cells_are_the_sources() {
    let (mut app, parent, child) = with_form("form-kaynak-alinmis", "table");
    // B01 taken into the drawing and its bedel changed there.
    let layer = node(&app, "bakim").expect("bakim").clone();
    let _ = app.data_edits_event(crate::data_edits::Event::Start(child.clone()));
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let rows = kentos_native_application::data_write::rows_to_take(&layer, &settings, &[1], &|| false).expect("read");
    let doc = app.document.as_mut().expect("a drawing");
    let uid = doc.model.take_data_rows(&child, rows).expect("taken")[0].expect("B01");
    let slot = doc.model.slot_of(uid).expect("its object");
    let mut e = doc.model.get(slot).cloned().expect("B01");
    e.base_mut().attrs.insert("bedel".into(), "75".into());
    assert!(doc.model.update(slot, e));
    let works = works_of(&mut app, &parent, 1);
    assert_eq!(works.len(), 2, "{works:?}");
    assert!(works.contains(&uid.to_string()), "the taken one as the drawing's object: {works:?}");
    assert!(works.contains(&"kaynak:2".to_owned()), "the other from its source: {works:?}");
    assert!(!works.contains(&"kaynak:1".to_owned()), "not twice: {works:?}");
    // The table's cells: the taken one's as edited, the other's as its source holds it.
    let e = app.gis.form_entry.as_ref().expect("the form");
    let d = app.document.as_ref().expect("a drawing");
    let bedel = |id: &str| super::entry::child_attrs_for_tests(d, e, &child, id).and_then(|a| a.get("bedel").cloned());
    assert_eq!(bedel(&uid.to_string()).as_deref(), Some("75"));
    assert_eq!(bedel("kaynak:2").as_deref(), Some("100"));
}

/// The picture (`.run/gui-evidence/kullanim-9165/`): M1's form listing its two works read from their source.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    crate::files_testing::usage_shots("9165", "01-kaynaktan-iliskili", || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (mut app, parent, _) = with_form(&format!("form-kaynak-resim-{n}"), "table");
        let _ = works_of(&mut app, &parent, 1);
        app
    });
}
