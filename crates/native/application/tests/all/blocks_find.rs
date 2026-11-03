//! Read-only candidate identity maps, policies, linked objects, limits and cooperative cancellation.
use kentos_commands::{BlocksFind, CommandResult, DocumentSnapshotV1};
use kentos_domain::{Document, Slot};
use kentos_native_application::blocks_find;
use serde_json::Value;

fn drawing() -> Document {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    Document::from_snapshot(DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap())
        .unwrap()
}

#[test]
fn limits_and_cancellation_keep_the_source_exact() {
    let doc = drawing();
    let before = doc.to_snapshot_v2();
    let uid = doc.uid(Slot(1)).unwrap().to_string();
    let input = BlocksFind {
        source: vec![uid.clone()],
        uids: Some(vec![uid; 50_001]),
        ..Default::default()
    };
    let CommandResult::Failed { error } = blocks_find::inspect(&doc, &input, || false) else {
        panic!("scope must refuse")
    };
    assert_eq!(error.code, "recognition_scope_limit");
    let input = BlocksFind {
        source: vec![doc.uid(Slot(1)).unwrap().to_string()],
        ..Default::default()
    };
    let mut checks = 0;
    let cancelled = blocks_find::inspect(&doc, &input, || {
        checks += 1;
        checks > 8
    });
    assert!(matches!(cancelled, CommandResult::Cancelled));
    assert_eq!(doc.to_snapshot_v2(), before);
    assert!(!doc.can_undo());
}

#[test]
fn source_order_is_an_explicit_identity_map_and_duplicates_are_taken_once() {
    let doc = drawing();
    let uid = doc.uid(Slot(1)).unwrap().to_string();
    let input = BlocksFind {
        source: vec![uid.clone(), uid.clone()],
        ..Default::default()
    };
    let CommandResult::Completed { output, warnings } =
        blocks_find::inspect(&doc, &input, || false)
    else {
        panic!("query")
    };
    assert_eq!(output.source, [uid]);
    assert_eq!(output.matches.len(), 2);
    assert!(warnings.is_empty());
    assert_eq!(
        output.matches[1].members,
        [doc.uid(Slot(2)).unwrap().to_string()]
    );
    assert_eq!(output.matches[1].at.x, 10.0);
    assert_eq!(output.matches[1].scale, 1.0);
}

#[test]
fn dense_coincident_objects_reach_the_result_limit_without_exhausting_the_work_budget() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut raw = fixture["setup"].clone();
    raw["entities"] = Value::Array(
        (1..=10_000)
            .map(|id| {
                serde_json::json!({
                    "kind":"circle", "id":id, "layerId":"a", "attrs":{},
                    "c":{"x":0.0,"y":0.0}, "r":0.4
                })
            })
            .collect(),
    );
    let doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let before = doc.to_snapshot_v2();
    let input = BlocksFind {
        source: vec![doc.uid(Slot(1)).unwrap().to_string()],
        ..Default::default()
    };
    let CommandResult::Completed { output, warnings } =
        blocks_find::inspect(&doc, &input, || false)
    else {
        panic!("query")
    };
    assert_eq!(output.matches.len(), 1000);
    assert!(output.truncated);
    assert!(
        output.comparisons < 3000,
        "{} comparisons",
        output.comparisons
    );
    assert!(warnings.iter().any(|w| w.code == "recognition_truncated"));
    let ids: std::collections::HashSet<_> = output.matches.iter().map(|m| &m.members[0]).collect();
    assert_eq!(ids.len(), 1000);
    assert_eq!(doc.to_snapshot_v2(), before);
    assert!(!doc.can_undo());
}

#[test]
fn aggregate_path_vertex_budget_refuses_before_matching_and_never_edits() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut raw = fixture["setup"].clone();
    raw["entities"].as_array_mut().unwrap().truncate(1);
    let pts: Vec<_> = (0..4000)
        .map(|i| serde_json::json!({"x":i as f64,"y":0.0}))
        .collect();
    for id in 2..=52 {
        raw["entities"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "kind":"polyline","id":id,"layerId":"a","attrs":{},"pts":pts
            }));
    }
    let doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let before = doc.to_snapshot_v2();
    let input = BlocksFind {
        source: vec![doc.uid(Slot(1)).unwrap().to_string()],
        ..Default::default()
    };
    let CommandResult::Failed { error } = blocks_find::inspect(&doc, &input, || false) else {
        panic!("scope must refuse")
    };
    assert_eq!(error.code, "recognition_scope_limit");
    assert_eq!(error.path.as_deref(), Some("uids"));
    assert_eq!(doc.to_snapshot_v2(), before);
    assert!(!doc.can_undo());
}

#[test]
fn unsupported_path_data_has_a_specific_reason_even_in_geometry_only_mode() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    for (key, values, reason) in [
        ("zs", serde_json::json!([0., 0., 1.]), "(Z)"),
        ("ms", serde_json::json!([0., 1., 2.]), "(M)"),
        (
            "pts",
            Value::Array(vec![serde_json::json!({"x":40.,"y":0.}); 4097]),
            "4 096",
        ),
    ] {
        let mut raw = fixture["setup"].clone();
        let mut entity = raw["entities"][4].clone();
        entity[key] = values;
        raw["entities"] = serde_json::json!([entity]);
        let doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap())
            .unwrap();
        let before = doc.to_snapshot_v2();
        let input = BlocksFind {
            source: vec![doc.entities_with_uids().next().unwrap().0.to_string()],
            geometry_only: Some(true),
            ..Default::default()
        };
        let CommandResult::Failed { error } = blocks_find::inspect(&doc, &input, || false) else {
            panic!("source must refuse")
        };
        assert_eq!(error.code, "recognition_unsupported");
        assert!(error.message.contains(reason), "{}", error.message);
        assert_eq!(error.path.as_deref(), Some("source[0]"));
        assert_eq!(doc.to_snapshot_v2(), before);
        assert!(!doc.can_undo());
    }
}

#[test]
fn nested_candidate_bounds_include_the_geometry_and_empty_scope_still_checks_the_source() {
    let fixture:Value=serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.blocks.find.json")).unwrap();
    let mut raw=fixture["setup"].clone();let id="01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4001";
    raw["blocks"]=serde_json::json!([{"id":id,"name":"Geniş parça","base":{"x":0.,"y":0.},"entities":[{"kind":"line","id":1,"layerId":"a","attrs":{},"a":{"x":-2.,"y":-1.},"b":{"x":3.,"y":4.}}]}]);
    raw["entities"]=serde_json::json!([{"kind":"insert","id":1,"layerId":"a","attrs":{},"block":id,"p":{"x":10.,"y":20.},"scale":1.,"rotation":0.}]);
    let doc=Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let query=BlocksFind {source:vec![doc.uid(Slot(1)).unwrap().to_string()],..Default::default()};
    let answer=blocks_find::inspect(&doc,&query,||false);
    let CommandResult::Completed {output,..}=answer else{panic!("{answer:?}")};let b=output.matches[0].bounds;
    assert_eq!([b.min_x,b.min_y,b.max_x,b.max_y],[8.,19.,13.,24.]);
    raw["blocks"][0]["entities"][0]=serde_json::json!({"kind":"point","id":1,"layerId":"a","attrs":{},"p":{"x":0.,"y":0.}});
    let doc=Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();let before=doc.to_snapshot_v2();
    let query=BlocksFind {source:vec![doc.uid(Slot(1)).unwrap().to_string()],uids:Some(vec![]),..Default::default()};
    assert!(matches!(blocks_find::inspect(&doc,&query,||false),CommandResult::Failed {error} if error.code=="recognition_unsupported"));
    assert_eq!(doc.to_snapshot_v2(),before);
}
