use kentos_commands::*;
use kentos_domain::Document;
use kentos_native_application::{
    ExecutionContext, openings_create, spaces_create, spaces_quantities, walls_partition,
    walls_rectangle,
};
use serde_json::{Value, json};
fn completed<T: serde::Serialize + std::fmt::Debug>(r: CommandResult<T>) -> Value {
    match r {
        CommandResult::Completed { output, .. } => serde_json::to_value(output).unwrap(),
        other => panic!("{other:?}"),
    }
}
fn room() -> (Document, String) {
    let f: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.walls.rectangle.json"
    ))
    .unwrap();
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&f["setup"].to_string()).unwrap())
            .unwrap();
    let result=completed(walls_rectangle::execute(&mut ExecutionContext::new(&mut doc),serde_json::from_value(json!({
        "layerId":"duvar","frame":{"kind":"corners","a":{"x":0,"y":0},"b":{"x":8,"y":6}},"reference":"inner","width":0.2
    })).unwrap()));
    (doc, result["uids"][0].as_str().unwrap().into())
}
fn request(uid: &str) -> WallsPartition {
    serde_json::from_value(json!({
        "source":{"wall":uid,"edge":0},"distance":3,
        "extent":{"kind":"between","start":{"wall":uid,"edge":3},"end":{"wall":uid,"edge":1}},
        "wall":{"layerId":"duvar","walls":[{"pts":[],"width":0.15,"closed":false}]}
    }))
    .unwrap()
}
#[test]
fn a_three_metre_clear_room_keeps_the_source_door_and_space_follows_a_partition() {
    let (mut doc, uid) = room();
    let door = completed(openings_create::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({
            "layerId":"duvar","openings":[{"host":uid,"at":2,"width":0.9,"form":"door"}]
        }))
        .unwrap(),
    ));
    completed(spaces_create::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({
            "layerId":"duvar","spaces":[{"seed":{"x":2,"y":1},"name":"Alt oda"}]
        }))
        .unwrap(),
    ));
    let before = doc.to_snapshot_v2();
    let input = request(&uid);
    let p = completed(walls_partition::plan_in(&doc, &input));
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(p["clearDistance"], 3.0);
    assert!((p["axisDistance"].as_f64().unwrap() - 3.175).abs() < 1e-12);
    assert!((p["wall"]["walls"][0]["pts"][0]["x"].as_f64().unwrap() + 0.1).abs() < 1e-12);
    completed(walls_partition::execute(
        &mut ExecutionContext::new(&mut doc),
        input,
    ));
    let q = completed(spaces_quantities::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({})).unwrap(),
    ));
    assert!(
        (q["spaces"][0]["area"].as_f64().unwrap() - 24.0).abs() < 1e-8,
        "{q}"
    );
    assert_eq!(
        doc.journal().last().unwrap().command.as_deref(),
        Some("cad.walls.partition")
    );
    let snap = serde_json::to_value(doc.to_snapshot_v2()).unwrap();
    assert!(snap.to_string().contains(door["uids"][0].as_str().unwrap()));
    assert_eq!(doc.undo().as_deref(), Some("Duvar"));
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
}
#[test]
fn directions_and_invalid_boundaries_are_explicit_and_atomic() {
    let (mut doc, uid) = room();
    for (direction, want) in [
        (PartitionDirection::Forward, [2.0, 4.0]),
        (PartitionDirection::Backward, [0.0, 2.0]),
        (PartitionDirection::Both, [1.0, 3.0]),
    ] {
        let mut i = request(&uid);
        i.extent = PartitionExtent::Length {
            anchor: 2.0,
            length: 2.0,
            direction,
        };
        let p = completed(walls_partition::plan_in(&doc, &i));
        let pts = &p["wall"]["walls"][0]["pts"];
        assert_eq!(pts[0]["x"], want[0]);
        assert_eq!(pts[1]["x"], want[1]);
    }
    let before = doc.to_snapshot_v2();
    for extent in [
        PartitionExtent::Between {
            start: WallEdgeRef {
                wall: uid.clone(),
                edge: 0,
            },
            end: WallEdgeRef {
                wall: uid.clone(),
                edge: 2,
            },
        },
        PartitionExtent::Length {
            anchor: 0.0,
            length: 2.0,
            direction: PartitionDirection::Both,
        },
    ] {
        let mut i = request(&uid);
        i.extent = extent;
        assert!(matches!(
            walls_partition::execute(&mut ExecutionContext::new(&mut doc), i),
            CommandResult::Failed { .. }
        ));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn locked_destination_missing_source_and_stale_revision_refuse_without_a_partial_wall() {
    let (mut doc, uid) = room();
    let before = doc.to_snapshot_v2();
    let mut locked = request(&uid);
    locked.wall.layer_id = "kilitli".into();
    let mut missing = request(&uid);
    missing.source.wall = "01900000-0000-7000-8000-000000009999".into();
    let mut stale = request(&uid);
    stale.wall.expected_revision = Some("0".into());
    for i in [locked, missing, stale] {
        assert!(!matches!(
            walls_partition::execute(&mut ExecutionContext::new(&mut doc), i),
            CommandResult::Completed { .. }
        ));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

/// A curved source is partitioned too since ADR 0734: the partition is a concentric arc.
#[test]
fn source_height_and_architecture_survive_on_straight_and_curved_sources() {
    let (mut doc, _) = room();
    for bulges in [None, Some(vec![0.2])] {
        let made = completed(kentos_native_application::walls_create::execute(
            &mut ExecutionContext::new(&mut doc),
            serde_json::from_value(
                json!({"layerId":"duvar","walls":[{"pts":[{"x":20,"y":0},{"x":28,"y":0}],
                "width":0.25,"height":3.1,"arch":{"discipline":"architecture"},"bulges":bulges}]}),
            )
            .unwrap(),
        ));
        let mut i = request(made["uids"][0].as_str().unwrap());
        i.extent = PartitionExtent::Source;
        let before = doc.to_snapshot_v2();
        let p = completed(walls_partition::plan_in(&doc, &i));
        let wall = &p["wall"]["walls"][0];
        assert_eq!(wall["height"], 3.1);
        assert_eq!(wall["arch"]["discipline"], "architecture");
        let curved = wall["bulges"]
            .as_array()
            .is_some_and(|b| b.iter().any(|v| v.as_f64().is_some_and(|v| v != 0.0)));
        assert_eq!(
            curved,
            bulges.is_some(),
            "a curved source gives a curved partition: {wall}"
        );
        assert_eq!(doc.to_snapshot_v2(), before, "a plan writes nothing");
    }
}
