//! The point clouds of docs/adr/0207 in a GIS project at 1:500, the shared
//! traces' ground (`pointclouds.kcad`, written by
//! `scripts/fixtures/pointcloud_scene.py`): a village's airborne LiDAR (a LAZ
//! indexed once into the cache) under its parcels: in its colours, by class,
//! by height; Nokta bulutu ekle, Nokta bulutu stili and XYZ sor. The web's
//! are `shots.mjs pointclouds`. `tools_screens` takes them in the dark and
//! the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=bulut-koy,bulut-siniflar,bulut-yukseklik,bulut-ekle,bulut-stili,bulut-xyz cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::path::PathBuf;

use iced::Point;
use kentos_contracts::{CloudRender, Entity};
use kentos_geometry_core::vec2::Vec2;
use kentos_ui::snapshot::Snapshot;

use crate::app::{App, Message};
use crate::document::Document;
use crate::tools_screens::Pointed;
use crate::viewport::Event;

const DRAWING: &str = include_str!("../../../fixtures/interaction/v1/pointclouds.kcad");
pub(crate) const X0: f64 = 487_600.0;
pub(crate) const Y0: f64 = 4_420_300.0;

/// The index cache of the pictures and tests: beside the build, not the user's.
pub(crate) fn test_cache() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/pointcloud-cache");
    *crate::pointclouds::index::TEST_FOLDER
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(dir);
}

/// The drawing, beside its clouds' folder, the view fitted to `bounds` (from the village's lower left).
pub(crate) fn opened(app: &mut App, bounds: [f64; 4]) {
    test_cache();
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/interaction/v1/pointclouds.kcad");
    let doc = Document::new(snapshot, Some(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.tab = "data";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: X0 + bounds[0],
            min_y: Y0 + bounds[1],
            max_x: X0 + bounds[2],
            max_y: Y0 + bounds[3],
        },
        24.0,
    );
}

/// The clouds' folder of the shared drawing.
pub(crate) fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/interaction/v1/pointclouds")
}

fn run(app: &mut App, id: &'static str) {
    let task = app.run(id);
    crate::files_testing::drive(app, task);
}

/// The cloud selected.
fn select(app: &mut App) {
    let doc = app.document.as_ref().expect("a drawing");
    let slot = doc
        .model
        .entities()
        .find(|e| matches!(e, Entity::PointCloud(_)))
        .map(|e| kentos_domain::Slot(e.base().id))
        .expect("the cloud");
    app.selection.set(vec![slot]);
}

/// The cloud's look set to `render`, as Nokta bulutu stili writes it.
fn look(app: &mut App, render: CloudRender) {
    let doc = app.document.as_mut().expect("a drawing");
    let (slot, mut c) = doc
        .model
        .entities()
        .find_map(|e| match e {
            Entity::PointCloud(c) => Some((kentos_domain::Slot(c.base.id), c.clone())),
            _ => None,
        })
        .expect("the cloud");
    c.cloud.style.render = render;
    if render == CloudRender::Elevation {
        c.cloud.style.ramp = Some("Spektral".to_owned());
    }
    let _ =
        kentos_interaction::properties::set_geometry(&mut doc.model, slot, &Entity::PointCloud(c));
    app.spatial.sync(&doc.model);
}

/// Draws until the cloud service has opened the files and made every node the view asked for.
pub(crate) fn clouds_made(s: &mut Snapshot, app: &mut App) {
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    // The nodes are asked for by a frame drawn and made off the thread: draw, let them come, draw again.
    for _ in 0..8 {
        s.settle(app, App::view, &mut update);
        let _ = s.render(app.view(), &app.theme());
        std::thread::sleep(std::time::Duration::from_millis(20));
        crate::pointclouds::service::service().settle(std::time::Duration::from_secs(30));
    }
    s.settle(app, App::view, &mut update);
}

/// Where a point (east and north from the village's lower left) is on the drawing area.
fn at(app: &App, p: [f64; 2]) -> Point {
    let [x, y] = app
        .viewport
        .camera
        .world_to_screen(Vec2::new(X0 + p[0], Y0 + p[1]));
    Point::new(x as f32, y as f32)
}

/// XYZ sor's click on a roof, its answer read and the point marked.
fn queried(s: &mut Snapshot, app: &mut App) {
    clouds_made(s, app);
    let point = at(app, [44.0, 70.5]);
    // Every message's tasks run, as the window's runtime runs them: the press asks the query.
    for e in [
        Event::Moved(point),
        Event::Pressed(point),
        Event::Released(point),
    ] {
        let task = app.update(Message::Viewport(e));
        crate::files_testing::drive(app, task);
    }
    clouds_made(s, app);
}

/// Zemin süzgeci's window over the village, its result written into a scratch folder.
fn ground_window(app: &mut App) {
    opened(app, [-6.0, -4.0, 126.0, 94.0]);
    let _ = app.update(Message::Run("processing.run.pointcloud.ground"));
    let _ = app.update(Message::Processing(crate::processing::Event::Value(
        "input".into(),
        serde_json::json!({ "scope": "layer", "layerId": "koy" }),
    )));
}

/// Zemin süzgeci run: its result by class on its own layer, the source hidden.
fn ground_run(s: &mut Snapshot, app: &mut App) {
    let dir = crate::files_testing::scratch("bulut-zemin");
    let _ = app.update(Message::Processing(crate::processing::Event::Value(
        "output".into(),
        serde_json::json!(dir.join("koy-zemin.laz").to_string_lossy()),
    )));
    let task = app.update(Message::Processing(crate::processing::Event::Run));
    crate::files_testing::drive(app, task);
    if let Some(doc) = app.document.as_mut() {
        doc.model.set_layer_visible("koy", false);
    }
    let _ = app.update(Message::Processing(crate::processing::Event::Close));
    clouds_made(s, app);
}

pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        // İşlemler's Zemin süzgeci over the village.
        ("bulut-zemin-penceresi", ground_window, clouds_made),
        // Its result: ground (brown), vegetation (greens), roofs (red), road (grey), noise.
        ("bulut-zemin-sonucu", ground_window, ground_run),
        // The village in its colours, the parcels over it.
        (
            "bulut-koy",
            |app| opened(app, [-6.0, -4.0, 126.0, 94.0]),
            clouds_made,
        ),
        // By class: ground, road, roofs, low to high trees, noise.
        (
            "bulut-siniflar",
            |app| {
                opened(app, [-6.0, -4.0, 126.0, 94.0]);
                look(app, CloudRender::Classification);
            },
            clouds_made,
        ),
        // By height through a ramp, close up on the houses north of the road.
        (
            "bulut-yukseklik",
            |app| {
                opened(app, [8.0, 50.0, 116.0, 92.0]);
                look(app, CloudRender::Elevation);
            },
            clouds_made,
        ),
        // Nokta bulutu ekle: the village's LAZ, in the project's system; its index to be made.
        (
            "bulut-ekle",
            |app| {
                opened(app, [-6.0, -4.0, 126.0, 94.0]);
                app.picker = crate::app::Picker::File(folder().join("koy.laz"));
                run(app, "pointcloud.add");
            },
            clouds_made,
        ),
        // Nokta bulutu ekle as the command opens it, its file dialog closed: a file or an address.
        (
            "bulut-ekle-bos",
            |app| {
                opened(app, [-6.0, -4.0, 126.0, 94.0]);
                app.cloud_add_open_for_tests();
            },
            clouds_made,
        ),
        // Nokta bulutu ekle from an address (docs/adr/0207 §1): the village's LAZ served by ranges
        // from this computer, its header and a sample read.
        (
            "bulut-ekle-adres",
            |app| {
                opened(app, [-6.0, -4.0, 126.0, 94.0]);
                let server = crate::range_server::serve(
                    vec![(
                        "koy.laz",
                        std::fs::read(folder().join("koy.laz")).expect("the village"),
                    )],
                    crate::range_server::Mode::Ranges,
                );
                app.cloud_add_open_for_tests();
                let add = |app: &mut App, e: crate::pointclouds::add::Event| {
                    let task = app.update(Message::PointClouds(crate::pointclouds::Event::Add(e)));
                    crate::files_testing::drive(app, task);
                };
                add(
                    app,
                    crate::pointclouds::add::Event::From(crate::pointclouds::add::From::Address),
                );
                add(
                    app,
                    crate::pointclouds::add::Event::Address(server.url("koy.laz")),
                );
                add(app, crate::pointclouds::add::Event::ReadAddress);
            },
            clouds_made,
        ),
        // Nokta bulutu stili over the cloud: by height, the classes, the size.
        (
            "bulut-stili",
            |app| {
                opened(app, [-6.0, -4.0, 126.0, 94.0]);
                select(app);
                run(app, "pointcloud.style");
            },
            clouds_made,
        ),
        // XYZ sor on a roof: its facts on the command line, the point marked.
        (
            "bulut-xyz",
            |app| {
                opened(app, [20.0, 52.0, 70.0, 86.0]);
                app.command_expanded = true;
                run(app, "pointcloud.query");
            },
            queried,
        ),
    ]
}
