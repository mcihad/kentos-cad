//! The one resolver of what a caller reads and takes out holds to the shared cases the independent
//! reference writes (scripts/fixtures/reach_cases.py, fixtures/reach/v1/cases.json; docs/adr/0550 §1):
//! each field's level, each record's reach and the attributes it gives on screen, in a file and in a
//! copy, the layers narrowed, whether a copy stored as it was written may be given.

use kentos_contracts::{FormBody, LayerField, ProjectForm};
use kentos_expression::{Unit, Value as ExprValue, Variable};
use kentos_forms::reach::{Caller, Level, Reach};
use kentos_forms::{Attrs, Object};
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../../fixtures/reach/v1/cases.json");

fn form(f: &Value) -> ProjectForm {
    let items = |list: &Value| -> Vec<Value> {
        list.as_array()
            .expect("items")
            .iter()
            .map(|i| {
                json!({ "type": "field", "id": format!("i-{}", i["field"].as_str().unwrap()),
                        "field": i["field"], "redact": i["redact"] })
            })
            .collect()
    };
    let body =
        |list: Vec<Value>, scope: &Value| json!({ "revision": 1, "items": list, "scope": scope });
    let scope = f.get("scope").cloned().unwrap_or(Value::Null);
    let live = body(items(&f["items"]), &scope);
    let draft = match f.get("draftItems") {
        Some(d) => body(items(d), &scope),
        None => live.clone(),
    };
    let release = |version: u32, body: &Value| {
        let typed: FormBody = serde_json::from_value(body.clone()).expect("a body");
        json!({ "version": version, "digest": kentos_contracts::body_digest(&typed), "body": body })
    };
    let releases = if f["published"].as_bool().unwrap() {
        let mut list = vec![release(1, &live)];
        if let Some(later) = f.get("laterItems") {
            list.push(release(2, &body(items(later), &scope)));
        }
        json!(list)
    } else {
        json!([])
    };
    serde_json::from_value(
        json!({ "id": f["id"], "name": f["id"], "layer": f["layer"], "draft": draft,
        "releases": releases, "withdrawn": f["withdrawn"] }),
    )
    .expect("a form")
}

fn attrs(v: &Value) -> Attrs {
    serde_json::from_value(v.clone()).expect("attributes")
}

fn level(name: &str) -> Level {
    match name {
        "hidden" => Level::Hidden,
        "redacted" => Level::Redacted,
        "open" => Level::Open,
        other => panic!("a level: {other}"),
    }
}

#[test]
fn the_resolver_holds_to_the_shared_cases() {
    let all: Value = serde_json::from_str(CASES).expect("the cases");
    let cases = all["cases"].as_array().expect("cases");
    assert!(cases.len() >= 15);
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let fields: std::collections::BTreeMap<String, Vec<LayerField>> =
            serde_json::from_value(c["fields"].clone()).expect("fields");
        let forms: Vec<ProjectForm> = c["forms"].as_array().unwrap().iter().map(form).collect();
        let me = vec![Variable {
            name: "kullanici_eposta".into(),
            value: ExprValue::text(c["email"].as_str().unwrap().to_owned()),
            description: String::new(),
        }];
        let empty: Vec<LayerField> = Vec::new();
        for (who, caller) in [
            ("through_forms", Caller::THROUGH_FORMS),
            ("reader", Caller::READER),
            ("whole", Caller::WHOLE),
        ] {
            let want = &c["expect"][who];
            let reach = Reach::new(
                &forms,
                |l| fields.get(l).unwrap_or(&empty).as_slice(),
                &me,
                Unit::METRE,
                caller,
            )
            .withholding(fields.iter().map(|(l, f)| (l.as_str(), f.as_slice())));
            let at = format!("{name} / {who}");
            for (layer, attrs_) in want["levels"].as_object().unwrap() {
                for (attr, expected) in attrs_.as_object().unwrap() {
                    assert_eq!(
                        reach.level(layer, attr),
                        level(expected.as_str().unwrap()),
                        "{at}: {layer}.{attr}"
                    );
                }
            }
            for (r, expected) in c["rows"]
                .as_array()
                .unwrap()
                .iter()
                .zip(want["rows"].as_array().unwrap())
            {
                let layer = r["layer"].as_str().unwrap();
                let a = attrs(&r["attrs"]);
                let object = Object::default();
                assert_eq!(
                    reach.sees(layer, &a, &object),
                    expected["sees"].as_bool().unwrap(),
                    "{at}: sees {r}"
                );
                assert_eq!(
                    reach.readable(layer, &a),
                    attrs(&expected["readable"]),
                    "{at}: readable {r}"
                );
                assert_eq!(
                    reach.exportable(layer, &a),
                    attrs(&expected["exportable"]),
                    "{at}: exportable {r}"
                );
                assert_eq!(
                    reach.copy_attrs(layer, &a),
                    attrs(&expected["copy"]),
                    "{at}: copy {r}"
                );
            }
            let limited: Vec<String> = want["limited"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().to_owned())
                .collect();
            assert_eq!(reach.limited_layers(), limited, "{at}: limited");
            assert_eq!(
                reach.narrows_rows(),
                want["narrowsRows"].as_bool().unwrap(),
                "{at}: narrows"
            );
            assert_eq!(
                reach.has_redacted(),
                want["hasRedacted"].as_bool().unwrap(),
                "{at}: marks"
            );
            assert_eq!(
                reach.whole_copy_ok(),
                want["wholeCopyOk"].as_bool().unwrap(),
                "{at}: whole copy"
            );
        }
    }
}
