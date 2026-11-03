//! Shared values must never leak into instance maps or destroy conflicting literal inputs.
use kentos_commands::{CallOp, DocumentSnapshotV2};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};
const BLOCK: &str = "0192f5a1-2900-7000-8000-0000000000c1";
const PARENT: &str = "0192f5a1-2900-7000-8000-0000000000c2";
fn source() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../fixtures/kcad/v2/block-parameter-groups.json"
    ))
    .unwrap()
}
fn open(v: Value) -> Document {
    Document::from_snapshot_v2(serde_json::from_value::<DocumentSnapshotV2>(v).unwrap()).unwrap()
}
fn snap(doc: &Document) -> Value {
    serde_json::to_value(doc.to_snapshot_v2()).unwrap()
}
fn run(doc: &mut Document, op: CallOp, dynamic: Value) -> Value {
    registry::run(
        &mut ExecutionContext::new(doc),
        "cad.blocks.edit",
        op,
        json!({"operation":"dynamic","block":BLOCK,"dynamic":dynamic}),
    )
    .unwrap()
}
fn common(mut d: Value, names: Value, values: Value) -> Value {
    d["typeParameters"] = names;
    d["typeValues"] = values;
    d
}
fn nested(v: &mut Value) {
    let mut i = v["entities"][1].clone();
    i["id"] = json!(1);
    v["blocks"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":PARENT,"name":"Üst blok","base":{"x":0,"y":0},"entities":[i]}));
    v["entities"][1]["block"] = json!(PARENT);
    v["entities"][1]
        .as_object_mut()
        .unwrap()
        .remove("parameters");
}
#[test]
fn conflicting_promotion_refuses_all_phases_without_changing_a_reference() {
    for nested_reference in [false, true] {
        let mut v = source();
        if nested_reference {
            nested(&mut v)
        }
        let mut doc = open(v);
        let before = snap(&doc);
        let revision = doc.revision();
        let d = common(
            before["blocks"][0]["dynamic"].clone(),
            json!(["Genişlik"]),
            json!({"Genişlik":0.9}),
        );
        for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
            let a = run(&mut doc, op, d.clone());
            assert_eq!(a["error"]["code"], "invalid_parameter_value", "{a}");
            assert_eq!(snap(&doc), before);
            assert_eq!(doc.revision(), revision);
        }
    }
}
#[test]
fn common_value_changes_every_root_and_nested_use_and_demotion_materializes_it() {
    let mut v = source();
    nested(&mut v);
    let mut doc = open(v);
    let before = snap(&doc);
    let d = common(
        before["blocks"][0]["dynamic"].clone(),
        json!(["Görünüş"]),
        json!({"Görünüş":"Açılım"}),
    );
    // #3 differs, so reconcile it explicitly before changing scope.
    let uid = doc.uid(kentos_domain::Slot(3)).unwrap().to_string();
    let a = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.blocks.parameters",
        CallOp::Execute,
        json!({"operation":"reset","uids":[uid]}),
    )
    .unwrap();
    assert_eq!(a["status"], "completed", "{a}");
    let baseline = snap(&doc);
    assert_eq!(
        run(&mut doc, CallOp::Execute, d.clone())["status"],
        "completed"
    );
    let mut changed = d;
    changed["typeValues"]["Görünüş"] = json!("Sade");
    assert_eq!(
        run(&mut doc, CallOp::Plan, changed.clone())["status"],
        "completed"
    );
    assert_eq!(
        run(&mut doc, CallOp::Execute, changed.clone())["status"],
        "completed"
    );
    let after = snap(&doc);
    let d = doc.blocks()[0].dynamic.as_ref().unwrap();
    for e in doc.entities().chain(doc.blocks()[1].entities.iter()) {
        if let kentos_commands::Entity::Insert(i) = e
            && i.block.to_string() == BLOCK
        {
            assert!(!i.parameters.contains_key("Görünüş"));
            assert_eq!(
                kentos_native_application::block_params::current(d, &i.parameters)["Görünüş"],
                kentos_commands::ParamValue::Text("Sade".into())
            );
        }
    }
    let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).unwrap();
    assert_eq!(kentos_kcad::decode(&bytes).unwrap(), doc.to_snapshot_v2());
    let mut demoted = changed;
    demoted.as_object_mut().unwrap().remove("typeParameters");
    demoted.as_object_mut().unwrap().remove("typeValues");
    assert_eq!(
        run(&mut doc, CallOp::Execute, demoted)["status"],
        "completed"
    );
    let de = snap(&doc);
    assert_eq!(de["entities"][0]["parameters"]["Görünüş"], "Sade");
    assert_eq!(
        de["blocks"][1]["entities"][0]["parameters"]["Görünüş"],
        "Sade"
    );
    assert_eq!(
        de["blocks"][1]["entities"][0]["parameters"]["Genişlik"],
        json!(1.0)
    );
    doc.undo();
    assert_eq!(snap(&doc), after);
    doc.undo();
    doc.undo();
    assert_eq!(snap(&doc), baseline);
}
#[test]
fn instance_set_lookup_and_named_reset_cannot_change_common_values() {
    let mut v = source();
    for e in v["entities"].as_array_mut().unwrap() {
        e.as_object_mut().unwrap().remove("parameters");
    }
    let d = common(
        v["blocks"][0]["dynamic"].clone(),
        json!(["Genişlik"]),
        json!({"Genişlik":0.9}),
    );
    let mut doc = open(v);
    assert_eq!(run(&mut doc, CallOp::Execute, d)["status"], "completed");
    let before = snap(&doc);
    let uid = doc.uid(kentos_domain::Slot(1)).unwrap().to_string();
    for input in [
        json!({"operation":"set","uids":[uid],"values":{"Genişlik":1.0}}),
        json!({"operation":"set","uids":[uid],"values":{"Tip":"Dar"}}),
        json!({"operation":"reset","uids":[uid],"names":["Genişlik"]}),
        json!({"operation":"reset","uids":[uid],"names":["Tip"]}),
    ] {
        let a = registry::run(
            &mut ExecutionContext::new(&mut doc),
            "cad.blocks.parameters",
            CallOp::Execute,
            input,
        )
        .unwrap();
        assert_eq!(a["error"]["code"], "invalid_parameter_value", "{a}");
        assert_eq!(snap(&doc), before);
    }
    let a = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.blocks.parameters",
        CallOp::Execute,
        json!({"operation":"reset","uids":[uid]}),
    )
    .unwrap();
    assert_eq!(a["status"], "completed", "{a}");
    assert_eq!(snap(&doc), before);
}
#[test]
fn type_formulas_only_read_type_inputs_and_instance_formulas_refresh_after_common_edits() {
    let mut v = source();
    for e in v["entities"].as_array_mut().unwrap() {
        e.as_object_mut().unwrap().remove("parameters");
    }
    let mut d = v["blocks"][0]["dynamic"].clone();
    d["parameters"].as_array_mut().unwrap().push(json!({"kind":"linear","name":"İki kat","from":{"x":0,"y":0},"to":{"x":1.8,"y":0},"rule":{"expression":"@genislik*2"}}));
    let mut doc = open(v);
    let before = snap(&doc);
    let bad = common(d.clone(), json!(["İki kat"]), json!({"İki kat":1.8}));
    let a = run(&mut doc, CallOp::Execute, bad);
    assert_eq!(a["error"]["code"], "invalid_dynamic_block", "{a}");
    assert_eq!(snap(&doc), before);
    let good = common(d.clone(), json!(["Genişlik"]), json!({"Genişlik":0.9}));
    assert_eq!(
        run(&mut doc, CallOp::Execute, good.clone())["status"],
        "completed"
    );
    let mut updated = good;
    updated["typeValues"]["Genişlik"] = json!(1.1);
    assert_eq!(
        run(&mut doc, CallOp::Execute, updated)["status"],
        "completed"
    );
    for e in snap(&doc)["entities"].as_array().unwrap() {
        assert_eq!(e["parameters"]["İki kat"], json!(2.2));
        assert!(e["parameters"].get("Genişlik").is_none());
    }
}
#[test]
fn common_scope_changes_refuse_a_locked_ancestor_placement() {
    let mut v = source();
    nested(&mut v);
    v["layers"][1]["locked"] = json!(true);
    let mut doc = open(v);
    let before = snap(&doc);
    let d = common(
        before["blocks"][0]["dynamic"].clone(),
        json!(["Görünüş"]),
        json!({"Görünüş":"Açılım"}),
    );
    let a = run(&mut doc, CallOp::Execute, d);
    assert_eq!(a["error"]["code"], "layer_locked", "{a}");
    assert_eq!(snap(&doc), before);
}

#[test]
fn common_formula_cache_is_recomputed_once_and_saved_only_on_the_definition() {
    let mut v = source();
    for e in v["entities"].as_array_mut().unwrap() {
        e.as_object_mut().unwrap().remove("parameters");
    }
    let mut d = v["blocks"][0]["dynamic"].clone();
    d["parameters"].as_array_mut().unwrap().push(json!({"kind":"linear","name":"İki kat","from":{"x":0,"y":0},"to":{"x":1.8,"y":0},"rule":{"expression":"@genislik*2"}}));
    let mut doc = open(v);
    assert_eq!(
        run(&mut doc, CallOp::Execute, d.clone())["status"],
        "completed"
    );
    let mut d = common(
        d,
        json!(["Genişlik", "İki kat"]),
        json!({"Genişlik":0.9,"İki kat":1.8}),
    );
    assert_eq!(
        run(&mut doc, CallOp::Execute, d.clone())["status"],
        "completed"
    );
    d["typeValues"]["Genişlik"] = json!(1.1);
    let a = run(&mut doc, CallOp::Execute, d);
    assert_eq!(a["status"], "completed", "{a}");
    let after = snap(&doc);
    assert_eq!(
        after["blocks"][0]["dynamic"]["typeValues"]["İki kat"],
        json!(2.2)
    );
    for e in after["entities"].as_array().unwrap() {
        assert!(e.get("parameters").is_none());
    }
    let revision = doc.revision();
    let dynamic = after["blocks"][0]["dynamic"].clone();
    assert_eq!(
        run(&mut doc, CallOp::Execute, dynamic)["status"],
        "completed"
    );
    assert_eq!(doc.revision(), revision);
}
#[test]
fn mixed_scope_lookup_and_type_values_in_instance_files_are_invalid() {
    let mut v = source();
    let mut d = common(
        v["blocks"][0]["dynamic"].clone(),
        json!(["Genişlik"]),
        json!({"Genişlik":0.9}),
    );
    d["parameters"][3]["rows"][0]["values"]["Yön"] = json!(false);
    v["blocks"][0]["dynamic"] = d;
    let typed: DocumentSnapshotV2 = serde_json::from_value(v).unwrap();
    assert!(Document::from_snapshot_v2(typed).is_err());
    let mut v = source();
    v["blocks"][0]["dynamic"] = common(
        v["blocks"][0]["dynamic"].clone(),
        json!(["Genişlik"]),
        json!({"Genişlik":0.9}),
    );
    let typed: DocumentSnapshotV2 = serde_json::from_value(v).unwrap();
    assert!(Document::from_snapshot_v2(typed).is_err());
}

#[test]
fn a_new_placement_computes_instance_formulas_from_common_inputs_without_explicit_overrides() {
    let v: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/kcad/v2/block-type-values.json"
    ))
    .unwrap();
    let mut doc = open(v);
    let mut d = snap(&doc)["blocks"][0]["dynamic"].clone();
    d["parameters"].as_array_mut().unwrap().push(json!({"kind":"linear","name":"İki kat","from":{"x":0,"y":0},"to":{"x":1.8,"y":0},"rule":{"expression":"@genislik*2"}}));
    assert_eq!(run(&mut doc, CallOp::Execute, d)["status"], "completed");
    let a = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.blocks.insert",
        CallOp::Execute,
        json!({"block":BLOCK,"at":{"x":6.,"y":0.},"layerId":"kapilar"}),
    )
    .unwrap();
    assert_eq!(a["status"], "completed", "{a}");
    let after = snap(&doc);
    assert_eq!(after["entities"][3]["parameters"]["İki kat"], json!(2.2));
    assert!(after["entities"][3]["parameters"].get("Genişlik").is_none());
}
