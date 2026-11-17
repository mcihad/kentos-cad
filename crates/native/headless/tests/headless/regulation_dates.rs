//! A headless audit's regulation group with the standards package's Python rules (docs/adr/8078; plan/bim-plan.md
//! BIM-13): `cad.arch.audit` over the protocol, as MCP and Python call it, lists the national regulation's findings
//! beside the Rust ones and says its rule set. The same drawing (three storeys of flats over a basement of storage
//! annexes and a boiler room, the elevator's shaft on the storeys only) applied for on 30.6.2026 passes 34/1 as of
//! 11.7.2021; applied for on 2.7.2026 it fails 34/8 as of 1.7.2026; with no date it is eksik veri.

use kentos_commands::{CallEnvelope, CallOp, CommandResult};
use kentos_domain::Document;
use serde_json::{Value, json};

fn run(doc: &mut Document, name: &str, input: Value) {
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        name,
        CallOp::Execute,
        input,
    )
    .unwrap();
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
}

/// The acceptance drawing, built with KentOS's commands from the audit facts' empty drawing.
fn basement_with_annexes() -> Document {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/audit-facts/v1/mimari.kcad");
    let mut snap: kentos_commands::DocumentSnapshotV2 = kentos_kcad::decode(&std::fs::read(path).unwrap()).unwrap();
    snap.entities.clear();
    snap.uids.clear();
    if let Some(arch) = snap.settings.arch.as_mut() {
        arch.levels.clear();
    }
    let mut doc = Document::from_snapshot_v2(snap).unwrap();
    let level = |id: &str, name: &str, z: f64| json!({ "id": id, "name": name, "elevation": z, "height": 3.0 });
    run(
        &mut doc,
        "cad.settings.set",
        json!({ "arch": { "levels": [
            level("b01", "Bodrum kat", -3.0), level("k00", "Zemin kat", 0.0), level("k01", "1. kat", 3.0),
            level("k02", "2. kat", 6.0)
        ] } }),
    );
    run(
        &mut doc,
        "cad.program.edit",
        json!({"program": {"units": [
            {"id": "d1", "number": "1", "nature": "Daire"},
            {"id": "d2", "number": "2", "nature": "Daire"},
            {"id": "d3", "number": "3", "nature": "Daire"},
            {"id": "e1", "number": "E1", "kind": "annex", "annexOf": "d1", "nature": "Depo"},
            {"id": "e2", "number": "E2", "kind": "annex", "annexOf": "d2", "nature": "Depo"},
            {"id": "o", "number": "O", "name": "Ortak alan", "kind": "common"}
        ]}}),
    );
    let layer = doc.layers().active().to_owned();
    let p = |x: f64, y: f64| json!({"x": x, "y": y});
    let room = |x0: f64, name: &str, level: &str, unit: &str, void: bool| {
        json!({"parts": [{"outer": {"pts": [p(x0, 0.0), p(x0 + 4.0, 0.0), p(x0 + 4.0, 4.0), p(x0, 4.0)]}}],
               "name": name, "unit": unit, "void": void, "arch": {"level": level}})
    };
    let mut spaces = vec![
        room(0.0, "Depo", "b01", "e1", false),
        room(4.0, "Depo", "b01", "e2", false),
        room(8.0, "Kazan dairesi", "b01", "o", false),
    ];
    for (level, unit) in [("k00", "d1"), ("k01", "d2"), ("k02", "d3")] {
        spaces.push(room(0.0, "Salon", level, unit, false));
        spaces.push(room(4.0, "Yatak odası", level, unit, false));
        spaces.push(room(8.0, "Merdiven holü", level, "o", false));
        spaces.push(room(12.0, "Asansör", level, "o", true));
    }
    run(&mut doc, "cad.spaces.create", json!({"layerId": layer, "spaces": spaces}));
    doc
}

fn dated(doc: &mut Document, date: Option<&str>) {
    let mut settings = doc.settings().clone();
    settings.variables.retain(|v| v.name != "basvuru_tarihi");
    if let Some(d) = date {
        settings.variables.push(kentos_commands::ProjectVariable {
            name: "basvuru_tarihi".into(),
            label: "Başvuru tarihi".into(),
            kind: kentos_commands::VariableKind::Text,
            value: kentos_commands::VariableValue::Text(d.into()),
        });
    }
    doc.set_settings(settings);
}

/// The regulation group asked of the whole drawing over the protocol: its output.
fn audited(doc: &mut Document, input: Value) -> Value {
    let answer = kentos_headless::call_envelope(
        doc,
        None,
        CallEnvelope {
            command: "cad.arch.audit".into(),
            version: None,
            op: CallOp::Execute,
            request_id: None,
            expected: None,
            relax_constraints: false,
            input,
        },
    );
    match answer.result {
        CommandResult::Completed { output, warnings } => {
            assert!(warnings.iter().all(|w| w.code != "regulation_rule_unavailable"), "{warnings:?}");
            output
        }
        other => panic!("{other:?}"),
    }
}

fn package(output: &Value) -> Vec<(String, Option<String>, String)> {
    output["findings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| matches!(f["code"].as_str(), Some("paiy_elevator_missing" | "paiy_elevator_not_serving" | "regulation_date_needed")))
        .map(|f| (f["code"].as_str().unwrap().to_owned(), f["state"].as_str().map(str::to_owned), f["message"].as_str().unwrap().to_owned()))
        .collect()
}

fn outcomes(output: &Value) -> Vec<(String, Value, String)> {
    output["ruleSet"]["rules"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| (r["rule"].as_str().unwrap().to_owned(), r["version"].clone(), r["state"].as_str().unwrap().to_owned()))
        .collect()
}

#[test]
fn a_headless_audit_reads_the_elevator_articles_by_the_application_date() {
    let mut doc = basement_with_annexes();
    let ask = json!({"checks": ["regulation"]});
    dated(&mut doc, Some("30.06.2026"));
    let out = audited(&mut doc, ask.clone());
    assert_eq!(package(&out), Vec::new());
    assert_eq!(
        outcomes(&out),
        [
            ("PAİY 34/1".to_owned(), json!("11.7.2021"), "passed".to_owned()),
            ("PAİY 34/8".to_owned(), json!("11.7.2021"), "outOfScope".to_owned()),
        ]
    );
    assert_eq!(out["ruleSet"]["date"], "2026-06-30");
    dated(&mut doc, Some("02.07.2026"));
    let out = audited(&mut doc, ask.clone());
    let found = package(&out);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!(found[0].0, "paiy_elevator_not_serving");
    assert!(found[0].2.starts_with("Bodrum kat: asansör bu kata ulaşmıyor"), "{}", found[0].2);
    let finding = out["findings"].as_array().unwrap().iter().find(|f| f["code"] == "paiy_elevator_not_serving").unwrap();
    assert_eq!(finding["check"], "regulation");
    assert_eq!(finding["rule"], "PAİY 34/8 · RG 1.7.2026/33297");
    assert!(finding["key"].as_str().unwrap().starts_with("paiy_elevator_not_serving::Bodrum kat"));
    assert_eq!(outcomes(&out)[1], ("PAİY 34/8".to_owned(), json!("1.7.2026"), "failed".to_owned()));
    // It follows the regulation group's own findings, before any other group's.
    let checks: Vec<&str> = out["findings"].as_array().unwrap().iter().map(|f| f["check"].as_str().unwrap()).collect();
    assert!(checks.iter().all(|c| *c == "regulation"), "{checks:?}");
    dated(&mut doc, None);
    let out = audited(&mut doc, ask);
    let found = package(&out);
    assert_eq!(found.len(), 1, "{found:#?}");
    assert_eq!((found[0].0.as_str(), found[0].1.as_deref()), ("regulation_date_needed", Some("missingData")));
    assert!(out["ruleSet"].get("date").is_none());
    // Of Etkin kat, or without the regulation group, the rule is not asked.
    let out = audited(&mut doc, json!({"checks": ["regulation"], "within": {"levels": ["k00"]}}));
    assert!(out.get("ruleSet").is_none() && package(&out).is_empty(), "{out}");
    let out = audited(&mut doc, json!({"checks": ["walls"]}));
    assert!(out.get("ruleSet").is_none(), "{out}");
}
