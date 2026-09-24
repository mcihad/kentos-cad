//! Path offsets (`apps/web/src/model/geom/offset.ts`): mitred corners with a bevel
//! past the limit; bulged paths grow or shrink their arcs about the centre.

use crate::api::Op;
use crate::geom::arc::norm_angle;
use crate::geom::bulge::{bulge_arc, bulge_of_sweep, clean_bulge_path};
use crate::geom::intersect::{circle_circle, line_circle_params, line_line};
use crate::jsmath::{atan2, js_cmp, js_hypot, js_max, js_min, or, stable_sort};
use crate::op;
use crate::vec2::Vec2;

/// Beyond this multiple of the offset distance a corner is bevelled, not mitred.
const MITER_LIMIT: f64 = 4.0;

#[derive(Clone, Copy)]
struct Seg {
    a: Vec2,
    b: Vec2,
}

/// Offsets a polyline by `d` (positive = left of travel); mitred corners, sharp ones bevelled.
pub fn offset_path(pts: &[Vec2], d: f64, closed: bool) -> Vec<Vec2> {
    let n = pts.len();
    if n < 2 {
        return pts.to_vec();
    }
    let seg_count = if closed { n } else { n - 1 };
    let mut segs: Vec<Seg> = Vec::new();
    for i in 0..seg_count {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let l = js_hypot(dx, dy);
        if l < 1e-12 {
            continue;
        }
        let nx = (-dy / l) * d;
        let ny = (dx / l) * d;
        segs.push(Seg {
            a: Vec2::new(a.x + nx, a.y + ny),
            b: Vec2::new(b.x + nx, b.y + ny),
        });
    }
    if segs.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let join = |s1: Seg, s2: Seg, corner: Vec2, out: &mut Vec<Vec2>| match line_line(
        s1.a, s1.b, s2.a, s2.b,
    ) {
        None => out.push(s2.a), // collinear
        Some(h) => {
            if js_hypot(h.p.x - corner.x, h.p.y - corner.y) > MITER_LIMIT * d.abs() {
                out.push(s1.b);
                out.push(s2.a);
            } else {
                out.push(h.p);
            }
        }
    };
    let m = segs.len();
    if closed {
        for i in 0..m {
            join(segs[(i + m - 1) % m], segs[i], pts[i], &mut out);
        }
    } else {
        out.push(segs[0].a);
        for i in 1..m {
            join(segs[i - 1], segs[i], pts[i], &mut out);
        }
        out.push(segs[m - 1].b);
    }
    out
}

/// Which side of a path a point lies on: +1 left, −1 right (the nearest segment decides).
pub fn side_of(pts: &[Vec2], closed: bool, p: Vec2) -> i32 {
    let mut best = f64::INFINITY;
    let mut side = 1;
    let n = pts.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    for i in 0..count {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let l2 = or(dx * dx + dy * dy, 1.0);
        let t = js_max(0.0, js_min(1.0, ((p.x - a.x) * dx + (p.y - a.y) * dy) / l2));
        let dd = js_hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
        if dd < best {
            best = dd;
            side = if dx * (p.y - a.y) - dy * (p.x - a.x) >= 0.0 {
                1
            } else {
                -1
            };
        }
    }
    side
}

#[derive(Clone, Copy)]
struct PieceArc {
    c: Vec2,
    r: f64,
    ccw: bool,
}

#[derive(Clone, Copy)]
struct Piece {
    a: Vec2,
    b: Vec2,
    /// Arc pieces keep their circle and turning direction; endpoints may move at joins.
    arc: Option<PieceArc>,
}

/// A bulged path, or why it could not be offset.
#[derive(Clone, Debug, PartialEq)]
pub enum OffsetResult {
    Path { pts: Vec<Vec2>, bulges: Vec<f64> },
    Error { error: String },
}

impl crate::api::json::ToJson for OffsetResult {
    fn write_json(&self, out: &mut String) {
        use crate::api::json::field;
        out.push('{');
        let mut first = true;
        match self {
            OffsetResult::Path { pts, bulges } => {
                field(out, &mut first, "pts", pts);
                field(out, &mut first, "bulges", bulges);
            }
            OffsetResult::Error { error } => field(out, &mut first, "error", error),
        }
        out.push('}');
    }
}

/// Offsets a path with arc segments by `d` (positive = left of travel).
pub fn offset_bulge_path(pts: &[Vec2], bulges: &[f64], d: f64, closed: bool) -> OffsetResult {
    let n = pts.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    let mut pieces: Vec<Piece> = Vec::new();
    let mut corners: Vec<Vec2> = Vec::new();
    for i in 0..count {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        if let Some(arc) = bulge_arc(a, b, bulges.get(i).copied().unwrap_or(0.0)) {
            let ccw = arc.sweep > 0.0;
            // Left of a counter-clockwise arc is its centre side.
            let r = arc.r + if ccw { -d } else { d };
            if r <= 1e-9 {
                return OffsetResult::Error {
                    error: "Öteleme mesafesi bir yay parçasının yarıçapından büyük.".into(),
                };
            }
            let k = r / arc.r;
            let scale =
                |p: Vec2| Vec2::new(arc.c.x + (p.x - arc.c.x) * k, arc.c.y + (p.y - arc.c.y) * k);
            pieces.push(Piece {
                a: scale(a),
                b: scale(b),
                arc: Some(PieceArc { c: arc.c, r, ccw }),
            });
        } else {
            let dx = b.x - a.x;
            let dy = b.y - a.y;
            let l = js_hypot(dx, dy);
            if l < 1e-12 {
                continue;
            }
            let nx = (-dy / l) * d;
            let ny = (dx / l) * d;
            pieces.push(Piece {
                a: Vec2::new(a.x + nx, a.y + ny),
                b: Vec2::new(b.x + nx, b.y + ny),
                arc: None,
            });
        }
        corners.push(b);
    }
    if pieces.is_empty() {
        return OffsetResult::Error {
            error: "Öteleme sonucu geçerli bir şekil oluşmadı.".into(),
        };
    }

    // Joins: piece k ends where piece k+1 starts (corner = the source vertex between them).
    let m = pieces.len();
    let mut bevels = vec![false; m];
    let joins = if closed { m } else { m - 1 };
    for k in 0..joins {
        let p1 = pieces[k];
        let p2 = pieces[(k + 1) % m];
        if js_hypot(p1.b.x - p2.a.x, p1.b.y - p2.a.y) < 1e-9 {
            continue; // tangent continuation
        }
        let corner = corners[k];
        let mut hits = support_hits(&p1, &p2);
        let dist = |u: &Vec2| js_hypot(u.x - corner.x, u.y - corner.y);
        stable_sort(&mut hits, &mut |u, v| js_cmp(dist(u) - dist(v), 0.0));
        match hits.first() {
            Some(&hit) if dist(&hit) <= MITER_LIMIT * d.abs() => {
                pieces[k].b = hit;
                pieces[(k + 1) % m].a = hit;
            }
            _ => bevels[k] = true,
        }
    }

    let mut out_p = vec![pieces[0].a];
    let mut out_b: Vec<f64> = Vec::new();
    for (k, p) in pieces.iter().enumerate() {
        out_b.push(match p.arc {
            Some(arc) => arc_bulge(p, arc),
            None => 0.0,
        });
        out_p.push(p.b);
        if bevels[k] && (closed || k < m - 1) {
            out_b.push(0.0);
            out_p.push(pieces[(k + 1) % m].a);
        }
    }
    if closed {
        out_p.pop(); // the ring closes on its first point
    } else {
        out_b.push(0.0);
    }
    let clean = clean_bulge_path(&out_p, Some(&out_b), closed, 1e-9);
    let len = clean.pts.len();
    OffsetResult::Path {
        pts: clean.pts,
        bulges: clean.bulges.unwrap_or_else(|| vec![0.0; len]),
    }
}

fn arc_bulge(p: &Piece, arc: PieceArc) -> f64 {
    let s = atan2(p.a.y - arc.c.y, p.a.x - arc.c.x);
    let e = atan2(p.b.y - arc.c.y, p.b.x - arc.c.x);
    bulge_of_sweep(if arc.ccw {
        norm_angle(e - s)
    } else {
        -norm_angle(s - e)
    })
}

/// Crossings of the supporting lines/circles of two pieces (unbounded).
fn support_hits(p1: &Piece, p2: &Piece) -> Vec<Vec2> {
    match (p1.arc, p2.arc) {
        (None, None) => line_line(p1.a, p1.b, p2.a, p2.b)
            .map(|h| h.p)
            .into_iter()
            .collect(),
        (Some(a1), Some(a2)) => circle_circle(a1.c, a1.r, a2.c, a2.r),
        (Some(circle), None) | (None, Some(circle)) => {
            let line = if p1.arc.is_some() { p2 } else { p1 };
            line_circle_params(line.a, line.b, circle.c, circle.r)
                .into_iter()
                .map(|t| {
                    Vec2::new(
                        line.a.x + (line.b.x - line.a.x) * t,
                        line.a.y + (line.b.y - line.a.y) * t,
                    )
                })
                .collect()
        }
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("offsetPath", |pts: Vec<Vec2>, d: f64, closed: bool| {
        offset_path(&pts, d, closed)
    }),
    op!("sideOf", |pts: Vec<Vec2>, closed: bool, p: Vec2| side_of(
        &pts, closed, p
    )),
    op!(
        "offsetBulgePath",
        |pts: Vec<Vec2>, bulges: Vec<f64>, d: f64, closed: bool| offset_bulge_path(
            &pts, &bulges, d, closed
        )
    ),
];
