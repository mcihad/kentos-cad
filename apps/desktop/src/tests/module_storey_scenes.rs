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
//! And docs/adr/0810: the ground floor's loose objects already copied to the 2nd floor, one copy deleted; Katları
//! tekrarla again plans to copy that one alone, the others' like already there:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=dubleks-yinelenen cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! And docs/adr/0811: Modül yerleştir on 2. kat with Tek kat (the duplex's upper storey alone); Oda modülleri with that
//! instance placed, saying it holds the module's 2nd storey only:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=dubleks-tek-kat,dubleks-tek-kat-pencere cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
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

const CASES: &str = include_str!("../../../../fixtures/commands/v1/cad.modules.edit.storeys.json");

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
    // Katları tekrarla is a part of its own (docs/adr/8050 §6).
    event(app, Event::Part(crate::room_modules_window::Part::Repeat));
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

/// The ground floor's loose objects copied to the 2nd floor (no module), the fixture's copy deleted: the plan of
/// Katları tekrarla again, the others' like already there (docs/adr/0810).
fn again(app: &mut App) {
    open(app, [-2.0, -6.0, 36.0, 5.0]);
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let input: kentos_commands::ModulesEdit = serde_json::from_value(json!({"operation": {
        "kind": "repeat", "from": "zemin", "to": [{"level": "kat2"}], "loose": true}}))
    .expect("input");
    let kentos_commands::CommandResult::Completed { output, .. } =
        kentos_native_application::modules_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(doc),
            input,
        )
    else {
        panic!("the first repeat")
    };
    let fixture = output
        .copied
        .iter()
        .find(|c| c.source == uid(18))
        .expect("the fixture's copy")
        .uid
        .clone();
    let _ = kentos_native_application::delete::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        serde_json::from_value(json!({"uids": [fixture]})).expect("input"),
    );
    run(app, "arch.modules");
    event(app, Event::RepeatFrom(1));
    let k2 = app
        .room_modules
        .as_ref()
        .and_then(|w| w.repeat_to.iter().position(|t| t.level == "kat2"))
        .expect("the 2nd floor");
    event(app, Event::RepeatChoose(k2, true));
    event(app, Event::RepeatLoose(true));
}

/// Modül yerleştir on 2. kat, Tek kat: the duplex's upper storey alone, seven objects (docs/adr/0811).
fn single(app: &mut App) {
    two(app);
    run(app, "tool.modulePlace");
    for (key, value) in [("L", "kat2"), ("K", "1")] {
        let _ = app.update(Message::ToolSetting(crate::architecture_settings::Commit {
            tool: "modulePlace",
            key,
            value: value.into(),
        }));
    }
    hover(app, [16.0 - E, 0.0 - N]);
}

/// The upper storey placed alone on 2. kat; Oda modülleri choosing it: “yalnız modülün 2. katı”.
fn single_window(app: &mut App) {
    two(app);
    edit(
        app,
        json!({"kind": "place", "module": "Dubleks", "at": {"x": 15, "y": 0}, "level": "kat2", "storey": 1}),
    );
    let _ = app.update(Message::ActiveLevel(Some("kat2".into())));
    run(app, "arch.modules");
    event(app, Event::Instance(2));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("dubleks-pencere", window),
        ("dubleks-tekrar", repeat),
        ("dubleks-katlar", floors),
        ("dubleks-yerlestir", place),
        ("dubleks-yinelenen", again),
        ("dubleks-tek-kat", single),
        ("dubleks-tek-kat-pencere", single_window),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(app: &App) -> kentos_commands::ModulesEdited {
        match &app.room_modules.as_ref().expect("the window").repeat_preview {
            Some(Ok(done)) => done.clone(),
            other => panic!("{other:?}"),
        }
    }

    /// docs/adr/0810: the repeat copies the deleted fixture alone, the others' like already on the 2nd floor;
    /// Hedefte aynısı olanı da kopyala copies them all; written, the fixture is copied once and the step undoes whole.
    #[test]
    fn a_second_repeat_copies_only_what_the_target_lacks() {
        let mut app = crate::files_testing::app_with_drawing();
        again(&mut app);
        let p = plan(&app);
        assert_eq!(p.copied.iter().map(|c| c.source.as_str()).collect::<Vec<_>>(), [uid(18)]);
        assert_eq!(p.report.matched.len(), 11);
        event(&mut app, Event::RepeatAgain(true));
        let p = plan(&app);
        assert_eq!(p.copied.len(), 12);
        assert!(p.report.matched.is_empty());
        event(&mut app, Event::RepeatAgain(false));
        let before = app.document.as_ref().unwrap().model.entities().count();
        event(&mut app, Event::Repeat);
        assert_eq!(app.document.as_ref().unwrap().model.entities().count(), before + 1);
        let _ = app.update(Message::Run("edit.undo"));
        assert_eq!(app.document.as_ref().unwrap().model.entities().count(), before);
    }

    /// docs/adr/0811: Tek kat places the duplex's upper storey alone on the top level (the whole duplex finds no
    /// level above it): seven objects on 2. kat, one undo step.
    #[test]
    fn a_single_storey_goes_where_the_whole_does_not() {
        let mut app = crate::files_testing::app_with_drawing();
        single(&mut app);
        let before_single = app.document.as_ref().unwrap().model.entities().count();
        crate::tools_scenes::click(&mut app, [16.0 - E, 0.0 - N]);
        let doc = &app.document.as_ref().unwrap().model;
        assert_eq!(doc.entities().count(), before_single + 7);
        let inst = doc.settings().arch.as_ref().unwrap().modules[0].instances.last().unwrap().clone();
        assert_eq!((inst.storey, inst.level.as_deref(), inst.members.len()), (Some(1), Some("kat2"), 7));
        let _ = app.update(Message::Run("edit.undo"));
        assert_eq!(app.document.as_ref().unwrap().model.entities().count(), before_single);
    }
}
