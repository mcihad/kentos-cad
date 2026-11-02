//! Derived values must remain valid points/numbers across all references and saves.
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
fn formula(mut d: Value, expression: &str) -> Value {
    d["parameters"].as_array_mut().unwrap().push(json!({"kind":"polar","name":"Uç","from":{"x":10.,"y":20.},"to":{"x":13.,"y":24.},"rule":{"expression":expression}}));
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
fn point(value: &Value, x: f64, y: f64) {
    assert!((value["x"].as_f64().unwrap() - x).abs() < 1e-12, "{value}");
    assert!((value["y"].as_f64().unwrap() - y).abs() < 1e-12, "{value}");
}
#[test]
fn polar_formula_is_a_point_on_the_definitions_direction_and_writes_a_readable_file() {
    let mut doc = open(source());
    let before = snap(&doc);
    let journal = doc.journal().count();
    let dynamic = formula(
        before["blocks"][0]["dynamic"].clone(),
        "@Genişlik * 2 + 0.1",
    );
    for op in [CallOp::Validate, CallOp::Plan] {
        let a = run(&mut doc, op, dynamic.clone());
        assert_eq!(a["status"], "completed", "{a}");
        assert_eq!(snap(&doc), before);
    }
    let a = run(&mut doc, CallOp::Execute, dynamic);
    assert_eq!(a["status"], "completed", "{a}");
    let after = snap(&doc);
    // No overrides on #1: it still needs the formula's 1.9 m result cached.
    point(&after["entities"][0]["parameters"]["Uç"], 11.14, 21.52);
    point(&after["entities"][1]["parameters"]["Uç"], 11.26, 21.68);
    assert_eq!(after["entities"][1]["parameters"]["Genişlik"], json!(1.));
    assert_eq!(after["entities"][1]["parameters"]["Yön"], json!(true));
    assert_eq!(doc.journal().count(), journal + 1);
    let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).unwrap();
    assert_eq!(kentos_kcad::decode(&bytes).unwrap(), doc.to_snapshot_v2());
    doc.undo();
    assert_eq!(snap(&doc), before);
    doc.redo();
    assert_eq!(snap(&doc), after);
}
#[test]
fn unchanged_parent_definitions_refresh_nested_formula_values_in_the_same_step() {
    let mut v = source();
    nested(&mut v);
    let mut doc = open(v);
    let before = snap(&doc);
    let a = run(
        &mut doc,
        CallOp::Execute,
        formula(
            before["blocks"][0]["dynamic"].clone(),
            "@Genişlik * 2 + 0.1",
        ),
    );
    assert_eq!(a["status"], "completed", "{a}");
    assert_eq!(a["output"]["changed"], json!([BLOCK, PARENT]));
    let after = snap(&doc);
    point(
        &after["blocks"][1]["entities"][0]["parameters"]["Uç"],
        11.26,
        21.68,
    );
    assert_eq!(after["entities"][1], before["entities"][1]);
    doc.undo();
    assert_eq!(snap(&doc), before);
}
#[test]
fn a_bad_existing_instance_refuses_the_formula_atomically_instead_of_erasing_its_inputs() {
    let mut doc = open(source());
    let before = snap(&doc);
    let revision = doc.revision();
    // Defaults compute, but #2 has Genişlik=1: a division by zero must not reset its width/flip.
    let dynamic = formula(
        before["blocks"][0]["dynamic"].clone(),
        "1 / abs(@Genişlik - 1)",
    );
    for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
        let a = run(&mut doc, op, dynamic.clone());
        assert_eq!(a["error"]["code"], "invalid_parameter_value", "{a}");
        assert_eq!(snap(&doc), before);
        assert_eq!(doc.revision(), revision);
    }
}
#[test]
fn a_bad_nested_instance_refuses_before_writing_either_definition() {
    let mut v = source();
    nested(&mut v);
    let mut doc = open(v);
    let before = snap(&doc);
    let a = run(
        &mut doc,
        CallOp::Execute,
        formula(
            before["blocks"][0]["dynamic"].clone(),
            "1 / abs(@Genişlik - 1)",
        ),
    );
    assert_eq!(a["error"]["code"], "invalid_parameter_value", "{a}");
    assert_eq!(snap(&doc), before);
}
#[test]
fn negative_or_overflowed_polar_results_are_rejected_as_invalid_definitions() {
    for expression in ["-1", "1e309"] {
        let mut doc = open(source());
        let before = snap(&doc);
        let a = run(
            &mut doc,
            CallOp::Execute,
            formula(before["blocks"][0]["dynamic"].clone(), expression),
        );
        assert_eq!(a["error"]["code"], "invalid_dynamic_block", "{a}");
        assert_eq!(snap(&doc), before);
    }
}
#[test]
fn removing_a_formula_discards_its_cache_and_preserves_literal_inputs_and_group_metadata() {
    let mut v = source();
    nested(&mut v);
    let mut doc = open(v);
    let original = snap(&doc);
    let d = formula(
        original["blocks"][0]["dynamic"].clone(),
        "@Genişlik * 2 + 0.1",
    );
    assert_eq!(
        run(&mut doc, CallOp::Execute, d.clone())["status"],
        "completed"
    );
    let before = snap(&doc);
    let mut next = d;
    next["parameters"].as_array_mut().unwrap().pop();
    let a = run(&mut doc, CallOp::Execute, next);
    assert_eq!(a["status"], "completed", "{a}");
    let after = snap(&doc);
    assert_eq!(after["entities"], original["entities"]);
    assert_eq!(after["blocks"][1], original["blocks"][1]);
    assert_eq!(
        after["blocks"][0]["dynamic"],
        original["blocks"][0]["dynamic"]
    );
    doc.undo();
    assert_eq!(snap(&doc), before);
}
#[test]
fn formula_cache_maintenance_on_locked_placements_retains_every_literal_input() {
    let mut v = source();
    v["layers"][1]["locked"] = json!(true);
    nested(&mut v);
    let mut doc = open(v);
    let before = snap(&doc);
    let a = run(
        &mut doc,
        CallOp::Execute,
        formula(
            before["blocks"][0]["dynamic"].clone(),
            "@Genişlik * 2 + 0.1",
        ),
    );
    assert_eq!(a["status"], "completed", "{a}");
    let after = snap(&doc);
    assert_eq!(
        after["blocks"][1]["entities"][0]["parameters"]["Genişlik"],
        json!(1.)
    );
    point(
        &after["blocks"][1]["entities"][0]["parameters"]["Uç"],
        11.26,
        21.68,
    );
    let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).unwrap();
    assert_eq!(kentos_kcad::decode(&bytes).unwrap(), doc.to_snapshot_v2());
}
#[test]
fn correcting_units_keeps_the_polar_formula_point_and_distance_consistent() {
    let mut doc = open(source());
    let d = formula(
        snap(&doc)["blocks"][0]["dynamic"].clone(),
        "@Genişlik * 2 + 0.1",
    );
    assert_eq!(run(&mut doc, CallOp::Execute, d)["status"], "completed");
    let a = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.blocks.edit",
        CallOp::Execute,
        json!({"operation":"units","block":BLOCK,"units":"mm","rescale":true}),
    )
    .unwrap();
    assert_eq!(a["status"], "completed", "{a}");
    point(
        &snap(&doc)["entities"][1]["parameters"]["Uç"],
        0.01126,
        0.02168,
    );
    let bytes = kentos_kcad::encode(&doc.to_snapshot_v2()).unwrap();
    assert_eq!(kentos_kcad::decode(&bytes).unwrap(), doc.to_snapshot_v2());
}

#[test]
fn changing_a_default_read_by_a_formula_refreshes_default_and_overridden_instances() {
    let mut doc = open(source());
    let dynamic = formula(
        snap(&doc)["blocks"][0]["dynamic"].clone(),
        "@Genişlik * 2 + 0.1",
    );
    assert_eq!(
        run(&mut doc, CallOp::Execute, dynamic.clone())["status"],
        "completed"
    );
    let before = snap(&doc);
    let mut next = dynamic;
    next["parameters"][0]["to"]["x"] = json!(0.95);
    let answer = run(&mut doc, CallOp::Execute, next);
    assert_eq!(answer["status"], "completed", "{answer}");
    let after = snap(&doc);
    point(&after["entities"][0]["parameters"]["Uç"], 11.2, 21.6);
    point(&after["entities"][1]["parameters"]["Uç"], 11.26, 21.68);
    assert_eq!(after["entities"][1]["parameters"]["Genişlik"], json!(1.));
    doc.undo();
    assert_eq!(snap(&doc), before);
}

#[test]
fn calculated_polar_point_drives_real_core_geometry_and_has_no_edit_grip() {
    use kentos_geometry_core::{
        api::json::{FromJson, Json},
        block_dynamic::Dynamic,
        entity::{Entity as CoreEntity, Shape},
    };
    let mut doc = open(source());
    let mut dynamic = formula(
        snap(&doc)["blocks"][0]["dynamic"].clone(),
        "@Genişlik * 2 + 0.1",
    );
    dynamic["actions"] = json!([{"kind":"move","parameter":"Uç","pieces":[4]}]);
    let answer = run(&mut doc, CallOp::Execute, dynamic);
    assert_eq!(answer["status"], "completed", "{answer}");
    let value = snap(&doc);
    let block = &value["blocks"][0];
    let dynamic = Dynamic::from_json(&Json::parse(&block["dynamic"].to_string()).unwrap());
    let insert =
        CoreEntity::from_json(&Json::parse(&value["entities"][1].to_string()).unwrap()).unwrap();
    let Shape::Insert { params, .. } = &insert.shape else {
        panic!("insert")
    };
    let pieces: Vec<_> = block["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| CoreEntity::from_json(&Json::parse(&e.to_string()).unwrap()).unwrap())
        .collect();
    let rendered = dynamic.vary(&pieces, params.as_ref());
    let Shape::Line { a, b, .. } = &rendered[3].shape else {
        panic!("line")
    };
    assert!((a.x + 0.84).abs() < 1e-12 && (b.x + 0.84).abs() < 1e-12);
    assert!((a.y + 2.42).abs() < 1e-12 && (b.y + 2.32).abs() < 1e-12);
    assert!(
        !dynamic
            .grips(params.as_ref(), 0.1)
            .iter()
            .any(|g| g.name == "Uç")
    );
}
