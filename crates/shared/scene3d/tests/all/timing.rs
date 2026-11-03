//! How long the core takes on a large terrain: a 2 000 × 1 000 cell grid (4 million triangles, 2 km × 1 km at TM
//! coordinates) cut into tiles, and a frame's selection over it from near, far and above, every content ready; and a
//! deep quadtree (8 levels, 21 845 tiles) selected while a camera flies over it.
//!
//! ```text
//! cargo test --release -p kentos-scene3d --test all timing -- --ignored --nocapture
//! ```

use std::time::Instant;

use kentos_scene3d::{Aabb, Camera, ContentRef, Exaggeration, Hierarchy, Limits, Projection, Refine, Settings, SourceMesh, Tile, TileKey, Viewport, chunk, select};

fn persp() -> Projection {
    Projection::Perspective { fov_y: 40f64.to_radians(), near: 0.5 }
}

#[test]
#[ignore = "timing, run by hand in release"]
fn timing() {
    let (nx, ny) = (2000usize, 1000usize);
    let (x0, y0) = (512_000.0, 4_401_000.0);
    let mut vs = Vec::with_capacity((nx + 1) * (ny + 1));
    for j in 0..=ny {
        for i in 0..=nx {
            let (x, y) = (i as f64, j as f64);
            vs.push([x0 + x, y0 + y, 1200.0 + 0.02 * x + 6.0 * (y / 90.0).sin()]);
        }
    }
    let mut ts = Vec::with_capacity(nx * ny * 2);
    for j in 0..ny {
        for i in 0..nx {
            let a = (j * (nx + 1) + i) as u32;
            let w = (nx + 1) as u32;
            ts.push([a, a + 1, a + w + 1]);
            ts.push([a, a + w + 1, a + w]);
        }
    }
    let t = Instant::now();
    let c = chunk(&[SourceMesh { part: 0, vertices: &vs, triangles: &ts }], Limits::default(), 1);
    let cut = t.elapsed();
    let error = c.contents.iter().map(|k| k.encoding_error).fold(0.0, f64::max);
    let bytes: u64 = c.contents.iter().map(|k| k.bytes()).sum();
    println!(
        "bölme: {} üçgen → {} karo, {} içerik, {:.1} MB, en büyük float32 farkı {:.2e} m: {:.0} ms",
        ts.len(),
        c.hierarchy.tiles.len(),
        c.contents.len(),
        bytes as f64 / 1e6,
        error,
        cut.as_secs_f64() * 1e3
    );
    assert!(error < 1e-4);
    let vp = Viewport { width: 1920.0, height: 1080.0 };
    let views = [
        ("yakın", Camera::orbit([x0 + 300.0, y0 + 300.0, 1210.0], 210.0, 25.0, 150.0, persp())),
        ("uzak", Camera::orbit([x0 + 1000.0, y0 + 500.0, 1220.0], 45.0, 35.0, 3000.0, persp())),
        ("üst", Camera { eye: [x0 + 1000.0, y0 + 500.0, 4000.0], target: [x0 + 1000.0, y0 + 500.0, 0.0], up: [0.0, 0.0, 1.0], projection: persp() }),
    ];
    for (name, cam) in views {
        let t = Instant::now();
        let mut s = None;
        for _ in 0..100 {
            s = Some(select(&c.hierarchy, &cam, &vp, &Exaggeration::NONE, Settings { max_screen_error: 16.0 }, |_| true));
        }
        let s = s.unwrap();
        println!(
            "seçim ({name}): {} karo çizilir, {} bakılır, {} elenir: {:.3} ms",
            s.draw.len(),
            s.visited,
            s.culled,
            t.elapsed().as_secs_f64() * 1e3 / 100.0
        );
    }
    // A deep quadtree, every tile a content, the error halving: a flight over it.
    let mut tiles = Vec::new();
    fn make(tiles: &mut Vec<Tile>, x: f64, y: f64, s: f64, level: u32, ge: f64) -> u32 {
        let at = tiles.len() as u32;
        tiles.push(Tile {
            key: TileKey(u64::from(at)),
            bounds: Aabb { min: [x, y, 0.0], max: [x + s, y + s, 50.0] },
            geometric_error: if level < 7 { ge } else { 0.0 },
            refine: Refine::Replace,
            content: Some(ContentRef { id: u64::from(at), bytes: 1 }),
            children: Vec::new(),
        });
        if level < 7 {
            let h = s / 2.0;
            let kids = vec![
                make(tiles, x, y, h, level + 1, ge / 2.0),
                make(tiles, x + h, y, h, level + 1, ge / 2.0),
                make(tiles, x, y + h, h, level + 1, ge / 2.0),
                make(tiles, x + h, y + h, h, level + 1, ge / 2.0),
            ];
            tiles[at as usize].children = kids;
        }
        at
    }
    make(&mut tiles, 0.0, 0.0, 20_000.0, 0, 512.0);
    let h = Hierarchy { tiles, roots: vec![0] };
    assert_eq!(h.problem(), None);
    let t = Instant::now();
    let mut drawn = 0;
    for k in 0..200 {
        let x = 1000.0 + 90.0 * f64::from(k);
        let cam = Camera::orbit([x, 10_000.0, 20.0], 270.0, 20.0, 400.0, persp());
        drawn += select(&h, &cam, &vp, &Exaggeration::NONE, Settings { max_screen_error: 16.0 }, |_| true).draw.len();
    }
    println!(
        "uçuş: {} karo, kare başına ortalama {} karo çizilir: {:.3} ms",
        h.tiles.len(),
        drawn / 200,
        t.elapsed().as_secs_f64() * 1e3 / 200.0
    );
}
