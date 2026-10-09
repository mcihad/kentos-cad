//! İnterpolasyon and Yoğunluk's pictures (docs/adr/0232 §13) over the shared
//! processing drawing (fixtures/processing/v1/interpolation.kcad: 40 survey
//! points over 120 × 80 m, 60 events, roads): the CBS ribbon's Raster tab with
//! its İnterpolasyon and Yoğunluk panels, IDW's and Kriging's windows, a run's
//! cross-validation table, each interpolation's surface drawn under its points
//! (a metre a cell), and the two densities. The web's are `shots.mjs
//! interpolation`. `tools_screens` takes them in the dark and the light theme
//! at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=interp-serit,interp-idw-cizim,yogunluk-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Each result is written into a scratch folder. Test code only.

use std::path::PathBuf;

use kentos_contracts::DocumentSnapshotV1;
use kentos_ui::snapshot::Snapshot;
use serde_json::json;

use crate::app::{App, Message};
use crate::document::Document;
use crate::processing::Event;
use crate::processing::surface_tests::run;
use crate::raster_scenes::tiles_made;
use crate::tools_screens::Pointed;

const DRAWING: &str = include_str!("../../../fixtures/processing/v1/interpolation.kcad");

/// The drawing open in `app`, the CBS ribbon's Raster on; the layers a scene does not show hidden.
fn opened(app: &mut App) {
    let snapshot = DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/processing/v1/interpolation.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.tab = "raster";
    app.command_expanded = false;
    if let Some(doc) = app.document.as_mut() {
        for id in ["alan", "izgara", "dogru", "kotsuz"] {
            doc.model.set_layer_visible(id, false);
        }
    }
}

fn fit(app: &mut App, b: [f64; 4]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: b[0],
            min_y: b[1],
            max_x: b[2],
            max_y: b[3],
        },
        24.0,
    );
}

/// The view on the survey points (fitted once the area has its size), then the rasters' tiles made.
fn on_points(s: &mut Snapshot, app: &mut App) {
    fit(app, [499_998.0, 4_419_998.0, 500_122.0, 4_420_082.0]);
    tiles_made(s, app);
}

/// The view on the events.
fn on_events(s: &mut Snapshot, app: &mut App) {
    fit(app, [499_990.0, 4_419_990.0, 500_210.0, 4_420_210.0]);
    tiles_made(s, app);
}

/// The view on the roads.
fn on_roads(s: &mut Snapshot, app: &mut App) {
    fit(app, [-6.0, -6.0, 106.0, 98.0]);
    tiles_made(s, app);
}

fn value(app: &mut App, name: &str, v: serde_json::Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

/// `tool`'s window on `layer` with `values`, its result to go to a scratch folder.
fn window(app: &mut App, tool: &'static str, layer: &str, values: &[(&str, serde_json::Value)]) {
    opened(app);
    let dir = crate::files_testing::scratch("interp-resim");
    let _ = app.update(Message::Run(tool));
    value(app, "input", json!({ "scope": "layer", "layerId": layer }));
    value(
        app,
        "output",
        json!(dir.join("sonuc.tif").to_string_lossy()),
    );
    for (name, v) in values {
        value(app, name, v.clone());
    }
}

/// `tool` run on `layer` with `values`, its window closed (or kept, `keep`).
fn ran(
    app: &mut App,
    tool: &'static str,
    layer: &str,
    values: &[(&str, serde_json::Value)],
    keep: bool,
) {
    window(app, tool, layer, values);
    let status = run(app);
    assert!(
        matches!(status, crate::processing::RunStatus::Ok { .. }),
        "{tool}: {status:?}"
    );
    if !keep {
        let _ = app.update(Message::Processing(Event::Close));
    }
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The CBS ribbon's Raster: the rasters, Yüzey analizi, İnterpolasyon and Yoğunluk.
        ("interp-serit", opened, on_points),
        // IDW's window over the survey points.
        (
            "interp-idw",
            |app| {
                window(
                    app,
                    "processing.run.interpolation.idw",
                    "noktalar",
                    &[("cellSize", json!(1)), ("cross", json!(true))],
                )
            },
            on_points,
        ),
        (
            "interp-idw-cizim",
            |app| {
                ran(
                    app,
                    "processing.run.interpolation.idw",
                    "noktalar",
                    &[("cellSize", json!(1))],
                    false,
                )
            },
            on_points,
        ),
        // A run's cross-validation table under the form.
        (
            "interp-capraz",
            |app| {
                ran(
                    app,
                    "processing.run.interpolation.idw",
                    "noktalar",
                    &[("cellSize", json!(1)), ("cross", json!(true))],
                    true,
                )
            },
            on_points,
        ),
        (
            "interp-dogal-cizim",
            |app| {
                ran(
                    app,
                    "processing.run.interpolation.naturalNeighbor",
                    "noktalar",
                    &[("cellSize", json!(1))],
                    false,
                )
            },
            on_points,
        ),
        (
            "interp-tin-cizim",
            |app| {
                ran(
                    app,
                    "processing.run.interpolation.tin",
                    "noktalar",
                    &[("cellSize", json!(1))],
                    false,
                )
            },
            on_points,
        ),
        (
            "interp-spline-cizim",
            |app| {
                ran(
                    app,
                    "processing.run.interpolation.spline",
                    "noktalar",
                    &[("cellSize", json!(1))],
                    false,
                )
            },
            on_points,
        ),
        // Kriging's window: a given variogram and its error surface.
        (
            "interp-kriging",
            |app| {
                window(
                    app,
                    "processing.run.interpolation.kriging",
                    "noktalar",
                    &[
                        ("variogram", json!("manual")),
                        ("nugget", json!(0.5)),
                        ("sill", json!(120)),
                        ("range", json!(90)),
                        ("errorSurface", json!(true)),
                    ],
                )
            },
            on_points,
        ),
        (
            "interp-kriging-cizim",
            |app| {
                ran(
                    app,
                    "processing.run.interpolation.kriging",
                    "noktalar",
                    &[
                        ("cellSize", json!(1)),
                        ("variogram", json!("manual")),
                        ("nugget", json!(0.5)),
                        ("sill", json!(120)),
                        ("range", json!(90)),
                    ],
                    false,
                )
            },
            on_points,
        ),
        // Çekirdek yoğunluğu over the events, 30 m, a metre a cell: a heat map, its zeros clear.
        (
            "yogunluk-cizim",
            |app| {
                ran(
                    app,
                    "processing.run.density.kernel",
                    "olaylar",
                    &[("radius", json!(30)), ("cellSize", json!(1))],
                    false,
                )
            },
            on_events,
        ),
        // Çizgi yoğunluğu over the roads, 12 m.
        (
            "cizgi-yogunlugu-cizim",
            |app| {
                ran(
                    app,
                    "processing.run.density.line",
                    "yollar",
                    &[("radius", json!(12)), ("cellSize", json!(0.5))],
                    false,
                )
            },
            on_roads,
        ),
    ]
}
