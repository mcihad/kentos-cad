//! Raster oturt (docs/adr/0204 §6) against fixtures/raster/v1/georef.json
//! (scripts/fixtures/raster_georef_cases.py: GDAL's GCP transformer for the
//! affine, the polynomials and the thin plate, numpy for Helmert and the
//! projective, Newton's method on GDAL's forward for the inverses; the
//! resampling worked out there and checked against gdalwarp): every
//! point's residual, m0,
//! where the probes land and where they come back from (the forward's
//! inverse: a pixel sent and brought back is where it was); and a raster
//! carried by a thin plate, nearest, sample for sample.

use std::path::PathBuf;

use kentos_formats::raster::source::{Put, open_bytes};
use kentos_formats::raster::warp::{self, CELL_TOLERANCE, Grid, STEP};
use kentos_formats::raster::{Samples, TILE};
use kentos_geometry_core::ops::georef::{Gcp, Method, solve};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/raster/v1")
}

fn fnv(bytes: &[u8]) -> String {
    let mut h: u64 = 0xCBF2_9CE4_8422_2325;
    for &b in bytes {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01B3);
    }
    format!("{h:016x}")
}

fn v2(v: &Value) -> Vec2 {
    Vec2::new(
        v[0].as_f64().unwrap_or(f64::NAN),
        v[1].as_f64().unwrap_or(f64::NAN),
    )
}

fn gcps(v: &Value) -> Vec<Gcp> {
    v.as_array()
        .expect("points")
        .iter()
        .map(|p| Gcp {
            pixel: v2(&p["pixel"]),
            target: v2(&p["target"]),
            used: p["used"].as_bool().unwrap_or(true),
        })
        .collect()
}

fn file() -> Value {
    serde_json::from_str(&std::fs::read_to_string(dir().join("georef.json")).expect("georef.json"))
        .expect("JSON")
}

#[test]
fn every_transform_is_what_gdal_fits() {
    let file = file();
    assert_eq!(file["format"], "kentos.raster-georef-cases");
    let probes: Vec<Vec2> = file["probes"]
        .as_array()
        .expect("probes")
        .iter()
        .map(v2)
        .collect();
    let mut off = Vec::new();
    for set in file["sets"].as_array().expect("sets") {
        let points = gcps(&set["points"]);
        for s in set["solutions"].as_array().expect("solutions") {
            let name = format!("{} {}", set["name"], s["method"]);
            let method = Method::from_name(s["method"].as_str().unwrap_or("")).expect("a method");
            let g = match solve(&points, method) {
                Ok(g) => g,
                Err(e) => {
                    off.push(format!("{name}: {e:?}"));
                    continue;
                }
            };
            for (i, (got, want)) in g
                .residuals
                .iter()
                .zip(s["residuals"].as_array().expect("residuals"))
                .enumerate()
            {
                for k in 0..2 {
                    let w = want[k].as_f64().unwrap_or(f64::NAN);
                    if (got[k] - w).abs() > 1e-6 {
                        off.push(format!("{name}: {i}. noktanın artığı {} ≠ {w}", got[k]));
                    }
                }
            }
            match (g.m0, s["m0"].as_f64()) {
                (Some(a), Some(b)) if (a - b).abs() <= 1e-9 * b.abs().max(1.0) => {}
                (None, None) => {}
                (a, b) => off.push(format!("{name}: m0 {a:?} ≠ {b:?}")),
            }
            for (i, p) in probes.iter().enumerate() {
                let fw = g.forward(*p).expect("forward");
                let want = v2(&s["forward"][i]);
                if (fw.x - want.x).abs() > 1e-6 || (fw.y - want.y).abs() > 1e-6 {
                    off.push(format!("{name}: {i}. yoklama {fw:?} ≠ {want:?}"));
                }
                let back = g.inverse(want).expect("inverse");
                let want_back = v2(&s["back"][i]);
                if (back.x - want_back.x).abs() > 1e-6 || (back.y - want_back.y).abs() > 1e-6 {
                    off.push(format!(
                        "{name}: {i}. geri yoklama {back:?} ≠ {want_back:?}"
                    ));
                }
            }
        }
    }
    assert!(off.is_empty(), "{} fark:\n{}", off.len(), off.join("\n"));
}

#[test]
fn a_raster_is_resampled_as_gdalwarp_does() {
    let file = file();
    let w = &file["warp"];
    let bytes = std::fs::read(
        dir()
            .join("files")
            .join(w["source"].as_str().expect("source")),
    )
    .expect("source");
    let mut reader = open_bytes(&bytes, None, 256 << 20).expect("reads");
    let (sw, sh) = (reader.info.width, reader.info.height);
    for need in reader.needs(0, 0, 0, sw, sh) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        assert_eq!(
            reader.put_block(&need, &bytes[a..b]).expect("a block"),
            Put::Done
        );
    }
    let method = Method::from_name(w["method"].as_str().unwrap_or("")).expect("a method");
    let g = solve(&gcps(&w["points"]), method).expect("solved");
    let border: Vec<Vec2> = w["border"]
        .as_array()
        .expect("border")
        .iter()
        .map(|p| g.forward(v2(p)).expect("forward"))
        .collect();
    let grid = Grid::covering(&border, w["pixel"].as_f64().expect("pixel")).expect("a grid");
    let want_grid = &w["grid"];
    let affine: Vec<f64> = want_grid["affine"]
        .as_array()
        .expect("affine")
        .iter()
        .filter_map(Value::as_f64)
        .collect();
    assert_eq!(
        (
            grid.affine.to_vec(),
            u64::from(grid.width),
            u64::from(grid.height)
        ),
        (
            affine,
            want_grid["width"].as_u64().unwrap_or(0),
            want_grid["height"].as_u64().unwrap_or(0)
        )
    );
    let (ow, oh) = (grid.width as usize, grid.height as usize);
    let mut gray = vec![0u8; ow * oh];
    let mut alpha = vec![0u8; ow * oh];
    let inverse = |p: Vec2| g.inverse(p);
    for ty in 0..grid.height.div_ceil(TILE) {
        for tx in 0..grid.width.div_ceil(TILE) {
            let (pixels, bbox) = warp::source_pixels(&grid, tx, ty, 1, &inverse);
            let Some(bbox) = bbox else {
                continue;
            };
            let Some((x, y, rw, rh)) = warp::region_for(bbox, sw, sh) else {
                continue;
            };
            let region = reader.region(0, x, y, rw, rh).expect("the region");
            let out = warp::render(
                &pixels,
                &region,
                sw,
                sh,
                true,
                reader.info.sample,
                true,
                0.0,
                None,
            );
            let Samples::U8(v) = out else {
                panic!("8 bits")
            };
            for j in 0..TILE as usize {
                for i in 0..TILE as usize {
                    let (gx, gy) = (
                        tx as usize * TILE as usize + i,
                        ty as usize * TILE as usize + j,
                    );
                    if gx < ow && gy < oh {
                        let k = (j * TILE as usize + i) * 4;
                        gray[gy * ow + gx] = v[k];
                        alpha[gy * ow + gx] = v[k + 3];
                    }
                }
            }
        }
    }
    // The knots' bilinear stays within the cells' tolerance of exact here.
    let mut worst: f64 = 0.0;
    for ty in 0..grid.height.div_ceil(TILE) {
        for tx in 0..grid.width.div_ceil(TILE) {
            let (exact, _) = warp::source_pixels(&grid, tx, ty, 1, &inverse);
            let (knotted, _) = warp::source_pixels(&grid, tx, ty, STEP, &inverse);
            for (p, q) in exact.iter().zip(&knotted) {
                if p[0].is_finite() {
                    worst = worst.max((p[0] - q[0]).abs()).max((p[1] - q[1]).abs());
                }
            }
        }
    }
    assert!(
        worst <= CELL_TOLERANCE,
        "the knots' bilinear is {worst} px off"
    );
    let inside = alpha.iter().filter(|&&a| a > 0).count();
    assert_eq!(
        inside as u64,
        w["inside"].as_u64().unwrap_or(0),
        "pixels inside"
    );
    assert_eq!(fnv(&alpha), w["alpha"].as_str().unwrap_or(""), "alpha");
    assert_eq!(fnv(&gray), w["gray"].as_str().unwrap_or(""), "grey");
}

/// Raster oturt's resampling job (`warp::Job`) over the same raster and
/// grid: its GeoTIFF read back is, pixel for pixel, the knots' bilinear
/// inverse sampled nearest (what the job computes tile by tile), its place
/// the grid's, its outside clear.
#[test]
fn a_resampling_job_writes_the_tiles_it_works_out() {
    let file = file();
    let w = &file["warp"];
    let bytes = std::fs::read(
        dir()
            .join("files")
            .join(w["source"].as_str().expect("source")),
    )
    .expect("source");
    let mut reader = open_bytes(&bytes, None, 256 << 20).expect("reads");
    let (sw, sh) = (reader.info.width, reader.info.height);
    for need in reader.needs(0, 0, 0, sw, sh) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        assert_eq!(
            reader.put_block(&need, &bytes[a..b]).expect("a block"),
            Put::Done
        );
    }
    let method = Method::from_name(w["method"].as_str().unwrap_or("")).expect("a method");
    let g = solve(&gcps(&w["points"]), method).expect("solved");
    let border: Vec<Vec2> = w["border"]
        .as_array()
        .expect("border")
        .iter()
        .map(|p| g.forward(v2(p)).expect("forward"))
        .collect();
    let grid = Grid::covering(&border, w["pixel"].as_f64().expect("pixel")).expect("a grid");
    let g2 = g.clone();
    let (mut job, header) = warp::Job::new(
        grid,
        Box::new(move |p| g2.inverse(p)),
        sw,
        sh,
        reader.info.bands,
        reader.info.sample,
        reader.nodata,
        reader.palette.clone(),
        true,
        Some(5256),
        false,
    )
    .expect("a job");
    let mut out = header;
    while let Some(region) = job.region() {
        let samples =
            region.map(|(x, y, rw, rh)| reader.region(0, x, y, rw, rh).expect("the region"));
        out.extend(job.tile(samples.as_ref()).expect("a tile"));
    }
    assert!((job.share() - 1.0).abs() < 1e-12);
    let (dirs, head) = job.finish().expect("finished");
    out.extend(dirs);
    out[..head.len()].copy_from_slice(&head);
    let mut written = open_bytes(&out, None, 256 << 20).expect("the output reads");
    let info = written.info.clone();
    assert_eq!(
        (info.width, info.height, info.bands),
        (grid.width, grid.height, 4)
    );
    assert_eq!(info.affine, Some(grid.affine));
    assert_eq!(info.epsg, Some(5256));
    for need in written.needs(0, 0, 0, info.width, info.height) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        assert_eq!(
            written.put_block(&need, &out[a..b]).expect("a block"),
            Put::Done
        );
    }
    let all = written
        .region(0, 0, 0, info.width, info.height)
        .expect("the output's samples");
    let inverse = |p: Vec2| g.inverse(p);
    let mut off = 0usize;
    for ty in 0..grid.height.div_ceil(TILE) {
        for tx in 0..grid.width.div_ceil(TILE) {
            let (pixels, bbox) = warp::source_pixels(&grid, tx, ty, STEP, &inverse);
            let want = match bbox.and_then(|b| warp::region_for(b, sw, sh)) {
                Some((x, y, rw, rh)) => {
                    let region = reader.region(0, x, y, rw, rh).expect("the region");
                    warp::render(
                        &pixels,
                        &region,
                        sw,
                        sh,
                        true,
                        reader.info.sample,
                        true,
                        0.0,
                        None,
                    )
                }
                None => Samples::filled(reader.info.sample, (TILE * TILE * 4) as usize, 0.0),
            };
            for j in 0..TILE {
                for i in 0..TILE {
                    let (gx, gy) = (tx * TILE + i, ty * TILE + j);
                    if gx >= grid.width || gy >= grid.height {
                        continue;
                    }
                    for b in 0..4 {
                        let a = all.at(gx, gy, b);
                        let e = want.get(((j * TILE + i) * 4 + b) as usize);
                        if a != e {
                            off += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(off, 0, "{off} samples differ");
}
