//! The pictures of docs/adr/0785 (building materials): the shared cases' drawing
//! (`fixtures/commands/v1/cad.materials.edit.json` `setup`): a style whose brick core is bound to Tuğla, its two walls, a
//! wall of its own, one on a locked layer, slabs bound to Betonarme; Malzemeler with Tuğla chosen, its hatch changed and
//! the plan counting what follows; the change applied; a wall's Öznitelikler (each component's material, bound or text);
//! Metraj by material; and the plaster that turns up to a window's frame at its jambs (§5), near. `tools_screens` takes
//! them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=malzeme-pencere,malzeme-onizleme,malzeme-uygulandi,malzeme-oznitelikler,malzeme-metraj,malzeme-dogramaya cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_ui::widget::docking;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::materials_window::Event;
use crate::tools_scenes::run;
use crate::tools_screens::Scene;

const CASES: &str = include_str!("../../../fixtures/commands/v1/cad.materials.edit.json");

fn setup() -> Value {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    file["setup"].clone()
}

fn open(app: &mut App, doc: Value, bounds: [f64; 4]) {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(doc).expect("the drawing reads");
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

fn event(app: &mut App, e: Event) {
    let _ = app.update(Message::Materials(e));
}

/// The cases' walls and slabs.
fn drawing(app: &mut App) {
    open(app, setup(), [-1.0, -1.0, 45.0, 9.5]);
}

/// Malzemeler with Tuğla chosen.
fn window(app: &mut App) {
    drawing(app);
    run(app, "arch.materials");
}

/// Tuğla's hatch typed ANSI32 at 0.8 and 45°: the plan counts the style, three walls and the one left locked.
fn preview(app: &mut App) {
    window(app);
    event(app, Event::Pattern("ANSI32".into()));
    event(app, Event::Scale("0.8".into()));
    event(app, Event::Angle("45".into()));
}

/// Uygula, near the walls: every bound core hatched anew, the locked wall as it was.
fn applied(app: &mut App) {
    preview(app);
    event(app, Event::Apply);
    event(app, Event::Close);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: -0.5, min_y: -0.6, max_x: 14.5, max_y: 8.6 },
        24.0,
    );
}

/// The wall of its own selected: Öznitelikler's Malzemeler, each component's material bound or text.
fn props(app: &mut App) {
    drawing(app);
    app.selection.set([Slot(3)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle("general")));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle("geometry")));
}

/// Metraj by material: Tuğla, Betonarme and the text materials, with their walls, slabs, areas and volumes.
fn quantities(app: &mut App) {
    drawing(app);
    run(app, "arch.quantities");
    let _ = app.update(Message::Quantities(crate::quantities::Event::Tab(crate::quantities::Tab::Materials)));
}

/// A wall plastered on both faces, its plasters turning at the jambs: up to the window's frame (7 cm deep, 5 cm from the
/// inner face) on the left window, to the middle at the right one without a frame depth; near, at 1:20.
fn framed(app: &mut App) {
    let mut doc = setup();
    let wrap = |c: &mut Value| {
        let name = c["name"].as_str().unwrap_or("").to_owned();
        if name.ends_with("sıva") {
            c["wrap"] = json!(true);
        }
    };
    for c in doc["settings"]["wallStyles"][0]["components"].as_array_mut().unwrap() {
        wrap(c);
    }
    let entities = doc["entities"].as_array_mut().unwrap();
    for c in entities[0]["components"].as_array_mut().unwrap() {
        wrap(c);
    }
    entities[0]["caps"] = json!("open");
    let window = |id: u32, at: f64, build: Value| {
        let (s0, s1) = (at - 0.6, at + 0.6);
        json!({"kind": "opening", "id": id, "layerId": "duvar", "attrs": {}, "host": "0192f5a1-7850-7000-8000-000000000001",
               "at": at, "width": 1.2, "form": "window", "leaves": 2, "frame": 0.06, "sill": 0.9, "head": 2.2,
               "build": build,
               "place": [{"x": s0, "y": 0.125}, {"x": s0, "y": -0.125}, {"x": s1, "y": 0.125}, {"x": s1, "y": -0.125}]})
    };
    entities.push(window(9, 1.6, json!({"frameDepth": 0.07, "frameOffset": 0.05, "sillBoard": 0.05})));
    entities.push(window(10, 4.4, json!({"sillBoard": 0.05})));
    doc["uids"].as_array_mut().unwrap().extend([
        json!("0192f5a1-7850-7000-8000-000000000009"),
        json!("0192f5a1-7850-7000-8000-00000000000a"),
    ]);
    doc["settings"]["plotScale"] = json!(20);
    doc["settings"]["arch"]["detail"] = json!("detailed");
    open(app, doc, [0.6, -0.55, 2.6, 0.55]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("malzeme-pencere", window),
        ("malzeme-onizleme", preview),
        ("malzeme-uygulandi", applied),
        ("malzeme-oznitelikler", props),
        ("malzeme-metraj", quantities),
        ("malzeme-dogramaya", framed),
    ]
}
