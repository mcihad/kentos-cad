//! Independent placement coordinates, not expected values from the planner.
use kentos_commands::*;
use kentos_domain::Document;
use kentos_native_application::{
    ExecutionContext, dimensions_create, opening_placement, openings_create, openings_edit,
    walls_create, walls_edit,
};
use serde_json::{Value, json};

fn done<T: serde::Serialize + std::fmt::Debug>(r: CommandResult<T>) -> Value {
    match r {
        CommandResult::Completed { output, .. } => serde_json::to_value(output).unwrap(),
        other => panic!("{other:?}"),
    }
}

fn opening(doc: &Document, uid: &str) -> OpeningFields {
    let slot = doc.slot_of(uid.parse().unwrap()).unwrap();
    let Entity::Opening(o) = doc.get(slot).unwrap() else {
        panic!("opening")
    };
    o.opening.clone()
}

#[test]
fn newly_placed_openings_keep_their_host_through_width_flip_thickness_split_rehost_and_undo() {
    for reverse in [false, true] {
        for two_jambs in [false, true] {
            let (mut doc, host) = wall(reverse, false);
            let target = done(walls_create::execute(
                &mut ExecutionContext::new(&mut doc),
                serde_json::from_value(json!({"layerId":"duvar","walls":[{"pts":[{"x":0,"y":20},{"x":10,"y":20}],"width":0.3,"closed":false}]})).unwrap(),
            ));
            let target = target["uids"][0].as_str().unwrap().to_owned();
            let input = if two_jambs {
                json!({"span":[{"x":7,"y":0.2},{"x":8.2,"y":0.2}]})
            } else {
                json!({"point":{"x":4,"y":0.3},"distance":2,"from":"end","distanceReference":"hinge","hand":"start","width":1})
            };
            let made = done(openings_create::execute(
                &mut ExecutionContext::new(&mut doc),
                req(&host, input),
            ));
            let uid = made["uids"][0].as_str().unwrap().to_owned();
            let original = opening(&doc, &uid);
            let edit = |doc: &mut Document, operation: Value| {
                done(openings_edit::execute(
                    &mut ExecutionContext::new(doc),
                    serde_json::from_value(json!({"uids":[uid],"operation":operation})).unwrap(),
                ))
            };
            edit(&mut doc, json!({"kind":"set","width":1.4}));
            let wider = opening(&doc, &uid);
            assert!((wider.at - original.at).abs() <= 4.0 * f64::EPSILON * 10.0);
            assert_eq!((wider.width, wider.host.as_str()), (1.4, host.as_str()));
            edit(&mut doc, json!({"kind":"flip","what":"both"}));
            let mirrored = opening(&doc, &uid);
            assert_eq!(mirrored.hand, original.hand.other());
            assert_ne!(mirrored.side, original.side);
            let wall_edit = |doc: &mut Document, operation: Value| {
                done(walls_edit::execute(
                    &mut ExecutionContext::new(doc),
                    serde_json::from_value(json!({"uids":[host],"operation":operation})).unwrap(),
                ))
            };
            wall_edit(&mut doc, json!({"kind":"set","width":0.4}));
            let thick = opening(&doc, &uid);
            assert!((thick.at - original.at).abs() <= 4.0 * f64::EPSILON * 10.0);
            assert_eq!((thick.width, thick.host.as_str()), (1.4, host.as_str()));
            for p in &thick.place {
                assert!((p.y.abs() - 0.2).abs() < 1e-12);
            }
            let before_split = doc.to_snapshot_v2();
            wall_edit(&mut doc, json!({"kind":"split","at":{"x":5,"y":0}}));
            let split = opening(&doc, &uid);
            assert!(doc.wall_line_of(&split.host).is_some());
            assert_eq!(split.width, 1.4);
            // Reparameterising subtracts/adds host coordinates up to 10 m.
            // Bound four f64 rounding steps by those operands, not the small
            // final coordinate after cancellation or a drawing/snap tolerance.
            for (a, b) in split.place.iter().zip(&thick.place) {
                for (a, b) in [(a.x, b.x), (a.y, b.y)] {
                    assert!(
                        (a - b).abs() <= 4.0 * f64::EPSILON * 10.0,
                        "reverse={reverse}, span={two_jambs}: {a} != {b}"
                    );
                }
            }
            assert!(split.broken.is_none());
            assert!(doc.undo().is_some());
            assert_eq!(doc.to_snapshot_v2().entities, before_split.entities);
            assert!(doc.redo().is_some());
            let before_rehost = doc.to_snapshot_v2();
            edit(
                &mut doc,
                json!({"kind":"rehost","to":target,"point":{"x":3,"y":20}}),
            );
            let rehosted = opening(&doc, &uid);
            assert_eq!(
                (rehosted.host.as_str(), rehosted.at, rehosted.width),
                (target.as_str(), 3.0, 1.4)
            );
            assert!(rehosted.broken.is_none());
            for p in &rehosted.place {
                assert!(((p.y - 20.0).abs() - 0.15).abs() < 1e-12);
            }
            assert!(doc.undo().is_some());
            assert_eq!(doc.to_snapshot_v2().entities, before_rehost.entities);
        }
    }
}
fn wall(reverse: bool, arc: bool) -> (Document, String) {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.walls.rectangle.json"
    ))
    .unwrap();
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap(),
    )
    .unwrap();
    let pts = if reverse {
        json!([{"x":10,"y":0},{"x":0,"y":0}])
    } else {
        json!([{"x":0,"y":0},{"x":10,"y":0}])
    };
    let mut w = json!({"layerId":"duvar","walls":[{"pts":pts,"width":0.2,"closed":false}]});
    if arc {
        w["walls"][0]["bulges"] = json!([1]);
    }
    let r = done(walls_create::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(w).unwrap(),
    ));
    (doc, r["uids"][0].as_str().unwrap().into())
}
fn req(host: &str, mut opening: Value) -> OpeningsCreate {
    opening["host"] = json!(host);
    serde_json::from_value(json!({"layerId":"duvar","openings":[opening]})).unwrap()
}
fn planned(doc: &Document, input: &OpeningsCreate) -> OpeningFields {
    let p = done(openings_create::plan_in(doc, input));
    let Entity::Opening(o) = serde_json::from_value(p["openings"][0].clone()).unwrap() else {
        panic!()
    };
    o.opening
}

fn opening_measure(uid: &str, a: Value, b: Value) -> DimensionsCreate {
    serde_json::from_value(json!({"layerId":"duvar","height":0.15,"at":{"x":4,"y":-1},
        "measure":{"kind":"aligned","a":{"kind":"place","uid":uid,"place":a},
        "b":{"kind":"place","uid":uid,"place":b}}}))
    .unwrap()
}

#[test]
fn opening_dimensions_bind_real_jambs_and_one_undo_through_width_move_and_flip() {
    for form in ["door", "window", "opening"] {
        for reverse in [false, true] {
            let (mut doc, host) = wall(reverse, false);
            let made = done(openings_create::execute(
                &mut ExecutionContext::new(&mut doc),
                req(&host, json!({"at":4,"width":1,"form":form})),
            ));
            let uid = made["uids"][0].as_str().unwrap();
            let input = opening_measure(
                uid,
                json!({"kind":"vertex","part":0,"ring":0,"index":0}),
                json!({"kind":"vertex","part":0,"ring":0,"index":2}),
            );
            let before = doc.to_snapshot_v2();
            let _ = done(dimensions_create::plan_in(&doc, &input));
            assert_eq!(doc.to_snapshot_v2(), before);
            let made = done(dimensions_create::execute(
                &mut ExecutionContext::new(&mut doc),
                input,
            ));
            let slot = doc
                .slot_of(made["created"][0].as_str().unwrap().parse().unwrap())
                .unwrap();
            let check = |doc: &Document, start: f64, end: f64| {
                let Entity::Dimension(d) = doc.get(slot).unwrap() else {
                    panic!()
                };
                let (x0, x1, y) = if reverse {
                    (10. - start, 10. - end, -0.1)
                } else {
                    (start, end, 0.1)
                };
                assert_eq!(d.a, Vec2 { x: x0, y });
                assert_eq!(d.b, Vec2 { x: x1, y });
                let refs = d.refs.as_ref().unwrap();
                assert_eq!(refs.a.as_ref().unwrap().uid.to_string(), uid);
                assert_eq!(refs.b.as_ref().unwrap().uid.to_string(), uid);
                assert!(refs.each().all(|(_, r)| r.live()));
            };
            check(&doc, 3.5, 4.5);
            let created = doc.to_snapshot_v2();
            assert_eq!(created.entities[..before.entities.len()], before.entities);
            assert_eq!(created.settings, before.settings);
            doc.undo().unwrap();
            assert_eq!(doc.to_snapshot_v2(), before);
            doc.redo().unwrap();
            assert_eq!(doc.to_snapshot_v2(), created);
            for (operation, start, end) in [
                (json!({"kind":"set","width":2}), 3., 5.),
                (json!({"kind":"move","at":6}), 5., 7.),
                (json!({"kind":"flip","what":"both"}), 5., 7.),
            ] {
                let prior = doc.to_snapshot_v2();
                done(openings_edit::execute(
                    &mut ExecutionContext::new(&mut doc),
                    serde_json::from_value(json!({"uids":[uid],"operation":operation})).unwrap(),
                ));
                check(&doc, start, end);
                let after = doc.to_snapshot_v2();
                assert_eq!(after.settings, prior.settings);
                for i in 0..before.entities.len() - 1 {
                    assert_eq!(after.entities[i], prior.entities[i]);
                }
                doc.undo().unwrap();
                assert_eq!(doc.to_snapshot_v2(), prior);
                doc.redo().unwrap();
                assert_eq!(doc.to_snapshot_v2(), after);
            }
            let prior = doc.to_snapshot_v2();
            done(walls_edit::execute(
                &mut ExecutionContext::new(&mut doc),
                serde_json::from_value(
                    json!({"uids":[host],"operation":{"kind":"set","width":0.4}}),
                )
                .unwrap(),
            ));
            let Entity::Dimension(d) = doc.get(slot).unwrap() else {
                panic!()
            };
            let y = if reverse { -0.2 } else { 0.2 };
            let (x0, x1) = if reverse { (5., 3.) } else { (5., 7.) };
            assert_eq!((d.a, d.b), (Vec2 { x: x0, y }, Vec2 { x: x1, y }));
            let thick = doc.to_snapshot_v2();
            doc.undo().unwrap();
            assert_eq!(doc.to_snapshot_v2(), prior);
            doc.redo().unwrap();
            assert_eq!(doc.to_snapshot_v2(), thick);
            let dimension_before_split = doc.get(slot).unwrap().clone();
            done(walls_edit::execute(
                &mut ExecutionContext::new(&mut doc),
                serde_json::from_value(
                    json!({"uids":[host],"operation":{"kind":"split","at":{"x":2,"y":0}}}),
                )
                .unwrap(),
            ));
            assert_eq!(doc.get(slot).unwrap(), &dimension_before_split);
            let split = doc.to_snapshot_v2();
            doc.undo().unwrap();
            assert_eq!(doc.to_snapshot_v2(), thick);
            doc.redo().unwrap();
            assert_eq!(doc.to_snapshot_v2(), split);
        }
    }
}

#[test]
fn opening_start_end_and_center_dimensions_break_if_host_is_removed() {
    let (mut doc, host) = wall(false, false);
    let made = done(openings_create::execute(
        &mut ExecutionContext::new(&mut doc),
        req(&host, json!({"at":4,"width":2,"form":"window"})),
    ));
    let uid = made["uids"][0].as_str().unwrap();
    let input = opening_measure(uid, json!({"kind":"start"}), json!({"kind":"end"}));
    let result = done(dimensions_create::execute(
        &mut ExecutionContext::new(&mut doc),
        input.clone(),
    ));
    let slot = doc
        .slot_of(result["created"][0].as_str().unwrap().parse().unwrap())
        .unwrap();
    let Entity::Dimension(d) = doc.get(slot).unwrap() else {
        panic!()
    };
    assert_eq!((d.a, d.b), (Vec2 { x: 3., y: 0. }, Vec2 { x: 5., y: 0. }));
    let centered = opening_measure(uid, json!({"kind":"start"}), json!({"kind":"center"}));
    let plan = done(dimensions_create::plan_in(&doc, &centered));
    assert_eq!(plan["entities"][0]["b"], json!({"x":4.,"y":0.}));
    let before = doc.to_snapshot_v2();
    let host_slot = doc.slot_of(host.parse().unwrap()).unwrap();
    assert_eq!(doc.remove(&[host_slot]), 1);
    let Entity::Dimension(d) = doc.get(slot).unwrap() else {
        panic!()
    };
    assert_eq!((d.a, d.b), (Vec2 { x: 3., y: 0. }, Vec2 { x: 5., y: 0. }));
    for (_, r) in d.refs.as_ref().unwrap().each() {
        assert_eq!(r.broken.as_deref(), Some("no_such_place"));
    }
    assert_eq!(opening(&doc, uid).broken, Some(OpeningBreak::HostGone));
    let broken = doc.to_snapshot_v2();
    let CommandResult::Failed { error } =
        dimensions_create::execute(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!()
    };
    assert_eq!(error.code, "no_such_place");
    assert_eq!(doc.to_snapshot_v2(), broken);
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), before);
    doc.redo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), broken);
}
#[test]
fn chosen_wall_end_and_distance_reference_place_the_jamb_centre_or_hinge_exactly() {
    for reverse in [false, true] {
        let (doc, host) = wall(reverse, false);
        let before = doc.to_snapshot_v2();
        for (from, reference, hand, expected) in [
            ("start", "jamb", "start", 2.5),
            ("end", "jamb", "end", 7.5),
            ("start", "center", "end", 2.0),
            ("end", "center", "start", 8.0),
            ("start", "hinge", "start", 2.5),
            ("start", "hinge", "end", 1.5),
            ("end", "hinge", "start", 8.5),
            ("end", "hinge", "end", 7.5),
        ] {
            let input = req(
                &host,
                json!({"point":{"x":4,"y":0.3},"distance":2,"width":1,"from":from,"distanceReference":reference,"hand":hand}),
            );
            let f = planned(&doc, &input);
            assert_eq!(f.at, expected, "{from} {reference} {hand}");
            assert_eq!(f.anchor, OpeningAnchor::Center);
            let m = opening_placement::measure(&doc, &f).unwrap();
            assert_eq!(m.start_clear, expected - 0.5);
            assert_eq!(m.end_clear, 9.5 - expected);
            assert_eq!(m.width, 1.0);
        }
        // Old v1 near-jamb behavior remains the same when the new fields are absent.
        for (x, want) in [(1.0, 2.5), (9.0, 7.5)] {
            let f = planned(
                &doc,
                &req(
                    &host,
                    json!({"point":{"x":x,"y":0.3},"distance":2,"width":1}),
                ),
            );
            assert_eq!(f.at, if reverse { 10.0 - want } else { want });
        }
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}
#[test]
fn two_jamb_points_use_host_arc_length_and_reject_ambiguous_or_zero_width_requests_atomically() {
    let (mut doc, host) = wall(false, false);
    let input = req(
        &host,
        json!({"span":[{"x":2,"y":0.2},{"x":3.2,"y":0.2}],"form":"window"}),
    );
    let f = planned(&doc, &input);
    assert!((f.width - 1.2).abs() < 1e-12);
    assert!((f.at - 2.6).abs() < 1e-12);
    let before = doc.to_snapshot_v2();
    for bad in [
        json!({"width":1}),
        json!({"at":2}),
        json!({"middle":true}),
        json!({"distance":2}),
        json!({"span":[{"x":2,"y":0},{"x":2,"y":0}]}),
    ] {
        let mut r = serde_json::to_value(&input.openings[0]).unwrap();
        for (k, v) in bad.as_object().unwrap() {
            r[k] = v.clone();
        }
        assert!(matches!(
            openings_create::execute(&mut ExecutionContext::new(&mut doc), req(&host, r)),
            CommandResult::Failed { .. }
        ));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    let out = done(openings_create::execute(
        &mut ExecutionContext::new(&mut doc),
        input,
    ));
    let saved = doc.to_snapshot_v2();
    assert_eq!(doc.undo().as_deref(), Some("Pencere"));
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
    assert!(doc.redo().is_some());
    assert_eq!(doc.to_snapshot_v2().entities, saved.entities);
    assert!(!out["uids"][0].as_str().unwrap().is_empty());
    let (doc, host) = wall(false, true);
    // Lower semicircle, radius 5. Points at angles 225° and 270°: arc width 5π/4,
    // not the 3.8268 m chord. Radius offset is deliberately projected away.
    let r = std::f64::consts::FRAC_1_SQRT_2 * 5.0;
    let f = planned(
        &doc,
        &req(&host, json!({"span":[{"x":5.0-r,"y":-r},{"x":5,"y":-5}]})),
    );
    assert!((f.width - 5.0 * std::f64::consts::FRAC_PI_4).abs() < 1e-10);
    assert!((f.at - 5.0 * 3.0 * std::f64::consts::PI / 8.0).abs() < 1e-10);
}

#[test]
fn overlap_corner_crossing_nonfinite_and_endpoint_overflow_refuse_the_entire_batch() {
    let (mut doc, host) = wall(false, false);
    let before = doc.to_snapshot_v2();
    for opening in [
        json!({"span":[{"x":-0.2,"y":0.2},{"x":1,"y":0.2}]}),
        json!({"span":[{"x":9,"y":0.2},{"x":10.2,"y":0.2}]}),
        json!({"point":{"x":2,"y":0},"distanceReference":"center"}),
    ] {
        let mut input = req(&host, json!({"at":5,"width":1}));
        input.openings.extend(req(&host, opening).openings);
        assert!(matches!(
            openings_create::execute(&mut ExecutionContext::new(&mut doc), input),
            CommandResult::Failed { .. }
        ));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    let mut input = req(&host, json!({"span":[{"x":2,"y":0},{"x":3,"y":0}]}));
    input.openings.push(input.openings[0].clone());
    let CommandResult::Failed { error } =
        openings_create::execute(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!()
    };
    assert_eq!(error.code, "openings_overlap");
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut input = req(&host, json!({"span":[{"x":2,"y":0},{"x":3,"y":0}]}));
    input.openings[0].span.as_mut().unwrap()[1].x = f64::NAN;
    let CommandResult::Failed { error } = openings_create::plan_in(&doc, &input) else {
        panic!()
    };
    assert_eq!(error.code, "not_finite");
    let r=done(walls_create::execute(&mut ExecutionContext::new(&mut doc),serde_json::from_value(json!({"layerId":"duvar","walls":[{"pts":[{"x":0,"y":10},{"x":5,"y":10},{"x":5,"y":15}],"width":0.2,"closed":false}]})).unwrap()));
    let corner_host = r["uids"][0].as_str().unwrap();
    let before = doc.to_snapshot_v2();
    let r = openings_create::execute(
        &mut ExecutionContext::new(&mut doc),
        req(corner_host, json!({"span":[{"x":4,"y":10},{"x":5,"y":11}]})),
    );
    let CommandResult::Failed { error } = r else {
        panic!()
    };
    assert_eq!(error.code, "opening_off_wall");
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn a_curved_opening_measures_the_real_chord_then_keeps_its_roles_after_rehost() {
    use std::f64::consts::PI;
    let (mut doc, host) = wall(false, true);
    let made = done(openings_create::execute(
        &mut ExecutionContext::new(&mut doc),
        req(&host, json!({"at":5.*PI/2.,"width":PI,"form":"window"})),
    ));
    let uid = made["uids"][0].as_str().unwrap();
    let made = done(dimensions_create::execute(
        &mut ExecutionContext::new(&mut doc),
        opening_measure(
            uid,
            json!({"kind":"vertex","part":0,"ring":0,"index":0}),
            json!({"kind":"vertex","part":0,"ring":0,"index":2}),
        ),
    ));
    let slot = doc
        .slot_of(made["created"][0].as_str().unwrap().parse().unwrap())
        .unwrap();
    let Entity::Dimension(d) = doc.get(slot).unwrap() else {
        panic!()
    };
    // A radius-5 semicircle; the left face radius is 4.9, half-angle pi/10.
    let dx = 4.9 * (PI / 10.).sin();
    let y = -4.9 * (PI / 10.).cos();
    for (a, b) in [(d.a.x, 5. - dx), (d.a.y, y), (d.b.x, 5. + dx), (d.b.y, y)] {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }
    assert!(((d.b.x - d.a.x) - 2. * dx).abs() < 1e-9);
    assert!(
        (2. * dx - PI).abs() > 0.1,
        "chord is not the nominal path width"
    );
    let target=done(walls_create::execute(&mut ExecutionContext::new(&mut doc),serde_json::from_value(json!({"layerId":"duvar","walls":[{"pts":[{"x":0,"y":20},{"x":10,"y":20}],"width":0.4,"closed":false}]})).unwrap()));
    let prior = doc.to_snapshot_v2();
    done(openings_edit::execute(&mut ExecutionContext::new(&mut doc),serde_json::from_value(json!({"uids":[uid],"operation":{"kind":"rehost","to":target["uids"][0],"point":{"x":4,"y":20}}})).unwrap()));
    let Entity::Dimension(d) = doc.get(slot).unwrap() else {
        panic!()
    };
    for (a, b) in [
        (d.a.x, 4. - PI / 2.),
        (d.a.y, 20.2),
        (d.b.x, 4. + PI / 2.),
        (d.b.y, 20.2),
    ] {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }
    for (_, r) in d.refs.as_ref().unwrap().each() {
        assert_eq!(r.uid.to_string(), uid);
        assert!(r.live());
    }
    let after = doc.to_snapshot_v2();
    doc.undo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), prior);
    doc.redo().unwrap();
    assert_eq!(doc.to_snapshot_v2(), after);
}
