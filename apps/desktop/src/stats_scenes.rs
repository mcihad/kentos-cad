//! Mekânsal istatistik's pictures (docs/adr/0238) over a district's roads,
//! blocks with a value per m² and traffic accidents by kind
//! (fixtures/interaction/v1/spatial-stats.kcad,
//! scripts/fixtures/spatial_stats_scene.py): the CBS ribbon's Analiz with
//! its panel, Yön dağılımı's window and the ellipses with the mean centres
//! by kind, Moran I's and En yakın komşu's tables after their runs, the
//! blocks' hot and cold spots and the accidents' DBSCAN clusters. The web's
//! are `shots.mjs stats`, at the same places with the same values.
//! `tools_screens` takes them in the dark and the light theme at 1440×900
//! and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=ist-serit,ist-elips,ist-elips-cizim,ist-moran,ist-komsu,ist-sicak-cizim,ist-dbscan-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::path::PathBuf;

use kentos_contracts::DocumentSnapshotV1;
use kentos_ui::snapshot::Snapshot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::processing::Event;
use crate::processing::surface_tests::run;
use crate::tools_screens::Pointed;

pub(crate) const STATS: &str = include_str!("../../../fixtures/interaction/v1/spatial-stats.kcad");

const DISTRICT: [f64; 4] = [486_960.0, 4_419_960.0, 488_240.0, 4_420_940.0];

/// The district open in `app`.
pub(crate) fn open_stats(app: &mut App) {
    let snapshot = DocumentSnapshotV1::from_json(STATS).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interaction/v1/spatial-stats.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
}

fn opened(app: &mut App) {
    open_stats(app);
    app.tab = "analysis";
    app.command_expanded = false;
}

fn view(_s: &mut Snapshot, app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: DISTRICT[0],
            min_y: DISTRICT[1],
            max_x: DISTRICT[2],
            max_y: DISTRICT[3],
        },
        24.0,
    );
}

/// [`view`], the window's form scrolled to its end: the run's table under it.
fn view_end(s: &mut Snapshot, app: &mut App) {
    view(s, app);
    s.operate(app.view(), Box::new(crate::files_testing::SnapAll));
}

pub(crate) fn layer(id: &str) -> Value {
    json!({ "scope": "layer", "layerId": id })
}

/// `tool`'s window with `values`.
pub(crate) fn window(app: &mut App, tool: &'static str, values: &[(&str, Value)]) {
    let _ = app.update(Message::Run(tool));
    for (name, v) in values {
        let _ = app.update(Message::Processing(Event::Value((*name).into(), v.clone())));
    }
}

/// `tool` run with `values` to its end; `close` its window after.
pub(crate) fn ran(app: &mut App, tool: &'static str, values: &[(&str, Value)], close: bool) {
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

/// Yön dağılımı of the accidents by kind.
pub(crate) fn ellipses() -> Vec<(&'static str, Value)> {
    vec![("input", layer("kaza")), ("groupField", json!("Tür"))]
}

/// Moran I of the blocks' values, the band left to itself.
pub(crate) fn moran() -> Vec<(&'static str, Value)> {
    vec![("input", layer("ada")), ("valueField", json!("Değer"))]
}

/// Gi* of the blocks' values over their 4 nearest.
pub(crate) fn hot() -> Vec<(&'static str, Value)> {
    vec![
        ("input", layer("ada")),
        ("valueField", json!("Değer")),
        ("concept", json!("nearest")),
        ("neighbors", json!(4)),
    ]
}

/// DBSCAN of the accidents: 30 m and 5 places.
pub(crate) fn clusters() -> Vec<(&'static str, Value)> {
    vec![
        ("input", layer("kaza")),
        ("radius", json!(30)),
        ("minPoints", json!(5)),
    ]
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The CBS ribbon's Analiz with Mekânsal istatistik, over the district.
        ("ist-serit", opened, view),
        // Yön dağılımı's window: the accidents by kind.
        (
            "ist-elips",
            |app| {
                opened(app);
                window(
                    app,
                    "processing.run.stats.directionalDistribution",
                    &ellipses(),
                );
            },
            view,
        ),
        // The ellipses and the mean centres by kind: pedestrians at the bazaar, cars along the avenue, bicycles north–south.
        (
            "ist-elips-cizim",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.stats.directionalDistribution",
                    &ellipses(),
                    true,
                );
                ran(app, "processing.run.stats.meanCenter", &ellipses(), true);
            },
            view,
        ),
        // Moran I's table after its run: the blocks' values cluster.
        (
            "ist-moran",
            |app| {
                opened(app);
                ran(app, "processing.run.stats.moransI", &moran(), false);
            },
            view_end,
        ),
        // En yakın komşu's table: the accidents cluster.
        (
            "ist-komsu",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.stats.nearestNeighbor",
                    &[("input", layer("kaza"))],
                    false,
                );
            },
            view_end,
        ),
        // The blocks' hot and cold spots in their classes' colours.
        (
            "ist-sicak-cizim",
            |app| {
                opened(app);
                ran(app, "processing.run.stats.hotSpot", &hot(), true);
            },
            view,
        ),
        // The accidents' DBSCAN clusters in their colours, the noise grey.
        (
            "ist-dbscan-cizim",
            |app| {
                opened(app);
                ran(app, "processing.run.stats.dbscan", &clusters(), true);
            },
            view,
        ),
    ]
}
