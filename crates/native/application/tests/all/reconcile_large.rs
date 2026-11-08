//! `map.features.reconcile` on a large candidate set (plan/map-cad.md MAP-04, docs/adr/2940): the shared cases' drawing
//! with thousands more candidates east of its own, clusters of a LiDAR and an orthophoto direk a decimetre apart, every
//! fifth with a third candidate, the clusters five metres apart, and a direk asset there already beside every tenth.
//! The answer is what the clusters make: one group each, “existing” where the asset is, “agreed” elsewhere, with the
//! shared case's groups unchanged before them. The timing, run by hand in release, is the ADR's number.

use kentos_native_application::{ExecutionContext, registry};
use kentos_commands::CallOp;
use kentos_domain::Document;
use serde_json::{Value, json};

fn cases() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/commands/v1/map.features.reconcile.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("cases")).expect("JSON")
}

/// The shared drawing with `n` clusters east of it; the first case's input.
fn drawing(n: usize) -> (Document, Value) {
    let c = cases();
    let mut setup = c["setup"].clone();
    let mut ents = setup["entities"].as_array().expect("entities").clone();
    let mut uids = setup["uids"].as_array().expect("uids").clone();
    let mut id = ents.iter().filter_map(|e| e["id"].as_u64()).max().unwrap_or(0);
    let mut push = |ents: &mut Vec<Value>, layer: &str, attrs: Value, x: f64, y: f64| {
        id += 1;
        ents.push(json!({"kind": "point", "layerId": layer, "attrs": attrs, "p": {"x": x, "y": y}, "id": id}));
        uids.push(json!(format!("0192f5c0-2940-7000-8000-{id:012x}")));
    };
    for k in 0..n {
        let (x, y) = (501_000.0 + (k % 200) as f64 * 5.0, 4_400_000.0 + (k / 200) as f64 * 5.0);
        push(&mut ents, "aday-lidar", json!({"Kod": "DRK", "Güven": "0.90"}), x, y);
        push(&mut ents, "aday-orto", json!({"Kod": "DRK", "Güven": "0.80"}), x + 0.1, y + 0.05);
        if k % 5 == 0 {
            push(&mut ents, "aday-orto", json!({"Kod": "DRK", "Güven": "0.60"}), x - 0.05, y + 0.1);
        }
        if k % 10 == 0 {
            push(&mut ents, "hv-drk", json!({"Kod": "DRK", "Ad": format!("D-{k}")}), x + 0.2, y);
        }
    }
    setup["entities"] = Value::Array(ents);
    setup["uids"] = Value::Array(uids);
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("setup");
    (Document::from_snapshot_v2(snapshot).expect("opens"), c["cases"][0]["steps"][0]["input"].clone())
}

fn plan(doc: &mut Document, input: Value) -> Value {
    let out = registry::run(&mut ExecutionContext::new(doc), "map.features.reconcile", CallOp::Plan, input).expect("runs");
    assert_eq!(out["status"], "completed", "{out}");
    out["output"].clone()
}

#[test]
fn a_large_set_groups_as_its_clusters() {
    let n = 3000;
    let shared = cases()["cases"][0]["steps"][0]["result"]["output"]["groups"].as_array().expect("groups").len();
    let (mut doc, input) = drawing(n);
    let out = plan(&mut doc, input);
    let groups = out["groups"].as_array().expect("groups");
    assert_eq!(groups.len(), shared + n, "the shared case's groups and one for each cluster");
    let ours = &groups[shared..];
    let sizes: Vec<usize> = ours.iter().map(|g| g["candidates"].as_array().map_or(0, Vec::len)).collect();
    assert!(sizes.iter().enumerate().all(|(k, s)| *s == if k % 5 == 0 { 3 } else { 2 }), "{:?}", &sizes[..12]);
    let existing = ours.iter().filter(|g| g["status"] == "existing").count();
    let agreed = ours.iter().filter(|g| g["status"] == "agreed").count();
    assert_eq!((existing, agreed), (n / 10, n - n / 10));
    // The shared case's groups are as the case says.
    let want = &cases()["cases"][0]["steps"][0]["result"]["output"]["groups"];
    for (got, want) in groups[..shared].iter().zip(want.as_array().expect("groups")) {
        assert_eq!((&got["status"], &got["candidates"]), (&want["status"], &want["candidates"]));
    }
}

/// The time of a plan with 40 000 clusters (88 000 candidates), printed; run on its own in release:
///
/// ```text
/// cargo test --release -p kentos-native-application --test all reconcile_large::timing -- --ignored --nocapture
/// ```
#[test]
#[ignore = "a timing, run on its own in release"]
fn timing() {
    // KENTOS_RECONCILE_SIZES="2000,10000": the sizes, for the all-pairs code that 40 000 would keep for hours.
    let sizes: Vec<usize> = std::env::var("KENTOS_RECONCILE_SIZES")
        .unwrap_or_else(|_| "2000,10000,40000".into())
        .split(',')
        .filter_map(|t| t.trim().parse().ok())
        .collect();
    for n in sizes {
        let (mut doc, input) = drawing(n);
        let t = std::time::Instant::now();
        let out = plan(&mut doc, input);
        println!("{n} küme, {} grup: {:.3} s", out["groups"].as_array().map_or(0, Vec::len), t.elapsed().as_secs_f64());
    }
}
