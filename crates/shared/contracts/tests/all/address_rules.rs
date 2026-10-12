//! The rules of a project's address data (docs/adr/0229 §2) against
//! fixtures/geocode/v1/rules.json: each case's words; the values and the
//! verdicts are the independent reader's (scripts/fixtures/address_rules_cases.py).
//! `KENTOS_WRITE_ADDRESS_RULES=1` writes the words where the verdicts agree;
//! read the diff before committing it.

use kentos_contracts::{AddressLocator, address_locators_problem};
use serde_json::Value;

const PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/geocode/v1/rules.json"
);

fn problem(group: &str, value: &Value) -> Option<String> {
    match group {
        "locators" => serde_json::from_value::<AddressLocator>(value.clone())
            .expect("a locator")
            .problem(),
        _ => address_locators_problem(
            &serde_json::from_value::<Vec<AddressLocator>>(value.clone()).expect("locators"),
        ),
    }
}

#[test]
fn every_case_has_the_contracts_words() {
    let mut cases: Value =
        serde_json::from_str(&std::fs::read_to_string(PATH).expect("rules.json")).expect("JSON");
    let write = std::env::var("KENTOS_WRITE_ADDRESS_RULES").as_deref() == Ok("1");
    let mut wrong = Vec::new();
    for group in ["locators", "lists"] {
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
