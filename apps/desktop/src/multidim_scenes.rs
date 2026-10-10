//! Mesh ve çok boyutlu veri's pictures (docs/adr/0243) over the shared
//! valley: its hourly rain as a NetCDF grid and its stream's flood as a UGRID
//! mesh, both following the time slider, a section across the stream and
//! three gauges (fixtures/interaction/v1/multidim.kcad,
//! scripts/fixtures/multidim_scene.py): the CBS ribbon's Raster with Mesh
//! ekle, Raster ekle's NetCDF window, Mesh ekle's window, the flood at a
//! later step of the slider, Raster stili's Veri seti, Kesit's table, Zaman
//! serisi's table and Mesh hesaplayıcı's largest depth. The web's are
//! `shots.mjs multidim`, at the same places with the same values.
//! `tools_screens` takes them in the dark and the light theme at 1440×900
//! and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=md-serit,md-ekle,md-mesh-ekle,md-zaman,md-stil,md-kesit,md-seri,md-hesap-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Mesh hesaplayıcı's file is written into a scratch folder. Test code only.

use std::path::PathBuf;

use kentos_contracts::DocumentSnapshotV1;
use kentos_ui::snapshot::Snapshot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::processing::Event;
use crate::processing::surface_tests::run;
use crate::raster_scenes::tiles_made;
use crate::tools_screens::Pointed;

pub(crate) const MULTIDIM: &str = include_str!("../../../fixtures/interaction/v1/multidim.kcad");

const VALLEY: [f64; 4] = [487_200.0, 4_420_081.6, 487_968.0, 4_420_600.0];

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/interaction/v1/multidim")
}

/// The valley's rain and flood, the section and the gauges open in `app`, the CBS ribbon's Raster on.
pub(crate) fn opened(app: &mut App) {
    let snapshot = DocumentSnapshotV1::from_json(MULTIDIM).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interaction/v1/multidim.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.tab = "raster";
    app.command_expanded = false;
}

/// Runs a command and every task it starts, as the window would.
fn command(app: &mut App, id: &'static str) {
    let task = app.run(id);
    crate::files_testing::drive(app, task);
}

/// The view on the valley, then the rasters' tiles made.
fn view(s: &mut Snapshot, app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: VALLEY[0],
            min_y: VALLEY[1],
            max_x: VALLEY[2],
            max_y: VALLEY[3],
        },
        24.0,
    );
    tiles_made(s, app);
}

/// [`view`], the window's form scrolled to its end: the run's table under it.
fn view_end(s: &mut Snapshot, app: &mut App) {
    view(s, app);
    s.operate(app.view(), Box::new(crate::files_testing::SnapDown));
}

/// `tool`'s window with `values`.
fn window(app: &mut App, tool: &'static str, values: &[(&str, Value)]) {
    let _ = app.update(Message::Run(tool));
    for (name, v) in values {
        let _ = app.update(Message::Processing(Event::Value((*name).into(), v.clone())));
    }
}

/// `tool` run with `values` to its end; `close` its window after.
fn ran(app: &mut App, tool: &'static str, values: &[(&str, Value)], close: bool) {
    window(app, tool, values);
    let status = run(app);
    assert!(
        matches!(status, crate::processing::RunStatus::Ok { .. }),
        "{tool}: {status:?}"
    );
    if close {
        let _ = app.update(Message::Processing(Event::Close));
    }
}

fn layer(id: &str) -> Value {
    json!({ "scope": "layer", "layerId": id })
}

/// The raster of `layer` selected.
fn select(app: &mut App, layer: &str) {
    let doc = app.document.as_ref().expect("a drawing");
    let slot = doc
        .model
        .entities()
        .find(|e| e.base().layer_id == layer)
        .map(|e| kentos_domain::Slot(e.base().id))
        .expect("the raster");
    app.selection.set(vec![slot]);
}

/// Kesit across and along the stream on the flood, its lines drawn with the depths.
pub(crate) fn profile() -> Vec<(&'static str, Value)> {
    vec![
        ("input", layer("taskin")),
        ("lines", layer("kesit")),
        ("step", json!(10)),
        ("draw", json!(true)),
    ]
}

/// Zaman serisi of the flood's depth at the three gauges.
pub(crate) fn series() -> Vec<(&'static str, Value)> {
    vec![("input", layer("taskin")), ("points", layer("istasyonlar"))]
}

/// Mesh hesaplayıcı: the largest depth over the twelve steps.
pub(crate) fn largest() -> Vec<(&'static str, Value)> {
    vec![
        ("input", layer("taskin")),
        ("expression", json!("derinlik")),
        ("summary", json!("max")),
        ("name", json!("En büyük derinlik")),
        (
            "output",
            json!(
                crate::files_testing::scratch("md-resim")
                    .join("en-buyuk.nc")
                    .to_string_lossy()
            ),
        ),
    ]
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The CBS ribbon's Raster with Mesh ekle, over the valley's flood and rain.
        ("md-serit", opened, view),
        // Raster ekle's NetCDF: the rain's variable, its hour and Zaman sürgüsünü izle.
        (
            "md-ekle",
            |app| {
                opened(app);
                app.picker = crate::app::Picker::File(folder().join("yagis.nc"));
                command(app, "raster.add");
            },
            view,
        ),
        // Mesh ekle: the flood's datasets, its step, the cell and the mesh's lines.
        (
            "md-mesh-ekle",
            |app| {
                opened(app);
                app.picker = crate::app::Picker::File(folder().join("taskin.nc"));
                command(app, "mesh.add");
                // Su derinliği, its step and following the slider, the mesh's lines on.
                for e in [
                    crate::rasters::multidim::Event::Choice(1),
                    crate::rasters::multidim::Event::Edges,
                ] {
                    let task = app.update(Message::Rasters(crate::rasters::Event::Multidim(e)));
                    crate::files_testing::drive(app, task);
                }
            },
            view,
        ),
        // The time slider at the ninth step: the flood wave moved downstream, the storm east.
        (
            "md-zaman",
            |app| {
                opened(app);
                command(app, "time.slider");
                let _ = app.update(Message::Time(crate::temporal::Event::Go(8)));
            },
            view,
        ),
        // Raster stili over the flood: its Veri seti, the step, following the slider and the mesh's lines.
        (
            "md-stil",
            |app| {
                opened(app);
                select(app, "taskin");
                command(app, "raster.style");
            },
            view,
        ),
        // Kesit across and along the stream: its table and the lines drawn with the depths.
        (
            "md-kesit",
            |app| {
                opened(app);
                ran(app, "processing.run.multidim.profile", &profile(), false);
            },
            view_end,
        ),
        // Zaman serisi at the three gauges: a row a half hour.
        (
            "md-seri",
            |app| {
                opened(app);
                ran(app, "processing.run.multidim.series", &series(), false);
            },
            view_end,
        ),
        // Mesh hesaplayıcı's largest depth over the twelve steps, right above the flood.
        (
            "md-hesap-cizim",
            |app| {
                opened(app);
                if let Some(doc) = app.document.as_mut() {
                    doc.model.set_layer_visible("yagis", false);
                }
                ran(
                    app,
                    "processing.run.multidim.meshCalculator",
                    &largest(),
                    true,
                );
            },
            view,
        ),
    ]
}
