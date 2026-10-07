//! Eğri boyunca yazı (docs/adr/0196): a text whose letters stand on a curve
//! kept in its own frame (`p` its start, x along its rotation). Each letter
//! stands where its middle falls on the curve, turned as the curve there;
//! past the curve's ends it goes on straight. The store draws the letters
//! as one line record each, the tools preview them so, and the box the
//! letters make is what picks, selects and opens a hatch.
//!
//! The rules are the ADR's, in its order of operations, so that
//! `scripts/fixtures/text_along_cases.py` holds this module to them
//! (`fixtures/text/v1/along.json`).

use crate::entity::{Shape, area_parts};
use crate::geom::affine::{Affine, apply, apply_linear, is_reflection, length_scale};
use crate::geom::bulge::{bulge_arc, bulge_of_sweep};
use crate::geom::intersect::{Edge, closest_on_edge, point_at};
use crate::jsmath::{PI, atan2, cos, js_hypot, js_max, js_min, sin};
use crate::ops::edges::edge_length;
use crate::ops::path::{Path, path_of, point_at_s, tangent_at_s};
use crate::store::labels::{LABEL_LINE, LABEL_PARAGRAPH_MASK, LABEL_PIECE_LINE};
use crate::text::paragraph::Run;
use crate::text::{Font, TextAlign};
use crate::vec2::Vec2;

/// A text's curve (`TextPath`): its vertices after its point, in its frame,
/// metres, and each edge's bulge (none: all straight).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Curve {
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
}

crate::json_struct!(Curve { pts, bulges });

/// Two points this close are one (a direction's and a piece's rule, §2.8, §4).
pub const ONE_POINT: f64 = 1e-9;

/// An angle in degrees from 0 up to 360.
pub fn degrees(rad: f64) -> f64 {
    let d = (rad * 180.0) / PI;
    ((d % 360.0) + 360.0) % 360.0
}

/// Whether a direction (degrees, 0 up to 360) reads upside down: over 90°
/// and at most 270° (ADR 0145 §3).
pub fn upside_down(d: f64) -> bool {
    d > 90.0 && d <= 270.0
}

/// The frame's axes for a rotation in degrees.
fn axes(rotation: f64) -> (f64, f64) {
    let r = (rotation * PI) / 180.0;
    (cos(r), sin(r))
}

/// `v` turned by `rotation` degrees, counter-clockwise.
fn turned(v: Vec2, rotation: f64) -> Vec2 {
    let (c, s) = axes(rotation);
    Vec2::new(c * v.x - s * v.y, s * v.x + c * v.y)
}

/// The tangent of an edge at its fraction `t`: a straight edge's direction,
/// an arc's square to its radius, turning as its sweep does.
fn tangent(e: &Edge, t: f64) -> Vec2 {
    match *e {
        Edge::Seg { a, b } => {
            let l = js_hypot(b.x - a.x, b.y - a.y);
            Vec2::new((b.x - a.x) / l, (b.y - a.y) / l)
        }
        Edge::Arc { a0, sweep, .. } => {
            let f = a0 + sweep * t;
            let s = if sweep >= 0.0 { 1.0 } else { -1.0 };
            Vec2::new(-sin(f) * s, cos(f) * s)
        }
    }
}

impl Curve {
    /// Edge i's bulge (0 when straight).
    pub fn bulge(&self, i: usize) -> f64 {
        self.bulges
            .as_ref()
            .and_then(|b| b.get(i).copied())
            .unwrap_or(0.0)
    }

    /// Its vertices in the world: `p`, then `p` plus each turned by the frame.
    pub fn world(&self, p: Vec2, rotation: f64) -> Vec<Vec2> {
        let mut out = Vec::with_capacity(self.pts.len() + 1);
        out.push(p);
        for &q in &self.pts {
            let d = turned(q, rotation);
            out.push(Vec2::new(p.x + d.x, p.y + d.y));
        }
        out
    }

    /// Its vertices `vs` (the first its point) and their edges' bulges seen
    /// from a frame turned `rotation` degrees: the curve in that frame.
    pub fn framed(vs: &[Vec2], bulges: &[f64], rotation: f64) -> Curve {
        let p = vs.first().copied().unwrap_or(Vec2::new(0.0, 0.0));
        let pts = vs
            .iter()
            .skip(1)
            .map(|v| turned(Vec2::new(v.x - p.x, v.y - p.y), -rotation))
            .collect();
        Curve {
            pts,
            bulges: bulges.iter().any(|&b| b != 0.0).then(|| bulges.to_vec()),
        }
    }

    /// Its edges in the world, straight or arcs (a bulge of 1e−12 or less, or
    /// a chord under 1e−12, is straight).
    pub fn edges(&self, p: Vec2, rotation: f64) -> Vec<Edge> {
        let vs = self.world(p, rotation);
        (0..self.pts.len())
            .map(|i| {
                let (a, b) = (vs[i], vs[i + 1]);
                match bulge_arc(a, b, self.bulge(i)) {
                    Some(arc) => Edge::Arc {
                        c: arc.c,
                        r: arc.r,
                        a0: arc.a0,
                        sweep: arc.sweep,
                    },
                    None => Edge::Seg { a, b },
                }
            })
            .collect()
    }

    /// Its length, metres.
    pub fn length(&self, p: Vec2, rotation: f64) -> f64 {
        self.edges(p, rotation).iter().map(edge_length).sum()
    }

    /// The curve turned the other way (§3): its last vertex the new point, its
    /// frame half a turn on, its vertices and bulges backwards, the bulges'
    /// signs turned. The new point, rotation and curve.
    pub fn reversed(&self, p: Vec2, rotation: f64) -> (Vec2, f64, Curve) {
        let n = self.pts.len();
        let Some(&last) = self.pts.last() else {
            return (p, rotation, self.clone());
        };
        let d = turned(last, rotation);
        let at = Vec2::new(p.x + d.x, p.y + d.y);
        let origin = Vec2::new(0.0, 0.0);
        let pts = (1..=n)
            .map(|k| {
                let q = if k == n { origin } else { self.pts[n - 1 - k] };
                Vec2::new(last.x - q.x, last.y - q.y)
            })
            .collect();
        let bulges = self.bulges.as_ref().map(|b| {
            (0..n)
                .map(|k| -b.get(n - 1 - k).copied().unwrap_or(0.0))
                .collect()
        });
        let r = ((rotation + 180.0) % 360.0 + 360.0) % 360.0;
        (at, r, Curve { pts, bulges })
    }

    /// The direction it runs in, degrees: from its first vertex to its last,
    /// or its first edge's starting tangent when they are within 1e−9 m (§3,
    /// the mirror's rule).
    pub fn chord(&self, p: Vec2, rotation: f64) -> f64 {
        let last = self.pts.last().copied().unwrap_or(Vec2::new(0.0, 0.0));
        let d = turned(last, rotation);
        if js_hypot(d.x, d.y) > ONE_POINT {
            return degrees(atan2(d.y, d.x));
        }
        self.edges(p, rotation)
            .iter()
            .find(|e| edge_length(e) > 0.0)
            .map_or(rotation, |e| {
                let t = tangent(e, 0.0);
                degrees(atan2(t.y, t.x))
            })
    }
}

/// The alignment with its share along swapped (left and right), and with
/// `up_too` its share up as well (baseline and bottom to top, top to
/// baseline, the middle kept): §3.
pub fn swapped(align: Option<TextAlign>, up_too: bool) -> Option<TextAlign> {
    use TextAlign::*;
    if up_too {
        return match align {
            None | Some(BottomLeft) => Some(TopRight),
            Some(BaselineCenter) | Some(BottomCenter) => Some(TopCenter),
            Some(BaselineRight) | Some(BottomRight) => Some(TopLeft),
            Some(MiddleLeft) => Some(MiddleRight),
            Some(MiddleCenter) => Some(MiddleCenter),
            Some(MiddleRight) => Some(MiddleLeft),
            Some(TopLeft) => Some(BaselineRight),
            Some(TopCenter) => Some(BaselineCenter),
            Some(TopRight) => None,
        };
    }
    match align {
        None => Some(BaselineRight),
        Some(BaselineRight) => None,
        Some(BottomLeft) => Some(BottomRight),
        Some(BottomRight) => Some(BottomLeft),
        Some(MiddleLeft) => Some(MiddleRight),
        Some(MiddleRight) => Some(MiddleLeft),
        Some(TopLeft) => Some(TopRight),
        Some(TopRight) => Some(TopLeft),
        centred => centred,
    }
}

/// A letter placed (§2.5): where its baseline starts, its turn (degrees),
/// its advance (metres), and its middle's point and tangent on the curve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Letter {
    pub at: Vec2,
    pub turn: f64,
    pub advance: f64,
    pub mid: Vec2,
    pub tangent: Vec2,
}

/// What lays a text out along its curve.
#[derive(Clone, Copy, Debug)]
pub struct Along<'a> {
    pub p: Vec2,
    pub rotation: f64,
    pub curve: &'a Curve,
    pub text: &'a str,
    pub runs: &'a [Run],
    pub height: f64,
    pub width_factor: f64,
    pub align: Option<TextAlign>,
    pub font: Font,
    /// Every letter bold: the text's own (with a typeface), its runs' on top.
    pub bold: bool,
    /// The slant's tangent (0 without a typeface or a slant).
    pub lean: f64,
}

/// The curve walked by distance.
struct Walk {
    edges: Vec<Edge>,
    lens: Vec<f64>,
    starts: Vec<Vec2>,
    ends: Vec<Vec2>,
    total: f64,
}

impl Walk {
    fn of(curve: &Curve, p: Vec2, rotation: f64) -> Walk {
        let vs = curve.world(p, rotation);
        let edges = curve.edges(p, rotation);
        let lens: Vec<f64> = edges.iter().map(edge_length).collect();
        let total = lens.iter().sum();
        Walk {
            starts: vs[..edges.len()].to_vec(),
            ends: vs[1..].to_vec(),
            edges,
            lens,
            total,
        }
    }

    /// The point and unit tangent `m` metres along (§2.4).
    fn locate(&self, m: f64, rotation: f64) -> (Vec2, Vec2) {
        let live = |i: &usize| self.lens[*i] > 0.0;
        let Some(first) = (0..self.edges.len()).find(live) else {
            let (c, s) = axes(rotation);
            let o = self.starts.first().copied().unwrap_or(Vec2::new(0.0, 0.0));
            return (Vec2::new(o.x + c * m, o.y + s * m), Vec2::new(c, s));
        };
        if m < 0.0 {
            let t = tangent(&self.edges[first], 0.0);
            let a = self.starts[first];
            return (Vec2::new(a.x + t.x * m, a.y + t.y * m), t);
        }
        if m > self.total {
            let last = (0..self.edges.len()).rev().find(live).unwrap_or(first);
            let t = tangent(&self.edges[last], 1.0);
            let b = self.ends[last];
            let over = m - self.total;
            return (Vec2::new(b.x + t.x * over, b.y + t.y * over), t);
        }
        let (mut cum, mut chosen) = (0.0, (first, 0.0));
        for (i, len) in self.lens.iter().enumerate() {
            if *len > 0.0 && cum <= m {
                chosen = (i, cum);
            }
            cum += len;
        }
        let (i, start) = chosen;
        let t = js_min(1.0, (m - start) / self.lens[i]);
        (point_at(&self.edges[i], t), tangent(&self.edges[i], t))
    }
}

impl<'a> Along<'a> {
    /// Each letter's advance, metres (§2.2).
    pub fn advances(&self) -> Vec<f64> {
        let letters: Vec<char> = self.text.chars().collect();
        crate::text::paragraph::advances(&letters, self.runs, self.font, self.bold)
            .into_iter()
            .map(|a| a / 1000.0 * self.height * self.width_factor)
            .collect()
    }

    /// The curve's length, metres.
    pub fn length(&self) -> f64 {
        self.curve.length(self.p, self.rotation)
    }

    /// Its letters in order (§2.3–§2.5).
    pub fn letters(&self) -> Vec<Letter> {
        let walk = Walk::of(self.curve, self.p, self.rotation);
        let advances = self.advances();
        let (along, up) = self.align.map_or((0.0, 0.0), |a| (a.along(), a.up()));
        let total: f64 = advances.iter().sum();
        let mut x = along * (walk.total - total);
        let h = self.height;
        advances
            .iter()
            .map(|&a| {
                let mid = x + a / 2.0;
                let (pt, t) = walk.locate(mid, self.rotation);
                let n = Vec2::new(-t.y, t.x);
                let at = Vec2::new(
                    pt.x - t.x * a / 2.0 - n.x * up * h,
                    pt.y - t.y * a / 2.0 - n.y * up * h,
                );
                x += a;
                Letter {
                    at,
                    turn: degrees(atan2(t.y, t.x)),
                    advance: a,
                    mid: pt,
                    tangent: t,
                }
            })
            .collect()
    }

    /// Its direction, degrees (§2.8): from the first letter's middle to the
    /// last one's; the first letter's tangent when there is one letter or the
    /// two are within 1e−9 m; its rotation when it has no letters.
    pub fn direction(&self, letters: &[Letter]) -> f64 {
        let (Some(first), Some(last)) = (letters.first(), letters.last()) else {
            return ((self.rotation % 360.0) + 360.0) % 360.0;
        };
        let (dx, dy) = (last.mid.x - first.mid.x, last.mid.y - first.mid.y);
        if letters.len() == 1 || js_hypot(dx, dy) <= ONE_POINT {
            return degrees(atan2(first.tangent.y, first.tangent.x));
        }
        degrees(atan2(dy, dx))
    }

    /// The letters' boxes as one ring (§2.7): their lower corners in order,
    /// then their upper corners from the last back; each box from its
    /// baseline start, 0.23 of the height under the baseline to 1.15 over
    /// it, leaning with the slant, `margin` wider all round.
    pub fn outline(&self, letters: &[Letter], margin: f64) -> Vec<Vec2> {
        let h = self.height;
        let (y0, y1) = (-0.23 * h - margin, 1.15 * h + margin);
        let corner = |l: &Letter, x: f64, y: f64| {
            let (t, n) = (l.tangent, Vec2::new(-l.tangent.y, l.tangent.x));
            let x = x + y * self.lean;
            Vec2::new(l.at.x + t.x * x + n.x * y, l.at.y + t.y * x + n.y * y)
        };
        let mut lower = Vec::with_capacity(letters.len() * 2);
        let mut upper = Vec::with_capacity(letters.len() * 2);
        for l in letters {
            lower.push(corner(l, -margin, y0));
            lower.push(corner(l, l.advance + margin, y0));
            upper.push(corner(l, l.advance + margin, y1));
            upper.push(corner(l, -margin, y1));
        }
        upper.reverse();
        lower.extend(upper);
        lower
    }

    /// Its label records (§2.6): per letter its mask when masked, then its
    /// line (`LABEL_LINE`, or `LABEL_PIECE_LINE` for a block's piece at
    /// `piece`), as a one-line text's.
    pub fn records(
        &self,
        letters: &[Letter],
        masked: bool,
        id: f64,
        piece: Option<f64>,
        out: &mut Vec<f64>,
    ) {
        let h = self.height;
        let (kind, c) = match piece {
            Some(place) => (LABEL_PIECE_LINE, place),
            None => (LABEL_LINE, self.width_factor),
        };
        for (i, l) in letters.iter().enumerate() {
            let (t, n) = (l.tangent, Vec2::new(-l.tangent.y, l.tangent.x));
            if masked {
                let (hh, mg) = (h * 1.15, h * 0.1);
                let y0 = -hh * 0.2 - mg;
                let x0 = -mg + y0 * self.lean;
                out.extend([
                    id,
                    LABEL_PARAGRAPH_MASK,
                    l.at.x + t.x * x0 + n.x * y0,
                    l.at.y + t.y * x0 + n.y * y0,
                    l.turn,
                    l.advance + 2.0 * mg,
                    hh * 1.2 + 2.0 * mg,
                    piece.unwrap_or(0.0),
                    0.0,
                ]);
            }
            out.extend([
                id,
                kind,
                l.at.x,
                l.at.y,
                l.turn,
                h,
                c,
                i as f64,
                (i + 1) as f64,
            ]);
        }
    }

    /// Okunur yap (§3): when it reads upside down, the curve turned the
    /// other way and the alignment's shares swapped, both; the new point,
    /// rotation, curve and alignment. None when it reads.
    pub fn readable(&self) -> Option<Placed> {
        let letters = self.letters();
        if !upside_down(self.direction(&letters)) {
            return None;
        }
        let (p, rotation, curve) = self.curve.reversed(self.p, self.rotation);
        Some(Placed {
            p,
            rotation,
            curve,
            align: swapped(self.align, true),
        })
    }

    /// Düzleştir (§4): the straight text's point (the first letter's
    /// baseline start) and rotation (the direction).
    pub fn straight(&self) -> (Vec2, f64) {
        let letters = self.letters();
        let p = letters.first().map_or(self.p, |l| l.at);
        (p, self.direction(&letters))
    }
}

/// A text's new place along a curve: its point, rotation, curve and alignment.
#[derive(Clone, Debug, PartialEq)]
pub struct Placed {
    pub p: Vec2,
    pub rotation: f64,
    pub curve: Curve,
    pub align: Option<TextAlign>,
}

/// A curved text under a similarity (§3): its point moved, its frame turned
/// as its baseline's direction, its curve scaled; mirrored, its curve
/// mirrored in its frame, turned the other way with its share along
/// swapped, and again with its share up swapped when the curve then reads
/// upside down.
pub fn transformed(
    p: Vec2,
    rotation: f64,
    curve: &Curve,
    align: Option<TextAlign>,
    m: &Affine,
) -> Placed {
    let s = length_scale(m);
    let at = apply(m, p);
    let (c, sn) = axes(rotation);
    let u = apply_linear(m, Vec2::new(c, sn));
    let turn = degrees(atan2(u.y, u.x));
    if !is_reflection(m) {
        return Placed {
            p: at,
            rotation: turn,
            curve: Curve {
                pts: curve
                    .pts
                    .iter()
                    .map(|q| Vec2::new(q.x * s, q.y * s))
                    .collect(),
                bulges: curve.bulges.clone(),
            },
            align,
        };
    }
    let mirrored = Curve {
        pts: curve
            .pts
            .iter()
            .map(|q| Vec2::new(q.x * s, -q.y * s))
            .collect(),
        bulges: curve
            .bulges
            .as_ref()
            .map(|b| b.iter().map(|x| -x).collect()),
    };
    let (p2, r2, c2) = mirrored.reversed(at, turn);
    let a2 = swapped(align, false);
    if upside_down(c2.chord(p2, r2)) {
        let (p3, r3, c3) = c2.reversed(p2, r2);
        return Placed {
            p: p3,
            rotation: r3,
            curve: c3,
            align: swapped(a2, true),
        };
    }
    Placed {
        p: p2,
        rotation: r2,
        curve: c2,
        align: a2,
    }
}

/// Doğrultuya döndür (§4): a direction's angle in degrees, half a turn less
/// when it reads upside down.
pub fn readable_turn(dx: f64, dy: f64) -> f64 {
    let a = degrees(atan2(dy, dx));
    if upside_down(a) {
        ((a - 180.0) % 360.0 + 360.0) % 360.0
    } else {
        a
    }
}

// ── A curve's piece (§4) ────────────────────────────────────────────────

/// A path from its edges.
fn path_from(edges: Vec<Edge>, closed: bool) -> Option<Path> {
    if edges.is_empty() {
        return None;
    }
    let mut cum = Vec::with_capacity(edges.len());
    let mut s = 0.0;
    for e in &edges {
        cum.push(s);
        s += edge_length(e);
    }
    Some(Path {
        edges,
        cum,
        length: s,
        closed,
    })
}

/// The paths a curve offers a text (§4): a line or an arc, a circle; each
/// part of a polyline; each ring and hole of an area; an ellipse or a spline
/// by its chords.
pub fn paths_of(shape: &Shape) -> Vec<Path> {
    match shape {
        Shape::Polyline { .. } => area_parts(shape).iter().filter_map(path_of).collect(),
        Shape::Polygon { .. } => {
            let mut out = Vec::new();
            for part in area_parts(shape).iter() {
                let Shape::Polygon {
                    pts, bulges, holes, ..
                } = part
                else {
                    continue;
                };
                let ring = |pts: &[Vec2], bulges: Option<&[f64]>| {
                    path_from(
                        crate::geom::bulge::bulge_path_edges(pts, bulges, true),
                        true,
                    )
                };
                out.extend(ring(pts, bulges.as_deref()));
                for h in holes.iter().flatten() {
                    out.extend(ring(&h.pts, h.bulges.as_deref()));
                }
            }
            out
        }
        Shape::Line { .. }
        | Shape::Arc { .. }
        | Shape::Circle { .. }
        | Shape::Ellipse { .. }
        | Shape::Spline { .. } => path_of(shape).into_iter().collect(),
        _ => Vec::new(),
    }
}

/// The path walked the other way.
fn backwards(path: &Path) -> Path {
    let edges = path
        .edges
        .iter()
        .rev()
        .map(|e| match *e {
            Edge::Seg { a, b } => Edge::Seg { a: b, b: a },
            Edge::Arc { c, r, a0, sweep } => Edge::Arc {
                c,
                r,
                a0: a0 + sweep,
                sweep: -sweep,
            },
        })
        .collect();
    path_from(edges, path.closed).unwrap_or_else(|| path.clone())
}

/// The vertices and edge bulges from `s0` to `s1` along the path (a closed
/// one's may run past its start); vertices within 1e−9 m of the one before
/// go, the edge after them keeping its bulge.
fn stretch(path: &Path, s0: f64, s1: f64) -> (Vec<Vec2>, Vec<f64>) {
    let total = path.length;
    let mut vs = vec![point_at_s(path, s0)];
    let mut bulges = Vec::new();
    let rounds = if path.closed { 2 } else { 1 };
    for round in 0..rounds {
        for (i, e) in path.edges.iter().enumerate() {
            let len = edge_length(e);
            let g0 = path.cum[i] + round as f64 * total;
            let lo = js_max(s0, g0);
            let hi = js_min(s1, g0 + len);
            if hi - lo <= 1e-9 * js_max(1.0, total) {
                continue;
            }
            let div = if len == 0.0 { 1.0 } else { len };
            let (t0, t1) = ((lo - g0) / div, (hi - g0) / div);
            bulges.push(match *e {
                Edge::Arc { sweep, .. } => bulge_of_sweep(sweep * (t1 - t0)),
                Edge::Seg { .. } => 0.0,
            });
            vs.push(point_at(e, js_min(1.0, t1)));
        }
    }
    let mut kept_v = vec![vs[0]];
    let mut kept_b = Vec::with_capacity(bulges.len());
    for i in 1..vs.len() {
        let last = kept_v[kept_v.len() - 1];
        if js_hypot(vs[i].x - last.x, vs[i].y - last.y) <= ONE_POINT {
            continue;
        }
        kept_v.push(vs[i]);
        kept_b.push(bulges[i - 1]);
    }
    (kept_v, kept_b)
}

/// The piece of a curve a text of `length` metres stands on (§4), clicked
/// at `click`, the click being the text's share `share` along it: the
/// text's point, rotation and curve. None for a curve with no path or a
/// piece of no edge.
pub fn piece(shape: &Shape, click: Vec2, length: f64, share: f64) -> Option<(Vec2, f64, Curve)> {
    let paths = paths_of(shape);
    let mut best: Option<(f64, usize, f64)> = None;
    for (k, path) in paths.iter().enumerate() {
        for (i, e) in path.edges.iter().enumerate() {
            let c = closest_on_edge(e, click);
            if best.is_none_or(|(d, _, _)| c.d < d) {
                best = Some((c.d, k, path.cum[i] + c.t * edge_length(e)));
            }
        }
    }
    let (_, k, s) = best?;
    let forward = &paths[k];
    let total = forward.length;
    if !(total > 0.0) {
        return None;
    }
    let t = tangent_at_s(forward, s);
    let (path, s) = if upside_down(degrees(atan2(t.y, t.x))) {
        (backwards(forward), total - s)
    } else {
        (forward.clone(), s)
    };
    let mut start = s - share * length;
    let end;
    if path.closed {
        start = ((start % total) + total) % total;
        end = start + js_min(length, total);
    } else if total < length {
        start = 0.0;
        end = total;
    } else {
        start = js_max(0.0, js_min(total - length, start));
        end = start + length;
    }
    let (vs, bulges) = stretch(&path, start, end);
    if vs.len() < 2 {
        return None;
    }
    let (first, last) = (vs[0], vs[vs.len() - 1]);
    let rotation = if js_hypot(last.x - first.x, last.y - first.y) <= ONE_POINT {
        let t = tangent_at_s(&path, start);
        degrees(atan2(t.y, t.x))
    } else {
        degrees(atan2(last.y - first.y, last.x - first.x))
    };
    Some((first, rotation, Curve::framed(&vs, &bulges, rotation)))
}

// ── The web's calls ────────────────────────────────────────────────────

crate::json_struct!(out Placed { p, rotation, curve => "path", align });

/// A piece's place for a text: its point, rotation and curve.
#[derive(Clone, Debug, PartialEq)]
pub struct PiecePlace {
    pub p: Vec2,
    pub rotation: f64,
    pub curve: Curve,
}

crate::json_struct!(out PiecePlace { p, rotation, curve => "path" });

/// A straight text's point and rotation (Düzleştir).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Straight {
    pub p: Vec2,
    pub rotation: f64,
}

crate::json_struct!(out Straight { p, rotation });

/// The length of a text's letters, metres (§2.2), as `textBox` reads a text
/// (`font` its typeface, its own or the drawing's): what a tool cuts a
/// curve's piece to.
fn letters_length(v: &crate::api::json::Json) -> Result<f64, String> {
    use crate::api::json::{Flat, Json, read_field};
    let font = match v.get("font") {
        Json::Str(id) => Font::from_id(id),
        _ => Font::DEFAULT,
    };
    let face = crate::text::face::Face::read_flat(v)?;
    let text: String = read_field(v, "text")?;
    let runs: Option<Vec<Run>> = read_field(v, "runs")?;
    let height: f64 = read_field(v, "height")?;
    let width: Option<f64> = read_field(v, "widthFactor")?;
    let letters: Vec<char> = text.chars().collect();
    let advances = crate::text::paragraph::advances(
        &letters,
        runs.as_deref().unwrap_or_default(),
        face.font_or(font),
        face.is_bold(),
    );
    Ok(advances
        .iter()
        .map(|a| a / 1000.0 * height * width.unwrap_or(1.0))
        .sum())
}

pub(crate) static OPS: &[crate::api::Op] = &[
    // Where a text of `length` metres stands on a curve clicked at `click`, the click its share along.
    crate::op!(
        "textAlongPiece",
        |e: crate::entity::Entity, click: Vec2, length: f64, share: f64| {
            piece(&e.shape, click, length, share).map(|(p, rotation, curve)| PiecePlace {
                p,
                rotation,
                curve,
            })
        }
    ),
    crate::op!("textAlongLength", |t: crate::api::json::Json| {
        letters_length(&t)
    }),
    crate::op!("textAlongReadable", |t: crate::api::json::Json| {
        crate::entity::along_json(&t, |a| a.readable()).map(Option::flatten)
    }),
    crate::op!("textAlongStraight", |t: crate::api::json::Json| {
        crate::entity::along_json(&t, |a| {
            let (p, rotation) = a.straight();
            Straight { p, rotation }
        })
    }),
    crate::op!("textAlongTurn", |d: Vec2| readable_turn(d.x, d.y)),
    // Its curve's length, metres (Öznitelikler's Eğri row); null for a straight text.
    crate::op!("textAlongCurveLength", |t: crate::api::json::Json| {
        crate::entity::along_json(&t, |a| a.length())
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn along<'a>(curve: &'a Curve, text: &'a str, align: Option<TextAlign>) -> Along<'a> {
        Along {
            p: Vec2::new(10.0, 20.0),
            rotation: 0.0,
            curve,
            text,
            runs: &[],
            height: 2.0,
            width_factor: 1.0,
            align,
            font: Font::DEFAULT,
            bold: false,
            lean: 0.0,
        }
    }

    #[test]
    fn a_straight_curve_puts_the_letters_on_its_line() {
        let curve = Curve {
            pts: vec![Vec2::new(30.0, 0.0)],
            bulges: None,
        };
        let a = along(&curve, "Ab", None);
        let letters = a.letters();
        assert_eq!(letters.len(), 2);
        assert_eq!(letters[0].at, Vec2::new(10.0, 20.0));
        assert_eq!(letters[0].turn, 0.0);
        assert!((letters[1].at.x - (10.0 + letters[0].advance)).abs() < 1e-12);
        assert_eq!(a.direction(&letters), 0.0);
        // Centred, the letters stand in the middle.
        let c = along(&curve, "Ab", Some(TextAlign::BaselineCenter)).letters();
        let width = c[0].advance + c[1].advance;
        assert!((c[0].at.x - (10.0 + (30.0 - width) / 2.0)).abs() < 1e-12);
    }

    #[test]
    fn turning_the_other_way_twice_gives_the_curve_back() {
        let curve = Curve {
            pts: vec![Vec2::new(12.0, 3.0), Vec2::new(20.0, -1.0)],
            bulges: Some(vec![0.3, -0.1]),
        };
        let (p, r, c) = curve.reversed(Vec2::new(5.0, 6.0), 30.0);
        let (p2, r2, c2) = c.reversed(p, r);
        assert!((p2.x - 5.0).abs() < 1e-12 && (p2.y - 6.0).abs() < 1e-12);
        assert!((r2 - 30.0).abs() < 1e-12);
        for (a, b) in c2.pts.iter().zip(&curve.pts) {
            assert!((a.x - b.x).abs() < 1e-12 && (a.y - b.y).abs() < 1e-12);
        }
        assert_eq!(c2.bulges, curve.bulges);
        assert_eq!(swapped(swapped(None, false), false), None);
        assert_eq!(swapped(Some(TextAlign::TopRight), true), None);
    }

    #[test]
    fn a_turn_reads() {
        assert_eq!(readable_turn(-2.0, 0.0), 0.0);
        assert_eq!(readable_turn(0.0, -1.0), 90.0);
        assert_eq!(readable_turn(0.0, 1.0), 90.0);
    }
}
