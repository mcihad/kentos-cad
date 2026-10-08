//! Raster layers' budgets (docs/adr/0204 §11), on this machine, release:
//!
//!     cargo test --release -p kentos-formats --test all raster_timing -- --ignored --nocapture
//!
//! A 256 × 256 Deflate 8-bit RGB tile opened and coloured (≤ 3 ms), a 32-bit
//! DEM tile's shaded relief (≤ 6 ms), the first image of a 2048 × 2048
//! raster without overviews (≤ 150 ms: the last level worked out from every
//! block), the pyramid pass's rate, and a thin plate's resampled tile.

use std::time::Instant;

use kentos_contracts::{RasterRender, RasterSample, RasterStyle};
use kentos_formats::raster::pyramid::Builder;
use kentos_formats::raster::source::{Put, Reader, open_bytes};
use kentos_formats::raster::style::default_style;
use kentos_formats::raster::warp::{self, Grid, STEP};
use kentos_formats::raster::write::{Image, Writer};
use kentos_formats::raster::{Samples, TILE};
use kentos_geometry_core::ops::georef::{Gcp, Method, solve};
use kentos_geometry_core::vec2::Vec2;

/// A photograph's like: smooth light with a little grain.
fn photo(w: u32, h: u32) -> Samples {
    let mut s = Samples::filled(RasterSample::U8, (w * h * 3) as usize, 0.0);
    let mut seed: u32 = 12345;
    for j in 0..h {
        for i in 0..w {
            for c in 0..3u32 {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                let grain = f64::from(seed >> 28) - 8.0;
                let base = 40.0 + f64::from((i * (c + 1) + j * (3 - c)) % 512) * 0.35;
                s.set(((j * w + i) * 3 + c) as usize, base + grain);
            }
        }
    }
    s
}

/// A DEM's like: hills, metres.
fn dem(w: u32, h: u32) -> Samples {
    let mut s = Samples::filled(RasterSample::F32, (w * h) as usize, 0.0);
    for j in 0..h {
        for i in 0..w {
            let (x, y) = (f64::from(i) / 97.0, f64::from(j) / 131.0);
            let z =
                800.0 + 120.0 * (libm::sin(x) * libm::cos(y)) + 35.0 * libm::sin(x * 2.3 + y * 1.7);
            s.set((j * w + i) as usize, z);
        }
    }
    s
}

/// A tiled Deflate file of the samples (as GDAL writes one, level 6).
fn file_of(w: u32, h: u32, bands: u32, samples: &Samples) -> Vec<u8> {
    let image = Image {
        width: w,
        height: h,
        bands,
        sample: samples.kind(),
        alpha: false,
        geo: None,
        nodata: None,
    };
    let (mut writer, header) = Writer::new(vec![image], 6).expect("a writer");
    let mut file = header;
    let b = bands as usize;
    for ty in 0..h.div_ceil(TILE) {
        for tx in 0..w.div_ceil(TILE) {
            let mut t = Samples::filled(samples.kind(), (TILE * TILE) as usize * b, 0.0);
            for j in 0..TILE.min(h - ty * TILE) {
                let from = (((ty * TILE + j) * w + tx * TILE) as usize) * b;
                let n = TILE.min(w - tx * TILE) as usize * b;
                t.copy_run((j * TILE) as usize * b, samples, from, n);
            }
            file.extend(writer.tile(0, tx, ty, &t).expect("a tile"));
        }
    }
    let (dirs, head) = writer.finish().expect("finished");
    file.extend(dirs);
    file[..head.len()].copy_from_slice(&head);
    file
}

fn fill(reader: &mut Reader, bytes: &[u8], level: usize, x: i64, y: i64, w: u32, h: u32) {
    for need in reader.needs(level, x, y, w, h) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        assert_eq!(
            reader.put_block(&need, &bytes[a..b]).expect("a block"),
            Put::Done
        );
    }
}

/// Every tile of level 0 opened and coloured; milliseconds a tile.
fn tiles_ms(bytes: &[u8], style: &RasterStyle) -> f64 {
    let mut reader = open_bytes(bytes, None, 512 << 20).expect("reads");
    let stats = reader.stats();
    let affine = [0.0, 1.0, 0.0, 0.0, 0.0, -1.0];
    let (w, h) = (reader.info.width, reader.info.height);
    let mut rgba = Vec::new();
    let start = Instant::now();
    let mut n = 0;
    for ty in 0..h.div_ceil(TILE) {
        for tx in 0..w.div_ceil(TILE) {
            let (x, y, rw, rh) = Reader::tile_region(tx, ty);
            fill(&mut reader, bytes, 0, x, y, rw, rh);
            assert!(reader.render_tile(0, tx, ty, style, stats.as_ref(), &affine, &mut rgba));
            n += 1;
        }
    }
    start.elapsed().as_secs_f64() * 1000.0 / f64::from(n)
}

#[test]
#[ignore = "timing: run in release"]
fn rgb_and_dem_tiles() {
    let (w, h) = (2048, 2048);
    let rgb = file_of(w, h, 3, &photo(w, h));
    let style = default_style(3, RasterSample::U8, false);
    let ms = tiles_ms(&rgb, &style);
    println!("8 bit RGB, Deflate: {ms:.2} ms a tile (budget 3)");
    let dem_file = file_of(w, h, 1, &dem(w, h));
    let mut shade = default_style(1, RasterSample::F32, false);
    shade.render = RasterRender::Hillshade;
    let ms_dem = tiles_ms(&dem_file, &shade);
    println!("32 bit DEM, shaded relief: {ms_dem:.2} ms a tile (budget 6)");
    shade.render = RasterRender::RampShade;
    let ms_ramp = tiles_ms(&dem_file, &shade);
    println!("32 bit DEM, ramp and shade: {ms_ramp:.2} ms a tile");
}

#[test]
#[ignore = "timing: run in release"]
fn first_image_of_a_raster_without_overviews() {
    let (w, h) = (2048, 2048);
    let rgb = file_of(w, h, 3, &photo(w, h));
    let start = Instant::now();
    let mut reader = open_bytes(&rgb, None, 512 << 20).expect("reads");
    let last = reader.levels.len() - 1;
    fill(&mut reader, &rgb, 0, 0, 0, w, h);
    let stats = reader.stats();
    let style = default_style(3, RasterSample::U8, false);
    let mut rgba = Vec::new();
    assert!(reader.render_tile(
        last,
        0,
        0,
        &style,
        stats.as_ref(),
        &[0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
        &mut rgba
    ));
    let ms = start.elapsed().as_secs_f64() * 1000.0;
    println!("2048 × 2048 without overviews, first image (level {last}): {ms:.1} ms (budget 150)");
}

#[test]
#[ignore = "timing: run in release"]
fn pyramid_pass() {
    let (w, h) = (6000, 6000);
    let samples = photo(w, h);
    let mut reader = Reader::image(w, h, 3, samples, None, None, 1 << 30).expect("a reader");
    let start = Instant::now();
    let (mut b, header) = Builder::new(w, h, 3, RasterSample::U8, None, false).expect("a pass");
    let mut written = header.len();
    let mut y = 0;
    while y < h {
        let n = 256.min(h - y);
        let region = reader.region(0, 0, i64::from(y), w, n).expect("rows");
        written += b.push(&region).expect("pushed").len();
        y += n;
    }
    let (dirs, _) = b.finish().expect("finished");
    written += dirs.len();
    let s = start.elapsed().as_secs_f64();
    let mb = f64::from(w) * f64::from(h) * 3.0 / 1e6;
    println!(
        "pyramid pass, 6000 × 6000 RGB: {s:.2} s, {:.0} MB/s of level 0, file {:.1} MB",
        mb / s,
        written as f64 / 1e6
    );
}

#[test]
#[ignore = "timing: run in release"]
fn thin_plate_tile() {
    let (w, h) = (2048u32, 2048u32);
    let mut points = Vec::new();
    for j in 0..6 {
        for i in 0..5 {
            let (c, r) = (f64::from(i) * 500.0 + 10.0, f64::from(j) * 400.0 + 20.0);
            let (u, v) = (c / 2048.0 - 0.5, r / 2048.0 - 0.5);
            points.push(Gcp {
                pixel: Vec2::new(c, r),
                target: Vec2::new(
                    487_000.0 + 0.5 * c + 3.0 * u * v,
                    4_420_000.0 - 0.5 * r + 2.0 * u * u,
                ),
                used: true,
            });
        }
    }
    let g = solve(&points, Method::ThinPlate).expect("solved");
    let border: Vec<Vec2> = [(0.0, 0.0), (2048.0, 0.0), (2048.0, 2048.0), (0.0, 2048.0)]
        .iter()
        .map(|&(c, r)| g.forward(Vec2::new(c, r)).expect("forward"))
        .collect();
    let grid = Grid::covering(&border, 0.5).expect("a grid");
    let rgb = file_of(w, h, 3, &photo(w, h));
    let mut reader = open_bytes(&rgb, None, 512 << 20).expect("reads");
    fill(&mut reader, &rgb, 0, 0, 0, w, h);
    let inverse = |p: Vec2| g.inverse(p);
    let (tx, ty) = (grid.width / TILE / 2, grid.height / TILE / 2);
    for step in [STEP, 1] {
        let start = Instant::now();
        let rounds = 10;
        for _ in 0..rounds {
            let (pixels, bbox) = warp::source_pixels(&grid, tx, ty, step, &inverse);
            let (x, y, rw, rh) = warp::region_for(bbox.expect("inside"), w, h).expect("a region");
            let region = reader.region(0, x, y, rw, rh).expect("the region");
            let out = warp::render(
                &pixels,
                &region,
                w,
                h,
                false,
                RasterSample::U8,
                true,
                0.0,
                None,
            );
            assert_eq!(out.len(), (TILE * TILE * 4) as usize);
        }
        let ms = start.elapsed().as_secs_f64() * 1000.0 / f64::from(rounds);
        println!("thin plate of 30 points, bilinear, exact every {step} px: {ms:.2} ms a tile");
    }
}
