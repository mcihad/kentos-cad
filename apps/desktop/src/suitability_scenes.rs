//! Uygunluk analizi's pictures (docs/adr/0237) over the shared valley's
//! elevation model, three criteria made from it (the slope, the distance
//! from the road, the land cover), the road and the landslides on the steep
//! sides (fixtures/interaction/v1/suitability.kcad,
//! scripts/fixtures/suitability_scene.py): Bulanık üyelik's window,
//! Ağırlıklı çakıştırma's window with its influences and class tables and
//! its result over the valley, İkili karşılaştırma's weights and ROC's curve
//! after their runs. The web's are `shots.mjs suitability`, at the same
//! places with the same values. `tools_screens` takes them in the dark and
//! the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=uyg-serit,uyg-cakistirma,uyg-cakistirma-cizim,uyg-ahp,uyg-roc cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Each raster result is written into a scratch folder. Test code only.

use std::path::PathBuf;

use kentos_contracts::DocumentSnapshotV1;
use kentos_domain::Slot;
use kentos_ui::snapshot::Snapshot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::processing::Event;
use crate::processing::surface_tests::run;
use crate::raster_scenes::tiles_made;
use crate::tools_screens::Pointed;

pub(crate) const SUITABILITY: &str =
    include_str!("../../../fixtures/interaction/v1/suitability.kcad");

const VALLEY: [f64; 4] = [487_200.0, 4_420_081.6, 487_968.0, 4_420_600.0];

/// The criteria rasters' objects: the slope, the distance from the road, the land cover.
pub(crate) const CRITERIA: [Slot; 3] = [Slot(26), Slot(27), Slot(28)];

/// The valley with its criteria, the road and the landslides open in `app`.
pub(crate) fn open_suitability(app: &mut App) {
    let snapshot = DocumentSnapshotV1::from_json(SUITABILITY).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interaction/v1/suitability.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
}

/// The valley open with its three criteria selected.
pub(crate) fn with_criteria(app: &mut App) {
    open_suitability(app);
    app.selection.set(CRITERIA);
}

fn opened(app: &mut App) {
    with_criteria(app);
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

/// [`view`], the window's form scrolled to its end: the run's table under it (the window does so after a run; the
/// pictures take no tasks).
fn view_end(s: &mut Snapshot, app: &mut App) {
    view(s, app);
    s.operate(app.view(), Box::new(crate::files_testing::SnapAll));
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
        crate::files_testing::scratch("uyg-resim")
            .join(name)
            .to_string_lossy()
    )
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

pub(crate) fn layer(id: &str) -> Value {
    json!({ "scope": "layer", "layerId": id })
}

/// Ağırlıklı çakıştırma over the three criteria: 45 %, 30 % and 25 %, each with its classes on 1–9.
pub(crate) fn overlay() -> Vec<(&'static str, Value)> {
    vec![
        ("input", json!({ "scope": "selection" })),
        (
            "influence",
            json!({ "Eğim": 45, "Yola uzaklık": 30, "Arazi örtüsü": 25 }),
        ),
        (
            "classes",
            json!({
                "Eğim": "* 10 9; 10 20 7; 20 35 4; 35 60 2; 60 * kısıt",
                "Yola uzaklık": "* 50 9; 50 150 6; 150 300 3; 300 * 1",
                "Arazi örtüsü": "1 2; 2 6; 3 9; 4 kısıt; 5 kısıt",
            }),
        ),
        ("output", scratch_file("cakistirma.tif")),
    ]
}

/// İkili karşılaştırma of the three criteria: the slope three times the road's distance, five times the land cover.
pub(crate) fn pairwise() -> Vec<(&'static str, Value)> {
    vec![
        ("input", json!({ "scope": "selection" })),
        (
            "comparisons",
            json!([
                ["Eğim", "Yola uzaklık", 3],
                ["Eğim", "Arazi örtüsü", 5],
                ["Yola uzaklık", "Arazi örtüsü", 2]
            ]),
        ),
        ("write", json!(false)),
    ]
}

/// ROC of the slope against the landslides: every cell the background.
pub(crate) fn roc() -> Vec<(&'static str, Value)> {
    vec![("input", layer("egim")), ("presence", layer("heyelan"))]
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The CBS ribbon's Raster with Uygunluk analizi, over the criteria.
        ("uyg-serit", opened, view),
        // Bulanık üyelik's window: the slope falling from 5 % (1) to 30 % (0).
        (
            "uyg-uyelik",
            |app| {
                opened(app);
                window(
                    app,
                    "processing.run.suitability.fuzzyMembership",
                    &[
                        ("input", layer("egim")),
                        ("low", json!(30)),
                        ("high", json!(5)),
                    ],
                );
            },
            view,
        ),
        // Ağırlıklı çakıştırma's window: the influences and the class tables.
        (
            "uyg-cakistirma",
            |app| {
                opened(app);
                window(
                    app,
                    "processing.run.suitability.weightedOverlay",
                    &overlay(),
                );
            },
            view_end,
        ),
        // The overlay over the valley: 9 the most suitable, the lake, the village and the steep sides restricted.
        (
            "uyg-cakistirma-cizim",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.suitability.weightedOverlay",
                    &overlay(),
                    true,
                );
            },
            view,
        ),
        // İkili karşılaştırma's weights and consistency after its run.
        (
            "uyg-ahp",
            |app| {
                opened(app);
                ran(
                    app,
                    "processing.run.suitability.pairwise",
                    &pairwise(),
                    false,
                );
            },
            view_end,
        ),
        // ROC's curve of the slope against the landslides after its run.
        (
            "uyg-roc",
            |app| {
                opened(app);
                ran(app, "processing.run.suitability.roc", &roc(), false);
            },
            view_end,
        ),
    ]
}
