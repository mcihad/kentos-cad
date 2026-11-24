//! Donatı araması (M24, docs/adr/8024): "lavabo" typed in Komut ara, Enter, a click by a wall: the washbasin taken in
//! alone, its back on the wall's face, its front toward the room. Three actions, ADV-12's first acceptance (before:
//! Donatı ▾, Kitaplık, the panel's search, a double click, H, a click).

use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

pub(super) fn fresh_mimari() -> App {
    kentos_postgis::connections::keep_in_memory();
    let (mut app, _) = App::start_in(
        Some(Module::Architecture),
        None,
        Settings::memory(),
        Recovery::off(),
    );
    app.db_store = Box::leak(Box::new(
        kentos_postgis::connections::ConnectionStore::open(None),
    ));
    app.hand_over = |m, _| Ok(m.program().to_owned());
    let task = app.update(Message::Run("file.new"));
    drive(&mut app, task);
    app
}

fn command(app: &mut App, name: &str, input: Value) -> Value {
    let doc = app.document.as_mut().expect("a drawing");
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        name,
        kentos_commands::CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

#[test]
fn three_balcony_edges_follow_walls_while_the_free_front_and_the_shaft_stay() {
    use kentos_commands::Entity;
    use kentos_domain::{Slot, Uuid};
    let mut app = fresh_mimari();
    let level = app.active_level().unwrap();
    let layer = app.document.as_ref().unwrap().model.layers().active().to_owned();
    let p = |x, y| json!({"x": x, "y": y});
    let part = |a, b, c, d| json!({"outer":{"pts":[p(a,b), p(c,b), p(c,d),p(a,d)]}});
    let wall = |a, b| json!({"pts":[a,b],"width":0.20,"justify":"center","arch":{"level":level}});
    let made = command(&mut app, "cad.walls.create", json!({"layerId":layer,"walls":[
        wall(p(0.0,0.0),p(4.0,0.0)), wall(p(4.0,0.0),p(4.0,4.0)), wall(p(0.0,4.0),p(0.0,0.0))
    ]}));
    let walls = made["output"]["uids"].as_array().unwrap();
    let made = command(&mut app, "cad.slabs.create", json!({"layerId":layer,"slabs":[{
        "role":"balcony", "parts":[part(0.0,0.0,4.0,4.0)], "arch":{"level":level},"edge":{"kind":"railing"}
    }]}));
    let uid = made["output"]["uids"][0].as_str().unwrap().to_owned();
    for (edge, wall) in [(0, &walls[0]), (1, &walls[1]), (3, &walls[2])] {
        command(&mut app, "cad.slabs.edit", json!({"uids":[uid],"operation":{
            "kind":"bindEdge", "edge":edge, "wall":wall, "segment":0, "face":0, "offset":0.05, "locked":edge == 0
        }}));
    }
    command(&mut app, "cad.slabs.edit", json!({"uids":[uid],"operation":{
        "kind":"subtract","parts":[part(1.0,1.0,1.5,1.5)]
    }}));
    let slot = |doc: &kentos_domain::Document, uid: &str| doc.slot_of(Uuid::parse_str(uid).unwrap()).unwrap();
    let slab = |doc: &kentos_domain::Document, s: Slot| match doc.get(s).unwrap() { Entity::Slab(s) => s.slab.clone(), _ => panic!() };
    let doc = &mut app.document.as_mut().unwrap().model;
    let s = slot(doc, &uid);
    let source = slot(doc, walls[0].as_str().unwrap());
    let before = slab(doc, s);
    assert_eq!(before.bound.as_ref().unwrap().edges.as_ref().unwrap().links.len(), 3);
    assert_eq!(before.parts[0].holes.len(), 1);
    let (head, objects) = doc.to_snapshot_v2_shared();
    let bytes = kentos_kcad::encode_verified_parts_watched(&head, &objects, &mut kentos_kcad::Quiet).unwrap();
    let decoded = kentos_domain::Document::from_snapshot_v2(kentos_kcad::decode(&bytes).unwrap()).unwrap();
    assert_eq!(slab(&decoded, slot(&decoded, &uid)), before);
    let Entity::Wall(mut w) = doc.get(source).unwrap().clone() else { panic!() };
    for p in &mut w.wall.pts { p.y += 0.30; }
    assert!(doc.update(source, Entity::Wall(w)));
    let moved = slab(doc, s);
    assert!((moved.parts[0].outer.pts[0].y - before.parts[0].outer.pts[0].y - 0.30).abs() < 1e-9);
    assert_eq!(moved.parts[0].outer.pts[2].y, before.parts[0].outer.pts[2].y);
    assert_eq!(moved.parts[0].holes, before.parts[0].holes);
    assert_eq!(doc.remove(&[source]), 1);
    let detached = slab(doc, s);
    assert_eq!(detached.parts, moved.parts);
    assert_eq!(detached.bound.as_ref().unwrap().broken, Some(kentos_commands::SlabBreak::EdgeSourceGone));
    doc.undo().unwrap();
    assert_eq!(slab(doc, s), moved);
    doc.undo().unwrap();
    assert_eq!(slab(doc, s), before);
}

#[test]
fn measured_fixture_front_uses_the_closest_face_across_its_width() {
    use kentos_interaction::Vec2 as V;
    let mut app = fresh_mimari();
    let layer = app.document.as_ref().unwrap().model.layers().active().to_owned();
    let level = app.active_level().unwrap();
    let p = |x, y| json!({"x": x,"y": y});
    command(&mut app,"cad.walls.create",json!({"layerId":layer,"walls":[{
        "pts":[p(0.0,3.1),p(4.0,3.1)],"width":0.2,"justify":"center","arch":{"level":level}
    }]}));
    let doc = &app.document.as_ref().unwrap().model;
    let wall = doc.entities().find(|e| matches!(e, kentos_commands::Entity::Wall(_))).unwrap();
    let candidates = || std::iter::once(kentos_domain::Slot(wall.base().id));
    let measured = kentos_native_application::fixture_front(doc, candidates(), [V::new(1.2,2.0), V::new(2.8,2.0)], V::new(0.0,1.0), Some(&level), 10.0).unwrap();
    assert!((measured.1 - 1.0).abs() < 1e-9);
    assert_eq!(measured.2, "duvar");
    assert!(kentos_native_application::fixture_front(doc, candidates(), [V::new(1.2,2.0), V::new(2.8,2.0)], V::new(0.0,1.0), None, 10.0).is_none());
}

#[test]
fn a_reflected_plan_shares_one_persisted_range_and_hides_lower_furniture() {
    use kentos_commands::{CeilingRange, Entity, CEILING_DEVICE_ATTR};
    let mut app = fresh_mimari();
    let level = app.active_level().unwrap();
    let layer = app.document.as_ref().unwrap().model.layers().active().to_owned();
    let p = |x, y| json!({"x":x,"y":y});
    let part = json!({"outer":{"pts":[p(0.0,0.0),p(4.0,0.0),p(4.0,4.0),p(0.0,4.0)]}});
    let room = command(&mut app, "cad.spaces.create", json!({"layerId":layer,"spaces":[{
        "name":"Tavan örneği", "parts":[part],"arch":{"level":level},"suspendedCeiling":2.45,
        "finishes":{"ceiling":"Akustik panel"}
    }]}));
    let uid = room["output"]["uids"][0].as_str().unwrap().to_owned();
    command(&mut app,"cad.spaces.finish",json!({"uids":[uid],"surface":"ceiling","grid":{"dx":0.6,"dy":0.6,"angle":0,"origin":"center"}}));
    command(&mut app,"cad.spaces.finish",json!({"uids":[uid],"surface":"floor"}));
    let schedule = command(&mut app, "cad.schedules.create", json!({
        "layerId": layer, "at": p(4.3, 2.0), "of": "spaces", "columns":[{"field":"ceilingFinish"}]
    }));
    let table_id = schedule["output"]["id"].as_u64().unwrap() as u32;
    let _ = app.update(Message::Run("tool.ceilingDevice"));
    assert_eq!(app.session.tool_id(), "ceilingDevice");
    let _ = app.submit_line("1.3,1.3");
    let _ = app.update(Message::Run("tool.cancel"));
    command(&mut app,"cad.views.edit",json!({"edit":{"kind":"save","name":"Tavan","center":p(2.0,2.0),"width":6.0,"height":6.0}}));
    command(&mut app,"cad.views.edit",json!({"edit":{"kind":"recipe","name":"Tavan","recipe":{"level":{"id":level},"ceiling":{"low":2.2,"high":2.6}}}}));
    let ids = |app: &App| {
        let doc = &app.document.as_ref().unwrap().model;
        let device = doc.entities().find_map(|e| match e {
            Entity::Insert(i) if i.base.attrs.contains_key(CEILING_DEVICE_ATTR) => Some(i), _ => None,
        }).expect("placed device");
        let floor = doc.entities().find(|e| e.base().attrs.get("Kaplama").is_some_and(|a| a == "Döşeme")).unwrap();
        let grid = doc.entities().find(|e| e.base().attrs.get("Kaplama").is_some_and(|a| a == "Tavan")).unwrap();
        let centre = device.p;
        assert!((centre.x-1.1).abs() < 1e-8 && (centre.y-1.1).abs() < 1e-8, "{centre:?}");
        (device.base.id, floor.base().id, grid.base().id)
    };
    let (device, floor, grid) = ids(&app);
    app.show_named_view("Tavan");
    let masks = app.spatial.level_looks().1;
    assert!(masks.hidden.contains(&floor) && !masks.hidden.contains(&device) && !masks.hidden.contains(&grid));
    let _ = app.update(Message::Run("view.previous"));
    assert!(!app.spatial.level_looks().1.hidden.contains(&floor));
    let _ = app.update(Message::Run("view.next"));
    assert!(app.spatial.level_looks().1.hidden.contains(&floor));
    let doc = &app.document.as_ref().unwrap().model;
    let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).unwrap();
    let saved = kentos_kcad::decode(&bytes).unwrap();
    assert_eq!(saved.settings.named_views[0].recipe.as_ref().unwrap().ceiling, Some(CeilingRange { low:2.2,high:2.6 }));
    command(&mut app,"cad.spaces.edit",json!({"uids":[uid],"operation":{"kind":"set","suspendedCeiling":2.85}}));
    let doc = &app.document.as_ref().unwrap().model;
    let hidden = kentos_native_application::ceiling_plan::hidden(doc, Some(&level), CeilingRange { low:2.2,high:2.6 });
    assert!(hidden.contains(&grid) && hidden.contains(&device));
    assert!(!hidden.contains(&table_id));
    let Some(Entity::Table(table)) = doc.get(kentos_domain::Slot(table_id)) else { panic!("the placed schedule") };
    assert!(table.cells.iter().flatten().any(|cell| cell == "Akustik panel"));
}

#[test]
fn the_other_programs_keep_duvara_hizala_off() {
    kentos_postgis::connections::keep_in_memory();
    let (app, _) = App::start_in(None, None, Settings::memory(), Recovery::off());
    assert!(!app.memory.block_to_wall);
}

#[test]
fn a_double_bed_keeps_its_size_and_head_anchor_in_metres_and_millimetres() {
    use kentos_commands::{DrawingUnit, Entity};
    use kentos_interaction::Vec2;

    for unit in [DrawingUnit::M, DrawingUnit::Mm] {
        let mut app = fresh_mimari();
        let doc = &mut app.document.as_mut().expect("a drawing").model;
        let mut settings = doc.settings().clone();
        settings.drawing_unit = Some(unit);
        doc.set_settings(settings);
        app.search_typed("çift kişilik yatak".into());
        assert_eq!(app.search_rows()[0].title, "Çift kişilik yatak");
        let task = app.search_run(0);
        drive(&mut app, task);
        let format = app.format();
        let settings = app.session.settings(&format);
        let size = settings.iter().find(|s| s.key == "size").expect("visible dimensions");
        if unit == DrawingUnit::Mm {
            assert_eq!(size.value, "1600 × 2000 mm");
        } else {
            assert_eq!(size.value, format!("{} × {} m", format.length_bare(1.6), format.length_bare(2.0)));
        }
        assert_eq!(app.with_tool(|tool, cx| tool.configure("anchor", "backCenter", cx)), Some(true));
        assert_eq!(app.with_tool(|tool, cx| tool.accept_point(Vec2::new(1.5, 0.0), cx)), Some(true));
        let doc = &app.document.as_ref().expect("a drawing").model;
        let bed = doc.entities().find_map(|e| match e {
            Entity::Insert(i) => Some(i),
            _ => None,
        }).expect("bed at the head centre");
        let footprint = doc.fixture_footprint(bed).expect("bed outline");
        let [a, _, c, _] = footprint;
        assert!((a.x - 0.7).abs() < 1e-9 && a.y.abs() < 1e-9, "{footprint:?}");
        assert!((c.x - 2.3).abs() < 1e-9 && (c.y - 2.0).abs() < 1e-9, "{footprint:?}");
    }
}

#[test]
fn door_front_approaches_follow_the_hinge_and_the_cabinet() {
    use kentos_commands::DoorApproachProfile;

    let mut app = fresh_mimari();
    let doc = &app.document.as_ref().unwrap().model;
    let layer = doc.layers().active().to_owned();
    let level = doc.settings().arch.as_ref().unwrap().levels[0].id.clone();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let wall = command(&mut app, "cad.walls.create", json!({
        "layerId": layer, "walls": [{"pts": [p(0.0, 0.0), p(6.0, 0.0)], "width": 0.2,
         "justify": "center", "arch": {"level": level}}],
    }));
    let door = command(&mut app, "cad.openings.create", json!({
        "layerId": layer, "openings": [{"form": "door", "host": wall["output"]["uids"][0],
         "at": 3.0, "width": 0.9, "side": "left", "hand": "start", "number": "K1"}],
    }));
    let uid = door["output"]["uids"][0].clone();
    command(&mut app, "cad.blocks.edit", json!({"operation": "import", "name": "kentos-temel"}));
    let block = app.document.as_ref().unwrap().model.block_named("Dolap").unwrap().id.to_text();
    let cabinet = command(&mut app, "cad.blocks.insert", json!({
        "layerId": layer, "block": block, "at": p(3.05, 0.30), "arch": {"level": level},
    }));
    let audit = |app: &mut App| command(app, "cad.arch.audit", json!({
        "checks": ["doors"], "doorApproach": DoorApproachProfile::FRONT,
    }))["output"]["findings"].as_array().unwrap().clone();
    let before = audit(&mut app);
    let pull = before.iter().find(|f| f["code"] == "arch_door_pull_blocked")
        .unwrap_or_else(|| panic!("{before:?}"));
    assert_eq!(pull["outline"].as_array().unwrap().len(), 4);
    assert!(pull["message"].as_str().unwrap().contains("Geçiş "));
    assert!(before.iter().all(|f| f["code"] != "arch_door_push_blocked"));
    command(&mut app, "cad.openings.edit", json!({
        "uids": [uid], "operation": {"kind": "flip", "what": "hand"},
    }));
    let after = audit(&mut app);
    let flipped = after.iter().find(|f| f["code"] == "arch_door_pull_blocked").unwrap();
    assert_ne!(pull["outline"][0]["x"], flipped["outline"][0]["x"]);
    command(&mut app, "cad.entities.edit", json!({
        "operation": "properties", "changes": [{"kind": "update", "uid": cabinet["output"]["uid"],
          "geometry": {"kind": "insert", "block": block, "p": p(3.05, 2.3), "scale": 1.0, "rotation": 0.0}}],
    }));
    let moved = audit(&mut app);
    assert!(moved.is_empty(), "{moved:?}");
}

#[test]
fn a_walk_uses_the_other_door_when_the_cabinet_blocks_the_direct_one() {
    let mut app = fresh_mimari();
    let doc = &app.document.as_ref().unwrap().model;
    let layer = doc.layers().active().to_owned();
    let level = doc.settings().arch.as_ref().unwrap().levels[0].id.clone();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let wall = command(&mut app, "cad.walls.create", json!({
        "layerId": layer, "walls": [{"pts": [p(4.0, 0.0), p(4.0, 4.0)], "width": 0.2,
         "justify": "center", "arch": {"level": level}}],
    }));
    let door = |at, number| json!({"host": wall["output"]["uids"][0],
        "at": at, "form": "door", "width": 1.1, "number": number});
    let doors = command(&mut app, "cad.openings.create", json!({
        "layerId": layer, "openings": [door(0.9, "K1"), door(3.1, "K2")],
    }));
    let room = |x, y, name| json!({"name": name, "arch": {"level": level}, "parts": [
        {"outer":{"pts": [p(x, 0.1), p(y, 0.1), p(y, 3.9), p(x, 3.9)]}}
    ]});
    command(&mut app, "cad.spaces.create", json!({
        "layerId": layer, "spaces": [room(0.1, 3.9, "Antre"), room(4.1, 7.9, "Banyo")],
    }));
    let route = |app: &mut App, shell_only| command(app, "cad.arch.walk", json!({
        "start": p(1.0, 0.9), "end": p(7.0, 0.9), "width": 0.8, "level": level,
        "shellOnly": shell_only,
    }))["output"].clone();
    let direct = route(&mut app, false);
    assert_eq!(direct["length"], 6.0, "{direct:?}");
    command(&mut app, "cad.blocks.edit", json!({"operation": "import", "name": "kentos-temel"}));
    let block = app.document.as_ref().unwrap().model.block_named("Dolap").unwrap().id.to_text();
    command(&mut app, "cad.blocks.insert", json!({
        "layerId": layer, "block": block, "at": p(2.69, 0.40), "arch": {"level": level},
    }));
    let round = route(&mut app, false);
    assert!(round["length"].as_f64().is_some_and(|l| l > 6.0), "{round:?}");
    let points = round["points"].as_array().unwrap();
    let y = points.windows(2).find_map(|ab| {
        let (a, b) = (&ab[0], &ab[1]);
        let (x0, x1) = (a["x"].as_f64()?, b["x"].as_f64()?);
        if x0 < 4.0 && x1 >= 4.0 {
            let (y0, y1) = (a["y"].as_f64()?, b["y"].as_f64()?);
            Some(y0 + (y1 - y0) * (4.0 - x0) / (x1 - x0))
        } else {
            None
        }
    }).unwrap();
    assert!(y > 2.5, "{round:?}");
    assert_eq!(route(&mut app, true), direct);
    command(&mut app, "cad.entities.delete", json!({"uids": [doors["output"]["uids"][1]]}));
    let blocked = route(&mut app, false);
    assert!(blocked["points"].as_array().unwrap().is_empty(), "{blocked:?}");
    assert!(blocked["blocked"].is_object());
}

/// Mimari denetim's Donatılar (M25, docs/adr/8025) on a 2.0 × 1.6 m bathroom: a wall-hung WC (its block's “Boşluk
/// zarfı” 80 × 60 in front), a washbasin of the older blocks (KentOS's 80 × 70 in front of it) whose zone takes the WC,
/// and a door in the west wall whose leaf opens into the WC's zone, its fix the door turned.
#[test]
fn a_bathroom_s_fixture_zones_are_found_taken_by_a_fixture_and_a_door() {
    let mut app = fresh_mimari();
    let (layer, level) = {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let level = doc
            .settings()
            .arch
            .as_ref()
            .and_then(|a| a.levels.first())
            .map(|l| l.id.clone())
            .expect("a storey");
        (doc.layers().active().to_owned(), level)
    };
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    // Walls 10 cm thick round the room's clear faces x 0..2.0, y 0..1.6.
    let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.10, "justify": "center", "arch": {"level": level}});
    let walls = command(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((-0.05, -0.05), (2.05, -0.05)), wall((2.05, -0.05), (2.05, 1.65)),
            wall((2.05, 1.65), (-0.05, 1.65)), wall((-0.05, 1.65), (-0.05, -0.05))
        ]}),
    );
    let west = walls["output"]["uids"][3]
        .as_str()
        .expect("the west wall")
        .to_owned();
    command(
        &mut app,
        "cad.blocks.edit",
        json!({"operation": "import", "name": "kentos-temel"}),
    );
    let block = |app: &App, name: &str| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.block_named(name)
            .map(|b| b.id.to_text())
            .expect("the library's block")
    };
    let (wc, basin) = (block(&app, "Asma klozet"), block(&app, "Lavabo"));
    // The WC on the south wall facing north; the washbasin on the east wall facing west, its zone over the WC.
    command(
        &mut app,
        "cad.blocks.insert",
        json!({"block": wc, "at": p(1.0, 0.0), "rotation": 0.0, "layerId": layer, "arch": {"level": level}}),
    );
    command(
        &mut app,
        "cad.blocks.insert",
        json!({"block": basin, "at": p(2.0, 0.4), "rotation": std::f64::consts::FRAC_PI_2, "layerId": layer, "arch": {"level": level}}),
    );
    command(
        &mut app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [{"host": west, "at": 0.75, "form": "door", "width": 0.8, "number": "K1"}]}),
    );
    // The WC's body without its hidden “Boşluk zarfı”: 54 cm from the wall, not 114; the zone 60 cm in front of it.
    {
        let doc = &app.document.as_ref().expect("a drawing").model;
        let wc_insert = doc
            .entities()
            .find_map(|e| match e {
                kentos_commands::Entity::Insert(i)
                    if doc.block(i.block).is_some_and(|b| b.name == "Asma klozet") =>
                {
                    Some(i.clone())
                }
                _ => None,
            })
            .expect("the WC");
        let depth = |c: [kentos_commands::Vec2; 4]| c.iter().map(|p| p.y).fold(f64::MIN, f64::max);
        let body = doc.fixture_footprint(&wc_insert).expect("its body");
        assert!((depth(body) - 0.54).abs() < 1e-9, "{body:?}");
        let zone = doc.fixture_zone(&wc_insert).expect("its zone");
        assert!((depth(zone) - 1.14).abs() < 1e-9, "{zone:?}");
    }
    // `KENTOS_M25_KCAD=path` keeps the bathroom for the real window.
    if let Ok(path) = std::env::var("KENTOS_M25_KCAD") {
        let doc = &app.document.as_ref().expect("a drawing").model;
        std::fs::write(
            path,
            kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes"),
        )
        .expect("kept");
    }
    let audit = command(&mut app, "cad.arch.audit", json!({"checks": ["fixtures"]}));
    let findings = audit["output"]["findings"]
        .as_array()
        .expect("findings")
        .clone();
    let codes: Vec<&str> = findings
        .iter()
        .map(|f| f["code"].as_str().unwrap_or_default())
        .collect();
    let messages: Vec<&str> = findings
        .iter()
        .map(|f| f["message"].as_str().unwrap_or_default())
        .collect();
    assert!(codes.contains(&"arch_fixture_zone_blocked"), "{messages:?}");
    assert!(
        messages.iter().any(|m| m.contains("“Lavabo”")
            && m.contains("0.80 × 0.70 m")
            && m.contains("“Asma klozet”")),
        "{messages:?}"
    );
    let door = findings
        .iter()
        .find(|f| f["code"] == "arch_fixture_zone_door")
        .unwrap_or_else(|| panic!("the door's leaf in the WC's zone: {messages:?}"));
    assert!(
        door["message"]
            .as_str()
            .unwrap_or_default()
            .contains("K1 kapısının kanadı “Asma klozet”"),
        "{door}"
    );
    assert_eq!(door["nature"], "passage");
    assert_eq!(
        door["fix"]["step"]["command"], "cad.openings.edit",
        "the door turned: {door}"
    );
    // Asked with the other groups but not by name: no fixture findings (an older call's answer stays).
    let all = command(&mut app, "cad.arch.audit", json!({}));
    assert!(
        !all["output"]["findings"]
            .as_array()
            .expect("findings")
            .iter()
            .any(|f| f["check"] == "fixtures"),
        "only when asked by name"
    );
}
