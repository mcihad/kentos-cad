//! The same persistent recipe lifecycle exposed to native tools, Python and MCP.
use kentos_commands::{
    CallAnswer, CallEnvelope, CallExpectations, CallOp, CommandResult, DimensionSetId,
    DocumentSnapshotV1, Entity, EntityId, ProjectPart, StateStamps, TextShift,
};
use kentos_domain::{Document, Uuid};
use kentos_native_application::{ExecutionContext, call::call, registry};
use serde_json::{Value, json};

const CREATE: &str = "cad.dimensionsets.create";
const EDIT: &str = "cad.dimensionsets.edit";
const QUERY: &str = "cad.dimensionsets.query";
fn run(doc: &mut Document, name: &str, op: CallOp, input: Value) -> Value {
    registry::run(&mut ExecutionContext::new(doc), name, op, input).unwrap()
}
fn done(doc: &mut Document, name: &str, input: Value) -> Value {
    let result = run(doc, name, CallOp::Execute, input);
    assert_eq!(result["status"], "completed", "{name}: {result}");
    result["output"].clone()
}
fn output(answer: &CallAnswer) -> &Value {
    match &answer.result {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}
fn envelope(command: &str, op: CallOp, input: Value) -> CallEnvelope {
    CallEnvelope {
        command: command.into(),
        version: None,
        op,
        input,
        request_id: None,
        expected: None,
        relax_constraints: false,
    }
}
fn fixture() -> (Document, String, String) {
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Ölçü API","settings":{"srid":0,"lengthDecimals":3,
        "areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50},"origin":{"x":500000.,"y":4400000.},
        "layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],"activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}});
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).unwrap())
            .unwrap();
    let wall=done(&mut doc,"cad.walls.create",json!({"layerId":"a","walls":[{"pts":[{"x":500000.,"y":4400000.},{"x":500010.,"y":4400000.}],"width":0.2}]}))["uids"][0].as_str().unwrap().to_owned();
    let window = done(
        &mut doc,
        "cad.openings.create",
        json!({"layerId":"a","openings":[{"host":wall,"at":4.,"width":1.}]}),
    )["uids"][0]
        .as_str()
        .unwrap()
        .to_owned();
    (doc, wall, window)
}
fn input(wall: &str) -> Value {
    json!({"layerId":"a","wall":wall,"at":{"x":500000.,"y":4400001.1},"gapMm":6.})
}
fn create(doc: &mut Document, wall: &str) -> DimensionSetId {
    DimensionSetId::parse(done(doc, CREATE, input(wall))["set"].as_str().unwrap()).unwrap()
}
fn edit(doc: &mut Document, id: DimensionSetId, operation: Value) -> Value {
    done(doc, EDIT, json!({"id":id,"operation":operation}))
}
fn dim(doc: &Document, id: EntityId) -> &kentos_commands::DimensionEntity {
    match doc
        .get(doc.slot_of(Uuid::from_bytes(id.0)).unwrap())
        .unwrap()
    {
        Entity::Dimension(d) => d,
        _ => panic!("dimension"),
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn ids(doc: &Document, id: DimensionSetId) -> Vec<EntityId> {
    doc.dimension_set(id)
        .unwrap()
        .members
        .iter()
        .map(|m| m.uid)
        .collect()
}
fn values(doc: &Document, id: DimensionSetId) -> Vec<f64> {
    ids(doc, id)
        .iter()
        .map(|uid| {
            let d = dim(doc, *uid);
            (d.b.x - d.a.x).hypot(d.b.y - d.a.y)
        })
        .collect()
}

#[test]
fn create_plan_is_readonly_and_physical_face_numbers_match_execution() {
    let (mut doc, wall, _) = fixture();
    let before = doc.to_snapshot_v2();
    let next = doc.next_slot();
    let mark = doc.change_mark();
    for op in [CallOp::Validate, CallOp::Plan] {
        let answer = run(&mut doc, CREATE, op, input(&wall));
        assert_eq!(answer["status"], "completed", "{answer}");
        if op == CallOp::Plan {
            let out = &answer["output"];
            assert!(out.get("set").is_none());
            assert_eq!(out["uids"], json!([]));
            let expected = [3.5, 1., 5.5, 10.];
            for (index, d) in out["entities"].as_array().unwrap().iter().enumerate() {
                close(
                    d["b"]["x"].as_f64().unwrap() - d["a"]["x"].as_f64().unwrap(),
                    expected[index],
                );
                close(d["a"]["y"].as_f64().unwrap(), 4400000.1);
                assert_eq!(d["refs"]["a"]["place"]["kind"], "vertex");
                assert_eq!(d["id"], 0);
                close(
                    d["offset"].as_f64().unwrap(),
                    if index == 3 { 1.3 } else { 1. },
                );
            }
        }
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.next_slot(), next);
        assert_eq!(doc.change_mark(), mark);
    }
    let id = create(&mut doc, &wall);
    assert_eq!(values(&doc, id), vec![3.5, 1., 5.5, 10.]);
    assert_eq!(
        done(&mut doc, QUERY, json!({"wall":wall}))["sets"][0]["id"],
        json!(id)
    );
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
    doc.redo();
    assert!(doc.dimension_set(id).is_some());
}

#[test]
fn opening_edit_insert_delete_have_complete_preview_delta_and_stable_pair_ids() {
    let (mut doc, wall, window) = fixture();
    let id = create(&mut doc, &wall);
    let original = ids(&doc, id);
    let source = json!({"uids":[window],"operation":{"kind":"set","width":2.}});
    let before = doc.to_snapshot_v2();
    let preview = run(&mut doc, "cad.openings.edit", CallOp::Plan, source.clone());
    assert_eq!(preview["status"], "completed", "{preview}");
    assert_eq!(
        preview["output"]["followers"]["updated"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["kind"] == "dimension")
            .count(),
        3
    );
    assert_eq!(doc.to_snapshot_v2(), before);
    done(&mut doc, "cad.openings.edit", source);
    assert_eq!(values(&doc, id), vec![3., 2., 5., 10.]);
    assert_eq!(ids(&doc, id), original);
    let source = json!({"layerId":"a","openings":[{"host":wall,"at":8.5,"width":1.}]});
    let before = doc.to_snapshot_v2();
    let next = doc.next_slot();
    let preview = run(
        &mut doc,
        "cad.openings.create",
        CallOp::Plan,
        source.clone(),
    );
    assert_eq!(
        preview["output"]["followers"]["added"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        preview["output"]["followers"]["removed"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.next_slot(), next);
    let new = done(&mut doc, "cad.openings.create", source)["uids"][0].clone();
    assert_eq!(values(&doc, id), vec![3., 2., 3., 1., 1., 10.]);
    let with_new = ids(&doc, id);
    assert_eq!(with_new[0], original[0]);
    assert_eq!(with_new[1], original[1]);
    assert_eq!(with_new[5], original[3]);
    done(&mut doc, "cad.entities.delete", json!({"uids":[new]}));
    assert_eq!(values(&doc, id), vec![3., 2., 5., 10.]);
    assert_ne!(ids(&doc, id)[2], original[2]);
    doc.undo();
    assert_eq!(ids(&doc, id), with_new);
}

#[test]
fn row_move_reset_text_suppression_restore_detach_and_delete_are_distinct_undoable_operations() {
    let (mut doc, wall, _) = fixture();
    let id = create(&mut doc, &wall);
    let original = ids(&doc, id);
    edit(
        &mut doc,
        id,
        json!({"kind":"rowOffset","row":"overall","offset":2.}),
    );
    close(
        dim(&doc, original[3]).a.y + dim(&doc, original[3]).offset,
        4400002.1,
    );
    let old = doc
        .get(doc.slot_of(Uuid::from_bytes(original[1].0)).unwrap())
        .unwrap()
        .clone();
    let mut appearance = old;
    if let Entity::Dimension(d) = &mut appearance {
        d.text_shift = Some(TextShift {
            along: 0.5,
            across: 0.2,
        });
    }
    doc.update(
        doc.slot_of(Uuid::from_bytes(original[1].0)).unwrap(),
        appearance,
    );
    edit(&mut doc, id, json!({"kind":"resetRows"}));
    assert!(dim(&doc, original[1]).text_shift.is_some());
    close(
        dim(&doc, original[3]).a.y + dim(&doc, original[3]).offset,
        4400001.4,
    );
    let before = doc.to_snapshot_v2();
    let planned = run(
        &mut doc,
        EDIT,
        CallOp::Plan,
        json!({"id":id,"operation":{"kind":"resetText","row":"openings"}}),
    );
    assert_eq!(planned["output"]["entities"].as_array().unwrap().len(), 1);
    assert_eq!(doc.to_snapshot_v2(), before);
    edit(&mut doc, id, json!({"kind":"resetText","row":"openings"}));
    assert!(dim(&doc, original[1]).text_shift.is_none());
    done(
        &mut doc,
        "cad.entities.delete",
        json!({"uids":[original[1]]}),
    );
    assert_eq!(doc.dimension_set(id).unwrap().suppressed.len(), 1);
    let before_restore = ids(&doc, id);
    edit(&mut doc, id, json!({"kind":"restoreHidden"}));
    assert_eq!(ids(&doc, id).len(), 4);
    assert_ne!(ids(&doc, id)[1], original[1]);
    doc.undo();
    assert_eq!(ids(&doc, id), before_restore);
    doc.redo();
    let before = doc.to_snapshot_v2();
    let snapshot_members: Vec<_> = doc
        .entities()
        .filter_map(|e| {
            if let Entity::Dimension(d) = e {
                Some(d.clone())
            } else {
                None
            }
        })
        .collect();
    let p = run(
        &mut doc,
        EDIT,
        CallOp::Plan,
        json!({"id":id,"operation":{"kind":"detach"}}),
    );
    assert_eq!(p["output"]["removed"], json!([]));
    assert_eq!(doc.to_snapshot_v2(), before);
    edit(&mut doc, id, json!({"kind":"detach"}));
    assert!(doc.dimension_set(id).is_none());
    assert_eq!(
        doc.entities()
            .filter_map(|e| if let Entity::Dimension(d) = e {
                Some(d.clone())
            } else {
                None
            })
            .collect::<Vec<_>>(),
        snapshot_members
    );
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
    let p = run(
        &mut doc,
        EDIT,
        CallOp::Plan,
        json!({"id":id,"operation":{"kind":"delete"}}),
    );
    assert_eq!(p["output"]["removed"].as_array().unwrap().len(), 4);
    edit(&mut doc, id, json!({"kind":"delete"}));
    assert!(doc.dimension_set(id).is_none());
    assert_eq!(doc.len(), 2);
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn locks_revision_and_unsupported_sources_refuse_without_touching_recipes() {
    let (mut doc, wall, _) = fixture();
    let id = create(&mut doc, &wall);
    doc.set_layer_locked("a", true);
    let before = doc.to_snapshot_v2();
    for op in [
        json!({"kind":"move","at":{"x":0.,"y":2.}}),
        json!({"kind":"detach"}),
        json!({"kind":"delete"}),
    ] {
        let answer = run(
            &mut doc,
            EDIT,
            CallOp::Execute,
            json!({"id":id,"operation":op}),
        );
        assert_eq!(answer["error"]["code"], "layer_locked", "{answer}");
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    doc.set_layer_locked("a", false);
    let before = doc.to_snapshot_v2();
    let answer = run(
        &mut doc,
        EDIT,
        CallOp::Execute,
        json!({"id":id,"operation":{"kind":"set","gapMm":8.},"expectedRevision":"0"}),
    );
    assert_eq!(answer["status"], "conflict");
    assert_eq!(doc.to_snapshot_v2(), before);
    for shape in [
        json!({"pts":[{"x":0.,"y":0.},{"x":10.,"y":0.},{"x":10.,"y":5.}]}),
        json!({"pts":[{"x":0.,"y":0.},{"x":10.,"y":0.}],"bulges":[0.3,0.]}),
    ] {
        let mut wall = shape;
        wall["kind"] = json!("wall");
        wall["id"] = json!(0);
        wall["layerId"] = json!("a");
        wall["attrs"] = json!({});
        wall["width"] = json!(0.2);
        let slot = doc.add(serde_json::from_value(wall).unwrap()).unwrap();
        let host = doc.uid(slot).unwrap().to_string();
        let before = doc.to_snapshot_v2();
        let answer = run(&mut doc, CREATE, CallOp::Execute, input(&host));
        assert_eq!(answer["error"]["code"], "invalid_dimension_set", "{answer}");
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn plan_hash_normalizes_new_set_ids_and_metadata_only_steps_have_stamps_undo_and_idempotency() {
    let (mut doc, wall, _) = fixture();
    let mut create_input = input(&wall);
    create_input["rows"] = json!(["openings"]);
    let before = doc.to_snapshot_v2();
    let plan = call(
        &mut doc,
        None,
        envelope(CREATE, CallOp::Plan, create_input.clone()),
    );
    output(&plan);
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut write = envelope(CREATE, CallOp::Execute, create_input);
    write.expected = Some(CallExpectations {
        plan: plan.plan_id.clone(),
        ..Default::default()
    });
    let answer = call(&mut doc, None, write);
    let id = DimensionSetId::parse(output(&answer)["set"].as_str().unwrap()).unwrap();
    assert_eq!(
        answer.plan_id, plan.plan_id,
        "new set and member ids are normalized in the fingerprint"
    );
    let members = doc.entities().cloned().collect::<Vec<_>>();
    let stamp = doc.dimension_sets_stamp();
    let input = json!({"id":id,"operation":{"kind":"set","gapMm":9.}});
    let plan = call(&mut doc, None, envelope(EDIT, CallOp::Plan, input.clone()));
    output(&plan);
    let summary = plan.summary.as_ref().unwrap();
    assert!(summary.project.contains(&ProjectPart::DimensionSets));
    assert_eq!(
        summary.counts.added + summary.counts.changed + summary.counts.removed,
        0
    );
    assert!(summary.undo.label.is_some());
    assert_eq!(
        summary.sources.dimension_sets.as_deref(),
        Some(stamp.as_str())
    );
    let mut request = envelope(EDIT, CallOp::Execute, input);
    request.request_id = Some("dimset-gap-1".into());
    request.expected = Some(CallExpectations {
        plan: plan.plan_id.clone(),
        dimension_sets: Some(stamp.clone()),
        ..Default::default()
    });
    let answer = call(&mut doc, None, request.clone());
    output(&answer);
    assert_ne!(doc.dimension_sets_stamp(), stamp);
    assert_eq!(doc.entities().cloned().collect::<Vec<_>>(), members);
    assert_eq!(answer.plan_id, plan.plan_id);
    let revision = doc.revision();
    let replay = call(&mut doc, None, request);
    assert!(replay.replayed);
    assert_eq!(doc.revision(), revision);
    let mut stale = envelope(
        EDIT,
        CallOp::Execute,
        json!({"id":id,"operation":{"kind":"set","gapMm":10.}}),
    );
    stale.expected = Some(CallExpectations {
        dimension_sets: Some(stamp),
        ..Default::default()
    });
    let stale = call(&mut doc, None, stale);
    assert!(matches!(stale.result,CommandResult::Conflict{ref error} if error.code=="stale_state"));
    doc.undo();
    assert_eq!(doc.dimension_set(id).unwrap().gap_mm, 6.);
}

#[test]
fn empty_new_fields_stay_absent_from_legacy_wire_results_and_owned_geometry_plan_refuses() {
    assert_eq!(
        serde_json::to_value(StateStamps::default()).unwrap(),
        json!({})
    );
    assert_eq!(
        serde_json::to_value(CallExpectations::default()).unwrap(),
        json!({})
    );
    let (mut doc, wall, window) = fixture();
    let ordinary = run(
        &mut doc,
        "cad.openings.edit",
        CallOp::Plan,
        json!({"uids":[window],"operation":{"kind":"set","width":1.1}}),
    );
    assert!(ordinary["output"].get("followers").is_none());
    let id = create(&mut doc, &wall);
    let member = ids(&doc, id)[0];
    let d = dim(&doc, member);
    let geometry = json!({"kind":"dimension","a":{"x":d.a.x+1.,"y":d.a.y},"b":d.b,"offset":d.offset,"height":d.height,"style":"linear","angle":d.angle});
    let before = doc.to_snapshot_v2();
    let answer = call(
        &mut doc,
        None,
        envelope(
            "cad.entities.edit",
            CallOp::Plan,
            json!({"operation":"properties","changes":[{"kind":"update","uid":member,"geometry":geometry}]}),
        ),
    );
    assert!(
        matches!(answer.result,CommandResult::Failed{ref error} if error.code=="dimension_set_owned"),
        "{answer:?}"
    );
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn journal_replay_maps_created_recipe_ids_through_later_metadata_and_member_operations() {
    use kentos_native_application::replay::{journal_of, replay};
    let (mut source, wall, _) = fixture();
    let mut base = source.to_snapshot();
    base.entities.clear();
    let mut target = Document::from_snapshot(base).unwrap();
    let id = create(&mut source, &wall);
    edit(&mut source, id, json!({"kind":"set","gapMm":8.}));
    edit(
        &mut source,
        id,
        json!({"kind":"rowOffset","row":"overall","offset":2.}),
    );
    let removed = ids(&source, id)[1];
    done(
        &mut source,
        "cad.entities.delete",
        json!({"uids":[removed]}),
    );
    edit(&mut source, id, json!({"kind":"restoreHidden"}));
    let journal = journal_of(&source);
    let created = journal
        .calls
        .iter()
        .find(|r| r.command.as_deref() == Some(CREATE))
        .unwrap();
    assert_eq!(created.dimension_sets.added, vec![id.to_string()]);
    let encoded = serde_json::to_value(&journal).unwrap();
    assert!(
        encoded["calls"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r.get("dimensionSets").is_some())
    );
    let decoded: kentos_commands::ReplayJournal = serde_json::from_value(encoded).unwrap();
    let report = replay(&mut target, None, &decoded);
    assert_eq!(report.failed, 0, "{report:?}");
    assert_eq!(report.differs, 0, "{report:?}");
    let mapped = DimensionSetId::parse(
        report
            .mapped
            .get(&id.to_string())
            .expect("recipe identity mapped"),
    )
    .unwrap();
    assert_ne!(id, mapped);
    assert_eq!(values(&target, mapped), values(&source, id));
    assert_eq!(
        target.dimension_set(mapped).unwrap().row_places,
        source.dimension_set(id).unwrap().row_places
    );
    assert_eq!(target.dimension_set(mapped).unwrap().gap_mm, 8.);
    assert!(target.dimension_set(mapped).unwrap().suppressed.is_empty());
    // A subsequent detach uses the remapped identity as well and must not leave an orphan recipe.
    edit(&mut source, id, json!({"kind":"detach"}));
    let mut base = source.to_snapshot();
    base.entities.clear();
    let mut all = Document::from_snapshot(base).unwrap();
    let report = replay(&mut all, None, &journal_of(&source));
    assert_eq!(report.failed, 0, "{report:?}");
    assert_eq!(report.differs, 0, "{report:?}");
    assert!(all.dimension_sets().is_empty());
    assert_eq!(all.len(), source.len());
}

#[test]
fn settings_scale_refreshes_row_gaps_when_all_member_heights_are_custom() {
    let (mut doc, wall, _) = fixture();
    let id = create(&mut doc, &wall);
    let members = ids(&doc, id);
    for uid in &members {
        let slot = doc.slot_of(Uuid::from_bytes(uid.0)).unwrap();
        let mut e = doc.get(slot).unwrap().clone();
        if let Entity::Dimension(d) = &mut e {
            d.height = 0.222;
        }
        doc.update(slot, e);
    }
    let before = doc.to_snapshot_v2();
    let next = doc.next_slot();
    let proposal = run(
        &mut doc,
        "cad.settings.set",
        CallOp::Plan,
        json!({"plotScale":100}),
    );
    assert_eq!(proposal["status"], "completed", "{proposal}");
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.next_slot(), next);
    done(&mut doc, "cad.settings.set", json!({"plotScale":100}));
    assert_eq!(ids(&doc, id), members);
    for uid in &members {
        close(dim(&doc, *uid).height, 0.222);
    }
    close(dim(&doc, members[3]).offset, 1.6);
    close(dim(&doc, members[0]).offset, 1.);
    let entities = doc.entities().cloned().collect::<Vec<_>>();
    done(&mut doc, "cad.settings.set", json!({"plotScale":100}));
    assert_eq!(doc.entities().cloned().collect::<Vec<_>>(), entities);
}

#[test]
fn scale_summary_reports_cached_rows_and_plan_stamps_guard_recipe_changes() {
    let (mut doc, wall, _) = fixture();
    let id = create(&mut doc, &wall);
    let members = ids(&doc, id);
    for uid in &members {
        let slot = doc.slot_of(Uuid::from_bytes(uid.0)).unwrap();
        let mut e = doc.get(slot).unwrap().clone();
        if let Entity::Dimension(d) = &mut e {
            d.height = 0.222;
        }
        doc.update(slot, e);
    }
    let before = doc.to_snapshot_v2();
    let member = members[3].to_string();
    let recipe_stamp = doc.dimension_sets_stamp();
    let proposal = call(
        &mut doc,
        None,
        envelope("cad.settings.set", CallOp::Plan, json!({"plotScale":100})),
    );
    output(&proposal);
    assert_eq!(doc.to_snapshot_v2(), before);
    let summary = proposal.summary.as_ref().unwrap();
    assert_eq!(summary.changed, vec![member.clone()]);
    assert_eq!(summary.counts.changed, 1);
    assert_eq!(
        summary.sources.dimension_sets.as_deref(),
        Some(recipe_stamp.as_str())
    );
    assert!(summary.sources.objects.contains_key(&member));
    assert!(summary.stamps.objects.contains_key(&member));
    assert!(summary.undo.label.is_none());
    assert!(summary.undo.outside.contains(&ProjectPart::Settings));
    let mut request = envelope(
        "cad.settings.set",
        CallOp::Execute,
        json!({"plotScale":100}),
    );
    request.expected = Some(CallExpectations {
        plan: proposal.plan_id.clone(),
        ..Default::default()
    });
    let answer = call(&mut doc, None, request.clone());
    output(&answer);
    assert_eq!(answer.plan_id, proposal.plan_id);
    assert_eq!(answer.summary.as_ref().unwrap().changed, vec![member]);
    // A manual row edit after a new proposal changes a source recipe even though settings did not change.
    request.input = json!({"plotScale":200});
    let proposal = call(
        &mut doc,
        None,
        envelope("cad.settings.set", CallOp::Plan, request.input.clone()),
    );
    output(&proposal);
    edit(
        &mut doc,
        id,
        json!({"kind":"rowOffset","row":"overall","offset":2.}),
    );
    let before = doc.to_snapshot_v2();
    request.expected = Some(CallExpectations {
        plan: proposal.plan_id,
        ..Default::default()
    });
    let refused = call(&mut doc, None, request);
    assert!(
        matches!(refused.result,CommandResult::Conflict{ref error} if error.code=="plan_changed"),
        "{refused:?}"
    );
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn scale_plan_guards_new_or_changed_recipes_even_when_no_member_geometry_would_change() {
    for create_later in [false, true] {
        let (mut doc, wall, _) = fixture();
        let create_single = |doc: &mut Document| {
            let mut value = input(&wall);
            value["rows"] = json!(["openings"]);
            let id =
                DimensionSetId::parse(done(doc, CREATE, value)["set"].as_str().unwrap()).unwrap();
            for uid in ids(doc, id) {
                let slot = doc.slot_of(Uuid::from_bytes(uid.0)).unwrap();
                let mut e = doc.get(slot).unwrap().clone();
                if let Entity::Dimension(d) = &mut e {
                    d.height = 0.222;
                }
                doc.update(slot, e);
            }
            id
        };
        let id = if create_later {
            None
        } else {
            Some(create_single(&mut doc))
        };
        let proposal = call(
            &mut doc,
            None,
            envelope("cad.settings.set", CallOp::Plan, json!({"plotScale":100})),
        );
        output(&proposal);
        assert_eq!(proposal.summary.as_ref().unwrap().counts.changed, 0);
        if let Some(id) = id {
            edit(&mut doc, id, json!({"kind":"set","gapMm":9.}));
        } else {
            create_single(&mut doc);
        }
        let before = doc.to_snapshot_v2();
        let mut request = envelope(
            "cad.settings.set",
            CallOp::Execute,
            json!({"plotScale":100}),
        );
        request.expected = Some(CallExpectations {
            plan: proposal.plan_id,
            ..Default::default()
        });
        let refused = call(&mut doc, None, request);
        assert!(
            matches!(refused.result,CommandResult::Conflict{ref error} if error.code=="plan_changed"),
            "{refused:?}"
        );
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn standard_new_intervals_use_current_paper_height_after_scale_change_and_preserve_manual_members()
{
    let (mut doc, wall, _) = fixture();
    let id = create(&mut doc, &wall);
    let old = ids(&doc, id);
    let manual = old[0];
    let slot = doc.slot_of(Uuid::from_bytes(manual.0)).unwrap();
    let mut e = doc.get(slot).unwrap().clone();
    if let Entity::Dimension(d) = &mut e {
        d.height = 0.222;
        d.text_shift = Some(TextShift {
            along: 0.2,
            across: 0.3,
        });
        d.look.prefix = Some("L=".into());
    }
    doc.update(slot, e);
    close(dim(&doc, old[1]).height, 0.125);
    done(&mut doc, "cad.settings.set", json!({"plotScale":100}));
    close(dim(&doc, old[1]).height, 0.25);
    close(dim(&doc, manual).height, 0.222);
    let before = doc.to_snapshot_v2();
    let source = json!({"layerId":"a","openings":[{"host":wall,"at":8.5,"width":1.}]});
    let proposal = run(
        &mut doc,
        "cad.openings.create",
        CallOp::Plan,
        source.clone(),
    );
    assert_eq!(proposal["status"], "completed", "{proposal}");
    let new = proposal["output"]["followers"]["added"].as_array().unwrap();
    assert_eq!(new.len(), 3);
    for d in new {
        assert_eq!(d["kind"], "dimension");
        close(d["height"].as_f64().unwrap(), 0.25);
    }
    assert_eq!(doc.to_snapshot_v2(), before);
    done(&mut doc, "cad.openings.create", source);
    let new: Vec<_> = ids(&doc, id)
        .into_iter()
        .filter(|uid| !old.contains(uid))
        .collect();
    assert_eq!(new.len(), 3);
    for uid in &new {
        close(dim(&doc, *uid).height, 0.25);
    }
    close(dim(&doc, manual).height, 0.222);
    assert_eq!(dim(&doc, manual).look.prefix.as_deref(), Some("L="));
    assert_eq!(
        dim(&doc, manual).text_shift,
        Some(TextShift {
            along: 0.2,
            across: 0.3
        })
    );
    let after = doc.to_snapshot_v2();
    let reopened = Document::from_snapshot_v2(after.clone()).unwrap();
    assert_eq!(reopened.to_snapshot_v2(), after);
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
    doc.redo();
    assert_eq!(doc.to_snapshot_v2(), after);
}
