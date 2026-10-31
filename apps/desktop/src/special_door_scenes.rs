//! The pictures of docs/adr/0777 (revolving, pocket and folding doors, the sliding door's track): the shared cases'
//! drawing (`fixtures/commands/v1/cad.openings.create.special.json` `setup`), a revolving door of the office type D01,
//! a pocket door CG1 with its pocket in the wall, a folding door KT1 and a two-leaf sliding door on its track; the
//! pocket and the folding door near at 1:20; Kapı with Açılış Cebe giren under the cursor (the command's plan) and its
//! settings; the revolving door's Öznitelikler (Açılış, Kanat, Tambur çapı); Kapı ve pencere tipleri over CG1 (its
//! lip); a pocket door the thin wall refuses. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=ozel-kapi-plan,ozel-kapi-yakin,ozel-kapi-arac,ozel-kapi-oznitelikler,ozel-kapi-tipler,ozel-kapi-ince cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use iced::Point;
use kentos_domain::Slot;
use kentos_render_wgpu::Vec2;
use kentos_ui::widget::docking;
use serde_json::Value;

use crate::app::{App, Message};
use crate::architecture_settings::Commit;
use crate::files_testing::make_cad;
use crate::tools_scenes::run;
use crate::tools_screens::Scene;

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.openings.create.special.json");

fn open(app: &mut App, bounds: [f64; 4]) {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(file["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(app);
    app.draft.snap = false;
    view(app, bounds);
}

fn view(app: &mut App, b: [f64; 4]) {
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

/// The pointer on `p` (the drawing's own coordinates).
fn hover(app: &mut App, p: [f64; 2]) {
    let [x, y] = app.viewport.camera.world_to_screen(Vec2::new(p[0], p[1]));
    let _ = app.update(Message::Viewport(crate::viewport::Event::Moved(
        Point::new(x as f32, y as f32),
    )));
}

fn set(app: &mut App, key: &'static str, value: &str) {
    let _ = app.update(Message::ToolSetting(Commit {
        tool: "door",
        key,
        value: value.into(),
    }));
}

/// The four special doors along the fifth wall: D01 revolving, CG1 pocket, KT1 folding, the sliding door on its track.
fn plan(app: &mut App) {
    open(app, [19.5, -2.2, 32.5, 2.2]);
}

/// The pocket door and the folding door near: the pocket's cavity in the wall, open at the jamb; the panels' zigzag.
fn near(app: &mut App) {
    open(app, [24.6, -1.0, 30.2, 1.4]);
}

/// Kapı, Açılış Cebe giren with a 3 cm lip, the cursor by the first wall: the pocket door the command would put there.
fn tool(app: &mut App) {
    open(app, [2.0, -2.0, 8.0, 1.5]);
    app.command_expanded = true;
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    run(app, "tool.door");
    set(app, "Ç", "pocket");
    set(app, "P", "3cm");
    hover(app, [5.0, -0.9]);
}

/// The revolving door selected: Öznitelikler's Açılış, Kanat and Tambur çapı.
fn props(app: &mut App) {
    plan(app);
    app.selection.set([Slot(6)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// Kapı ve pencere tipleri over CG1: its operation and its lip.
fn types(app: &mut App) {
    plan(app);
    run(app, "arch.openingTypes");
    if let Some(i) = app
        .opening_types
        .as_ref()
        .and_then(|w| w.uses.iter().position(|u| u.kind.id == "cg1"))
    {
        let _ = app.update(Message::OpeningTypes(
            crate::opening_types_window::Event::Choose(i),
        ));
    }
}

/// Kapı, Cebe giren, by the 10 cm wall: refused before the click, the reason beside the cursor.
fn thin(app: &mut App) {
    open(app, [-0.5, 3.0, 6.5, 7.0]);
    app.command_expanded = true;
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    run(app, "tool.door");
    set(app, "Ç", "pocket");
    hover(app, [3.0, 4.4]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("ozel-kapi-plan", plan),
        ("ozel-kapi-yakin", near),
        ("ozel-kapi-arac", tool),
        ("ozel-kapi-oznitelikler", props),
        ("ozel-kapi-tipler", types),
        ("ozel-kapi-ince", thin),
    ]
}
