//! Independent placements, gaps, ambiguity, scope, cooperative stop and the real adoption gate.
use kentos_commands::{
    ArrayLayout, ArraysFind, ArraysFound, CommandResult, DocumentSnapshotV1, Entity,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, arrays_adopt, arrays_find};
use serde_json::{Value, json};

fn drawing(points: &[(f64, f64)]) -> Document {
    let mut raw: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.arrays.adopt.json"
    ))
    .unwrap();
    raw = raw["setup"].clone();
    raw["entities"] = json!(points.iter().enumerate().map(|(k,(x,y))|json!({
        "kind":"insert","id":k+1,"layerId":"a","attrs":{},
        "block":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4001","p":{"x":x,"y":y},"scale":1.,"rotation":0.
    })).collect::<Vec<_>>());
    Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap()
}
fn request(doc: &Document, slot: u32) -> ArraysFind {
    ArraysFind {
        source: doc.uid(Slot(slot)).unwrap().to_string(),
        uids: None,
        tolerance: None,
        limit: None,
        expected_revision: None,
    }
}
fn find(doc: &Document, input: &ArraysFind) -> ArraysFound {
    let before = doc.to_snapshot_v2();
    let result = arrays_find::inspect(doc, input, || false);
    assert_eq!(doc.to_snapshot_v2(), before);
    let CommandResult::Completed { output, .. } = result else {
        panic!("{result:?}")
    };
    output
}

#[test]
fn two_gaps_and_an_interior_seed_adopt_without_replacing_any_root() {
    let mut doc = drawing(&[
        (0., 0.),
        (2., 0.),
        (6., 0.),
        (8., 0.),
        (10., 0.),
        (14., 0.),
        (2.5, 0.5),
    ]);
    let before = doc.to_snapshot_v2();
    let input = request(&doc, 4);
    let found = find(&doc, &input);
    assert!(!found.truncated);
    assert_eq!(found.matches.len(), 1);
    let candidate = &found.matches[0];
    assert_eq!(
        (candidate.present, candidate.missing, candidate.deviation),
        (6, 2, 0.)
    );
    assert_eq!(
        candidate.adoption.layout,
        ArrayLayout::Line {
            count: 8,
            dx: 2.,
            dy: 0.,
            fit: false
        }
    );
    assert!(candidate.adoption.items[2].uid.is_none());
    assert!(candidate.adoption.items[6].uid.is_none());
    assert!(candidate.adoption.items.iter().all(|i| !i.kept));
    let unmatched = doc.uid(Slot(7)).unwrap().to_string();
    assert!(found.eligible.contains(&unmatched));
    assert!(
        !candidate
            .adoption
            .items
            .iter()
            .any(|i| i.uid.is_some_and(|u| u.to_text() == unmatched))
    );
    let saved = kentos_kcad::encode_verified(&before).unwrap();
    let result = arrays_adopt::execute(
        &mut ExecutionContext::new(&mut doc),
        candidate.adoption.clone(),
    );
    assert!(
        matches!(result, CommandResult::Completed { .. }),
        "{result:?}"
    );
    assert_eq!(doc.to_snapshot_v2().entities, before.entities);
    assert_eq!(doc.to_snapshot_v2().uids, before.uids);
    assert_eq!(doc.arrays().len(), 1);
    doc.undo();
    assert_eq!(
        kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap(),
        saved
    );
}

#[test]
fn diagonal_large_coordinates_do_not_use_fp32_or_assume_the_seed_is_the_first_item() {
    let points: Vec<_> = [0, 1, 3, 4, 7]
        .into_iter()
        .map(|k| {
            (
                700_000_000. + k as f64 * 0.03,
                4_000_000_000. + k as f64 * 0.04,
            )
        })
        .collect();
    let doc = drawing(&points);
    let input = request(&doc, 3);
    let result = find(&doc, &input);
    assert!(!result.matches.is_empty(), "{result:?}");
    let best = &result.matches[0];
    assert_eq!(best.present, 5);
    assert_eq!(best.missing, 3);
    let ArrayLayout::Line { dx, dy, .. } = best.adoption.layout else {
        panic!("line")
    };
    assert!(
        (dx - 0.03).abs() < 1e-6 && (dy - 0.04).abs() < 1e-6,
        "{dx},{dy}"
    );
    let plan = arrays_adopt::inspect(&doc, &best.adoption, || false);
    assert!(matches!(plan, CommandResult::Completed { .. }), "{plan:?}");
}

#[test]
fn every_second_place_is_observationally_twice_the_spacing_and_two_items_are_insufficient() {
    let doc = drawing(&[(0., 0.), (4., 0.), (8., 0.), (12., 0.)]);
    let found = find(&doc, &request(&doc, 1));
    assert_eq!(found.matches.len(), 1);
    assert_eq!(found.matches[0].missing, 0);
    assert_eq!(
        found.matches[0].adoption.layout,
        ArrayLayout::Line {
            count: 4,
            dx: 4.,
            dy: 0.,
            fit: false
        }
    );
    let doc = drawing(&[(0., 0.), (2., 0.)]);
    assert!(find(&doc, &request(&doc, 1)).matches.is_empty());
}

#[test]
fn duplicate_coincident_uuids_are_not_collapsed_or_chosen_arbitrarily() {
    let doc = drawing(&[(0., 0.), (2., 0.), (2., 0.), (4., 0.)]);
    assert!(find(&doc, &request(&doc, 1)).matches.is_empty());
}

#[test]
fn crossing_patterns_are_reviewable_exclusive_alternatives_and_limits_are_honest() {
    let doc = drawing(&[(0., 0.), (2., 0.), (4., 0.), (0., 3.), (0., 6.)]);
    let found = find(&doc, &request(&doc, 1));
    assert_eq!(found.matches.len(), 2);
    assert!(found.matches.iter().all(|m| m.overlaps));
    let mut input = request(&doc, 1);
    input.limit = Some(1);
    let CommandResult::Completed { output, warnings } =
        arrays_find::inspect(&doc, &input, || false)
    else {
        panic!("query")
    };
    assert_eq!(output.matches.len(), 1);
    assert!(output.truncated);
    assert_eq!(warnings[0].code, "recognition_truncated");
}

#[test]
fn scope_order_deduplication_empty_scope_and_explicit_template_exclusions() {
    let mut doc = drawing(&[(0., 0.), (2., 0.), (4., 0.), (6., 0.)]);
    let mut changed = doc.get(Slot(4)).unwrap().clone();
    let Entity::Insert(i) = &mut changed else {
        unreachable!()
    };
    i.rotation = 0.2;
    doc.update(Slot(4), changed);
    let input = request(&doc, 1);
    let found = find(&doc, &input);
    assert_eq!(found.matches.len(), 1);
    assert_eq!(found.matches[0].present, 3);
    assert_eq!(
        found.excluded[&doc.uid(Slot(4)).unwrap().to_string()],
        "template_differs"
    );
    let mut input = input;
    let mut ids: Vec<_> = doc
        .entities_with_uids()
        .map(|(u, _)| u.to_string())
        .collect();
    ids.reverse();
    ids.push(ids[0].clone());
    input.uids = Some(ids);
    assert_eq!(find(&doc, &input), found);
    input.uids = Some(vec![]);
    let empty = find(&doc, &input);
    assert_eq!(empty.eligible.len(), 1);
    assert!(empty.matches.is_empty());
}

#[test]
fn invalid_scope_revision_source_and_cancellation_never_write() {
    let doc = drawing(&[(0., 0.), (2., 0.), (4., 0.)]);
    let mut input = request(&doc, 1);
    for (field, code) in [
        ("scope", "recognition_scope_limit"),
        ("revision", "revision_conflict"),
        ("source", "invalid_uid"),
        ("limit", "invalid_limit"),
        ("tolerance", "invalid_tolerance"),
    ] {
        let mut bad = input.clone();
        match field {
            "scope" => bad.uids = Some(vec![input.source.clone(); 2049]),
            "revision" => bad.expected_revision = Some("999".into()),
            "source" => bad.source = "bad".into(),
            "limit" => bad.limit = Some(0),
            _ => bad.tolerance = Some(f64::NAN),
        }
        let before = doc.to_snapshot_v2();
        let result = arrays_find::inspect(&doc, &bad, || false);
        let (CommandResult::Failed { error } | CommandResult::Conflict { error }) = result else {
            panic!("{field}:{result:?}")
        };
        assert_eq!(error.code, code);
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    let mut count = 0;
    let before = doc.to_snapshot_v2();
    let result = arrays_find::inspect(&doc, &input, || {
        count += 1;
        count > 12
    });
    assert!(matches!(result, CommandResult::Cancelled));
    assert_eq!(doc.to_snapshot_v2(), before);
    input.source = doc.uid(Slot(2)).unwrap().to_string();
    assert_eq!(find(&doc, &input).matches[0].present, 3);
}

#[test]
fn an_already_adopted_array_is_never_proposed_again() {
    let mut doc = drawing(&[(0., 0.), (2., 0.), (4., 0.)]);
    let input = request(&doc, 1);
    let found = find(&doc, &input);
    let result = arrays_adopt::execute(
        &mut ExecutionContext::new(&mut doc),
        found.matches[0].adoption.clone(),
    );
    assert!(matches!(result, CommandResult::Completed { .. }));
    let before = doc.to_snapshot_v2();
    let result = arrays_find::inspect(&doc, &input, || false);
    let CommandResult::Failed { error } = result else {
        panic!("{result:?}")
    };
    assert_eq!(error.code, "array_adoption_refused");
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn dense_non_patterns_reach_the_work_budget_and_can_be_stopped_during_geometry_search() {
    let points: Vec<_> = (0..1800)
        .map(|k| {
            let k = k as f64;
            (k * 0.07, k * k * 0.003)
        })
        .collect();
    let doc = drawing(&points);
    let input = request(&doc, 1);
    let before = doc.to_snapshot_v2();
    let CommandResult::Completed { output, warnings } =
        arrays_find::inspect(&doc, &input, || false)
    else {
        panic!("query")
    };
    assert!(output.truncated);
    assert_eq!(output.comparisons, 2_000_000);
    assert_eq!(warnings[0].code, "recognition_truncated");
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut calls = 0;
    let answer = arrays_find::inspect(&doc, &input, || {
        calls += 1;
        calls > 10000
    });
    assert!(matches!(answer, CommandResult::Cancelled));
    assert_eq!(doc.to_snapshot_v2(), before);
}
