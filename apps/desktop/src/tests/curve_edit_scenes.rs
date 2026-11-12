//! The pictures of docs/adr/0303 (Eğriyi düzenle) in a CAD project: a stream's bank drawn as a polyline with an
//! arc edge, picked, the prompt with what a polyline takes; made a curve through its vertices (Eğriye çevir) and
//! a fit point moving with the command's plan drawn; the old NURBS bank by control points, one of them moving
//! over its dashed control polygon; and what the click wrote. `tools_screens` takes them in the dark and the light
//! theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=egri-duzenle-coklu,egri-duzenle-gecis,egri-duzenle-denetim,egri-duzenle-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::json;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// The bank as a polyline with one arc edge (1); the old bank, a cubic curve by control points (2).
fn ground() -> Objects {
    let mut o = Objects::new();
    o.bulged(
        "cizim",
        &[
            [0.0, 0.0],
            [6.0, 3.0],
            [12.0, 1.0],
            [18.0, 5.0],
            [24.0, 2.0],
        ],
        &[0.0, 0.0, -0.45, 0.0, 0.0],
    );
    o.nurbs(
        "yol",
        3,
        &[
            [0.0, 10.0],
            [5.0, 15.0],
            [10.0, 9.0],
            [15.0, 16.0],
            [20.0, 11.0],
            [25.0, 14.0],
        ],
        &[0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 3.0, 3.0, 3.0, 3.0],
        json!({}),
    );
    o
}

fn drawn(app: &mut App) {
    open(app, ground());
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 2.0,
            min_y: N - 2.0,
            max_x: E + 27.0,
            max_y: N + 18.0,
        },
        24.0,
    );
}

/// The polyline picked: Kapat, Düzleştir, Eğriye çevir, Denetim noktalarıyla and Derece offered.
fn polyline(app: &mut App) {
    drawn(app);
    run(app, "tool.curveEdit");
    click(app, [3.0, 1.5]);
    hover(app, [26.0, -1.0]);
}

/// Made a curve through its vertices; Noktayı taşı on its third point, the new place following the mouse.
fn through(app: &mut App) {
    polyline(app);
    typed(app, "E");
    typed(app, "T");
    click(app, [12.0, 1.0]);
    hover(app, [12.5, 5.5]);
}

/// The old bank picked; Denetim taşı on its third control point, the plan over the dashed polygon.
fn control(app: &mut App) {
    drawn(app);
    run(app, "tool.curveEdit");
    click(app, [0.4, 10.5]);
    typed(app, "D");
    click(app, [10.0, 9.0]);
    hover(app, [11.0, 5.5]);
}

/// The click after Noktayı taşı: the curve written through the new place, its message.
fn written(app: &mut App) {
    through(app);
    click(app, [12.5, 5.5]);
    hover(app, [26.0, -1.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("egri-duzenle-coklu", polyline),
        ("egri-duzenle-gecis", through),
        ("egri-duzenle-denetim", control),
        ("egri-duzenle-yazildi", written),
    ]
}
