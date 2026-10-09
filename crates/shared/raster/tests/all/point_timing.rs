//! The budgets of docs/adr/0232 §14, measured in a release build:
//!
//! ```text
//! cargo test --release -p kentos-raster --test all interpolation_timing -- --ignored --nocapture --test-threads=1
//! ```
//!
//! 100 000 random points over 2 km with a hill's heights, a 2048 × 2048
//! grid (about a metre a cell) worked out whole and written as the GeoTIFF:
//! the points gathered, the index or triangulation, the cells, the levels
//! and the coding all counted. Then a million points' Delaunay alone.

use std::time::Instant;

use kentos_contracts::{PointPart, Vec2 as Place};
use kentos_geometry_core::geom::delaunay::triangulate;
use kentos_geometry_core::vec2::Vec2;
use kentos_raster::from_points::{PointInput, PointJob, PointSpec};
use kentos_raster::par;
use kentos_raster::points::Source;
use serde_json::json;

/// Deterministic random numbers (xorshift64*).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn points(n: usize) -> Vec<Source> {
    let mut g = Rng(0x5eed);
    (0..n)
        .map(|_| {
            let (x, y) = (g.next() * 2048.0, g.next() * 2048.0);
            let z = 400.0 + 60.0 * libm::sin(x / 230.0) * libm::cos(y / 310.0) + 0.05 * x;
            Source::Point {
                p: Place {
                    x: 500_000.0 + x,
                    y: 4_420_000.0 + y,
                },
                z: Some(z),
                parts: None::<Vec<PointPart>>,
            }
        })
        .collect()
}

fn run(sources: &[Source], tool: serde_json::Value, threads: usize) -> usize {
    let spec: PointSpec = serde_json::from_value(json!({
        "tool": tool,
        "grid": { "affine": [500_000.0, 1.0, 0.0, 4_422_048.0, 0.0, -1.0], "width": 2048, "height": 2048 },
        "epsg": 5254,
    }))
    .unwrap();
    let input = PointInput::Sources {
        sources: sources.to_vec(),
        values: None,
    };
    let (mut job, header) = PointJob::new(input, &spec, threads).unwrap();
    let mut bytes = header.len();
    while !job.done() {
        bytes += job.step().unwrap().len();
    }
    bytes + job.finish().unwrap().tail.len()
}

/// The median and the slowest of `runs` runs (s).
fn time(runs: usize, f: &dyn Fn()) -> (f64, f64) {
    let mut t: Vec<f64> = (0..runs)
        .map(|_| {
            let s = Instant::now();
            f();
            s.elapsed().as_secs_f64()
        })
        .collect();
    t.sort_by(f64::total_cmp);
    (t[t.len() / 2], t[t.len() - 1])
}

#[test]
#[ignore = "timing, release build"]
fn interpolation_timing() {
    let threads = par::threads();
    println!("threads {threads}");
    let pts = points(100_000);
    let manual = json!({ "fit": "manual", "nugget": 0.0, "sill": 1800.0, "range": 400.0 });
    for (name, tool, budget, one) in [
        (
            "IDW (12 nokta)",
            json!({ "kind": "idw", "power": 2.0, "points": 12 }),
            1.0,
            true,
        ),
        ("TIN'den raster", json!({ "kind": "tin" }), 0.5, false),
        (
            "Doğal komşu",
            json!({ "kind": "naturalNeighbor" }),
            3.0,
            true,
        ),
        (
            "Spline (düzenlemeli, 12)",
            json!({ "kind": "spline", "spline": "regularized", "weight": 0.1, "points": 12 }),
            2.0,
            false,
        ),
        (
            "Kriging (küresel, 12)",
            json!({ "kind": "kriging", "model": "spherical", "variogram": manual, "points": 12 }),
            2.0,
            false,
        ),
        (
            "Çekirdek yoğunluğu (20 m)",
            json!({ "kind": "kernel", "radius": 20.0, "kernel": "quartic", "unit": "squareKilometre" }),
            1.0,
            false,
        ),
    ] {
        let (p50, worst) = time(3, &|| {
            run(&pts, tool.clone(), threads);
        });
        println!("{name:<28} 2048²  p50 {p50:6.3} s  en yavaş {worst:6.3} s  bütçe {budget} s");
        if one {
            let (p50, _) = time(1, &|| {
                run(&pts, tool.clone(), 1);
            });
            println!("{name:<28} 2048²  tek iş parçacığı {p50:6.3} s");
        }
    }
    let mut g = Rng(42);
    let million: Vec<Vec2> = (0..1_000_000)
        .map(|_| Vec2::new(g.next() * 1e4, g.next() * 1e4))
        .collect();
    let (p50, worst) = time(3, &|| {
        triangulate(&million).unwrap();
    });
    println!(
        "{:<28} 10⁶ nokta  p50 {p50:6.3} s  en yavaş {worst:6.3} s  bütçe 1,5 s",
        "Delaunay"
    );
}
