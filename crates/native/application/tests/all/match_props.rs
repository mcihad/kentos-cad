//! Özellik kopyala by its meaning over a large drawing (docs/adr/0304 §5): what a write and a plan take when 10 000
//! parcels take a parcel's properties in a drawing of 100 000 objects. The shared cases are
//! `fixtures/commands/v1/cad.entities.match.json`.

use kentos_domain::contracts::{CommandResult, DocumentSnapshotV1, EntitiesMatch, Entity};
use kentos_domain::{Document, Slot};
use kentos_native_application::{ExecutionContext, entities_match};
use serde_json::json;

fn drawing(parcels: u32) -> Document {
    let parcel = |i: u32| {
        let (x, y) = (f64::from(i % 100) * 12.0, f64::from(i / 100) * 12.0);
        json!({"kind":"polygon","id":i + 1,"layerId":"a","attrs":{},
               "pts":[{"x":x,"y":y},{"x":x + 10.0,"y":y},{"x":x + 10.0,"y":y + 10.0},{"x":x,"y":y + 10.0}]})
    };
    let mut entities: Vec<serde_json::Value> = (0..parcels).map(parcel).collect();
    entities[0]["layerId"] = json!("b");
    entities[0]["color"] = json!("#E5484D");
    entities[0]["lineWeight"] = json!(0.5);
    entities[0]["lineType"] = json!("dashed");
    let layer = |id: &str| {
        json!({"id":id,"name":id,"type":"layer","visible":true,"locked":false,"expanded":true,
                                  "style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]})
    };
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Özellik kopyala","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[layer("a"), layer("b")],"activeLayer":"a","entities":entities,
        "styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn input(doc: &Document, targets: std::ops::Range<u32>) -> EntitiesMatch {
    EntitiesMatch {
        source: doc.uid(Slot(1)).expect("known").to_string(),
        uids: targets
            .filter_map(|s| doc.uid(Slot(s)))
            .map(|u| u.to_string())
            .collect(),
        properties: None,
        expected_revision: None,
    }
}

#[test]
fn every_parcel_takes_the_source_s_layer_and_look_in_one_step() {
    let mut doc = drawing(50);
    let matching = input(&doc, 2..51);
    let result = entities_match::execute(&mut ExecutionContext::new(&mut doc), matching);
    let CommandResult::Completed { output, .. } = result else {
        panic!("{result:?}")
    };
    assert_eq!(output.changed.len(), 49);
    let Some(Entity::Polygon(p)) = doc.get(Slot(50)) else {
        panic!("a parcel")
    };
    assert_eq!(p.base.layer_id, "b");
    assert_eq!(p.base.line_weight, Some(0.5));
    assert_eq!(doc.undo().as_deref(), Some("Özellik kopyala"));
    assert_eq!(
        doc.get(Slot(50)).map(|e| e.base().layer_id.as_str()),
        Some("a")
    );
}

/// `cargo test --release -p kentos-native-application --test all match_props::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    use std::time::Instant;
    let doc = drawing(100_000);
    let matching = input(&doc, 2..10_002);
    let t = Instant::now();
    let CommandResult::Completed { output, .. } = entities_match::plan_in(&doc, &matching) else {
        panic!("not completed")
    };
    let plan = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(output.changed.len(), 10_000);
    let mut doc = doc;
    let t = Instant::now();
    let CommandResult::Completed { .. } =
        entities_match::execute(&mut ExecutionContext::new(&mut doc), matching)
    else {
        panic!("not completed")
    };
    let write = t.elapsed().as_secs_f64() * 1e3;
    let one = input(&doc, 50_000..50_001);
    let t = Instant::now();
    let CommandResult::Completed { .. } =
        entities_match::execute(&mut ExecutionContext::new(&mut doc), one)
    else {
        panic!("not completed")
    };
    let click = t.elapsed().as_secs_f64() * 1e3;
    println!(
        "100 000 nesnede: 10 000 parselin planı {plan:.1} ms, yazılması {write:.1} ms; tek tıklama {click:.2} ms"
    );
}
