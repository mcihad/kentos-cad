//! A layer described as a dataset, timed (docs/adr/0633 §6): 100 000 parcels with three fields, and a layer recorded
//! as made from them; describing both (the extent, the WGS 84 box, the dictionary, the inputs' digests, the graph)
//! and the digest a processing run takes of its input before it writes.
//!
//! ```text
//! cargo test --release -p kentos-native-application --test all dataset_timing -- --ignored --nocapture
//! ```

use std::time::Instant;

use kentos_commands::{DatasetsDescribe, DocumentSnapshotV1};
use kentos_domain::Document;
use kentos_native_application::{ExecutionContext, datasets_describe, sources};
use serde_json::json;

const PARCELS: usize = 100_000;

fn drawing() -> Document {
    let entities: Vec<serde_json::Value> = (0..PARCELS)
        .map(|i| {
            let (x, y) = (500_000.0 + (i % 1000) as f64 * 20.0, 4_400_000.0 + (i / 1000) as f64 * 20.0);
            json!({"kind": "polygon", "id": i + 1, "layerId": "parsel",
                   "attrs": {"no": format!("{}/{}", i / 1000, i % 1000), "alan": "400"},
                   "pts": [{"x": x, "y": y}, {"x": x + 20.0, "y": y}, {"x": x + 20.0, "y": y + 20.0}, {"x": x, "y": y + 20.0}]})
        })
        .collect();
    let layer = |id: &str, extra: serde_json::Value| {
        let mut n = json!({"id": id, "name": id, "type": "layer", "visible": true, "locked": false, "expanded": true,
                           "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": []});
        if let (Some(n), Some(e)) = (n.as_object_mut(), extra.as_object()) {
            n.extend(e.clone());
        }
        n
    };
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "Kent",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000},
        "origin": {"x": 500000, "y": 4400000},
        "layers": [
            layer("parsel", json!({"fields": [{"name": "no", "kind": "text"}, {"name": "alan", "kind": "decimal"}]})),
            layer("tampon", json!({"derivation": {"tool": "geometry.buffer", "label": "Tampon", "toolVersion": 1,
                "inputs": [{"layer": "parsel", "name": "parsel", "digest": "0".repeat(64)}],
                "documentRevision": "0", "engine": "KentOS", "at": "2026-10-25T00:00:00Z"}})),
        ],
        "activeLayer": "parsel", "entities": entities, "styles": {"items": [], "categories": []}
    });
    Document::from_snapshot(
        DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("a drawing"),
    )
    .expect("opens")
}

#[test]
#[ignore = "a measurement, run by hand"]
fn dataset_timing() {
    let mut doc = drawing();
    let best = |f: &mut dyn FnMut()| {
        (0..3)
            .map(|_| {
                let t = Instant::now();
                f();
                t.elapsed().as_secs_f64() * 1000.0
            })
            .fold(f64::MAX, f64::min)
    };
    let digest = best(&mut || {
        let _ = sources::layer_digest(&doc, "parsel");
    });
    println!("{:<44} {digest:>9.1} ms", "100 000 parselin içerik özeti");
    for (what, layers) in [
        ("parsellerin tanımı", vec!["parsel".to_owned()]),
        (
            "türetilmiş katmanın tanımı (girdinin özeti)",
            vec!["tampon".to_owned()],
        ),
        ("bütün katmanlar", Vec::new()),
    ] {
        let ms = best(&mut || {
            let _ = datasets_describe::execute(
                &mut ExecutionContext::new(&mut doc),
                DatasetsDescribe {
                    layers: layers.clone(),
                    records: true,
                },
            );
        });
        println!("{what:<44} {ms:>9.1} ms");
    }
}
