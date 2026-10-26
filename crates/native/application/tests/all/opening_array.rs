//! Independent analytic expectations for array placement, arc paths and anchors.
use kentos_commands::*;
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, openings_create, openings_edit, walls_create};
use serde_json::{Value, json};

fn done<T: serde::Serialize + std::fmt::Debug>(r: CommandResult<T>) -> Value {
    match r {
        CommandResult::Completed { output, .. } => serde_json::to_value(output).unwrap(),
        other => panic!("{other:?}"),
    }
}
fn setup(arc: bool, at: f64, anchor: OpeningAnchor) -> (Document, String, Slot) {
    let mut data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.walls.rectangle.json"
    ))
    .unwrap();
    data["setup"]["entities"] = json!([]);
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&data["setup"].to_string()).unwrap())
            .unwrap();
    let mut wall = json!({"pts":[{"x":0,"y":0},{"x":20,"y":0}],"width":0.2,"closed":false});
    if arc {
        wall["bulges"] = json!([1.0]);
    }
    let made = done(walls_create::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({"layerId":"duvar","walls":[wall]})).unwrap(),
    ));
    let host = made["uids"][0].as_str().unwrap().to_owned();
    let made = done(openings_create::execute(&mut ExecutionContext::new(&mut doc), serde_json::from_value(json!({"layerId":"duvar","openings":[{"host":host,"at":at,"width":1,"number":"K1","attrs":{"office":"örnek"}}]})).unwrap()));
    let uid = made["uids"][0].as_str().unwrap().to_owned();
    let slot = doc.slot_of(uid.parse().unwrap()).unwrap();
    if anchor != OpeningAnchor::Center {
        let Entity::Opening(mut o) = doc.get(slot).unwrap().clone() else {
            panic!("opening")
        };
        o.opening.anchor = anchor;
        o.opening.at = if anchor == OpeningAnchor::Start {
            at - 0.5
        } else {
            at + 0.5
        };
        doc.update_many(vec![(slot, Entity::Opening(o))], "test anchor");
    }
    (doc, uid, slot)
}
fn input(uid: &str, layout: Value, numbering: Option<Value>) -> OpeningsEdit {
    let mut operation = json!({"kind":"array","layout":layout});
    if let Some(n) = numbering {
        operation["numbering"] = n;
    }
    serde_json::from_value(json!({"uids":[uid],"operation":operation})).unwrap()
}
fn spacing(count: u32, direction: &str) -> Value {
    json!({"kind":"spacing","count":count,"distance":1,"reference":"clear","direction":direction})
}

#[test]
fn arrays_keep_source_and_anchors_and_follow_analytic_arc_length_with_one_undo() {
    for arc in [false, true] {
        for anchor in [
            OpeningAnchor::Start,
            OpeningAnchor::Center,
            OpeningAnchor::End,
        ] {
            for direction in ["start", "end"] {
                let start = if direction == "end" { 1.0 } else { 9.0 };
                let sign = if direction == "end" { 1.0 } else { -1.0 };
                let (mut doc, uid, source) = setup(arc, start, anchor);
                let before = doc.to_snapshot_v2();
                let request = input(
                    &uid,
                    spacing(3, direction),
                    Some(json!({"prefix":"K","start":10,"step":2})),
                );
                done(openings_edit::validate(
                    &ExecutionContext::new(&mut doc),
                    &request,
                ));
                let plan = done(openings_edit::plan(
                    &ExecutionContext::new(&mut doc),
                    &request,
                ));
                assert_eq!(doc.to_snapshot_v2(), before, "validate/plan are read-only");
                assert_eq!(plan["added"].as_array().unwrap().len(), 3);
                let written = done(openings_edit::execute(
                    &mut ExecutionContext::new(&mut doc),
                    request,
                ));
                assert_eq!(
                    doc.get(source),
                    before.entities.iter().find(|e| e.base().id == source.0)
                );
                assert_eq!(
                    doc.journal().last().unwrap().command.as_deref(),
                    Some("cad.openings.edit")
                );
                let mut uids = std::collections::HashSet::new();
                for (i, e) in written["added"].as_array().unwrap().iter().enumerate() {
                    let Entity::Opening(o): Entity = serde_json::from_value(e.clone()).unwrap()
                    else {
                        panic!("opening")
                    };
                    let centre = start + sign * 2.0 * (i + 1) as f64;
                    let at = match anchor {
                        OpeningAnchor::Start => centre - 0.5,
                        OpeningAnchor::End => centre + 0.5,
                        _ => centre,
                    };
                    assert_eq!(o.opening.at, at);
                    assert_eq!(o.opening.anchor, anchor);
                    assert_eq!(
                        o.opening.number.as_deref(),
                        Some(format!("K{}", 10 + 2 * i).as_str())
                    );
                    assert_eq!(
                        o.base.attrs.get("office").map(String::as_str),
                        Some("örnek")
                    );
                    assert!(uids.insert(doc.uid(Slot(o.base.id)).unwrap()));
                    for (j, p) in o.opening.place.iter().enumerate() {
                        let s = centre + if j < 2 { -0.5 } else { 0.5 };
                        let (x, y) = if arc {
                            // Diameter 20, positive semicircle: centre (10,0), radius 10,
                            // angle pi+s/10. Faces lie at radius 9.9 and 10.1.
                            let angle = std::f64::consts::PI + s / 10.0;
                            let radius = if j % 2 == 0 { 9.9 } else { 10.1 };
                            (10.0 + radius * angle.cos(), radius * angle.sin())
                        } else {
                            (s, if j % 2 == 0 { 0.1 } else { -0.1 })
                        };
                        assert!(
                            (p.x - x).abs() < 1e-12 && (p.y - y).abs() < 1e-12,
                            "arc={arc} {p:?} != ({x},{y})"
                        );
                    }
                }
                let after = doc.to_snapshot_v2();
                assert!(doc.undo().is_some());
                assert_eq!(doc.to_snapshot_v2().entities, before.entities);
                assert!(doc.redo().is_some());
                assert_eq!(doc.to_snapshot_v2().entities, after.entities);
                assert_eq!(doc.to_snapshot_v2().uids, after.uids);
            }
        }
    }
}

#[test]
fn fit_count_and_jamb_margins_have_closed_form_centres() {
    for count in [1, 2, 3, 5] {
        let (mut doc, uid, _) = setup(false, 1.0, OpeningAnchor::Center);
        let expected = match count {
            1 => vec![10.5],
            2 => vec![2.5, 18.5],
            3 => vec![2.5, 10.5, 18.5],
            _ => vec![2.5, 6.5, 10.5, 14.5, 18.5],
        };
        let plan = done(openings_edit::plan(
            &ExecutionContext::new(&mut doc),
            &input(
                &uid,
                json!({"kind":"fit","count":count,"startMargin":2,"endMargin":1}),
                None,
            ),
        ));
        let actual: Vec<_> = plan["added"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["at"].as_f64().unwrap())
            .collect();
        assert_eq!(actual, expected);
    }
}

#[test]
fn array_refusals_and_number_conflicts_never_leave_partial_writes() {
    for operation in [
        json!({"kind":"array","layout":{"kind":"spacing","count":10,"distance":1,"direction":"end"}}),
        json!({"kind":"array","layout":{"kind":"spacing","count":3,"distance":1,"direction":"end"},"numbering":{"prefix":"K","start":1,"step":1}}),
        json!({"kind":"array","layout":{"kind":"fit","count":10001,"startMargin":2,"endMargin":1}}),
    ] {
        let (mut doc, uid, _) = setup(false, 1.0, OpeningAnchor::Center);
        let before = doc.to_snapshot_v2();
        let revision = doc.revision();
        let rows = doc.journal().count();
        let request: OpeningsEdit =
            serde_json::from_value(json!({"uids":[uid],"operation":operation})).unwrap();
        assert!(!matches!(
            openings_edit::execute(&mut ExecutionContext::new(&mut doc), request),
            CommandResult::Completed { .. }
        ));
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.revision(), revision);
        assert_eq!(doc.journal().count(), rows);
    }
}

#[test]
fn array_non_finite_distances_are_refused_before_writing() {
    let (mut doc, uid, _) = setup(false, 1.0, OpeningAnchor::Center);
    let before = doc.to_snapshot_v2();
    let mut request = input(&uid, spacing(3, "end"), None);
    if let OpeningsOperation::Array {
        layout: OpeningArrayLayout::Spacing { distance, .. },
        ..
    } = &mut request.operation
    {
        *distance = f64::NAN;
    }
    let result = openings_edit::execute(&mut ExecutionContext::new(&mut doc), request);
    let CommandResult::Failed { error } = result else {
        panic!("non-finite rejected")
    };
    assert_eq!(error.code, "not_finite");
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn existing_layer_locks_and_readonly_object_policy_guard_the_whole_array() {
    for policy in ["locked", "hidden", "attributesOnly"] {
        let (mut doc, uid, _) = setup(false, 1.0, OpeningAnchor::Center);
        match policy {
            "locked" => {
                doc.set_layer_locked("duvar", true);
            }
            "hidden" => doc.set_layer_visible("duvar", false),
            _ => doc.set_object_policy(kentos_domain::ObjectPolicy::AttributesOnly),
        }
        let before = doc.to_snapshot_v2();
        let revision = doc.revision();
        let result = openings_edit::execute(
            &mut ExecutionContext::new(&mut doc),
            input(&uid, spacing(3, "end"), None),
        );
        if policy == "hidden" {
            // Same policy as the original copy: explicit API edits can target a hidden layer.
            assert!(
                matches!(result, CommandResult::Completed { .. }),
                "{result:?}"
            );
            assert!(!doc.layers().get("duvar").unwrap().visible);
            assert_eq!(doc.len(), 5);
        } else {
            assert!(matches!(result, CommandResult::Failed { .. }), "{result:?}");
            if policy == "locked" {
                let CommandResult::Failed { error } = result else {
                    unreachable!()
                };
                assert_eq!(error.code, "layer_locked");
            }
            assert_eq!(doc.to_snapshot_v2(), before, "{policy}");
            assert_eq!(doc.revision(), revision, "{policy}");
        }
    }
}

#[test]
fn decimal_fit_that_exactly_touches_jambs_uses_the_existing_span_tolerance() {
    let (mut doc, uid, _) = setup(false, 1.0, OpeningAnchor::Center);
    let request: OpeningsEdit = serde_json::from_value(
        json!({"uids":[uid],"operation":{"kind":"set","width":0.1,"frame":0.0}}),
    )
    .unwrap();
    done(openings_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        request,
    ));
    let before = doc.to_snapshot_v2();
    // Independent decimal geometry: 20 - 17.8 - 2 = 0.2 m, two 0.1 m copies fit.
    let layout = json!({"kind":"fit","count":2,"startMargin":2.0,"endMargin":17.8});
    let result = done(openings_edit::execute(
        &mut ExecutionContext::new(&mut doc),
        input(&uid, layout, None),
    ));
    let added = result["added"].as_array().unwrap();
    for (e, expected) in added.iter().zip([2.05, 2.15]) {
        assert!((e["at"].as_f64().unwrap() - expected).abs() < 1e-12);
    }
    assert!(doc.undo().is_some());
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
    // A real shortfall, beyond the unchanged 1e-9 m opening tolerance, still refuses all.
    let request = input(
        &uid,
        json!({"kind":"fit","count":2,"startMargin":2.0,"endMargin":17.800001}),
        None,
    );
    let refused = openings_edit::execute(&mut ExecutionContext::new(&mut doc), request);
    assert!(matches!(refused,CommandResult::Failed { ref error } if error.code=="invalid_opening"));
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
}
