//! The generation core against brute force: cells by every other site's
//! bisector, hulls by every edge's side, boxes by every hull edge, circles by
//! every pair and triple.

use super::bounding::{Bound, Kind, bound, convex_hull, min_area_box, min_circle};
use super::concave::concave_hull;
use super::points::outline_points;
use super::voronoi::{Cell, Frame, delaunay_triangles, frame_of, normalised, voronoi};
use crate::entity::Shape;
use crate::jsmath::js_hypot;
use crate::predicates::orient2d;
use crate::vec2::Vec2;

/// A fixed sequence of numbers in [0, 1).
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
}

fn sites(n: usize, seed: u64, x0: f64, y0: f64) -> Vec<Vec2> {
    let mut r = Lcg(seed);
    (0..n)
        .map(|_| Vec2::new(x0 + 100.0 * r.next(), y0 + 70.0 * r.next()))
        .collect()
}

fn cut(poly: &[Vec2], p: Vec2, q: Vec2) -> Vec<Vec2> {
    let (mx, my) = ((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
    let (nx, ny) = (q.x - p.x, q.y - p.y);
    let side = |v: Vec2| (v.x - mx) * nx + (v.y - my) * ny;
    let n = poly.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        let (sa, sb) = (side(a), side(b));
        if sa <= 0.0 {
            out.push(a);
        }
        if (sa < 0.0 && sb > 0.0) || (sa > 0.0 && sb < 0.0) {
            let t = sa / (sa - sb);
            out.push(Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        }
    }
    out
}

/// A cell by every other site's bisector.
fn brute_cell(pts: &[Vec2], i: usize, f: Frame) -> Vec<Vec2> {
    let mut ring = vec![
        f.min,
        Vec2::new(f.max.x, f.min.y),
        f.max,
        Vec2::new(f.min.x, f.max.y),
    ];
    for (j, &q) in pts.iter().enumerate() {
        if j != i {
            ring = cut(&ring, pts[i], q);
        }
    }
    // Corners closer than a nanometre are one (a cut through a corner leaves two).
    let mut out: Vec<Vec2> = Vec::new();
    for p in ring {
        if out
            .last()
            .is_none_or(|l| js_hypot(l.x - p.x, l.y - p.y) > 1e-9)
        {
            out.push(p);
        }
    }
    while out.len() > 1
        && js_hypot(
            out[0].x - out[out.len() - 1].x,
            out[0].y - out[out.len() - 1].y,
        ) <= 1e-9
    {
        out.pop();
    }
    normalised(out)
}

fn same_ring(a: &[Vec2], b: &[Vec2], tol: f64) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(p, q)| js_hypot(p.x - q.x, p.y - q.y) <= tol)
}

#[test]
fn every_cell_is_its_sites_bisectors_cut() {
    for (seed, n, x0) in [
        (7, 40, 0.0),
        (11, 120, 487_000.0),
        (19, 3, 0.0),
        (23, 2, 4_420_000.0),
    ] {
        let pts = sites(n, seed, x0, x0 / 10.0);
        let f = frame_of(&pts, 0.1).unwrap();
        let cells = voronoi(&pts, f).unwrap();
        for (i, c) in cells.iter().enumerate() {
            let Cell::Ring(r) = c else {
                panic!("{seed}: site {i} has no cell: {c:?}");
            };
            let want = brute_cell(&pts, i, f);
            assert!(
                same_ring(r, &want, 1e-7),
                "{seed}: site {i}\n{r:?}\n{want:?}"
            );
        }
    }
}

#[test]
fn neighbouring_cells_share_their_corners_bit_for_bit() {
    let pts = sites(200, 3, 487_000.0, 4_420_000.0);
    let cells = voronoi(&pts, frame_of(&pts, 0.05).unwrap()).unwrap();
    let corners: std::collections::HashMap<(u64, u64), usize> = cells
        .iter()
        .flat_map(|c| match c {
            Cell::Ring(r) => r.clone(),
            _ => Vec::new(),
        })
        .fold(std::collections::HashMap::new(), |mut m, p| {
            *m.entry((p.x.to_bits(), p.y.to_bits())).or_insert(0) += 1;
            m
        });
    // A corner inside the frame belongs to three cells; one on the frame's side to two; a frame corner to one.
    let lone = corners.values().filter(|&&k| k == 1).count();
    assert_eq!(lone, 4, "only the frame's corners stand alone");
}

#[test]
fn a_grid_shares_its_cocircular_centres() {
    let pts: Vec<Vec2> = (0..5)
        .flat_map(|i| (0..4).map(move |j| Vec2::new(i as f64 * 10.0, j as f64 * 10.0)))
        .collect();
    let f = frame_of(&pts, 0.1).unwrap();
    let cells = voronoi(&pts, f).unwrap();
    for (i, c) in cells.iter().enumerate() {
        let Cell::Ring(r) = c else { panic!("{i}") };
        let want = brute_cell(&pts, i, f);
        assert!(same_ring(r, &want, 1e-9), "site {i}\n{r:?}\n{want:?}");
    }
}

#[test]
fn repeated_and_lined_up_sites() {
    let pts = vec![
        Vec2::new(0.0, 0.0),
        Vec2::new(10.0, 0.0),
        Vec2::new(0.0, 0.0),
        Vec2::new(20.0, 0.0),
    ];
    let f = Frame {
        min: Vec2::new(-5.0, -5.0),
        max: Vec2::new(25.0, 5.0),
    };
    let cells = voronoi(&pts, f).unwrap();
    assert_eq!(cells[2], Cell::Same);
    assert_eq!(
        cells[0],
        Cell::Ring(vec![
            Vec2::new(-5.0, -5.0),
            Vec2::new(5.0, -5.0),
            Vec2::new(5.0, 5.0),
            Vec2::new(-5.0, 5.0)
        ])
    );
    assert!(voronoi(&[Vec2::new(1.0, 1.0), Vec2::new(1.0, 1.0)], f).is_err());
    // A frame with no height leaves no area.
    let flat = Frame {
        min: Vec2::new(-5.0, 0.0),
        max: Vec2::new(25.0, 0.0),
    };
    assert!(
        voronoi(&pts, flat)
            .unwrap()
            .iter()
            .all(|c| matches!(c, Cell::Empty | Cell::Same))
    );
}

#[test]
fn triangles_are_counter_clockwise_from_their_first_site() {
    let pts = sites(60, 5, 0.0, 0.0);
    let tris = delaunay_triangles(&pts).unwrap();
    let hull = convex_hull(&pts).len();
    assert_eq!(tris.len(), 2 * pts.len() - hull - 2);
    for t in &tris {
        assert!(t[0] < t[1] && t[0] < t[2]);
        assert!(orient2d(pts[t[0]], pts[t[1]], pts[t[2]]) > 0.0);
    }
    assert!(tris.windows(2).all(|w| w[0] < w[1]));
    assert!(
        delaunay_triangles(&[
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(2.0, 2.0)
        ])
        .is_err()
    );
}

#[test]
fn hulls_leave_every_point_on_their_left() {
    for seed in [1, 2, 3] {
        let pts = sites(80, seed, 487_000.0, 4_420_000.0);
        let h = convex_hull(&pts);
        let n = h.len();
        for i in 0..n {
            let (a, b) = (h[i], h[(i + 1) % n]);
            assert!(pts.iter().all(|&p| orient2d(a, b, p) >= 0.0));
            assert!(orient2d(a, b, h[(i + 2) % n]) > 0.0, "no point on an edge");
        }
        let low = pts
            .iter()
            .copied()
            .min_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)))
            .unwrap();
        assert_eq!(h[0], low);
    }
    assert_eq!(
        convex_hull(&[
            Vec2::new(0.0, 0.0),
            Vec2::new(1.0, 1.0),
            Vec2::new(3.0, 3.0)
        ])
        .len(),
        2
    );
}

/// The least box over every hull edge, each looked at in full.
fn brute_box(h: &[Vec2]) -> f64 {
    let n = h.len();
    let mut best = f64::INFINITY;
    for i in 0..n {
        let (a, b) = (h[i], h[(i + 1) % n]);
        let l = js_hypot(b.x - a.x, b.y - a.y);
        let u = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
        let (mut s0, mut s1, mut t1) = (f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);
        for p in h {
            let (dx, dy) = (p.x - a.x, p.y - a.y);
            let (s, t) = (dx * u.x + dy * u.y, dy * u.x - dx * u.y);
            s0 = if s < s0 { s } else { s0 };
            s1 = if s > s1 { s } else { s1 };
            t1 = if t > t1 { t } else { t1 };
        }
        let area = (s1 - s0) * t1;
        if area < best {
            best = area;
        }
    }
    best
}

#[test]
fn the_rotated_box_is_the_least_of_every_edge() {
    for seed in [4, 8, 15, 16, 23, 42] {
        let pts = sites(30, seed, 487_000.0, 4_420_000.0);
        let h = convex_hull(&pts);
        let Some(Bound::Rotated {
            ring,
            length,
            width,
            bearing,
        }) = min_area_box(&h)
        else {
            panic!("{seed}")
        };
        assert!(
            (length * width - brute_box(&h)).abs() <= 1e-6 * length * width,
            "{seed}"
        );
        assert!(length >= width && (0.0..std::f64::consts::PI).contains(&bearing));
        // Every point inside the box (with its rounding).
        for p in &pts {
            for i in 0..4 {
                let (a, b) = (ring[i], ring[(i + 1) % 4]);
                let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
                assert!(cross >= -1e-6, "{seed}: {p:?} outside");
            }
        }
    }
    // A square: its own box.
    let sq = [
        Vec2::new(0.0, 0.0),
        Vec2::new(4.0, 0.0),
        Vec2::new(4.0, 4.0),
        Vec2::new(0.0, 4.0),
    ];
    let Some(Bound::Rotated {
        length,
        width,
        bearing,
        ..
    }) = min_area_box(&sq)
    else {
        panic!()
    };
    assert_eq!((length, width), (4.0, 4.0));
    assert!((bearing - std::f64::consts::FRAC_PI_2).abs() < 1e-15);
}

/// The least circle among every pair's and triple's that holds every point.
fn brute_circle(pts: &[Vec2]) -> (Vec2, f64) {
    let holds = |c: Vec2, r: f64| {
        pts.iter()
            .all(|p| js_hypot(p.x - c.x, p.y - c.y) <= r * (1.0 + 1e-9) + 1e-9)
    };
    let mut best = (Vec2::default(), f64::INFINITY);
    let n = pts.len();
    for i in 0..n {
        for j in i + 1..n {
            let c = Vec2::new((pts[i].x + pts[j].x) / 2.0, (pts[i].y + pts[j].y) / 2.0);
            let r = js_hypot(pts[i].x - c.x, pts[i].y - c.y);
            if r < best.1 && holds(c, r) {
                best = (c, r);
            }
            for k in j + 1..n {
                let t = orient2d(pts[i], pts[j], pts[k]);
                if t == 0.0 {
                    continue;
                }
                let c = if t > 0.0 {
                    super::voronoi::centre(pts[i], pts[j], pts[k])
                } else {
                    super::voronoi::centre(pts[i], pts[k], pts[j])
                };
                let r = js_hypot(pts[i].x - c.x, pts[i].y - c.y);
                if r < best.1 && holds(c, r) {
                    best = (c, r);
                }
            }
        }
    }
    best
}

#[test]
fn the_circle_is_the_least_that_holds_them() {
    for seed in [6, 7, 8, 9] {
        let pts = sites(18, seed, 487_000.0, 4_420_000.0);
        let (c, r) = min_circle(&pts).unwrap();
        let (bc, br) = brute_circle(&pts);
        assert!(
            (r - br).abs() <= 1e-8 && js_hypot(c.x - bc.x, c.y - bc.y) <= 1e-7,
            "{seed}: {c:?} {r} / {bc:?} {br}"
        );
    }
    assert_eq!(
        min_circle(&[Vec2::new(3.0, 4.0)]),
        Some((Vec2::new(3.0, 4.0), 0.0))
    );
}

#[test]
fn a_circle_bounds_itself() {
    let c = Shape::Circle {
        c: Vec2::new(487_100.0, 4_420_050.0),
        r: 12.5,
    };
    let pts = outline_points(&c);
    let Some(Bound::Box { width, height, .. }) = bound(&pts, Kind::Box) else {
        panic!()
    };
    assert_eq!((width, height), (25.0, 25.0), "its extremes are exact");
    let Some(Bound::Circle { c: centre, r }) = bound(&pts, Kind::Circle) else {
        panic!()
    };
    assert!(
        (r - 12.5).abs() < 1e-9 && js_hypot(centre.x - 487_100.0, centre.y - 4_420_050.0) < 1e-9
    );
}

#[test]
fn the_concave_hull_wears_down_to_the_ratio() {
    // A C: a 3 × 3 grid with its middle column's top two cells empty, points every 5 m.
    let mut pts = Vec::new();
    for i in 0..=6 {
        for j in 0..=6 {
            let (x, y) = (i as f64 * 5.0, j as f64 * 5.0);
            if (2..=4).contains(&i) && j >= 2 {
                continue;
            }
            pts.push(Vec2::new(x, y));
        }
    }
    let area = |r: &[Vec2]| super::ring_area(r);
    let convex = concave_hull(&pts, 1.0, false).unwrap();
    assert_eq!(convex.len(), 1);
    assert_eq!(area(&convex[0]), area(&convex_hull(&pts)));
    let worn = concave_hull(&pts, 0.0, false).unwrap();
    assert!(area(&worn[0]) < area(&convex[0]));
    assert!(area(&worn[0]) > 0.0);
    assert!(
        concave_hull(
            &[
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
                Vec2::new(2.0, 0.0)
            ],
            0.5,
            false
        )
        .is_none()
    );
}

#[test]
fn a_pole_is_inside_and_a_circles_is_its_centre() {
    let sq = Shape::Polygon {
        pts: vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(40.0, 0.0),
            Vec2::new(40.0, 10.0),
            Vec2::new(0.0, 10.0),
        ],
        bulges: None,
        holes: None,
        parts: None,
    };
    let (p, d) = super::pole::pole(&sq, 0.01).unwrap();
    assert!((d - 5.0).abs() <= 0.01 && p.y > 4.9 && p.y < 5.1);
    let c = Shape::Circle {
        c: Vec2::new(3.0, 4.0),
        r: 2.0,
    };
    assert_eq!(super::pole::pole(&c, 0.1), Some((Vec2::new(3.0, 4.0), 2.0)));
}

/// Where Sınırlayıcı geometri's time goes on 100 000 parcels (run by hand in release).
#[test]
#[ignore = "timings, run by hand in release"]
fn bounding_parts_timing() {
    use std::time::Instant;
    let shapes: Vec<Shape> = (0..100_000usize)
        .map(|k| {
            let (x, y) = (
                487_000.0 + (k % 316) as f64 * 20.0,
                4_420_000.0 + (k / 316) as f64 * 20.0,
            );
            let s = 10.0 + (k % 7) as f64;
            Shape::Polygon {
                pts: vec![
                    Vec2::new(x, y),
                    Vec2::new(x + s, y),
                    Vec2::new(x + s, y + s),
                    Vec2::new(x, y + s),
                ],
                bulges: None,
                holes: None,
                parts: None,
            }
        })
        .collect();
    let t = Instant::now();
    let pts: Vec<Vec<Vec2>> = shapes.iter().map(outline_points).collect();
    println!("noktalar {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
    let t = Instant::now();
    let hulls: Vec<Vec<Vec2>> = pts.iter().map(|p| convex_hull(p)).collect();
    println!("zarflar {:.1} ms", t.elapsed().as_secs_f64() * 1e3);
    let t = Instant::now();
    let boxes: Vec<_> = hulls.iter().map(|h| min_area_box(h)).collect();
    println!(
        "kutular {:.1} ms ({})",
        t.elapsed().as_secs_f64() * 1e3,
        boxes.len()
    );
    let t = Instant::now();
    let shaped: Vec<_> = boxes.iter().flatten().map(super::bound_shape).collect();
    println!(
        "şekiller {:.1} ms ({})",
        t.elapsed().as_secs_f64() * 1e3,
        shaped.len()
    );
}
