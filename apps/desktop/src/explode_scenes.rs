//! The pictures of docs/adr/0308 (Patlat on curves, as dense as asked): in a CAD project a tree's circle and a kerb's
//! arc selected and Patlat started (the polylines to come and their vertices at the session's 0.01 m deviation), then
//! at 12 pieces each, and what Enter wrote; in a CBS project an ellipse at a 2 m step. `tools_screens` takes them in
//! the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=patlat-egri,patlat-sayi,patlat-yazildi,patlat-cbs cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, open, run, typed};
use crate::tools_screens::Scene;

fn ground() -> Objects {
    let mut o = Objects::new();
    o.circle("cizim", [6.0, 6.0], 5.0);
    o.arc("yol", [22.0, 2.0], 8.0, 0.0, std::f64::consts::FRAC_PI_2);
    o.ellipse(
        "parsel",
        [44.0, 6.0],
        [8.0, 0.0],
        0.5,
        0.0,
        std::f64::consts::TAU,
    );
    o
}

fn drawn(app: &mut App, cad: bool) {
    open(app, ground());
    if cad {
        make_cad(app);
    }
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 2.0,
            min_y: N - 2.0,
            max_x: E + 54.0,
            max_y: N + 14.0,
        },
        24.0,
    );
}

/// The circle and the arc selected, Patlat started: the default density shown.
fn curves(app: &mut App) {
    drawn(app, true);
    app.selection.set([Slot(1), Slot(2)]);
    run(app, "tool.explode");
    hover(app, [50.0, -1.0]);
}

/// Sayı, 12: twelve pieces each.
fn counted(app: &mut App) {
    curves(app);
    typed(app, "S");
    typed(app, "12");
    hover(app, [50.0, -1.0]);
}

/// Enter: the polylines written.
fn written(app: &mut App) {
    counted(app);
    typed(app, "");
    hover(app, [50.0, -1.0]);
}

/// A CBS project: the ellipse at a 2 m step.
fn gis(app: &mut App) {
    drawn(app, false);
    app.selection.set([Slot(3)]);
    run(app, "tool.explode");
    typed(app, "A");
    typed(app, "2");
    hover(app, [50.0, -1.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("patlat-egri", curves),
        ("patlat-sayi", counted),
        ("patlat-yazildi", written),
        ("patlat-cbs", gis),
    ]
}
