//! The budgets of docs/adr/0233 §15, measured in a release build:
//!
//! ```text
//! cargo test --release -p kentos-raster --test all ops_timing -- --ignored --nocapture --test-threads=1
//! ```
//!
//! 4096 × 4096 Float32 rasters (tiled, Deflate GeoTIFFs in memory) worked
//! out whole as a host runs a job: the blocks handed over and decoded, the
//! cells, the levels and the coding all counted; 10 000 parcels for the
//! mask and the zones.

use std::time::Instant;

use kentos_contracts::RasterSample;
use kentos_formats::raster::source::open_bytes;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::vec2::Vec2;
use kentos_raster::inputs::Input;
use kentos_raster::job::READER_BUDGET;
use kentos_raster::ops::{OpsJob, OpsSpec};
use kentos_raster::par;
use serde_json::{Value, json};

use crate::raster_ops::{Raster, tiff};

const NAMES: [&str; 5] = ["A", "B", "C", "D", "E"];

fn field(n: u32, affine: [f64; 6], seed: f64) -> Vec<u8> {
    let values = (0..n * n)
        .map(|k| {
            let (x, y) = (f64::from(k % n), f64::from(k / n));
            400.0 + 120.0 * libm::sin(x / 230.0 + seed) * libm::cos(y / 310.0) + 0.05 * x
        })
        .collect();
    tiff(&Raster {
        affine,
        width: n,
        height: n,
        bands: 1,
        sample: RasterSample::F32,
        nodata: Some(f64::NAN),
        alpha: false,
        values,
    })
}

/// 10 000 parcels, 100 × 100 of them over the raster's 8192 m.
fn parcels() -> Vec<Shape> {
    let mut out = Vec::new();
    for j in 0..100 {
        for i in 0..100 {
            let (x0, y0) = (
                500_000.0 + f64::from(i) * 81.92 + 3.0,
                4_420_000.0 - 8192.0 + f64::from(j) * 81.92 + 3.0,
            );
            out.push(Shape::Polygon {
                pts: vec![
                    Vec2::new(x0, y0),
                    Vec2::new(x0 + 70.0, y0 + 5.0),
                    Vec2::new(x0 + 75.0, y0 + 72.0),
                    Vec2::new(x0 - 2.0, y0 + 66.0),
                ],
                bulges: None,
                holes: None,
                parts: None,
            });
        }
    }
    out
}

fn run(files: &[(Vec<u8>, [f64; 6])], tool: &Value, shapes: &[Shape], threads: usize) -> usize {
    let spec: OpsSpec = serde_json::from_value(json!({
        "tool": tool,
        "inputs": files.iter().enumerate().map(|(k, (_, a))| json!({ "affine": a, "name": NAMES[k] })).collect::<Vec<_>>(),
    }))
    .unwrap();
    let inputs = files
        .iter()
        .map(|(f, a)| Input::new(open_bytes(f, None, READER_BUDGET).unwrap(), *a, None).unwrap())
        .collect();
    let (mut job, header) = OpsJob::new(inputs, &spec, shapes.to_vec(), threads).unwrap();
    let mut bytes = header.len();
    while !job.done() {
        let blocks = job
            .needs()
            .into_iter()
            .map(|(k, n)| {
                let f = &files[k as usize].0;
                (
                    k,
                    n,
                    f[n.offset as usize..(n.offset + n.len) as usize].to_vec(),
                )
            })
            .collect();
        job.put_all(blocks).unwrap();
        bytes += job.step().unwrap().len();
    }
    let _ = job.finish().unwrap();
    bytes
}

/// A timed job: its name, input files and places, the tool, whether it reads the parcels, its budget (s).
type Job = (&'static str, Vec<(Vec<u8>, [f64; 6])>, Value, bool, f64);

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
fn ops_timing() {
    let threads = par::threads();
    println!("threads {threads}");
    let n = 4096;
    let place = [500_000.0, 2.0, 0.0, 4_420_000.0, 0.0, -2.0];
    let a = (field(n, place, 0.0), place);
    let b = (field(n, place, 1.0), place);
    let quarter = |i: f64, j: f64| {
        [
            500_000.0 + i * 4096.0,
            2.0,
            0.0,
            4_420_000.0 - j * 4096.0,
            0.0,
            -2.0,
        ]
    };
    let tiles: Vec<(Vec<u8>, [f64; 6])> = [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)]
        .iter()
        .map(|&(i, j)| (field(2048, quarter(i, j), i + 2.0 * j), quarter(i, j)))
        .collect();
    let stack: Vec<(Vec<u8>, [f64; 6])> = (0..5)
        .map(|k| (field(n, place, f64::from(k)), place))
        .collect();
    let shapes = parcels();
    let jobs: Vec<Job> = vec![
        (
            "Hesaplayıcı [A] * 2 + [B]",
            vec![a.clone(), b.clone()],
            json!({ "kind": "calculator", "expression": "[A] * 2 + [B]", "empty": "propagate", "sample": "f32" }),
            false,
            1.0,
        ),
        (
            "Yeniden sınıflandır, 10 kural",
            vec![a.clone()],
            json!({ "kind": "reclassify", "band": 1, "table": "* 300 1; 300 320 2; 320 340 3; 340 360 4; 360 380 5; 380 400 6; 400 420 7; 420 440 8; 440 460 9; 460 * 10", "bounds": "upperClosed", "unmatched": "keep", "sample": "i32" }),
            false,
            0.6,
        ),
        (
            "Maskeyle kırp, 10 000 parsel",
            vec![a.clone()],
            json!({ "kind": "clipByMask", "crop": true }),
            true,
            1.0,
        ),
        (
            "Mozaik, dört 2048²",
            tiles,
            json!({ "kind": "mosaic", "overlap": "top", "sampling": "nearest" }),
            false,
            1.5,
        ),
        (
            "Yeniden örnekle, Ortalama 2×",
            vec![a.clone()],
            json!({ "kind": "resample", "cell": 4.0, "method": "mean" }),
            false,
            1.5,
        ),
        (
            "Yeniden örnekle, Çift doğrusal ½×",
            vec![a.clone()],
            json!({ "kind": "resample", "cell": 1.0, "method": "bilinear" }),
            false,
            1.5,
        ),
        (
            "Bölgesel istatistik, 10 000 parsel",
            vec![a.clone()],
            json!({ "kind": "zonalStatistics", "band": 1, "stat": "mean" }),
            true,
            1.5,
        ),
        (
            "Histogram",
            vec![a.clone()],
            json!({ "kind": "histogram", "band": 1, "bins": 100 }),
            false,
            0.6,
        ),
        (
            "Komşuluk 5 × 5 Ortalama",
            vec![a.clone()],
            json!({ "kind": "focalStatistics", "band": 1, "shape": "rect", "width": 5, "height": 5, "stat": "mean", "ignore": true }),
            false,
            1.0,
        ),
        (
            "Komşuluk Daire 15 En büyük",
            vec![a.clone()],
            json!({ "kind": "focalStatistics", "band": 1, "shape": "circle", "radius": 15, "stat": "max", "ignore": true }),
            false,
            2.0,
        ),
        (
            "Komşuluk 5 × 5 Ortanca",
            vec![a.clone()],
            json!({ "kind": "focalStatistics", "band": 1, "shape": "rect", "width": 5, "height": 5, "stat": "median", "ignore": true }),
            false,
            3.0,
        ),
        (
            "Hücre istatistiği, beş raster",
            stack,
            json!({ "kind": "cellStatistics", "band": 1, "stat": "mean", "ignore": true }),
            false,
            1.5,
        ),
    ];
    for (name, files, tool, with_shapes, budget) in &jobs {
        let shapes: &[Shape] = if *with_shapes { &shapes } else { &[] };
        let (p50, worst) = time(3, &|| {
            run(files, tool, shapes, threads);
        });
        println!("{name:<36} p50 {p50:6.3} s  en yavaş {worst:6.3} s  bütçe {budget} s");
    }
}
