//! CAD-ADV-05 circular path-edge editing through the shared product commands.
use kentos_commands::{
    CommandResult, ConstraintsQuery, DocumentSnapshotV1, Entity, ParametersEdit,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{
    ExecutionContext, constraints_edit, constraints_query, parameters_edit,
};
use serde_json::{Value, json};
fn done<T: std::fmt::Debug>(r: CommandResult<T>) -> T {
    match r {
        CommandResult::Completed { output, .. } => output,
        r => panic!("{r:?}"),
    }
}
fn drawing() -> Document {
    Document::from_snapshot(
        DocumentSnapshotV1::from_json(include_str!(
            "../../../../../fixtures/interaction/v1/constraint-arc-edge.kcad"
        ))
        .unwrap(),
    )
    .unwrap()
}
fn point(d: &Document, slot: u32, index: u32) -> Value {
    json!({"uid":d.uid(Slot(slot)).unwrap().to_string(),"element":{"kind":"point","place":{"kind":"vertex","part":0,"ring":0,"index":index}}})
}
fn circle(d: &Document) -> Value {
    json!({"uid":d.uid(Slot(1)).unwrap().to_string(),"element":{"kind":"arcEdge","part":0,"ring":0,"index":0}})
}
fn add(d: &mut Document, rules: Value) {
    done(constraints_edit::execute(
        &mut ExecutionContext::new(d),
        serde_json::from_value(json!({"edit":{"kind":"add","constraints":rules}})).unwrap(),
    ));
}
fn setup() -> Document {
    constrained(drawing())
}
fn constrained(mut d: Document) -> Document {
    let (a, b, arc) = (point(&d, 1, 0), point(&d, 1, 1), circle(&d));
    add(
        &mut d,
        json!([{"rule":{"kind":"fix","a":a}},{"rule":{"kind":"fix","a":b}},{"rule":{"kind":"radius","a":arc,"parameter":"r1"},"value":"2.5m"}]),
    );
    d
}
fn set(value: &str) -> ParametersEdit {
    serde_json::from_value(json!({"edits":[{"kind":"set","name":"r1","expression":value}]}))
        .unwrap()
}
#[test]
fn radius_plan_execute_kcad_and_single_undo_keep_vertices_ids_and_widths() {
    let mut d = setup();
    let before = d.to_snapshot_v2();
    let input = set("3m");
    let planned = done(parameters_edit::plan_in(&d, &input));
    assert_eq!(d.to_snapshot_v2(), before);
    done(parameters_edit::execute(
        &mut ExecutionContext::new(&mut d),
        input,
    ));
    let after = d.to_snapshot_v2();
    assert_eq!(planned.changed, vec![d.get(Slot(1)).unwrap().clone()]);
    assert_eq!(after.uids, before.uids);
    let (Entity::Polyline(old), Entity::Polyline(new)) = (&before.entities[0], &after.entities[0])
    else {
        panic!("path")
    };
    assert_eq!(old.pts, new.pts);
    assert_eq!(old.widths, new.widths);
    assert_eq!(old.base, new.base);
    assert!((new.bulges.as_ref().unwrap()[0] - (3. - 5_f64.sqrt()) / 2.).abs() < 1e-9);
    let q = done(constraints_query::read(&d, &ConstraintsQuery::default()));
    assert!(q.constraints.iter().all(|r| r.holds));
    assert_eq!(q.diagnosis.dof, 0);
    let bytes = kentos_kcad::encode_verified(&after).unwrap();
    assert_eq!(kentos_kcad::decode(&bytes).unwrap(), after);
    let reopened = Document::from_snapshot_v2(kentos_kcad::decode(&bytes).unwrap()).unwrap();
    assert!(
        done(constraints_query::read(
            &reopened,
            &ConstraintsQuery::default()
        ))
        .constraints
        .iter()
        .all(|r| r.holds)
    );
    d.undo();
    assert_eq!(d.to_snapshot_v2(), before);
}
#[test]
fn too_small_radius_is_atomic_and_a_locked_path_does_not_move() {
    let mut d = setup();
    let before = d.to_snapshot_v2();
    for value in ["1.5m", "0m", "-3m"] {
        assert!(!matches!(
            parameters_edit::execute(&mut ExecutionContext::new(&mut d), set(value)),
            CommandResult::Completed { .. }
        ));
        assert_eq!(d.to_snapshot_v2(), before);
    }
    let mut snap = before.clone();
    snap.layers[0].locked = true;
    let mut locked = Document::from_snapshot_v2(snap.clone()).unwrap();
    assert!(!matches!(
        parameters_edit::execute(&mut ExecutionContext::new(&mut locked), set("3m")),
        CommandResult::Completed { .. }
    ));
    assert_eq!(locked.to_snapshot_v2(), snap);
}
#[test]
fn drawing_sense_and_endpoint_tangency_are_preserved_by_radius_changes() {
    let mut d = setup();
    let a = point(&d, 1, 0);
    let uid = d.uid(Slot(2)).unwrap().to_string();
    let end = json!({"uid":uid,"element":{"kind":"point","place":{"kind":"end"}}});
    add(
        &mut d,
        json!([{"rule":{"kind":"coincident","a":a,"b":end}}]),
    );
    let arc = circle(&d);
    let seg = json!({"uid":uid,"element":{"kind":"segment","part":0,"ring":0,"index":0}});
    add(
        &mut d,
        json!([{"rule":{"kind":"distance","a":{"uid":uid,"element":{"kind":"point","place":{"kind":"start"}}},"b":end,"parameter":"boy"},"value":"2.5m"},{"rule":{"kind":"tangent","a":seg,"b":arc}}]),
    );
    let before = d.to_snapshot_v2();
    done(parameters_edit::execute(
        &mut ExecutionContext::new(&mut d),
        set("3m"),
    ));
    let q = done(constraints_query::read(&d, &ConstraintsQuery::default()));
    assert!(q.constraints.iter().all(|r| r.holds), "{q:?}");
    assert_eq!(q.diagnosis.dof, 0);
    let Entity::Line(l) = d.get(Slot(2)).unwrap() else {
        panic!("line")
    };
    assert!(l.a.x < 0. && l.a.y > 0.);
    assert!((l.b.x * l.b.x + l.b.y * l.b.y).sqrt() < 1e-10);
    d.undo();
    assert_eq!(d.to_snapshot_v2(), before);
}

#[test]
fn the_stored_large_coordinate_tangent_and_length_hold_together_without_changing_tolerance() {
    let mut snapshot = drawing().to_snapshot_v2();
    for e in &mut snapshot.entities {
        match e {
            Entity::Polyline(p) => {
                for v in &mut p.pts {
                    v.x += 487000.;
                    v.y += 4420000.;
                }
            }
            Entity::Line(l) => {
                for v in [&mut l.a, &mut l.b] {
                    v.x += 487000.;
                    v.y += 4420000.;
                }
            }
            _ => {}
        }
    }
    let mut d = constrained(Document::from_snapshot_v2(snapshot).unwrap());
    let a = point(&d, 1, 0);
    let uid = d.uid(Slot(2)).unwrap().to_string();
    let end = json!({"uid":uid,"element":{"kind":"point","place":{"kind":"end"}}});
    add(
        &mut d,
        json!([{"rule":{"kind":"coincident","a":a,"b":end}}]),
    );
    let arc = circle(&d);
    let seg = json!({"uid":uid,"element":{"kind":"segment","part":0,"ring":0,"index":0}});
    add(
        &mut d,
        json!([{"rule":{"kind":"distance","a":{"uid":uid,"element":{"kind":"point","place":{"kind":"start"}}},"b":end,"parameter":"boy"},"value":"2.5m"},{"rule":{"kind":"tangent","a":seg,"b":arc}}]),
    );
    let before = d.to_snapshot_v2();
    done(parameters_edit::execute(
        &mut ExecutionContext::new(&mut d),
        set("3m"),
    ));
    let after = d.to_snapshot_v2();
    let q = done(constraints_query::read(&d, &ConstraintsQuery::default()));
    assert!(q.constraints.iter().all(|r| r.holds), "{q:?}");
    assert_eq!(d.to_snapshot_v2(), after, "query never repairs a shape");
    let bytes = kentos_kcad::encode_verified(&after).unwrap();
    let reopened = Document::from_snapshot_v2(kentos_kcad::decode(&bytes).unwrap()).unwrap();
    assert!(
        done(constraints_query::read(
            &reopened,
            &ConstraintsQuery::default()
        ))
        .constraints
        .iter()
        .all(|r| r.holds)
    );
    d.undo();
    assert_eq!(d.to_snapshot_v2(), before);
}
