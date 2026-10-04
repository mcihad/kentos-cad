//! The layer style window's classes as `fixtures/style/v1/classify.json`
//! holds them (the web records it, `apps/web/scripts/fixtures/record-classify.test.ts`,
//! and checks it in `style/classifyFixture.test.ts`): values and categories,
//! numeric classes and their counts, ramps, plain symbols, labels, the class
//! count field and the texts. The desktop's window computes them here.

use std::path::PathBuf;

use kentos_contracts::Entity;
use kentos_geometry_core::store::Store;
use kentos_native_application::geometry::shape;
use kentos_native_style::classify::{
    CLASS_COUNT_DEFAULT, CLASS_COUNT_MAX, CLASS_COUNT_MIN, DEFAULT_RAMP, ExprScope, Method,
    NumericClass, OTHER_COLOR, Present, QUALITATIVE, RAMPS, categories_of, category_counts,
    class_count, class_label, classes_present, count_in, graduated_of, new_category, numbers_of,
    plain_symbols, ramp, ramp_colors, texts, unique_values, values_of,
};
use kentos_native_style::renderer::Category;
use serde_json::{Value, json};

fn fixture() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/style/v1/classify.json");
    let text = std::fs::read_to_string(&path).expect("classify.json");
    serde_json::from_str(&text).expect("classify.json is JSON")
}

fn present(v: &Value) -> Present {
    Present {
        fill: v["fill"].as_u64().unwrap() as usize,
        line: v["line"].as_u64().unwrap() as usize,
        marker: v["marker"].as_u64().unwrap() as usize,
    }
}

/// Numbers compared as numbers (1 and 1.0 alike), everything else as it is.
fn same(got: &Value, want: &Value) -> bool {
    match (got, want) {
        (Value::Number(a), Value::Number(b)) => a.as_f64() == b.as_f64(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same(x, y))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len() && a.iter().all(|(k, x)| b.get(k).is_some_and(|y| same(x, y)))
        }
        _ => got == want,
    }
}

fn check(got: &Value, want: &Value, what: &str, problems: &mut Vec<String>) {
    if !same(got, want) {
        problems.push(format!("{what}:\n  got  {got}\n  want {want}"));
    }
}

#[test]
fn makes_the_web_s_classes() {
    let f = fixture();
    assert_eq!(f["format"], "kentos.style-classify");
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
        (
            f64::from(b.id),
            b.layer_id.as_str(),
            b.label.as_deref().is_some_and(|l| !l.is_empty()),
            shape(e),
        )
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
    let fp = present(&f["present"]);
    let mut problems = Vec::new();

    // The constants and texts the window uses.
    let p = classes_present(&list);
    check(
        &json!({ "fill": p.fill, "line": p.line, "marker": p.marker }),
        &f["present"],
        "present",
        &mut problems,
    );
    check(
        &json!(QUALITATIVE),
        &f["qualitative"],
        "qualitative",
        &mut problems,
    );
    let ramps: serde_json::Map<String, Value> = RAMPS
        .iter()
        .map(|r| {
            (
                r.key.to_owned(),
                json!({ "label": r.label, "stops": r.stops }),
            )
        })
        .collect();
    check(&Value::Object(ramps), &f["ramps"], "ramps", &mut problems);
    check(
        &json!({
            "classCount": { "default": CLASS_COUNT_DEFAULT, "min": CLASS_COUNT_MIN, "max": CLASS_COUNT_MAX },
            "ramp": DEFAULT_RAMP,
            "method": "interval",
            "otherColor": OTHER_COLOR,
        }),
        &f["defaults"],
        "defaults",
        &mut problems,
    );
    check(
        &json!({
            "noValues": texts::NO_VALUES,
            "found": { "n": 7, "text": texts::found(7) },
            "noNumbers": texts::NO_NUMBERS,
            "classified": { "classes": 5, "values": 6, "text": texts::classified(5, 6) },
            "newCategory": texts::NEW_CATEGORY,
            "other": texts::OTHER,
            "categoriesHelp": texts::CATEGORIES_HELP,
            "classesHelp": texts::CLASSES_HELP,
            "classesEmptyHelp": texts::CLASSES_EMPTY_HELP,
        }),
        &f["texts"],
        "texts",
        &mut problems,
    );

    // Values of expressions, and their distinct values.
    for v in f["values"].as_array().unwrap() {
        let expr = v["expr"].as_str().unwrap();
        let r = values_of(&list, expr, &scope);
        let unique: Vec<Value> = unique_values(&r.values)
            .iter()
            .map(|u| json!({ "value": u.value, "count": u.count }))
            .collect();
        check(
            &json!({ "values": r.values, "unique": unique, "error": r.error }),
            &json!({ "values": v["values"], "unique": v["unique"], "error": v["error"] }),
            &format!("values of {expr}"),
            &mut problems,
        );
    }

    // Categories made from the values, keeping the ones there were.
    for c in f["categories"].as_array().unwrap() {
        let expr = c["expr"].as_str().unwrap();
        let old: Vec<Category> = serde_json::from_value(c["old"].clone()).expect("categories");
        let values = values_of(&list, expr, &scope).values;
        let made = categories_of(&unique_values(&values), &fp, &old);
        let (counts, rest) = category_counts(&values, &made);
        check(
            &json!({ "categories": made, "counts": { "counts": counts, "rest": rest } }),
            &c["expect"],
            &format!("categories {}", c["id"]),
            &mut problems,
        );
    }
    for n in f["newCategory"].as_array().unwrap() {
        let count = n["count"].as_u64().unwrap() as usize;
        check(
            &json!(new_category(count, &fp)),
            &n["category"],
            &format!("new category after {count}"),
            &mut problems,
        );
    }

    // Numeric classes and what each takes.
    for g in f["graduated"].as_array().unwrap() {
        let expr = g["expr"].as_str().unwrap();
        let method = match g["method"].as_str().unwrap() {
            "count" => Method::Count,
            _ => Method::Interval,
        };
        let n = g["n"].as_u64().unwrap() as usize;
        let numbers = numbers_of(&list, expr, &scope).values;
        let classes = graduated_of(&numbers, method, n, g["ramp"].as_str().unwrap(), &fp);
        let counts: Vec<usize> = classes
            .iter()
            .enumerate()
            .map(|(i, c)| count_in(&numbers, c.min, c.max, i == classes.len() - 1))
            .collect();
        check(
            &json!({ "numbers": numbers, "classes": classes, "counts": counts }),
            &g["expect"],
            &format!("classes of {expr} by {method:?}, {n}"),
            &mut problems,
        );
    }

    // Ramps, plain symbols, labels and the class count field.
    for r in f["rampColors"].as_array().unwrap() {
        let key = r["ramp"].as_str().unwrap();
        let n = r["n"].as_u64().unwrap() as usize;
        check(
            &json!(ramp_colors(ramp(key).stops, n)),
            &r["colors"],
            &format!("ramp {key} {n}"),
            &mut problems,
        );
    }
    for p in f["plain"].as_array().unwrap() {
        check(
            &json!(plain_symbols(
                p["color"].as_str().unwrap(),
                &present(&p["present"])
            )),
            &p["symbols"],
            &format!("plain symbols {}", p["present"]),
            &mut problems,
        );
    }
    for l in f["labels"].as_array().unwrap() {
        let c = NumericClass {
            min: l["min"].as_f64().unwrap(),
            max: l["max"].as_f64().unwrap(),
        };
        let digits = l["digits"].as_u64().map_or(2, |d| d as u32);
        check(
            &json!(class_label(c, digits)),
            &l["label"],
            &format!("label {} {}", c.min, c.max),
            &mut problems,
        );
    }
    for c in f["classCount"].as_array().unwrap() {
        let typed = c["typed"].as_str().unwrap();
        check(
            &json!(class_count(typed)),
            &c["count"],
            &format!("class count {typed:?}"),
            &mut problems,
        );
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
