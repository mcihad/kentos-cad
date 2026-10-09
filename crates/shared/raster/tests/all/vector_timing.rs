//! The budgets of docs/adr/0234 §11, measured in a release build:
//!
//! ```text
//! cargo test --release -p kentos-raster --test all vector_timing -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Rasters as tiled, Deflate GeoTIFFs in memory, worked out whole as a host
//! runs a job (the blocks handed over and decoded, the work, the features
//! or the result's coding all counted): 10 000 parcels burnt onto 4096²;
//! a 4096² class raster of some 50 000 regions; 4096² with about 3 % line
//! cells; a 4096² surface's points; a capture and a closing on an 8192²
//! scanned sheet; 10 000 curves given their elevations.

use std::time::Instant;

use kentos_contracts::RasterSample;
use kentos_formats::raster::TILE;
use kentos_formats::raster::samples::Samples;
use kentos_formats::raster::source::open_bytes;
use kentos_formats::raster::write::Geo;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::contour_elevations::contour_elevations;
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

/// A raster of `n` × `n` cells of 2 m whose `bands` samples `cell(i, j, out)` writes, as a GeoTIFF.
fn raster(
    n: u32,
    bands: u32,
    sample: RasterSample,
    nodata: Option<f64>,
    cell: &(dyn Fn(u32, u32, &mut [f64]) + Sync),
) -> Vec<u8> {
    let affine = [X0, 2.0, 0.0, Y1, 0.0, -2.0];
    let (mut out, header) = Out::new(
        OutSpec {
            width: n,
            height: n,
            bands,
            sample,
            alpha: false,
            nodata,
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
    let b = bands as usize;
    let mut px = vec![0.0; b];
    while j0 < n {
        let rows = TILE.min(n - j0);
        let mut s = Samples::filled(sample, (rows * n) as usize * b, 0.0);
        for j in 0..rows {
            for i in 0..n {
                cell(i, j0 + j, &mut px);
                for (k, v) in px.iter().enumerate() {
                    s.set(((j * n + i) as usize) * b + k, *v);
                }
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

fn hash(i: u32, j: u32) -> u32 {
    let mut h = i.wrapping_mul(0x9e37_79b9) ^ j.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^ (h >> 12)
}

/// A vectorizing job over `file`, its phases timed: reading the blocks, the steps, the end.
fn phases(file: &[u8], tool: &Value, threads: usize) -> (f64, f64, f64) {
    let affine = [X0, 2.0, 0.0, Y1, 0.0, -2.0];
    let spec: OpsSpec = serde_json::from_value(
        json!({ "tool": tool, "inputs": [{ "affine": affine, "name": "A" }] }),
    )
    .unwrap();
    let input = Input::new(open_bytes(file, None, READER_BUDGET).unwrap(), affine, None).unwrap();
    let (mut job, _) = OpsJob::new(vec![input], &spec, Vec::new(), threads).unwrap();
    let (mut read, mut step) = (0.0, 0.0);
    while !job.done() {
        let t = Instant::now();
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
        read += t.elapsed().as_secs_f64();
        let t = Instant::now();
        job.step().unwrap();
        step += t.elapsed().as_secs_f64();
    }
    let t = Instant::now();
    let _ = job.finish().unwrap();
    (read, step, t.elapsed().as_secs_f64())
}

/// A vectorizing job over `file` (`n` × `n`); its features' count.
fn vectorize(file: &[u8], tool: &Value, threads: usize) -> usize {
    let affine = [X0, 2.0, 0.0, Y1, 0.0, -2.0];
    let spec: OpsSpec = serde_json::from_value(
        json!({ "tool": tool, "inputs": [{ "affine": affine, "name": "A" }] }),
    )
    .unwrap();
    let input = Input::new(open_bytes(file, None, READER_BUDGET).unwrap(), affine, None).unwrap();
    let (mut job, _) = OpsJob::new(vec![input], &spec, Vec::new(), threads).unwrap();
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
        job.step().unwrap();
    }
    match job.finish().unwrap() {
        OpsFinished::Features(f) => f.len(),
        _ => panic!("features were wanted"),
    }
}

/// 10 000 parcels, 100 × 100 of them over 8192 m.
fn parcels() -> Vec<Shape> {
    let mut out = Vec::new();
    for j in 0..100 {
        for i in 0..100 {
            let (x0, y0) = (
                X0 + f64::from(i) * 81.92 + 3.0,
                Y1 - 8192.0 + f64::from(j) * 81.92 + 3.0,
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

/// The median and the slowest of `runs` runs (s).
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
    println!("{name:<44} p50 {p50:6.3} s  en yavaş {worst:6.3} s  ({n})  bütçe {budget} s");
}

#[test]
#[ignore = "timing, release build"]
fn vector_timing() {
    let threads = par::threads();
    println!("threads {threads}");
    let n = 4096;

    // Rasterleştir: 10 000 parcels with their numbers onto 4096² cells of 2 m.
    let shapes = parcels();
    let texts: Vec<Option<String>> = (0..shapes.len()).map(|k| Some(k.to_string())).collect();
    let spec: PointSpec = serde_json::from_value(json!({
        "tool": { "kind": "rasterize", "value": 1.0, "overlap": "last", "sample": "i32" },
        "grid": { "affine": [X0, 2.0, 0.0, Y1, 0.0, -2.0], "width": n, "height": n },
    }))
    .unwrap();
    report(
        "Rasterleştir, 10 000 parsel → 4096²",
        time(3, &|| {
            let input = PointInput::Lines {
                shapes: shapes.clone(),
                weights: Some(texts.clone()),
            };
            let (mut job, header) = PointJob::new(input, &spec, threads).unwrap();
            let mut bytes = header.len();
            while !job.done() {
                bytes += job.step().unwrap().len();
            }
            bytes + job.finish().unwrap().tail.len()
        }),
        1.0,
    );

    // Rasterden alan: 18-cell blocks of 50 classes (some 52 000 regions).
    let classes = raster(n, 1, RasterSample::U8, None, &|i, j, out| {
        out[0] = f64::from(hash(i / 18, j / 18) % 50);
    });
    let tool = json!({ "kind": "toPolygons", "band": 1, "connect": "four" });
    report(
        "Rasterden alan, 4096², bloklar",
        time(3, &|| vectorize(&classes, &tool, threads)),
        2.0,
    );

    // Rasterden çizgi: 41 waving lines three cells thick (about 3 % of the cells).
    let lines = raster(n, 1, RasterSample::U8, None, &|i, j, out| {
        let x = f64::from(i);
        let k = f64::from(j) / 100.0;
        let centre = (k.round() * 100.0) + 20.0 * libm::sin(x / 180.0 + k.round());
        out[0] = if (f64::from(j) - centre).abs() <= 1.0 {
            1.0
        } else {
            0.0
        };
    });
    let tool =
        json!({ "kind": "toLines", "band": 1, "select": "nonZero", "spur": 3, "simplify": 1.0 });
    report(
        "Rasterden çizgi, 4096², %3 çizgi",
        time(3, &|| vectorize(&lines, &tool, threads)),
        2.0,
    );

    // Rasterden nokta: a surface, every tenth cell.
    let surface = raster(n, 1, RasterSample::F32, Some(f64::NAN), &|i, j, out| {
        let (x, y) = (f64::from(i), f64::from(j));
        out[0] = 400.0 + 120.0 * libm::sin(x / 230.0) * libm::cos(y / 310.0) + 0.05 * x;
    });
    let tool = json!({ "kind": "toPoints", "band": 1, "mode": "step", "step": 10 });
    report(
        "Rasterden nokta, 4096², Adım 10",
        time(3, &|| vectorize(&surface, &tool, threads)),
        0.5,
    );
    let tool = json!({ "kind": "toPoints", "band": 1, "mode": "extrema", "radius": 1 });
    report(
        "Rasterden nokta, 4096², Tepeler ve çukurlar",
        time(3, &|| vectorize(&surface, &tool, threads)),
        1.0,
    );

    // An 8192² scanned sheet: white, dark parcel lines every 400 cells and a contour across it.
    let m = 8192;
    let sheet = raster(m, 3, RasterSample::U8, None, &|i, j, out| {
        let grid = i % 400 < 3 || j % 400 < 3;
        let contour =
            (f64::from(j) - 4100.0 - 900.0 * libm::sin(f64::from(i) / 1300.0)).abs() <= 1.5;
        let v = if grid || contour { 25.0 } else { 250.0 };
        out.copy_from_slice(&[v, v, v]);
    });
    let at = |i: f64, j: f64| (X0 + 2.0 * i + 1.0, Y1 - 2.0 * j - 1.0);
    let (x, y) = at(3000.0, 4100.0 + 900.0 * libm::sin(3000.0 / 1300.0));
    let tool = json!({ "kind": "captureLine", "x": x, "y": y, "tolerance": 60.0, "spur": 5, "simplify": 1.0 });
    report(
        "Çizgi yakala, 8192² pafta, ızgaraya bağlı ağ",
        time(3, &|| vectorize(&sheet, &tool, threads)),
        1.0,
    );
    if std::env::var("KENTOS_PHASES").is_ok() {
        println!(
            "okuma, adımlar, bitiş: {:?}",
            phases(&sheet, &tool, threads)
        );
    }
    // The contour brown, the grid black: the contour alone, across the whole sheet.
    let sheet2 = raster(m, 3, RasterSample::U8, None, &|i, j, out| {
        let grid = i % 400 < 3 || j % 400 < 3;
        let contour =
            (f64::from(j) - 4100.0 - 900.0 * libm::sin(f64::from(i) / 1300.0)).abs() <= 1.5;
        let v = if contour {
            [150.0, 80.0, 30.0]
        } else if grid {
            [25.0, 25.0, 25.0]
        } else {
            [250.0, 250.0, 250.0]
        };
        out.copy_from_slice(&v);
    });
    report(
        "Çizgi yakala, 8192² pafta, boydan boya eğri",
        time(3, &|| vectorize(&sheet2, &tool, threads)),
        0.3,
    );
    let (x, y) = at(1810.0, 1190.0);
    let tool = json!({ "kind": "closeArea", "x": x, "y": y, "tolerance": 60.0, "holes": "fill", "simplify": 1.0 });
    report(
        "Alan kapat, 8192² pafta, bir parsel",
        time(3, &|| vectorize(&sheet, &tool, threads)),
        0.3,
    );

    // Eğrilere kot ver: 10 000 contour-like curves of 60 vertices and a cut across them.
    let curves: Vec<Shape> = (0..10_000)
        .map(|k| {
            let base = f64::from(k) * 2.0;
            Shape::Polyline {
                pts: (0..60)
                    .map(|q| {
                        let x = f64::from(q) * 50.0;
                        Vec2::new(
                            X0 + x,
                            Y1 + base + 0.6 * libm::sin(x / 300.0 + f64::from(k) * 0.01),
                        )
                    })
                    .collect(),
                bulges: None,
                holes: None,
                parts: None,
            }
        })
        .collect();
    report(
        "Eğrilere kot ver, 10 000 eğri",
        time(3, &|| {
            contour_elevations(
                &curves,
                Vec2::new(X0 + 1234.0, Y1 - 5.0),
                Vec2::new(X0 + 1500.0, Y1 + 20_010.0),
                100.0,
                1.0,
            )
            .iter()
            .filter(|z| z.is_some())
            .count()
        }),
        0.2,
    );
}
