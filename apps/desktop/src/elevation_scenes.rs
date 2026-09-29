//! The drawing and the steps of the pictures of docs/adr/0142 (vertex
//! elevations): Kot ver at its number step and after its write, Öznitelikler's
//! Kot and 3B rows, the grip's tag, Koordinat oku and the hover card, and the
//! GeoJSON window with `Kotlu nesne`. `tools_screens` takes them (its scenes
//! list adds these) in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kot-ver-adim2 cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_ui::widget::docking;

use crate::app::{App, Message};
use crate::tools_scenes::{Objects, click, forget, hover, open, run, typed};
use crate::tools_screens::Scene;

/// A road axis 40 m long rising 9 m (41 m in space), a kerb with an elevation
/// on every vertex, a footpath measured at only some of its corners, a parcel
/// with a hole and every corner at its height, two survey points and a circle
/// that takes no elevation. Slots 1 to 7 in that order.
pub(crate) fn elevation_ground() -> Objects {
    let mut o = Objects::new();
    o.line_z("yol", [2.0, 8.0], [42.0, 8.0], [Some(100.0), Some(109.0)]);
    o.path_z(
        "yol",
        &[[2.0, 16.0], [14.0, 16.0], [26.0, 22.0], [42.0, 22.0]],
        false,
        &[Some(98.5), Some(101.25), Some(104.0), Some(105.25)],
        &[],
    );
    o.path_z(
        "cizim",
        &[
            [2.0, 28.0],
            [12.0, 28.0],
            [22.0, 32.0],
            [32.0, 32.0],
            [42.0, 30.0],
        ],
        false,
        &[Some(100.0), None, Some(102.5), None, Some(104.0)],
        &[],
    );
    o.path_z(
        "parsel",
        &[[50.0, 4.0], [70.0, 4.0], [70.0, 16.0], [50.0, 16.0]],
        true,
        &[Some(120.0), Some(120.5), Some(121.0), Some(120.5)],
        &[(
            &[[56.0, 8.0], [62.0, 8.0], [62.0, 12.0], [56.0, 12.0]],
            &[Some(120.25); 4],
        )],
    );
    o.point("parsel", [48.0, 24.0], Some(101.3));
    o.point("parsel", [52.0, 24.0], None);
    o.circle("cizim", [58.0, 30.0], 4.0);
    o
}

pub(crate) const ROAD: Slot = Slot(1);
pub(crate) const KERB: Slot = Slot(2);
pub(crate) const FOOTPATH: Slot = Slot(3);
pub(crate) const PARCEL: Slot = Slot(4);
pub(crate) const SPOT: Slot = Slot(5);
pub(crate) const BARE_SPOT: Slot = Slot(6);
pub(crate) const CIRCLE: Slot = Slot(7);

/// Objects selected, as a click and a window would leave them; the geometry store follows the
/// drawing, so that their grips are there to rest on.
fn select(app: &mut App, slots: &[Slot]) {
    app.selection.set(slots.iter().copied());
    if let Some(doc) = app.document.as_ref() {
        app.spatial.sync(&doc.model);
    }
}

/// Öznitelikler with the room to show its Geometri rows: the dock's upper stack (Katmanlar,
/// İşlemler) folded to its header and the drawing's own rows (Genel) shut.
fn panel_room(app: &mut App) {
    fold_upper_stack(app);
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle(
        "general",
    )));
}

/// The dock's upper stack folded to its header: Öznitelikler has the height of the dock.
fn fold_upper_stack(app: &mut App) {
    let upper = docking::Slot::Docked(docking::Side::Right, 0);
    let _ = app.update(Message::Dock(docking::Event::Collapsed(upper, true)));
}

/// The pointer resting on an object long enough for its rollover card to show.
fn rest(app: &mut App, at: [f64; 2]) {
    hover(app, at);
    app.hover_card_due(app.selection.hover_version());
}

/// The GeoJSON window over the sample drawing (TUREF / TM36, like the file), the file read.
fn geojson_window(app: &mut App) {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/formats/v1/gis/kotlu.geojson");
    app.picker = crate::app::Picker::File(path);
    let task = app.run("file.import.geojson");
    crate::files_testing::drive(app, task);
}

/// Kot ver at its steps and after its write, Öznitelikler's rows, the grip's tag, Koordinat oku,
/// the rollover card and the GeoJSON window.
pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kot-ver-adim2", |app| {
            open(app, elevation_ground());
            select(app, &[KERB, FOOTPATH, SPOT]);
            run(app, "tool.setElevation");
            hover(app, [30.0, 40.0]);
        }),
        ("kot-ver-adim2-artir", |app| {
            open(app, elevation_ground());
            select(app, &[KERB, FOOTPATH, SPOT]);
            run(app, "tool.setElevation");
            typed(app, "a");
            hover(app, [30.0, 40.0]);
        }),
        ("kot-ver-sonuc", |app| {
            open(app, elevation_ground());
            select(app, &[ROAD, KERB, FOOTPATH, CIRCLE]);
            run(app, "tool.setElevation");
            forget(app);
            typed(app, "a");
            typed(app, "2.5");
            hover(app, [30.0, 40.0]);
        }),
        ("kot-ver-secim", |app| {
            open(app, elevation_ground());
            run(app, "tool.setElevation");
            click(app, [20.0, 8.0]);
            click(app, [8.0, 16.0]);
            hover(app, [30.0, 40.0]);
        }),
        ("oznitelikler-cizgi-kot", |app| {
            open(app, elevation_ground());
            select(app, &[ROAD]);
            panel_room(app);
        }),
        ("oznitelikler-coklu-cizgi-kot", |app| {
            open(app, elevation_ground());
            select(app, &[FOOTPATH]);
            panel_room(app);
        }),
        ("oznitelikler-coklu-cizgi-aralik", |app| {
            open(app, elevation_ground());
            select(app, &[KERB]);
            panel_room(app);
        }),
        ("oznitelikler-alan-kot", |app| {
            open(app, elevation_ground());
            select(app, &[PARCEL]);
            panel_room(app);
        }),
        ("oznitelikler-coklu-kot", |app| {
            open(app, elevation_ground());
            select(app, &[ROAD, KERB, FOOTPATH]);
            fold_upper_stack(app);
        }),
        ("tutamac-kot", |app| {
            open(app, elevation_ground());
            select(app, &[KERB]);
            hover(app, [26.0, 22.0]);
        }),
        ("tutamac-kot-tasima", |app| {
            open(app, elevation_ground());
            select(app, &[KERB]);
            click(app, [26.0, 22.0]);
            hover(app, [30.0, 27.0]);
        }),
        ("koordinat-oku-kot", |app| {
            open(app, elevation_ground());
            run(app, "crs.query");
            click(app, [26.0, 22.0]);
            click(app, [12.0, 28.0]);
            click(app, [2.0, 8.0]);
            hover(app, [42.0, 22.0]);
        }),
        ("kart-3b-uzunluk", |app| {
            open(app, elevation_ground());
            rest(app, [20.0, 8.0]);
        }),
        ("kart-3b-cevre", |app| {
            open(app, elevation_ground());
            rest(app, [52.0, 10.0]);
        }),
        ("geojson-kotlu", geojson_window),
    ]
}
