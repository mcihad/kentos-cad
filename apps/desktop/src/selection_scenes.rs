//! The pictures of docs/adr/0187 (Seçim ekleri) in a CAD project at 1:500:
//! two parcels sharing an edge, a red parcel above them, a road, a point on
//! their shared corner, a line and a circle. A click on the shared edge with
//! Sıradakini seç's chip and its list; Çokgenle seç on Kesişenler before the
//! last corner and its answer; Benzerini seç from the first parcel; Seçim
//! süzgeci without Kapalı alan after a window over the drawing, and the Süzgeç cell's
//! menu. The web's are `shots.mjs selecting`. `tools_screens` takes them in
//! the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=secim-cip,secim-cokgen cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::json;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, at, click, hover, open, run, typed};
use crate::tools_screens::{Pointed, Scene, press_caption};
use crate::viewport::Event;

/// Parcels A (1) and B (2), the road (3), the corner point (4), the red
/// parcel (5), the line (6) and the circle (7).
fn ground() -> Objects {
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[[-20.0, -10.0], [0.0, -10.0], [0.0, 10.0], [-20.0, 10.0]],
        true,
    );
    o.path(
        "parsel",
        &[[0.0, -10.0], [20.0, -10.0], [20.0, 10.0], [0.0, 10.0]],
        true,
    );
    o.line("yol", [-25.0, -14.0], [25.0, -14.0]);
    o.point("cizim", [0.0, 10.0], None);
    let red = o.path(
        "parsel",
        &[[-20.0, 12.0], [0.0, 12.0], [0.0, 18.0], [-20.0, 18.0]],
        true,
    );
    o.fields(red, json!({ "color": "#E5484D" }));
    o.line("cizim", [5.0, 0.0], [15.0, 0.0]);
    o.circle("cizim", [-10.0, 0.0], 3.0);
    o
}

/// The drawing in a CAD project at 1:500, Giriş's tab open, the view about it.
fn shown(app: &mut App) {
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
            min_x: E - 30.0,
            min_y: N - 18.0,
            max_x: E + 30.0,
            max_y: N + 22.0,
        },
        24.0,
    );
}

/// A click on the parcels' shared edge: the first selected, the chip “1/2” beside it.
fn chip(app: &mut App) {
    shown(app);
    click(app, [0.0, 0.0]);
    hover(app, [8.0, -5.0]);
}

/// The concave polygon's corners, east and north from the drawing's origin.
const CORNERS: [[f64; 2]; 6] = [
    [-24.0, -12.0],
    [4.0, -12.0],
    [4.0, 2.0],
    [-6.0, 2.0],
    [-6.0, 12.0],
    [-24.0, 12.0],
];

/// Çokgenle seç on Kesişenler, five corners given, the pointer at the sixth.
fn polygon(app: &mut App) {
    shown(app);
    run(app, "tool.selectPolygon");
    typed(app, "K");
    for p in &CORNERS[..5] {
        click(app, *p);
    }
    hover(app, CORNERS[5]);
}

/// Its answer: the two parcels, the red one along its edge and the circle.
fn polygon_done(app: &mut App) {
    polygon(app);
    click(app, CORNERS[5]);
    run(app, "tool.confirm");
    hover(app, [26.0, 18.0]);
}

/// Benzerini seç from the first parcel: its like selected, the criteria in the prompt.
fn similar(app: &mut App) {
    shown(app);
    click(app, [-10.0, 6.0]);
    run(app, "tool.selectSimilar");
    hover(app, [26.0, 18.0]);
}

/// Seçim süzgeci without Kapalı alan, then a window over the drawing: the
/// rest selected, the cell on and what it left out said.
fn filtered(app: &mut App) {
    shown(app);
    run(app, "edit.selectFilter.polygon");
    let (from, to) = (at(app, [-28.0, 21.0]), at(app, [28.0, -18.0]));
    let _ = app.update(Message::Viewport(Event::Moved(from)));
    let _ = app.update(Message::Viewport(Event::Pressed(from)));
    let _ = app.update(Message::Viewport(Event::Moved(to)));
    let _ = app.update(Message::Viewport(Event::Released(to)));
    hover(app, [26.0, 18.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("secim-cip", chip),
        ("secim-cokgen", polygon),
        ("secim-cokgen-sonuc", polygon_done),
        ("secim-benzeri", similar),
        ("secim-suzgec", filtered),
    ]
}

/// The chip's list and the Süzgeç cell's menu.
pub(crate) fn pointed() -> Vec<Pointed> {
    vec![
        ("secim-cip-liste", chip, |s, app| {
            press_caption(s, app, "1/2", false, false)
        }),
        ("secim-suzgec-menu", filtered, |s, app| {
            press_caption(s, app, "Süzgeç", true, false)
        }),
    ]
}
