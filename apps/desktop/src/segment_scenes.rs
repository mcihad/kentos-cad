//! The pictures of docs/adr/0288 (Çoklu çizgi's PLINE pairs, widths and the edge edits): the grip menu of a
//! polyline's edge with Noktadan ya da yarıçapla yay… and Kenar genişliği…; Kenar düzenle's arc through the pointer;
//! the edge after Kenar genişliği (2 m to 0); Çoklu çizgi in arc mode after Açı 90 and Merkez, the centre under the
//! pointer; a polyline drawn with a tapered width. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kenar-menusu,kenar-yay,kenar-genislik,cizgi-aci-merkez,cizgi-genislik cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_interaction::grip_menu::GripAction;

use crate::app::{App, Message};
use crate::drawing_menus::Event;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// A road's edge as a polyline and a parcel beside it, in a CAD project, the view on them.
fn ground(app: &mut App) {
    let mut o = Objects::new();
    o.path("yol", &[[0.0, 0.0], [20.0, 0.0], [30.0, 8.0]], false);
    o.path(
        "parsel",
        &[[0.0, 4.0], [14.0, 4.0], [14.0, 14.0], [0.0, 14.0]],
        true,
    );
    open(app, o);
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 10.0,
            max_x: E + 34.0,
            max_y: N + 16.0,
        },
        24.0,
    );
}

/// The road selected, the grip menu of its first edge's middle open.
fn edge_menu(app: &mut App) {
    ground(app);
    click(app, [5.0, 0.0]);
    let middle = crate::tools_scenes::at(app, [10.0, 0.0]);
    let _ = app.update(Message::Viewport(crate::viewport::Event::Moved(middle)));
    let _ = app.update(Message::Viewport(crate::viewport::Event::RightPressed(
        middle,
    )));
    let _ = app.update(Message::Viewport(crate::viewport::Event::RightClick(
        middle,
    )));
}

/// Noktadan ya da yarıçapla yay… chosen: the arc through the pointer.
fn edge_arc(app: &mut App) {
    edge_menu(app);
    let _ = app.update(Message::DrawingMenu(Event::Grip(GripAction::ArcBy)));
    hover(app, [10.0, -5.0]);
}

/// Kenar genişliği… chosen, 2 then 0: the edge narrows to its end.
fn edge_width(app: &mut App) {
    edge_menu(app);
    let _ = app.update(Message::DrawingMenu(Event::Grip(GripAction::EdgeWidth)));
    typed(app, "2");
    typed(app, "0");
    let _ = app.update(Message::Run("edit.deselect"));
    hover(app, [26.0, -6.0]);
}

/// Çoklu çizgi: a line, arc mode, Açı 90, Merkez, the centre under the pointer.
fn angle_centre(app: &mut App) {
    ground(app);
    run(app, "tool.polyline");
    click(app, [2.0, -6.0]);
    click(app, [14.0, -6.0]);
    typed(app, "Y");
    typed(app, "A");
    typed(app, "90");
    typed(app, "M");
    hover(app, [14.0, -1.0]);
}

/// Çoklu çizgi with Genişlik 1.5 → 0: an arrow's head after a shaft of 0.4.
fn tapered(app: &mut App) {
    ground(app);
    run(app, "tool.polyline");
    typed(app, "E");
    typed(app, "0.4");
    let _ = app.update(Message::CommandSubmitted);
    click(app, [2.0, -7.0]);
    click(app, [16.0, -7.0]);
    typed(app, "E");
    typed(app, "1.5");
    typed(app, "0");
    hover(app, [21.0, -7.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kenar-menusu", edge_menu),
        ("kenar-yay", edge_arc),
        ("kenar-genislik", edge_width),
        ("cizgi-aci-merkez", angle_centre),
        ("cizgi-genislik", tapered),
    ]
}
