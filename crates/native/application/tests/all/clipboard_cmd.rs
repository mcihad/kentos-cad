//! Panoya kopyala and Yapıştır by their meaning over a large drawing (docs/adr/0309 §6): what copying and pasting
//! take for 100 000 objects on ten layers of a group, copied whole and in part, pasted back into the drawing and
//! into an empty one. The shared cases are `fixtures/commands/v1/cad.entities.{copy,paste}.json`.

use std::time::Instant;

use kentos_commands::{
    Clip, CommandResult, DocumentSnapshotV1, EntitiesCopy, EntitiesPaste, Vec2,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, entities_copy, entities_paste};
use serde_json::json;

fn layer(id: &str, name: &str) -> serde_json::Value {
    json!({"id":id,"name":name,"type":"layer","visible":true,"locked":false,"expanded":true,
        "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]})
}

fn drawing(n: u32) -> Document {
    let mut entities = Vec::new();
    for i in 0..n {
        let (x, y) = (f64::from(i % 300) * 3.0, f64::from(i / 300) * 3.0);
        entities.push(
            json!({"kind":"line","a":{"x":x,"y":y},"b":{"x":x + 2.0,"y":y},"id":i + 1,
            "layerId":format!("k{}", i % 10),"attrs":{"No":i.to_string()}}),
        );
    }
    let leaves: Vec<_> = (0..10)
        .map(|i| layer(&format!("k{i}"), &format!("Katman {i}")))
        .collect();
    let mut group = layer("g", "Kadastro");
    group["type"] = json!("group");
    group["children"] = json!(leaves);
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Pano","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[group, layer("bos", "Boş")],"activeLayer":"k0","entities":entities,
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn copy(doc: &Document, n: usize) -> (Clip, f64) {
    let uids: Vec<String> = doc
        .entities()
        .take(n)
        .filter_map(|e| doc.uid(Slot(e.base().id)))
        .map(|u| u.to_string())
        .collect();
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        entities_copy::answer(doc, &EntitiesCopy { uids, base: None })
    else {
        panic!("not copied")
    };
    (output, t.elapsed().as_secs_f64() * 1e3)
}

fn paste(doc: &mut Document, clip: Clip) -> f64 {
    let n = clip.objects.len();
    let t = Instant::now();
    let CommandResult::Completed { output, .. } = entities_paste::execute(
        &mut ExecutionContext::new(doc),
        EntitiesPaste {
            clip,
            at: Some(Vec2 { x: 5000.0, y: 0.0 }),
            as_block: None,
            conflicts: Default::default(),
            expected_revision: None,
        },
    ) else {
        panic!("not pasted")
    };
    let ms = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(output.created.len(), n);
    ms
}

/// `cargo test --release -p kentos-native-application --test all clipboard_cmd::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    let mut doc = drawing(100_000);
    for n in [1, 1_000, 10_000, 100_000] {
        let (clip, copied) = copy(&doc, n);
        let mut empty = drawing(0);
        let elsewhere = paste(&mut empty, clip.clone());
        assert_eq!(empty.layers().leaves().len(), 11);
        let here = paste(&mut doc, clip);
        println!(
            "{n} nesne: kopyalama {copied:.1} ms, boş çizime yapıştırma {elsewhere:.1} ms, aynı çizime yapıştırma {here:.1} ms"
        );
        let _ = doc.undo();
    }
}
