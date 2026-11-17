//! A model the agent writes for a run, through the desktop's door (docs/adr/8057 §108): planned, then executed with
//! the plan's id after the user's Onayla, as on the real window.

use kentos_commands::DocumentSnapshotV1;
use kentos_headless::agent_tools::{self, Guard};
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;

fn parcels() -> App {
    // The desktop registers İşlemler's tools for the commands when it starts (main.rs).
    kentos_processing::command::register_as("desktop");
    let (mut app, _) = App::boot(None);
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/processing/v1/parcels.kcad");
    let text = std::fs::read_to_string(path).expect("the drawing is there");
    let doc =
        Document::new(DocumentSnapshotV1::from_json(&text).expect("reads"), None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn buffer_and_dissolve() -> Value {
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
      "parameters": { "parseller": { "scope": "layer", "layerId": "parsel" } } })
}

#[test]
fn a_model_the_agent_wrote_is_planned_then_written() {
    let mut app = parcels();
    let mut guard = Guard::new();
    let mut host = super::host::AppHost(&mut app);
    let input = buffer_and_dissolve();
    let planned = agent_tools::call(
        &mut host,
        &mut guard,
        "processing.run",
        &json!({ "input": input, "mode": "plan" }),
    )
    .expect("planned");
    let plan = planned["planId"]
        .as_str()
        .unwrap_or_else(|| panic!("{planned:#}"))
        .to_owned();
    let ran = agent_tools::call(
        &mut host,
        &mut guard,
        "processing.run",
        &json!({ "input": input, "mode": "execute", "planId": plan }),
    )
    .expect("ran");
    assert_eq!(ran["status"], "completed", "{ran:#}");
}
