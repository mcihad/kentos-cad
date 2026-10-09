//! Delaunay triangulation (docs/adr/0232 §4): the cases of
//! `fixtures/delaunay/v1/cases.json` (`scripts/fixtures/delaunay_cases.py`:
//! qhull's triangles checked exactly), and the rules every triangulation
//! keeps on random and degenerate sets, checked with the exact predicates:
//! counter-clockwise triangles, twins that match, no point inside a
//! neighbour's circle, a convex hull, Euler's count; then `locate`, `star`
//! and `fill_star` against plain answers.

use kentos_geometry_core::geom::delaunay::{
    Delaunay, Located, NONE, TOO_FEW, fill_star, next, triangulate,
};
use kentos_geometry_core::jsmath::{cos, js_round, sin};
use kentos_geometry_core::predicates::{incircle, orient2d};
use kentos_geometry_core::vec2::Vec2;
use serde_json::Value;

fn cases() -> Value {
    let path = format!(
        "{}/../../../fixtures/delaunay/v1/cases.json",
        env!("CARGO_MANIFEST_DIR")
    );
    serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON")
}

fn points_of(c: &Value) -> Vec<Vec2> {
    c["points"]
        .as_array()
        .expect("points")
        .iter()
        .map(|p| Vec2::new(p[0].as_f64().expect("x"), p[1].as_f64().expect("y")))
        .collect()
}

/// Each triangle rotated to start at its least point, the list sorted.
fn canonical(d: &Delaunay) -> Vec<[u32; 3]> {
    let mut out: Vec<[u32; 3]> = (0..d.len() as u32)
        .map(|t| {
            let mut c = d.corners(t);
            while c[0] != *c.iter().min().expect("three") {
                c = [c[1], c[2], c[0]];
            }
            c
        })
        .collect();
    out.sort_unstable();
    out
}

/// The rules every triangulation of `points` keeps.
fn check(points: &[Vec2], d: &Delaunay) {
    let n_tri = d.len();
    for t in 0..n_tri as u32 {
        let [a, b, c] = d.corners(t).map(|i| points[i as usize]);
        assert!(
            orient2d(a, b, c) > 0.0,
            "triangle {t} is not counter-clockwise"
        );
    }
    let mut hull_edges = 0;
    for e in 0..d.halfedges.len() as u32 {
        let tw = d.halfedges[e as usize];
        if tw == NONE {
            hull_edges += 1;
            continue;
        }
        assert_eq!(d.halfedges[tw as usize], e, "twins of {e}");
        assert_eq!(d.triangles[e as usize], d.triangles[next(tw) as usize]);
        assert_eq!(d.triangles[next(e) as usize], d.triangles[tw as usize]);
        // The point across is not inside this triangle's circle.
        let t = e / 3;
        let [a, b, c] = d.corners(t).map(|i| points[i as usize]);
        let across = d.triangles[(3 * (tw / 3) + (tw % 3 + 2) % 3) as usize];
        assert!(
            incircle(a, b, c, points[across as usize]) <= 0.0,
            "edge {e}: point {across} inside triangle {t}'s circle"
        );
    }
    assert_eq!(hull_edges, d.hull.len(), "hull edges and hull points");
    let h = d.hull.len();
    for k in 0..h {
        let (a, b, c) = (
            points[d.hull[k] as usize],
            points[d.hull[(k + 1) % h] as usize],
            points[d.hull[(k + 2) % h] as usize],
        );
        assert!(orient2d(a, b, c) >= 0.0, "the hull turns right at {k}");
    }
    let mut used = vec![false; points.len()];
    for &v in &d.triangles {
        used[v as usize] = true;
    }
    let used_count = used.iter().filter(|&&u| u).count();
    assert_eq!(
        used_count + d.skipped.len(),
        points.len(),
        "every point used or skipped"
    );
    for &s in &d.skipped {
        assert!(!used[s as usize]);
        assert!(
            points
                .iter()
                .enumerate()
                .any(|(i, p)| used[i] && *p == points[s as usize]),
            "skipped point {s} equals no used point"
        );
    }
    assert_eq!(n_tri, 2 * used_count - h - 2, "Euler's count");
}

#[test]
fn the_cases_triangulate_as_the_reference_says() {
    let all = cases();
    let list = all["cases"].as_array().expect("cases");
    assert!(list.len() >= 11);
    for c in list {
        let name = c["name"].as_str().expect("name");
        let points = points_of(c);
        let got = triangulate(&points);
        if c.get("error").is_some() {
            assert_eq!(got.err().as_deref(), Some(TOO_FEW), "{name}");
            continue;
        }
        let d = got.unwrap_or_else(|e| panic!("{name}: {e}"));
        check(&points, &d);
        if let Some(want) = c.get("triangles") {
            let want: Vec<[u32; 3]> = want
                .as_array()
                .expect("triangles")
                .iter()
                .map(|t| [0, 1, 2].map(|k| t[k].as_u64().expect("index") as u32))
                .collect();
            assert_eq!(canonical(&d), want, "{name}: the triangles");
            let mut hull = d.hull.clone();
            hull.sort_unstable();
            let want_hull: Vec<u32> = c["hull"]
                .as_array()
                .expect("hull")
                .iter()
                .map(|v| v.as_u64().expect("index") as u32)
                .collect();
            assert_eq!(hull, want_hull, "{name}: the hull");
        } else {
            let used = points.len() - d.skipped.len();
            assert_eq!(used as u64, c["used"].as_u64().expect("used"), "{name}");
            assert_eq!(
                d.hull.len() as u64,
                c["hullPoints"].as_u64().expect("hull"),
                "{name}"
            );
            assert_eq!(
                d.len() as u64,
                c["triangleCount"].as_u64().expect("count"),
                "{name}"
            );
        }
    }
}

/// Deterministic random numbers (xorshift64*).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next()
    }
}

#[test]
fn random_and_degenerate_sets_keep_the_rules() {
    let mut g = Rng(0xde1a_0aa7);
    // Uniform, at TM coordinates on the millimetre grid, a grid with
    // repeated points, points on a few circles, and a dense fan.
    let mut sets: Vec<Vec<Vec2>> = Vec::new();
    sets.push(
        (0..2_000)
            .map(|_| Vec2::new(g.range(0.0, 1.0), g.range(0.0, 1.0)))
            .collect(),
    );
    sets.push(
        (0..2_000)
            .map(|_| {
                Vec2::new(
                    js_round((500_000.0 + g.range(0.0, 2_000.0)) * 1000.0) / 1000.0,
                    js_round((4_420_000.0 + g.range(0.0, 2_000.0)) * 1000.0) / 1000.0,
                )
            })
            .collect(),
    );
    let mut grid: Vec<Vec2> = (0..30)
        .flat_map(|j| (0..40).map(move |i| Vec2::new(f64::from(i) * 2.5, f64::from(j) * 2.5)))
        .collect();
    grid.extend(grid.clone().into_iter().step_by(7));
    sets.push(grid);
    let mut rings = Vec::new();
    for r in [5.0, 10.0, 25.0] {
        for (x, y) in [(3.0, 4.0), (4.0, 3.0), (5.0, 0.0), (0.0, 5.0)] {
            for (sx, sy) in [(1.0, 1.0), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
                rings.push(Vec2::new(sx * x * r / 5.0, sy * y * r / 5.0));
            }
        }
    }
    rings.push(Vec2::new(0.25, 0.5));
    sets.push(rings);
    sets.push(
        (0..500)
            .map(|k| {
                let t = f64::from(k) * 0.01;
                Vec2::new(cos(t) * (1.0 + t), sin(t) * (1.0 + t))
            })
            .collect(),
    );
    for points in &sets {
        let d = triangulate(points).expect("a triangulation");
        check(points, &d);
    }
}

#[test]
fn bad_input_is_refused() {
    assert_eq!(triangulate(&[]).err().as_deref(), Some(TOO_FEW));
    let nan = [
        Vec2::new(0.0, 0.0),
        Vec2::new(1.0, 0.0),
        Vec2::new(f64::NAN, 1.0),
    ];
    assert!(triangulate(&nan).is_err());
}

#[test]
fn a_point_is_located_in_its_triangle_or_outside() {
    let mut g = Rng(77);
    let points: Vec<Vec2> = (0..500)
        .map(|_| Vec2::new(g.range(0.0, 100.0), g.range(0.0, 50.0)))
        .collect();
    let d = triangulate(&points).expect("triangulates");
    let mut start = 0;
    for _ in 0..5_000 {
        let q = Vec2::new(g.range(-20.0, 120.0), g.range(-20.0, 70.0));
        // The plain answer: a triangle with q on no edge's right.
        let holder = (0..d.len() as u32).find(|&t| {
            let [a, b, c] = d.corners(t).map(|i| points[i as usize]);
            orient2d(a, b, q) >= 0.0 && orient2d(b, c, q) >= 0.0 && orient2d(c, a, q) >= 0.0
        });
        match (d.locate(&points, q, start), holder) {
            (Located::Inside(t), Some(_)) => {
                let [a, b, c] = d.corners(t).map(|i| points[i as usize]);
                assert!(
                    orient2d(a, b, q) >= 0.0
                        && orient2d(b, c, q) >= 0.0
                        && orient2d(c, a, q) >= 0.0
                );
                start = t;
            }
            (Located::Outside(e), None) => {
                assert_eq!(d.halfedges[e as usize], NONE);
                start = e / 3;
            }
            (got, want) => panic!("{q:?}: located {got:?}, the plain answer {want:?}"),
        }
    }
}

#[test]
fn a_star_runs_round_its_point_and_its_hole_fills_as_the_triangulation_without_it() {
    let mut g = Rng(4242);
    let points: Vec<Vec2> = (0..300)
        .map(|_| Vec2::new(g.range(0.0, 10.0), g.range(0.0, 10.0)))
        .collect();
    let d = triangulate(&points).expect("triangulates");
    let out = d.out_edges(points.len());
    let mut interior = 0;
    for v in 0..points.len() as u32 {
        let (ring, on_hull) = d.star(&out, v);
        assert_eq!(on_hull, d.hull.contains(&v), "point {v}");
        // Each neighbour shares a triangle edge with v.
        for w in &ring {
            assert!(
                (0..d.halfedges.len()).any(|e| d.triangles[e] == v
                    && d.triangles[next(e as u32) as usize] == *w
                    || d.triangles[e] == *w && d.triangles[next(e as u32) as usize] == v),
                "{v}–{w} is no edge"
            );
        }
        // Counter-clockwise: consecutive neighbours turn left round v.
        let pv = points[v as usize];
        let m = ring.len();
        let turns = if on_hull { m - 1 } else { m };
        for k in 0..turns {
            let (a, b) = (points[ring[k] as usize], points[ring[(k + 1) % m] as usize]);
            assert!(
                orient2d(pv, a, b) > 0.0,
                "point {v}: the star turns right at {k}"
            );
        }
        if on_hull || interior >= 60 {
            continue;
        }
        interior += 1;
        // The hole v leaves, filled: the triangles of the set without v that use only its neighbours.
        let mut filled: Vec<[u32; 3]> = fill_star(&points, &ring);
        let others: Vec<Vec2> = points
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != v as usize)
            .map(|(_, p)| *p)
            .collect();
        let without = triangulate(&others).expect("triangulates");
        let back = |i: u32| if i >= v { i + 1 } else { i };
        // Inside the hole: three neighbours of v whose triangle's centroid is
        // inside the ring (a triangle of three neighbours can lie in a pocket
        // outside it).
        let inside = |p: Vec2| {
            let mut odd = false;
            for k in 0..ring.len() {
                let (a, b) = (
                    points[ring[k] as usize],
                    points[ring[(k + 1) % ring.len()] as usize],
                );
                if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y)
                {
                    odd = !odd;
                }
            }
            odd
        };
        let mut want: Vec<[u32; 3]> = (0..without.len() as u32)
            .map(|t| without.corners(t).map(back))
            .filter(|c| c.iter().all(|i| ring.contains(i)))
            .filter(|c| {
                let [a, b, cc] = c.map(|i| points[i as usize]);
                inside(Vec2::new(
                    (a.x + b.x + cc.x) / 3.0,
                    (a.y + b.y + cc.y) / 3.0,
                ))
            })
            .collect();
        let rot = |c: &mut [u32; 3]| {
            while c[0] != *c.iter().min().expect("three") {
                *c = [c[1], c[2], c[0]];
            }
        };
        filled.iter_mut().for_each(rot);
        want.iter_mut().for_each(rot);
        filled.sort_unstable();
        want.sort_unstable();
        assert_eq!(filled, want, "point {v}'s hole");
    }
    assert!(interior >= 60);
}
