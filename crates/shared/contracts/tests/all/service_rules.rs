//! The rules of a layer's map service, a feed and the project's connections
//! (docs/adr/0208 §2) against fixtures/services/v1/rules.json: each case's
//! words, the same the browser's port gives (apps/web/src/model/serviceRules.ts).
//! `KENTOS_WRITE_SERVICE_RULES=1` writes the words; read the diff before
//! committing it.

use kentos_contracts::{
    FeatureFeed, ServiceConnection, ServiceLayer, connections_problem, service::origin_of,
};
use serde_json::Value;

const PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/services/v1/rules.json"
);

fn problem(group: &str, value: &Value) -> Option<String> {
    match group {
        "services" => serde_json::from_value::<ServiceLayer>(value.clone())
            .expect("a service layer")
            .problem(),
        "feeds" => serde_json::from_value::<FeatureFeed>(value.clone())
            .expect("a feed")
            .problem(),
        _ => connections_problem(
            &serde_json::from_value::<Vec<ServiceConnection>>(value.clone()).expect("connections"),
        ),
    }
}

#[test]
fn every_case_has_the_contracts_words() {
    let mut cases: Value =
        serde_json::from_str(&std::fs::read_to_string(PATH).expect("rules.json")).expect("JSON");
    let write = std::env::var("KENTOS_WRITE_SERVICE_RULES").as_deref() == Ok("1");
    let mut wrong = Vec::new();
    for group in ["services", "feeds", "connections"] {
        for case in cases[group].as_array_mut().expect("a list") {
            let got = problem(group, &case["value"]);
            let want = case["problem"].as_str().map(str::to_owned);
            if write {
                case["problem"] = got.map_or(Value::Null, Value::String);
            } else if got != want {
                wrong.push(format!("{group}/{}: {got:?} ≠ {want:?}", case["name"]));
            }
        }
    }
    for case in cases["origins"].as_array_mut().expect("a list") {
        let got = origin_of(case["url"].as_str().expect("an address"));
        let want = case["origin"].as_str().map(str::to_owned);
        if write {
            case["origin"] = got.map_or(Value::Null, Value::String);
        } else if got != want {
            wrong.push(format!("origins/{}: {got:?} ≠ {want:?}", case["url"]));
        }
    }
    if write {
        let text = serde_json::to_string_pretty(&cases).expect("JSON") + "\n";
        std::fs::write(PATH, text).expect("writes rules.json");
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
