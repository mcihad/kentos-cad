//! Veride ara (docs/adr/0178) against the independent reference in
//! `fixtures/search/v1/cases.json` (`scripts/fixtures/data_search_cases.py`,
//! no KentOS code): `text::edit::matches` and the operation `dataSearch`,
//! called by name as the web calls it through WASM, give every case's
//! answer exactly (a missing field is null).

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use kentos_geometry_core::api::run_named;
use kentos_geometry_core::text::edit::matches;
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/search/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.search-fixtures");
    assert_eq!(file["version"], 1);
    file
}

/// Equal, a missing object field being null.
fn same(a: &Value, e: &Value) -> bool {
    match (a, e) {
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<&String> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();
            keys.into_iter().all(|k| {
                same(
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                )
            })
        }
        _ => a == e,
    }
}

#[test]
fn a_word_matches_as_the_reference() {
    let file = fixture();
    let cases = file["match"].as_array().expect("cases");
    assert!(cases.len() >= 100, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let got = matches(
            c["text"].as_str().unwrap(),
            c["pattern"].as_str().unwrap(),
            !c["matchCase"].as_bool().unwrap(),
            c["wholeWord"].as_bool().unwrap(),
        );
        if got != c["expected"].as_bool().unwrap() {
            failures.push(format!(
                "{}: {got} ≠ {}",
                c["name"].as_str().unwrap(),
                c["expected"]
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

#[test]
fn records_answer_as_the_reference() {
    let file = fixture();
    let drawings = file["drawings"].as_array().expect("drawings");
    let cases = file["search"].as_array().expect("cases");
    assert!(cases.len() >= 100, "{} cases", cases.len());
    let mut failures = Vec::new();
    for c in cases {
        let name = c["name"].as_str().unwrap();
        let records = &drawings[c["drawing"].as_u64().unwrap() as usize];
        let got = match run_named("dataSearch", &json!([records, c["query"]]).to_string()) {
            Ok(text) => serde_json::from_str::<Value>(&text).expect("answer JSON"),
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };
        if !same(&got, &c["expected"]) {
            failures.push(format!("{name}: {got} ≠ {}", c["expected"]));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} cases differ:\n{}",
        failures.len(),
        cases.len(),
        failures.join("\n")
    );
}

#[test]
fn a_word_may_ask_nothing_and_the_core_says_so() {
    let records = json!([{ "kind": "Nokta", "layer": "N", "label": "P1", "attrs": [] }]);
    let query = |pattern: &str| {
        json!({
            "pattern": pattern, "matchCase": false, "wholeWord": false,
            "fields": { "label": true, "text": true, "block": true, "attrs": true, "attrName": null },
            "sort": null, "descending": false, "limit": 0
        })
    };
    let ask = |pattern: &str| -> Value {
        let text = run_named("dataSearch", &json!([records, query(pattern)]).to_string())
            .expect("a search");
        serde_json::from_str(&text).expect("JSON")
    };
    assert_eq!(ask("  ")["total"], 0);
    assert_eq!(ask("p1")["total"], 1);
    // A record without optional fields is read as it stands.
    assert!(ask("p1")["rows"][0]["name"].is_null());
    // A malformed query is an error, not a panic.
    assert!(run_named("dataSearch", &json!([records, {}]).to_string()).is_err());
}
