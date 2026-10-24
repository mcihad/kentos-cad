//! The additive ARCH-DRAW-01 command uses the same native registry through the headless host.

use kentos_commands::{DrawingFont, Workspace};
use kentos_headless::{NewDrawing, Op, Session};
use serde_json::json;

#[test]
fn rectangular_walls_use_the_native_command_with_a_nonmutating_plan_and_one_undo() {
    let mut session = Session::new(&NewDrawing {
        name: "Dikdörtgen mimari duvar".into(),
        srid: 0,
        plot_scale: 100.0,
        workspace: Workspace::Cad,
        drawing_font: DrawingFont::Barlow,
        province: None,
        drawing_unit: None,
        annotation: None,
    })
    .unwrap();
    let layer = session.summary()["activeLayer"]
        .as_str()
        .unwrap()
        .to_owned();
    let input = json!({"layerId":layer,"frame":{"kind":"corners",
        "a":{"x":0,"y":0},"b":{"x":4,"y":3}},"width":0.2});
    let before = session.document().to_snapshot_v2();
    let plan = session
        .run("cad.walls.rectangle", Some(1), Op::Plan, input.clone())
        .unwrap();
    assert_eq!(plan["status"], "completed", "{plan}");
    assert_eq!(plan["output"]["walls"][0]["justify"], "left");
    assert_eq!(session.document().to_snapshot_v2(), before);
    let made = session
        .run("cad.walls.rectangle", Some(1), Op::Execute, input)
        .unwrap();
    assert_eq!(made["status"], "completed", "{made}");
    assert_eq!(session.summary()["objects"], 1);
    let uid = made["output"]["uids"][0].as_str().unwrap();
    let entity = session.entity(uid).unwrap();
    assert_eq!(entity["entity"]["kind"], "wall");
    assert_eq!(entity["entity"]["closed"], true);
    assert_eq!(session.document().journal().count(), 1);
    assert_eq!(session.undo().as_deref(), Some("Duvar"));
    assert_eq!(session.summary()["objects"], 0);
    assert_eq!(session.document().settings(), &before.settings);
}
