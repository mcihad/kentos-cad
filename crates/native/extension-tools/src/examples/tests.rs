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

/// One rule's examples to run.
fn of_rules(rule: Rule) -> Examples {
    Examples {
        rules: vec![rule],
        ..Examples::default()
    }
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
    let passed = run(&of_rules(rule("error")), json!({}), &stop);
    assert_eq!(passed["summary"], "Örnek geçti.", "{passed}");
    assert_eq!(passed["examples"][0]["findings"], 1);
    let failed = run(&of_rules(rule("warning")), json!({}), &stop);
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
    let report = run(&of_rules(broken), json!({}), &stop);
    let row = &report["examples"][0];
    assert_eq!(
        row["problems"][0], "kural hata verdi: KeyError: 'alan'",
        "{row}"
    );
    assert!(row["detail"].as_str().unwrap().contains("Traceback"));
}

/// An expression function's example: each call runs as a label calls it (a row of arguments) and its value is the one
/// expected; a call that gives another says both, a call whose code fails says why. A call without a title is named
/// as an expression writes it.
#[test]
fn an_expressions_example_is_checked_against_its_value() {
    use kentos_extensions::manifest::ExpressionExample;
    let example = |args: Value, expect: Value| -> ExpressionExample {
        serde_json::from_value(json!({ "args": args, "expect": expect })).expect("an example")
    };
    // The stand-in function: "ada/parsel", none when either is missing; "0" fails as its code would.
    let function = Expression {
        id: "ext.tr.ornek.x.ada-parsel".into(),
        title: "Ada/parsel".into(),
        examples: vec![
            example(json!(["101", "1"]), json!("101/1")),
            example(json!([null, "1"]), json!(null)),
            example(json!(["1", "2"]), json!("9/9")),
            example(json!(["0", "0"]), json!("0/0")),
        ],
        run: Box::new(|invoke: Value| {
            assert_eq!(invoke["kind"], "expression");
            let row = &invoke["rows"][0];
            if row[0] == "0" {
                return Ok(
                    json!({ "ok": true, "result": { "kind": "values", "values": [null],
                                  "failed": 1, "error": "ValueError: sıfır" } }),
                );
            }
            let value = match (row[0].as_str(), row[1].as_str()) {
                (Some(a), Some(p)) => json!(format!("{a}/{p}")),
                _ => Value::Null,
            };
            Ok(json!({ "ok": true, "result": { "kind": "values", "values": [value] } }))
        }),
    };
    let report = run(
        &Examples {
            expressions: vec![function],
            ..Examples::default()
        },
        json!({}),
        &AtomicBool::new(false),
    );
    assert_eq!(
        report["summary"], "4 örnekten 2 tanesi geçti, 2 tanesi kaldı.",
        "{report}"
    );
    let rows = report["examples"].as_array().unwrap();
    assert_eq!(
        rows[0]["title"],
        "ext.tr.ornek.x.ada-parsel(\"101\", \"1\")"
    );
    assert_eq!(rows[1]["status"], "passed", "null as expected");
    assert_eq!(rows[2]["problems"][0], "\"9/9\" beklendi, \"1/2\" geldi");
    assert_eq!(
        rows[3]["problems"][0],
        "işlev hata verdi: ValueError: sıfır"
    );
}
