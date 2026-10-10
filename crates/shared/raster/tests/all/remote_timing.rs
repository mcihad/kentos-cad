//! The budgets of docs/adr/0242 §12, measured in a release build:
//!
//! ```text
//! cargo test --release -p kentos-raster --test all remote_timing -- --ignored --nocapture --test-threads=1
//! ```
//!
//! A 4096² four-band 16-bit image of 5 m cells (six kinds of cover in
//! blocks, each band its own signature and noise; the corner without a
//! value), the same two years on, its four bands apart, its classes, and a
//! 2048² multispectral image with a 4096² panchromatic one, all tiled,
//! Deflate GeoTIFFs in memory; each tool run whole as a host runs it (the
//! blocks handed over and decoded, the cells worked out, the result written).

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
const AFFINE: [f64; 6] = [X0, CELL, 0.0, Y1, 0.0, -CELL];
const COARSE: [f64; 6] = [X0, 2.0 * CELL, 0.0, Y1, 0.0, -2.0 * CELL];

fn hash(i: u32, j: u32, b: u32) -> u32 {
    let mut h =
        i.wrapping_mul(0x9e37_79b9) ^ j.wrapping_mul(0x85eb_ca6b) ^ b.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^ (h >> 12)
}

/// A 16-bit raster of `bands` bands of `value(i, j, b)` (0: none) on `affine` as a GeoTIFF.
fn raster(n: u32, bands: u32, affine: [f64; 6], value: &dyn Fn(u32, u32, u32) -> f64) -> Vec<u8> {
    let (mut out, header) = Out::new(
        OutSpec {
            width: n,
            height: n,
            bands,
            sample: RasterSample::U16,
            alpha: false,
            nodata: Some(0.0),
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
        let mut s = Samples::filled(RasterSample::U16, (rows * n * bands) as usize, 0.0);
        for j in 0..rows {
            for i in 0..n {
                for b in 0..bands {
                    s.set(((j * n + i) * bands + b) as usize, value(i, j0 + j, b));
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

/// The six covers' signatures: blue, green, red, near infrared.
const SIGNATURES: [[f64; 4]; 6] = [
    [600.0, 520.0, 330.0, 180.0],
    [300.0, 560.0, 380.0, 3400.0],
    [520.0, 820.0, 760.0, 2700.0],
    [980.0, 1250.0, 1600.0, 2150.0],
    [420.0, 900.0, 560.0, 4100.0],
    [1250.0, 1330.0, 1420.0, 1850.0],
];

fn cover(i: u32, j: u32) -> u32 {
    (i / 256 + 3 * (j / 256) + hash(i / 256, j / 256, 9) % 2) % 6
}

/// The corner without a value.
fn empty(i: u32, j: u32) -> bool {
    i < 40 && j < 40
}

fn image(i: u32, j: u32, b: u32) -> f64 {
    if empty(i, j) {
        return 0.0;
    }
    let noise = f64::from(hash(i, j, b) % 201) - 100.0;
    (SIGNATURES[cover(i, j) as usize][b as usize] + noise).max(1.0)
}

fn later(i: u32, j: u32, b: u32) -> f64 {
    let v = image(i, j, b);
    if v == 0.0 {
        return 0.0;
    }
    // A felled block and a sensor 2 % brighter.
    if (1024..1536).contains(&i) && (2048..2560).contains(&j) {
        return SIGNATURES[3][b as usize];
    }
    v * 1.02
}

/// An area over the cells `i0..i0 + k`, `j0..j0 + k` (a tenth of a cell inside).
fn square(i0: u32, j0: u32, k: u32) -> Shape {
    let (x0, y0) = (
        X0 + CELL * f64::from(i0) + 0.5,
        Y1 - CELL * f64::from(j0) - 0.5,
    );
    let (x1, y1) = (
        x0 + CELL * f64::from(k) - 1.0,
        y0 - CELL * f64::from(k) + 1.0,
    );
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
}

/// An ops job over `files` (each on its affine) with `shapes`; what it made.
fn run(files: &[(&[u8], [f64; 6])], tool: &Value, shapes: &[Shape], threads: usize) -> usize {
    let spec: OpsSpec = serde_json::from_value(json!({
        "tool": tool,
        "inputs": files.iter().enumerate().map(|(k, (_, a))| json!({ "affine": a, "name": format!("R{k}") })).collect::<Vec<_>>(),
    }))
    .unwrap();
    let inputs = files
        .iter()
        .map(|(f, a)| Input::new(open_bytes(f, None, READER_BUDGET).unwrap(), *a, None).unwrap())
        .collect();
    let (mut job, header) = OpsJob::new(inputs, &spec, shapes.to_vec(), threads).unwrap();
    let mut made = header.len();
    while !job.done() {
        let blocks = job
            .needs()
            .into_iter()
            .map(|(k, n)| {
                let f = files[k as usize].0;
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
    let notes = job.notes().clone();
    match job.finish().unwrap() {
        OpsFinished::Raster { tail, .. } => made += tail.len(),
        OpsFinished::Report => made = notes.remote.table.map_or(0, |t| t.rows.len()),
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

/// A job: its name, its files on their affines, the tool, the objects and the budget (s).
type Job<'a> = (&'a str, Vec<(&'a [u8], [f64; 6])>, Value, Vec<Shape>, f64);

#[test]
#[ignore = "timing, release build"]
fn remote_timing() {
    let threads = par::threads();
    println!("threads {threads}");
    let n = 4096;
    let img = raster(n, 4, AFFINE, &image);
    let after = raster(n, 4, AFFINE, &later);
    let bands: Vec<Vec<u8>> = (0..4)
        .map(|b| raster(n, 1, AFFINE, &|i, j, _| image(i, j, b)))
        .collect();
    let classes = raster(n, 1, AFFINE, &|i, j, _| {
        if empty(i, j) {
            0.0
        } else {
            f64::from(1 + cover(i, j))
        }
    });
    let ms = raster(n / 2, 4, COARSE, &|i, j, b| image(2 * i, 2 * j, b));
    let pan = raster(n, 1, AFFINE, &|i, j, _| {
        if empty(i, j) {
            0.0
        } else {
            (0..3).map(|b| image(i, j, b)).sum::<f64>() / 3.0
        }
    });
    // Six training squares of 64 × 64 cells, one a cover; 10 000 reference points.
    let mut training = Vec::new();
    let mut texts = Vec::new();
    for c in 0..6u32 {
        let (bi, bj) = (0..16u32)
            .flat_map(|bj| (0..16u32).map(move |bi| (bi, bj)))
            .find(|&(bi, bj)| cover(bi * 256, bj * 256) == c && bi * 256 >= 64)
            .unwrap();
        training.push(square(bi * 256 + 96, bj * 256 + 96, 64));
        texts.push(Some(format!("Örtü {}", c + 1)));
    }
    let mut points = Vec::new();
    let mut refs = Vec::new();
    for k in 0..10_000u32 {
        let (i, j) = (hash(k, 1, 2) % n, hash(k, 3, 4) % n);
        points.push(Shape::Point {
            p: Vec2::new(
                X0 + CELL * (f64::from(i) + 0.5),
                Y1 - CELL * (f64::from(j) + 0.5),
            ),
            z: None,
            parts: None,
        });
        refs.push(Some((1 + cover(i, j)).to_string()));
    }
    let a = AFFINE;
    let jobs: Vec<Job> = vec![
        (
            "Bant birleştir, dört tek bantlı",
            bands.iter().map(|b| (b.as_slice(), a)).collect(),
            json!({ "kind": "composite", "sampling": "nearest" }),
            vec![],
            1.5,
        ),
        (
            "NDVI",
            vec![(&img, a)],
            json!({ "kind": "index", "index": "ndvi", "bands": { "blue": 1, "green": 2, "red": 3, "nir": 4, "swir": 1, "a": 4, "b": 3 },
                    "scale": 1.0, "offset": 0.0, "saviL": 0.5, "g": 2.5, "c1": 6.0, "c2": 7.5, "eviL": 1.0 }),
            vec![],
            1.0,
        ),
        (
            "Denetimli, en büyük olabilirlik, 6 sınıf",
            vec![(&img, a)],
            json!({ "kind": "supervised", "method": "likelihood", "texts": texts }),
            training.clone(),
            2.0,
        ),
        (
            "Denetimsiz, 8 küme, 20 yineleme",
            vec![(&img, a)],
            json!({ "kind": "unsupervised", "clusters": 8, "iterations": 20 }),
            vec![],
            2.0,
        ),
        (
            "Doğruluk analizi, 10 000 nokta",
            vec![(&classes, a)],
            json!({ "kind": "accuracy", "band": 1, "reference": refs }),
            points.clone(),
            0.5,
        ),
        (
            "Değişim tespiti, fark",
            vec![(&img, a), (&after, a)],
            json!({ "kind": "change", "band": 4, "method": "difference" }),
            vec![],
            1.0,
        ),
        (
            "Görüntü birleştirme, Brovey, 2048² → 4096²",
            vec![(&ms, COARSE), (&pan, a)],
            json!({ "kind": "pansharpen", "method": "brovey", "sampling": "cubic" }),
            vec![],
            2.0,
        ),
    ];
    for (name, files, tool, shapes, budget) in &jobs {
        report(
            name,
            time(3, &|| run(files, tool, shapes, threads)),
            *budget,
        );
    }
    // Bantlara ayır: a run a band.
    report(
        "Bantlara ayır, dört bant",
        time(3, &|| {
            (1..=4)
                .map(|b| {
                    run(
                        &[(&img, a)],
                        &json!({ "kind": "band", "band": b }),
                        &[],
                        threads,
                    )
                })
                .sum()
        }),
        2.0,
    );
}
