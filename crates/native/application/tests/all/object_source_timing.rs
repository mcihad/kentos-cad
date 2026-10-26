//! An object's source, timed (docs/adr/0651 §1): the 100 000 parcels of `.run/perf/atlas-100k.gpkg` (written by the
//! atlas timing, docs/adr/0638) bound to a layer, one of them changed; how long `object_source` takes for one object,
//! as Bilgi kartı asks it once per change.
//!
//! ```text
//! cargo test --release -p kentos-native-application --test all object_source_timing -- --ignored --nocapture
//! ```

use std::path::PathBuf;
use std::time::Instant;

use kentos_commands::{CallOp, DocumentSnapshotV1, ObjectSourceState};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, registry, sources};

#[test]
#[ignore = "süre ölçümü: --release ve --ignored ile"]
fn object_source_timing() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../.run/perf/atlas-100k.gpkg");
    if !file.is_file() {
        eprintln!(
            "⚠ {} yok: önce kentos-plot'un atlas_timing'i onu yazar",
            file.display()
        );
        return;
    }
    let dir = std::env::temp_dir().join(format!("kentos-kaynak-satiri-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let copy = dir.join("parsel.gpkg");
    std::fs::copy(&file, &copy).expect("copied");
    // The table is in TM30 (EPSG:5253): the project too.
    let text = include_str!("../../../../../fixtures/document/v1/sample.json").replacen(
        "\"srid\": 5256",
        "\"srid\": 5253",
        1,
    );
    let snapshot = DocumentSnapshotV1::from_json(&text).expect("the sample");
    let mut doc = Document::from_snapshot(snapshot).expect("a drawing");
    let run = |doc: &mut Document, command: &str, input: serde_json::Value| {
        let answer = registry::run(
            &mut ExecutionContext::new(doc),
            command,
            CallOp::Execute,
            input,
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
        answer
    };
    let t = Instant::now();
    let linked = run(
        &mut doc,
        "cad.sources.edit",
        serde_json::json!({ "edit": { "kind": "link", "path": copy.display().to_string(), "table": "parsel" } }),
    );
    println!("bağlama: {:.2} s", t.elapsed().as_secs_f64());
    let layer = linked["output"]["link"]["layer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let uids: Vec<_> = doc.by_layer_with_uids(&layer).map(|(u, _)| u).collect();
    println!("nesne: {}", uids.len());
    let slot = |doc: &Document, i: usize| doc.slot_of(uids[i]).expect("a slot");
    let t = Instant::now();
    let same = sources::object_source(&doc, slot(&doc, 7)).expect("a source");
    println!(
        "değişmemiş katmanda bir nesnenin kaynağı: {:.1} ms",
        t.elapsed().as_secs_f64() * 1e3
    );
    assert_eq!(same.state, ObjectSourceState::Matched);
    run(
        &mut doc,
        "cad.entities.set",
        serde_json::json!({ "uids": [uids[7].to_string()], "attrs": { "durum": "x" }, "operation": "attributes" }),
    );
    let mut best = f64::MAX;
    for _ in 0..5 {
        let t = Instant::now();
        let o = sources::object_source(&doc, slot(&doc, 7)).expect("a source");
        best = best.min(t.elapsed().as_secs_f64() * 1e3);
        assert_eq!(o.state, ObjectSourceState::Edited);
    }
    println!("değişmiş katmanda bir nesnenin kaynağı: {best:.1} ms (en iyi 5)");
    let _ = std::fs::remove_dir_all(&dir);
}
