//! The pictures of docs/adr/0298 (Uzat-kısalt by the product command
//! `cad.entities.lengthen`) in a CAD project: a pipe's run (an open polyline
//! with an arc corner), a door's swing (an arc) and an oval path's edge (an
//! elliptical arc). Toplam 15 m hovered over the pipe's end: the command's
//! plan drawn with the lengths beside it; Toplam <200 (grads, the drawing's unit) over the swing: the arc
//! becomes a half circle; Dinamik on the elliptical arc: its end follows the
//! cursor on its own ellipse; what was written with the command's message.
//! `tools_screens` takes them in the dark and the light theme at 1440×900 and
//! 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=uzat-kisalt-toplam,uzat-kisalt-yay-aci,uzat-kisalt-elips,uzat-kisalt-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::f64::consts::{FRAC_PI_2, PI};

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// The pipe (1), the swing (2) and the oval's edge (3).
fn ground() -> Objects {
    let mut o = Objects::new();
    let quarter = (PI / 8.0).tan();
    o.bulged(
        "cizim",
        &[[0.0, 0.0], [5.0, 0.0], [7.0, 2.0], [7.0, 6.0]],
        &[0.0, quarter, 0.0, 0.0],
    );
    o.arc("cizim", [12.0, 1.0], 2.0, 0.0, FRAC_PI_2);
    o.ellipse("cizim", [4.0, -4.0], [4.0, 0.0], 0.5, PI, 1.5 * PI);
    o
}

/// The drawing in a CAD project, Uzat-kısalt running.
fn opened(app: &mut App) {
    open(app, ground());
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 2.0,
            min_y: N - 7.0,
            max_x: E + 16.0,
            max_y: N + 8.0,
        },
        24.0,
    );
    run(app, "tool.lengthen");
}

/// Toplam 15 m, the cursor over the pipe's end: the plan of what a click writes.
fn total(app: &mut App) {
    opened(app);
    typed(app, "T");
    typed(app, "15");
    hover(app, [7.0, 5.6]);
}

/// Toplam <200 (grads, the drawing's angle unit), the cursor over the swing's end: a half circle.
fn arc_angle(app: &mut App) {
    opened(app);
    typed(app, "T");
    typed(app, "<200");
    hover(app, [12.1, 2.9]);
}

/// Dinamik on the oval's edge: picked near its end, the cursor further round its ellipse.
fn ellipse(app: &mut App) {
    opened(app);
    click(app, [4.1, -5.95]);
    hover(app, [7.0, -6.2]);
}

/// What was written: the pipe 15 m long, the message.
fn written(app: &mut App) {
    total(app);
    click(app, [7.0, 5.6]);
    hover(app, [14.0, 6.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("uzat-kisalt-toplam", total),
        ("uzat-kisalt-yay-aci", arc_angle),
        ("uzat-kisalt-elips", ellipse),
        ("uzat-kisalt-yazildi", written),
    ]
}
