//! Uzaktan algılama's pictures (docs/adr/0242) over a synthetic satellite
//! image of the shared valley: four bands (blue, green, red, near infrared)
//! made from its land cover and its slopes' light, the same two years on
//! (new houses north of the road, a felled stand), its coarse multispectral
//! and fine panchromatic pair, training areas and reference points
//! (fixtures/interaction/v1/remote.kcad, scripts/fixtures/remote_scene.py):
//! the CBS ribbon's Raster with its panel, Spektral indis' window and its
//! NDVI, Denetimli sınıflandırma's window and its classes, Doğruluk
//! analizi's matrix, the near infrared's change and the pansharpened image.
//! The web's are `shots.mjs remote`, at the same places with the same
//! values. `tools_screens` takes them in the dark and the light theme at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=ua-serit,ua-indis,ua-indis-cizim,ua-denetimli,ua-denetimli-cizim,ua-dogruluk,ua-degisim-cizim,ua-birlestirme-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
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

pub(crate) const REMOTE: &str = include_str!("../../../fixtures/interaction/v1/remote.kcad");

const VALLEY: [f64; 4] = [487_200.0, 4_420_081.6, 487_968.0, 4_420_600.0];
/// Fields in the valley's north, their furrows finer than the multispectral image's cells.
const FIELDS: [f64; 4] = [487_640.0, 4_420_500.0, 487_760.0, 4_420_580.0];

/// The valley's image with its pair, the training areas and the reference points open in `app`.
pub(crate) fn open_remote(app: &mut App) {
    let snapshot = DocumentSnapshotV1::from_json(REMOTE).expect("the drawing reads");
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/interaction/v1/remote.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
}

fn opened(app: &mut App) {
    open_remote(app);
    app.tab = "raster";
    app.command_expanded = false;
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

/// The view on the valley, then the rasters' tiles made.
fn view(s: &mut Snapshot, app: &mut App) {
    fit(app, VALLEY);
    tiles_made(s, app);
}

/// The view on the northern fields.
fn view_fields(s: &mut Snapshot, app: &mut App) {
    fit(app, FIELDS);
    tiles_made(s, app);
}

/// [`view`], the window's form scrolled to its end: the run's table under it, its first column in sight.
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

/// A raster result's file in a scratch folder.
fn scratch_file(name: &str) -> Value {
    json!(
        crate::files_testing::scratch("ua-resim")
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

/// Spektral indis: NDVI of the 2024 image.
pub(crate) fn ndvi() -> Vec<(&'static str, Value)> {
    vec![
        ("input", layer("goruntu")),
        ("index", json!("ndvi")),
        ("output", scratch_file("ndvi.tif")),
    ]
}

/// Denetimli sınıflandırma of the 2024 image by its training areas, the largest likelihood.
pub(crate) fn supervised() -> Vec<(&'static str, Value)> {
    vec![
        ("input", layer("goruntu")),
        ("training", layer("egitim")),
        ("classField", json!("Sınıf")),
        ("output", scratch_file("siniflar.tif")),
    ]
}

/// Doğruluk analizi of the classes against the reference points.
pub(crate) fn accuracy() -> Vec<(&'static str, Value)> {
    vec![
        ("input", layer("islem-denetimli-siniflandirma")),
        ("reference", layer("referans")),
        ("referenceField", json!("Sınıf")),
    ]
}

/// Değişim tespiti of the near infrared from 2024 to 2026.
pub(crate) fn change() -> Vec<(&'static str, Value)> {
    vec![
        ("input", layer("goruntu")),
        ("after", layer("sonraki")),
        ("band", json!(4)),
        ("output", scratch_file("degisim.tif")),
    ]
}

/// Görüntü birleştirme: the 3.2 m bands on the 1.6 m panchromatic, Brovey.
pub(crate) fn pansharpen() -> Vec<(&'static str, Value)> {
    vec![
        ("input", layer("cok-bantli")),
        ("pan", layer("pankromatik")),
        ("output", scratch_file("birlesim.tif")),
    ]
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The CBS ribbon's Raster with Uzaktan algılama, over the valley's image.
        ("ua-serit", opened, view),
        // Spektral indis' window: NDVI from the red and near infrared bands.
        (
            "ua-indis",
            |app| {
                opened(app);
                window(app, "processing.run.remote.index", &ndvi());
            },
            view,
        ),
        // The NDVI over the valley: the forest and the green crops high, the lake and the houses low.
        (
            "ua-indis-cizim",
            |app| {
                opened(app);
                ran(app, "processing.run.remote.index", &ndvi(), true);
            },
            view,
        ),
        // Denetimli sınıflandırma's window over the training areas.
        (
            "ua-denetimli",
            |app| {
                opened(app);
                window(app, "processing.run.remote.supervised", &supervised());
            },
            view,
        ),
        // The five classes over the valley.
        (
            "ua-denetimli-cizim",
            |app| {
                opened(app);
                ran(app, "processing.run.remote.supervised", &supervised(), true);
            },
            view,
        ),
        // Doğruluk analizi's matrix of the classes against the reference points.
        (
            "ua-dogruluk",
            |app| {
                opened(app);
                ran(app, "processing.run.remote.supervised", &supervised(), true);
                ran(app, "processing.run.remote.accuracy", &accuracy(), false);
            },
            view_end,
        ),
        // The near infrared's difference, 2024 to 2026: the houses and the felled stand.
        (
            "ua-degisim-cizim",
            |app| {
                opened(app);
                ran(app, "processing.run.remote.change", &change(), true);
            },
            view,
        ),
        // The pansharpened image over the northern fields: their furrows sharp.
        (
            "ua-birlestirme-cizim",
            |app| {
                opened(app);
                ran(app, "processing.run.remote.pansharpen", &pansharpen(), true);
            },
            view_fields,
        ),
    ]
}
