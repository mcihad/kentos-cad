//! The pictures of docs/adr/0775 (room modules): the shared cases' bathroom (`fixtures/commands/v1/cad.modules.edit.json`
//! `setup`) defined as a module from the selection in Oda modülleri; Modül yerleştir mirrored under the pointer with its
//! settings; four instances on two levels after the source update (one mirrored with D2- numbers, one detached keeping
//! its 80 cm door); Oda modülleri with the update's plan from the first instance; a mirrored instance's door selected,
//! its Modül section in Öznitelikler. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=modul-tanimla,modul-yerlestir,modul-dort,modul-guncelle,modul-oznitelikler cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_ui::widget::docking;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::room_modules_window::Event;
use crate::tools_scenes::{E, N, hover, run, typed};
use crate::tools_screens::Scene;

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.modules.edit.json");

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

/// The bathroom's objects selected, Oda modülleri open with the name typed: Seçimden tanımla is the next press.
fn define(app: &mut App) {
    open(app, [-1.5, -3.0, 8.5, 6.0]);
    app.selection.set((1..=9).map(Slot));
    run(app, "arch.modules");
    event(app, Event::Name("Banyo".into()));
}

fn defined(app: &mut App) {
    define(app);
    event(app, Event::Define);
    event(app, Event::Close);
    app.selection.set(Vec::<Slot>::new());
}

/// Modül yerleştir mirrored, the pointer at 20 m along: the instance drawn from the plan, its settings beside.
fn place(app: &mut App) {
    defined(app);
    fit(app, [-1.5, -3.0, 24.0, 6.0]);
    run(app, "tool.modulePlace");
    typed(app, "A");
    hover(app, [20.0 - E, 0.0 - N]);
}

/// Four instances on two levels, one mirrored and numbered D2-, one detached; the first's door made 90 cm and the
/// source updated from it.
fn four(app: &mut App) {
    defined(app);
    edit(
        app,
        json!({"kind": "place", "module": "Banyo", "at": {"x": 10, "y": 0}, "level": "zemin"}),
    );
    edit(
        app,
        json!({"kind": "place", "module": "Banyo", "at": {"x": 20, "y": 0}, "mirror": true, "level": "kat1",
                    "numbering": {"prefix": "D2-"}}),
    );
    edit(
        app,
        json!({"kind": "place", "module": "Banyo", "at": {"x": 30, "y": 0}, "level": "kat1"}),
    );
    edit(
        app,
        json!({"kind": "detach", "module": "Banyo", "instance": "banyo-4"}),
    );
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let door: kentos_commands::OpeningsEdit = serde_json::from_value(json!({
        "uids": ["0192f5a1-7750-7000-8000-000000000005"], "operation": {"kind": "set", "width": 0.9}
    }))
    .expect("input");
    let _ = kentos_native_application::openings_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        door,
    );
}

/// Oda modülleri on the first instance: the plan of the update, before it is written.
fn plan(app: &mut App) {
    four(app);
    fit(app, [-1.5, -3.0, 34.0, 6.0]);
    app.selection.set([Slot(5)]);
    run(app, "arch.modules");
}

fn updated(app: &mut App) {
    plan(app);
    event(app, Event::Update);
    event(app, Event::Close);
    app.selection.set(Vec::<Slot>::new());
}

/// The mirrored instance's door selected: its Modül section, Genel and Geometri folded.
fn props(app: &mut App) {
    updated(app);
    app.selection.set([Slot(28)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "geometry",
    )));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("modul-tanimla", define),
        ("modul-yerlestir", place),
        ("modul-guncelle", plan),
        ("modul-dort", updated),
        ("modul-oznitelikler", props),
    ]
}
