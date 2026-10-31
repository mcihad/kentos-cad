//! A Veri girişi's drawing (docs/adr/0411 §5): with the document's `AttributesOnly` policy a command
//! changes existing records' attributes alone; adding, moving or deleting an object is taken back
//! whole with the reason.

use kentos_commands::DocumentSnapshotV1;
use kentos_domain::{Document, ObjectPolicy, Slot};
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

/// Layer A with a published form of its `Ad` (its `Gizli` the form does not show); layer B has none
/// (docs/adr/0764 §3).
fn drawing() -> Document {
    let body = json!({ "revision": 1, "items": [{ "type": "field", "id": "ad", "field": "Ad" }] });
    let typed: kentos_commands::FormBody = serde_json::from_value(body.clone()).expect("a body");
    let layer = |id: &str, name: &str| {
        json!({"id":id,"name":name,"type":"layer","visible":true,"locked":false,"expanded":true,
            "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[],
            "fields":[{"name":"Ad","kind":"text"},{"name":"Gizli","kind":"text"}]})
    };
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Veri girişi","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100,
        "forms":[{"id":"f","name":"A formu","layer":"a","draft":body,
            "releases":[{"version":1,"digest":kentos_commands::body_digest(&typed),"body":body}]}]},
        "origin":{"x":0,"y":0},"layers":[layer("a","A"),layer("b","B")],
        "activeLayer":"a","entities":[{"kind":"point","id":1,"layerId":"a","attrs":{"Ad":"N1"},"p":{"x":1,"y":2}},
            {"kind":"point","id":2,"layerId":"b","attrs":{"Ad":"M1"},"p":{"x":3,"y":2}}],
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
        kentos_commands::CallOp::Execute,
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

// The Veri girişi writes through the layer's live form, the fields it shows (docs/adr/0764 §3): the
// server's rule, here before anything is sent.
#[test]
fn a_data_entry_writes_what_its_forms_show() {
    let mut doc = drawing();
    let a = doc.uid(Slot(1)).expect("an id").to_string();
    let b = doc.uid(Slot(2)).expect("an id").to_string();
    let revision = doc.revision();
    for (uids, attrs, said) in [
        (
            &a,
            json!({ "Gizli": "x" }),
            "“Gizli” alanı “A” katmanının formlarında yok",
        ),
        (
            &b,
            json!({ "Ad": "M2" }),
            "“B” katmanının yayımlanmış formu yok",
        ),
    ] {
        let answer = run(
            &mut doc,
            "cad.entities.set",
            json!({ "uids": [uids], "operation": "attributes", "attrs": attrs }),
        );
        assert_eq!(answer["status"], "failed", "{answer}");
        let message = answer["error"]["message"].as_str().unwrap_or_default();
        assert!(message.contains(said), "{message}");
        assert_eq!(doc.revision(), revision, "nothing written");
    }
    // An editor's drawing writes them freely.
    doc.set_object_policy(ObjectPolicy::Free);
    let answer = run(
        &mut doc,
        "cad.entities.set",
        json!({ "uids": [b], "operation": "attributes", "attrs": { "Ad": "M2" } }),
    );
    assert_eq!(answer["status"], "completed", "{answer}");
}
