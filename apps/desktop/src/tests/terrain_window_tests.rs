//! Arazi yüzeyleri (terrain_window.rs, plan/road.md Y2.4): the window writes through `road.terrains`, and its pictures.

use crate::road;
use kentos_domain::Slot;

use crate::app::{App, Dialog, Message};
use crate::terrain_window::Event as W;

/// The terrain commands' shared drawing (`fixtures/commands/v1/road.terrains.query.json`'s setup) with no surface.
fn app(work: &str) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.query.json")).expect("cases");
    let mut setup = cases["setup"].clone();
    setup["settings"].as_object_mut().expect("settings").remove("road");
    setup["settings"]["cadWork"] = serde_json::json!(work);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Road(road::Event::Terrains(e)));
}

/// The boundary of the shared drawing: its tenth object.
const BOUNDARY: Slot = Slot(10);
const VOID: Slot = Slot(11);

fn made(app: &mut App) {
    let _ = app.update(Message::Run("terrain.new"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::Terrains))
    );
    send(app, W::PointLayer("zemin".into()));
    send(app, W::BreaklineLayer("kirik".into()));
    app.selection.set([BOUNDARY]);
    send(app, W::Boundary);
    app.selection.set([VOID]);
    send(app, W::Voids);
    send(app, W::MaxEdge("60".into()));
    send(app, W::Save);
}

#[test]
fn the_window_makes_a_surface_and_accepts_it_again() {
    let mut app = app("road");
    made(&mut app);
    assert!(
        app.road.terrain_window.is_none(),
        "Kaydet closes the window"
    );
    let doc = &app.document.as_ref().expect("a drawing").model;
    let road = doc.settings().road.clone().expect("road settings");
    assert_eq!(road.surfaces.len(), 1);
    let s = &road.surfaces[0];
    assert_eq!((s.id.as_str(), s.name.as_str()), ("t-1", "Mevcut zemin"));
    assert_eq!(s.points.layers, ["zemin"]);
    assert_eq!(s.voids.len(), 1);
    assert_eq!(s.max_edge, Some(60.0));
    assert!(s.built.is_some(), "a new surface is accepted");
    // A changed edge method makes it stale; Kabul et accepts it as its sources give it now.
    let _ = app.update(Message::Run("terrain.surfaces"));
    send(&mut app, W::MaxEdge("25".into()));
    send(&mut app, W::Save);
    let made = kentos_native_application::road_terrains::made_surfaces(&app.document.as_ref().expect("doc").model);
    assert_eq!(made[0].1.state, kentos_commands::TerrainState::Stale);
    let _ = app.update(Message::Run("terrain.surfaces"));
    send(&mut app, W::Accept);
    let made = kentos_native_application::road_terrains::made_surfaces(&app.document.as_ref().expect("doc").model);
    assert_eq!(made[0].1.state, kentos_commands::TerrainState::Current);
    // Eğrileri çiz: the saved surface's contour lines on a layer, one undo step.
    let before = app.document.as_ref().expect("doc").model.entities().count();
    send(&mut app, W::Interval("0,5".into()));
    send(&mut app, W::ContourLayer("esyukselti".into()));
    send(&mut app, W::Contours);
    let doc = &app.document.as_ref().expect("doc").model;
    assert!(doc.entities().count() > before, "the lines are drawn");
    assert!(doc.can_undo());
    // Removed and saved: the surface goes; the contour lines' record stays, its source gone (docs/adr/0911 §2), so the
    // Yol projesi panel says “Kaynağı yok”.
    send(&mut app, W::Remove);
    send(&mut app, W::Save);
    let road = app.document.as_ref().expect("doc").model.settings().road.clone().expect("the drawing's record");
    assert!(road.surfaces.is_empty());
    let tree = kentos_native_application::road_project_query::preview(&app.document.as_ref().expect("doc").model, false)
        .expect("a road project");
    assert!(tree.nodes.iter().any(|n| n.kind == kentos_commands::RoadNodeKind::Drawing && n.state == kentos_commands::RoadNodeState::Gone));
}

#[test]
fn landxml_comes_in_as_a_surface() {
    let mut app = app("road");
    let _ = app.update(Message::Run("terrain.surfaces"));
    send(&mut app, W::ContourLayer("esyukselti".into()));
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/landxml/v1/kirikli.xml");
    send(&mut app, W::LandXmlChosen(Some(path)));
    let doc = &app.document.as_ref().expect("doc").model;
    let road = doc.settings().road.clone().expect("road settings");
    assert_eq!(road.surfaces[0].name, "Zemin");
    assert!(road.surfaces[0].boundary.is_some() && road.surfaces[0].voids.len() == 1);
    assert!(doc.can_undo(), "the objects are one undo step");
}

#[test]
fn an_architecture_project_has_no_surfaces() {
    let mut app = app("architecture");
    let _ = app.update(Message::Run("terrain.surfaces"));
    assert!(app.road.terrain_window.is_none());
    assert_ne!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::Terrains))
    );
}

/// docs/adr/1520: a Harita project makes, accepts and draws a surface as a Yol project does; the Arazi tab offers the
/// surfaces and their analysis, and since docs/adr/1580 Tesviye and Drenaj; the section query stays Yol's.
#[test]
fn a_map_project_makes_a_surface_and_its_contours() {
    let mut app = app("map");
    app.program = Some(kentos_launch::Module::Map);
    assert!(app.offers("terrain.surfaces") && app.offers("terrain.analysis"));
    assert!(app.offers("platform.editor") && app.offers("drainage.editor"), "a map project grades and drains");
    assert!(!app.offers("mesh.query"), "Yol's own section query");
    made(&mut app);
    let made = kentos_native_application::road_terrains::made_surfaces(&app.document.as_ref().expect("doc").model);
    assert_eq!((made.len(), made[0].1.state), (1, kentos_commands::TerrainState::Current));
    let _ = app.update(Message::Run("terrain.surfaces"));
    let before = app.document.as_ref().expect("doc").model.entities().count();
    send(&mut app, W::Interval("0,5".into()));
    send(&mut app, W::ContourLayer("esyukselti".into()));
    send(&mut app, W::Contours);
    assert!(app.document.as_ref().expect("doc").model.entities().count() > before, "the contour lines are drawn");
}

/// A Harita project's surface (docs/adr/1520): the Arazi tab, the surface over the drawing and its window;
/// `.run/gui-evidence/harita-yuzey-1520/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib terrain_window_tests::screens_map -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_map() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/harita-yuzey-1520");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = app("map");
        app.program = Some(kentos_launch::Module::Map);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        made(&mut app);
        let _ = app.update(Message::Run("terrain.surfaces"));
        send(&mut app, W::Interval("0,5".into()));
        send(&mut app, W::ContourLayer("esyukselti".into()));
        send(&mut app, W::Contours);
        send(&mut app, W::Cancel);
        app.selection.clear();
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-arazi-sekmesi-yuzey-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::Run("terrain.surfaces"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-yuzey-penceresi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// The window and the surface over the drawing, wide and narrow, light and dark; `.run/gui-evidence/y2-4-*`:
///
/// ```text
/// cargo test -p kentos-desktop --lib terrain_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = app("road");
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::RibbonTab("terrain"));
        snapshot.settle(&mut app, App::view, &mut update);
        made(&mut app);
        app.selection.clear();
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("view.zoomExtents"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y2-4-yuzey-cizimde-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::Run("terrain.surfaces"));
        send(&mut app, W::Interval("0,25".into()));
        send(&mut app, W::ContourLayer("esyukselti".into()));
        send(&mut app, W::Contours);
        let file = out.join(format!("y2-5-esyukselti-penceresi-{name}.png"));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        send(&mut app, W::Cancel);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y2-5-esyukselti-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = app.update(Message::Run("terrain.surfaces"));
        send(&mut app, W::MaxEdge("25".into()));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y2-4-yuzey-penceresi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        // LandXML al: a file's surface beside the drawing's, on its own layer.
        send(&mut app, W::Cancel);
        let _ = app.update(Message::Run("terrain.surfaces"));
        send(&mut app, W::ContourLayer("0".into()));
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/landxml/v1/kirikli.xml");
        send(&mut app, W::LandXmlChosen(Some(path)));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("y2-5-landxml-penceresi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// BÖHHBÜY m. 78(1)(c): the window starts at the regulation's interval for the drawing's scale.
#[test]
fn the_contour_interval_starts_by_the_scale() {
    use crate::terrain_window::contour_interval;
    assert_eq!(
        [5000.0, 2000.0, 1000.0, 500.0, 2500.0].map(contour_interval),
        ["5", "2", "1", "1", "1"]
    );
}

/// Düzenlemeler (docs/adr/1560): each tool steps the window aside for a pick on the drawing, its edit goes to the
/// chosen definition with what it came to, Sonuncuyu kaldır takes the last back, Kaydet writes them and the surface
/// made again from its sources keeps them.
#[test]
fn the_surface_is_edited_by_picks_and_kaydet_writes_the_edits() {
    use crate::terrain_edits::EditTool;
    use kentos_commands::SurfaceEdit;
    let at = |x: f64, y: f64| Some(kentos_interaction::Vec2 { x, y });
    let mut app = app("map");
    made(&mut app);
    let _ = app.update(Message::Run("terrain.surfaces"));
    send(&mut app, W::Choose(0));
    send(&mut app, W::EditZ("104,5".into()));
    send(&mut app, W::EditTool(EditTool::AddPoint));
    assert!(app.dialog.is_none(), "the window steps aside for the pick");
    assert!(app.terrain_picked(at(25.0, 25.0)));
    assert_eq!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::Terrains)),
        "and comes back"
    );
    send(&mut app, W::EditZ("103,5".into()));
    send(&mut app, W::EditTool(EditTool::SetHeight));
    assert!(app.terrain_picked(at(21.2, 23.9)));
    send(&mut app, W::EditTool(EditTool::RemoveTriangle));
    assert!(app.terrain_picked(at(6.0, 10.0)));
    // A pick outside the surface makes no edit, and says why.
    send(&mut app, W::EditTool(EditTool::RemoveTriangle));
    assert!(app.terrain_picked(at(500.0, 500.0)));
    // Esc: none.
    send(&mut app, W::EditTool(EditTool::FlipEdge));
    assert!(app.terrain_picked(None));
    assert!(
        !app.terrain_picked(at(1.0, 1.0)),
        "no pick is awaited any more"
    );
    let w = app.road.terrain_window.as_ref().expect("the window");
    let edits = w.chosen_def().expect("a surface").edits.clone();
    assert_eq!(edits.len(), 3, "{edits:?}");
    assert_eq!(
        edits[0],
        SurfaceEdit::AddPoint {
            x: 25.0,
            y: 25.0,
            z: 104.5
        }
    );
    assert_eq!(
        edits[1],
        SurfaceEdit::SetHeight {
            x: 21.0,
            y: 24.0,
            z: 103.5
        }
    );
    assert!(matches!(edits[2], SurfaceEdit::RemoveTriangle { .. }));
    let report = w.edit_report.as_ref().expect("made with its edits");
    assert_eq!(report.figures.as_ref().map(|f| f.removed), Some(1));
    assert!(report.left.iter().all(|l| l.role != "edits"), "{:?}", report.left);
    // Sonuncuyu kaldır, then the triangle again.
    send(&mut app, W::EditUndo);
    assert_eq!(
        app.road
            .terrain_window
            .as_ref()
            .and_then(|w| w.chosen_def())
            .map(|d| d.edits.len()),
        Some(2)
    );
    send(&mut app, W::EditTool(EditTool::RemoveTriangle));
    assert!(app.terrain_picked(at(6.0, 10.0)));
    send(&mut app, W::Save);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let saved = &doc.settings().road.as_ref().expect("road settings").surfaces[0];
    assert_eq!(saved.edits.len(), 3);
    // The drawing's surface is made with them.
    let (_, report, surface) = kentos_native_application::road_terrains::made_surfaces(doc).remove(0);
    assert_eq!(report.figures.as_ref().map(|f| f.removed), Some(1));
    let s = surface.expect("a surface");
    assert_eq!(s.height(25.0, 25.0).z, Some(104.5));
    assert_eq!(s.height(21.0, 24.0).z, Some(103.5));
    assert_eq!(s.height(6.0, 10.0).z, None, "the triangle is out");
}

/// The window's Düzenlemeler with its edits, and the drawing's surface with them, wide light and narrow dark;
/// `.run/gui-evidence/yuzey-duzenle-1560/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib terrain_window_tests::screens_edits -- --ignored --nocapture
/// ```
#[test]
#[ignore = "writes pictures"]
fn screens_edits() {
    use crate::terrain_edits::EditTool;
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yuzey-duzenle-1560");
    std::fs::create_dir_all(&out).expect("a folder");
    // The pick through the tool itself: the coordinates typed on the command line (Y, X), as a user may.
    let pick = |app: &mut App, x: f64, y: f64| {
        let _ = app.update(Message::CommandInput(format!("{x},{y}")));
        let _ = app.update(Message::CommandSubmitted);
    };
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = app("map");
        app.program = Some(kentos_launch::Module::Map);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        made(&mut app);
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::RibbonTab("terrain"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-yuzey-once-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::Run("terrain.surfaces"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::Pane(true));
        send(&mut app, W::EditZ("104,5".into()));
        send(&mut app, W::EditTool(EditTool::AddPoint));
        pick(&mut app, 25.0, 25.0);
        assert_eq!(
            app.dialog,
            Some(Dialog::Road(road::RoadDialogKind::Terrains)),
            "the pick came back through the tool"
        );
        send(&mut app, W::EditZ("103,5".into()));
        send(&mut app, W::EditTool(EditTool::SetHeight));
        pick(&mut app, 21.2, 23.9);
        send(&mut app, W::EditTool(EditTool::RemoveTriangle));
        pick(&mut app, 6.0, 10.0);
        send(&mut app, W::EditTool(EditTool::RemoveTriangle));
        pick(&mut app, 30.0, 26.0);
        // An edge put in that a later source would cross is said, not dropped: here one crossing the breakline.
        if let Some(w) = app.road.terrain_window.as_mut() {
            w.push_edit(kentos_commands::SurfaceEdit::AddEdge {
                ax: 21.0,
                ay: 24.0,
                bx: 10.0,
                by: 5.0,
            });
        }
        app.refresh_edit_report();
        send(&mut app, W::EditZ(String::new()));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-duzenlemeler-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        send(&mut app, W::Save);
        app.selection.clear();
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("3-yuzey-sonra-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// The shared paste case's drawing (`road.terrains.query.json`, docs/adr/3920): the ground's surfaces and the pad
/// (`Dolgu`, t-8) on `dolgu`, without the surface that pastes it.
fn pad_app(work: &str) -> App {
    pad_app_keeping(work, &[])
}

/// [`pad_app`] with only the surfaces named (none: all but the one that pastes).
fn pad_app_keeping(work: &str, keep: &[&str]) -> App {
    let (mut app, _) = App::boot(None);
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.query.json")).expect("cases");
    let case = cases["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Yapıştırılmış yüzeyde kot")))
        .expect("the paste case");
    let mut setup = case["setup"].clone();
    setup["settings"]["cadWork"] = serde_json::json!(work);
    setup["settings"]["road"]["surfaces"]
        .as_array_mut()
        .expect("surfaces")
        .retain(|s| s["id"] != "t-9" && (keep.is_empty() || keep.iter().any(|k| s["id"] == *k)));
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("the drawing reads");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The window's row of the surface named so.
fn row_of(app: &App, name: &str) -> usize {
    app.road
        .terrain_window
        .as_ref()
        .expect("the window")
        .kept()
        .iter()
        .position(|k| k.name == name)
        .expect("a surface")
}

#[test]
fn a_saved_surface_is_pasted_into_another_and_kaydet_writes_the_paste() {
    use crate::terrain_edits::{PASTE, PASTE_FROM};
    use crate::traces::Control;
    use kentos_commands::SurfaceEdit;
    let mut app = pad_app("map");
    let _ = app.update(Message::Run("terrain.surfaces"));
    let open = row_of(&app, "Açık zemin");
    send(&mut app, W::Choose(open));
    send(&mut app, W::Pane(true));
    // Nothing chosen yet: Yüzey yapıştır says so.
    send(&mut app, W::Paste);
    assert!(
        app.road
            .terrain_window
            .as_ref()
            .and_then(|w| w.chosen_def())
            .is_some_and(|d| d.edits.is_empty())
    );
    // Dolgu by its name, as a trace does.
    let pick = app
        .terrain_control(Control::Pick(PASTE_FROM, "Dolgu"))
        .expect("the box")
        .expect("a message");
    let _ = app.update(pick);
    let press = app
        .terrain_control(Control::Press(PASTE))
        .expect("the button")
        .expect("a message");
    let _ = app.update(press);
    let w = app.road.terrain_window.as_ref().expect("the window");
    assert_eq!(
        w.chosen_def().expect("a surface").edits,
        vec![SurfaceEdit::Paste {
            terrain: "t-8".into()
        }]
    );
    let report = w.edit_report.as_ref().expect("made with its edits");
    assert!(
        report.left.iter().all(|l| l.role != "edits"),
        "{:?}",
        report.left
    );
    send(&mut app, W::Save);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let made = kentos_native_application::road_terrains::made_surfaces(doc);
    let surface = |id: &str| made.iter().find(|(d, _, _)| d.id == id).and_then(|(_, _, s)| s.clone()).expect("made");
    let (pad, son) = (surface("t-8"), surface("t-3"));
    // Every triangle of the pad is the ground's now, by its corners' places.
    let corners = |s: &kentos_road::surface::Surface, t: [u32; 3]| {
        let mut c = t.map(|i| (s.vertices[i as usize][0].to_bits(), s.vertices[i as usize][1].to_bits()));
        c.sort_unstable();
        c
    };
    let all: Vec<_> = son.triangles.iter().map(|t| corners(&son, *t)).collect();
    assert!(pad.triangles.iter().all(|t| all.contains(&corners(&pad, *t))));
    let top = son.height(20.0, 15.0).z.expect("on the pad");
    assert!((top - 103.0).abs() < 1e-9, "{top}");
}

#[test]
fn a_paste_back_is_a_cycle_and_a_surface_does_not_paste_itself() {
    let mut app = pad_app("map");
    let _ = app.update(Message::Run("terrain.surfaces"));
    let (open, pad) = (row_of(&app, "Açık zemin"), row_of(&app, "Dolgu"));
    send(&mut app, W::Choose(open));
    send(&mut app, W::PasteFrom("t-8".into()));
    send(&mut app, W::Paste);
    send(&mut app, W::Save);
    // Dolgu pasting Açık zemin back: said, not done; Dolgu stays its own.
    let _ = app.update(Message::Run("terrain.surfaces"));
    send(&mut app, W::Choose(pad));
    send(&mut app, W::PasteFrom("t-3".into()));
    send(&mut app, W::Paste);
    let w = app.road.terrain_window.as_ref().expect("the window");
    let report = w.edit_report.as_ref().expect("made with its edits");
    let left: Vec<(&str, &str)> = report
        .left
        .iter()
        .map(|l| (l.id.as_str(), l.why.as_str()))
        .collect();
    assert_eq!(left, [("edits[0]", "cycle")]);
    assert_eq!(report.figures.as_ref().map(|f| f.triangles), Some(10), "the pad's own triangles");
    // Itself: no edit, a warning.
    send(&mut app, W::EditClear);
    send(&mut app, W::PasteFrom("t-8".into()));
    send(&mut app, W::Paste);
    assert!(
        app.road
            .terrain_window
            .as_ref()
            .and_then(|w| w.chosen_def())
            .is_some_and(|d| d.edits.is_empty())
    );
}

/// Yüzey yapıştır's row, the table with the paste and a refused paste back, and the ground drawn with the pad in it,
/// wide light and narrow dark; `.run/gui-evidence/yuzey-yapistir-3920/`:
///
/// ```text
/// cargo test -p kentos-desktop --lib terrain_window_tests::screens_paste -- --ignored --nocapture
/// ```
#[test]
#[ignore = "writes pictures"]
fn screens_paste() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yuzey-yapistir-3920");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = pad_app("map");
        app.program = Some(kentos_launch::Module::Map);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("terrain.surfaces"));
        let open = row_of(&app, "Açık zemin");
        send(&mut app, W::Choose(open));
        send(&mut app, W::Pane(true));
        send(&mut app, W::PasteFrom("t-8".into()));
        send(&mut app, W::Paste);
        // A paste of a surface that cannot be made (Az nokta: two points) is said in the table.
        send(&mut app, W::PasteFrom("t-5".into()));
        send(&mut app, W::Paste);
        send(&mut app, W::PasteFrom("t-8".into()));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-yapistir-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        // The drawing with Açık zemin and Dolgu alone: the pad's edges (red: they are the pasted surface's
        // breaklines now) inside the ground's triangles.
        let mut app = pad_app_keeping("map", &["t-3", "t-8"]);
        app.program = Some(kentos_launch::Module::Map);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        // A renderer of its own: the drawing area's size is told to a new app on its first layout.
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::RibbonTab("terrain"));
        let _ = app.update(Message::Run("terrain.surfaces"));
        send(&mut app, W::Choose(0));
        send(&mut app, W::PasteFrom("t-8".into()));
        send(&mut app, W::Paste);
        send(&mut app, W::Save);
        app.selection.clear();
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-yuzey-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
