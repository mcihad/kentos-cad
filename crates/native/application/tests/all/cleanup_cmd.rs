//! Çizimi temizle by its meaning over a large drawing (docs/adr/0305 §5): what the plan and the write take for
//! 100 000 objects, a tenth of them duplicates, a tenth pieces of longer lines, and 10 000 paths with straight
//! vertices. The shared cases are `fixtures/commands/v1/cad.entities.cleanup.json`.

use kentos_domain::Document;
use kentos_domain::contracts::{CommandResult, DocumentSnapshotV1, EntitiesCleanup};
use kentos_native_application::{ExecutionContext, entities_cleanup};
use serde_json::json;

fn drawing() -> Document {
    let mut entities = Vec::new();
    let mut id = 0;
    let mut push = |mut e: serde_json::Value| {
        id += 1;
        e["id"] = json!(id);
        e["layerId"] = json!("a");
        e["attrs"] = json!({});
        entities.push(e);
    };
    for i in 0..70_000u32 {
        let (x, y) = (f64::from(i % 300) * 3.0, f64::from(i / 300) * 3.0);
        push(json!({"kind":"line","a":{"x":x,"y":y},"b":{"x":x + 2.0,"y":y}}));
        if i % 7 == 0 {
            push(json!({"kind":"line","a":{"x":x + 2.0,"y":y},"b":{"x":x,"y":y}}));
        }
        if i % 7 == 1 {
            push(json!({"kind":"line","a":{"x":x + 1.0,"y":y},"b":{"x":x + 2.5,"y":y}}));
        }
    }
    for i in 0..10_000u32 {
        let (x, y) = (
            f64::from(i % 100) * 10.0,
            1000.0 + f64::from(i / 100) * 10.0,
        );
        push(
            json!({"kind":"polyline","pts":[{"x":x,"y":y},{"x":x + 3.0,"y":y},{"x":x + 6.0,"y":y},{"x":x + 6.0,"y":y + 4.0}]}),
        );
    }
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Çizimi temizle","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":entities,"styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn whole() -> EntitiesCleanup {
    EntitiesCleanup {
        uids: None,
        tolerance: None,
        ignore: Vec::new(),
        overlaps: None,
        end_to_end: None,
        straight_vertices: None,
        expected_revision: None,
    }
}

/// `cargo test --release -p kentos-native-application --test all cleanup_cmd::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    use std::time::Instant;
    let mut doc = drawing();
    let n = doc.len();
    let t = Instant::now();
    let CommandResult::Completed { output: plan, .. } = entities_cleanup::plan_in(&doc, &whole())
    else {
        panic!("not completed")
    };
    let planned = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        entities_cleanup::execute(&mut ExecutionContext::new(&mut doc), whole())
    else {
        panic!("not completed")
    };
    let written = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(output.report, plan.report);
    let r = &output.report;
    println!(
        "{n} nesne: plan {planned:.0} ms, yazma {written:.0} ms; {} yinelenen, {} çizgi {} çizgiye, {} köşe",
        r.duplicates, r.merged, r.merging, r.vertices
    );
}
