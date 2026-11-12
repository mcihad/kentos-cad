//! What a reading copy, a settings change and an ordinary edit cost on a project whose settings and style library are
//! heavy (plan/argon-tavsiye.md ADIM 01): bound sources' bases (a base's fields may be 8 MiB), a symbol library. A
//! reading copy is taken for every plan, every follower preview of a grip drag and every background job. Measurement
//! only; nothing is written.
//!
//!     cargo run --release -p kentos-domain --example reading_copy -- [MB of bases] [symbols]
use std::time::Instant;

use kentos_contracts::{DocumentSnapshotV1, Entity, SourceBase};
use kentos_domain::Document;

fn main() {
    let mut args = std::env::args().skip(1);
    let megabytes: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(16);
    let symbols: usize = args.next().and_then(|a| a.parse().ok()).unwrap_or(2000);
    let snapshot = serde_json::json!({
        "format": "kentos.document", "version": 1, "name": "olcum",
        "settings": {"srid": 0, "lengthDecimals": 2, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 500.0, "workspace": "cad"},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [{"id": "cizim", "name": "Çizim", "type": "layer", "visible": true, "locked": false,
            "expanded": true, "style": {"color": "#E5484D", "lineType": "continuous", "lineWeight": 0.25},
            "children": []}],
        "activeLayer": "cizim", "entities": [], "styles": {"items": [], "categories": []}
    });
    let mut snapshot: DocumentSnapshotV1 = serde_json::from_value(snapshot).expect("a snapshot");
    // The bases: four bound sources of `megabytes` / 4 each.
    let each = megabytes * 1024 * 1024 / 4;
    snapshot.settings.source_bases = (0..4)
        .map(|i| SourceBase {
            source: format!("kaynak-{i}"),
            revision: "1".into(),
            digest: "0".repeat(64),
            rows: "r".repeat(each / 2),
            fields: Some("f".repeat(each / 2)),
        })
        .collect();
    // The symbol library: `symbols` items of about 2 KB each.
    snapshot.styles.items = (0..symbols)
        .map(|i| serde_json::json!({"id": format!("sembol-{i}"), "svg": "M0 0 L1 1 ".repeat(200)}))
        .collect();
    let mut doc = Document::from_snapshot(snapshot).expect("opens");
    let line = |x: f64| -> Entity {
        serde_json::from_value(
            serde_json::json!({"kind": "line", "id": 0, "layerId": "cizim", "attrs": {},
            "a": {"x": x, "y": 0.0}, "b": {"x": x, "y": 1.0}}),
        )
        .expect("a line")
    };
    let rounds = 100;
    let t = Instant::now();
    for _ in 0..rounds {
        std::hint::black_box(doc.reading_copy());
    }
    let copy = t.elapsed().as_secs_f64() * 1000.0 / f64::from(rounds);
    let t = Instant::now();
    for i in 0..rounds {
        doc.add(line(f64::from(i))).expect("a slot");
    }
    let add = t.elapsed().as_secs_f64() * 1000.0 / f64::from(rounds);
    let t = Instant::now();
    for i in 0..rounds {
        let mut settings = doc.settings().clone();
        settings.plot_scale = 500.0 + f64::from(i % 2);
        doc.set_settings(settings);
    }
    let set = t.elapsed().as_secs_f64() * 1000.0 / f64::from(rounds);
    let t = Instant::now();
    for _ in 0..rounds {
        doc.undo();
    }
    let undo = t.elapsed().as_secs_f64() * 1000.0 / f64::from(rounds);
    println!(
        "kaynak özetleri {megabytes} MB, {symbols} sembol: okuma kopyası {copy:.3} ms, çizgi ekleme {add:.3} ms, \
         ayar değişikliği {set:.3} ms, geri alma {undo:.3} ms"
    );
}
