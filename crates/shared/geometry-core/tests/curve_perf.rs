//! How long snapping, picking and window selection take among many curves
//! now that a fit-point curve and an ellipse are taken by chords within
//! 0.1 mm (docs/adr/0149 §5.3): a TM-sized drawing of 2 000 curves through
//! six fit points (spans of 10–40 m) and 500 ellipses, every snap kind on.
//! Run by hand in release:
//!
//! ```text
//! cargo test --release -p kentos-geometry-core --test curve_perf -- --ignored --nocapture
//! ```

// Test harness code, not the core: the std float methods are fine here.
#![allow(clippy::disallowed_methods)]

use std::time::Instant;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::snap::SnapKind;
use serde_json::{Value, json};

#[test]
#[ignore = "timings, run by hand in release"]
fn snaps_picks_and_selections_among_many_curves() {
    let (e, n) = (487_000.0, 4_420_000.0);
    let mut seed = 20_261_001_u64;
    let mut random = move || {
        seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
        (seed >> 11) as f64 / (1u64 << 53) as f64
    };
    let mut objects: Vec<Value> = Vec::new();
    for i in 0..2000 {
        let (x0, y0) = (e + random() * 3000.0, n + random() * 3000.0);
        let pts: Vec<Value> = (0..6)
            .map(|k| {
                let t = k as f64;
                json!({ "x": x0 + t * (10.0 + random() * 30.0), "y": y0 + (random() - 0.5) * 40.0 })
            })
            .collect();
        objects.push(json!({ "id": i + 1, "layerId": "a", "attrs": {}, "kind": "spline", "pts": pts, "closed": false }));
    }
    for i in 0..500 {
        objects.push(json!({
            "id": 2001 + i, "layerId": "a", "attrs": {}, "kind": "ellipse",
            "c": { "x": e + random() * 3000.0, "y": n + random() * 3000.0 },
            "major": { "x": 5.0 + random() * 60.0, "y": random() * 20.0 },
            "ratio": 0.2 + random() * 0.8, "t0": 0.0, "t1": 0.0
        }));
    }
    let mut store = Store::new();
    store
        .put_json(&Value::Array(objects).to_string())
        .expect("objects");
    store
        .set_layers_json(&json!([{ "id": "a", "visible": true, "locked": false, "pickInterior": false, "label": { "placement": "center", "minFeaturePx": 20 } }]).to_string())
        .expect("layers");
    let all = SnapKind::ALL.iter().fold(0, |m, k| m | k.bit());
    let points: Vec<Vec2> = (0..2000)
        .map(|_| Vec2::new(e + random() * 3000.0, n + random() * 3000.0))
        .collect();
    let time = |what: &str, f: &mut dyn FnMut(Vec2)| {
        let start = Instant::now();
        for &p in &points {
            f(p);
        }
        let each = start.elapsed().as_secs_f64() * 1000.0 / points.len() as f64;
        println!("{what}: {each:.3} ms a query");
        each
    };
    // A 20 m reach, about 40 px at a 1:1000 view.
    let snap = time("kenet (bütün türler)", &mut |p| {
        let _ = store.snap(p, 20.0, all, None);
    });
    let hit = time("tıklama", &mut |p| {
        let _ = store.hit(p, 2.0);
    });
    // A crossing window of 200 m, as a drag at an overview.
    let window = time("kesişim penceresi (200 m)", &mut |p| {
        let r = kentos_geometry_core::geometry::Bounds {
            min_x: p.x - 100.0,
            min_y: p.y - 100.0,
            max_x: p.x + 100.0,
            max_y: p.y + 100.0,
        };
        let _ = store.in_rect(&r, true);
    });
    assert!(
        snap < 5.0 && hit < 5.0 && window < 50.0,
        "kenet {snap} ms, tıklama {hit} ms, pencere {window} ms"
    );
}
