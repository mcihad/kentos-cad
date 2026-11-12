//! ADV-07's regression scenes (docs/adr/0776 §3): the apartment plan of the walkthrough far from the origin
//! (E 487 000, N 4 420 000) with millimetre details, seen at the screen scales 1:20, 1:50 and 1:100 (its
//! dimensions, tags and walls, the walls' detail by the zoom; ADR 0696), the walls' components through a T at 1:20,
//! and the tests that hold what the pictures show by number: a millimetre told apart and kept through a long pan,
//! the drawing's unit, its drawing scale and the screen's zoom kept apart (docs/adr/0626 unchanged). `tools_screens`
//! takes the scenes in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kalite-ayrinti-1mm,kalite-1-100,kalite-1-50,kalite-1-20,kalite-bilesen-1-20,yazi-100-daire,yazi-100-daire-iki,yazi-100-daire-cizelge cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
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

/// The 2+1 apartment of the acceptance flow copied into 100 with its two schedules each (200 schedules, as
/// `perf::flat`'s `each`), then the design change of the measures (Bölme 3's distance) and the drawing followed as
/// the app's update does: the text layer kept object by object after an edit (docs/adr/0786).
#[cfg(unix)]
fn hundred_flats(app: &mut App) {
    use crate::app::Message;
    use serde_json::{Value, json};
    let ask = |app: &mut App, params: Value| {
        let answer = app
            .agents_answer(&json!({ "id": 1, "method": "run", "params": params }))
            .unwrap_or_else(|(code, message)| panic!("{code}: {message}"));
        assert_eq!(answer["status"], "completed", "{answer}");
        answer
    };
    let scenario: Value =
        serde_json::from_str(crate::acceptance_flat_tests::SCENARIO).expect("the scenario");
    let dir = crate::files_testing::scratch("yazi-100-daire");
    let choices = kentos_headless::new_drawing(&scenario["project"]).expect("the choices");
    let made = kentos_headless::Session::new(&choices).expect("a new project");
    let path = dir.join("daire.kcad");
    std::fs::write(&path, made.to_bytes().expect("its bytes")).expect("written");
    let doc = crate::document::Document::read(&path).expect("opens");
    let task = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::files_testing::drive(app, task);
    // The commands go as the agent link's (as `acceptance_flat_tests::opened`): it must be up.
    let (link, _task) = crate::python::link::Link::open(&dir.join("m.sock"), |heard| {
        Message::Python(crate::python::Event::Agent(heard))
    })
    .expect("the agent link");
    app.python.link = Some(link);
    let names = crate::acceptance_flat_tests::drawn(app);
    let all = |app: &App| -> Vec<String> {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.entities_with_uids().map(|(u, _)| u.to_string()).collect()
    };
    let one = all(app);
    for c in 1..10 {
        ask(app, json!({ "command": "cad.entities.transform", "op": "execute", "input": {
            "uids": one, "transform": { "kind": "move", "dx": 14.0 * f64::from(c), "dy": 0.0 }, "copy": true } }));
    }
    let row = all(app);
    for r in 1..10 {
        ask(app, json!({ "command": "cad.entities.transform", "op": "execute", "input": {
            "uids": row, "transform": { "kind": "move", "dx": 0.0, "dy": 12.0 * f64::from(r) }, "copy": true } }));
    }
    app.follow_document();
    ask(app, json!({ "command": "cad.walls.edit", "op": "execute", "input": {
        "uids": [names["p3"]], "operation": { "kind": "distance", "side": "right", "distance": 3.8 } } }));
    app.follow_document();
    app.selection.clear();
}

/// The hundred flats after the change, the command history folded: the whole plan, or round `centre` at 1:`n`.
#[cfg(unix)]
fn hundred_at(app: &mut App, view: Option<([f64; 2], f64)>) {
    hundred_flats(app);
    app.command_expanded = false;
    match view {
        Some((centre, n)) => {
            app.viewport.camera.center_on(kentos_interaction::Vec2::new(centre[0], centre[1]));
            app.viewport.camera.zoom_to_screen_scale(n);
        }
        None => app.viewport.camera.fit(
            &kentos_render_wgpu::Bounds { min_x: -3.0, min_y: -15.0, max_x: 150.0, max_y: 118.0 },
            24.0,
        ),
    }
}

/// docs/adr/0793 §2: the tagged room (walls, openings, room, their tags) copied 14 m east with Kopyala: each copied
/// tag names its object's copy, so the copy reads its own number and area, and the room's label is not drawn twice.
fn copied_room(app: &mut App) {
    crate::tag_scenes::tagged(app);
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let uids: Vec<String> = doc
        .entities()
        .filter(|e| matches!(e.kind(), "wall" | "opening" | "space" | "tag"))
        .filter_map(|e| doc.uid(kentos_domain::Slot(e.base().id)))
        .map(|u| u.to_string())
        .collect();
    let input: kentos_commands::EntitiesTransform = serde_json::from_value(serde_json::json!({
        "uids": uids, "transform": { "kind": "move", "dx": 14.0, "dy": 0.0 }, "copy": true
    }))
    .expect("a copy");
    let done = kentos_native_application::transform::execute(
        &mut kentos_native_application::ExecutionContext::new(doc),
        input,
    );
    assert!(matches!(done, kentos_commands::CommandResult::Completed { .. }), "{done:?}");
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 1.5,
            min_y: N - 2.5,
            max_x: E + 28.0,
            max_y: N + 9.0,
        },
        24.0,
    );
    hover(app, [27.0, -2.0]);
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
        ("kalite-etiket-kopya", copied_room),
        ("kalite-1-100", |app| at_scale(app, 100.0)),
        ("kalite-1-50", |app| at_scale(app, 50.0)),
        ("kalite-1-20", |app| at_scale(app, 20.0)),
        ("kalite-bilesen-1-20", |app| {
            crate::wall_scenes::joined_room(app, 20.0);
            app.viewport.camera.center_on(kentos_interaction::Vec2::new(E + 4.2, N + 0.1));
            app.viewport.camera.zoom_to_screen_scale(20.0);
            hover(app, [6.0, 3.0]);
        }),
        // The hundred flats and their 200 schedules after Bölme 3 moved (docs/adr/0786): the whole plan, two flats
        // with their schedules, a schedule's rows close.
        #[cfg(unix)]
        ("yazi-100-daire", |app| hundred_at(app, None)),
        #[cfg(unix)]
        ("yazi-100-daire-iki", |app| hundred_at(app, Some(([14.0, -2.0], 200.0)))),
        #[cfg(unix)]
        ("yazi-100-daire-cizelge", |app| hundred_at(app, Some(([24.0, 114.0], 50.0)))),
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

    /// docs/adr/0793 §1: at 0.1 mm a pixel (1:0.38) the drawing area holds every paper width of the 1:100 detail to
    /// its paper width at 1:100, so the walls' 0.5 mm cut lines stay a couple of pixels and the millimetre between
    /// the two lines (ten pixels) stays clear; a sheet's map holds nothing. Before, a 0.25 mm pen was 25 mm in the
    /// world and drew 250 pixels wide here.
    #[test]
    fn a_paper_width_is_held_where_the_millimetre_is_ten_pixels() {
        use kentos_native_style::batches::{BatchKind, FillPaintBatch, Unit};
        let (mut app, _) = App::boot(None);
        detail(&mut app);
        let doc = app.document.as_ref().expect("a drawing");
        app.spatial.sync(&doc.model);
        let palette = crate::viewport::palette(app.canvas());
        let styling = app.styles.styling(&app.spatial, &app.settings);
        let scene = app.viewport.styled_scene(doc, &palette, styling);
        let held = 1.0 / (kentos_native_style::program::METRES_PER_PX * 100.0);
        let px_per_m = app.viewport.camera.scale;
        assert!(px_per_m > 25.0 * held, "zoomed far past 1:100: {px_per_m}");
        let (mut widths, mut wide) = (0, 0.0_f64);
        for part in &scene.layers {
            for b in &part.layer.batches {
                let width = match &b.kind {
                    BatchKind::Stroke { width, unit: Unit::World, .. } => *width,
                    BatchKind::Fill { paint: FillPaintBatch::Hatch { width, unit: Unit::World, .. } } => *width,
                    _ => continue,
                };
                if width <= 0.0 {
                    continue;
                }
                assert!((b.weight_cap - held).abs() < 1e-9, "a paper width held to 1:100's: {}", b.weight_cap);
                let px = width * px_per_m.min(b.weight_cap);
                assert!(width * px_per_m > 100.0, "the world would draw it {} px", width * px_per_m);
                wide = wide.max(px);
                widths += 1;
            }
        }
        assert!(widths > 0, "the walls and lines draw paper widths");
        assert!(wide < 5.0, "the widest paper width is {wide} px: the millimetre's ten pixels stay clear");
    }

    /// docs/adr/0793 §2: Kopyala on a tagged room: every copied tag names its own object's copy, every original tag
    /// its original, and each copy's words are its object's.
    #[test]
    fn a_copied_room_tags_its_own_copies() {
        let mut app = crate::files_testing::app_with_drawing();
        app.window_size = iced::Size::new(1440., 900.);
        let mut shot = kentos_ui::snapshot::Snapshot::software(app.window_size).expect("a renderer");
        shot.settle(&mut app, App::view, &mut |app, message| {
            let _ = app.update(message);
        });
        crate::tag_scenes::tagged(&mut app);
        let first = app.document.as_ref().expect("a drawing").model.entities().map(|e| e.base().id).max();
        copied_room(&mut app);
        let doc = &app.document.as_ref().expect("a drawing").model;
        let first = first.expect("a drawing with objects");
        let slot_of = |uid: &str| {
            kentos_domain::Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u)).map(|s| s.0)
        };
        let mut tags = 0;
        for e in doc.entities() {
            let kentos_commands::Entity::Tag(t) = e else { continue };
            let object = slot_of(&t.tag.of).expect("a tag's object in the drawing");
            assert_eq!(object > first, t.base.id > first, "tag {} names {}", t.base.id, object);
            if t.base.id > first {
                let original = doc
                    .entities()
                    .find_map(|o| match o {
                        kentos_commands::Entity::Tag(o)
                            if o.base.id <= first
                                && (o.tag.at.x + 14.0 - t.tag.at.x).abs() < 1e-9
                                && (o.tag.at.y - t.tag.at.y).abs() < 1e-9 =>
                        {
                            Some(o.tag.text.clone())
                        }
                        _ => None,
                    })
                    .expect("its original");
                assert_eq!(t.tag.text, original, "the copy reads as its original");
                tags += 1;
            }
        }
        assert!(tags >= 3, "the room's and its openings' tags copied: {tags}");
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
