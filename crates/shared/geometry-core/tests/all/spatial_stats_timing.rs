//! The budgets of docs/adr/0238 §12, measured in a release build:
//!
//! ```text
//! cargo test --release -p kentos-geometry-core --test all spatial_stats_timing -- --ignored --nocapture --test-threads=1
//! ```
//!
//! 100 000 points over 10 km × 10 km (a thousand a km², on a 1 cm lattice)
//! with a smooth value and noise; the band for Moran's I and Gi\* gives about
//! ten neighbours a point (λπr² = 10). Each tool's function as the desktop
//! calls it; Moran's I also through the call the web makes (its JSON both ways).

use std::time::Instant;

use kentos_geometry_core::api::run_named;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::jsmath::{cos, sin};
use kentos_geometry_core::ops::spatial_stats::autocorrelation::{
    Neighbourhood, hot_spots, morans_i,
};
use kentos_geometry_core::ops::spatial_stats::centers::{CenterInput, CenterKind, centers};
use kentos_geometry_core::ops::spatial_stats::clusters::{dbscan, k_means};
use kentos_geometry_core::ops::spatial_stats::nearest::nearest;
use kentos_geometry_core::ops::spatial_stats::weights::Concept;
use kentos_geometry_core::vec2::Vec2;

fn hash(i: u64) -> u64 {
    let mut h = i.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    h ^= h >> 31;
    h = h.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    h ^ (h >> 29)
}

fn scene(n: usize) -> (Vec<Shape>, Vec<Option<String>>) {
    let mut shapes = Vec::with_capacity(n);
    let mut values = Vec::with_capacity(n);
    for i in 0..n as u64 {
        let x = (hash(2 * i) % 1_000_000) as f64 / 100.0;
        let y = (hash(2 * i + 1) % 1_000_000) as f64 / 100.0;
        shapes.push(Shape::Point {
            p: Vec2::new(500_000.0 + x, 4_400_000.0 + y),
            z: None,
            parts: None,
        });
        let v =
            100.0 + 40.0 * sin(x / 1500.0) * cos(y / 2100.0) + (hash(i + 7) % 1000) as f64 / 50.0;
        values.push(Some(format!("{v:.2}")));
    }
    (shapes, values)
}

fn time<T>(name: &str, budget: f64, f: impl Fn() -> T) -> T {
    let mut best = f64::INFINITY;
    let mut out = None;
    for _ in 0..3 {
        let s = Instant::now();
        let r = f();
        let t = s.elapsed().as_secs_f64();
        if t < best {
            best = t;
        }
        out = Some(r);
    }
    println!("{name:<48} {best:8.4} s  bütçe {budget} s");
    out.expect("ran")
}

#[test]
#[ignore = "timing, release build"]
fn spatial_stats_timing() {
    let n = 100_000;
    let (shapes, values) = scene(n);
    let band = (10.0 / (std::f64::consts::PI * 1e-3)).sqrt();
    let plain = CenterInput {
        weights: None,
        groups: None,
        weight_field: "",
        k: 1.0,
    };
    for (name, kind) in [
        ("Ortalama merkez", CenterKind::Mean),
        ("Ortanca merkez", CenterKind::Median),
        ("Standart uzaklık", CenterKind::Distance),
        ("Yön dağılımı", CenterKind::Ellipse),
    ] {
        time(name, 0.2, || centers(&shapes, kind, &plain).expect("ran"));
    }
    time("En yakın komşu", 0.5, || {
        nearest(&shapes, None).expect("ran")
    });
    let hood = Neighbourhood {
        concept: Concept::Band,
        band: Some(band),
        k: 8,
    };
    let r = time("Moran I, sabit bant", 1.0, || {
        morans_i(&shapes, &values, "Değer", hood, true).expect("ran")
    });
    println!("  {}", r.summary);
    let args = serde_json::to_string(&serde_json::json!([
        shapes
            .iter()
            .map(|s| match s {
                Shape::Point { p, .. } =>
                    serde_json::json!({ "kind": "point", "p": { "x": p.x, "y": p.y } }),
                _ => serde_json::Value::Null,
            })
            .collect::<Vec<_>>(),
        values,
        "Değer",
        "band",
        band,
        8,
        true
    ]))
    .expect("JSON");
    time("Moran I, web'in çağrısıyla (JSON)", 1.5, || {
        run_named("statsMoran", &args).expect("ran")
    });
    let r = time("Sıcak nokta, sabit bant", 1.0, || {
        hot_spots(&shapes, &values, "Değer", hood).expect("ran")
    });
    println!("  {}", r.summary);
    let r = time("DBSCAN, ε 40 m, 5 nokta", 1.0, || {
        dbscan(&shapes, 40.0, 5, false).expect("ran")
    });
    println!("  {}", r.summary);
    let r = time("k-ortalamalar, k = 10", 1.0, || {
        k_means(&shapes, 10).expect("ran")
    });
    println!("  {}", r.summary);
}
