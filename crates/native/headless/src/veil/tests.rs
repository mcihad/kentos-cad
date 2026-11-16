//! The veil (veil.rs, docs/adr/0738, 0743; docs/adr/8057 §103).

use super::*;
use serde_json::json;

fn veil() -> Veil {
    Veil::new(
        vec!["Malik".into(), "TC".into()],
        ["Ali Yılmaz".to_owned(), "12345678901".to_owned()],
    )
}

#[test]
fn an_attribute_map_hides_the_fields_and_keeps_the_rest() {
    let got = veil().cut(
        json!({ "entities": [{ "uid": "u", "entity": { "kind": "point", "id": 1,
        "attrs": { "Malik": "Ali Yılmaz", "Ada": "101", "TC": "12345678901" } } }] }),
    );
    assert_eq!(
        got["entities"][0]["entity"]["attrs"],
        json!({ "Malik": HIDDEN, "Ada": "101", "TC": HIDDEN })
    );
    // A structural key of the same name elsewhere is not an attribute.
    let named = Veil::new(vec!["name".into()], ["x".to_owned()]);
    assert_eq!(
        named.cut(json!({ "name": "Parseller" })),
        json!({ "name": "Parseller" })
    );
}

#[test]
fn a_quoted_hidden_value_is_written_hidden() {
    assert_eq!(
        veil().text("“TC” katmanda tekil olmalı; “12345678901” başka bir nesnede var."),
        format!("“TC” katmanda tekil olmalı; “{HIDDEN}” başka bir nesnede var.")
    );
    // An unclosed quote and a value not hidden stay.
    assert_eq!(veil().text("“Ada” “101"), "“Ada” “101");
}

/// fixtures/agent-veil/v1/cases.json, written by scripts/fixtures/agent_veil_cases.py from the ADR without
/// KentOS code.
#[test]
fn the_shared_cases_are_cut_alike() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/agent-veil/v1/cases.json"
    ))
    .expect("the cases");
    assert_eq!(data["hidden"], HIDDEN);
    let cases = data["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    for case in cases {
        let strings = |k: &str| -> Vec<String> {
            serde_json::from_value(case[k].clone()).expect("a list of texts")
        };
        let veil = Veil::new(strings("names"), strings("values"));
        assert_eq!(
            veil.cut(case["answer"].clone()),
            case["cut"],
            "{}",
            case["name"]
        );
    }
}

/// fixtures/agent-veil/v1/oracle.json, written by scripts/fixtures/agent_oracle_cases.py from docs/adr/0743 §1
/// without KentOS code.
#[test]
fn the_shared_cases_name_a_hidden_field_alike() {
    let data: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/agent-veil/v1/oracle.json"
    ))
    .expect("the cases");
    let cases = data["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20);
    for case in cases {
        let names: Vec<String> = serde_json::from_value(case["names"].clone()).expect("the names");
        let got = Veil::new(names, Vec::new())
            .named_in(&case["input"])
            .map(|(path, field)| json!({ "path": path, "field": field }))
            .unwrap_or(Value::Null);
        assert_eq!(got, case["named"], "{}", case["name"]);
    }
}

#[test]
fn a_join_a_conflict_and_a_statistic_hide_the_field() {
    let got = veil().cut(json!({
        "columns": [{ "name": "Ada", "field": "Ada" }, { "name": "Malik", "field": "Malik" }],
        "rows": [
            { "id": "a/b", "left": "a", "cells": ["101", "Ali Yılmaz"] },
            { "id": "c/", "left": "c", "cells": ["102", null] }
        ],
        "totals": [{ "column": 1, "sum": "3", "count": 1 }]
    }));
    assert_eq!(got["rows"][0]["cells"], json!(["101", HIDDEN]));
    assert_eq!(got["rows"][1]["cells"], json!(["102", null]));
    assert_eq!(
        got["totals"][0],
        json!({ "column": 1, "sum": HIDDEN, "count": HIDDEN })
    );
    let conflict = veil().cut(json!({ "name": "Malik", "layer": "Ali", "table": "Veli" }));
    assert_eq!(
        conflict,
        json!({ "name": "Malik", "layer": HIDDEN, "table": HIDDEN })
    );
    let stat = veil().cut(json!({ "field": "TC", "read": 3, "skipped": 0, "sum": "6" }));
    assert_eq!(
        stat,
        json!({ "field": "TC", "read": HIDDEN, "skipped": HIDDEN, "sum": HIDDEN })
    );
}

/// Every field hidden (docs/adr/8057 §103): every value in an attribute map; a value standing as words in a free text
/// (three letters and more, quoted or not, the longest first), never in what names an answer's parts; a call naming a
/// field refused with why, writing a value not. The fields a drawing hides keep their rules: only quoted values in
/// texts, and writing them is the owner's.
#[test]
fn every_field_hides_every_value_and_the_agent_may_write_one() {
    let veil = Veil::new_everything(
        vec!["Malik".into(), "Ada".into(), "Kod".into()],
        ["Ayşe Yılmaz".to_owned(), "101".to_owned(), "A".to_owned()],
    );
    assert!(veil.hides_everything());
    let summary = veil.summary(json!({ "objects": 4 }));
    assert_eq!(summary["valuesWithheld"], every::WITHHELD);
    assert!(
        Veil::new(vec!["Malik".into()], Vec::new())
            .summary(json!({}))
            .get("valuesWithheld")
            .is_none()
    );
    let cut = veil.cut(json!({
        "id": "101", "layerId": "Ayşe Yılmaz",
        "attrs": { "Malik": "Ayşe Yılmaz", "Ada": "101", "Kod": "A" },
        "message": "Ayşe Yılmaz iki parselde; 101 numaralı ada, A sınıfı, 1012 m².",
        "findings": [{ "message": "Malik “Ayşe Yılmaz” boş değil" }]
    }));
    assert_eq!(
        cut["attrs"],
        json!({ "Malik": HIDDEN, "Ada": HIDDEN, "Kod": HIDDEN })
    );
    assert_eq!(
        cut["message"],
        format!("{HIDDEN} iki parselde; {HIDDEN} numaralı ada, A sınıfı, 1012 m².")
    );
    assert_eq!(
        cut["findings"][0]["message"],
        format!("Malik “{HIDDEN}” boş değil")
    );
    assert_eq!(
        (&cut["id"], &cut["layerId"]),
        (&json!("101"), &json!("Ayşe Yılmaz"))
    );
    let refused = veil
        .refusal(&json!({ "layer": "bina", "filter": "Malik = 'Ali'" }))
        .expect("refused");
    assert!(
        refused
            .message
            .contains("öznitelik değerleri gönderilmiyor"),
        "{}",
        refused.message
    );
    assert!(
        veil.refusal(&json!({ "uids": ["u"], "attrs": { "Malik": "Yeni" } }))
            .is_none()
    );
    let hidden = Veil::new(vec!["Malik".into()], ["Ayşe Yılmaz".to_owned()]);
    assert!(
        hidden
            .refusal(&json!({ "attrs": { "Malik": "Yeni" } }))
            .is_some()
    );
    assert_eq!(
        hidden.text("Ayşe Yılmaz iki parselde"),
        "Ayşe Yılmaz iki parselde"
    );
}
