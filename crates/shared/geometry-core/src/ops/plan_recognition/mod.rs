//! Planı tanı (plan/architecture-advanced.md M48, docs/adr/8048 §1): an exploded plan's line work — an office's DWG or
//! DXF whose walls, doors and windows are plain lines and arcs — read back as walls and their openings, as BricsCAD's
//! BIMIFY and Chief Architect's Convert CAD to Walls do. A line's role is its layer's (a wall's, a door's, a window's,
//! a column's); lengths are metres.
//!
//! - **Lines on one line are one line.** Wall lines whose directions are within `angle` of each other and that lie
//!   within `tol` of one line are one run where they overlap or meet: a face drawn in pieces, twice, or broken where
//!   nothing is.
//! - **A wall is a band.** Along each stretch where the same runs are present, the parallel runs are taken in their
//!   order across and grouped: a group is at most `max_width` wide, at least `min_width`, and has at most one gap
//!   wider than 0.1 m (its core; the narrower ones are its components' lines, an insulation's, a plaster's). Stretches
//!   of one band along its line are a piece; a band whose pieces are together no longer than it is wide is no wall
//!   (the two jambs of a narrow opening, a column drawn as lines).
//! - **What breaks a band.** Between two pieces of one band, in this order: a face running on is a joint; a door's
//!   swing (an arc about a jamb as wide as the gap, or two about both as wide as half of it) or a window's lines
//!   along it is that opening; a column, or another wall crossing or meeting it there, is a joint; a gap no wider
//!   than `max_opening` with nothing in it is a bare opening, a narrower one than 0.3 m a slip of the pen (the wall
//!   runs on); a wider one ends the wall. A joint's wall runs through.
//! - **Walls meet.** A wall's end within reach of another wall's line (their half widths, or the column it stops at)
//!   runs to it: to the other's line in a tee, both to their crossing at a corner; ADR 0340 §2 then joins them.
//! - **What is taken.** A wall's line within a wall, a door's or window's line and arc within its opening (a wall's
//!   line there too: a plan drawn on one layer); a source is taken when all of it is. A column is not made yet.

use std::f64::consts::PI;

use crate::Vec2;
use crate::jsmath::{atan2, cos, js_hypot, js_max, js_max_all, js_min, js_round, sin};

#[cfg(test)]
mod tests;

/// A component's line lies at most this far from its neighbour: a wall's core is its one wider gap, metres.
const LAYER: f64 = 0.1;
/// The narrowest bare opening: a narrower gap with nothing in it is a slip of the pen and the wall runs on, metres.
const BARE_MIN: f64 = 0.3;
/// Two walls meet at an angle wider than this, degrees.
const JOIN_ANGLE: f64 = 15.0;
/// A door's hinge lies this near its jamb along the wall and this far out of its faces, metres.
const HINGE: f64 = 0.15;
/// A window's lines lie this far out of its wall's faces and past its jambs (its sills), metres.
const SILL: f64 = 0.25;

/// What a source's line is taken as: its layer's role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Wall,
    Door,
    Window,
    Column,
}

/// A straight line of a source (a line, a polyline's edge).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Segment {
    pub a: Vec2,
    pub b: Vec2,
    pub role: Role,
    /// The source it is of.
    pub source: usize,
}

/// An arc of a source, counter-clockwise from `a0` over `sweep` radians.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arc {
    pub c: Vec2,
    pub r: f64,
    pub a0: f64,
    pub sweep: f64,
    pub role: Role,
    pub source: usize,
}

/// A closed outline of a source: a column's.
#[derive(Clone, Debug, PartialEq)]
pub struct Ring {
    pub pts: Vec<Vec2>,
    pub role: Role,
    pub source: usize,
}

/// The plan's line work.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lines {
    pub segments: Vec<Segment>,
    pub arcs: Vec<Arc>,
    pub rings: Vec<Ring>,
}

/// How the plan is read.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    /// The thinnest wall, metres.
    pub min_width: f64,
    /// The thickest wall, metres.
    pub max_width: f64,
    /// The widest opening, metres.
    pub max_opening: f64,
    /// Degrees two lines may be apart and be parallel.
    pub angle: f64,
    /// Lines this near are one line, ends this near meet, metres.
    pub tol: f64,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            min_width: 0.05,
            max_width: 0.6,
            max_opening: 3.0,
            angle: 1.0,
            tol: 0.005,
        }
    }
}

/// What an opening is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Door,
    Window,
    /// Boşluk: a gap and nothing in it.
    Opening,
}

/// An end of an opening along its wall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    /// Its jamb nearer the wall's start.
    Start,
    End,
}

/// A side of a wall's line, looking along it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// A wall: its centre line from `a` to `b`, its width and its first line (an index of the segments).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wall {
    pub a: Vec2,
    pub b: Vec2,
    pub width: f64,
    pub line: usize,
}

/// An opening in a wall.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Opening {
    /// Its wall, an index of the walls.
    pub wall: usize,
    /// Its jambs along its wall from the wall's start, metres.
    pub from: f64,
    pub to: f64,
    pub form: Form,
    /// A door's leaves: two when it swings from both jambs.
    pub leaves: u8,
    /// A door's hinge jamb.
    pub hand: End,
    /// The side a door opens to.
    pub side: Side,
    /// A door's swing in degrees, when its arc's is not 90°.
    pub swing: Option<f64>,
}

/// The plan read: its walls in the order of their first line, their openings by wall and along it, and the sources
/// taken whole (their indices, ascending).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recognized {
    pub walls: Vec<Wall>,
    pub openings: Vec<Opening>,
    pub used: Vec<usize>,
}

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn length(v: Vec2) -> f64 {
    js_hypot(v.x, v.y)
}

/// A number to the nanometre: a plan's coordinates are drawn to the millimetre and their sums carry nothing finer.
fn clean(x: f64) -> f64 {
    let r = js_round(x * 1e9) / 1e9;
    if r == 0.0 { 0.0 } else { r }
}

/// A direction and its left normal: a point is `t` along it and `rho` across.
#[derive(Clone, Copy, Debug)]
struct Frame {
    u: Vec2,
    n: Vec2,
}

impl Frame {
    fn of(u: Vec2) -> Self {
        Self {
            u,
            n: Vec2::new(-u.y, u.x),
        }
    }

    fn t(&self, p: Vec2) -> f64 {
        dot(p, self.u)
    }

    fn rho(&self, p: Vec2) -> f64 {
        dot(p, self.n)
    }

    fn at(&self, t: f64, rho: f64) -> Vec2 {
        Vec2::new(self.u.x * t + self.n.x * rho, self.u.y * t + self.n.y * rho)
    }
}

/// An interval's overlap with another.
fn overlap(a0: f64, a1: f64, b0: f64, b1: f64) -> f64 {
    let lo = if a0 > b0 { a0 } else { b0 };
    let hi = if a1 < b1 { a1 } else { b1 };
    hi - lo
}

/// The extent of points in a frame: `(t0, t1, rho0, rho1)`.
fn extent(f: &Frame, pts: impl IntoIterator<Item = Vec2>) -> (f64, f64, f64, f64) {
    let mut e = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for p in pts {
        let (t, r) = (f.t(p), f.rho(p));
        e.0 = js_min(e.0, t);
        e.1 = js_max(e.1, t);
        e.2 = js_min(e.2, r);
        e.3 = js_max(e.3, r);
    }
    e
}

/// The part of `a`→`b` inside the box `t0..t1 × r0..r1` of a frame, as the line's parameters (Liang–Barsky).
fn clip(f: &Frame, a: Vec2, b: Vec2, bx: (f64, f64, f64, f64)) -> Option<(f64, f64)> {
    let (ta, ra, tb, rb) = (f.t(a), f.rho(a), f.t(b), f.rho(b));
    let (dt, dr) = (tb - ta, rb - ra);
    let mut lo = 0.0_f64;
    let mut hi = 1.0_f64;
    for (p, q) in [
        (-dt, ta - bx.0),
        (dt, bx.1 - ta),
        (-dr, ra - bx.2),
        (dr, bx.3 - ra),
    ] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
            }
            continue;
        }
        let r = q / p;
        if p < 0.0 {
            lo = js_max(lo, r);
        } else {
            hi = js_min(hi, r);
        }
        if lo > hi {
            return None;
        }
    }
    Some((lo, hi))
}

/// Whether the parameters' intervals cover `0..1` but for `gap` of it.
fn covers(mut parts: Vec<(f64, f64)>, gap: f64) -> bool {
    parts.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut reach = 0.0_f64;
    for (lo, hi) in parts {
        if lo > reach + gap {
            return false;
        }
        reach = js_max(reach, hi);
    }
    reach >= 1.0 - gap
}

/// Groups of items sorted by `key`, each a chain whose neighbours lie within `tol`.
fn chains<T>(items: &[T], key: impl Fn(&T) -> f64, tol: f64) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let mut k = 0;
    while k < items.len() {
        let mut m = k + 1;
        while m < items.len() && key(&items[m]) - key(&items[m - 1]) <= tol {
            m += 1;
        }
        out.push(k..m);
        k = m;
    }
    out
}

/// The wall lines' directions: each class's frame (along its longest line, `t` growing east, else north) and its
/// lines (indices of the segments), each lying along it within `tol` from end to end. A line askew to its class's
/// longest is a class of its own.
fn classes(segments: &[Segment], o: &Options) -> Vec<(Frame, Vec<usize>)> {
    let angle = o.angle * PI / 180.0;
    let mut items: Vec<(f64, usize)> = segments
        .iter()
        .enumerate()
        .filter(|(_, s)| s.role == Role::Wall && length(sub(s.b, s.a)) > o.tol)
        .map(|(i, s)| {
            let mut th = atan2(s.b.y - s.a.y, s.b.x - s.a.x);
            if th < 0.0 {
                th += PI;
            }
            if th >= PI {
                th -= PI;
            }
            (th, i)
        })
        .collect();
    items.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut last = f64::NEG_INFINITY;
    for (th, i) in &items {
        match groups.last_mut() {
            Some(g) if th - last <= angle => g.push(*i),
            _ => groups.push(vec![*i]),
        }
        last = *th;
    }
    // The lines near π lie along those near 0.
    if groups.len() > 1
        && let (Some(first), Some(end)) = (items.first(), items.last())
        && first.0 + PI - end.0 <= angle
        && let Some(tail) = groups.pop()
    {
        groups[0].extend(tail);
    }
    let mut out = Vec::new();
    let frame_of = |i: usize| {
        let s = &segments[i];
        let d = sub(s.b, s.a);
        let l = length(d);
        let mut u = Vec2::new(d.x / l, d.y / l);
        if u.x < -1e-12 || (u.x.abs() <= 1e-12 && u.y < 0.0) {
            u = Vec2::new(-u.x, -u.y);
        }
        Frame::of(u)
    };
    for g in groups {
        let mut longest = g[0];
        for &i in &g {
            let (l, m) = (
                length(sub(segments[i].b, segments[i].a)),
                length(sub(segments[longest].b, segments[longest].a)),
            );
            if l > m || (l == m && i < longest) {
                longest = i;
            }
        }
        let f = frame_of(longest);
        let (along, askew): (Vec<usize>, Vec<usize>) = g
            .into_iter()
            .partition(|&i| (f.rho(segments[i].a) - f.rho(segments[i].b)).abs() <= 2.0 * o.tol);
        out.push((f, along));
        for i in askew {
            out.push((frame_of(i), vec![i]));
        }
    }
    out
}

/// Wall lines on one line that overlap or meet: their class, place across, extent along and lines.
#[derive(Clone, Debug)]
struct Run {
    class: usize,
    rho: f64,
    t0: f64,
    t1: f64,
    segs: Vec<usize>,
}

fn runs(segments: &[Segment], classes: &[(Frame, Vec<usize>)], tol: f64) -> Vec<Run> {
    let mut out = Vec::new();
    for (c, (f, members)) in classes.iter().enumerate() {
        let mut items: Vec<(f64, f64, f64, usize)> = members
            .iter()
            .map(|&i| {
                let s = &segments[i];
                let (ta, tb) = (f.t(s.a), f.t(s.b));
                let (t0, t1) = if ta <= tb { (ta, tb) } else { (tb, ta) };
                ((f.rho(s.a) + f.rho(s.b)) / 2.0, t0, t1, i)
            })
            .collect();
        items.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.3.cmp(&b.3)));
        for range in chains(&items, |x| x.0, tol) {
            let mut line = items[range].to_vec();
            line.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.3.cmp(&b.3)));
            // Each run with its length-weighted place across.
            let mut cur: Option<(Run, f64, f64)> = None;
            for (rho, t0, t1, i) in line {
                let w = js_max(t1 - t0, tol);
                if let Some((r, sum, weight)) = &mut cur
                    && t0 <= r.t1 + tol
                {
                    r.t1 = js_max(r.t1, t1);
                    r.segs.push(i);
                    *sum += rho * w;
                    *weight += w;
                    continue;
                }
                if let Some((mut r, sum, weight)) = cur.take() {
                    r.rho = sum / weight;
                    out.push(r);
                }
                cur = Some((
                    Run {
                        class: c,
                        rho,
                        t0,
                        t1,
                        segs: vec![i],
                    },
                    rho * w,
                    w,
                ));
            }
            if let Some((mut r, sum, weight)) = cur {
                r.rho = sum / weight;
                out.push(r);
            }
        }
    }
    out
}

/// The bands among parallel runs across a stretch (their places across, ascending): each its first and last.
fn bands(rhos: &[f64], o: &Options) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < rhos.len() {
        let mut j = i;
        let mut core = false;
        while j + 1 < rhos.len() {
            if rhos[j + 1] - rhos[i] > o.max_width + o.tol {
                break;
            }
            if rhos[j + 1] - rhos[j] > LAYER + o.tol {
                if core {
                    break;
                }
                core = true;
            }
            j += 1;
        }
        if j > i && rhos[j] - rhos[i] >= o.min_width - o.tol {
            out.push((i, j));
        }
        i = j + 1;
    }
    out
}

/// A stretch of one band along its line: its class, faces across, extent along, first line (the input's order).
#[derive(Clone, Debug)]
struct Piece {
    class: usize,
    lo: f64,
    hi: f64,
    t0: f64,
    t1: f64,
    first: usize,
}

fn pieces(runs: &[Run], classes: usize, o: &Options) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    for c in 0..classes {
        let mut rs: Vec<usize> = (0..runs.len()).filter(|&r| runs[r].class == c).collect();
        rs.sort_by(|&a, &b| runs[a].t0.total_cmp(&runs[b].t0).then(a.cmp(&b)));
        let mut ts: Vec<f64> = rs.iter().flat_map(|&r| [runs[r].t0, runs[r].t1]).collect();
        ts.sort_by(f64::total_cmp);
        let mut stops: Vec<f64> = Vec::new();
        for t in ts {
            if stops.last().is_none_or(|&l| t - l > o.tol) {
                stops.push(t);
            }
        }
        let mut next = 0;
        let mut active: Vec<usize> = Vec::new();
        let mut open: Vec<usize> = Vec::new();
        for w in stops.windows(2) {
            let (s, e) = (w[0], w[1]);
            while next < rs.len() && runs[rs[next]].t0 <= s + o.tol {
                active.push(rs[next]);
                next += 1;
            }
            active.retain(|&r| runs[r].t1 >= e - o.tol);
            let mut here: Vec<usize> = active.clone();
            here.sort_by(|&a, &b| runs[a].rho.total_cmp(&runs[b].rho).then(a.cmp(&b)));
            let rhos: Vec<f64> = here.iter().map(|&r| runs[r].rho).collect();
            let mut still = Vec::new();
            for (i, j) in bands(&rhos, o) {
                let (lo, hi) = (rhos[i], rhos[j]);
                let first = here[i..=j]
                    .iter()
                    .flat_map(|&r| runs[r].segs.iter().copied())
                    .min()
                    .unwrap_or(usize::MAX);
                let same = open
                    .iter()
                    .copied()
                    .find(|&p| (out[p].lo - lo).abs() <= o.tol && (out[p].hi - hi).abs() <= o.tol);
                if let Some(p) = same {
                    out[p].t1 = e;
                    out[p].first = out[p].first.min(first);
                    still.push(p);
                } else {
                    out.push(Piece {
                        class: c,
                        lo,
                        hi,
                        t0: s,
                        t1: e,
                        first,
                    });
                    still.push(out.len() - 1);
                }
            }
            open = still;
        }
    }
    out
}

/// What fills a gap in a wall: its opening, or nothing (the wall runs through).
#[derive(Clone, Debug)]
struct Found {
    /// Its jambs along the wall's frame.
    t0: f64,
    t1: f64,
    form: Form,
    leaves: u8,
    hand: End,
    side: Side,
    swing: Option<f64>,
}

/// A wall before it meets the others: its class, faces, extent along its frame, first line and openings.
#[derive(Clone, Debug)]
struct Draft {
    class: usize,
    lo: f64,
    hi: f64,
    t0: f64,
    t1: f64,
    first: usize,
    openings: Vec<Found>,
}

impl Draft {
    fn mid(&self) -> f64 {
        (self.lo + self.hi) / 2.0
    }

    fn width(&self) -> f64 {
        self.hi - self.lo
    }
}

/// What a gap between two pieces of one band is.
enum Gap {
    /// The wall runs through.
    Joint,
    Opening(Found),
    /// The wall ends.
    End,
}

/// The read's parts that the gaps are judged by.
struct Plan<'a> {
    lines: &'a Lines,
    o: &'a Options,
    frames: Vec<Frame>,
    runs: Vec<Run>,
    pieces: Vec<Piece>,
    /// The arcs taken as doors' swings.
    swings: Vec<bool>,
}

impl Plan<'_> {
    /// The gap `g0..g1` between two pieces of the band `lo..hi` of class `c`.
    fn gap(&mut self, c: usize, lo: f64, hi: f64, g0: f64, g1: f64) -> Gap {
        let o = *self.o;
        let f = self.frames[c];
        let span = g1 - g0;
        if span <= o.tol {
            return Gap::Joint;
        }
        // A face running on: a tee from the other side.
        let face = self.runs.iter().any(|r| {
            r.class == c
                && ((r.rho - lo).abs() <= o.tol || (r.rho - hi).abs() <= o.tol)
                && r.t0 <= g0 + o.tol
                && r.t1 >= g1 - o.tol
        });
        if face {
            return Gap::Joint;
        }
        if span <= o.max_opening + o.tol
            && let Some(found) = self
                .door(&f, lo, hi, g0, g1)
                .or_else(|| self.window(&f, lo, hi, g0, g1))
        {
            return Gap::Opening(found);
        }
        // A column across it.
        let column = self.lines.rings.iter().any(|ring| {
            if ring.role != Role::Column {
                return false;
            }
            let e = extent(&f, ring.pts.iter().copied());
            overlap(e.0, e.1, g0, g1) >= span / 2.0 && overlap(e.2, e.3, lo, hi) > o.tol
        });
        if column {
            return Gap::Joint;
        }
        // Another wall crossing it or meeting it there; or this band's wider stretch.
        let crossing = self.pieces.iter().any(|p| {
            if p.class == c {
                return p.lo <= lo + o.tol
                    && p.hi >= hi - o.tol
                    && (p.lo < lo - o.tol || p.hi > hi + o.tol)
                    && overlap(p.t0, p.t1, g0, g1) >= span / 2.0;
            }
            let pf = self.frames[p.class];
            let corners = [
                pf.at(p.t0, p.lo),
                pf.at(p.t0, p.hi),
                pf.at(p.t1, p.lo),
                pf.at(p.t1, p.hi),
            ];
            let e = extent(&f, corners);
            let across = overlap(e.2, e.3, lo - o.tol, hi + o.tol);
            // Two jambs of a narrow opening make a band inside this one's faces: no wall crosses there.
            let beyond = js_max(lo - o.tol - e.2, 0.0) + js_max(e.3 - hi - o.tol, 0.0);
            overlap(e.0, e.1, g0, g1) >= span / 2.0 && across >= 0.0 && beyond > 2.0 * o.tol
        });
        if crossing {
            return Gap::Joint;
        }
        if span < BARE_MIN {
            return Gap::Joint;
        }
        if span <= o.max_opening + o.tol {
            let (t0, t1) = self.clear(&f, lo, hi, g0, g1);
            return Gap::Opening(Found {
                t0,
                t1,
                form: Form::Opening,
                leaves: 1,
                hand: End::Start,
                side: Side::Left,
                swing: None,
            });
        }
        Gap::End
    }

    /// The gap without the columns at its ends: a door beside a column opens from the column's face.
    fn clear(&self, f: &Frame, lo: f64, hi: f64, g0: f64, g1: f64) -> (f64, f64) {
        let (mut t0, mut t1) = (g0, g1);
        for ring in &self.lines.rings {
            if ring.role != Role::Column {
                continue;
            }
            let e = extent(f, ring.pts.iter().copied());
            if overlap(e.2, e.3, lo, hi) <= self.o.tol {
                continue;
            }
            if e.0 <= t0 + self.o.tol && e.1 > t0 && e.1 < t1 {
                t0 = e.1;
            }
            if e.1 >= t1 - self.o.tol && e.0 < t1 && e.0 > t0 {
                t1 = e.0;
            }
        }
        (t0, t1)
    }

    /// A door's swing in the gap: an arc about a jamb as wide as the gap, or two about both as wide as half of it.
    fn door(&mut self, f: &Frame, lo: f64, hi: f64, g0: f64, g1: f64) -> Option<Found> {
        let o = *self.o;
        let (t0, t1) = self.clear(f, lo, hi, g0, g1);
        let span = t1 - t0;
        let mid = (lo + hi) / 2.0;
        // (arc, at the start jamb, its radius against the gap, its side).
        let mut found: Vec<(usize, bool, f64, Side)> = Vec::new();
        for (k, a) in self.lines.arcs.iter().enumerate() {
            if self.swings[k]
                || !matches!(a.role, Role::Door | Role::Wall)
                || a.sweep < 20.0 * PI / 180.0
            {
                continue;
            }
            let (tc, rc) = (f.t(a.c), f.rho(a.c));
            if rc < lo - HINGE || rc > hi + HINGE {
                continue;
            }
            let start = (tc - t0).abs() <= HINGE;
            let end = (tc - t1).abs() <= HINGE;
            if !(start || end) || !(a.r >= 0.35 * span && a.r <= 1.15 * span) {
                continue;
            }
            let m = a.a0 + a.sweep / 2.0;
            let p = Vec2::new(a.c.x + a.r * cos(m), a.c.y + a.r * sin(m));
            let side = if f.rho(p) >= mid {
                Side::Left
            } else {
                Side::Right
            };
            found.push((k, start, a.r / span, side));
        }
        if found.is_empty() {
            // A door's lines in the gap without a swing: a sliding or a folding door.
            let lines = self.lines.segments.iter().any(|s| {
                s.role == Role::Door
                    && clip(f, s.a, s.b, (t0 - o.tol, t1 + o.tol, lo - span, hi + span)).is_some()
            });
            return lines.then_some(Found {
                t0,
                t1,
                form: Form::Door,
                leaves: 1,
                hand: End::Start,
                side: Side::Left,
                swing: None,
            });
        }
        // A leaf of two is about half the gap.
        let starts: Vec<&(usize, bool, f64, Side)> =
            found.iter().filter(|x| x.1 && x.2 <= 0.7).collect();
        let ends: Vec<&(usize, bool, f64, Side)> =
            found.iter().filter(|x| !x.1 && x.2 <= 0.7).collect();
        let (leaves, hand, side, used): (u8, End, Side, Vec<usize>) =
            if let (Some(a), Some(b)) = (starts.first(), ends.first()) {
                (2, End::Start, a.3, vec![a.0, b.0])
            } else {
                // The one leaf as wide as the gap: the arc nearest it.
                let best = found.iter().min_by(|x, y| {
                    (x.2 - 1.0)
                        .abs()
                        .total_cmp(&(y.2 - 1.0).abs())
                        .then(x.0.cmp(&y.0))
                })?;
                (
                    1,
                    if best.1 { End::Start } else { End::End },
                    best.3,
                    vec![best.0],
                )
            };
        let sweep = self.lines.arcs[used[0]].sweep * 180.0 / PI;
        for k in used {
            self.swings[k] = true;
        }
        Some(Found {
            t0,
            t1,
            form: Form::Door,
            leaves,
            hand,
            side,
            swing: ((sweep - 90.0).abs() > 1.0).then(|| js_round(sweep)),
        })
    }

    /// A window's lines in the gap: along the wall, within its sills, over half of the gap.
    fn window(&self, f: &Frame, lo: f64, hi: f64, g0: f64, g1: f64) -> Option<Found> {
        let o = self.o;
        let (t0, t1) = self.clear(f, lo, hi, g0, g1);
        let span = t1 - t0;
        let parallel = sin(o.angle * PI / 180.0);
        let lines = self.lines.segments.iter().any(|s| {
            if s.role != Role::Window {
                return false;
            }
            let d = sub(s.b, s.a);
            let l = length(d);
            if l <= o.tol || (dot(d, f.n) / l).abs() > parallel {
                return false;
            }
            let rho = (f.rho(s.a) + f.rho(s.b)) / 2.0;
            let (ta, tb) = (f.t(s.a), f.t(s.b));
            let (a, b) = if ta <= tb { (ta, tb) } else { (tb, ta) };
            rho >= lo - SILL && rho <= hi + SILL && overlap(a, b, t0, t1) >= span / 2.0
        });
        lines.then_some(Found {
            t0,
            t1,
            form: Form::Window,
            leaves: 1,
            hand: End::Start,
            side: Side::Left,
            swing: None,
        })
    }
}

/// The walls and their openings read from the plan's line work.
pub fn recognize(lines: &Lines, o: &Options) -> Recognized {
    let classes = classes(&lines.segments, o);
    let frames: Vec<Frame> = classes.iter().map(|(f, _)| *f).collect();
    let runs = runs(&lines.segments, &classes, o.tol);
    let pieces = pieces(&runs, classes.len(), o);
    // The bands: pieces of one class and faces, along their line.
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    order.sort_by(|&a, &b| {
        let (p, q) = (&pieces[a], &pieces[b]);
        p.class
            .cmp(&q.class)
            .then(p.lo.total_cmp(&q.lo))
            .then(a.cmp(&b))
    });
    let mut strips: Vec<Vec<usize>> = Vec::new();
    for class_range in chains(&order, |&i| pieces[i].class as f64, 0.5) {
        let by_lo = &order[class_range];
        for lo_range in chains(by_lo, |&i| pieces[i].lo, o.tol) {
            let mut by_hi = by_lo[lo_range].to_vec();
            by_hi.sort_by(|&a, &b| pieces[a].hi.total_cmp(&pieces[b].hi).then(a.cmp(&b)));
            for hi_range in chains(&by_hi, |&i| pieces[i].hi, o.tol) {
                let mut strip = by_hi[hi_range].to_vec();
                strip.sort_by(|&a, &b| pieces[a].t0.total_cmp(&pieces[b].t0).then(a.cmp(&b)));
                let long: f64 = strip.iter().map(|&i| pieces[i].t1 - pieces[i].t0).sum();
                let wide = pieces[strip[0]].hi - pieces[strip[0]].lo;
                if long > wide + o.tol {
                    strips.push(strip);
                }
            }
        }
    }
    let mut plan = Plan {
        lines,
        o,
        frames,
        runs,
        pieces,
        swings: vec![false; lines.arcs.len()],
    };
    let mut drafts: Vec<Draft> = Vec::new();
    for strip in &strips {
        let p0 = plan.pieces[strip[0]].clone();
        let mut draft = Draft {
            class: p0.class,
            lo: p0.lo,
            hi: p0.hi,
            t0: p0.t0,
            t1: p0.t1,
            first: p0.first,
            openings: Vec::new(),
        };
        for &k in &strip[1..] {
            let p = plan.pieces[k].clone();
            match plan.gap(draft.class, draft.lo, draft.hi, draft.t1, p.t0) {
                Gap::Joint => {}
                Gap::Opening(found) => draft.openings.push(found),
                Gap::End => {
                    drafts.push(draft.clone());
                    draft.openings.clear();
                    draft.t0 = p.t0;
                    draft.first = usize::MAX;
                }
            }
            draft.t1 = js_max(draft.t1, p.t1);
            draft.first = draft.first.min(p.first);
        }
        drafts.push(draft);
    }
    meet(&mut drafts, &plan.frames, &lines.rings, o);
    drafts.sort_by(|a, b| a.first.cmp(&b.first).then(a.t0.total_cmp(&b.t0)));
    let mut out = Recognized::default();
    for (w, d) in drafts.iter().enumerate() {
        let f = plan.frames[d.class];
        let at = |t: f64| {
            let p = f.at(t, d.mid());
            Vec2::new(clean(p.x), clean(p.y))
        };
        out.walls.push(Wall {
            a: at(d.t0),
            b: at(d.t1),
            width: clean(d.width()),
            line: d.first,
        });
        for found in &d.openings {
            out.openings.push(Opening {
                wall: w,
                from: clean(found.t0 - d.t0),
                to: clean(found.t1 - d.t0),
                form: found.form,
                leaves: found.leaves,
                hand: found.hand,
                side: found.side,
                swing: found.swing,
            });
        }
    }
    out.used = taken(lines, &drafts, &plan.frames, &plan.swings, o);
    out
}

/// Walls running to the walls they meet: an end within reach of another wall's line runs to it; at a corner both
/// run to their crossing.
fn meet(drafts: &mut [Draft], frames: &[Frame], rings: &[Ring], o: &Options) {
    let turn = sin(JOIN_ANGLE * PI / 180.0);
    // How far past a point a column it lies in reaches along `dir`.
    let column_reach = |p: Vec2, dir: Vec2| -> f64 {
        let mut reach = 0.0_f64;
        for ring in rings.iter().filter(|r| r.role == Role::Column) {
            let (x0, x1, y0, y1) =
                extent(&Frame::of(Vec2::new(1.0, 0.0)), ring.pts.iter().copied());
            // The frame of east: t is x and rho is y.
            if p.x >= x0 - o.tol && p.x <= x1 + o.tol && p.y >= y0 - o.tol && p.y <= y1 + o.tol {
                for q in &ring.pts {
                    reach = js_max(reach, dot(sub(*q, p), dir));
                }
            }
        }
        reach
    };
    // (draft, end) → the point it runs to.
    let mut to: Vec<[Option<Vec2>; 2]> = vec![[None, None]; drafts.len()];
    let mut corners: Vec<(usize, usize, Vec2)> = Vec::new();
    for (i, d) in drafts.iter().enumerate() {
        let f = frames[d.class];
        for (end, t, dir) in [(0, d.t0, Vec2::new(-f.u.x, -f.u.y)), (1, d.t1, f.u)] {
            let p = f.at(t, d.mid());
            let mut best: Option<(f64, usize, Vec2, Option<usize>)> = None;
            for (j, h) in drafts.iter().enumerate() {
                if j == i {
                    continue;
                }
                let hf = frames[h.class];
                let s = cross(dir, hf.u);
                if s.abs() < turn {
                    continue;
                }
                let q = hf.at(h.t0, h.mid());
                let delta = cross(sub(q, p), hf.u) / s;
                let x = Vec2::new(p.x + dir.x * delta, p.y + dir.y * delta);
                let reach = js_max(h.width() / 2.0 + d.width() / 2.0, column_reach(p, dir)) + o.tol;
                if delta < -(h.width() / 2.0) - o.tol || delta > reach {
                    continue;
                }
                let tx = hf.t(x);
                let near = js_max_all([
                    d.width() / 2.0,
                    h.width() / 2.0,
                    column_reach(x, hf.u),
                    column_reach(x, Vec2::new(-hf.u.x, -hf.u.y)),
                ]) + o.tol;
                let corner = if (tx - h.t0).abs() <= near {
                    Some(0)
                } else if (tx - h.t1).abs() <= near {
                    Some(1)
                } else if tx > h.t0 && tx < h.t1 {
                    None
                } else {
                    continue;
                };
                if best
                    .as_ref()
                    .is_none_or(|b| delta.abs() < b.0.abs() - o.tol)
                {
                    best = Some((delta, j, x, corner));
                }
            }
            if let Some((_, j, x, corner)) = best {
                to[i][end] = Some(x);
                if let Some(e) = corner {
                    corners.push((j, e, x));
                }
            }
        }
    }
    // A corner's other wall runs to it too, unless its end runs elsewhere of its own.
    for (j, e, x) in corners {
        if to[j][e].is_none() {
            to[j][e] = Some(x);
        }
    }
    for (d, ends) in drafts.iter_mut().zip(to) {
        let f = frames[d.class];
        if let Some(x) = ends[0] {
            d.t0 = f.t(x);
        }
        if let Some(x) = ends[1] {
            d.t1 = f.t(x);
        }
    }
}

/// A box of a frame: `t0..t1 × rho0..rho1`.
type Region = (Frame, (f64, f64, f64, f64));

/// The sources taken whole: a wall's line within a wall (its ends reaching the half width of the widest wall past
/// it, a corner's outer face), a door's and window's line and arc within its opening, a wall's line there too.
fn taken(
    lines: &Lines,
    drafts: &[Draft],
    frames: &[Frame],
    swings: &[bool],
    o: &Options,
) -> Vec<usize> {
    let widest = js_max(js_max_all(drafts.iter().map(Draft::width)), 0.0);
    let ext = widest / 2.0 + o.tol;
    let walls: Vec<Region> = drafts
        .iter()
        .map(|d| {
            (
                frames[d.class],
                (d.t0 - ext, d.t1 + ext, d.lo - o.tol, d.hi + o.tol),
            )
        })
        .collect();
    let mut openings: Vec<Region> = Vec::new();
    for d in drafts {
        for found in &d.openings {
            let reach = if found.form == Form::Door {
                found.t1 - found.t0 + HINGE
            } else {
                SILL
            };
            let past = if found.form == Form::Door {
                HINGE
            } else {
                SILL
            };
            openings.push((
                frames[d.class],
                (found.t0 - past, found.t1 + past, d.lo - reach, d.hi + reach),
            ));
        }
    }
    let inside = |boxes: &[Region], a: Vec2, b: Vec2| {
        let l = length(sub(b, a));
        let parts: Vec<(f64, f64)> = boxes
            .iter()
            .filter_map(|(f, bx)| clip(f, a, b, *bx))
            .collect();
        covers(parts, if l > 0.0 { o.tol / l } else { 1.0 })
    };
    let point_in = |p: Vec2| {
        openings.iter().any(|(f, bx)| {
            let (t, r) = (f.t(p), f.rho(p));
            t >= bx.0 && t <= bx.1 && r >= bx.2 && r <= bx.3
        })
    };
    let mut seen = std::collections::BTreeSet::new();
    let mut left = std::collections::BTreeSet::new();
    for s in &lines.segments {
        seen.insert(s.source);
        let ok = match s.role {
            Role::Wall => inside(&walls, s.a, s.b) || inside(&openings, s.a, s.b),
            Role::Door | Role::Window => inside(&openings, s.a, s.b),
            Role::Column => false,
        };
        if !ok {
            left.insert(s.source);
        }
    }
    for (k, a) in lines.arcs.iter().enumerate() {
        seen.insert(a.source);
        let at = |th: f64| Vec2::new(a.c.x + a.r * cos(th), a.c.y + a.r * sin(th));
        let ok = swings[k]
            || (matches!(a.role, Role::Door | Role::Wall)
                && point_in(at(a.a0))
                && point_in(at(a.a0 + a.sweep / 2.0))
                && point_in(at(a.a0 + a.sweep)));
        if !ok {
            left.insert(a.source);
        }
    }
    for r in &lines.rings {
        seen.insert(r.source);
        left.insert(r.source);
    }
    seen.difference(&left).copied().collect()
}
