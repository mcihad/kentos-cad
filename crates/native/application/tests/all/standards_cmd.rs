//! Standardı denetle and Standardı uygula over a large drawing (docs/adr/0312 Doğrulama): 500 layers holding
//! 100 000 objects, a third named the old way (`duvar-1`, `kapi-2`, `aks-3`), the architectural office's standard
//! checked, then applied (plan and write). The shared cases are `fixtures/commands/v1/cad.standards.{check,apply}.json`.

use std::time::Instant;

use kentos_commands::{CommandResult, DocumentSnapshotV1, StandardsApply, StandardsCheck};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, standards};
use serde_json::json;

#[test]
fn unknown_standard_rules_are_refused_by_command_registry_without_mutation() {
    use kentos_commands::CallOp;
    use kentos_native_application::registry;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let snapshot = DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap();
    let mut doc = Document::from_snapshot(snapshot).unwrap();
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    let standard = json!({"name":"Ofis", "layers":[{
        "name":"A-DUVAR", "color":"ink", "lineType":"continuous",
        "lineWeight":0.5, "unsupportedRule":true
    }]});
    for command in ["cad.standards.check", "cad.standards.apply"] {
        for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
            let error = registry::run(
                &mut ExecutionContext::new(&mut doc),
                command,
                op,
                json!({"standard": standard}),
            )
            .unwrap_err();
            assert_eq!(error.code, "invalid_input", "{command}: {error:?}");
            assert_eq!(doc.to_snapshot_v2(), before);
            assert_eq!(doc.undo_labels(), undo);
        }
    }
}

#[test]
fn project_standards_resolve_by_order() {
    use kentos_commands::{
        CallOp, ProjectStandardDocument, STANDARD_PACKAGE_FORMAT, STANDARD_PACKAGE_VERSION,
        StandardPackage,
    };
    use kentos_native_application::registry;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap(),
    )
    .unwrap();
    let query = |doc: &mut Document| {
        registry::run(
            &mut ExecutionContext::new(doc),
            "cad.standards.resolve",
            CallOp::Execute,
            json!({}),
        )
        .unwrap()
    };
    assert_eq!(query(&mut doc)["status"], "failed");
    let first = kentos_commands::built_in_standard("mimari-ofis").unwrap();
    let mut second = first.clone();
    second.by_layer = !first.by_layer;
    second.layers[0].name = format!(" {} ", first.layers[0].name);
    second.layers[0].color = "#ff0000".into();
    let mut added = second.layers[0].clone();
    added.name = "A-EK".into();
    second.layers.push(added);
    let package = |id: &str, standard| ProjectStandardDocument {
        text: serde_json::to_string(&StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_VERSION,
            id: id.into(),
            revision: "1".into(),
            standard,
        })
        .unwrap(),
    };
    let mut settings = doc.settings().clone();
    settings.standard_docs = vec![
        package("first", first.clone()),
        package("second", second.clone()),
    ];
    doc.set_settings(settings);
    let before = doc.to_snapshot_v2();
    let resolved = query(&mut doc);
    assert_eq!(resolved["status"], "completed", "{resolved}");
    assert_eq!(resolved["output"]["sources"], json!(["first", "second"]));
    let standard: kentos_commands::DrawingStandard =
        serde_json::from_value(resolved["output"]["standard"].clone()).unwrap();
    assert_eq!(standard.layers.len(), first.layers.len() + 1);
    assert_eq!(standard.layers[0].color, first.layers[0].color);
    assert_eq!(standard.by_layer, first.by_layer);
    let checked = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.check",
        CallOp::Execute,
        json!({"standard":standard}),
    )
    .unwrap();
    assert_eq!(checked["status"], "completed", "{checked}");
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut settings = doc.settings().clone();
    settings.standard_docs.reverse();
    doc.set_settings(settings);
    let resolved = query(&mut doc);
    let standard: kentos_commands::DrawingStandard =
        serde_json::from_value(resolved["output"]["standard"].clone()).unwrap();
    assert_eq!(standard.layers[0].color, "#ff0000");
    assert_eq!(standard.by_layer, second.by_layer);
    let mut settings = doc.settings().clone();
    settings.standard_docs[0].text = "{}".into();
    doc.set_settings(settings);
    assert_eq!(query(&mut doc)["status"], "failed");
}

#[test]
fn project_standard_conflicts_report_sources_and_fields_without_mutating() {
    use kentos_commands::{
        CallOp, ProjectStandardDocument, STANDARD_PACKAGE_FORMAT, STANDARD_PACKAGE_VERSION,
        StandardPackage,
    };
    use kentos_native_application::registry;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap(),
    )
    .unwrap();
    let first = kentos_commands::built_in_standard("mimari-ofis").unwrap();
    let mut second = first.clone();
    second.by_layer = !first.by_layer;
    second.layers[0].name = format!(" {} ", second.layers[0].name);
    second.text_styles[0].name = format!(" {} ", second.text_styles[0].name);
    second.dimension_styles[0].name = format!(" {} ", second.dimension_styles[0].name);
    second.layers[0].color = "#ff0000".into();
    second.text_styles[0].height = Some(7.0);
    second.dimension_styles[0].height = 8.0;
    let package = |id: &str, standard| ProjectStandardDocument {
        text: serde_json::to_string(&StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_VERSION,
            id: id.into(),
            revision: "1".into(),
            standard,
        })
        .unwrap(),
    };
    let mut settings = doc.settings().clone();
    settings.standard_docs = vec![package("first", first), package("second", second)];
    doc.set_settings(settings);
    let before = doc.to_snapshot_v2();
    let result = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.conflicts",
        CallOp::Execute,
        json!({}),
    )
    .unwrap();
    assert_eq!(result["status"], "completed", "{result}");
    let rows = result["output"]["conflicts"].as_array().unwrap();
    assert_eq!(rows.len(), 4, "{result}");
    assert_eq!(rows[0]["kind"], "rules");
    assert_eq!(rows[0]["fields"], json!(["byLayer"]));
    assert_eq!(rows[1]["kind"], "layer");
    assert_eq!(rows[1]["fields"], json!(["color"]));
    assert_eq!(rows[0]["firstId"], "first");
    assert_eq!(rows[0]["laterId"], "second");
    assert_eq!(doc.to_snapshot_v2(), before);
    let mut settings = doc.settings().clone();
    settings.standard_docs.reverse();
    doc.set_settings(settings);
    let result = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.conflicts",
        CallOp::Plan,
        json!({}),
    )
    .unwrap();
    assert_eq!(result["output"]["conflicts"][0]["firstId"], "second");
    assert_eq!(result["output"]["conflicts"][0]["laterId"], "first");
    let mut settings = doc.settings().clone();
    settings.standard_docs[0].text = settings.standard_docs[0]
        .text
        .replace("\"layers\":", "\"badkey\":");
    doc.set_settings(settings);
    let before_invalid = doc.to_snapshot_v2();
    let result = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.conflicts",
        CallOp::Execute,
        json!({}),
    )
    .unwrap();
    assert_eq!(result["status"], "failed");
    assert_eq!(doc.to_snapshot_v2(), before_invalid);
}

#[test]
fn removing_unsupported_standards_preserves_only_existing_bytes_and_order() {
    use kentos_commands::{CallOp, ProjectStandardDocument};
    use kentos_native_application::registry;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap(),
    )
    .unwrap();
    let item = |id: &str| ProjectStandardDocument {
        text: json!({"format":"kentos.drawing-standard", "version":1,
        "id":id, "revision":"1", "standard":{"unsupportedRule":true}})
        .to_string(),
    };
    let (a, b, c) = (item("a"), item("b"), item("c"));
    let mut settings = doc.settings().clone();
    settings.standard_docs = vec![a.clone(), b.clone(), c.clone()];
    doc.set_settings(settings);
    let before = doc.to_snapshot_v2();
    for documents in [vec![c.clone(), a.clone()], vec![a.clone(), item("new")]] {
        let result = registry::run(
            &mut ExecutionContext::new(&mut doc),
            "cad.settings.set",
            CallOp::Execute,
            json!({"standardDocs":documents}),
        )
        .unwrap();
        assert_eq!(result["status"], "failed", "{result}");
        assert_eq!(doc.to_snapshot_v2(), before);
    }
    let result = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.settings.set",
        CallOp::Execute,
        json!({"standardDocs":[a,c]}),
    )
    .unwrap();
    assert_eq!(result["status"], "completed", "{result}");
    assert_eq!(doc.settings().standard_docs.len(), 2);
}

#[test]
fn project_standard_associations_are_validated_and_revision_checked() {
    use kentos_commands::{
        CallOp, STANDARD_PACKAGE_FORMAT, STANDARD_PACKAGE_VERSION, StandardPackage,
    };
    use kentos_native_application::registry;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let snapshot = DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap();
    let mut doc = Document::from_snapshot(snapshot).unwrap();
    let package = |id: &str| StandardPackage {
        format: STANDARD_PACKAGE_FORMAT.into(),
        version: STANDARD_PACKAGE_VERSION,
        id: id.into(),
        revision: "2026.1".into(),
        standard: kentos_commands::built_in_standard("mimari-ofis").unwrap(),
    };
    let a = json!({"text":serde_json::to_string(&package("office-a")).unwrap()});
    let b = json!({"text":serde_json::to_string(&package("office-b")).unwrap()});
    let before = doc.to_snapshot_v2();
    let revision = doc.revision().to_string();
    let input = json!({"standardDocs":[a.clone(),b.clone()],"expectedRevision":revision});
    let planned = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.settings.set",
        CallOp::Plan,
        input.clone(),
    )
    .unwrap();
    assert_eq!(planned["status"], "completed", "{planned}");
    assert_eq!(doc.to_snapshot_v2(), before);
    let written = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.settings.set",
        CallOp::Execute,
        input.clone(),
    )
    .unwrap();
    assert_eq!(written["status"], "completed", "{written}");
    assert_eq!(doc.settings().standard_docs.len(), 2);
    let saved = doc.to_snapshot_v2();
    let stale = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.settings.set",
        CallOp::Execute,
        input,
    )
    .unwrap();
    assert_eq!(stale["status"], "conflict", "{stale}");
    assert_eq!(doc.to_snapshot_v2(), saved);
    let invalid = json!({"standardDocs":[{"text":"{}"}]});
    let refused = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.settings.set",
        CallOp::Execute,
        invalid,
    )
    .unwrap();
    assert_eq!(refused["status"], "failed", "{refused}");
    assert_eq!(doc.to_snapshot_v2(), saved);
    let mut unrecognized = serde_json::to_value(package("unknown-rule")).unwrap();
    unrecognized["standard"]["unsupportedRule"] = json!(true);
    let refused = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.settings.set",
        CallOp::Execute,
        json!({"standardDocs":[{"text":unrecognized.to_string()}]}),
    )
    .unwrap();
    assert_eq!(refused["status"], "failed", "{refused}");
    assert_eq!(doc.to_snapshot_v2(), saved);
    let reordered = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.settings.set",
        CallOp::Execute,
        json!({"standardDocs":[b,a]}),
    )
    .unwrap();
    assert_eq!(reordered["status"], "completed", "{reordered}");
    let first: serde_json::Value =
        serde_json::from_str(&doc.settings().standard_docs[0].text).unwrap();
    assert_eq!(first["id"], "office-b");
    let removed = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.settings.set",
        CallOp::Execute,
        json!({"standardDocs":[]}),
    )
    .unwrap();
    assert_eq!(removed["status"], "completed", "{removed}");
    assert!(doc.settings().standard_docs.is_empty());
}

fn drawing() -> Document {
    let old = ["duvar", "kapi", "aks", "mobilya", "A-PLAN"];
    let layers: Vec<_> = (0..500u32)
        .map(|k| {
            json!({"id": format!("l{k}"), "name": format!("{}-{k}", old[(k % 5) as usize]),
                   "type":"layer","visible":true,"locked":false,"expanded":true,
                   "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]})
        })
        .collect();
    let entities: Vec<_> = (0..100_000u32)
        .map(|i| {
            let (x, y) = (f64::from(i % 300) * 3.0, f64::from(i / 300) * 3.0);
            let mut e = json!({"kind":"line","id": i + 1,"layerId": format!("l{}", i % 500),"attrs":{},
                   "a":{"x":x,"y":y},"b":{"x":x + 2.0,"y":y}});
            if i % 7 == 0 {
                e["color"] = json!("#FF0000");
            }
            e
        })
        .collect();
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Standart","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":layers,"activeLayer":"l4","entities":entities,
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

/// `cargo test --release -p kentos-native-application --test all standards_cmd::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    let mut doc = drawing();
    let t = Instant::now();
    let CommandResult::Completed { output, .. } = standards::execute_check(
        &mut ExecutionContext::new(&mut doc),
        StandardsCheck {
            name: Some("mimari-ofis".into()),
            standard: None,
        },
    ) else {
        panic!("not checked")
    };
    println!(
        "Standardı denetle: {} bulgu, {:.1} ms",
        output.issues.len(),
        t.elapsed().as_secs_f64() * 1e3
    );
    let input = StandardsApply {
        name: Some("mimari-ofis".into()),
        ..StandardsApply::default()
    };
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        standards::plan_apply(&ExecutionContext::new(&mut doc), &input)
    else {
        panic!("not planned")
    };
    println!(
        "Standardı uygula, plan: {} katman birleşir, {} nesne, {} katman eklenir, {:.1} ms",
        output.translations.len(),
        output.objects,
        output.created.len(),
        t.elapsed().as_secs_f64() * 1e3
    );
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        standards::execute_apply(&mut ExecutionContext::new(&mut doc), input)
    else {
        panic!("not applied")
    };
    println!(
        "Standardı uygula, yazma: {} katman birleşti, {:.1} ms",
        output.translations.len(),
        t.elapsed().as_secs_f64() * 1e3
    );
    assert!(output.changed);
}

#[test]
fn named_layer_pattern_is_reported_and_preserved_by_standard_application() {
    let raw = json!({"format":"kentos.document","version":1,"name":"Standart","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A-DUVAR","type":"layer","visible":true,
        "locked":false,"expanded":true,"style":{"color":"ink","lineType":"continuous","lineWeight":0.5,"linePattern":"WATER"},"children":[]}],
        "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}});
    let mut doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let standard = kentos_commands::built_in_standard("mimari-ofis").unwrap();
    let before = doc.to_snapshot_v2();
    let findings = standards::issues(&doc, &standard);
    let issue = findings
        .iter()
        .find(|i| i.layer.as_deref() == Some("a") && i.found.as_deref() == Some("WATER"))
        .unwrap();
    assert_eq!(
        issue.kind,
        kentos_commands::StandardIssueKind::LayerLineType
    );
    assert!(!issue.fixable);
    assert_eq!(doc.to_snapshot_v2(), before);
    let input = StandardsApply {
        name: Some("mimari-ofis".into()),
        restyle: true,
        ..Default::default()
    };
    assert!(matches!(
        standards::execute_apply(&mut ExecutionContext::new(&mut doc), input),
        CommandResult::Completed { .. }
    ));
    assert_eq!(
        doc.layers().get("a").unwrap().style.line_pattern.as_deref(),
        Some("WATER")
    );
    assert!(
        standards::issues(&doc, &standard)
            .iter()
            .any(|i| i.found.as_deref() == Some("WATER") && !i.fixable)
    );
}

#[test]
fn named_entity_and_allowed_layer_patterns_are_not_mistaken_for_bylayer() {
    let raw = json!({"format":"kentos.document","version":1,"name":"Standart","settings":{"srid":0,
      "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
      "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A-DETAY","type":"layer","visible":true,
      "locked":false,"expanded":true,"style":{"color":"ink","lineType":"continuous","lineWeight":0.5,"linePattern":"GAS"},"children":[]}],
      "activeLayer":"a","entities":[{"kind":"line","id":1,"layerId":"a","attrs":{},"linePattern":"WATER","a":{"x":0.,"y":0.},"b":{"x":2.,"y":0.}}],"styles":{"items":[],"categories":[]}});
    let doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let standard = kentos_commands::built_in_standard("mimari-ofis").unwrap();
    let before = doc.to_snapshot_v2();
    let findings = standards::issues(&doc, &standard);
    assert!(findings.iter().any(|i| i.layer.as_deref() == Some("a")
        && i.found.as_deref() == Some("GAS")
        && !i.fixable));
    assert!(
        findings
            .iter()
            .any(|i| i.kind == kentos_commands::StandardIssueKind::OwnLook && i.objects == 1)
    );
    let named = findings
        .iter()
        .find(|i| {
            i.kind == kentos_commands::StandardIssueKind::ObjectLineType
                && i.found.as_deref() == Some("WATER")
        })
        .unwrap();
    assert_eq!(named.objects, 1);
    assert!(!named.fixable);
    let mut allow_own = standard.clone();
    allow_own.by_layer = false;
    let findings = standards::issues(&doc, &allow_own);
    assert!(findings.iter().any(
        |i| i.kind == kentos_commands::StandardIssueKind::ObjectLineType
            && i.found.as_deref() == Some("WATER")
            && i.objects == 1
    ));
    assert!(
        !findings
            .iter()
            .any(|i| i.kind == kentos_commands::StandardIssueKind::OwnLook)
    );
    allow_own.line_types.clear();
    assert!(
        !standards::issues(&doc, &allow_own)
            .iter()
            .any(|i| i.kind == kentos_commands::StandardIssueKind::ObjectLineType)
    );
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn named_patterns_in_block_definitions_are_reported_once_even_without_insertions() {
    let raw = json!({"format":"kentos.document","version":1,"name":"Standart","settings":{"srid":0,
      "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
      "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A-DUVAR","type":"layer","visible":true,
      "locked":false,"expanded":true,"style":{"color":"ink","lineType":"continuous","lineWeight":0.5},"children":[]}],
      "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]},
      "blocks":[{"id":"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d2101","name":"Test","base":{"x":0,"y":0},
      "entities":[{"kind":"line","id":1,"layerId":"a","attrs":{},"linePattern":"WATER","a":{"x":0,"y":0},"b":{"x":1,"y":0}},
                  {"kind":"line","id":2,"layerId":"a","attrs":{},"linePattern":"WATER","a":{"x":0,"y":1},"b":{"x":1,"y":1}},
                  {"kind":"line","id":3,"layerId":"a","attrs":{},"lineType":"dotted","a":{"x":0,"y":2},"b":{"x":1,"y":2}}]}]});
    let doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let before = doc.to_snapshot_v2();
    let mut standard = kentos_commands::built_in_standard("mimari-ofis").unwrap();
    standard.by_layer = false;
    let issues = standards::issues(&doc, &standard);
    let found: Vec<_> = issues
        .iter()
        .filter(|i| {
            i.kind == kentos_commands::StandardIssueKind::ObjectLineType
                && i.subject == "Blok: Test"
                && i.found.as_deref() == Some("WATER")
        })
        .collect();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].objects, 0);
    assert!(found[0].message.contains("2 parça"));
    assert!(!found[0].fixable);
    assert!(issues.iter().any(|i| i.subject == "Blok: Test"
        && i.found.as_deref() == Some("dotted")
        && i.message.contains("1 parça")
        && !i.fixable));
    assert_eq!(doc.to_snapshot_v2(), before);
    standard.by_layer = true;
    let own = standards::issues(&doc, &standard);
    assert!(
        own.iter()
            .any(|i| i.kind == kentos_commands::StandardIssueKind::OwnLook
                && i.subject == "Blok: Test"
                && i.found.as_deref() == Some("3 parça")
                && i.expected.as_deref() == Some("Katmana göre")
                && !i.fixable)
    );
    standard.by_layer = false;
    standard.line_types.clear();
    assert!(
        !standards::issues(&doc, &standard)
            .iter()
            .any(|i| i.subject == "Blok: Test")
    );
}

#[test]
fn differing_style_heights_are_reported_without_changing_the_drawing() {
    let raw = json!({"format":"kentos.document","version":1,"name":"Standart","settings":{"srid":0,
      "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100,
      "textStyles":[{"id":"0192f5a0-0000-7000-8000-000000000001","name":"A-Yazı","font":"barlow","height":4}],
      "dimensionStyles":[{"id":"0192f5a0-0000-7000-8000-000000000011","name":"A-Ölçü","height":3}]},
      "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A-DUVAR","type":"layer","visible":true,
      "locked":false,"expanded":true,"style":{"color":"ink","lineType":"continuous","lineWeight":0.5},"children":[]}],
      "activeLayer":"a","entities":[],"styles":{"items":[],"categories":[]}});
    let doc =
        Document::from_snapshot(DocumentSnapshotV1::from_json(&raw.to_string()).unwrap()).unwrap();
    let before = doc.to_snapshot_v2();
    let standard = kentos_commands::built_in_standard("mimari-ofis").unwrap();
    let findings = standards::issues(&doc, &standard);
    for (kind, found) in [
        (kentos_commands::StandardIssueKind::TextStyleHeight, "4 mm"),
        (
            kentos_commands::StandardIssueKind::DimensionStyleHeight,
            "3 mm",
        ),
    ] {
        let issue = findings.iter().find(|i| i.kind == kind).unwrap();
        assert_eq!(issue.expected.as_deref(), Some("2.5 mm"));
        assert_eq!(issue.found.as_deref(), Some(found));
        assert!(!issue.fixable);
    }
    assert_eq!(doc.to_snapshot_v2(), before);

    let mut matching = standard.clone();
    matching
        .text_styles
        .iter_mut()
        .find(|s| s.name == "A-Yazı")
        .unwrap()
        .height = Some(4.0);
    matching
        .dimension_styles
        .iter_mut()
        .find(|s| s.name == "A-Ölçü")
        .unwrap()
        .height = 3.0;
    assert!(!standards::issues(&doc, &matching).iter().any(|i| matches!(
        i.kind,
        kentos_commands::StandardIssueKind::TextStyleHeight
            | kentos_commands::StandardIssueKind::DimensionStyleHeight
    )));
    matching
        .text_styles
        .iter_mut()
        .find(|s| s.name == "A-Yazı")
        .unwrap()
        .height = None;
    assert!(
        !standards::issues(&doc, &matching)
            .iter()
            .any(|i| i.kind == kentos_commands::StandardIssueKind::TextStyleHeight)
    );
}
