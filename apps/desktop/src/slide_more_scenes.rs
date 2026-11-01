//! The pictures of docs/adr/0807 (XOX and two unequal sliding leaves): the `.kcad` sample's drawing
//! (`fixtures/kcad/v2/slide-more.json`): X3 three leaves whose outer two slide over a fixed middle (XOX), the window
//! P3 the same on its rails, S2 two leaves meeting at 60 % of the light; X3 near at 1:20 with its arrows; Kapı with
//! Açılış Sürme, three leaves, Ortası sabit (XOX) under the cursor and its settings; Kapı with two leaves and
//! Birinci kanadın payı; S2's Öznitelikler (Birinci kanadın payı, Kanatlar); Kapı ve pencere tipleri over X3.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=surme-xox-plan,surme-xox-yakin,surme-xox-arac,surme-pay-arac,surme-pay-oznitelikler,surme-xox-tipler cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
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

const MODEL: &str = include_str!("../../../fixtures/kcad/v2/slide-more.json");

fn open(app: &mut App, b: [f64; 4]) {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_str(MODEL).expect("the sample");
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

fn set(app: &mut App, key: &'static str, value: &str) {
    let _ = app.update(Message::ToolSetting(Commit {
        tool: "door",
        key,
        value: value.into(),
    }));
}

/// X3, P3 and S2 along the wall.
fn plan(app: &mut App) {
    open(app, [0.3, -1.4, 11.7, 1.4]);
}

/// X3 near: the middle leaf fixed on the inner track, the outer two on the outer track with their arrows inward.
fn near(app: &mut App) {
    open(app, [0.5, -0.8, 3.5, 0.8]);
}

fn door_tool(app: &mut App) {
    open(app, [0.3, -2.0, 11.7, 1.5]);
    app.command_expanded = true;
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    run(app, "tool.door");
    set(app, "Ç", "slide");
}

fn hover(app: &mut App, x: f64) {
    let [x, y] = app.viewport.camera.world_to_screen(Vec2::new(x, -0.1));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Moved(Point::new(x as f32, y as f32))));
}

/// Kapı, Sürme, three leaves, Ortası sabit (XOX), on rails, between X3 and P3.
fn tool(app: &mut App) {
    door_tool(app);
    set(app, "E", "1.4");
    set(app, "K", "3");
    set(app, "İ", "on");
    set(app, "H", "on");
    hover(app, 4.0);
}

/// Kapı, Sürme, two leaves, the first 70 % of the light.
fn share_tool(app: &mut App) {
    door_tool(app);
    set(app, "E", "1.4");
    set(app, "K", "2");
    set(app, "Ş", "%70");
    hover(app, 8.0);
}

/// S2 selected: Öznitelikler's Kanat bindirmesi, Birinci kanadın payı and Kanatlar.
fn props(app: &mut App) {
    plan(app);
    app.selection.set([Slot(4)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Kapı ve pencere tipleri over X3: three leaves, İki yana.
fn types(app: &mut App) {
    plan(app);
    run(app, "arch.openingTypes");
    if let Some(i) = app
        .opening_types
        .as_ref()
        .and_then(|w| w.uses.iter().position(|u| u.kind.id == "x3"))
    {
        let _ = app.update(Message::OpeningTypes(crate::opening_types_window::Event::Choose(i)));
    }
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("surme-xox-plan", plan),
        ("surme-xox-yakin", near),
        ("surme-xox-arac", tool),
        ("surme-pay-arac", share_tool),
        ("surme-pay-oznitelikler", props),
        ("surme-xox-tipler", types),
    ]
}
