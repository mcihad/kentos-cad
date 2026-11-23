//! A value never written, an empty text and a code told apart (docs/adr/9188), the report's 9th and 29th items: a data
//! layer's table draws a NULL as a muted “NULL”, an empty text empty and a code by its label; Bilgi's card says NULL,
//! shows an empty text as a dash and a code by its label; a drawing layer's table and card the same. A data layer's
//! table and card leave out the fields the caller's role does not read, and name them.

use kentos_data::{Coord, Driver as _, Field, FieldKind, Geometry, GeometryField, NewFeature, NewTable, Value};
use kentos_domain::ObjectPolicy;
use kentos_native_application::ExecutionContext;

use crate::app::{App, Message};
use crate::data_layers_tests::{answer_pages, node};
use crate::files_testing::{app_with_drawing, scratch};

/// A GeoPackage of two points: K with no note (NULL) and 0, P with an empty note and no count (NULL).
fn durumlar(tag: &str) -> std::path::PathBuf {
    let path = scratch(tag).join("durumlar.gpkg");
    kentos_geopackage::GeoPackage::create(&path).expect("a file");
    let source = kentos_data_gpkg::Gpkg.open(path.to_str().expect("utf-8")).expect("open");
    let mut tx = source.begin().expect("begin");
    tx.create_table(
        &NewTable {
            name: "durumlar".into(),
            title: None,
            description: None,
            key: "fid".into(),
            fields: vec![
                Field::new("kod", FieldKind::Text { max: Some(4) }),
                Field::new("aciklama", FieldKind::Text { max: Some(40) }),
                Field::new("sayi", FieldKind::INT32),
            ],
            geometry: Some(GeometryField { name: "geom".into(), type_name: "POINT".into(), z: false, m: false, srid: Some(5256) }),
            spatial_index: true,
            system: None,
        },
        false,
    )
    .expect("table");
    let rows = vec![
        NewFeature {
            fid: None,
            geometry: Some(Geometry::Point(Some(Coord::xy(486_500.0, 4_420_200.0)))),
            values: vec![("kod".into(), Value::Text("K".into())), ("aciklama".into(), Value::Null), ("sayi".into(), Value::Int(0))],
        },
        NewFeature {
            fid: None,
            geometry: Some(Geometry::Point(Some(Coord::xy(486_520.0, 4_420_200.0)))),
            values: vec![("kod".into(), Value::Text("P".into())), ("aciklama".into(), Value::Text(String::new())), ("sayi".into(), Value::Null)],
        },
    ];
    tx.insert("durumlar", &rows).expect("rows");
    tx.commit().expect("commit");
    path
}

/// The table bound as a data layer, its `kod` a value list (K Konut alanı, P Park alanı), shown in the Tablo tab with
/// its pages read.
fn on_table(tag: &str) -> (App, String) {
    crate::data_layers::register();
    let path = durumlar(tag);
    let mut app = app_with_drawing();
    let id = app
        .add_data_layers(vec![crate::data_layers::describe(&path.display().to_string(), "durumlar").expect("described")])
        .expect("added")
        .remove(0);
    let doc = app.document.as_mut().expect("a drawing");
    let mut fields = doc.model.layers().get(&id).expect("the layer").fields.clone();
    let kod = fields.iter_mut().find(|f| f.name == "kod").expect("kod");
    kod.values = Some(
        serde_json::from_value(serde_json::json!([{ "code": "K", "label": "Konut alanı" }, { "code": "P", "label": "Park alanı" }]))
            .expect("a value list"),
    );
    let mut input = kentos_commands::LayersEdit::new(kentos_commands::LayerEditOperation::Fields);
    input.layer = Some(id.clone());
    input.fields = Some(fields);
    let done = kentos_native_application::layers_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
    assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
    app.gis.features.layer = Some(id.clone());
    app.command_expanded = true;
    app.bottom_tab = crate::bottom::BottomTab::Table;
    let _ = app.data_table_event(crate::features::data::DataEvent::Range(0, 12));
    answer_pages(&mut app);
    (app, id)
}

/// The texts drawn on the screen exactly so.
fn drawn(app: &mut App, caption: &str) -> usize {
    let size = iced::Size::new(1440.0, 900.0);
    let _ = app.update(Message::WindowResized(size));
    let mut shot = crate::files_testing::offscreen(size);
    shot.settle(app, App::view, &mut |app: &mut App, m| {
        let _ = app.update(m);
    });
    crate::files_testing::find_texts(&mut shot, app, caption).len()
}

#[test]
fn a_never_written_value_an_empty_text_and_a_code_are_told_apart_on_a_data_layer() {
    let (mut app, id) = on_table("null-veri");
    let layer = node(&app, "durumlar").expect("the layer").clone();
    assert_eq!(app.data_shown_fields(&layer), ["kod", "aciklama", "sayi"]);
    // The table: K's note and P's count never written, P's note an empty text; the codes by their labels.
    assert_eq!(drawn(&mut app, "NULL"), 2, "two values never written");
    assert_eq!((drawn(&mut app, "Konut alanı"), drawn(&mut app, "Park alanı")), (1, 1));
    assert_eq!(drawn(&mut app, "K"), 0, "a code is shown by its label");
    // The card of P: its note an empty text (a dash), its count NULL, its code by its label.
    let card_rows = |app: &App| {
        let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
        let (found, said) = crate::data_info::ask(&[layer.clone()], &settings, kentos_interaction::Vec2::new(486_520.0, 4_420_200.0), 0.5);
        assert!(said.is_empty(), "{said:?}");
        found.expect("the row")
    };
    let found = card_rows(&app);
    let asked = app.gis.data_info.asked;
    let _ = app.data_info_event(crate::data_info::Event::Answered(asked, Some(Box::new(found)), Vec::new(), Vec::new(), false));
    let (card, _, _) = app.data_card().expect("its card");
    let cell = |name: &str| card.rows.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone());
    assert_eq!(
        (cell("kod").as_deref(), cell("aciklama").as_deref(), cell("sayi").as_deref()),
        (Some("Park alanı"), Some(""), Some(crate::features::texts::NULL))
    );
    let _ = id;
}

#[test]
fn a_drawing_layers_table_and_card_say_a_never_written_value() {
    let mut app = app_with_drawing();
    // The sample's Parsel polygon with a declared field never written.
    let doc = app.document.as_mut().expect("a drawing");
    let mut fields = doc.model.layers().get("parsel").expect("Parsel").fields.clone();
    fields.push(kentos_commands::LayerField::new("Malik", kentos_commands::LayerFieldKind::Text));
    let mut input = kentos_commands::LayersEdit::new(kentos_commands::LayerEditOperation::Fields);
    input.layer = Some("parsel".into());
    input.fields = Some(fields);
    let done = kentos_native_application::layers_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
    assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
    let slot = doc.model.by_layer("parsel").next().map(|e| kentos_domain::Slot(e.base().id)).expect("the parcel");
    app.gis.features.reset(Some("parsel".into()));
    app.command_expanded = true;
    app.bottom_tab = crate::bottom::BottomTab::Table;
    assert!(drawn(&mut app, "NULL") >= 1, "the parcel's Malik never written");
    let card = app.slot_card(slot).expect("its card");
    assert!(card.rows.iter().any(|(k, v)| k == "Malik" && v == crate::features::texts::NULL), "{:?}", card.rows);
}

#[test]
fn a_data_layers_table_and_card_leave_out_what_the_role_does_not_read() {
    let (mut app, id) = on_table("null-rol");
    // A published form showing kod alone; one who works through forms reads nothing else.
    let body = serde_json::json!({ "revision": 1, "items": [{ "type": "field", "id": "kod", "field": "kod" }] });
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = serde_json::json!({
        "id": "durum-formu", "name": "Durum formu", "layer": id, "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body, "note": "İlk yayın" }]
    });
    let doc = app.document.as_mut().expect("a drawing");
    let mut settings = doc.model.settings().clone();
    settings.forms.push(serde_json::from_value(form).expect("a form"));
    doc.model.set_settings(settings);
    let layer = node(&app, "durumlar").expect("the layer").clone();
    assert_eq!(app.data_shown_fields(&layer), ["kod", "aciklama", "sayi"], "the owner reads every field");
    app.document.as_mut().expect("a drawing").model.set_object_policy(ObjectPolicy::AttributesOnly);
    assert_eq!(app.data_shown_fields(&layer), ["kod"], "a Veri girişi reads the form's");
    assert_eq!(drawn(&mut app, "Rolünüz bu alanları okumuyor, gösterilmiyor: aciklama, sayi"), 1, "the foot names them");
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let (found, _) = crate::data_info::ask(&[layer.clone()], &settings, kentos_interaction::Vec2::new(486_500.0, 4_420_200.0), 0.5);
    let asked = app.gis.data_info.asked;
    let _ = app.data_info_event(crate::data_info::Event::Answered(asked, found.map(Box::new), Vec::new(), Vec::new(), false));
    let (card, _, _) = app.data_card().expect("its card");
    assert!(!card.rows.iter().any(|(k, _)| k == "aciklama" || k == "sayi"), "{:?}", card.rows);
    assert!(card.rows.iter().any(|(k, v)| k == "Rolünüz okumuyor" && v == "aciklama, sayi"), "{:?}", card.rows);
}

/// Pictures for the owner (`.run/gui-evidence/kullanim-9188/`): the data layer's table with its NULLs, its empty text
/// and its codes by their labels.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    crate::files_testing::usage_shots("9188", "1-null-bos-kod", || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        on_table(&format!("null-resim-{n}")).0
    });
}
