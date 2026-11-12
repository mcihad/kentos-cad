//! Eğimle kot ver › Yüzeyden's Kot farkı and Aralık (grade_surface.rs, docs/adr/4580) on the shared cases' drawing
//! (`fixtures/commands/v1/cad.entities.drape.json`: the surface “Zemin”, its diagonal AC). The polyline (slot 8) with
//! Kot farkı 0.15, Aralık from vertex 1 to 2 and Ara kırık noktaları previews the shared case's counts (2 vertices, 1
//! break point) and Uygula writes the case's vertices 0.15 m over the surface, the last one left, in one undo step; the
//! range is marked on the drawing while the window is open. Docs/adr/4980: on the two-part area (slot 18) Sahneden
//! seç's first click by its second part's hole's vertex chooses that ring, the second its end; the preview and Uygula
//! are the shared case's, the other rings left. And the pictures.

use crate::road;
use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::grade_window::{Event as G, Target};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/cad.entities.drape.json")).expect("cases")
}

/// The polyline (slot 8) selected, Eğimle kot ver on Yüzeyden with Kot farkı 0.15, Aralık 1–2 and break points.
fn ranged() -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.selection.set([Slot(8)]);
    let _ = app.update(Message::Run("tool.grade"));
    let _ = app.update(Message::Road(road::Event::Grade(G::Target(
        Target::Surface,
    ))));
    let _ = app.update(Message::Road(road::Event::Grade(G::Above(0.15))));
    let _ = app.update(Message::Road(road::Event::Grade(G::Partial(true))));
    let _ = app.update(Message::Road(road::Event::Grade(G::From(1.0))));
    let _ = app.update(Message::Road(road::Event::Grade(G::To(2.0))));
    let _ = app.update(Message::Road(road::Event::Grade(G::Breaks(true))));
    app
}

#[test]
fn kot_farki_and_aralik_write_the_range_over_the_surface() {
    let mut app = ranged();
    assert_eq!(app.dialog, Some(Dialog::Road(road::RoadDialogKind::Grade)));
    let plan = app
        .road
        .grade
        .draped
        .clone()
        .expect("a plan")
        .expect("planned");
    assert_eq!((plan.set, plan.outside, plan.inserted), (2, 0, 1));
    // The range is marked: the path from the first vertex to the second and its two ends.
    let (path, ends) = app.grade_marks();
    assert!(!path.is_empty() && ends.len() == 2, "{path:?} {ends:?}");
    let _ = app.update(Message::Road(road::Event::Grade(G::Apply)));
    assert_eq!(app.dialog, None);
    let doc = app.document.as_ref().expect("a drawing");
    let Some(kentos_commands::Entity::Polyline(p)) = doc.model.get(Slot(8)) else { panic!("a polyline") };
    let zs = p.zs.clone().expect("heights");
    // ABC under AC (z = 100 + 0.04x + 0.1y), ACD over it (z = 100 + 0.072x + 0.06y), local from X0, Y0.
    let ground = |x: f64, y: f64| if y <= 0.8 * x { 100.0 + 0.04 * x + 0.1 * y } else { 100.0 + 0.072 * x + 0.06 * y };
    let local: Vec<(f64, f64)> = p.pts.iter().map(|v| (v.x - 500_000.0, v.y - 4_400_000.0)).collect();
    assert_eq!(local.len(), 4);
    for k in 0..3 {
        let z = zs[k].expect("a height");
        assert!((z - (ground(local[k].0, local[k].1) + 0.15)).abs() < 1e-9, "vertex {k}: {z}");
    }
    assert_eq!(zs[3], None, "out of the range");
    let _ = app.update(Message::Run("edit.undo"));
    let doc = app.document.as_ref().expect("a drawing");
    let Some(kentos_commands::Entity::Polyline(p)) = doc.model.get(Slot(8)) else { panic!("a polyline") };
    assert_eq!((p.pts.len(), p.zs.is_none()), (3, true), "one undo step");
}

/// The two-part area (slot 18) selected, Eğimle kot ver on Yüzeyden with Kot farkı 0.15, Aralık and break points,
/// the range picked on the drawing: by its second part's hole's vertices 2 and 4 (65, 35 … 65, 65 from X0, Y0).
fn island() -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.selection.set([Slot(18)]);
    let _ = app.update(Message::Run("tool.grade"));
    let _ = app.update(Message::Road(road::Event::Grade(G::Target(
        Target::Surface,
    ))));
    let _ = app.update(Message::Road(road::Event::Grade(G::Above(0.15))));
    let _ = app.update(Message::Road(road::Event::Grade(G::Partial(true))));
    let _ = app.update(Message::Road(road::Event::Grade(G::Breaks(true))));
    let near = |x: f64, y: f64| Some(kentos_interaction::Vec2 { x: 500_000.0 + x + 0.01, y: 4_400_000.0 + y - 0.01 });
    let _ = app.update(Message::Road(road::Event::Grade(G::Pick)));
    assert!(app.grade_picked(near(85.0, 35.0)));
    assert_eq!(
        (app.road.grade.ring, app.road.grade.from),
        ((1, 1), 2.0),
        "the hole of the second part, its second vertex"
    );
    assert!(app.grade_picked(near(65.0, 65.0)));
    // A real click ends its pick step before the point arrives; called directly here, the step is ended as it would be.
    let _ = app.with_tool(|s, cx| s.cancel(cx));
    app
}

#[test]
fn the_range_lies_on_the_ring_its_first_click_is_nearest() {
    let mut app = island();
    assert_eq!(
        (app.dialog, app.road.grade.to),
        (Some(Dialog::Road(road::RoadDialogKind::Grade)), 4.0)
    );
    let plan = app
        .road
        .grade
        .draped
        .clone()
        .expect("a plan")
        .expect("planned");
    assert_eq!((plan.set, plan.outside, plan.inserted), (3, 0, 1));
    // The hole's range is marked: its path and its two ends.
    let (path, ends) = app.grade_marks();
    assert!(!path.is_empty() && ends.len() == 2, "{path:?} {ends:?}");
    assert!((ends[0].x - 500_085.0).abs() < 1e-9 && (ends[1].y - 4_400_065.0).abs() < 1e-9, "{ends:?}");
    let _ = app.update(Message::Road(road::Event::Grade(G::Apply)));
    assert_eq!(app.dialog, None);
    let doc = app.document.as_ref().expect("a drawing");
    let Some(kentos_commands::Entity::Polygon(p)) = doc.model.get(Slot(18)) else { panic!("an area") };
    let hole = &p.parts.as_ref().expect("parts")[0].holes.as_ref().expect("holes")[0];
    let ground = |x: f64, y: f64| if y <= 0.8 * x { 100.0 + 0.04 * x + 0.1 * y } else { 100.0 + 0.072 * x + 0.06 * y };
    let local: Vec<(f64, f64)> = hole.pts.iter().map(|v| (v.x - 500_000.0, v.y - 4_400_000.0)).collect();
    assert_eq!(local.len(), 5, "the break point on AC");
    let zs = hole.zs.clone().expect("heights");
    assert_eq!(zs[0], None, "out of the range");
    for k in 1..5 {
        let z = zs[k].expect("a height");
        assert!((z - (ground(local[k].0, local[k].1) + 0.15)).abs() < 1e-9, "vertex {k}: {z}");
    }
    assert!(p.zs.is_none() && p.parts.as_ref().expect("parts")[0].zs.is_none(), "the other rings are left");
    let _ = app.update(Message::Run("edit.undo"));
    let doc = app.document.as_ref().expect("a drawing");
    let Some(kentos_commands::Entity::Polygon(p)) = doc.model.get(Slot(18)) else { panic!("an area") };
    let hole = &p.parts.as_ref().expect("parts")[0].holes.as_ref().expect("holes")[0];
    assert_eq!((hole.pts.len(), hole.zs.is_none()), (4, true), "one undo step");
}

/// `.run/gui-evidence/yuzeye-goreli-kot-4580/` and, for the ring of a multi-part area (docs/adr/4980),
/// `.run/gui-evidence/cok-parcali-aralik-4980/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib grade_surface_relative_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yuzeye-goreli-kot-4580");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = ranged();
        let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-kot-farki-aralik-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cok-parcali-aralik-4980");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = island();
        let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        // The second part (50–95, 15–80 from X0, Y0) left of the window, which stands in the middle; once the drawing
        // area has its size.
        let camera = &mut app.viewport.camera;
        camera.scale = if size.width > 1200.0 { 4.0 } else { 2.2 };
        camera.center = kentos_interaction::Vec2::new(
            500_072.5 + (camera.width / 2.0 - camera.width * 0.15) / camera.scale,
            4_400_047.5,
        );
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-delikte-aralik-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
