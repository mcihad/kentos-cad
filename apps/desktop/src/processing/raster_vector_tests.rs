//! Raster ve vektör and Taranmış harita in the desktop's window
//! (docs/adr/0234): Rasterleştir over the valley's parcels writes its file
//! and puts the raster right below the parcels' layer, in one step; Çizgi
//! yakala on the scanned sheet (fixtures/interaction/v1/scanned.kcad)
//! captures the clicked contour as one closed polyline at the elevation
//! given; Eğrilere kot ver gives the captured curves their elevations, its
//! step undone whole.

use std::path::PathBuf;

use kentos_contracts::{DocumentSnapshotV1, Entity, RasterSample};
use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const SCANNED: &str = include_str!("../../../../fixtures/interaction/v1/scanned.kcad");

fn value(app: &mut App, name: &str, v: Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

fn run(app: &mut App) -> RunStatus {
    super::surface_tests::run(app)
}

fn scanned() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(SCANNED).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interaction/v1/scanned.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn layer_named(app: &App, name: &str) -> (String, usize) {
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
fn the_parcels_burn_below_their_layer_in_one_step() {
    let dir = crate::files_testing::scratch("vek-rasterlestir");
    let (mut app, _) = App::boot(None);
    super::surface_tests::open_valley(&mut app);
    let _ = app.update(Message::Run("processing.run.raster.rasterize"));
    value(
        &mut app,
        "input",
        json!({ "scope": "layer", "layerId": "parsel" }),
    );
    value(&mut app, "valueFrom", json!("field"));
    value(&mut app, "field", json!("Parsel"));
    value(&mut app, "cellSize", json!(2));
    value(&mut app, "sample", json!("i32"));
    let out = dir.join("parseller.tif");
    value(&mut app, "output", json!(out.to_string_lossy()));
    let s = run(&mut app);
    let RunStatus::Ok { text, undo, .. } = &s else {
        panic!("{s:?}");
    };
    assert!(text.starts_with("5 nesneden "), "{text}");
    assert!(undo);
    let bytes = std::fs::read(&out).expect("the result");
    let reader = kentos_formats::raster::source::open_bytes(&bytes, None, 1 << 26).expect("reads");
    assert_eq!(reader.info.sample, RasterSample::I32);
    // Right below Parsel: the parcels drawn over their cells.
    let (made, at) = layer_named(&app, "Rasterleştirilmiş");
    let (_, parsel) = layer_named(&app, "Parsel");
    assert_eq!(at, parsel + 1);
    let doc = &mut app.document.as_mut().expect("open").model;
    assert!(doc.entities().any(|e| matches!(e, Entity::Raster(r) if r.base.layer_id == made && r.raster.sample == RasterSample::I32)));
    assert_eq!(doc.undo().as_deref(), Some("Rasterleştir"));
    assert!(
        doc.layers()
            .leaves()
            .iter()
            .all(|l| l.name != "Rasterleştirilmiş")
    );
}

#[test]
fn a_contour_is_captured_and_given_its_elevation() {
    let mut app = scanned();
    let _ = app.update(Message::Run("processing.run.scan.captureLine"));
    value(
        &mut app,
        "input",
        json!({ "scope": "layer", "layerId": "pafta" }),
    );
    value(
        &mut app,
        "at",
        json!({ "x": 487_212.25, "y": 4_421_124.75 }),
    );
    value(&mut app, "z", json!(1000));
    let s = run(&mut app);
    let RunStatus::Ok { text, .. } = &s else {
        panic!("{s:?}");
    };
    assert_eq!(text, "1 çizgi yakalandı.");
    let (made, _) = layer_named(&app, "Yakalanan çizgiler");
    let doc = &app.document.as_ref().expect("open").model;
    let lines: Vec<_> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Polyline(p) if p.base.layer_id == made => Some(p),
            _ => None,
        })
        .collect();
    assert_eq!(lines.len(), 1);
    let p = lines[0];
    // A ring: its first point again at its end; every vertex at 1000.
    assert_eq!(p.pts.first(), p.pts.last());
    assert!(p.pts.len() > 8);
    assert!(
        p.zs.as_ref()
            .is_some_and(|zs| zs.iter().all(|z| *z == Some(1000.0)))
    );
}

#[test]
fn the_curves_take_their_elevations_in_one_step() {
    let mut app = scanned();
    let _ = app.update(Message::Run("processing.run.raster.toLines"));
    value(
        &mut app,
        "input",
        json!({ "scope": "layer", "layerId": "pafta" }),
    );
    value(&mut app, "select", json!("color"));
    value(&mut app, "color", json!("#9C5C2C"));
    value(&mut app, "spur", json!(5));
    let s = run(&mut app);
    assert!(matches!(s, RunStatus::Ok { .. }), "{s:?}");
    let _ = app.update(Message::Processing(Event::Close));
    let _ = app.update(Message::Run("processing.run.scan.contourElevations"));
    value(
        &mut app,
        "curves",
        json!({ "scope": "layer", "layerId": "islem-cizgiler" }),
    );
    value(
        &mut app,
        "start",
        json!({ "x": 487_187.75, "y": 4_421_049.75 }),
    );
    value(
        &mut app,
        "end",
        json!({ "x": 487_187.75, "y": 4_421_123.75 }),
    );
    value(&mut app, "first", json!(980));
    value(&mut app, "step", json!(5));
    let s = run(&mut app);
    let RunStatus::Ok { text, .. } = &s else {
        panic!("{s:?}");
    };
    assert!(text.contains("eğriye kot verildi: 980 ile "), "{text}");
    let doc = &mut app.document.as_mut().expect("open").model;
    let given = doc
        .entities()
        .filter(|e| matches!(e, Entity::Polyline(p) if p.zs.is_some()))
        .count();
    assert!(given >= 4, "{given}");
    assert_eq!(doc.undo().as_deref(), Some("Eğrilere kot ver"));
    assert!(
        doc.entities()
            .all(|e| !matches!(e, Entity::Polyline(p) if p.zs.is_some()))
    );
}
