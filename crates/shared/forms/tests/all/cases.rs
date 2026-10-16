//! The forms core holds to the shared cases the independent reference writes
//! (scripts/fixtures/forms_cases.py, fixtures/forms/v1/cases.json): a form's preflight, a record's
//! items (what a choice filter offers, a note's text) and issues, a record completed (defaults,
//! calculations, hidden fields emptied), a write through the form, the write profile, a unique field,
//! the defaults and the form made of a layer's fields. A case's `variables` are the `@` values given,
//! its `object` the object's measures in metres. The issues are compared by their code, weight, item,
//! field and rule.

use kentos_contracts::{FormBody, FormIssue, LayerField};
use kentos_expression::{Measured, Value as ExprValue, Variable};
use kentos_forms::{Attrs, Object, OtherLayer, Prepared, World, generate, preflight_in};
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

/// The `@` values the case gives.
fn variables(c: &Value) -> Vec<Variable> {
    c["variables"]
        .as_array()
        .map(|vs| {
            vs.iter()
                .map(|v| Variable {
                    name: v["name"].as_str().expect("a name").to_owned(),
                    value: match &v["value"] {
                        Value::Number(n) => ExprValue::Num(n.as_f64().expect("a number")),
                        Value::String(s) => ExprValue::text(s.clone()),
                        other => panic!("a variable's value: {other}"),
                    },
                    description: String::new(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The other layers the case gives (docs/adr/0404): by id, their name, fields and objects.
fn world(c: &Value) -> World {
    let Some(all) = c["world"].as_object() else {
        return World::new();
    };
    all.iter()
        .map(|(id, l)| {
            let rows = l["rows"]
                .as_array()
                .expect("rows")
                .iter()
                .map(|r| (r[0].as_str().expect("a uid").to_owned(), attrs(&r[1])))
                .collect();
            (
                id.clone(),
                OtherLayer {
                    name: l["name"].as_str().expect("a name").to_owned(),
                    fields: serde_json::from_value(l["fields"].clone()).expect("fields"),
                    rows,
                },
            )
        })
        .collect()
}

/// The object the case gives: its measures in metres.
fn object(c: &Value) -> Object {
    let o = &c["object"];
    Object {
        measured: Measured {
            length: o["length"].as_f64(),
            area: o["area"].as_f64(),
            anchor: None,
        },
        vertices: o["vertices"].as_f64(),
        uid: o["uid"].as_str().map(str::to_owned),
        ..Object::default()
    }
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
    Prepared::new_with(
        &body(c),
        &fields(c),
        &variables(c),
        kentos_expression::Unit::METRE,
        &world(c),
    )
    .unwrap_or_else(|i| panic!("{}: {i:?}", c["name"]))
}

#[test]
fn the_preflight_is_as_the_cases_say() {
    for c in cases("preflight") {
        let got = bare(&preflight_in(
            &body(&c),
            &fields(&c),
            &variables(&c),
            &world(&c),
        ));
        assert_eq!(got, c["issues"], "{}", c["name"]);
    }
}

#[test]
fn a_record_s_items_and_issues_are_as_the_cases_say() {
    for c in cases("records") {
        let form = prepared(&c);
        let a = attrs(&c["attrs"]);
        let object = object(&c);
        assert_eq!(
            json!(form.described(&a, &object)),
            c["items"],
            "{}",
            c["name"]
        );
        assert_eq!(
            bare(&form.validate(&a, &object)),
            c["issues"],
            "{}",
            c["name"]
        );
    }
}

#[test]
fn a_record_is_completed_as_the_cases_say() {
    for c in cases("complete") {
        let form = prepared(&c);
        let new = c["new"].as_bool().expect("new");
        let (out, issues) = form.complete(&attrs(&c["attrs"]), &object(&c), new);
        assert_eq!(json!(out), c["out"], "{}", c["name"]);
        assert_eq!(bare(&issues), c["issues"], "{}", c["name"]);
    }
}

#[test]
fn a_write_through_the_form_is_as_the_cases_say() {
    for c in cases("assess") {
        let form = prepared(&c);
        let values: Vec<(String, String)> =
            serde_json::from_value(c["values"].clone()).expect("values");
        let new = c["new"].as_bool().unwrap_or(false);
        let object = object(&c);
        let (after, issues) = form.assess(&attrs(&c["before"]), &values, &object, new);
        assert_eq!(json!(after), c["after"], "{}", c["name"]);
        assert_eq!(bare(&issues), c["issues"], "{}", c["name"]);
        // A write goes in exactly when nothing is an error.
        let applied = form.apply(&attrs(&c["before"]), &values, &object, new);
        assert_eq!(
            applied.is_ok(),
            !kentos_forms::has_errors(&issues),
            "{}",
            c["name"]
        );
    }
}

#[test]
fn the_write_profile_is_as_the_cases_say() {
    for c in cases("enforce") {
        let form = prepared(&c);
        let issues = form.enforce(&attrs(&c["before"]), &attrs(&c["after"]), &object(&c));
        assert_eq!(bare(&issues), c["issues"], "{}", c["name"]);
    }
}

#[test]
fn a_unique_field_is_as_the_cases_say() {
    for c in cases("unique") {
        let form = prepared(&c);
        let others: Vec<Attrs> = c["others"]
            .as_array()
            .expect("others")
            .iter()
            .map(attrs)
            .collect();
        let issues = form.unique_issues(&attrs(&c["attrs"]), &object(&c), others.iter());
        assert_eq!(bare(&issues), c["issues"], "{}", c["name"]);
    }
}

#[test]
fn the_defaults_and_the_generated_forms_are_as_the_cases_say() {
    for c in cases("defaults") {
        let out = prepared(&c).with_defaults(&attrs(&c["attrs"]), &Object::default());
        assert_eq!(json!(out), c["out"], "{}", c["name"]);
    }
    for c in cases("generate") {
        assert_eq!(json!(generate(&fields(&c))), c["body"], "{}", c["name"]);
    }
}

#[test]
fn a_form_with_an_error_is_not_prepared() {
    let c = &cases("preflight")[1];
    let issues = Prepared::new(&body(c), &fields(c), &[]).expect_err("refused");
    // A list from another layer the world does not hold: refused, said by layer.
    let looks = cases("preflight")
        .into_iter()
        .find(|c| c["world"].as_object().is_some_and(|w| w.is_empty()))
        .expect("a case without the other layers");
    assert!(Prepared::new(&body(&looks), &fields(&looks), &[]).is_err());
    assert!(kentos_forms::has_errors(&issues));
}
