//! Donatı araması (M24, docs/adr/3024): "lavabo" typed in Komut ara, Enter, a click by a wall: the washbasin taken in
//! alone, its back on the wall's face, its front toward the room. Three actions, ADV-12's first acceptance (before:
//! Donatı ▾, Kitaplık, the panel's search, a double click, H, a click).

use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

fn fresh_mimari() -> App {
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

fn screen(app: &App, x: f64, y: f64) -> iced::Point {
    let [sx, sy] = app
        .viewport
        .camera
        .world_to_screen(kentos_interaction::Vec2::new(x, y));
    iced::Point::new(sx as f32, sy as f32)
}

#[test]
fn lavabo_enter_and_a_click_by_a_wall_place_the_washbasin_against_it() {
    use crate::viewport::Event;
    let mut app = fresh_mimari();
    let _ = app.update(Message::WindowResized(iced::Size::new(1440.0, 900.0)));
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
    // A wall running north at x = 0, 25 cm thick: its east face at x = 0.125.
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    command(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [{"pts": [p(0.0, 0.0), p(0.0, 5.0)], "width": 0.25, "justify": "center", "arch": {"level": level}}]}),
    );
    let blocks = |app: &App| {
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .blocks()
            .len()
    };
    let before = blocks(&app);
    // 1: the word typed in Komut ara.
    app.search_typed("lavabo".into());
    let rows = app.search_rows();
    assert_eq!(rows[0].title, "Lavabo", "the washbasin first");
    assert_eq!(rows[0].detail, "Donatı › Islak hacim › Lavabo");
    assert!(rows.iter().any(|r| r.title == "Dolaplı lavabo"));
    // 2: Enter.
    let task = app.search_run(0);
    drive(&mut app, task);
    assert_eq!(app.session.tool_id(), "blockInsert");
    assert_eq!(
        blocks(&app),
        before + 1,
        "the washbasin alone, not the library's 38"
    );
    assert!(
        app.memory.block_to_wall,
        "Duvara hizala on in KentOS Mimari"
    );
    // 3: a click 12.5 cm off the wall's face, in the room east of it.
    app.viewport.camera.set_size(1100.0, 600.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: -2.0,
            min_y: -1.0,
            max_x: 3.0,
            max_y: 6.0,
        },
        24.0,
    );
    let at = screen(&app, 0.25, 2.3);
    let _ = app.update(Message::Viewport(Event::Moved(at)));
    let _ = app.update(Message::Viewport(Event::Pressed(at)));
    let _ = app.update(Message::Viewport(Event::Released(at)));
    let doc = &app.document.as_ref().expect("a drawing").model;
    let insert = doc
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Insert(i) => Some(i.clone()),
            _ => None,
        })
        .expect("the washbasin placed");
    assert!(
        (insert.p.x - 0.125).abs() < 1e-9,
        "its back on the face: {:?}",
        insert.p
    );
    assert!(
        (insert.p.y - 2.3).abs() < 0.02,
        "where it was clicked: {:?}",
        insert.p
    );
    // Its front (+y) toward the room, east.
    let (fx, fy) = (-insert.rotation.sin(), insert.rotation.cos());
    assert!(
        (fx - 1.0).abs() < 1e-9 && fy.abs() < 1e-9,
        "turned {}",
        insert.rotation
    );
    // On the tefriş layer of sanitary ware, opened with it (docs/adr/3008's key): one undo takes both.
    let layer = |app: &App, id: &str| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.layers().get(id).map(|l| l.name.clone())
    };
    let id = insert.base.layer_id.clone();
    assert_eq!(layer(&app, &id).as_deref(), Some("MM-TEFR-VITRIFIYE"));
    let _ = app.update(Message::Run("edit.undo"));
    let doc = &app.document.as_ref().expect("a drawing").model;
    assert!(
        !doc.entities().any(|e| e.kind() == "insert"),
        "the washbasin undone"
    );
    assert_eq!(layer(&app, &id), None, "and its layer");
}

#[test]
fn the_other_programs_keep_duvara_hizala_off() {
    kentos_postgis::connections::keep_in_memory();
    let (app, _) = App::start_in(None, None, Settings::memory(), Recovery::off());
    assert!(!app.memory.block_to_wall);
}

/// Mimari denetim's Donatılar (M25, docs/adr/3025) on a 2.0 × 1.6 m bathroom: a wall-hung WC (its block's “Boşluk
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
