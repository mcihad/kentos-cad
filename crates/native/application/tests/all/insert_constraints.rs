//! Public commands constrain insertion anchors without rewriting an existing block placement.
use kentos_commands::*;
use kentos_domain::{Document, Slot};
use kentos_native_application::{
    ExecutionContext, constraints_edit, constraints_query, parameters_edit,
};
use serde_json::{Value, json};
fn done<T: std::fmt::Debug>(r: CommandResult<T>) -> T {
    match r {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}
fn drawing() -> Document {
    Document::from_snapshot(
        DocumentSnapshotV1::from_json(include_str!(
            "../../../../../fixtures/interaction/v1/constraint-block-anchors.kcad"
        ))
        .unwrap(),
    )
    .unwrap()
}
fn part(d: &Document, id: u32, place: Value) -> Value {
    json!({"uid":d.uid(Slot(id)).unwrap().to_string(),"element":{"kind":"point","place":place}})
}
fn setup(d: &mut Document, fix_reference: bool) {
    let a = part(d, 1, json!({"kind":"node","part":0}));
    let b = part(d, 2, json!({"kind":"node","part":0}));
    let l = part(d, 3, json!({"kind":"start"}));
    let mut rules = json!([{ "rule":{"kind":"horizontal","a":a,"b":b}},{"rule":{"kind":"distance","a":a,"b":b,"parameter":"yerlesim"},"value":"6m"},{"rule":{"kind":"coincident","a":b,"b":l}}]);
    if fix_reference {
        rules
            .as_array_mut()
            .unwrap()
            .insert(0, json!({"rule":{"kind":"fix","a":a}}));
    }
    done(constraints_edit::execute(
        &mut ExecutionContext::new(d),
        serde_json::from_value(json!({"edit":{"kind":"add","constraints":rules}})).unwrap(),
    ));
}
#[test]
fn mirrored_rotated_anchor_updates_a_follower_preserves_definition_and_placement_then_undoes_once()
{
    let mut d = drawing();
    setup(&mut d, true);
    let before = d.to_snapshot_v2();
    let uid = d.uid(Slot(2));
    let old = d.get(Slot(2)).unwrap().clone();
    let input = serde_json::from_value(
        json!({"edits":[{"kind":"set","name":"yerlesim","expression":"8m"}]}),
    )
    .unwrap();
    done(parameters_edit::plan(
        &ExecutionContext::new(&mut d),
        &input,
    ));
    assert_eq!(d.to_snapshot_v2(), before);
    done(parameters_edit::execute(
        &mut ExecutionContext::new(&mut d),
        input,
    ));
    let Entity::Insert(mut expected) = old else {
        panic!()
    };
    expected.p.x = 8.;
    assert_eq!(d.get(Slot(2)), Some(&Entity::Insert(expected)));
    assert_eq!(d.uid(Slot(2)), uid);
    assert_eq!(d.to_snapshot_v2().blocks, before.blocks);
    let Some(Entity::Line(l)) = d.get(Slot(3)) else {
        panic!()
    };
    assert_eq!((l.a.x, l.a.y, l.b.x, l.b.y), (8., 0., 10., 0.));
    let q = done(constraints_query::read(&d, &ConstraintsQuery::default()));
    assert!(q.constraints.iter().all(|c| c.holds));
    assert_eq!(q.diagnosis.dof, 2);
    let saved = d.to_snapshot_v2();
    assert_eq!(
        Document::from_snapshot_v2(saved.clone())
            .unwrap()
            .to_snapshot_v2(),
        saved
    );
    d.undo().unwrap();
    assert_eq!(d.to_snapshot_v2(), before);
}
#[test]
fn locked_reference_allows_anchor_movement_but_locked_anchor_and_inner_parts_are_atomic() {
    let mut d = drawing();
    let Entity::Point(mut p) = d.get(Slot(1)).unwrap().clone() else {
        panic!()
    };
    p.base.layer_id = "kilitli".into();
    d.update(Slot(1), Entity::Point(p));
    setup(&mut d, false);
    let change = || {
        serde_json::from_value(
            json!({"edits":[{"kind":"set","name":"yerlesim","expression":"8m"}]}),
        )
        .unwrap()
    };
    done(parameters_edit::execute(
        &mut ExecutionContext::new(&mut d),
        change(),
    ));
    let Some(Entity::Point(p)) = d.get(Slot(1)) else {
        panic!()
    };
    assert_eq!((p.p.x, p.p.y), (0., 0.));
    let mut snapshot = d.to_snapshot_v2();
    snapshot.layers[0].locked = true;
    let mut d = Document::from_snapshot_v2(snapshot).unwrap();
    let before = d.to_snapshot_v2();
    let input = serde_json::from_value(
        json!({"edits":[{"kind":"set","name":"yerlesim","expression":"9m"}]}),
    )
    .unwrap();
    assert!(!matches!(
        parameters_edit::execute(&mut ExecutionContext::new(&mut d), input),
        CommandResult::Completed { .. }
    ));
    assert_eq!(d.to_snapshot_v2(), before);
    let mut d = drawing();
    let before = d.to_snapshot_v2();
    let inner = part(&d, 2, json!({"kind":"node","part":1}));
    let result = constraints_edit::execute(
        &mut ExecutionContext::new(&mut d),
        serde_json::from_value(
            json!({"edit":{"kind":"add","constraints":[{"rule":{"kind":"fix","a":inner}}]}}),
        )
        .unwrap(),
    );
    let CommandResult::Failed { error, .. } = result else {
        panic!("{result:?}")
    };
    assert!(error.message.contains("yerleştirme noktası"));
    assert_eq!(d.to_snapshot_v2(), before);
}

#[test]
fn same_definition_geometry_updates_preserve_local_instance_data_without_transferring_it_to_another_definition()
 {
    use kentos_native_application::{edit, geometry};
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/constraint-block-anchors.kcad"
    ))
    .unwrap();
    raw["blocks"][0]["dynamic"] = json!({"parameters":[{"kind":"flip","name":"Taraf","a":{"x":1,"y":1},"b":{"x":1,"y":2}}],"actions":[{"kind":"flip","parameter":"Taraf","pieces":[1,2]}]});
    raw["entities"][1]["parameters"] = json!({"Taraf":true});
    raw["entities"][1]["clipping"] =
        json!({"boundary":[{"x":0,"y":0},{"x":5,"y":0},{"x":5,"y":5},{"x":0,"y":5}]});
    raw["entities"][1]["arch"] = json!({"phase":"new"});
    let mut other = raw["blocks"][0].clone();
    other["id"] = json!("018f3a2b-0000-7000-8000-0000000000d2");
    other["name"] = json!("Diğer tanım");
    other.as_object_mut().unwrap().remove("dynamic");
    raw["blocks"].as_array_mut().unwrap().push(other);
    let mut d =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let before = d.to_snapshot_v2();
    let Entity::Insert(mut expected) = d.get(Slot(2)).unwrap().clone() else {
        panic!()
    };
    assert!(!expected.parameters.is_empty());
    assert!(expected.clipping.is_some());
    assert!(!expected.attribute_places.is_empty());
    assert!(!expected.arch.is_empty());
    expected.p.x = 8.;
    let geometry =
        geometry::edit_geometry(geometry::shape(&Entity::Insert(expected.clone()))).unwrap();
    let uid = d.uid(Slot(2)).unwrap().to_string();
    let input = EntitiesEdit {
        operation: EditOperation::Grip,
        changes: vec![EntityEdit::Update { uid, geometry }],
        expected_revision: None,
    };
    done(edit::plan(&ExecutionContext::new(&mut d), &input));
    assert_eq!(d.to_snapshot_v2(), before);
    done(edit::execute(&mut ExecutionContext::new(&mut d), input));
    assert_eq!(d.get(Slot(2)), Some(&Entity::Insert(expected)));
    let saved = d.to_snapshot_v2();
    assert_eq!(
        Document::from_snapshot_v2(saved.clone())
            .unwrap()
            .to_snapshot_v2(),
        saved
    );
    d.undo().unwrap();
    assert_eq!(d.to_snapshot_v2(), before);
    let mut changed = geometry::edit_geometry(geometry::shape(d.get(Slot(2)).unwrap())).unwrap();
    let EntityGeometry::Insert { block, .. } = &mut changed else {
        panic!()
    };
    *block = d.to_snapshot_v2().blocks[1].id;
    let uid = d.uid(Slot(2)).unwrap().to_string();
    done(edit::execute(
        &mut ExecutionContext::new(&mut d),
        EntitiesEdit {
            operation: EditOperation::Properties,
            changes: vec![EntityEdit::Update {
                uid,
                geometry: changed,
            }],
            expected_revision: None,
        },
    ));
    let Some(Entity::Insert(now)) = d.get(Slot(2)) else {
        panic!()
    };
    assert!(now.parameters.is_empty() && now.attribute_places.is_empty() && now.clipping.is_none());
    assert!(!now.arch.is_empty());
}

#[test]
fn a_temporary_first_object_hold_does_not_report_a_permanent_fix_as_redundant() {
    let mut d = drawing();
    let a = part(&d, 1, json!({"kind":"node","part":0}));
    let b = part(&d, 2, json!({"kind":"node","part":0}));
    done(constraints_edit::execute(
        &mut ExecutionContext::new(&mut d),
        serde_json::from_value(
            json!({"edit":{"kind":"add","constraints":[{"rule":{"kind":"fix","a":a}}]}}),
        )
        .unwrap(),
    ));
    let before = d.to_snapshot_v2();
    let input: ConstraintsEdit = serde_json::from_value(json!({"hold":[d.uid(Slot(1)).unwrap().to_string()],"edit":{"kind":"add","constraints":[{"rule":{"kind":"coincident","a":a,"b":b}}]}})).unwrap();
    let planned = done(constraints_edit::plan(
        &ExecutionContext::new(&mut d),
        &input,
    ));
    assert_eq!(d.to_snapshot_v2(), before);
    assert_eq!(planned.diagnosis.status, SolveStatus::Fully);
    assert!(planned.diagnosis.redundant.is_empty());
    let written = done(constraints_edit::execute(
        &mut ExecutionContext::new(&mut d),
        input,
    ));
    let queried = done(constraints_query::read(&d, &ConstraintsQuery::default()));
    let mut expected = queried.diagnosis;
    expected.iterations = written.diagnosis.iterations;
    assert_eq!(written.diagnosis, expected);
    assert_eq!(planned.diagnosis, written.diagnosis);
    assert_eq!(d.get(Slot(1)).unwrap().clone(), before.entities[0]);
    d.undo().unwrap();
    assert_eq!(d.to_snapshot_v2(), before);
}

#[test]
fn a_temporary_hold_preserves_first_object_without_hiding_persistent_freedoms_or_duplicates() {
    let mut d = drawing();
    let a = part(&d, 1, json!({"kind":"node","part":0}));
    let b = part(&d, 2, json!({"kind":"node","part":0}));
    let input: ConstraintsEdit = serde_json::from_value(json!({"hold":[d.uid(Slot(1)).unwrap().to_string()],"edit":{"kind":"add","constraints":[{"rule":{"kind":"coincident","a":a,"b":b}}]}})).unwrap();
    let written = done(constraints_edit::execute(
        &mut ExecutionContext::new(&mut d),
        input.clone(),
    ));
    assert_eq!(written.diagnosis.status, SolveStatus::Under);
    assert_eq!(
        written.diagnosis.dof, 2,
        "the first object is only held during this operation"
    );
    let Some(Entity::Point(p)) = d.get(Slot(1)) else {
        panic!()
    };
    let Some(Entity::Insert(b)) = d.get(Slot(2)) else {
        panic!()
    };
    assert_eq!((p.p.x, p.p.y, b.p.x, b.p.y), (0., 0., 0., 0.));
    let before = d.to_snapshot_v2();
    let result = constraints_edit::execute(&mut ExecutionContext::new(&mut d), input);
    let CommandResult::Failed { error } = result else {
        panic!("{result:?}")
    };
    assert_eq!(error.code, "constraint_redundant");
    assert_eq!(d.to_snapshot_v2(), before);
}

#[test]
fn an_unsatisfied_temporary_hold_is_not_released_to_make_an_invalid_edit_succeed() {
    let mut d = drawing();
    let a = part(&d, 1, json!({"kind":"node","part":0}));
    let b = part(&d, 2, json!({"kind":"node","part":0}));
    let before = d.to_snapshot_v2();
    let input: ConstraintsEdit = serde_json::from_value(json!({"hold":[d.uid(Slot(1)).unwrap().to_string(),d.uid(Slot(2)).unwrap().to_string()],"edit":{"kind":"add","constraints":[{"rule":{"kind":"coincident","a":a,"b":b}}]}})).unwrap();
    fn conflict<T: std::fmt::Debug>(r: CommandResult<T>) {
        let CommandResult::Failed { error } = r else {
            panic!("{r:?}")
        };
        assert_eq!(error.code, "constraint_conflict");
    }
    conflict(constraints_edit::plan(
        &ExecutionContext::new(&mut d),
        &input,
    ));
    conflict(constraints_edit::execute(
        &mut ExecutionContext::new(&mut d),
        input,
    ));
    assert_eq!(d.to_snapshot_v2(), before);
}
