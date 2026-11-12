//! The pictures of docs/adr/0299 (Böl and Ölç with blocks, a first distance
//! and the ends) in a CAD project: a road's centreline (an open polyline with
//! an arc bend) and a street light block (a pole and its arm). Aralık 12 m from
//! 3 m, both ends, the light block aligned: the command's plan drawn on the
//! road before the write; what was written with the command's message; the
//! same with Hizala off. `tools_screens` takes them in the dark and the light
//! theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=bol-blok,bol-blok-yazildi,bol-blok-hizasiz cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::json;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open_with_blocks, run, typed};
use crate::tools_screens::Scene;

/// The road's centreline (1).
fn ground() -> Objects {
    let mut o = Objects::new();
    let quarter = (std::f64::consts::PI / 8.0).tan();
    o.bulged(
        "yol",
        &[[0.0, 0.0], [40.0, 0.0], [52.0, 12.0], [52.0, 40.0]],
        &[0.0, quarter, 0.0, 0.0],
    );
    o
}

/// The street light: a pole (a small circle) and its arm reaching north, the lamp at its end.
fn blocks() -> serde_json::Value {
    json!([{
        "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d3101", "name": "Aydınlatma direği", "base": { "x": 0, "y": 0 },
        "entities": [
            { "kind": "circle", "id": 1, "layerId": "0", "attrs": {}, "c": { "x": 0, "y": 0 }, "r": 0.4 },
            { "kind": "line", "id": 2, "layerId": "0", "attrs": {}, "a": { "x": 0, "y": 0.4 }, "b": { "x": 0, "y": 3.0 } },
            { "kind": "polygon", "id": 3, "layerId": "0", "attrs": {},
              "pts": [{ "x": -0.9, "y": 3.0 }, { "x": 0.9, "y": 3.0 }, { "x": 0.6, "y": 3.6 }, { "x": -0.6, "y": 3.6 }] }
        ]
    }])
}

/// The drawing in a CAD project, Böl running with the light block, a 12 m step from 3 m and both ends.
fn opened(app: &mut App, aligned: bool) {
    open_with_blocks(app, ground(), blocks());
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 8.0,
            max_x: E + 62.0,
            max_y: N + 44.0,
        },
        24.0,
    );
    run(app, "tool.divide");
    typed(app, "B");
    if !aligned {
        typed(app, "H");
    }
    typed(app, "A");
    typed(app, "12");
    typed(app, "İ");
    typed(app, "3");
    typed(app, "U");
    typed(app, "U");
    typed(app, "U");
}

/// The road picked near its start, not yet written: the plan.
fn planned(app: &mut App) {
    opened(app, true);
    click(app, [6.0, 0.0]);
    hover(app, [30.0, 30.0]);
}

/// What was written: the lights along the road, the message.
fn written(app: &mut App) {
    planned(app);
    typed(app, "12");
    hover(app, [30.0, 30.0]);
}

/// Hizala off: every light keeps the block's own turn.
fn unaligned(app: &mut App) {
    opened(app, false);
    click(app, [6.0, 0.0]);
    typed(app, "12");
    hover(app, [30.0, 30.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("bol-blok", planned),
        ("bol-blok-yazildi", written),
        ("bol-blok-hizasiz", unaligned),
    ]
}
