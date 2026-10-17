//! The pictures of docs/adr/0328 (taramanın kaynak sınırı ve yüzün bağı) in a CAD project at 1:100: a flat's walls
//! drawn as lines, the living room hatched with Sınır: çizgiler and its right wall then moved 1.5 m west (the hatch
//! follows the line work in the same step, Öznitelikler says what it follows: tarama-yuz-bagi); a room with a bay
//! window's arc and an elliptical column hatched by its closed object, the hatch selected (the boundary's arc and
//! ellipse exact: tarama-kesin-sinir). `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=tarama-yuz-bagi,tarama-kesin-sinir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_ui::widget::docking;
use std::f64::consts::TAU;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, layer, open_edited, run, typed};
use crate::tools_screens::Scene;

/// A drawing in a CAD project at 1:100, Açıklama's tab open, the view on a box of it.
fn cad(app: &mut App, objects: Objects, min: [f64; 2], max: [f64; 2]) {
    open_edited(app, objects, |d| {
        d["layers"] = serde_json::json!([
            layer("duvar", "Duvarlar", "fg", 0.5),
            layer("tarama", "Tarama", "#3E63DD", 0.18)
        ]);
        d["activeLayer"] = serde_json::json!("tarama");
    });
    make_cad(app);
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.plot_scale = 100.0;
        doc.model.set_settings(settings);
    }
    app.tab = "annotate";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + min[0],
            min_y: N + min[1],
            max_x: E + max[0],
            max_y: N + max[1],
        },
        24.0,
    );
}

/// The selected object's rows with the layers folded away and Genel closed.
fn rows_of(app: &mut App, slot: u32) {
    app.selection.set([Slot(slot)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
}

/// A flat's walls as lines running past their corners (1–7), the living room hatched by its line work, its right
/// wall (2) moved 1.5 m west.
fn face(app: &mut App) {
    let mut o = Objects::new();
    o.line("duvar", [-1.0, 0.0], [13.0, 0.0]);
    o.line("duvar", [7.0, -1.0], [7.0, 9.0]);
    o.line("duvar", [13.0, 8.0], [-1.0, 8.0]);
    o.line("duvar", [0.0, 9.0], [0.0, -1.0]);
    o.line("duvar", [12.0, -1.0], [12.0, 9.0]);
    o.line("duvar", [7.0, 4.5], [13.0, 4.5]);
    o.circle("duvar", [3.5, 4.0], 0.6);
    cad(app, o, [-3.0, -3.0], [15.0, 11.0]);
    app.memory.hatch_by_lines = true;
    app.memory.hatch_choice = 1;
    run(app, "tool.hatch");
    click(app, [2.0, 2.0]);
    let _ = app.update(Message::Run("tool.cancel"));
    app.selection.set([Slot(2)]);
    run(app, "tool.move");
    click(app, [7.0, 0.0]);
    typed(app, "@-1.5,0");
    app.selection.set([]);
    rows_of(app, 8);
    hover(app, [14.0, 10.0]);
}

/// A room whose top wall is a bay window's arc, an elliptical column inside; hatched by the room, selected.
fn exact(app: &mut App) {
    let mut o = Objects::new();
    let room = o.push(
        "duvar",
        serde_json::json!({ "kind": "polygon",
            "pts": [ { "x": E, "y": N }, { "x": E + 10.0, "y": N }, { "x": E + 10.0, "y": N + 6.0 }, { "x": E, "y": N + 6.0 } ],
            "bulges": [0.0, 0.0, 0.35, 0.0] }),
    );
    o.ellipse("duvar", [6.5, 3.0], [1.4, 0.6], 0.45, 0.0, TAU);
    cad(app, o, [-2.0, -2.0], [12.0, 10.0]);
    let _ = room;
    app.memory.hatch_by_lines = false;
    app.memory.hatch_choice = 1;
    run(app, "tool.hatch");
    click(app, [2.0, 2.0]);
    let _ = app.update(Message::Run("tool.cancel"));
    rows_of(app, 3);
    hover(app, [11.5, 9.5]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("tarama-yuz-bagi", face), ("tarama-kesin-sinir", exact)]
}
