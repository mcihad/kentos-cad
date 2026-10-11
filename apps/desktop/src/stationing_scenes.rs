//! The pictures of docs/adr/0189 (Km yaz) in a CAD project at 1:500, the
//! shared trace's ground (`stationing.kcad`): a road axis with a straight
//! and a quarter-turn arc, a line and a point. Km yaz on the axis, its
//! stations faint; every 10 m with cross-sections and points; and what it
//! wrote. The web's are `shots.mjs stationing`. `tools_screens` takes them
//! in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=km-yaz,km-yaz-enkesit,km-yaz-yazildi,km-yaz-olcu cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! `km-yaz-olcu` is Km yaz on a measured route (docs/adr/0224 §6), the
//! shared trace's ground `stationing-measure.kcad`: Ölçü on, the km its
//! measures.
//!
//! Test code only.

use kentos_contracts::DocumentSnapshotV1;

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// The axis (1), the line (2) and the point (3).
fn ground() -> Objects {
    let mut o = Objects::new();
    let quarter = (std::f64::consts::PI / 8.0).tan();
    o.bulged(
        "yol",
        &[[-40.0, -10.0], [-20.0, -10.0], [0.0, -10.0]],
        &[0.0, quarter],
    );
    o.line("cizim", [-40.0, 10.0], [-20.0, 10.0]);
    o.point("cizim", [20.0, 10.0], None);
    o
}

/// The drawing in a CAD project at 1:500, Km yaz on the axis.
fn shown(app: &mut App) {
    open(app, ground());
    make_cad(app);
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.plot_scale = 500.0;
        doc.model.set_settings(settings);
    }
    app.tab = "annotate";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 46.0,
            min_y: N - 30.0,
            max_x: E + 26.0,
            max_y: N + 18.0,
        },
        24.0,
    );
    run(app, "tool.stationLabels");
    click(app, [-30.0, -10.0]);
    hover(app, [12.0, 12.0]);
}

/// Every 10 m, with 8 m cross-sections and points 4 m to the right.
fn sections(app: &mut App) {
    shown(app);
    for t in ["A", "10", "E", "8", "N", "4"] {
        typed(app, t);
    }
    hover(app, [12.0, 12.0]);
}

/// What it wrote: the ticks, the texts square to the axis, the sections and points.
fn written(app: &mut App) {
    sections(app);
    let _ = app.update(crate::app::Message::Run("tool.confirm"));
    hover(app, [12.0, 12.0]);
}

/// The measured route of the shared trace (M 1005, 1050, 1062) and a line without measures.
const MEASURED: &str = include_str!("../../../fixtures/interaction/v1/stationing-measure.kcad");

/// Km yaz on the measured route: Ölçü on, its stations at the measures' multiples and ends, faint.
fn by_measure(app: &mut App) {
    let snapshot = DocumentSnapshotV1::from_json(MEASURED).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.tab = "annotate";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 38.0,
            min_y: N - 20.0,
            max_x: E + 22.0,
            max_y: N + 16.0,
        },
        24.0,
    );
    run(app, "tool.stationLabels");
    click(app, [-20.0, -10.0]);
    hover(app, [0.0, 12.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("km-yaz", shown),
        ("km-yaz-enkesit", sections),
        ("km-yaz-yazildi", written),
        ("km-yaz-olcu", by_measure),
    ]
}
