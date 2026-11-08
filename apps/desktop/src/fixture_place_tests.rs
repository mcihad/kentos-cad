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
