//! Denetle by its meaning over a large drawing (docs/adr/0306 §5): what the plan and the write take for 100 000
//! objects, 10 000 of them linked texts whose objects are gone, 10 000 texts naming a style the project lacks, with 20
//! text styles of which ten are unused. The shared cases are `fixtures/commands/v1/cad.drawing.audit.json`.

use kentos_domain::Document;
use kentos_commands::{AuditPurge, CommandResult, DocumentSnapshotV1, DrawingAudit};
use kentos_native_application::{ExecutionContext, drawing_audit};
use serde_json::json;

const GONE: &str = "0192f5a2-0000-7000-8000-00000000ffff";

fn style(i: u32) -> String {
    format!("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d{i:04}")
}

fn drawing() -> Document {
    let mut entities = Vec::new();
    for i in 0..100_000u32 {
        let (x, y) = (f64::from(i % 300) * 3.0, f64::from(i / 300) * 3.0);
        let mut e = match i % 10 {
            0 => {
                json!({"kind":"text","p":{"x":x,"y":y},"text":"101","height":1,"rotation":0,"labelOf":GONE,"labelScale":500})
            }
            1 => {
                json!({"kind":"text","p":{"x":x,"y":y},"text":"A","height":1,"rotation":0,"textStyle":style(9999)})
            }
            2 => {
                json!({"kind":"text","p":{"x":x,"y":y},"text":"B","height":1,"rotation":0,"textStyle":style((i / 10) % 10)})
            }
            _ => json!({"kind":"line","a":{"x":x,"y":y},"b":{"x":x + 2.0,"y":y}}),
        };
        e["id"] = json!(i + 1);
        e["layerId"] = json!("a");
        e["attrs"] = json!({});
        entities.push(e);
    }
    let styles: Vec<_> = (0..20)
        .map(|i| json!({"id": style(i), "name": format!("Stil {i}"), "font": "arimo"}))
        .collect();
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Denetle","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100,"textStyles":styles},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":entities,"styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn all() -> DrawingAudit {
    DrawingAudit {
        fix: true,
        purge: vec![AuditPurge::TextStyles],
        expected_revision: None,
    }
}

/// `cargo test --release -p kentos-native-application --test all audit_cmd::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    use std::time::Instant;
    let mut doc = drawing();
    let n = doc.len();
    let t = Instant::now();
    let CommandResult::Completed { output: plan, .. } = drawing_audit::plan_in(&doc, &all()) else {
        panic!("not completed")
    };
    let planned = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        drawing_audit::execute(&mut ExecutionContext::new(&mut doc), all())
    else {
        panic!("not completed")
    };
    let written = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(output.findings.len(), plan.findings.len());
    assert_eq!(output.changed.len(), 20_000);
    let purged = output.unused.iter().filter(|u| u.purged).count();
    assert_eq!(purged, 10);
    println!(
        "{n} nesne: plan {planned:.0} ms, yazma {written:.0} ms; {} bulgu, {} onarılan, {purged} stil",
        output.findings.len(),
        output.changed.len()
    );
}

#[test]
fn linetype_only_text_style_is_used_protected_renamed_and_undoable() {
    use kentos_commands::StylesEdit;
    use kentos_native_application::{style_tables, styles_edit};
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/native-preflight.json"
    ))
    .unwrap();
    let mut raw = fixture["model"].clone();
    let used = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0100";
    let unused = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101";
    raw["settings"]["textStyles"] = json!([{"id":used,"name":"UtilityText","font":"barlow"},{"id":unused,"name":"Unused","font":"barlow"}]);
    raw["settings"]["lineTypes"] = json!([
        {"name":"P","unit":"paper","elements":[{"length":2.},{"length":-1.},{"length":0.,"text":{"text":"HW","style":"utilitytext"}},{"length":-1.}]},
        {"name":"Q","unit":"paper","elements":[{"length":2.},{"length":-1.},{"length":0.,"text":{"text":"GAS","style":"UTILITYTEXT"}},{"length":-1.}]}]);
    let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
    let original = doc.to_snapshot_v2();
    assert_eq!(style_tables::usage(&doc, true)[used], 2);
    assert!(!doc.unused_styles().0.contains(&used.to_owned()));
    assert!(doc.unused_styles().0.contains(&unused.to_owned()));
    let input = |op: &str, name: Option<&str>| -> StylesEdit {
        let mut v = json!({"kind":"text","operation":op,"style":used});
        if let Some(name) = name {
            v["name"] = name.into();
        }
        serde_json::from_value(v).unwrap()
    };
    let remove = input("remove", None);
    for result in [
        serde_json::to_value(styles_edit::validate(
            &ExecutionContext::new(&mut doc),
            &remove,
        ))
        .unwrap(),
        serde_json::to_value(styles_edit::plan(&ExecutionContext::new(&mut doc), &remove)).unwrap(),
        serde_json::to_value(styles_edit::execute(
            &mut ExecutionContext::new(&mut doc),
            remove,
        ))
        .unwrap(),
    ] {
        assert_eq!(result["error"]["code"], "style_in_use", "{result}");
    }
    assert_eq!(doc.to_snapshot_v2(), original);
    let next = doc
        .settings()
        .text_styles
        .iter()
        .filter(|s| s.id != used)
        .cloned()
        .collect();
    assert!(
        !style_tables::save_text_styles(&mut doc, next)
            .said
            .is_empty()
    );
    assert_eq!(doc.to_snapshot_v2(), original);
    let rename = input("rename", Some("Utility Renamed"));
    assert!(matches!(
        styles_edit::plan(&ExecutionContext::new(&mut doc), &rename),
        CommandResult::Completed { .. }
    ));
    assert_eq!(doc.to_snapshot_v2(), original);
    assert!(matches!(
        styles_edit::execute(&mut ExecutionContext::new(&mut doc), rename),
        CommandResult::Completed { .. }
    ));
    assert!(
        doc.settings()
            .line_types
            .iter()
            .all(|p| p.elements[2].text.as_ref().unwrap().style == "Utility Renamed")
    );
    // A later independent edit of the dash length must survive style undo.
    let mut settings = doc.settings().clone();
    settings.line_types[0].elements[0].length = 9.;
    doc.set_settings(settings);
    doc.undo();
    assert_eq!(doc.settings().text_styles[0].name, "UtilityText");
    assert!(
        doc.settings()
            .line_types
            .iter()
            .all(|p| p.elements[2].text.as_ref().unwrap().style == "UtilityText")
    );
    assert_eq!(doc.settings().line_types[0].elements[0].length, 9.);
    doc.redo();
    assert_eq!(doc.settings().text_styles[0].name, "Utility Renamed");
    assert!(
        doc.settings()
            .line_types
            .iter()
            .all(|p| p.elements[2].text.as_ref().unwrap().style == "Utility Renamed")
    );
    assert_eq!(doc.settings().line_types[0].elements[0].length, 9.);
    let purge: DrawingAudit = serde_json::from_value(json!({"purge":["textStyles"]})).unwrap();
    let CommandResult::Completed { output, .. } =
        drawing_audit::execute(&mut ExecutionContext::new(&mut doc), purge)
    else {
        panic!("audit refused")
    };
    assert!(!output.unused.iter().any(|s| s.id == used));
    assert!(doc.settings().text_styles.iter().any(|s| s.id == used));
    assert!(!doc.settings().text_styles.iter().any(|s| s.id == unused));
}

#[test]
fn simultaneous_text_style_name_swap_keeps_each_linetype_reference_through_undo() {
    use kentos_native_application::style_tables;
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/sheet/v1/native-preflight.json"
    ))
    .unwrap();
    let mut raw = fixture["model"].clone();
    raw["settings"]["textStyles"] =
        json!([{"id":"a","name":"Alpha","font":"barlow"},{"id":"b","name":"Beta","font":"barlow"}]);
    raw["settings"]["lineTypes"] = json!([
        {"name":"P","unit":"paper","elements":[{"length":2.},{"length":-1.},{"length":0.,"text":{"text":"A","style":"Alpha"}},{"length":-1.}]},
        {"name":"Q","unit":"paper","elements":[{"length":2.},{"length":-1.},{"length":0.,"text":{"text":"B","style":"Beta"}},{"length":-1.}]}]);
    let mut doc = Document::from_snapshot(serde_json::from_value(raw).unwrap()).unwrap();
    let refs = |d: &Document| {
        d.settings()
            .line_types
            .iter()
            .map(|p| p.elements[2].text.as_ref().unwrap().style.clone())
            .collect::<Vec<_>>()
    };
    let mut next = doc.settings().text_styles.clone();
    next[0].name = "Beta".into();
    next[1].name = "Alpha".into();
    let saved = doc.with_style_tables("swap", |d| style_tables::save_text_styles(d, next));
    assert_eq!(
        refs(&doc),
        ["Beta", "Alpha"],
        "saved={saved:?}; styles={:?}",
        doc.settings().text_styles
    );
    doc.undo();
    assert_eq!(refs(&doc), ["Alpha", "Beta"]);
    doc.redo();
    assert_eq!(refs(&doc), ["Beta", "Alpha"]);
    // A concurrent style edit is retained. Reverting only its partner would
    // duplicate the occupied name, so neither references nor that name move.
    let mut foreign = doc.settings().clone();
    foreign.text_styles[0].bold = true;
    doc.set_settings(foreign);
    doc.undo();
    assert_eq!(doc.settings().text_styles[0].name, "Beta");
    assert!(doc.settings().text_styles[0].bold);
    assert_eq!(doc.settings().text_styles[1].name, "Alpha");
    assert_eq!(refs(&doc), ["Beta", "Alpha"]);
}
