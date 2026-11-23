//! The municipal report's 9th item, its chain and examples (docs/adr/9170), over the report's GeoPackage (`bakim`: B01–B06,
//! bedel 0, 100, 250, NULL, 50, 100, mahalle M1, M1, M2, M2, M9, NULL; `mahalleler`: Çınar, İstasyon, Kale). A row
//! chosen by a condition, its card, its form, the table filtered and Seçimi kaldır keep the layer and the row's key all
//! the way; 0 is a value and NULL none, the language holding an empty text as empty too; Turkish search and order find
//! and place İstasyon and Çınar.

use std::collections::BTreeSet;

use kentos_data::Driver as _;
use serde_json::json;

use crate::app::{App, Message};
use crate::data_layers_tests::{answer_pages, municipal, node};
use crate::features::data::DataEvent;
use crate::files_testing::{app_with_drawing, drive};
use crate::processing::Event as Processing;

fn processing(app: &mut App, e: Processing) {
    let task = app.update(Message::Gis(crate::gis::Event::Processing(e)));
    drive(app, task);
}

/// İfadeyle seç on `layer` with `condition`, a new selection.
fn choose(app: &mut App, layer: &str, condition: &str) -> BTreeSet<i64> {
    let task = app.update(Message::Run("processing.run.selection.byExpression"));
    drive(app, task);
    processing(app, Processing::Value("input".into(), json!({ "scope": "layer", "layerId": layer })));
    processing(app, Processing::Text("condition".into(), condition.into()));
    processing(app, Processing::Value("mode".into(), json!("new")));
    processing(app, Processing::Run);
    app.gis.features.data.selected.clone()
}

/// The report's two data layers; `extra`, a seventh work with an empty mahalle (an empty text, not NULL) and bedel 0.
fn report(tag: &str, extra: bool) -> (App, String, String) {
    crate::data_layers::register();
    let path = municipal(tag);
    let address = path.display().to_string();
    if extra {
        let source = kentos_data_gpkg::Gpkg.open(&address).expect("open");
        let mut tx = source.begin().expect("begin");
        tx.insert(
            "bakim",
            &[kentos_data::NewFeature {
                fid: None,
                geometry: Some(kentos_data::Geometry::Point(Some(kentos_data::Coord::xy(484_070.0, 4_420_000.0)))),
                values: vec![
                    ("kod".into(), kentos_data::Value::Text("B07".into())),
                    ("bedel".into(), kentos_data::Value::Real(0.0)),
                    ("mahalle".into(), kentos_data::Value::Text(String::new())),
                ],
            }],
        )
        .expect("a row");
        tx.commit().expect("written");
        kentos_data::registry().forget(&address);
    }
    let mut app = app_with_drawing();
    app.add_data_layers(vec![
        crate::data_layers::describe(&address, "bakim").expect("described"),
        crate::data_layers::describe(&address, "mahalleler").expect("described"),
    ])
    .expect("added");
    let bakim = node(&app, "bakim").expect("bakim").id.clone();
    let mahalleler = node(&app, "mahalleler").expect("mahalleler").id.clone();
    (app, bakim, mahalleler)
}

/// The table on `layer`, its first rows read.
fn table_on(app: &mut App, layer: &str) {
    app.gis.features.layer = Some(layer.to_owned());
    app.command_expanded = true;
    app.bottom_tab = crate::bottom::BottomTab::Table;
    let _ = app.data_table_event(DataEvent::Range(0, 12));
    answer_pages(app);
}

#[test]
fn choose_card_form_filter_clear_keep_the_layer_and_the_key() {
    let (mut app, bakim, _) = report("zincir", false);
    // Seç: NULL is its own.
    assert_eq!(choose(&mut app, &bakim, "bedel IS NULL"), BTreeSet::from([4]));
    assert_eq!(app.gis.features.data.selected_of_for_test(), Some(bakim.as_str()));
    // Kart: the chosen row under Bilgi: its layer, its file's table and its key; NULL an empty cell, the code kept.
    let n = node(&app, "bakim").expect("bakim").clone();
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let input = kentos_commands::LayersRows { layer: bakim.clone(), keys: Some(vec![4]), geometry: Some(true), ..Default::default() };
    let row = kentos_native_application::layers_rows::read(&n, &settings, &input, &|| false).expect("read").rows;
    let p = kentos_data::wkt::read_wkt(row[0].wkt.as_deref().expect("a shape")).expect("wkt");
    let kentos_data::Geometry::Point(Some(c)) = p else { panic!("{p:?}") };
    let (found, said) = crate::data_info::ask(&[n.clone()], &settings, kentos_interaction::Vec2::new(c.x, c.y), 0.5);
    assert!(said.is_empty(), "{said:?}");
    let asked = app.gis.data_info.asked;
    let _ = app.data_info_event(crate::data_info::Event::Answered(asked, found.map(Box::new), Vec::new(), Vec::new(), false));
    let (card, _, _) = app.data_card().expect("its card");
    assert!(card.source.as_ref().is_some_and(|(l, _)| l.ends_with("› bakim · fid 4")), "{:?}", card.source);
    let cell = |name: &str| card.rows.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone());
    assert_eq!(cell("bedel").as_deref(), Some(""), "NULL is no value");
    assert_eq!(cell("mahalle").as_deref(), Some("M2"));
    // Form: the chosen row's form, by its key.
    let body = json!({ "revision": 1, "items": [{ "type": "field", "id": "kod", "field": "kod" }] });
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let form = json!({
        "id": "bakim-formu", "name": "Bakım formu", "layer": bakim, "draft": body.clone(),
        "releases": [{ "version": 1, "digest": kentos_commands::body_digest(&typed), "body": body, "note": "İlk yayın" }]
    });
    let doc = app.document.as_mut().expect("a drawing");
    let mut s = doc.model.settings().clone();
    s.forms.push(serde_json::from_value(form).expect("a form"));
    doc.model.set_settings(s);
    let _ = app.data_edits_event(crate::data_edits::Event::Start(bakim.clone()));
    let task = app.run("data.formEdit");
    drive(&mut app, task);
    let e = app.gis.form_entry.as_ref().expect("the row's form");
    let title = app.entry_title(e);
    assert!(title.contains("bakim · fid 4"), "{title}");
    app.gis.form_entry = None;
    app.dialog = None;
    // Süz: the table filtered on the same layer, the choice kept.
    table_on(&mut app, &bakim);
    app.gis.features.filter = "mahalle = 'M1'".into();
    let _ = app.data_table_event(DataEvent::Apply);
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.total_rows(), Some(2));
    assert_eq!(app.gis.features.data.selected_of(&bakim), [4], "the choice and its layer kept");
    // Seçimi kaldır: no row chosen, the table on its layer still.
    assert!(app.available("edit.deselect"));
    let _ = app.update(Message::Run("edit.deselect"));
    assert!(app.gis.features.data.selected.is_empty());
    assert_eq!(app.gis.features.data.selected_of_for_test(), Some(bakim.as_str()));
    assert!(!app.available("edit.deselect"));
}

#[test]
fn zero_is_a_value_null_and_an_empty_text_are_empty_and_turkish_search_and_order_hold() {
    let (mut app, bakim, mahalleler) = report("ornekler", true);
    assert_eq!(choose(&mut app, &bakim, "bedel = 0"), BTreeSet::from([1, 7]), "0 is a value");
    assert_eq!(choose(&mut app, &bakim, "bedel IS NULL"), BTreeSet::from([4]), "NULL is not 0");
    // The language holds a NULL and an empty text as one empty value (docs/adr/0581 §3, 9109): either asks both.
    assert_eq!(choose(&mut app, &bakim, "mahalle IS NULL"), BTreeSet::from([6, 7]));
    assert_eq!(choose(&mut app, &bakim, "mahalle = ''"), BTreeSet::from([6, 7]));
    // Turkish order: Çınar before İstasyon before Kale (not the letters' code order).
    table_on(&mut app, &mahalleler);
    let _ = app.data_table_event(DataEvent::Sort("ad".into()));
    answer_pages(&mut app);
    let ad = |app: &App, at: usize| app.gis.features.data.row(at).and_then(|r| r.attrs.get("ad").cloned());
    assert_eq!([ad(&app, 0), ad(&app, 1), ad(&app, 2)], [Some("Çınar".into()), Some("İstasyon".into()), Some("Kale".into())]);
    // Turkish search: “istasyon” finds İstasyon.
    app.gis.features.search = "istasyon".into();
    let _ = app.data_table_event(DataEvent::Apply);
    answer_pages(&mut app);
    assert_eq!(app.gis.features.data.total_rows(), Some(1));
    assert_eq!(ad(&app, 0).as_deref(), Some("İstasyon"));
}

/// The picture (`.run/gui-evidence/kullanim-9170/`): B04 chosen in the table, Seçimi kaldır on in the ribbon.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    crate::files_testing::usage_shots("9170", "01-secimi-kaldir", || {
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let (mut app, bakim, _) = report(&format!("zincir-resim-{n}"), false);
        let _ = choose(&mut app, &bakim, "bedel IS NULL");
        app.gis.processing.dialog = None;
        table_on(&mut app, &bakim);
        app
    });
}
