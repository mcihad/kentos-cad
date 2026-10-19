//! The rasters of docs/adr/0204 in a GIS project at 1:1000, the shared
//! traces' ground (`rasters.kcad`, written by `scripts/fixtures/raster_scene.py`):
//! a valley's elevation model as a ramp lit by its relief, its orthophoto
//! (JPEG in a tiled GeoTIFF with overviews) over it with parcels along the
//! road, and a scanned sheet's corner beside them. The web's are `shots.mjs
//! rasters`. `tools_screens` takes them in the dark and the light theme at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=raster-vadi,raster-orto,raster-dem,raster-ekle,raster-stili,raster-oturt,raster-oturt-ince-plaka,raster-oturt-kayit cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::path::PathBuf;

use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::document::Document;
use crate::tools_screens::Pointed;

const DRAWING: &str = include_str!("../../../fixtures/interaction/v1/rasters.kcad");
const X0: f64 = 487_200.0;
const Y0: f64 = 4_420_600.0;

/// The drawing, beside its rasters' folder, the view fitted to `bounds`.
fn opened(app: &mut App, bounds: [f64; 4]) {
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interaction/v1/rasters.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.tab = "data";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: bounds[0],
            min_y: bounds[1],
            max_x: bounds[2],
            max_y: bounds[3],
        },
        24.0,
    );
}

/// The rasters' folder of the shared drawing.
fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/interaction/v1/rasters")
}

/// Runs a command and every task it starts, as the window would.
fn run(app: &mut App, id: &'static str) {
    let task = app.run(id);
    crate::files_testing::drive(app, task);
}

/// Raster ekle with `file` chosen: the window over the valley.
fn adding(app: &mut App, file: &str) {
    opened(app, [X0 - 360.0, Y0 - 540.0, X0 + 800.0, Y0 + 20.0]);
    app.picker = crate::app::Picker::File(folder().join(file));
    run(app, "raster.add");
}

/// The mosaic's photographs and elevation models (fixtures/raster/v1/mosaic, `raster_mosaic_cases.py --files`).
const ORTO: &[&str] = &["orto-2019.tif", "orto-2021.tif", "orto-2023.tif"];
const DEMS: &[&str] = &["dem-a.tif", "dem-b.tif", "dem-c.tif", "dem-d.tif", "dem-e.tif"];

/// Sanal mozaik over the valley's drawing, its own rasters hidden, with `files` chosen.
fn mosaic_window(app: &mut App, files: &[&str], order: crate::rasters::mosaic::Order) {
    opened(app, [X0 - 60.0, Y0 - 560.0, X0 + 760.0, Y0 + 40.0]);
    if let Some(doc) = app.document.as_mut() {
        for layer in ["dem", "orto", "tarama"] {
            doc.model.set_layer_visible(layer, false);
        }
    }
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/raster/v1/mosaic");
    app.raster_mosaic_with(files.iter().map(|f| folder.join(f)).collect(), order);
}

/// Oluştur, the VRT written to a folder of the pictures' (never beside the fixtures).
fn mosaic_run(app: &mut App, name: &str) {
    let out = std::env::temp_dir().join(format!("kentos-mozaik-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&out);
    let send = |app: &mut App, e: crate::rasters::mosaic::Event| {
        let task = app.update(Message::Rasters(crate::rasters::Event::Mosaic(e)));
        crate::files_testing::drive(app, task);
    };
    send(app, crate::rasters::mosaic::Event::PathPicked(Some(out.join(name))));
    send(app, crate::rasters::mosaic::Event::Run);
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

/// Raster oturt's points on the scanned sheet (Kullan, Denetim, Ad, Sütun, Satır, Hedef Y, Hedef X): four corners
/// fitted to, two check points (docs/adr/0518).
const FIT_ROWS: [[&str; 7]; 6] = [
    ["1", "0", "P1", "12.5", "14.0", "486882.6", "4420546.1"],
    ["1", "0", "P2", "287.0", "11.5", "487157.2", "4420548.4"],
    ["1", "0", "P3", "283.5", "248.0", "487153.9", "4420312.0"],
    ["1", "0", "P4", "9.0", "251.5", "486879.0", "4420308.6"],
    ["1", "1", "D1", "148.0", "130.0", "487018.3", "4420429.6"],
    ["1", "1", "D2", "60.0", "200.0", "486929.9", "4420359.8"],
];

/// The rows into Raster oturt's window, solved.
fn fit_rows(app: &mut App, rows: &[[&str; 7]]) {
    let form = &mut app.calc.raster_fit;
    form.rows = rows
        .iter()
        .map(|r| {
            let mut row: crate::calc::raster_fit::Row = Default::default();
            for (k, v) in r.iter().enumerate() {
                row[k] = (*v).to_owned();
            }
            row
        })
        .collect();
    let doc = app.document.as_ref().expect("a drawing");
    app.calc.raster_fit.solve(&doc.model);
}

/// Draws until the raster service has made every tile the view asked for.
pub(crate) fn tiles_made(s: &mut Snapshot, app: &mut App) {
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    // The tiles are asked for by a frame drawn and made off the thread: draw, let them come, draw again.
    for _ in 0..6 {
        s.settle(app, App::view, &mut update);
        let _ = s.render(app.view(), &app.theme());
        std::thread::sleep(std::time::Duration::from_millis(20));
        crate::rasters::tiles::service().settle(std::time::Duration::from_secs(20));
    }
    s.settle(app, App::view, &mut update);
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The whole valley: the relief, the photograph over it, the scanned sheet beside.
        (
            "raster-vadi",
            |app| opened(app, [X0 - 360.0, Y0 - 540.0, X0 + 800.0, Y0 + 20.0]),
            tiles_made,
        ),
        // The photograph close up, with its parcels.
        (
            "raster-orto",
            |app| opened(app, [X0 + 100.0, Y0 - 300.0, X0 + 460.0, Y0 - 80.0]),
            tiles_made,
        ),
        // Raster ekle: the elevation model, in the project's system.
        ("raster-ekle", |app| adding(app, "dem.tif"), tiles_made),
        // Raster ekle from an address (docs/adr/0207 §1): the photograph, a tiled GeoTIFF with its
        // overviews, served by ranges from this computer; its header read, the address kept.
        (
            "raster-ekle-adres",
            |app| {
                adding(app, "dem.tif");
                let server = crate::range_server::serve(
                    vec![(
                        "orto.tif",
                        std::fs::read(folder().join("orto.tif")).expect("the photograph"),
                    )],
                    crate::range_server::Mode::Ranges,
                );
                let add = |app: &mut App, e: crate::rasters::add::Event| {
                    let task = app.update(Message::Rasters(crate::rasters::Event::Add(e)));
                    crate::files_testing::drive(app, task);
                };
                add(
                    app,
                    crate::rasters::add::Event::From(crate::rasters::add::From::Address),
                );
                add(
                    app,
                    crate::rasters::add::Event::Address(server.url("orto.tif")),
                );
                add(app, crate::rasters::add::Event::ReadAddress);
            },
            tiles_made,
        ),
        // Raster ekle: the scanned sheet, its world file says no system: the user's yes is asked.
        (
            "raster-ekle-tarama",
            |app| adding(app, "tarama.png"),
            tiles_made,
        ),
        // Raster stili over the elevation model: a ramp lit by its relief, the bands' statistics.
        (
            "raster-stili",
            |app| {
                opened(app, [X0 - 20.0, Y0 - 540.0, X0 + 790.0, Y0 + 20.0]);
                if let Some(doc) = app.document.as_mut() {
                    doc.model.set_layer_visible("orto", false);
                }
                select(app, "dem");
                run(app, "raster.style");
            },
            tiles_made,
        ),
        // Raster oturt over the scanned sheet: four points, the affine solved, their residuals.
        (
            "raster-oturt",
            |app| {
                opened(app, [X0 - 360.0, Y0 - 540.0, X0 + 800.0, Y0 + 20.0]);
                select(app, "tarama");
                run(app, "raster.georef");
                fit_rows(app, &FIT_ROWS);
            },
            tiles_made,
        ),
        // Raster oturt by the thin plate: the check points' errors, the output grid the command plans (docs/adr/0518).
        (
            "raster-oturt-ince-plaka",
            |app| {
                opened(app, [X0 - 360.0, Y0 - 540.0, X0 + 800.0, Y0 + 20.0]);
                select(app, "tarama");
                run(app, "raster.georef");
                app.calc.raster_fit.method = kentos_contracts::GeorefMethod::ThinPlate;
                fit_rows(app, &FIT_ROWS);
            },
            tiles_made,
        ),
        // The affine applied, the window opened again: the rows from the raster's record, its line (docs/adr/0518).
        (
            "raster-oturt-kayit",
            |app| {
                opened(app, [X0 - 360.0, Y0 - 540.0, X0 + 800.0, Y0 + 20.0]);
                select(app, "tarama");
                run(app, "raster.georef");
                fit_rows(app, &FIT_ROWS);
                let task = app.update(crate::calc::raster_fit::raster_fit_event(
                    crate::calc::raster_fit::Event::Apply,
                ));
                crate::files_testing::drive(app, task);
                app.calc.raster_fit.rows = vec![Default::default(); 4];
                select(app, "tarama");
                run(app, "raster.georef");
            },
            tiles_made,
        ),
        // Sanal mozaik (docs/adr/0517 §5): three years' photographs, the newest on top; the plan's size and order.
        ("mozaik-pencere", |app| mosaic_window(app, ORTO, crate::rasters::mosaic::Order::Newest), tiles_made),
        // The mosaic written and added as a raster on its own layer: the seams where the newer year lies over.
        (
            "mozaik-cizim",
            |app| {
                mosaic_window(app, ORTO, crate::rasters::mosaic::Order::Newest);
                mosaic_run(app, "orto.vrt");
            },
            tiles_made,
        ),
        // Elevation models of 10, 7.5 and 5 m, one undated: the warnings of the plan.
        ("mozaik-dem", |app| mosaic_window(app, DEMS, crate::rasters::mosaic::Order::Newest), tiles_made),
        // The elevation mosaic drawn: the 5 m model's hole shows the one below, no seam.
        (
            "mozaik-dem-cizim",
            |app| {
                mosaic_window(app, DEMS, crate::rasters::mosaic::Order::Finest);
                mosaic_run(app, "dem.vrt");
            },
            tiles_made,
        ),
        // The elevation model alone: the photograph's layer hidden.
        (
            "raster-dem",
            |app| {
                opened(app, [X0 - 20.0, Y0 - 540.0, X0 + 790.0, Y0 + 20.0]);
                if let Some(doc) = app.document.as_mut() {
                    doc.model.set_layer_visible("orto", false);
                }
            },
            tiles_made,
        ),
    ]
}
