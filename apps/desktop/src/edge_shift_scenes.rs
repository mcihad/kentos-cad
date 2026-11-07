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
//! KENTOS_SHOTS_ONLY=paralel-kaydir-ortak-sinir,paralel-kaydir-ortak-sinir-yazildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! docs/adr/1960: two parcels sharing an edge and a road line ending at its north corner, Topolojik düzenleme on:
//! the west parcel's east edge 3 m out drags the neighbour's corners and the road's end with it, the preview draws them
//! and the neighbour's new size, and the two areas' sum stays.
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
    // docs/adr/1960: two parcels sharing the edge x = 85 (right of the kerb's view), a road line ending at its corner.
    o.path("parsel", &[[70.0, -10.0], [85.0, -10.0], [85.0, 10.0], [70.0, 10.0]], true);
    o.path("parsel", &[[85.0, -10.0], [100.0, -10.0], [100.0, 10.0], [85.0, 10.0]], true);
    o.path("yol", &[[85.0, 10.0], [85.0, 22.0]], false);
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

/// docs/adr/1960: Topolojik düzenleme on, the west parcel's east edge picked inside it and the cursor 3 m east.
fn shared_shown(app: &mut App) {
    opened_over(app, [62.0, -16.0, 108.0, 26.0]);
    run(app, "draft.topology");
    click(app, [84.9, 0.0]);
    hover(app, [88.0, 0.0]);
}

/// The edge 3 m out, written: one step, the neighbour and the road's end with it.
fn shared_written(app: &mut App) {
    shared_shown(app);
    typed(app, "3");
    hover(app, [104.0, -14.0]);
}

/// The areas of the two parcels sharing x = 85, west first.
fn pair(app: &App) -> (f64, f64) {
    use kentos_geometry_core::ops::edge_shift::area_of;
    let doc = app.document.as_ref().expect("a drawing");
    let mut areas: Vec<(f64, f64)> = doc
        .model
        .entities()
        .filter(|e| matches!(e, kentos_commands::Entity::Polygon(_)))
        .map(kentos_native_application::geometry::shape)
        .filter_map(|s| {
            let west = kentos_geometry_core::entity::entity_bounds(&s).min_x - E;
            (west >= 69.0).then(|| (west, area_of(&s).unwrap_or(0.0)))
        })
        .collect();
    areas.sort_by(|a, b| a.0.total_cmp(&b.0));
    (areas[0].1, areas[1].1)
}

#[test]
fn a_shared_edge_shifted_drags_its_neighbour_and_keeps_the_sum() {
    let mut app = App::boot(None).0;
    shared_shown(&mut app);
    let (west, east) = pair(&app);
    assert!((west - 300.0).abs() < 1e-6 && (east - 300.0).abs() < 1e-6, "{west} {east}");
    typed(&mut app, "3");
    let (west, east) = pair(&app);
    let moved = (west - 300.0).abs();
    assert!((moved - 60.0).abs() < 1e-6, "one parcel grew or shrank by 3 m × 20 m: {west} {east}");
    assert!((west + east - 600.0).abs() < 1e-6, "the sum stays: {west} + {east}");
    let said = app.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(said.contains("Topolojik düzenleme: 2 komşu nesne de değişti."), "{said}");
    // One undo step takes all three back.
    let _ = app.update(crate::app::Message::Run("edit.undo"));
    let (west, east) = pair(&app);
    assert!((west - 300.0).abs() < 1e-6 && (east - 300.0).abs() < 1e-6, "undone together: {west} {east}");
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("paralel-kaydir-ortak-sinir", shared_shown),
        ("paralel-kaydir-ortak-sinir-yazildi", shared_written),
        ("paralel-kaydir-yay", arc_shown),
        ("paralel-kaydir-yay-yazildi", arc_written),
        ("paralel-kaydir", shown),
        ("paralel-kaydir-alan", area),
        ("paralel-kaydir-yazildi", written),
    ]
}
