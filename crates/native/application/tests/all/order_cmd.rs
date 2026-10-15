//! The drawing order by its meaning over a large drawing (docs/adr/0310 §6): what Öne getir of 10 000 texts and
//! Nesnenin üstüne and altına take on a layer of 100 000 objects, the reference near its start (the most to move to
//! make room) and near its end. The shared cases are `fixtures/commands/v1/cad.entities.order.json`.

use std::time::Instant;

use kentos_domain::contracts::{CommandResult, DocumentSnapshotV1, EntitiesOrder, OrderTo};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, entities_order};
use serde_json::json;

fn drawing() -> Document {
    let mut entities = Vec::new();
    for i in 0..100_000u32 {
        let (x, y) = (f64::from(i % 300) * 3.0, f64::from(i / 300) * 3.0);
        let mut e = if i % 10 == 0 {
            json!({"kind":"text","p":{"x":x,"y":y},"text":"101","height":1,"rotation":0})
        } else {
            json!({"kind":"line","a":{"x":x,"y":y},"b":{"x":x + 2.0,"y":y}})
        };
        e["id"] = json!(i + 1);
        e["layerId"] = json!("a");
        e["attrs"] = json!({});
        entities.push(e);
    }
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Sıra","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":entities,"styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn uid(doc: &Document, slot: u32) -> String {
    doc.uid(Slot(slot)).expect("an id").to_string()
}

fn run(doc: &mut Document, input: EntitiesOrder) -> (usize, f64) {
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        entities_order::execute(&mut ExecutionContext::new(doc), input)
    else {
        panic!("not completed")
    };
    (output.changed.len(), t.elapsed().as_secs_f64() * 1e3)
}

/// `cargo test --release -p kentos-native-application --test all order_cmd::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    let mut doc = drawing();
    let texts: Vec<String> = (0..10_000u32).map(|k| uid(&doc, k * 10 + 1)).collect();
    let (n, ms) = run(
        &mut doc,
        EntitiesOrder {
            uids: texts,
            to: OrderTo::Front,
            reference: None,
            expected_revision: None,
        },
    );
    println!("Yazıları öne getir: {n} nesne, {ms:.0} ms");
    let _ = doc.undo();
    for (what, to, mover, reference) in [
        ("Nesnenin üstüne, başvuru başta", OrderTo::Above, 99_999, 2),
        ("Nesnenin üstüne, başvuru sonda", OrderTo::Above, 2, 99_998),
        ("Nesnenin altına, başvuru başta", OrderTo::Under, 99_999, 3),
        ("Nesnenin altına, başvuru sonda", OrderTo::Under, 2, 99_998),
    ] {
        let input = EntitiesOrder {
            uids: vec![uid(&doc, mover)],
            to,
            reference: Some(uid(&doc, reference)),
            expected_revision: None,
        };
        let (n, ms) = run(&mut doc, input);
        println!("{what}: {n} nesne, {ms:.0} ms");
        let _ = doc.undo();
    }
}
