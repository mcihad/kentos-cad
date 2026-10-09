//! Areas onto a grid's cells (docs/adr/0233 §6): which cells' centres an
//! area holds, a row at a time, for Maskeyle kırp and Bölgesel istatistik.
//!
//! Every ring of an area (its own, its holes', its parts') is taken into the
//! grid's cell space (u along a row, v down the rows; §2's formula), where
//! row j's centres lie on v = j + ½ at u = i + ½. An edge crosses the row
//! by the half-open rule (one end above it, v > j + ½, the other on it or
//! below), and the centres at or past the crossing get one more crossing
//! to their left; a centre is inside when it has an odd number. A straight
//! edge is decided exactly at the centres (the geometry core's exact
//! orientation on the edge's float64 ends); an arc (a bulged edge, a
//! circle) is cut into pieces that only rise, its crossing worked out in
//! float64. On a grid whose cells are not squares (a skewed or stretched
//! affine) arcs are followed by chords within 0.1 mm, as ellipses and
//! curves always are (ADR 0149).

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::geom::bulge::{bulge_at, is_arc_bulge};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::predicates::orient2d;
use kentos_geometry_core::vec2::Vec2;

use kentos_formats::raster::TILE;

use crate::grid::Grid;
use crate::inputs::place_in;

/// The chords' largest distance from an arc they follow (metres).
const CHORD_TOLERANCE: f64 = 1e-4;

/// An edge in cell space that only rises: `lo` its lower end, `hi` its upper (lo.y < hi.y).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Piece {
    Seg {
        lo: Vec2,
        hi: Vec2,
    },
    /// A piece of a circle (centre `c`, radius `r`) on one side of it:
    /// its crossing is at u = c.x + √(r² − (v − c.y)²) when `right`, else −.
    Arc {
        c: Vec2,
        r: f64,
        lo: Vec2,
        hi: Vec2,
        right: bool,
    },
}

impl Piece {
    fn ends(&self) -> (Vec2, Vec2) {
        match *self {
            Piece::Seg { lo, hi } | Piece::Arc { lo, hi, .. } => (lo, hi),
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Placed {
    object: u32,
    piece: Piece,
    /// The rows it may cross.
    first: u32,
    last: u32,
}

/// A row's cells inside an object: columns `i0..i1`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub object: u32,
    pub i0: u32,
    pub i1: u32,
}

/// Areas taken onto a grid.
#[derive(Clone, Debug)]
pub struct Areas {
    pieces: Vec<Placed>,
    width: u32,
    height: u32,
    objects: usize,
}

/// The grid's cells are squares (turned or mirrored): circles stay circles in cell space.
/// The sign of its determinant (a mirror turns an arc's way), and the cell's side.
fn similarity(affine: &[f64; 6]) -> Option<(f64, f64)> {
    let [_, a, b, _, c, d] = *affine;
    let (p, q) = (a * a + c * c, b * b + d * d);
    let det = a * d - b * c;
    let tol = 1e-12 * p.max(q);
    ((p - q).abs() <= tol && (a * b + c * d).abs() <= tol && det != 0.0)
        .then(|| (det.signum(), det.abs().sqrt()))
}

/// Points along an arc (centre `c`, radius `r`, from angle `a0` by `sweep`)
/// whose chords stay within the tolerance; the ends left to the caller.
pub(crate) fn arc_inner_points(c: Vec2, r: f64, a0: f64, sweep: f64) -> Vec<Vec2> {
    let step = if r > CHORD_TOLERANCE {
        2.0 * libm::acos(1.0 - CHORD_TOLERANCE / r)
    } else {
        std::f64::consts::PI
    };
    let n = ((sweep.abs() / step).ceil() as usize).clamp(1, 1 << 16);
    (1..n)
        .map(|k| {
            let t = a0 + sweep * k as f64 / n as f64;
            Vec2::new(c.x + r * libm::cos(t), c.y + r * libm::sin(t))
        })
        .collect()
}

/// The bulged arc from `a` to `b` in cell space (KentOS's rule: the centre
/// on the chord's left normal at chord·(1 − b²)/(4b)), cut where it turns
/// in v: its pieces.
fn arc_pieces(a: Vec2, b: Vec2, bulge: f64, out: &mut Vec<Piece>) {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    if dx == 0.0 && dy == 0.0 {
        return;
    }
    let k = (1.0 - bulge * bulge) / (4.0 * bulge);
    let c = Vec2::new((a.x + b.x) / 2.0 - dy * k, (a.y + b.y) / 2.0 + dx * k);
    let r = (dx * dx + dy * dy).sqrt() * (1.0 + bulge * bulge) / (4.0 * bulge.abs());
    // Counter-clockwise from s to e.
    let (s, e) = if bulge > 0.0 { (a, b) } else { (b, a) };
    let sweep = 4.0 * libm::atan(bulge.abs());
    let a_s = libm::atan2(s.y - c.y, s.x - c.x);
    circle_pieces(c, r, Some((s, a_s, sweep, e)), out);
}

/// A circle's pieces in cell space, whole or from `s` (at angle `a_s`)
/// counter-clockwise by `sweep` to `e`.
fn circle_pieces(c: Vec2, r: f64, arc: Option<(Vec2, f64, f64, Vec2)>, out: &mut Vec<Piece>) {
    use std::f64::consts::{FRAC_PI_2, PI};
    if !(r > 0.0) || !r.is_finite() {
        return;
    }
    let bottom = Vec2::new(c.x, c.y - r);
    let top = Vec2::new(c.x, c.y + r);
    let Some((s, a_s, sweep, e)) = arc else {
        out.push(Piece::Arc {
            c,
            r,
            lo: bottom,
            hi: top,
            right: true,
        });
        out.push(Piece::Arc {
            c,
            r,
            lo: bottom,
            hi: top,
            right: false,
        });
        return;
    };
    // The turning angles −π/2 + kπ inside (a_s, a_s + sweep), and the points between them.
    let end = a_s + sweep;
    let mut k = ((a_s + FRAC_PI_2) / PI).floor() + 1.0;
    let mut stops: Vec<(f64, Vec2)> = vec![(a_s, s)];
    loop {
        let t = -FRAC_PI_2 + k * PI;
        if t >= end {
            break;
        }
        // Even k: the bottom (−π/2 + 2mπ); odd k: the top.
        let p = if (k as i64).rem_euclid(2) == 0 {
            bottom
        } else {
            top
        };
        stops.push((t, p));
        k += 1.0;
    }
    stops.push((end, e));
    for w in stops.windows(2) {
        let ((t0, p0), (t1, p1)) = (w[0], w[1]);
        if p0.y == p1.y {
            continue;
        }
        let right = libm::cos((t0 + t1) / 2.0) > 0.0;
        let (lo, hi) = if p0.y < p1.y { (p0, p1) } else { (p1, p0) };
        out.push(Piece::Arc {
            c,
            r,
            lo,
            hi,
            right,
        });
    }
}

fn seg_piece(a: Vec2, b: Vec2, out: &mut Vec<Piece>) {
    match a.y.partial_cmp(&b.y) {
        Some(std::cmp::Ordering::Less) => out.push(Piece::Seg { lo: a, hi: b }),
        Some(std::cmp::Ordering::Greater) => out.push(Piece::Seg { lo: b, hi: a }),
        // Level, or not a number: it never crosses a row.
        _ => {}
    }
}

impl Areas {
    /// The areas `shapes` (one object each; an open curve or a shape that
    /// is not an area takes no part) on `grid`.
    pub fn new(grid: &Grid, shapes: &[Shape]) -> Areas {
        let affine = grid.affine;
        let cell = |p: Vec2| {
            let (u, v) = place_in(&affine, p.x, p.y);
            Vec2::new(u, v)
        };
        let similar = similarity(&affine);
        let mut pieces = Vec::new();
        let mut buf = Vec::new();
        // A ring's edges, bulged ones as arcs (or their chords).
        let ring = |pts: &[Vec2], bulges: Option<&[f64]>, buf: &mut Vec<Piece>| {
            let n = pts.len();
            if n < 2 {
                return;
            }
            for i in 0..n {
                let (a, b) = (pts[i], pts[(i + 1) % n]);
                let bulge = bulge_at(bulges, i);
                if !is_arc_bulge(bulge) {
                    seg_piece(cell(a), cell(b), buf);
                    continue;
                }
                match similar {
                    Some((sign, _)) => arc_pieces(cell(a), cell(b), sign * bulge, buf),
                    None => {
                        let Some(arc) = kentos_geometry_core::geom::bulge::bulge_arc(a, b, bulge)
                        else {
                            seg_piece(cell(a), cell(b), buf);
                            continue;
                        };
                        let mut prev = cell(a);
                        for p in arc_inner_points(arc.c, arc.r, arc.a0, arc.sweep) {
                            let q = cell(p);
                            seg_piece(prev, q, buf);
                            prev = q;
                        }
                        seg_piece(prev, cell(b), buf);
                    }
                }
            }
        };
        let rings =
            |own: (&[Vec2], Option<&[f64]>), holes: Option<&Vec<Ring>>, buf: &mut Vec<Piece>| {
                ring(own.0, own.1, buf);
                for h in holes.into_iter().flatten() {
                    ring(&h.pts, h.bulges.as_deref(), buf);
                }
            };
        for (o, shape) in shapes.iter().enumerate() {
            buf.clear();
            match shape {
                Shape::Polygon {
                    pts,
                    bulges,
                    holes,
                    parts,
                } => {
                    rings((pts, bulges.as_deref()), holes.as_ref(), &mut buf);
                    for p in parts.iter().flatten() {
                        rings((&p.pts, p.bulges.as_deref()), p.holes.as_ref(), &mut buf);
                    }
                }
                Shape::Circle { c, r } => match similar {
                    Some((_, side)) => circle_pieces(cell(*c), r / side, None, &mut buf),
                    None => {
                        let first = Vec2::new(c.x + r, c.y);
                        let mut prev = cell(first);
                        for p in arc_inner_points(*c, *r, 0.0, std::f64::consts::TAU) {
                            let q = cell(p);
                            seg_piece(prev, q, &mut buf);
                            prev = q;
                        }
                        seg_piece(prev, cell(first), &mut buf);
                    }
                },
                Shape::Spline { closed: false, .. } => {}
                Shape::Ellipse { .. } | Shape::Spline { .. } | Shape::Hatch { .. } => {
                    for e in entity_edges(shape) {
                        match e {
                            Edge::Seg { a, b } => seg_piece(cell(a), cell(b), &mut buf),
                            Edge::Arc { c, r, a0, sweep } => {
                                let at = |t: f64| {
                                    Vec2::new(c.x + r * libm::cos(t), c.y + r * libm::sin(t))
                                };
                                let mut prev = cell(at(a0));
                                for p in arc_inner_points(c, r, a0, sweep) {
                                    let q = cell(p);
                                    seg_piece(prev, q, &mut buf);
                                    prev = q;
                                }
                                seg_piece(prev, cell(at(a0 + sweep)), &mut buf);
                            }
                        }
                    }
                }
                _ => {}
            }
            for &piece in &buf {
                let (lo, hi) = piece.ends();
                // Rows j with lo.y ≤ j + ½ < hi.y.
                let first = (lo.y - 0.5).ceil();
                let last = (hi.y - 0.5).ceil() - 1.0;
                if !(first.is_finite() && last.is_finite())
                    || last < 0.0
                    || first > f64::from(grid.height) - 1.0
                    || first > last
                {
                    continue;
                }
                pieces.push(Placed {
                    object: o as u32,
                    piece,
                    first: first.max(0.0) as u32,
                    last: last.min(f64::from(grid.height) - 1.0) as u32,
                });
            }
        }
        pieces.sort_by_key(|p| p.first);
        Areas {
            pieces,
            width: grid.width,
            height: grid.height,
            objects: shapes.len(),
        }
    }

    pub fn objects(&self) -> usize {
        self.objects
    }

    /// No edge reaches the grid.
    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    /// The pieces rows `j0..j1` may cross (their indices).
    pub fn strip(&self, j0: u32, j1: u32) -> Vec<u32> {
        let end = self.pieces.partition_point(|p| p.first < j1);
        (0..end)
            .filter(|&k| self.pieces[k].last >= j0)
            .map(|k| k as u32)
            .collect()
    }

    /// Row `j`'s cells inside each object, by object and column, into
    /// `out`; `pieces` are the strip's ([`Areas::strip`]).
    pub fn row(&self, j: u32, pieces: &[u32], out: &mut Vec<Span>, cuts: &mut Vec<(u32, i64)>) {
        out.clear();
        cuts.clear();
        let vr = f64::from(j) + 0.5;
        let w = i64::from(self.width);
        for &k in pieces {
            let p = &self.pieces[k as usize];
            if j < p.first || j > p.last {
                continue;
            }
            let (lo, hi) = p.piece.ends();
            if !(lo.y <= vr && vr < hi.y) {
                continue;
            }
            let cut = match p.piece {
                Piece::Seg { lo, hi } => {
                    let ux = lo.x + (hi.x - lo.x) * ((vr - lo.y) / (hi.y - lo.y));
                    first_right(lo, hi, ux, vr, w)
                }
                Piece::Arc { c, r, right, .. } => {
                    let dv = vr - c.y;
                    let h = (r * r - dv * dv).max(0.0).sqrt();
                    let ux = if right { c.x + h } else { c.x - h };
                    clamp_cut((ux - 0.5).ceil(), w)
                }
            };
            cuts.push((p.object, cut));
        }
        cuts.sort_unstable();
        let mut at = 0;
        while at < cuts.len() {
            let o = cuts[at].0;
            let mut e = at;
            while e < cuts.len() && cuts[e].0 == o {
                e += 1;
            }
            // Pairs: inside from the first crossing to the second, and so on.
            for pair in cuts[at..e].chunks_exact(2) {
                let (i0, i1) = (pair[0].1, pair[1].1);
                if i0 < i1 {
                    out.push(Span {
                        object: o,
                        i0: i0 as u32,
                        i1: i1 as u32,
                    });
                }
            }
            at = e;
        }
    }

    /// Every row's spans, then the box of the cells inside any object:
    /// columns `i0..i1`, rows `j0..j1`; none when no centre is inside.
    pub fn inside_box(&self) -> Option<(u32, u32, u32, u32)> {
        let (mut i0, mut i1, mut j0, mut j1) = (u32::MAX, 0, u32::MAX, 0);
        let (first, last) = self.rows()?;
        let (mut spans, mut cuts) = (Vec::new(), Vec::new());
        // Rows in strips, each with the pieces that reach it.
        let mut j = first;
        while j <= last {
            let end = (j + TILE).min(last + 1);
            let near = self.strip(j, end);
            for r in j..end {
                self.row(r, &near, &mut spans, &mut cuts);
                for s in &spans {
                    i0 = i0.min(s.i0);
                    i1 = i1.max(s.i1);
                    j0 = j0.min(r);
                    j1 = j1.max(r + 1);
                }
            }
            j = end;
        }
        (i0 < i1 && j0 < j1).then_some((i0, i1, j0, j1))
    }

    /// The rows the areas reach: `first..=last`; none when none.
    pub fn rows(&self) -> Option<(u32, u32)> {
        Some((
            self.pieces.first()?.first,
            self.pieces.iter().map(|p| p.last).max()?,
        ))
    }

    pub fn height(&self) -> u32 {
        self.height
    }
}

/// Spans of several objects as one: their union, by column.
pub fn union(spans: &[Span], out: &mut Vec<(u32, u32)>) {
    out.clear();
    out.extend(spans.iter().map(|s| (s.i0, s.i1)));
    out.sort_unstable();
    let mut merged: Vec<(u32, u32)> = Vec::with_capacity(out.len());
    for &(a, b) in out.iter() {
        match merged.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => merged.push((a, b)),
        }
    }
    *out = merged;
}

fn clamp_cut(k: f64, w: i64) -> i64 {
    if k.is_nan() {
        return w;
    }
    k.clamp(0.0, w as f64) as i64
}

/// The first column whose centre is on the crossing of the rising edge
/// lo → hi with row v = vr or past it (to its right), decided exactly: a
/// centre C is there when orient(lo, hi, C) ≤ 0. Columns past the grid
/// stand for "none" (`w`) and "all" (0).
fn first_right(lo: Vec2, hi: Vec2, ux: f64, vr: f64, w: i64) -> i64 {
    let est = (ux - 0.5).ceil();
    if !(est > -2.0) {
        return 0;
    }
    if est > (w + 1) as f64 {
        return w;
    }
    let right = |k: i64| orient2d(lo, hi, Vec2::new(k as f64 + 0.5, vr)) <= 0.0;
    let mut k = est as i64;
    while k < w + 1 && !right(k) {
        k += 1;
    }
    while k > -1 && right(k - 1) {
        k -= 1;
    }
    k.clamp(0, w)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_geometry_core::geom::arrangement::Ring;

    fn grid(w: u32, h: u32) -> Grid {
        // A metre a cell, north up: row 0 at the top (y from 10 down).
        Grid {
            affine: [0.0, 1.0, 0.0, f64::from(h), 0.0, -1.0],
            width: w,
            height: h,
        }
    }

    fn square(x0: f64, y0: f64, x1: f64, y1: f64) -> Shape {
        Shape::Polygon {
            pts: vec![
                Vec2::new(x0, y0),
                Vec2::new(x1, y0),
                Vec2::new(x1, y1),
                Vec2::new(x0, y1),
            ],
            bulges: None,
            holes: None,
            parts: None,
        }
    }

    fn rows(a: &Areas, h: u32) -> Vec<Vec<Span>> {
        let near = a.strip(0, h);
        let (mut spans, mut cuts) = (Vec::new(), Vec::new());
        (0..h)
            .map(|j| {
                a.row(j, &near, &mut spans, &mut cuts);
                spans.clone()
            })
            .collect()
    }

    #[test]
    fn a_square_takes_the_centres_inside_it() {
        let g = grid(10, 10);
        // x 2..5, y 3..7: centres x 2.5, 3.5, 4.5; rows whose centre y is 3.5..6.5 → j 3..6.
        let a = Areas::new(&g, &[square(2.0, 3.0, 5.0, 7.0)]);
        let r = rows(&a, 10);
        for (j, spans) in r.iter().enumerate() {
            if (3..7).contains(&j) {
                assert_eq!(
                    spans,
                    &[Span {
                        object: 0,
                        i0: 2,
                        i1: 5
                    }],
                    "row {j}"
                );
            } else {
                assert!(spans.is_empty(), "row {j}: {spans:?}");
            }
        }
        assert_eq!(a.inside_box(), Some((2, 5, 3, 7)));
    }

    #[test]
    fn centres_on_the_boundary_follow_the_half_open_rule() {
        let g = grid(10, 10);
        // Edges through centres: x from 2.5 to 5.5, y from 2.5 to 6.5.
        let a = Areas::new(&g, &[square(2.5, 2.5, 5.5, 6.5)]);
        let r = rows(&a, 10);
        // The left edge's centres are in, the right edge's out: columns 2, 3, 4.
        // Row centre y = 10 − (j + ½): 6.5 (j 3) is on the top edge, 2.5 (j 7) on the bottom.
        // In cell space v = 10 − y: the top edge at v = 3.5 is the lower end (in), the bottom at v = 7.5 out.
        let inside: Vec<usize> = (0..10).filter(|&j| !r[j].is_empty()).collect();
        assert_eq!(inside, vec![3, 4, 5, 6]);
        assert_eq!(
            r[3],
            vec![Span {
                object: 0,
                i0: 2,
                i1: 5
            }]
        );
    }

    #[test]
    fn holes_and_objects_apart() {
        let g = grid(12, 12);
        let mut outer = square(1.0, 1.0, 11.0, 11.0);
        if let Shape::Polygon { holes, .. } = &mut outer {
            *holes = Some(vec![Ring {
                pts: vec![
                    Vec2::new(4.0, 4.0),
                    Vec2::new(8.0, 4.0),
                    Vec2::new(8.0, 8.0),
                    Vec2::new(4.0, 8.0),
                ],
                bulges: None,
            }]);
        }
        let a = Areas::new(&g, &[outer, square(0.0, 0.0, 2.0, 2.0)]);
        let r = rows(&a, 12);
        // Row j = 6 (centre y 5.5): the ring's cells 1..4 and 8..11.
        assert_eq!(
            r[6],
            vec![
                Span {
                    object: 0,
                    i0: 1,
                    i1: 4
                },
                Span {
                    object: 0,
                    i0: 8,
                    i1: 11
                }
            ]
        );
        // Row j = 10 (centre y 1.5): both objects.
        assert_eq!(
            r[10],
            vec![
                Span {
                    object: 0,
                    i0: 1,
                    i1: 11
                },
                Span {
                    object: 1,
                    i0: 0,
                    i1: 2
                }
            ]
        );
        let mut u = Vec::new();
        union(&r[10], &mut u);
        assert_eq!(u, vec![(0, 11)]);
    }

    #[test]
    fn a_circle_holds_the_centres_within_its_radius() {
        let g = grid(21, 21);
        // Centre on cell (10, 10)'s centre, radius 5: (3, 4) away is on the circle.
        let a = Areas::new(
            &g,
            &[Shape::Circle {
                c: Vec2::new(10.5, 10.5),
                r: 5.0,
            }],
        );
        let r = rows(&a, 21);
        let mut n = 0;
        for (j, spans) in r.iter().enumerate() {
            for s in spans {
                for i in s.i0..s.i1 {
                    let (dx, dy) = (i as i64 - 10, j as i64 - 10);
                    assert!(dx * dx + dy * dy <= 25, "cell ({i}, {j})");
                    n += 1;
                }
            }
        }
        // 69 cells have di² + dj² < 25. Of the 12 on the circle the half-open rule takes a row's
        // left one and not its right one; at the circle's top and bottom rows the two crossings
        // meet at one column and take nothing: 5 more.
        assert_eq!(n, 74);
    }

    #[test]
    fn a_bulged_edge_is_an_arc() {
        let g = grid(20, 20);
        // A half disc: the chord from (15, 10) to (5, 10) with bulge 1, centre (10, 10), radius 5, above it.
        let half = Shape::Polygon {
            pts: vec![Vec2::new(5.0, 10.0), Vec2::new(15.0, 10.0)],
            bulges: Some(vec![0.0, 1.0]),
            holes: None,
            parts: None,
        };
        let a = Areas::new(&g, &[half]);
        let r = rows(&a, 20);
        let mut n = 0;
        for (j, spans) in r.iter().enumerate() {
            for s in spans {
                for i in s.i0..s.i1 {
                    let (x, y) = (i as f64 + 0.5, 20.0 - (j as f64 + 0.5));
                    let (dx, dy) = (x - 10.0, y - 10.0);
                    assert!(y > 10.0 && dx * dx + dy * dy < 25.0 + 1e-9, "({x}, {y})");
                    n += 1;
                }
            }
        }
        assert!(n > 30, "{n}");
    }
}
