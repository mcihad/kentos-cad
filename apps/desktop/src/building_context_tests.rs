//! A building object's context (M17, docs/adr/8017): the Duvar tab holds Duvarı kaydır, the selection's tab's own
//! panel opens the right click under the tab's name, and the card says what an architect asks of the object (a
//! wall its style, thickness and storey, not its footprint's area; a door its width and number under Kapı; a room
//! its number, name and storey).

use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::files_testing::drive;
use crate::recovery::Recovery;
use crate::settings::Settings;

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

/// A new KentOS Mimari project: a 5 × 4 m room of Dış duvar 25 on its first storey, a door K1 in the first wall and
/// the room 101 Salon; the first wall's, the door's and the room's slots.
pub(crate) fn building() -> (App, [Slot; 3]) {
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

#[test]
fn the_selection_s_building_panel_opens_the_right_click_under_its_tab_s_name() {
    let (mut app, [wall, door, room]) = building();
    app.selection.set([wall]);
    assert_eq!(app.selection_tab().map(|t| t.id), Some("ctx.wall"));
    let (name, tools) = app.building_tools().expect("a wall's tools");
    assert_eq!(name, "Duvar");
    assert_eq!(tools[0], "tool.wallShift", "Duvarı kaydır first");
    for id in [
        "tool.wallSplit",
        "tool.wallExtend",
        "tool.wallTrim",
        "view.properties",
    ] {
        assert!(tools.contains(&id), "{id}");
    }
    app.selection.set([door]);
    let (name, tools) = app.building_tools().expect("a door's tools");
    assert_eq!(name, "Açıklık");
    for id in ["tool.openingMove", "tool.openingFlip", "view.properties"] {
        assert!(tools.contains(&id), "{id}");
    }
    app.selection.set([room]);
    let (name, tools) = app.building_tools().expect("a room's tools");
    assert_eq!(name, "Mahal");
    for id in ["tool.spaceSplit", "tool.spaceNumber", "view.properties"] {
        assert!(tools.contains(&id), "{id}");
    }
    assert_eq!(tools[0], "tool.spaceSplit", "the room's own tools lead");
    assert_eq!(
        tools.last(),
        Some(&"view.properties"),
        "Nesne özellikleri closes the section"
    );
    assert!(
        !tools.contains(&"tool.scheduleSpaces"),
        "the schedule is Belgele's, not the room's own"
    );
    app.selection.set([wall, door]);
    assert!(
        app.building_tools().is_none(),
        "a mixed selection opens Seçim: the edits every object has"
    );
}

#[test]
fn the_duvar_tab_holds_duvari_kaydir_its_style_and_its_bonds() {
    let tab = crate::catalog::catalog()
        .contextual_in(kentos_commands::Workspace::Cad)
        .find(|t| t.id == "ctx.wall")
        .expect("Duvar");
    let commands: Vec<&str> = tab.panels.iter().flat_map(|p| p.commands()).collect();
    for id in [
        "tool.wallShift",
        "tool.wallRestyle",
        "arch.wallStyles",
        "arch.materials",
        "constraint.show",
        "constraint.relaxNext",
    ] {
        assert!(commands.contains(&id), "{id}");
    }
}

#[test]
fn the_card_says_what_an_architect_asks_of_a_wall_a_door_and_a_room() {
    let (app, [wall, door, room]) = building();
    let doc = &app.document.as_ref().expect("a drawing").model;
    let format = kentos_interaction::Format::of(doc.settings());
    let rows = |slot: Slot| {
        let card =
            crate::hover_card::card(doc.get(slot).expect("the object"), doc, &format, |_| {
                iced::Color::BLACK
            });
        let rows: Vec<(&str, String)> = card.rows.iter().map(|(k, v, _)| (*k, v.clone())).collect();
        (card.title, rows)
    };
    let storey = doc.settings().arch.as_ref().unwrap().levels[0].name.clone();
    let (title, wall_rows) = rows(wall);
    assert_eq!(title, "Duvar");
    assert_eq!(wall_rows[0], ("Stil", "Dış duvar 25".to_owned()));
    assert_eq!(wall_rows[1], ("Kalınlık", format.length(0.25)));
    assert_eq!(wall_rows[2], ("Kat", storey.clone()));
    assert!(wall_rows.iter().any(|(k, _)| *k == "Uzunluk"));
    assert!(
        !wall_rows.iter().any(|(k, _)| *k == "Alan"),
        "a wall's footprint area is no one's question: {wall_rows:?}"
    );
    let (title, door_rows) = rows(door);
    assert_eq!(title, "Kapı", "named by its form");
    assert!(door_rows.contains(&("Genişlik", format.length(0.9))));
    assert!(door_rows.contains(&("Numara", "K1".to_owned())));
    let (_, room_rows) = rows(room);
    assert_eq!(room_rows[0], ("Numara", "101".to_owned()));
    assert_eq!(room_rows[1], ("Ad", "Salon".to_owned()));
    assert_eq!(room_rows[2], ("Kat", storey));
    assert!(room_rows.iter().any(|(k, _)| *k == "Alan"), "{room_rows:?}");
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

#[test]
fn the_snap_marker_says_a_wall_s_face_and_axis_in_a_building_s_words() {
    use kentos_geometry_core::entity::Shape;
    use kentos_interaction::{SnapHit, SnapKind, Vec2};
    let (mut app, [a, _], _) = two_walls();
    let doc = app.document.take().expect("a drawing");
    app.spatial.sync(&doc.model);
    app.document = Some(doc);
    let id = f64::from(a.0);
    let Some(Shape::Wall { wall: spec, .. }) = app.spatial.store().get(id).map(|i| i.shape.clone())
    else {
        panic!("the wall in the store");
    };
    // The path runs north at x = 20: its left is west.
    let left = 20.0 - kentos_geometry_core::geom::wall::left_face(&spec);
    let right = left + spec.width;
    let axis = (left + right) / 2.0;
    let note = |app: &mut App, x: f64, kind: SnapKind| {
        app.snap = Some(SnapHit {
            kind,
            point: Vec2::new(x, 22.5),
            id,
        });
        app.snap_marker_note()
    };
    assert_eq!(
        note(&mut app, right, SnapKind::Midpoint).as_deref(),
        Some("Duvar yüzü")
    );
    assert_eq!(
        note(&mut app, left, SnapKind::Midpoint).as_deref(),
        Some("Duvar yüzü")
    );
    assert_eq!(
        note(&mut app, axis, SnapKind::Nearest).as_deref(),
        Some("Duvar ekseni")
    );
}

/// A new KentOS Mimari project, nothing drawn.
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

fn counts(app: &App) -> (usize, usize, usize) {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let n = |kind: &str| doc.entities().filter(|e| e.kind() == kind).count();
    (n("wall"), n("space"), n("tag"))
}

#[test]
fn oda_writes_the_walls_the_room_and_its_tag_in_one_undo_step() {
    let mut app = fresh_mimari();
    let _ = app.update(Message::Run("tool.room"));
    assert_eq!(app.session.tool_id(), "room");
    let keys: Vec<&str> = app.field_settings().iter().map(|s| s.key).collect();
    assert_eq!(
        keys.first(),
        Some(&"B"),
        "Boyutlar first beside the cursor: {keys:?}"
    );
    assert!(
        keys.contains(&"A"),
        "the room's name beside the cursor: {keys:?}"
    );
    app.with_tool(|s, cx| s.configure("A", "Salon", cx));
    app.with_tool(|s, cx| s.configure("B", "4,3", cx));
    let _ = app.submit_line("10,10");
    let _ = app.submit_line("12,12");
    assert_eq!(counts(&app), (1, 1, 1), "the walls, the room and its tag");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let room = doc
        .entities()
        .find(|e| e.kind() == "space")
        .expect("the room");
    let kentos_commands::Entity::Space(sp) = room else {
        unreachable!()
    };
    assert_eq!(sp.space.name.as_deref(), Some("Salon"));
    match doc.arch_value(room, "area", None) {
        Some(kentos_domain::arch_values::ArchValue::Area(a)) => {
            assert!((a - 12.0).abs() < 1e-6, "4 × 3 clear: {a}")
        }
        other => panic!("an area: {other:?}"),
    }
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(counts(&app), (0, 0, 0), "one undo takes all three");
}

#[test]
fn oda_s_name_beside_the_cursor_takes_the_list_or_any_word() {
    let mut app = fresh_mimari();
    let _ = app.update(Message::Run("tool.room"));
    let name = app
        .field_settings()
        .into_iter()
        .find(|s| s.key == "A")
        .expect("the room's name");
    assert!(name.open, "the list offers, any word is taken");
    let _ = app.with_tool(|s, cx| s.configure("A", "Kiler", cx));
    let shown = app.session.settings(&app.format());
    let row = shown.iter().find(|r| r.key == "A").expect("Mahal adı");
    assert_eq!(row.value, "", "a word of no list is not a checked choice");
    assert!(
        app.session.prompt().text().contains("Mahal adı (A): Kiler"),
        "{}",
        app.session.prompt().text()
    );
}

#[test]
fn duvar_shows_the_loop_s_area_and_kapat_writes_its_room_when_asked() {
    use crate::viewport::Event;
    let mut app = fresh_mimari();
    let _ = app.update(Message::WindowResized(iced::Size::new(1440.0, 900.0)));
    let _ = app.update(Message::Run("tool.wall"));
    // The drawing area's size first: without it the camera's scale is unsettled and the snap's reach covers the room.
    app.viewport.camera.set_size(1100.0, 600.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: -10.0,
            min_y: -10.0,
            max_x: 60.0,
            max_y: 50.0,
        },
        24.0,
    );
    let _ = app.with_tool(|s, cx| s.configure("M", "yes", cx));
    assert!(app.memory.wall_room, "Kapanınca mahal on");
    // Large enough that its corners stand apart on the screen the test's camera gives (no snap between them).
    for p in ["0,0", "50,0", "50,40"] {
        let _ = app.submit_line(p);
    }
    // The pointer off the first corner's lines (tracking would take it there): (2, 38), the loop 1910 m².
    let _ = app.update(Message::Viewport(Event::Moved(screen(&app, 2.0, 38.0))));
    let format = app.format();
    let lines = app
        .session
        .preview(&format)
        .and_then(|p| p.tag)
        .map(|t| t.lines)
        .unwrap_or_default();
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("Döngü alanı") && l.contains("1910")),
        "{lines:?}"
    );
    let _ = app.submit_line("0,40");
    let _ = app.submit_line("K");
    assert_eq!(counts(&app), (1, 1, 0), "the closed wall and its room");
    assert!(crate::files_testing::last_said(&app).contains("mahali eklendi"));
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(counts(&app), (0, 0, 0), "one undo takes both");
}

fn escape(app: &mut App) {
    use iced::keyboard::key::{Code, Named, Physical};
    let _ = app.update(Message::Key(crate::keys::KeyPress {
        key: iced::keyboard::Key::Named(Named::Escape),
        physical: Physical::Code(Code::Escape),
        modifiers: iced::keyboard::Modifiers::default(),
        text: None,
        repeat: false,
    }));
}

fn room_area(app: &App) -> f64 {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let room = doc
        .entities()
        .find(|e| e.kind() == "space")
        .expect("the room");
    match doc.arch_value(room, "area", None) {
        Some(kentos_domain::arch_values::ArchValue::Area(a)) => a,
        other => panic!("an area: {other:?}"),
    }
}

/// Odayı büyüt (M23, docs/adr/8023): Oda's room is picked on its floor; each of its four edges has a grip. The right
/// one dragged a metre out moves the wall behind it and the room grows 4 × 3 → 5 × 3 in one undo step; the top one
/// given 3.5 makes it 3,50 across, and the right one given −0.5 takes half a metre off.
#[test]
fn a_room_s_edge_grip_moves_its_wall_and_the_room_grows() {
    use crate::viewport::Event;
    let mut app = fresh_mimari();
    let _ = app.update(Message::WindowResized(iced::Size::new(1440.0, 900.0)));
    let _ = app.update(Message::Run("tool.room"));
    app.with_tool(|s, cx| s.configure("A", "Salon", cx));
    app.with_tool(|s, cx| s.configure("B", "4,3", cx));
    let _ = app.submit_line("10,10");
    let _ = app.submit_line("12,12");
    assert_eq!(counts(&app), (1, 1, 1), "the walls, the room and its tag");
    assert!((room_area(&app) - 12.0).abs() < 1e-6);
    escape(&mut app);
    escape(&mut app);
    app.viewport.camera.set_size(1100.0, 600.0);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: 8.0,
            min_y: 8.0,
            max_x: 17.0,
            max_y: 15.0,
        },
        24.0,
    );
    let click = |app: &mut App, at: iced::Point| {
        let _ = app.update(Message::Viewport(Event::Moved(at)));
        let _ = app.update(Message::Viewport(Event::Pressed(at)));
        let _ = app.update(Message::Viewport(Event::Released(at)));
    };
    // Its floor, away from its tag in its middle: the room.
    let at = screen(&app, 10.5, 12.6);
    click(&mut app, at);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let room = doc
        .entities()
        .find(|e| e.kind() == "space")
        .map(|e| Slot(e.base().id))
        .expect("the room");
    assert_eq!(
        app.selection.ids(),
        [room],
        "a click on its floor picks the room"
    );
    let grips = |app: &App| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        kentos_interaction::room_grip::grips(doc, &app.spatial, app.selection.ids())
    };
    let found = grips(&app);
    assert_eq!(found.len(), 4, "a grip on each edge: {found:?}");
    let right = found
        .iter()
        .max_by(|a, b| a.at.x.total_cmp(&b.at.x))
        .cloned()
        .expect("the right edge's");
    assert!((right.outward.x - 1.0).abs() < 1e-9, "{right:?}");
    // Dragged a metre out: the card says the inner size and the area, the release writes it.
    let from = screen(&app, right.at.x, right.at.y);
    let to = screen(&app, right.at.x + 1.0, right.at.y);
    let _ = app.update(Message::Viewport(Event::Moved(from)));
    let _ = app.update(Message::Viewport(Event::Pressed(from)));
    let _ = app.update(Message::Viewport(Event::Moved(to)));
    let format = app.format();
    let lines = app
        .session
        .preview(&format)
        .and_then(|p| p.tag)
        .map(|t| t.lines)
        .unwrap_or_default();
    assert!(
        lines.first().is_some_and(|l| l.starts_with("İç ölçü")),
        "{lines:?}"
    );
    assert!(lines.iter().any(|l| l.contains("Salon")), "{lines:?}");
    let _ = app.update(Message::Viewport(Event::Released(to)));
    assert!(
        (room_area(&app) - 15.0).abs() < 1e-6,
        "5 × 3: {}",
        room_area(&app)
    );
    assert!(crate::files_testing::last_said(&app).contains("Oda büyütüldü"));
    let _ = app.update(Message::Run("edit.undo"));
    assert!(
        (room_area(&app) - 12.0).abs() < 1e-6,
        "one undo: {}",
        room_area(&app)
    );
    // The top one clicked, then 3.5 typed: 3,50 across.
    app.selection.set([room]);
    let top = grips(&app)
        .into_iter()
        .max_by(|a, b| a.at.y.total_cmp(&b.at.y))
        .expect("the top edge's");
    let at = screen(&app, top.at.x, top.at.y);
    click(&mut app, at);
    let _ = app.submit_line("3.5");
    assert!(
        (room_area(&app) - 14.0).abs() < 1e-6,
        "4 × 3,5: {}",
        room_area(&app)
    );
    // The right one given −0.5: half a metre off.
    app.selection.set([room]);
    let right = grips(&app)
        .into_iter()
        .max_by(|a, b| a.at.x.total_cmp(&b.at.x))
        .expect("the right edge's");
    let at = screen(&app, right.at.x, right.at.y);
    click(&mut app, at);
    let _ = app.submit_line("-0.5");
    assert!(
        (room_area(&app) - 12.25).abs() < 1e-6,
        "3,5 × 3,5: {}",
        room_area(&app)
    );
    assert!(crate::files_testing::last_said(&app).contains("Oda küçültüldü"));
}

/// A room's edge has a grip only where a wall it follows stands (docs/adr/8023): none on a locked wall's layer, none on
/// a room whose boundary was drawn (it does not follow its walls).
#[test]
fn a_room_s_edge_grips_need_an_unlocked_wall_and_a_room_that_follows_it() {
    let mut app = fresh_mimari();
    let _ = app.update(Message::Run("tool.room"));
    app.with_tool(|s, cx| s.configure("B", "4,3", cx));
    let _ = app.submit_line("10,10");
    let _ = app.submit_line("12,12");
    escape(&mut app);
    escape(&mut app);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let room = doc
        .entities()
        .find(|e| e.kind() == "space")
        .expect("the room");
    let (slot, level, parts) = match room {
        kentos_commands::Entity::Space(s) => (
            Slot(s.base.id),
            s.space.arch.level.clone(),
            serde_json::to_value(&s.space.parts).expect("its parts"),
        ),
        _ => unreachable!(),
    };
    let walls = doc
        .entities()
        .find(|e| e.kind() == "wall")
        .map(|e| e.base().layer_id.clone())
        .expect("the walls' layer");
    let rooms = doc
        .entities()
        .find(|e| e.kind() == "space")
        .map(|e| e.base().layer_id.clone())
        .expect("its layer");
    let grips = |app: &App, slot: Slot| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        kentos_interaction::room_grip::grips(doc, &app.spatial, &[slot]).len()
    };
    assert_eq!(grips(&app, slot), 4);
    command(
        &mut app,
        "cad.layers.set",
        json!({"operation": "locked", "layer": walls, "value": true}),
    );
    assert_eq!(grips(&app, slot), 0, "the walls locked");
    command(
        &mut app,
        "cad.layers.set",
        json!({"operation": "locked", "layer": walls, "value": false}),
    );
    assert_eq!(grips(&app, slot), 4);
    // The same boundary drawn: it does not follow the walls, no grips.
    let drawn = command(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": rooms, "spaces": [{"parts": parts, "arch": {"level": level}}]}),
    );
    let doc = &app.document.as_ref().expect("a drawing").model;
    let drawn = doc
        .slot_of(uids(&drawn)[0].parse().expect("an id"))
        .expect("the drawn room");
    assert_eq!(grips(&app, drawn), 0, "a drawn boundary");
}

/// Planı ölçülendir (M26, docs/adr/8026) on the 5 × 4 m room of Dış duvar 25: Enter writes the four sides' chains (an ön
/// proje's three rows each; with no grid, two) and two interior chains each way, all in one undo step.
#[test]
fn plan_dimensions_writes_four_sides_and_two_chains_each_way_in_one_step() {
    let (mut app, _) = building();
    let dims = |app: &App| {
        let doc = &app.document.as_ref().expect("a drawing").model;
        doc.entities().filter(|e| e.kind() == "dimension").count()
    };
    assert_eq!(dims(&app), 0);
    let _ = app.update(Message::Run("tool.planDimensions"));
    assert_eq!(app.session.tool_id(), "planDimensions");
    let format = app.format();
    let tag = app
        .session
        .preview(&format)
        .and_then(|p| p.tag)
        .map(|t| t.lines)
        .unwrap_or_default();
    assert!(
        tag.iter()
            .any(|l| l.starts_with("Ön proje: dört cephe, 3 zincir")),
        "{tag:?}"
    );
    assert!(
        tag.iter()
            .any(|l| l.starts_with("İç: 4 zincir") && l.contains("1 mahal")),
        "{tag:?}"
    );
    let _ = app.submit_line("");
    let written = dims(&app);
    assert!(written > 0, "dimensions written");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let sets = doc.dimension_sets().len();
    assert_eq!(sets, 4, "a chain set on each side");
    assert!(
        crate::files_testing::last_said(&app).starts_with("Plan ölçülendi"),
        "{}",
        crate::files_testing::last_said(&app)
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(dims(&app), 0, "one undo takes them all");
}

/// EK-3 numbering (M27, docs/adr/8027): four rooms in the four quarters of Zemin kat; EK-3 on and the section D01;
/// Enter with none clicked numbers the storey Kat_Bölüm_Sıra, clockwise from north: KD, GD, GB, KB.
#[test]
fn ek3_numbers_the_storey_s_rooms_kat_bolum_sira_clockwise_from_north() {
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
    let room = |name: &str, x: f64, y: f64| json!({"name": name, "arch": {"level": level}, "parts": [{"outer": {"pts": [p(x, y), p(x + 4.0, y), p(x + 4.0, y + 3.0), p(x, y + 3.0)]}, "holes": []}]});
    command(
        &mut app,
        "cad.spaces.create",
        json!({"layerId": layer, "spaces": [room("KB", 0.0, 3.0), room("KD", 4.0, 3.0), room("GD", 4.0, 0.0), room("GB", 0.0, 0.0)]}),
    );
    let _ = app.update(Message::Run("tool.spaceNumber"));
    assert_eq!(app.session.tool_id(), "spaceNumber");
    app.with_tool(|s, cx| s.configure("E", "yes", cx));
    app.with_tool(|s, cx| s.configure("B", "D01", cx));
    let format = app.format();
    let rows = app.session.settings(&format);
    let first = rows.iter().find(|s| s.key == "N").map(|s| s.value.clone());
    assert_eq!(first.as_deref(), Some("K00_D01_001"));
    let _ = app.submit_line("");
    let doc = &app.document.as_ref().expect("a drawing").model;
    let numbers: std::collections::BTreeMap<String, String> = doc
        .entities()
        .filter_map(|e| match e {
            kentos_commands::Entity::Space(s) => Some((
                s.space.name.clone().unwrap_or_default(),
                s.space.number.clone().unwrap_or_default(),
            )),
            _ => None,
        })
        .collect();
    assert_eq!(numbers["KD"], "K00_D01_001", "{numbers:?}");
    assert_eq!(numbers["GD"], "K00_D01_002", "{numbers:?}");
    assert_eq!(numbers["GB"], "K00_D01_003", "{numbers:?}");
    assert_eq!(numbers["KB"], "K00_D01_004", "{numbers:?}");
}

/// The MO room tag (M27, docs/adr/8027): Etiket's all-at-once with Elips içinde numara frames the room's number in an
/// ellipse, its name and net area below it; the drawing then writes KCAD schema 219, and reads back the same.
#[test]
fn a_room_tag_in_an_ellipse_writes_schema_207() {
    let (mut app, _) = building();
    let layer = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .layers()
        .active()
        .to_owned();
    command(
        &mut app,
        "cad.tags.create",
        json!({"layerId": layer, "tags": [], "all": {"kinds": ["space"], "shape": "ellipse", "content": "{number}\n{name}\n{netArea}"}}),
    );
    let doc = &app.document.as_ref().expect("a drawing").model;
    let tag = doc
        .entities()
        .find_map(|e| match e {
            kentos_commands::Entity::Tag(t) => Some(t.clone()),
            _ => None,
        })
        .expect("the room's tag");
    assert_eq!(tag.tag.shape, kentos_commands::TagShape::Ellipse);
    assert_eq!(tag.tag.text.lines().next(), Some("101"), "{}", tag.tag.text);
    // Drawn: an ellipse round the number, the name and the area under it.
    let shape =
        kentos_native_application::geometry::shape(&kentos_commands::Entity::Tag(tag.clone()));
    let pieces = kentos_geometry_core::geom::arch::pieces(&shape, doc.settings().plot_scale);
    assert!(
        pieces
            .iter()
            .any(|p| matches!(p.shape, kentos_geometry_core::entity::Shape::Ellipse { .. }))
    );
    let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
    assert_eq!(
        bytes
            .windows(10)
            .filter(|w| *w == b"\x67version\x18\xdb")
            .count(),
        1,
        "schema 219"
    );
    let back = kentos_kcad::decode(&bytes).expect("reads");
    assert!(
        back.entities
            .iter()
            .any(kentos_commands::needs_ellipse_tags)
    );
}
