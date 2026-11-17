//! The same İşlemler run through every door gives the same result (plan/eklenti.md §5.2, ECO-QA-06; docs/adr/8057
//! §116): the product command as a program calls it (Python's `cad.processing.run` and MCP's command tool reach the
//! same call), an agent's `processing.run` planned then executed, a one-step model written for the run, and a batch of
//! one row. On the same drawing each writes the same objects (shape, attributes, layer) and says the same typed counts.

use std::path::Path;

use kentos_headless::Session;
use kentos_headless::agent_tools::{self, Guard};
use serde_json::{Value, json};

fn parcels() -> Session {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/processing/v1/parcels.kcad");
    Session::from_bytes(&std::fs::read(path).expect("the drawing")).expect("opens")
}

/// The objects a run added, as they are without their ids, in one order; and the run's answer.
fn written(run: impl FnOnce(&mut Session) -> Value) -> (Vec<String>, Value) {
    let mut s = parcels();
    let before: Vec<String> = all(&s)
        .iter()
        .map(|e| e["uid"].as_str().unwrap_or_default().to_owned())
        .collect();
    let answer = run(&mut s);
    let mut added: Vec<String> = all(&s)
        .into_iter()
        .filter(|e| !before.iter().any(|u| Some(u.as_str()) == e["uid"].as_str()))
        .map(|e| {
            let mut entity = e["entity"].clone();
            if let Some(map) = entity.as_object_mut() {
                map.remove("id");
            }
            entity.to_string()
        })
        .collect();
    added.sort();
    (added, answer)
}

/// Every object of the drawing, as the wire carries it.
fn all(s: &Session) -> Vec<Value> {
    let page = s
        .entities(None, None, None, None, Some(10_000))
        .expect("a page");
    serde_json::to_value(&page).expect("reads")["items"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}

const TOOL: &str = "geometry.buffer";

fn parameters() -> Value {
    json!({ "input": { "scope": "layer", "layerId": "parsel" }, "distance": 2 })
}

#[test]
fn the_same_run_gives_the_same_result_through_every_door() {
    // The product command, as a program calls it.
    let (command, ran) = written(|s| {
        s.call(json!({ "command": "cad.processing.run", "op": "execute",
                       "input": { "tool": TOOL, "parameters": parameters() } }))
            .expect("ran")
    });
    assert_eq!(ran["status"], "completed", "{ran:#}");
    assert_eq!(command.len(), 3, "a buffer a parcel");
    assert_eq!(ran["output"]["outputs"]["count"], 3, "{ran:#}");
    // An agent's door: planned, then executed with the plan's id.
    let (agent, ran) = written(|s| {
        let mut guard = Guard::new();
        let input = json!({ "tool": TOOL, "parameters": parameters() });
        let planned = agent_tools::call(
            s,
            &mut guard,
            "processing.run",
            &json!({ "input": input, "mode": "plan" }),
        )
        .expect("planned");
        let plan = planned["planId"].as_str().expect("a plan id").to_owned();
        agent_tools::call(
            s,
            &mut guard,
            "processing.run",
            &json!({ "input": input, "mode": "execute", "planId": plan }),
        )
        .expect("ran")
    });
    assert_eq!(ran["status"], "completed", "{ran:#}");
    assert_eq!(ran["output"]["outputs"]["count"], 3);
    assert_eq!(
        agent, command,
        "the agent's door wrote what the command wrote"
    );
    // A one-step model written for the run.
    let (model, ran) = written(|s| {
        s.call(json!({ "command": "cad.processing.run", "op": "execute", "input": {
            "tool": "model:", "parameters": { "parseller": { "scope": "layer", "layerId": "parsel" } },
            "model": { "id": "tampon", "label": "Tampon", "inputs": [
                { "name": "parseller", "label": "Parseller", "type": "features", "kinds": ["polygon"] }],
              "steps": [{ "id": "t", "tool": TOOL, "values": {
                "input": { "kind": "input", "name": "parseller" },
                "distance": { "kind": "value", "value": 2 } } }],
              "outputs": [] } } }))
            .expect("ran")
    });
    assert_eq!(ran["status"], "completed", "{ran:#}");
    assert_eq!(model, command, "the model's step wrote what the tool wrote");
    // A batch of one row.
    let (batch, ran) = written(|s| {
        s.call(json!({ "command": "cad.processing.batch", "op": "execute",
                       "input": { "tool": TOOL, "parameters": parameters(), "rows": [{}] } }))
            .expect("ran")
    });
    assert_eq!(ran["status"], "completed", "{ran:#}");
    assert_eq!(batch, command, "the batch's row wrote what the run wrote");
}
