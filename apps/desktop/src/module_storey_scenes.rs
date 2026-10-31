//! The pictures of docs/adr/0796 (room modules of several storeys and a level's loose objects repeated): the shared
//! cases' duplex (`fixtures/commands/v1/cad.modules.edit.storeys.json` `setup`) defined as one module of two storeys
//! and placed again in the basement; Oda modülleri saying it is of two storeys and each instance's levels; Katları
//! tekrarla with Modülsüz nesneler de, its plan saying the copies and what is left; the first floor after it, its
//! lower floor faint; Modül yerleştir on the top level, refused for its missing upper level.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=dubleks-pencere,dubleks-tekrar,dubleks-katlar,dubleks-yerlestir cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::room_modules_window::Event;
use crate::tools_scenes::{E, N, hover, run};
use crate::tools_screens::Scene;

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.modules.edit.storeys.json");

fn uid(n: u32) -> String {
    format!("0192f5a1-7960-7000-8000-{n:012x}")
}

fn open(app: &mut App, bounds: [f64; 4]) {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(file["setup"].clone()).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(app);
    app.draft.snap = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: bounds[0],
            min_y: bounds[1],
            max_x: bounds[2],
            max_y: bounds[3],
        },
        24.0,
    );
}

fn edit(app: &mut App, op: Value) {
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let input: kentos_commands::ModulesEdit =
        serde_json::from_value(json!({ "operation": op })).expect("input");
    let _ = kentos_native_application::modules_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        input,
    );
}

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::RoomModules(e));
}

/// The duplex defined and placed again in the basement, 30 m right.
fn two(app: &mut App) {
    open(app, [-2.0, -6.0, 36.0, 5.0]);
    edit(
        app,
        json!({"kind": "define", "name": "Dubleks", "uids": (1..=11).map(uid).collect::<Vec<_>>(),
               "base": {"x": 0, "y": 0}}),
    );
    edit(
        app,
        json!({"kind": "place", "module": "Dubleks", "at": {"x": 30, "y": 0}, "level": "bodrum",
               "numbering": {"prefix": "B-"}}),
    );
}

/// Oda modülleri: the duplex of two storeys, its instances on Zemin kat–1. kat and Bodrum–Zemin kat.
fn window(app: &mut App) {
    two(app);
    run(app, "arch.modules");
    event(app, Event::Instance(1));
}

/// Katları tekrarla from the ground floor to the first with Modülsüz nesneler de: the plan before it is written.
fn repeat(app: &mut App) {
    two(app);
    run(app, "arch.modules");
    // The project lists its levels 1. kat, zemin, 2. kat, bodrum: the ground floor is the second.
    event(app, Event::RepeatFrom(1));
    let k1 = app
        .room_modules
        .as_ref()
        .and_then(|w| w.repeat_to.iter().position(|t| t.level == "kat1"))
        .expect("the first floor");
    event(app, Event::RepeatChoose(k1, true));
    event(app, Event::RepeatPrefix(k1, "A1-".into()));
    event(app, Event::RepeatLoose(true));
}

/// After the repeat: the first floor active, the ground floor faint below it.
fn floors(app: &mut App) {
    repeat(app);
    event(app, Event::Repeat);
    event(app, Event::Close);
    app.selection.set(Vec::<Slot>::new());
    let _ = app.update(Message::ActiveLevel(Some("kat1".into())));
    let _ = app.update(Message::Run("view.levels.below"));
}

/// Modül yerleştir on 2. kat, the top level: the duplex's upper storey finds no level, the tool says so.
fn place(app: &mut App) {
    two(app);
    run(app, "tool.modulePlace");
    let _ = app.update(Message::ToolSetting(crate::architecture_settings::Commit {
        tool: "modulePlace",
        key: "L",
        value: "kat2".into(),
    }));
    hover(app, [16.0 - E, 0.0 - N]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("dubleks-pencere", window),
        ("dubleks-tekrar", repeat),
        ("dubleks-katlar", floors),
        ("dubleks-yerlestir", place),
    ]
}
