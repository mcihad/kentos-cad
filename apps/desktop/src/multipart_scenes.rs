//! The pictures of docs/adr/0307 (Kır, Buda and Uzat on a multi-part polyline) in a CAD project: a kerb line of three
//! parts, one object with its name, crossed by a road's axis and a boundary. Buda shown before its click on the first
//! part (only that part's piece dashed), then after it and a second click (the line still one object, its other
//! parts in place), and Kır splitting the third part in two parts of the same line. `tools_screens` takes them in
//! the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=parca-buda,parca-budandi,parca-kir cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::{Value, json};

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

fn xy(p: [f64; 2]) -> Value {
    json!({ "x": E + p[0], "y": N + p[1] })
}

fn ground() -> Objects {
    let mut o = Objects::new();
    let kerb = o.path("parsel", &[[0.0, 0.0], [20.0, 0.0]], false);
    o.fields(
        kerb,
        json!({ "parts": [
            { "pts": [xy([0.0, 6.0]), xy([20.0, 6.0]), xy([20.0, 14.0])] },
            { "pts": [xy([0.0, 18.0]), xy([8.0, 18.0])] },
        ] }),
    );
    o.data(kerb, &[("Ad", "Bordür 12")], None);
    // The road's axis across the first two parts, a boundary across the second's corner.
    o.line("yol", [12.0, -3.0], [12.0, 21.0]);
    o.line("yol", [16.0, 10.0], [24.0, 10.0]);
    o
}

fn drawn(app: &mut App) {
    open(app, ground());
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 3.0,
            min_y: N - 4.0,
            max_x: E + 26.0,
            max_y: N + 22.0,
        },
        24.0,
    );
}

/// Buda over the first part, right of the axis: only that piece is dashed.
fn trim_shown(app: &mut App) {
    drawn(app);
    run(app, "tool.trim");
    hover(app, [16.0, 0.0]);
}

/// Two clicks: the first part right of the axis and the second part's leg past the boundary.
fn trimmed(app: &mut App) {
    trim_shown(app);
    click(app, [16.0, 0.0]);
    click(app, [20.0, 12.0]);
    hover(app, [25.0, -3.0]);
}

/// Kır at one point of the third part, Enter: two parts of the same line.
fn broken(app: &mut App) {
    drawn(app);
    run(app, "tool.break");
    click(app, [4.0, 18.0]);
    typed(app, "");
    hover(app, [25.0, -3.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("parca-buda", trim_shown),
        ("parca-budandi", trimmed),
        ("parca-kir", broken),
    ]
}
