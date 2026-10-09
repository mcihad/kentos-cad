//! Raster işlemleri and Raster istatistiği's pictures (docs/adr/0233) over
//! the shared valley (fixtures/interaction/v1/rasters.kcad: its elevation
//! model 480 × 324 cells of 1.6 m, the photograph 1280 × 864 of 0.6 m, five
//! parcels): the CBS ribbon's Raster tab with the two panels, Raster
//! hesaplayıcı's window naming the rasters, its index of the photograph's
//! bands drawn, Yeniden sınıflandır's classes, the elevation model cut by
//! the parcels, a neighbourhood's range, a coarser grid, Bölgesel
//! istatistik's and Histogram's tables. The web's are `shots.mjs
//! rasterops`. `tools_screens` takes them in the dark and the light theme at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=ops-serit,ops-hesap,ops-hesap-cizim,ops-bolge cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Each result is written into a scratch folder. Test code only.

use kentos_ui::snapshot::Snapshot;
use serde_json::json;

use crate::app::{App, Message};
use crate::processing::Event;
use crate::processing::surface_tests::{open_valley, run};
use crate::raster_scenes::tiles_made;
use crate::tools_screens::Pointed;

/// The valley open in `app`, the scanned sheet hidden (and the photograph unless `photo`), the CBS ribbon's Raster on.
fn opened(app: &mut App, photo: bool) {
    open_valley(app);
    app.tab = "raster";
    app.command_expanded = false;
    if let Some(doc) = app.document.as_mut() {
        doc.model.set_layer_visible("orto", photo);
        doc.model.set_layer_visible("tarama", false);
    }
}

/// The view on the elevation model, then the rasters' tiles made.
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

/// The view on the parcels (the cut elevation model), then the rasters' tiles made.
fn on_parcels(s: &mut Snapshot, app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: 487_310.0,
            min_y: 4_420_360.0,
            max_x: 487_642.0,
            max_y: 4_420_462.0,
        },
        24.0,
    );
    tiles_made(s, app);
}

/// The view on the elevation model, the window's form scrolled to its end: the run's table.
fn table_shown(s: &mut Snapshot, app: &mut App) {
    framed(s, app);
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    // Over the form at both sizes (the window is centred, its form on the left).
    s.step(
        app,
        App::view,
        &mut update,
        &[
            iced::Event::Mouse(iced::mouse::Event::CursorMoved {
                position: iced::Point::new(400.0, 400.0),
            }),
            iced::Event::Mouse(iced::mouse::Event::WheelScrolled {
                delta: iced::mouse::ScrollDelta::Lines { x: 0.0, y: -40.0 },
            }),
        ],
    );
    s.settle(app, App::view, &mut update);
}

fn value(app: &mut App, name: &str, v: serde_json::Value) {
    let _ = app.update(Message::Processing(Event::Value(name.into(), v)));
}

/// `tool`'s window with `values` (the input first), its result to go to a scratch folder.
fn window(app: &mut App, tool: &'static str, photo: bool, values: &[(&str, serde_json::Value)]) {
    opened(app, photo);
    let dir = crate::files_testing::scratch("ops-resim");
    let _ = app.update(Message::Run(tool));
    value(
        app,
        "output",
        json!(dir.join("sonuc.tif").to_string_lossy()),
    );
    for (name, v) in values {
        value(app, name, v.clone());
    }
}

/// `tool` run with `values` (the photograph shown while it runs if `photo`); its window closed unless `keep`,
/// the layers `hide` hidden.
fn ran(
    app: &mut App,
    (tool, photo): (&'static str, bool),
    values: &[(&str, serde_json::Value)],
    keep: bool,
    hide: &[&str],
) {
    window(app, tool, photo, values);
    let status = run(app);
    assert!(
        matches!(status, crate::processing::RunStatus::Ok { .. }),
        "{tool}: {status:?}"
    );
    if !keep {
        let _ = app.update(Message::Processing(Event::Close));
    }
    if let Some(doc) = app.document.as_mut() {
        for id in hide {
            doc.model.set_layer_visible(id, false);
        }
    }
}

fn on_dem() -> (&'static str, serde_json::Value) {
    ("input", json!({ "scope": "layer", "layerId": "dem" }))
}

const INDEX: &str = "([Ortofoto@1] - [Ortofoto@2]) / ([Ortofoto@1] + [Ortofoto@2])";

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // The CBS ribbon's Raster: the rasters, Yüzey analizi, İnterpolasyon, Yoğunluk, Raster işlemleri and istatistiği.
        ("ops-serit", |app| opened(app, false), framed),
        // Raster hesaplayıcı's window: the visible rasters, their names as chips, an index of the photograph's bands.
        (
            "ops-hesap",
            |app| {
                window(
                    app,
                    "processing.run.raster.calculator",
                    true,
                    &[("expression", json!(INDEX))],
                )
            },
            framed,
        ),
        // The index drawn: only the photograph read, its grid; the parcels over it.
        (
            "ops-hesap-cizim",
            |app| {
                ran(
                    app,
                    ("processing.run.raster.calculator", true),
                    &[
                        ("input", json!({ "scope": "all" })),
                        ("expression", json!(INDEX)),
                    ],
                    false,
                    &["dem", "orto"],
                )
            },
            framed,
        ),
        // The elevations in four classes.
        (
            "ops-sinif-cizim",
            |app| {
                ran(
                    app,
                    ("processing.run.raster.reclassify", false),
                    &[
                        on_dem(),
                        ("table", json!("* 910 1; 910 930 2; 930 955 3; 955 * 4")),
                    ],
                    false,
                    &["dem"],
                )
            },
            framed,
        ),
        // The elevation model cut by the parcels, to their box: the view on them.
        (
            "ops-kirp-cizim",
            |app| {
                ran(
                    app,
                    ("processing.run.raster.clipByMask", false),
                    &[
                        on_dem(),
                        ("mask", json!({ "scope": "layer", "layerId": "parsel" })),
                    ],
                    false,
                    &["dem"],
                )
            },
            on_parcels,
        ),
        // A 9 × 9 neighbourhood's range: the steep slopes bright.
        (
            "ops-komsuluk-cizim",
            |app| {
                ran(
                    app,
                    ("processing.run.raster.focalStatistics", false),
                    &[
                        on_dem(),
                        ("width", json!(9)),
                        ("height", json!(9)),
                        ("stat", json!("range")),
                    ],
                    false,
                    &["dem"],
                )
            },
            framed,
        ),
        // A grid of 16 m, each cell the mean of the elevations it covers.
        (
            "ops-ornek-cizim",
            |app| {
                ran(
                    app,
                    ("processing.run.raster.resample", false),
                    &[on_dem(), ("cell", json!(16)), ("method", json!("mean"))],
                    false,
                    &["dem"],
                )
            },
            framed,
        ),
        // Bölgesel istatistik over the parcels: their mean elevation written, the table under the form.
        (
            "ops-bolge",
            |app| {
                ran(
                    app,
                    ("processing.run.raster.zonalStatistics", false),
                    &[
                        on_dem(),
                        ("zones", json!({ "scope": "layer", "layerId": "parsel" })),
                        ("output", json!("Ortalama kot")),
                    ],
                    true,
                    &[],
                )
            },
            table_shown,
        ),
        // The elevations' histogram in 16 intervals.
        (
            "ops-histogram",
            |app| {
                ran(
                    app,
                    ("processing.run.raster.histogram", false),
                    &[on_dem(), ("bins", json!(16))],
                    true,
                    &[],
                )
            },
            table_shown,
        ),
    ]
}
