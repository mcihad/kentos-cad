//! Yüzey analizi in the desktop's window (docs/adr/0231), over the shared
//! valley (fixtures/interaction/v1/rasters.kcad): each tool run from its
//! window as the user runs it, in the background through the desktop's
//! files, its GeoTIFF written where asked (a scratch folder, never beside the
//! fixtures) and drawn on a layer of its own in one step; the contours as
//! lines with their levels; Eşyükselti üret and Eğim analizi open the tools;
//! a raster whose file is gone is said.

use crate::gis;
use std::path::PathBuf;

use kentos_commands::{DocumentSnapshotV1, Entity, RasterRender, RasterSample};
use serde_json::{Value, json};

use super::{Event, RunStatus};
use crate::app::{App, Message};
use crate::document::Document;

const DRAWING: &str = include_str!("../../../../../fixtures/interaction/v1/rasters.kcad");

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
    let _ = app.update(Message::Gis(gis::Event::Processing(Event::Value(
        name.into(),
        v,
    ))));
}

/// Runs the tool whose window is open, in the background, to its end.
pub(crate) fn run(app: &mut App) -> RunStatus {
    let task = app.update(Message::Gis(gis::Event::Processing(Event::Run)));
    crate::files_testing::drive(app, task);
    app.gis
        .processing
        .dialog
        .as_ref()
        .map_or(RunStatus::Idle, |w| w.status.clone())
}

/// Opens `tool`'s window on the elevation model, its result to be written at `out`.
pub(crate) fn open_on_dem(app: &mut App, tool: &'static str, out: &std::path::Path) {
    let _ = app.update(Message::Run(tool));
    app.keep_tool_names();
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
        app.keep_tool_names();
        let open = app
            .gis
            .processing
            .dialog
            .as_ref()
            .map(|w| w.tool.id.clone());
        assert_eq!(open.as_deref(), Some(tool), "{command}");
        let _ = app.update(Message::Gis(gis::Event::Processing(Event::Close)));
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

// docs/adr/0535: Kapsam nesnelerin kutusu: the result is the chosen parcel's box's cells of the source's grid.
#[test]
fn slope_on_an_area_is_written_on_the_source_s_grid() {
    let dir = crate::files_testing::scratch("yuzey-alan");
    let out = dir.join("alan.tif");
    let mut app = app();
    crate::surface_scenes::area_chosen(&mut app);
    open_on_dem(&mut app, "processing.run.surface.slope", &out);
    value(&mut app, "area", json!("objects"));
    value(&mut app, "areaObjects", json!({ "scope": "selection" }));
    let s = run(&mut app);
    let RunStatus::Ok { text, .. } = &s else {
        panic!("{s:?}");
    };
    assert!(out.exists(), "{text}");
    let doc = app.document.as_ref().expect("open");
    let made = doc
        .model
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Raster(r)
                if r.raster.file.as_deref() == Some(&*out.to_string_lossy()) =>
            {
                Some(r.raster.clone())
            }
            _ => None,
        })
        .expect("the result on the drawing");
    let dem = doc
        .model
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Raster(r) if r.base.layer_id == "dem" => {
                Some(r.raster.clone())
            }
            _ => None,
        })
        .expect("the source");
    assert!(made.width < dem.width && made.height < dem.height, "{text}");
    // The same cell size and direction; the origin a whole number of the source's cells away.
    assert_eq!(made.affine[1..3], dem.affine[1..3]);
    assert_eq!(made.affine[4..6], dem.affine[4..6]);
    let i = (made.affine[0] - dem.affine[0]) / dem.affine[1];
    let j = (made.affine[3] - dem.affine[3]) / dem.affine[5];
    assert!(
        (i - i.round()).abs() < 1e-9 && (j - j.round()).abs() < 1e-9,
        "{i}, {j}"
    );
}

/// GIS-012 (docs/adr/1078): with no output file asked, each run takes a file of its own beside the model — a degree
/// slope `dem-egim.tif`, a percent one `dem-egim-yuzde.tif`, a second degree one `dem-egim-2.tif` — so no layer's file
/// is written over; undone and opened again, the first result's file still holds its own values.
#[test]
fn an_automatic_result_file_never_writes_over_an_earlier_one() {
    let dir = crate::files_testing::scratch("yuzey-otomatik-ad");
    std::fs::create_dir_all(dir.join("rasters")).expect("a folder");
    std::fs::copy(fixtures().join("interaction/v1/rasters/dem.tif"), dir.join("rasters/dem.tif")).expect("the model");
    let snapshot = DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let doc = Document::new(snapshot, Some(dir.join("vadi.kcad"))).expect("opens");
    let (mut app, _) = App::boot(None);
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let slope = |app: &mut App, unit: &str| {
        let _ = app.update(Message::Run("processing.run.surface.slope"));
        app.keep_tool_names();
        value(app, "input", json!({ "scope": "layer", "layerId": "dem" }));
        value(app, "unit", json!(unit));
        let s = run(app);
        assert!(matches!(s, RunStatus::Ok { .. }), "{s:?}");
    };
    slope(&mut app, "degrees");
    let first = dir.join("rasters/dem-egim.tif");
    let before = std::fs::read(&first).expect("the degree slope");
    slope(&mut app, "percent");
    assert!(dir.join("rasters/dem-egim-yuzde.tif").exists());
    // GIS-016: each file names its band's unit in GDAL's metadata, whatever it is renamed to.
    let unit_of = |path: &std::path::Path| {
        let bytes = std::fs::read(path).expect("written");
        let text = String::from_utf8_lossy(&bytes).into_owned();
        ["degree", "percent"].into_iter().find(|u| {
            text.contains(&format!("<Item name=\"UNITTYPE\" sample=\"0\" role=\"unittype\">{u}</Item>"))
        })
    };
    assert_eq!(unit_of(&first), Some("degree"));
    assert_eq!(unit_of(&dir.join("rasters/dem-egim-yuzde.tif")), Some("percent"));
    slope(&mut app, "degrees");
    assert!(dir.join("rasters/dem-egim-2.tif").exists(), "a second degree slope takes its own file");
    assert_eq!(std::fs::read(&first).expect("still there"), before, "the first result is not written over");
    // Each result's object reads its own file.
    let doc = &mut app.document.as_mut().expect("open").model;
    let files: Vec<String> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Raster(r) if r.raster.file.as_deref().is_some_and(|f| f.contains("egim")) => r.raster.file.clone(),
            _ => None,
        })
        .collect();
    let unique: std::collections::BTreeSet<&String> = files.iter().collect();
    assert_eq!((files.len(), unique.len()), (3, 3), "{files:?}");
    doc.undo();
    assert_eq!(std::fs::read(&first).expect("kept after undo"), before);
}
