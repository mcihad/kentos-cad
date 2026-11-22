//! A model written for a run (docs/adr/8057 §108, PY-AI-08): `cad.processing.run` with `model` runs its steps as a
//! saved model runs, the next step reading the one before's new objects, one undo step for the whole; a model that
//! does not read, names a tool this host has not or whose wiring does not hold is refused, where, before anything is
//! written.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use kentos_commands::{DocumentSnapshotV1, ProcessingRun};
use kentos_domain::Document;
use kentos_native_application::processing_host::{BatchFlow, RunEnd, tools};
use serde_json::{Value, json};

fn parcels() -> Document {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/processing/v1/parcels.kcad");
    let text = std::fs::read_to_string(path).expect("the drawing is there");
    Document::from_snapshot(DocumentSnapshotV1::from_json(&text).expect("reads")).expect("opens")
}

struct Never;

impl BatchFlow for Never {
    fn stopped(&self) -> bool {
        false
    }
    fn progress(&self, _: u64, _: u64) {}
    fn stop_flag(&self) -> Option<Arc<AtomicBool>> {
        None
    }
}

/// Tampon and Birleştir chained: the parcels buffered by 2 m, the buffers dissolved into one.
fn buffer_and_dissolve() -> Value {
    json!({
        "id": "tampon-birlestir",
        "label": "Tampon ve birleştir",
        "category": "analysis",
        "description": "Parsellerin 2 m tamponunu çizer ve birleştirir.",
        "inputs": [{ "name": "parseller", "label": "Parseller", "type": "features", "kinds": ["polygon"] }],
        "steps": [
            { "id": "tampon", "tool": "geometry.buffer", "values": {
                "input": { "kind": "input", "name": "parseller" },
                "distance": { "kind": "value", "value": 2 } } },
            { "id": "birlestir", "tool": "geometry.dissolve", "values": {
                "input": { "kind": "output", "step": "tampon", "output": "buffers" } } }
        ],
        "outputs": []
    })
}

#[allow(clippy::result_large_err)]
fn run(
    doc: &mut Document,
    model: Value,
) -> Result<kentos_commands::ProcessingRan, kentos_commands::CommandError> {
    kentos_processing::command::register();
    let run: ProcessingRun = serde_json::from_value(json!({
        "tool": "model:",
        "model": model,
        "parameters": { "parseller": { "scope": "layer", "layerId": "parsel" } }
    }))
    .expect("the input reads");
    match tools().expect("registered").run(doc, &run, true, &Never) {
        Ok(RunEnd::Ran(ran, _)) => Ok(ran),
        Ok(RunEnd::Cancelled) => panic!("it was not stopped"),
        Err(e) => Err(*e),
    }
}

#[test]
fn a_model_written_for_a_run_chains_its_tools_in_one_undo_step() {
    let mut doc = parcels();
    let count = doc.entities().count();
    let ran = run(&mut doc, buffer_and_dissolve()).expect("ran");
    assert_eq!(ran.tool, "model:tampon-birlestir");
    assert!(!ran.added.is_empty(), "{ran:?}");
    assert!(doc.entities().count() > count);
    // One undo step for the whole chain.
    assert!(doc.undo().is_some(), "an undo step");
    assert_eq!(doc.entities().count(), count);
    assert!(!doc.can_undo(), "the whole chain was one step");
}

#[test]
fn a_model_that_cannot_run_is_refused_where_before_anything_is_written() {
    let mut doc = parcels();
    let count = doc.entities().count();
    let refused = |doc: &mut Document, model: Value| run(doc, model).expect_err("refused");
    // A tool this host has not.
    let mut unknown = buffer_and_dissolve();
    unknown["steps"][1]["tool"] = json!("geometry.yok");
    let e = refused(&mut doc, unknown);
    assert_eq!(
        (e.code.as_str(), e.path.as_deref()),
        ("unknown_tool", Some("model.steps[1].tool"))
    );
    // Wired to an output the step before has not.
    let mut wired = buffer_and_dissolve();
    wired["steps"][1]["values"]["input"]["output"] = json!("yok");
    let e = refused(&mut doc, wired);
    assert_eq!(e.code, "invalid_parameters");
    assert_eq!(e.path.as_deref(), Some("model.steps[1]"));
    assert!(e.message.contains("adım “birlestir”"), "{}", e.message);
    // An empty list written as an empty object reads as the list (as an agent wrote it): the chain runs.
    let mut objects = buffer_and_dissolve();
    objects["outputs"] = json!({});
    let mut copy = parcels();
    assert!(run(&mut copy, objects).is_ok());
    // Not a model at all, and one without steps.
    let e = refused(&mut doc, json!({ "steps": "bir" }));
    assert_eq!(
        (e.code.as_str(), e.path.as_deref()),
        ("invalid_parameters", Some("model"))
    );
    let mut empty = buffer_and_dissolve();
    empty["steps"] = json!([]);
    let e = refused(&mut doc, empty);
    assert_eq!(e.path.as_deref(), Some("model.steps"));
    assert_eq!(doc.entities().count(), count, "nothing written");
    assert!(!doc.can_undo());
}
