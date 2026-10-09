//! Hidroloji's pictures (docs/adr/0235) over the shared valley's elevation
//! model with a road's axis and two outlets (fixtures/interaction/v1/
//! hydrology.kcad, scripts/fixtures/hydrology_scene.py): the filled lakes'
//! depth, the flow accumulation, the wetness index, the stream network, the
//! outlets' watersheds, the links' sub-basins and the basins of the streams
//! the road crosses. The web's are `shots.mjs hydrology`, at the same places
//! with the same values. `tools_screens` takes them in the dark and the
//! light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=hid-serit,hid-dere-cizim,hid-guzergah-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
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

pub(crate) const HYDROLOGY: &str = include_str!("../../../fixtures/interaction/v1/hydrology.kcad");

const VALLEY: [f64; 4] = [487_200.0, 4_420_081.6, 487_968.0, 4_420_600.0];
/// The eastern streams and the road across them, the view's right side the model's east edge.
const EAST: [f64; 4] = [487_400.0, 4_420_160.0, 487_968.0, 4_420_490.0];

/// The valley with the road and the outlets open in `app`, the CBS ribbon's Raster on.
pub(crate) fn open_hydrology(app: &mut App) {
    let snapshot = DocumentSnapshotV1::from_json(HYDROLOGY).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interaction/v1/hydrology.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
}

fn opened(app: &mut App) {
    open_hydrology(app);
    app.tab = "raster";
    app.command_expanded = false;
}

/// The view on `b`, then the rasters' tiles made.
fn view_on(s: &mut Snapshot, app: &mut App, b: [f64; 4]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: b[0],
            min_y: b[1],
            max_x: b[2],
            max_y: b[3],
        },
        24.0,
    );
    tiles_made(s, app);
}

fn value(app: &mut App, name: &str, v: Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

/// `tool`'s window with `values`.
fn window(app: &mut App, tool: &'static str, values: &[(&str, Value)]) {
    let _ = app.update(Message::Run(tool));
    for (name, v) in values {
        value(app, name, v.clone());
    }
}

/// A raster result's file in a scratch folder.
fn scratch_file(name: &str) -> Value {
    json!(
        crate::files_testing::scratch("hid-resim")
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

fn layer(id: &str) -> Value {
    json!({ "scope": "layer", "layerId": id })
}

fn dem() -> (&'static str, Value) {
    ("input", layer("dem"))
}

/// Dere ağı at 2000 m².
fn streams(app: &mut App) {
    ran(
        app,
        "processing.run.hydrology.streams",
        &[dem(), ("threshold", json!(2000))],
    );
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The CBS ribbon's Raster with Hidroloji.
        ("hid-serit", opened, |s, app| view_on(s, app, VALLEY)),
        // The filled lakes' depth over the valley.
        (
            "hid-doldur-cizim",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.hydrology.fill",
                    &[
                        dem(),
                        ("result", json!("depth")),
                        ("output", scratch_file("derinlik.tif")),
                    ],
                );
            },
            |s, app| view_on(s, app, VALLEY),
        ),
        // The D8 accumulation in cells: the network bright on the dark slopes.
        (
            "hid-birikim-cizim",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.hydrology.flowAccumulation",
                    &[dem(), ("output", scratch_file("birikim.tif"))],
                );
            },
            |s, app| view_on(s, app, VALLEY),
        ),
        // The wetness index by the multiple flow directions: wet floors blue, dry ridges red.
        (
            "hid-twi-cizim",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.hydrology.wetness",
                    &[dem(), ("output", scratch_file("twi.tif"))],
                );
            },
            |s, app| view_on(s, app, VALLEY),
        ),
        // Dere ağı's window at 2000 m².
        (
            "hid-dere",
            |app| {
                opened(app);
                window(
                    app,
                    "processing.run.hydrology.streams",
                    &[dem(), ("threshold", json!(2000))],
                );
            },
            |s, app| view_on(s, app, VALLEY),
        ),
        // The network over the shaded relief.
        (
            "hid-dere-cizim",
            |app| {
                opened(app);
                streams(app);
            },
            |s, app| view_on(s, app, VALLEY),
        ),
        // The two outlets moved onto their streams and their watersheds.
        (
            "hid-havza-cizim",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.hydrology.watershed",
                    &[dem(), ("points", layer("cikis")), ("snap", json!(10))],
                );
                streams(app);
            },
            |s, app| view_on(s, app, VALLEY),
        ),
        // Every link's own basin at 2000 m², the network over them.
        (
            "hid-alt-havzalar-cizim",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.hydrology.basins",
                    &[dem(), ("mode", json!("sub")), ("threshold", json!(2000))],
                );
                streams(app);
            },
            |s, app| view_on(s, app, VALLEY),
        ),
        // Havzalar's window: the streams the road crosses.
        (
            "hid-guzergah",
            |app| {
                opened(app);
                streams(app);
                window(
                    app,
                    "processing.run.hydrology.basins",
                    &[
                        dem(),
                        ("mode", json!("route")),
                        ("threshold", json!(2000)),
                        ("routes", layer("yol")),
                    ],
                );
            },
            |s, app| view_on(s, app, EAST),
        ),
        // The crossings' basins, each whole, with their km on the road.
        (
            "hid-guzergah-cizim",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.hydrology.basins",
                    &[
                        dem(),
                        ("mode", json!("route")),
                        ("threshold", json!(2000)),
                        ("routes", layer("yol")),
                    ],
                );
                streams(app);
            },
            |s, app| view_on(s, app, EAST),
        ),
    ]
}
