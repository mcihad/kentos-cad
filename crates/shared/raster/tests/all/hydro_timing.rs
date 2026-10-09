//! The budgets of docs/adr/0235 §12, measured in a release build:
//!
//! ```text
//! cargo test --release -p kentos-raster --test all hydro_timing -- --ignored --nocapture --test-threads=1
//! ```
//!
//! A 4096² 32-bit DEM of 5 m cells (hills, a trend and a little noise: pits
//! and flats as a real model has them) as a tiled, Deflate GeoTIFF in memory,
//! each tool run whole as a host runs it (the blocks handed over and decoded,
//! the work, the result written or the objects made). `KENTOS_PHASES=1` also
//! times the stages of Akış birikimi.

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
use serde_json::{Value, json};

const X0: f64 = 500_000.0;
const Y1: f64 = 4_420_000.0;
const CELL: f64 = 5.0;

fn hash(i: u32, j: u32) -> u32 {
    let mut h = i.wrapping_mul(0x9e37_79b9) ^ j.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^ (h >> 12)
}

/// The DEM as a GeoTIFF.
fn dem(n: u32) -> Vec<u8> {
    let affine = [X0, CELL, 0.0, Y1, 0.0, -CELL];
    let (mut out, header) = Out::new(
        OutSpec {
            width: n,
            height: n,
            bands: 1,
            sample: RasterSample::F32,
            alpha: false,
            nodata: Some(f64::NAN),
            geo: Geo {
                affine,
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
                let (x, y) = (f64::from(i), f64::from(j0 + j));
                let noise = f64::from(hash(i, j0 + j) % 1000) / 1000.0;
                let z = 400.0
                    + 120.0 * libm::sin(x / 230.0) * libm::cos(y / 310.0)
                    + 0.05 * x
                    + 3.0 * libm::sin(x / 7.0) * libm::cos(y / 9.0)
                    + 0.8 * noise;
                s.set((j * n + i) as usize, z);
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

/// A hydrology job over `file` with `shapes`; what it made (bytes written or objects), and the stages' times.
fn run(file: &[u8], tool: &Value, shapes: &[Shape], threads: usize) -> (usize, Vec<(String, f64)>) {
    let affine = [X0, CELL, 0.0, Y1, 0.0, -CELL];
    let spec: OpsSpec = serde_json::from_value(
        json!({ "tool": tool, "inputs": [{ "affine": affine, "name": "A" }] }),
    )
    .unwrap();
    let input = Input::new(open_bytes(file, None, READER_BUDGET).unwrap(), affine, None).unwrap();
    let (mut job, header) = OpsJob::new(vec![input], &spec, shapes.to_vec(), threads).unwrap();
    let mut made = header.len();
    let mut stages: Vec<(String, f64)> = Vec::new();
    let mut at = Instant::now();
    let mut share = 0.0;
    while !job.done() {
        let blocks = job
            .needs()
            .into_iter()
            .map(|(k, n)| {
                (
                    k,
                    n,
                    file[n.offset as usize..(n.offset + n.len) as usize].to_vec(),
                )
            })
            .collect();
        job.put_all(blocks).unwrap();
        made += job.step().unwrap().len();
        let now = job.share();
        // A stage ends where the share jumps over a stage's start (0.2 read, 0.55 fill, 0.6 directions, 0.9 sums).
        for (edge, name) in [
            (0.2, "okuma"),
            (0.55, "doldurma"),
            (0.6, "yönler"),
            (0.9, "birikim"),
        ] {
            if share < edge && now >= edge {
                stages.push((name.into(), at.elapsed().as_secs_f64()));
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
    stages.push(("sonuç".into(), at.elapsed().as_secs_f64()));
    (made, stages)
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
    println!("{name:<40} p50 {p50:6.3} s  en yavaş {worst:6.3} s  ({n})  bütçe {budget} s");
}

#[test]
#[ignore = "timing, release build"]
fn hydro_timing() {
    let threads = par::threads();
    println!("threads {threads}");
    let n = 4096;
    let file = dem(n);
    let mut jobs: Vec<(&str, Value, f64)> = vec![
        (
            "Çukur doldur",
            json!({ "kind": "fill", "band": 1, "slope": 0.0, "result": "filled" }),
            1.5,
        ),
        (
            "Akış yönü",
            json!({ "kind": "flowDirection", "band": 1, "fill": true, "coding": "esri" }),
            2.0,
        ),
        (
            "Akış birikimi, D8",
            json!({ "kind": "flowAccumulation", "band": 1, "fill": true, "method": "d8", "exponent": 0.0, "unit": "cells" }),
            2.5,
        ),
        (
            "Akış birikimi, Çoklu yön",
            json!({ "kind": "flowAccumulation", "band": 1, "fill": true, "method": "mfd", "exponent": 0.0, "unit": "cells" }),
            3.5,
        ),
        (
            "Akış birikimi, D∞",
            json!({ "kind": "flowAccumulation", "band": 1, "fill": true, "method": "dinf", "exponent": 0.0, "unit": "cells" }),
            3.5,
        ),
        (
            "Topografik nemlilik indisi",
            json!({ "kind": "wetness", "band": 1, "fill": true, "method": "mfd", "exponent": 0.0, "slope": 0.1 }),
            4.0,
        ),
        (
            "Havzalar, ana havzalar",
            json!({ "kind": "basins", "band": 1, "fill": true, "mode": "main", "threshold": 0.0, "least": 0.0 }),
            3.0,
        ),
        (
            "Dere ağı",
            json!({ "kind": "streams", "band": 1, "fill": true, "threshold": 0.0, "simplify": 1.0 }),
            3.0,
        ),
    ];
    let points: Vec<Shape> = (0..10)
        .map(|k| Shape::Point {
            p: Vec2::new(
                X0 + 2000.0 * f64::from(k) + 777.0,
                Y1 - 1500.0 * f64::from(k % 7) - 999.0,
            ),
            z: None,
            parts: None,
        })
        .collect();
    for (name, tool, budget) in jobs.drain(..) {
        report(name, time(3, &|| run(&file, &tool, &[], threads).0), budget);
    }
    let tool = json!({ "kind": "watershed", "band": 1, "fill": true, "snap": 50.0 });
    report(
        "Noktadan havza, 10 nokta",
        time(3, &|| run(&file, &tool, &points, threads).0),
        2.5,
    );
    if std::env::var("KENTOS_PHASES").is_ok() {
        for tool in [
            json!({ "kind": "flowAccumulation", "band": 1, "fill": true, "method": "d8", "exponent": 0.0, "unit": "cells" }),
            json!({ "kind": "flowAccumulation", "band": 1, "fill": true, "method": "mfd", "exponent": 0.0, "unit": "cells" }),
        ] {
            let (_, stages) = run(&file, &tool, &[], threads);
            println!("aşamalar: {stages:?}");
        }
    }
}
