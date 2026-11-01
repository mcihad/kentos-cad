//! CAD-ADV-04: analytic ellipse extension through the native semantic command.
use kentos_commands::{
    CommandResult, DocumentSnapshotV1, EntitiesTrim, Entity, TrimBoundaries, TrimPick, TrimWindow,
    Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, entities_extend};
use serde_json::json;
use std::f64::consts::{FRAC_PI_2, PI};

fn drawing() -> Document {
    drawing_to(FRAC_PI_2)
}
fn drawing_to(t1: f64) -> Document {
    let mut raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/objects.kcad"
    ))
    .unwrap();
    raw["settings"]["workspace"] = json!("cad");
    raw["origin"] = json!({"x":0,"y":0});
    raw["entities"] = json!([
        {"kind":"ellipse","id":1,"layerId":"cizim","attrs":{"QA":"ellipse"},"c":{"x":0,"y":0},"major":{"x":4,"y":0},"ratio":0.5,"t0":0,"t1":t1},
        {"kind":"line","id":2,"layerId":"cizim","attrs":{},"a":{"x":-2,"y":0},"b":{"x":-2,"y":3}},
        {"kind":"line","id":3,"layerId":"cizim","attrs":{},"a":{"x":-3,"y":0},"b":{"x":-3,"y":3}},
        {"kind":"line","id":4,"layerId":"cizim","attrs":{},"a":{"x":2,"y":-1},"b":{"x":5,"y":-1}}
    ]);
    Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap()
}
fn point(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}
fn input(doc: &Document) -> EntitiesTrim {
    EntitiesTrim {
        picks: vec![TrimPick {
            uid: doc.uid(Slot(1)).unwrap().to_string(),
            at: point(0., 2.),
        }],
        boundaries: Some(TrimBoundaries::Objects {
            uids: [2, 3, 4]
                .map(|i| doc.uid(Slot(i)).unwrap().to_string())
                .to_vec(),
        }),
        ..Default::default()
    }
}
fn check_end(doc: &Document, t0: f64, t1: f64) {
    let Entity::Ellipse(e) = doc.get(Slot(1)).unwrap() else {
        panic!("stays analytic")
    };
    assert!((e.t0 - t0).abs() < 1e-12, "{e:?}");
    assert!((e.t1 - t1).abs() < 1e-12, "{e:?}");
    assert_eq!(e.ratio, 0.5);
    assert_eq!(e.major, point(4., 0.));
}
#[test]
fn ellipse_plan_execute_first_boundary_metadata_and_single_undo() {
    let mut doc = drawing();
    let before = doc.to_snapshot_v2();
    let uid = doc.uid(Slot(1));
    let original = doc.get(Slot(1)).unwrap().base().clone();
    let request = input(&doc);
    let CommandResult::Completed { output: plan, .. } =
        entities_extend::plan(&ExecutionContext::new(&mut doc), &request)
    else {
        panic!("plan")
    };
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(plan.changed.len(), 1);
    let result = entities_extend::execute(&mut ExecutionContext::new(&mut doc), request);
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    check_end(&doc, 0., 2. * PI / 3.);
    assert_eq!(doc.uid(Slot(1)), uid);
    assert_eq!(doc.get(Slot(1)).unwrap().base(), &original);
    assert_eq!(doc.get(Slot(1)).unwrap(), &plan.changed[0]);
    assert_eq!(doc.undo().as_deref(), Some("Uzat"));
    assert_eq!(doc.to_snapshot_v2(), before);
}
#[test]
fn ellipse_fence_and_window_use_the_same_analytic_endpoint() {
    for fence in [true, false] {
        let mut doc = drawing();
        let mut request = input(&doc);
        request.picks.clear();
        if fence {
            request.fence = Some(vec![point(0.5, 1.5), point(0.5, 2.5)]);
        } else {
            request.crossing = Some(TrimWindow {
                min: point(-0.1, 1.9),
                max: point(0.1, 2.1),
            });
        }
        let result = entities_extend::execute(&mut ExecutionContext::new(&mut doc), request);
        assert!(
            matches!(result, CommandResult::Completed { .. }),
            "{result:?}"
        );
        check_end(&doc, 0., 2. * PI / 3.);
    }
}
#[test]
fn ellipse_both_ends_grow_once_and_duplicate_picks_do_not_skip_the_other_end() {
    let mut doc = drawing();
    let mut request = input(&doc);
    let uid = doc.uid(Slot(1)).unwrap().to_string();
    request.picks.extend([
        TrimPick {
            uid: uid.clone(),
            at: point(-2., 3f64.sqrt()),
        },
        TrimPick {
            uid,
            at: point(4., 0.),
        },
    ]);
    let result = entities_extend::execute(&mut ExecutionContext::new(&mut doc), request);
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    check_end(&doc, 11. * PI / 6., 2. * PI / 3.);
}
#[test]
fn a_whole_ellipse_is_refused_without_a_write() {
    let mut doc = drawing_to(0.);
    let before = doc.to_snapshot_v2();
    let request = input(&doc);
    let result = entities_extend::execute(&mut ExecutionContext::new(&mut doc), request);
    assert!(matches!(result, CommandResult::Failed { .. }), "{result:?}");
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn folded_path_marks_the_end_the_core_actually_grew() {
    let mut raw: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/interaction/v1/objects.kcad"
    ))
    .unwrap();
    raw["entities"] = json!([
        {"kind":"polyline","id":1,"layerId":"cizim","attrs":{},"pts":[{"x":0,"y":0},{"x":100,"y":0},{"x":100,"y":1},{"x":1,"y":1}]},
        {"kind":"line","id":2,"layerId":"cizim","attrs":{},"a":{"x":-1,"y":-1},"b":{"x":-1,"y":2}}
    ]);
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let uid = doc.uid(Slot(1)).unwrap().to_string();
    let request = EntitiesTrim {
        picks: vec![
            TrimPick {
                uid: uid.clone(),
                at: point(2., 0.),
            },
            TrimPick {
                uid,
                at: point(0., 0.),
            },
        ],
        boundaries: Some(TrimBoundaries::Objects {
            uids: vec![doc.uid(Slot(2)).unwrap().to_string()],
        }),
        ..Default::default()
    };
    let result = entities_extend::execute(&mut ExecutionContext::new(&mut doc), request);
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    let Entity::Polyline(p) = doc.get(Slot(1)).unwrap() else {
        panic!("path")
    };
    assert_eq!(p.pts.first(), Some(&point(-1., 0.)));
    assert_eq!(p.pts.last(), Some(&point(-1., 1.)));
}
