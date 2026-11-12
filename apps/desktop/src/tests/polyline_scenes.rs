//! The pictures of docs/adr/0258 (closed and wide polylines): a room drawn as a
//! closed line beside an area, walls 25 cm wide, an arrow whose width runs
//! from nothing to 60 cm and an arc edge 1 m wide; the closed wall selected,
//! Öznitelikler's Kapalı, Genişlik and Alan. Çoklu çizgi with Genişlik 50 cm
//! drawing (the band under the line, Genişlik and Kapat in the prompt) and
//! what Kapat wrote. `tools_screens` takes them in the dark and the light
//! theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=cokluc-cizim,cokluc-oznitelikler,cokluc-genislik,cokluc-kapat cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::json;

use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// The room (1, a closed line), the parcel (2, an area), the walls (3, closed,
/// 25 cm), the arrow (4, 0 → 60 cm, then 60 cm), the curve (5, its arc edge 1 m).
fn ground() -> Objects {
    let mut o = Objects::new();
    let room = o.path(
        "cizim",
        &[[0.0, 0.0], [8.0, 0.0], [8.0, 6.0], [0.0, 6.0]],
        false,
    );
    o.fields(room, json!({ "closed": true }));
    o.path(
        "parsel",
        &[[12.0, 0.0], [20.0, 0.0], [20.0, 6.0], [12.0, 6.0]],
        true,
    );
    let walls = o.path(
        "yol",
        &[
            [0.0, 10.0],
            [9.0, 10.0],
            [9.0, 16.0],
            [4.0, 18.0],
            [0.0, 16.0],
        ],
        false,
    );
    o.fields(
        walls,
        json!({ "closed": true, "widths": [[0.25, 0.25], [0.25, 0.25], [0.25, 0.25], [0.25, 0.25], [0.25, 0.25]] }),
    );
    let arrow = o.path("cizim", &[[12.0, 10.0], [16.0, 10.0], [20.0, 14.0]], false);
    o.fields(
        arrow,
        json!({ "widths": [[0.0, 0.6], [0.6, 0.6], [0.0, 0.0]] }),
    );
    let curve = o.bulged(
        "parsel",
        &[[22.0, 10.0], [26.0, 10.0], [29.0, 15.0]],
        &[0.6, 0.0, 0.0],
    );
    o.fields(
        curve,
        json!({ "widths": [[1.0, 1.0], [0.0, 0.0], [0.0, 0.0]] }),
    );
    o
}

fn fit(app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 2.0,
            min_y: N - 2.0,
            max_x: E + 31.0,
            max_y: N + 20.0,
        },
        24.0,
    );
}

/// The drawing: closed and wide lines beside an area.
fn drawn(app: &mut App) {
    open(app, ground());
    fit(app);
    hover(app, [30.0, 1.0]);
}

/// The closed wall selected: Kapalı, Genişlik and Alan in Öznitelikler, the
/// layers folded away and Genel closed so that Geometri shows whole.
fn props(app: &mut App) {
    drawn(app);
    app.selection.set([Slot(3)]);
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
    hover(app, [30.0, 1.0]);
}

/// Çoklu çizgi at Genişlik 50 cm: two points given, the cursor's segment,
/// the band under the line.
fn widening(app: &mut App) {
    open(app, ground());
    fit(app);
    run(app, "tool.polyline");
    typed(app, "E");
    typed(app, "0.5");
    click(app, [2.0, 2.0]);
    click(app, [6.0, 2.0]);
    hover(app, [6.0, 4.5]);
}

/// Kapat after three points: the closed line written, 50 cm wide all round.
fn closing(app: &mut App) {
    widening(app);
    click(app, [6.0, 4.5]);
    typed(app, "K");
    hover(app, [30.0, 1.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("cokluc-cizim", drawn),
        ("cokluc-oznitelikler", props),
        ("cokluc-genislik", widening),
        ("cokluc-kapat", closing),
    ]
}
