//! Uzaklık ve maliyet's pictures (docs/adr/0236) over the shared valley's
//! elevation model, a cost raster made from its slope with a lake that
//! cannot be crossed, five villages and the road along the valley's south
//! (fixtures/interaction/v1/distance.kcad, scripts/fixtures/distance_scene.py):
//! the distance from the road, the villages' nearest one, the cost from
//! Köy A, the least cost paths to two villages climbing at most 30 % and the
//! corridor to Köy B. The web's are `shots.mjs distance`, at the same places
//! with the same values. `tools_screens` takes them in the dark and the
//! light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=uzk-serit,uzk-yol-cizim,uzk-koridor-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Each raster result is written into a scratch folder. Test code only.

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

pub(crate) const DISTANCE: &str = include_str!("../../../fixtures/interaction/v1/distance.kcad");

const VALLEY: [f64; 4] = [487_200.0, 4_420_081.6, 487_968.0, 4_420_600.0];

/// The valley with the villages, the road and the cost raster open in `app`.
pub(crate) fn open_distance(app: &mut App) {
    let snapshot = DocumentSnapshotV1::from_json(DISTANCE).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interaction/v1/distance.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
}

fn opened(app: &mut App) {
    open_distance(app);
    app.tab = "raster";
    app.command_expanded = false;
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

/// `tool`'s window with `values`.
fn window(app: &mut App, tool: &'static str, values: &[(&str, Value)]) {
    let _ = app.update(Message::Run(tool));
    for (name, v) in values {
        let _ = app.update(Message::Processing(Event::Value((*name).into(), v.clone())));
    }
}

/// A raster result's file in a scratch folder.
fn scratch_file(name: &str) -> Value {
    json!(
        crate::files_testing::scratch("uzk-resim")
            .join(name)
            .to_string_lossy()
    )
}

/// `tool` run with `values` to its end, its window closed.
fn ran(app: &mut App, tool: &'static str, values: &[(&str, Value)]) {
    window(app, tool, values);
    let status = run(app);
    assert!(
        matches!(status, crate::processing::RunStatus::Ok { .. }),
        "{tool}: {status:?}"
    );
    let _ = app.update(Message::Processing(Event::Close));
}

pub(crate) fn layer(id: &str) -> Value {
    json!({ "scope": "layer", "layerId": id })
}

/// Uzaklık yüzeyi from the road onto the cost raster's grid.
pub(crate) fn from_road() -> Vec<(&'static str, Value)> {
    vec![
        ("sources", layer("yol")),
        ("extent", json!("raster")),
        ("grid", layer("maliyet")),
        ("output", scratch_file("yoldan.tif")),
    ]
}

/// En düşük maliyetli yol from Köy A to Köy B and Köy C, climbing at most 30 % on the elevation model.
pub(crate) fn paths() -> Vec<(&'static str, Value)> {
    vec![
        ("input", layer("maliyet")),
        ("sources", layer("baslangic")),
        ("targets", layer("varis")),
        ("useSurface", json!(true)),
        ("surface", layer("dem")),
        ("surfaceLength", json!(true)),
        ("slope", json!(30)),
    ]
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The CBS ribbon's Raster with Uzaklık ve maliyet, over the cost raster.
        ("uzk-serit", opened, view),
        // Uzaklık yüzeyi's window: from the road onto the cost raster's grid.
        (
            "uzk-yuzey",
            |app| {
                opened(app);
                window(app, "processing.run.distance.euclidean", &from_road());
            },
            view,
        ),
        // The distance from the road over the valley.
        (
            "uzk-yuzey-cizim",
            |app| {
                opened(app);
                ran(app, "processing.run.distance.euclidean", &from_road());
            },
            view,
        ),
        // Every cell's nearest village.
        (
            "uzk-tahsis-cizim",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.distance.euclidean",
                    &[
                        ("sources", layer("koyler")),
                        ("result", json!("allocation")),
                        ("extent", json!("raster")),
                        ("grid", layer("maliyet")),
                        ("output", scratch_file("koyler.tif")),
                    ],
                );
            },
            view,
        ),
        // The cost from Köy A: the lake and the hill's steep sides dear.
        (
            "uzk-maliyet-cizim",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.distance.cost",
                    &[
                        ("input", layer("maliyet")),
                        ("sources", layer("baslangic")),
                        ("output", scratch_file("maliyet-a.tif")),
                    ],
                );
            },
            view,
        ),
        // En düşük maliyetli yol's window with the elevation model.
        (
            "uzk-yol",
            |app| {
                opened(app);
                window(app, "processing.run.distance.path", &paths());
            },
            view,
        ),
        // The paths to Köy B and Köy C over the cost raster.
        (
            "uzk-yol-cizim",
            |app| {
                opened(app);
                ran(app, "processing.run.distance.path", &paths());
            },
            view,
        ),
        // The corridor from Köy A to Köy B within 5 % of the cheapest, its path over it (the path's layer
        // first, the corridor's then goes right above the cost raster, under it).
        (
            "uzk-koridor-cizim",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.distance.path",
                    &[
                        ("input", layer("maliyet")),
                        ("sources", layer("baslangic")),
                        ("targets", layer("ikinci")),
                    ],
                );
                ran(
                    app,
                    "processing.run.distance.corridor",
                    &[
                        ("input", layer("maliyet")),
                        ("sources", layer("baslangic")),
                        ("targets", layer("ikinci")),
                        ("threshold", json!("percent")),
                        ("percent", json!(5)),
                        ("output", scratch_file("koridor.tif")),
                    ],
                );
            },
            view,
        ),
    ]
}
