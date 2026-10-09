//! Yüzey analizi in the desktop's window (docs/adr/0231), over the shared
//! valley (fixtures/interaction/v1/rasters.kcad): each tool run from its
//! window as the user runs it, in the background through the desktop's
//! files, its GeoTIFF written where asked (a scratch folder, never beside the
//! fixtures) and drawn on a layer of its own in one step; the contours as
//! lines with their levels; Eşyükselti üret and Eğim analizi open the tools;
//! a raster whose file is gone is said.

use std::path::PathBuf;

use kentos_contracts::{DocumentSnapshotV1, Entity, RasterRender, RasterSample};
use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const DRAWING: &str = include_str!("../../../../fixtures/interaction/v1/rasters.kcad");

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

/// The valley opened in `app` beside its rasters' folder (its elevation model 480 × 324 cells of 1.6 m).
pub(crate) fn open_valley(app: &mut App) {
    let snapshot = DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = Document::new(
        snapshot,
        Some(fixtures().join("interaction/v1/rasters.kcad")),
    )
    .expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
}

/// The app with the valley open.
fn app() -> App {
    let (mut app, _) = App::boot(None);
    open_valley(&mut app);
    app
}

fn value(app: &mut App, name: &str, v: Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

/// Runs the tool whose window is open, in the background, to its end.
pub(crate) fn run(app: &mut App) -> RunStatus {
    let task = app.update(Message::Processing(Event::Run));
    crate::files_testing::drive(app, task);
    app.processing
        .dialog
        .as_ref()
        .map_or(RunStatus::Idle, |w| w.status.clone())
}

/// Opens `tool`'s window on the elevation model, its result to be written at `out`.
pub(crate) fn open_on_dem(app: &mut App, tool: &'static str, out: &std::path::Path) {
    let _ = app.update(Message::Run(tool));
    value(app, "input", json!({ "scope": "layer", "layerId": "dem" }));
    value(app, "output", json!(out.to_string_lossy()));
}

fn layer_named(app: &App, name: &str) -> String {
    let doc = app.document.as_ref().expect("open");
    doc.model
        .layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == name)
        .map(|l| l.id.clone())
        .unwrap_or_else(|| panic!("{name}"))
}

#[test]
fn slope_is_written_where_asked_and_drawn_on_a_layer_of_its_own() {
    let dir = crate::files_testing::scratch("yuzey-egim");
    let out = dir.join("vadi egimi.tiff");
    let mut app = app();
    open_on_dem(&mut app, "processing.run.surface.slope", &out);
    let written = dir.join("vadi egimi.tif");
    let s = run(&mut app);
    let RunStatus::Ok { text, undo, .. } = &s else {
        panic!("{s:?}");
    };
    assert_eq!(
        text,
        &format!(
            "480 × 324 hücrelik raster; “{}” yazıldı.",
            written.display()
        )
    );
    assert!(undo, "the drawing changed");
    // The file is whole (renamed from .yaziliyor), its level 0 the model's size, 32-bit.
    let bytes = std::fs::read(&written).expect("the result");
    assert!(!dir.join("vadi egimi.tif.yaziliyor").exists());
    let reader = kentos_formats::raster::source::open_bytes(&bytes, None, 1 << 26).expect("reads");
    assert_eq!(
        (reader.info.width, reader.info.height, reader.info.sample),
        (480, 324, RasterSample::F32)
    );
    // Its object on Eğim, in the model's place, drawn in Spektral over its least to its most.
    let layer = layer_named(&app, "Eğim");
    let doc = &mut app.document.as_mut().expect("open").model;
    let raster = doc
        .entities()
        .find_map(|e| match e {
            Entity::Raster(r) if r.base.layer_id == layer => Some(r.raster.clone()),
            _ => None,
        })
        .expect("the result's object");
    assert_eq!(
        raster.file.as_deref(),
        Some(written.to_string_lossy().as_ref())
    );
    assert_eq!(raster.affine, [487200.0, 1.6, 0.0, 4420600.0, 0.0, -1.6]);
    assert_eq!(
        (raster.style.render, raster.style.ramp.as_deref()),
        (RasterRender::Ramp, Some("Spektral"))
    );
    assert_eq!(doc.undo().as_deref(), Some("Eğim"));
    assert!(doc.layers().leaves().iter().all(|l| l.name != "Eğim"));
}

#[test]
fn contours_are_lines_at_their_levels_with_kot_and_tur() {
    let dir = crate::files_testing::scratch("yuzey-egri");
    let mut app = app();
    open_on_dem(
        &mut app,
        "processing.run.surface.contours",
        &dir.join("yok.tif"),
    );
    value(&mut app, "interval", json!(2));
    value(&mut app, "indexEvery", json!(5));
    let s = run(&mut app);
    let RunStatus::Ok { text, .. } = &s else {
        panic!("{s:?}");
    };
    assert!(
        text.contains(" eğri (") && text.ends_with(" arası."),
        "{text}"
    );
    let layer = layer_named(&app, "Eş yükselti eğrileri");
    let doc = &app.document.as_ref().expect("open").model;
    let lines: Vec<_> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Polyline(p) if p.base.layer_id == layer => Some(p.clone()),
            _ => None,
        })
        .collect();
    assert!(lines.len() > 10, "{} lines", lines.len());
    for l in &lines {
        let kot: f64 = l.base.attrs["Kot"].parse().expect("a level");
        assert_eq!(kot % 2.0, 0.0, "a level of 2 m");
        let main = (kot / 2.0) as i64 % 5 == 0;
        assert_eq!(l.base.attrs["Tür"], if main { "Ana" } else { "Ara" });
        assert_eq!(l.base.line_weight, main.then_some(0.35));
        assert!(
            l.zs.as_ref()
                .expect("their elevations")
                .iter()
                .all(|z| *z == Some(kot))
        );
    }
    // Lines write no file.
    assert!(
        std::fs::read_dir(&dir)
            .expect("the folder")
            .next()
            .is_none()
    );
}

#[test]
fn the_terrain_commands_open_the_surface_tools() {
    let mut app = app();
    for (command, tool) in [
        ("map.contours", "surface.contours"),
        ("analysis.slope", "surface.slope"),
    ] {
        let _ = app.update(Message::Run(command));
        let open = app.processing.dialog.as_ref().map(|w| w.tool.id.clone());
        assert_eq!(open.as_deref(), Some(tool), "{command}");
        let _ = app.update(Message::Processing(Event::Close));
    }
}

#[test]
fn a_raster_whose_file_is_gone_is_said() {
    let dir = crate::files_testing::scratch("yuzey-yok");
    let mut app = app();
    if let Some(doc) = app.document.as_mut() {
        let slot = doc
            .model
            .entities()
            .find(|e| e.base().layer_id == "dem")
            .map(|e| kentos_domain::Slot(e.base().id))
            .expect("the model");
        let mut e = doc.model.get(slot).cloned().expect("the model");
        if let Entity::Raster(r) = &mut e {
            r.raster.file = Some("rasters/olmayan.tif".into());
        }
        let _ = kentos_interaction::properties::set_geometry(&mut doc.model, slot, &e);
    }
    open_on_dem(
        &mut app,
        "processing.run.surface.hillshade",
        &dir.join("golge.tif"),
    );
    let s = run(&mut app);
    let RunStatus::Error(why) = &s else {
        panic!("{s:?}");
    };
    assert!(why.contains("olmayan.tif"), "{why}");
    assert!(
        std::fs::read_dir(&dir)
            .expect("the folder")
            .next()
            .is_none()
    );
}
