//! The numeric policy against the independent reference in
//! `fixtures/numeric/v1` (Python decimal/fractions, not KentOS code).

use kentos_geometry_core::numeric::{
    RemainderRule, RoundingMode, Share, distribute, round_decimal, sum_shares,
};
use serde_json::Value;

fn load(name: &str) -> Vec<Value> {
    let path = format!(
        "{}/../../../fixtures/numeric/v1/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value = serde_json::from_str(&std::fs::read_to_string(path).expect("fixture file"))
        .expect("fixture JSON");
    assert_eq!(file["format"], "kentos.numeric-fixtures");
    assert_eq!(file["version"], 1);
    file["cases"].as_array().expect("cases").clone()
}

#[test]
fn rounding_matches_the_reference_for_every_mode_scale_and_refusal() {
    let mut failures = Vec::new();
    for c in load("rounding") {
        let input = c["input"].as_str().unwrap();
        let scale = c["scale"].as_u64().unwrap() as u32;
        let mode = RoundingMode::parse(c["mode"].as_str().unwrap()).expect("mode");
        let got = match round_decimal(input, scale, mode) {
            Ok(v) => serde_json::json!({ "value": v }),
            Err(e) => serde_json::json!({ "error": e.code() }),
        };
        if got != c["expected"] {
            failures.push(format!(
                "{input} @{scale} {}: {got} ≠ {}",
                c["mode"], c["expected"]
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} rounding case(s) differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn shares_are_exact_fractions() {
    for c in load("shares") {
        let parts: Vec<Share> = c["input"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| Share::parse(s.as_str().unwrap()).unwrap())
            .collect();
        let reduced: Vec<String> = parts.iter().map(|s| s.to_string()).collect();
        let sum = sum_shares(&parts).unwrap();
        assert_eq!(
            serde_json::json!(reduced),
            c["expected"]["reduced"],
            "{}",
            c["input"]
        );
        assert_eq!(
            sum.to_string(),
            c["expected"]["sum"].as_str().unwrap(),
            "{}",
            c["input"]
        );
        assert_eq!(
            sum.is_whole(),
            c["expected"]["whole"].as_bool().unwrap(),
            "{}",
            c["input"]
        );
    }
    assert!(Share::parse("1/0").is_err());
}

#[test]
fn distribution_hands_out_remainders_only_by_an_explicit_rule() {
    for c in load("distribution") {
        let i = &c["input"];
        let weights: Vec<&str> = i["weights"]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| w.as_str().unwrap())
            .collect();
        let got = distribute(
            i["total"].as_str().unwrap(),
            &weights,
            i["scale"].as_u64().unwrap() as u32,
            RoundingMode::parse(i["mode"].as_str().unwrap()).unwrap(),
            RemainderRule::parse(i["remainder"].as_str().unwrap()).unwrap(),
        );
        let got = match got {
            Ok(d) => serde_json::json!({ "parts": d.parts, "remainder": d.remainder }),
            Err(kentos_geometry_core::numeric::NumericError::NeedsRule(r)) => {
                serde_json::json!({ "error": "needs_rule", "remainder": r })
            }
            Err(e) => serde_json::json!({ "error": e.code() }),
        };
        assert_eq!(got, c["expected"], "{}", i);
    }
}
