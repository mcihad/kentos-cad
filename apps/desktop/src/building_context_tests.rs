//! A building object's context (M17, docs/adr/3017): the Duvar tab holds Duvarı kaydır, the selection's tab's own
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
