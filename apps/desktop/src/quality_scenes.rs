//! ADV-07's regression scenes (docs/adr/0776 §3): the apartment plan of the walkthrough far from the origin
//! (E 487 000, N 4 420 000) with millimetre details, seen at the screen scales 1:20, 1:50 and 1:100 (its
//! dimensions, tags and walls, the walls' detail by the zoom; ADR 0696), the walls' components through a T at 1:20,
//! and the tests that hold what the pictures show by number: a millimetre told apart and kept through a long pan,
//! the drawing's unit, its drawing scale and the screen's zoom kept apart (docs/adr/0626 unchanged). `tools_screens`
//! takes the scenes in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kalite-ayrinti-1mm,kalite-1-100,kalite-1-50,kalite-1-20,kalite-bilesen-1-20 cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::App;
use crate::tools_scenes::{E, N, Objects, click, hover, run, typed};
use crate::tools_screens::Scene;
use crate::wall_scenes::opened;

/// A 12 × 9 m shell of 25 cm walls at E, N with two 30 cm lines a millimetre apart inside its south-west corner.
fn detail(app: &mut App) {
    let mut objects = Objects::new();
    objects.line("cizim", [0.30, 0.300], [0.60, 0.300]);
    objects.line("cizim", [0.30, 0.301], [0.60, 0.301]);
    opened(app, objects, 100.0);
    crate::arch_scenes::on_level(app);
    app.command_expanded = true;
    run(app, "tool.wallRectangle");
    typed(app, "E");
    typed(app, "0.25m");
    click(app, [0.0, 0.0]);
    click(app, [12.0, 9.0]);
    typed(app, "");
    app.selection.clear();
    // 0.1 mm a pixel: the millimetre is ten pixels, the wall's corner beside it.
    app.viewport.camera.center_on(kentos_interaction::Vec2::new(E + 0.40, N + 0.3005));
    app.viewport.camera.scale = 10_000.0;
    hover(app, [0.45, 0.2]);
}

/// The same at a screen scale of 1:`n` round the corner.
fn detail_at(app: &mut App, n: f64, centre: [f64; 2]) {
    detail(app);
    app.viewport.camera.center_on(kentos_interaction::Vec2::new(E + centre[0], N + centre[1]));
    app.viewport.camera.zoom_to_screen_scale(n);
    hover(app, [0.45, 0.2]);
}

/// The walkthrough's dimensioned plan at a screen scale of 1:`n`, round its south-west room.
fn at_scale(app: &mut App, n: f64) {
    crate::walkthrough_scenes::dimensions(app);
    app.viewport.camera.center_on(kentos_interaction::Vec2::new(E + 2.0, N + 0.5));
    app.viewport.camera.zoom_to_screen_scale(n);
    hover(app, [14.0, 11.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kalite-ayrinti-1mm", detail),
        // Görünüm › Çizgi kalınlığı off: every line a pixel, the millimetre ten pixels apart (docs/adr/0776 §3).
        ("kalite-ayrinti-1mm-ince", |app| {
            let _ = app.settings.choose(&[("graphics.lineWeights", serde_json::json!(false))]);
            app.apply_settings();
            detail(app);
        }),
        ("kalite-ayrinti-1-5", |app| detail_at(app, 5.0, [0.25, 0.25])),
        ("kalite-ayrinti-1-1", |app| detail_at(app, 1.0, [0.40, 0.30])),
        ("kalite-1-100", |app| at_scale(app, 100.0)),
        ("kalite-1-50", |app| at_scale(app, 50.0)),
        ("kalite-1-20", |app| at_scale(app, 20.0)),
        ("kalite-bilesen-1-20", |app| {
            crate::wall_scenes::joined_room(app, 20.0);
            app.viewport.camera.center_on(kentos_interaction::Vec2::new(E + 4.2, N + 0.1));
            app.viewport.camera.zoom_to_screen_scale(20.0);
            hover(app, [6.0, 3.0]);
        }),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_interaction::Vec2;
    use serde_json::json;

    /// docs/adr/0776 §3: at E 487 000, N 4 420 000 a millimetre is ten pixels at 0.1 mm a pixel, the screen gives
    /// the world back within a micrometre, a click picks the nearer of two lines a millimetre apart, and a
    /// kilometre's pan out and back leaves the view where it was to the bit.
    #[test]
    fn a_millimetre_far_from_the_origin_is_drawn_picked_and_kept_through_a_pan() {
        let (mut app, _) = App::boot(None);
        detail(&mut app);
        let camera = app.viewport.camera;
        let a = camera.world_to_screen(Vec2::new(E + 0.45, N + 0.300));
        let b = camera.world_to_screen(Vec2::new(E + 0.45, N + 0.301));
        assert!(((a[1] - b[1]).abs() - 10.0).abs() < 1e-4, "a millimetre is ten pixels: {a:?} {b:?}");
        for (x, y) in [(E + 0.3, N + 0.3), (E + 12.125, N + 9.125), (E - 0.125, N + 4.5)] {
            let [sx, sy] = camera.world_to_screen(Vec2::new(x, y));
            let back = camera.screen_to_world(sx, sy);
            assert!((back.x - x).abs() < 1e-6 && (back.y - y).abs() < 1e-6, "{x},{y} → {back:?}");
        }
        for (y, want) in [(N + 0.3002, 1), (N + 0.3008, 2)] {
            let hits = app.spatial.hits(Vec2::new(E + 0.45, y), 0.0004);
            assert_eq!(hits.first().copied(), Some(kentos_domain::Slot(want)), "at {y}: {hits:?}");
        }
        let before = app.viewport.camera.center;
        let mut cam = app.viewport.camera;
        cam.pan_by(1_000_000.0, -500_000.0);
        cam.pan_by(-1_000_000.0, 500_000.0);
        assert_eq!(cam.center, before, "a pan out and back is the view it left");
    }

    /// docs/adr/0776 §3, 0626: the screen's zoom changes neither the drawing nor its settings; the drawing scale
    /// changes the annotations' paper heights only, never a wall, an opening, a room or a slab; the drawing's unit
    /// changed with its lengths kept changes no coordinate.
    #[cfg(unix)]
    #[test]
    fn the_unit_the_drawing_scale_and_the_screens_zoom_stay_apart() {
        let (mut app, _dir) = crate::acceptance_flat_tests::opened("kalite-olcek");
        let _ = crate::acceptance_flat_tests::drawn(&mut app);
        let model = |app: &App| -> Vec<serde_json::Value> {
            let doc = &app.document.as_ref().expect("a drawing").model;
            doc.entities()
                .filter(|e| matches!(e.kind(), "wall" | "opening" | "space" | "slab" | "insert"))
                .map(|e| serde_json::to_value(e).expect("json"))
                .collect()
        };
        let settings = |app: &App| app.document.as_ref().expect("a drawing").model.settings().clone();
        let (geometry, kept) = (model(&app), settings(&app));
        for n in [20.0, 50.0, 100.0, 500.0] {
            app.viewport.camera.zoom_to_screen_scale(n);
            app.viewport.camera.pan_by(3.0, -2.0);
        }
        assert_eq!(model(&app), geometry, "the zoom drew, wrote nothing");
        assert_eq!(settings(&app), kept, "nor set anything");
        let answer = app
            .agents_answer(&json!({ "id": 1, "method": "run", "params": {
                "command": "cad.settings.set", "op": "execute", "input": { "plotScale": 50 } } }))
            .expect("an answer");
        assert_eq!(answer["status"], "completed", "{answer}");
        assert_eq!(model(&app), geometry, "the drawing scale moved no wall, opening, room or slab");
        assert_eq!(settings(&app).plot_scale, 50.0);
        let answer = app
            .agents_answer(&json!({ "id": 1, "method": "run", "params": {
                "command": "cad.units.change", "op": "execute", "input": { "from": "m", "to": "cm", "geometry": "keep" } } }))
            .expect("an answer");
        assert_eq!(answer["status"], "completed", "{answer}");
        assert_eq!(model(&app), geometry, "the unit changed, every length kept");
        assert_eq!(settings(&app).drawing_unit, Some(kentos_commands::DrawingUnit::Cm));
    }
}
