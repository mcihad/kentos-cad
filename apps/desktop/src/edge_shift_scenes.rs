//! The pictures of docs/adr/0191 (Paralel kaydır) in a CAD project at
//! 1:500, the shared trace's ground (`edge-shift.kcad`): a square parcel, a
//! trapeze and a road's polyline. The trapeze's top edge following the
//! cursor, its distance and the area's new size beside it; Alan's question
//! on the square; and what was written. The web's are `shots.mjs
//! edgeshift`. `tools_screens` takes them in the dark and the light theme
//! at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=paralel-kaydir,paralel-kaydir-alan,paralel-kaydir-yazildi,paralel-kaydir-yay,paralel-kaydir-yay-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// The square (1), the trapeze (2) and the road (3).
fn ground() -> Objects {
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[[-30.0, -10.0], [-10.0, -10.0], [-10.0, 10.0], [-30.0, 10.0]],
        true,
    );
    o.path(
        "parsel",
        &[[0.0, -10.0], [20.0, -10.0], [15.0, 0.0], [5.0, 0.0]],
        true,
    );
    o.path(
        "yol",
        &[[0.0, 10.0], [10.0, 10.0], [10.0, 20.0], [20.0, 20.0]],
        false,
    );
    // docs/adr/1024: a parcel whose east edge is a curved kerb (right of the other scenes' view).
    let kerb = o.path("parsel", &[[34.0, -10.0], [48.0, -10.0], [48.0, 10.0], [34.0, 10.0]], true);
    o.fields(kerb, serde_json::json!({"bulges": [0.0, 0.3, 0.0, 0.0]}));
    o
}

/// The drawing in a CAD project at 1:500, Paralel kaydır running.
fn opened(app: &mut App) {
    opened_over(app, [-36.0, -16.0, 26.0, 26.0]);
}

/// As [`opened`], the view over `b` (west, south, east, north from the origin).
fn opened_over(app: &mut App, b: [f64; 4]) {
    open(app, ground());
    make_cad(app);
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.plot_scale = 500.0;
        doc.model.set_settings(settings);
    }
    app.tab = "home";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + b[0],
            min_y: N + b[1],
            max_x: E + b[2],
            max_y: N + b[3],
        },
        24.0,
    );
    run(app, "tool.edgeShift");
}

/// The trapeze's top edge picked and the cursor 3 m above it.
fn shown(app: &mut App) {
    opened(app);
    click(app, [10.0, 0.0]);
    hover(app, [10.0, 3.0]);
}

/// Alan's question on the square's south edge.
fn area(app: &mut App) {
    opened(app);
    click(app, [-20.0, -10.0]);
    typed(app, "A");
    hover(app, [-20.0, -12.0]);
}

/// What was written: the trapeze at 180 m², the square's south edge 2 m
/// out, the road's middle edge 3 m to the east.
fn written(app: &mut App) {
    shown(app);
    for t in ["A", "180"] {
        typed(app, t);
    }
    click(app, [-20.0, -10.0]);
    typed(app, "2");
    click(app, [10.0, 15.0]);
    click(app, [13.0, 15.0]);
    hover(app, [24.0, -6.0]);
}

/// docs/adr/1024: the kerb's arc picked, the cursor 2 m out: the arc grows on its own circle, its straight
/// neighbours slide.
fn arc_shown(app: &mut App) {
    opened_over(app, [26.0, -16.0, 60.0, 16.0]);
    click(app, [51.0, 0.0]);
    hover(app, [53.0, 0.0]);
}

/// The kerb 2 m out, written: one step, its arc still an arc.
fn arc_written(app: &mut App) {
    arc_shown(app);
    typed(app, "2");
    hover(app, [58.0, -14.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("paralel-kaydir-yay", arc_shown),
        ("paralel-kaydir-yay-yazildi", arc_written),
        ("paralel-kaydir", shown),
        ("paralel-kaydir-alan", area),
        ("paralel-kaydir-yazildi", written),
    ]
}
