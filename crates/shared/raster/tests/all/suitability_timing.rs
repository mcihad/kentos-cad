//! The budgets of docs/adr/0237 §11, measured in a release build:
//!
//! ```text
//! cargo test --release -p kentos-raster --test all suitability_timing -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Four 4096² 32-bit criteria of 5 m cells (a slope, a distance, classes and
//! a membership, with holes without a value) as tiled, Deflate GeoTIFFs in
//! memory, each tool run whole as a host runs it (the blocks handed over and
//! decoded, the cells worked out, the result written); İkili karşılaştırma's
//! weights of 15 criteria alone; ROC over every cell with 10 000 presence
//! cells (an area).

use std::time::Instant;

use kentos_contracts::RasterSample;
use kentos_formats::raster::TILE;
use kentos_formats::raster::samples::Samples;
use kentos_formats::raster::source::open_bytes;
use kentos_formats::raster::write::Geo;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::vec2::Vec2;
use kentos_raster::inputs::Input;
use kentos_raster::job::READER_BUDGET;
use kentos_raster::ops::{OpsFinished, OpsJob, OpsSpec};
use kentos_raster::out::{Out, OutSpec, Rows};
use kentos_raster::par;
use kentos_raster::suitability::pairwise::pairwise;
use serde_json::{Value, json};

const X0: f64 = 500_000.0;
const Y1: f64 = 4_420_000.0;
const CELL: f64 = 5.0;
const AFFINE: [f64; 6] = [X0, CELL, 0.0, Y1, 0.0, -CELL];

fn hash(i: u32, j: u32) -> u32 {
    let mut h = i.wrapping_mul(0x9e37_79b9) ^ j.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^ (h >> 12)
}

/// A float32 raster of `value(i, j)` (NaN: none) as a GeoTIFF.
fn raster(n: u32, value: &dyn Fn(u32, u32) -> f64) -> Vec<u8> {
    let (mut out, header) = Out::new(
        OutSpec {
            width: n,
            height: n,
            bands: 1,
            sample: RasterSample::F32,
            alpha: false,
            nodata: Some(f64::NAN),
            geo: Geo {
                affine: AFFINE,
                epsg: None,
                geographic: false,
            },
        },
        par::threads(),
    )
    .unwrap();
    let mut file = header;
    let mut j0 = 0;
    while j0 < n {
        let rows = TILE.min(n - j0);
        let mut s = Samples::filled(RasterSample::F32, (rows * n) as usize, 0.0);
        for j in 0..rows {
            for i in 0..n {
                s.set((j * n + i) as usize, value(i, j0 + j));
            }
        }
        file.extend(out.push(Rows::Any(&s), rows).unwrap());
        j0 += rows;
    }
    let (tail, header) = out.finish().unwrap();
    file.extend(tail);
    file[..header.len()].copy_from_slice(&header);
    file
}

/// About 2 % of the cells without a value.
fn hole(i: u32, j: u32) -> bool {
    let (x, y) = (f64::from(i), f64::from(j));
    libm::sin(x / 97.0) * libm::cos(y / 131.0) + 0.3 * libm::sin((x + y) / 41.0) > 1.15
}

fn slope(i: u32, j: u32) -> f64 {
    if hole(i, j) {
        return f64::NAN;
    }
    let (x, y) = (f64::from(i), f64::from(j));
    (40.0 * (libm::sin(x / 230.0) * libm::cos(y / 310.0)).abs()
        + f64::from(hash(i, j) % 100) / 20.0)
        .max(0.0)
}

fn distance(i: u32, j: u32) -> f64 {
    let (x, y) = (f64::from(i), f64::from(j));
    5.0 * (libm::hypot(x - 1800.0, y - 2300.0) % 900.0)
}

fn classes(i: u32, j: u32) -> f64 {
    f64::from(1 + (i / 37 + j / 53 + hash(i / 37, j / 53)) % 5)
}

fn membership(i: u32, j: u32) -> f64 {
    f64::from(hash(j, i) % 1001) / 1000.0
}

/// An ops job over `files` named `names` with `shapes`; what it made.
fn run(files: &[&[u8]], names: &[&str], tool: &Value, shapes: &[Shape], threads: usize) -> usize {
    let spec: OpsSpec = serde_json::from_value(json!({
        "tool": tool,
        "inputs": names.iter().map(|n| json!({ "affine": AFFINE, "name": n })).collect::<Vec<_>>(),
    }))
    .unwrap();
    let inputs = files
        .iter()
        .map(|f| Input::new(open_bytes(f, None, READER_BUDGET).unwrap(), AFFINE, None).unwrap())
        .collect();
    let (mut job, header) = OpsJob::new(inputs, &spec, shapes.to_vec(), threads).unwrap();
    let mut made = header.len();
    while !job.done() {
        let blocks = job
            .needs()
            .into_iter()
            .map(|(k, n)| {
                let f = files[k as usize];
                (
                    k,
                    n,
                    f[n.offset as usize..(n.offset + n.len) as usize].to_vec(),
                )
            })
            .collect();
        job.put_all(blocks).unwrap();
        made += job.step().unwrap().len();
    }
    match job.finish().unwrap() {
        OpsFinished::Raster { tail, .. } => made += tail.len(),
        OpsFinished::Roc(r) => made = r.rows.len(),
        _ => {}
    }
    made
}

fn time(runs: usize, f: &dyn Fn() -> usize) -> (f64, f64, usize) {
    let mut n = 0;
    let mut t: Vec<f64> = (0..runs)
        .map(|_| {
            let s = Instant::now();
            n = f();
            s.elapsed().as_secs_f64()
        })
        .collect();
    t.sort_by(f64::total_cmp);
    (t[t.len() / 2], t[t.len() - 1], n)
}

fn report(name: &str, (p50, worst, n): (f64, f64, usize), budget: f64) {
    println!("{name:<52} p50 {p50:8.4} s  en yavaş {worst:8.4} s  ({n})  bütçe {budget} s");
}

/// A job: its name, its files and names, the tool, the objects and the budget (s).
type Job<'a> = (&'a str, Vec<&'a [u8]>, Vec<&'a str>, Value, Vec<Shape>, f64);

#[test]
#[ignore = "timing, release build"]
fn suitability_timing() {
    let threads = par::threads();
    println!("threads {threads}");
    let n = 4096;
    let s = raster(n, &slope);
    let d = raster(n, &distance);
    let c = raster(n, &classes);
    let m = raster(n, &membership);
    let names = ["Eğim", "Uzaklık", "Örtü", "Üyelik"];
    // 10 000 presence cells: a 100 × 100 cell square on the slope.
    let square = |i0: f64, j0: f64, k: f64| {
        let (x0, y0) = (X0 + CELL * i0 + 0.1, Y1 - CELL * j0 - 0.1);
        let (x1, y1) = (x0 + CELL * k - 0.2, y0 - CELL * k + 0.2);
        Shape::Polygon {
            pts: vec![
                Vec2::new(x0, y1),
                Vec2::new(x1, y1),
                Vec2::new(x1, y0),
                Vec2::new(x0, y0),
            ],
            bulges: None,
            holes: None,
            parts: None,
        }
    };
    let jobs: Vec<Job> = vec![
        (
            "Bulanık üyelik, Gauss",
            vec![&s],
            vec!["Eğim"],
            json!({ "kind": "fuzzyMembership", "band": 1, "function": "gaussian", "midpoint": 10.0, "spread": 0.01, "sample": "f32" }),
            vec![],
            0.6,
        ),
        (
            "Bulanık çakıştırma, dört raster, Gamma",
            vec![&m, &m, &m, &m],
            vec!["A", "B", "C", "D"],
            json!({ "kind": "fuzzyOverlay", "band": 1, "op": "gamma", "gamma": 0.9, "sample": "f32" }),
            vec![],
            1.5,
        ),
        (
            "Ağırlıklı toplam, dört raster",
            vec![&s, &d, &c, &m],
            names.to_vec(),
            json!({ "kind": "weightedSum", "band": 1, "weights": { "Eğim": -0.4, "Uzaklık": 0.001, "Örtü": 0.5, "Üyelik": 2.0 }, "sample": "f32" }),
            vec![],
            1.5,
        ),
        (
            "Ağırlıklı çakıştırma, dört raster, sınıf tablolarıyla",
            vec![&s, &d, &c, &m],
            names.to_vec(),
            json!({ "kind": "weightedOverlay", "band": 1, "low": 1.0, "high": 9.0,
                    "influence": { "Eğim": 40.0, "Uzaklık": 25.0, "Örtü": 20.0, "Üyelik": 15.0 },
                    "classes": { "Eğim": "* 5 9; 5 15 6; 15 30 3; 30 * kısıt", "Uzaklık": "* 500 9; 500 1500 5; 1500 * 1",
                                 "Örtü": "1 9; 2 7; 3 5; 4 3; 5 kısıt", "Üyelik": "* 0,25 1; 0,25 0,5 4; 0,5 0,75 7; 0,75 * 9" },
                    "bounds": "upperClosed" }),
            vec![],
            1.5,
        ),
        (
            "ROC, bütün hücreler, 10 000 varlık hücresi",
            vec![&s],
            vec!["Eğim"],
            json!({ "kind": "roc", "band": 1, "first": 1, "absence": false, "higher": true }),
            vec![square(1000.0, 1200.0, 100.0)],
            1.5,
        ),
    ];
    for (name, files, names, tool, shapes, budget) in &jobs {
        report(
            name,
            time(3, &|| run(files, names, tool, shapes, threads)),
            *budget,
        );
    }
    // İkili karşılaştırma's weights of 15 criteria, a thousand times.
    let criteria: Vec<String> = (1..=15).map(|k| format!("Ö{k}")).collect();
    let pairs: Vec<(String, String, f64)> = (0..15)
        .flat_map(|i| (i + 1..15).map(move |j| (i, j)))
        .map(|(i, j)| {
            let v = [-9, -7, -5, -3, -2, 1, 2, 3, 5, 7, 9][(i * 5 + j * 3) % 11];
            (criteria[i].clone(), criteria[j].clone(), f64::from(v))
        })
        .collect();
    let t = Instant::now();
    let mut steps = 0;
    for _ in 0..1000 {
        steps += pairwise(&criteria, &pairs).unwrap().steps;
    }
    println!(
        "{:<52} {:8.4} ms bir kez ({} adım)  bütçe 1 ms",
        "İkili karşılaştırma, 15 ölçüt, yalnız tablo",
        t.elapsed().as_secs_f64(),
        steps / 1000
    );
    // One thread.
    let (files, names, tool) = (&jobs[3].1, &jobs[3].2, &jobs[3].3);
    report(
        "Ağırlıklı çakıştırma, 1 iş parçacığı",
        time(3, &|| run(files, names, tool, &[], 1)),
        3.0,
    );
}
