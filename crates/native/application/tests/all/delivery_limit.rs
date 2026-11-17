//! Teslim kaydının sınırı (docs/adr/8099): a drawing keeps at most 1000 deliveries; the 1001st is refused with
//! `too_many_deliveries` at `change` and nothing is written. The shared cases (fixtures/commands/v1/cad.delivery.edit.json)
//! would need a thousand records in their drawing; the limit is tried here on the cases' own drawing.

use kentos_commands::{CallOp, DocumentSnapshotV2, MAX_DELIVERIES};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn run(doc: &mut Document, input: Value) -> Value {
    registry::run(&mut ExecutionContext::new(doc), "cad.delivery.edit", CallOp::Execute, input).expect("runs")
}

fn add(id: &str) -> Value {
    json!({"change": {"kind": "add", "id": id, "revision": "000",
        "package": {"manifest": "3f0a5c1e9b7d2846a1c0e5f3b8d9274c6e1f0a3b5c7d9e2f4a6b8c0d1e3f5a7b", "files": 14},
        "to": "Sivas Belediyesi İmar ve Şehircilik Müdürlüğü", "channel": "epys", "at": "2026-11-17"}})
}

#[test]
fn the_thousand_and_first_delivery_is_refused_and_nothing_is_written() {
    let cases: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.delivery.edit.json")).expect("cases");
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("the drawing");
    let mut doc = Document::from_snapshot_v2(snapshot).expect("opens");
    let first = run(&mut doc, add("0192f5a1-8099-7000-8000-000000000000"));
    assert_eq!(first["status"], "completed", "{first}");
    // A thousand records as the command writes them, each its own id and number.
    let one = doc.settings().arch.as_ref().expect("the records").deliveries[0].clone();
    let mut settings = doc.settings().clone();
    let arch = settings.arch.as_mut().expect("the records");
    arch.deliveries = (0..MAX_DELIVERIES)
        .map(|k| {
            let mut d = one.clone();
            d.id = format!("0192f5a1-8099-7000-8000-{k:012x}");
            d.number = u32::try_from(k + 1).expect("a number");
            d
        })
        .collect();
    doc.set_settings(settings);
    let before = doc.settings().clone();
    let refused = run(&mut doc, add("0192f5a1-8099-7000-8001-000000000000"));
    assert_eq!(refused["status"], "failed", "{refused}");
    assert_eq!(refused["error"]["code"], "too_many_deliveries");
    assert_eq!(refused["error"]["path"], "change");
    assert_eq!(*doc.settings(), before, "nothing written");
}
