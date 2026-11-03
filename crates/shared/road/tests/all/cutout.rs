//! `kentos_road::cutout` against fixtures/road/v1/cutout.json (scripts/fixtures/road_cutout_cases.py: the areas by
//! shapely, no KentOS code): the plan area kept and lost; every triangle given counterclockwise, on the terrain (its
//! centroid's height the terrain's there) and outside the corridors (its centroid in none of their triangles); the
//! triangles' areas summing to the area kept, so none overlaps another.

use kentos_road::cutout::cut_out;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/road/v1/cutout.json");

fn p3(v: &Value) -> [f64; 3] {
    [v[0].as_f64().unwrap(), v[1].as_f64().unwrap(), v[2].as_f64().unwrap()]
}

fn area(a: [f64; 3], b: [f64; 3], c: [f64; 3]) -> f64 {
    0.5 * ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]))
}

/// Barycentric weights of `p` in `t`, none outside (a hair allowed).
fn inside(t: [[f64; 3]; 3], p: [f64; 2]) -> Option<[f64; 3]> {
    let d = area(t[0], t[1], t[2]);
    if d.abs() < 1e-12 {
        return None;
    }
    let q = [p[0], p[1], 0.0];
    let w = [area(q, t[1], t[2]) / d, area(t[0], q, t[2]) / d, area(t[0], t[1], q) / d];
    w.iter().all(|x| *x >= -1e-9).then_some(w)
}

#[test]
fn corridors_cut_out_of_the_terrain_as_shapely_says() {
    let all: Value = serde_json::from_str(CASES).expect("json");
    for c in all["cases"].as_array().unwrap() {
        let name = c["name"].as_str().unwrap();
        let vs: Vec<[f64; 3]> = c["vertices"].as_array().unwrap().iter().map(p3).collect();
        let ts: Vec<[u32; 3]> = c["triangles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| [0, 1, 2].map(|k| t[k].as_u64().unwrap() as u32))
            .collect();
        let foot: Vec<[[f64; 3]; 3]> =
            c["footprint"].as_array().unwrap().iter().map(|t| [p3(&t[0]), p3(&t[1]), p3(&t[2])]).collect();
        let cut = cut_out(&vs, &ts, &foot);
        let terrain: Vec<[[f64; 3]; 3]> = ts.iter().map(|t| t.map(|k| vs[k as usize])).collect();
        let mut sum = 0.0;
        for t in &cut.triangles {
            let [a, b, cc] = t.map(|k| cut.vertices[k as usize]);
            let s = area(a, b, cc);
            assert!(s > -1e-9, "{name}: a clockwise triangle {s}");
            sum += s;
            let m = [(a[0] + b[0] + cc[0]) / 3.0, (a[1] + b[1] + cc[1]) / 3.0];
            let z = (a[2] + b[2] + cc[2]) / 3.0;
            let on = terrain.iter().find_map(|tt| inside(*tt, m).map(|w| w[0] * tt[0][2] + w[1] * tt[1][2] + w[2] * tt[2][2]));
            let ground = on.unwrap_or_else(|| panic!("{name}: a triangle off the terrain at {m:?}"));
            assert!((ground - z).abs() < 1e-6, "{name}: {z} not on the terrain's {ground} at {m:?}");
            if s > 1e-6 {
                assert!(foot.iter().all(|f| inside(*f, m).is_none_or(|w| w.iter().any(|x| *x < 1e-9))), "{name}: a triangle under a corridor at {m:?}");
            }
        }
        let want = c["keptArea"].as_f64().unwrap();
        assert!((sum - want).abs() <= 1e-6 * want.max(1.0), "{name}: kept {sum}, shapely {want}");
        assert_eq!(cut.kept + cut.dropped + cut.clipped, ts.len(), "{name}: every triangle accounted for");
        println!("{name}: {} kept, {} dropped, {} clipped → {} triangles", cut.kept, cut.dropped, cut.clipped, cut.triangles.len());
    }
}

/// A 1 m grid terrain of 1 000 × 1 000 cells (2 million triangles) and a 2 km winding road 14 m wide in triangles every
/// 5 m: how long the cut takes.
///
/// ```text
/// cargo test --release -p kentos-road --test all cutout_timing -- --ignored --nocapture
/// ```
#[test]
#[ignore = "timing, run by hand in release"]
fn cutout_timing() {
    let n = 1000usize;
    let mut vs = Vec::with_capacity((n + 1) * (n + 1));
    for j in 0..=n {
        for i in 0..=n {
            vs.push([i as f64, j as f64, 100.0 + 0.01 * i as f64]);
        }
    }
    let w = (n + 1) as u32;
    let mut ts = Vec::with_capacity(n * n * 2);
    for j in 0..n {
        for i in 0..n {
            let a = (j * (n + 1) + i) as u32;
            ts.push([a, a + 1, a + w + 1]);
            ts.push([a, a + w + 1, a + w]);
        }
    }
    let mut foot = Vec::new();
    let line: Vec<[f64; 2]> = (0..=400).map(|k| {
        let s = k as f64 * 2.5;
        [s, 500.0 + 200.0 * (s / 160.0).sin()]
    }).collect();
    for p in line.windows(2) {
        let (dx, dy) = (p[1][0] - p[0][0], p[1][1] - p[0][1]);
        let l = dx.hypot(dy);
        let (nx, ny) = (-dy / l * 7.0, dx / l * 7.0);
        let a = [p[0][0] + nx, p[0][1] + ny, 0.0];
        let b = [p[0][0] - nx, p[0][1] - ny, 0.0];
        let c = [p[1][0] - nx, p[1][1] - ny, 0.0];
        let d = [p[1][0] + nx, p[1][1] + ny, 0.0];
        foot.push([b, c, d]);
        foot.push([b, d, a]);
    }
    let t = std::time::Instant::now();
    let cut = cut_out(&vs, &ts, &foot);
    println!(
        "{} arazi üçgeni, {} koridor üçgeni: {} kaldı, {} atıldı, {} kırpıldı → {} üçgen: {:.0} ms",
        ts.len(),
        foot.len(),
        cut.kept,
        cut.dropped,
        cut.clipped,
        cut.triangles.len(),
        t.elapsed().as_secs_f64() * 1e3
    );
}
