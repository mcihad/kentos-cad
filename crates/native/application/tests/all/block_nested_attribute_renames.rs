//! Opt-in tag identity migration preserves nested values, geometry and one-step undo.
use kentos_commands::{CallOp, DocumentSnapshotV2};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

const BLOCK: &str = "0192f5a1-2900-7000-8000-0000000000c1";
const PARENT: &str = "0192f5a1-2900-7000-8000-0000000000c2";
fn source() -> Value {
    serde_json::to_value(
        kentos_kcad::decode(include_bytes!(
            "../../../../../fixtures/interaction/v1/block-parametric-component.kcad"
        ))
        .unwrap(),
    )
    .unwrap()
}
fn open(v: Value) -> Document {
    Document::from_snapshot_v2(serde_json::from_value::<DocumentSnapshotV2>(v).unwrap()).unwrap()
}
fn snap(doc: &Document) -> Value {
    serde_json::to_value(doc.to_snapshot_v2()).unwrap()
}
fn input(v: &Value, nested: bool) -> Value {
    let mut attrs = v["blocks"][0]["attributes"].clone();
    attrs[0]["tag"] = json!("ODA");
    json!({"operation":"attributes","block":BLOCK,"attributes":attrs,"renames":{"NO":"ODA"},"renameNested":nested})
}
fn run(doc: &mut Document, op: CallOp, input: Value) -> Value {
    registry::run(
        &mut ExecutionContext::new(doc),
        "cad.blocks.edit",
        op,
        input,
    )
    .unwrap()
}
#[test]
fn explicit_scope_preserves_values_places_parameters_measurement_and_one_undo() {
    for nested in [false, true] {
        let mut v = source();
        v["blocks"][1]["card"] = json!({"version":4,"category":"Giriş"});
        v["entities"][0]["attributePlaces"] = json!({"NO":{"x":0.3,"y":-0.4}});
        if nested {
            v["blocks"][1]["entities"][0]["attributePlaces"] = json!({"NO":{"x":0.45,"y":-0.6}});
        }
        let args = input(&v, nested);
        let mut doc = open(v);
        let before = snap(&doc);
        let journal = doc.journal().count();
        for op in [CallOp::Validate, CallOp::Plan] {
            let answer = run(&mut doc, op, args.clone());
            assert_eq!(answer["status"], "completed", "{answer}");
            assert_eq!(snap(&doc), before);
            if op == CallOp::Plan {
                assert_eq!(
                    answer["output"]["changed"].as_array().unwrap().len(),
                    if nested { 2 } else { 1 }
                );
            }
        }
        let answer = run(&mut doc, CallOp::Execute, args);
        assert_eq!(answer["status"], "completed", "{answer}");
        let after = snap(&doc);
        for (idx, value) in [(0, "K-1"), (1, "K-2")] {
            assert_eq!(after["entities"][idx]["attrs"]["ODA"], value);
            assert!(after["entities"][idx]["attrs"].get("NO").is_none());
        }
        assert_eq!(
            after["entities"][0]["attributePlaces"]["ODA"],
            before["entities"][0]["attributePlaces"]["NO"]
        );
        if nested {
            let mut expected = before["blocks"][1]["entities"][0].clone();
            expected["attrs"].as_object_mut().unwrap().remove("NO");
            expected["attrs"]["ODA"] = json!("İÇ-1");
            let p = expected["attributePlaces"]
                .as_object_mut()
                .unwrap()
                .remove("NO")
                .unwrap();
            expected["attributePlaces"]["ODA"] = p;
            assert_eq!(after["blocks"][1]["entities"][0], expected);
            assert_eq!(after["blocks"][1]["card"]["version"], 5);
        } else {
            assert_eq!(after["blocks"][1], before["blocks"][1]);
        }
        assert_eq!(after["entities"][3], before["entities"][3]);
        assert_eq!(
            after["blocks"][0]["dynamic"],
            before["blocks"][0]["dynamic"]
        );
        assert_eq!(
            after["entities"][0]["parameters"],
            before["entities"][0]["parameters"]
        );
        assert_eq!(doc.journal().count(), journal + 1);
        let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).unwrap();
        assert_eq!(kentos_kcad::decode(&bytes).unwrap(), doc.to_snapshot_v2());
        doc.undo();
        assert_eq!(snap(&doc), before);
        doc.redo();
        assert_eq!(snap(&doc), after);
    }
}

#[test]
fn stale_revision_and_unrelated_locks_do_not_corrupt_scope_or_create_extra_edits() {
    let v = source();
    let mut doc = open(v.clone());
    let mut args = input(&v, true);
    args["expectedRevision"] = json!("999");
    let before = snap(&doc);
    for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
        let a = run(&mut doc, op, args.clone());
        assert_ne!(a["status"], "completed", "{a}");
        assert_eq!(snap(&doc), before);
    }
    args.as_object_mut().unwrap().remove("expectedRevision");
    assert_eq!(run(&mut doc, CallOp::Execute, args)["status"], "completed");
    let mut after = snap(&doc);
    let layer = after["entities"][2]["layerId"].clone();
    for l in after["layers"].as_array_mut().unwrap() {
        if l["id"] == layer {
            l["locked"] = json!(true);
        }
    }
    let mut locked = open(after);
    let before = snap(&locked);
    let revision = locked.revision();
    let a = run(
        &mut locked,
        CallOp::Execute,
        json!({"operation":"sync","block":BLOCK,"renames":{"NO":"ODA"},"renameNested":true}),
    );
    assert_eq!(a["status"], "completed", "{a}");
    assert_eq!(snap(&locked), before);
    assert_eq!(locked.revision(), revision);
}
#[test]
fn unrelated_target_value_or_position_conflicts_refuse_all_phases_atomically() {
    for place in [false, true] {
        let mut v = source();
        if place {
            let mut other = v["blocks"][0]["attributes"][0].clone();
            other["tag"] = json!("ODA");
            v["blocks"][0]["attributes"]
                .as_array_mut()
                .unwrap()
                .push(other);
            v["blocks"][1]["entities"][0]["attributePlaces"] =
                json!({"NO":{"x":0,"y":0},"ODA":{"x":1,"y":1}});
        } else {
            v["blocks"][1]["entities"][0]["attrs"]["ODA"] = json!("Başka kayıt");
        }
        let mut args = input(&v, true);
        if place {
            args["attributes"].as_array_mut().unwrap().truncate(1);
        }
        let mut doc = open(v);
        let before = snap(&doc);
        let rev = doc.revision();
        for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
            let a = run(&mut doc, op, args.clone());
            assert_eq!(a["error"]["code"], "duplicate_tag", "{a}");
            assert!(
                a["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("İç içe kapılı giriş")
            );
            assert_eq!(snap(&doc), before);
            assert_eq!(doc.revision(), rev);
        }
    }
}
#[test]
fn locked_direct_or_parent_uses_refuse_opt_in_without_partial_root_changes() {
    for root in [0, 2] {
        let mut v = source();
        let layer = v["entities"][root]["layerId"].clone();
        for l in v["layers"].as_array_mut().unwrap() {
            if l["id"] == layer {
                l["locked"] = json!(true);
            }
        }
        let args = input(&v, true);
        let mut doc = open(v);
        let before = snap(&doc);
        for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
            let a = run(&mut doc, op, args.clone());
            assert_eq!(a["error"]["code"], "layer_locked", "{a}");
            assert_eq!(snap(&doc), before);
        }
    }
}
#[test]
fn posthoc_sync_moves_only_tag_identity_and_reapplication_is_an_exact_noop() {
    let v = source();
    let mut doc = open(v.clone());
    assert_eq!(
        run(&mut doc, CallOp::Execute, input(&v, false))["status"],
        "completed"
    );
    let before = snap(&doc);
    let args = json!({"operation":"sync","block":BLOCK,"renames":{"NO":"ODA"},"renameNested":true});
    let a = run(&mut doc, CallOp::Execute, args.clone());
    assert_eq!(a["status"], "completed", "{a}");
    assert_eq!(a["output"]["changed"], json!([PARENT]));
    let after = snap(&doc);
    assert_eq!(after["entities"], before["entities"]);
    assert_eq!(after["blocks"][1]["entities"][0]["attrs"]["ODA"], "İÇ-1");
    let rev = doc.revision();
    let a = run(&mut doc, CallOp::Execute, args);
    assert_eq!(a["output"]["changed"], json!([]));
    assert_eq!(snap(&doc), after);
    assert_eq!(doc.revision(), rev);
    doc.undo();
    assert_eq!(snap(&doc), before);
}
#[test]
fn invalid_scope_operation_and_sync_disabled_refuse_before_writes() {
    let v = source();
    let mut doc = open(v.clone());
    let before = snap(&doc);
    for operation in ["export", "rename", "dynamic", "import", "attributes"] {
        let mut args = input(&v, true);
        args["operation"] = json!(operation);
        if operation == "attributes" {
            args["sync"] = json!(false);
        }
        for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
            let a = run(&mut doc, op, args.clone());
            assert_eq!(a["error"]["code"], "invalid_parameter_value", "{a}");
            assert_eq!(a["error"]["path"], "renameNested");
            assert_eq!(snap(&doc), before);
        }
    }
}

#[test]
fn simultaneous_swap_and_chain_preserve_all_source_values_and_custom_positions() {
    for chain in [false, true] {
        let mut v = source();
        let mut second = v["blocks"][0]["attributes"][0].clone();
        second["tag"] = json!("ODA");
        v["blocks"][0]["attributes"]
            .as_array_mut()
            .unwrap()
            .push(second);
        for e in v["entities"].as_array_mut().unwrap().iter_mut().take(2) {
            e["attrs"]["ODA"] = json!("Oda değeri");
            e["attributePlaces"] = json!({"NO":{"x":0.3,"y":0.4},"ODA":{"x":0.7,"y":0.8}});
        }
        let local = &mut v["blocks"][1]["entities"][0];
        local["attrs"]["ODA"] = json!("İç oda");
        local["attributePlaces"] = json!({"NO":{"x":0.1,"y":0.2},"ODA":{"x":0.5,"y":0.6}});
        let mut args = input(&v, true);
        let last = if chain { "KOD" } else { "NO" };
        args["attributes"][1]["tag"] = json!(last);
        args["renames"]["ODA"] = json!(last);
        let mut doc = open(v);
        let before = snap(&doc);
        let a = run(&mut doc, CallOp::Execute, args);
        assert_eq!(a["status"], "completed", "{a}");
        let after = snap(&doc);
        for (a, b) in [
            (after["entities"][0].clone(), before["entities"][0].clone()),
            (
                after["blocks"][1]["entities"][0].clone(),
                before["blocks"][1]["entities"][0].clone(),
            ),
        ] {
            assert_eq!(a["attrs"]["ODA"], b["attrs"]["NO"]);
            assert_eq!(a["attrs"][last], b["attrs"]["ODA"]);
            assert_eq!(a["attributePlaces"]["ODA"], b["attributePlaces"]["NO"]);
            assert_eq!(a["attributePlaces"][last], b["attributePlaces"]["ODA"]);
            if chain {
                assert!(a["attrs"].get("NO").is_none());
            }
        }
        doc.undo();
        assert_eq!(snap(&doc), before);
    }
}

#[test]
fn a_locked_definition_local_ancestor_refuses_even_without_a_root_instance() {
    let mut v = source();
    let ancestor = "0192f5a1-2900-7000-8000-0000000000c3";
    let mut insert = v["entities"][2].clone();
    insert["id"] = json!(1);
    insert["layerId"] = json!("locked-ancestor");
    let mut layer = v["layers"][1].clone();
    layer["id"] = json!("locked-ancestor");
    layer["name"] = json!("Kilitli üst kullanım");
    layer["locked"] = json!(true);
    v["layers"].as_array_mut().unwrap().push(layer);
    v["blocks"].as_array_mut().unwrap().push(json!({"id":ancestor,"name":"Kilitle korunan üst tanım","base":{"x":0,"y":0},"units":"m","entities":[insert]}));
    let args = input(&v, true);
    let mut doc = open(v);
    let before = snap(&doc);
    for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
        let a = run(&mut doc, op, args.clone());
        assert_eq!(a["error"]["code"], "layer_locked", "{a}");
        assert_eq!(snap(&doc), before);
    }
}
