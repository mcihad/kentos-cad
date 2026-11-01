//! Endpoint-only NURBS constraints through the same document commands as UI/Python/MCP.
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
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/objects.kcad"
    ))
    .unwrap();
    raw["settings"]["workspace"] = json!("cad");
    raw["settings"]["drawingUnit"] = json!("mm");
    raw["origin"] = json!({"x":0,"y":0});
    raw["entities"] = json!([
        {"kind":"nurbs","id":1,"layerId":"cizim","attrs":{"QA":"rational"},"degree":3,"ctrl":[{"x":0,"y":0},{"x":1,"y":3},{"x":3,"y":3},{"x":4,"y":3},{"x":6,"y":0}],"knots":[2,2,2,2,3,8,8,8,8],"weights":[1,2,0.7,3,1],"fit":[{"x":0,"y":0},{"x":6,"y":0}],"fitTolerance":0.00001,"tangents":[{"x":1,"y":2},{"x":1,"y":-2}]},
        {"kind":"line","id":2,"layerId":"cizim","attrs":{},"a":{"x":6,"y":0},"b":{"x":10,"y":0}}
    ]);
    Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap()
}
fn part(d: &Document, id: u32, p: &str) -> Value {
    json!({"uid":d.uid(Slot(id)).unwrap().to_string(),"element":{"kind":"point","place":{"kind":p}}})
}
fn setup(d: &mut Document) {
    let (a, b, l) = (part(d, 1, "start"), part(d, 1, "end"), part(d, 2, "start"));
    done(constraints_edit::execute(&mut ExecutionContext::new(d),serde_json::from_value(json!({"edit":{"kind":"add","constraints":[{"rule":{"kind":"fix","a":a}},{"rule":{"kind":"horizontal","a":a,"b":b}},{"rule":{"kind":"distance","a":a,"b":b,"parameter":"aciklik"},"value":"6m"},{"rule":{"kind":"coincident","a":b,"b":l}}]}})).unwrap()));
}
#[test]
fn rational_nurbs_end_drives_a_follower_in_mm_preserves_source_and_undoes_once() {
    let mut d = drawing();
    setup(&mut d);
    let before = d.to_snapshot_v2();
    let Some(Entity::Nurbs(old)) = d.get(Slot(1)).cloned() else {
        panic!()
    };
    assert!(old.fit.is_some());
    let input = serde_json::from_value(
        json!({"edits":[{"kind":"set","name":"aciklik","expression":"8m"}]}),
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
    let Some(Entity::Nurbs(n)) = d.get(Slot(1)) else {
        panic!()
    };
    assert_eq!(n.ctrl[0], old.ctrl[0]);
    assert_eq!(&n.ctrl[1..4], &old.ctrl[1..4]);
    assert!((n.ctrl[4].x - 8.).abs() < 1e-8);
    assert!(n.ctrl[4].y.abs() < 1e-8);
    assert_eq!(n.knots, old.knots);
    assert_eq!(n.weights, old.weights);
    assert_eq!(n.degree, old.degree);
    assert_eq!(n.base, old.base);
    assert!(n.fit.is_none() && n.fit_tolerance.is_none() && n.tangents.is_none());
    let Some(Entity::Line(l)) = d.get(Slot(2)) else {
        panic!()
    };
    assert!((l.a.x - 8.).abs() < 1e-8);
    assert_eq!(l.b.x, 10.);
    let q = done(constraints_query::read(&d, &ConstraintsQuery::default()));
    assert_eq!(q.diagnosis.dof, 2);
    assert!(q.constraints.iter().all(|c| c.holds));
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
fn locked_nurbs_and_unsupported_endpoint_structure_are_explained_and_atomic() {
    let mut d = drawing();
    setup(&mut d);
    let mut snapshot = d.to_snapshot_v2();
    snapshot.layers[0].locked = true;
    let mut d = Document::from_snapshot_v2(snapshot).unwrap();
    let before = d.to_snapshot_v2();
    let result = parameters_edit::execute(
        &mut ExecutionContext::new(&mut d),
        serde_json::from_value(
            json!({"edits":[{"kind":"set","name":"aciklik","expression":"8m"}]}),
        )
        .unwrap(),
    );
    assert!(!matches!(result, CommandResult::Completed { .. }));
    assert_eq!(d.to_snapshot_v2(), before);
    let mut d = drawing();
    let Entity::Nurbs(mut n) = d.get(Slot(1)).unwrap().clone() else {
        panic!()
    };
    n.closed = true;
    d.update(Slot(1), Entity::Nurbs(n));
    let start = part(&d, 1, "start");
    let before = d.to_snapshot_v2();
    let result = constraints_edit::execute(
        &mut ExecutionContext::new(&mut d),
        serde_json::from_value(
            json!({"edit":{"kind":"add","constraints":[{"rule":{"kind":"fix","a":start}}]}}),
        )
        .unwrap(),
    );
    let CommandResult::Failed { error, .. } = result else {
        panic!("{result:?}")
    };
    assert!(error.message.contains("NURBS"));
    assert_eq!(d.to_snapshot_v2(), before);
}
