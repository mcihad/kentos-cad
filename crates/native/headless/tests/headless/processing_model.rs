//! An İşlemler run through an agent's door (docs/adr/8057 §108): planned, then executed with the plan's id, for one
//! tool and for a model the agent wrote for the run. Neither executed as planned (`plan_changed`) when the run's new
//! layer stood in a group (the plan's stamp read only the tree's first level, the moment the tool ran with it), nor a
//! chained model anywhere (its later step's layer kept the digest of the earlier step's new objects).

use std::path::Path;

use kentos_headless::Session;
use kentos_headless::agent_tools::{self, Guard};
use serde_json::{Value, json};

/// The parcels' drawing, a copy of its own.
fn parcels(name: &str) -> Session {
    let from =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/processing/v1/parcels.kcad");
    let dir = std::env::temp_dir().join(format!("kentos-model-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let to = dir.join("parcels.kcad");
    std::fs::copy(from, &to).unwrap();
    Session::open(&to).expect("opens")
}

fn plan_then_run(s: &mut Session, input: Value) -> Value {
    let mut guard = Guard::new();
    let planned = agent_tools::call(
        s,
        &mut guard,
        "processing.run",
        &json!({ "input": input, "mode": "plan" }),
    )
    .expect("planned");
    let plan = planned["planId"]
        .as_str()
        .unwrap_or_else(|| panic!("a plan id: {planned:#}"))
        .to_owned();
    agent_tools::call(
        s,
        &mut guard,
        "processing.run",
        &json!({ "input": input, "mode": "execute", "planId": plan }),
    )
    .expect("ran")
}

#[test]
fn one_tool_planned_then_run() {
    let mut s = parcels("one");
    let ran = plan_then_run(
        &mut s,
        json!({ "tool": "geometry.buffer", "parameters": {
            "input": { "scope": "layer", "layerId": "parsel" }, "distance": 2 } }),
    );
    assert_eq!(ran["status"], "completed", "{ran:#}");
}

#[test]
fn a_model_planned_then_run() {
    let mut s = parcels("model");
    let ran = plan_then_run(
        &mut s,
        json!({ "tool": "model:", "model": {
            "id": "tampon-birlestir", "label": "Tampon ve birleştir", "category": "analysis", "description": "",
            "inputs": [{ "name": "parseller", "label": "Parseller", "type": "features", "kinds": ["polygon"] }],
            "steps": [
                { "id": "tampon", "tool": "geometry.buffer", "values": {
                    "input": { "kind": "input", "name": "parseller" },
                    "distance": { "kind": "value", "value": 2 } } },
                { "id": "birlestir", "tool": "geometry.dissolve", "values": {
                    "input": { "kind": "output", "step": "tampon", "output": "buffers" } } }
            ],
            "outputs": [] },
          "parameters": { "parseller": { "scope": "layer", "layerId": "parsel" } } }),
    );
    assert_eq!(ran["status"], "completed", "{ran:#}");
    // Three buffers and their dissolved area, on two new layers.
    assert_eq!(ran["summary"]["counts"]["added"], 4, "{ran:#}");
    assert_eq!(
        ran["summary"]["layers"]["added"],
        json!(["islem-tampon", "islem-birlestirilen"])
    );
}

/// The model an agent wrote on the real window (17 Nov), as it wrote it: no inputs of its own, its first step's input
/// a value, the new layers named; on the demo drawing, whose parcels stand in the Kadastro group.
#[test]
fn a_run_on_a_layer_in_a_group_is_written_as_planned() {
    let from =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../samples/eklenti-vitrini/vitrin.kcad");
    let bytes = std::fs::read(from).expect("the demo drawing");
    let mut s = Session::from_bytes(&bytes).expect("opens");
    let one = plan_then_run(
        &mut s,
        json!({ "tool": "geometry.buffer", "parameters": {
            "input": { "scope": "layer", "layerId": "parsel" }, "distance": 3 } }),
    );
    assert_eq!(one["status"], "completed", "{one:#}");
    let mut s = Session::from_bytes(&bytes).expect("opens");
    let chained = plan_then_run(
        &mut s,
        json!({ "tool": "model:", "parameters": {}, "model": {
            "id": "tampon-3m-birlesim", "label": "3 m tampon ve birleşim", "inputs": [], "outputs": [],
            "steps": [
                { "id": "tampon", "tool": "geometry.buffer", "values": {
                    "input": { "kind": "value", "value": { "scope": "layer", "layerId": "parsel" } },
                    "distance": { "kind": "value", "value": 3 },
                    "dissolve": { "kind": "value", "value": false },
                    "layer": { "kind": "value", "value": { "newName": "Parsel tamponu" } } } },
                { "id": "birles", "tool": "geometry.dissolve", "values": {
                    "input": { "kind": "output", "step": "tampon", "output": "buffers" },
                    "group": { "kind": "value", "value": "" },
                    "multi": { "kind": "value", "value": true },
                    "layer": { "kind": "value", "value": { "newName": "Parsel tamponu birleşik" } } } }
            ] } }),
    );
    assert_eq!(chained["status"], "completed", "{chained:#}");
    assert_eq!(
        chained["summary"]["counts"]["added"], 5,
        "four buffers, one area"
    );
}
