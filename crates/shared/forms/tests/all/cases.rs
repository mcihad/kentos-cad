//! The forms core holds to the shared cases the independent reference writes
//! (scripts/fixtures/forms_cases.py, fixtures/forms/v1/cases.json): a form's preflight, a record's
//! items and issues, a write through the form, the write profile, the defaults and the form made
//! of a layer's fields. The issues are compared by their code, weight, item, field and rule.

use kentos_contracts::{FormBody, FormIssue, LayerField};
use kentos_forms::{Attrs, Object, Prepared, generate, preflight};
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../../fixtures/forms/v1/cases.json");

fn cases(group: &str) -> Vec<Value> {
    let all: Value = serde_json::from_str(CASES).expect("the cases");
    let list = all[group].as_array().expect(group).clone();
    assert!(!list.is_empty(), "{group}");
    list
}

fn fields(c: &Value) -> Vec<LayerField> {
    serde_json::from_value(c["fields"].clone()).expect("the fields")
}

fn body(c: &Value) -> FormBody {
    serde_json::from_value(c["body"].clone()).expect("the body")
}

fn attrs(v: &Value) -> Attrs {
    serde_json::from_value(v.clone()).expect("attributes")
}

/// The issues without their sentences, which are the core's own.
fn bare(issues: &[FormIssue]) -> Value {
    let mut v = serde_json::to_value(issues).expect("json");
    for i in v.as_array_mut().expect("a list") {
        let m = i.as_object_mut().expect("an issue").remove("message");
        assert!(
            m.is_some_and(|m| m.as_str().is_some_and(|s| !s.is_empty())),
            "{i}"
        );
    }
    v
}

fn prepared(c: &Value) -> Prepared {
    Prepared::new(&body(c), &fields(c), &[]).unwrap_or_else(|i| panic!("{}: {i:?}", c["name"]))
}

#[test]
fn the_preflight_is_as_the_cases_say() {
    for c in cases("preflight") {
        let got = bare(&preflight(&body(&c), &fields(&c), &[]));
        assert_eq!(got, c["issues"], "{}", c["name"]);
    }
}

#[test]
fn a_record_s_items_and_issues_are_as_the_cases_say() {
    for c in cases("records") {
        let form = prepared(&c);
        let a = attrs(&c["attrs"]);
        let object = Object::default();
        assert_eq!(json!(form.states(&a, &object)), c["items"], "{}", c["name"]);
        assert_eq!(
            bare(&form.validate(&a, &object)),
            c["issues"],
            "{}",
            c["name"]
        );
    }
}

#[test]
fn a_write_through_the_form_is_as_the_cases_say() {
    for c in cases("assess") {
        let form = prepared(&c);
        let values: Vec<(String, String)> =
            serde_json::from_value(c["values"].clone()).expect("values");
        let (after, issues) = form.assess(&attrs(&c["before"]), &values, &Object::default());
        assert_eq!(json!(after), c["after"], "{}", c["name"]);
        assert_eq!(bare(&issues), c["issues"], "{}", c["name"]);
        // A write goes in exactly when nothing is wrong with it.
        let applied = form.apply(&attrs(&c["before"]), &values, &Object::default());
        assert_eq!(applied.is_ok(), issues.is_empty(), "{}", c["name"]);
    }
}

#[test]
fn the_write_profile_is_as_the_cases_say() {
    for c in cases("enforce") {
        let form = prepared(&c);
        let issues = form.enforce(
            &attrs(&c["before"]),
            &attrs(&c["after"]),
            &Object::default(),
        );
        assert_eq!(bare(&issues), c["issues"], "{}", c["name"]);
    }
}

#[test]
fn the_defaults_and_the_generated_forms_are_as_the_cases_say() {
    for c in cases("defaults") {
        let out = prepared(&c).with_defaults(&attrs(&c["attrs"]));
        assert_eq!(json!(out), c["out"], "{}", c["name"]);
    }
    for c in cases("generate") {
        assert_eq!(json!(generate(&fields(&c))), c["body"], "{}", c["name"]);
    }
}

#[test]
fn a_form_with_an_error_is_not_prepared() {
    let c = &cases("preflight")[1];
    let issues = Prepared::new(&body(c), &fields(c), &[])
        .err()
        .expect("refused");
    assert!(kentos_forms::has_errors(&issues));
}
