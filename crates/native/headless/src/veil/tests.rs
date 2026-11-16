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

/// A text under a key that names a thing (a layer's id, a model's) is no field read, however it reads as an expression
/// (docs/adr/8057 §108); a model step's given value is read as its parameter's own.
#[test]
fn an_identifier_names_a_thing_not_a_field() {
    let v = Veil::new(vec!["parsel".into()], ["12".to_owned()]);
    let layer = json!({ "parameters": { "input": { "scope": "layer", "layerId": "parsel" } } });
    assert_eq!(v.named_in(&layer), None, "the parcels' layer");
    let model =
        json!({ "tool": "model:", "model": { "id": "parsel-tampon-birlesik", "steps": [] } });
    assert_eq!(v.named_in(&model), None, "a model named after the parcels");
    // A filter that reads the field still does.
    let filtered = json!({ "parameters": { "input": { "scope": "layer", "layerId": "parsel",
        "filter": "parsel = '12'" } } });
    assert_eq!(
        v.named_in(&filtered),
        Some(("parameters.input.filter".into(), "parsel".into()))
    );
    // A step grouping by the field, its value given in the model.
    let grouped = json!({ "model": { "steps": [{ "id": "b", "tool": "geometry.dissolve", "values": {
        "group": { "kind": "value", "value": "parsel" } } }] } });
    assert_eq!(
        v.named_in(&grouped),
        Some(("model.steps[0].values.group.value".into(), "parsel".into()))
    );
    // A step's features input given in the model, filtered by the field.
    let input = json!({ "model": { "steps": [{ "id": "t", "tool": "geometry.buffer", "values": {
        "input": { "kind": "value",
            "value": { "scope": "layer", "layerId": "parsel", "filter": "parsel > 3" } } } }] } });
    assert!(v.named_in(&input).is_some());
}

/// An İşlemler tool's expression and field parameters read by their kind, wherever they stand: Hesapla's expression
/// under `value`, a dissolve's sums with commas, a model's step, a batch's row (docs/adr/8057 §108).
#[cfg(feature = "mod-gis")]
#[test]
fn a_tools_expression_and_fields_read_wherever_they_stand() {
    let v = Veil::new(vec!["Malik".into()], ["Ali Yılmaz".to_owned()]);
    let read = |input: Value| super::reads::processing_read(&v, "cad.processing.run", &input);
    let calculate = json!({ "tool": "attributes.calculate",
        "parameters": { "field": "kopya", "value": "upper(Malik)" } });
    assert_eq!(
        read(calculate),
        Some(("parameters.value".into(), "Malik".into()))
    );
    let sums = json!({ "tool": "geometry.dissolve", "parameters": { "sums": "alan, Malik" } });
    assert_eq!(read(sums).map(|(at, _)| at), Some("parameters.sums".into()));
    let model = json!({ "tool": "model:", "model": { "steps": [{ "id": "h", "tool": "attributes.calculate",
        "values": { "field": { "kind": "value", "value": "kopya" },
            "value": { "kind": "value", "value": "Malik" } } }] } });
    assert_eq!(
        read(model).map(|(at, _)| at),
        Some("model.steps[0].values.value.value".into())
    );
    let batch = json!({ "tool": "attributes.calculate", "parameters": { "field": "kopya" },
        "rows": [{ "parameters": { "value": "'sabit'" } }, { "parameters": { "value": "Malik || ''" } }] });
    assert_eq!(
        super::reads::processing_read(&v, "cad.processing.batch", &batch).map(|(at, _)| at),
        Some("rows[1].parameters.value".into())
    );
    // An expression that reads no hidden field, and another command, pass.
    let fixed = json!({ "tool": "attributes.calculate",
        "parameters": { "field": "kopya", "value": "yuvarla($alan, 2)" } });
    assert_eq!(read(fixed), None);
    let other = json!({ "tool": "attributes.calculate", "parameters": { "value": "Malik" } });
    assert_eq!(
        super::reads::processing_read(&v, "cad.entities.set", &other),
        None
    );
}
