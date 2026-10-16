//! A Veri girişi's drawing (docs/adr/0411 §5): with the document's `AttributesOnly` policy a command
//! changes existing records' attributes alone; adding, moving or deleting an object is taken back
//! whole with the reason.

use kentos_domain::contracts::DocumentSnapshotV1;
use kentos_domain::{Document, ObjectPolicy, Slot};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn drawing() -> Document {
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Veri girişi","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":[{"kind":"point","id":1,"layerId":"a","attrs":{"Ad":"N1"},"p":{"x":1,"y":2}}],
        "styles":{"items":[],"categories":[]}});
    let mut doc = Document::from_snapshot(
        DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"),
    )
    .expect("opens");
    doc.set_object_policy(ObjectPolicy::AttributesOnly);
    doc
}

fn run(doc: &mut Document, command: &str, input: Value) -> Value {
    registry::run(
        &mut ExecutionContext::new(doc),
        command,
        kentos_domain::contracts::CallOp::Execute,
        input,
    )
    .expect("a known command")
}

#[test]
fn a_data_entry_changes_attributes_alone() {
    let mut doc = drawing();
    let uid = doc.uid(Slot(1)).expect("an id").to_string();
    let revision = doc.revision();
    // A new object, a move and a deletion are taken back whole.
    for (command, input) in [
        (
            "cad.point.create",
            json!({ "layerId": "a", "p": { "x": 5, "y": 5 } }),
        ),
        (
            "cad.entities.transform",
            json!({ "uids": [uid], "transform": { "kind": "move", "dx": 1, "dy": 0 } }),
        ),
        ("cad.entities.delete", json!({ "uids": [uid] })),
    ] {
        let answer = run(&mut doc, command, input);
        assert_eq!(answer["status"], "failed", "{command}: {answer}");
        assert!(
            answer["error"]["message"]
                .as_str()
                .is_some_and(|m| m.contains("Veri girişi")),
            "{command}: {answer}"
        );
        assert_eq!(doc.revision(), revision, "{command} wrote nothing");
    }
    // The record's attributes are written.
    let answer = run(
        &mut doc,
        "cad.entities.set",
        json!({ "uids": [uid], "operation": "attributes", "attrs": { "Ad": "N2" } }),
    );
    assert_eq!(answer["status"], "completed", "{answer}");
    assert_eq!(
        doc.get(Slot(1)).map(|e| e.base().attrs.get("Ad").cloned()),
        Some(Some("N2".to_owned()))
    );
}
