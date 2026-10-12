//! The pictures of docs/adr/0246 (Ötele through `cad.entities.offset`): a
//! road's centre line with a curve, a parcel and a well. The copy following
//! the cursor while the side is chosen (the command's plan), İki yana's two
//! copies, and what was written with the command's message. `tools_screens`
//! takes them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=otele-onizleme,otele-iki-yana,otele-sonuc cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::App;
use crate::tools_scenes::{E, N, Objects, click, forget, hover, open, run, typed};
use crate::tools_screens::Scene;

/// The road (1: straight, a quarter turn, straight), the parcel (2) and the well (3).
fn ground() -> Objects {
    let mut o = Objects::new();
    o.bulged(
        "yol",
        &[[-30.0, 0.0], [0.0, 0.0], [10.0, 10.0], [10.0, 30.0]],
        &[0.0, 0.414_213_562_373_095_1, 0.0, 0.0],
    );
    o.path(
        "parsel",
        &[[16.0, 4.0], [34.0, 4.0], [34.0, 22.0], [16.0, 22.0]],
        true,
    );
    o.circle("cizim", [-14.0, 14.0], 4.0);
    o
}

/// The drawing, Ötele running at 2 m.
fn opened(app: &mut App) {
    open(app, ground());
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 36.0,
            min_y: N - 12.0,
            max_x: E + 40.0,
            max_y: N + 34.0,
        },
        24.0,
    );
    run(app, "tool.offset");
    typed(app, "2");
}

/// The road picked and the cursor north of its straight part: the copy 2 m
/// to that side, dashed, the distance beside the cursor.
fn preview(app: &mut App) {
    opened(app);
    click(app, [-15.0, 0.0]);
    hover(app, [-15.0, 6.0]);
}

/// İki yana: a copy on each side of the road, the curve's two radii.
fn both(app: &mut App) {
    opened(app);
    typed(app, "I");
    click(app, [-15.0, 0.0]);
    hover(app, [-15.0, 6.0]);
}

/// What was written: the road's two copies and the parcel's inner copy,
/// each one step, and what the command said.
fn written(app: &mut App) {
    opened(app);
    typed(app, "I");
    click(app, [-15.0, 0.0]);
    forget(app);
    click(app, [-15.0, 4.0]);
    typed(app, "I");
    click(app, [25.0, 4.0]);
    click(app, [25.0, 8.0]);
    hover(app, [-30.0, 28.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("otele-onizleme", preview),
        ("otele-iki-yana", both),
        ("otele-sonuc", written),
    ]
}
