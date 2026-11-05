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
fn unknown_resource_style_fields_are_refused_before_normalization_or_execution() {
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

    for (section, unknown) in [
        ("textStyleDefinitions", "missingFontRule"),
        ("dimensionStyleDefinitions", "toleranceRule"),
    ] {
        let input = json!({
            "name": "mimari-ofis",
            "resources": {section: [{"name": "A-NOT", unknown: true}]}
        });
        let normalized = registry::normalize("cad.standards.check", input.clone()).unwrap_err();
        assert_eq!(normalized.code, "invalid_input");
        assert!(normalized.message.contains(unknown), "{}", normalized.message);
        for op in [CallOp::Validate, CallOp::Plan, CallOp::Execute] {
            let error = registry::run(
                &mut ExecutionContext::new(&mut doc),
                "cad.standards.check",
                op,
                input.clone(),
            )
            .unwrap_err();
            assert_eq!(error.code, "invalid_input");
            assert!(error.message.contains(unknown), "{}", error.message);
            assert_eq!(doc.to_snapshot_v2(), before);
            assert_eq!(doc.undo_labels(), undo);
        }
    }
}

#[test]
fn standard_conflicts_use_the_same_numeric_tolerance_as_the_checker() {
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
    let mut a = kentos_commands::built_in_standard("mimari-ofis").unwrap();
    a.text_styles[0].height = Some(2.5);
    let mut b = a.clone();
    b.layers[0].line_weight += 0.5e-9;
    b.text_styles[0].height = Some(2.5 + 0.5e-9);
    b.dimension_styles[0].height += 0.5e-9;
    let item = |id: &str, standard| ProjectStandardDocument {
        text: serde_json::to_string(&StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_VERSION,
            id: id.into(),
            revision: "1".into(),
            standard,
            resources: None,
        })
        .unwrap(),
    };
    let mut settings = doc.settings().clone();
    settings.standard_docs = vec![item("a", a.clone()), item("b", b.clone())];
    doc.set_settings(settings);
    let report = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.conflicts",
        CallOp::Execute,
        json!({}),
    )
    .unwrap();
    assert_eq!(report["output"]["conflicts"], json!([]));
    b.layers[0].line_weight += 2e-9;
    b.text_styles[0].height = Some(2.5 + 2e-9);
    b.dimension_styles[0].height += 2e-9;
    let mut settings = doc.settings().clone();
    settings.standard_docs = vec![item("a", a), item("b", b)];
    doc.set_settings(settings);
    let report = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.conflicts",
        CallOp::Execute,
        json!({}),
    )
    .unwrap();
    assert_eq!(report["output"]["conflicts"].as_array().unwrap().len(), 3);
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
            resources: None,
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
    let provenance = &resolved["output"]["provenance"];
    assert_eq!(provenance["rules"], json!({"id":"first", "revision":"1"}));
    assert_eq!(provenance["layers"][&first.layers[0].name]["id"], "first");
    assert_eq!(provenance["layers"]["A-EK"]["id"], "second");
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
    assert_eq!(resolved["output"]["provenance"]["rules"]["id"], "second");
    assert_eq!(
        resolved["output"]["provenance"]["layers"][&second.layers[0].name]["id"],
        "second"
    );
    let mut settings = doc.settings().clone();
    settings.standard_docs[0].text = "{}".into();
    doc.set_settings(settings);
    assert_eq!(query(&mut doc)["status"], "failed");
}

#[test]
fn version_two_resource_definitions_resolve_by_order_and_report_content_conflicts() {
    use kentos_commands::{
        CallOp, ProjectStandardDocument, STANDARD_PACKAGE_FORMAT,
        STANDARD_PACKAGE_RESOURCES_VERSION, StandardPackage, StandardResources,
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
    let standard = kentos_commands::built_in_standard("mimari-ofis").unwrap();
    let first_line = serde_json::from_value(json!({"name":"OFFICE-DASH",
        "elements":[{"length":2.0},{"length":-1.0}]}))
    .unwrap();
    let later_line = serde_json::from_value(json!({"name":"office-dash",
        "elements":[{"length":3.0},{"length":-1.0}]}))
    .unwrap();
    let first_hatch = serde_json::from_value(json!({"name":"OFFICE-HATCH",
        "lines":[{"angle":45.0,"origin":[0.0,0.0],"offset":[0.0,2.0]}]}))
    .unwrap();
    let later_hatch = serde_json::from_value(json!({"name":"office-hatch",
        "lines":[{"angle":45.0,"origin":[0.0,0.0],"offset":[0.0,3.0]}]}))
    .unwrap();
    let document = |id: &str, line, hatch| ProjectStandardDocument {
        text: serde_json::to_string(&StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_RESOURCES_VERSION,
            id: id.into(),
            revision: "2".into(),
            standard: standard.clone(),
            resources: Some(StandardResources {
                line_types: vec![line],
                hatch_patterns: vec![hatch],
                blocks: vec![],
                sheet_templates: vec![],
                leader_styles: vec![],
                ..Default::default()
            }),
        })
        .unwrap(),
    };
    let first = document("office-a", first_line, first_hatch);
    let later = document("office-b", later_line, later_hatch);
    let input = json!({"standardDocs":[first.clone(), later.clone()],
        "expectedRevision":doc.revision().to_string()});
    let before_binding = doc.to_snapshot_v2();
    let planned = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.settings.set",
        CallOp::Plan,
        input.clone(),
    )
    .unwrap();
    assert_eq!(planned["status"], "completed", "{planned}");
    assert_eq!(doc.to_snapshot_v2(), before_binding);
    let bound = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.settings.set",
        CallOp::Execute,
        input,
    )
    .unwrap();
    assert_eq!(bound["status"], "completed", "{bound}");
    let before = doc.to_snapshot_v2();
    let bytes = kentos_kcad::encode_verified(&before).unwrap();
    let reopened = kentos_kcad::decode(&bytes).unwrap();
    assert_eq!(
        reopened.settings.standard_docs,
        before.settings.standard_docs
    );
    let read = |doc: &mut Document, command| {
        registry::run(
            &mut ExecutionContext::new(doc),
            command,
            CallOp::Execute,
            json!({}),
        )
        .unwrap()
    };
    let resolved = read(&mut doc, "cad.standards.resolve");
    assert_eq!(
        resolved["output"]["resources"]["lineTypes"][0]["elements"][0]["length"],
        2.0
    );
    assert_eq!(
        resolved["output"]["resources"]["hatchPatterns"][0]["lines"][0]["offset"][1],
        2.0
    );
    assert_eq!(
        resolved["output"]["provenance"]["lineTypes"]["OFFICE-DASH"]["id"],
        "office-a"
    );
    let conflicts = read(&mut doc, "cad.standards.conflicts");
    let items = conflicts["output"]["conflicts"].as_array().unwrap();
    assert_eq!(items.len(), 2, "{conflicts}");
    assert!(
        items
            .iter()
            .any(|item| item["kind"] == "lineTypeDefinition"
                && item["fields"] == json!(["elements"]))
    );
    assert!(
        items
            .iter()
            .any(|item| item["kind"] == "hatchPatternDefinition"
                && item["fields"] == json!(["lines"]))
    );
    assert_eq!(
        doc.to_snapshot_v2(),
        before,
        "resolution and conflict queries are read-only"
    );
    let revision = doc.revision().to_string();
    let reordered = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.settings.set",
        CallOp::Execute,
        json!({"standardDocs":[later, first], "expectedRevision":revision}),
    )
    .unwrap();
    assert_eq!(reordered["status"], "completed", "{reordered}");
    let reversed = read(&mut doc, "cad.standards.resolve");
    assert_eq!(
        reversed["output"]["resources"]["lineTypes"][0]["elements"][0]["length"],
        3.0
    );
    assert_eq!(
        reversed["output"]["provenance"]["lineTypes"]["office-dash"]["id"],
        "office-b"
    );
}

#[test]
fn named_resources_are_checked_without_installing_or_replacing_drawing_definitions() {
    use kentos_commands::{CallOp, StandardResources};
    use kentos_native_application::registry;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/commands/v1/cad.blocks.find.json"
    ))
    .unwrap();
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).unwrap(),
    )
    .unwrap();
    let line = serde_json::from_value(
        json!({"name":"OFFICE-DASH", "elements":[{"length":2.0},{"length":-1.0}]}),
    )
    .unwrap();
    let hatch = serde_json::from_value(json!({"name":"OFFICE-HATCH", "lines":[{"angle":45.0,"origin":[0.0,0.0],"offset":[0.0,2.0]}]})).unwrap();
    let resources = StandardResources {
        line_types: vec![line],
        hatch_patterns: vec![hatch],
        blocks: vec![],
        sheet_templates: vec![],
        leader_styles: vec![],
        ..Default::default()
    };
    let input = json!({"standard":kentos_commands::built_in_standard("mimari-ofis").unwrap(), "resources":resources});
    let kinds = |doc: &mut Document| {
        let before = doc.to_snapshot_v2();
        let undo = doc.undo_labels();
        let planned = registry::run(
            &mut ExecutionContext::new(doc),
            "cad.standards.check",
            CallOp::Plan,
            input.clone(),
        )
        .unwrap();
        let executed = registry::run(
            &mut ExecutionContext::new(doc),
            "cad.standards.check",
            CallOp::Execute,
            input.clone(),
        )
        .unwrap();
        assert_eq!(planned["output"], executed["output"]);
        assert_eq!(doc.to_snapshot_v2(), before);
        assert_eq!(doc.undo_labels(), undo);
        executed["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|issue| issue["kind"].as_str())
            .filter(|kind| kind.contains("Definition"))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        kinds(&mut doc),
        ["missingLineTypeDefinition", "missingHatchPatternDefinition"]
    );
    let mut settings = doc.settings().clone();
    settings.line_types = resources.line_types.clone();
    settings.hatch_patterns = resources.hatch_patterns.clone();
    settings.line_types[0].elements[0].length = 3.0;
    settings.hatch_patterns[0].lines[0].offset = [0.0, 3.0];
    doc.set_settings(settings);
    assert_eq!(
        kinds(&mut doc),
        ["lineTypeDefinition", "hatchPatternDefinition"]
    );
    let mut settings = doc.settings().clone();
    settings.line_types = resources.line_types.clone();
    settings.hatch_patterns = resources.hatch_patterns.clone();
    settings.line_types[0].name = "office-dash".into();
    settings.hatch_patterns[0].name = "office-hatch".into();
    settings.line_types[0].source = Some("drawing.lin".into());
    settings.hatch_patterns[0].description = "yerel açıklama".into();
    doc.set_settings(settings);
    assert!(kinds(&mut doc).is_empty());
}

#[test]
fn published_block_references_resolve_conflict_and_check_without_replacing_geometry() {
    use kentos_commands::{
        CallOp, DocumentSnapshotV2, ProjectStandardDocument, STANDARD_PACKAGE_FORMAT,
        STANDARD_PACKAGE_RESOURCES_VERSION, StandardBlockReference, StandardPackage,
        StandardResources,
    };
    use kentos_native_application::registry;

    let snapshot: DocumentSnapshotV2 = serde_json::from_str(include_str!(
        "../../../../../fixtures/kcad/v2/block-publication.json"
    ))
    .unwrap();
    let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
    let block_name = doc.blocks()[0].name.clone();
    let publication = doc.blocks()[0]
        .card
        .as_ref()
        .unwrap()
        .publication
        .clone()
        .unwrap();
    let reference = StandardBlockReference {
        name: block_name.clone(),
        publication: publication.clone(),
    };
    let standard = kentos_commands::built_in_standard("mimari-ofis").unwrap();
    let resources = |reference: StandardBlockReference| StandardResources {
        blocks: vec![reference],
        ..Default::default()
    };
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    let run_check = |doc: &mut Document, reference: StandardBlockReference| {
        registry::run(
            &mut ExecutionContext::new(doc),
            "cad.standards.check",
            CallOp::Execute,
            json!({"standard":standard,"resources":resources(reference)}),
        )
        .unwrap()
    };
    let equal = run_check(&mut doc, reference.clone());
    assert!(
        !equal["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["kind"].as_str().unwrap().contains("Block"))
    );
    let mut wrong = reference.clone();
    wrong.publication.release += 1;
    let mismatch = run_check(&mut doc, wrong.clone());
    assert!(
        mismatch["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["kind"] == "blockPublication"
                && i["fixable"] == false
                && i["objects"] == 3
                && i["message"].as_str().unwrap().contains("3 doğrudan yerleştirme"))
    );
    let mut missing = reference.clone();
    missing.name = "Ofis Kapısı".into();
    let missing_report = run_check(&mut doc, missing);
    assert!(
        missing_report["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| i["kind"] == "missingBlockReference" && i["fixable"] == false)
    );
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.undo_labels(), undo);

    let package = |id: &str, reference| ProjectStandardDocument {
        text: serde_json::to_string(&StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_RESOURCES_VERSION,
            id: id.into(),
            revision: "1".into(),
            standard: standard.clone(),
            resources: Some(resources(reference)),
        })
        .unwrap(),
    };
    let mut settings = doc.settings().clone();
    settings.standard_docs = vec![package("ofis-a", reference), package("ofis-b", wrong)];
    doc.set_settings(settings);
    let bound = doc.to_snapshot_v2();
    let resolved = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.resolve",
        CallOp::Execute,
        json!({}),
    )
    .unwrap();
    assert_eq!(
        resolved["output"]["resources"]["blocks"][0]["publication"]["release"],
        publication.release
    );
    assert_eq!(
        resolved["output"]["provenance"]["blocks"][&block_name]["id"],
        "ofis-a"
    );
    let conflicts = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.conflicts",
        CallOp::Execute,
        json!({}),
    )
    .unwrap();
    assert!(
        conflicts["output"]["conflicts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["kind"] == "blockReference" && c["fields"] == json!(["publication"]))
    );
    assert_eq!(doc.to_snapshot_v2(), bound);
    assert_eq!(doc.undo_labels(), undo);
}

#[test]
fn office_leader_style_checks_appearance_without_matching_local_ids() {
    use kentos_commands::{
        CallOp, LeaderStyleDef, ProjectStandardDocument, STANDARD_PACKAGE_FORMAT,
        STANDARD_PACKAGE_RESOURCES_VERSION, StandardPackage, StandardResources,
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
    let local: LeaderStyleDef = serde_json::from_value(json!({
        "id":"local-leader", "name":"A-NOT", "height":2.5,
        "arrow":"open", "arrowSize":1.5
    }))
    .unwrap();
    let mut settings = doc.settings().clone();
    settings.leader_styles = vec![local.clone()];
    doc.set_settings(settings);
    let office: LeaderStyleDef = serde_json::from_value(json!({
        "id":"office-leader", "name":"a-not", "height":2.5,
        "arrow":"open", "arrowSize":1.5
    }))
    .unwrap();
    let mut changed = office.clone();
    changed.height = 3.0;
    let standard = kentos_commands::built_in_standard("mimari-ofis").unwrap();
    let package = |id: &str, style: LeaderStyleDef| ProjectStandardDocument {
        text: serde_json::to_string(&StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_RESOURCES_VERSION,
            id: id.into(),
            revision: "2".into(),
            standard: standard.clone(),
            resources: Some(StandardResources {
                leader_styles: vec![style],
                ..Default::default()
            }),
        })
        .unwrap(),
    };
    let mut settings = doc.settings().clone();
    settings.standard_docs = vec![
        package("office-a", office.clone()),
        package("office-b", changed.clone()),
    ];
    doc.set_settings(settings);
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    let resolved = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.resolve",
        CallOp::Execute,
        json!({}),
    )
    .unwrap();
    assert_eq!(
        resolved["output"]["resources"]["leaderStyles"][0]["id"],
        "office-leader"
    );
    assert_eq!(
        resolved["output"]["provenance"]["leaderStyles"]["a-not"]["id"],
        "office-a"
    );
    let conflicts = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.conflicts",
        CallOp::Execute,
        json!({}),
    )
    .unwrap();
    assert!(
        conflicts["output"]["conflicts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["kind"] == "leaderStyleDefinition"
                && item["fields"] == json!(["height"]))
    );
    let check = |doc: &mut Document, style: LeaderStyleDef| {
        registry::run(
            &mut ExecutionContext::new(doc),
            "cad.standards.check",
            CallOp::Execute,
            json!({"standard":standard,"resources":StandardResources {
                leader_styles: vec![style], ..Default::default()
            }}),
        )
        .unwrap()
    };
    let equal = check(&mut doc, office.clone());
    assert!(
        !equal["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["kind"] == "leaderStyleDefinition"
                || issue["kind"] == "missingLeaderStyleDefinition")
    );
    let mut within_tolerance = office.clone();
    within_tolerance.height += 0.5e-9;
    let near = check(&mut doc, within_tolerance);
    assert!(
        !near["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["kind"] == "leaderStyleDefinition")
    );
    let different = check(&mut doc, changed);
    assert!(
        different["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["kind"] == "leaderStyleDefinition"
                && issue["expected"].as_str().unwrap().contains("height"))
    );
    let mut absent = office;
    absent.name = "A-OK".into();
    let missing = check(&mut doc, absent);
    assert!(
        missing["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["kind"] == "missingLeaderStyleDefinition")
    );
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.undo_labels(), undo);
}

#[test]
fn office_complete_annotation_styles_report_appearance_without_replacing_local_styles() {
    use kentos_commands::{
        CallOp, DimensionStyleDef, ProjectStandardDocument, STANDARD_PACKAGE_FORMAT,
        STANDARD_PACKAGE_RESOURCES_VERSION, StandardPackage, StandardResources, TextStyleDef,
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
    let local_text: TextStyleDef = serde_json::from_value(json!({
        "id":"local-text", "name":"A-NOT", "font":"arimo", "height":2.5,
        "widthFactor":0.9
    }))
    .unwrap();
    let local_dim: DimensionStyleDef = serde_json::from_value(json!({
        "id":"local-dim", "name":"A-DIM", "height":2.5,
        "dimLineColor":"#ff0000", "options":{"round":0.001}
    }))
    .unwrap();
    let mut settings = doc.settings().clone();
    settings.text_styles = vec![local_text.clone()];
    settings.dimension_styles = vec![local_dim.clone()];
    doc.set_settings(settings);
    let mut office_text = local_text.clone();
    office_text.id = "office-text".into();
    office_text.name = "a-not".into();
    let mut office_dim = local_dim.clone();
    office_dim.id = "office-dim".into();
    office_dim.name = "a-dim".into();
    let standard = kentos_commands::built_in_standard("mimari-ofis").unwrap();
    let package =
        |id: &str, text: TextStyleDef, dimension: DimensionStyleDef| ProjectStandardDocument {
            text: serde_json::to_string(&StandardPackage {
                format: STANDARD_PACKAGE_FORMAT.into(),
                version: STANDARD_PACKAGE_RESOURCES_VERSION,
                id: id.into(),
                revision: "2".into(),
                standard: standard.clone(),
                resources: Some(StandardResources {
                    text_style_definitions: vec![text],
                    dimension_style_definitions: vec![dimension],
                    ..Default::default()
                }),
            })
            .unwrap(),
        };
    let mut changed_text = office_text.clone();
    changed_text.width_factor = Some(0.8);
    let mut changed_dim = office_dim.clone();
    changed_dim.dim_line_color = Some("#0000ff".into());
    let mut settings = doc.settings().clone();
    settings.standard_docs = vec![
        package("office-a", office_text.clone(), office_dim.clone()),
        package("office-b", changed_text.clone(), changed_dim.clone()),
    ];
    doc.set_settings(settings);
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    let resolved = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.resolve",
        CallOp::Execute,
        json!({}),
    )
    .unwrap();
    assert_eq!(
        resolved["output"]["resources"]["textStyleDefinitions"][0]["id"],
        "office-text"
    );
    assert_eq!(
        resolved["output"]["provenance"]["dimensionStyleDefinitions"]["a-dim"]["id"],
        "office-a"
    );
    let conflicts = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.conflicts",
        CallOp::Execute,
        json!({}),
    )
    .unwrap();
    let entries = conflicts["output"]["conflicts"].as_array().unwrap();
    assert!(
        entries
            .iter()
            .any(|item| item["kind"] == "textStyleDefinition"
                && item["fields"] == json!(["widthFactor"]))
    );
    assert!(
        entries
            .iter()
            .any(|item| item["kind"] == "dimensionStyleDefinition"
                && item["fields"] == json!(["dimLineColor"]))
    );
    let check = |doc: &mut Document, text: TextStyleDef, dim: DimensionStyleDef| {
        registry::run(
            &mut ExecutionContext::new(doc),
            "cad.standards.check",
            CallOp::Execute,
            json!({"standard":standard,"resources":StandardResources {
                text_style_definitions: vec![text], dimension_style_definitions: vec![dim],
                ..Default::default()
            }}),
        )
        .unwrap()
    };
    let equal = check(&mut doc, office_text.clone(), office_dim.clone());
    let resource_kinds = [
        "textStyleDefinition",
        "dimensionStyleDefinition",
        "missingTextStyleDefinition",
        "missingDimensionStyleDefinition",
    ];
    assert!(
        !equal["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| resource_kinds.iter().any(|kind| issue["kind"] == *kind))
    );
    let mut near_text = office_text.clone();
    near_text.height = Some(2.5 + 0.5e-9);
    let near = check(&mut doc, near_text, office_dim.clone());
    assert!(
        !near["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["kind"] == "textStyleDefinition")
    );
    let different = check(&mut doc, changed_text, changed_dim);
    assert!(
        different["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["kind"] == "textStyleDefinition"
                && !issue["fixable"].as_bool().unwrap())
    );
    assert!(
        different["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["kind"] == "dimensionStyleDefinition"
                && !issue["fixable"].as_bool().unwrap())
    );
    let mut missing_text = office_text;
    missing_text.name = "A-OTHER".into();
    let missing = check(&mut doc, missing_text, office_dim);
    assert!(
        missing["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["kind"] == "missingTextStyleDefinition")
    );
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.undo_labels(), undo);
}

#[test]
fn office_sheet_template_revision_is_checked_without_rebuilding_the_sheet() {
    use kentos_commands::{
        CallOp, ProjectStandardDocument, STANDARD_PACKAGE_FORMAT,
        STANDARD_PACKAGE_RESOURCES_VERSION, StandardPackage, StandardResources,
        StandardSheetTemplateReference,
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
    let sheet_fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/drawing-set.json"
    ))
    .unwrap();
    let mut book: kentos_sheet::SheetBook =
        serde_json::from_value(sheet_fixture["cases"][0]["book"].clone()).unwrap();
    book.sheets[0].origin = Some(kentos_sheet::model::TemplateOrigin {
        template_id: "office-a3".into(),
        revision: 1,
    });
    doc.set_sheets(kentos_sheet::project::project_sheets(&book, |_| None).unwrap())
        .unwrap();
    let standard = kentos_commands::built_in_standard("mimari-ofis").unwrap();
    let package = |id: &str, revision: u32| ProjectStandardDocument {
        text: serde_json::to_string(&StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_RESOURCES_VERSION,
            id: id.into(),
            revision: "2".into(),
            standard: standard.clone(),
            resources: Some(StandardResources {
                sheet_templates: vec![StandardSheetTemplateReference {
                    template_id: "office-a3".into(),
                    revision,
                }],
                ..Default::default()
            }),
        })
        .unwrap(),
    };
    let mut settings = doc.settings().clone();
    settings.standard_docs = vec![package("ofis-a", 2), package("ofis-b", 3)];
    doc.set_settings(settings);
    let before = doc.to_snapshot_v2();
    let undo = doc.undo_labels();
    let resolved = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.resolve",
        CallOp::Execute,
        json!({}),
    )
    .unwrap();
    assert_eq!(
        resolved["output"]["resources"]["sheetTemplates"][0]["revision"],
        2
    );
    assert_eq!(
        resolved["output"]["provenance"]["sheetTemplates"]["office-a3"]["id"],
        "ofis-a"
    );
    let conflicts = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.conflicts",
        CallOp::Execute,
        json!({}),
    )
    .unwrap();
    assert!(
        conflicts["output"]["conflicts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["kind"] == "sheetTemplateReference"
                && item["fields"] == json!(["revision"]))
    );
    let input = json!({"standard":standard,
        "resources":resolved["output"]["resources"]});
    let report = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.check",
        CallOp::Execute,
        input,
    )
    .unwrap();
    assert!(
        report["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["kind"] == "sheetTemplateOrigin"
                && issue["found"] == "office-a3 · sürüm 1")
    );
    let matching = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.standards.check",
        CallOp::Execute,
        json!({"standard":standard,
        "resources": StandardResources {
            sheet_templates: vec![StandardSheetTemplateReference {
                template_id: "office-a3".into(), revision: 1,
            }], ..Default::default()
        }}),
    )
    .unwrap();
    assert!(
        !matching["output"]["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["kind"] == "sheetTemplateOrigin")
    );
    assert_eq!(doc.to_snapshot_v2(), before);
    assert_eq!(doc.undo_labels(), undo);
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
    second.layers[0].plot = !first.layers[0].plot;
    second.text_styles[0].height = Some(7.0);
    second.dimension_styles[0].height = 8.0;
    let package = |id: &str, standard| ProjectStandardDocument {
        text: serde_json::to_string(&StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_VERSION,
            id: id.into(),
            revision: "1".into(),
            standard,
            resources: None,
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
    assert_eq!(rows[0]["firstValues"]["byLayer"], json!(first.by_layer));
    assert_eq!(rows[0]["laterValues"]["byLayer"], json!(second.by_layer));
    assert_eq!(rows[1]["kind"], "layer");
    assert_eq!(rows[1]["fields"], json!(["color", "plot"]));
    assert_eq!(
        rows[1]["firstValues"]["color"],
        json!(first.layers[0].color)
    );
    assert_eq!(rows[1]["laterValues"]["color"], "#ff0000");
    assert_eq!(rows[1]["firstValues"]["plot"], json!(first.layers[0].plot));
    assert_eq!(rows[1]["laterValues"]["plot"], json!(second.layers[0].plot));
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
        resources: None,
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
    let duplicate = r#"{"format":"kentos.drawing-standard","version":1,"id":"duplicate","revision":"1","standard":{"name":"A","layers":[],"byLayer":true,"byLayer":false}}"#;
    let refused = registry::run(
        &mut ExecutionContext::new(&mut doc),
        "cad.settings.set",
        CallOp::Execute,
        json!({"standardDocs":[{"text":duplicate}]}),
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
            resources: None,
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
