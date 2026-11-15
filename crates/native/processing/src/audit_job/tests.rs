use std::path::PathBuf;

use kentos_commands::DocumentSnapshotV2;
use serde_json::Value;

use super::*;

/// The GIS fixture of the facts' first slice (fixtures/audit-facts/v1/denetim.kcad), as JSON.
fn snapshot() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/audit-facts/v1/denetim.kcad");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn open(v: Value) -> Document {
    Document::from_snapshot_v2(serde_json::from_value::<DocumentSnapshotV2>(v).unwrap()).unwrap()
}

fn never() -> bool {
    false
}

/// The facts are what the desktop's Eklenti denetimi has sent a rule since ADR 8057 §36: the layers' objects (50 000
/// at most) with the measured facts as `project`.
#[test]
fn the_facts_are_the_desktop_s() {
    let doc = open(snapshot());
    let mut desktop = crate::external::audit_facts(&doc, 50_000);
    let project =
        crate::audit_facts::project_facts(&doc, crate::audit_facts::MOST_OBJECTS, &never).unwrap();
    desktop["project"] = serde_json::to_value(&project).unwrap();
    assert_eq!(facts_of(&doc, &never), Some(desktop.clone()));
    assert_eq!(
        message(desktop.clone()),
        serde_json::json!({ "kind": "audit", "facts": desktop })
    );
    assert_eq!(facts_of(&doc, &|| true), None, "stopped while measured");
}

/// The drawing's stamp comes from its content alone: the same twice and in another order, another when an object
/// changes.
#[test]
fn the_drawing_stamp_is_its_content_s() {
    let first = drawing_stamp(&open(snapshot()));
    assert_eq!(first.len(), 16);
    assert_eq!(drawing_stamp(&open(snapshot())), first);
    let mut reversed = snapshot();
    for key in ["entities", "uids"] {
        reversed[key].as_array_mut().unwrap().reverse();
    }
    assert_eq!(drawing_stamp(&open(reversed)), first, "the drawing's order");
    let mut moved = snapshot();
    moved["entities"][0]["pts"][0]["x"] = serde_json::json!(0.5);
    assert_ne!(drawing_stamp(&open(moved)), first);
}

/// What the provenance costs on a large drawing (docs/adr/8057 §42): 50 000 parcels' facts, the content stamp and the
/// facts' digest (`stamp_of` does the digest's work: canonical JSON and SHA-256), beside the message the rule is sent
/// anyway. A measurement, run by hand:
/// `cargo test -p kentos-processing --lib audit_job::tests::provenance_cost -- --ignored --nocapture`.
#[test]
#[ignore = "a measurement, run by hand"]
fn provenance_cost() {
    let mut v = snapshot();
    let parcel = v["entities"][0].clone();
    let (mut entities, mut uids) = (Vec::new(), Vec::new());
    for i in 0..50_000u64 {
        let (x, y) = ((i % 250) as f64 * 12.0, (i / 250) as f64 * 12.0);
        let mut e = parcel.clone();
        e["id"] = serde_json::json!(i + 1);
        e["attrs"] =
            serde_json::json!({ "Ada": (i / 250).to_string(), "Parsel": (i % 250).to_string() });
        e["pts"] = serde_json::json!([{ "x": x, "y": y }, { "x": x + 10.0, "y": y },
            { "x": x + 10.0, "y": y + 10.0 }, { "x": x, "y": y + 10.0 }]);
        entities.push(e);
        uids.push(format!("0192f5a1-2590-7000-8000-{:012x}", i + 1));
    }
    v["entities"] = entities.into();
    v["uids"] = uids.into();
    let doc = open(v);
    let clock = std::time::Instant::now();
    let facts = facts_of(&doc, &never).unwrap();
    let facts_ms = clock.elapsed().as_millis();
    let clock = std::time::Instant::now();
    let sent = serde_json::to_string(&message(facts.clone())).unwrap();
    let sent_ms = clock.elapsed().as_millis();
    let clock = std::time::Instant::now();
    let stamp = drawing_stamp(&doc);
    let stamp_ms = clock.elapsed().as_millis();
    let clock = std::time::Instant::now();
    let digest = stamp_of(&facts);
    let digest_ms = clock.elapsed().as_millis();
    eprintln!(
        "50 000 parsel: olgular {facts_ms} ms; gönderilen {} MB {sent_ms} ms; damga {stamp} {stamp_ms} ms; özet {digest} \
         {digest_ms} ms",
        sent.len() / 1_000_000
    );
}
