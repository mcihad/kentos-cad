//! A layer's fields (docs/adr/0199 §1) hold to the shared cases the
//! independent reference writes (scripts/fixtures/layer_field_cases.py,
//! fixtures/layer-fields/v1/cases.json): a value's canonical text or why
//! not, a field list's first problem and a value's display, word for word.
//! The inferred fields are the document's to compose (kentos-domain's
//! tests), with the geometry core's natural order.

use kentos_contracts::{
    LayerField, LayerFieldKind, check_value, compare_decimals, display_value, fields_problem,
    layer_fields_problem,
};
use serde_json::{Value, json};
use std::cmp::Ordering;

const CASES: &str = include_str!("../../../../../fixtures/layer-fields/v1/cases.json");

fn cases() -> Value {
    serde_json::from_str(CASES).expect("the cases")
}

fn field(v: &Value) -> LayerField {
    serde_json::from_value(v.clone()).expect("a field")
}

#[test]
fn values_take_their_canonical_text_or_are_refused_as_the_cases_say() {
    let cases = cases();
    let checks = cases["checks"].as_array().expect("checks");
    assert!(checks.len() > 50);
    for c in checks {
        let f = field(&c["field"]);
        let text = c["text"].as_str().expect("a text");
        let got = match check_value(&f, text) {
            Ok(v) => json!({ "value": v }),
            Err(r) => json!({ "error": r.code, "message": r.message }),
        };
        assert_eq!(got, c["want"], "{} ← {text:?}", f.name);
    }
}

#[test]
fn field_lists_have_the_first_problem_the_cases_say() {
    for c in cases()["problems"].as_array().expect("problems") {
        let fields: Vec<LayerField> = c["fields"]
            .as_array()
            .expect("fields")
            .iter()
            .map(field)
            .collect();
        let want = c["want"].as_str().map(str::to_owned);
        assert_eq!(fields_problem(&fields), want, "{}", c["name"]);
    }
}

#[test]
fn values_show_as_the_cases_say() {
    for c in cases()["display"].as_array().expect("display") {
        let f = field(&c["field"]);
        let v = c["value"].as_str().expect("a value");
        assert_eq!(
            display_value(&f, v),
            c["want"].as_str().expect("a text"),
            "{v}"
        );
    }
}

#[test]
fn a_stored_list_of_fields_is_never_empty() {
    assert!(layer_fields_problem(&[]).is_some());
    assert_eq!(
        layer_fields_problem(&[LayerField::new("Ad", LayerFieldKind::Text)]),
        None
    );
}

#[test]
fn decimals_compare_by_their_exact_values() {
    let order = |a, b| compare_decimals(a, b).expect("numbers");
    assert_eq!(order("0.5", "0.45"), Ordering::Greater);
    assert_eq!(order("12.50", "12.5"), Ordering::Equal);
    assert_eq!(order("-0.00", "0"), Ordering::Equal);
    assert_eq!(order("-2", "-10"), Ordering::Greater);
    assert_eq!(
        order(
            "123456789012345678901234567890",
            "123456789012345678901234567891"
        ),
        Ordering::Less
    );
    assert_eq!(order("-0.1", "0.05"), Ordering::Less);
    assert_eq!(compare_decimals("1e3", "1"), None);
}

#[test]
fn a_field_reads_and_writes_the_contract_shape() {
    let f = field(&json!({ "name": "Kat", "kind": "integer", "required": true, "default": "1" }));
    assert!(f.required);
    assert_eq!(f.default.as_deref(), Some("1"));
    let back = serde_json::to_value(&f).expect("json");
    assert_eq!(
        back,
        json!({ "name": "Kat", "kind": "integer", "required": true, "default": "1" })
    );
    let plain = serde_json::to_value(LayerField::new("Ad", LayerFieldKind::Text)).expect("json");
    assert_eq!(plain, json!({ "name": "Ad", "kind": "text" }));
}
