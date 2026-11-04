use kentos_commands::{CommandResult, DocumentSnapshotV1, DrawingInspect};
use kentos_domain::Document;
use kentos_native_application::drawing_inspect;
use serde_json::{Value, json};

fn document() -> Document {
    let fixture: Value = serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.blocks.find.json")).unwrap();
    let mut raw = fixture["setup"].clone();
    raw["entities"] = json!([
        {"kind":"line","id":1,"layerId":"a","attrs":{},"a":{"x":0.,"y":0.},"b":{"x":1.,"y":0.}},
        {"kind":"line","id":2,"layerId":"a","attrs":{},"a":{"x":1.004,"y":0.},"b":{"x":1.,"y":1.}},
        {"kind":"line","id":3,"layerId":"a","attrs":{},"a":{"x":1.,"y":1.},"b":{"x":0.,"y":1.}},
        {"kind":"line","id":4,"layerId":"a","attrs":{},"a":{"x":0.,"y":1.},"b":{"x":0.,"y":0.}}
    ]);
    Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap()
}

#[test]
fn gap_inspection_is_read_only_and_threshold_is_diagnostic() {
    let doc = document();
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    let input = DrawingInspect { max_gap: Some(0.01), ..Default::default() };
    let CommandResult::Completed { output, .. } = drawing_inspect::inspect(&doc, &input, || false) else { panic!("inspection failed") };
    let gap = output.findings.iter().find(|f| f.code == "boundary_gap").expect("4 mm gap");
    assert!((gap.distance.unwrap() - 0.004).abs() < 1e-9);
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.undo_labels(), undo);
    let input = DrawingInspect { max_gap: Some(0.), ..input };
    let CommandResult::Completed { output, .. } = drawing_inspect::inspect(&doc, &input, || false) else { panic!("inspection failed") };
    assert!(!output.findings.iter().any(|f| f.code == "boundary_gap"));
    assert!(output.findings.iter().any(|f| f.code == "boundary_open_end"));
}

#[test]
fn unicode_readable_is_one_atomic_edit_preserving_fields_and_measured_frame() {
    use kentos_commands::CallOp;
    use kentos_geometry_core::{entity::TextPlace, text::Font};
    use kentos_native_application::{ExecutionContext, geometry::shape, registry};
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut raw = fixture["setup"].clone();
    raw["name"] = json!("Ö");
    raw["entities"] = json!([{"kind":"text","id":1,"layerId":"a","attrs":{},"p":{"x":5000000.,"y":4400000.},
        "text":"Ög\u{301}\u{306}\u{323}ç — AV","height":0.3,"rotation":137.,"font":"arimo","oblique":24.,"mask":true,
        "layout":{"engine":"unicode-v1"},"fields":[{"start":0,"end":1,"source":{"kind":"project","value":"name"}}]}]);
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let before = doc.to_snapshot_v2();
    let (uid, entity) = doc
        .entities_with_uids()
        .next()
        .map(|(u, e)| (u.to_string(), e.clone()))
        .unwrap();
    let old_shape = shape(&entity);
    let old_ring = TextPlace::of(&old_shape).unwrap().outline(Font::DEFAULT);
    let result = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.shapes.edit",
        CallOp::Execute,
        json!({"work":{"kind":"readable","ids":[uid]}}),
    )
    .unwrap();
    assert_eq!(result["status"], "completed", "{result}");
    assert_eq!(result["output"]["count"], 1);
    let after = doc.entities().next().unwrap();
    let new_shape = shape(after);
    let new_ring = TextPlace::of(&new_shape).unwrap().outline(Font::DEFAULT);
    for (i, a) in old_ring.iter().enumerate() {
        let b = new_ring[(i + 2) % 4];
        assert!((a.x - b.x).abs() < 2e-9 && (a.y - b.y).abs() < 2e-9);
    }
    let mut actual = serde_json::to_value(after).unwrap();
    let expected = serde_json::to_value(entity).unwrap();
    actual["p"] = expected["p"].clone();
    actual["rotation"] = expected["rotation"].clone();
    assert_eq!(
        actual, expected,
        "the command must preserve field ranges, formatting and masks"
    );
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn unreadable_unicode_profile_refuses_the_entire_readable_selection_without_undo() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut raw = fixture["setup"].clone();
    raw["entities"] = json!([
        {"kind":"text","id":1,"layerId":"a","attrs":{},"p":{"x":0.,"y":0.},"text":"Sağlam","height":0.3,"rotation":180.,"layout":{"engine":"unicode-v1"}},
        {"kind":"text","id":2,"layerId":"a","attrs":{},"p":{"x":0.,"y":1.},"text":"Eksik 🛸","height":0.3,"rotation":180.,"layout":{"engine":"unicode-v1"}}
    ]);
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    let ids: Vec<_> = doc
        .entities_with_uids()
        .map(|(u, _)| u.to_string())
        .collect();
    for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
        let result = registry::run(
            &mut ExecutionContext::new(&mut doc),
            "cad.shapes.edit",
            op,
            json!({"work":{"kind":"readable","ids":ids}}),
        )
        .unwrap();
        assert_eq!(result["status"], "failed", "{result}");
        assert_eq!(result["error"]["code"], "invalid_input");
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.undo_labels(), undo);
    }
}

#[test]
fn readable_does_not_remeasure_after_silently_refreshing_a_stale_field() {
    use kentos_commands::CallOp;
    use kentos_native_application::{ExecutionContext, registry};
    for layout in [json!({}), json!({"engine":"unicode-v1"})] {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../../../../fixtures/commands/v1/cad.blocks.find.json"
        ))
        .unwrap();
        let mut raw = fixture["setup"].clone();
        raw["name"] = json!("Yeni proje adı");
        raw["entities"] = json!([
            {"kind":"text","id":1,"layerId":"a","attrs":{},"p":{"x":0.,"y":0.},"text":"Sağlam","height":0.3,"rotation":180.},
            {"kind":"text","id":2,"layerId":"a","attrs":{},"p":{"x":0.,"y":1.},"text":"Eski","height":0.3,"rotation":180.,"layout":layout,
                "fields":[{"start":0,"end":4,"source":{"kind":"project","value":"name"}}]}
        ]);
        let mut doc =
            Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap())
                .unwrap();
        let before = doc.to_snapshot_v2();
        let undo = doc.undo_labels();
        let ids: Vec<_> = doc
            .entities_with_uids()
            .map(|(u, _)| u.to_string())
            .collect();
        for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
            let result = registry::run(
                &mut ExecutionContext::new(&mut doc),
                "cad.shapes.edit",
                op,
                json!({"work":{"kind":"readable","ids":ids}}),
            )
            .unwrap();
            assert_eq!(result["status"], "failed", "{result}");
            assert_eq!(result["error"]["code"], "invalid_input");
            assert!(
                result["error"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("açıkça yenileyip")
            );
            assert_eq!(doc.to_snapshot_v2(), before);
            assert_eq!(doc.undo_labels(), undo);
        }
    }
}

#[test]
fn cancelled_inspection_does_not_change_document() {
    let doc = document();
    let before = doc.to_snapshot_v2();
    assert!(matches!(drawing_inspect::inspect(&doc, &DrawingInspect::default(), || true), CommandResult::Cancelled));
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn blocks_outside_boundary_coverage_are_reported() {
    let fixture: Value = serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.blocks.find.json")).unwrap();
    let mut raw = fixture["setup"].clone();
    let block = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4001";
    raw["blocks"] = json!([{"id":block,"name":"Test","base":{"x":0.,"y":0.},"units":"m","entities":[{"kind":"line","id":1,"layerId":"a","attrs":{},"a":{"x":0.,"y":0.},"b":{"x":1.,"y":0.}}]}]);
    raw["entities"] = json!([{"kind":"insert","id":1,"layerId":"a","attrs":{},"block":block,"p":{"x":0.,"y":0.},"scale":1.,"rotation":0.}]);
    let doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let CommandResult::Completed { output, .. } = drawing_inspect::inspect(&doc, &DrawingInspect::default(), || false) else { panic!("inspection failed") };
    assert_eq!(output.unsupported.len(), 1, "block geometry must not be silently treated as inspected");
}

#[test]
fn deleted_source_warns_and_undo_restores_association() {
    use kentos_commands::{CallOp, Entity};
    use kentos_domain::Slot;
    use kentos_native_application::{registry, ExecutionContext};
    let fixture: Value = serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.blocks.find.json")).unwrap();
    let mut raw = fixture["setup"].clone();
    raw["entities"] = json!([{"kind":"polygon","id":1,"layerId":"a","attrs":{},"pts":[{"x":0.,"y":0.},{"x":4.,"y":0.},{"x":4.,"y":3.},{"x":0.,"y":3.}]}]);
    let mut doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let source = doc.uid(Slot(1)).unwrap().to_string();
    let created = registry::run(&mut ExecutionContext::new(&mut doc), "cad.entities.hatch", CallOp::Execute,
        json!({"region":{"kind":"point","at":{"x":2.,"y":1.},"associative":true},"pattern":{"name":"Dolu"},"layerId":"a"})).unwrap();
    assert_eq!(created["status"], "completed", "{created}");
    let before = doc.to_snapshot_v2();
    let deleted = registry::run(&mut ExecutionContext::new(&mut doc), "cad.entities.delete", CallOp::Execute, json!({"uids":[source]})).unwrap();
    assert_eq!(deleted["status"], "completed", "{deleted}");
    assert!(deleted["warnings"].as_array().unwrap().iter().any(|w| w["code"] == "hatch_association_lost"), "{deleted}");
    let h = doc.entities().find_map(|e| if let Entity::Hatch(h) = e { Some(h) } else { None }).unwrap();
    assert!(h.assoc.is_none());
    let CommandResult::Completed { output, .. } = drawing_inspect::inspect(&doc, &DrawingInspect::default(), || false) else { panic!("inspection failed") };
    assert!(output.findings.iter().any(|f| f.code == "hatch_unassociated"));
    doc.undo();
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn caps_and_stale_revision_are_explicit_and_read_only() {
    let doc = document();
    let before = doc.to_snapshot_v2();
    let input = DrawingInspect { uids: Some(vec!["bad".into(); 50_001]), ..Default::default() };
    let CommandResult::Failed { error } = drawing_inspect::inspect(&doc, &input, || false) else { panic!("scope cap") };
    assert_eq!(error.code, "inspection_scope_limit");
    let input = DrawingInspect { limit: Some(1), max_gap: Some(0.), ..Default::default() };
    let CommandResult::Completed { output, warnings } = drawing_inspect::inspect(&doc, &input, || false) else { panic!("partial") };
    assert!(output.truncated);
    assert!(warnings.iter().any(|w| w.code == "inspection_truncated"));
    let input = DrawingInspect { expected_revision: Some("bad".into()), ..Default::default() };
    assert!(matches!(drawing_inspect::inspect(&doc, &input, || false), CommandResult::Failed { error } if error.code == "invalid_revision"));
    let input = DrawingInspect { expected_revision: Some("999999999".into()), ..Default::default() };
    assert!(matches!(drawing_inspect::inspect(&doc, &input, || false), CommandResult::Conflict { error } if error.code == "revision_conflict"));
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn duplicate_crossing_and_tangent_are_distinguished_without_healing() {
    let fixture: Value = serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.blocks.find.json")).unwrap();
    let mut raw = fixture["setup"].clone();
    raw["entities"] = json!([
        {"kind":"line","id":1,"layerId":"a","attrs":{},"a":{"x":0.,"y":0.},"b":{"x":2.,"y":0.}},
        {"kind":"line","id":2,"layerId":"a","attrs":{},"a":{"x":2.,"y":0.},"b":{"x":0.,"y":0.}},
        {"kind":"line","id":3,"layerId":"a","attrs":{},"a":{"x":1.,"y":-1.},"b":{"x":1.,"y":1.}},
        {"kind":"circle","id":4,"layerId":"a","attrs":{},"c":{"x":4.,"y":1.},"r":1.},
        {"kind":"line","id":5,"layerId":"a","attrs":{},"a":{"x":3.,"y":0.},"b":{"x":5.,"y":0.}}
    ]);
    let doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let before=doc.to_snapshot_v2();
    let CommandResult::Completed { output, .. } = drawing_inspect::inspect(&doc, &DrawingInspect::default(), || false) else { panic!("inspection failed") };
    for code in ["boundary_duplicate", "boundary_crossing", "boundary_tangent"] {
        assert!(output.findings.iter().any(|f| f.code == code), "missing {code}: {:?}", output.findings);
    }
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn self_intersection_is_identified_on_the_source_at_large_coordinates() {
    let fixture: Value = serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.blocks.find.json")).unwrap();
    let mut raw = fixture["setup"].clone();
    raw["entities"] = json!([{"kind":"polygon","id":1,"layerId":"a","attrs":{},"pts":[
        {"x":500000.,"y":4500000.}, {"x":500004.,"y":4500004.},
        {"x":500000.,"y":4500004.}, {"x":500004.,"y":4500000.}
    ]}]);
    let doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let before = doc.to_snapshot_v2();
    let CommandResult::Completed { output, .. } = drawing_inspect::inspect(&doc, &DrawingInspect::default(), || false) else { panic!("inspection failed") };
    let hit = output.findings.iter().find(|f| f.code == "boundary_self_intersection").expect("bow-tie crossing");
    assert_eq!(hit.uids.len(), 1);
    assert_eq!(hit.points.len(), 1);
    assert!((hit.points[0].x - 500002.).abs() < 1e-8);
    assert!((hit.points[0].y - 4500002.).abs() < 1e-8);
    assert!(!output.findings.iter().any(|f| f.code == "boundary_open_end"));
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn millimetre_gap_keeps_its_distance_at_map_coordinates() {
    // Reuse the supported v1 input and translate every endpoint, preserving a 4 mm gap.
    let fixture: Value = serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.blocks.find.json")).unwrap();
    let mut raw = fixture["setup"].clone();
    raw["entities"] = json!([
        {"kind":"line","id":1,"layerId":"a","attrs":{},"a":{"x":500000.,"y":4500000.},"b":{"x":500001.,"y":4500000.}},
        {"kind":"line","id":2,"layerId":"a","attrs":{},"a":{"x":500001.004,"y":4500000.},"b":{"x":500002.,"y":4500000.}}
    ]);
    let doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let CommandResult::Completed { output, .. } = drawing_inspect::inspect(&doc, &DrawingInspect {max_gap: Some(0.005), ..Default::default()}, || false) else { panic!("inspection failed") };
    let gap = output.findings.iter().find(|f| f.code == "boundary_gap").expect("4 mm map-coordinate gap");
    assert!((gap.distance.unwrap() - 0.004).abs() < 1e-8);
}

#[test]
fn analytic_hatch_with_two_islands_keeps_area_origin_and_single_undo_on_rebind() {
    use kentos_commands::{CallOp, Entity};
    use kentos_native_application::{registry, ExecutionContext};
    let fixture: Value = serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.blocks.find.json")).unwrap();
    let mut raw = fixture["setup"].clone();
    raw["entities"] = json!([
        {"kind":"circle","id":1,"layerId":"a","attrs":{},"c":{"x":0.,"y":0.},"r":10.},
        {"kind":"circle","id":2,"layerId":"a","attrs":{},"c":{"x":-3.,"y":0.},"r":1.},
        {"kind":"circle","id":3,"layerId":"a","attrs":{},"c":{"x":3.,"y":0.},"r":1.}
    ]);
    let mut doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let created = registry::run(&mut ExecutionContext::new(&mut doc), "cad.entities.hatch", CallOp::Execute,
        json!({"region":{"kind":"point","at":{"x":0.,"y":4.},"associative":true},"pattern":{"name":"ANSI31","angle":30.,"scale":2.,"origin":{"kind":"point","at":{"x":0.123,"y":0.456}}},"layerId":"a"})).unwrap();
    assert_eq!(created["status"], "completed", "{created}");
    let (uid, original) = doc.entities_with_uids().find_map(|(u,e)| if let Entity::Hatch(h)=e {Some((u.to_string(),h.clone()))} else {None}).unwrap();
    assert_eq!(original.holes.as_ref().unwrap().len(), 2);
    assert!(original.boundary.is_some(), "circular boundary must remain analytic");
    let CommandResult::Completed { output, .. } = drawing_inspect::inspect(&doc, &DrawingInspect::default(), || false) else {panic!("inspect")};
    assert!((output.hatches[0].area - 98. * std::f64::consts::PI).abs() < 1e-6, "{:?}", output.hatches);
    assert_eq!(output.hatches[0].pattern_origin, original.pattern.origin);
    let untied = registry::run(&mut ExecutionContext::new(&mut doc), "cad.hatches.edit", CallOp::Execute,
        json!({"edit":{"kind":"untie","uids":[uid]}})).unwrap();
    assert_eq!(untied["status"], "completed", "{untied}");
    let before = doc.to_snapshot_v2();
    let rebound = registry::run(&mut ExecutionContext::new(&mut doc), "cad.hatches.edit", CallOp::Execute,
        json!({"edit":{"kind":"reselect","uid":uid,"region":{"kind":"point","at":{"x":0.,"y":4.},"associative":true}}})).unwrap();
    assert_eq!(rebound["status"], "completed", "{rebound}");
    let h = doc.entities().find_map(|e| if let Entity::Hatch(h)=e {Some(h)} else {None}).unwrap();
    assert_eq!(h.pattern, original.pattern);
    let a = h.boundary.as_ref().unwrap();
    let b = original.boundary.as_ref().unwrap();
    assert_eq!(a.len(), b.len());
    for (a, b) in a.iter().zip(b) {
        assert_eq!(a.pts.len(), b.pts.len());
        // A closed loop may start at a different vertex; each analytic edge must be retained.
        assert!((0..a.pts.len()).any(|shift| (0..a.pts.len()).all(|i| {
            a.pts[(i + shift) % a.pts.len()] == b.pts[i]
                && a.bulges.as_ref().unwrap()[(i + shift) % a.pts.len()] == b.bulges.as_ref().unwrap()[i]
        })));
        assert_eq!(a.curves, b.curves);
    }
    assert_eq!(h.assoc, original.assoc);
    assert_eq!(doc.undo().as_deref(), Some("Sınırı yeniden seç"));
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn moving_hatch_origins_is_atomic_and_preserves_pattern_boundary_and_ties() {
    use kentos_commands::{CallOp, Entity};
    use kentos_native_application::{ExecutionContext, registry};
    let fixture: Value = serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.blocks.find.json")).unwrap();
    let mut raw = fixture["setup"].clone();
    raw["entities"] = json!([
        {"kind":"circle","id":1,"layerId":"a","attrs":{},"c":{"x":0.,"y":0.},"r":5.},
        {"kind":"circle","id":2,"layerId":"a","attrs":{},"c":{"x":12.,"y":0.},"r":2.}
    ]);
    let mut doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    for (x, name) in [(0., "ANSI31"), (12., "Dolu")] {
        let pattern = if name == "Dolu" { json!({"name":name}) } else {
            json!({"name":name,"angle":31.,"scale":2.,"origin":{"kind":"point","at":{"x":0.123,"y":0.456}}})
        };
        let made = registry::run(&mut ExecutionContext::new(&mut doc), "cad.entities.hatch", CallOp::Execute,
            json!({"region":{"kind":"point","at":{"x":x,"y":0.},"associative":true},
                "pattern":pattern,"layerId":"a"})).unwrap();
        assert_eq!(made["status"], "completed", "{made}");
    }
    let hatches: Vec<_> = doc.entities().filter_map(|e| match e { Entity::Hatch(h) => Some((doc.uid(kentos_domain::Slot(h.base.id)).unwrap().to_string(), h.clone())), _ => None }).collect();
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    let failed = registry::run(&mut ExecutionContext::new(&mut doc), "cad.hatches.edit", CallOp::Execute,
        json!({"edit":{"kind":"origin","uids":[hatches[0].0,hatches[1].0],"origin":{"kind":"point","at":{"x":7.,"y":8.}}}})).unwrap();
    assert_eq!(failed["status"], "failed", "{failed}");
    assert_eq!(doc.to_snapshot_v2(), before, "solid in a batch rejects the entire change");
    assert_eq!(doc.undo_labels(), undo);
    for (origin, expected) in [
        (json!({"kind":"point","at":{"x":7.,"y":8.}}), Some(kentos_commands::Vec2 { x:7., y:8. })),
        (json!({"kind":"world"}), None),
    ] {
        let done = registry::run(&mut ExecutionContext::new(&mut doc), "cad.hatches.edit", CallOp::Execute,
            json!({"edit":{"kind":"origin","uids":[hatches[0].0],"origin":origin}})).unwrap();
        assert_eq!(done["status"], "completed", "{done}");
        let mut wanted = hatches[0].1.clone();
        wanted.pattern.origin = expected;
        let current = doc.entities().find_map(|e| match e { Entity::Hatch(h) if h.base.id == wanted.base.id => Some(h), _ => None }).unwrap();
        assert_eq!(current, &wanted);
        assert_eq!(doc.undo().as_deref(), Some("Tarama başlangıcı"));
        assert_eq!(doc.to_snapshot_v2(), before);
    }
}

#[test]
fn annotation_inspection_distinguishes_partial_broken_and_overridden_values_without_writing() {
    use kentos_commands::{CallOp, Entity, InspectionGroup};
    use kentos_domain::Slot;
    use kentos_native_application::{ExecutionContext, registry};
    let fixture: Value = serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.blocks.find.json")).unwrap();
    let mut raw = fixture["setup"].clone();
    raw["settings"]["drawingUnit"] = json!("mm");
    raw["entities"] = json!([
        {"kind":"line","id":1,"layerId":"a","attrs":{},"a":{"x":0.,"y":0.},"b":{"x":2.5,"y":0.}},
        {"kind":"line","id":2,"layerId":"a","attrs":{},"a":{"x":0.,"y":1.},"b":{"x":2.5,"y":1.}}
    ]);
    let mut doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let source = doc.uid(Slot(1)).unwrap().to_string();
    let removed = doc.uid(Slot(2)).unwrap().to_string();
    let mut dimensions = Vec::new();
    for (i, text) in [None, None, Some("2400"), Some("<> mm")].into_iter().enumerate() {
        let mut e = json!({"kind":"dimension","id":0,"layerId":"a","attrs":{},"a":{"x":0.,"y":0.},"b":{"x":2.5,"y":0.},"height":0.1,"offset":0.3 + i as f64 * 0.2});
        if let Some(text) = text { e["text"] = json!(text); }
        let slot = doc.add(serde_json::from_value(e).unwrap()).unwrap();
        dimensions.push(doc.uid(slot).unwrap().to_string());
    }
    let mut execute = |command, input| {
        let r = registry::run(&mut ExecutionContext::new(&mut doc), command, CallOp::Execute, input).unwrap();
        assert_eq!(r["status"], "completed", "{r}");
    };
    execute("cad.dimensions.reassociate", json!({"uid":dimensions[0],"points":[{"point":"a","to":{"uid":source,"place":{"kind":"start"}}}]}));
    execute("cad.dimensions.reassociate", json!({"uid":dimensions[1],"points":[
        {"point":"a","to":{"uid":source,"place":{"kind":"start"}}},
        {"point":"b","to":{"uid":removed,"place":{"kind":"end"}}}]}));
    let leader: Entity = serde_json::from_value(json!({"kind":"leader","id":0,"layerId":"a","attrs":{},
        "pts":[{"x":0.,"y":0.},{"x":3.,"y":0.}],"height":0.1,"rotation":0.,"text":"QA",
        "arms":[{"pts":[{"x":2.5,"y":1.}],"tip":{"uid":removed,"place":{"kind":"end"}}}]})).unwrap();
    let slot = doc.add(leader).unwrap();
    let leader = doc.uid(slot).unwrap().to_string();
    let deleted = registry::run(&mut ExecutionContext::new(&mut doc), "cad.entities.delete", CallOp::Execute, json!({"uids":[removed]})).unwrap();
    assert_eq!(deleted["status"], "completed", "{deleted}");
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    let CommandResult::Completed { output, .. } = drawing_inspect::inspect(&doc, &DrawingInspect { checks: vec![InspectionGroup::Annotations], ..Default::default() }, || false) else { panic!("inspection") };
    let has = |code: &str, uid: &str| output.findings.iter().any(|f| f.code == code && f.uids.first().map(String::as_str) == Some(uid));
    assert!(has("dimension_partial", &dimensions[0]));
    assert_eq!(output.findings.iter().find(|f| f.code == "dimension_partial").unwrap().points,
        vec![kentos_commands::Vec2 { x:2.5, y:0. }], "mark the free end, not the still-bound end");
    assert!(has("dimension_broken", &dimensions[1]));
    assert!(has("dimension_override", &dimensions[2]));
    assert!(!has("dimension_override", &dimensions[3]), "<> still displays the real measure");
    assert!(has("leader_broken", &leader), "all arms, not only the first tip, are inspected");
    let value = output.dimensions.iter().find(|d| d.uid == dimensions[2]).unwrap();
    assert_eq!(value.measured, 2.5);
    assert_eq!(value.shown, "2400");
    assert!(value.overridden);
    let broken = output.findings.iter().find(|f| f.code == "dimension_broken").unwrap();
    assert!(broken.uids.contains(&removed), "missing source identity remains explainable");
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.undo_labels(), undo);
}

fn field_drawing(count: usize) -> kentos_commands::DocumentSnapshotV2 {
    let fixture: Value = serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.blocks.find.json")).unwrap();
    let mut raw = fixture["setup"].clone();
    raw["name"] = json!("Güncel proje");
    raw["entities"] = Value::Array((1..=count).map(|i| json!({
        "kind":"text", "id":i, "layerId":"a", "attrs":{}, "p":{"x":i as f64,"y":0.},
        "text":"Eski", "height":0.125, "rotation":0.
    })).collect());
    Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap().to_snapshot_v2()
}

fn set_field(snapshot: &mut kentos_commands::DocumentSnapshotV2, index: usize, source: Value) {
    let kentos_commands::Entity::Text(t) = &mut snapshot.entities[index] else { panic!() };
    t.paragraph.fields = serde_json::from_value(json!([{"start":0,"end":4,"source":source}])).unwrap();
}

#[test]
fn opened_field_cache_is_compared_to_live_value_without_rewriting_the_file() {
    use kentos_commands::{Entity, InspectionSeverity};
    let mut snapshot = field_drawing(1);
    set_field(&mut snapshot, 0, json!({"kind":"project","value":"name"}));
    let doc = Document::from_snapshot_v2(snapshot).unwrap();
    let before = doc.to_snapshot_v2();
    let uid = doc.uid(kentos_domain::Slot(1)).unwrap();
    let e = doc.get(kentos_domain::Slot(1)).unwrap();
    let found = drawing_inspect::annotations::findings(&doc, uid, e);
    let stale = found.iter().find(|f| f.code == "text_field_stale").unwrap();
    assert_eq!(stale.severity, InspectionSeverity::Error);
    assert!(stale.message.contains("Eski") && stale.message.contains("Güncel proje"));
    assert_eq!(doc.to_snapshot_v2(), before);
    assert!(!doc.is_dirty());
    let Entity::Text(t) = e else { panic!() };
    assert_eq!(t.text, "Eski");
}

#[test]
fn hidden_stale_text_source_and_cycles_cannot_be_mistaken_for_current_values() {
    let mut snapshot = field_drawing(3);
    let uids = snapshot.uids.clone();
    set_field(&mut snapshot, 0, json!({"kind":"object","uid":uids[1],"value":"text"}));
    set_field(&mut snapshot, 1, json!({"kind":"project","value":"name"}));
    let doc = Document::from_snapshot_v2(snapshot.clone()).unwrap();
    let found = drawing_inspect::annotations::findings(&doc, kentos_domain::Uuid::from_bytes(uids[0].0), &snapshot.entities[0]);
    assert!(found.iter().any(|f| f.code == "text_field_stale"));
    // Close a dependency ring. The detector is iterative and reports live failure,
    // even if the imported file did not store a broken flag.
    set_field(&mut snapshot, 1, json!({"kind":"object","uid":uids[2],"value":"text"}));
    set_field(&mut snapshot, 2, json!({"kind":"object","uid":uids[1],"value":"text"}));
    let doc = Document::from_snapshot_v2(snapshot.clone()).unwrap();
    let before = doc.to_snapshot_v2();
    let found = drawing_inspect::annotations::findings(&doc, kentos_domain::Uuid::from_bytes(uids[0].0), &snapshot.entities[0]);
    assert!(found.iter().any(|f| f.code == "text_field_stale"));
    assert!(found.iter().any(|f| f.code == "text_field_broken"));
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn long_field_chains_stop_with_an_explicit_unverified_finding() {
    let mut snapshot = field_drawing(513);
    let uids = snapshot.uids.clone();
    for i in 0..512 {
        set_field(&mut snapshot, i, json!({"kind":"object","uid":uids[i+1],"value":"text"}));
    }
    let doc = Document::from_snapshot_v2(snapshot.clone()).unwrap();
    let found = drawing_inspect::annotations::findings(&doc, kentos_domain::Uuid::from_bytes(uids[0].0), &snapshot.entities[0]);
    assert!(found.iter().any(|f| f.code == "text_field_unverified"));
}

#[test]
fn unicode_profile_reports_real_fallback_and_unsupported_text_without_rewriting_it() {
    use kentos_commands::{Entity,InspectionSeverity};
    for (words,code,severity) in [
        ("Çap ⌀32 — العربية","text_font_fallback",InspectionSeverity::Info),
        ("Eksik 🛸","text_layout_unsupported",InspectionSeverity::Error),
    ] {
        let mut snapshot=field_drawing(1);
        let Entity::Text(t)=&mut snapshot.entities[0] else {panic!()};
        t.text=words.into();t.face.font=Some(kentos_commands::DrawingFont::Arimo);
        t.paragraph.layout=serde_json::from_value(json!({"engine":"unicode-v1"})).unwrap();
        let doc=Document::from_snapshot_v2(snapshot).unwrap();
        let before=doc.to_snapshot_v2();
        let CommandResult::Completed {output,..}=drawing_inspect::inspect(&doc,&DrawingInspect::default(),||false) else {panic!()};
        let f=output.findings.iter().find(|f|f.code==code).expect(words);
        assert_eq!(f.severity,severity);
        if code=="text_font_fallback" {assert!(f.message.contains("noto-sans-math")&&f.message.contains("noto-sans-arabic"));}
        assert_eq!(doc.to_snapshot_v2(),before);
    }
}
fn field_table(source: Value) -> kentos_commands::Entity {
    serde_json::from_value(json!({"kind":"table","id":1,"layerId":"a","attrs":{},
        "p":{"x":1.,"y":2.},"rotation":30.,"height":0.125,"rows":[0.5],"columns":[3.],
        "cells":[["Eski"]],"fields":[{"row":0,"col":0,"source":source}]}))
    .unwrap()
}

#[test]
fn standalone_table_field_is_checked_read_only_refreshed_explicitly_and_undoable() {
    use kentos_commands::{CallOp, Entity};
    use kentos_domain::Slot;
    use kentos_native_application::{ExecutionContext, registry};
    let mut snapshot = field_drawing(1);
    snapshot.entities[0] = field_table(json!({"kind":"project","value":"name"}));
    let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
    let before = doc.to_snapshot_v2();
    let uid = doc.uid(Slot(1)).unwrap();
    let found = drawing_inspect::annotations::findings(&doc, uid, doc.get(Slot(1)).unwrap());
    let stale = found
        .iter()
        .find(|f| f.code == "table_field_stale")
        .unwrap();
    assert_eq!(stale.uids, [uid.to_string()]);
    assert!(
        stale.message.contains("R1C1")
            && stale.message.contains("Eski")
            && stale.message.contains("Güncel proje")
    );
    assert!((stale.points[0].x - (1. + 1.5 * 30_f64.to_radians().cos() + 0.125)).abs() < 1e-12);
    assert_eq!(doc.to_snapshot_v2(), before);
    assert!(!doc.is_dirty());
    let result = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.tables.edit",
        CallOp::Execute,
        json!({"operation":{"kind":"refresh","uids":[uid.to_string()]}}),
    )
    .unwrap();
    assert_eq!(result["status"], "completed", "{result}");
    let Entity::Table(table) = doc.get(Slot(1)).unwrap() else {
        panic!()
    };
    assert_eq!(table.cells, [vec!["Güncel proje".to_owned()]]);
    assert!(
        drawing_inspect::annotations::findings(&doc, uid, doc.get(Slot(1)).unwrap()).is_empty()
    );
    assert!(doc.undo().is_some());
    assert_eq!(doc.to_snapshot_v2(), before);
    assert!(
        drawing_inspect::annotations::findings(&doc, uid, doc.get(Slot(1)).unwrap())
            .iter()
            .any(|f| f.code == "table_field_stale")
    );
}

#[test]
fn table_field_checks_stale_text_source_graph_and_missing_source_without_mutation() {
    use kentos_domain::{Slot, Uuid};
    let mut snapshot = field_drawing(3);
    let uids = snapshot.uids.clone();
    snapshot.entities[0] = field_table(json!({"kind":"object","uid":uids[1],"value":"text"}));
    set_field(
        &mut snapshot,
        1,
        json!({"kind":"object","uid":uids[2],"value":"text"}),
    );
    set_field(
        &mut snapshot,
        2,
        json!({"kind":"object","uid":uids[1],"value":"text"}),
    );
    let doc = Document::from_snapshot_v2(snapshot.clone()).unwrap();
    let before = doc.to_snapshot_v2();
    let uid = doc.uid(Slot(1)).unwrap();
    let found = drawing_inspect::annotations::findings(&doc, uid, doc.get(Slot(1)).unwrap());
    for code in ["table_field_source_stale", "table_field_source_broken"] {
        let f = found.iter().find(|f| f.code == code).unwrap();
        assert_eq!(f.uids.first(), Some(&uid.to_string()));
        assert!(f.uids.contains(&Uuid::from_bytes(uids[1].0).to_string()));
    }
    assert_eq!(doc.to_snapshot_v2(), before);
    snapshot.entities[0] = field_table(
        json!({"kind":"object","uid":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4099","value":"text"}),
    );
    let doc = Document::from_snapshot_v2(snapshot).unwrap();
    let uid = doc.uid(Slot(1)).unwrap();
    let found = drawing_inspect::annotations::findings(&doc, uid, doc.get(Slot(1)).unwrap());
    assert!(found.iter().any(|f| f.code == "table_field_stale"));
    assert!(found.iter().any(|f| f.code == "table_field_broken"));
}
fn nested_field_block_drawing() -> kentos_commands::DocumentSnapshotV2 {
    let mut snapshot = field_drawing(1);
    let outer = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4011";
    let inner = "01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4012";
    snapshot.blocks = serde_json::from_value(json!([
        {"id":outer,"name":"Dış","base":{"x":0.,"y":0.},"units":"m","entities":[
            {"kind":"insert","id":7,"layerId":"a","attrs":{},"block":inner,"p":{"x":2.,"y":2.},"scale":2.,"rotation":1.5707963267948966}]},
        {"id":inner,"name":"Alanlı iç","base":{"x":0.,"y":0.},"units":"m","entities":[
            {"kind":"text","id":17,"layerId":"a","attrs":{},"p":{"x":1.,"y":0.},"height":0.125,"rotation":0.,"text":"Eski",
             "fields":[{"start":0,"end":4,"source":{"kind":"project","value":"name"}}]}]}
    ])).unwrap();
    snapshot.entities[0] =
        serde_json::from_value(json!({"kind":"insert","id":1,"layerId":"a","attrs":{},
        "block":outer,"p":{"x":10.,"y":20.},"scale":1.,"rotation":0.}))
        .unwrap();
    snapshot
}

#[test]
fn anonymous_nested_block_field_uses_real_owner_and_transformed_source_without_mutation() {
    use kentos_commands::{Entity, InspectionGroup};
    use kentos_domain::Slot;
    let snapshot = nested_field_block_drawing();
    let doc = Document::from_snapshot_v2(snapshot).unwrap();
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    let uid = doc.uid(Slot(1)).unwrap();
    let found = drawing_inspect::annotations::findings(&doc, uid, doc.get(Slot(1)).unwrap());
    let stale = found
        .iter()
        .find(|f| f.code == "block_text_field_stale")
        .expect("anonymous text must be evaluated directly");
    assert_eq!(stale.uids, [uid.to_string()]);
    assert_eq!(stale.points, [kentos_commands::Vec2 { x: 12., y: 24. }]);
    assert!(
        stale.message.contains("Alanlı iç")
            && stale.message.contains("Eski")
            && stale.message.contains("Güncel proje")
    );
    let input = DrawingInspect {
        checks: vec![InspectionGroup::Annotations],
        ..Default::default()
    };
    let CommandResult::Completed { output, .. } = drawing_inspect::inspect(&doc, &input, || false)
    else {
        panic!()
    };
    assert!(output.findings.iter().any(|f| f == stale));
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.undo_labels(), undo);
    let Entity::Text(original) = &doc.blocks()[1].entities[0] else {
        panic!()
    };
    assert_eq!(original.text, "Eski");
}

#[test]
fn hidden_or_clipped_block_field_is_not_reported_as_a_visible_error() {
    use kentos_commands::{Entity, InspectionGroup};
    use kentos_domain::Slot;
    let mut snapshot = nested_field_block_drawing();
    let mut hidden = snapshot.layers[0].clone();
    hidden.id = "hidden".into();
    hidden.name = "Gizli alanlar".into();
    hidden.visible = false;
    snapshot.layers.push(hidden);
    let block = &mut snapshot.blocks[1];
    let Entity::Text(text) = &mut block.entities[0] else {
        panic!()
    };
    text.base.layer_id = "hidden".into();
    let doc = Document::from_snapshot_v2(snapshot).unwrap();
    let uid = doc.uid(Slot(1)).unwrap();
    assert!(
        drawing_inspect::annotations::findings(&doc, uid, doc.get(Slot(1)).unwrap()).is_empty()
    );
    let input = DrawingInspect {
        checks: vec![InspectionGroup::Annotations],
        include_hidden: true,
        ..Default::default()
    };
    let CommandResult::Completed { output, .. } = drawing_inspect::inspect(&doc, &input, || false)
    else {
        panic!()
    };
    assert!(
        output
            .findings
            .iter()
            .any(|f| f.code == "block_text_field_stale")
    );
    let mut snapshot = nested_field_block_drawing();
    let Entity::Insert(insert) = &mut snapshot.entities[0] else {
        panic!()
    };
    insert.clipping=Some(serde_json::from_value(json!({"boundary":[{"x":100.,"y":100.},{"x":101.,"y":100.},{"x":101.,"y":101.},{"x":100.,"y":101.}]})).unwrap());
    let doc = Document::from_snapshot_v2(snapshot).unwrap();
    let uid = doc.uid(Slot(1)).unwrap();
    assert!(
        drawing_inspect::annotations::findings(&doc, uid, doc.get(Slot(1)).unwrap()).is_empty()
    );
}

#[test]
fn dynamic_block_visibility_preserves_original_annotation_ordinal() {
    use kentos_commands::Entity;
    use kentos_domain::Slot;
    let mut snapshot = nested_field_block_drawing();
    snapshot.blocks[1].entities.insert(0,serde_json::from_value(json!({"kind":"line","id":3,"layerId":"a","attrs":{},"a":{"x":0.,"y":0.},"b":{"x":1.,"y":0.}})).unwrap());
    snapshot.blocks[1].dynamic=Some(serde_json::from_value(json!({"parameters":[{"kind":"visibility","name":"Görünüş","states":[{"name":"Alan","hidden":[3]},{"name":"Sade","hidden":[17]}]}],"actions":[]})).unwrap());
    let doc = Document::from_snapshot_v2(snapshot.clone()).unwrap();
    let uid = doc.uid(Slot(1)).unwrap();
    let found = drawing_inspect::annotations::findings(&doc, uid, doc.get(Slot(1)).unwrap());
    let stale = found
        .iter()
        .find(|f| f.code == "block_text_field_stale")
        .unwrap();
    assert!(
        stale.message.contains("2. parça"),
        "original ordinal survives filtering: {stale:?}"
    );
    let Entity::Insert(nested) = &mut snapshot.blocks[0].entities[0] else {
        panic!()
    };
    nested.parameters = serde_json::from_value(json!({"Görünüş":"Sade"})).unwrap();
    let doc = Document::from_snapshot_v2(snapshot).unwrap();
    let uid = doc.uid(Slot(1)).unwrap();
    assert!(
        drawing_inspect::annotations::findings(&doc, uid, doc.get(Slot(1)).unwrap()).is_empty()
    );
}

#[test]
fn anonymous_block_field_checks_global_source_graph_with_real_source_uid() {
    use kentos_commands::Entity;
    use kentos_domain::Slot;
    let mut snapshot = nested_field_block_drawing();
    let sources = field_drawing(2);
    snapshot.entities.push(sources.entities[1].clone());
    snapshot.uids.push(sources.uids[1]);
    let source = snapshot.uids[1];
    let Entity::Text(leaf) = &mut snapshot.blocks[1].entities[0] else {
        panic!()
    };
    leaf.paragraph.fields = serde_json::from_value(
        json!([{"start":0,"end":4,"source":{"kind":"object","uid":source,"value":"text"}}]),
    )
    .unwrap();
    set_field(&mut snapshot, 1, json!({"kind":"project","value":"name"}));
    let doc = Document::from_snapshot_v2(snapshot).unwrap();
    let before = doc.to_snapshot_v2();
    let uid = doc.uid(Slot(1)).unwrap();
    let found = drawing_inspect::annotations::findings(&doc, uid, doc.get(Slot(1)).unwrap());
    let stale = found
        .iter()
        .find(|f| f.code == "block_text_field_stale")
        .expect("equal cache does not hide stale upstream text");
    assert_eq!(stale.uids[0], uid.to_string());
    assert!(
        stale
            .uids
            .contains(&kentos_domain::Uuid::from_bytes(source.0).to_string())
    );
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn invalid_in_memory_block_table_is_checked_defensively_without_mutation() {
    use kentos_domain::Slot;
    let mut snapshot = nested_field_block_drawing();
    snapshot.blocks[1].entities[0] = field_table(json!({"kind":"project","value":"name"}));
    let doc = Document::from_snapshot_v2(snapshot).unwrap();
    let before = doc.to_snapshot_v2();
    let uid = doc.uid(Slot(1)).unwrap();
    let found = drawing_inspect::annotations::findings(&doc, uid, doc.get(Slot(1)).unwrap());
    let stale = found
        .iter()
        .find(|f| f.code == "block_table_field_stale")
        .unwrap();
    assert_eq!(stale.uids, [uid.to_string()]);
    // Local table cell center, followed by inner scale/rotation/translation and root translation.
    let local_x = 1. + 1.5 * 30_f64.to_radians().cos() + 0.125;
    let local_y = 2. + 1.5 * 30_f64.to_radians().sin() - 0.25 * 30_f64.to_radians().cos();
    assert!(
        (stale.points[0].x - (12. - 2. * local_y)).abs() < 1e-12,
        "{stale:?}"
    );
    assert!(
        (stale.points[0].y - (22. + 2. * local_x)).abs() < 1e-12,
        "{stale:?}"
    );
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn block_inspection_checks_cancellation_inside_a_clean_single_insert() {
    use kentos_commands::InspectionGroup;
    let mut snapshot = nested_field_block_drawing();
    snapshot.blocks[1].entities=(1..=1000).map(|id|serde_json::from_value(json!({"kind":"line","id":id,"layerId":"a","attrs":{},"a":{"x":0.,"y":0.},"b":{"x":1.,"y":0.}})).unwrap()).collect();
    let doc = Document::from_snapshot_v2(snapshot).unwrap();
    let before = doc.to_snapshot_v2();
    let mut ticks = 0;
    let input = DrawingInspect {
        checks: vec![InspectionGroup::Annotations],
        ..Default::default()
    };
    let result = drawing_inspect::inspect(&doc, &input, || {
        ticks += 1;
        ticks > 100
    });
    assert!(matches!(result, CommandResult::Cancelled), "{result:?}");
    assert_eq!(ticks, 101);
    assert_eq!(doc.to_snapshot_v2(), before);
}


#[test]
fn annotation_visitor_preserves_rendered_shapes_and_honours_early_stop() {
    use kentos_commands::Entity;
    use kentos_native_application::{blocks::core_blocks, geometry::shape};
    let mut snapshot=nested_field_block_drawing();
    // More than one visitor batch; each leaf retains its original definition ordinal.
    let Entity::Text(first)=snapshot.blocks[1].entities[0].clone() else {panic!()};
    snapshot.blocks[1].entities=(1..=130).map(|i| {
        let mut t=first.clone();t.base.id=i;t.p.x=i as f64;Entity::Text(t)
    }).collect();
    let doc=Document::from_snapshot_v2(snapshot).unwrap();
    let library=core_blocks(doc.blocks());
    let insert=shape(doc.get(kentos_domain::Slot(1)).unwrap());
    let all=library.expand(&insert);
    let mut visited=Vec::new();
    let complete:Result<bool,()>=library.try_visit_annotations(&insert,|piece|{visited.extend(piece);Ok(true)});
    assert_eq!(complete,Ok(true));assert_eq!(visited,all);
    let mut seen=0;
    let complete:Result<bool,()>=library.try_visit_annotations(&insert,|piece| {
        if piece.is_some() {seen+=1;} Ok(seen<3)
    });
    assert_eq!(complete,Ok(false));assert_eq!(seen,3);
    let mut ticks=0;
    let complete=library.try_visit_annotations(&insert,|_| {
        ticks+=1;if ticks==7 {Err("cancelled")} else {Ok(true)}
    });
    assert_eq!(complete,Err("cancelled"));assert_eq!(ticks,7);
}

#[test]
fn invalid_block_annotation_provenance_is_an_explicit_error() {
    use kentos_commands::InspectionSeverity;
    use kentos_domain::Slot;
    use kentos_geometry_core::block::AnnotationOrigin;
    let doc=Document::from_snapshot_v2(nested_field_block_drawing()).unwrap();
    let uid=doc.uid(Slot(1)).unwrap();
    let shape=kentos_native_application::geometry::shape(&doc.blocks()[1].entities[0]);
    for origin in [
        AnnotationOrigin {definition:"missing".into(),entity:0},
        AnnotationOrigin {definition:doc.blocks()[1].id.to_text(),entity:99},
    ] {
        let found=drawing_inspect::annotations::piece_findings(&doc,uid,&origin,&shape);
        assert_eq!(found.len(),1);
        assert_eq!(found[0].code,"block_field_unverified");
        assert_eq!(found[0].severity,InspectionSeverity::Error);
        assert_eq!(found[0].uids,[uid.to_string()]);
    }
}
