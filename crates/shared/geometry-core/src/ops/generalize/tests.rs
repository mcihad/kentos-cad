//! Kartografik genelleştirme's own checks (docs/adr/0221); the shared cases
//! with the independent reference are fixtures/processing/v1/generalize.json.

use super::Track;
use super::building::{Building, BuildingRing, simplify_rings};
use super::coverage::{self, build};
use super::orthogonal::{Squaring, square_track};
use super::simplify::simplify;
use super::smooth::{Smoothing, smooth_track};
use crate::vec2::Vec2;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

fn ring(pts: &[(f64, f64)]) -> Track {
    Track {
        pts: pts.iter().map(|&(x, y)| v(x, y)).collect(),
        bulges: vec![0.0; pts.len()],
        zs: vec![None; pts.len()],
        closed: true,
    }
}

#[test]
fn smoothing_cuts_corners_and_keeps_the_ends() {
    let mut path = ring(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
    path.closed = false;
    path.zs = vec![Some(0.0), Some(4.0), Some(8.0)];
    let s = Smoothing {
        iterations: 1,
        offset: 0.25,
        max_turn: std::f64::consts::PI,
    };
    let out = smooth_track(&path, &s);
    assert_eq!(
        out.pts,
        vec![v(0.0, 0.0), v(7.5, 0.0), v(10.0, 2.5), v(10.0, 10.0)]
    );
    assert_eq!(out.zs, vec![Some(0.0), Some(3.0), Some(5.0), Some(8.0)]);
    // A square ring: eight points; at 0.5 the midpoints, four.
    let square = ring(&[(0.0, 0.0), (4.0, 0.0), (4.0, 4.0), (0.0, 4.0)]);
    assert_eq!(smooth_track(&square, &s).pts.len(), 8);
    let half = Smoothing { offset: 0.5, ..s };
    assert_eq!(
        smooth_track(&square, &half).pts,
        vec![v(2.0, 0.0), v(4.0, 2.0), v(2.0, 4.0), v(0.0, 2.0)]
    );
    // A turn above the largest angle stays sharp.
    let sharp = Smoothing {
        max_turn: std::f64::consts::FRAC_PI_4,
        ..s
    };
    assert_eq!(smooth_track(&square, &sharp).pts, square.pts);
}

#[test]
fn squaring_makes_near_right_angles_right() {
    let t = ring(&[(0.0, 0.0), (10.0, 0.3), (10.2, 8.0), (-0.1, 7.9)]);
    let (out, moved) = square_track(
        &t,
        &Squaring {
            threshold: 15.0,
            iterations: 1000,
        },
    );
    assert!(moved);
    let n = out.pts.len();
    for i in 0..n {
        let (a, b, c) = (out.pts[(i + n - 1) % n], out.pts[i], out.pts[(i + 1) % n]);
        let (px, py, qx, qy) = (a.x - b.x, a.y - b.y, c.x - b.x, c.y - b.y);
        let d = (px * qx + py * qy) / ((px * px + py * py).sqrt() * (qx * qx + qy * qy).sqrt());
        assert!(d.abs() < 1e-4, "corner {i}: {d}");
    }
}

#[test]
fn a_short_jog_goes_and_right_angles_stay() {
    // A rectangle with a 1 m step on its top side: the step's two short edges become one line.
    let rule = Building {
        min_edge: 2.0,
        parallel: 20f64.to_radians(),
        orthogonal: 20f64.to_radians(),
    };
    let r = BuildingRing {
        pts: vec![
            v(0.0, 0.0),
            v(20.0, 0.0),
            v(20.0, 10.0),
            v(10.0, 10.0),
            v(10.0, 11.0),
            v(0.0, 11.0),
        ],
        bulges: vec![0.0; 6],
    };
    let mut rings = vec![r];
    assert!(simplify_rings(&mut rings, &rule) > 0);
    assert_eq!(rings[0].pts.len(), 4);
}

#[test]
fn neighbours_share_their_edge_once() {
    // Two squares side by side: their common side is one edge both use.
    let a = ring(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)]);
    let b = ring(&[(10.0, 0.0), (20.0, 0.0), (20.0, 10.0), (10.0, 10.0)]);
    let c = build(&[&a, &b]);
    assert_eq!(c.uses.iter().filter(|&&u| u == 2).count(), 1);
    let edges = c.edges.clone();
    let back = coverage::ring(&c, &edges, 1);
    assert_eq!(back.pts.len(), 4);
    // Simplified, the shared side with a wobble loses it once, for both.
    let a2 = ring(&[
        (0.0, 0.0),
        (10.0, 0.0),
        (10.05, 5.0),
        (10.0, 10.0),
        (0.0, 10.0),
    ]);
    let b2 = ring(&[
        (10.0, 0.0),
        (20.0, 0.0),
        (20.0, 10.0),
        (10.0, 10.0),
        (10.05, 5.0),
    ]);
    let c2 = build(&[&a2, &b2]);
    let out = simplify(&c2, 1.0, false);
    assert_eq!(coverage::ring(&c2, &out, 0).pts.len(), 4);
    assert_eq!(coverage::ring(&c2, &out, 1).pts.len(), 4);
}

/// Where Alanları kümele's time goes on a block of four buildings: each overlay step on its own.
#[test]
#[ignore = "timing, run on purpose in release"]
fn closing_phases() {
    use crate::geom::arrangement::{Area, Ring};
    use crate::ops::geoprocess::buffer::{Side, buffer};
    use crate::ops::geoprocess::{Class, subtract_all, union_all};
    use std::time::Instant;
    let block: Vec<Area> = [(0.0, 0.0), (15.0, 0.0), (0.0, 11.8), (15.0, 11.8)]
        .iter()
        .map(|&(x, y)| Area {
            outer: Ring {
                pts: [
                    (0.0, 0.0),
                    (12.0, 0.0),
                    (12.0, 8.0),
                    (6.0, 8.0),
                    (6.0, 8.8),
                    (0.0, 8.8),
                ]
                .iter()
                .map(|&(a, b)| v(487_000.0 + x + a, 4_420_000.0 + y + b))
                .collect(),
                bulges: None,
            },
            holes: Vec::new(),
        })
        .collect();
    let n = 200;
    let mut t = [0.0f64; 5];
    for _ in 0..n {
        let s = Instant::now();
        let u = union_all(&block);
        t[0] += s.elapsed().as_secs_f64();
        let s = Instant::now();
        let out = buffer(&Class::Areas(u.clone()), 2.5, Side::Both);
        t[1] += s.elapsed().as_secs_f64();
        let s = Instant::now();
        let closing = buffer(&Class::Areas(out), -2.5, Side::Both);
        t[2] += s.elapsed().as_secs_f64();
        let s = Instant::now();
        let extra = subtract_all(&closing, &u);
        t[3] += s.elapsed().as_secs_f64();
        let s = Instant::now();
        let mut all = u.clone();
        all.extend(extra);
        let _ = union_all(&all);
        t[4] += s.elapsed().as_secs_f64();
    }
    for (name, x) in ["union", "out", "in", "extra", "join"].iter().zip(t) {
        println!("{name}: {:.3} ms", x * 1e3 / n as f64);
    }
}

/// Where Ortak kenarları koruyarak sadeleştir's time goes on 100 000 wobbling parcels.
#[test]
#[ignore = "timing, run on purpose in release"]
fn coverage_phases() {
    use std::time::Instant;
    let wobble =
        |a: usize, b: usize, k: usize| (((a * 31 + b * 17 + k * 7) % 9) as f64 - 4.0) * 0.02;
    let (cols, rows) = (320, 313);
    let mut rings = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            let (x0, y0) = (487_000.0 + c as f64 * 20.0, 4_420_000.0 + r as f64 * 20.0);
            let (x1, y1) = (x0 + 20.0, y0 + 20.0);
            let mut p = vec![(x0, y0)];
            for k in 1..4 {
                p.push((x0 + 5.0 * k as f64, y0 + wobble(c, 2 * r, k)));
            }
            p.push((x1, y0));
            for k in 1..4 {
                p.push((x1 + wobble(c + 1, 2 * r + 1, k), y0 + 5.0 * k as f64));
            }
            p.push((x1, y1));
            for k in (1..4).rev() {
                p.push((x0 + 5.0 * k as f64, y1 + wobble(c, 2 * (r + 1), k)));
            }
            p.push((x0, y1));
            for k in (1..4).rev() {
                p.push((x0 + wobble(c, 2 * r + 1, k), y0 + 5.0 * k as f64));
            }
            rings.push(ring(&p));
        }
    }
    let refs: Vec<&Track> = rings.iter().collect();
    let t = Instant::now();
    let c = build(&refs);
    println!(
        "build: {:.1} ms, {} edges",
        t.elapsed().as_secs_f64() * 1e3,
        c.edges.len()
    );
    let t = Instant::now();
    let out = simplify(&c, 1.0, false);
    println!("simplify: {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
    let t = Instant::now();
    let mut n = 0;
    for k in 0..rings.len() {
        n += coverage::ring(&c, &out, k).pts.len();
    }
    println!(
        "rings: {:.1} ms, {n} vertices",
        t.elapsed().as_secs_f64() * 1e3
    );
}

/// A block of four turned buildings 3 m apart joins into one at 5 m (the timings' scene).
#[test]
fn a_turned_block_joins() {
    use super::aggregate::{Aggregation, aggregate};
    use crate::entity::Shape;
    let shape = [
        (0.0, 0.0),
        (12.0, 0.0),
        (12.0, 8.0),
        (6.0, 8.0),
        (6.0, 8.8),
        (0.0, 8.8),
    ];
    let mut shapes = Vec::new();
    let mut id = 0usize;
    for (ox, oy) in [(0.0, 0.0), (15.0, 0.0), (0.0, 11.8), (15.0, 11.8)] {
        id += 1;
        let turn = ((id % 7) as f64 - 3.0) * 0.4 * std::f64::consts::PI / 180.0;
        let (s, c) = (crate::jsmath::sin(turn), crate::jsmath::cos(turn));
        let pts: Vec<Vec2> = shape
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| {
                let (nx, ny) = (
                    ((id * 7 + i * 3) % 7) as f64 * 0.05 - 0.15,
                    ((id * 5 + i * 11) % 7) as f64 * 0.05 - 0.15,
                );
                let (x, y) = (x - 6.0 + nx, y - 4.4 + ny);
                v(
                    487_000.0 + ox + 6.0 + x * c - y * s,
                    4_420_000.0 + oy + 4.4 + x * s + y * c,
                )
            })
            .collect();
        shapes.push(Shape::Polygon {
            pts,
            bulges: None,
            holes: None,
            parts: None,
        });
    }
    let refs: Vec<&Shape> = shapes.iter().collect();
    let got = aggregate(
        &refs,
        &Aggregation {
            distance: 5.0,
            min_area: 0.0,
            min_hole: 0.0,
            fill_notches: false,
        },
    );
    let counts: Vec<usize> = got.results.iter().map(|r| r.members.len()).collect();
    assert_eq!(counts, vec![4], "{counts:?}");
}
