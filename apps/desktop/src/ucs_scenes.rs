//! The pictures of docs/adr/0261 (the user coordinate system): a building
//! turned 30° drawn in the world's axes; the UCS aligned to its wall by
//! Nesneye hizala, the grid, the axes' icon and the status bar following
//! it; İki nokta while its x axis is given; a wall typed along the UCS's
//! axes (`@8,0`, `@0,5`); the Koordinatlar panel of the Görünüm tab.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=ucs-dunya,ucs-nesne,ucs-iki-nokta,ucs-yazilan,ucs-serit cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_geometry_core::jsmath::{cos, sin};

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// How far the building is turned, radians (30°).
const TURN: f64 = std::f64::consts::PI / 6.0;

/// A point `(u, v)` along the building's axes from its corner, east and north of the origin.
fn on(u: f64, v: f64) -> [f64; 2] {
    let (c, s) = (cos(TURN), sin(TURN));
    [4.0 + u * c - v * s, 2.0 + u * s + v * c]
}

/// The building (12 × 8 along its turned axes) and a road along the world's axes.
fn ground() -> Objects {
    let mut o = Objects::new();
    o.path(
        "cizim",
        &[on(0.0, 0.0), on(12.0, 0.0), on(12.0, 8.0), on(0.0, 8.0)],
        true,
    );
    o.line("yol", [-2.0, -4.0], [24.0, -4.0]);
    o
}

fn fit(app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 7.0,
            max_x: E + 24.0,
            max_y: N + 17.0,
        },
        24.0,
    );
}

fn drawn(app: &mut App) {
    open(app, ground());
    make_cad(app);
    fit(app);
}

/// The world's axes: the grid square to the screen.
fn world(app: &mut App) {
    drawn(app);
    app.viewport.grid_shown = true;
    hover(app, on(6.0, 4.0));
}

/// Nesneye hizala on the building's south wall: the grid, the icon and the status bar turn.
fn object(app: &mut App) {
    world(app);
    run(app, "tool.ucsObject");
    click(app, on(1.0, 0.0));
    hover(app, on(6.0, 4.0));
}

/// İki nokta: the origin given, the x axis following the pointer.
fn two_points(app: &mut App) {
    world(app);
    run(app, "tool.ucsTwoPoints");
    click(app, on(0.0, 0.0));
    hover(app, on(10.0, 0.0));
}

/// A wall typed along the UCS: from the corner, 8 along x, 5 along y.
fn typed_wall(app: &mut App) {
    object(app);
    run(app, "tool.polyline");
    click(app, on(2.0, 2.0));
    typed(app, "@8,0");
    typed(app, "@0,4");
    hover(app, on(2.0, 6.0));
}

/// The Görünüm tab's Koordinatlar panel.
fn ribbon(app: &mut App) {
    object(app);
    app.tab = "view";
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("ucs-dunya", world),
        ("ucs-nesne", object),
        ("ucs-iki-nokta", two_points),
        ("ucs-yazilan", typed_wall),
        ("ucs-serit", ribbon),
    ]
}
