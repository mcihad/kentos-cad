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

fn arc_drag_drawing() -> Document {
    Document::from_snapshot(
        DocumentSnapshotV1::from_json(include_str!(
            "../../../../../fixtures/interaction/v1/constraint-ellipse-drag.kcad"
        ))
        .unwrap(),
    )
    .unwrap()
}
fn arc_drag_setup(d: &mut Document, fixed_start: bool) {
    let c = part(d, 1, "center");
    let a = part(d, 1, "start");
    let b = part(d, 1, "end");
    let l = part(d, 2, "start");
    let mut rules = json!([{"rule":{"kind":"fix","a":c}},{"rule":{"kind":"distance","a":a,"b":b,"parameter":"kiris"},"value":"4.47213595499958m"},{"rule":{"kind":"coincident","a":b,"b":l}}]);
    if fixed_start {
        rules
            .as_array_mut()
            .unwrap()
            .push(json!({"rule":{"kind":"fix","a":a}}));
    }
    add(d, rules);
}
#[test]
fn an_elliptical_arc_grip_keeps_only_the_requested_endpoint_and_solves_its_other_end_and_follower()
{
    use kentos_native_application::{edit, geometry};
    let mut d = arc_drag_drawing();
    arc_drag_setup(&mut d, false);
    let before = d.to_snapshot_v2();
    let mut target = ellipse(&d).clone();
    target.t0 = 0.3;
    let target_point = (4. * 0.3f64.cos(), 2. * 0.3f64.sin());
    let input: EntitiesEdit=serde_json::from_value(json!({"operation":"grip","changes":[{"kind":"update","uid":d.uid(Slot(1)).unwrap().to_string(),"geometry":geometry::edit_geometry(geometry::shape(&Entity::Ellipse(target))).unwrap()}]})).unwrap();
    let planned = done(edit::plan(&ExecutionContext::new(&mut d), &input));
    assert_eq!(d.to_snapshot_v2(), before);
    done(edit::execute(&mut ExecutionContext::new(&mut d), input));
    assert_eq!(
        planned.changed.len(),
        1,
        "the public plan keeps directly edited objects in input order"
    );
    for e in &planned.changed {
        assert_eq!(d.get(Slot(e.base().id)), Some(e));
    }
    assert_eq!(
        d.uid(Slot(1)).unwrap().to_string(),
        before.uids[0].to_string()
    );
    let e = ellipse(&d);
    assert_eq!(e.base.attrs, before.entities[0].base().attrs);
    let g = kentos_geometry_core::geom::ellipse::EllipseGeom {
        c: kentos_geometry_core::vec2::Vec2::new(e.c.x, e.c.y),
        major: kentos_geometry_core::vec2::Vec2::new(e.major.x, e.major.y),
        ratio: e.ratio,
        t0: e.t0,
        t1: e.t1,
    };
    let start = kentos_geometry_core::geom::ellipse::ellipse_point(&g, e.t0);
    let end = kentos_geometry_core::geom::ellipse::ellipse_point(&g, e.t1);
    assert!((start.x - target_point.0).hypot(start.y - target_point.1) < 1e-8);
    assert!(end.x.hypot(end.y - 2.) > 1e-3);
    let Some(Entity::Line(l)) = d.get(Slot(2)) else {
        panic!()
    };
    assert!((l.a.x - end.x).hypot(l.a.y - end.y) < 1e-8);
    assert_eq!((l.b.x, l.b.y), (-2., 3.));
    let q = done(constraints_query::read(&d, &ConstraintsQuery::default()));
    assert!(q.constraints.iter().all(|c| c.holds));
    assert_eq!(q.diagnosis.dof, 6);
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
fn an_elliptical_arc_grip_conflicting_with_a_fixed_start_preserves_the_document() {
    use kentos_native_application::{edit, geometry};
    let mut d = arc_drag_drawing();
    arc_drag_setup(&mut d, true);
    let before = d.to_snapshot_v2();
    let mut e = ellipse(&d).clone();
    e.t0 = 0.3;
    let input:EntitiesEdit=serde_json::from_value(json!({"operation":"grip","changes":[{"kind":"update","uid":d.uid(Slot(1)).unwrap().to_string(),"geometry":geometry::edit_geometry(geometry::shape(&Entity::Ellipse(e))).unwrap()}]})).unwrap();
    let result = edit::execute(&mut ExecutionContext::new(&mut d), input);
    assert!(
        !matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    assert_eq!(d.to_snapshot_v2(), before);
}

#[test]
fn repeated_settled_preview_frames_leave_the_reused_copy_and_its_journal_unchanged() {
    use kentos_native_application::{edit, geometry};
    let mut d = arc_drag_drawing();
    arc_drag_setup(&mut d, false);
    let before = d.to_snapshot_v2();
    let revision = d.revision();
    let journal = d.journal().count();
    let history = d.undo_labels();
    let mut copy = d.reading_copy();
    let copy_journal = copy.journal().count();
    for t in [0.1, 0.3, 0.5, 0.2, 0.3] {
        let mut e = ellipse(&d).clone();
        e.t0 = t;
        let input=serde_json::from_value(json!({"operation":"grip","changes":[{"kind":"update","uid":d.uid(Slot(1)).unwrap().to_string(),"geometry":geometry::edit_geometry(geometry::shape(&Entity::Ellipse(e))).unwrap()}]})).unwrap();
        let (plan, followers) = edit::preview_update_in(&mut copy, &input);
        let plan = done(plan);
        assert_eq!(plan.changed.len(), 1);
        assert_eq!(followers.updated.len(), 1);
        assert_eq!(copy.to_snapshot_v2(), before);
        assert_eq!(copy.revision(), revision);
        assert_eq!(copy.journal().count(), copy_journal);
        assert!(!copy.can_undo());
        assert!(!copy.is_busy());
    }
    assert_eq!(d.to_snapshot_v2(), before);
    assert_eq!(d.journal().count(), journal);
    assert_eq!(d.undo_labels(), history);
}

#[test]
fn thirty_two_endpoint_placements_at_map_coordinates_remain_valid_after_writeback_and_undo() {
    use kentos_native_application::{edit, geometry};
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/constraint-ellipse-drag.kcad"
    ))
    .unwrap();
    for e in raw["entities"].as_array_mut().unwrap() {
        for key in ["c", "a", "b"] {
            if let Some(p) = e.get_mut(key) {
                p["x"] = json!(p["x"].as_f64().unwrap() + 487000.);
                p["y"] = json!(p["y"].as_f64().unwrap() + 4420000.);
            }
        }
    }
    raw["origin"] = json!({"x":487000.,"y":4420000.});
    let mut d =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    arc_drag_setup(&mut d, false);
    let before = d.to_snapshot_v2();
    for k in 1..=32 {
        let t = k as f64 / 64.;
        let mut e = ellipse(&d).clone();
        e.t0 = t;
        let input=serde_json::from_value(json!({"operation":"grip","changes":[{"kind":"update","uid":d.uid(Slot(1)).unwrap().to_string(),"geometry":geometry::edit_geometry(geometry::shape(&Entity::Ellipse(e))).unwrap()}]})).unwrap();
        done(edit::execute(&mut ExecutionContext::new(&mut d), input));
        let q = done(constraints_query::read(&d, &ConstraintsQuery::default()));
        assert!(
            q.constraints.iter().all(|c| c.holds),
            "placement {t}: {q:?}"
        );
        d.undo().unwrap();
        assert_eq!(d.to_snapshot_v2(), before);
    }
}

#[test]
fn full_ellipse_axis_grips_keep_nonzero_seam_and_followers_across_axis_swaps() {
    use kentos_geometry_core::{Vec2 as V, entity::Entity as CoreEntity, ops::grips::move_grip};
    use kentos_native_application::{edit, geometry};
    for origin in [(0., 0.), (487000., 4420000.)] {
        for index in 1..=4 {
            for length in [1.5, 6.] {
                let mut d = drawing();
                // Prepare a true full ellipse whose seam is deliberately not an axis end.
                let mut raw = d.to_snapshot_v2();
                if let Entity::Ellipse(e) = &mut raw.entities[0] {
                    e.c = Vec2 {
                        x: origin.0,
                        y: origin.1,
                    };
                    e.t0 = 0.6;
                    e.t1 = 0.6;
                }
                if let Entity::Line(l) = &mut raw.entities[1] {
                    l.a = Vec2 {
                        x: origin.0 + 4. * 0.6f64.cos(),
                        y: origin.1 + 2. * 0.6f64.sin(),
                    };
                    l.b = Vec2 {
                        x: origin.0 + 8.,
                        y: origin.1 + 3.,
                    };
                }
                d = Document::from_snapshot_v2(raw).unwrap();
                let center = part(&d, 1, "center");
                let start = part(&d, 1, "start");
                let follower = part(&d, 2, "start");
                add(
                    &mut d,
                    json!([{"rule":{"kind":"fix","a":center}},
                    {"rule":{"kind":"coincident","a":start,"b":follower}}]),
                );
                let before = d.to_snapshot_v2();
                let major = index == 1 || index == 3;
                let sign = if index <= 2 { 1. } else { -1. };
                let target = if major {
                    V::new(origin.0 + sign * length, origin.1)
                } else {
                    V::new(origin.0, origin.1 + sign * length)
                };
                let moved = move_grip(
                    &CoreEntity::new(geometry::shape(d.get(Slot(1)).unwrap())),
                    index,
                    target,
                )
                .unwrap();
                let input: EntitiesEdit =
                    serde_json::from_value(json!({"operation":"grip","changes":[{
                    "kind":"update","uid":d.uid(Slot(1)).unwrap().to_string(),
                    "geometry":geometry::edit_geometry(moved.shape).unwrap()}]}))
                    .unwrap();
                let planned = done(edit::plan(&ExecutionContext::new(&mut d), &input));
                assert_eq!(d.to_snapshot_v2(), before);
                done(edit::execute(&mut ExecutionContext::new(&mut d), input));
                assert_eq!(planned.changed[0], *d.get(Slot(1)).unwrap());
                let (a, b) = if major { (length, 2.) } else { (4., length) };
                let Some(Entity::Line(l)) = d.get(Slot(2)) else {
                    panic!()
                };
                assert!(
                    (l.a.x - (origin.0 + a * 0.6f64.cos()))
                        .hypot(l.a.y - (origin.1 + b * 0.6f64.sin()))
                        < 3e-9,
                    "{index} {length} {l:?}"
                );
                let e = ellipse(&d);
                assert_eq!(e.t0, e.t1);
                assert_eq!(e.base.attrs, before.entities[0].base().attrs);
                let q = done(constraints_query::read(&d, &ConstraintsQuery::default()));
                assert!(q.constraints.iter().all(|c| c.holds));
                assert_eq!(q.diagnosis.dof, 5);
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
        }
    }
}

fn axis_dimension_drawing(origin: (f64, f64)) -> Document {
    let mut raw = drawing().to_snapshot_v2();
    if let Entity::Ellipse(e) = &mut raw.entities[0] {
        e.c = Vec2 {
            x: origin.0,
            y: origin.1,
        };
        e.t0 = 0.6;
        e.t1 = 0.6;
    }
    let p = Vec2 {
        x: origin.0 + 4. * 0.6f64.cos(),
        y: origin.1 + 2. * 0.6f64.sin(),
    };
    if let Entity::Line(l) = &mut raw.entities[1] {
        l.a = p;
        l.b = Vec2 {
            x: origin.0 + 8.,
            y: origin.1 + 3.,
        };
    }
    let mut d = Document::from_snapshot_v2(raw).unwrap();
    let c = part(&d, 1, "center");
    let s = part(&d, 1, "start");
    let f = part(&d, 2, "start");
    let length = (p.x - origin.0).hypot(p.y - origin.1);
    add(
        &mut d,
        json!([{"rule":{"kind":"fix","a":c}},
        {"rule":{"kind":"distance","a":c,"b":s,"parameter":"uzaklik"},"value":format!("{length:.17}m")},
        {"rule":{"kind":"coincident","a":s,"b":f}}]),
    );
    d
}

#[test]
fn named_axis_grips_keep_the_axis_target_and_let_the_other_radius_satisfy_its_dimension() {
    use kentos_geometry_core::ops::grips::{ellipse_axis_after_grip, ellipse_axis_point};
    use kentos_native_application::{edit, geometry};
    for origin in [(0., 0.), (487000., 4420000.)] {
        for (index, delta) in [
            (0, (1.5, 0.)),
            (1, (0., 3.)),
            (2, (-1.5, 0.)),
            (3, (0., -3.)),
        ] {
            let mut d = axis_dimension_drawing(origin);
            let before = d.to_snapshot_v2();
            let uid = d.uid(Slot(1)).unwrap().to_string();
            let at = Vec2 {
                x: origin.0 + delta.0,
                y: origin.1 + delta.1,
            };
            let shape = geometry::shape(d.get(Slot(1)).unwrap());
            let axis =
                ellipse_axis_after_grip(&shape, index, kentos_geometry_core::Vec2::new(at.x, at.y))
                    .unwrap();
            let input:EntitiesEdit=serde_json::from_value(json!({"operation":"grip","changes":[{"kind":"ellipseAxis","uid":uid,"index":index,"at":at}]})).unwrap();
            done(edit::validate(&ExecutionContext::new(&mut d), &input));
            let planned = done(edit::plan(&ExecutionContext::new(&mut d), &input));
            assert_eq!(d.to_snapshot_v2(), before);
            done(edit::execute(&mut ExecutionContext::new(&mut d), input));
            assert_eq!(planned.changed[0], *d.get(Slot(1)).unwrap());
            let point =
                ellipse_axis_point(&geometry::shape(d.get(Slot(1)).unwrap()), axis).unwrap();
            assert!(
                (point.x - at.x).hypot(point.y - at.y) < 3e-9,
                "{index}: {point:?} != {at:?}"
            );
            let Some(Entity::Line(line)) = d.get(Slot(2)) else {
                panic!()
            };
            let old = before.entities[1].clone();
            let Entity::Line(old) = old else { panic!() };
            assert!(
                (line.a.x - old.a.x).hypot(line.a.y - old.a.y) > 0.1,
                "the seam must be free to follow its dimension"
            );
            let q = done(constraints_query::read(&d, &ConstraintsQuery::default()));
            assert!(q.constraints.iter().all(|c| c.holds));
            assert_eq!(q.diagnosis.dof, 4);
            assert_eq!(ellipse(&d).base.attrs, before.entities[0].base().attrs);
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
    }
}

#[test]
fn invalid_named_axis_grips_and_a_fixed_seam_are_atomic() {
    use kentos_native_application::edit;
    let mut d = axis_dimension_drawing((0., 0.));
    let uid = d.uid(Slot(1)).unwrap().to_string();
    for value in [
        json!({"operation":"properties","changes":[{"kind":"ellipseAxis","uid":uid,"index":1,"at":{"x":0,"y":3}}]}),
        json!({"operation":"grip","changes":[{"kind":"ellipseAxis","uid":uid,"index":4,"at":{"x":0,"y":3}}]}),
        json!({"operation":"grip","changes":[{"kind":"ellipseAxis","uid":uid,"index":0,"at":{"x":0,"y":0}}]}),
    ] {
        let before = d.to_snapshot_v2();
        let input: EntitiesEdit = serde_json::from_value(value).unwrap();
        assert!(!matches!(
            edit::execute(&mut ExecutionContext::new(&mut d), input),
            CommandResult::Completed { .. }
        ));
        assert_eq!(d.to_snapshot_v2(), before);
    }
    let query = done(constraints_query::read(&d, &ConstraintsQuery::default()));
    let ids: Vec<_> = query
        .constraints
        .iter()
        .filter(|r| matches!(r.constraint.rule, ConstraintRule::Distance { .. }))
        .map(|r| r.constraint.id)
        .collect();
    done(constraints_edit::execute(
        &mut ExecutionContext::new(&mut d),
        serde_json::from_value(json!({"edit":{"kind":"remove","ids":ids}})).unwrap(),
    ));
    let start = part(&d, 1, "start");
    add(&mut d, json!([{"rule":{"kind":"fix","a":start}}]));
    let before = d.to_snapshot_v2();
    let input:EntitiesEdit=serde_json::from_value(json!({"operation":"grip","changes":[{"kind":"ellipseAxis","uid":uid,"index":1,"at":{"x":0,"y":3}}]})).unwrap();
    let validated = edit::validate(&ExecutionContext::new(&mut d), &input);
    assert!(
        matches!(validated,CommandResult::Failed{ref error} if error.code=="constraint_conflict"),
        "{validated:?}"
    );
    assert_eq!(d.to_snapshot_v2(), before);
    let planned = edit::plan(&ExecutionContext::new(&mut d), &input);
    assert!(
        matches!(planned,CommandResult::Failed{ref error} if error.code=="constraint_conflict"),
        "{planned:?}"
    );
    assert_eq!(d.to_snapshot_v2(), before);
    let written = edit::execute(&mut ExecutionContext::new(&mut d), input);
    assert!(
        matches!(written,CommandResult::Failed{ref error} if error.code=="constraint_conflict"),
        "{written:?}"
    );
    assert_eq!(d.to_snapshot_v2(), before);
}

#[test]
fn named_axis_grips_in_a_changeset_keep_the_same_target_diagnosis_and_undo() {
    use kentos_native_application::{changeset, edit};
    let mut d = axis_dimension_drawing((487000., 4420000.));
    let before = d.to_snapshot_v2();
    let input = json!({"operation":"grip","changes":[{"kind":"ellipseAxis","uid":d.uid(Slot(1)).unwrap().to_string(),"index":1,"at":{"x":487000,"y":4420003}}]});
    let direct: EntitiesEdit = serde_json::from_value(input.clone()).unwrap();
    let planned = done(edit::plan(&ExecutionContext::new(&mut d), &direct));
    done(changeset::execute(
        &mut ExecutionContext::new(&mut d),
        serde_json::from_value(json!({"steps":[{"command":"cad.entities.edit","input":input}]}))
            .unwrap(),
    ));
    assert_eq!(planned.changed[0], *d.get(Slot(1)).unwrap());
    let q = done(constraints_query::read(&d, &ConstraintsQuery::default()));
    assert!(q.constraints.iter().all(|c| c.holds));
    assert_eq!(q.diagnosis.dof, 4);
    d.undo().unwrap();
    assert_eq!(d.to_snapshot_v2(), before);
}
