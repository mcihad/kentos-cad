//! The pictures of docs/adr/0799 (a room's interior elevation, a derived drawing): the shared cases' drawing
//! (`fixtures/commands/v1/cad.elevations.edit.json` `setup`); Oda iç görünüşü with the room picked and its plan drawn
//! at the pointer with its settings and what it leaves out; the four sides written beside the plan; the elevations near;
//! the door moved, a side's frame selected and Öznitelikler saying the drawing is stale; and after Yeniden üret, current.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=ic-gorunus-arac,ic-gorunus-cizildi,ic-gorunus-yakin,ic-gorunus-eski,ic-gorunus-guncel cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_ui::widget::docking;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, click, hover, run};
use crate::tools_screens::Scene;

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.elevations.edit.json");
/// The first object the elevation writes: side A's frame.
const FIRST: u32 = 25;

fn open(app: &mut App, bounds: [f64; 4]) {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(file["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(app);
    app.draft.snap = false;
    fit(app, bounds);
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

/// Oda iç görünüşü: the room picked, its four sides drawn from the plan at the pointer, the tag saying what is left out.
fn tool(app: &mut App) {
    open(app, [-1.0, -2.0, 28.0, 6.0]);
    run(app, "tool.roomElevation");
    click(app, [2.0 - E, 2.0 - N]);
    hover(app, [7.5 - E, 0.0 - N]);
}

fn derived(app: &mut App) {
    tool(app);
    click(app, [7.5 - E, 0.0 - N]);
    run(app, "tool.select");
}

/// The plan and its four sides, the elevation's layer in İç görünüşler.
fn written(app: &mut App) {
    derived(app);
    fit(app, [-1.0, -2.0, 28.0, 6.0]);
}

/// The sides near: frames, the door, the windows (the one without a head dashed), the niche, titles and dimensions.
fn near(app: &mut App) {
    derived(app);
    fit(app, [7.0, -1.5, 27.0, 3.5]);
}

fn select_first(app: &mut App) {
    app.selection.set([Slot(FIRST)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "geometry",
    )));
}

/// The door moved 1.5 m along its wall: side A's frame selected, Öznitelikler says Eski.
fn stale(app: &mut App) {
    derived(app);
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let door: kentos_commands::OpeningsEdit = serde_json::from_value(json!({
        "uids": ["0192f5a1-7990-7000-8000-000000000006"], "operation": {"kind": "move", "at": 3.0}
    }))
    .expect("input");
    let _ = kentos_native_application::openings_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        door,
    );
    fit(app, [-1.0, -2.0, 28.0, 6.0]);
    select_first(app);
}

/// Yeniden üret from Öznitelikler: the door drawn where it is, the same objects, Güncel.
fn current(app: &mut App) {
    stale(app);
    let _ = app.update(Message::Properties(crate::properties::Event::Elevation(
        kentos_commands::ElevationsOperation::Refresh {
            layer: "ic-gorunus-1".into(),
        },
    )));
    select_first(app);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("ic-gorunus-arac", tool),
        ("ic-gorunus-cizildi", written),
        ("ic-gorunus-yakin", near),
        ("ic-gorunus-eski", stale),
        ("ic-gorunus-guncel", current),
    ]
}
