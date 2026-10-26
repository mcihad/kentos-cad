//! Walls where they meet (docs/adr/0340 §2, plan/cad.md `CAD-ARCH-06`, `07`): a wall's lines are its own offsets
//! (`geom::wall`); where walls meet they are cut, extended and interrupted so the walls read as one construction.
//! Nothing is stored: the joints follow from the walls' paths, their join modes and their components' priorities.
//!
//! - **Ends that meet** (two open walls' path ends within [`JOIN_TOLERANCE`]): at an angle a corner, every line of
//!   both cut at the mitre line through the two faces' meeting points; along one line (anti-parallel within
//!   [`INLINE_DEGREES`]) an inline join, both cut across at the point. A corner where one end is `butt` (or the other
//!   `through`) is a butt: the through wall runs on to the butting wall's far face and the butting wall tees into it.
//!   Sharper than [`ACUTE_DEGREES`] with both ends automatic, or (nearly) the same direction, it is not joined and
//!   said. Three or more ends: the anti-parallel pair joins inline and the others tee into it; without such a pair
//!   it is said.
//! - **An end inside another wall** (its path end between the other's faces, square to its path): a tee. The joining
//!   wall's components pass through the host's from the near face while the host's are weaker (a larger priority),
//!   and stop at the first that is not; each line of the joining wall stops where its components do. The host's
//!   near face is interrupted across the joining wall, and each line of the host a component passes, or meets at
//!   the same priority, under it; the host's bands the joining wall passes through are cut there.
//! - **Paths that cross** away from both walls' ends: a cross, the stronger wall (its strongest component, then its
//!   width, then its order) the host: the other's lines are interrupted inside it as two tees from both sides.
//! - `cleanup: false` on either wall: no component passes; the joining wall stops at the near face, which is still
//!   interrupted. A `none` end does not join.
//!
//! A cut that cannot be made (a wall shorter than the joint) leaves both walls as they were and is said, with the
//! join mode that would resolve it ([`Issue::fix`]).

use crate::entity::Shape;
use crate::geom::bulge::{bulge_path_edges, reverse_bulge_path, segment_tangent};
use crate::geom::intersect::{
    Edge, circle_circle, intersect_edges, line_circle_params, line_line, ray_edge,
};
use crate::geom::wall::{self, Body, DEFAULT_PRIORITY, Drawn, JoinMode, Line, Spec, Stroke};
use crate::jsmath::{PI, TAU, acos, cos, js_hypot, js_max, js_min};
use crate::ops::edges::edge_length;
use crate::ops::path::{Path, nearest_s, path_of, point_at_s, sub_path, tangent_at_s};
use crate::vec2::Vec2;

/// Two path ends this close meet; an end this close to a host's path or faces is in it, metres.
pub const JOIN_TOLERANCE: f64 = 1e-3;
/// Two ends this close to one line join inline.
pub const INLINE_DEGREES: f64 = 1.0;
/// Two ends this close to one direction overlap.
pub const PARALLEL_DEGREES: f64 = 3.0;
/// A corner sharper than this is not mitred by the rule.
pub const ACUTE_DEGREES: f64 = 15.0;
/// How far a cut may trim or extend a line, in the two walls' widths.
const REACH: f64 = 8.0;

/// A wall to join: its path and its fields.
#[derive(Clone, Copy, Debug)]
pub struct WallIn<'a> {
    pub pts: &'a [Vec2],
    pub bulges: Option<&'a [f64]>,
    pub spec: &'a Spec,
}

/// A wall's end.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Start,
    End,
}

impl End {
    pub fn name(self) -> &'static str {
        match self {
            End::Start => "start",
            End::End => "end",
        }
    }

    pub fn index(self) -> usize {
        match self {
            End::Start => 0,
            End::End => 1,
        }
    }
}

/// What kind of joint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointKind {
    /// Two ends at an angle (mitred or butt).
    Corner,
    /// Two ends along one line.
    Inline,
    /// An end in another wall.
    Tee,
    /// Two paths crossing.
    Cross,
}

impl JointKind {
    pub fn name(self) -> &'static str {
        match self {
            JointKind::Corner => "corner",
            JointKind::Inline => "inline",
            JointKind::Tee => "tee",
            JointKind::Cross => "cross",
        }
    }
}

/// A wall in a joint: by its end, or along its length (a tee's host, a cross's walls).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Member {
    pub wall: usize,
    pub end: Option<End>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Joint {
    pub kind: JointKind,
    pub at: Vec2,
    pub members: Vec<Member>,
}

/// Why walls that meet are not joined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IssueCode {
    /// Two walls meet running the same way.
    Overlap,
    /// More than two ends meet with no two along one line.
    Crowded,
    /// A corner too sharp to mitre.
    Acute,
    /// A line could not be cut to the joint (a wall shorter than it, a host that ends before it).
    Unresolved,
}

impl IssueCode {
    pub fn code(self) -> &'static str {
        match self {
            IssueCode::Overlap => "wall_join_overlap",
            IssueCode::Crowded => "wall_join_crowded",
            IssueCode::Acute => "wall_join_acute",
            IssueCode::Unresolved => "wall_join_unresolved",
        }
    }

    /// What is wrong, and what resolves it.
    pub fn message(self) -> (&'static str, &'static str) {
        match self {
            IssueCode::Overlap => (
                "İki duvar aynı doğrultuda üst üste geliyor; birleştirilmedi.",
                "Duvarlardan birinin ucunu Yok yapın ya da duvarları Birleştir ile tek duvar yapın.",
            ),
            IssueCode::Crowded => (
                "Bu noktada ikiden çok duvar ucu buluşuyor ve ikisi bir doğru üzerinde değil; birleştirilmedi.",
                "En kısa duvarın ucunu Yok yapın ya da duvarlardan birini diğerinin ortasına uzatın.",
            ),
            IssueCode::Acute => (
                "Köşe çok dar; gönye birleşimi duvarın çok dışına taşar, birleştirilmedi.",
                "Bir ucu Alın yapın: o duvar diğerinin yüzünde durur.",
            ),
            IssueCode::Unresolved => (
                "Birleşimin çizgileri kesilemedi: duvar birleşimden kısa ya da diğer duvar birleşimden önce bitiyor.",
                "Duvarı uzatın ya da bu ucu Yok yapın.",
            ),
        }
    }
}

/// A join mode that resolves an issue: the end of one wall set to it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fix {
    pub wall: usize,
    pub end: End,
    pub mode: JoinMode,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    pub code: IssueCode,
    pub at: Vec2,
    pub walls: Vec<usize>,
    pub fix: Option<Fix>,
}

/// The walls' drawings (for those asked, none for a wall that cannot be drawn), their joints and what is not joined.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Solved {
    pub drawn: Vec<Option<Drawn>>,
    pub joints: Vec<Joint>,
    pub issues: Vec<Issue>,
}

fn add(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x + b.x, a.y + b.y)
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

fn mul(a: Vec2, k: f64) -> Vec2 {
    Vec2::new(a.x * k, a.y * k)
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    js_hypot(a.x - b.x, a.y - b.y)
}

fn unit(v: Vec2) -> Option<Vec2> {
    let l = js_hypot(v.x, v.y);
    (l > 1e-12).then(|| Vec2::new(v.x / l, v.y / l))
}

fn left_of(t: Vec2) -> Vec2 {
    Vec2::new(-t.y, t.x)
}

fn cos_deg(d: f64) -> f64 {
    cos(d * PI / 180.0)
}

/// A line's points and per-vertex bulges, the end to cut last.
fn oriented(l: &Line, end: End) -> (Vec<Vec2>, Vec<f64>) {
    let n = l.pts.len();
    match end {
        End::End => (l.pts.clone(), l.bulges.clone()),
        End::Start => {
            let r = reverse_bulge_path(&l.pts, Some(&l.bulges), false);
            let b = r.bulges.unwrap_or_else(|| vec![0.0; n]);
            (r.pts, b)
        }
    }
}

fn line_of_shape(shape: &Shape, n_hint: usize) -> Option<Line> {
    match shape {
        Shape::Line { a, b } => Some(Line {
            pts: vec![*a, *b],
            bulges: vec![0.0, 0.0],
        }),
        Shape::Polyline { pts, bulges, .. } => {
            let mut b = bulges.clone().unwrap_or_default();
            b.resize(pts.len(), 0.0);
            if let Some(last) = b.last_mut() {
                *last = 0.0;
            }
            let _ = n_hint;
            Some(Line {
                pts: pts.clone(),
                bulges: b,
            })
        }
        _ => None,
    }
}

fn open_shape(pts: Vec<Vec2>, bulges: Vec<f64>) -> Shape {
    Shape::Polyline {
        pts,
        bulges: Some(bulges),
        holes: None,
        parts: None,
        closed: false,
        widths: None,
    }
}

fn line_path(l: &Line, closed: bool) -> Option<(Path, Shape)> {
    let shape = l.shape(closed);
    path_of(&shape).map(|p| (p, shape))
}

/// The line `l` cut at `end` where it meets `target`: trimmed back to the crossing nearest its end, or extended along
/// its end's tangent to the nearest edge ahead, whichever moves the end less, within `reach`; none when neither is.
fn cut_end(l: &Line, end: End, target: &[Edge], reach: f64) -> Option<Line> {
    let (pts, bulges) = oriented(l, end);
    let n = pts.len();
    if n < 2 {
        return None;
    }
    let shape = open_shape(pts.clone(), bulges.clone());
    let path = path_of(&shape)?;
    let last = pts[n - 1];
    let tan = segment_tangent(pts[n - 2], pts[n - 1], bulges[n - 2], true);
    let eps = 1e-9 * js_max(path.length, 1.0);
    // (moved by, trim at s or extend by d)
    let mut best: Option<(f64, bool, f64)> = None;
    for (i, ed) in path.edges.iter().enumerate() {
        let len = edge_length(ed);
        for t in target {
            for h in intersect_edges(ed, t) {
                let s = path.cum[i] + h.t * len;
                let moved = js_max(path.length - s, 0.0);
                if s > eps && moved <= reach && best.is_none_or(|b| moved < b.0) {
                    best = Some((moved, true, s));
                }
            }
        }
    }
    for t in target {
        for d in ray_edge(last, tan, t, 0.0) {
            if d <= reach && best.is_none_or(|b| d < b.0) {
                best = Some((d, false, d));
            }
        }
    }
    let (moved, trim, v) = best?;
    let (mut p, mut b) = if moved <= eps {
        (pts, bulges)
    } else if trim {
        let cut = line_of_shape(&sub_path(&path, 0.0, v, &shape).shape, n)?;
        (cut.pts, cut.bulges)
    } else {
        let (mut p, mut b) = (pts, bulges);
        let to = add(last, mul(tan, v));
        if b[n - 2] == 0.0 {
            p[n - 1] = to;
        } else {
            p.push(to);
            b[n - 1] = 0.0;
            b.push(0.0);
        }
        (p, b)
    };
    if let Some(x) = b.last_mut() {
        *x = 0.0;
    }
    if end == End::Start {
        let r = reverse_bulge_path(&p, Some(&b), false);
        let len = r.pts.len();
        p = r.pts;
        b = r.bulges.unwrap_or_else(|| vec![0.0; len]);
    }
    Some(Line { pts: p, bulges: b })
}

/// A line's edge at an end.
fn end_edge(l: &Line, end: End) -> Option<Edge> {
    let edges = bulge_path_edges(&l.pts, Some(&l.bulges), false);
    match end {
        End::Start => edges.first().copied(),
        End::End => edges.last().copied(),
    }
}

/// Where the carriers of two edges (a segment's line, an arc's circle) meet, nearest `near`.
fn meet(e1: Edge, e2: Edge, near: Vec2) -> Option<Vec2> {
    let on_line = |a: Vec2, b: Vec2, c: Vec2, r: f64| -> Vec<Vec2> {
        line_circle_params(a, b, c, r)
            .into_iter()
            .map(|t| add(a, mul(sub(b, a), t)))
            .collect()
    };
    let found: Vec<Vec2> = match (e1, e2) {
        (Edge::Seg { a, b }, Edge::Seg { a: c, b: d }) => {
            line_line(a, b, c, d).map(|h| h.p).into_iter().collect()
        }
        (Edge::Seg { a, b }, Edge::Arc { c, r, .. })
        | (Edge::Arc { c, r, .. }, Edge::Seg { a, b }) => on_line(a, b, c, r),
        (Edge::Arc { c, r, .. }, Edge::Arc { c: c2, r: r2, .. }) => circle_circle(c, r, c2, r2),
    };
    found
        .into_iter()
        .min_by(|p, q| dist(*p, near).total_cmp(&dist(*q, near)))
}

/// A segment through `p` and `q` running on `ext` past both.
fn long_seg(p: Vec2, q: Vec2, ext: f64) -> Option<Edge> {
    let u = unit(sub(q, p))?;
    Some(Edge::Seg {
        a: sub(p, mul(u, ext)),
        b: add(q, mul(u, ext)),
    })
}

/// The carrier of a line's end edge, as far as a cut may reach: a long segment, or the arc's whole circle.
fn carrier(l: &Line, end: End, ext: f64) -> Option<Edge> {
    match end_edge(l, end)? {
        Edge::Seg { a, b } => long_seg(a, b, ext),
        Edge::Arc { c, r, .. } => Some(Edge::Arc {
            c,
            r,
            a0: 0.0,
            sweep: TAU,
        }),
    }
}

/// An interval of a line to leave out (or cut out of a band): from `a` to `b` along it, `mid` a point of the
/// interval's own (a closed line's interval may run past its seam).
#[derive(Clone, Copy, Debug)]
struct Span {
    a: f64,
    b: f64,
    mid: f64,
}

/// Spans normalised, merged and in order: on an open line within it, on a closed one starting in [0, L) and running
/// past L when they wrap.
fn spans(path: &Path, raw: &[Span]) -> Vec<(f64, f64)> {
    let l = path.length;
    let mut v: Vec<(f64, f64)> = raw
        .iter()
        .map(|s| {
            let (lo, hi) = if s.a <= s.b { (s.a, s.b) } else { (s.b, s.a) };
            if path.closed && !(lo - 1e-9..=hi + 1e-9).contains(&s.mid) {
                (hi, lo + l)
            } else if path.closed {
                (lo, hi)
            } else {
                (js_max(lo, 0.0), js_min(hi, l))
            }
        })
        .filter(|(a, b)| b > a)
        .collect();
    v.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut out: Vec<(f64, f64)> = Vec::new();
    for (a, b) in v {
        match out.last_mut() {
            Some(last) if a <= last.1 => last.1 = js_max(last.1, b),
            _ => out.push((a, b)),
        }
    }
    if path.closed && out.len() > 1 {
        // The last running past the seam may cover the first.
        let first = out[0];
        let n = out.len();
        if out[n - 1].1 >= first.0 + l {
            out[n - 1].1 = js_max(out[n - 1].1, first.1 + l);
            out.remove(0);
        }
    }
    out
}

/// What is left of a line outside its spans: on an open line the stretches between them, on a closed one round from
/// the last to the first.
fn kept(path: &Path, gaps: &[(f64, f64)]) -> Vec<(f64, f64)> {
    let l = path.length;
    let eps = 1e-9 * js_max(l, 1.0);
    let mut out = Vec::new();
    if path.closed {
        for i in 0..gaps.len() {
            let from = gaps[i].1;
            let to = if i + 1 < gaps.len() {
                gaps[i + 1].0
            } else {
                gaps[0].0 + l
            };
            if to - from > eps {
                out.push((from, to));
            }
        }
    } else {
        let mut at = 0.0;
        for &(a, b) in gaps {
            if a - at > eps {
                out.push((at, a));
            }
            at = b;
        }
        if l - at > eps {
            out.push((at, l));
        }
    }
    out
}

fn sub_line(path: &Path, shape: &Shape, s0: f64, s1: f64) -> Option<Line> {
    line_of_shape(&sub_path(path, s0, s1, shape).shape, 0)
}

fn strokes(l: &Line, closed: bool, raw: &[Span]) -> Vec<Stroke> {
    if raw.is_empty() {
        return vec![Stroke {
            line: l.clone(),
            closed,
        }];
    }
    let Some((path, shape)) = line_path(l, closed) else {
        return Vec::new();
    };
    let gaps = spans(&path, raw);
    kept(&path, &gaps)
        .into_iter()
        .filter_map(|(s0, s1)| sub_line(&path, &shape, s0, s1))
        .map(|line| Stroke {
            line,
            closed: false,
        })
        .collect()
}

/// The band between two lines less the stretches cut out of it, each a pair of spans (on `a`, on `b`).
fn bands_between(a: &Line, b: &Line, closed: bool, cuts: &[(Span, Span)]) -> Vec<Body> {
    if cuts.is_empty() {
        return vec![wall::band(a, b, closed)];
    }
    let (Some((pa, sa)), Some((pb, sb))) = (line_path(a, closed), line_path(b, closed)) else {
        return Vec::new();
    };
    // Pairs in order along `a`, merged where they overlap there.
    let mut pairs: Vec<((f64, f64), (f64, f64))> = cuts
        .iter()
        .filter_map(|(x, y)| Some((*spans(&pa, &[*x]).first()?, *spans(&pb, &[*y]).first()?)))
        .collect();
    pairs.sort_by(|x, y| x.0.0.total_cmp(&y.0.0));
    let mut merged: Vec<((f64, f64), (f64, f64))> = Vec::new();
    for p in pairs {
        match merged.last_mut() {
            Some(last) if p.0.0 <= last.0.1 => {
                last.0.1 = js_max(last.0.1, p.0.1);
                last.1.0 = js_min(last.1.0, p.1.0);
                last.1.1 = js_max(last.1.1, p.1.1);
            }
            _ => merged.push(p),
        }
    }
    let ka = kept(&pa, &merged.iter().map(|p| p.0).collect::<Vec<_>>());
    let kb = kept(&pb, &merged.iter().map(|p| p.1).collect::<Vec<_>>());
    if ka.len() != kb.len() {
        return vec![wall::band(a, b, closed)];
    }
    ka.into_iter()
        .zip(kb)
        .filter_map(|((a0, a1), (b0, b1))| {
            let la = sub_line(&pa, &sa, a0, a1)?;
            let lb = sub_line(&pb, &sb, b0, b1)?;
            let mut ring = la.outline(false);
            ring.extend(lb.outline(false).into_iter().rev());
            (ring.len() >= 3).then_some((ring, None))
        })
        .collect()
}

/// Where `other` crosses the line `path` (its own edges, and when `extend` straight on past both its ends), the
/// crossing nearest `near`, as the arc length along `path`.
fn cross_s(path: &Path, other: &Line, near: Vec2, extend: f64) -> Option<f64> {
    let mut edges = bulge_path_edges(&other.pts, Some(&other.bulges), false);
    if extend > 0.0 && other.pts.len() >= 2 {
        let n = other.pts.len();
        let t0 = segment_tangent(other.pts[0], other.pts[1], other.bulges[0], false);
        let t1 = segment_tangent(
            other.pts[n - 2],
            other.pts[n - 1],
            other.bulges[n - 2],
            true,
        );
        edges.push(Edge::Seg {
            a: sub(other.pts[0], mul(t0, extend)),
            b: other.pts[0],
        });
        edges.push(Edge::Seg {
            a: other.pts[n - 1],
            b: add(other.pts[n - 1], mul(t1, extend)),
        });
    }
    let mut best: Option<(f64, f64)> = None;
    for (i, ed) in path.edges.iter().enumerate() {
        let len = edge_length(ed);
        for o in &edges {
            for h in intersect_edges(ed, o) {
                let d = dist(h.p, near);
                if best.is_none_or(|b| d < b.0) {
                    best = Some((d, path.cum[i] + h.t * len));
                }
            }
        }
    }
    best.map(|b| b.1)
}

/// An end of a wall: where its path ends and the direction into the wall.
#[derive(Clone, Copy, Debug)]
struct EndAt {
    wall: usize,
    end: End,
    at: Vec2,
    dir: Vec2,
}

/// A tee to make: the joining end, its hosts (more than one where it meets two walls joined inline), and whether to
/// list it as a joint of its own (a butt corner's tee is the corner's).
struct PendingTee {
    joiner: EndAt,
    hosts: Vec<usize>,
    listed: bool,
}

/// A tee made: what its gaps need.
struct Tee {
    joiner: usize,
    at: Vec2,
    dir: Vec2,
    hosts: Vec<usize>,
    depth: Vec<usize>,
    cleanup: bool,
}

/// A cross: the host, the wall crossing it, where.
struct Cross {
    host: usize,
    crosser: usize,
    at: Vec2,
}

struct Solver<'a> {
    walls: &'a [WallIn<'a>],
    lines: Vec<Option<Vec<Line>>>,
    joined: Vec<[bool; 2]>,
    gaps: Vec<Vec<Vec<Span>>>,
    cuts: Vec<Vec<Vec<(Span, Span)>>>,
    tees: Vec<Tee>,
    crosses: Vec<Cross>,
    joints: Vec<Joint>,
    issues: Vec<Issue>,
    /// Each wall's openings' intervals along its base line (docs/adr/0341 §2).
    openings: Vec<Vec<(f64, f64)>>,
}

/// The wall's base line as a path.
fn base_path(w: &WallIn<'_>) -> Option<Path> {
    path_of(&wall::path_shape(w.pts, w.bulges, w.spec.closed))
}

fn ends_of(w: &WallIn<'_>) -> Option<[(Vec2, Vec2); 2]> {
    let n = w.pts.len();
    if n < 2 || w.spec.closed {
        return None;
    }
    let b = |i: usize| w.bulges.and_then(|b| b.get(i)).copied().unwrap_or(0.0);
    let t0 = segment_tangent(w.pts[0], w.pts[1], b(0), false);
    let t1 = segment_tangent(w.pts[n - 2], w.pts[n - 1], b(n - 2), true);
    Some([(w.pts[0], t0), (w.pts[n - 1], mul(t1, -1.0))])
}

/// The box of a wall's lines, or of its path.
fn bounds_of(w: &WallIn<'_>, lines: Option<&Vec<Line>>) -> [f64; 4] {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut take = |p: Vec2| {
        b[0] = js_min(b[0], p.x);
        b[1] = js_min(b[1], p.y);
        b[2] = js_max(b[2], p.x);
        b[3] = js_max(b[3], p.y);
    };
    match lines {
        Some(ls) => {
            for l in ls {
                for p in l.outline(w.spec.closed) {
                    take(p);
                }
            }
        }
        None => w.pts.iter().copied().for_each(&mut take),
    }
    b
}

/// The pairs of walls whose boxes (grown by `grow`) meet, each once, by a sweep along x.
fn near_pairs(boxes: &[[f64; 4]], grow: f64) -> Vec<(usize, usize)> {
    let mut order: Vec<usize> = (0..boxes.len())
        .filter(|&i| boxes[i][0].is_finite())
        .collect();
    order.sort_by(|&i, &j| boxes[i][0].total_cmp(&boxes[j][0]));
    let mut out = Vec::new();
    for (k, &i) in order.iter().enumerate() {
        for &j in &order[k + 1..] {
            if boxes[j][0] > boxes[i][2] + grow {
                break;
            }
            if boxes[j][1] <= boxes[i][3] + grow && boxes[i][1] <= boxes[j][3] + grow {
                out.push((i.min(j), i.max(j)));
            }
        }
    }
    out
}

impl<'a> Solver<'a> {
    fn spec(&self, i: usize) -> &Spec {
        self.walls[i].spec
    }

    /// The priorities of a wall's components from its left face; a wall without fitting components one of the rule.
    fn ranks(&self, i: usize) -> Vec<u8> {
        let n = self.lines[i].as_ref().map_or(0, Vec::len);
        let comps = &self.spec(i).components;
        if !comps.is_empty() && n == comps.len() + 1 {
            comps.iter().map(wall::Component::rank).collect()
        } else {
            vec![DEFAULT_PRIORITY]
        }
    }

    fn mode(&self, e: &EndAt) -> JoinMode {
        self.spec(e.wall).joins[e.end.index()]
    }

    fn reach(&self, a: usize, b: usize) -> f64 {
        REACH * (self.spec(a).width + self.spec(b).width) + 0.01
    }

    fn issue(&mut self, code: IssueCode, at: Vec2, walls: Vec<usize>, fix: Option<Fix>) {
        self.issues.push(Issue {
            code,
            at,
            walls,
            fix,
        });
    }

    /// Every line of wall `w` cut at `end` to `target`; none (nothing changed) when one cannot be.
    fn cut_all(&self, w: usize, end: End, target: &[Edge], reach: f64) -> Option<Vec<Line>> {
        self.lines[w]
            .as_ref()?
            .iter()
            .map(|l| cut_end(l, end, target, reach))
            .collect()
    }

    fn unresolved(&mut self, e: &EndAt, others: &[usize]) {
        let mut walls = vec![e.wall];
        walls.extend_from_slice(others);
        self.issue(
            IssueCode::Unresolved,
            e.at,
            walls,
            Some(Fix {
                wall: e.wall,
                end: e.end,
                mode: JoinMode::None,
            }),
        );
    }

    fn corner(&mut self, a: EndAt, b: EndAt, tees: &mut Vec<PendingTee>) {
        let cosv = dot(a.dir, b.dir);
        let (ma, mb) = (self.mode(&a), self.mode(&b));
        if cosv > cos_deg(PARALLEL_DEGREES) {
            let fix = Fix {
                wall: b.wall,
                end: b.end,
                mode: JoinMode::None,
            };
            self.issue(IssueCode::Overlap, a.at, vec![a.wall, b.wall], Some(fix));
            return;
        }
        if cosv < -cos_deg(INLINE_DEGREES) {
            self.inline(a, b);
            return;
        }
        let butt_a = (ma == JoinMode::Butt && mb != JoinMode::Butt)
            || (mb == JoinMode::Through && ma != JoinMode::Through);
        let butt_b = (mb == JoinMode::Butt && ma != JoinMode::Butt)
            || (ma == JoinMode::Through && mb != JoinMode::Through);
        if butt_a != butt_b {
            let (butting, through) = if butt_a { (a, b) } else { (b, a) };
            self.butt(butting, through, tees);
            return;
        }
        let theta = acos(cosv.clamp(-1.0, 1.0)) * 180.0 / PI;
        if theta < ACUTE_DEGREES && ma == JoinMode::Auto && mb == JoinMode::Auto {
            let fix = Fix {
                wall: a.wall,
                end: a.end,
                mode: JoinMode::Butt,
            };
            self.issue(IssueCode::Acute, a.at, vec![a.wall, b.wall], Some(fix));
            return;
        }
        self.miter(a, b);
    }

    /// The line of the most-left and of the most-right relative to the direction into the wall at `end`.
    fn faces(&self, w: usize, end: End) -> (usize, usize) {
        let last = self.lines[w].as_ref().map_or(0, |l| l.len() - 1);
        match end {
            End::Start => (0, last),
            End::End => (last, 0),
        }
    }

    fn miter(&mut self, a: EndAt, b: EndAt) {
        let (Some(la), Some(lb)) = (self.lines[a.wall].clone(), self.lines[b.wall].clone()) else {
            return;
        };
        let reach = self.reach(a.wall, b.wall);
        let ((amax, amin), (bmax, bmin)) = (self.faces(a.wall, a.end), self.faces(b.wall, b.end));
        let x1 = end_edge(&la[amax], a.end)
            .zip(end_edge(&lb[bmin], b.end))
            .and_then(|(p, q)| meet(p, q, a.at));
        let x2 = end_edge(&la[amin], a.end)
            .zip(end_edge(&lb[bmax], b.end))
            .and_then(|(p, q)| meet(p, q, a.at));
        let target = match (x1, x2) {
            (Some(p), Some(q)) if dist(p, q) > 1e-9 => long_seg(p, q, reach),
            _ => unit(add(a.dir, b.dir)).and_then(|u| long_seg(a.at, add(a.at, u), reach)),
        };
        let Some(target) = target else {
            self.unresolved(&a, &[b.wall]);
            return;
        };
        let Some(na) = self.cut_all(a.wall, a.end, &[target], reach) else {
            self.unresolved(&a, &[b.wall]);
            return;
        };
        let before = self.lines[a.wall].replace(na);
        let Some(nb) = self.cut_all(b.wall, b.end, &[target], reach) else {
            self.lines[a.wall] = before;
            self.unresolved(&b, &[a.wall]);
            return;
        };
        self.lines[b.wall] = Some(nb);
        self.joined[a.wall][a.end.index()] = true;
        self.joined[b.wall][b.end.index()] = true;
        self.joints.push(Joint {
            kind: JointKind::Corner,
            at: a.at,
            members: vec![
                Member {
                    wall: a.wall,
                    end: Some(a.end),
                },
                Member {
                    wall: b.wall,
                    end: Some(b.end),
                },
            ],
        });
    }

    /// The distances of a wall's faces to the left of the direction into it at `end`.
    fn rel_faces(&self, w: usize, end: End) -> (f64, f64) {
        let d = wall::boundaries(self.spec(w));
        let (l, r) = (d[0], d[d.len() - 1]);
        match end {
            End::Start => (l, r),
            End::End => (-r, -l),
        }
    }

    fn inline(&mut self, a: EndAt, b: EndAt) {
        let reach = self.reach(a.wall, b.wall);
        let n = left_of(a.dir);
        let Some(target) = long_seg(a.at, add(a.at, n), reach) else {
            return;
        };
        let Some(na) = self.cut_all(a.wall, a.end, &[target], reach) else {
            self.unresolved(&a, &[b.wall]);
            return;
        };
        let before = self.lines[a.wall].replace(na);
        let Some(nb) = self.cut_all(b.wall, b.end, &[target], reach) else {
            self.lines[a.wall] = before;
            self.unresolved(&b, &[a.wall]);
            return;
        };
        self.lines[b.wall] = Some(nb);
        // The faces run on into each other when they lie as far to either side: no line across.
        let (al, ar) = self.rel_faces(a.wall, a.end);
        let (bl, br) = self.rel_faces(b.wall, b.end);
        if (al + br).abs() < 1e-9 && (ar + bl).abs() < 1e-9 {
            self.joined[a.wall][a.end.index()] = true;
            self.joined[b.wall][b.end.index()] = true;
        }
        self.joints.push(Joint {
            kind: JointKind::Inline,
            at: a.at,
            members: vec![
                Member {
                    wall: a.wall,
                    end: Some(a.end),
                },
                Member {
                    wall: b.wall,
                    end: Some(b.end),
                },
            ],
        });
    }

    /// `butting` stops at `through`'s face; `through` runs on to `butting`'s far face.
    fn butt(&mut self, butting: EndAt, through: EndAt, tees: &mut Vec<PendingTee>) {
        let Some(lb) = self.lines[butting.wall].clone() else {
            return;
        };
        let reach = self.reach(butting.wall, through.wall);
        let last = lb.len() - 1;
        let tip = |l: &Line| match butting.end {
            End::Start => l.pts[0],
            End::End => l.pts[l.pts.len() - 1],
        };
        let far = if dot(sub(tip(&lb[0]), through.at), through.dir)
            <= dot(sub(tip(&lb[last]), through.at), through.dir)
        {
            0
        } else {
            last
        };
        let Some(target) = carrier(&lb[far], butting.end, reach) else {
            self.unresolved(&through, &[butting.wall]);
            return;
        };
        let Some(nt) = self.cut_all(through.wall, through.end, &[target], reach) else {
            self.unresolved(&through, &[butting.wall]);
            return;
        };
        // Its end line stays: it closes the corner's outside across the through wall.
        self.lines[through.wall] = Some(nt);
        self.joints.push(Joint {
            kind: JointKind::Corner,
            at: butting.at,
            members: vec![
                Member {
                    wall: butting.wall,
                    end: Some(butting.end),
                },
                Member {
                    wall: through.wall,
                    end: Some(through.end),
                },
            ],
        });
        tees.push(PendingTee {
            joiner: butting,
            hosts: vec![through.wall],
            listed: false,
        });
    }

    /// The host an end lies in: between its faces, square to its path; the one it lies most centrally in.
    fn host_of(&mut self, e: &EndAt, near: &[usize]) -> Option<usize> {
        let mut best: Option<(f64, usize)> = None;
        for &h in near {
            if h == e.wall || self.lines[h].is_none() {
                continue;
            }
            let w = &self.walls[h];
            let Some(path) = base_path(w) else { continue };
            let s = nearest_s(&path, e.at);
            // Not at an open host's end: there two ends meet, or the host's end does not join.
            if !path.closed && (s <= JOIN_TOLERANCE || s >= path.length - JOIN_TOLERANCE) {
                continue;
            }
            let q = point_at_s(&path, s);
            let t = tangent_at_s(&path, s);
            let off = sub(e.at, q);
            if dot(off, t).abs() > JOIN_TOLERANCE {
                continue;
            }
            let across = dot(off, left_of(t));
            let lf = wall::left_face(w.spec);
            let rf = lf - w.spec.width;
            if across > lf + JOIN_TOLERANCE || across < rf - JOIN_TOLERANCE {
                continue;
            }
            if dot(e.dir, t).abs() > cos_deg(PARALLEL_DEGREES) {
                // Said once for the two walls, however many of their ends lie in each other.
                let said = self.issues.iter().any(|x| {
                    x.code == IssueCode::Overlap
                        && x.walls.contains(&e.wall)
                        && x.walls.contains(&h)
                });
                if !said {
                    let fix = Fix {
                        wall: e.wall,
                        end: e.end,
                        mode: JoinMode::None,
                    };
                    self.issue(IssueCode::Overlap, e.at, vec![e.wall, h], Some(fix));
                }
                return None;
            }
            let central = (across - (lf + rf) / 2.0).abs();
            if best.is_none_or(|b| central < b.0) {
                best = Some((central, h));
            }
        }
        best.map(|b| b.1)
    }

    /// From the side of the host `dir` comes from: the host's line index at each position from the near face, and
    /// the host's priorities from the near face.
    fn host_side(&self, host: usize, at: Vec2, dir: Vec2) -> Option<(Vec<usize>, Vec<u8>)> {
        let path = base_path(&self.walls[host])?;
        let t = tangent_at_s(&path, nearest_s(&path, at));
        let left = dot(dir, left_of(t)) > 0.0;
        let n = self.lines[host].as_ref()?.len() - 1;
        let mut ranks = self.ranks(host);
        let lines: Vec<usize> = if left {
            (0..=n).collect()
        } else {
            ranks.reverse();
            (0..=n).rev().collect()
        };
        Some((lines, ranks))
    }

    /// How many of the host's components (from the near face) each of the joining wall's passes.
    fn depths(&self, joiner: usize, host_ranks: &[u8], cleanup: bool) -> Vec<usize> {
        self.ranks(joiner)
            .iter()
            .map(|&p| {
                if cleanup {
                    host_ranks.iter().take_while(|&&q| q > p).count()
                } else {
                    0
                }
            })
            .collect()
    }

    fn tee(&mut self, p: PendingTee) {
        let e = p.joiner;
        let h = p.hosts[0];
        let (Some(lh), Some(lj)) = (self.lines[h].clone(), self.lines[e.wall].clone()) else {
            return;
        };
        let Some((pos, ranks)) = self.host_side(h, e.at, e.dir) else {
            return;
        };
        let cleanup = self.spec(e.wall).cleanup && self.spec(h).cleanup;
        let depth = self.depths(e.wall, &ranks, cleanup);
        let nj = lj.len() - 1;
        let line_depth = |k: usize| {
            if depth.len() != nj || k == 0 {
                depth[0]
            } else if k == nj {
                depth[nj - 1]
            } else {
                depth[k - 1].max(depth[k])
            }
        };
        let reach = self.reach(e.wall, h);
        // Each host's line at the depth (two walls joined inline both): its own side's.
        let mut sides = vec![(h, pos)];
        for &other in &p.hosts[1..] {
            if let Some((op, _)) = self.host_side(other, e.at, e.dir) {
                sides.push((other, op));
            }
        }
        let _ = &lh;
        let mut cut = Vec::with_capacity(lj.len());
        for (k, l) in lj.iter().enumerate() {
            let mut edges = Vec::new();
            for (w, pw) in &sides {
                let Some(target) = self.lines[*w]
                    .as_ref()
                    .and_then(|ls| ls.get(pw[line_depth(k).min(pw.len() - 1)]))
                else {
                    continue;
                };
                edges.extend(bulge_path_edges(
                    &target.pts,
                    Some(&target.bulges),
                    self.spec(*w).closed,
                ));
            }
            match cut_end(l, e.end, &edges, reach) {
                Some(c) => cut.push(c),
                None => {
                    self.unresolved(&e, &p.hosts);
                    return;
                }
            }
        }
        self.lines[e.wall] = Some(cut);
        self.joined[e.wall][e.end.index()] = true;
        if p.listed {
            let mut members = vec![Member {
                wall: e.wall,
                end: Some(e.end),
            }];
            members.extend(p.hosts.iter().map(|&w| Member { wall: w, end: None }));
            self.joints.push(Joint {
                kind: JointKind::Tee,
                at: e.at,
                members,
            });
        }
        self.tees.push(Tee {
            joiner: e.wall,
            at: e.at,
            dir: e.dir,
            hosts: p.hosts,
            depth,
            cleanup,
        });
    }

    /// The host's lines and bands interrupted where `joiner`'s components, `depth` deep from the side `dir` comes
    /// from, pass or meet at the same priority (`extend`: the joining lines ran out to the host).
    #[allow(clippy::too_many_arguments)]
    fn host_cuts(
        &mut self,
        host: usize,
        joiner: usize,
        at: Vec2,
        dir: Vec2,
        depth: &[usize],
        cleanup: bool,
        extend: f64,
    ) {
        let (Some(lh), Some(lj)) = (self.lines[host].clone(), self.lines[joiner].clone()) else {
            return;
        };
        let Some((pos, ranks)) = self.host_side(host, at, dir) else {
            return;
        };
        let jr = self.ranks(joiner);
        let nh = lh.len() - 1;
        let nc = depth.len();
        if nc + 1 != lj.len() {
            return;
        }
        let closed = self.spec(host).closed;
        let runs = |open: &dyn Fn(usize) -> bool| -> Vec<(usize, usize)> {
            let mut out = Vec::new();
            let mut c = 0;
            while c < nc {
                if open(c) {
                    let from = c;
                    while c < nc && open(c) {
                        c += 1;
                    }
                    out.push((from, c));
                } else {
                    c += 1;
                }
            }
            out
        };
        let span_on = |line: &Line, lo: usize, hi: usize| -> Option<Span> {
            let (path, _) = line_path(line, closed)?;
            let a = cross_s(&path, &lj[lo], at, extend);
            let b = cross_s(&path, &lj[hi], at, extend);
            let mid = nearest_s(&path, at);
            let end_near = if dist(line.pts[0], at) <= dist(line.pts[line.pts.len() - 1], at) {
                0.0
            } else {
                path.length
            };
            match (a, b) {
                (Some(a), Some(b)) => Some(Span {
                    a,
                    b,
                    mid: (a + b) / 2.0,
                }),
                (Some(a), None) | (None, Some(a)) if !closed => Some(Span {
                    a,
                    b: end_near,
                    mid,
                }),
                _ => None,
            }
        };
        for m in 0..nh {
            let open = |c: usize| {
                m == 0
                    || (cleanup && (depth[c] > m || (depth[c] == m && ranks.get(m) == jr.get(c))))
            };
            for (lo, hi) in runs(&open) {
                if let Some(s) = span_on(&lh[pos[m]], lo, hi) {
                    self.gaps[host][pos[m]].push(s);
                }
            }
        }
        if self.cuts[host].len() != nh {
            return;
        }
        for q in 0..nh {
            let open = |c: usize| cleanup && depth[c] > q;
            for (lo, hi) in runs(&open) {
                let (near, far) = (pos[q], pos[q + 1]);
                let (Some(sn), Some(sf)) = (span_on(&lh[near], lo, hi), span_on(&lh[far], lo, hi))
                else {
                    continue;
                };
                let comp = near.min(far);
                let pair = if near < far { (sn, sf) } else { (sf, sn) };
                self.cuts[host][comp].push(pair);
            }
        }
    }

    fn cross(&mut self, x: &Cross) {
        let (Some(lh), Some(lc)) = (self.lines[x.host].clone(), self.lines[x.crosser].clone())
        else {
            return;
        };
        let Some(cpath) = base_path(&self.walls[x.crosser]) else {
            return;
        };
        let t = tangent_at_s(&cpath, nearest_s(&cpath, x.at));
        let cleanup = self.spec(x.host).cleanup && self.spec(x.crosser).cleanup;
        let mut sides = Vec::new();
        for dir in [mul(t, -1.0), t] {
            let Some((pos, ranks)) = self.host_side(x.host, x.at, dir) else {
                return;
            };
            let depth = self.depths(x.crosser, &ranks, cleanup);
            sides.push((dir, pos, depth));
        }
        let nh = lh.len() - 1;
        let nc = sides[0].2.len();
        let closed = self.spec(x.crosser).closed;
        if nc + 1 == lc.len() {
            let line_depth = |d: &[usize], k: usize| {
                if k == 0 {
                    d[0]
                } else if k == nc {
                    d[nc - 1]
                } else {
                    d[k - 1].max(d[k])
                }
            };
            let gap_on = |k: usize, da: usize, db: usize| -> Option<Span> {
                if da >= nh && db >= nh {
                    return None;
                }
                let (path, _) = line_path(&lc[k], closed)?;
                let a = cross_s(&path, &lh[sides[0].1[da]], x.at, 0.0)?;
                let b = cross_s(&path, &lh[sides[1].1[db]], x.at, 0.0)?;
                Some(Span {
                    a,
                    b,
                    mid: (a + b) / 2.0,
                })
            };
            for k in 0..=nc {
                let (da, db) = (line_depth(&sides[0].2, k), line_depth(&sides[1].2, k));
                if let Some(s) = gap_on(k, da, db) {
                    self.gaps[x.crosser][k].push(s);
                }
            }
            if self.cuts[x.crosser].len() == nc {
                for c in 0..nc {
                    let (da, db) = (sides[0].2[c], sides[1].2[c]);
                    if let (Some(a), Some(b)) = (gap_on(c, da, db), gap_on(c + 1, da, db)) {
                        self.cuts[x.crosser][c].push((a, b));
                    }
                }
            }
        }
        for (dir, _, depth) in sides {
            self.host_cuts(x.host, x.crosser, x.at, dir, &depth, cleanup, 0.0);
        }
    }

    /// Wall `i`'s openings on one edge of its base line within its ends (docs/adr/0341 §2): each its jambs and, on
    /// each of the wall's lines, the span it leaves out.
    fn opening_spans(&self, i: usize, lines: &[Line]) -> Vec<([Vec2; 4], Vec<Span>)> {
        let Some(list) = self.openings.get(i).filter(|l| !l.is_empty()) else {
            return Vec::new();
        };
        let Some(base) = base_path(&self.walls[i]) else {
            return Vec::new();
        };
        let spec = self.spec(i);
        let offsets = wall::boundaries(spec);
        let paths: Vec<Option<(Path, Shape)>> = lines.iter().map(|l| line_path(l, spec.closed)).collect();
        list.iter()
            .filter_map(|&(s0, s1)| {
                let jambs = crate::geom::opening::jambs(&base, spec, s0, s1)?;
                let spans = offsets
                    .iter()
                    .zip(&paths)
                    .map(|(&d, lp)| {
                        let (lp, _) = lp.as_ref()?;
                        let on = |s: f64| nearest_s(lp, crate::geom::opening::beside(&base, s, d));
                        Some(Span {
                            a: on(s0),
                            b: on(s1),
                            mid: on((s0 + s1) / 2.0),
                        })
                    })
                    .collect::<Option<Vec<Span>>>()?;
                Some((jambs, spans))
            })
            .collect()
    }

    fn drawn(&self, i: usize) -> Option<Drawn> {
        let lines = self.lines[i].as_ref()?;
        let spec = self.spec(i);
        let closed = spec.closed;
        let mut d = wall::drawn_of(lines, spec);
        let holes = self.opening_spans(i, lines);
        let gaps_of = |k: usize| -> Vec<Span> {
            let mut g = self.gaps[i][k].clone();
            g.extend(holes.iter().map(|(_, spans)| spans[k]));
            g
        };
        d.lines = lines
            .iter()
            .enumerate()
            .map(|(k, l)| strokes(l, closed, &gaps_of(k)))
            .collect();
        d.caps.clear();
        if !closed && !spec.caps_open {
            let (left, right) = (&lines[0], &lines[lines.len() - 1]);
            if !self.joined[i][0] {
                d.caps.push((left.pts[0], right.pts[0]));
            }
            if !self.joined[i][1] {
                d.caps
                    .push((left.pts[left.pts.len() - 1], right.pts[right.pts.len() - 1]));
            }
        }
        if !d.bands.is_empty() {
            d.bands = (0..d.bands.len())
                .map(|c| {
                    let mut cuts = self.cuts[i][c].clone();
                    cuts.extend(holes.iter().map(|(_, spans)| (spans[c], spans[c + 1])));
                    bands_between(&lines[c], &lines[c + 1], closed, &cuts)
                })
                .collect();
        }
        if !holes.is_empty() {
            let last = lines.len() - 1;
            let cuts: Vec<(Span, Span)> = holes.iter().map(|(_, spans)| (spans[0], spans[last])).collect();
            d.fill = bands_between(&lines[0], &lines[last], closed, &cuts);
            for (j, _) in &holes {
                d.caps.push((j[0], j[1]));
                d.caps.push((j[2], j[3]));
            }
        }
        Some(d)
    }
}

/// The walls joined: their joints, what is not joined, and the drawings of those in `only` (all when none).
/// Only walls with the same explicit level (or both unassigned) can join.
pub fn solve(walls: &[WallIn<'_>], only: Option<&[usize]>) -> Solved {
    solve_with(walls, &[], only)
}

fn level(spec: &Spec) -> Option<&str> {
    match spec.arch.as_ref()?.get("level") {
        crate::api::json::Json::Str(id) => Some(id.as_str()),
        _ => None,
    }
}

/// [`solve`] with each wall's openings' intervals along its base line (docs/adr/0341 §2; a missing list: none): its
/// lines, bands and fill cut out between each opening's jambs and the jambs drawn.
pub fn solve_with(walls: &[WallIn<'_>], openings: &[Vec<(f64, f64)>], only: Option<&[usize]>) -> Solved {
    let n = walls.len();
    // A level is a semantic identity, not its elevation: equal elevations may
    // belong to different storeys. Unassigned walls retain their legacy group.
    let levels: Vec<_> = walls.iter().map(|w| level(w.spec)).collect();
    let lines: Vec<Option<Vec<Line>>> = walls
        .iter()
        .map(|w| wall::lines(w.pts, w.bulges, w.spec).ok())
        .collect();
    let mut s = Solver {
        walls,
        gaps: lines
            .iter()
            .map(|l| vec![Vec::new(); l.as_ref().map_or(0, Vec::len)])
            .collect(),
        cuts: lines
            .iter()
            .zip(walls)
            .map(|(l, w)| {
                let k = l.as_ref().map_or(0, Vec::len);
                if !w.spec.components.is_empty() && k == w.spec.components.len() + 1 {
                    vec![Vec::new(); k - 1]
                } else {
                    Vec::new()
                }
            })
            .collect(),
        lines,
        joined: vec![[false; 2]; n],
        tees: Vec::new(),
        crosses: Vec::new(),
        joints: Vec::new(),
        issues: Vec::new(),
        openings: openings.to_vec(),
    };
    let boxes: Vec<[f64; 4]> = (0..n)
        .map(|i| bounds_of(&walls[i], s.lines[i].as_ref()))
        .collect();
    let pairs: Vec<_> = near_pairs(&boxes, JOIN_TOLERANCE)
        .into_iter()
        .filter(|&(i, j)| levels[i] == levels[j])
        .collect();
    let mut near: Vec<Vec<usize>> = vec![Vec::new(); n];
    for &(i, j) in &pairs {
        near[i].push(j);
        near[j].push(i);
    }

    // The ends that may join, grouped where they meet.
    let mut ends: Vec<EndAt> = Vec::new();
    for (i, w) in walls.iter().enumerate() {
        if s.lines[i].is_none() {
            continue;
        }
        if let Some(two) = ends_of(w) {
            for (k, (at, dir)) in two.into_iter().enumerate() {
                let end = if k == 0 { End::Start } else { End::End };
                if w.spec.joins[k] != JoinMode::None {
                    ends.push(EndAt {
                        wall: i,
                        end,
                        at,
                        dir,
                    });
                }
            }
        }
    }
    let mut order: Vec<usize> = (0..ends.len()).collect();
    order.sort_by(|&a, &b| ends[a].at.x.total_cmp(&ends[b].at.x));
    let mut group: Vec<usize> = (0..ends.len()).collect();
    fn root(g: &mut [usize], mut i: usize) -> usize {
        while g[i] != i {
            g[i] = g[g[i]];
            i = g[i];
        }
        i
    }
    for (k, &a) in order.iter().enumerate() {
        for &b in &order[k + 1..] {
            if ends[b].at.x - ends[a].at.x > JOIN_TOLERANCE {
                break;
            }
            if levels[ends[a].wall] == levels[ends[b].wall]
                && dist(ends[a].at, ends[b].at) <= JOIN_TOLERANCE {
                let (ra, rb) = (root(&mut group, a), root(&mut group, b));
                group[ra.max(rb)] = ra.min(rb);
            }
        }
    }
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); ends.len()];
    for i in 0..ends.len() {
        let r = root(&mut group, i);
        groups[r].push(i);
    }

    let mut tees: Vec<PendingTee> = Vec::new();
    let mut singles: Vec<EndAt> = Vec::new();
    for g in groups.into_iter().filter(|g| !g.is_empty()) {
        match g.len() {
            1 => singles.push(ends[g[0]]),
            2 => s.corner(ends[g[0]], ends[g[1]], &mut tees),
            _ => {
                let mut best: Option<(f64, usize, usize)> = None;
                for (x, &a) in g.iter().enumerate() {
                    for &b in &g[x + 1..] {
                        let c = dot(ends[a].dir, ends[b].dir);
                        if c < -cos_deg(INLINE_DEGREES) && best.is_none_or(|bb| c < bb.0) {
                            best = Some((c, a, b));
                        }
                    }
                }
                match best {
                    Some((_, a, b)) => {
                        s.inline(ends[a], ends[b]);
                        for &k in g.iter().filter(|&&k| k != a && k != b) {
                            tees.push(PendingTee {
                                joiner: ends[k],
                                hosts: vec![ends[a].wall, ends[b].wall],
                                listed: true,
                            });
                        }
                    }
                    None => {
                        let shortest = g
                            .iter()
                            .copied()
                            .min_by(|&a, &b| {
                                let len =
                                    |e: &EndAt| base_path(&walls[e.wall]).map_or(0.0, |p| p.length);
                                len(&ends[a]).total_cmp(&len(&ends[b]))
                            })
                            .map(|k| ends[k]);
                        let at = ends[g[0]].at;
                        let ws = g.iter().map(|&k| ends[k].wall).collect();
                        let fix = shortest.map(|e| Fix {
                            wall: e.wall,
                            end: e.end,
                            mode: JoinMode::None,
                        });
                        s.issue(IssueCode::Crowded, at, ws, fix);
                    }
                }
            }
        }
    }
    for e in singles {
        if let Some(h) = s.host_of(&e, &near[e.wall].clone()) {
            tees.push(PendingTee {
                joiner: e,
                hosts: vec![h],
                listed: true,
            });
        }
    }
    // A butt's through wall is cut before any wall tees into it.
    for p in tees {
        s.tee(p);
    }

    // Paths that cross away from the walls' ends.
    for &(i, j) in &pairs {
        if s.lines[i].is_none() || s.lines[j].is_none() {
            continue;
        }
        let (wi, wj) = (&walls[i], &walls[j]);
        let ei = bulge_path_edges(wi.pts, wi.bulges, wi.spec.closed);
        let ej = bulge_path_edges(wj.pts, wj.bulges, wj.spec.closed);
        let clear = wi.spec.width
            + wj.spec.width
            + wi.spec.offset.abs()
            + wj.spec.offset.abs()
            + JOIN_TOLERANCE;
        let ends_near = |p: Vec2| {
            [wi, wj]
                .iter()
                .filter_map(|w| ends_of(w))
                .flatten()
                .any(|(e, _)| dist(e, p) <= clear)
        };
        let mut seen: Vec<Vec2> = Vec::new();
        for a in &ei {
            for b in &ej {
                for h in intersect_edges(a, b) {
                    if ends_near(h.p) || seen.iter().any(|q| dist(*q, h.p) <= JOIN_TOLERANCE) {
                        continue;
                    }
                    seen.push(h.p);
                    let strength = |k: usize| {
                        let r = s.ranks(k).into_iter().min().unwrap_or(DEFAULT_PRIORITY);
                        (r, -walls[k].spec.width, k)
                    };
                    let (si, sj) = (strength(i), strength(j));
                    let host_i = si.0 < sj.0
                        || (si.0 == sj.0 && (si.1 < sj.1 || (si.1 == sj.1 && si.2 < sj.2)));
                    let (host, crosser) = if host_i { (i, j) } else { (j, i) };
                    s.crosses.push(Cross {
                        host,
                        crosser,
                        at: h.p,
                    });
                    s.joints.push(Joint {
                        kind: JointKind::Cross,
                        at: h.p,
                        members: vec![
                            Member {
                                wall: host,
                                end: None,
                            },
                            Member {
                                wall: crosser,
                                end: None,
                            },
                        ],
                    });
                }
            }
        }
    }

    // The interruptions, on the lines as the cuts left them.
    let tee_list = std::mem::take(&mut s.tees);
    for t in &tee_list {
        let extend = s.reach(t.joiner, t.hosts[0]);
        for &h in &t.hosts {
            s.host_cuts(h, t.joiner, t.at, t.dir, &t.depth, t.cleanup, extend);
        }
    }
    let crosses = std::mem::take(&mut s.crosses);
    for x in &crosses {
        s.cross(x);
    }

    let wanted = |i: usize| only.is_none_or(|o| o.contains(&i));
    let drawn = (0..n)
        .map(|i| if wanted(i) { s.drawn(i) } else { None })
        .collect();
    Solved {
        drawn,
        joints: s.joints,
        issues: s.issues,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::wall::{Component, Justify};

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn plain(width: f64) -> Spec {
        Spec {
            width,
            justify: Justify::Center,
            ..Spec::default()
        }
    }

    fn near(a: Vec2, b: Vec2) -> bool {
        dist(a, b) < 1e-9
    }

    #[test]
    fn a_corner_is_mitred_and_has_no_caps() {
        let s = plain(0.2);
        let a = [v(0.0, 0.0), v(4.0, 0.0)];
        let b = [v(4.0, 0.0), v(4.0, 3.0)];
        let walls = [
            WallIn {
                pts: &a,
                bulges: None,
                spec: &s,
            },
            WallIn {
                pts: &b,
                bulges: None,
                spec: &s,
            },
        ];
        let got = solve(&walls, None);
        assert_eq!(got.joints.len(), 1);
        assert_eq!(got.joints[0].kind, JointKind::Corner);
        let da = got.drawn[0].as_ref().unwrap();
        // The left face (north) ends at the inner corner, the right face (south) at the outer one.
        assert!(near(*da.lines[0][0].line.pts.last().unwrap(), v(3.9, 0.1)));
        assert!(near(*da.lines[1][0].line.pts.last().unwrap(), v(4.1, -0.1)));
        assert_eq!(da.caps.len(), 1, "only the free end");
    }

    #[test]
    fn a_wall_tees_into_another_and_breaks_its_near_face() {
        let s = plain(0.2);
        let host = [v(0.0, 0.0), v(6.0, 0.0)];
        let joiner = [v(3.0, 4.0), v(3.0, 0.0)];
        let walls = [
            WallIn {
                pts: &host,
                bulges: None,
                spec: &s,
            },
            WallIn {
                pts: &joiner,
                bulges: None,
                spec: &s,
            },
        ];
        let got = solve(&walls, None);
        assert_eq!(got.joints[0].kind, JointKind::Tee);
        let h = got.drawn[0].as_ref().unwrap();
        // The north face in two, around the joining wall.
        assert_eq!(h.lines[0].len(), 2);
        assert!(near(*h.lines[0][0].line.pts.last().unwrap(), v(2.9, 0.1)));
        assert!(near(h.lines[0][1].line.pts[0], v(3.1, 0.1)));
        assert_eq!(h.lines[1].len(), 1);
        let j = got.drawn[1].as_ref().unwrap();
        assert!(near(*j.lines[0][0].line.pts.last().unwrap(), v(3.1, 0.1)));
        assert_eq!(j.caps.len(), 1);
    }

    #[test]
    fn a_stronger_core_passes_a_weaker_finish() {
        let c = |t: Option<f64>, p: u8| Component {
            thickness: t,
            priority: Some(p),
            ..Component::default()
        };
        let s = Spec {
            width: 0.2,
            components: vec![c(Some(0.02), 5), c(None, 1), c(Some(0.02), 5)],
            ..Spec::default()
        };
        let host = [v(0.0, 0.0), v(6.0, 0.0)];
        let joiner = [v(3.0, 4.0), v(3.0, 0.0)];
        let walls = [
            WallIn {
                pts: &host,
                bulges: None,
                spec: &s,
            },
            WallIn {
                pts: &joiner,
                bulges: None,
                spec: &s,
            },
        ];
        let got = solve(&walls, None);
        let j = got.drawn[1].as_ref().unwrap();
        // The core's lines reach the host's core (y = 0.08), the finish's faces stop at its face (y = 0.1).
        assert!((j.lines[0][0].line.pts.last().unwrap().y - 0.1).abs() < 1e-9);
        assert!((j.lines[1][0].line.pts.last().unwrap().y - 0.08).abs() < 1e-9);
        let h = got.drawn[0].as_ref().unwrap();
        // The host's finish line meets the joining core and is broken under it: the cores merge.
        assert_eq!(h.lines[1].len(), 2);
        assert_eq!(h.lines[2].len(), 1);
        // The host's finish band is cut where the joining core passes it.
        assert_eq!(h.bands[0].len(), 2);
    }

    #[test]
    fn crossing_walls_interrupt_the_weaker() {
        let s = plain(0.2);
        let a = [v(0.0, 0.0), v(6.0, 0.0)];
        let b = [v(3.0, -3.0), v(3.0, 3.0)];
        let walls = [
            WallIn {
                pts: &a,
                bulges: None,
                spec: &s,
            },
            WallIn {
                pts: &b,
                bulges: None,
                spec: &s,
            },
        ];
        let got = solve(&walls, None);
        assert_eq!(got.joints[0].kind, JointKind::Cross);
        // The first is the host by order: its faces are broken, the other's stop at them.
        let host = got.drawn[0].as_ref().unwrap();
        assert_eq!((host.lines[0].len(), host.lines[1].len()), (2, 2));
        let crosser = got.drawn[1].as_ref().unwrap();
        assert_eq!((crosser.lines[0].len(), crosser.lines[1].len()), (2, 2));
    }

    #[test]
    fn an_acute_corner_is_said_with_its_fix() {
        let s = plain(0.2);
        let a = [v(0.0, 0.0), v(4.0, 0.0)];
        let b = [v(4.0, 0.0), v(0.0, 0.5)];
        let walls = [
            WallIn {
                pts: &a,
                bulges: None,
                spec: &s,
            },
            WallIn {
                pts: &b,
                bulges: None,
                spec: &s,
            },
        ];
        let got = solve(&walls, None);
        assert!(got.joints.is_empty());
        assert_eq!(got.issues[0].code, IssueCode::Acute);
        assert_eq!(got.issues[0].fix.unwrap().mode, JoinMode::Butt);
    }
}
