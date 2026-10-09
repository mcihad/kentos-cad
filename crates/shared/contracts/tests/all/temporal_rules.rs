//! The rules of a layer's time setting, a scenario and the layer tree's
//! scenarios (docs/adr/0210 §2) against fixtures/temporal/v1/rules.json: each
//! case's words, the same the browser's port gives
//! (apps/web/src/model/temporalRules.ts); the values and the verdicts are the
//! independent reader's (scripts/fixtures/temporal_rules_cases.py).
//! `KENTOS_WRITE_TEMPORAL_RULES=1` writes the words where the verdicts agree;
//! read the diff before committing it.

use kentos_contracts::{LayerNode, LayerTime, ScenarioInfo, scenarios_problem};
use serde_json::Value;

const PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/temporal/v1/rules.json"
);

fn problem(group: &str, value: &Value) -> Option<String> {
    match group {
        "times" => serde_json::from_value::<LayerTime>(value.clone())
            .expect("a time setting")
            .problem(),
        "scenarios" => serde_json::from_value::<ScenarioInfo>(value.clone())
            .expect("a scenario")
            .problem(),
        _ => scenarios_problem(
            &serde_json::from_value::<Vec<LayerNode>>(value.clone()).expect("a layer tree"),
        ),
    }
}

#[test]
fn every_case_has_the_contracts_words() {
    let mut cases: Value =
        serde_json::from_str(&std::fs::read_to_string(PATH).expect("rules.json")).expect("JSON");
    let write = std::env::var("KENTOS_WRITE_TEMPORAL_RULES").as_deref() == Ok("1");
    let mut wrong = Vec::new();
    for group in ["times", "scenarios", "trees"] {
        for case in cases[group].as_array_mut().expect("a list") {
            let got = problem(group, &case["value"]);
            let want = case["problem"].as_str().map(str::to_owned);
            if got.is_some() != want.is_some() {
                wrong.push(format!("{group}/{}: {got:?} ≠ {want:?}", case["name"]));
            } else if write {
                case["problem"] = got.map_or(Value::Null, Value::String);
            } else if got != want {
                wrong.push(format!("{group}/{}: {got:?} ≠ {want:?}", case["name"]));
            }
        }
    }
    if write && wrong.is_empty() {
        let text = serde_json::to_string_pretty(&cases).expect("JSON") + "\n";
        std::fs::write(PATH, text).expect("writes rules.json");
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
