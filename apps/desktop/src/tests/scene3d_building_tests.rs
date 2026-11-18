//! 3B görünüm of a building (scene3d_building.rs, docs/adr/8067, 8119; plan/bim-plan.md BIM-06): on the section
//! building (fixtures/arch/v1/section-building.kcad, written without KentOS code) the window's viewer shows the bodies
//! the IFC file holds, as many as it holds; a click on a wall selects it in the plan and the plan's selection shows in
//! 3D; the storeys and the rooms hide; an edit makes the model again and the view stays; two rooms of one storey that
//! share ground are the Mimari audit's finding and red in 3D.

use kentos_commands::Entity;
use kentos_model_view::{Category, parts};
use kentos_scene3d::Viewport;

use crate::app::{App, Message};
use crate::scene3d_window::Event as W;

/// The viewer's parts (kentos_model_view::parts): a wall, a room, the selected, a found body, a room found something
/// about.
const WALL: u32 = 0;
const ROOM: u32 = parts::SPACE;
const SELECTED: u32 = parts::SELECTED;
const FOUND: u32 = parts::FOUND;
const FOUND_ROOM: u32 = parts::FOUND_SPACE;

pub(crate) fn open(snapshot: kentos_commands::DocumentSnapshotV2) -> App {
    let (mut app, _) = App::boot(None);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

pub(crate) fn building() -> kentos_commands::DocumentSnapshotV2 {
    kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/section-building.kcad")).expect("decodes")
}

fn send(app: &mut App, e: W) {
    let _ = app.update(Message::Scene3d(e));
}

/// The STEP record's arguments at the top level (strings with their quotes, lists with their parentheses).
pub(crate) fn arguments(record: &str) -> Vec<String> {
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

/// How many building elements, spaces and spatial zones the IFC file `cad.arch.ifc` writes has with a body (an
/// `IfcProduct`'s seventh argument, its representation, given), read from the file's text: the bodies another reader
/// shows (an opening is a cut, not a body).
fn bodies_in_the_file(app: &mut App) -> usize {
    const CLASSES: [&str; 18] = [
        "IFCWALL", "IFCSLAB", "IFCCOLUMN", "IFCBEAM", "IFCROOF", "IFCRAILING", "IFCSTAIR", "IFCCOVERING", "IFCDOOR",
        "IFCWINDOW", "IFCTRANSPORTELEMENT", "IFCFURNITURE", "IFCSANITARYTERMINAL", "IFCMEMBER", "IFCCHIMNEY", "IFCRAMP",
        "IFCSPACE", "IFCSPATIALZONE",
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

/// A click on the `k`th mesh of `part` seen from the viewer's camera: the first of its triangles' middles the click
/// meets it at; the object's persistent id.
fn click_on(app: &mut App, part: u32) -> Option<String> {
    let vp = Viewport { width: 1240.0, height: 560.0 };
    for k in 0..40 {
        for n in 0..24 {
            let w = app.scene3d_window.as_ref().expect("the window");
            let (cam, ex) = w.drawn_camera();
            let Some((p, owner)) = w.owner_on(part, k, n) else { break };
            let Some((x, y, _)) = cam.project(ex.shown(p), &vp) else { continue };
            send(app, W::Pick(x as f32, y as f32, 1240.0, 560.0));
            if app.scene3d_window.as_ref().expect("the window").picked_owner().as_deref() == Some(owner.as_str()) {
                return Some(owner);
            }
        }
    }
    None
}

/// The viewer's bodies (the openings' volumes are holes, not bodies) and its rooms' and zones' bodies.
fn shown_bodies(app: &App) -> (usize, usize) {
    let b = app.scene3d_window.as_ref().and_then(|w| w.building.as_ref()).expect("a building");
    let v = b.viewer.view().expect("the model shown");
    let input = v.input();
    let rooms = input.bodies.iter().filter(|b| input.objects[b.object].category == Category::Space).count();
    (v.counts().1, rooms)
}

#[test]
fn the_section_building_shows_the_bodies_its_ifc_file_holds() {
    let mut app = open(building());
    let _ = app.update(Message::Run("scene3d.view"));
    // Beside the plan, in a window of its own (scene3d_view.rs): nothing over the drawing.
    assert!(app.dialog.is_none() && app.surface_wanted(crate::windows::Surface::Scene3d));
    let w = app.scene3d_window.as_ref().expect("the window");
    let b = w.building.as_ref().expect("a building");
    let (bodies, rooms, zones, storeys) =
        (b.bodies(), b.with_body_of("IfcSpace"), b.with_body_of("IfcSpatialZone"), b.storeys_count());
    let (all, room_bodies) = shown_bodies(&app);
    assert_eq!(w.scene_sources(), all, "every body and room in the scene");
    assert!(w.scene_parts().contains(&WALL) && w.scene_parts().contains(&ROOM), "{:?}", w.scene_parts());
    assert_eq!(bodies + rooms + zones, bodies_in_the_file(&mut app), "as many objects with a body as the IFC file holds");
    // 16 walls, 5 doors, 9 windows, 3 slabs, 16 columns, 8 beams, a stair, 10 rooms and the 4 far zones (each storey's
    // included and excluded area, docs/adr/8069): the 72 bodies an independent reader shows of the file. The zones are
    // see-through volumes like the rooms.
    assert_eq!((bodies, rooms, zones), (58, 10, 4), "the section building's elements, rooms and zones");
    assert_eq!(storeys, 2);
    // The viewer's panes: the tree and the details beside the scene, the model's schema in the status bar.
    let b = app.scene3d_window.as_ref().and_then(|w| w.building.as_ref()).expect("a building");
    assert!(!b.viewer.rows().is_empty(), "the tree");
    assert!(b.viewer.facts.iter().any(|(words, _)| words.starts_with("IFC4")), "{:?}", b.viewer.facts);
    // A storey hidden takes its bodies away; the rooms hidden take the pale volumes away; both come back.
    send(&mut app, W::Storey(storeys - 1));
    let w = app.scene3d_window.as_ref().expect("the window");
    assert!(w.scene_sources() < all);
    send(&mut app, W::Storey(storeys - 1));
    send(&mut app, W::Rooms);
    let w = app.scene3d_window.as_ref().expect("the window");
    assert_eq!(w.scene_sources(), all - room_bodies);
    assert!(!w.scene_parts().contains(&ROOM));
    send(&mut app, W::Rooms);
    assert_eq!(app.scene3d_window.as_ref().expect("the window").scene_sources(), all);
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
    let v = w.building.as_ref().and_then(|b| b.viewer.view()).expect("the model shown");
    let chosen = v.selected().expect("chosen in the viewer");
    assert_eq!(v.input().objects[chosen].class, "IfcWall");
    assert!(w.building.as_ref().and_then(|b| b.viewer.details()).is_some(), "its details read from the model");
    // The sky empties the selection, as a click on nothing in the plan does.
    send(&mut app, W::Pick(2.0, 2.0, 1240.0, 560.0));
    assert!(app.selection.ids().is_empty());
    assert!(!app.scene3d_window.as_ref().expect("the window").scene_parts().contains(&SELECTED));
    // Selected in the plan, shown selected when the window opens.
    send(&mut app, W::Close);
    app.selection.set(vec![slot]);
    let _ = app.update(Message::Run("scene3d.view"));
    assert_eq!(app.scene3d_window.as_ref().expect("the window").parts_of(&wall), vec![SELECTED]);
    // The plan's selection changes while the window is open beside it: the next message paints it.
    app.selection.set(Vec::new());
    send(&mut app, W::Legend);
    assert_eq!(app.scene3d_window.as_ref().expect("the window").parts_of(&wall), vec![WALL]);
    app.selection.set(vec![slot]);
    send(&mut app, W::Legend);
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

#[test]
fn an_edit_makes_the_model_again_and_the_view_stays() {
    let mut app = open(building());
    let _ = app.update(Message::Run("scene3d.view"));
    // The view as the user left it: turned, the ground storey hidden, a search typed, a level cut.
    send(&mut app, W::View(2));
    send(&mut app, W::Storey(0));
    send(&mut app, W::Viewer(kentos_model_view_ui::Message::Search("kolon".into())));
    send(&mut app, W::Viewer(kentos_model_view_ui::Message::Cut(kentos_model_view::Cut::Level)));
    let before = {
        let b = app.scene3d_window.as_ref().and_then(|w| w.building.as_ref()).expect("a building");
        (b.viewer.orbit, b.viewer.query.clone(), b.viewer.section, b.viewer.found().len(), b.bodies())
    };
    // A column deleted, as an undo would while the window is open.
    let column = {
        let snap = app.document.as_ref().expect("the building").model.to_snapshot_v2();
        snap.entities.iter().zip(&snap.uids).find(|(e, _)| matches!(e, Entity::Column(_))).map(|(_, u)| u.to_string()).expect("a column")
    };
    let doc = &mut app.document.as_mut().expect("the building").model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        "cad.entities.delete",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"uids": [column]}),
    )
    .expect("deletes the column");
    assert_eq!(answer["status"], "completed", "{answer}");
    send(&mut app, W::Legend);
    let b = app.scene3d_window.as_ref().and_then(|w| w.building.as_ref()).expect("a building");
    assert_eq!(b.bodies(), before.4 - 1, "made again from the drawing");
    assert_eq!(b.viewer.orbit, before.0, "the camera stays");
    assert_eq!(b.viewer.query, before.1, "the search stays");
    assert_eq!(b.viewer.section, before.2, "the section stays");
    assert_eq!(b.viewer.found().len(), before.3 - 1, "the search finds one column fewer");
    assert_eq!(b.storeys_count(), 2);
    let w = app.scene3d_window.as_ref().expect("the window");
    assert!(w.parts_of(&column).is_empty(), "the column is gone from 3D");
    // The ground storey still hidden, by its key.
    let hidden = b.viewer.view().map(|v| {
        v.input().objects.iter().enumerate().filter(|(i, o)| o.category == Category::Storey && v.is_hidden(*i)).count()
    });
    assert_eq!(hidden, Some(1), "the storey hidden stays hidden");
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
    assert_eq!(b.found_shown(), 2, "the status bar's findings");
    assert!(b.viewer.facts.iter().any(|(words, _)| words == "2 nesnede bulgu"), "{:?}", b.viewer.facts);
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

/// The building in 3D, wide and light, narrow and dark: the ribbon's place of 3B görünüm, the viewer with its tree and
/// details (docs/adr/8119) beside the plan, Katlar ▾, the bodies from two sides, a wall clicked and selected in the
/// plan, the ground storey alone, the overlapping rooms red after Denetle and the finding in Denetim;
/// the pictures are the renderer's own (KENTOS_SNAPSHOT_BACKEND=wgpu); `.run/gui-evidence/bina-3b-8067/`:
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
            snapshot.settle(app, App::scene3d_shown, &mut update);
            // A second frame: the first uploads, the second draws what is resident (the same frame selects again).
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let _ = app.update(Message::RibbonTab("arch"));
        shot(&mut app, "1-serit");
        let _ = app.update(Message::Run("scene3d.view"));
        shot(&mut app, "2-kuzeydogu");
        let seen = *app.scene3d_window.as_ref().expect("the window").seen.lock().expect("seen");
        assert!(seen.tiles > 0 && seen.triangles > 0 && seen.waiting == 0, "{seen:?}");
        if name == "acik" {
            // Katlar ▾ open on the strip (its button at the strip's end, before Ana pencereye al), the 1. kat
            // unticked from it; a click on the strip's empty middle (after Kesit ▾) closes it.
            use kentos_ui::snapshot::Input;
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            let mut menu = Snapshot::new(size).expect("a renderer");
            menu.settle(&mut app, App::scene3d_shown, &mut update);
            menu.input(&mut app, App::scene3d_shown, &mut update, Input::Click(iced::Point::new(1355.0, 15.0)));
            send(&mut app, W::Storey(1));
            menu.settle(&mut app, App::scene3d_shown, &mut update);
            menu.settle(&mut app, App::scene3d_shown, &mut update);
            let file = out.join(format!("2b-katlar-menusu-{name}.png"));
            menu.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
            menu.input(&mut app, App::scene3d_shown, &mut update, Input::Click(iced::Point::new(1150.0, 15.0)));
            send(&mut app, W::Storey(1));
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
            snapshot.settle(app, App::scene3d_shown, &mut update);
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
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
            snapshot.settle(app, App::scene3d_shown, &mut update);
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
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
        // The spiral stair (docs/adr/8071 §9) from 60° over the ground, nearer: its treads climbing round the post.
        send(&mut app, W::Orbit(0.0, -97.0));
        send(&mut app, W::Dolly(3.0));
        shot(&mut app, "5-3b-doner-merdiven");
        send(&mut app, W::Close);
    }
}

/// Dosya › Dışa aktar with the curved building open (docs/adr/8071 §4): IFC dışa aktar (bina) and, beside it, IFC dışa
/// aktar (bina, kesilmiş gövde). `.run/gui-evidence/egri-bina-8071/5-disa-aktar-*.png`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::ifc_export_menu_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn ifc_export_menu_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/egri-bina-8071");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(
            kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/curved-building.kcad")).expect("decodes"),
        );
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        app.tab = "file";
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        crate::tools_screens::open_ribbon_menu(&mut snapshot, &mut app, "Dışa aktar");
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("5-disa-aktar-{name}.png"));
        snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}

/// The section building with a 1 × 1 m shaft (a void, docs/adr/0708) on its 1. kat (docs/adr/8071 §5): in 3D its
/// storey's floor slab has the shaft's hole beside the stair's. `.run/gui-evidence/egri-bina-8071/6-saft-*.png`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::shaft_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn shaft_screens() {
    use iced::Size;
    use kentos_native_application::{ExecutionContext, registry};
    use kentos_ui::snapshot::Snapshot;

    let mut doc = kentos_domain::Document::from_snapshot_v2(building()).expect("opens");
    let p = |x: f64, y: f64| serde_json::json!({"x": x, "y": y});
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.spaces.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": "0", "spaces": [{
            "parts": [{"outer": {"pts": [p(9.0, 6.0), p(10.0, 6.0), p(10.0, 7.0), p(9.0, 7.0)]}}],
            "number": "1S1", "name": "Şaft", "void": true, "arch": {"level": "k1"}
        }]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    // The terrace over the 1. kat would hide its floor from above: left out of the picture.
    let terrace = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::Slab(sl) if sl.slab.role == kentos_commands::SlabRole::Terrace => Some(u.to_string()),
            _ => None,
        })
        .expect("the terrace");
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.entities.delete",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"uids": [terrace]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/egri-bina-8071");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(doc.to_snapshot_v2());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let _ = app.update(Message::Run("scene3d.view"));
        // The 1. kat alone, its rooms off, from above: its floor slab with the stair's hole and the shaft's.
        send(&mut app, W::Storey(0));
        send(&mut app, W::Rooms);
        send(&mut app, W::View(3));
        snapshot.settle(&mut app, App::view, &mut update);
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("6-saft-{name}.png"));
        snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
        send(&mut app, W::Close);
    }
}

/// Eğimli döşeme (docs/adr/8071 §6): the section building with a 4 × 3 m slab east of it rising 10 % northward from
/// 3.00 m over the ground, selected: its Öznitelikler's Eğim and Eğim yönü, and in 3D from the south west.
/// `.run/gui-evidence/egri-bina-8071/7-egimli-doseme-*.png`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::sloped_slab_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn sloped_slab_screens() {
    use iced::Size;
    use kentos_native_application::{ExecutionContext, registry};
    use kentos_ui::snapshot::Snapshot;

    let mut doc = kentos_domain::Document::from_snapshot_v2(building()).expect("opens");
    let p = |x: f64, y: f64| serde_json::json!({"x": x, "y": y});
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.slabs.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": "0", "slabs": [{
            "parts": [{"outer": {"pts": [p(13.0, 0.0), p(17.0, 0.0), p(17.0, 3.0), p(13.0, 3.0)]}}],
            "thickness": 0.2, "offset": 3.0, "arch": {"level": "zemin"},
            "slope": {"from": p(13.0, 0.0), "to": p(13.0, 3.0), "rise": 0.3}
        }]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let uid = kentos_domain::Uuid::parse_str(answer["output"]["uids"][0].as_str().expect("its id")).expect("an id");
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/egri-bina-8071");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    // Tall enough for Öznitelikler's Eğim rows under the slab's others.
    for (theme, size, name) in [("light", Size::new(1440.0, 1250.0), "acik"), ("dark", Size::new(1280.0, 1150.0), "koyu")] {
        let mut app = open(doc.to_snapshot_v2());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let slot = app.document.as_ref().and_then(|d| d.model.slot_of(uid)).expect("the sloped slab");
        app.selection.set(vec![slot]);
        app.viewport.camera.fit(&kentos_render_wgpu::Bounds { min_x: -1.0, min_y: -1.0, max_x: 18.0, max_y: 10.0 }, 24.0);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::scene3d_shown, &mut update);
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        shot(&mut app, "7-egimli-doseme-oznitelikler");
        let _ = app.update(Message::Run("scene3d.view"));
        send(&mut app, W::View(1));
        shot(&mut app, "8-egimli-doseme-3b");
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
            snapshot.settle(app, App::scene3d_shown, &mut update);
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
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

/// The facade building with the curtain wall of `with_a_curtain_wall` (east of it, 9 m along y, 6 m high, 1.50 m bays
/// and rows), a door in its third bay (its head 2.10 rounded up to the transom at 3.00) and a window over its fifth
/// and sixth bays on the third row (docs/adr/8129); then the east elevation derived 20 m east.
fn with_curtain_openings() -> kentos_commands::DocumentSnapshotV2 {
    use kentos_native_application::{ExecutionContext, registry, sections_edit};
    let snap = kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/facade-building.kcad")).expect("decodes");
    let mut doc = kentos_domain::Document::from_snapshot_v2(snap).expect("opens");
    let run = |doc: &mut kentos_domain::Document, name: &str, input: serde_json::Value| {
        let answer = registry::run(&mut ExecutionContext::new(doc), name, kentos_commands::CallOp::Execute, input)
            .expect("runs");
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
        answer
    };
    let answer = run(
        &mut doc,
        "cad.walls.create",
        serde_json::json!({"layerId": "0", "walls": [{
            "pts": [{"x": 13.5, "y": 0.0}, {"x": 13.5, "y": 9.0}], "width": 0.15, "height": 6.0,
            "arch": {"level": "zemin"},
            "curtain": {"spacing": 1.5, "rows": 1.5, "panel": "Cam", "frames": "Alüminyum"}
        }]}),
    );
    let wall = answer["output"]["uids"][0].as_str().expect("its id").to_owned();
    run(
        &mut doc,
        "cad.openings.create",
        serde_json::json!({"layerId": "0", "openings": [
            {"host": wall, "at": 3.75, "width": 1.0, "side": "right"},
            {"host": wall, "form": "window", "at": 7.5, "width": 3.0, "sill": 3.2, "head": 4.4}
        ]}),
    );
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

/// Giydirme cephede kapı ve pencere (docs/adr/8129): the plan near the door (its leaf and swing between the mullions'
/// faces, the glass stopped at its jambs), the east elevation (the door's and the window's lines in their cells, the
/// mullion between the window's bays gone there), and the building from the north-east and from the east in 3D.
/// `.run/gui-evidence/giydirme-kapi-8129/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::curtain_opening_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn curtain_opening_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/giydirme-kapi-8129");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(with_curtain_openings());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::scene3d_shown, &mut update);
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let fit = |app: &mut App, min: [f64; 2], max: [f64; 2]| {
            app.viewport.camera.fit(
                &kentos_render_wgpu::Bounds { min_x: min[0], min_y: min[1], max_x: max[0], max_y: max[1] },
                24.0,
            );
        };
        shot(&mut app, "0-acilis");
        fit(&mut app, [11.0, 1.0], [16.0, 6.0]);
        shot(&mut app, "1-plan-kapi");
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
            snapshot.settle(app, App::scene3d_shown, &mut update);
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
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
            snapshot.settle(app, App::scene3d_shown, &mut update);
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
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

/// The section building with railings both sides of its stair and of a ramp east of it (B9's ramp, rising 0.45 m to the
/// ground storey), then sections A and B derived 20 and 45 m east (docs/adr/8128).
fn with_railings() -> kentos_commands::DocumentSnapshotV2 {
    use kentos_native_application::{ExecutionContext, registry, sections_edit};
    let mut doc = kentos_domain::Document::from_snapshot_v2(building()).expect("opens");
    let run = |doc: &mut kentos_domain::Document, name: &str, input: serde_json::Value| {
        let answer = registry::run(&mut ExecutionContext::new(doc), name, kentos_commands::CallOp::Execute, input)
            .expect("runs");
        assert_eq!(answer["status"], "completed", "{name}: {answer}");
    };
    let stair = doc
        .entities_with_uids()
        .find_map(|(u, e)| matches!(e, Entity::Stair(_)).then(|| u.to_string()))
        .expect("its stair");
    run(&mut doc, "cad.arch.edit", serde_json::json!({"uids": [stair], "operation": {"kind": "set", "railings": "both"}}));
    let p = |x: f64, y: f64| serde_json::json!({"x": x, "y": y});
    run(
        &mut doc,
        "cad.ramps.create",
        serde_json::json!({"layerId": "0", "ramps": [{"pts": [p(13.8, 7.0), p(13.8, 1.5), p(16.0, 1.5)],
            "width": 1.2, "rise": 0.45, "base": -0.45, "railings": "both", "arch": {"level": "zemin"}}]}),
    );
    for (letter, x) in [("A", 20.0), ("B", 45.0)] {
        let mark = doc
            .entities_with_uids()
            .find_map(|(u, e)| match e {
                Entity::ViewMark(v) if v.mark.form == kentos_commands::ViewMarkForm::Section && v.mark.mark == letter => {
                    Some(u)
                }
                _ => None,
            })
            .expect("the section");
        let derived = sections_edit::execute(
            &mut ExecutionContext::new(&mut doc),
            kentos_commands::BuildingSectionsEdit {
                operation: kentos_commands::BuildingSectionsOperation::Derive {
                    mark: mark.to_string(),
                    at: Some(kentos_commands::Vec2 { x, y: 0.0 }),
                },
                expected_revision: None,
            },
        );
        assert!(matches!(derived, kentos_commands::CommandResult::Completed { .. }), "{derived:?}");
    }
    doc.to_snapshot_v2()
}

/// Merdiven ve rampa korkuluğu in section and in 3D (docs/adr/8128): the plan, sections A and B with the stair's and the
/// ramp's top rails and balusters, the building from the north-east with the ramp's railings, and the ground storey
/// alone with the stair's. `.run/gui-evidence/korkuluk-8128/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::railing_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn railing_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/korkuluk-8128");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(with_railings());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::scene3d_shown, &mut update);
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let fit = |app: &mut App, min: [f64; 2], max: [f64; 2]| {
            app.viewport.camera.fit(
                &kentos_render_wgpu::Bounds { min_x: min[0], min_y: min[1], max_x: max[0], max_y: max[1] },
                24.0,
            );
        };
        fit(&mut app, [-1.0, -1.0], [65.0, 10.0]);
        shot(&mut app, "0-hepsi");
        fit(&mut app, [-1.0, -1.0], [17.0, 10.0]);
        shot(&mut app, "1-plan");
        fit(&mut app, [18.0, -2.0], [40.0, 9.0]);
        shot(&mut app, "2-kesit-a");
        fit(&mut app, [43.0, -2.0], [65.0, 9.0]);
        shot(&mut app, "3-kesit-b");
        let _ = app.update(Message::Run("scene3d.view"));
        shot(&mut app, "4-3b-kuzeydogu");
        send(&mut app, W::Storey(1));
        send(&mut app, W::Rooms);
        send(&mut app, W::Dolly(1.5));
        shot(&mut app, "5-3b-zemin-kat");
        send(&mut app, W::View(1));
        send(&mut app, W::Dolly(1.5));
        shot(&mut app, "6-3b-zemin-kat-guneybati");
        send(&mut app, W::Close);
    }
}

/// The section building with a hip roof over its 1. kat at 33 % (its eaves 50 cm out of the walls) in five layers top
/// down — kiremit 3 cm, lata 3 cm, çam kaplama tahtası 2.5 cm, ısı yalıtımı 5 cm, çam mertek 12.5 cm (docs/adr/8071 §8)
/// — and section B derived 20 m east.
fn with_a_layered_roof() -> kentos_commands::DocumentSnapshotV2 {
    use kentos_native_application::{ExecutionContext, registry, roofs, sections_edit};
    let mut doc = kentos_domain::Document::from_snapshot_v2(building()).expect("opens");
    let eaves = roofs::eaves_from_walls(&doc, kentos_commands::Vec2 { x: 6.0, y: 4.5 }, Some("k1"), 0.5)
        .expect("the walls enclose the point");
    let slopes = vec![33.0; eaves.len()];
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.roofs.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": "0", "roofs": [{"pts": eaves, "slopes": slopes, "overhang": 0.5,
            "arch": {"level": "k1"}, "layers": [
                {"kind": "covering", "thickness": 0.03, "material": "Kiremit"},
                {"kind": "battens", "thickness": 0.03},
                {"kind": "boarding", "thickness": 0.025, "material": "Çam"},
                {"kind": "insulation", "thickness": 0.05},
                {"kind": "rafters", "thickness": 0.125, "material": "Çam"}
            ]}]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let section = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::ViewMark(v) if v.mark.form == kentos_commands::ViewMarkForm::Section && v.mark.mark == "B" => Some(u),
            _ => None,
        })
        .expect("section B");
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

/// Çatı katmanları in section and in 3D (docs/adr/8071 §8): section B with the roof's five layers cut down its fall
/// lines, its eaves near, and the building in 3D. `.run/gui-evidence/cati-katmanlari-8071/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::roof_layers_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn roof_layers_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/cati-katmanlari-8071");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(with_a_layered_roof());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::scene3d_shown, &mut update);
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        let fit = |app: &mut App, min: [f64; 2], max: [f64; 2]| {
            app.viewport.camera.fit(
                &kentos_render_wgpu::Bounds { min_x: min[0], min_y: min[1], max_x: max[0], max_y: max[1] },
                24.0,
            );
        };
        fit(&mut app, [18.0, -2.0], [36.0, 9.5]);
        shot(&mut app, "1-kesit-b");
        // Its south eaves near: the five layers each under the one over it, the covering and the insulation hatched.
        fit(&mut app, [20.4, 5.3], [22.6, 6.6]);
        shot(&mut app, "2-kesit-b-sacak");
        let _ = app.update(Message::Run("scene3d.view"));
        send(&mut app, W::View(1));
        shot(&mut app, "3-3b-guneybati");
        send(&mut app, W::Close);
    }
}

/// The section building with an L roof over its 1. kat (docs/adr/8071 §10): 33 % on every edge, its eaves 50 cm out of
/// the walls on the south and west and turning in at (6.0, 4.0).
fn with_an_l_roof() -> kentos_commands::DocumentSnapshotV2 {
    use kentos_native_application::{ExecutionContext, registry};
    let mut doc = kentos_domain::Document::from_snapshot_v2(building()).expect("opens");
    let l = [(-0.625, -0.625), (12.625, -0.625), (12.625, 4.0), (6.0, 4.0), (6.0, 9.625), (-0.625, 9.625)];
    let pts: Vec<serde_json::Value> = l.iter().map(|(x, y)| serde_json::json!({"x": x, "y": y})).collect();
    let answer = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.roofs.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": "0", "roofs": [{"pts": pts, "slopes": vec![33.0; 6], "arch": {"level": "k1"}}]}),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    doc.to_snapshot_v2()
}

/// İçe dönen saçak in plan and in 3D (docs/adr/8071 §10): the L roof's eaves, ridges, hips, its valley and an arrow on
/// each face, then the building from the south-west and the north-east. `.run/gui-evidence/ic-donen-sacak-8071/`:
///
/// ```text
/// KENTOS_SNAPSHOT_BACKEND=wgpu RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib scene3d_building_tests::l_roof_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn l_roof_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/ic-donen-sacak-8071");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik"), ("dark", Size::new(1100.0, 700.0), "koyu")] {
        let mut app = open(with_an_l_roof());
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::scene3d_shown, &mut update);
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        app.viewport.camera.fit(&kentos_render_wgpu::Bounds { min_x: -2.0, min_y: -2.0, max_x: 14.0, max_y: 11.0 }, 24.0);
        shot(&mut app, "1-plan");
        let _ = app.update(Message::Run("scene3d.view"));
        send(&mut app, W::View(1));
        shot(&mut app, "2-3b-guneybati");
        send(&mut app, W::View(0));
        shot(&mut app, "3-3b-kuzeydogu");
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
            snapshot.settle(app, App::scene3d_shown, &mut update);
            snapshot.settle(app, App::scene3d_shown, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.scene3d_shown(), &app.theme()).save(&file).expect("writes the picture");
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
