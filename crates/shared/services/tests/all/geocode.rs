//! Adres arama's online providers (docs/adr/0229 §10) against the
//! independent reference in `fixtures/geocode/v1/online.json`
//! (`scripts/fixtures/geocode_online_cases.py`, no KentOS code): each
//! kind's requests, its answers read (real ones from OpenStreetMap's
//! servers and ones written by the providers' documents), the KentOS score
//! of each candidate and a batch's pick, a batch's unique addresses and a
//! row's one line. An answer's error is compared by its being one.

use kentos_geometry_core::ops::geocode::parse;
use kentos_services::geocode::providers::{ProviderSetting, Providers};
use kentos_services::geocode::read::{self, OnlineCandidate};
use kentos_services::geocode::request::{self, Bias};
use kentos_services::geocode::score::{kind_word, pick, score};
use kentos_services::geocode::{ProviderKind, batch};
use serde_json::{Value, json};

fn fixture() -> Value {
    let path = format!(
        "{}/../../../fixtures/geocode/v1/online.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.geocode-online-cases");
    assert_eq!(file["version"], 1);
    file
}

fn answer(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/../../../fixtures/geocode/v1/answers/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
    .expect("an answer")
}

fn kind_of(k: &str) -> ProviderKind {
    serde_json::from_value(json!(k)).expect("a kind")
}

fn candidate_json(c: &OnlineCandidate) -> Value {
    json!({
        "lon": c.lon, "lat": c.lat, "label": c.label, "street": c.street, "door": c.door, "areas": c.areas,
        "neighbourhood": c.neighbourhood, "district": c.district, "province": c.province, "postcode": c.postcode,
        "name": c.name, "kind": c.kind, "providerScore": c.provider_score,
    })
}

#[test]
fn requests_are_the_references() {
    let f = fixture();
    for case in f["requests"].as_array().expect("requests") {
        let own: Option<ProviderSetting> =
            serde_json::from_value(case["own"].clone()).expect("a setting");
        let file = Providers {
            providers: own.into_iter().collect(),
            ..Providers::default()
        };
        let id = case["id"].as_str().expect("an id");
        let p = file
            .resolved()
            .into_iter()
            .find(|p| p.id == id)
            .expect("the provider");
        let got = if let Some(fw) = case.get("forward") {
            let bias = fw["bias"].as_object().map(|b| Bias {
                lon: b["lon"].as_f64().unwrap_or(0.0),
                lat: b["lat"].as_f64().unwrap_or(0.0),
                west: b["west"].as_f64().unwrap_or(0.0),
                south: b["south"].as_f64().unwrap_or(0.0),
                east: b["east"].as_f64().unwrap_or(0.0),
                north: b["north"].as_f64().unwrap_or(0.0),
            });
            request::forward(
                &p,
                fw["text"].as_str().unwrap_or(""),
                fw["limit"].as_u64().unwrap_or(10) as u32,
                bias.as_ref(),
            )
        } else {
            let rv = &case["reverse"];
            request::reverse(
                &p,
                rv["lon"].as_f64().unwrap_or(0.0),
                rv["lat"].as_f64().unwrap_or(0.0),
            )
        };
        assert_eq!(got.url, case["url"].as_str().unwrap_or(""), "{case}");
        assert_eq!(
            got.key_param.as_deref(),
            case["keyParam"].as_str(),
            "{case}"
        );
    }
}

#[test]
fn answers_read_as_the_reference_reads_them() {
    let f = fixture();
    for case in f["answers"].as_array().expect("answers") {
        let kind = kind_of(case["kind"].as_str().expect("a kind"));
        let name = case["file"].as_str().expect("a file");
        let body = answer(name);
        if case["direction"] == "forward" {
            match read::forward(kind, &body) {
                Err(why) => assert!(case["error"] == json!(true), "{name}: {why}"),
                Ok(r) => {
                    assert!(
                        case.get("error").is_none(),
                        "{name}: read, an error was expected"
                    );
                    let got: Vec<Value> = r.candidates.iter().map(candidate_json).collect();
                    assert_eq!(Value::Array(got), case["candidates"], "{name}");
                    assert_eq!(
                        r.skipped as u64,
                        case["skipped"].as_u64().unwrap_or(0),
                        "{name}"
                    );
                }
            }
        } else {
            match read::reverse(kind, &body) {
                Err(why) => assert!(case["error"] == json!(true), "{name}: {why}"),
                Ok(c) => {
                    assert!(
                        case.get("error").is_none(),
                        "{name}: read, an error was expected"
                    );
                    let got = c.as_ref().map_or(Value::Null, candidate_json);
                    assert_eq!(got, case["candidate"], "{name}");
                }
            }
        }
    }
}

#[test]
fn candidates_score_as_the_reference_scores_them() {
    let f = fixture();
    for case in f["scores"].as_array().expect("scores") {
        let text = case["text"].as_str().expect("a text");
        let kind = kind_of(case["kind"].as_str().expect("a kind"));
        let r =
            read::forward(kind, &answer(case["file"].as_str().expect("a file"))).expect("reads");
        let q = parse(text);
        let got: Vec<Value> = r
            .candidates
            .iter()
            .map(|c| {
                let (s, k) = score(&q, c);
                json!([s, kind_word(k)])
            })
            .collect();
        assert_eq!(Value::Array(got), case["scores"], "{text}");
        let p = pick(&q, &r.candidates, 85).map(|(i, s, k, st)| {
            json!({"index": i, "score": s, "kind": kind_word(k), "status": st.word()})
        });
        assert_eq!(p.unwrap_or(Value::Null), case["pick"], "{text}");
    }
}

#[test]
fn batches_and_lines_are_the_references() {
    let f = fixture();
    for case in f["batches"].as_array().expect("batches") {
        let rows: Vec<String> = case["rows"]
            .as_array()
            .expect("rows")
            .iter()
            .map(|r| r.as_str().unwrap_or("").to_owned())
            .collect();
        let (keys, at) = batch::unique(&rows);
        assert_eq!(json!(keys), case["keys"]);
        assert_eq!(json!(at), case["at"]);
    }
    for case in f["lines"].as_array().expect("lines") {
        let c: Vec<&str> = case["columns"]
            .as_array()
            .expect("columns")
            .iter()
            .map(|x| x.as_str().unwrap_or(""))
            .collect();
        assert_eq!(
            request::line_of(c[0], c[1], c[2], c[3], c[4], c[5]),
            case["line"].as_str().unwrap_or(""),
            "{c:?}"
        );
    }
}
