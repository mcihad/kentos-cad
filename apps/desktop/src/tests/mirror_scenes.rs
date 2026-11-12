//! The pictures of docs/adr/0292 (Aynala's Yazılar, AutoCAD's MIRRTEXT): a door block with its number attribute and
//! a room's name mirrored across a level axis: the door's swing turns over, its number and the name read upright;
//! the same across a vertical axis; the same with Yazılar aynalı, the texts turning with the drawing; Aynala's
//! prompt with Yazılar while the axis is shown. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=ayna-kapi,ayna-yazilar-okunur,ayna-yazilar-aynali,ayna-secenek cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::json;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open_with_blocks, run, typed};
use crate::tools_screens::Scene;

const DOOR: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d7701";

/// A wall, a door on it with its number “K-12”, the room's name; a CAD project, the view on them.
fn ground(app: &mut App) {
    let mut o = Objects::new();
    o.line("cizim", [-2.0, 4.0], [4.0, 4.0]);
    o.line("cizim", [6.0, 4.0], [14.0, 4.0]);
    let door = o.insert("cizim", DOOR, [4.0, 4.0], 1.0, 0.0, false);
    o.fields(door, json!({ "attrs": { "NO": "K-12" } }));
    o.text("cizim", [7.0, 6.0], "Salon", 0.6, 0.0);
    let blocks = json!([{
        "id": DOOR,
        "name": "Kapı",
        "base": { "x": 0.0, "y": 0.0 },
        "entities": [
            { "kind": "line", "id": 1, "layerId": "cizim", "attrs": {}, "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 0.0, "y": 2.0 } },
            { "kind": "arc", "id": 2, "layerId": "cizim", "attrs": {}, "c": { "x": 0.0, "y": 0.0 }, "r": 2.0, "a0": 0.0,
              "a1": std::f64::consts::FRAC_PI_2 }
        ],
        "attributes": [{ "tag": "NO", "p": { "x": 0.5, "y": 0.4 }, "height": 0.35, "rotation": 0.0, "value": "?" }]
    }]);
    open_with_blocks(app, o, blocks);
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 6.0,
            max_x: E + 16.0,
            max_y: N + 10.0,
        },
        24.0,
    );
}

/// The door and the name selected, Aynala started.
fn mirror_started(app: &mut App) {
    ground(app);
    run(app, "edit.selectAll");
    run(app, "tool.mirror");
}

/// Across the wall's line below (a level axis): the swing turns over, the number and the name upright.
fn door(app: &mut App) {
    mirror_started(app);
    click(app, [-2.0, 2.5]);
    click(app, [14.0, 2.5]);
    hover(app, [12.0, -4.0]);
}

/// Across a vertical axis: the copies read upright, their sides swapped.
fn readable(app: &mut App) {
    mirror_started(app);
    click(app, [10.0, -4.0]);
    click(app, [10.0, 8.0]);
    hover(app, [12.0, -4.0]);
}

/// Yazılar aynalı: the copies' texts turn with the drawing.
fn mirrored(app: &mut App) {
    mirror_started(app);
    typed(app, "Y");
    click(app, [-2.0, 2.5]);
    click(app, [14.0, 2.5]);
    hover(app, [12.0, -4.0]);
}

/// The prompt with Yazılar while the axis is shown.
fn option(app: &mut App) {
    mirror_started(app);
    click(app, [-2.0, 2.5]);
    hover(app, [14.0, 2.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("ayna-kapi", door),
        ("ayna-yazilar-okunur", readable),
        ("ayna-yazilar-aynali", mirrored),
        ("ayna-secenek", option),
    ]
}
