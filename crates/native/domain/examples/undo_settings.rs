//! What undoing and redoing a settings step costs when the setting is large (plan/argon-tavsiye.md ADIM 03): value
//! domains of many choices, a GIS project's code lists. Measurement only; nothing is written.
//!
//!     cargo run --release -p kentos-domain --example undo_settings -- [domains] [choices each]
use std::time::Instant;

use kentos_contracts::{DocumentSnapshotV1, FieldDomain};
use kentos_domain::Document;

fn main() {
    let mut args = std::env::args().skip(1);
    let domains: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(200);
    let choices: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(500);
    let snapshot: DocumentSnapshotV1 = serde_json::from_value(serde_json::json!({
        "format": "kentos.document", "version": 1, "name": "olcum",
        "settings": {"srid": 0, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 500.0, "workspace": "gis"},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false,
            "expanded": true, "style": {"color": "#E5484D", "lineType": "continuous", "lineWeight": 0.25},
            "children": []}],
        "activeLayer": "cizim", "entities": [], "styles": {"items": [], "categories": []}
    }))
    .expect("a snapshot");
    let mut doc = Document::from_snapshot(snapshot).expect("opens");
    let list = |tag: &str| -> Vec<FieldDomain> {
        (0..domains)
            .map(|d| {
                serde_json::from_value(serde_json::json!({
                    "id": format!("alan-{d}"), "name": format!("Alan {d} {tag}"), "kind": "text",
                    "values": (0..choices)
                        .map(|c| serde_json::json!({"code": format!("K{c}"), "label": format!("Seçenek {c} {tag}")}))
                        .collect::<Vec<_>>()
                }))
                .expect("a domain")
            })
            .collect()
    };
    doc.set_domains(list("a"), "Değer alanları").expect("set");
    doc.set_domains(list("b"), "Değer alanları").expect("set");
    let rounds = 50;
    let t = Instant::now();
    for _ in 0..rounds {
        doc.undo();
        doc.redo();
    }
    let each = t.elapsed().as_secs_f64() * 1000.0 / f64::from(2 * rounds);
    println!("{domains} alan × {choices} seçenek: bir geri alma ya da yineleme {each:.3} ms");
}
