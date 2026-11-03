//! A surface made by tiles (src/surface/tiled.rs, docs/adr/0916 §7): on points in general position the merged
//! triangles are the whole surface's, triangle for triangle; on a lattice (four points on a circle everywhere) the
//! count and the area hold; a halo grows where a tile's triangles are not certified.

use kentos_road::surface::{self, SpotPoint, SurfaceInput, TiledError, build_tiled};

/// Points from a fixed linear congruential sequence: the same everywhere.
fn scattered(n: usize, w: f64, h: f64) -> Vec<SpotPoint> {
    let mut s: u64 = 0x2545_f491_4f6c_dd1d;
    let mut next = || {
        s = s.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (s >> 11) as f64 / (1u64 << 53) as f64
    };
    (0..n)
        .map(|_| {
            let (x, y) = (500_000.0 + next() * w, 4_400_000.0 + next() * h);
            SpotPoint { x, y, z: 100.0 + 0.01 * (x - 500_000.0) + 5.0 * next(), source: None }
        })
        .collect()
}

fn keys(tris: impl IntoIterator<Item = [[f64; 3]; 3]>) -> Vec<[(u64, u64); 3]> {
    let mut out: Vec<[(u64, u64); 3]> = tris
        .into_iter()
        .map(|t| {
            let mut k = t.map(|v| (v[0].to_bits(), v[1].to_bits()));
            k.sort_unstable();
            k
        })
        .collect();
    out.sort_unstable();
    out
}

#[test]
fn tiles_merge_to_the_whole_surface() {
    let points = scattered(3000, 1000.0, 700.0);
    let whole = surface::build(&SurfaceInput { points: points.clone(), ..SurfaceInput::default() }).expect("builds");
    let want = keys(whole.triangles.iter().map(|t| t.map(|v| whole.vertices[v as usize])));
    // A small first halo: tiles near the hull grow theirs.
    let tiled = build_tiled(&points, None, 150.0, 5.0).expect("tiles");
    assert_eq!(keys(tiled.triangles.iter().copied()), want);
    assert_eq!(tiled.tiles, 7 * 5);
    assert!(tiled.builds > tiled.tiles, "some halos grew: {}", tiled.builds);
    assert!(tiled.seams > 0 && tiled.merges == 1, "{} {}", tiled.seams, tiled.merges);
    // One tile is the whole surface.
    let one = build_tiled(&points, None, 5000.0, 1.0).expect("one tile");
    assert_eq!(keys(one.triangles.iter().copied()), want);
}

#[test]
fn a_lattice_keeps_its_count_and_area() {
    let mut points = Vec::new();
    for j in 0..=40 {
        for i in 0..=60 {
            let (x, y) = (f64::from(i), f64::from(j));
            points.push(SpotPoint { x, y, z: 0.1 * x + 0.05 * y, source: None });
        }
    }
    let tiled = build_tiled(&points, None, 9.0, 1.0).expect("tiles");
    // 61 × 41 corners, 2 × 60 + 2 × 40 on the hull: 2·2501 − 2 − 200 triangles over 60 × 40.
    assert_eq!(tiled.triangles.len(), 2 * 2501 - 2 - 200);
    let area: f64 = tiled
        .triangles
        .iter()
        .map(|t| ((t[1][0] - t[0][0]) * (t[2][1] - t[0][1]) - (t[2][0] - t[0][0]) * (t[1][1] - t[0][1])) / 2.0)
        .sum();
    assert!((area - 2400.0).abs() < 1e-9, "{area}");
}

#[test]
fn the_refusals() {
    let points = scattered(10, 10.0, 10.0);
    assert_eq!(build_tiled(&points, None, 0.0, 1.0).err(), Some(TiledError::Tile));
    assert_eq!(build_tiled(&points, None, 1.0, f64::NAN).err(), Some(TiledError::Tile));
}

#[test]
fn any_tile_and_halo_give_the_same_surface() {
    let points = scattered(1500, 600.0, 900.0);
    let whole = surface::build(&SurfaceInput { points: points.clone(), ..SurfaceInput::default() }).expect("builds");
    let want = keys(whole.triangles.iter().map(|t| t.map(|v| whole.vertices[v as usize])));
    for (tile, halo) in [(37.0, 0.5), (100.0, 10.0), (250.0, 3.0), (600.0, 50.0)] {
        let t = build_tiled(&points, None, tile, halo).expect("tiles");
        assert_eq!(keys(t.triangles.iter().copied()), want, "tile {tile}, halo {halo}");
    }
}

/// `cargo test --release -p kentos-road --test all tiled::timing -- --ignored --nocapture` (docs/adr/0916 §7).
#[test]
#[ignore = "a measurement"]
fn timing() {
    let points = scattered(200_000, 4000.0, 3000.0);
    let t = std::time::Instant::now();
    let whole = surface::build(&SurfaceInput { points: points.clone(), ..SurfaceInput::default() }).expect("builds");
    let whole_ms = t.elapsed().as_secs_f64() * 1000.0;
    for (tile, halo) in [(500.0, 20.0), (1000.0, 40.0)] {
        let t = std::time::Instant::now();
        let tiled = build_tiled(&points, None, tile, halo).expect("tiles");
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(tiled.triangles.len(), whole.triangles.len());
        println!(
            "200 000 nokta: bütün {whole_ms:.0} ms; {tile} m karo, {halo} m pay: {ms:.0} ms, {} karo, {} kurma, {} dikiş, en büyük pay {} m",
            tiled.tiles, tiled.builds, tiled.seams, tiled.largest_halo
        );
    }
}
