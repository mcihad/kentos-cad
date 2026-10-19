//! STAC discovery's core against the independent reference (docs/adr/0528; scripts/fixtures/stac_cases.py): an
//! endpoint's normal form, a datetime's, an asset address without its signature, a search's request and what a
//! page of recorded answers reads as (three of the answers are real Earth Search answers, trimmed).

use std::path::PathBuf;

use kentos_contracts::{StacMethod, StacSearch};
use kentos_services::stac::{datetime, endpoint, href, read, request};
use serde_json::{Value, json};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures")
}

fn cases() -> Value {
    let text = std::fs::read_to_string(root().join("stac/v1/cases.json")).expect("the cases file");
    serde_json::from_str(&text).expect("JSON")
}

/// JSON equality where a number is its value (the file's `1` is the code's `1.0`).
fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.as_f64() == y.as_f64(),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same(p, q))
        }
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len() && x.iter().all(|(k, v)| y.get(k).is_some_and(|w| same(v, w)))
        }
        _ => a == b,
    }
}

#[test]
fn endpoints_have_one_normal_form() {
    let cases = cases();
    let list = cases["endpoints"].as_array().expect("endpoints");
    assert!(list.len() >= 10);
    for c in list {
        let input = c["input"].as_str().expect("input");
        let got = endpoint::normalize(input).ok();
        assert_eq!(got.as_deref(), c["normalized"].as_str(), "{input:?}");
    }
}

#[test]
fn datetimes_are_read_as_the_reference_reads_them() {
    let cases = cases();
    let list = cases["datetimes"].as_array().expect("datetimes");
    assert!(list.len() >= 20);
    for c in list {
        let input = c["input"].as_str().expect("input");
        let got = datetime::normalize(input).ok();
        assert_eq!(got.as_deref(), c["normalized"].as_str(), "{input:?}");
    }
}

#[test]
fn signatures_are_dropped_from_addresses() {
    let cases = cases();
    let list = cases["hrefs"].as_array().expect("hrefs");
    assert!(list.len() >= 10);
    for c in list {
        let input = c["input"].as_str().expect("input");
        let (got, signed) = href::stable(input);
        assert_eq!(Some(got.as_str()), c["href"].as_str(), "{input:?}");
        assert_eq!(Some(signed), c["signed"].as_bool(), "{input:?}");
    }
}

#[test]
fn a_search_is_the_request_the_reference_writes() {
    let cases = cases();
    let list = cases["requests"].as_array().expect("requests");
    assert!(list.len() >= 10);
    for c in list {
        let input: StacSearch = serde_json::from_value(c["input"].clone()).expect("a search");
        match request::plan(&input) {
            Ok(p) => {
                let want = &c["request"];
                let method = match p.request.method {
                    StacMethod::Get => "GET",
                    StacMethod::Post => "POST",
                };
                assert_eq!(Some(method), want["method"].as_str(), "{}", c["input"]);
                assert_eq!(
                    Some(p.request.url.as_str()),
                    want["url"].as_str(),
                    "{}",
                    c["input"]
                );
                assert_eq!(
                    p.request.body.as_deref(),
                    want["body"].as_str(),
                    "{}",
                    c["input"]
                );
            }
            Err(problem) => {
                assert_eq!(Some(problem.code), c["code"].as_str(), "{}", c["input"]);
                assert_eq!(
                    problem.path.as_deref(),
                    c["path"].as_str(),
                    "{}",
                    c["input"]
                );
            }
        }
    }
}

#[test]
fn recorded_pages_read_as_the_reference_reads_them() {
    let cases = cases();
    let list = cases["pages"].as_array().expect("pages");
    assert!(list.len() >= 8);
    let dir = root().join("services/v1/stac");
    for c in list {
        let file = c["file"].as_str().expect("file");
        let endpoint = c["endpoint"].as_str().expect("endpoint");
        let bytes = std::fs::read(dir.join(file)).expect("the recorded answer");
        let page = read::read_page(endpoint, &bytes);
        if c["unreadable"] == json!(true) {
            assert!(page.is_err(), "{file}");
            continue;
        }
        let page = page.unwrap_or_else(|e| panic!("{file}: {e}"));
        assert_eq!(
            Some(u64::from(page.returned)),
            c["returned"].as_u64(),
            "{file}"
        );
        assert_eq!(page.matched, c["matched"].as_u64(), "{file}");
        let got = serde_json::to_value(&page.items).expect("items");
        assert!(
            same(&got, &c["items"]),
            "{file}: items differ:\n{got}\n{}",
            c["items"]
        );
        let next = page.next.map(|n| serde_json::to_value(n).expect("next"));
        match (&next, c.get("next")) {
            (None, None) => {}
            (Some(g), Some(w)) => assert!(same(g, w), "{file}: next {g} vs {w}"),
            (g, w) => panic!("{file}: next {g:?} vs {w:?}"),
        }
    }
}
