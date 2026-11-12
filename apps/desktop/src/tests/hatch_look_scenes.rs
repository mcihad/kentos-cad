//! The pictures of docs/adr/0332 (taramanın saydamlığı ve sırası, boş bıraktığı yazılar) in a CAD project at 1:100:
//! a half seen solid hatch behind its walls with a masked text on it, a pattern sent back under an area's mask, a
//! text a hatch leaves open moved and its hole with it, and Tarama's Sıra menu. `tools_screens` takes them in the
//! dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=tarama-saydam,tarama-yazi-tasindi,tarama-sira cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::{Entity, TextEntity, Vec2};
use kentos_domain::Slot;
use serde_json::json;

use crate::app::{App, Message};
use crate::files_testing::{find_text, make_cad};
use crate::tools_scenes::{E, N, Objects, click, hover, layer, open_edited, run};
use crate::tools_screens::{Pointed, Scene, press_caption};

const LEFT: [[f64; 2]; 4] = [[0.0, 0.0], [9.0, 0.0], [9.0, 7.0], [0.0, 7.0]];
const RIGHT: [[f64; 2]; 4] = [[9.0, 0.0], [17.5, 0.0], [17.5, 7.0], [9.0, 7.0]];

/// Two rooms on Duvarlar (1, 2), a masked text in the left (3), an area's mask in the right (4) and a text (5);
/// Duvarlar active, the view on them.
fn rooms(app: &mut App) {
    let mut o = Objects::new();
    o.path("duvar", &LEFT, true);
    o.path("duvar", &RIGHT, true);
    let label = o.text("duvar", [2.0, 3.2], "SALON 24 m²", 0.6, 0.0);
    o.text_extras(label, None, None, true);
    let mask = o.path(
        "duvar",
        &[[12.0, 2.0], [15.5, 2.0], [15.5, 5.0], [12.0, 5.0]],
        true,
    );
    o.fields(mask, json!({ "wipeout": { "frame": true } }));
    o.text("duvar", [11.0, 0.8], "ODA 2", 0.6, 0.0);
    open_edited(app, o, |d| {
        d["layers"] = json!([layer("duvar", "Duvarlar", "fg", 0.5)]);
        d["activeLayer"] = json!("duvar");
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
            min_x: E - 2.0,
            min_y: N - 2.0,
            max_x: E + 19.5,
            max_y: N + 9.0,
        },
        24.0,
    );
}

/// Dolu at %50 behind its walls in the left room; ANSI31 sent back in the right, under its mask.
fn transparent(app: &mut App) {
    rooms(app);
    app.memory.hatch_by_lines = false;
    app.memory.hatch_choice =
        kentos_geometry_core::tools::hatch::choice_named("Dolu").expect("Dolu");
    app.memory.hatch_transparency = 0.5;
    app.memory.hatch_order = 3;
    run(app, "tool.hatch");
    click(app, [6.0, 1.0]);
    let _ = app.update(Message::Run("tool.cancel"));
    app.memory.hatch_choice =
        kentos_geometry_core::tools::hatch::choice_named("ANSI31").expect("ANSI31");
    app.memory.hatch_scale = 4.0;
    app.memory.hatch_transparency = 0.0;
    app.memory.hatch_order = 1;
    // Adalar taranır: the mask is no island; it covers the pattern under it.
    app.memory.hatch_islands = false;
    run(app, "tool.hatch");
    click(app, [10.0, 6.0]);
    let _ = app.update(Message::Run("tool.cancel"));
    app.memory.hatch_order = 0;
    app.memory.hatch_islands = true;
    hover(app, [18.5, 8.5]);
}

/// ANSI31 leaving ODA 2 open, then ODA 2 moved: its hole goes with it in the same step.
fn moved_text(app: &mut App) {
    rooms(app);
    app.memory.hatch_by_lines = false;
    app.memory.hatch_choice =
        kentos_geometry_core::tools::hatch::choice_named("ANSI31").expect("ANSI31");
    app.memory.hatch_scale = 3.0;
    app.memory.hatch_texts = true;
    run(app, "tool.hatch");
    click(app, [10.0, 6.0]);
    let _ = app.update(Message::Run("tool.cancel"));
    app.memory.hatch_texts = false;
    let doc = app.document.as_mut().expect("a drawing");
    let Some(Entity::Text(t)) = doc.model.get(Slot(5)).cloned() else {
        panic!("ODA 2")
    };
    let moved = Entity::Text(TextEntity {
        p: Vec2 {
            x: E + 15.8,
            y: N + 5.8,
        },
        ..t
    });
    assert!(doc.model.update(Slot(5), moved));
    hover(app, [18.5, 8.5]);
}

/// Tarama with its options shown, before Sıra's menu opens.
fn with_order(app: &mut App) {
    rooms(app);
    app.command_expanded = true;
    run(app, "tool.hatch");
    hover(app, [4.0, 1.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("tarama-saydam", transparent),
        ("tarama-yazi-tasindi", moved_text),
    ]
}

/// Sıra's menu.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![("tarama-sira", with_order, |s, app| {
        if find_text(s, app, "Sıra: atama").is_some() {
            press_caption(s, app, "Sıra: atama", false, false);
        } else {
            press_caption(s, app, "Diğer", false, false);
            let mut update = |app: &mut App, message: Message| {
                let _ = app.update(message);
            };
            s.settle(app, App::view, &mut update);
            press_caption(s, app, "Sıra: atama", false, false);
        }
    })]
}
