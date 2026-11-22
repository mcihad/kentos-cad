//! Katmanlar › Veri kaynağına taşı… (docs/adr/9107 §8): a CBS project's drawing layer of points moved into a
//! GeoPackage in one step — its objects the table's rows, a data layer in its place with its name, colour and active
//! role — and taken back whole by one undo, the table staying in the file.

use serde_json::json;

use crate::app::{App, Message};
use crate::layer_migrate::Event;
use crate::tools_scenes::{Objects, layer, open_edited};

#[test]
fn a_drawing_layer_moves_into_a_geopackage_in_one_undo_step() {
    crate::data_layers::register();
    let (mut app, _) = App::boot(None);
    let mut o = Objects::new();
    for i in 0..3 {
        o.point("direk", [10.0 * f64::from(i), 5.0 * f64::from(i)], None);
    }
    open_edited(&mut app, o, |d| {
        d["settings"]["workspace"] = json!("gis");
        let mut poles = layer("direk", "Direkler", "#C0392B", 0.35);
        poles["geometry"] = json!("point");
        d["layers"] = json!([layer("0", "0", "fg", 0.25), poles]);
        d["activeLayer"] = json!("direk");
    });
    let node = app.document.as_ref().and_then(|d| d.model.layers().get("direk")).cloned().expect("the layer");
    assert!(app.migrates(&node), "a CBS drawing layer holding objects moves");
    let gpkg = crate::files_testing::scratch("kaynaga-tasi").join("sivas.gpkg");
    let send = |app: &mut App, e: Event| drop(app.update(Message::Gis(crate::gis::Event::LayerMigrate(e))));
    send(&mut app, Event::Open("direk".into()));
    send(&mut app, Event::Path(gpkg.display().to_string()));
    send(&mut app, Event::Run);
    assert!(app.gis.layer_migrate.is_none(), "{}", crate::files_testing::last_said(&app));

    let doc = &app.document.as_ref().expect("a drawing").model;
    assert!(doc.layers().get("direk").is_none(), "the drawing layer went");
    let moved = doc.layers().leaves().into_iter().find(|n| n.name == "Direkler").cloned().expect("in its place");
    let data = moved.data.clone().expect("a data layer");
    assert_eq!((data.address.as_str(), data.table.as_str()), (gpkg.display().to_string().as_str(), "direkler"));
    assert_eq!(moved.style.color, node.style.color, "its colour kept");
    assert_eq!(doc.layers().active(), moved.id, "active, as the layer was");
    assert_eq!(doc.len(), 0, "no object left in the project file");
    let settings = doc.settings().clone();
    let rows = kentos_native_application::layers_rows::read(
        &moved,
        &settings,
        &kentos_commands::LayersRows { layer: moved.id.clone(), count: Some(true), limit: Some(0), ..Default::default() },
        &|| false,
    )
    .expect("read from the table");
    assert_eq!(rows.total, Some(3));

    // One undo: the drawing layer and its objects back; the table stays in the file.
    let _ = app.update(Message::Run("edit.undo"));
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert!(doc.layers().get("direk").is_some_and(|n| n.data.is_none()), "the drawing layer back");
    assert_eq!(doc.layers().active(), "direk", "active again, as before");
    assert_eq!(doc.len(), 3, "its objects back");
    assert!(doc.layers().leaves().iter().all(|n| n.data.is_none()), "the data layer gone");
    let tables = kentos_data::registry().source(&gpkg.display().to_string()).expect("source").tables().expect("tables");
    assert!(tables.iter().any(|t| t.name == "direkler"), "the table stays");
    let _ = std::fs::remove_dir_all(gpkg.parent().expect("a folder"));
}

/// A drawn text's words, height, angle, alignment and width factor.
type Drawn = (String, f64, f64, Option<kentos_commands::TextAlign>, Option<f64>);

/// A layer of points and texts (docs/adr/9114 §2–§3): the points go into its table, the texts into a table of their own
/// beside it, read by a data layer above it named after it; that layer's rows are drawn as the texts they were, their
/// words, height, angle and alignment from their columns.
#[test]
fn a_layers_texts_move_into_a_table_of_their_own_and_are_drawn_as_texts() {
    crate::data_layers::register();
    let (mut app, _) = App::boot(None);
    let mut o = Objects::new();
    for i in 0..2 {
        o.point("direk", [10.0 * f64::from(i), 220.0], None);
    }
    o.push("direk", json!({ "kind": "text", "p": crate::tools_scenes::xy([2.0, 225.0]), "text": "D-1", "height": 1.5,
        "rotation": 30.0, "align": "middleCenter" }));
    o.push("direk", json!({ "kind": "text", "p": crate::tools_scenes::xy([12.0, 225.0]), "text": "D-2", "height": 1.5,
        "rotation": 0.0, "widthFactor": 0.8 }));
    open_edited(&mut app, o, |d| {
        d["settings"]["workspace"] = json!("gis");
        let mut poles = layer("direk", "Direkler", "#C0392B", 0.35);
        poles["geometry"] = json!("point");
        d["layers"] = json!([layer("0", "0", "fg", 0.25), poles]);
        d["activeLayer"] = json!("direk");
    });
    let gpkg = crate::files_testing::scratch("kaynaga-tasi-yazi").join("sivas.gpkg");
    let send = |app: &mut App, e: Event| drop(app.update(Message::Gis(crate::gis::Event::LayerMigrate(e))));
    send(&mut app, Event::Open("direk".into()));
    send(&mut app, Event::Path(gpkg.display().to_string()));
    send(&mut app, Event::Run);
    assert!(app.gis.layer_migrate.is_none(), "{}", crate::files_testing::last_said(&app));

    let doc = &app.document.as_ref().expect("a drawing").model;
    let leaves = doc.layers().leaves();
    let at = |name: &str| leaves.iter().position(|n| n.name == name).expect(name);
    let texts = leaves[at("Direkler (yazılar)")].clone();
    assert!(at("Direkler (yazılar)") < at("Direkler"), "the texts above the points");
    let data = texts.data.clone().expect("a data layer");
    assert_eq!(data.table, "direkler_yazilar");
    assert_eq!(data.text, Some(kentos_commands::DataText::written()));
    assert_eq!(texts.style.color, "#C0392B", "in the layer's colour");
    assert_eq!(doc.len(), 0, "no object left in the project file");

    // Drawn: its rows the texts they were, kept for the drawing's text layer.
    let (_, _, drawn) = crate::data_layers_tests::read_whole_as(&crate::data_view::DataView::new(), &app, &texts, false)
        .expect("a layer of texts keeps its read");
    let mut words: Vec<Drawn> = drawn
        .entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Text(t) => Some((t.text.clone(), t.height, t.rotation, t.align, t.width_factor)),
            _ => None,
        })
        .collect();
    words.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(
        words,
        [
            ("D-1".to_owned(), 1.5, 30.0, Some(kentos_commands::TextAlign::MiddleCenter), None),
            ("D-2".to_owned(), 1.5, 0.0, None, Some(0.8)),
        ]
    );
    let _ = std::fs::remove_dir_all(gpkg.parent().expect("a folder"));
}

/// A layer of texts edited (docs/adr/9114 §4): its row taken into the drawing as the text it says, its words changed,
/// turned and moved, a new text written on it; the save writes them back as rows, the texts' parts in their columns.
#[test]
fn a_layer_of_texts_is_edited_as_texts_and_saved_as_rows() {
    use kentos_native_application::data_write;
    crate::data_layers::register();
    let (mut app, _) = App::boot(None);
    let mut o = Objects::new();
    o.push("yazi", json!({ "kind": "text", "p": crate::tools_scenes::xy([2.0, 225.0]), "text": "Ada 101", "height": 1.5,
        "rotation": 0.0 }));
    open_edited(&mut app, o, |d| {
        d["settings"]["workspace"] = json!("gis");
        d["layers"] = json!([layer("0", "0", "fg", 0.25), layer("yazi", "Yazılar", "#2E86C1", 0.25)]);
        d["activeLayer"] = json!("yazi");
    });
    let gpkg = crate::files_testing::scratch("yazi-duzenle").join("yazi.gpkg");
    let send = |app: &mut App, e: Event| drop(app.update(Message::Gis(crate::gis::Event::LayerMigrate(e))));
    send(&mut app, Event::Open("yazi".into()));
    send(&mut app, Event::Path(gpkg.display().to_string()));
    send(&mut app, Event::Run);
    let texts = app
        .document
        .as_ref()
        .and_then(|d| d.model.layers().leaves().into_iter().find(|n| n.name == "Yazılar").cloned())
        .expect("the texts' layer in its place");
    let id = texts.id.clone();
    assert_eq!(texts.data.as_ref().map(|d| d.table.as_str()), Some("yazilar"), "a layer of texts only: its own table");

    // Düzenlemeyi başlat; the row taken as the text it says.
    let _ = app.data_edits_event(crate::data_edits::Event::Start(id.clone()));
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let taken = data_write::rows_to_take(&texts, &settings, &[1], &|| false).expect("the row");
    assert!(matches!(&taken[0].1, kentos_commands::Entity::Text(t) if t.text == "Ada 101" && t.height == 1.5));
    let asked = app.gis.data_pick.asked;
    let _ = app.data_pick_event(crate::data_pick::Event::Answered(
        asked,
        vec![crate::data_pick::Got::Rows(id.clone(), taken)],
        Vec::new(),
    ));
    let slot = *app.selection.ids().first().expect("the taken text is selected");
    let doc = app.document.as_mut().expect("a drawing");
    let kentos_commands::Entity::Text(mut t) = doc.model.get(slot).cloned().expect("its object") else {
        panic!("taken as a text");
    };
    t.text = "Ada 102".into();
    t.rotation = 45.0;
    t.p.x += 2.0;
    t.align = Some(kentos_commands::TextAlign::TopRight);
    assert!(doc.model.update(slot, kentos_commands::Entity::Text(t)));
    // A new text written on the layer.
    let p = crate::tools_scenes::xy([20.0, 230.0]);
    let input = serde_json::from_value(json!({
        "layerId": id,
        "objects": [{ "geometry": { "kind": "text", "p": p, "text": "Yeni", "height": 2.0, "rotation": 90.0 } }]
    }))
    .expect("an input");
    let created = kentos_native_application::create::execute(&mut kentos_native_application::ExecutionContext::new(&mut doc.model), input);
    assert!(matches!(created, kentos_commands::CommandResult::Completed { .. }), "{created:?}");
    let c = doc.model.data_changes(&id);
    assert_eq!((c.added.len(), c.changed.len()), (1, 1));

    // Kaydet: both written as rows; read back as the texts they are now.
    let model = &app.document.as_ref().expect("a drawing").model;
    let writes = data_write::writes_of(model, &id).expect("being edited");
    data_write::write_group(std::slice::from_ref(&writes), model.settings(), false).expect("written");
    let rows = data_write::rows_to_take(&texts, &settings, &[1, 2], &|| false).expect("read back");
    let read: Vec<(String, f64, f64, Option<kentos_commands::TextAlign>, f64)> = rows
        .iter()
        .map(|(_, e)| match e {
            kentos_commands::Entity::Text(t) => (t.text.clone(), t.rotation, t.height, t.align, t.p.x),
            other => panic!("a text: {other:?}"),
        })
        .collect();
    let e = crate::tools_scenes::E;
    assert_eq!(
        read,
        [
            ("Ada 102".to_owned(), 45.0, 1.5, Some(kentos_commands::TextAlign::TopRight), e + 4.0),
            ("Yeni".to_owned(), 90.0, 2.0, None, e + 20.0),
        ]
    );
    // The row's columns as another program reads them.
    let attrs = &rows[0].1.base().attrs;
    assert_eq!((attrs.get("metin").map(String::as_str), attrs.get("hiza").map(String::as_str)), (Some("Ada 102"), Some("sağ üst")));
    let _ = std::fs::remove_dir_all(gpkg.parent().expect("a folder"));
}
