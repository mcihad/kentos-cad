//! The layer style window's counts as `fixtures/style/v1/tally.json` holds
//! them (written by hand; the web checks the same file in
//! `apps/web/src/ui/style/tally.test.ts`): what each category, class and
//! rule takes of the drawn objects, which categories are shadowed, and how
//! a field is written in an expression.

use std::path::PathBuf;

use kentos_contracts::Entity;
use kentos_geometry_core::store::Store;
use kentos_native_application::geometry::shape;
use kentos_native_style::classify::{ExprScope, numbers_per_object, values_of};
use kentos_native_style::renderer::{Category, GraduatedClass, Rule};
use kentos_native_style::tally::{
    RuleCount, category_tally, class_tally, drawable, field_token, rule_counts, shadowed,
};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/style/v1/tally.json");
    let text = std::fs::read_to_string(&path).expect("tally.json");
    serde_json::from_str(&text).expect("tally.json is JSON")
}

#[test]
fn counts_what_the_drawing_draws() {
    let f = fixture();
    assert_eq!(f["format"], "kentos.style-tally");
    assert_eq!(f["version"], 1);
    let layer_name = f["layerName"].as_str().unwrap().to_owned();
    let entities: Vec<Entity> = f["entities"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| serde_json::from_value(e.clone()).expect("entity"))
        .collect();
    let list: Vec<&Entity> = entities.iter().collect();
    let mut store = Store::new();
    store.put_many(entities.iter().map(|e| {
        let b = e.base();
        (f64::from(b.id), b.layer_id.as_str(), false, shape(e))
    }));
    let name = move |_: &str| layer_name.clone();
    let measures = |list: &[&Entity]| {
        let ids: Vec<f64> = list.iter().map(|e| f64::from(e.base().id)).collect();
        store.measures(&ids)
    };
    let scope = ExprScope {
        layer_name: &name,
        measures: &measures,
    };
    let drawn = drawable(&list);
    let mut problems = Vec::new();
    let mut check = |got: Value, want: &Value, what: String| {
        if got != *want {
            problems.push(format!("{what}:\n  got  {got}\n  want {want}"));
        }
    };

    for c in f["categories"].as_array().unwrap() {
        let categories: Vec<Category> =
            serde_json::from_value(c["categories"].clone()).expect("categories");
        let values = values_of(&list, c["expr"].as_str().unwrap(), &scope).values;
        let (counts, rest) = category_tally(&values, &drawn, &categories);
        let shadows: Vec<bool> = (0..categories.len())
            .map(|i| shadowed(&categories, i))
            .collect();
        check(
            json!({ "counts": counts, "rest": rest, "shadowed": shadows }),
            &c["expect"],
            format!("categories {}", c["id"]),
        );
    }

    for c in f["classes"].as_array().unwrap() {
        let classes: Vec<GraduatedClass> =
            serde_json::from_value(c["classes"].clone()).expect("classes");
        let numbers = numbers_per_object(&list, c["expr"].as_str().unwrap(), &scope).values;
        let (counts, rest) = class_tally(&numbers, &drawn, &classes);
        check(
            json!({ "counts": counts, "rest": rest }),
            &c["expect"],
            format!("classes {}", c["id"]),
        );
    }

    for r in f["rules"].as_array().unwrap() {
        let rules: Vec<Rule> = serde_json::from_value(r["rules"].clone()).expect("rules");
        let counts = rule_counts(&rules, &list, &scope);
        let got: Vec<Value> = r["expect"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                let path: Vec<usize> = serde_json::from_value(e["path"].clone()).expect("path");
                match counts.get(&path) {
                    Some(RuleCount::Count(n)) => json!({ "path": path, "count": n }),
                    Some(RuleCount::Error(m)) => json!({ "path": path, "error": m }),
                    None => json!({ "path": path }),
                }
            })
            .collect();
        check(json!(got), &r["expect"], format!("rules {}", r["id"]));
        assert_eq!(
            counts.len(),
            got.len(),
            "rules {}: every rule counted",
            r["id"]
        );
    }

    for t in f["fieldTokens"].as_array().unwrap() {
        let name = t["name"].as_str().unwrap();
        check(
            json!(field_token(name)),
            &t["token"],
            format!("field {name}"),
        );
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
