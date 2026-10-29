//! The split area of docs/adr/0576 in pictures: each view with a time of its own (2015 and 2024 side by side), the
//! scale link with a scale chip per view, the other view as the drawing some undo steps back (what the drawing
//! changed since marked), and as a drawing file. `tools_screens` takes them in the dark and the light theme:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=bolunmus-zaman,bolunmus-bag-merkez,bolunmus-bag-olcek,bolunmus-revizyon-gecmis,bolunmus-revizyon-dosya cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! and up to four views (docs/adr/0637): three side by side with a revision at its own time, four in two rows, a
//! selection shown in a revision as the same records:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=bolunmus-uc,bolunmus-dort,bolunmus-secim-revizyon cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! and each view's own walls' detail (docs/adr/0696's 29 October addendum): the room at 1:100 far, near and close in
//! three views, schematic, standard and detailed; the close one pressed, the active one, each as it was:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=bolunmus-duvar-ayrintisi,bolunmus-duvar-ayrintisi-etkin cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use iced::Point;
use kentos_geometry_core::tools::view_link::Link;
use kentos_native_application::{ExecutionContext, registry};

use crate::app::{App, Message};
use crate::document::Document;
use crate::split_view::{Event, Source, TimeAsk};
use crate::tools_screens::Scene;
use crate::viewport::Event as ViewEvent;

fn run(app: &mut App, id: &'static str) {
    let task = app.update(Message::Run(id));
    crate::files_testing::drive(app, task);
}

/// The temporal drawing (parcels split in 2018, one ended in 2021, a new one in 2022) split in two views.
fn split(app: &mut App) {
    crate::temporal_scenes::opened(app);
    run(app, "view.split");
}

fn command(app: &mut App, name: &str, input: serde_json::Value) {
    let doc = app.document.as_mut().expect("open");
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
}

fn uid(app: &App, slot: u32) -> String {
    app.document
        .as_ref()
        .expect("open")
        .model
        .uid(kentos_domain::Slot(slot))
        .expect("an object")
        .to_string()
}

/// The door-and-window room at 1:100, its walls' detail by the view (Proje ayarları › Mimari: Otomatik), in three
/// views: the active one far (1:150, schematic: the bodies filled), the next near on the inner wall's tee (1:50,
/// standard: the brick hatched), the last close on the bottom left door (1:15, detailed: the plaster, its frame).
fn three_details(app: &mut App) {
    use crate::tools_scenes::{E, N};
    crate::opening_scenes::room(app, 100.0);
    run(app, "view.split");
    run(app, "view.splitAdd");
    let centre = |x: f64, y: f64| kentos_render_wgpu::Vec2::new(E + x, N + y);
    app.viewport.camera.zoom_to_screen_scale(150.0);
    app.viewport.camera.center_on(centre(3.5, 2.5));
    if let Some(split) = app.split.as_mut() {
        for (i, scale, at) in [(0, 50.0, centre(4.2, 0.6)), (1, 15.0, centre(1.6, 0.3))] {
            let camera = &mut split.others[i].viewport.camera;
            camera.zoom_to_screen_scale(scale);
            camera.center_on(at);
        }
    }
    let _ = app.update(Message::DialogClosed);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("bolunmus-duvar-ayrintisi", three_details),
        // The close view pressed: the active one now, the store at its detail; the far and near ones as they were.
        ("bolunmus-duvar-ayrintisi-etkin", |app| {
            three_details(app);
            app.activate_other(1);
            let _ = app.update(Message::DialogClosed);
        }),
        // 2015 in the other view, the slider's 2024 in the active one: parcel 101 whole on the right, split on the left.
        ("bolunmus-zaman", |app| {
            split(app);
            run(app, "time.slider");
            let _ = app.update(Message::Split(Event::Time {
                view: Some(0),
                ask: TimeAsk::Own,
            }));
            for _ in 0..9 {
                let _ = app.update(Message::Split(Event::Time {
                    view: Some(0),
                    ask: TimeAsk::Step(-1),
                }));
            }
        }),
        // The link on the centre alone: the other view at three times the scale keeps it while the centre follows.
        ("bolunmus-bag-merkez", |app| {
            split(app);
            let _ = app.update(Message::OtherViewport(
                0,
                ViewEvent::Zoomed {
                    factor: 3.0,
                    at: Point::new(150.0, 300.0),
                },
            ));
            let _ = app.update(Message::Split(Event::Link(Link::Centre)));
            app.viewport.camera.zoom_at(1.2, 200.0, 300.0);
            let _ = app.update(Message::DialogClosed);
        }),
        // The link on the centre and the scale: both chips say the same scale.
        ("bolunmus-bag-olcek", |app| {
            split(app);
            let _ = app.update(Message::OtherViewport(
                0,
                ViewEvent::Zoomed {
                    factor: 3.0,
                    at: Point::new(150.0, 300.0),
                },
            ));
            let _ = app.update(Message::Split(Event::Link(Link::CentreScale)));
        }),
        // The drawing two undo steps back in the other view: a parcel moved and another deleted since, both marked.
        ("bolunmus-revizyon-gecmis", |app| {
            split(app);
            let (moved, gone) = (uid(app, 2), uid(app, 4));
            command(
                app,
                "cad.entities.transform",
                serde_json::json!({ "uids": [moved], "transform": { "kind": "move", "dx": 14.0, "dy": 6.0 } }),
            );
            command(
                app,
                "cad.entities.delete",
                serde_json::json!({ "uids": [gone] }),
            );
            let _ = app.update(Message::Split(Event::Source(0, Source::History(2))));
        }),
        // A drawing file in the other view: a parcel moved, one the file lacks, a line the drawing lacks.
        ("bolunmus-revizyon-dosya", |app| {
            let path = |name: &str| {
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../fixtures/compare/v1/revisions")
                    .join(name)
            };
            let open = Document::read(&path("ayni.kcad")).expect("reads");
            let _ = app.update(Message::Opened(Some(Ok(Box::new(open)))));
            app.tab = "map";
            run(app, "view.zoomExtents");
            run(app, "view.split");
            app.revision_from_file(0, &path("tasindi.kcad"));
            let _ = app.update(Message::Run("view.zoomExtents"));
        }),
        // Three views (docs/adr/0637 §1, §2): the active at 2024, the drawing at 2015, two steps back at 2012.
        ("bolunmus-uc", |app| {
            split(app);
            let (moved, gone) = (uid(app, 2), uid(app, 4));
            command(
                app,
                "cad.entities.transform",
                serde_json::json!({ "uids": [moved], "transform": { "kind": "move", "dx": 14.0, "dy": 6.0 } }),
            );
            command(
                app,
                "cad.entities.delete",
                serde_json::json!({ "uids": [gone] }),
            );
            run(app, "view.splitAdd");
            run(app, "time.slider");
            let _ = app.update(Message::Split(Event::Source(1, Source::History(2))));
            for (view, years) in [(0, 9), (1, 12)] {
                let _ = app.update(Message::Split(Event::Time {
                    view: Some(view),
                    ask: TimeAsk::Own,
                }));
                for _ in 0..years {
                    let _ = app.update(Message::Split(Event::Time {
                        view: Some(view),
                        ask: TimeAsk::Step(-1),
                    }));
                }
            }
        }),
        // Four views in two rows, linked on the centre: each its own scale and theme.
        ("bolunmus-dort", |app| {
            split(app);
            run(app, "view.splitAdd");
            run(app, "view.splitAdd");
            for (i, f) in [(0, 2.0), (1, 0.6), (2, 4.0)] {
                let _ = app.update(Message::OtherViewport(
                    i,
                    ViewEvent::Zoomed {
                        factor: f,
                        at: Point::new(150.0, 150.0),
                    },
                ));
            }
            // Linked last: each view comes to the active one's centre and keeps its own scale.
            let _ = app.update(Message::Split(Event::Link(Link::Centre)));
        }),
        // A parcel selected in the open drawing: the revision two steps back shows the same record, in the accent.
        ("bolunmus-secim-revizyon", |app| {
            split(app);
            let moved = uid(app, 2);
            command(
                app,
                "cad.entities.transform",
                serde_json::json!({ "uids": [moved], "transform": { "kind": "move", "dx": 14.0, "dy": 6.0 } }),
            );
            let _ = app.update(Message::Split(Event::Source(0, Source::History(1))));
            app.selection.set([kentos_domain::Slot(2)]);
            let _ = app.update(Message::DialogClosed);
        }),
    ]
}
