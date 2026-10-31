//! The pictures of docs/adr/0792 (three- and four-leaf sliding doors and windows): the `.kcad` sample's drawing
//! (`fixtures/kcad/v2/multi-slide.json`): S3 three leaves on three tracks stacking at its start (OXX), S4 four leaves
//! parting to both jambs on its rails (OXXO), the window P3 three leaves on two tracks (OXO); S4 near at 1:20 with its
//! arrows; Kapı with Açılış Sürme, four leaves, İki yana under the cursor (the command's plan) and its settings; S4's
//! Öznitelikler (Ray sayısı, İki yana açılır, Kanat bindirmesi, Kanat düzeni); Kapı ve pencere tipleri over S3.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=surme-cok-plan,surme-cok-yakin,surme-cok-arac,surme-cok-oznitelikler,surme-cok-tipler cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use iced::Point;
use kentos_domain::Slot;
use kentos_render_wgpu::Vec2;
use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::architecture_settings::Commit;
use crate::files_testing::make_cad;
use crate::tools_scenes::run;
use crate::tools_screens::Scene;

const MODEL: &str = include_str!("../../../fixtures/kcad/v2/multi-slide.json");
const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.openings.create.special.json");

fn open_doc(app: &mut App, snapshot: kentos_commands::DocumentSnapshotV2, b: [f64; 4]) {
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(app);
    app.draft.snap = false;
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

fn open(app: &mut App, b: [f64; 4]) {
    open_doc(app, serde_json::from_str(MODEL).expect("the sample"), b);
}

fn set(app: &mut App, key: &'static str, value: &str) {
    let _ = app.update(Message::ToolSetting(Commit {
        tool: "door",
        key,
        value: value.into(),
    }));
}

/// S3, S4 and P3 along the wall.
fn plan(app: &mut App) {
    open(app, [0.3, -1.6, 11.7, 1.6]);
}

/// S4 near: four leaves on two tracks, the two in the middle parting to both jambs, their arrows and rails.
fn near(app: &mut App) {
    open(app, [4.2, -0.9, 7.8, 0.9]);
}

/// Kapı, Açılış Sürme, four leaves parting both ways on rails, 3,2 m, by the first wall of the shared cases.
fn tool(app: &mut App) {
    let file: serde_json::Value = serde_json::from_str(CASES).expect("the cases");
    open_doc(app, serde_json::from_value(file["setup"].clone()).expect("the drawing"), [2.0, -2.0, 8.0, 1.5]);
    app.command_expanded = true;
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    run(app, "tool.door");
    set(app, "Ç", "slide");
    set(app, "E", "3.2");
    set(app, "K", "4");
    set(app, "İ", "on");
    set(app, "H", "on");
    let [x, y] = app.viewport.camera.world_to_screen(Vec2::new(5.0, -0.9));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Moved(Point::new(x as f32, y as f32))));
}

/// S4 selected: Öznitelikler's Ray sayısı, İki yana açılır, Kanat bindirmesi and Kanat düzeni.
fn props(app: &mut App) {
    plan(app);
    app.selection.set([Slot(3)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Kapı ve pencere tipleri over S3: its operation, leaves and tracks.
fn types(app: &mut App) {
    plan(app);
    run(app, "arch.openingTypes");
    if let Some(i) = app
        .opening_types
        .as_ref()
        .and_then(|w| w.uses.iter().position(|u| u.kind.id == "s3"))
    {
        let _ = app.update(Message::OpeningTypes(crate::opening_types_window::Event::Choose(i)));
    }
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("surme-cok-plan", plan),
        ("surme-cok-yakin", near),
        ("surme-cok-arac", tool),
        ("surme-cok-oznitelikler", props),
        ("surme-cok-tipler", types),
    ]
}
