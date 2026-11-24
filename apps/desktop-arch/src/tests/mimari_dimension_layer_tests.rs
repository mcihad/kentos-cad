//! KentOS Mimari's wall distance written as a dimension (docs/adr/0710, 8008): the + beside a distance writes a
//! permanent dimension on Mimari's dimension layer (the layer key its program starts with), following the walls. Moved
//! from the core's building_context_tests.rs with the module's package (docs/adr/9199 §4.11).

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::{Value, json};
use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

/// A new KentOS Mimari project: a 5 × 4 m room of Dış duvar 25 on its first storey, a door K1 in the first wall and
/// the room 101 Salon; the first wall's, the door's and the room's slots.
fn building() -> (App, [Slot; 3]) {
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
    let doc = &app.document.as_ref().expect("a drawing").model;
    let layer = doc.layers().active().to_owned();
    let level = doc
        .settings()
        .arch
        .as_ref()
        .and_then(|a| a.levels.first())
        .map(|l| l.id.clone())
        .expect("a storey");
    let style = kentos_interaction::wall::offered(doc.settings())
        .into_iter()
        .find(|s| s.name == "Dış duvar 25")
        .expect("KentOS's wall styles")
        .id;
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let wall = |a: (f64, f64), b: (f64, f64)| json!({"pts": [p(a.0, a.1), p(b.0, b.1)], "width": 0.25, "style": style, "arch": {"level": level}});
    let walls = command(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [
            wall((0.0, 0.0), (5.0, 0.0)), wall((5.0, 0.0), (5.0, 4.0)),
            wall((5.0, 4.0), (0.0, 4.0)), wall((0.0, 4.0), (0.0, 0.0))
        ]}),
    );
    let w = uids(&walls);
    let door = command(
        &mut app,
        "cad.openings.create",
        json!({"layerId": layer, "openings": [
            {"host": w[0], "at": 1.5, "form": "door", "width": 0.9, "number": "K1"}
        ]}),
    );
    let room = command(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [
            {"seed": p(2.5, 2.0), "arch": {"level": level}, "name": "Salon", "number": "101"}
        ]}),
    );
    let doc = &app.document.as_ref().expect("a drawing").model;
    let slot = |uid: &str| {
        doc.slot_of(uid.parse().expect("an id"))
            .expect("the object")
    };
    let slots = [slot(&w[0]), slot(&uids(&door)[0]), slot(&uids(&room)[0])];
    (app, slots)
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

fn uids(answer: &Value) -> Vec<String> {
    answer["output"]["uids"]
        .as_array()
        .expect("ids")
        .iter()
        .map(|u| u.as_str().expect("an id").to_owned())
        .collect()
}

/// Two parallel walls of Dış duvar 25, 4 m apart axis to axis, in a new KentOS Mimari project: 3.75 m clear between
/// them. Their slots and ids.
fn two_walls() -> (App, [Slot; 2], Vec<String>) {
    let (mut app, _) = building();
    let doc = &app.document.as_ref().expect("a drawing").model;
    let layer = doc.layers().active().to_owned();
    let level = doc.settings().arch.as_ref().unwrap().levels[0].id.clone();
    let style = kentos_interaction::wall::offered(doc.settings())
        .into_iter()
        .find(|s| s.name == "Dış duvar 25")
        .expect("KentOS's wall styles")
        .id;
    let wall = |x: f64| {
        json!({"pts": [{"x": 20.0 + x, "y": 20.0}, {"x": 20.0 + x, "y": 25.0}], "width": 0.25, "style": style,
               "arch": {"level": level}})
    };
    let made = command(
        &mut app,
        "cad.walls.create",
        json!({"layerId": layer, "walls": [wall(0.0), wall(4.0)]}),
    );
    let ids = uids(&made);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let slots = [
        doc.slot_of(ids[0].parse().unwrap()).unwrap(),
        doc.slot_of(ids[1].parse().unwrap()).unwrap(),
    ];
    (app, slots, ids)
}

fn screen(app: &App, x: f64, y: f64) -> iced::Point {
    let [sx, sy] = app
        .viewport
        .camera
        .world_to_screen(kentos_interaction::Vec2::new(x, y));
    iced::Point::new(sx as f32, sy as f32)
}

fn dimensions(app: &App) -> Vec<(f64, String)> {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Dimension(d) => Some((
                (d.b.x - d.a.x).hypot(d.b.y - d.a.y),
                d.base.layer_id.clone(),
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn the_plus_beside_a_wall_s_distance_writes_a_permanent_dimension_that_follows_the_walls() {
    use crate::viewport::Event;
    let (mut app, [a, _], ids) = two_walls();
    let _ = app.update(Message::WindowResized(iced::Size::new(1440.0, 900.0)));
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: 17.0,
            min_y: 18.0,
            max_x: 27.0,
            max_y: 27.0,
        },
        24.0,
    );
    app.selection.set([a]);
    // The pointer by the wall's right face: its distances drawn, the + across from each padlock.
    let _ = app.update(Message::Viewport(Event::Moved(screen(&app, 20.4, 22.5))));
    let plus = app.session.temporary_permanents();
    assert!(!plus.is_empty(), "a + beside the clear distance");
    assert_eq!(dimensions(&app).len(), 0);
    let p = screen(&app, plus[0].x, plus[0].y);
    let _ = app.update(Message::Viewport(Event::Moved(p)));
    let _ = app.update(Message::Viewport(Event::Pressed(p)));
    let _ = app.update(Message::Viewport(Event::Released(p)));
    let written = dimensions(&app);
    assert_eq!(written.len(), 1, "one dimension, not a chain");
    assert!(
        (written[0].0 - 3.75).abs() < 1e-6,
        "the clear distance: {written:?}"
    );
    assert_eq!(
        written[0].1, "mm-xxxx-olcu",
        "KentOS Mimari's dimension layer"
    );
    assert!(crate::files_testing::last_said(&app).contains("Kalıcı ölçü yazıldı"));
    // The second wall moved half a metre away: the dimension, bound to both faces, says 4.25.
    command(
        &mut app,
        "cad.entities.transform",
        json!({"uids": [ids[1]], "transform": {"kind": "move", "dx": 0.5, "dy": 0.0}}),
    );
    let moved = dimensions(&app);
    assert!(
        (moved[0].0 - 4.25).abs() < 1e-6,
        "it follows the walls: {moved:?}"
    );
}
