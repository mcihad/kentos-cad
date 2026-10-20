//! A layer's digest is the SHA-256 of the canonical JSON of its objects' `[uid, essence]` list in the ids' order
//! (docs/adr/0511); made across threads for a big layer (docs/adr/0551 §5) it is the same bytes, so the same digest.

use kentos_commands::DocumentSnapshotV1;
use kentos_domain::Document;
use kentos_native_application::geopackage::essence;
use kentos_native_application::sources::layer_digest;
use serde_json::{Value, json};

fn drawing(points: usize) -> Document {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/processing/v1/geometry.kcad");
    let mut doc: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("reads")).expect("JSON");
    let list = doc["entities"].as_array_mut().expect("entities");
    for i in 0..points {
        list.push(json!({
            "kind": "point", "id": 1000 + i, "layerId": "agac",
            "attrs": { "Tür": format!("Çam {i}"), "Yarıçap": format!("{},5", i % 9) },
            "p": { "x": 487000.0 + (i as f64) * 0.37, "y": 4420000.0 + (i % 97) as f64 * 1.25 },
            "z": 1200.0 + (i % 13) as f64 / 8.0
        }));
    }
    Document::from_snapshot(DocumentSnapshotV1::from_json(&doc.to_string()).expect("a drawing"))
        .expect("opens")
}

/// What one thread would hash: the whole list's canonical JSON, built at once.
fn serial(doc: &Document, layer: &str) -> String {
    let mut items: Vec<_> = doc.by_layer_with_uids(layer).collect();
    items.sort_by_key(|(u, _)| *u);
    let list: Vec<Value> = items
        .iter()
        .map(|(u, e)| json!([u.to_string(), essence(e)]))
        .collect();
    let bytes = kentos_commands::canonical::canonical_json(&Value::Array(list));
    kentos_commands::encoding::sha256_hex(bytes.as_bytes())
}

#[test]
fn a_big_layers_digest_is_what_one_thread_would_make() {
    // Past one round (32 768) so a second round starts with a comma, and past the split.
    let doc = drawing(40_000);
    assert_eq!(layer_digest(&doc, "agac"), serial(&doc, "agac"));
}

#[test]
fn small_and_empty_layers_keep_their_digests() {
    let doc = drawing(10);
    for layer in ["parsel", "agac", "yol", "bos"] {
        assert_eq!(layer_digest(&doc, layer), serial(&doc, layer), "{layer}");
    }
    let doc = drawing(3_000);
    assert_eq!(layer_digest(&doc, "agac"), serial(&doc, "agac"));
}
