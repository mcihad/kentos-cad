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
/// elements and spaces, then its spatial zones (an area's volume, not a body: docs/adr/8123). An opening is a cut, not a
/// body.
fn shaped(path: &std::path::Path) -> (usize, usize) {
    const CLASSES: [&str; 17] = [
        "IFCWALL", "IFCSLAB", "IFCCOLUMN", "IFCBEAM", "IFCROOF", "IFCRAILING", "IFCSTAIR", "IFCCOVERING", "IFCDOOR",
        "IFCWINDOW", "IFCTRANSPORTELEMENT", "IFCFURNITURE", "IFCSANITARYTERMINAL", "IFCMEMBER", "IFCCHIMNEY", "IFCRAMP",
        "IFCSPACE",
    ];
    let text = std::fs::read_to_string(path).expect("reads the file");
    let with_body: Vec<&str> = text
        .lines()
        .filter_map(|l| l.split_once('=').map(|(_, r)| r.trim()))
        .filter(|r| arguments(r).get(6).is_some_and(|a| a.trim() != "$"))
        .collect();
    let of = |classes: &[&str]| with_body.iter().filter(|r| classes.iter().any(|c| r.starts_with(&format!("{c}(")))).count();
    (of(&CLASSES), of(&["IFCSPATIALZONE"]))
}

fn opened(app: &mut App, path: PathBuf) {
    send(app, Event::Open(path));
    app.ifc_viewer_read_now();
}

/// A click on the `k`th wall mesh's first triangle seen from the viewer's camera; the object chosen.
fn click_a_wall(app: &mut App) -> usize {
    click_one(app, Category::Wall)
}

/// A click on the first triangle of the meshes in turn seen from the viewer's camera until an object of `category` is
/// chosen; that object.
fn click_one(app: &mut App, category: Category) -> usize {
    let vp = Viewport { width: 1000.0, height: 700.0 };
    for k in 0..400 {
        let w = app.ifc_viewer.as_ref().expect("the viewer");
        let cam = w.viewer.camera();
        let Some(m) = w.viewer.scene().meshes().iter().filter(|m| m.part == 0).nth(k) else { break };
        let Some(t) = m.triangles.first() else { continue };
        let [a, b, c] = t.map(|i| m.vertices[i as usize]);
        let middle = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
        let Some((x, y, _)) = cam.project(middle, &vp) else { continue };
        send(app, Event::Viewer(V::Gesture(Gesture::Pick(x as f32, y as f32, 1000.0, 700.0))));
        let v = app.ifc_viewer.as_ref().and_then(|w| w.viewer.view()).expect("the model");
        if let Some(o) = v.selected().filter(|&o| v.input().objects[o].category == category) {
            return o;
        }
    }
    panic!("no {category:?} met a click");
}

/// Eğrisel aks (docs/adr/8169), light and dark: the section building with a curved grid system drawn beside its east
/// façade by Eğrisel aks ağı (radial axes 4 to 7 from −45° by 30°, arcs E at 6 m and F at 10 m); the plan, then its
/// file in IFC modeli with the curved grid chosen in the tree, its details: Ön tanımlı tür RADIAL and its axes, U the
/// radial ones counter-clockwise, V the arcs from the centre out. `.run/gui-evidence/egrisel-aks-8169/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ifc_viewer_tests::radial_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn radial_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/egrisel-aks-8169");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = open(building());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let size = Size::new(1440.0, 900.0);
        let _ = app.update(Message::WindowResized(size));
        let turn = 30f64.to_radians();
        let doc = &mut app.document.as_mut().expect("the building").model;
        let answer = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(doc),
            "cad.grids.create",
            kentos_commands::CallOp::Execute,
            serde_json::json!({"layerId": "aks", "radial": {
                "center": {"x": 12.0, "y": 4.5}, "angle": -45f64.to_radians(), "turns": [turn, turn, turn],
                "radius": 6.0, "rings": [4.0], "radialStart": "4", "arcStart": "E", "extension": 1.0}}),
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        let _ = app.update(Message::Run("view.zoomExtents"));
        let mut main = Snapshot::new(size).expect("a renderer");
        main.settle(&mut app, App::view, &mut update);
        main.settle(&mut app, App::view, &mut update);
        let plan = out.join(format!("1-plan-{name}.png"));
        main.render(App::view(&app), &app.theme()).save(&plan).expect("writes the picture");
        println!("{}", plan.display());
        let path = exported(&mut app, &format!("egrisel-{name}"));
        opened(&mut app, path.clone());
        let v = app.ifc_viewer.as_ref().and_then(|w| w.viewer.view()).expect("the model");
        let grid = v
            .input()
            .objects
            .iter()
            .position(|o| o.class == "IfcGrid" && o.label.contains("Eğrisel"))
            .expect("the curved grid");
        send(&mut app, Event::Viewer(V::Choose(Some(grid))));
        let w = app.ifc_viewer.as_ref().expect("the viewer");
        let details = w.viewer.details().expect("its details");
        let line = |name: &str| -> Vec<&str> {
            details.groups.iter().flat_map(|g| &g.lines).filter(|l| l.name == name).map(|l| l.value.as_str()).collect()
        };
        assert_eq!(line("Ön tanımlı tür"), ["RADIAL"]);
        assert_eq!((line("U aksları"), line("V aksları")), (vec!["4, 5, 6, 7"], vec!["E, F"]));
        let mut apart = Snapshot::new(Size::new(1400.0, 860.0)).expect("a renderer");
        apart.settle(&mut app, App::ifc_viewer_shown, &mut update);
        apart.settle(&mut app, App::ifc_viewer_shown, &mut update);
        let picture = out.join(format!("2-ifc-modeli-{name}.png"));
        apart.render(App::ifc_viewer_shown(&app), &app.theme()).save(&picture).expect("writes the picture");
        println!("{}", picture.display());
        let _ = std::fs::remove_dir_all(path.parent().expect("its folder"));
    }
}

/// Uzandığı kat (docs/adr/8168), light and dark: the section building with its east wall drawn once over both storeys
/// and on over the roof (fixtures/arch/v1/ifc/kesit-binasi-kalkan-duvari.ifc, the application test's) in IFC modeli,
/// the plan's east wall chosen; its details name the storey it stands in and the one it rises into.
/// `.run/gui-evidence/iki-kat-duvar-8168/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ifc_viewer_tests::span_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn span_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/iki-kat-duvar-8168");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = open(building());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let dir = std::env::temp_dir().join(format!("kentos-iki-kat-duvar-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let path = dir.join("kalkan-duvari.ifc");
        let file = include_bytes!("../../../../fixtures/arch/v1/ifc/kesit-binasi-kalkan-duvari.ifc");
        std::fs::write(&path, file).expect("writes");
        opened(&mut app, path);
        // The plan's east wall on the ground storey: the file's tall wall, its id kept.
        let doc = &app.document.as_ref().expect("the drawing").model;
        let (tall, slot) = doc
            .entities_with_uids()
            .find_map(|(u, e)| {
                let Entity::Wall(w) = e else { return None };
                let (p, q) = (w.wall.pts[0], w.wall.pts[w.wall.pts.len() - 1]);
                let east = p.x == 12.0 && q.x == 12.0 && p.y == 0.0 && q.y == 9.0;
                (east && w.wall.arch.level.as_deref() == Some("zemin")).then(|| (u, doc.slot_of(u)))
            })
            .expect("the east wall");
        app.selection.set(vec![slot.expect("its slot")]);
        app.follow_ifc_viewer();
        let w = app.ifc_viewer.as_ref().expect("the viewer");
        let v = w.viewer.view().expect("the model");
        let key = kentos_ifc::step::guid(tall.as_u128());
        assert_eq!(v.selected().map(|o| v.input().objects[o].key.as_str()), Some(key.as_str()));
        let details = w.viewer.details().expect("its details");
        let line = |name: &str| -> Vec<&str> {
            details.groups.iter().flat_map(|g| &g.lines).filter(|l| l.name == name).map(|l| l.value.as_str()).collect()
        };
        assert_eq!((line("Kat"), line("Uzandığı kat")), (vec!["MM-BNK-Zemin_kat"], vec!["MM-BNK-1._kat"]));
        let mut apart = Snapshot::new(Size::new(1400.0, 860.0)).expect("a renderer");
        apart.settle(&mut app, App::ifc_viewer_shown, &mut update);
        apart.settle(&mut app, App::ifc_viewer_shown, &mut update);
        let picture = out.join(format!("1-iki-kat-duvar-{name}.png"));
        apart.render(App::ifc_viewer_shown(&app), &app.theme()).save(&picture).expect("writes the picture");
        println!("{}", picture.display());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Kesit (tipten) (docs/adr/8167), light and dark: the typed section building's file (its column type's and beam type's
/// sections as profile sets) in IFC modeli, a column clicked, its details showing its type's section.
/// `.run/gui-evidence/kolon-kesiti-8167/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ifc_viewer_tests::section_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn section_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kolon-kesiti-8167");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = open(building());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let dir = std::env::temp_dir().join(format!("kentos-kolon-kesiti-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a folder");
        let path = dir.join("tipli.ifc");
        let typed = include_bytes!("../../../../fixtures/arch/v1/ifc/kesit-binasi-eleman-tipli.ifc");
        std::fs::write(&path, typed).expect("writes");
        opened(&mut app, path);
        let column = click_one(&mut app, Category::Column);
        let v = app.ifc_viewer.as_ref().and_then(|w| w.viewer.view()).expect("the model");
        assert_eq!(v.selected(), Some(column));
        let mut apart = Snapshot::new(Size::new(1400.0, 860.0)).expect("a renderer");
        apart.settle(&mut app, App::ifc_viewer_shown, &mut update);
        apart.settle(&mut app, App::ifc_viewer_shown, &mut update);
        let file = out.join(format!("1-kolon-kesiti-{name}.png"));
        apart.render(App::ifc_viewer_shown(&app), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// Malzeme katmanları (docs/adr/8166), light and dark: the section building's file in IFC modeli, a layered slab
/// clicked, its details listing how its usage lays its set and its layers from the top.
/// `.run/gui-evidence/doseme-katmanlari-8166/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ifc_viewer_tests::layer_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn layer_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out =
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/doseme-katmanlari-8166");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = open(building());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let path = exported(&mut app, &format!("katman-{name}"));
        opened(&mut app, path.clone());
        let slab = click_one(&mut app, Category::Slab);
        let v = app.ifc_viewer.as_ref().and_then(|w| w.viewer.view()).expect("the model");
        assert_eq!(v.selected(), Some(slab));
        let mut apart = Snapshot::new(Size::new(1400.0, 860.0)).expect("a renderer");
        apart.settle(&mut app, App::ifc_viewer_shown, &mut update);
        apart.settle(&mut app, App::ifc_viewer_shown, &mut update);
        let file = out.join(format!("1-doseme-katmanlari-{name}.png"));
        apart.render(App::ifc_viewer_shown(&app), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        let _ = std::fs::remove_dir_all(path.parent().expect("its folder"));
    }
}

#[test]
fn the_section_building_s_file_opens_whole_and_chooses_with_the_plan() {
    let mut app = open(building());
    let path = exported(&mut app, "bina");
    let (expected, zones) = shaped(&path);
    let _ = app.update(Message::Run("ifc.view"));
    opened(&mut app, path.clone());
    // Beside the plan, in a window of its own: nothing over the drawing.
    assert!(app.dialog.is_none(), "{:?}", app.dialog);
    let w = app.ifc_viewer.as_ref().expect("the viewer");
    let v = w.viewer.view().expect("the model read");
    assert_eq!(v.counts(), (expected, expected), "every body the file holds, as many as it holds");
    // Its spatial zones read too, an area's volume, not a body: hidden until Zonlar (docs/adr/8123).
    let zone_bodies = v.input().bodies.iter().filter(|b| v.input().objects[b.object].category == Category::Zone).count();
    assert_eq!(zone_bodies, zones, "the file's zones");
    assert!(zones > 0 && !v.zones, "zones hidden until asked for");
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
/// its own window (always: docs/adr/8122), a wall clicked; a level cut; the plan with the wall chosen; light and dark.
/// Drawn on the GPU:
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
            send(&mut app, Event::Viewer(V::AtDone));
            shot(&mut apart, &mut app, App::ifc_viewer_shown, "3-yatay-kesit");
            send(&mut app, Event::Viewer(V::Uncut));
            shot(&mut main, &mut app, App::view, "4-plan-secili");
        }
        let _ = std::fs::remove_dir_all(path.parent().expect("its folder"));
    }
}

/// Kesit yüzü (docs/adr/8131), light and dark: the section building's file in IFC modeli cut level at a third of its
/// height (its walls, columns and the stair cut filled dark), cut north–south through its middle (also at the fastest
/// quality), and boxed about a chosen wall. `.run/gui-evidence/kesit-kapagi-8131/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ifc_viewer_tests::cap_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn cap_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kesit-kapagi-8131");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut app = open(building());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(Size::new(1440.0, 900.0)));
        let path = exported(&mut app, &format!("kapak-{name}"));
        let mut apart = Snapshot::new(Size::new(1400.0, 860.0)).expect("a renderer");
        let mut shot = |app: &mut App, file: &str| {
            apart.settle(app, App::ifc_viewer_shown, &mut update);
            apart.settle(app, App::ifc_viewer_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            apart.render(App::ifc_viewer_shown(app), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        opened(&mut app, path.clone());
        for (cut, share, file) in [
            (kentos_model_view::Cut::Level, 0.3, "1-yatay-kesit"),
            (kentos_model_view::Cut::NorthSouth, 0.5, "2-kuzey-guney-kesiti"),
        ] {
            send(&mut app, Event::Viewer(V::Cut(cut)));
            let (lo, hi) = app.ifc_viewer.as_ref().and_then(|w| w.viewer.span(cut)).expect("a span");
            send(&mut app, Event::Viewer(V::At(lo + (hi - lo) * share)));
            send(&mut app, Event::Viewer(V::AtDone));
            shot(&mut app, file);
        }
        // The fastest quality, one sample a pixel: the same caps, their edges unsmoothed.
        let _ = app.settings.choose(&[("graphics.scene3dQuality", serde_json::Value::from("fast"))]);
        app.apply_settings();
        shot(&mut app, "2-kuzey-guney-kesiti-hizli");
        let _ = app.settings.choose(&[("graphics.scene3dQuality", serde_json::Value::from("high"))]);
        app.apply_settings();
        click_a_wall(&mut app);
        send(&mut app, Event::Viewer(V::Boxed));
        shot(&mut app, "3-kutu");
        let _ = std::fs::remove_dir_all(path.parent().expect("its folder"));
    }
}

/// Kesitin tutamakları (docs/adr/8132), light and dark, `.run/gui-evidence/kesit-tutamaklari-8132/`: the section
/// building's file cut level at a little over half its height, the handle dragged 60 cm down (lit, the cut faces
/// waiting) and let go (filled); a box about a chosen wall, its east handle dragged 1.5 m out; Yüzden kesit waiting for
/// its click; the curved building's file, a click on its arc wall's outer face (turned off the axes) making a plane
/// parallel to it, its handle dragged 2.5 m in.
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ifc_viewer_tests::handle_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn handle_screens() {
    use iced::Size;
    use kentos_model_view::{Cut, Section};
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kesit-tutamaklari-8132");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let mut update = |app: &mut App, message| {
        let _ = app.update(message);
    };
    let curved = || -> kentos_commands::DocumentSnapshotV2 {
        kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/curved-building.kcad")).expect("decodes")
    };
    let gesture = |app: &mut App, g: Gesture| send(app, Event::Viewer(V::Gesture(g)));
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        let mut apart = Snapshot::new(Size::new(1400.0, 860.0)).expect("a renderer");
        let mut shot = |app: &mut App, file: &str| {
            apart.settle(app, App::ifc_viewer_shown, &mut update);
            apart.settle(app, App::ifc_viewer_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            apart.render(App::ifc_viewer_shown(app), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let themed = |snapshot: kentos_commands::DocumentSnapshotV2| {
            let mut app = open(snapshot);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(Size::new(1440.0, 900.0)));
            app
        };
        let mut app = themed(building());
        let path = exported(&mut app, &format!("tutamak-{name}"));
        opened(&mut app, path.clone());
        send(&mut app, Event::Viewer(V::Cut(Cut::Level)));
        let (lo, hi) = app.ifc_viewer.as_ref().and_then(|w| w.viewer.span(Cut::Level)).expect("a span");
        send(&mut app, Event::Viewer(V::At(lo + (hi - lo) * 0.55)));
        send(&mut app, Event::Viewer(V::AtDone));
        gesture(&mut app, Gesture::Handle(0, -0.6));
        shot(&mut app, "1-yatay-kesit-surukleniyor");
        gesture(&mut app, Gesture::HandleDone(0));
        shot(&mut app, "2-yatay-kesit-birakildi");
        click_a_wall(&mut app);
        send(&mut app, Event::Viewer(V::Boxed));
        send(&mut app, Event::Viewer(V::Focus));
        gesture(&mut app, Gesture::Handle(1, 1.5));
        gesture(&mut app, Gesture::HandleDone(1));
        shot(&mut app, "3-kutu-tutamaklari");
        send(&mut app, Event::Viewer(V::Uncut));
        send(&mut app, Event::Viewer(V::View(0)));
        send(&mut app, Event::Viewer(V::FromFace));
        shot(&mut app, "4-yuzden-kesit-bekliyor");
        let _ = std::fs::remove_dir_all(path.parent().expect("its folder"));

        let mut app = themed(curved());
        let path = exported(&mut app, &format!("egri-{name}"));
        opened(&mut app, path.clone());
        send(&mut app, Event::Viewer(V::FromFace));
        // A wall face turned off the axes (between a sixth and a third of a right angle) that faces the camera.
        let vp = Viewport { width: 1000.0, height: 700.0 };
        let candidates: Vec<[f64; 3]> = {
            let w = app.ifc_viewer.as_ref().expect("the viewer");
            let eye = w.viewer.camera().eye;
            w.viewer
                .scene()
                .meshes()
                .iter()
                .filter(|m| m.part == 0)
                .flat_map(|m| m.triangles.iter().map(move |t| t.map(|i| m.vertices[i as usize])))
                .filter_map(|[a, b, c]| {
                    let n = kentos_model_view::section::normal_of(a, b, c)?;
                    let middle = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0, (a[2] + b[2] + c[2]) / 3.0];
                    let towards = (0..3).map(|k| n[k] * (eye[k] - middle[k])).sum::<f64>();
                    let turned = n[2].abs() < 0.05 && n[0].abs() > 0.25 && n[1].abs() > 0.5;
                    (turned && towards > 0.0).then_some(middle)
                })
                .collect()
        };
        assert!(!candidates.is_empty(), "the arc wall's faces");
        // A click may meet another face in front: Yüzden kesit again, the next face.
        for middle in candidates.iter().take(200) {
            let viewer = &app.ifc_viewer.as_ref().expect("the viewer").viewer;
            let (cam, facing) = (viewer.camera(), viewer.facing);
            let Some((x, y, _)) = cam.project(*middle, &vp) else { continue };
            if !facing {
                send(&mut app, Event::Viewer(V::FromFace));
            }
            gesture(&mut app, Gesture::Pick(x as f32, y as f32, 1000.0, 700.0));
            if matches!(app.ifc_viewer.as_ref().expect("the viewer").viewer.section, Section::Face { .. }) {
                break;
            }
        }
        let section = app.ifc_viewer.as_ref().expect("the viewer").viewer.section;
        assert!(matches!(section, Section::Face { .. }), "a plane parallel to the arc wall: {section:?}");
        gesture(&mut app, Gesture::Handle(0, -2.5));
        gesture(&mut app, Gesture::HandleDone(0));
        shot(&mut app, "5-yuze-paralel-kesit");
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

/// Gelişim seviyesi (docs/adr/8133), light and dark, `.run/gui-evidence/gelisim-seviyesi-8133/`: Model kalite kontrol
/// formu's table beside the section building, its general and model-structure rows with KentOS's notes: the 7th
/// answered from the written file's bodies (every element a solid); then beside the building with its library blocks
/// (the lift, the basin, the table), whose boxes leave the 7th to the author with what was measured.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib ifc_viewer_tests::lod_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn lod_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/gelisim-seviyesi-8133");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    fn update(app: &mut App, message: Message) {
        let _ = app.update(message);
    }
    for (theme, name) in [("light", "acik"), ("dark", "koyu")] {
        for (snapshot, file) in [
            (building(), "1-kesit-binasi-gecti"),
            (crate::scene3d_building_tests::with_the_missing_elements(), "2-kitaplik-bloklari-inceleme"),
        ] {
            let mut app = open(snapshot);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::RibbonTab("arch"));
            // The form's table beside the building, its answers the written file's.
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
            // Its top half (the general rows and the model's structure, the 7th among them), the questions' titles
            // through KentOS's notes, the bottom panel closed.
            let large = Size::new(2400.0, 1500.0);
            let _ = app.update(Message::WindowResized(large));
            if app.command_expanded {
                let _ = app.update(Message::CommandHistoryToggled);
            }
            let mut wide = Snapshot::new(large).expect("a renderer");
            wide.settle(&mut app, App::view, &mut update);
            let bounds = kentos_render_wgpu::Bounds { min_x: x0 + 0.42 * (x1 - x0), min_y: y0 + 0.5 * (y1 - y0), max_x: x1, max_y: y1 };
            app.viewport.camera.fit(&bounds, 24.0);
            wide.settle(&mut app, App::view, &mut update);
            wide.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            wide.render(App::view(&app), &app.theme()).save(&path).expect("writes the picture");
            println!("{}", path.display());
        }
    }
}
