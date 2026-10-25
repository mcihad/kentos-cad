//! A revision's difference over a large drawing (docs/adr/0637 §4): what `drawing_compare::diff` takes for 100 000
//! parcels, a twentieth of them moved and a twentieth with an attribute changed, as the split area's revision view
//! marks them after every edit of the open drawing. The shared cases are `fixtures/commands/v1/cad.drawing.compare.json`.

use std::time::Instant;

use kentos_commands::{CallOp, DocumentSnapshotV1};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, drawing_compare, registry};
use serde_json::json;

fn parcels(n: u32) -> Document {
    let mut entities = Vec::with_capacity(n as usize);
    for i in 0..n {
        let (x, y) = (f64::from(i % 400) * 12.0, f64::from(i / 400) * 12.0);
        entities.push(json!({"kind":"polygon","id":i + 1,"layerId":"parsel","attrs":{"ada":(i / 40).to_string(),"parsel":(i % 40).to_string()},
            "pts":[{"x":x,"y":y},{"x":x + 10.0,"y":y},{"x":x + 10.0,"y":y + 10.0},{"x":x,"y":y + 10.0}]}));
    }
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Fark","settings":{"srid":5256,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"gis"},
        "origin":{"x":0,"y":0},"layers":[{"id":"parsel","name":"Parsel","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"parsel","entities":entities,"styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

/// `cargo test --release -p kentos-native-application --test all compare_timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn compare_timing() {
    let n = 100_000u32;
    let old = parcels(n);
    let mut new = old.clone();
    let uids: Vec<String> = new
        .entities()
        .map(|e| e.base().id)
        .filter(|id| id % 20 == 0)
        .filter_map(|id| new.uid(kentos_domain::Slot(id)).map(|u| u.to_string()))
        .collect();
    let moved: Vec<String> = uids.iter().step_by(2).cloned().collect();
    let renamed: Vec<String> = uids.iter().skip(1).step_by(2).cloned().collect();
    let run = |doc: &mut Document, command: &str, input: serde_json::Value| {
        let a = registry::run(
            &mut ExecutionContext::new(doc),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(a["status"], "completed", "{a}");
    };
    run(
        &mut new,
        "cad.entities.transform",
        json!({ "uids": moved, "transform": { "kind": "move", "dx": 0.5, "dy": 0.0 } }),
    );
    run(
        &mut new,
        "cad.entities.set",
        json!({ "uids": renamed, "attrs": { "ada": "yeni" }, "operation": "attributes" }),
    );
    let mut times = Vec::new();
    let mut d = drawing_compare::Diff::default();
    for _ in 0..5 {
        let t = Instant::now();
        d = drawing_compare::diff(&old, &new, kentos_commands::COMPARE_TOLERANCE);
        times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    println!(
        "{n} parsel: fark {:.0} ms (en az {:.0}); değişen {}, aynı {}",
        times[2],
        times[0],
        d.changed.len(),
        d.same
    );
    assert_eq!(d.changed.len(), uids.len());
    assert_eq!(d.same, n as usize - uids.len());
}
