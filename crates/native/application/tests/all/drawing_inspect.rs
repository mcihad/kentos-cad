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
