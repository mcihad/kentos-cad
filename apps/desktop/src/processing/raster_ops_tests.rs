//! Raster işlemleri and Raster istatistiği in the desktop's window
//! (docs/adr/0233), over the shared processing drawing
//! (fixtures/processing/v1/raster-ops.kcad, opened beside its rasters'
//! folder): Raster hesaplayıcı over every raster reads only the two its
//! expression names (the orthophotos' files are not there), its result on
//! the grid of the one named first, named after it and right above its
//! layer, in one step; the expression field offers the rasters' names;
//! Bölgesel istatistik fills the zones' field and gives its table; Histogram
//! gives a table and leaves the drawing.

use std::path::PathBuf;

use kentos_contracts::{DocumentSnapshotV1, Entity, RasterSample};
use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const DRAWING: &str = include_str!("../../../../fixtures/processing/v1/raster-ops.kcad");

fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    // Its rasters name files beside it: the drawing is taken to be in their folder.
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/processing/v1/raster-ops/raster-ops.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn value(app: &mut App, name: &str, v: Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

fn run(app: &mut App) -> RunStatus {
    super::surface_tests::run(app)
}

/// The id of the layer named `name`, and its place in the tree.
fn layer(app: &App, name: &str) -> (String, usize) {
    let layers = app.document.as_ref().expect("open").model.layers();
    let id = layers
        .leaves()
        .into_iter()
        .find(|l| l.name == name)
        .map(|l| l.id.clone())
        .unwrap_or_else(|| panic!("{name}"));
    let (_, at) = layers.place_of(&id).expect("placed");
    (id, at)
}

#[test]
fn the_calculator_over_every_raster_opens_only_those_it_names() {
    let dir = crate::files_testing::scratch("ops-calculator");
    let mut app = app();
    let _ = app.update(Message::Run("processing.run.raster.calculator"));
    value(&mut app, "input", json!({ "scope": "all" }));
    value(&mut app, "expression", json!("[Yıl 3] - [Yıl 1]"));
    // Left empty, the file is named after the raster named first, beside it: here asked for in a scratch folder.
    let out = dir.join("fark.tif");
    value(&mut app, "output", json!(out.to_string_lossy()));
    let s = run(&mut app);
    let RunStatus::Ok { text, undo, .. } = &s else {
        panic!("{s:?}");
    };
    assert_eq!(
        text,
        &format!("8 × 7 hücrelik raster; “{}” yazıldı.", out.display())
    );
    assert!(undo, "the drawing changed");
    let bytes = std::fs::read(&out).expect("the result");
    let reader = kentos_formats::raster::source::open_bytes(&bytes, None, 1 << 26).expect("reads");
    assert_eq!(
        (reader.info.width, reader.info.height, reader.info.sample),
        (8, 7, RasterSample::F32)
    );
    // On Hesap, right above Yıl 3 (the raster named first), not above the top raster.
    let (hesap, at) = layer(&app, "Hesap");
    let (_, yil3) = layer(&app, "Yıl 3");
    assert_eq!(at + 1, yil3, "right above Yıl 3");
    let doc = &mut app.document.as_mut().expect("open").model;
    let made: Vec<_> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Raster(r) if r.base.layer_id == hesap => Some(r.raster.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(made.len(), 1);
    assert_eq!(made[0].style.ramp.as_deref(), Some("Viridis"));
    assert_eq!(doc.undo().as_deref(), Some("Raster hesaplayıcı"));
    assert!(doc.layers().leaves().iter().all(|l| l.name != "Hesap"));
}

#[test]
fn the_expression_field_offers_every_raster_s_names_and_no_preview() {
    let mut app = app();
    let _ = app.update(Message::Run("processing.run.raster.calculator"));
    value(&mut app, "input", json!({ "scope": "all" }));
    value(&mut app, "expression", json!("[DEM] * 2"));
    let window = app.processing.dialog.as_ref().expect("the window");
    let names: Vec<&str> = window.inputs["input"]
        .fields
        .iter()
        .map(|(n, _)| n.as_str())
        .collect();
    for name in [
        "DEM",
        "Yıl 1",
        "Ortofoto",
        "Ortofoto@3",
        "Ortofoto (2)",
        "Ortofoto (2)@4",
    ] {
        assert!(names.contains(&name), "{name} in {names:?}");
    }
    assert!(
        !names.contains(&"Ortofoto@4"),
        "the later orthophoto has three bands"
    );
    // A raster's expression runs cell by cell: no line of a first object's value.
    assert!(
        !window.previews.contains_key("expression"),
        "{:?}",
        window.previews
    );
}

#[test]
fn zonal_statistics_fills_the_zones_field_and_gives_the_table() {
    let mut app = app();
    let _ = app.update(Message::Run("processing.run.raster.zonalStatistics"));
    value(
        &mut app,
        "input",
        json!({ "scope": "layer", "layerId": "bolge" }),
    );
    value(
        &mut app,
        "zones",
        json!({ "scope": "layer", "layerId": "bolgeler" }),
    );
    let s = run(&mut app);
    let RunStatus::Ok {
        text, undo, table, ..
    } = &s
    else {
        panic!("{s:?}");
    };
    assert_eq!(text, "3 bölgeye “Ortalama” yazıldı (ortalama).");
    assert!(undo);
    let table = table.as_ref().expect("the zones' table");
    assert_eq!(table.rows.len(), 4);
    assert_eq!(table.columns[..3], ["Nesne", "Sayı", "Toplam"]);
    let doc = &app.document.as_ref().expect("open").model;
    let filled = doc
        .by_layer("bolgeler")
        .filter(|e| e.base().attrs.contains_key("Ortalama"))
        .count();
    assert_eq!(filled, 3, "the zone without a cell is left as it was");
}

#[test]
fn a_histogram_gives_its_table_and_leaves_the_drawing() {
    let mut app = app();
    let _ = app.update(Message::Run("processing.run.raster.histogram"));
    value(
        &mut app,
        "input",
        json!({ "scope": "layer", "layerId": "bolge" }),
    );
    value(&mut app, "bins", json!(8));
    let s = run(&mut app);
    let RunStatus::Ok {
        text, undo, table, ..
    } = &s
    else {
        panic!("{s:?}");
    };
    assert!(text.starts_with("117 hücre, "), "{text}");
    assert!(!undo, "the drawing is as it was");
    assert_eq!(table.as_ref().expect("the table").rows.len(), 8);
}
