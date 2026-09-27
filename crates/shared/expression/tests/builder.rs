//! The expression builder's services against fixtures/expression/v2/builder.json:
//! the answers the web's dialog gets through WASM as well (docs/adr/0100 §5;
//! apps/web/src/model/expression/builder.test.ts). An argument "@fields" is
//! the fixture's field list. KENTOS_WRITE_BUILDER=1 writes every answer
//! anew: read the difference before keeping it.

use serde_json::Value;

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/expression/v2/builder.json"
);

#[test]
fn the_builder_answers_as_pinned() {
    let text = std::fs::read_to_string(FIXTURE).expect("the fixture is readable");
    let mut doc: Value = serde_json::from_str(&text).expect("the fixture is JSON");
    let fields = doc["fields"].clone();
    let write = std::env::var_os("KENTOS_WRITE_BUILDER").is_some();
    let mut failures = Vec::new();
    let cases = doc["cases"].as_array_mut().expect("cases");
    for case in cases.iter_mut() {
        let op = case["op"].as_str().expect("op").to_string();
        let args: Vec<Value> = case["args"]
            .as_array()
            .expect("args")
            .iter()
            .map(|a| {
                if a == "@fields" {
                    fields.clone()
                } else {
                    a.clone()
                }
            })
            .collect();
        let args = serde_json::to_string(&args).expect("args");
        let got: Value = match kentos_expression::api::run_named(&op, &args) {
            Ok(r) => serde_json::from_str(&r).expect("the answer is JSON"),
            Err(e) => serde_json::json!({ "thrown": e }),
        };
        if write {
            case["result"] = got;
        } else if case["result"] != got {
            failures.push(format!(
                "{op} {args}\n  pinned: {}\n  now:    {got}",
                case["result"]
            ));
        }
    }
    if write {
        let out = serde_json::to_string_pretty(&doc).expect("JSON") + "\n";
        std::fs::write(FIXTURE, out).expect("the fixture is writable");
    }
    assert!(
        failures.is_empty(),
        "{} answers differ:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
