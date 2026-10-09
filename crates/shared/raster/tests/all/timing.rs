//! The budgets of docs/adr/0231 §11, measured in a release build:
//!
//! ```text
//! cargo test --release -p kentos-raster --test all timing -- --ignored --nocapture --test-threads=1
//! ```
//!
//! A DEM of n × n 32-bit cells (a tiled, Deflate GeoTIFF the result writer
//! itself writes) is worked out whole, its blocks handed over from memory:
//! reading, decoding, the kernel, the levels and the coding all counted.

use std::time::Instant;

use kentos_contracts::RasterSample;
use kentos_formats::raster::TILE;
use kentos_formats::raster::write::Geo;
use kentos_raster::job::Spec;
use kentos_raster::out::{Out, OutSpec, Rows};
use kentos_raster::par;
use serde_json::json;

use crate::host::{Ran, run};

/// A hilly DEM of `n` × `n` cells, 5 m, as a GeoTIFF's bytes.
fn dem(n: u32) -> Vec<u8> {
    let (mut out, header) = Out::new(
        OutSpec {
            width: n,
            height: n,
            bands: 1,
            sample: RasterSample::F32,
            alpha: false,
            nodata: Some(f64::NAN),
            geo: Geo {
                affine: [500_000.0, 5.0, 0.0, 4_420_000.0, 0.0, -5.0],
                epsg: Some(5254),
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
        let mut v = Vec::with_capacity((rows * n) as usize);
        for j in j0..j0 + rows {
            for i in 0..n {
                let (x, y) = (f64::from(i), f64::from(j));
                v.push(
                    (400.0
                        + 120.0 * libm::sin(x / 230.0) * libm::cos(y / 310.0)
                        + 0.05 * x
                        + 3.0 * libm::sin(x / 7.0) * libm::cos(y / 9.0)) as f32,
                );
            }
        }
        file.extend(out.push(Rows::F32(&v), rows).unwrap());
        j0 += rows;
    }
    let (tail, header) = out.finish().unwrap();
    file.extend(tail);
    file[..header.len()].copy_from_slice(&header);
    file
}

fn spec(tool: serde_json::Value) -> Spec {
    serde_json::from_value(json!({
        "tool": tool,
        "band": 1,
        "affine": [500_000.0, 5.0, 0.0, 4_420_000.0, 0.0, -5.0],
        "epsg": 5254,
        "system": {"kind": "tm", "datum": "TUREF", "centralMeridian": 30, "scaleFactor": 1, "falseEasting": 500000, "falseNorthing": 0}
    }))
    .unwrap()
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
fn timing() {
    let threads = par::threads();
    println!("threads {threads}");
    let big = dem(4096);
    println!("4096² DEM: {:.1} MB", big.len() as f64 / 1e6);
    for (name, tool, budget) in [
        (
            "Eğim",
            json!({"kind": "slope", "method": "horn", "unit": "degrees", "zFactor": 1.0}),
            1.5,
        ),
        (
            "Bakı",
            json!({"kind": "aspect", "method": "horn", "zFactor": 1.0}),
            1.5,
        ),
        (
            "Gölgeli kabartma",
            json!({"kind": "hillshade", "azimuth": 315.0, "altitude": 45.0, "zFactor": 1.0}),
            1.5,
        ),
        (
            "Eğrilik (profil)",
            json!({"kind": "curvature", "curvature": "profile", "zFactor": 1.0}),
            1.5,
        ),
        (
            "TRI",
            json!({"kind": "ruggedness", "index": "triRiley"}),
            1.5,
        ),
        (
            "Renkli kabartma (en küçük–en büyük)",
            json!({"kind": "colorRelief", "ramp": "Arazi", "interp": "linear"}),
            1.5,
        ),
        (
            "Eş yükselti (5 m, ~100 düzey)",
            json!({"kind": "contours", "interval": 5.0, "base": 0.0, "indexEvery": 5}),
            1.5,
        ),
    ] {
        let s = spec(tool);
        let (p50, worst) = time(3, &|| {
            let r = run(&big, &s, threads).unwrap();
            if let Ran::Lines(l) = r {
                assert!(!l.is_empty());
            }
        });
        println!("{name:<38} 4096²  p50 {p50:6.3} s  en yavaş {worst:6.3} s  bütçe {budget} s");
    }
    let mid = dem(2048);
    let s = spec(
        json!({"kind": "insolation", "firstDay": 1, "lastDay": 365, "dayStep": 14, "hourStep": 0.5, "transmissivity": 0.5, "zFactor": 1.0}),
    );
    let (p50, worst) = time(3, &|| {
        run(&mid, &s, threads).unwrap();
    });
    println!(
        "{:<38} 2048²  p50 {p50:6.3} s  en yavaş {worst:6.3} s  bütçe 3 s",
        "Güneşlenme (yıl, 14 gün, 0,5 saat)"
    );
    let s = spec(json!({"kind": "slope", "method": "horn", "unit": "degrees", "zFactor": 1.0}));
    let (p50, worst) = time(3, &|| {
        run(&big, &s, 1).unwrap();
    });
    println!(
        "{:<38} 4096²  p50 {p50:6.3} s  en yavaş {worst:6.3} s  (tek iş parçacığı: web'in işçisi gibi)",
        "Eğim"
    );
}
