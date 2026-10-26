use super::wall_contours::{completed, drawing};
use kentos_commands::*;
use kentos_domain::{Document, Slot};
use kentos_native_application::{
    ExecutionContext, schedules_create, spaces_create, spaces_number, tags_create,
};
use serde_json::json;
#[test]
fn moved_tag_schedule_persistence_and_replay_follow_the_same_room_identity() {
    let mut doc = drawing(json!([]), "m");
    let made=completed(spaces_create::execute(&mut ExecutionContext::new(&mut doc),serde_json::from_value(json!({"layerId":"cizim","spaces":[
        {"number":"OLD-1","name":"Salon","usage":"Konut","parts":[{"outer":{"pts":[{"x":0.,"y":0.},{"x":4.,"y":0.},{"x":4.,"y":3.},{"x":0.,"y":3.}]}}]},
        {"number":"OLD-2","name":"Mutfak","parts":[{"outer":{"pts":[{"x":6.,"y":0.},{"x":10.,"y":0.},{"x":10.,"y":3.},{"x":6.,"y":3.}]}}]}]})).unwrap()));
    let tags=completed(tags_create::execute(&mut ExecutionContext::new(&mut doc),serde_json::from_value(json!({"layerId":"cizim","tags":[{"of":made.uids[0],"at":{"x":-3.,"y":5.},"content":"{number} {name}","leader":true},{"of":made.uids[1]}]})).unwrap()));
    let schedule = completed(schedules_create::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({"layerId":"cizim","at":{"x":0.,"y":-5.},"of":"spaces"}))
            .unwrap(),
    ));
    let before = doc.to_snapshot_v2();
    let input = SpacesNumber {
        uids: made.uids.iter().rev().cloned().collect(),
        first: "Z-009".into(),
        order: SpaceNumberOrder::Input,
        expected_revision: Some(doc.revision().to_string()),
    };
    let plan = completed(spaces_number::plan_in(&doc, &input));
    assert_eq!(doc.to_snapshot_v2(), before);
    let out = completed(spaces_number::execute(
        &mut ExecutionContext::new(&mut doc),
        input.clone(),
    ));
    assert_eq!(out.spaces, plan.spaces);
    assert_eq!(out.kept, input.uids);
    let after = doc.to_snapshot_v2();
    for id in &made.ids {
        let mut old = before
            .entities
            .iter()
            .find(|e| e.base().id == *id)
            .unwrap()
            .clone();
        if let Entity::Space(s) = &mut old {
            s.space.number = Some(if *id == made.ids[0] { "Z-010" } else { "Z-009" }.into());
        }
        assert_eq!(doc.get(Slot(*id)), Some(&old));
    }
    let Entity::Tag(tag) = doc.get(Slot(tags.ids[0])).unwrap() else {
        panic!()
    };
    assert_eq!(tag.tag.at, Vec2 { x: -3., y: 5. });
    assert_eq!(tag.tag.text, "Z-010 Salon");
    assert_eq!(tag.tag.of, made.uids[0]);
    let table = serde_json::to_value(doc.get(Slot(schedule.id)).unwrap()).unwrap();
    let text = table.to_string();
    assert!(text.contains("Z-009") && text.contains("Z-010"));
    assert_eq!(
        doc.journal().last().unwrap().command.as_deref(),
        Some("cad.spaces.number")
    );
    let bytes = kentos_kcad::encode_verified(&after).unwrap();
    assert_eq!(kentos_kcad::decode(&bytes).unwrap(), after);
    let reopened = Document::from_snapshot_v2(kentos_kcad::decode(&bytes).unwrap()).unwrap();
    assert_eq!(reopened.to_snapshot_v2(), after);
    // Replay the actual full command journal through the protocol, including
    // created UUID remapping and the numbering preview revision.
    let journal = kentos_native_application::replay::journal_of(&doc);
    let mut replay = drawing(json!([]), "m");
    let report = kentos_native_application::replay::replay(&mut replay, None, &journal);
    assert_eq!(
        (report.same, report.differs, report.failed, report.skipped),
        (4, 0, 0, 0),
        "{report:?}"
    );
    fn remap(v: &mut serde_json::Value, m: &std::collections::BTreeMap<String, String>) {
        match v {
            serde_json::Value::String(s) => {
                if let Some(new) = m.get(s) {
                    *s = new.clone();
                }
            }
            serde_json::Value::Array(a) => {
                for v in a {
                    remap(v, m);
                }
            }
            serde_json::Value::Object(o) => {
                for v in o.values_mut() {
                    remap(v, m);
                }
            }
            _ => {}
        }
    }
    let mut expected = serde_json::to_value(&after).unwrap();
    remap(&mut expected, &report.mapped);
    assert_eq!(
        serde_json::to_value(replay.to_snapshot_v2()).unwrap(),
        expected
    );
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), before);
    doc.redo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), after);
}

#[test]
fn coincident_label_anchors_use_uuid_ties_and_overflowing_direction_is_atomic() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.spaces.number.json"
    ))
    .unwrap();
    let mut setup = fixture["setup"].clone();
    setup["entities"][1]["parts"] = setup["entities"][0]["parts"].clone();
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&setup.to_string()).unwrap())
            .unwrap();
    let ids: Vec<_> = [Slot(1), Slot(2)]
        .into_iter()
        .map(|s| doc.uid(s).unwrap().to_string())
        .collect();
    let before = doc.to_snapshot_v2();
    let mut expected = ids.clone();
    expected.sort();
    let mut input = SpacesNumber {
        uids: ids,
        first: "T-001".into(),
        order: SpaceNumberOrder::Direction {
            from: Vec2 { x: 0., y: 0. },
            to: Vec2 { x: 1., y: 0. },
        },
        expected_revision: None,
    };
    assert_eq!(
        completed(spaces_number::plan_in(&doc, &input)).kept,
        expected
    );
    input.uids.reverse();
    assert_eq!(
        completed(spaces_number::plan_in(&doc, &input)).kept,
        expected
    );
    input.order = SpaceNumberOrder::Direction {
        from: Vec2 { x: 1e308, y: 0. },
        to: Vec2 { x: -1e308, y: 0. },
    };
    assert!(
        matches!(spaces_number::execute(&mut ExecutionContext::new(&mut doc),input),CommandResult::Failed {error} if error.code=="invalid_space_number")
    );
    assert_eq!(doc.to_snapshot_v2(), before);
    assert!(!doc.can_undo());
}
