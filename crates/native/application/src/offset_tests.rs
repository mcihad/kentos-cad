//! Semantic acceptance of user-toleranced parallels, including readonly plans,
//! persistent source/result trace, atomic refusals and one Undo/Redo.
use super::*;
use kentos_commands::{CallOp, DocumentSnapshotV1};
use kentos_domain::Slot;
use serde_json::{Value, json};
fn drawing() -> Document {
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../fixtures/interaction/v1/objects.kcad"
    ))
    .unwrap();
    raw["entities"] = json!([
        {"kind":"nurbs","id":1,"layerId":"cizim","attrs":{"QA":"source"},"degree":2,"ctrl":[{"x":1.,"y":0.},{"x":1.,"y":1.},{"x":0.,"y":1.}],"knots":[0.,0.,0.,1.,1.,1.],"weights":[1.,std::f64::consts::FRAC_1_SQRT_2,1.],"closed":false},
        {"kind":"line","id":2,"layerId":"cizim","attrs":{},"a":{"x":-2.,"y":-2.},"b":{"x":2.,"y":-2.}}
    ]);
    Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap()
}
fn request(d: &Document) -> Value {
    json!({"uids":[d.uid(Slot(1)).unwrap().to_string()],"by":{"kind":"distance","distance":0.3,"side":{"x":2.,"y":2.}},"approximation":{"tolerance":0.0001}})
}
fn call(d: &mut Document, op: CallOp, input: Value) -> Value {
    crate::registry::run(
        &mut ExecutionContext::new(d),
        "cad.entities.offset",
        op,
        input,
    )
    .unwrap()
}
#[test]
fn bounded_offset_plan_execute_trace_and_undo_are_the_same_operation() {
    let mut d = drawing();
    let before = d.to_snapshot_v2();
    let input = request(&d);
    let plan = call(&mut d, CallOp::Plan, input.clone());
    assert_eq!(plan["status"], "completed", "{plan}");
    assert_eq!(d.to_snapshot_v2(), before);
    let done = call(&mut d, CallOp::Execute, input);
    assert_eq!(done["status"], "completed", "{done}");
    assert_eq!(
        plan["output"]["approximations"],
        done["output"]["approximations"]
    );
    let r = &done["output"]["approximations"][0];
    assert_eq!(r["source"], d.uid(Slot(1)).unwrap().to_string());
    assert_eq!(r["copyIndex"], 0);
    assert!(r["errorBound"].as_f64().unwrap() <= 0.0001);
    assert!(r["vertices"].as_u64().unwrap() > 20);
    assert_eq!(r["crossings"], 0);
    assert_eq!(d.get(Slot(1)), before.entities.first());
    assert_eq!(d.len(), 3);
    let after = d.to_snapshot_v2();
    assert_eq!(d.undo().as_deref(), Some("Ötele"));
    assert_eq!(d.to_snapshot_v2(), before);
    d.redo();
    assert_eq!(d.to_snapshot_v2(), after);
}
#[test]
fn missing_consent_bad_tolerance_and_later_curve_failure_are_atomic() {
    let mut d = drawing();
    let before = d.to_snapshot_v2();
    let mut input = request(&d);
    input.as_object_mut().unwrap().remove("approximation");
    let failed = call(&mut d, CallOp::Execute, input);
    assert_eq!(failed["status"], "failed", "{failed}");
    assert_eq!(failed["error"]["code"], "invalid_tolerance");
    assert_eq!(d.to_snapshot_v2(), before);
    for tol in [0., -0.1, 1e-18] {
        let mut input = request(&d);
        input["approximation"]["tolerance"] = json!(tol);
        input["deleteSource"] = json!(true);
        let failed = call(&mut d, CallOp::Execute, input);
        assert_eq!(failed["status"], "failed", "{failed}");
        assert_eq!(d.to_snapshot_v2(), before);
    }
    let mut input = request(&d);
    input["uids"] = json!([
        d.uid(Slot(2)).unwrap().to_string(),
        d.uid(Slot(1)).unwrap().to_string()
    ]);
    input["by"] = json!({"kind":"distance","distance":1.2,"side":{"x":0.,"y":0.}});
    input["deleteSource"] = json!(true);
    let failed = call(&mut d, CallOp::Execute, input);
    assert_eq!(failed["status"], "failed", "{failed}");
    assert_eq!(d.to_snapshot_v2(), before);
}
#[test]
fn both_sides_reports_the_created_order_and_tolerance_changes_the_plan_fingerprint() {
    let mut d = drawing();
    let mut input = request(&d);
    input["bothSides"] = json!(true);
    let plan = call(&mut d, CallOp::Plan, input.clone());
    assert_eq!(plan["status"], "completed", "{plan}");
    let reports = plan["output"]["approximations"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0]["copyIndex"], 0);
    assert_eq!(reports[1]["copyIndex"], 1);
    let old = plan["output"]["entities"][0]["pts"]
        .as_array()
        .unwrap()
        .len();
    input["approximation"]["tolerance"] = json!(0.00001);
    let tighter = call(&mut d, CallOp::Plan, input);
    assert!(
        tighter["output"]["entities"][0]["pts"]
            .as_array()
            .unwrap()
            .len()
            > old
    );
    // The envelope protocol binds the requested tolerance to the plan.
    let uid = d.uid(Slot(1)).unwrap().to_string();
    let envelope = |tolerance| {
        serde_json::from_value(
            json!({"command":"cad.entities.offset","version":1,"op":"plan","input":{
        "uids":[uid],"by":{"kind":"distance","distance":0.3,"side":{"x":2.,"y":2.}},
        "approximation":{"tolerance":tolerance}}}),
        )
    };
    let a = crate::call::call(&mut d, None, envelope(0.0001).unwrap());
    let b = crate::call::call(&mut d, None, envelope(0.00001).unwrap());
    assert!(a.plan_id.is_some());
    assert_ne!(a.plan_id, b.plan_id);
}

#[test]
fn a_through_point_is_measured_on_the_curve_and_impossible_endpoint_is_refused() {
    let mut d = drawing();
    let before = d.to_snapshot_v2();
    let mut input = request(&d);
    let p = 1.3 * std::f64::consts::FRAC_1_SQRT_2;
    input["by"] = json!({"kind":"through","point":{"x":p,"y":p}});
    let out = call(&mut d, CallOp::Plan, input.clone());
    assert_eq!(out["status"], "completed", "{out}");
    assert!((out["output"]["distances"][0].as_f64().unwrap() - 0.3).abs() < 1e-10);
    assert_eq!(d.to_snapshot_v2(), before);
    input["by"] = json!({"kind":"through","point":{"x":1.3,"y":-1.}});
    let out = call(&mut d, CallOp::Execute, input);
    assert_eq!(out["status"], "failed", "{out}");
    assert_eq!(d.to_snapshot_v2(), before);
}
