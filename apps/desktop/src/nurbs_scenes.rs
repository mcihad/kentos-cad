//! The pictures of docs/adr/0259 (NURBS curves): a stream bank drawn through
//! its points, an exact quarter circle by weighted control points, a closed
//! periodic pond and the old Catmull–Rom curve beside them; the pond selected
//! with Öznitelikler's Derece, Denetim noktası, Ağırlıklı, Kapalı, Uzunluk and
//! Alan; NURBS eğrisi drawing through points (the command's plan: the curve
//! solid, its control polygon dashed), by control points at degree 2, and
//! what Enter wrote. `tools_screens` takes them in both themes at 1440×900
//! and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=nurbs-cizim,nurbs-oznitelikler,nurbs-gecis,nurbs-denetim,nurbs-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::json;

use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// The bank (1, through 5 points), the quarter circle (2, weighted), the pond
/// (3, closed and periodic), the old curve (4, Catmull–Rom).
fn ground() -> Objects {
    let mut o = Objects::new();
    o.nurbs(
        "cizim",
        3,
        &[[0.0, 0.0], [3.6, 3.4], [8.5, -0.6], [12.3, 4.4], [16.0, 2.0]],
        &[0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0, 1.0],
        json!({}),
    );
    o.nurbs(
        "yol",
        2,
        &[[28.0, 0.0], [28.0, 8.0], [20.0, 8.0]],
        &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
        json!({ "weights": [1.0, std::f64::consts::FRAC_1_SQRT_2, 1.0] }),
    );
    o.nurbs(
        "parsel",
        3,
        &[
            [2.0, 9.0],
            [9.0, 8.0],
            [12.0, 14.0],
            [6.0, 17.0],
            [1.0, 14.0],
            [2.0, 9.0],
            [9.0, 8.0],
            [12.0, 14.0],
        ],
        &(0..12).map(f64::from).collect::<Vec<_>>(),
        json!({ "closed": true, "periodic": true }),
    );
    let old = o.path("cizim", &[[16.0, 10.0], [20.0, 15.0], [25.0, 11.0], [29.0, 16.0]], false);
    o.fields(old, json!({ "kind": "spline", "closed": false }));
    o
}

fn fit(app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 2.0,
            min_y: N - 2.0,
            max_x: E + 31.0,
            max_y: N + 19.0,
        },
        24.0,
    );
}

/// The drawing: three NURBS curves and the old curve.
fn drawn(app: &mut App) {
    open(app, ground());
    make_cad(app);
    fit(app);
    hover(app, [30.0, -1.0]);
}

/// The pond selected: its rows in Öznitelikler.
fn props(app: &mut App) {
    drawn(app);
    app.selection.set([Slot(3)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
    hover(app, [30.0, -1.0]);
}

/// NURBS eğrisi through points: three given, the cursor the fourth.
fn through(app: &mut App) {
    drawn(app);
    run(app, "tool.nurbs");
    click(app, [14.0, 1.0]);
    click(app, [17.0, 6.0]);
    click(app, [21.0, 3.0]);
    hover(app, [24.5, 7.5]);
}

/// By control points at degree 2: Yöntem (Y), Derece (D) 2.
fn control(app: &mut App) {
    drawn(app);
    run(app, "tool.nurbs");
    typed(app, "Y");
    typed(app, "D");
    typed(app, "2");
    click(app, [14.0, 1.0]);
    click(app, [17.0, 6.0]);
    click(app, [21.0, 3.0]);
    hover(app, [24.5, 7.5]);
}

/// Enter after four points through: the curve written, its message.
fn written(app: &mut App) {
    through(app);
    click(app, [24.5, 7.5]);
    typed(app, "");
    hover(app, [30.0, -1.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("nurbs-cizim", drawn),
        ("nurbs-oznitelikler", props),
        ("nurbs-gecis", through),
        ("nurbs-denetim", control),
        ("nurbs-yazildi", written),
    ]
}
