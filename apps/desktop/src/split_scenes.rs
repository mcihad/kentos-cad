//! The split area of docs/adr/0576 in pictures: each view with a time of its own (2015 and 2024 side by side), the
//! scale link with a scale chip per view, the other view as the drawing some undo steps back (what the drawing
//! changed since marked), and as a drawing file. `tools_screens` takes them in the dark and the light theme:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=bolunmus-zaman,bolunmus-bag-merkez,bolunmus-bag-olcek,bolunmus-revizyon-gecmis,bolunmus-revizyon-dosya cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
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

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        // 2015 in the other view, the slider's 2024 in the active one: parcel 101 whole on the right, split on the left.
        ("bolunmus-zaman", |app| {
            split(app);
            run(app, "time.slider");
            let _ = app.update(Message::Split(Event::Time {
                active: false,
                ask: TimeAsk::Own,
            }));
            for _ in 0..9 {
                let _ = app.update(Message::Split(Event::Time {
                    active: false,
                    ask: TimeAsk::Step(-1),
                }));
            }
        }),
        // The link on the centre alone: the other view at three times the scale keeps it while the centre follows.
        ("bolunmus-bag-merkez", |app| {
            split(app);
            let _ = app.update(Message::OtherViewport(ViewEvent::Zoomed {
                factor: 3.0,
                at: Point::new(150.0, 300.0),
            }));
            let _ = app.update(Message::Split(Event::Link(Link::Centre)));
            app.viewport.camera.zoom_at(1.2, 200.0, 300.0);
            let _ = app.update(Message::DialogClosed);
        }),
        // The link on the centre and the scale: both chips say the same scale.
        ("bolunmus-bag-olcek", |app| {
            split(app);
            let _ = app.update(Message::OtherViewport(ViewEvent::Zoomed {
                factor: 3.0,
                at: Point::new(150.0, 300.0),
            }));
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
            let _ = app.update(Message::Split(Event::Source(Source::History(2))));
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
            app.revision_from_file(&path("tasindi.kcad"));
            let _ = app.update(Message::Run("view.zoomExtents"));
        }),
    ]
}
