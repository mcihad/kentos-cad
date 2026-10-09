//! Raster ve vektör and Taranmış harita's pictures (docs/adr/0234):
//! Rasterleştir, Rasterden alan and Rasterden nokta over the shared valley
//! (fixtures/interaction/v1/rasters.kcad); Rasterden çizgi, Çizgi yakala,
//! Alan kapat and Eğrilere kot ver over a scanned topographic sheet
//! (fixtures/interaction/v1/scanned.kcad, scripts/fixtures/scanned_scene.py:
//! brown contours, a blue stream, a road and six parcels in black). The
//! web's are `shots.mjs rastervector`, at the same places with the same
//! values. `tools_screens` takes them in the dark and the light theme at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=vek-serit,vek-yakala-cizim,vek-kapat-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
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
use crate::processing::surface_tests::{open_valley, run};
use crate::raster_scenes::tiles_made;
use crate::tools_screens::Pointed;

const SCANNED: &str = include_str!("../../../fixtures/interaction/v1/scanned.kcad");

const VALLEY: [f64; 4] = [487_200.0, 4_420_081.6, 487_968.0, 4_420_600.0];
const PARCELS: [f64; 4] = [487_310.0, 4_420_360.0, 487_642.0, 4_420_462.0];
const SHEET: [f64; 4] = [486_900.0, 4_420_850.0, 487_500.0, 4_421_300.0];
const HILL: [f64; 4] = [487_120.0, 4_421_040.0, 487_290.0, 4_421_160.0];

/// The valley (the scanned sheet and the photograph hidden) or the scanned sheet open in `app`, the CBS ribbon's Raster on.
fn opened(app: &mut App, scanned: bool) {
    if scanned {
        let snapshot = DocumentSnapshotV1::from_json(SCANNED).expect("the drawing reads");
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/interaction/v1/scanned.kcad");
        let doc = Document::new(snapshot, Some(path)).expect("opens");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    } else {
        open_valley(app);
        if let Some(doc) = app.document.as_mut() {
            doc.model.set_layer_visible("orto", false);
            doc.model.set_layer_visible("tarama", false);
        }
    }
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
        crate::files_testing::scratch("vek-resim")
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

fn hide(app: &mut App, layers: &[&str]) {
    if let Some(doc) = app.document.as_mut() {
        for id in layers {
            doc.model.set_layer_visible(id, false);
        }
    }
}

fn layer(id: &str) -> Value {
    json!({ "scope": "layer", "layerId": id })
}

/// On the second ring round the hill top, clicked two pixels short of it.
fn on_ring() -> Value {
    json!({ "x": 487_212.25, "y": 4_421_124.75 })
}

/// Rasterden çizgi over the sheet's brown.
fn contours(app: &mut App) {
    ran(
        app,
        "processing.run.raster.toLines",
        &[
            ("input", layer("pafta")),
            ("select", json!("color")),
            ("color", json!("#9C5C2C")),
            ("tolerance", json!(60)),
            ("spur", json!(5)),
            ("simplify", json!(1)),
        ],
    );
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The CBS ribbon's Raster with Raster ve vektör and Taranmış harita.
        (
            "vek-serit",
            |app| opened(app, false),
            |s, app| view_on(s, app, VALLEY),
        ),
        // The parcels burnt by their numbers onto 2 m cells, under them.
        (
            "vek-rasterlestir-cizim",
            |app| {
                opened(app, false);
                ran(
                    app,
                    "processing.run.raster.rasterize",
                    &[
                        ("input", layer("parsel")),
                        ("valueFrom", json!("field")),
                        ("field", json!("Parsel")),
                        ("cellSize", json!(2)),
                        ("output", scratch_file("parseller.tif")),
                    ],
                );
                hide(app, &["dem"]);
            },
            |s, app| view_on(s, app, PARCELS),
        ),
        // The elevations in five classes, then their regions as areas over them.
        (
            "vek-alan-cizim",
            |app| {
                opened(app, false);
                ran(
                    app,
                    "processing.run.raster.reclassify",
                    &[
                        ("input", layer("dem")),
                        (
                            "table",
                            json!("* 905 1; 905 920 2; 920 935 3; 935 950 4; 950 * 5"),
                        ),
                        ("sample", json!("u8")),
                        ("output", scratch_file("siniflar.tif")),
                    ],
                );
                ran(
                    app,
                    "processing.run.raster.toPolygons",
                    &[("input", layer("islem-siniflar"))],
                );
                hide(app, &["dem"]);
            },
            |s, app| view_on(s, app, VALLEY),
        ),
        // The elevation model's peaks and pits within twelve cells.
        (
            "vek-nokta-cizim",
            |app| {
                opened(app, false);
                ran(
                    app,
                    "processing.run.raster.toPoints",
                    &[
                        ("input", layer("dem")),
                        ("mode", json!("extrema")),
                        ("radius", json!(12)),
                    ],
                );
            },
            |s, app| view_on(s, app, VALLEY),
        ),
        // Çizgi yakala's window, the point picked beside a contour.
        (
            "vek-yakala",
            |app| {
                opened(app, true);
                window(
                    app,
                    "processing.run.scan.captureLine",
                    &[
                        ("input", layer("pafta")),
                        ("at", on_ring()),
                        ("z", json!(1000)),
                    ],
                );
            },
            |s, app| view_on(s, app, HILL),
        ),
        // The contour captured, its elevation 1000.
        (
            "vek-yakala-cizim",
            |app| {
                opened(app, true);
                ran(
                    app,
                    "processing.run.scan.captureLine",
                    &[
                        ("input", layer("pafta")),
                        ("at", on_ring()),
                        ("z", json!(1000)),
                    ],
                );
            },
            |s, app| view_on(s, app, HILL),
        ),
        // The north-west parcel closed: the contours and the stream within 300 of the paper, the black lines not.
        (
            "vek-kapat-cizim",
            |app| {
                opened(app, true);
                ran(
                    app,
                    "processing.run.scan.closeArea",
                    &[
                        ("input", layer("pafta")),
                        ("at", json!({ "x": 487_262.0, "y": 4_421_025.0 })),
                        ("tolerance", json!(300)),
                    ],
                );
            },
            |s, app| view_on(s, app, SHEET),
        ),
        // Every brown line as polylines, the sheet hidden.
        (
            "vek-cizgi-cizim",
            |app| {
                opened(app, true);
                contours(app);
                hide(app, &["pafta"]);
            },
            |s, app| view_on(s, app, HILL),
        ),
        // Eğrilere kot ver's window over the contours: a cut from the valley floor up the hill.
        (
            "vek-kot",
            |app| {
                opened(app, true);
                contours(app);
                window(
                    app,
                    "processing.run.scan.contourElevations",
                    &[
                        ("curves", layer("islem-cizgiler")),
                        ("start", json!({ "x": 487_187.75, "y": 4_421_049.75 })),
                        ("end", json!({ "x": 487_187.75, "y": 4_421_123.75 })),
                        ("first", json!(980)),
                        ("step", json!(5)),
                    ],
                );
            },
            |s, app| view_on(s, app, HILL),
        ),
    ]
}
