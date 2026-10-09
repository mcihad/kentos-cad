//! İnterpolasyon and Yoğunluk in the desktop's window (docs/adr/0232), over
//! the shared processing drawing (fixtures/processing/v1/interpolation.kcad:
//! survey points with their elevations, events, roads): each tool run from
//! its window as the user runs it, in the background through the desktop's
//! files, its GeoTIFF written where asked (a scratch folder) and drawn on a
//! layer of its own right below its points' in one step; the
//! cross-validation's table; Kriging's error on a second layer from the same
//! file; a density's zeros clear.

use std::path::PathBuf;

use kentos_contracts::{DocumentSnapshotV1, Entity, RasterSample};
use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const DRAWING: &str = include_str!("../../../../fixtures/processing/v1/interpolation.kcad");

fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/processing/v1/interpolation.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn value(app: &mut App, name: &str, v: Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

/// Opens `tool`'s window on `layer`, its result to be written at `out`.
fn open(app: &mut App, tool: &'static str, layer: &str, out: &std::path::Path) {
    let _ = app.update(Message::Run(tool));
    value(app, "input", json!({ "scope": "layer", "layerId": layer }));
    value(app, "output", json!(out.to_string_lossy()));
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

fn rasters_on(app: &App, layer: &str) -> Vec<kentos_contracts::RasterFields> {
    app.document
        .as_ref()
        .expect("open")
        .model
        .entities()
        .filter_map(|e| match e {
            Entity::Raster(r) if r.base.layer_id == layer => Some(r.raster.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn idw_is_written_where_asked_below_its_points_with_its_table() {
    let dir = crate::files_testing::scratch("interp-idw");
    let mut app = app();
    open(
        &mut app,
        "processing.run.interpolation.idw",
        "noktalar",
        &dir.join("yuzey.tiff"),
    );
    value(&mut app, "cellSize", json!(5));
    value(&mut app, "cross", json!(true));
    let written = dir.join("yuzey.tif");
    let s = run(&mut app);
    let RunStatus::Ok {
        text, undo, table, ..
    } = &s
    else {
        panic!("{s:?}");
    };
    let start = format!(
        "40 noktadan 23 × 16 hücrelik raster; “{}” yazıldı. Çapraz doğrulama: 40 noktada ",
        written.display()
    );
    assert!(text.starts_with(&start), "{text}");
    assert!(undo, "the drawing changed");
    let table = table.as_ref().expect("the cross-validation's table");
    assert_eq!(table.rows.len(), 40);
    assert_eq!(table.columns[..4], ["Sıra", "Ad", "Y", "X"]);
    assert_eq!(table.rows[0][1], "N1");
    // The file is whole, the grid's size, 32-bit.
    let bytes = std::fs::read(&written).expect("the result");
    assert!(!dir.join("yuzey.tif.yaziliyor").exists());
    let reader = kentos_formats::raster::source::open_bytes(&bytes, None, 1 << 26).expect("reads");
    assert_eq!(
        (reader.info.width, reader.info.height, reader.info.sample),
        (23, 16, RasterSample::F32)
    );
    // Its object on IDW, right below Noktalar, drawn in Arazi.
    let (idw, at) = layer(&app, "IDW");
    let (_, points) = layer(&app, "Noktalar");
    assert_eq!(at, points + 1, "right below the points");
    let r = rasters_on(&app, &idw);
    assert_eq!(r.len(), 1);
    assert_eq!(
        r[0].file.as_deref(),
        Some(written.to_string_lossy().as_ref())
    );
    assert_eq!(r[0].style.ramp.as_deref(), Some("Arazi"));
    let doc = &mut app.document.as_mut().expect("open").model;
    assert_eq!(doc.undo().as_deref(), Some("Ters uzaklık (IDW)"));
    assert!(doc.layers().leaves().iter().all(|l| l.name != "IDW"));
}

#[test]
fn kriging_s_error_is_a_second_band_on_a_layer_below_its_prediction() {
    let dir = crate::files_testing::scratch("interp-kriging");
    let mut app = app();
    open(
        &mut app,
        "processing.run.interpolation.kriging",
        "noktalar",
        &dir.join("kriging.tif"),
    );
    value(&mut app, "cellSize", json!(5));
    value(&mut app, "variogram", json!("manual"));
    value(&mut app, "nugget", json!(0.5));
    value(&mut app, "sill", json!(120));
    value(&mut app, "range", json!(90));
    value(&mut app, "errorSurface", json!(true));
    let s = run(&mut app);
    let RunStatus::Ok { text, .. } = &s else {
        panic!("{s:?}");
    };
    assert!(
        text.contains("Variogram: Küresel, külçe 0.500, kısmi eşik 120.000, erim 90.000 m."),
        "{text}"
    );
    let (_, points) = layer(&app, "Noktalar");
    let (prediction, p) = layer(&app, "Kriging");
    let (error, e) = layer(&app, "Kriging standart hatası");
    assert_eq!(
        (p, e),
        (points + 1, points + 2),
        "the prediction under its points, the error under it"
    );
    let (a, b) = (rasters_on(&app, &prediction), rasters_on(&app, &error));
    assert_eq!((a.len(), b.len()), (1, 1));
    assert_eq!(a[0].file, b[0].file, "one file");
    assert_eq!(
        (
            a[0].bands,
            a[0].style.bands.clone(),
            b[0].style.bands.clone()
        ),
        (2, vec![1], vec![2])
    );
    assert_eq!(b[0].style.ramp.as_deref(), Some("Viridis"));
}

#[test]
fn a_kernel_density_draws_its_zeros_clear() {
    let dir = crate::files_testing::scratch("interp-kernel");
    let mut app = app();
    open(
        &mut app,
        "processing.run.density.kernel",
        "olaylar",
        &dir.join("yogunluk.tif"),
    );
    value(&mut app, "radius", json!(30));
    value(&mut app, "cellSize", json!(10));
    let s = run(&mut app);
    let RunStatus::Ok { text, .. } = &s else {
        panic!("{s:?}");
    };
    assert!(
        text.starts_with("60 noktanın yoğunluğu, yarıçap 30.000 m; 26 × 26 hücre;"),
        "{text}"
    );
    let (id, _) = layer(&app, "Yoğunluk");
    let r = rasters_on(&app, &id);
    assert_eq!(
        (r[0].style.ramp.as_deref(), r[0].style.nodata),
        (Some("Sıcaklık"), Some(0.0))
    );
}
