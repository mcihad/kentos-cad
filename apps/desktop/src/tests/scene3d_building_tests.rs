//! 3B görünüm of a building (scene3d_building.rs, docs/adr/8067; plan/bim-plan.md BIM-06): on the section building
//! (fixtures/arch/v1/section-building.kcad, written without KentOS code) the window shows the bodies the IFC file holds,
//! as many as it holds; a click on a wall selects it in the plan and the plan's selection shows in 3D; the storeys and
//! the rooms hide; two rooms of one storey that share ground are the Mimari audit's finding and red in 3D.

use kentos_commands::Entity;
use kentos_scene3d::Viewport;

use crate::app::{App, Dialog, Message};
use crate::scene3d_window::Event as W;

/// The parts (scene3d_building::PARTS): a wall, a room, the selected, a room found something about.
const WALL: u32 = 0;
const ROOM: u32 = 9;
const SELECTED: u32 = 11;
const FOUND: u32 = 12;
const FOUND_ROOM: u32 = 14;

fn open(snapshot: kentos_commands::DocumentSnapshotV2) -> App {
    let (mut app, _) = App::boot(None);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn building() -> kentos_commands::DocumentSnapshotV2 {
    kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/section-building.kcad")).expect("decodes")
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Scene3d(e));
}

/// The STEP record's arguments at the top level (strings with their quotes, lists with their parentheses).
fn arguments(record: &str) -> Vec<String> {
    let inner = record.split_once('(').map_or("", |(_, r)| r);
    let inner = inner.trim_end().trim_end_matches(';').trim_end_matches(')');
    let (mut out, mut cur, mut depth, mut quoted) = (Vec::new(), String::new(), 0, false);
    for ch in inner.chars() {
        match ch {
            '\'' => quoted = !quoted,
            '(' if !quoted => depth += 1,
            ')' if !quoted => depth -= 1,
            ',' if !quoted && depth == 0 => {
                out.push(std::mem::take(&mut cur));
                continue;
            }
            _ => {}
        }
        cur.push(ch);
    }
    out.push(cur);
    out
}

/// How many building elements and spaces the IFC file `cad.arch.ifc` writes has with a body (an `IfcProduct`'s seventh
/// argument, its representation, given), read from the file's text: the bodies another reader shows (an opening is a
/// cut, not a body).
fn bodies_in_the_file(app: &mut App) -> usize {
    const CLASSES: [&str; 17] = [
        "IFCWALL", "IFCSLAB", "IFCCOLUMN", "IFCBEAM", "IFCROOF", "IFCRAILING", "IFCSTAIR", "IFCCOVERING", "IFCDOOR",
        "IFCWINDOW", "IFCTRANSPORTELEMENT", "IFCFURNITURE", "IFCSANITARYTERMINAL", "IFCMEMBER", "IFCCHIMNEY", "IFCRAMP",
        "IFCSPACE",
    ];
    let dir = std::env::temp_dir().join(format!("kentos-bina-3b-{}", std::process::id()));
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
    let text = std::fs::read_to_string(&path).expect("reads the file");
    let _ = std::fs::remove_dir_all(&dir);
    text.lines()
        .filter_map(|l| l.split_once('=').map(|(_, r)| r.trim()))
        .filter(|r| CLASSES.iter().any(|c| r.starts_with(&format!("{c}("))))
        .filter(|r| arguments(r).get(6).is_some_and(|a| a.trim() != "$"))
        .count()
}

/// A click on the `k`th mesh of `part` seen from the window's camera: the first of its triangles' middles the click
/// meets it at; the object's id.
fn click_on(app: &mut App, part: u32) -> Option<String> {
    let vp = Viewport { width: 1240.0, height: 560.0 };
    for k in 0..40 {
        for n in 0..24 {
            let w = app.scene3d_window.as_ref().expect("the window");
            let (cam, ex) = (w.camera(), w.exaggeration());
            let Some((p, owner)) = w.owner_on(part, k, n) else { break };
            let Some((x, y, _)) = cam.project(ex.shown(p), &vp) else { continue };
            send(app, W::Pick(x as f32, y as f32, 1240.0, 560.0));
            let picked = app.scene3d_window.as_ref().expect("the window").picked.clone();
            if picked.is_some_and(|q| q.owner.as_deref() == Some(owner.as_str())) {
                return Some(owner);
            }
        }
    }
    None
}

#[test]
fn the_section_building_shows_the_bodies_its_ifc_file_holds() {
    let mut app = open(building());
    let _ = app.update(Message::Run("scene3d.view"));
    assert_eq!(app.dialog, Some(Dialog::Scene3d));
    let w = app.scene3d_window.as_ref().expect("the window");
    let b = w.building.as_ref().expect("a building");
    let (bodies, rooms, storeys) = (b.bodies(), b.rooms_count(), b.storeys.len());
    assert_eq!(w.scene_sources(), bodies + rooms, "every body and room in the scene");
    assert!(w.scene_parts().contains(&WALL) && w.scene_parts().contains(&ROOM), "{:?}", w.scene_parts());
    assert_eq!(bodies + rooms, bodies_in_the_file(&mut app), "as many bodies as the IFC file holds");
    // 16 walls, 5 doors, 9 windows, 3 slabs, 16 columns, 8 beams, a stair and 10 rooms: the 68 bodies an independent
    // reader shows of the file.
    assert_eq!((bodies, rooms), (58, 10), "the section building's elements and rooms");
    assert_eq!(storeys, 2);
    // A storey hidden takes its bodies away; the rooms hidden take the pale volumes away; both come back.
    send(&mut app, W::Storey(storeys - 1));
    let w = app.scene3d_window.as_ref().expect("the window");
    assert!(w.scene_sources() < bodies + rooms);
    send(&mut app, W::Storey(storeys - 1));
    send(&mut app, W::Rooms);
    let w = app.scene3d_window.as_ref().expect("the window");
    assert_eq!(w.scene_sources(), bodies);
    assert!(!w.scene_parts().contains(&ROOM));
    send(&mut app, W::Rooms);
    assert_eq!(app.scene3d_window.as_ref().expect("the window").scene_sources(), bodies + rooms);
    // The building is seen as it is: no vertical exaggeration.
    assert_eq!(app.scene3d_window.as_ref().expect("the window").exaggeration().factor, 1.0);
}

#[test]
fn a_click_on_a_wall_selects_it_in_the_plan_and_the_plan_s_selection_shows_in_3d() {
    let mut app = open(building());
    let _ = app.update(Message::Run("scene3d.view"));
    let wall = click_on(&mut app, WALL).expect("a wall seen and clicked");
    let doc = &app.document.as_ref().expect("the building").model;
    let slot = doc.slot_of(kentos_domain::Uuid::parse_str(&wall).expect("an id")).expect("the wall's object");
    assert!(matches!(doc.get(slot), Some(Entity::Wall(_))), "a wall in the plan");
    assert_eq!(app.selection.ids().to_vec(), vec![slot], "the wall selected in the plan");
    let w = app.scene3d_window.as_ref().expect("the window");
    assert_eq!(w.parts_of(&wall), vec![SELECTED], "the wall drawn selected");
    assert!(w.picked.as_ref().is_some_and(|p| p.what.starts_with("Duvar")), "{:?}", w.picked);
    // The sky empties the selection, as a click on nothing in the plan does.
    send(&mut app, W::Pick(2.0, 2.0, 1240.0, 560.0));
    assert!(app.selection.ids().is_empty());
    assert!(!app.scene3d_window.as_ref().expect("the window").scene_parts().contains(&SELECTED));
    // Selected in the plan, shown selected when the window opens.
    send(&mut app, W::Close);
    app.selection.set(vec![slot]);
    let _ = app.update(Message::Run("scene3d.view"));
    assert_eq!(app.scene3d_window.as_ref().expect("the window").parts_of(&wall), vec![SELECTED]);
    // The model changes (the wall deleted, as an undo would while the window is open): the scene is made again.
    let before = app.scene3d_window.as_ref().expect("the window").building.as_ref().expect("a building").bodies();
    let doc = &mut app.document.as_mut().expect("the building").model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        "cad.entities.delete",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"uids": [wall]}),
    )
    .expect("deletes the wall");
    assert_eq!(answer["status"], "completed", "{answer}");
    send(&mut app, W::Fit);
    let w = app.scene3d_window.as_ref().expect("the window");
    assert_eq!(w.building.as_ref().expect("a building").bodies(), before - 1);
    assert!(w.parts_of(&wall).is_empty(), "the wall is gone from 3D");
}

/// The section building with a 1 m square room drawn about the centre of its first rectangular room, on that room's
/// storey.
fn with_an_overlapping_room() -> (kentos_commands::DocumentSnapshotV2, String, String) {
    let mut snapshot = building();
    let at = snapshot
        .entities
        .iter()
        .position(|e| matches!(e, Entity::Space(s) if s.space.parts.first().is_some_and(|p| p.outer.pts.len() == 4)))
        .expect("a room");
    let first = kentos_domain::Uuid::from_bytes(snapshot.uids[at].0).to_string();
    let Entity::Space(room) = snapshot.entities[at].clone() else { unreachable!() };
    let pts = &room.space.parts[0].outer.pts;
    let (x, y) = (pts.iter().map(|p| p.x).sum::<f64>() / pts.len() as f64, pts.iter().map(|p| p.y).sum::<f64>() / pts.len() as f64);
    let mut square = room.clone();
    square.base.id = snapshot.entities.iter().map(|e| e.base().id).max().unwrap_or(0) + 1;
    // Drawn, not found round from the walls: its boundary is the square.
    square.space.sources = None;
    square.space.seed = None;
    square.space.number = None;
    square.space.name = Some("Örtüşen".into());
    square.space.parts = vec![kentos_commands::SpacePart {
        outer: kentos_commands::SpaceRing {
            pts: vec![
                kentos_commands::Vec2 { x: x - 0.5, y: y - 0.5 },
                kentos_commands::Vec2 { x: x + 0.5, y: y - 0.5 },
                kentos_commands::Vec2 { x: x + 0.5, y: y + 0.5 },
                kentos_commands::Vec2 { x: x - 0.5, y: y + 0.5 },
            ],
            bulges: None,
        },
        holes: Vec::new(),
    }];
    let id = kentos_domain::Uuid::from_u128(0x8067_0000_0000_0000_0000_0000_0000_0001);
    snapshot.entities.push(Entity::Space(square));
    snapshot.uids.push(kentos_commands::EntityId(id.into_bytes()));
    (snapshot, first, id.to_string())
}

#[test]
fn two_rooms_of_one_storey_that_share_ground_are_found_and_red_in_3d() {
    // As drawn, the building's rooms do not overlap.
    let mut app = open(building());
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    let found = |app: &App| {
        app.arch.arch_audit.audited.as_ref().expect("checked").findings.iter().filter(|f| f.code == "digital_quality_room_overlap").cloned().collect::<Vec<_>>()
    };
    assert!(found(&app).is_empty(), "{:?}", found(&app));
    let (snapshot, first, square) = with_an_overlapping_room();
    let mut app = open(snapshot);
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    let f = found(&app);
    assert_eq!(f.len(), 1, "{f:?}");
    let mut uids = f[0].uids.clone();
    uids.sort();
    let mut want = vec![first.clone(), square.clone()];
    want.sort();
    assert_eq!(uids, want);
    assert!(f[0].message.contains("1.00 m² örtüşüyor"), "{}", f[0].message);
    // The finding's row chosen in Denetim: 3B görünüm paints its two rooms, nothing else.
    choose_the_overlap(&mut app);
    let _ = app.update(Message::Run("scene3d.view"));
    let w = app.scene3d_window.as_ref().expect("the window");
    assert_eq!(w.parts_of(&first), vec![FOUND_ROOM]);
    assert_eq!(w.parts_of(&square), vec![FOUND_ROOM]);
    let b = w.building.as_ref().expect("a building");
    assert!(b.legend().iter().any(|(name, _)| *name == "Bulgulu mahal"));
    assert!(!w.scene_parts().contains(&FOUND), "only the rooms found: {:?}", w.scene_parts());
    assert!(b.counted().contains("2 nesnede Denetim bulgusu"), "{}", b.counted());
}

/// The overlap's row chosen in Denetim, as a click on it chooses it.
fn choose_the_overlap(app: &mut App) {
    let at = app
        .arch
        .arch_audit
        .audited
        .as_ref()
        .expect("checked")
        .findings
        .iter()
        .position(|f| f.code == "digital_quality_room_overlap")
        .expect("the overlap");
    app.arch.arch_audit.chosen.clear();
    app.arch.arch_audit.selected = Some(at);
}

/// The building in 3D, wide and light, narrow and dark: the ribbon's place of 3B görünüm, the bodies from two sides, a
/// wall clicked and selected in the plan, the ground storey alone, the overlapping rooms red after Denetle and the
/// finding in Denetim; the pictures are the renderer's own (KENTOS_SNAPSHOT_BACKEND=wgpu);
/// `.run/gui-evidence/bina-3b-8067/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/bina-3b-8067");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let (doc, _, _) = with_an_overlapping_room();
        let mut app = open(doc);
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            // A second frame: the first uploads, the second draws what is resident (the same frame selects again).
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("arch"));
        shot(&mut app, "1-serit");
        let _ = app.update(Message::Run("scene3d.view"));
        shot(&mut app, "2-kuzeydogu");
        let seen = *app.scene3d_window.as_ref().expect("the window").seen.lock().expect("seen");
        assert!(seen.tiles > 0 && seen.triangles > 0 && seen.waiting == 0, "{seen:?}");
        if name == "acik" {
            send(&mut app, W::View(1));
            shot(&mut app, "3-guneybati");
            send(&mut app, W::View(0));
        }
        let wall = click_on(&mut app, WALL).expect("a wall clicked");
        shot(&mut app, "4-duvar-secili");
        send(&mut app, W::Close);
        shot(&mut app, "5-plan-secili");
        let _ = app.update(Message::Run("scene3d.view"));
        send(&mut app, W::Storey(1));
        if name == "acik" {
            shot(&mut app, "6-zemin-kat");
        }
        send(&mut app, W::Close);
        // Denetle: the overlapping rooms are a finding; 3B görünüm paints them red.
        let _ = app.update(Message::Run("arch.audit"));
        app.arch_audit_standards_settle();
        choose_the_overlap(&mut app);
        let _ = app.update(Message::Run("scene3d.view"));
        send(&mut app, W::Storey(1));
        send(&mut app, W::Dolly(2.0));
        shot(&mut app, "7-ortusen-mahaller");
        assert!(app.scene3d_window.as_ref().expect("the window").scene_parts().contains(&FOUND_ROOM));
        send(&mut app, W::Close);
        shot(&mut app, "8-denetim");
        let _ = wall;
    }
}

/// The section building with B9's elements (docs/adr/8070): a hip roof over its 1. kat with a chimney over the flues
/// of the hall's west wall on both storeys, an L ramp rising along its east side to ±0.00, and KentOS's library lift,
/// basin and table on the ground floor. Its snapshot after the commands that draw them.
fn with_the_missing_elements() -> kentos_commands::DocumentSnapshotV2 {
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = kentos_domain::Document::from_snapshot_v2(building()).expect("opens");
    let run = |doc: &mut kentos_domain::Document, name: &str, input: serde_json::Value| -> serde_json::Value {
        let answer = registry::run(&mut ExecutionContext::new(doc), name, kentos_commands::CallOp::Execute, input)
            .expect("runs");
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
        answer
    };
    let eaves = kentos_native_application::roofs::eaves_from_walls(
        &doc,
        kentos_commands::Vec2 { x: 6.0, y: 4.5 },
        Some("k1"),
        0.5,
    )
    .expect("the walls enclose the point");
    let slopes = vec![33.0; eaves.len()];
    run(
        &mut doc,
        "cad.roofs.create",
        serde_json::json!({"layerId": "0", "roofs": [{"pts": eaves, "slopes": slopes, "overhang": 0.5,
            "arch": {"level": "k1"},
            "parts": [{"kind": "chimney", "at": {"x": 4.5, "y": 8.0}, "width": 0.4, "depth": 0.4}]}]}),
    );
    let halls: Vec<String> = doc
        .entities_with_uids()
        .filter_map(|(u, e)| match e {
            Entity::Wall(w)
                if w.wall.pts.len() == 2 && w.wall.pts.iter().all(|q| (q.x - 4.5).abs() < 1e-9) =>
            {
                Some(u.to_string())
            }
            _ => None,
        })
        .collect();
    for wall in &halls {
        run(
            &mut doc,
            "cad.openings.create",
            serde_json::json!({"layerId": "0", "openings": [
                {"host": wall, "at": 8.0, "width": 0.2, "form": "flue", "operation": "fixed", "depth": 0.07}]}),
        );
    }
    let p = |x: f64, y: f64| serde_json::json!({"x": x, "y": y});
    run(
        &mut doc,
        "cad.ramps.create",
        serde_json::json!({"layerId": "0", "ramps": [{"pts": [p(13.8, 7.0), p(13.8, 1.5), p(16.0, 1.5)],
            "width": 1.2, "rise": 0.45, "base": -0.45, "arch": {"level": "zemin"}}]}),
    );
    let library = include_str!("../../../../catalog/v1/blocks/kentos-temel.kblok");
    run(&mut doc, "cad.blocks.edit", serde_json::json!({"operation": "import", "library": library}));
    for (name, x, y, extra) in [
        ("Asansör", 5.0, 6.5, serde_json::json!({"attributes": {"KISI": "8", "YUK": "630"}})),
        ("Lavabo", 9.0, 0.2, serde_json::json!({})),
        ("Masa", 2.2, 6.6, serde_json::json!({})),
    ] {
        let mut input = serde_json::json!({"name": name, "at": p(x, y), "layerId": "0", "arch": {"level": "zemin"}});
        for (k, v) in extra.as_object().into_iter().flatten() {
            input[k] = v.clone();
        }
        run(&mut doc, "cad.blocks.insert", input);
    }
    doc.to_snapshot_v2()
}

/// B9's elements in 3D (docs/adr/8070): the building from the north-east with its roof, its chimney and the ramp; then
/// the ground floor alone, its rooms hidden, with the lift, the basin and the table, from the north-east and the
/// south-west. `.run/gui-evidence/eksik-elemanlar-8070/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::missing_elements_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn missing_elements_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/eksik-elemanlar-8070");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(with_the_missing_elements());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::Run("scene3d.view"));
        shot(&mut app, "1-cati-baca-rampa");
        send(&mut app, W::Storey(1));
        send(&mut app, W::Rooms);
        send(&mut app, W::Dolly(1.5));
        shot(&mut app, "2-zemin-kat-donati");
        send(&mut app, W::View(1));
        send(&mut app, W::Dolly(1.5));
        shot(&mut app, "3-zemin-kat-guneybati");
        send(&mut app, W::View(3));
        shot(&mut app, "4-zemin-kat-ustten");
        send(&mut app, W::Close);
    }
}

/// Geometri doğruluğu (docs/adr/8071; plan/bim-plan.md BIM-10): the curved building (fixtures/arch/v1/curved-building.kcad)
/// in plan and in 3D, its north wall's and its slabs' arcs smooth (the view's own step), its storeys at ±0.00 and +3.35.
/// `.run/gui-evidence/egri-bina-8071/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::curved_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn curved_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/egri-bina-8071");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let curved = || -> kentos_commands::DocumentSnapshotV2 {
        kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/curved-building.kcad")).expect("decodes")
    };
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(curved());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        app.viewport.camera.fit(&kentos_render_wgpu::Bounds { min_x: -1.0, min_y: -1.0, max_x: 11.0, max_y: 9.0 }, 24.0);
        shot(&mut app, "1-plan");
        let _ = app.update(Message::Run("scene3d.view"));
        shot(&mut app, "2-3b");
        send(&mut app, W::View(1));
        shot(&mut app, "3-3b-guneybati");
        send(&mut app, W::Storey(1));
        send(&mut app, W::View(3));
        shot(&mut app, "4-3b-zemin-kat-ustten");
        send(&mut app, W::Close);
    }
}

/// The facade building (fixtures/arch/v1/facade-building.kcad) with a curtain wall 1.5 m east of its east facade
/// (docs/adr/8070 §7): 9 m long, 6 m high from the ground floor, at most 1.50 m between mullions and transoms (6 bays,
/// 4 rows), and its east elevation (mark 3, looking west) drawn at (20, 0).
fn with_a_curtain_wall() -> kentos_commands::DocumentSnapshotV2 {
    use kentos_native_application::{ExecutionContext, registry, sections_edit};
    let snap = kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/facade-building.kcad")).expect("decodes");
    let mut doc = kentos_domain::Document::from_snapshot_v2(snap).expect("opens");
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.walls.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": "0", "walls": [{
            "pts": [{"x": 13.5, "y": 0.0}, {"x": 13.5, "y": 9.0}], "width": 0.15, "height": 6.0,
            "arch": {"level": "zemin"},
            "curtain": {"spacing": 1.5, "rows": 1.5, "panel": "Cam", "frames": "Alüminyum"}
        }]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let east = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::ViewMark(v) if v.mark.form == kentos_commands::ViewMarkForm::Elevation && v.mark.mark == "3" => Some(u),
            _ => None,
        })
        .expect("mark 3");
    let derived = sections_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        kentos_commands::BuildingSectionsEdit {
            operation: kentos_commands::BuildingSectionsOperation::Derive {
                mark: east.to_string(),
                at: Some(kentos_commands::Vec2 { x: 20.0, y: 0.0 }),
            },
            expected_revision: None,
        },
    );
    assert!(matches!(derived, kentos_commands::CommandResult::Completed { .. }), "{derived:?}");
    doc.to_snapshot_v2()
}

/// Giydirme cephe in plan, in its elevation and in 3D (docs/adr/8070 §7): the plan near it (its mullions on its line,
/// its glass down its middle), the east elevation (its mullions and transoms over the building's east facade), and the
/// building from the north-east. `.run/gui-evidence/giydirme-cephe-8070/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::curtain_wall_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn curtain_wall_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/giydirme-cephe-8070");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(with_a_curtain_wall());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let fit = |app: &mut App, min: [f64; 2], max: [f64; 2]| {
            app.viewport.camera.fit(
                &kentos_render_wgpu::Bounds { min_x: min[0], min_y: min[1], max_x: max[0], max_y: max[1] },
                24.0,
            );
        };
        // A first frame lays the window out; the plan is framed after it.
        shot(&mut app, "0-acilis");
        fit(&mut app, [10.0, -1.0], [15.0, 4.0]);
        shot(&mut app, "1-plan");
        fit(&mut app, [18.0, -2.0], [34.0, 9.0]);
        shot(&mut app, "2-dogu-gorunusu");
        let _ = app.update(Message::Run("scene3d.view"));
        shot(&mut app, "3-3b-kuzeydogu");
        send(&mut app, W::Close);
    }
}

/// The facade building with a pergola east of it (docs/adr/8070 §8): four 10/10 Çelik posts 2.60 m tall (their own
/// height) at x 12.6 and 14.6, y 1 and 4; two 5/10 rails on them (y 1 and 4, their undersides at 2.50) and four 4/8
/// rafters across (RAFTER, over the rails at 2.60); then the east elevation derived 20 m east.
fn with_a_pergola() -> kentos_commands::DocumentSnapshotV2 {
    use kentos_native_application::{ExecutionContext, registry, sections_edit};
    let snap = kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/facade-building.kcad")).expect("decodes");
    let mut doc = kentos_domain::Document::from_snapshot_v2(snap).expect("opens");
    let mut run = |name: &str, input: serde_json::Value| {
        let answer = registry::run(&mut ExecutionContext::new(&mut doc), name, kentos_commands::CallOp::Execute, input)
            .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    };
    let post = |x: f64, y: f64| {
        serde_json::json!({"at": {"x": x, "y": y}, "width": 0.1, "depth": 0.1, "height": 2.6, "material": "Çelik",
                           "member": {"predefined": "POST"}, "arch": {"level": "zemin"}})
    };
    run("cad.columns.create", serde_json::json!({"layerId": "0", "columns": [
        post(12.6, 1.0), post(14.6, 1.0), post(12.6, 4.0), post(14.6, 4.0)
    ]}));
    let rail = |y: f64| {
        serde_json::json!({"pts": [{"x": 12.5, "y": y}, {"x": 14.7, "y": y}], "width": 0.05, "height": 0.1,
                           "baseOffset": 2.5, "material": "Çelik",
                           "member": {"predefined": "USERDEFINED", "name": "Kayıt"}, "arch": {"level": "zemin"}})
    };
    let rafter = |x: f64| {
        serde_json::json!({"pts": [{"x": x, "y": 0.7}, {"x": x, "y": 4.3}], "width": 0.04, "height": 0.08,
                           "baseOffset": 2.6, "material": "Ahşap", "member": {"predefined": "RAFTER"},
                           "arch": {"level": "zemin"}})
    };
    run("cad.beams.create", serde_json::json!({"layerId": "0", "beams": [
        rail(1.0), rail(4.0), rafter(12.9), rafter(13.4), rafter(13.9), rafter(14.4)
    ]}));
    let east = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::ViewMark(v) if v.mark.form == kentos_commands::ViewMarkForm::Elevation && v.mark.mark == "3" => Some(u),
            _ => None,
        })
        .expect("mark 3");
    let derived = sections_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        kentos_commands::BuildingSectionsEdit {
            operation: kentos_commands::BuildingSectionsOperation::Derive {
                mark: east.to_string(),
                at: Some(kentos_commands::Vec2 { x: 20.0, y: 0.0 }),
            },
            expected_revision: None,
        },
    );
    assert!(matches!(derived, kentos_commands::CommandResult::Completed { .. }), "{derived:?}");
    doc.to_snapshot_v2()
}

/// Dikme ve kayıt in plan, in the east elevation and in 3D (docs/adr/8070 §8): the pergola's posts and rails in plan, the
/// same over the east facade in its elevation, and the building from the north-east. `.run/gui-evidence/dikme-kayit-8070/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::member_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn member_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dikme-kayit-8070");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(with_a_pergola());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let fit = |app: &mut App, min: [f64; 2], max: [f64; 2]| {
            app.viewport.camera.fit(
                &kentos_render_wgpu::Bounds { min_x: min[0], min_y: min[1], max_x: max[0], max_y: max[1] },
                24.0,
            );
        };
        shot(&mut app, "0-acilis");
        fit(&mut app, [10.5, 0.0], [16.0, 5.0]);
        shot(&mut app, "1-plan");
        fit(&mut app, [18.0, -2.0], [34.0, 9.0]);
        shot(&mut app, "2-dogu-gorunusu");
        let _ = app.update(Message::Run("scene3d.view"));
        shot(&mut app, "3-3b-kuzeydogu");
        send(&mut app, W::Close);
    }
}

/// The facade building with an asma tavan in every ground storey room (docs/adr/8070 §10): each a 12.5 mm Alçıpan
/// covering bound to its room, its underside 2.60 over the storey's floor; then section A derived 20 m east.
fn with_ceilings() -> kentos_commands::DocumentSnapshotV2 {
    use kentos_native_application::{ExecutionContext, registry, sections_edit};
    let snap = kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/facade-building.kcad")).expect("decodes");
    let mut doc = kentos_domain::Document::from_snapshot_v2(snap).expect("opens");
    let rooms: Vec<String> = doc
        .entities_with_uids()
        .filter_map(|(u, e)| match e {
            Entity::Space(s) if !s.space.void && s.space.arch.level.as_deref() == Some("zemin") => Some(u.to_string()),
            _ => None,
        })
        .collect();
    let slabs: Vec<serde_json::Value> = rooms
        .iter()
        .map(|r| {
            serde_json::json!({"space": r, "thickness": 0.0125, "offset": 2.6125, "material": "Alçıpan",
                               "arch": {"level": "zemin"}, "covering": {"predefined": "CEILING", "name": "Asma tavan"}})
        })
        .collect();
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.slabs.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": "0", "slabs": slabs}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let section = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::ViewMark(v) if v.mark.form == kentos_commands::ViewMarkForm::Section && v.mark.mark == "A" => Some(u),
            _ => None,
        })
        .expect("section A");
    let derived = sections_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        kentos_commands::BuildingSectionsEdit {
            operation: kentos_commands::BuildingSectionsOperation::Derive {
                mark: section.to_string(),
                at: Some(kentos_commands::Vec2 { x: 20.0, y: 0.0 }),
            },
            expected_revision: None,
        },
    );
    assert!(matches!(derived, kentos_commands::CommandResult::Completed { .. }), "{derived:?}");
    doc.to_snapshot_v2()
}

/// Kaplama in section and in 3D (docs/adr/8070 §10): section A with the asma tavan under each ground storey room's
/// slab, and the ground storey alone in 3D from the south-west and from above. `.run/gui-evidence/kaplama-8070/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::covering_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn covering_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/kaplama-8070");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(with_ceilings());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let fit = |app: &mut App, min: [f64; 2], max: [f64; 2]| {
            app.viewport.camera.fit(
                &kentos_render_wgpu::Bounds { min_x: min[0], min_y: min[1], max_x: max[0], max_y: max[1] },
                24.0,
            );
        };
        shot(&mut app, "0-acilis");
        fit(&mut app, [18.0, -2.0], [38.0, 9.0]);
        shot(&mut app, "1-kesit-a");
        let _ = app.update(Message::Run("scene3d.view"));
        send(&mut app, W::Storey(1));
        send(&mut app, W::Rooms);
        send(&mut app, W::View(1));
        shot(&mut app, "2-3b-zemin-kat-guneybati");
        send(&mut app, W::View(3));
        shot(&mut app, "3-3b-zemin-kat-ustten");
        send(&mut app, W::Close);
    }
}

/// Kiriş çizelgesi and Kaplama çizelgesi (docs/adr/8070 §11): the pergola's building with an asma tavan in its first
/// ground storey room, the two schedules placed south of it. `.run/gui-evidence/eleman-cizelgeleri-8070/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::element_schedule_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn element_schedule_screens() {
    use iced::Size;
    use kentos_native_application::{ExecutionContext, registry};
    use kentos_ui::snapshot::Snapshot;

    let mut doc = kentos_domain::Document::from_snapshot_v2(with_a_pergola()).expect("opens");
    let room = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::Space(s) if !s.space.void && s.space.arch.level.as_deref() == Some("zemin") => Some(u.to_string()),
            _ => None,
        })
        .expect("a room");
    for (name, input) in [
        (
            "cad.slabs.create",
            serde_json::json!({"layerId": "0", "slabs": [{"space": room, "thickness": 0.0125, "offset": 2.6125,
                "material": "Alçıpan", "arch": {"level": "zemin"},
                "covering": {"predefined": "CEILING", "name": "Asma tavan"}}]}),
        ),
        ("cad.schedules.create", serde_json::json!({"layerId": "0", "at": {"x": 0.0, "y": -4.0}, "of": "beams"})),
        ("cad.schedules.create", serde_json::json!({"layerId": "0", "at": {"x": 0.0, "y": -10.0}, "of": "coverings"})),
    ] {
        let answer = registry::run(&mut ExecutionContext::new(&mut doc), name, kentos_commands::CallOp::Execute, input)
            .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    let snap = doc.to_snapshot_v2();
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/eleman-cizelgeleri-8070");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(snap.clone());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        shot(&mut app, "0-acilis");
        app.viewport.camera.fit(
            &kentos_render_wgpu::Bounds { min_x: -1.0, min_y: -13.0, max_x: 9.0, max_y: -3.0 },
            24.0,
        );
        shot(&mut app, "1-cizelgeler");
    }
}
