//! The CBS review's first item (docs/adr/9318): a field made for a data layer's form is a column of its table. Yeni alan
//! asks first (the table, the column, its kind, the rows empty in it) and adds it through `cad.layers.columns`; the table
//! keeps reading every row. Alanlar adds a new field to the table, refuses a rename, and reads the table's columns again
//! (Kaynak şemasını yenile). A field the table does not have leaves the other columns read, is named under the table and
//! keeps the form back from being published.

use kentos_data::Driver;

use crate::app::{App, Message};
use crate::data_layers_tests::{node, parcels};
use crate::files_testing::{app_with_drawing, drive};
use crate::forms::designer::Event;
use crate::gis;

fn ev(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Forms(e)));
    drive(app, task);
}

/// The parcels' table of `rows` rows bound as a data layer; the app, the file's address and the layer's id.
fn bound(name: &str, rows: usize) -> (App, String, String) {
    crate::data_layers::register();
    let address = parcels(name, rows).display().to_string();
    let mut app = app_with_drawing();
    let described = crate::data_layers::describe(&address, "parseller").expect("described");
    app.add_data_layers(vec![described]).expect("added");
    let id = node(&app, "parseller").expect("the layer").id.clone();
    (app, address, id)
}

/// The table's columns as the file says them, read apart from the app.
fn columns(address: &str) -> Vec<String> {
    kentos_data::registry().forget(address);
    let source = kentos_data_gpkg::Gpkg.open(address).expect("open");
    source.schema("parseller").expect("schema").fields.into_iter().map(|f| f.name).collect()
}

fn said(app: &App) -> Vec<String> {
    app.log.lines().map(|l| l.text.clone()).collect()
}

#[test]
fn yeni_alan_on_a_data_layer_asks_then_adds_the_column_and_every_row_stays_read() {
    let (mut app, address, layer) = bound("form-sutun", 3);
    let task = app.update(Message::Run("data.forms"));
    drive(&mut app, task);
    ev(&mut app, Event::NewForm);
    ev(&mut app, Event::NewLayer(layer.clone()));
    ev(&mut app, Event::NewName("Parsel formu".into()));
    ev(&mut app, Event::Create);
    ev(&mut app, Event::NewFieldName("saha_notu".into()));
    ev(&mut app, Event::CreateField);
    // Asked first: the table, the column, its kind, the rows; nothing written yet.
    let ask = app.gis.forms_window.as_ref().and_then(|w| w.column_ask.clone()).expect("asked");
    assert_eq!((ask.table.as_str(), ask.field.name.as_str(), ask.rows), ("parseller", "saha_notu", Some(3)));
    assert!(!columns(&address).contains(&"saha_notu".to_owned()), "nothing added before the answer");
    // Vazgeç leaves it all as it was.
    ev(&mut app, Event::CancelColumn);
    assert!(app.gis.forms_window.as_ref().is_some_and(|w| w.column_ask.is_none()));
    ev(&mut app, Event::CreateField);
    ev(&mut app, Event::ConfirmColumn);
    assert!(columns(&address).contains(&"saha_notu".to_owned()), "the table has the column");
    assert!(node(&app, "parseller").is_some_and(|n| n.fields.iter().any(|f| f.name == "saha_notu")));
    let w = app.gis.forms_window.as_ref().expect("the designer stays open");
    let body = w.body.as_ref().expect("the draft");
    assert!(body.walk().iter().any(|(_, i)| matches!(i, kentos_commands::FormItem::Field(f) if f.field == "saha_notu")));
    assert!(said(&app).iter().any(|t| t.contains("“saha_notu” sütunu “parseller” tablosuna eklendi")), "{:#?}", said(&app));
    // Every row read, the new field empty in each; nothing named missing.
    let node = node(&app, "parseller").expect("the layer").clone();
    let settings = app.document.as_ref().expect("open").model.settings().clone();
    let input = kentos_commands::LayersRows { layer: layer.clone(), ..kentos_commands::LayersRows::default() };
    let answer = kentos_native_application::layers_rows::read(&node, &settings, &input, &|| false).expect("read");
    assert_eq!(answer.rows.len(), 3);
    assert!(answer.missing.is_empty(), "{:?}", answer.missing);
    assert!(answer.rows.iter().all(|r| !r.attrs.contains_key("saha_notu")));
}

#[test]
fn alanlar_adds_a_new_field_to_the_table_refuses_a_rename_and_reads_the_table_again() {
    let (mut app, address, layer) = bound("alanlar-sutun", 2);
    let _ = app.open_layer_fields(&layer);
    let ev = |app: &mut App, e: crate::layer_fields::Event| {
        let task = app.update(Message::Gis(gis::Event::LayerFields(e)));
        drive(app, task);
    };
    // A rename of a column is refused, the table and the layer as they were.
    ev(&mut app, crate::layer_fields::Event::Type(0, crate::layer_fields::Cell::Name, "ada_no".into()));
    ev(&mut app, crate::layer_fields::Event::Save);
    assert!(app.gis.layer_fields.is_some(), "the window stays");
    assert!(columns(&address).contains(&"ada".to_owned()) && !columns(&address).contains(&"ada_no".to_owned()));
    ev(&mut app, crate::layer_fields::Event::Type(0, crate::layer_fields::Cell::Name, "ada".into()));
    // A new field: a column of the table.
    ev(&mut app, crate::layer_fields::Event::Add);
    let last = app.gis.layer_fields.as_ref().map_or(0, |w| w.row_count() - 1);
    ev(&mut app, crate::layer_fields::Event::Type(last, crate::layer_fields::Cell::Name, "denetci".into()));
    ev(&mut app, crate::layer_fields::Event::Save);
    assert!(columns(&address).contains(&"denetci".to_owned()), "the table has the column");
    assert!(node(&app, "parseller").is_some_and(|n| n.fields.iter().any(|f| f.name == "denetci")));
    // A column made elsewhere: Kaynak şemasını yenile takes it in.
    {
        let source = kentos_data_gpkg::Gpkg.open(&address).expect("open");
        let mut tx = source.begin().expect("begin");
        tx.add_columns("parseller", &[kentos_data::Field::new("baska", kentos_data::FieldKind::Text { max: None })])
            .expect("added elsewhere");
        tx.commit().expect("commit");
    }
    kentos_data::registry().forget(&address);
    let _ = app.open_layer_fields(&layer);
    ev(&mut app, crate::layer_fields::Event::Refresh);
    assert!(node(&app, "parseller").is_some_and(|n| n.fields.iter().any(|f| f.name == "baska")));
    assert_eq!(
        app.gis.layer_fields.as_ref().and_then(|w| w.said_text()).as_deref(),
        Some("Tablodan alındı: “baska”.")
    );
}

#[test]
fn a_field_the_table_lacks_leaves_the_rest_read_is_named_and_keeps_the_form_back() {
    let (mut app, _address, layer) = bound("eksik-sutun", 2);
    // The layer given a field its table does not have (an older project, a column dropped elsewhere).
    {
        let doc = app.document.as_mut().expect("open");
        let mut fields = doc.model.layers().get(&layer).expect("the layer").fields.clone();
        fields.push(kentos_commands::LayerField::new("kayip", kentos_commands::LayerFieldKind::Text));
        doc.model.set_layer_fields(&layer, fields, &[]).expect("set");
    }
    let node = node(&app, "parseller").expect("the layer").clone();
    let settings = app.document.as_ref().expect("open").model.settings().clone();
    let input = kentos_commands::LayersRows { layer: layer.clone(), ..kentos_commands::LayersRows::default() };
    let answer = kentos_native_application::layers_rows::read(&node, &settings, &input, &|| false).expect("read");
    assert_eq!(answer.rows.len(), 2, "the other columns still read");
    assert_eq!(answer.missing, ["kayip"]);
    // A form on it: the field keeps it back from being published, said with the way out.
    let body: kentos_commands::FormBody = serde_json::from_value(serde_json::json!({
        "revision": 1, "items": [{ "type": "field", "id": "kayip", "field": "kayip" }]
    }))
    .expect("a body");
    // The table reads the rest and names the field under it.
    feed_table(&mut app, &layer);
    assert_eq!(app.gis.features.data.missing(), ["kayip"]);
    let issues = app.source_issues(&layer, &body);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].code, kentos_forms::codes::FIELD_NOT_IN_SOURCE);
    assert!(issues[0].message.contains("Kaynak şemasını yenile"), "{}", issues[0].message);
    assert!(!kentos_forms::publish_blockers(&issues).is_empty());
}

/// The data table on `layer` in the bottom panel, its first page read and handed to it as the reader answers.
fn feed_table(app: &mut App, layer: &str) {
    use crate::features::data::DataEvent;
    app.gis.features.layer = Some(layer.to_owned());
    app.command_expanded = true;
    app.bottom_tab = crate::bottom::BottomTab::Table;
    let _ = app.data_table_event(DataEvent::Range(0, 12));
    let _ = app.data_table_tasks();
    let node = node(app, "parseller").expect("the layer").clone();
    let settings = app.document.as_ref().expect("open").model.settings().clone();
    for (generation, page, input) in app.gis.features.data.pending() {
        let answer = kentos_native_application::layers_rows::read(&node, &settings, &input, &|| false).map_err(|e| e.message);
        let _ = app.data_table_event(DataEvent::Answered(generation, page, answer.map(Box::new)));
    }
}

/// The pictures (`.run/gui-evidence/kullanim-9318/`): Yeni alan's question on a data layer, the table naming a field its
/// source lacks, Alanlar after Kaynak şemasını yenile.
#[test]
#[ignore = "pictures: cargo test -p kentos-desktop --lib source_columns_tests::screens -- --ignored --nocapture"]
fn screens() {
    crate::drawing_fonts::load();
    crate::files_testing::usage_shots("9318", "01-yeni-alan-sorusu", || {
        let (mut app, _, layer) = bound("resim-soru", 3);
        let task = app.update(Message::Run("data.forms"));
        drive(&mut app, task);
        ev(&mut app, Event::NewForm);
        ev(&mut app, Event::NewLayer(layer.clone()));
        ev(&mut app, Event::NewName("Parsel formu".into()));
        ev(&mut app, Event::Create);
        ev(&mut app, Event::NewFieldName("saha_notu".into()));
        ev(&mut app, Event::CreateField);
        app
    });
    crate::files_testing::usage_shots("9318", "02-kaynakta-olmayan-alan", || {
        let (mut app, _, layer) = bound("resim-eksik", 3);
        let doc = app.document.as_mut().expect("open");
        let mut fields = doc.model.layers().get(&layer).expect("the layer").fields.clone();
        fields.push(kentos_commands::LayerField::new("kayip", kentos_commands::LayerFieldKind::Text));
        doc.model.set_layer_fields(&layer, fields, &[]).expect("set");
        feed_table(&mut app, &layer);
        app
    });
    crate::files_testing::usage_shots("9318", "03-alanlar-yenile", || {
        let (mut app, address, layer) = bound("resim-yenile", 3);
        let source = kentos_data_gpkg::Gpkg.open(&address).expect("open");
        let mut tx = source.begin().expect("begin");
        tx.add_columns("parseller", &[kentos_data::Field::new("baska", kentos_data::FieldKind::Text { max: None })])
            .expect("added elsewhere");
        tx.commit().expect("commit");
        kentos_data::registry().forget(&address);
        let _ = app.open_layer_fields(&layer);
        let task = app.update(Message::Gis(gis::Event::LayerFields(crate::layer_fields::Event::Refresh)));
        drive(&mut app, task);
        app
    });
}
