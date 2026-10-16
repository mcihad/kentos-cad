//! Annotative annotations (docs/adr/0323 §2, §3) hold to the shared cases the
//! independent reference writes (scripts/fixtures/annotative_cases.py,
//! fixtures/annotative/v1/cases.json): what a map at a scale draws of each
//! object, and an object rebased to another scale, exactly.

use kentos_contracts::{AtScale, Entity, annotative_problem, at_scale, rebase_annotative};
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/annotative/v1/cases.json");

fn entity(v: &Value) -> Entity {
    serde_json::from_value(v.clone()).expect("an object")
}

#[test]
fn the_representations_hold_to_the_shared_cases() {
    let doc: Value = serde_json::from_str(CASES).expect("the cases read");
    let cases = doc["cases"].as_array().expect("cases");
    assert!(cases.len() >= 20, "{} cases", cases.len());
    for (i, c) in cases.iter().enumerate() {
        let e = entity(&c["object"]);
        assert_eq!(annotative_problem(&e), None, "case {i}");
        if let Some(scale) = c["scale"].as_f64() {
            let got = at_scale(&e, scale);
            match &c["drawn"] {
                Value::String(own) if own == "own" => assert_eq!(got, AtScale::Own, "case {i}"),
                Value::Null => assert_eq!(got, AtScale::Hidden, "case {i}"),
                drawn => assert_eq!(got, AtScale::Drawn(Box::new(entity(drawn))), "case {i}"),
            }
        } else {
            let scale = c["rebase"].as_f64().expect("a scale");
            let got = rebase_annotative(&e, scale).expect("rebased");
            assert_eq!(got, entity(&c["result"]), "case {i}");
            assert_eq!(annotative_problem(&got), None, "case {i}");
        }
    }
}

/// Rebased there and back, an object draws at every scale as it did (the one it went to stays one of its scales).
#[test]
fn a_rebase_there_and_back_draws_the_same() {
    let doc: Value = serde_json::from_str(CASES).expect("the cases read");
    for c in doc["cases"].as_array().expect("cases") {
        if c["rebase"].is_null() {
            continue;
        }
        let e = entity(&c["object"]);
        let at = kentos_contracts::annotative_of(&e).expect("annotative").at;
        let to = c["rebase"].as_f64().expect("a scale");
        let there = rebase_annotative(&e, to).expect("there");
        let back = rebase_annotative(&there, at).expect("back");
        for scale in [50.0, 100.0, 200.0, 500.0] {
            if scale == to {
                continue;
            }
            let (a, b) = (at_scale(&e, scale), at_scale(&back, scale));
            let drawn = |x: AtScale, e: &Entity| match x {
                AtScale::Own => Some(e.clone()),
                AtScale::Hidden => None,
                AtScale::Drawn(d) => Some(*d),
            };
            let strip = |x: Option<Entity>| {
                x.map(|mut x| {
                    if let Some(f) = kentos_contracts::annotative_field(&mut x) {
                        *f = None;
                    }
                    serde_json::to_value(x).expect("json")
                })
            };
            assert_eq!(strip(drawn(a, &e)), strip(drawn(b, &back)), "at 1:{scale}");
        }
    }
}
