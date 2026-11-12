//! The pictures of docs/adr/0185 (Koordinat yaz) in a CAD project at 1:500:
//! two parcels and their numbered corner points; Koordinat yaz's label at the
//! cursor over a corner after one written, Köşelere koordinat yaz's labels at
//! every corner before Enter, its numbers and the coordinate schedule hanging
//! from the cursor, and the drawing with the labels and the schedule written.
//! The web's are `shots.mjs coordinates`. `tools_screens` takes them in the
//! dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=koordinat-yaz cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use kentos_interaction::Name;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// The parcels' corners: 7, and 8 beside it.
const PARCEL: [[f64; 2]; 5] = [
    [0.0, 0.0],
    [42.5, -3.25],
    [47.0, 24.0],
    [18.0, 31.5],
    [-2.0, 22.0],
];
const NEIGHBOUR: [[f64; 2]; 4] = [[42.5, -3.25], [71.0, -6.0], [74.5, 19.0], [47.0, 24.0]];

/// The parcels and parcel 7's numbered corners (101 …) with elevations.
fn ground() -> Objects {
    let mut o = Objects::new();
    let a = o.path("parsel", &PARCEL, true);
    o.data(a, &[("Ada", "1043"), ("Parsel", "7")], Some("7"));
    let b = o.path("parsel", &NEIGHBOUR, true);
    o.data(b, &[("Ada", "1043"), ("Parsel", "8")], Some("8"));
    for (k, p) in PARCEL.iter().enumerate() {
        let id = o.point("cizim", *p, Some(812.4 + k as f64 * 0.35));
        o.data(id, &[], Some(&format!("{}", 101 + k)));
    }
    o
}

/// The ground in a CAD project at 1:500, the view on the parcels.
fn drawn(app: &mut App) {
    open(app, ground());
    make_cad(app);
    {
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.plot_scale = 500.0;
        doc.model.set_settings(settings);
    }
    app.tab = "annotate";
    app.command_expanded = false;
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 14.0,
            min_y: N - 14.0,
            max_x: E + 82.0,
            max_y: N + 46.0,
        },
        24.0,
    );
}

/// Every object of the ground selected: the parcels and the points.
fn select_ground(app: &mut App) {
    app.selection.set((1..=7).map(Slot));
}

/// Koordinat yaz: 103 written, the cursor over 104 shows its label.
fn label_at_cursor(app: &mut App) {
    drawn(app);
    run(app, "tool.coordinateLabel");
    click(app, PARCEL[2]);
    hover(app, PARCEL[3]);
}

/// Köşelere koordinat yaz over both parcels and the points: every corner's
/// label, away from its parcel, before Enter.
fn every_corner(app: &mut App) {
    drawn(app);
    select_ground(app);
    run(app, "tool.coordinateVertices");
    hover(app, [62.0, 36.0]);
}

/// Numbers at the corners (template {ad}, no leader) and the coordinate
/// schedule of the same objects hanging from the cursor.
fn schedule(app: &mut App) {
    drawn(app);
    app.memory.coordinate_template = Name::new("{ad}").expect("a template");
    app.memory.coordinate_leader = false;
    app.memory.coordinate_schedule = true;
    select_ground(app);
    run(app, "tool.coordinateVertices");
    // Enter: the numbers written, the schedule hangs from the cursor.
    typed(app, "");
    hover(app, [56.0, 44.0]);
}

/// The drawing with every corner's label written, numbered, and the schedule placed.
fn written(app: &mut App) {
    drawn(app);
    select_ground(app);
    run(app, "tool.coordinateVertices");
    typed(app, "");
    app.memory.coordinate_template = Name::new("{ad}").expect("a template");
    app.memory.coordinate_leader = false;
    app.memory.coordinate_schedule = true;
    app.memory.coordinate_direction =
        kentos_geometry_core::ops::coordinate_labels::Direction::SouthWest;
    select_ground(app);
    run(app, "tool.coordinateVertices");
    typed(app, "");
    click(app, [56.0, 44.0]);
    app.selection.set([]);
    hover(app, [-10.0, 40.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("koordinat-yaz", label_at_cursor),
        ("koordinat-koseler", every_corner),
        ("koordinat-cizelge", schedule),
        ("koordinat-cizim", written),
    ]
}
