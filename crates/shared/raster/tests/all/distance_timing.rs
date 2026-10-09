//! The budgets of docs/adr/0236 §8, measured in a release build:
//!
//! ```text
//! cargo test --release -p kentos-raster --test all distance_timing -- --ignored --nocapture --test-threads=1
//! ```
//!
//! A 4096² 32-bit cost raster of 5 m cells (a terrain's cost with lakes
//! without a value), its sources' raster and the DEM of hydrology's timing as
//! tiled, Deflate GeoTIFFs in memory, each tool run whole as a host runs it
//! (the blocks handed over and decoded, the work, the result written or the
//! paths made). Uzaklık yüzeyi from objects runs as the point job.

use std::time::Instant;

use kentos_contracts::RasterSample;
use kentos_formats::raster::TILE;
use kentos_formats::raster::samples::Samples;
use kentos_formats::raster::source::open_bytes;
use kentos_formats::raster::write::Geo;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::vec2::Vec2;
use kentos_raster::from_points::{PointInput, PointJob, PointSpec};
use kentos_raster::inputs::Input;
use kentos_raster::job::READER_BUDGET;
use kentos_raster::ops::{OpsFinished, OpsJob, OpsSpec};
use kentos_raster::out::{Out, OutSpec, Rows};
use kentos_raster::par;
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

fn height(i: u32, j: u32) -> f64 {
    let (x, y) = (f64::from(i), f64::from(j));
    let noise = f64::from(hash(i, j) % 1000) / 1000.0;
    400.0
        + 120.0 * libm::sin(x / 230.0) * libm::cos(y / 310.0)
        + 0.05 * x
        + 3.0 * libm::sin(x / 7.0) * libm::cos(y / 9.0)
        + 0.8 * noise
}

/// The cost: dearer up the hills, lakes (about 4 %) without a value.
fn cost(i: u32, j: u32) -> f64 {
    let (x, y) = (f64::from(i), f64::from(j));
    let lake = libm::sin(x / 97.0) * libm::cos(y / 131.0) + 0.3 * libm::sin((x + y) / 41.0);
    if lake > 1.05 {
        return f64::NAN;
    }
    1.0 + 4.0 * (libm::sin(x / 230.0) * libm::cos(y / 310.0)).abs()
        + f64::from(hash(i, j) % 100) / 100.0
}

/// An ops job over `files` (the first the raster, the second the surface) with `shapes`; what it made.
fn run(files: &[&[u8]], tool: &Value, shapes: &[Shape], threads: usize) -> usize {
    run_stages(files, tool, shapes, threads).0
}

/// [`run`], and the stages' times (reading, the search, the result, writing).
fn run_stages(
    files: &[&[u8]],
    tool: &Value,
    shapes: &[Shape],
    threads: usize,
) -> (usize, Vec<(&'static str, f64)>) {
    let names = ["A", "B"];
    let spec: OpsSpec = serde_json::from_value(json!({
        "tool": tool,
        "inputs": files.iter().zip(names).map(|(_, n)| json!({ "affine": AFFINE, "name": n })).collect::<Vec<_>>(),
    }))
    .unwrap();
    let inputs = files
        .iter()
        .map(|f| Input::new(open_bytes(f, None, READER_BUDGET).unwrap(), AFFINE, None).unwrap())
        .collect();
    let (mut job, header) = OpsJob::new(inputs, &spec, shapes.to_vec(), threads).unwrap();
    let mut made = header.len();
    let mut stages = Vec::new();
    let mut at = Instant::now();
    let mut share = 0.0;
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
        let now = job.share();
        for (edge, name) in [
            (0.2, "okuma"),
            (0.25, "hazırlık"),
            (0.85, "arama"),
            (0.9, "sonuç"),
        ] {
            if share < edge && now >= edge {
                stages.push((name, at.elapsed().as_secs_f64()));
                at = Instant::now();
            }
        }
        share = now;
    }
    match job.finish().unwrap() {
        OpsFinished::Raster { tail, .. } => made += tail.len(),
        OpsFinished::Features(f) => made = f.sizes.len(),
        _ => {}
    }
    stages.push(("yazma", at.elapsed().as_secs_f64()));
    (made, stages)
}

/// A job: its name, its files (the raster, the surface), the tool, the objects and the budget (s).
type Job<'a> = (&'a str, Vec<&'a [u8]>, Value, &'a [Shape], f64);

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
    println!("{name:<48} p50 {p50:6.3} s  en yavaş {worst:6.3} s  ({n})  bütçe {budget} s");
}

fn point(x: f64, y: f64) -> Shape {
    Shape::Point {
        p: Vec2::new(x, y),
        z: None,
        parts: None,
    }
}

#[test]
#[ignore = "timing, release build"]
fn distance_timing() {
    let threads = par::threads();
    println!("threads {threads}");
    let n = 4096;
    let costs = raster(n, &cost);
    let dem = raster(n, &height);
    let wells = raster(n, &|i, j| {
        if hash(i, j).is_multiple_of(20_000) {
            f64::from(hash(j, i) % 50 + 1)
        } else {
            f64::NAN
        }
    });
    // Ten sources, a destination far from each, the two ends of a corridor.
    let sources: Vec<Shape> = (0..10)
        .map(|k| {
            point(
                X0 + 2000.0 * f64::from(k) + 777.0,
                Y1 - 1500.0 * f64::from(k % 7) - 999.0,
            )
        })
        .collect();
    let far = |k: u32| {
        point(
            X0 + 20_000.0 - 1700.0 * f64::from(k) - 333.0,
            Y1 - 19_000.0 + 2100.0 * f64::from(k),
        )
    };
    let mut path = vec![point(X0 + 777.0, Y1 - 999.0)];
    path.extend((0..5).map(far));
    let ends = vec![
        point(X0 + 777.0, Y1 - 999.0),
        point(X0 + 19_777.0, Y1 - 19_333.0),
    ];
    let cost16 = |result: &str, max: f64| json!({ "kind": "costDistance", "band": 1, "neighbours": "16", "surfaceLength": false, "slope": 0.0, "max": max, "result": result, "sample": "f32" });
    let jobs: Vec<Job> = vec![
        (
            "Uzaklık yüzeyi, rasterden (839 kaynak)",
            vec![&wells],
            json!({ "kind": "distance", "band": 1, "max": 0.0, "result": "distance" }),
            &[],
            1.0,
        ),
        (
            "Uzaklık yüzeyi, rasterden, en yakın kaynak",
            vec![&wells],
            json!({ "kind": "distance", "band": 1, "max": 0.0, "result": "allocation" }),
            &[],
            1.0,
        ),
        (
            "Birikimli maliyet, 8 komşu, 10 kaynak",
            vec![&costs],
            json!({ "kind": "costDistance", "band": 1, "neighbours": "8", "surfaceLength": false, "slope": 0.0, "max": 0.0, "result": "cost", "sample": "f32" }),
            &sources,
            4.0,
        ),
        (
            "Birikimli maliyet, 16 komşu, 10 kaynak",
            vec![&costs],
            cost16("cost", 0.0),
            &sources,
            5.0,
        ),
        (
            "Birikimli maliyet, 16 komşu, kaynak",
            vec![&costs],
            cost16("allocation", 0.0),
            &sources,
            5.5,
        ),
        (
            "Birikimli maliyet, 16 komşu, yüzey ve eğim",
            vec![&costs, &dem],
            json!({ "kind": "costDistance", "band": 1, "neighbours": "16", "surfaceLength": true, "slope": 30.0, "max": 0.0, "result": "cost", "sample": "f32" }),
            &sources,
            6.0,
        ),
        (
            "En düşük maliyetli yol, 16 komşu, 5 varış",
            vec![&costs],
            json!({ "kind": "costPath", "band": 1, "neighbours": "16", "surfaceLength": false, "slope": 0.0, "simplify": 1.0, "first": 1 }),
            &path,
            5.0,
        ),
        (
            "Maliyet koridoru, 16 komşu",
            vec![&costs],
            json!({ "kind": "costCorridor", "band": 1, "neighbours": "16", "surfaceLength": false, "slope": 0.0, "first": 1, "threshold": "percent", "value": 5.0, "sample": "f32" }),
            &ends,
            6.0,
        ),
    ];
    for (name, files, tool, shapes, budget) in &jobs {
        report(
            name,
            time(3, &|| run(files, tool, shapes, threads)),
            *budget,
        );
    }
    // Uzaklık yüzeyi from 2000 points and 100 lines on the 4096² grid.
    let mut objects: Vec<Shape> = (0..2000u32)
        .map(|k| {
            point(
                X0 + f64::from(hash(k, 1) % 20_480) + 0.37,
                Y1 - f64::from(hash(k, 2) % 20_480) - 0.41,
            )
        })
        .collect();
    objects.extend((0..100u32).map(|k| Shape::Line {
        a: Vec2::new(
            X0 + f64::from(hash(k, 3) % 20_480) + 0.13,
            Y1 - f64::from(hash(k, 4) % 20_480) - 0.29,
        ),
        b: Vec2::new(
            X0 + f64::from(hash(k, 5) % 20_480) + 0.71,
            Y1 - f64::from(hash(k, 6) % 20_480) - 0.53,
        ),
    }));
    let spec: PointSpec = serde_json::from_value(json!({
        "tool": { "kind": "distance", "max": 0.0, "result": "distance", "margin": 0.0 },
        "grid": { "affine": AFFINE, "width": n, "height": n },
    }))
    .unwrap();
    report(
        "Uzaklık yüzeyi, 2000 nokta ve 100 çizgiden",
        time(3, &|| {
            let (mut job, header) = PointJob::new(
                PointInput::Lines {
                    shapes: objects.clone(),
                    weights: None,
                },
                &spec,
                threads,
            )
            .unwrap();
            let mut made = header.len();
            while !job.done() {
                made += job.step().unwrap().len();
            }
            made + job.finish().unwrap().tail.len()
        }),
        1.5,
    );
    if std::env::var("KENTOS_PHASES").is_ok() {
        for (name, files, tool, shapes, _) in &jobs {
            let (_, stages) = run_stages(files, tool, shapes, threads);
            println!("{name}: {stages:?}");
        }
    }
    // One search alone, on one thread.
    report(
        "Birikimli maliyet, 16 komşu, 1 iş parçacığı",
        time(3, &|| run(&[&costs], &cost16("cost", 0.0), &sources, 1)),
        6.0,
    );
}
