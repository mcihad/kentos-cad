//! The Örnekleme rules against brute force (docs/adr/0220): grids that tile
//! with shared corners, Nesnelere değenler as the queries' Kesişen, points
//! in and along objects, pieces that keep the whole.

use std::collections::HashMap;

use super::along::{Spacing, distances, marks};
use super::cover::Cover;
use super::grid::{Kind, Spec, pieces};
use super::inside::Inside;
use super::random::Sequence;
use super::regular::regular;
use super::scatter::{Target, scatter};
use super::subdivide::subdivide;
use super::walk::walks_of;
use super::{inside_of, target_of};
use crate::entity::Shape;
use crate::geom::arrangement::{Area, Ring};
use crate::geom::region::{inside_area, net_area};
use crate::geometry::Bounds;
use crate::jsmath::{cos, js_max, js_min, sin};
use crate::ops::areas::areas_of_entity;
use crate::ops::edges::{edge_length, entity_edges};
use crate::ops::geoprocess::{Class, class_of};
use crate::ops::spatial_query::{geometry_of, intersects};
use crate::vec2::Vec2;

fn v(x: f64, y: f64) -> Vec2 {
    Vec2::new(x, y)
}

fn polygon(pts: &[(f64, f64)]) -> Shape {
    Shape::Polygon {
        pts: pts.iter().map(|&(x, y)| v(x, y)).collect(),
        bulges: None,
        holes: None,
        parts: None,
    }
}

fn polyline(pts: &[(f64, f64)]) -> Shape {
    Shape::Polyline {
        pts: pts.iter().map(|&(x, y)| v(x, y)).collect(),
        bulges: None,
        holes: None,
        parts: None,
        closed: false,
        widths: None,
    }
}

fn shoelace(r: &[Vec2]) -> f64 {
    let n = r.len();
    (0..n)
        .map(|i| {
            let (a, b) = (r[i], r[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum::<f64>()
        / 2.0
}

fn bits(p: Vec2) -> (u64, u64) {
    (p.x.to_bits(), p.y.to_bits())
}

fn bx(x0: f64, y0: f64, x1: f64, y1: f64) -> Bounds {
    Bounds {
        min_x: x0,
        min_y: y0,
        max_x: x1,
        max_y: y1,
    }
}

#[test]
fn sequences_repeat_and_stay_below_one() {
    let (mut a, mut b) = (Sequence::new(7), Sequence::new(7));
    for _ in 0..1000 {
        let (x, y) = (a.unit(), b.unit());
        assert_eq!(x.to_bits(), y.to_bits());
        assert!((0.0..1.0).contains(&x));
    }
    assert_ne!(Sequence::new(7).next_u64(), Sequence::new(8).next_u64());
}

#[test]
fn rectangles_cover_the_box_on_multiples() {
    let b = bx(10.5, 20.25, 95.1, 61.3);
    let spec = Spec {
        kind: Kind::Rect,
        dx: 10.0,
        dy: 5.0,
        side: 0.0,
        snap: true,
    };
    let cells = pieces(&b, &spec, None).unwrap();
    assert_eq!(cells.len(), 9 * 9);
    // Row 1 is the top, column 1 the left; corners on multiples.
    assert_eq!(cells[0].pts[0], v(10.0, 60.0));
    assert_eq!(cells[0].pts[2], v(20.0, 65.0));
    let last = cells.last().unwrap();
    assert_eq!((last.row, last.col), (Some(9), Some(9)));
    assert_eq!(last.pts[0], v(90.0, 20.0));
    let lines = pieces(
        &b,
        &Spec {
            kind: Kind::Lines,
            ..spec
        },
        None,
    )
    .unwrap();
    assert_eq!(lines.len(), 10 + 10);
    assert_eq!(lines[0].pts, vec![v(10.0, 65.0), v(100.0, 65.0)]);
    assert_eq!(lines[10].pts, vec![v(10.0, 20.0), v(10.0, 65.0)]);
}

/// Every cell exactly once over a point of the box: no gap, no overlap.
fn tiles(kind: Kind, side: f64, around: usize) {
    let b = bx(103.7, -41.2, 318.4, 77.9);
    let spec = Spec {
        kind,
        dx: 0.0,
        dy: 0.0,
        side,
        snap: true,
    };
    let cells = pieces(&b, &spec, None).unwrap();
    let want = match kind {
        Kind::Hexagon => 1.5 * 3f64.sqrt() * side * side,
        _ => 3f64.sqrt() / 4.0 * side * side,
    };
    let mut seen: HashMap<(u64, u64), usize> = HashMap::new();
    for c in &cells {
        let a = shoelace(&c.pts);
        assert!(a > 0.0 && (a - want).abs() < 1e-9 * want, "{a} {want}");
        for &p in &c.pts {
            *seen.entry(bits(p)).or_default() += 1;
        }
    }
    assert!(seen.values().all(|&n| n <= around));
    assert!(
        seen.values().any(|&n| n == around),
        "corners are shared bit for bit"
    );
    let mut seq = Sequence::new(3);
    for _ in 0..2000 {
        let p = v(
            b.min_x + seq.unit() * (b.max_x - b.min_x),
            b.min_y + seq.unit() * (b.max_y - b.min_y),
        );
        let n = cells
            .iter()
            .filter(|c| {
                let area = Area {
                    outer: Ring {
                        pts: c.pts.clone(),
                        bulges: None,
                    },
                    holes: Vec::new(),
                };
                inside_area(&area, p)
            })
            .count();
        assert_eq!(n, 1, "{p:?}");
    }
}

#[test]
fn hexagons_tile_the_box() {
    tiles(Kind::Hexagon, 17.3, 3);
}

#[test]
fn triangles_tile_the_box() {
    tiles(Kind::Triangle, 21.9, 6);
}

fn objects() -> Vec<Shape> {
    vec![
        polygon(&[(120.0, 10.0), (190.0, 4.0), (176.0, 60.0), (131.0, 47.0)]),
        polyline(&[(210.0, -30.0), (260.0, 20.0), (300.0, -10.0)]),
        Shape::Circle {
            c: v(240.0, 55.0),
            r: 12.5,
        },
        Shape::Point {
            p: v(150.5, -30.25),
            z: None,
            parts: None,
        },
        Shape::Arc {
            c: v(280.0, 50.0),
            r: 20.0,
            a0: 0.3,
            a1: 2.4,
        },
        // A cell's corner touching a line exactly.
        Shape::Line {
            a: v(100.0, 70.0),
            b: v(110.0, 80.0),
        },
    ]
}

#[test]
fn meeting_cells_are_the_queries_kesişen() {
    let shapes = objects();
    let cover = Cover::new(&shapes);
    let geoms: Vec<_> = shapes.iter().filter_map(geometry_of).collect();
    let b = bx(100.0, -50.0, 320.0, 80.0);
    let mut near = Vec::new();
    for (kind, side) in [
        (Kind::Rect, 0.0),
        (Kind::Hexagon, 7.0),
        (Kind::Triangle, 9.0),
    ] {
        let spec = Spec {
            kind,
            dx: 10.0,
            dy: 10.0,
            side,
            snap: true,
        };
        let (mut kept, mut both) = (0, 0);
        for c in pieces(&b, &spec, None).unwrap() {
            let cell = geometry_of(&polygon(
                &c.pts.iter().map(|p| (p.x, p.y)).collect::<Vec<_>>(),
            ))
            .unwrap();
            let want = geoms.iter().any(|g| intersects(&cell, g));
            assert_eq!(cover.meets(&c.pts, &mut near), want, "{kind:?} {:?}", c.pts);
            kept += usize::from(want);
            both += 1;
        }
        assert!(kept > 0 && kept < both);
    }
}

#[test]
fn inside_is_the_areas_own() {
    let s = Shape::Polygon {
        pts: vec![v(0.0, 0.0), v(100.0, 0.0), v(100.0, 60.0), v(0.0, 60.0)],
        bulges: Some(vec![0.0, 0.4, 0.0, -0.3]),
        holes: Some(vec![Ring {
            pts: vec![v(20.0, 20.0), v(40.0, 20.0), v(40.0, 40.0), v(20.0, 40.0)],
            bulges: None,
        }]),
        parts: None,
    };
    let areas = areas_of_entity(&s);
    let inside = Inside::new(&areas).unwrap();
    let mut near = Vec::new();
    let mut seq = Sequence::new(11);
    for _ in 0..5000 {
        let p = v(-30.0 + seq.unit() * 160.0, -30.0 + seq.unit() * 120.0);
        let want = areas.iter().any(|a| inside_area(a, p));
        assert_eq!(inside.holds(p, &mut near), want, "{p:?}");
    }
}

#[test]
fn regular_points_take_the_boundary() {
    let square = inside_of(&polygon(&[
        (0.0, 0.0),
        (100.0, 0.0),
        (100.0, 100.0),
        (0.0, 100.0),
    ]));
    let on = regular(std::slice::from_ref(&square), 10.0, 10.0, true, 0.0, 1).unwrap();
    assert_eq!(on[0].len(), 121);
    assert_eq!(on[0][0], v(0.0, 100.0));
    let middles = regular(std::slice::from_ref(&square), 10.0, 10.0, false, 0.0, 1).unwrap();
    assert_eq!(middles[0].len(), 100);
    assert_eq!(middles[0][0], v(5.0, 95.0));
    let shifted = regular(std::slice::from_ref(&square), 10.0, 10.0, true, 2.0, 9).unwrap();
    let again = regular(std::slice::from_ref(&square), 10.0, 10.0, true, 2.0, 9).unwrap();
    assert_eq!(shifted, again);
    assert!(
        shifted[0]
            .iter()
            .all(|p| (0.0..=100.0).contains(&p.x) && (0.0..=100.0).contains(&p.y))
    );
}

#[test]
fn distances_follow_the_rule() {
    let interval = |step, start, end| Spacing::Interval { step, start, end };
    assert_eq!(
        distances(100.0, false, &interval(30.0, 0.0, 0.0), false),
        [0.0, 30.0, 60.0, 90.0]
    );
    assert_eq!(
        distances(100.0, false, &interval(30.0, 0.0, 0.0), true),
        [0.0, 30.0, 60.0, 90.0, 100.0]
    );
    assert_eq!(
        distances(90.0, true, &interval(30.0, 0.0, 0.0), true),
        [0.0, 30.0, 60.0]
    );
    assert_eq!(
        distances(100.0, false, &interval(30.0, 5.0, 5.0), false),
        [5.0, 35.0, 65.0, 95.0]
    );
    assert_eq!(
        distances(100.0, false, &interval(30.0, 5.0, 5.0), true),
        [0.0, 5.0, 35.0, 65.0, 95.0, 100.0]
    );
    assert_eq!(
        distances(100.0, false, &Spacing::Parts(4), true),
        [0.0, 25.0, 50.0, 75.0, 100.0]
    );
    assert_eq!(
        distances(100.0, true, &Spacing::Parts(4), true),
        [0.0, 25.0, 50.0, 75.0]
    );
    assert_eq!(
        distances(100.0, false, &Spacing::Parts(4), false),
        [25.0, 50.0, 75.0]
    );
}

#[test]
fn marks_run_along_with_their_bearings() {
    let s = polyline(&[(0.0, 0.0), (0.0, 30.0), (40.0, 30.0)]);
    let zs = vec![vec![Some(10.0), Some(13.0), Some(5.0)]];
    let walks = walks_of(&s, Some(&zs));
    let m = marks(
        &walks,
        &Spacing::Interval {
            step: 20.0,
            start: 0.0,
            end: 0.0,
        },
        true,
    );
    let got: Vec<_> = m.iter().map(|m| (m.p, m.s, m.z)).collect();
    assert_eq!(
        got,
        vec![
            (v(0.0, 0.0), 0.0, Some(10.0)),
            (v(0.0, 20.0), 20.0, Some(12.0)),
            (v(10.0, 30.0), 40.0, Some(11.0)),
            (v(30.0, 30.0), 60.0, Some(7.0)),
            (v(40.0, 30.0), 70.0, Some(5.0)),
        ]
    );
    // North, then east: at the corner the next edge's.
    assert_eq!(m[0].bearing, 0.0);
    assert!((m[2].bearing - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
}

#[test]
fn scattered_points_are_seeded_and_apart() {
    let square = || {
        target_of(
            &polygon(&[(0.0, 0.0), (100.0, 0.0), (100.0, 100.0), (0.0, 100.0)]),
            None,
        )
    };
    let run = || scatter(&[square()], &[60], 8.0, 30, 42);
    let (a, b) = (run(), run());
    assert_eq!(a.dots, b.dots);
    let dots = &a.dots[0];
    assert_eq!(dots.len() + a.refused + a.thin_lost, 60);
    for (i, p) in dots.iter().enumerate() {
        assert!((0.0..=100.0).contains(&p.p.x) && (0.0..=100.0).contains(&p.p.y));
        for q in &dots[i + 1..] {
            let (dx, dy) = (p.p.x - q.p.x, p.p.y - q.p.y);
            assert!(dx * dx + dy * dy >= 64.0);
        }
    }
    let line = target_of(&polyline(&[(0.0, 0.0), (50.0, 0.0), (50.0, 50.0)]), None);
    let on = scatter(&[line], &[25], 0.0, 1, 5);
    for d in &on.dots[0] {
        let s = d.s.unwrap();
        let want = if s <= 50.0 {
            v(s, 0.0)
        } else {
            v(50.0, s - 50.0)
        };
        assert!((d.p.x - want.x).abs() < 1e-9 && (d.p.y - want.y).abs() < 1e-9);
    }
    // A target with nothing to hold points writes none.
    let nothing = scatter(&[Target::Nothing], &[5], 0.0, 1, 5);
    assert!(nothing.dots[0].is_empty());
}

fn star(n: usize, r0: f64, r1: f64) -> Vec<(f64, f64)> {
    (0..2 * n)
        .map(|i| {
            let t = std::f64::consts::PI * i as f64 / n as f64;
            let r = if i % 2 == 0 { r1 } else { r0 };
            (500.0 + r * cos(t), 300.0 + r * sin(t))
        })
        .collect()
}

#[test]
fn subdivided_pieces_keep_the_whole() {
    let pts = star(20, 40.0, 100.0);
    let hole = star(30, 10.0, 18.0);
    let s = Shape::Polygon {
        pts: pts.iter().map(|&(x, y)| v(x, y)).collect(),
        bulges: None,
        holes: Some(vec![Ring {
            pts: hole.iter().map(|&(x, y)| v(x, y)).rev().collect(),
            bulges: None,
        }]),
        parts: None,
    };
    let whole: f64 = areas_of_entity(&s).iter().map(net_area).sum();
    let pieces = subdivide(&s, None, 8);
    assert!(pieces.len() > 10);
    let mut sum = 0.0;
    for p in &pieces {
        let Class::Areas(areas) = class_of(&p.shape) else {
            panic!("an area's piece is an area");
        };
        let n: usize = areas
            .iter()
            .map(|a| a.outer.pts.len() + a.holes.iter().map(|h| h.pts.len()).sum::<usize>())
            .sum();
        assert!(n <= 8, "{n}");
        sum += areas.iter().map(net_area).sum::<f64>();
    }
    assert!((sum - whole).abs() < 1e-7 * whole, "{sum} {whole}");

    let zig: Vec<(f64, f64)> = (0..50)
        .map(|i| (i as f64 * 7.0, if i % 2 == 0 { 0.0 } else { 9.0 }))
        .collect();
    let line = polyline(&zig);
    let length: f64 = entity_edges(&line).iter().map(edge_length).sum();
    let parts = subdivide(&line, None, 6);
    let mut total = 0.0;
    for p in &parts {
        let edges = entity_edges(&p.shape);
        assert!(edges.len() < 6);
        total += edges.iter().map(edge_length).sum::<f64>();
    }
    assert!((total - length).abs() < 1e-9 * length, "{total} {length}");
}

/// One cut of a 10 000-vertex star by half its box, timed (the subdivision's unit of work). Run on purpose:
/// `cargo test --release -p kentos-geometry-core --lib sampling::tests::cut_timing -- --ignored --nocapture`.
#[test]
#[ignore = "timings, run by hand in release"]
fn cut_timing() {
    use crate::ops::geoprocess::intersect_all;
    let pts: Vec<Vec2> = (0..10_000usize)
        .map(|i| {
            let t = std::f64::consts::PI * i as f64 / 5000.0;
            let r = if i % 2 == 0 {
                400.0
            } else {
                300.0 + (i % 7) as f64 * 10.0
            };
            v(487_000.0 + r * cos(t), 4_420_000.0 + r * sin(t))
        })
        .collect();
    let star = Area {
        outer: Ring { pts, bulges: None },
        holes: Vec::new(),
    };
    let half = Area {
        outer: Ring {
            pts: vec![
                v(486_500.0, 4_419_500.0),
                v(487_000.0, 4_419_500.0),
                v(487_000.0, 4_420_500.0),
                v(486_500.0, 4_420_500.0),
            ],
            bulges: None,
        },
        holes: Vec::new(),
    };
    let t = std::time::Instant::now();
    let got = intersect_all(std::slice::from_ref(&star), std::slice::from_ref(&half));
    println!(
        "one cut by the overlay: {:.1} ms, {} areas",
        t.elapsed().as_secs_f64() * 1e3,
        got.len()
    );
    let t = std::time::Instant::now();
    let [low, _] = super::cut::cut_area(
        &star,
        super::cut::Line {
            across_x: true,
            at: 487_000.0,
        },
    );
    println!(
        "one cut along the line: {:.1} ms, {} areas",
        t.elapsed().as_secs_f64() * 1e3,
        low.len()
    );
    let t = std::time::Instant::now();
    let pieces = subdivide(
        &Shape::Polygon {
            pts: star.outer.pts.clone(),
            bulges: None,
            holes: None,
            parts: None,
        },
        None,
        256,
    );
    println!(
        "whole star: {:.1} ms, {} pieces",
        t.elapsed().as_secs_f64() * 1e3,
        pieces.len()
    );
}

/// The areas' vertices, sorted, for comparing cuts.
fn corners(areas: &[Area]) -> Vec<(f64, f64)> {
    let mut out: Vec<(f64, f64)> = areas
        .iter()
        .flat_map(|a| std::iter::once(&a.outer).chain(&a.holes))
        .flat_map(|r| r.pts.iter().map(|p| (p.x, p.y)))
        .collect();
    // By the micrometre: the overlay's cut points are within a few units of the last place off the line.
    let k = |v: f64| crate::jsmath::js_round(v * 1e6);
    out.sort_by(|a, b| k(a.0).total_cmp(&k(b.0)).then(k(a.1).total_cmp(&k(b.1))));
    out
}

#[test]
fn cuts_along_a_line_are_the_overlays() {
    use super::cut::{Line, cut_area};
    use crate::ops::geoprocess::intersect_all;
    let star = |n: usize, r0: f64, r1: f64, cx: f64, cy: f64| -> Vec<Vec2> {
        (0..2 * n)
            .map(|i| {
                let t = std::f64::consts::PI * i as f64 / n as f64;
                let r = if i % 2 == 0 { r1 } else { r0 };
                v(cx + r * cos(t), cy + r * sin(t))
            })
            .collect()
    };
    let rect =
        |x0: f64, y0: f64, x1: f64, y1: f64| vec![v(x0, y0), v(x1, y0), v(x1, y1), v(x0, y1)];
    let areas = vec![
        Area {
            outer: Ring {
                pts: star(12, 30.0, 70.0, 0.0, 0.0),
                bulges: None,
            },
            holes: vec![Ring {
                pts: star(9, 6.0, 14.0, 3.0, -2.0).into_iter().rev().collect(),
                bulges: None,
            }],
        },
        // A block of rectangles: edges along the cutting line, a notch, a hole on it.
        Area {
            outer: Ring {
                pts: vec![
                    v(0.0, 0.0),
                    v(40.0, 0.0),
                    v(40.0, 30.0),
                    v(25.0, 30.0),
                    v(25.0, 18.0),
                    v(15.0, 18.0),
                    v(15.0, 30.0),
                    v(0.0, 30.0),
                ],
                bulges: None,
            },
            holes: vec![Ring {
                pts: rect(5.0, 5.0, 25.0, 12.0).into_iter().rev().collect(),
                bulges: None,
            }],
        },
        Area {
            outer: Ring {
                pts: rect(0.0, 0.0, 100.0, 60.0),
                bulges: Some(vec![0.0, 0.45, 0.0, -0.3]),
            },
            holes: Vec::new(),
        },
    ];
    for a in &areas {
        let b = {
            let p = &a.outer.pts;
            let mut b = bx(p[0].x, p[0].y, p[0].x, p[0].y);
            for e in crate::geom::region::ring_edges(&a.outer) {
                let t = crate::geom::arrangement::tight_box(&e);
                b = bx(
                    js_min(b.min_x, t.min_x),
                    js_min(b.min_y, t.min_y),
                    js_max(b.max_x, t.max_x),
                    js_max(b.max_y, t.max_y),
                );
            }
            b
        };
        let mut lines: Vec<(bool, f64)> = Vec::new();
        for p in &a.outer.pts {
            lines.push((true, p.x));
            lines.push((false, p.y));
        }
        lines.push((true, (b.min_x + b.max_x) / 2.0 + 0.123));
        lines.push((false, (b.min_y + b.max_y) / 2.0 - 0.321));
        for (across_x, at) in lines {
            let (lo, hi) = if across_x {
                (b.min_x, b.max_x)
            } else {
                (b.min_y, b.max_y)
            };
            if at <= lo || at >= hi {
                continue;
            }
            let [low, high] = cut_area(a, Line { across_x, at });
            let pad = 10.0;
            for (got, is_low) in [(&low, true), (&high, false)] {
                let half = if across_x {
                    if is_low {
                        bx(b.min_x - pad, b.min_y - pad, at, b.max_y + pad)
                    } else {
                        bx(at, b.min_y - pad, b.max_x + pad, b.max_y + pad)
                    }
                } else if is_low {
                    bx(b.min_x - pad, b.min_y - pad, b.max_x + pad, at)
                } else {
                    bx(b.min_x - pad, at, b.max_x + pad, b.max_y + pad)
                };
                let boxed = Area {
                    outer: Ring {
                        pts: rect(half.min_x, half.min_y, half.max_x, half.max_y),
                        bulges: None,
                    },
                    holes: Vec::new(),
                };
                let want = intersect_all(std::slice::from_ref(a), std::slice::from_ref(&boxed));
                let (sa, sb): (f64, f64) = (
                    got.iter().map(net_area).sum(),
                    want.iter().map(net_area).sum(),
                );
                assert!(
                    (sa - sb).abs() < 1e-9 * js_max(sb, 1.0),
                    "{across_x} {at}: {sa} {sb}\nbox {b:?}\ngot {got:?}\nwant {want:?}\narea {a:?}"
                );
                assert_eq!(got.len(), want.len(), "{across_x} {at}");
                // Straight rings: the overlay's vertices exactly (its box corners off the area dropped).
                if a.outer.bulges.is_none() {
                    let (cg, cw) = (corners(got), corners(&want));
                    assert_eq!(
                        cg.len(),
                        cw.len(),
                        "{across_x} {at} {is_low}\n{cg:?}\n{cw:?}"
                    );
                    for (p, q) in cg.iter().zip(&cw) {
                        assert!(
                            (p.0 - q.0).abs() < 1e-9 && (p.1 - q.1).abs() < 1e-9,
                            "{across_x} {at}: {p:?} {q:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn a_line_touching_the_area_at_its_vertices_leaves_it_whole() {
    use super::cut::{Line, cut_area};
    // The west side is an arc bending into the area between two vertices on x = 0.
    let a = Area {
        outer: Ring {
            pts: vec![v(0.0, 0.0), v(100.0, 0.0), v(100.0, 60.0), v(0.0, 60.0)],
            bulges: Some(vec![0.0, 0.45, 0.0, -0.3]),
        },
        holes: Vec::new(),
    };
    let [low, high] = cut_area(
        &a,
        Line {
            across_x: true,
            at: 0.0,
        },
    );
    assert!(low.is_empty());
    assert_eq!(high.len(), 1);
    assert!((net_area(&high[0]) - net_area(&a)).abs() < 1e-9 * net_area(&a));
}
