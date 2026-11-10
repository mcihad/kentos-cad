//! Eğimle kot ver (grade_window.rs, docs/adr/1290): the shared cases' polyline (a straight and an arc edge) selected,
//! the window previews 2 % from its own start elevation; Uygula writes one undo step; by end elevation the grade
//! follows; a text selected is refused; Harita and Yol offer it, GIS and Mimari do not. And its pictures. The high or
//! low point (docs/adr/3380): the preview says where it falls and rings it; Uygula adds the vertex in one undo step.
//! Docs/adr/4980: on the two-part area (slot 6) Halka chooses the second part's hole; the range, the preview and Uygula
//! are that ring's, the other rings left; and its pictures.

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::grade_window::Event as G;

fn opened(program: Option<Module>) -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.entities.grade.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn zs(app: &App) -> Vec<Option<f64>> {
    match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
        Some(kentos_commands::Entity::Polyline(p)) => p.zs.clone().unwrap_or_default(),
        _ => panic!("the polyline"),
    }
}

#[test]
fn a_grade_previews_and_writes_one_step() {
    let mut app = opened(Some(Module::Map));
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("tool.grade"));
    assert_eq!(app.dialog, Some(Dialog::Grade));
    let p = app.grade.plan.clone().expect("a plan").expect("previewed");
    assert!((p.grade - 0.02).abs() < 1e-12 && (p.start - 850.0).abs() < 1e-12);
    let _ = app.update(Message::Grade(G::Apply));
    let z = zs(&app);
    assert!((z[4].expect("graded") - (850.0 + 0.02 * p.length)).abs() < 1e-9);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(zs(&app)[4], None, "one step");
    // By the end's elevation: the grade follows.
    let _ = app.update(Message::Run("tool.grade"));
    let _ = app.update(Message::Grade(G::Target(crate::grade_window::Target::End)));
    let _ = app.update(Message::Grade(G::End(851.0)));
    let p = app.grade.plan.clone().expect("a plan").expect("previewed");
    assert!((p.grade - 1.0 / p.length).abs() < 1e-12);
    // A text is no path.
    let _ = app.update(Message::Grade(G::Close));
    app.selection.set([Slot(5)]);
    let _ = app.update(Message::Run("tool.grade"));
    assert_ne!(app.dialog, Some(Dialog::Grade));
}

#[test]
fn harita_and_yol_offer_it_gis_and_mimari_do_not() {
    for (module, yes) in [(Module::Map, true), (Module::Road, true), (Module::Gis, false), (Module::Architecture, false)] {
        assert_eq!(opened(Some(module)).offers("tool.grade"), yes, "{module:?}");
    }
}

/// `.run/gui-evidence/egimle-kot-1290/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib grade_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/egimle-kot-1290");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        app.selection.set([Slot(1)]);
        app.follow_selection_tab();
        let _ = app.update(Message::Run("tool.grade"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("onizleme-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// Sahneden seç (docs/adr/2180): two clicks take the range, each the path's vertex nearest it within the pick
/// aperture; the window opens again with the range previewed; a click by no vertex is said, Esc goes back.
#[test]
fn two_clicks_on_the_drawing_take_the_range() {
    let mut app = opened(Some(Module::Map));
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("tool.grade"));
    let pts = match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
        Some(kentos_commands::Entity::Polyline(p)) => p.pts.clone(),
        _ => panic!("the polyline"),
    };
    let near = |i: usize| Some(kentos_interaction::Vec2 { x: pts[i].x + 0.01, y: pts[i].y - 0.01 });
    let _ = app.update(Message::Grade(G::Pick));
    assert_eq!((app.dialog, app.grade.picking), (None, Some(crate::grade_window::Pick::Start)));
    assert!(app.grade_picked(near(1)));
    assert_eq!((app.grade.from, app.grade.picking), (2.0, Some(crate::grade_window::Pick::End)), "the second click awaited");
    assert!(app.grade_picked(near(3)));
    assert_eq!((app.grade.from, app.grade.to, app.dialog), (2.0, 4.0, Some(Dialog::Grade)));
    let p = app.grade.plan.clone().expect("a plan").expect("previewed");
    assert_eq!(p.zs.len(), 3, "vertices 2 to 4");
    // A click by no vertex: said, the range kept; Esc: back to the window.
    let _ = app.update(Message::Grade(G::Pick));
    assert!(app.grade_picked(Some(kentos_interaction::Vec2 { x: pts[0].x + 500.0, y: pts[0].y + 500.0 })));
    assert_eq!((app.grade.from, app.dialog), (2.0, Some(Dialog::Grade)));
    let _ = app.update(Message::Grade(G::Pick));
    assert!(app.grade_picked(None));
    assert_eq!((app.grade.from, app.grade.to, app.dialog), (2.0, 4.0, Some(Dialog::Grade)));
    // Nothing awaited: another window's point.
    assert!(!app.grade_picked(near(0)));
}

/// The window after Sahneden seç took vertices 2 to 4, light wide, `.run/gui-evidence/egimle-kot-2180/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib grade_window_tests::pick_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn pick_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/egimle-kot-2180");
    std::fs::create_dir_all(&out).expect("a folder");
    let size = Size::new(1440.0, 900.0);
    let mut app = opened(Some(Module::Map));
    let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from("light"))]);
    app.apply_settings();
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Run("view.zoomExtents"));
    app.selection.set([Slot(1)]);
    app.follow_selection_tab();
    let _ = app.update(Message::Run("tool.grade"));
    let pts = match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
        Some(kentos_commands::Entity::Polyline(p)) => p.pts.clone(),
        _ => panic!("the polyline"),
    };
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    // The first click awaited: the drawing with the prompt.
    let _ = app.update(Message::Grade(G::Pick));
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("1-baslangic-koseye-tiklayin-acik-genis.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
    let _ = app.grade_picked(Some(kentos_interaction::Vec2 { x: pts[1].x, y: pts[1].y }));
    let _ = app.grade_picked(Some(kentos_interaction::Vec2 { x: pts[3].x, y: pts[3].y }));
    // A real click ends its pick step before the point arrives; called directly here, the step is ended as it would be.
    let _ = app.with_tool(|s, cx| s.cancel(cx));
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("2-aralik-secildi-acik-genis.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
}

/// The range on the drawing (docs/adr/2980): open, the path from vertex 2 to 4 (its arc as points along it) and the two
/// vertices; while the end is being picked the start alone, while the start is nothing; closed, nothing.
#[test]
fn the_range_is_marked_on_the_drawing() {
    let mut app = opened(Some(Module::Map));
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("tool.grade"));
    let pts = match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
        Some(kentos_commands::Entity::Polyline(p)) => p.pts.clone(),
        _ => panic!("the polyline"),
    };
    // The whole path at first: its first and last vertices.
    let (path, ends) = app.grade_marks();
    assert_eq!(ends, [pts[0], *pts.last().expect("a last vertex")]);
    assert!(path.len() > pts.len(), "the arc's points along it: {}", path.len());
    let _ = app.update(Message::Grade(G::From(2.0)));
    let _ = app.update(Message::Grade(G::To(4.0)));
    let (path, ends) = app.grade_marks();
    assert_eq!(ends, [pts[1], pts[3]]);
    assert_eq!((path.first(), path.last()), (Some(&pts[1]), Some(&pts[3])));
    // The marks layer rings the two vertices.
    let marked = app.marked_points();
    assert!(marked.iter().any(|p| p.x == pts[1].x && p.y == pts[1].y) && marked.iter().any(|p| p.x == pts[3].x && p.y == pts[3].y));
    // Picking: the start awaited, nothing; the end awaited, the start alone.
    let _ = app.update(Message::Grade(G::Pick));
    assert_eq!(app.grade_marks(), (Vec::new(), Vec::new()));
    let near = |i: usize| Some(kentos_interaction::Vec2 { x: pts[i].x + 0.01, y: pts[i].y - 0.01 });
    assert!(app.grade_picked(near(1)));
    assert_eq!(app.grade_marks(), (Vec::new(), vec![pts[1]]));
    assert!(app.grade_picked(near(2)));
    // Closed: nothing.
    let _ = app.update(Message::Grade(G::Close));
    assert_eq!(app.grade_marks(), (Vec::new(), Vec::new()));
}

/// The range 2–4 marked on the drawing beside the window, light wide and dark narrow, `.run/gui-evidence/aralik-2980/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib grade_window_tests::range_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn range_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/aralik-2980");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 650.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.selection.set([Slot(1)]);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        // The first frame fits the drawing; then the polyline in the left third, where the window does not cover it.
        snapshot.settle(&mut app, App::view, &mut update);
        let pts = match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
            Some(kentos_commands::Entity::Polyline(p)) => p.pts.clone(),
            _ => panic!("the polyline"),
        };
        let (x0, x1) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
        let (y0, y1) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.y), b.max(p.y)));
        let w = x1 - x0 + 10.0;
        app.viewport.camera.fit(
            &kentos_render_wgpu::Bounds { min_x: x0 - 5.0, min_y: y0 - 15.0, max_x: x0 - 5.0 + 3.2 * w, max_y: y1 + 15.0 },
            24.0,
        );
        let _ = app.update(Message::Run("tool.grade"));
        let _ = app.update(Message::Grade(G::From(2.0)));
        let _ = app.update(Message::Grade(G::To(4.0)));
        // The window keeps its path by id: the selection's own colour off, the range stands out.
        app.selection.set(Vec::new());
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("aralik-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// Yüksek/alçak nokta (docs/adr/3380): vertices 1 to 4 of the shared polyline, its own 851.25 m at the end, 4 % from the
/// start and 1 % from the end; the preview is the shared case's point (a new vertex between the 2nd and 3rd), ringed
/// with the range's ends; Uygula adds it, one undo takes it back; parallel grades are said and Uygula stays off.
#[test]
fn a_high_point_previews_ringed_and_adds_a_vertex() {
    use crate::grade_window::Target;
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.entities.grade.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Yüksek nokta düz"))).expect("the case");
    let want = &case["steps"][0]["result"]["output"];
    let mut app = opened(Some(Module::Map));
    app.selection.set([Slot(1)]);
    let _ = app.update(Message::Run("tool.grade"));
    for e in [G::To(4.0), G::Target(Target::Peak), G::End(851.25), G::Ahead(4.0), G::Back(1.0)] {
        let _ = app.update(Message::Grade(e));
    }
    let p = app.grade.plan.clone().expect("a plan").expect("previewed");
    let at = p.point.clone().expect("a point");
    assert_eq!((at.index, at.inserted, at.kind), (2, true, kentos_commands::GradePointKind::High));
    assert!((at.x - want["point"]["x"].as_f64().expect("x")).abs() < 1e-6 && (at.z - want["point"]["z"].as_f64().expect("z")).abs() < 1e-9);
    // Ringed with the range's ends.
    let (_, ringed) = app.grade_marks();
    assert_eq!(ringed.len(), 3);
    assert!((ringed[2].x - at.x).abs() < 1e-9 && (ringed[2].y - at.y).abs() < 1e-9);
    let _ = app.update(Message::Grade(G::Apply));
    let pts = |app: &App| match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
        Some(kentos_commands::Entity::Polyline(p)) => p.pts.len(),
        _ => panic!("the polyline"),
    };
    assert_eq!(pts(&app), 6, "the vertex added");
    assert!((zs(&app)[2].expect("its elevation") - at.z).abs() < 1e-9);
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(pts(&app), 5, "one step");
    // Parallel grades: said, nothing to apply.
    let _ = app.update(Message::Run("tool.grade"));
    for e in [G::To(4.0), G::Target(Target::Peak), G::Ahead(2.0), G::Back(-2.0)] {
        let _ = app.update(Message::Grade(e));
    }
    let Some(Err(why)) = app.grade.plan.clone() else { panic!("refused") };
    assert!(why.contains("paralel"), "{why}");
}

/// The high point previewed beside the window, light wide and dark narrow, and the polyline after Uygula,
/// `.run/gui-evidence/yuksek-nokta-3380/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib grade_window_tests::peak_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn peak_screens() {
    use crate::grade_window::Target;
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yuksek-nokta-3380");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.selection.set([Slot(1)]);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let pts = match app.document.as_ref().and_then(|d| d.model.get(Slot(1))) {
            Some(kentos_commands::Entity::Polyline(p)) => p.pts.clone(),
            _ => panic!("the polyline"),
        };
        // The polyline in the left third, where the window does not cover it.
        let (x0, x1) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
        let (y0, y1) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.y), b.max(p.y)));
        let w = x1 - x0 + 10.0;
        app.viewport.camera.fit(
            &kentos_render_wgpu::Bounds { min_x: x0 - 5.0, min_y: y0 - 15.0, max_x: x0 - 5.0 + 3.2 * w, max_y: y1 + 15.0 },
            24.0,
        );
        let _ = app.update(Message::Run("tool.grade"));
        for e in [G::To(4.0), G::Target(Target::Peak), G::End(851.25), G::Ahead(4.0), G::Back(1.0)] {
            let _ = app.update(Message::Grade(e));
        }
        app.selection.set(Vec::new());
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("onizleme-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        if theme == "light" {
            // After Uygula: the vertex on the polyline, its vertices shown by selecting it.
            let _ = app.update(Message::Grade(G::Apply));
            app.selection.set([Slot(1)]);
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("uygulandi-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        }
    }
}

/// The two-part area (slot 6) selected, Eğimle kot ver on Bitiş kotu, Halka the second part's hole, its vertices 2 to 4
/// from 101 m to 101.6 m (docs/adr/4980).
fn on_a_hole() -> App {
    let mut app = opened(Some(Module::Map));
    app.selection.set([Slot(6)]);
    let _ = app.update(Message::Run("tool.grade"));
    let _ = app.update(Message::Grade(G::Ring("1:1".into())));
    assert_eq!((app.grade.ring, app.grade.from, app.grade.to), ((1, 1), 1.0, 4.0), "the ring, its first and last vertex");
    for e in [G::From(2.0), G::Start(101.0), G::Target(crate::grade_window::Target::End), G::End(101.6)] {
        let _ = app.update(Message::Grade(e));
    }
    app
}

#[test]
fn halka_puts_the_range_on_a_further_part_s_hole() {
    let mut app = on_a_hole();
    let plan = app.grade.plan.clone().expect("a plan").expect("planned");
    assert!((plan.grade - 0.02).abs() < 1e-12 && (plan.length - 30.0).abs() < 1e-9, "{plan:?}");
    // The hole's range is marked: (430, 10) to (410, 20) from X0, Y0.
    let (path, ends) = app.grade_marks();
    assert!(!path.is_empty() && ends.len() == 2, "{path:?} {ends:?}");
    let _ = app.update(Message::Grade(G::Apply));
    assert_eq!(app.dialog, None);
    let area = |app: &App| match app.document.as_ref().and_then(|d| d.model.get(Slot(6))) {
        Some(kentos_commands::Entity::Polygon(p)) => p.clone(),
        _ => panic!("the area"),
    };
    let p = area(&app);
    let hole = &p.parts.as_ref().expect("parts")[0].holes.as_ref().expect("holes")[0];
    let zs = hole.zs.clone().expect("elevations");
    assert_eq!(zs[0], None);
    for (got, want) in zs[1..].iter().zip([101.0, 101.2, 101.6]) {
        assert!((got.expect("an elevation") - want).abs() < 1e-9, "{zs:?}");
    }
    assert!(p.zs.is_none() && p.parts.as_ref().expect("parts")[0].zs.is_none(), "the other rings are left");
    let _ = app.update(Message::Run("edit.undo"));
    let p = area(&app);
    assert!(p.parts.as_ref().expect("parts")[0].holes.as_ref().expect("holes")[0].zs.is_none(), "one undo step");
}

/// `.run/gui-evidence/cok-parcali-aralik-4980/`: Halka on the second part's hole, light wide and dark narrow.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib grade_window_tests::screens_on_a_ring -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_on_a_ring() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cok-parcali-aralik-4980");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = on_a_hole();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        // The second part (400–440, 0–30 from X0, Y0) left of the window, which stands in the middle.
        let camera = &mut app.viewport.camera;
        camera.scale = if size.width > 1200.0 { 4.0 } else { 2.2 };
        let (x0, y0) = (500_000.0, 4_400_000.0);
        camera.center = kentos_interaction::Vec2::new(x0 + 420.0 + (camera.width / 2.0 - camera.width * 0.15) / camera.scale, y0 + 15.0);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-egimle-delikte-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
