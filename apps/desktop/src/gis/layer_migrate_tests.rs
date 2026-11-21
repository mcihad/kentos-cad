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
