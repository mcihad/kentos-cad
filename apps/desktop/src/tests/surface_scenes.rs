//! Yüzey analizi's pictures (docs/adr/0231 §10) over the shared valley
//! (fixtures/interaction/v1/rasters.kcad, its elevation model 480 × 324
//! cells of 1.6 m): a CBS project's Raster tab with its Yüzey analizi panel,
//! Eğim's, Renkli kabartma's and Güneşlenme's windows, and each tool's result
//! drawn over the valley, the photograph's layer hidden. The web's are
//! `shots.mjs surface`. `tools_screens` takes them in the dark and the light
//! theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=yuzey-serit,yuzey-egim,yuzey-egim-cizim,yuzey-esyukselti-cizim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! KENTOS_SHOTS_ONLY=yuzey-alan cargo test -p kentos-desktop tools_screens -- --ignored --nocapture   # Kapsam: the area's window and result (docs/adr/0535)
//! ```
//!
//! Each result is written into a scratch folder. Test code only.

use kentos_ui::snapshot::Snapshot;
use serde_json::json;

use crate::app::{App, Message};
use crate::processing::Event;
use crate::processing::surface_tests::{open_on_dem, open_valley, run};
use crate::raster_scenes::tiles_made;
use crate::tools_screens::Pointed;

/// The valley open in `app`, the photograph and the scanned sheet hidden, the CBS ribbon's Raster on.
fn opened(app: &mut App) {
    open_valley(app);
    app.tab = "raster";
    app.command_expanded = false;
    if let Some(doc) = app.document.as_mut() {
        doc.model.set_layer_visible("orto", false);
        doc.model.set_layer_visible("tarama", false);
    }
}

/// The view on the elevation model, then its tiles made. The area has its size only once the drawing is
/// open and drawn (the start screen has none); fitted before, the drawing's own view would follow it.
fn framed(s: &mut Snapshot, app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: 487_200.0,
            min_y: 4_420_081.6,
            max_x: 487_968.0,
            max_y: 4_420_600.0,
        },
        24.0,
    );
    tiles_made(s, app);
}

fn value(app: &mut App, name: &str, v: serde_json::Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

/// `tool` run on the elevation model with `values`, its result in a scratch folder, its window closed.
fn ran(app: &mut App, tool: &'static str, values: &[(&str, serde_json::Value)]) {
    opened(app);
    let dir = crate::files_testing::scratch("yuzey-resim");
    open_on_dem(app, tool, &dir.join("sonuc.tif"));
    for (name, v) in values {
        value(app, name, v.clone());
    }
    let status = run(app);
    assert!(
        matches!(status, crate::processing::RunStatus::Ok { .. }),
        "{tool}: {status:?}"
    );
    let _ = app.update(Message::Processing(Event::Close));
}

/// A parcel over the valley's middle drawn and chosen: Kapsam's Alan nesneleri (docs/adr/0535).
pub(crate) fn area_chosen(app: &mut App) {
    let doc = app.document.as_mut().expect("open");
    let layer = doc.model.layers().active().to_owned();
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.polygon.create",
        kentos_commands::CallOp::Execute,
        json!({ "layerId": layer, "pts": [
            { "x": 487_450.0, "y": 4_420_250.0 }, { "x": 487_750.0, "y": 4_420_250.0 },
            { "x": 487_750.0, "y": 4_420_450.0 }, { "x": 487_450.0, "y": 4_420_450.0 }
        ] }),
    )
    .expect("runs");
    let uid: kentos_domain::Uuid = answer["output"]["uid"]
        .as_str()
        .and_then(|u| u.parse().ok())
        .expect("the parcel");
    let slot = doc.model.slot_of(uid).expect("its slot");
    app.selection.set(vec![slot]);
}

/// Eğim's window with Kapsam nesnelerin kutusu over the chosen parcel.
fn on_area(app: &mut App) {
    value(app, "area", json!("objects"));
    value(app, "areaObjects", json!({ "scope": "selection" }));
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The CBS ribbon's Raster: the rasters' panel and Yüzey analizi's eight tools.
        ("yuzey-serit", opened, framed),
        // Eğim's window over the valley: the raster, the method, the unit, the output and its layer.
        (
            "yuzey-egim",
            |app| {
                opened(app);
                open_on_dem(
                    app,
                    "processing.run.surface.slope",
                    std::path::Path::new(""),
                );
            },
            framed,
        ),
        (
            "yuzey-egim-cizim",
            |app| ran(app, "processing.run.surface.slope", &[]),
            framed,
        ),
        // Kapsam (docs/adr/0535): Eğim's window on the parcel's box, then its result, only the box's cells.
        (
            "yuzey-alan",
            |app| {
                opened(app);
                area_chosen(app);
                open_on_dem(
                    app,
                    "processing.run.surface.slope",
                    std::path::Path::new(""),
                );
                on_area(app);
            },
            framed,
        ),
        (
            "yuzey-alan-cizim",
            |app| {
                opened(app);
                area_chosen(app);
                let dir = crate::files_testing::scratch("yuzey-alan-resim");
                open_on_dem(app, "processing.run.surface.slope", &dir.join("alan.tif"));
                on_area(app);
                let status = run(app);
                assert!(
                    matches!(status, crate::processing::RunStatus::Ok { .. }),
                    "{status:?}"
                );
                let _ = app.update(Message::Processing(Event::Close));
            },
            framed,
        ),
        (
            "yuzey-baki-cizim",
            |app| ran(app, "processing.run.surface.aspect", &[]),
            framed,
        ),
        (
            "yuzey-golge-cizim",
            |app| ran(app, "processing.run.surface.hillshade", &[]),
            framed,
        ),
        // Renkli kabartma's window with a colour table.
        (
            "yuzey-renkli",
            |app| {
                opened(app);
                open_on_dem(
                    app,
                    "processing.run.surface.colorRelief",
                    std::path::Path::new(""),
                );
                value(app, "colors", json!("table"));
                value(
                    app,
                    "table",
                    json!("1010 #2E7D32; 1040 #9CCC65; 1070 #FFF59D; 1100 #A1887F; 1130 #FAFAFA"),
                );
            },
            framed,
        ),
        (
            "yuzey-renkli-cizim",
            |app| ran(app, "processing.run.surface.colorRelief", &[]),
            framed,
        ),
        (
            "yuzey-egrilik-cizim",
            |app| {
                ran(
                    app,
                    "processing.run.surface.curvature",
                    &[("curvature", json!("profile"))],
                )
            },
            framed,
        ),
        (
            "yuzey-puruzluluk-cizim",
            |app| ran(app, "processing.run.surface.ruggedness", &[]),
            framed,
        ),
        // Güneşlenme's window: a date range, its days and months.
        (
            "yuzey-gunes",
            |app| {
                opened(app);
                open_on_dem(
                    app,
                    "processing.run.surface.insolation",
                    std::path::Path::new(""),
                );
                value(app, "period", json!("range"));
            },
            framed,
        ),
        (
            "yuzey-gunes-cizim",
            |app| ran(app, "processing.run.surface.insolation", &[]),
            framed,
        ),
        // The contours over the elevation model: 2 m, every fifth an Ana eğri.
        (
            "yuzey-esyukselti-cizim",
            |app| {
                ran(
                    app,
                    "processing.run.surface.contours",
                    &[("interval", json!(2))],
                );
            },
            framed,
        ),
    ]
}
