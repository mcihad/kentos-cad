//! Independent geometry, identity, reference, undo and stale-job adoption acceptance.
use kentos_commands::{
    ArrayEdit, ArrayItem, ArrayLayout, ArraysAdopt, ArraysEdit, CallEnvelope, CommandResult,
    DocumentSnapshotV1, Entity, EntityId, JobPhase, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, arrays_adopt, arrays_edit, jobs::Job};
use serde_json::{Value, json};

const BLOCK: &str = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4001";

fn drawing(points: &[(usize, f64, f64, f64)]) -> Document {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut raw = fixture["setup"].clone();
    raw["settings"]["drawingUnit"] = json!("mm");
    raw["blocks"] = json!([{"id":BLOCK,"name":"Teknik parça","base":{"x":0.,"y":0.},"units":"m",
        "entities":[{"kind":"line","id":1,"layerId":"a","attrs":{},"a":{"x":0.,"y":0.},"b":{"x":1.,"y":0.}}]}]);
    raw["entities"] = json!(
        points
            .iter()
            .map(|(k, x, y, r)| json!({
                "kind":"insert","id":k+1,"layerId":"a","attrs":{},"block":BLOCK,
                "p":{"x":x,"y":y},"scale":1.,"rotation":r
            }))
            .collect::<Vec<_>>()
    );
    Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap()
}
fn uid(doc: &Document, k: usize) -> String {
    doc.uid(Slot(k as u32 + 1)).unwrap().to_string()
}
fn request(doc: &Document, layout: ArrayLayout, count: usize, kept: &[usize]) -> ArraysAdopt {
    ArraysAdopt {
        source: uid(doc, 0),
        layout,
        items: (0..count)
            .map(|k| ArrayItem {
                uid: doc
                    .uid(Slot(k as u32 + 1))
                    .map(|u| EntityId(u.into_bytes())),
                kept: kept.contains(&k),
            })
            .collect(),
        tolerance: None,
        expected_revision: None,
    }
}
fn line(count: u32, dx: f64) -> ArrayLayout {
    ArrayLayout::Line {
        count,
        dx,
        dy: 0.,
        fit: false,
    }
}
fn accept(doc: &mut Document, input: ArraysAdopt) -> String {
    let answer = arrays_adopt::execute(&mut ExecutionContext::new(doc), input);
    let CommandResult::Completed { output, .. } = answer else {
        panic!("{answer:?}");
    };
    output.array.to_text()
}
fn edit(doc: &mut Document, array: &str, edit: ArrayEdit) {
    let answer = arrays_edit::execute(
        &mut ExecutionContext::new(doc),
        ArraysEdit {
            array: array.into(),
            edit,
            expected_revision: None,
        },
    );
    assert!(
        matches!(answer, CommandResult::Completed { .. }),
        "{answer:?}"
    );
}
fn refused(doc: &mut Document, input: ArraysAdopt, code: &str) {
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    let answer = arrays_adopt::execute(&mut ExecutionContext::new(doc), input);
    let actual = match answer {
        CommandResult::Failed { error } | CommandResult::Conflict { error } => error.code,
        _ => panic!("must refuse: {answer:?}"),
    };
    assert_eq!(actual, code);
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.undo_labels(), undo);
}
fn at(doc: &Document, id: &str) -> Vec2 {
    let Entity::Insert(i) = doc.get(doc.slot_of(id.parse().unwrap()).unwrap()).unwrap() else {
        panic!("insert");
    };
    i.p
}

#[test]
fn adoption_preserves_two_gaps_override_bound_dimension_and_every_existing_byte() {
    let mut doc = drawing(&[
        (0, 0., 0., 0.),
        (1, 2.5, 0.25, 0.),
        (3, 6., 0., 0.),
        (4, 8., 0., 0.),
        (5, 10., 0., 0.),
        (7, 14., 0., 0.),
    ]);
    let bound = uid(&doc, 3);
    let dimension: Entity =
        serde_json::from_value(json!({"kind":"dimension","id":0,"layerId":"a","attrs":{},
        "a":{"x":6.,"y":0.},"b":{"x":20.,"y":0.},"offset":1.,"height":0.1,
        "refs":{"a":{"uid":bound,"path":[0],"place":{"kind":"start"}}}}))
        .unwrap();
    let dim = doc.add(dimension).unwrap();
    let input = request(&doc, line(8, 2.), 8, &[1]);
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    let CommandResult::Completed { output: plan, .. } =
        arrays_adopt::inspect(&doc, &input, || false)
    else {
        panic!("plan");
    };
    assert_eq!((plan.missing, plan.kept, plan.deviation), (2, 1, 0.));
    assert!(plan.array.id.is_nil());
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.undo_labels(), undo);
    let id = accept(&mut doc, input.clone());
    let adopted = doc.to_snapshot_v2();
    let mut metadata_removed = adopted.clone();
    metadata_removed.arrays.clear();
    assert_eq!(
        metadata_removed, before,
        "only metadata changes; roots, slots, definitions, geometry, settings and refs identical"
    );
    assert_eq!(doc.undo_labels().len(), undo.len() + 1);
    assert_eq!(doc.undo().as_deref(), Some(arrays_adopt::LABEL));
    assert_eq!(doc.to_snapshot_v2(), before);
    doc.redo();
    assert_eq!(doc.to_snapshot_v2(), adopted);
    refused(&mut doc, input, "array_adoption_refused");
    let originals = doc.arrays()[0].items.clone();
    edit(
        &mut doc,
        &id,
        ArrayEdit::Layout {
            layout: line(10, 3.),
        },
    );
    let items = &doc.arrays()[0].items;
    for k in [0, 1, 3, 4, 5, 7] {
        assert_eq!(items[k].uid, originals[k].uid);
    }
    assert!(items[2].uid.is_none() && items[6].uid.is_none());
    assert_eq!(at(&doc, &uid(&doc, 1)), Vec2 { x: 2.5, y: 0.25 });
    assert_eq!(at(&doc, &bound), Vec2 { x: 9., y: 0. });
    let Some(Entity::Dimension(d)) = doc.get(dim) else {
        panic!("dimension");
    };
    assert_eq!(d.a, Vec2 { x: 9., y: 0. });
    assert!(d.refs.as_ref().unwrap().a.as_ref().unwrap().live());
    let expanded = doc.to_snapshot_v2();
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), adopted);
    doc.redo();
    assert_eq!(doc.to_snapshot_v2(), expanded);
    edit(&mut doc, &id, ArrayEdit::Remove { items: vec![4] });
    let removed = doc.to_snapshot_v2();
    edit(&mut doc, &id, ArrayEdit::Reset { items: None });
    assert!(
        doc.arrays()[0]
            .items
            .iter()
            .all(|i| i.uid.is_some() && !i.kept)
    );
    assert_eq!(at(&doc, &uid(&doc, 1)), Vec2 { x: 3., y: 0. });
    for k in [0, 1, 3, 5, 7] {
        assert_eq!(doc.arrays()[0].items[k].uid, originals[k].uid);
    }
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), removed);
    let snapshot = doc.to_snapshot_v2();
    let bytes = kentos_kcad::encode_verified(&snapshot).unwrap();
    let reopened = kentos_kcad::decode(&bytes).unwrap();
    assert_eq!(reopened.arrays, snapshot.arrays);
    assert_eq!(reopened.uids, snapshot.uids);
    assert_eq!(kentos_kcad::encode_verified(&reopened).unwrap(), bytes);
}

#[test]
fn grid_and_both_polar_windings_use_explicit_layout_order_and_preserve_root_ids() {
    let mut doc = drawing(&[
        (0, 10., 20., 0.4),
        (1, 7., 20., 0.4),
        (3, 1., 20., 0.4),
        (4, 10., 16., 0.4),
        (5, 7., 16., 0.4),
        (7, 1., 16., 0.4),
    ]);
    let input = request(
        &doc,
        ArrayLayout::Grid {
            rows: 2,
            cols: 4,
            dx: -3.,
            dy: -4.,
        },
        8,
        &[],
    );
    accept(&mut doc, input);
    assert_eq!(
        doc.arrays()[0].items[7].uid.unwrap().to_text(),
        uid(&doc, 7)
    );
    for direction in [-1., 1.] {
        let points: Vec<_> = (0..8)
            .filter(|k| ![2, 6].contains(k))
            .map(|k| {
                let a = direction * k as f64 * std::f64::consts::TAU / 8.;
                (k, 10. + 4. * a.cos(), 20. + 4. * a.sin(), a + 0.3)
            })
            .collect();
        let mut doc = drawing(&points);
        let input = request(
            &doc,
            ArrayLayout::Polar {
                center: Vec2 { x: 10., y: 20. },
                count: 8,
                fill: direction * 360.,
                rotate: true,
            },
            8,
            &[],
        );
        accept(&mut doc, input);
        assert_eq!(
            doc.arrays()[0]
                .items
                .iter()
                .filter(|i| i.uid.is_none())
                .count(),
            2
        );
    }
}

#[test]
fn closed_path_adoption_follows_path_changes_with_the_same_ids_and_missing_places() {
    let mut doc = drawing(&[
        (0, 0., 0., 0.),
        (1, 2., 0., 0.),
        (3, 4., 2., 0.),
        (4, 4., 4., 0.),
        (5, 2., 4., 0.),
        (7, 0., 2., 0.),
    ]);
    let path: Entity = serde_json::from_value(
        json!({"kind":"polyline","id":0,"layerId":"a","attrs":{},"closed":true,
        "pts":[{"x":0.,"y":0.},{"x":4.,"y":0.},{"x":4.,"y":4.},{"x":0.,"y":4.}]}),
    )
    .unwrap();
    let slot = doc.add(path).unwrap();
    let path = doc.uid(slot).unwrap().to_string();
    let input = request(
        &doc,
        ArrayLayout::Path {
            path,
            count: 8,
            spacing: None,
            align: false,
        },
        8,
        &[],
    );
    let last = uid(&doc, 7);
    accept(&mut doc, input);
    let before = doc.to_snapshot_v2();
    let Some(Entity::Polyline(mut curve)) = doc.get(slot).cloned() else {
        panic!("path");
    };
    curve.closed = false;
    doc.update(slot, Entity::Polyline(curve));
    assert_eq!(at(&doc, &last), Vec2 { x: 0., y: 4. });
    assert!(doc.arrays()[0].items[2].uid.is_none());
    assert!(doc.arrays()[0].items[6].uid.is_none());
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn mismatches_are_never_implicitly_moved_or_marked_kept_and_all_refusals_are_atomic() {
    let mut doc = drawing(&[(0, 0., 0., 0.), (1, 2.5, 0.25, 0.), (2, 4., 0., 0.)]);
    let input = request(&doc, line(3, 2.), 3, &[]);
    refused(&mut doc, input.clone(), "array_adoption_refused");
    let mut valid = input;
    valid.items[1].kept = true;
    let mut bad = valid.clone();
    bad.items[2] = bad.items[0].clone();
    refused(&mut doc, bad, "array_adoption_refused");
    let mut bad = valid.clone();
    bad.items[0].kept = true;
    refused(&mut doc, bad, "array_adoption_refused");
    let mut bad = valid.clone();
    bad.items[2].uid = None;
    bad.items[2].kept = true;
    refused(&mut doc, bad, "array_adoption_refused");
    let mut bad = valid.clone();
    bad.items[2].kept = true;
    refused(&mut doc, bad, "array_adoption_refused");
    let mut bad = valid.clone();
    bad.items.pop();
    refused(&mut doc, bad, "array_adoption_refused");
    let mut bad = valid.clone();
    bad.tolerance = Some(f64::NAN);
    refused(&mut doc, bad, "invalid_tolerance");
    let mut bad = valid.clone();
    bad.layout = line(1, 2.);
    refused(&mut doc, bad, "invalid_count");
    let mut bad = valid.clone();
    bad.expected_revision = Some("999".into());
    refused(&mut doc, bad, "revision_conflict");
    for change in [
        "mirror",
        "scale",
        "rotation",
        "yScale",
        "attrs",
        "color",
        "parameters",
    ] {
        let mut changed = doc.get(Slot(3)).unwrap().clone();
        let Entity::Insert(i) = &mut changed else {
            panic!("insert");
        };
        match change {
            "mirror" => i.mirror = true,
            "scale" => i.scale = 1.1,
            "rotation" => i.rotation = 0.1,
            "yScale" => i.y_scale = Some(1.1),
            "attrs" => {
                i.base.attrs.insert("CODE".into(), "A".into());
            }
            "color" => i.base.color = Some("#aabbcc".into()),
            _ => {
                i.parameters
                    .insert("W".into(), kentos_commands::ParamValue::Number(2.));
            }
        }
        doc.update(Slot(3), changed);
        refused(&mut doc, valid.clone(), "array_adoption_refused");
        doc.undo();
    }
    accept(&mut doc, valid);
}

#[test]
fn missing_and_kept_places_cannot_hide_an_overflowing_proposed_pose() {
    for kept in [false, true] {
        let mut doc = drawing(&[(0, 0., 0., 0.), (1, 2., 0., 0.), (2, 4., 0., 0.)]);
        let mut input = request(&doc, line(3, 1e308), 3, &[]);
        input.tolerance = Some(1e308);
        input.items[2] = ArrayItem {
            uid: kept.then(|| EntityId(doc.uid(Slot(3)).unwrap().into_bytes())),
            kept,
        };
        let before = doc.to_snapshot_v2();
        let CommandResult::Failed { error } = arrays_adopt::inspect(&doc, &input, || false) else {
            panic!("an infinite third place must be refused even when missing or kept");
        };
        assert_eq!(error.code, "array_adoption_refused");
        assert_eq!(error.path.as_deref(), Some("layout"));
        refused(&mut doc, input, "array_adoption_refused");
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

fn envelope(input: &ArraysAdopt) -> CallEnvelope {
    serde_json::from_value(
        json!({"command":"cad.arrays.adopt","version":1,"op":"execute","input":input}),
    )
    .unwrap()
}

#[test]
fn a_metadata_only_plan_guards_source_kept_item_and_definition_reads() {
    use kentos_commands::{CallExpectations, CallOp};
    use kentos_native_application::call::call;
    for change in ["whole placement", "kept item", "definition"] {
        let mut doc = drawing(&[(0, 0., 0., 0.), (1, 2.5, 0.25, 0.), (2, 4., 0., 0.)]);
        let input = request(&doc, line(3, 2.), 3, &[1]);
        let mut planned = envelope(&input);
        planned.op = CallOp::Plan;
        let plan = call(&mut doc, None, planned);
        assert!(
            matches!(plan.result, CommandResult::Completed { .. }),
            "{plan:?}"
        );
        let summary = plan.summary.as_ref().unwrap();
        assert_eq!(summary.sources.objects.len(), 3);
        assert_eq!(summary.sources.blocks.len(), 1);
        assert!(summary.sources.layers.is_some());
        assert_eq!(summary.undo.label.as_deref(), Some(arrays_adopt::LABEL));
        match change {
            "definition" => {
                let mut block = (*doc.blocks()[0]).clone();
                let Entity::Line(l) = &mut block.entities[0] else {
                    panic!("line");
                };
                l.b.x += 1.;
                doc.update_block(block).unwrap();
            }
            _ => {
                for k in if change == "kept item" {
                    vec![1]
                } else {
                    vec![0, 1, 2]
                } {
                    let slot = Slot(k + 1);
                    let Some(Entity::Insert(mut i)) = doc.get(slot).cloned() else {
                        panic!("insert");
                    };
                    i.p.x += 1.;
                    i.p.y += 1.;
                    doc.update(slot, Entity::Insert(i));
                }
            }
        }
        // The proposed membership still fits; rejection must be the old plan guard,
        // not a geometry mismatch or expectedRevision supplied by the caller.
        assert!(matches!(
            arrays_adopt::inspect(&doc, &input, || false),
            CommandResult::Completed { .. }
        ));
        let before = doc.to_snapshot_v2();
        let labels = doc.undo_labels();
        let mut write = envelope(&input);
        write.expected = Some(CallExpectations {
            plan: plan.plan_id,
            ..Default::default()
        });
        let answer = call(&mut doc, None, write);
        let CommandResult::Conflict { error } = answer.result else {
            panic!("must refuse old plan {answer:?}");
        };
        assert_eq!(error.code, "plan_changed", "{change}");
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.undo_labels(), labels);
    }
}

#[test]
fn an_array_metadata_only_reset_reports_its_real_undo_step() {
    use kentos_native_application::call::call;
    let mut doc = drawing(&[(0, 0., 0., 0.), (1, 2., 0., 0.), (2, 4., 0., 0.)]);
    let input = request(&doc, line(3, 2.), 3, &[]);
    let id = accept(&mut doc, input);
    let mut array = (*doc.arrays()[0]).clone();
    array.items[1].kept = true;
    doc.update_array(array).unwrap();
    let before = doc.to_snapshot_v2();
    let reset: CallEnvelope =
        serde_json::from_value(json!({"command":"cad.arrays.edit","op":"execute",
        "input":{"array":id,"edit":{"kind":"reset"}}}))
        .unwrap();
    let answer = call(&mut doc, None, reset);
    assert!(
        matches!(answer.result, CommandResult::Completed { .. }),
        "{answer:?}"
    );
    let summary = answer.summary.unwrap();
    assert!(summary.changed.is_empty() && summary.added.is_empty() && summary.removed.is_empty());
    assert_eq!(summary.undo.label.as_deref(), Some(arrays_edit::LABEL));
    assert_eq!(doc.undo().as_deref(), Some(arrays_edit::LABEL));
    assert_eq!(doc.to_snapshot_v2(), before);
}
#[test]
fn metadata_only_job_refuses_concurrent_geometry_edits_and_undo_redo_generations() {
    for undo in [false, true] {
        let mut doc = drawing(&[(0, 0., 0., 0.), (1, 2., 0., 0.), (2, 4., 0., 0.)]);
        let input = request(&doc, line(3, 2.), 3, &[]);
        let mut job = Job::start("adopt-stale".into(), &doc, envelope(&input));
        let Some(Entity::Insert(mut i)) = doc.get(Slot(1)).cloned() else {
            panic!("insert");
        };
        i.p.y += 1.;
        doc.update(Slot(1), Entity::Insert(i));
        if undo {
            doc.undo();
        }
        let before = doc.to_snapshot_v2();
        let labels = doc.undo_labels();
        let status = job.wait(&mut doc);
        assert_eq!(status.phase, JobPhase::Conflict, "{status:?}");
        let CommandResult::Conflict { error } = status.answer.unwrap().result else {
            panic!("conflict");
        };
        assert_eq!(error.code, "document_changed");
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.undo_labels(), labels);
    }
    let mut doc = drawing(&[(0, 0., 0., 0.), (1, 2., 0., 0.), (2, 4., 0., 0.)]);
    let input = request(&doc, line(3, 2.), 3, &[]);
    let before = doc.to_snapshot_v2();
    let mut job = Job::start("adopt-cancel".into(), &doc, envelope(&input));
    job.cancel();
    assert_eq!(job.wait(&mut doc).phase, JobPhase::Cancelled);
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut calls = 0;
    assert!(matches!(
        arrays_adopt::inspect(&doc, &input, || {
            calls += 1;
            calls > 2
        }),
        CommandResult::Cancelled
    ));
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut job = Job::start("adopt-success".into(), &doc, envelope(&input));
    assert_eq!(job.wait(&mut doc).phase, JobPhase::Done);
    assert!(doc.can_undo());
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
}


#[test]
fn skew_grid_edits_keep_row_column_identity_overrides_gaps_and_bound_dimensions() {
    let angle=30_f64.to_radians();
    let coords:Vec<_>=(0..6).filter(|k|*k!=2).map(|k| {
        let (c,r)=((k%3) as f64,(k/3) as f64);
        (k,c*2.*angle.cos(),c*2.*angle.sin()+r*3.,0.)
    }).collect();
    let mut doc=drawing(&coords);
    let bound=uid(&doc,3);
    let dim=doc.add(serde_json::from_value(json!({"kind":"dimension","id":0,"layerId":"a","attrs":{},
        "a":{"x":0.,"y":3.},"b":{"x":20.,"y":3.},"offset":1.,"height":0.1,
        "refs":{"a":{"uid":bound,"path":[0],"place":{"kind":"start"}}}})).unwrap()).unwrap();
    let layout=ArrayLayout::Lattice {rows:2,cols:3,dx:2.,dy:3.,column_angle:30.,row_angle:90.};
    let input=request(&doc,layout,6,&[1]);let id=accept(&mut doc,input);
    let adopted=doc.to_snapshot_v2();
    let original=doc.arrays()[0].items.clone();let held=at(&doc,&uid(&doc,1));
    edit(&mut doc,&id,ArrayEdit::Layout {layout:ArrayLayout::Lattice {rows:3,cols:4,dx:4.,dy:5.,column_angle:0.,row_angle:60.}});
    for (old,new) in [(0,0),(1,1),(2,2),(3,4),(4,5),(5,6)] {assert_eq!(doc.arrays()[0].items[new],original[old]);}
    assert_eq!(at(&doc,&uid(&doc,1)),held);
    let p=at(&doc,&bound);assert!((p.x-2.5).abs()<1e-12 && (p.y-5.*60_f64.to_radians().sin()).abs()<1e-12);
    let Some(Entity::Dimension(d))=doc.get(dim) else {panic!()};assert_eq!(d.a,p);assert!(d.refs.as_ref().unwrap().a.as_ref().unwrap().live());
    let edited=doc.to_snapshot_v2();doc.undo();assert_eq!(doc.to_snapshot_v2(),adopted);doc.redo();assert_eq!(doc.to_snapshot_v2(),edited);
    edit(&mut doc,&id,ArrayEdit::Remove {items:vec![5]});let removed=doc.to_snapshot_v2();
    edit(&mut doc,&id,ArrayEdit::Reset {items:None});
    assert!(doc.arrays()[0].items.iter().all(|i|i.uid.is_some()&&!i.kept));
    assert_eq!(doc.arrays()[0].items[4].uid,original[3].uid);
    assert_ne!(doc.arrays()[0].items[5].uid,original[4].uid);
    // KCAD compacts vacant runtime slots after deletion; persistent UIDs and array addresses survive.
    let snapshot=doc.to_snapshot_v2();let bytes=kentos_kcad::encode_verified(&snapshot).unwrap();
    let reopened=kentos_kcad::decode(&bytes).unwrap();assert_eq!(reopened.uids,snapshot.uids);assert_eq!(reopened.arrays,snapshot.arrays);
    assert_eq!(kentos_kcad::encode_verified(&reopened).unwrap(),bytes);
    doc.undo();assert_eq!(doc.to_snapshot_v2(),removed);
}
