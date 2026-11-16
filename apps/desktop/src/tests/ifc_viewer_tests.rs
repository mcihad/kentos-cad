//! IFC modeli (ifc_viewer.rs, plan/ifc-uyum.md IFC-12 "İlk kabul", docs/adr/8073): the section building's file
//! (fixtures/arch/v1/section-building.kcad, written without KentOS code, exported by `cad.arch.ifc`) opens in the
//! viewer with as many bodies as the file holds and its storeys in the tree; a click on a wall reads its details as the
//! drawing made them and chooses it in the plan, and the plan's choice is chosen in the viewer; a broken file is refused
//! with its reason; a stopped read and one a newer read replaced write nothing; the window closes with its model; the
//! export offers Görüntüle. Denetim's chosen finding paints its objects in the viewer (docs/adr/8074 §2).

use std::path::PathBuf;

use kentos_commands::Entity;
use kentos_model_view::Category;
use kentos_model_view_ui::Message as V;
use kentos_model_view_ui::scene::Gesture;
use kentos_scene3d::Viewport;

use crate::app::{App, Message};
use crate::ifc_viewer::Event;
use crate::scene3d_building_tests::{arguments, building, open};

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::IfcViewer(e));
}

/// The open drawing written as IFC into a folder of its own.
fn exported(app: &mut App, name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-ifc-modeli-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("bina.ifc");
    let doc = &mut app.document.as_mut().expect("the building").model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        "cad.arch.ifc",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"path": path.to_string_lossy(), "overwrite": true}),
    )
    .expect("writes the file");
    assert_eq!(answer["status"], "completed", "{answer}");
    path
}

/// How many products the file holds with a shape (a representation given), read from its text: its building
/// elements, spaces and spatial zones. An opening is a cut, not a body.
fn shaped(path: &std::path::Path) -> usize {
    const CLASSES: [&str; 18] = [
        "IFCWALL", "IFCSLAB", "IFCCOLUMN", "IFCBEAM", "IFCROOF", "IFCRAILING", "IFCSTAIR", "IFCCOVERING", "IFCDOOR",
        "IFCWINDOW", "IFCTRANSPORTELEMENT", "IFCFURNITURE", "IFCSANITARYTERMINAL", "IFCMEMBER", "IFCCHIMNEY", "IFCRAMP",
        "IFCSPACE", "IFCSPATIALZONE",
    ];
    let text = std::fs::read_to_string(path).expect("reads the file");
    text.lines()
        .filter_map(|l| l.split_once('=').map(|(_, r)| r.trim()))
        .filter(|r| CLASSES.iter().any(|c| r.starts_with(&format!("{c}("))))
        .filter(|r| arguments(r).get(6).is_some_and(|a| a.trim() != "$"))
        .count()
}

fn opened(app: &mut App, path: PathBuf) {
    send(app, Event::Open(path));
    app.ifc_viewer_read_now();
}

/// A click on the `k`th wall mesh's first triangle seen from the viewer's camera; the object chosen.
fn click_a_wall(app: &mut App) -> usize {
    let vp = Viewport { width: 1000.0, height: 700.0 };
    for k in 0..40 {
        let w = app.ifc_viewer.as_ref().expect("the viewer");
        let cam = w.viewer.camera();
        let Some(m) = w.viewer.scene().meshes().iter().filter(|m| m.part == 0).nth(k) else { break };
        let Some(t) = m.triangles.first() else { continue };
        let [a, b, c] = t.map(|i| m.vertices[i as usize]);
        let middle = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
        let Some((x, y, _)) = cam.project(middle, &vp) else { continue };
        send(app, Event::Viewer(V::Gesture(Gesture::Pick(x as f32, y as f32, 1000.0, 700.0))));
        let v = app.ifc_viewer.as_ref().and_then(|w| w.viewer.view()).expect("the model");
        if let Some(o) = v.selected().filter(|&o| v.input().objects[o].category == Category::Wall) {
            return o;
        }
    }
    panic!("no wall met a click");
}

#[test]
fn the_section_building_s_file_opens_whole_and_chooses_with_the_plan() {
    let mut app = open(building());
    let path = exported(&mut app, "bina");
    let expected = shaped(&path);
    let _ = app.update(Message::Run("ifc.view"));
    opened(&mut app, path.clone());
    // Beside the plan, in a window of its own: nothing over the drawing.
    assert!(app.dialog.is_none(), "{:?}", app.dialog);
    let w = app.ifc_viewer.as_ref().expect("the viewer");
    let v = w.viewer.view().expect("the model read");
    assert_eq!(v.counts(), (expected, expected), "every body the file holds, as many as it holds");
    assert!(v.input().not_shown.iter().all(|(o, _)| v.input().objects[*o].class == "IfcGrid"), "{:?}", v.input().not_shown);
    let storeys: Vec<&str> =
        w.viewer.rows().iter().filter(|r| r.object.is_some_and(|o| v.input().objects[o].category == Category::Storey)).map(|r| r.label.as_str()).collect();
    assert_eq!(storeys.len(), 2, "the storeys open in the tree: {storeys:?}");

    // A click on a wall: its details are the drawing's wall, and the plan chooses it.
    let wall = click_a_wall(&mut app);
    let w = app.ifc_viewer.as_ref().expect("the viewer");
    let key = w.viewer.view().expect("the model").input().objects[wall].key.clone();
    let doc = &app.document.as_ref().expect("the drawing").model;
    let uid = kentos_ifc::step::unguid(&key).map(kentos_domain::Uuid::from_u128).expect("KentOS's GlobalId");
    let slot = doc.slot_of(uid).expect("the drawing's wall");
    assert_eq!(app.selection.ids(), [slot], "chosen in the plan too");
    let Some(Entity::Wall(drawn)) = doc.get(slot) else { panic!("a wall in the drawing") };
    let details = w.viewer.details().expect("its details");
    let line = |name: &str| details.groups.iter().flat_map(|g| &g.lines).find(|l| l.name == name).map(|l| l.value.clone());
    assert_eq!(line("GlobalId").as_deref(), Some(key.as_str()));
    let width: f64 = line("Width").expect("its width").parse().expect("a number");
    assert!((width - drawn.wall.width).abs() < 1e-9, "the file's width is the drawing's: {width} {}", drawn.wall.width);
    if drawn.wall.pts.len() == 2 && drawn.wall.bulges.is_none() {
        let (p, q) = (drawn.wall.pts[0], drawn.wall.pts[1]);
        let length: f64 = line("Length").expect("its length").parse().expect("a number");
        assert!((length - (q.x - p.x).hypot(q.y - p.y)).abs() < 1e-6, "{length}");
    }

    // The plan chooses another wall: the viewer chooses it, its details read.
    let other_uid = doc
        .entities_with_uids()
        .find(|(u, e)| *u != uid && matches!(e, Entity::Wall(_)))
        .map(|(u, _)| u)
        .expect("another wall");
    let other = doc.slot_of(other_uid).expect("its slot");
    let other_key = kentos_ifc::step::guid(other_uid.as_u128());
    app.selection.set(vec![other]);
    app.follow_ifc_viewer();
    let w = app.ifc_viewer.as_ref().expect("the viewer");
    let v = w.viewer.view().expect("the model");
    assert_eq!(v.selected().map(|o| v.input().objects[o].key.as_str()), Some(other_key.as_str()));
    assert!(w.viewer.details().is_some(), "its details read");

    // Kapat: the viewer and its model go.
    send(&mut app, Event::Viewer(V::Close));
    assert!(app.ifc_viewer.is_none());
    let _ = std::fs::remove_dir_all(path.parent().expect("its folder"));
}

/// The audit's model quality control form reads the file KentOS would write (docs/adr/8074): its finding of the walls'
/// missing fire rating, chosen in Denetim, paints those walls in the viewer of the drawing's file, nothing else.
#[test]
fn a_chosen_finding_paints_its_objects_in_the_viewer() {
    let mut app = open(building());
    let path = exported(&mut app, "bulgu");
    let _ = app.update(Message::Run("ifc.view"));
    opened(&mut app, path.clone());
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    let a = app.arch.arch_audit.audited.as_ref().expect("checked");
    let at = a
        .findings
        .iter()
        .position(|f| f.code == "digital_required_field" && f.message.contains("Pset_WallCommon.FireRating"))
        .unwrap_or_else(|| panic!("{:#?}", a.findings.iter().map(|f| (&f.code, &f.message)).collect::<Vec<_>>()));
    let walls: std::collections::BTreeSet<String> = a.findings[at].uids.iter().cloned().collect();
    assert!(!walls.is_empty(), "the finding names its walls");
    app.arch.arch_audit.chosen.clear();
    app.arch.arch_audit.selected = Some(at);
    app.follow_ifc_viewer();
    let v = app.ifc_viewer.as_ref().and_then(|w| w.viewer.view()).expect("the model");
    let painted: std::collections::BTreeSet<String> = v
        .found()
        .iter()
        .map(|&o| v.input().objects[o].key.clone())
        .filter_map(|k| kentos_ifc::step::unguid(&k).map(|n| kentos_domain::Uuid::from_u128(n).to_string()))
        .collect();
    assert_eq!(painted, walls, "the finding's walls, nothing else");
    let _ = std::fs::remove_dir_all(path.parent().expect("its folder"));
}

#[test]
fn a_broken_file_is_refused_and_a_stopped_or_replaced_read_writes_nothing() {
    let (mut app, _) = App::boot(None);
    let dir = std::env::temp_dir().join(format!("kentos-ifc-bozuk-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let broken = dir.join("bozuk.ifc");
    std::fs::write(&broken, "Bu bir IFC dosyası değil.\n").expect("writes");
    opened(&mut app, broken.clone());
    match &app.ifc_viewer.as_ref().expect("the viewer").viewer.stage {
        kentos_model_view_ui::Stage::Failed(why) => assert!(why.contains("ISO-10303-21") || why.contains("STEP"), "{why}"),
        other => panic!("refused, not {other:?}"),
    }

    // İptal stops the read: the window goes, and the read's late result finds nothing to write to.
    let good = dir.join("iyi.ifc");
    std::fs::write(&good, include_bytes!("../../../../fixtures/arch/v1/ifc/kesit-binasi.ifc")).expect("writes");
    send(&mut app, Event::Open(good.clone()));
    let stopped = app.ifc_viewer_id().expect("reading");
    send(&mut app, Event::Viewer(V::Cancel));
    assert!(app.ifc_viewer.is_none(), "İptal closes the window");
    app.ifc_viewer_late(stopped, &good);
    assert!(app.ifc_viewer.is_none(), "a stopped read writes nothing");

    // A newer read replaces the one under way: the older one's result is not shown.
    send(&mut app, Event::Open(broken.clone()));
    let older = app.ifc_viewer_id().expect("reading");
    send(&mut app, Event::Open(good.clone()));
    app.ifc_viewer_late(older, &broken);
    let w = app.ifc_viewer.as_ref().expect("the viewer");
    assert!(matches!(w.viewer.stage, kentos_model_view_ui::Stage::Reading { .. }), "{:?}", w.viewer.stage);
    app.ifc_viewer_read_now();
    assert_eq!(app.ifc_viewer.as_ref().expect("the viewer").viewer.stage, kentos_model_view_ui::Stage::Shown);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_export_offers_to_view_its_file() {
    let mut app = open(building());
    let dir = std::env::temp_dir().join(format!("kentos-ifc-gor-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("bina.ifc");
    let _ = app.ifc_building_chosen(Some(path.clone()), kentos_commands::IfcView::DesignTransfer);
    let offer = app
        .toasts
        .iter()
        .find_map(|(_, t, _)| t.action_of().filter(|(words, _)| *words == "Görüntüle").map(|(_, m)| m.clone()))
        .expect("Görüntüle offered");
    let _ = app.update(offer);
    app.ifc_viewer_read_now();
    let w = app.ifc_viewer.as_ref().expect("the viewer");
    assert_eq!(w.path, path);
    assert_eq!(w.viewer.stage, kentos_model_view_ui::Stage::Shown);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Pictures for the owner (`.run/gui-evidence/ifc-modeli-8073/`): Mimari's Proje panel with IFC modeli; the viewer in
/// its own window, a wall clicked; a level cut; the plan with the wall chosen; the viewer over the drawing, light and
/// dark. Drawn on the GPU:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ifc_viewer_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ifc-modeli-8073");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(building());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let path = exported(&mut app, &format!("resim-{name}"));
        let mut main = Snapshot::new(size).expect("a renderer");
        let mut apart = Snapshot::new(Size::new(1400.0, 860.0)).expect("a renderer");
        let mut shot = |snapshot: &mut Snapshot, app: &mut App, view: fn(&App) -> iced::Element<'_, Message>, file: &str| {
            snapshot.settle(app, view, &mut update);
            snapshot.settle(app, view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(view(app), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("arch"));
        shot(&mut main, &mut app, App::view, "1-serit");
        opened(&mut app, path.clone());
        click_a_wall(&mut app);
        shot(&mut apart, &mut app, App::ifc_viewer_shown, "2-ayri-pencere-duvar");
        if name == "acik" {
            send(&mut app, Event::Viewer(V::Cut(kentos_model_view::Cut::Level)));
            let (lo, hi) = app.ifc_viewer.as_ref().and_then(|w| w.viewer.span(kentos_model_view::Cut::Level)).expect("a span");
            send(&mut app, Event::Viewer(V::At(lo + (hi - lo) * 0.3)));
            shot(&mut apart, &mut app, App::ifc_viewer_shown, "3-yatay-kesit");
            send(&mut app, Event::Viewer(V::Uncut));
            shot(&mut main, &mut app, App::view, "4-plan-secili");
        }
        let _ = app.update(Message::Windows(crate::windows::Event::Apart(crate::windows::Surface::IfcViewer, false)));
        shot(&mut main, &mut app, App::view, "5-cizimin-ustunde");
        let _ = app.update(Message::Windows(crate::windows::Event::Apart(crate::windows::Surface::IfcViewer, true)));
        let _ = std::fs::remove_dir_all(path.parent().expect("its folder"));
    }
}

/// EK-9 from the written file (docs/adr/8074), light and dark; `.run/gui-evidence/ek9-dosyadan-8074/`: Denetim's
/// Mevzuat group with the form's findings, the project's information first; IFC modeli painting the walls the fire
/// rating finding names; the form's table in the drawing, its answers the file's; Ruhsat takımı with the project's
/// description among the digital delivery's particulars.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ifc_viewer_tests::ek9_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn ek9_screens() {
    use iced::{Point, Size};
    use kentos_commands::ArchAuditCheck;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ek9-dosyadan-8074");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    fn update(app: &mut App, message: Message) {
        let _ = app.update(message);
    }
    for (theme, size, name) in [("light", Size::new(1500.0, 1000.0), "acik"), ("dark", Size::new(1280.0, 900.0), "koyu")] {
        let mut app = open(building());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::RibbonTab("arch"));
        let mut main = Snapshot::new(size).expect("a renderer");
        let mut apart = Snapshot::new(Size::new(1400.0, 860.0)).expect("a renderer");
        let shot = |snapshot: &mut Snapshot, app: &mut App, view: fn(&App) -> iced::Element<'_, Message>, file: &str| {
            snapshot.settle(app, view, &mut update);
            snapshot.settle(app, view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(view(app), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        // Denetim: the form's findings from the file, in Mevzuat.
        let _ = app.update(Message::Run("arch.audit"));
        app.arch_audit_standards_settle();
        app.arch.arch_audit.group = Some(ArchAuditCheck::Regulation);
        let position = |app: &App, code: &str, words: &str| {
            app.arch.arch_audit.audited.as_ref().expect("checked").findings.iter().position(|f| f.code == code && f.message.contains(words))
        };
        app.arch.arch_audit.selected = position(&app, "digital_project_info_missing", "");
        shot(&mut main, &mut app, App::view, "1-denetim-proje-bilgileri");
        // IFC modeli: the drawing's file, the walls without a fire rating painted.
        let path = exported(&mut app, &format!("ek9-{name}"));
        let _ = app.update(Message::Run("ifc.view"));
        opened(&mut app, path.clone());
        app.arch.arch_audit.chosen.clear();
        app.arch.arch_audit.selected = position(&app, "digital_required_field", "Pset_WallCommon.FireRating");
        app.follow_ifc_viewer();
        shot(&mut apart, &mut app, App::ifc_viewer_shown, "2-ifc-modeli-bulgu");
        send(&mut app, Event::Viewer(V::Close));
        let _ = std::fs::remove_dir_all(path.parent().expect("its folder"));
        // The form's table in the drawing beside the building, its answers the written file's.
        let rows = crate::ifc_facts::form_rows(&app.document.as_ref().expect("the building").model).expect("the form");
        let doc = &mut app.document.as_mut().expect("the building").model;
        let all = kentos_native_application::sheets_fit::extent_of(doc, doc.entities()).expect("the building's extent");
        let look = kentos_interaction::table::Look { header: true, height: 0.25, ..Default::default() };
        let cells = kentos_geometry_core::ops::table::from_rows(rows, true);
        let geometry =
            kentos_interaction::table::new_table(doc, &cells, &look, kentos_interaction::Vec2::new(all[2] + 4.0, all[3]), None);
        let layer_id = doc.layers().active().to_owned();
        let made = kentos_native_application::create::execute(
            &mut kentos_native_application::ExecutionContext::new(doc),
            kentos_commands::EntitiesCreate {
                layer_id,
                objects: vec![kentos_commands::NewObject {
                    geometry,
                    color: None,
                    line_weight: None,
                    line_type: None,
                    line_pattern: None,
                    attrs: None,
                    label: None,
                    label_of: None,
                    label_scale: None,
                    symbol: None,
                    order: None,
                    transparency: None,
                }],
                operation: None,
                expected_revision: None,
            },
        );
        assert!(matches!(made, kentos_commands::CommandResult::Completed { .. }), "{made:?}");
        let table = doc.entities().filter(|e| matches!(e, Entity::Table(_))).last().expect("the table");
        let [x0, y0, x1, y1] = kentos_native_application::sheets_fit::extent_of(doc, std::iter::once(table)).expect("its box");
        // Read in two halves on a large canvas, the bottom panel closed: the questions with their answers, then the
        // answers with KentOS's notes.
        let large = Size::new(2400.0, 1500.0);
        let _ = app.update(Message::WindowResized(large));
        let _ = app.update(Message::CommandHistoryToggled);
        let mut wide = Snapshot::new(large).expect("a renderer");
        // Laid out once at its size first: the camera fits the box in the canvas it now has.
        wide.settle(&mut app, App::view, &mut update);
        for (from, to, file) in [(0.0, 0.5, "3-ek9-tablosu-sorular"), (0.45, 1.0, "4-ek9-tablosu-notlar")] {
            let bounds = kentos_render_wgpu::Bounds { min_x: x0 + from * (x1 - x0), min_y: y0, max_x: x0 + to * (x1 - x0), max_y: y1 };
            app.viewport.camera.fit(&bounds, 24.0);
            shot(&mut wide, &mut app, App::view, file);
        }
        let _ = app.update(Message::CommandHistoryToggled);
        let _ = app.update(Message::WindowResized(size));
        // Ruhsat takımı: the project's description, the IFC model's (EK-7 Tablo 7.1), among the digital delivery's.
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("arch.permitSet"));
        let middle = Point::new(size.width / 2.0, size.height / 2.0);
        main.input(&mut app, App::view, &mut update, Input::Scroll(middle, -12.0));
        shot(&mut main, &mut app, App::view, "5-ruhsat-proje-aciklamasi");
    }
}
