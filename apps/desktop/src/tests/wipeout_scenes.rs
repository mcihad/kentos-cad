//! The pictures of docs/adr/0310 (draw order and masks): a parcel with a hatch and the lines across it, a framed mask
//! and a frameless one over them, Maske drawing a third (its options and the ring under the cursor); a mask selected
//! with Öznitelikler's Maske row; a line put right above the mask by Nesnenin üstüne, drawn over it again; a CBS
//! project's Düzenle with the new buttons. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=maske-ciz,maske-oznitelikler,maske-ustune,maske-cbs cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::json;

use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run};
use crate::tools_screens::Scene;

/// The parcel (1) and its hatch (2), two lines across (3, 4), a text (5), a framed mask over the middle (6) and a
/// frameless one (7), both drawn after the rest of their layer.
fn ground() -> Objects {
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[[0.0, 0.0], [30.0, 0.0], [30.0, 16.0], [0.0, 16.0]],
        true,
    );
    let h = o.hatch(
        "parsel",
        &[[0.0, 0.0], [30.0, 0.0], [30.0, 16.0], [0.0, 16.0]],
        json!({ "type": "lines", "angle": 45, "spacing": 1.2 }),
    );
    o.fields(h, json!({ "color": "#3E63DD" }));
    let a = o.line("cizim", [-2.0, 8.0], [32.0, 8.0]);
    o.fields(a, json!({ "lineWeight": 0.7 }));
    o.line("cizim", [-2.0, 2.0], [32.0, 14.0]);
    o.text("cizim", [3.0, 11.0], "101/5", 1.6, 0.0);
    let m = o.path(
        "cizim",
        &[[6.0, 4.0], [14.0, 4.0], [14.0, 12.0], [6.0, 12.0]],
        true,
    );
    o.fields(m, json!({ "wipeout": { "frame": true } }));
    let f = o.path("cizim", &[[18.0, 3.0], [26.0, 3.0], [22.0, 13.0]], true);
    o.fields(f, json!({ "wipeout": { "frame": false } }));
    o
}

fn fit(app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 4.0,
            max_x: E + 36.0,
            max_y: N + 20.0,
        },
        24.0,
    );
}

fn drawn(app: &mut App, cad: bool) {
    open(app, ground());
    if cad {
        make_cad(app);
    }
    fit(app);
}

/// Maske drawing a third mask: two corners clicked, the cursor at the third.
fn drawing(app: &mut App) {
    drawn(app, true);
    app.command_expanded = true;
    run(app, "tool.wipeout");
    click(app, [26.0, 15.0]);
    click(app, [33.0, 15.0]);
    hover(app, [33.0, 18.5]);
}

/// The framed mask selected: Öznitelikler's Maske row.
fn props(app: &mut App) {
    drawn(app, true);
    app.selection.set([Slot(6)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    hover(app, [34.0, -2.5]);
}

/// The thick line put right above the framed mask: drawn over it again.
fn above(app: &mut App) {
    drawn(app, true);
    app.command_expanded = true;
    app.selection.set([Slot(3)]);
    run(app, "tool.drawOrderAbove");
    click(app, [6.0, 6.0]);
    app.selection.set(Vec::new());
    hover(app, [34.0, -2.5]);
}

/// A CBS project's Düzenle: Maske and the drawing order's buttons in Nesne.
fn gis(app: &mut App) {
    drawn(app, false);
    app.tab = "edit";
    hover(app, [34.0, -2.5]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("maske-ciz", drawing),
        ("maske-oznitelikler", props),
        ("maske-ustune", above),
        ("maske-cbs", gis),
    ]
}
