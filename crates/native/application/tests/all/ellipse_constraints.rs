//! CAD-ADV-05: true ellipse constraints through the public product commands, file and Undo.
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
    raw["origin"] = json!({"x":0,"y":0});
    raw["entities"] = json!([
        {"kind":"ellipse","id":1,"layerId":"cizim","attrs":{"QA":"analytic"},"c":{"x":0,"y":0},"major":{"x":4,"y":0},"ratio":0.5,"t0":0,"t1":0},
        {"kind":"line","id":2,"layerId":"cizim","attrs":{},"a":{"x":4,"y":0},"b":{"x":7,"y":0}}
    ]);
    Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap()
}
fn part(d: &Document, i: u32, p: &str) -> Value {
    json!({"uid":d.uid(Slot(i)).unwrap().to_string(),"element":{"kind":"point","place":{"kind":p}}})
}
fn add(d: &mut Document, rules: Value) -> ConstraintsEdited {
    done(constraints_edit::execute(
        &mut ExecutionContext::new(d),
        serde_json::from_value(json!({"edit":{"kind":"add","constraints":rules}})).unwrap(),
    ))
}
fn setup(d: &mut Document) {
    let center = part(d, 1, "center");
    let start = part(d, 1, "start");
    let tip = part(d, 2, "start");
    add(
        d,
        json!([
            {"rule":{"kind":"fix","a":center}},
            {"rule":{"kind":"horizontal","a":center,"b":start}},
            {"rule":{"kind":"distance","a":center,"b":start,"parameter":"eksen"},"value":"4"},
            {"rule":{"kind":"coincident","a":start,"b":tip}}
        ]),
    );
}
fn ellipse(d: &Document) -> &EllipseEntity {
    let Some(Entity::Ellipse(e)) = d.get(Slot(1)) else {
        panic!()
    };
    e
}
#[test]
fn changing_ellipse_axis_updates_attached_line_query_and_single_undo() {
    let mut d = drawing();
    setup(&mut d);
    let before = d.to_snapshot_v2();
    let uid = d.uid(Slot(1));
    let input =
        serde_json::from_value(json!({"edits":[{"kind":"set","name":"eksen","expression":"6"}]}))
            .unwrap();
    done(parameters_edit::execute(
        &mut ExecutionContext::new(&mut d),
        input,
    ));
    let e = ellipse(&d);
    assert!((e.major.x - 6.).abs() < 1e-9);
    assert!((e.ratio - 1. / 3.).abs() < 1e-9);
    assert_eq!(e.t0, e.t1);
    let Some(Entity::Line(l)) = d.get(Slot(2)) else {
        panic!()
    };
    assert!((l.a.x - 6.).abs() < 1e-9);
    let q = done(constraints_query::read(&d, &ConstraintsQuery::default()));
    assert!(q.diagnosis.broken.is_empty());
    assert!(q.diagnosis.conflicting.is_empty());
    assert_eq!(d.uid(Slot(1)), uid);
    let snapshot = d.to_snapshot_v2();
    let reopened = Document::from_snapshot_v2(snapshot.clone()).unwrap();
    assert_eq!(reopened.to_snapshot_v2(), snapshot);
    d.undo().unwrap();
    assert_eq!(d.to_snapshot_v2(), before);
}
#[test]
fn conflicting_ellipse_parameter_edit_on_locked_layer_is_atomic() {
    let mut d = drawing();
    setup(&mut d);
    let mut snapshot = d.to_snapshot_v2();
    snapshot.layers[0].locked = true;
    let mut d = Document::from_snapshot_v2(snapshot).unwrap();
    let before = d.to_snapshot_v2();
    let input =
        serde_json::from_value(json!({"edits":[{"kind":"set","name":"eksen","expression":"8"}]}))
            .unwrap();
    let result = parameters_edit::execute(&mut ExecutionContext::new(&mut d), input);
    assert!(
        matches!(
            result,
            CommandResult::Failed { .. } | CommandResult::Conflict { .. }
        ),
        "{result:?}"
    );
    assert_eq!(d.to_snapshot_v2(), before);
}
#[test]
fn an_explicitly_held_ellipse_does_not_move_for_a_new_point_constraint() {
    let mut d = drawing();
    let center = part(&d, 1, "center");
    // Explicit hold is the same solver path the locked layer adds automatically.
    let before = d.to_snapshot_v2();
    let uid = d.uid(Slot(1)).unwrap().to_string();
    let start = part(&d, 2, "start");
    let input=serde_json::from_value(json!({"hold":[uid],"edit":{"kind":"add","constraints":[{"rule":{"kind":"coincident","a":start,"b":center}}]}})).unwrap();
    done(constraints_edit::execute(
        &mut ExecutionContext::new(&mut d),
        input,
    ));
    assert_eq!(d.get(Slot(1)).unwrap(), &before.entities[0]);
}

#[test]
fn quarter_turn_with_a_fixed_axis_and_follower_is_planned_saved_undoable_and_conflict_atomic() {
    for reverse in [false, true] {
        let mut d = drawing();
        let center = part(&d, 1, "center");
        let start = part(&d, 1, "start");
        let tip = part(&d, 2, "start");
        add(
            &mut d,
            json!([
                {"rule":{"kind":"fix","a":center}},
                {"rule":{"kind":"distance","a":center,"b":start,"parameter":"eksen"},"value":"4"},
                {"rule":{"kind":"coincident","a":start,"b":tip}}
            ]),
        );
        let before = d.to_snapshot_v2();
        let (a, b) = if reverse {
            (&start, &center)
        } else {
            (&center, &start)
        };
        let input: ConstraintsEdit = serde_json::from_value(
            json!({"edit":{"kind":"add","constraints":[{"rule":{"kind":"vertical","a":a,"b":b}}]}}),
        )
        .unwrap();
        let planned = done(constraints_edit::plan(
            &ExecutionContext::new(&mut d),
            &input,
        ));
        assert_eq!(d.to_snapshot_v2(), before);
        assert!(planned.diagnosis.conflicting.is_empty());
        let out = done(constraints_edit::execute(
            &mut ExecutionContext::new(&mut d),
            input,
        ));
        assert!(out.diagnosis.conflicting.is_empty());
        let e = ellipse(&d);
        assert!(e.major.x.abs() < 1e-8);
        assert!((e.major.y.abs() - 4.).abs() < 1e-8);
        assert!((e.ratio - 0.5).abs() < 1e-8);
        assert_eq!((e.c.x, e.c.y, e.t0, e.t1), (0., 0., 0., 0.));
        let Some(Entity::Line(l)) = d.get(Slot(2)) else {
            panic!()
        };
        assert!((l.a.x - e.major.x).abs() < 1e-8);
        assert!((l.a.y - e.major.y).abs() < 1e-8);
        let after = d.to_snapshot_v2();
        let q = done(constraints_query::read(&d, &ConstraintsQuery::default()));
        assert_eq!(q.diagnosis.dof, 3);
        assert!(q.diagnosis.conflicting.is_empty());
        let reopened = Document::from_snapshot_v2(after.clone()).unwrap();
        assert_eq!(reopened.to_snapshot_v2(), after);
        let conflict = serde_json::from_value(json!({"edit":{"kind":"add","constraints":[{"rule":{"kind":"horizontal","a":center,"b":start}}]}})).unwrap();
        let rejected = constraints_edit::execute(&mut ExecutionContext::new(&mut d), conflict);
        assert!(
            !matches!(rejected, CommandResult::Completed { .. }),
            "{rejected:?}"
        );
        assert_eq!(d.to_snapshot_v2(), after);
        d.undo().unwrap();
        assert_eq!(d.to_snapshot_v2(), before);
    }
}
