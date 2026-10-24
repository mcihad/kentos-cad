//! Independent placement coordinates, not expected values from the planner.
use kentos_commands::*;
use kentos_domain::Document;
use kentos_native_application::{
    ExecutionContext, opening_placement, openings_create, walls_create,
};
use serde_json::{Value, json};

fn done<T: serde::Serialize + std::fmt::Debug>(r: CommandResult<T>) -> Value {
    match r {
        CommandResult::Completed { output, .. } => serde_json::to_value(output).unwrap(),
        other => panic!("{other:?}"),
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
