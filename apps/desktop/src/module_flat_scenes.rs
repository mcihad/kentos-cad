//! The pictures of docs/adr/0779 (room modules after their first slice): the shared cases' two-room flat
//! (`fixtures/commands/v1/cad.modules.edit.flats.json` `setup`) defined as a module and a mirrored second flat beside it in
//! its own group; Oda modülleri with Katları tekrarla's plan (the ground floor on the first and second floors, each with
//! its number prefix); the four flats after it; a bed moved by hand in the second flat, its Modül section saying it
//! drifted from the source; the window counting it beside the instance; Modül yerleştir with its Bölüm setting.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=daire-tanim,daire-tekrar,daire-katlar,daire-elle,daire-pencere,daire-yerlestir cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::{Slot, Uuid};
use kentos_ui::widget::docking;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::room_modules_window::Event;
use crate::tools_scenes::{E, N, hover, run};
use crate::tools_screens::Scene;

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.modules.edit.flats.json");

fn uid(n: u32) -> String {
    format!("0192f5a1-7790-7000-8000-{n:012x}")
}

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

/// The flat defined and a mirrored second flat beside it, in group d2 and numbered 2-.
fn two(app: &mut App) {
    open(app, [-1.5, -4.5, 22.5, 6.0]);
    edit(
        app,
        json!({"kind": "define", "name": "Daire", "uids": (1..=13).map(uid).collect::<Vec<_>>(),
               "base": {"x": 0, "y": 0}}),
    );
    edit(
        app,
        json!({"kind": "place", "module": "Daire", "at": {"x": 21, "y": 0}, "mirror": true, "level": "zemin",
               "numbering": {"prefix": "2-"}, "unit": "d2"}),
    );
}

/// The two flats, the second's living room selected: its Modül section (in group d2, same as the source).
fn defined(app: &mut App) {
    two(app);
    let living = slot_of(app, "daire-2", 8);
    app.selection.set([living]);
    fold_properties(app);
}

/// Oda modülleri with Katları tekrarla on the first and second floors, prefixes 1- and 2-: the plan before it is written.
fn repeat(app: &mut App) {
    two(app);
    run(app, "arch.modules");
    for (i, prefix) in [(0, "1-"), (1, "2-")] {
        event(app, Event::RepeatChoose(i, true));
        event(app, Event::RepeatPrefix(i, prefix.into()));
    }
}

/// After the repeat: six flats, the first floor shown.
fn floors(app: &mut App) {
    repeat(app);
    event(app, Event::Repeat);
    event(app, Event::Close);
    app.selection.set(Vec::<Slot>::new());
}

fn slot_of(app: &App, instance: &str, key: u32) -> Slot {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let m = &doc.arch_modules()[0];
    let inst = m.instance(instance).expect("the instance");
    let o = inst
        .members
        .iter()
        .find(|o| o.key == uid(key))
        .expect("the key");
    doc.slot_of(Uuid::parse_str(&o.uid).expect("an id"))
        .expect("the object")
}

fn fold_properties(app: &mut App) {
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "geometry",
    )));
}

/// The second flat's bed moved by hand half a metre: selected, its Modül section says it drifted.
fn moved(app: &mut App) {
    two(app);
    let bed = slot_of(app, "daire-2", 10);
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let mut e = doc.get(bed).expect("the bed").clone();
    if let kentos_commands::Entity::Insert(b) = &mut e {
        b.p.x -= 0.5;
    }
    doc.update(bed, e);
    app.selection.set([bed]);
    fold_properties(app);
}

/// Oda modülleri on the first flat: its entrance door made 95 cm (the source to give), the second flat's made 100 cm by
/// hand; the update's plan says the second's goes back to the source.
fn window(app: &mut App) {
    moved(app);
    let theirs = slot_of(app, "daire-2", 6);
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    for (door, width) in [(Slot(6), 0.95), (theirs, 1.0)] {
        let mut e = doc.get(door).expect("the door").clone();
        if let kentos_commands::Entity::Opening(o) = &mut e {
            o.opening.width = width;
        }
        doc.update(door, e);
    }
    app.selection.set([Slot(6)]);
    run(app, "arch.modules");
}

/// Modül yerleştir with the flat's group: Bölüm d3, the instance under the pointer.
fn place(app: &mut App) {
    two(app);
    fit(app, [-1.5, -4.5, 32.0, 6.0]);
    run(app, "tool.modulePlace");
    let _ = app.update(Message::ToolSetting(crate::architecture_settings::Commit {
        tool: "modulePlace",
        key: "B",
        value: "d3".into(),
    }));
    hover(app, [24.5 - E, 0.0 - N]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("daire-tanim", defined),
        ("daire-tekrar", repeat),
        ("daire-katlar", floors),
        ("daire-elle", moved),
        ("daire-pencere", window),
        ("daire-yerlestir", place),
    ]
}
