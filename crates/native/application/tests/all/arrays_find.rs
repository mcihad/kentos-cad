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
        kind: None,
        path: None,
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

fn grid_request(doc: &Document, slot: u32) -> ArraysFind {
    let mut input = request(doc, slot);
    input.kind = Some(kentos_commands::ArrayFindKind::Grid);
    input
}

#[test]
fn grid_with_two_gaps_interior_seed_and_unmatched_block_adopts_metadata_only() {
    let mut ps = Vec::new();
    for row in 0..3 {
        for col in 0..4 {
            if [(0, 2), (2, 0)].contains(&(row, col)) {
                continue;
            }
            ps.push((col as f64 * 2., row as f64 * 3.));
        }
    }
    ps.push((2.25, 0.75));
    let mut doc = drawing(&ps);
    let before = doc.to_snapshot_v2();
    let found = find(&doc, &grid_request(&doc, 6));
    assert!(!found.truncated);
    assert_eq!(found.matches.len(), 1, "{found:?}");
    let p = &found.matches[0];
    assert_eq!((p.present, p.missing), (10, 2));
    assert_eq!(
        p.adoption.layout,
        ArrayLayout::Grid {
            rows: 3,
            cols: 4,
            dx: 2.,
            dy: 3.
        }
    );
    assert_eq!(p.adoption.source, doc.uid(Slot(1)).unwrap().to_string());
    assert!(p.adoption.items[2].uid.is_none() && p.adoption.items[8].uid.is_none());
    assert!(p.adoption.items.iter().all(|i| !i.kept));
    assert_eq!(found.eligible.len(), 11);
    let answer = arrays_adopt::execute(&mut ExecutionContext::new(&mut doc), p.adoption.clone());
    assert!(
        matches!(answer, CommandResult::Completed { .. }),
        "{answer:?}"
    );
    let after = doc.to_snapshot_v2();
    assert_eq!(after.entities, before.entities);
    assert_eq!(after.uids, before.uids);
    assert_eq!(after.blocks, before.blocks);
    assert_eq!(after.settings, before.settings);
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
    doc.redo();
    assert_eq!(doc.to_snapshot_v2(), after);
}

#[test]
fn grid_uses_an_occupied_corner_and_signed_spacing_without_filling_a_missing_origin() {
    let doc = drawing(&[(2., 0.), (4., 0.), (0., 3.), (2., 3.), (4., 3.)]);
    let found = find(&doc, &grid_request(&doc, 4));
    assert_eq!(found.matches.len(), 1, "{found:?}");
    let p = &found.matches[0];
    assert_eq!(p.adoption.source, doc.uid(Slot(2)).unwrap().to_string());
    assert_eq!(
        p.adoption.layout,
        ArrayLayout::Grid {
            rows: 2,
            cols: 3,
            dx: -2.,
            dy: 3.
        }
    );
    assert_eq!((p.present, p.missing), (5, 1));
    assert!(p.adoption.items[2].uid.is_none());
}

#[test]
fn grid_never_invents_half_steps_missing_bounding_corners_or_rotated_axes() {
    let doc = drawing(&[(0., 0.), (4., 0.), (8., 0.), (0., 3.), (4., 3.), (8., 3.)]);
    let found = find(&doc, &grid_request(&doc, 2));
    assert_eq!(found.matches.len(), 1);
    assert_eq!(
        found.matches[0].adoption.layout,
        ArrayLayout::Grid {
            rows: 2,
            cols: 3,
            dx: 4.,
            dy: 3.
        }
    );
    assert_eq!(found.matches[0].missing, 0);
    for ps in [
        vec![(1., 1.), (0., 1.), (2., 1.), (1., 0.), (1., 2.)],
        vec![(0., 0.), (1., 1.), (-1., 1.), (0., 2.)],
        vec![(0., 0.), (2., 0.), (0., 3.)],
        vec![(0., 0.), (2., 0.), (4., 0.), (6., 0.)],
    ] {
        let doc = drawing(&ps);
        assert!(
            find(&doc, &grid_request(&doc, 1)).matches.is_empty(),
            "{ps:?}"
        );
    }
}

#[test]
fn grid_colliding_ids_are_not_arbitrarily_chosen_and_scope_order_is_irrelevant() {
    let doc = drawing(&[(0., 0.), (2., 0.), (0., 3.), (2., 3.), (0., 0.)]);
    assert!(find(&doc, &grid_request(&doc, 1)).matches.is_empty());
    let doc = drawing(&[(0., 0.), (2., 0.), (0., 3.), (2., 3.)]);
    let baseline = find(&doc, &grid_request(&doc, 3));
    let mut input = grid_request(&doc, 3);
    input.uids = Some(
        [4, 3, 2, 1, 4]
            .map(|i| doc.uid(Slot(i)).unwrap().to_string())
            .to_vec(),
    );
    assert_eq!(find(&doc, &input), baseline);
    let mut calls = 0;
    let answer = arrays_find::inspect(&doc, &input, || {
        calls += 1;
        calls == 17
    });
    assert!(matches!(answer, CommandResult::Cancelled), "{answer:?}");
}

#[test]
fn grid_small_spacing_at_map_coordinates_preserves_the_shared_adoption_gate() {
    let ps: Vec<_> = (0..3)
        .flat_map(|r| {
            (0..4).filter_map(move |c| {
                if (r, c) == (1, 2) {
                    None
                } else {
                    Some((487_000. + c as f64 * 0.02, 4_420_000. + r as f64 * 0.03))
                }
            })
        })
        .collect();
    let doc = drawing(&ps);
    let found = find(&doc, &grid_request(&doc, 6));
    assert!(!found.matches.is_empty(), "{found:?}");
    let p = &found.matches[0];
    assert_eq!((p.present, p.missing), (11, 1));
    let ArrayLayout::Grid { rows, cols, dx, dy } = p.adoption.layout else {
        panic!("grid")
    };
    assert_eq!((rows, cols), (3, 4));
    assert!((dx - 0.02).abs() < 1e-9 && (dy - 0.03).abs() < 1e-9);
    assert!(matches!(
        arrays_adopt::inspect(&doc, &p.adoption, || false),
        CommandResult::Completed { .. }
    ));
}

#[test]
fn grid_interval_discovery_bounds_are_partial_and_remain_cancellable() {
    let ps: Vec<_> = (0..140).map(|i| ((i * i) as f64, 0.)).collect();
    let doc = drawing(&ps);
    let input = grid_request(&doc, 1);
    let result = arrays_find::inspect(&doc, &input, || false);
    let CommandResult::Completed { output, warnings } = result else {
        panic!("{result:?}")
    };
    assert!(output.truncated);
    assert!(output.comparisons <= 2_000_000);
    assert_eq!(warnings[0].code, "recognition_truncated");
    let before = doc.to_snapshot_v2();
    let mut count = 0;
    let result = arrays_find::inspect(&doc, &input, || {
        count += 1;
        count > 400
    });
    assert!(matches!(result, CommandResult::Cancelled));
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn a_valid_millimetre_line_is_still_found_when_an_earlier_float_spacing_proposal_is_rejected() {
    for step in [0.003, 0.03] {
        let doc = drawing(
            &(0..8)
                .map(|k| (4_000_000_000. + k as f64 * step, 1_000_000.))
                .collect::<Vec<_>>(),
        );
        let input = request(&doc, 5);
        let known = kentos_commands::ArraysAdopt {
            source: doc.uid(Slot(1)).unwrap().to_string(),
            layout: ArrayLayout::Line {
                count: 8,
                dx: step,
                dy: 0.,
                fit: false,
            },
            items: (1..=8)
                .map(|k| kentos_commands::ArrayItem {
                    uid: Some(kentos_commands::EntityId(
                        doc.uid(Slot(k)).unwrap().into_bytes(),
                    )),
                    kept: false,
                })
                .collect(),
            tolerance: Some(1e-6),
            expected_revision: None,
        };
        assert!(
            matches!(
                arrays_adopt::inspect(&doc, &known, || false),
                CommandResult::Completed { .. }
            ),
            "the independent exact layout must be valid"
        );
        let found = find(&doc, &input);
        assert!(
            found.matches.iter().any(|p| p.present == 8),
            "{step}: {found:?}"
        );
    }
}

#[test]
fn a_valid_millimetre_grid_survives_rejected_rounded_interval_proposals() {
    for dy in [0.004, 0.3] {
        let points = (0..3)
            .flat_map(|r| {
                (0..4).map(move |c| {
                    (
                        4_000_000_000. + c as f64 * 0.03,
                        4_000_000_000. + r as f64 * dy,
                    )
                })
            })
            .collect::<Vec<_>>();
        let doc = drawing(&points);
        let known = kentos_commands::ArraysAdopt {
            source: doc.uid(Slot(1)).unwrap().to_string(),
            layout: ArrayLayout::Grid {
                rows: 3,
                cols: 4,
                dx: 0.03,
                dy,
            },
            items: (1..=12)
                .map(|k| kentos_commands::ArrayItem {
                    uid: Some(kentos_commands::EntityId(
                        doc.uid(Slot(k)).unwrap().into_bytes(),
                    )),
                    kept: false,
                })
                .collect(),
            tolerance: Some(1e-6),
            expected_revision: None,
        };
        assert!(matches!(
            arrays_adopt::inspect(&doc, &known, || false),
            CommandResult::Completed { .. }
        ));
        let found = find(&doc, &grid_request(&doc, 6));
        assert!(
            found.matches.iter().any(|p| p.present == 12),
            "dy={dy}: {found:?}"
        );
    }
}

#[test]
fn full_span_refinement_rechecks_the_line_membership_and_recovers_the_eleventh_endpoint() {
    let doc = drawing(
        &(0..11)
            .map(|k| (4_000_000_000. + k as f64 * 0.003, 1_000_000.))
            .collect::<Vec<_>>(),
    );
    let found = find(&doc, &request(&doc, 6));
    assert_eq!(found.matches.len(), 1, "{found:?}");
    assert_eq!(
        (found.matches[0].present, found.matches[0].missing),
        (11, 0)
    );
    assert_eq!(
        found.matches[0].adoption.source,
        doc.uid(Slot(1)).unwrap().to_string()
    );
    assert!(matches!(
        arrays_adopt::inspect(&doc, &found.matches[0].adoption, || false),
        CommandResult::Completed { .. }
    ));
}

fn polar_drawing(center: (f64,f64), count: u32, missing: &[usize], rotate: bool) -> (Document, Vec<usize>) {
    use kentos_geometry_core::{Vec2,ops::assoc_array::{Layout,Placement,item_placements}};
    let source=Placement {p:Vec2::new(center.0+8.,center.1),scale:1.,rotation:0.3,mirror:false};
    let poses=item_placements(source,&Layout::Polar {center:Vec2::new(center.0,center.1),count,fill:360.,rotate}).unwrap();
    let occupied:Vec<_>=(0..count as usize).filter(|k|!missing.contains(k)).collect();
    let mut doc=drawing(&occupied.iter().map(|&k|(poses[k].p.x,poses[k].p.y)).collect::<Vec<_>>());
    for (at,&k) in occupied.iter().enumerate() {
        let slot=Slot(at as u32+1);
        let mut e=doc.get(slot).unwrap().clone();
        let Entity::Insert(i)=&mut e else {unreachable!()};
        i.rotation=poses[k].rotation;
        doc.update(slot,e);
    }
    (doc,occupied)
}
fn polar_request(doc:&Document,slot:u32)->ArraysFind {
    let mut input=request(doc,slot);
    input.kind=Some(kentos_commands::ArrayFindKind::Polar);
    input
}

#[test]
fn full_circle_gaps_and_an_interior_source_retain_fixed_and_rotating_roots() {
    for rotate in [false,true] {
        for seed in [1,4] {
            let (mut doc,_)=polar_drawing((20.,-15.),8,&[2,6],rotate);
            let before=doc.to_snapshot_v2();
            let bytes=kentos_kcad::encode_verified(&before).unwrap();
            let input=polar_request(&doc,seed);
            let found=find(&doc,&input);
            assert!(!found.truncated);
            assert_eq!(found.matches.len(),1,"{found:?}");
            let candidate=&found.matches[0];
            assert_eq!((candidate.present,candidate.missing),(6,2));
            assert_eq!(candidate.adoption.source,input.source);
            let ArrayLayout::Polar {center,count,fill,rotate:r}=candidate.adoption.layout else {panic!()};
            assert_eq!((count,fill,r),(8,360.,rotate));
            assert!((center.x-20.).hypot(center.y+15.)<1e-8);
            assert!(candidate.adoption.items.iter().all(|i|!i.kept));
            let applied=arrays_adopt::execute(&mut ExecutionContext::new(&mut doc),candidate.adoption.clone());
            assert!(matches!(applied,CommandResult::Completed {..}),"{applied:?}");
            let after=doc.to_snapshot_v2();
            assert_eq!(after.entities,before.entities);
            assert_eq!(after.uids,before.uids);
            assert_eq!(after.blocks,before.blocks);
            doc.undo();
            assert_eq!(kentos_kcad::encode_verified(&doc.to_snapshot_v2()).unwrap(),bytes);
        }
    }
}

#[test]
fn known_valid_circles_at_large_positive_and_negative_coordinates_are_found() {
    use kentos_commands::{ArraysAdopt,ArrayItem,Vec2};
    for center in [(487_000.,4_420_000.),(4_000_000_000.,1_000_000.),(-4_000_000_000.,-1_000_000.)] {
        for rotate in [false,true] {
            let (doc,occupied)=polar_drawing(center,12,&[3,6,10],rotate);
            let input=polar_request(&doc,1);
            let items=(0..12).map(|k|ArrayItem {uid:occupied.iter().position(|v|*v==k).map(|at|kentos_commands::EntityId(doc.uid(Slot(at as u32+1)).unwrap().into_bytes())),kept:false}).collect();
            let known=ArraysAdopt {source:input.source.clone(),layout:ArrayLayout::Polar {center:Vec2 {x:center.0,y:center.1},count:12,fill:360.,rotate},items,tolerance:Some(1e-6),expected_revision:None};
            assert!(matches!(arrays_adopt::inspect(&doc,&known,||false),CommandResult::Completed {..}));
            let found=find(&doc,&input);
            assert!(!found.truncated,"{found:?}");
            assert_eq!(found.matches.len(),1,"center={center:?} rotate={rotate}: {found:?}");
            assert_eq!((found.matches[0].present,found.matches[0].missing),(9,3),"{found:?}");
            assert!(matches!(found.matches[0].adoption.layout,ArrayLayout::Polar {count:12,rotate:r,..} if r==rotate));
        }
    }
}

#[test]
fn a_circle_does_not_invent_a_hidden_half_angular_spacing() {
    let (doc,_)=polar_drawing((0.,0.),8,&[1,3,5,7],false);
    let found=find(&doc,&polar_request(&doc,3));
    assert_eq!(found.matches.len(),1);
    assert!(matches!(found.matches[0].adoption.layout,ArrayLayout::Polar {count:4,..}));
    assert_eq!(found.matches[0].missing,0);
}

#[test]
fn full_circle_is_an_explicit_interpretation_even_when_only_an_arc_is_occupied() {
    let (doc,_)=polar_drawing((10.,20.),12,&[4,5,6,7,8,9,10,11],false);
    let found=find(&doc,&polar_request(&doc,2));
    assert_eq!(found.matches.len(),1,"{found:?}");
    assert!(matches!(found.matches[0].adoption.layout,ArrayLayout::Polar {count:12,fill:360.,..}));
    assert_eq!((found.matches[0].present,found.matches[0].missing),(4,8));
}

#[test]
fn collinear_and_three_point_scopes_are_not_polar_patterns() {
    for points in [vec![(0.,0.),(1.,1.),(2.,2.),(3.,3.),(4.,4.)],vec![(0.,0.),(2.,0.),(1.,1.)]] {
        let doc=drawing(&points);
        assert!(find(&doc,&polar_request(&doc,1)).matches.is_empty());
    }
}

#[test]
fn coincident_roots_are_not_silently_chosen_for_a_circle_slot() {
    let doc=drawing(&[(8.,0.),(0.,8.),(-8.,0.),(0.,-8.),(8.,0.)]);
    assert!(find(&doc,&polar_request(&doc,1)).matches.is_empty());
}

#[test]
fn a_wrong_turn_stays_unmatched_and_is_never_implicitly_kept() {
    let (mut doc,_)=polar_drawing((0.,0.),8,&[],true);
    let mut e=doc.get(Slot(5)).unwrap().clone();
    let Entity::Insert(i)=&mut e else {unreachable!()};i.rotation=0.45;
    doc.update(Slot(5),e);
    let found=find(&doc,&polar_request(&doc,1));
    assert_eq!(found.matches.len(),1,"{found:?}");
    assert_eq!((found.matches[0].present,found.matches[0].missing),(7,1));
    let wrong=kentos_commands::EntityId(doc.uid(Slot(5)).unwrap().into_bytes());
    assert!(found.eligible.contains(&wrong.to_text()));
    assert!(found.matches[0].adoption.items.iter().all(|i|i.uid!=Some(wrong) && !i.kept));
    assert!(find(&doc,&request(&doc,1)).eligible.len()<found.eligible.len());
}

#[test]
fn polar_stop_discards_the_query_and_never_writes_array_metadata() {
    let (doc,_)=polar_drawing((0.,0.),80,&[],false);
    let before=doc.to_snapshot_v2();let mut calls=0;
    let result=arrays_find::inspect(&doc,&polar_request(&doc,1),||{calls+=1;calls>1000});
    assert!(matches!(result,CommandResult::Cancelled),"{result:?}");
    assert_eq!(doc.to_snapshot_v2(),before);
}


fn path_drawing() -> (Document, ArraysFind) {
    let raw: Value=serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.arrays.find.json")).unwrap();
    let case=raw["cases"].as_array().unwrap().iter().find(|c|c["name"]=="yol: köşede teğet hizalama ve iki boş yer").unwrap();
    let doc=Document::from_snapshot(DocumentSnapshotV1::from_json(&case["setup"].to_string()).unwrap()).unwrap();
    let mut input=request(&doc,1);input.kind=Some(kentos_commands::ArrayFindKind::Path);
    input.path=Some(doc.uid(Slot(8)).unwrap().to_string());
    (doc,input)
}

#[test]
fn path_candidates_keep_two_gaps_and_an_explicit_override_then_follow_the_guide_with_same_ids() {
    let (mut doc,input)=path_drawing();let before=doc.to_snapshot_v2();let found=find(&doc,&input);
    assert_eq!(found.matches.len(),1,"{found:?}");assert!(!found.truncated);
    let mut proposal=found.matches[0].adoption.clone();
    assert!(matches!(proposal.layout,ArrayLayout::Path {count:8,spacing:Some(2.),align:true,..}));
    assert_eq!((found.matches[0].present,found.matches[0].missing),(6,2));
    proposal.items[2]=kentos_commands::ArrayItem {uid:Some(kentos_commands::EntityId(doc.uid(Slot(7)).unwrap().into_bytes())),kept:true};
    assert!(matches!(arrays_adopt::execute(&mut ExecutionContext::new(&mut doc),proposal),CommandResult::Completed {..}));
    let bound=doc.to_snapshot_v2();assert_eq!(bound.entities,before.entities);assert_eq!(bound.uids,before.uids);
    let mut guide=doc.get(Slot(8)).unwrap().clone();let Entity::Polyline(p)=&mut guide else{unreachable!()};p.pts[1].x=8.;p.pts[2].x=8.;
    let held=doc.get(Slot(7)).unwrap().clone();let members=doc.arrays()[0].items.clone();
    doc.update(Slot(8),guide);let moved=doc.to_snapshot_v2();
    assert_eq!(doc.arrays()[0].items,members);assert_eq!(doc.get(Slot(7)),Some(&held));
    assert_ne!(moved.entities,bound.entities);doc.undo();assert_eq!(doc.to_snapshot_v2(),bound);
    doc.undo();assert_eq!(doc.to_snapshot_v2(),before);doc.redo();assert_eq!(doc.to_snapshot_v2(),bound);
}

#[test]
fn path_detection_requires_the_stored_start_and_does_not_relocate_an_interior_seed() {
    let (doc,mut input)=path_drawing();input.source=doc.uid(Slot(3)).unwrap().to_string();let before=doc.to_snapshot_v2();
    assert!(matches!(arrays_find::inspect(&doc,&input,||false),CommandResult::Failed {error} if error.code==kentos_commands::codes::RECOGNITION_UNSUPPORTED));
    assert_eq!(doc.to_snapshot_v2(),before);
}

#[test]
fn path_duplicate_slot_is_ambiguous_and_cancellation_discards_all_proposals() {
    let (mut doc,input)=path_drawing();let e=doc.get(Slot(2)).unwrap().clone();doc.add(e).unwrap();
    assert!(find(&doc,&input).matches.is_empty());
    let (doc,input)=path_drawing();let before=doc.to_snapshot_v2();let mut calls=0;
    let result=arrays_find::inspect(&doc,&input,||{calls+=1;calls>40});
    assert!(matches!(result,CommandResult::Cancelled),"{result:?}");assert_eq!(doc.to_snapshot_v2(),before);
}
