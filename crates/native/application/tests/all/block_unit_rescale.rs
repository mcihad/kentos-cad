//! Scoped block units preserve dynamic semantics and reference-local coordinates.
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
fn snapshot(doc: &Document) -> Value {
    serde_json::to_value(doc.to_snapshot_v2()).unwrap()
}
fn input(to: &str, rescale: bool) -> Value {
    json!({"operation":"units","block":BLOCK,"units":to,"rescale":rescale})
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
fn apply(doc: &mut Document, to: &str) -> Value {
    let answer = run(doc, CallOp::Execute, input(to, true));
    assert_eq!(answer["status"], "completed", "{answer}");
    answer
}
fn nested(v: &mut Value) {
    let insert = v["entities"][1].clone();
    v["blocks"].as_array_mut().unwrap().push(json!({"id":PARENT,"name":"Üst blok", "base":{"x":0,"y":0},"units":"m","entities":[insert]}));
    v["entities"][1]["block"] = json!(PARENT);
    v["entities"][1]
        .as_object_mut()
        .unwrap()
        .remove("parameters");
    v["entities"][1]
        .as_object_mut()
        .unwrap()
        .remove("attributePlaces");
    v["entities"][1].as_object_mut().unwrap().remove("clipping");
}
#[test]
fn geometry_rules_lookup_values_and_reference_values_scale_as_one_undo_step() {
    let mut doc = open(source());
    let before = snapshot(&doc);
    let journal = doc.journal().count();
    for op in [CallOp::Validate, CallOp::Plan] {
        let answer = run(&mut doc, op, input("mm", true));
        assert_eq!(answer["status"], "completed", "{answer}");
        assert_eq!(snapshot(&doc), before);
    }
    apply(&mut doc, "mm");
    let after = snapshot(&doc);
    let block = &after["blocks"][0];
    assert_eq!(block["entities"][0]["b"]["y"], json!(0.0009));
    let width = &block["dynamic"]["parameters"][0];
    assert_eq!(width["to"]["x"], json!(0.0009));
    assert_eq!(
        width["rule"],
        json!({"min":0.0006,"max":0.0012,"step":0.00005})
    );
    assert_eq!(
        block["dynamic"]["parameters"][3]["rows"][0]["values"]["Genişlik"],
        json!(0.0007)
    );
    assert_eq!(after["entities"][1]["parameters"]["Genişlik"], json!(0.001));
    assert_eq!(after["entities"][1]["parameters"]["Yön"], json!(true));
    assert_eq!(after["entities"][1]["p"], before["entities"][1]["p"]);
    assert_eq!(
        block["dynamic"]["parameterGroups"],
        before["blocks"][0]["dynamic"]["parameterGroups"]
    );
    assert_eq!(
        block["card"]["version"],
        json!(before["blocks"][0]["card"]["version"].as_u64().unwrap() + 1)
    );
    assert_eq!(doc.journal().count(), journal + 1);
    doc.undo();
    assert_eq!(snapshot(&doc), before);
    doc.redo();
    assert_eq!(snapshot(&doc), after);
    let encoded = kentos_kcad::encode(&doc.to_snapshot_v2()).unwrap();
    assert_eq!(kentos_kcad::decode(&encoded).unwrap(), doc.to_snapshot_v2());
}
#[test]
fn nested_overrides_and_custom_places_change_without_moving_the_parent() {
    let mut v = source();
    v["blocks"][0]["attributes"] = json!([{"tag":"NO","value":"K1","p":{"x":0.3,"y":0.45},"height":0.15,"rotation":0,"movable":true}]);
    v["entities"][1]["attributePlaces"] = json!({"NO":{"x":0.5,"y":0.6}});
    v["entities"][1]["clipping"] = json!({"boundary":[{"x":0,"y":0},{"x":1,"y":0},{"x":0,"y":1}]});
    v["entities"][0]["attributePlaces"] = json!({"NO":{"x":0.4,"y":0.5}});
    nested(&mut v);
    let mut doc = open(v);
    let before = snapshot(&doc);
    let answer = apply(&mut doc, "mm");
    let after = snapshot(&doc);
    assert_eq!(answer["output"]["changed"], json!([BLOCK, PARENT]));
    let i = &after["blocks"][1]["entities"][0];
    assert_eq!(i["parameters"]["Genişlik"], json!(0.001));
    assert_eq!(i["clipping"]["boundary"][1]["x"], json!(0.001));
    assert_eq!(i["attributePlaces"]["NO"], json!({"x":0.0005,"y":0.0006}));
    assert_eq!(i["p"], before["blocks"][1]["entities"][0]["p"]);
    assert_eq!(i["scale"], before["blocks"][1]["entities"][0]["scale"]);
    assert_eq!(
        after["entities"][0]["attributePlaces"]["NO"],
        json!({"x":0.0004,"y":0.0005})
    );
    assert_eq!(after["entities"][1], before["entities"][1]);
    doc.undo();
    assert_eq!(snapshot(&doc), before);
}
#[test]
fn conjugated_formulas_recompute_root_and_nested_caches() {
    let mut v = source();
    let p = json!({"kind":"linear","name":"İkinci", "from":{"x":0,"y":0}, "to":{"x":1.8,"y":0},"rule":{"expression":"@Genişlik * 2 + 0.1"}});
    v["blocks"][0]["dynamic"]["parameters"]
        .as_array_mut()
        .unwrap()
        .push(p);
    v["entities"][1]["parameters"]["İkinci"] = json!(2.1);
    nested(&mut v);
    let mut doc = open(v);
    apply(&mut doc, "mm");
    let after = snapshot(&doc);
    let formula = after["blocks"][0]["dynamic"]["parameters"][4]["rule"]["expression"]
        .as_str()
        .unwrap();
    assert!(formula.contains("@Genişlik*1000/1"), "{formula}");
    let n = after["blocks"][1]["entities"][0]["parameters"]["İkinci"]
        .as_f64()
        .unwrap();
    assert!((n - 0.0021).abs() < 1e-15, "{n}");
    assert!(
        (after["entities"][0]["parameters"]["İkinci"]
            .as_f64()
            .unwrap()
            - 0.0019)
            .abs()
            < 1e-15
    );
}
#[test]
fn locked_direct_or_ancestor_references_refuse_every_surface_without_writes() {
    for parent in [false, true] {
        let mut v = source();
        if parent {
            nested(&mut v);
        }
        v["layers"][1]["locked"] = json!(true);
        let mut doc = open(v);
        let before = snapshot(&doc);
        let revision = doc.revision();
        for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
            let answer = run(&mut doc, op, input("mm", true));
            assert_eq!(answer["error"]["code"], "layer_locked", "{answer}");
            assert_eq!(snapshot(&doc), before);
            assert_eq!(doc.revision(), revision);
        }
        let answer = run(&mut doc, CallOp::Execute, input("mm", false));
        assert_eq!(answer["status"], "completed", "{answer}");
        assert_eq!(snapshot(&doc)["entities"], before["entities"]);
    }
}
#[test]
fn metadata_only_and_same_unit_leave_dynamic_geometry_and_values_exact() {
    let mut doc = open(source());
    let before = snapshot(&doc);
    let answer = run(&mut doc, CallOp::Execute, input("mm", false));
    assert_eq!(answer["status"], "completed");
    let after = snapshot(&doc);
    assert_eq!(after["entities"], before["entities"]);
    assert_eq!(
        after["blocks"][0]["dynamic"],
        before["blocks"][0]["dynamic"]
    );
    let revision = doc.revision();
    apply(&mut doc, "mm");
    assert_eq!(snapshot(&doc), after);
    assert_eq!(doc.revision(), revision);
}
#[test]
fn overflow_is_refused_atomically() {
    let mut v = source();
    v["blocks"][0]["units"] = json!("mm");
    v["blocks"][0]["entities"][0]["b"]["y"] = json!(1e308);
    let mut doc = open(v);
    let before = snapshot(&doc);
    for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
        let answer = run(&mut doc, op, input("m", true));
        assert_eq!(answer["error"]["code"], "invalid_scale", "{answer}");
        assert_eq!(snapshot(&doc), before);
    }
}

#[test]
fn scaling_a_parent_scales_child_placement_but_keeps_child_values_and_local_places() {
    let mut v = source();
    nested(&mut v);
    // Correct the parent's unit, leaving the referenced child's coordinates alone.
    let mut doc = open(v);
    let before = snapshot(&doc);
    let mut args = input("mm", true);
    args["block"] = json!(PARENT);
    let answer = run(&mut doc, CallOp::Execute, args);
    assert_eq!(answer["status"], "completed", "{answer}");
    let after = snapshot(&doc);
    assert_eq!(after["blocks"][0], before["blocks"][0]);
    let child = &after["blocks"][1]["entities"][0];
    assert_eq!(
        child["parameters"],
        before["blocks"][1]["entities"][0]["parameters"]
    );
    assert_eq!(child["p"]["x"], json!(0.002));
    assert_eq!(child["scale"], json!(0.001));
    assert_eq!(after["entities"], before["entities"]);
}
#[test]
fn unused_unitless_conversion_scales_dynamic_coordinates_and_is_reversible() {
    let mut v = source();
    v["entities"] = json!([]);
    v["uids"] = json!([]);
    v["settings"]["drawingUnit"] = json!("mm");
    let mut doc = open(v);
    let before = snapshot(&doc);
    apply(&mut doc, "unitless");
    let after = snapshot(&doc);
    assert_eq!(
        after["blocks"][0]["dynamic"]["parameters"][0]["to"]["x"],
        json!(900.0)
    );
    doc.undo();
    assert_eq!(snapshot(&doc), before);
}

#[test]
fn unrelated_locked_geometry_stays_unchanged_and_does_not_block_correction() {
    let mut v = source();
    v["layers"][0]["locked"] = json!(true);
    v["entities"].as_array_mut().unwrap().push(json!({"kind":"line","id":4,"layerId":"duvar","attrs":{},"a":{"x":20,"y":20},"b":{"x":21,"y":20}}));
    v["uids"]
        .as_array_mut()
        .unwrap()
        .push(json!("0192f5a1-2900-7000-8000-0000000000d4"));
    let mut doc = open(v);
    let before = snapshot(&doc);
    apply(&mut doc, "mm");
    assert_eq!(snapshot(&doc)["entities"][3], before["entities"][3]);
}
#[test]
fn a_clipping_boundary_below_the_file_contract_limit_refuses_before_writing() {
    let mut v = source();
    v["entities"][0]["clipping"] =
        json!({"boundary":[{"x":0,"y":0},{"x":0.001,"y":0},{"x":0,"y":0.001}]});
    let mut doc = open(v);
    let before = snapshot(&doc);
    for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
        let answer = run(&mut doc, op, input("mm", true));
        assert_eq!(answer["error"]["code"], "invalid_scale", "{answer}");
        assert_eq!(snapshot(&doc), before);
    }
}

#[test]
fn rendered_variants_and_grips_scale_together_for_default_and_overridden_instances() {
    use kentos_geometry_core::{
        api::json::{FromJson, Json},
        block_dynamic::Dynamic,
        entity::{Entity as CoreEntity, Shape, entity_bounds},
    };
    type Rendered = (Vec<[f64; 4]>, Vec<(String, f64, f64)>);
    fn rendered(doc: &Document, index: usize) -> Rendered {
        let v = snapshot(doc);
        let block = &v["blocks"][0];
        let dynamic = Dynamic::from_json(&Json::parse(&block["dynamic"].to_string()).unwrap());
        let insert =
            CoreEntity::from_json(&Json::parse(&v["entities"][index].to_string()).unwrap())
                .unwrap();
        let Shape::Insert {
            params: parameters, ..
        } = &insert.shape
        else {
            panic!("an insert")
        };
        let entities: Vec<_> = block["entities"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| CoreEntity::from_json(&Json::parse(&e.to_string()).unwrap()).unwrap())
            .collect();
        let boxes = dynamic
            .vary(&entities, parameters.as_ref())
            .iter()
            .map(|e| {
                let b = entity_bounds(&e.shape);
                [b.min_x, b.min_y, b.max_x, b.max_y]
            })
            .collect();
        let grips = dynamic
            .grips(parameters.as_ref(), 0.1)
            .into_iter()
            .filter(|g| g.name == "Genişlik")
            .map(|g| (g.name, g.at.x, g.at.y))
            .collect();
        (boxes, grips)
    }
    let mut doc = open(source());
    let before: Vec<_> = (0..3).map(|i| rendered(&doc, i)).collect();
    apply(&mut doc, "mm");
    for (i, (boxes, grips)) in before.iter().enumerate() {
        let (next, next_grips) = rendered(&doc, i);
        assert_eq!(next.len(), boxes.len());
        for (a, b) in boxes.iter().zip(next) {
            for (x, y) in a.iter().zip(b) {
                assert!((x / 1000. - y).abs() < 1e-14);
            }
        }
        assert_eq!(grips.len(), next_grips.len());
        for ((name, x, y), (next_name, nx, ny)) in grips.iter().zip(next_grips) {
            assert_eq!(name, &next_name);
            assert!((x / 1000. - nx).abs() < 1e-14);
            assert!((y / 1000. - ny).abs() < 1e-14);
        }
    }
}
