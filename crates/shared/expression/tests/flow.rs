//! The expression's flow (docs/adr/0101) against fixtures/expression/v2/flow.json:
//! the nodes, places and texts `exprFlow` gives and the trees `exprFlowEdit`
//! leaves, as the web gets them through WASM
//! (apps/web/src/model/expression/flow.test.ts). An argument "@fields" is
//! the fixture's field list. KENTOS_WRITE_FLOW=1 writes every answer anew:
//! read the difference before keeping it.
//!
//! The text is the flow's state, so the round trip is checked on every
//! answer: the texts a flow writes give the same flow again, and the trees
//! a change leaves read as the language reads them (`?` an empty input).

use serde_json::{Value, json};

const FIXTURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../fixtures/expression/v2/flow.json"
);

fn run(op: &str, args: &[Value]) -> Value {
    let args = serde_json::to_string(args).expect("args");
    match kentos_expression::api::run_named(op, &args) {
        Ok(r) => serde_json::from_str(&r).expect("the answer is JSON"),
        Err(e) => json!({ "thrown": e }),
    }
}

/// The trees of a flow's answer as the page would send them back.
fn trees_of(input: &Value, answer: &Value) -> Vec<Value> {
    let texts = answer["texts"].as_array().expect("texts");
    input
        .as_array()
        .expect("trees")
        .iter()
        .zip(texts)
        .map(|(t, text)| {
            let mut t = t.clone();
            t["text"] = text.clone();
            t
        })
        .collect()
}

#[test]
fn the_flow_answers_as_pinned_and_round_trips() {
    let text = std::fs::read_to_string(FIXTURE).expect("the fixture is readable");
    let mut doc: Value = serde_json::from_str(&text).expect("the fixture is JSON");
    let fields = doc["fields"].clone();
    let write = std::env::var_os("KENTOS_WRITE_FLOW").is_some();
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
        let got = run(&op, &args);
        // The round trip: a flow's texts give the same flow; a change's trees read.
        if op == "exprFlow" && got.get("error").is_none() {
            let again = run(&op, &[json!(trees_of(&args[0], &got)), fields.clone()]);
            if again != got {
                failures.push(format!(
                    "{op} {}: the written texts give another flow",
                    args[0]
                ));
            }
        }
        if op == "exprFlowEdit"
            && let Some(trees) = got.get("trees")
        {
            let flow = run("exprFlow", &[trees.clone(), fields.clone()]);
            if flow.get("error").is_some() {
                failures.push(format!("{op} {}: the trees do not read: {flow}", args[1]));
            }
            if flow["texts"]
                != json!(
                    trees
                        .as_array()
                        .expect("trees")
                        .iter()
                        .map(|t| t["text"].clone())
                        .collect::<Vec<_>>()
                )
            {
                failures.push(format!(
                    "{op} {}: the trees are not written as the flow writes them",
                    args[1]
                ));
            }
        }
        if write {
            case["result"] = got;
        } else if case["result"] != got {
            failures.push(format!(
                "{op} {}\n  pinned: {}\n  now:    {got}",
                serde_json::to_string(&args[..args.len() - 1]).unwrap_or_default(),
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
