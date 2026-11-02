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

#[test]
fn a_fixed_chord_refusal_explains_the_actual_project_units_and_preserves_the_exact_boundary() {
    use kentos_commands::DrawingUnit;
    for (unit, value, requested, chord, minimum) in [
        (DrawingUnit::M, "1.5m", "1.5 m", "4 m", "2 m"),
        (DrawingUnit::Mm, "1500mm", "1500 mm", "4000 mm", "2000 mm"),
        (DrawingUnit::Cm, "150cm", "150 cm", "400 cm", "200 cm"),
        (
            DrawingUnit::In,
            "60in",
            "60 in",
            "157.48031496063 in",
            "78.740157480315 in",
        ),
        (
            DrawingUnit::Ft,
            "5ft",
            "5 ft",
            "13.123359580052 ft",
            "6.561679790026 ft",
        ),
    ] {
        let mut snap = setup().to_snapshot_v2();
        snap.settings.drawing_unit = Some(unit);
        let mut d = Document::from_snapshot_v2(snap.clone()).unwrap();
        let input = set(value);
        let CommandResult::Failed { error: planned } = parameters_edit::plan_in(&d, &input) else {
            panic!("accepted impossible radius")
        };
        let CommandResult::Failed { error } =
            parameters_edit::execute(&mut ExecutionContext::new(&mut d), input)
        else {
            panic!("accepted impossible radius")
        };
        assert_eq!(error, planned);
        assert_eq!(error.code, "constraint_conflict");
        for words in [
            requested,
            chord,
            minimum,
            "sabit uçları",
            "yarıçap en az",
            "Sabit bağını kaldırın",
        ] {
            assert!(
                error.message.contains(words),
                "{unit:?}: {} lacks {words}",
                error.message
            );
        }
        assert_eq!(error.uids.len(), 3);
        for c in &snap.constraints.constraints {
            assert!(error.uids.contains(&c.id.to_string()));
        }
        assert_eq!(d.to_snapshot_v2(), snap);
        assert_eq!(d.undo(), None, "a rejected edit must not add history");
        // Exactly half the chord is feasible in every unit, including in/ft conversion roundoff.
        done(parameters_edit::execute(
            &mut ExecutionContext::new(&mut d),
            set("2m"),
        ));
        let Entity::Polyline(e) = d.get(Slot(1)).unwrap() else {
            panic!("path")
        };
        assert!(e.bulges.as_ref().unwrap()[0] > 0.0 && e.bulges.as_ref().unwrap()[0] <= 1.0);
        let query = done(constraints_query::read(&d, &ConstraintsQuery::default()));
        assert!(query.constraints.iter().all(|c| c.holds));
        assert!((query.constraints[2].measured.unwrap() - 2.0).abs() < 1e-9);
        let Entity::Polyline(original) = &snap.entities[0] else { panic!("path") };
        assert_eq!(e.pts, original.pts);
        d.undo();
        assert_eq!(d.to_snapshot_v2(), snap);
    }
}

#[test]
fn a_free_endpoint_is_allowed_to_shorten_the_chord_for_a_smaller_radius() {
    let mut d = drawing();
    let a = point(&d, 1, 0);
    let arc = circle(&d);
    add(
        &mut d,
        json!([{"rule":{"kind":"fix","a":a}}, {"rule":{"kind":"radius","a":arc,"parameter":"r1"},"value":"2.5m"}]),
    );
    let before = d.to_snapshot_v2();
    done(parameters_edit::execute(
        &mut ExecutionContext::new(&mut d),
        set("1.5m"),
    ));
    let Entity::Polyline(p) = d.get(Slot(1)).unwrap() else {
        panic!("path")
    };
    assert_eq!(p.pts[0], kentos_commands::Vec2 { x: 0.0, y: 0.0 });
    assert!(p.pts[1].x.hypot(p.pts[1].y) <= 3.0 + 1e-8);
    assert!(
        done(constraints_query::read(&d, &ConstraintsQuery::default()))
            .constraints
            .iter()
            .all(|c| c.holds)
    );
    d.undo();
    assert_eq!(d.to_snapshot_v2(), before);
}

#[test]
fn the_limit_uses_a_holes_closing_vertices_and_distinguishes_diameter_from_radius() {
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/constraint-arc-edge.kcad"
    ))
    .unwrap();
    raw["entities"] = json!([{"kind":"polygon","id":1,"layerId":"cizim","attrs":{},
        "pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}],
        "holes":[{"pts":[{"x":1,"y":1},{"x":4,"y":1},{"x":1,"y":5}],"bulges":[0,0,0.5]}]}]);
    let mut d =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let uid = d.uid(Slot(1)).unwrap().to_string();
    let vertex = |index| json!({"uid":uid,"element":{"kind":"point","place":{"kind":"vertex","part":0,"ring":1,"index":index}}});
    let arc = json!({"uid":uid,"element":{"kind":"arcEdge","part":0,"ring":1,"index":2}});
    add(
        &mut d,
        json!([{"rule":{"kind":"fix","a":vertex(2)}},{"rule":{"kind":"fix","a":vertex(0)}},{"rule":{"kind":"diameter","a":arc,"parameter":"r1"},"value":"5m"}]),
    );
    let before = d.to_snapshot_v2();
    let CommandResult::Failed { error } =
        parameters_edit::execute(&mut ExecutionContext::new(&mut d), set("3m"))
    else {
        panic!("accepted impossible diameter")
    };
    assert!(
        error
            .message
            .contains("sabit uçları 4 m uzaklıkta; çap en az 4 m olmalı"),
        "{}",
        error.message
    );
    assert_eq!(d.to_snapshot_v2(), before);
}
