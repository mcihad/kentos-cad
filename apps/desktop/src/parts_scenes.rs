//! The drawing and the steps of the pictures of docs/adr/0143 (multi-part
//! areas): a parcel of three parts selected, its grips on every part and
//! Öznitelikler's Parça sayısı, the hover card with its Parça row, and
//! Değiştir's Parçaları birleştir on two parcels.
//! `tools_screens` takes them in the dark and the light theme at 1440×900
//! and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=cok-parcali-secim cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::tools_scenes::{Objects, hover, open, run};
use crate::tools_screens::Scene;

/// Parcel 12 of three parts (the largest with a hole), a neighbouring
/// parcel and a road; slots 1 to 3.
pub(crate) fn parts_ground() -> Objects {
    let mut o = Objects::new();
    let parcel = o.parts(
        "parsel",
        &[
            (
                &[[4.0, 4.0], [30.0, 4.0], [30.0, 22.0], [4.0, 22.0]],
                &[&[[12.0, 10.0], [20.0, 10.0], [20.0, 16.0], [12.0, 16.0]]],
            ),
            (&[[36.0, 4.0], [50.0, 4.0], [50.0, 14.0], [36.0, 14.0]], &[]),
            (&[[36.0, 18.0], [44.0, 18.0], [40.0, 26.0]], &[]),
        ],
    );
    o.data(parcel, &[("Ada", "215"), ("Parsel", "12")], Some("215/12"));
    let neighbour = o.path(
        "parsel",
        &[[4.0, 28.0], [30.0, 28.0], [30.0, 40.0], [4.0, 40.0]],
        true,
    );
    o.data(
        neighbour,
        &[("Ada", "215"), ("Parsel", "13")],
        Some("215/13"),
    );
    o.path("yol", &[[0.0, 25.0], [56.0, 25.0]], false);
    o
}

pub(crate) const PARCEL: Slot = Slot(1);
pub(crate) const NEIGHBOUR: Slot = Slot(2);

/// Öznitelikler with the room to show its Geometri rows: the dock's upper
/// stack folded to its header and the drawing's own rows shut.
fn panel_room(app: &mut App) {
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
}

/// The parcel selected, the geometry store following the drawing so that its grips show.
fn select_parcel(app: &mut App) {
    app.selection.set([PARCEL]);
    if let Some(doc) = app.document.as_ref() {
        app.spatial.sync(&doc.model);
    }
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("cok-parcali-secim", |app| {
            open(app, parts_ground());
            select_parcel(app);
            panel_room(app);
            hover(app, [52.0, 34.0]);
        }),
        ("cok-parcali-kart", |app| {
            open(app, parts_ground());
            hover(app, [44.0, 8.0]);
            app.hover_card_due(app.selection.hover_version());
        }),
        // Düzenle's Parçaları birleştir on the parcel and its neighbour: one area of four parts.
        ("parcalari-birlestir", |app| {
            open(app, parts_ground());
            app.tab = "edit";
            app.selection.set([PARCEL, NEIGHBOUR]);
            run(app, "tool.partsJoin");
            panel_room(app);
            hover(app, [52.0, 34.0]);
        }),
    ]
}
