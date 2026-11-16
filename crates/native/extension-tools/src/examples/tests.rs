//! The examples checked: a tool's values, attribute sets and words; a rule's findings.

use kentos_extensions::audit_examples::AuditExample;
use serde_json::json;

use super::*;

/// Numbers compare as numbers (text too, a comma for the point), text as written; an object holds an attribute set
/// when every key matches.
#[test]
fn an_expected_value_is_met_as_a_number_or_as_text() {
    assert!(same(&json!(1), &json!(1.0)));
    assert!(same(&json!("0,4"), &json!(0.4)));
    assert!(!same(&json!(2), &json!(1)));
    assert!(same(&json!("aşıyor"), &json!("aşıyor")) && !same(&json!("uygun"), &json!("aşıyor")));
    let attrs: Map<String, Value> =
        serde_json::from_value(json!({ "durum": "aşıyor", "taks": "0.5" })).unwrap();
    assert!(has(&attrs, &json!({ "durum": "aşıyor", "taks": 0.5 })));
    assert!(!has(&attrs, &json!({ "durum": "uygun" })));
    assert!(!has(&attrs, &json!({ "yok": 1 })));
    // How many passed, without a suffix after the number.
    assert_eq!(said(2, 2), "2 örneğin hepsi geçti.");
    assert_eq!(said(3, 2), "3 örnekten 2 tanesi geçti, 1 tanesi kaldı.");
    assert_eq!(said(1, 1), "Örnek geçti.");
    assert_eq!(said(2, 0), "2 örneğin hiçbiri geçmedi.");
}

fn square(x: f64) -> Value {
    json!({ "type": "Polygon",
            "coordinates": [[[x, 0.0], [x + 10.0, 0.0], [x + 10.0, 10.0], [x, 10.0], [x, 0.0]]] })
}

/// A rule's example is checked against the findings its run gives: how many, and the one it must have by its
/// severity, words, object (the example's place, its persistent id in the drawing) and fix; when one is missing, what
/// came is said; the architecture an example gives reaches the rule's facts; a failure of its code comes with its
/// traceback.
#[test]
fn a_rules_example_is_checked_against_its_findings() {
    let example: AuditExample = serde_json::from_value(json!({
        "title": "Çakışan iki parsel",
        "features": [{ "layer": "parsel", "geometry": square(0.0) },
                     { "layer": "parsel", "geometry": square(5.0) }],
        "architecture": { "levels": [], "exits": [], "rooms": [], "units": [], "doors": [], "sprinklers": true },
        "expect": { "count": 1, "has": [{ "severity": "error", "message": "çakışıyor", "feature": 1,
                                          "fix": "ext.tr.ornek.r.ayir" }] }
    }))
    .expect("an example");
    // The stand-in rule: names the second object it is given, and says whether the architecture reached it.
    let rule = |severity: &'static str| Rule {
        id: "ext.tr.ornek.r.cakisma".into(),
        title: "Parsel çakışması".into(),
        examples: vec![example.clone()],
        run: Box::new(move |invoke: Value| {
            let facts = &invoke["facts"];
            assert_eq!(invoke["kind"], "audit");
            assert_eq!(
                facts["project"]["architecture"]["sprinklers"], true,
                "{facts}"
            );
            let second = facts["layers"][0]["features"][1]["uid"].clone();
            Ok(
                json!({ "ok": true, "result": { "kind": "findings", "findings": [
                { "severity": severity, "message": "Parsel 2 parsel 1 ile çakışıyor.", "featureId": second,
                  "fix": { "title": "Ayır", "command": "ext.tr.ornek.r.ayir" } }] } }),
            )
        }),
    };
    let stop = AtomicBool::new(false);
    let passed = run(&[], &[rule("error")], json!({}), &stop);
    assert_eq!(passed["summary"], "Örnek geçti.", "{passed}");
    assert_eq!(passed["examples"][0]["findings"], 1);
    let failed = run(&[], &[rule("warning")], json!({}), &stop);
    let problems = failed["examples"][0]["problems"].to_string();
    assert!(
        problems.contains(
            "(error, “çakışıyor”, features[1], düzeltme ext.tr.ornek.r.ayir) bulgusu gelmedi"
        ) && problems.contains("gelenler: warning “Parsel 2 parsel 1 ile çakışıyor.”"),
        "{problems}"
    );
    // Its code failed: said with its traceback.
    let broken = Rule {
        run: Box::new(|_| {
            Ok(
                json!({ "ok": false, "exception": "KeyError", "message": "'alan'",
                       "error": "Traceback …\nKeyError: 'alan'" }),
            )
        }),
        ..rule("error")
    };
    let report = run(&[], &[broken], json!({}), &stop);
    let row = &report["examples"][0];
    assert_eq!(
        row["problems"][0], "kural hata verdi: KeyError: 'alan'",
        "{row}"
    );
    assert!(row["detail"].as_str().unwrap().contains("Traceback"));
}
