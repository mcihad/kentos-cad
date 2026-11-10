//! A building section from a section line (docs/adr/8042 §2; M42, ADV-19): what the line's vertical plane cuts and
//! what is seen past it, from the solids the model gives (prisms and flights), in the section's own plane: `u` along
//! the line as the viewer sees it (its left end 0, its right end its length), `z` up (metres over ±0.00).
//!
//! - **Frame.** The line from a section mark's first point to its second; the viewer looks to its left (its right when
//!   flipped), as the mark's arrows show (docs/adr/0345 §1), as far as `depth` (none: no limit).
//! - **Solid.** A footprint (its rings, read by the even-odd rule) between two heights: a prism (a wall's layer, a
//!   slab's layer, a column, a beam, a landing); or a flight, whose top steps up along its walking direction and whose
//!   underside is a slope never below its floor (its footprint convex: one leg). Boxes may be taken out of it (an
//!   opening through a wall); segments may be laid on its faces (an opening's outline, its frame), seen, never cut.
//! - **Cut.** Where the line runs through a footprint (the footprint on both sides of the plane: a face lying on the
//!   line is seen, not cut), the solid between its heights there, less its boxes: rectangles for a prism; for a flight
//!   its steps over its slope.
//! - **Seen.** A vertical face past the line whose outward normal points back at the viewer, a flight's riser that
//!   does, and a flight's underside where it does; within the line's length and the depth. Their edges (a vertical one
//!   only at a corner of the footprint, never where the view clips a face) and the marks on them are drawn where
//!   nothing nearer covers them: a point is hidden behind what is cut there, and behind a face whose projection holds
//!   it strictly inside and which is nearer than it by more than a micrometre. A face is taken as closed over its
//!   solid's boxes: an opening is a window or a door one does not see through; its outline is a mark.
//! - **Sloped** (docs/adr/8044 §5). A roof's face: its footprint (the face's ring in plan), its top a plane, its bottom
//!   a thickness under it (measured upright). Cut: between its bottom and top along the line, a band that slopes as the
//!   plane does. Seen: its top where the plane rises away from the viewer, its underside where it rises towards them
//!   (looking into a roof space), each with its footprint's edges; and on its edges along the roof's eaves line a
//!   vertical face (the eaves' fascia, a gable end's rake) with its top, its bottom and a vertical edge at each corner.
//!   An edge one roof's faces share (a hip, a ridge) is drawn once.
//! - **Joint** (docs/adr/8043 §7). A prism's face edge with a prism face in its own plane on both sides is where two
//!   faces meet under one finish (a plastered wall and the beam, the column or the slab flush with it): not drawn,
//!   unless the two solids' finishes differ (a stone plinth under plaster). A crease (two planes) and a mark always are.
//!
//! `scripts/fixtures/section_cases.py` computes the same from these rules without KentOS's code, in exact fractions.

use crate::jsmath::{js_hypot, js_max, js_min};
use crate::vec2::Vec2;

/// Two heights or two places closer than this are one, metres.
const EPS: f64 = 1e-9;
/// How far behind a face a point must be to be hidden by it, metres: an edge lying on a face is never hidden by it.
const BEHIND: f64 = 1e-6;
/// How far either side of a line a footprint is looked at, metres: to tell a cut from a face on the line, and a
/// face's outward side.
const SIDE: f64 = 1e-6;
/// A seen piece shorter than this is dropped, metres.
const SHORTEST: f64 = 1e-6;

/// Where a section looks from and to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// The line's end on the viewer's left: `u` = 0.
    pub origin: Vec2,
    /// Along the line to the viewer's right (a unit vector).
    pub right: Vec2,
    /// Into the view (a unit vector).
    pub look: Vec2,
    /// The line's length: `u` runs from 0 to it.
    pub length: f64,
    /// How far past the line the view reaches; none: no limit.
    pub depth: Option<f64>,
}

impl Frame {
    /// A section line's frame: from `a` to `b`, looking to its left (its right when `flip`), `depth` deep (none: no
    /// limit). None for a line shorter than a millimetre or a depth not above 0.
    pub fn new(a: Vec2, b: Vec2, flip: bool, depth: Option<f64>) -> Option<Frame> {
        let length = js_hypot(b.x - a.x, b.y - a.y);
        if !(length >= 1e-3) || depth.is_some_and(|d| !(d > 0.0)) {
            return None;
        }
        let t = Vec2::new((b.x - a.x) / length, (b.y - a.y) / length);
        let left = Vec2::new(-t.y, t.x);
        Some(if flip {
            Frame {
                origin: b,
                right: Vec2::new(-t.x, -t.y),
                look: Vec2::new(-left.x, -left.y),
                length,
                depth,
            }
        } else {
            Frame {
                origin: a,
                right: t,
                look: left,
                length,
                depth,
            }
        })
    }

    /// A plan point's distance along the line from its left end.
    pub fn u(&self, p: Vec2) -> f64 {
        (p.x - self.origin.x) * self.right.x + (p.y - self.origin.y) * self.right.y
    }

    /// A plan point's distance past the line, into the view (negative: behind the viewer).
    pub fn d(&self, p: Vec2) -> f64 {
        (p.x - self.origin.x) * self.look.x + (p.y - self.origin.y) * self.look.y
    }

    /// The plan point `u` along the line and `d` past it.
    pub fn at(&self, u: f64, d: f64) -> Vec2 {
        Vec2::new(
            self.origin.x + self.right.x * u + self.look.x * d,
            self.origin.y + self.right.y * u + self.look.y * d,
        )
    }

    /// The part `[t0, t1]` of the plan segment from `p` to `q` that is in view: past the line, within the depth and
    /// within the line's length. None when none is.
    fn clip(&self, p: Vec2, q: Vec2) -> Option<(f64, f64)> {
        let (mut lo, mut hi) = (0.0, 1.0);
        // Each bound: value(t) = a + b t ≥ 0.
        let mut keep = |a: f64, b: f64| {
            if b.abs() <= 1e-15 {
                if a < -EPS {
                    hi = -1.0;
                }
            } else if b > 0.0 {
                lo = js_max(lo, -a / b);
            } else {
                hi = js_min(hi, -a / b);
            }
        };
        let (dp, dq) = (self.d(p), self.d(q));
        let (up, uq) = (self.u(p), self.u(q));
        keep(dp, dq - dp);
        if let Some(depth) = self.depth {
            keep(depth - dp, dp - dq);
        }
        keep(up, uq - up);
        keep(self.length - up, up - uq);
        (hi - lo > 1e-12).then_some((lo, hi))
    }
}

/// A flight of steps (docs/adr/8042 §2): along `dir` (a unit vector) from `start`, a point's distance being
/// `s = (p − start)·dir`.
#[derive(Clone, Debug, PartialEq)]
pub struct Flight {
    pub start: Vec2,
    pub dir: Vec2,
    /// Its treads' heights: each from its distance on, in order; before the first, the first's.
    pub steps: Vec<(f64, f64)>,
    /// Its underside: its height at distance 0 and its rise per metre along `dir`; never below `floor`.
    pub soffit: f64,
    pub slope: f64,
    pub floor: f64,
}

impl Flight {
    /// Its distance at plan point `p`.
    pub fn s(&self, p: Vec2) -> f64 {
        (p.x - self.start.x) * self.dir.x + (p.y - self.start.y) * self.dir.y
    }

    /// Its top at distance `s`: the tread that holds it.
    pub fn top(&self, s: f64) -> f64 {
        let mut z = self.steps.first().map_or(0.0, |st| st.1);
        for &(from, h) in &self.steps {
            if s >= from - EPS {
                z = h;
            } else {
                break;
            }
        }
        z
    }

    /// Its underside at distance `s`, never over its top there.
    pub fn bottom(&self, s: f64) -> f64 {
        js_min(
            js_max(self.floor, self.soffit + self.slope * s),
            self.top(s),
        )
    }

    /// Where its slope leaves its floor, if it does.
    fn kink(&self) -> Option<f64> {
        (self.slope.abs() > EPS).then(|| (self.floor - self.soffit) / self.slope)
    }

    /// The distances its top steps at or its underside bends, in order.
    fn breaks(&self) -> Vec<f64> {
        let mut out: Vec<f64> = self.steps.iter().skip(1).map(|s| s.0).collect();
        out.extend(self.kink());
        out.sort_by(f64::total_cmp);
        out
    }
}

/// A roof's face as a sloped slab (docs/adr/8044 §5): its top the plane `top + dx·x + dy·y` (metres over ±0.00 at a
/// plan point), its bottom `thickness` under it, measured upright.
#[derive(Clone, Debug, PartialEq)]
pub struct Sloped {
    pub top: f64,
    pub dx: f64,
    pub dy: f64,
    pub thickness: f64,
    /// The roof's eaves line (closed): a footprint edge on it has a vertical face (the fascia, a rake); the others meet
    /// the roof's other faces (a hip, a ridge, a valley) and have none.
    pub eaves: Vec<Vec2>,
    /// Which roof the face is of: an edge one roof's faces share is drawn once.
    pub roof: usize,
}

impl Sloped {
    /// Its top at plan point `p`.
    pub fn z(&self, p: Vec2) -> f64 {
        self.top + self.dx * p.x + self.dy * p.y
    }

    /// Whether the plan edge `p`→`q` lies on its roof's eaves line (its middle within `SIDE` of an eaves edge).
    fn open(&self, p: Vec2, q: Vec2) -> bool {
        let m = Vec2::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
        let n = self.eaves.len();
        (0..n).any(|i| {
            let (a, b) = (self.eaves[i], self.eaves[(i + 1) % n]);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let len2 = dx * dx + dy * dy;
            if len2 <= EPS * EPS {
                return false;
            }
            let t = ((m.x - a.x) * dx + (m.y - a.y) * dy) / len2;
            let off = ((m.x - a.x) * dy - (m.y - a.y) * dx).abs() / len2.sqrt();
            off <= SIDE && t >= -EPS && t <= 1.0 + EPS
        })
    }
}

/// How high a solid stands.
#[derive(Clone, Debug, PartialEq)]
pub enum Height {
    /// Between two heights, metres over ±0.00.
    Prism {
        bottom: f64,
        top: f64,
    },
    Flight(Flight),
    /// A roof's face (docs/adr/8044 §5).
    Sloped(Sloped),
}

/// A box taken out of a solid: its outline in plan, its bottom and its top.
#[derive(Clone, Debug, PartialEq)]
pub struct Void {
    pub ring: Vec<Vec2>,
    pub bottom: f64,
    pub top: f64,
}

/// A solid of the model as the section reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct Solid {
    /// Its footprint's rings (outer rings and holes alike: the even-odd rule), each closed implicitly.
    pub rings: Vec<Vec<Vec2>>,
    pub height: Height,
    pub voids: Vec<Void>,
    /// Segments on its faces, each end a plan point and a height: seen where visible, never cut.
    pub marks: Vec<[(Vec2, f64); 2]>,
    /// Its faces' finish (a wall layer's material); none: a solid that takes the finish of what it is flush with (a
    /// beam, a column, a slab). Two faces in one plane meet without a line unless both have one and they differ.
    pub finish: Option<String>,
}

/// What the line cuts of a solid: a ring in the section's plane (`x` = u, `y` = z).
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub solid: usize,
    pub ring: Vec<Vec2>,
}

/// A piece of an edge seen past the line, in the section's plane, with its solid and how far past the line its
/// nearer end lies.
#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    pub solid: usize,
    pub a: Vec2,
    pub b: Vec2,
    pub depth: f64,
}

/// A section: what is cut and what is seen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Section {
    pub cut: Vec<Cut>,
    pub seen: Vec<Seen>,
}

/// Even-odd: whether `p` is inside the rings.
fn inside(rings: &[Vec<Vec2>], p: Vec2) -> bool {
    let mut odd = false;
    for r in rings {
        let n = r.len();
        for i in 0..n {
            let (a, b) = (r[i], r[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) && p.x < a.x + (p.y - a.y) * (b.x - a.x) / (b.y - a.y) {
                odd = !odd;
            }
        }
    }
    odd
}

/// The stretches of the line through `at` along `along` (unit), `w` from `lo` to `hi` (the points `at + w·along`),
/// that run through the rings: inside them on both sides of the line, `SIDE` away along `normal`. In order, joined
/// where they meet.
fn through(
    rings: &[Vec<Vec2>],
    at: Vec2,
    along: Vec2,
    normal: Vec2,
    lo: f64,
    hi: f64,
) -> Vec<(f64, f64)> {
    if hi - lo <= EPS {
        return Vec::new();
    }
    let w = |p: Vec2| (p.x - at.x) * along.x + (p.y - at.y) * along.y;
    let n = |p: Vec2| (p.x - at.x) * normal.x + (p.y - at.y) * normal.y;
    let mut breaks = vec![lo, hi];
    for r in rings {
        let k = r.len();
        for i in 0..k {
            let (p, q) = (r[i], r[(i + 1) % k]);
            let (np, nq) = (n(p), n(q));
            if np.abs() <= EPS {
                breaks.push(w(p));
            }
            if (np > EPS && nq < -EPS) || (np < -EPS && nq > EPS) {
                let t = np / (np - nq);
                breaks.push(w(p) + (w(q) - w(p)) * t);
            }
        }
    }
    breaks.retain(|b| *b > lo - EPS && *b < hi + EPS);
    breaks.sort_by(f64::total_cmp);
    breaks.dedup_by(|a, b| (*a - *b).abs() <= EPS);
    let mut out: Vec<(f64, f64)> = Vec::new();
    for pair in breaks.windows(2) {
        let (a, b) = (js_max(pair[0], lo), js_min(pair[1], hi));
        if b - a <= EPS {
            continue;
        }
        let m = (a + b) / 2.0;
        let p = Vec2::new(at.x + along.x * m, at.y + along.y * m);
        let off = |s: f64| Vec2::new(p.x + normal.x * s, p.y + normal.y * s);
        if inside(rings, off(SIDE)) && inside(rings, off(-SIDE)) {
            match out.last_mut() {
                Some(last) if (a - last.1).abs() <= EPS => last.1 = b,
                _ => out.push((a, b)),
            }
        }
    }
    out
}

/// The stretches of the section line, from 0 to its length, that cut through the rings.
pub fn cut_stretches(frame: &Frame, rings: &[Vec<Vec2>]) -> Vec<(f64, f64)> {
    through(
        rings,
        frame.origin,
        frame.right,
        frame.look,
        0.0,
        frame.length,
    )
}

/// An axis-aligned rectangle in the section's plane: u from, u to, z from, z to.
type Rect = (f64, f64, f64, f64);

/// `r` less `v`: what of it lies outside `v`.
fn subtract(r: Rect, v: Rect) -> Vec<Rect> {
    let (u0, u1, z0, z1) = r;
    let (v0, v1, w0, w1) = v;
    if v1 <= u0 + EPS || v0 >= u1 - EPS || w1 <= z0 + EPS || w0 >= z1 - EPS {
        return vec![r];
    }
    let mut out = Vec::new();
    let (m0, m1) = (js_max(u0, v0), js_min(u1, v1));
    if m0 > u0 + EPS {
        out.push((u0, m0, z0, z1));
    }
    if m1 < u1 - EPS {
        out.push((m1, u1, z0, z1));
    }
    if w0 > z0 + EPS {
        out.push((m0, m1, z0, w0));
    }
    if w1 < z1 - EPS {
        out.push((m0, m1, w1, z1));
    }
    out
}

fn rect_ring((u0, u1, z0, z1): Rect) -> Vec<Vec2> {
    vec![
        Vec2::new(u0, z0),
        Vec2::new(u1, z0),
        Vec2::new(u1, z1),
        Vec2::new(u0, z1),
    ]
}

/// A trapezoid of a flight's cut or face: from `u0` to `u1`, its top, its underside at both ends; with its depth at
/// both ends (0 for a cut).
#[derive(Clone, Copy, Debug)]
struct Trap {
    u0: f64,
    u1: f64,
    top: f64,
    b0: f64,
    b1: f64,
    d0: f64,
    d1: f64,
}

/// A flight over the plan segment from `p` to `q`, `t` from `t0` to `t1`: its trapezoids between the places its top
/// steps or its underside bends, in order of `t`.
fn flight_traps(frame: &Frame, f: &Flight, p: Vec2, q: Vec2, (t0, t1): (f64, f64)) -> Vec<Trap> {
    let at = |t: f64| Vec2::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t);
    let (sp, sq) = (f.s(p), f.s(q));
    let mut ts = vec![t0, t1];
    if (sq - sp).abs() > EPS {
        for b in f.breaks() {
            ts.push((b - sp) / (sq - sp));
        }
    }
    ts.retain(|t| *t > t0 - 1e-12 && *t < t1 + 1e-12);
    ts.sort_by(f64::total_cmp);
    ts.dedup_by(|a, b| (*a - *b).abs() <= 1e-12);
    let s = |t: f64| sp + (sq - sp) * t;
    ts.windows(2)
        .filter(|w| w[1] - w[0] > 1e-12)
        .map(|w| {
            let (a, b) = (at(w[0]), at(w[1]));
            Trap {
                u0: frame.u(a),
                u1: frame.u(b),
                top: f.top(s((w[0] + w[1]) / 2.0)),
                b0: f.bottom(s(w[0])),
                b1: f.bottom(s(w[1])),
                d0: frame.d(a),
                d1: frame.d(b),
            }
        })
        .filter(|tr| tr.top > js_min(tr.b0, tr.b1) + EPS)
        .collect()
}

/// A trapezoid's ring, turning counter-clockwise in the section's plane.
fn trap_ring(tr: &Trap) -> Vec<Vec2> {
    let (l, r) = if tr.u0 <= tr.u1 {
        ((tr.u0, tr.b0), (tr.u1, tr.b1))
    } else {
        ((tr.u1, tr.b1), (tr.u0, tr.b0))
    };
    let mut ring = vec![Vec2::new(l.0, l.1), Vec2::new(r.0, r.1)];
    if tr.top > r.1 + EPS {
        ring.push(Vec2::new(r.0, tr.top));
    }
    if tr.top > l.1 + EPS {
        ring.push(Vec2::new(l.0, tr.top));
    }
    ring
}

/// What hides what lies behind it: a convex ring in the section's plane (counter-clockwise) and its depth there,
/// `a + b·u + c·z`; its box and its nearest depth; whether it is what the plane cuts.
#[derive(Clone, Debug)]
struct Cover {
    ring: Vec<Vec2>,
    a: f64,
    b: f64,
    c: f64,
    lo: Vec2,
    hi: Vec2,
    near: f64,
    /// A cut hides what lies in the plane too: a riser on the line is the cut's own face, its edges inside the
    /// hatch (a section line drawn on a riser in plan drew the tread's nosing through the cut flight).
    cut: bool,
    /// A prism's face: its solid (a joint is where two such faces meet in one plane).
    face_of: Option<usize>,
}

impl Cover {
    fn new(ring: Vec<Vec2>, (a, b, c): (f64, f64, f64)) -> Option<Cover> {
        let mut ring = ring;
        let n = ring.len();
        if n < 3 {
            return None;
        }
        let area: f64 = (0..n)
            .map(|i| {
                let (p, q) = (ring[i], ring[(i + 1) % n]);
                p.x * q.y - q.x * p.y
            })
            .sum::<f64>()
            / 2.0;
        if area.abs() <= 1e-12 {
            return None;
        }
        if area < 0.0 {
            ring.reverse();
        }
        let lo = Vec2::new(
            ring.iter().map(|p| p.x).fold(f64::INFINITY, js_min),
            ring.iter().map(|p| p.y).fold(f64::INFINITY, js_min),
        );
        let hi = Vec2::new(
            ring.iter().map(|p| p.x).fold(f64::NEG_INFINITY, js_max),
            ring.iter().map(|p| p.y).fold(f64::NEG_INFINITY, js_max),
        );
        let near = ring
            .iter()
            .map(|p| a + b * p.x + c * p.y)
            .fold(f64::INFINITY, js_min);
        Some(Cover {
            ring,
            a,
            b,
            c,
            lo,
            hi,
            near,
            cut: false,
            face_of: None,
        })
    }

    /// A prism's face of `solid`.
    fn face(ring: Vec<Vec2>, depth: (f64, f64, f64), solid: usize) -> Option<Cover> {
        Cover::new(ring, depth).map(|c| Cover {
            face_of: Some(solid),
            ..c
        })
    }

    /// Whether it lies in the plane `(a, b, c)`: the same slopes, the same depth at `at`.
    fn in_plane(&self, (a, b, c): (f64, f64, f64), at: Vec2) -> bool {
        (self.b - b).abs() <= 1e-9 * (1.0 + b.abs())
            && (self.c - c).abs() <= 1e-9 * (1.0 + c.abs())
            && ((self.a + self.b * at.x + self.c * at.y) - (a + b * at.x + c * at.y)).abs() <= 1e-7
    }

    /// The part `(t0, t1)` of `p`→`q` inside it or on its outline.
    fn holds(&self, p: Vec2, q: Vec2) -> Option<(f64, f64)> {
        let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
        let n = self.ring.len();
        let dir = Vec2::new(q.x - p.x, q.y - p.y);
        for i in 0..n {
            let (r, s) = (self.ring[i], self.ring[(i + 1) % n]);
            let side = Vec2::new(s.x - r.x, s.y - r.y);
            let len = js_hypot(side.x, side.y);
            if len <= 1e-15 {
                continue;
            }
            let a = (side.x * (p.y - r.y) - side.y * (p.x - r.x)) / len + EPS;
            let b = (side.x * dir.y - side.y * dir.x) / len;
            if b.abs() <= 1e-15 {
                if a < 0.0 {
                    return None;
                }
            } else if b > 0.0 {
                lo = js_max(lo, -a / b);
            } else {
                hi = js_min(hi, -a / b);
            }
        }
        (hi - lo > 1e-12).then_some((lo, hi))
    }

    /// What the plane cuts: at depth 0, hiding what lies in the plane as well as behind it.
    fn cut(ring: Vec<Vec2>) -> Option<Cover> {
        Cover::new(ring, (0.0, 0.0, 0.0)).map(|c| Cover { cut: true, ..c })
    }

    /// The part `(t0, t1)` of the edge it hides: inside it or on its outline, and behind it (a cut: or in it).
    fn hides(&self, e: &Edge) -> Option<(f64, f64)> {
        let behind = if self.cut { -BEHIND } else { BEHIND };
        if js_max(e.p.x, e.q.x) < self.lo.x - EPS
            || js_min(e.p.x, e.q.x) > self.hi.x + EPS
            || js_max(e.p.y, e.q.y) < self.lo.y - EPS
            || js_min(e.p.y, e.q.y) > self.hi.y + EPS
            || self.near > js_max(e.dp, e.dq) - behind
        {
            return None;
        }
        let (mut lo, mut hi) = (0.0_f64, 1.0_f64);
        // Each bound: value(t) = a + b t > 0.
        let mut keep = |a: f64, b: f64| {
            if b.abs() <= 1e-15 {
                if a <= 0.0 {
                    hi = -1.0;
                }
            } else if b > 0.0 {
                lo = js_max(lo, -a / b);
            } else {
                hi = js_min(hi, -a / b);
            }
        };
        let n = self.ring.len();
        let dir = Vec2::new(e.q.x - e.p.x, e.q.y - e.p.y);
        for i in 0..n {
            let (r, s) = (self.ring[i], self.ring[(i + 1) % n]);
            let side = Vec2::new(s.x - r.x, s.y - r.y);
            let len = js_hypot(side.x, side.y);
            if len <= 1e-15 {
                continue;
            }
            // Left of the side or on it: a line behind the edge of what covers it lies on a drawn line (its edge or
            // a cut's outline) and two covers that meet leave no crack between them.
            let a = (side.x * (e.p.y - r.y) - side.y * (e.p.x - r.x)) / len + EPS;
            let b = (side.x * dir.y - side.y * dir.x) / len;
            keep(a, b);
        }
        // Behind it: the edge's depth over the cover's there by BEHIND (a cut: not under it by BEHIND).
        let cover_p = self.a + self.b * e.p.x + self.c * e.p.y;
        let cover_q = self.a + self.b * e.q.x + self.c * e.q.y;
        keep(e.dp - cover_p - behind, (e.dq - e.dp) - (cover_q - cover_p));
        (hi - lo > 1e-12).then_some((lo, hi))
    }
}

/// An edge to draw where it is seen: its ends in the section's plane and their depths, and its solid.
#[derive(Clone, Copy, Debug)]
struct Edge {
    solid: usize,
    p: Vec2,
    q: Vec2,
    dp: f64,
    dq: f64,
    /// A prism face's edge: the face's plane (a joint is looked for); none for a mark or a flight's edge.
    plane: Option<(f64, f64, f64)>,
}

/// Spans in order, the overlapping ones joined.
fn joined(mut spans: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out: Vec<(f64, f64)> = Vec::new();
    for (a, b) in spans {
        match out.last_mut() {
            Some(last) if a <= last.1 => last.1 = js_max(last.1, b),
            _ => out.push((a, b)),
        }
    }
    out
}

/// The parts of a prism face's edge that are a joint (docs/adr/8043 §7): a prism face in its plane on both sides,
/// `SIDE` away, whose solid's finish goes with the edge's solid's (the same, or either has none).
fn joints(e: &Edge, covers: &[Cover], finishes: &[Option<&str>]) -> Vec<(f64, f64)> {
    let Some(plane) = e.plane else {
        return Vec::new();
    };
    let (du, dz) = (e.q.x - e.p.x, e.q.y - e.p.y);
    let norm = du.abs() + dz.abs();
    if norm <= EPS {
        return Vec::new();
    }
    let mid = Vec2::new((e.p.x + e.q.x) / 2.0, (e.p.y + e.q.y) / 2.0);
    let mine = finishes.get(e.solid).copied().flatten();
    let side = |sign: f64| {
        let o = Vec2::new(-dz * SIDE * sign / norm, du * SIDE * sign / norm);
        let (p, q) = (
            Vec2::new(e.p.x + o.x, e.p.y + o.y),
            Vec2::new(e.q.x + o.x, e.q.y + o.y),
        );
        joined(
            covers
                .iter()
                .filter(|c| {
                    c.face_of.is_some_and(|s| {
                        let theirs = finishes.get(s).copied().flatten();
                        mine.is_none() || theirs.is_none() || mine == theirs
                    }) && c.in_plane(plane, mid)
                })
                .filter_map(|c| c.holds(p, q))
                .collect(),
        )
    };
    let (left, right) = (side(1.0), side(-1.0));
    let mut out = Vec::new();
    for &(a0, a1) in &left {
        for &(b0, b1) in &right {
            let (lo, hi) = (js_max(a0, b0), js_min(a1, b1));
            if hi > lo {
                out.push((lo, hi));
            }
        }
    }
    out
}

/// A ring without its repeated points and the points it runs straight through.
fn clean(ring: &[Vec2]) -> Vec<Vec2> {
    let mut out: Vec<Vec2> = Vec::new();
    for &p in ring {
        if out
            .last()
            .is_none_or(|l| js_hypot(p.x - l.x, p.y - l.y) > EPS)
        {
            out.push(p);
        }
    }
    while out.len() > 1 && {
        let (f, l) = (out[0], out[out.len() - 1]);
        js_hypot(f.x - l.x, f.y - l.y) <= EPS
    } {
        out.pop();
    }
    loop {
        let n = out.len();
        if n < 3 {
            return out;
        }
        let straight = (0..n).find(|&i| {
            let (a, b, c) = (out[(i + n - 1) % n], out[i], out[(i + 1) % n]);
            let (u, v) = (
                Vec2::new(b.x - a.x, b.y - a.y),
                Vec2::new(c.x - b.x, c.y - b.y),
            );
            let cross = u.x * v.y - u.y * v.x;
            let dot = u.x * v.x + u.y * v.y;
            cross.abs() <= EPS * js_hypot(u.x, u.y) * js_hypot(v.x, v.y) + 1e-15 && dot > 0.0
        });
        match straight {
            Some(i) => {
                out.remove(i);
            }
            None => return out,
        }
    }
}

/// The section of `solids` along `frame`: what is cut, each solid's in order, and what is seen.
pub fn section(frame: &Frame, solids: &[Solid]) -> Section {
    let mut out = Section::default();
    let mut covers: Vec<Cover> = Vec::new();
    let mut edges: Vec<Edge> = Vec::new();
    for (i, solid) in solids.iter().enumerate() {
        let rings: Vec<Vec<Vec2>> = solid
            .rings
            .iter()
            .map(|r| clean(r))
            .filter(|r| r.len() >= 3)
            .collect();
        // A solid without a footprint (a door's or a window's lines on its wall's faces, docs/adr/8043) is its marks
        // alone: nothing of it is cut and it has no faces; its marks are seen where nothing nearer hides them.
        let stretches = cut_stretches(frame, &rings);
        match &solid.height {
            _ if rings.is_empty() => {}
            Height::Prism { bottom, top } if top - bottom <= EPS => {}
            Height::Prism { bottom, top } => {
                let mut rects: Vec<Rect> = stretches
                    .iter()
                    .map(|&(a, b)| (a, b, *bottom, *top))
                    .collect();
                for v in &solid.voids {
                    let ring = clean(&v.ring);
                    for (a, b) in cut_stretches(frame, std::slice::from_ref(&ring)) {
                        let hole = (a, b, v.bottom, v.top);
                        rects = rects.into_iter().flat_map(|r| subtract(r, hole)).collect();
                    }
                }
                for r in rects {
                    let ring = rect_ring(r);
                    covers.extend(Cover::cut(ring.clone()));
                    out.cut.push(Cut {
                        solid: i,
                        ring: canonical(ring),
                    });
                }
                prism_faces(frame, i, &rings, *bottom, *top, &mut covers, &mut edges);
            }
            Height::Flight(f) => {
                for &(a, b) in &stretches {
                    let (p, q) = (frame.at(a, 0.0), frame.at(b, 0.0));
                    let traps = flight_traps(frame, f, p, q, (0.0, 1.0));
                    for tr in &traps {
                        covers.extend(Cover::cut(trap_ring(tr)));
                    }
                    out.cut
                        .extend(flight_cut_rings(&traps).into_iter().map(|ring| Cut {
                            solid: i,
                            ring: canonical(ring),
                        }));
                }
                flight_faces(frame, i, &rings, f, &mut covers, &mut edges);
            }
            Height::Sloped(s) => {
                // A void whose heights hold the band where the line crosses it (a chimney through the roof, docs/adr/8044
                // §6) takes its stretch out of the band.
                let mut runs = stretches.clone();
                for v in &solid.voids {
                    let ring = clean(&v.ring);
                    for (va, vb) in cut_stretches(frame, std::slice::from_ref(&ring)) {
                        runs = runs
                            .into_iter()
                            .flat_map(|(a, b)| {
                                let (lo, hi) = (js_max(a, va), js_min(b, vb));
                                if hi - lo <= EPS {
                                    return vec![(a, b)];
                                }
                                let (zl, zh) = (s.z(frame.at(lo, 0.0)), s.z(frame.at(hi, 0.0)));
                                if v.bottom > js_min(zl, zh) - s.thickness + EPS
                                    || v.top < js_max(zl, zh) - EPS
                                {
                                    return vec![(a, b)];
                                }
                                [(a, lo), (hi, b)]
                                    .into_iter()
                                    .filter(|(p, q)| q - p > EPS)
                                    .collect()
                            })
                            .collect();
                    }
                }
                for &(a, b) in &runs {
                    let (ta, tb) = (s.z(frame.at(a, 0.0)), s.z(frame.at(b, 0.0)));
                    let ring = vec![
                        Vec2::new(a, ta - s.thickness),
                        Vec2::new(b, tb - s.thickness),
                        Vec2::new(b, tb),
                        Vec2::new(a, ta),
                    ];
                    covers.extend(Cover::cut(ring.clone()));
                    out.cut.push(Cut {
                        solid: i,
                        ring: canonical(ring),
                    });
                }
                sloped_faces(frame, i, &rings, s, &mut covers, &mut edges);
            }
        }
        for [(p, zp), (q, zq)] in &solid.marks {
            if let Some((t0, t1)) = frame.clip(*p, *q) {
                let at = |t: f64| Vec2::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t);
                let (a, b) = (at(t0), at(t1));
                edges.push(Edge {
                    solid: i,
                    p: Vec2::new(frame.u(a), zp + (zq - zp) * t0),
                    q: Vec2::new(frame.u(b), zp + (zq - zp) * t1),
                    dp: frame.d(a),
                    dq: frame.d(b),
                    plane: None,
                });
            }
        }
    }
    let finishes: Vec<Option<&str>> = solids.iter().map(|s| s.finish.as_deref()).collect();
    // An edge is drawn once for its solid, or for its roof: a hip or a ridge two of a roof's faces share.
    let group = |solid: usize| match solids.get(solid).map(|s| &s.height) {
        Some(Height::Sloped(s)) => solids.len() + s.roof,
        _ => solid,
    };
    let mut drawn = std::collections::HashSet::new();
    for e in &edges {
        let length = js_hypot(e.q.x - e.p.x, e.q.y - e.p.y);
        if length <= SHORTEST {
            continue;
        }
        let mut hidden: Vec<(f64, f64)> = covers.iter().filter_map(|c| c.hides(e)).collect();
        hidden.extend(joints(e, &covers, &finishes));
        hidden.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut t = 0.0;
        let mut pieces: Vec<(f64, f64)> = Vec::new();
        for (a, b) in hidden {
            if a > t {
                pieces.push((t, a));
            }
            t = js_max(t, b);
        }
        if t < 1.0 {
            pieces.push((t, 1.0));
        }
        for (a, b) in pieces {
            if (b - a) * length <= SHORTEST {
                continue;
            }
            let at = |t: f64| Vec2::new(e.p.x + (e.q.x - e.p.x) * t, e.p.y + (e.q.y - e.p.y) * t);
            let (da, db) = (e.dp + (e.dq - e.dp) * a, e.dp + (e.dq - e.dp) * b);
            let seen = Seen {
                solid: e.solid,
                a: at(a),
                b: at(b),
                depth: js_min(da, db),
            };
            // An edge two faces share (a corner's vertical edge) is drawn once.
            let (ka, kb) = (key(seen.a), key(seen.b));
            if drawn.insert((group(seen.solid), ka.min(kb), ka.max(kb))) {
                out.seen.push(seen);
            }
        }
    }
    out
}

/// What the viewer sees at `at` (`u`, `z`) past the line (docs/adr/8043 §10): the solid whose prism face looking back
/// at them is the nearest there, and that face's depth; none where no prism face is (a flight is not a facade). A face
/// is closed over its solid's boxes, as the drawing takes it.
pub fn seen_at(frame: &Frame, solids: &[Solid], at: Vec2) -> Option<(usize, f64)> {
    let mut best: Option<(usize, f64)> = None;
    for (i, solid) in solids.iter().enumerate() {
        if let Height::Sloped(s) = &solid.height
            && let Some(d) = sloped_seen_at(frame, solid, s, at)
            && best.is_none_or(|(_, n)| d < n)
        {
            best = Some((i, d));
        }
        let Height::Prism { bottom, top } = &solid.height else {
            continue;
        };
        if at.y < bottom - EPS || at.y > top + EPS {
            continue;
        }
        let rings: Vec<Vec<Vec2>> = solid
            .rings
            .iter()
            .map(|r| clean(r))
            .filter(|r| r.len() >= 3)
            .collect();
        for (p, q, m) in outward(&rings) {
            if !facing(frame, m) {
                continue;
            }
            let Some((t0, t1)) = frame.clip(p, q) else {
                continue;
            };
            let on = |t: f64| Vec2::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t);
            let (a, b) = (on(t0), on(t1));
            let (u0, u1) = (frame.u(a), frame.u(b));
            if (u1 - u0).abs() <= EPS || at.x < js_min(u0, u1) - EPS || at.x > js_max(u0, u1) + EPS
            {
                continue;
            }
            let k = (at.x - u0) / (u1 - u0);
            let d = frame.d(a) + (frame.d(b) - frame.d(a)) * k;
            if d >= -EPS && best.is_none_or(|(_, n)| d < n) {
                best = Some((i, d));
            }
        }
    }
    best
}

/// A ring as a cut gives it: counter-clockwise, from its lowest-leftmost point (the smallest `u`, then `z`), without
/// repeated points and points it runs straight through.
fn canonical(ring: Vec<Vec2>) -> Vec<Vec2> {
    let mut r = clean(&ring);
    let n = r.len();
    let area: f64 = (0..n)
        .map(|i| {
            let (p, q) = (r[i], r[(i + 1) % n]);
            p.x * q.y - q.x * p.y
        })
        .sum();
    if area < 0.0 {
        r.reverse();
    }
    if let Some(k) =
        (0..r.len()).min_by(|&a, &b| r[a].x.total_cmp(&r[b].x).then(r[a].y.total_cmp(&r[b].y)))
    {
        r.rotate_left(k);
    }
    r
}

/// A point to a micrometre, to tell the same edge again.
fn key(p: Vec2) -> (i64, i64) {
    (
        crate::jsmath::js_round(p.x * 1e6) as i64,
        crate::jsmath::js_round(p.y * 1e6) as i64,
    )
}

/// A ring's edges with their outward normals (unit): the side of each that is outside the rings, `SIDE` away from its
/// middle.
fn outward(rings: &[Vec<Vec2>]) -> Vec<(Vec2, Vec2, Vec2)> {
    let mut out = Vec::new();
    for r in rings {
        let n = r.len();
        for k in 0..n {
            let (p, q) = (r[k], r[(k + 1) % n]);
            let len = js_hypot(q.x - p.x, q.y - p.y);
            if len <= EPS {
                continue;
            }
            let m = Vec2::new((q.y - p.y) / len, -(q.x - p.x) / len);
            let mid = Vec2::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
            let m = if inside(rings, Vec2::new(mid.x + m.x * SIDE, mid.y + m.y * SIDE)) {
                Vec2::new(-m.x, -m.y)
            } else {
                m
            };
            out.push((p, q, m));
        }
    }
    out
}

/// Whether a normal looks back at the viewer.
fn facing(frame: &Frame, m: Vec2) -> bool {
    m.x * frame.look.x + m.y * frame.look.y < -1e-9
}

/// A vertical face's depth as a function of `u` (`(a, b, 0)`), from its ends; none when it stands edge-on.
fn vertical_depth(u0: f64, d0: f64, u1: f64, d1: f64) -> Option<(f64, f64, f64)> {
    if (u1 - u0).abs() <= EPS {
        return None;
    }
    let b = (d1 - d0) / (u1 - u0);
    Some((d0 - b * u0, b, 0.0))
}

/// A prism's vertical faces that look back at the viewer: covers, and their edges to draw.
fn prism_faces(
    frame: &Frame,
    solid: usize,
    rings: &[Vec<Vec2>],
    bottom: f64,
    top: f64,
    covers: &mut Vec<Cover>,
    edges: &mut Vec<Edge>,
) {
    for (p, q, m) in outward(rings) {
        if !facing(frame, m) {
            continue;
        }
        let Some((t0, t1)) = frame.clip(p, q) else {
            continue;
        };
        let at = |t: f64| Vec2::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t);
        let (a, b) = (at(t0), at(t1));
        let (u0, u1, d0, d1) = (frame.u(a), frame.u(b), frame.d(a), frame.d(b));
        let plane = vertical_depth(u0, d0, u1, d1);
        if let Some(depth) = plane {
            covers.extend(Cover::face(
                vec![
                    Vec2::new(u0, bottom),
                    Vec2::new(u1, bottom),
                    Vec2::new(u1, top),
                    Vec2::new(u0, top),
                ],
                depth,
                solid,
            ));
        }
        for z in [bottom, top] {
            edges.push(Edge {
                solid,
                p: Vec2::new(u0, z),
                q: Vec2::new(u1, z),
                dp: d0,
                dq: d1,
                plane,
            });
        }
        // A corner's vertical edge, never where the view clips the face.
        for (t, u, d) in [(t0, u0, d0), (t1, u1, d1)] {
            if t <= 1e-12 || t >= 1.0 - 1e-12 {
                edges.push(Edge {
                    solid,
                    p: Vec2::new(u, bottom),
                    q: Vec2::new(u, top),
                    dp: d,
                    dq: d,
                    plane,
                });
            }
        }
    }
}

/// A flight's cut, its trapezoids in order, as rings: one for each run of them that meet.
fn flight_cut_rings(traps: &[Trap]) -> Vec<Vec<Vec2>> {
    let mut sorted: Vec<Trap> = traps
        .iter()
        .map(|t| {
            if t.u0 <= t.u1 {
                *t
            } else {
                Trap {
                    u0: t.u1,
                    u1: t.u0,
                    b0: t.b1,
                    b1: t.b0,
                    d0: t.d1,
                    d1: t.d0,
                    ..*t
                }
            }
        })
        .collect();
    sorted.sort_by(|a, b| a.u0.total_cmp(&b.u0));
    let mut runs: Vec<Vec<Trap>> = Vec::new();
    for t in sorted {
        match runs.last_mut() {
            Some(run) if (run[run.len() - 1].u1 - t.u0).abs() <= EPS => run.push(t),
            _ => runs.push(vec![t]),
        }
    }
    runs.into_iter()
        .map(|run| {
            let mut ring = vec![Vec2::new(run[0].u0, run[0].b0)];
            for t in &run {
                ring.push(Vec2::new(t.u0, t.top));
                ring.push(Vec2::new(t.u1, t.top));
            }
            let last = run[run.len() - 1];
            ring.push(Vec2::new(last.u1, last.b1));
            for t in run.iter().rev() {
                ring.push(Vec2::new(t.u0, t.b0));
            }
            clean(&ring)
        })
        .filter(|r| r.len() >= 3)
        .collect()
}

/// A flight's faces that look back at the viewer: its sides, its risers and its underside; covers, and their edges.
fn flight_faces(
    frame: &Frame,
    solid: usize,
    rings: &[Vec<Vec2>],
    f: &Flight,
    covers: &mut Vec<Cover>,
    edges: &mut Vec<Edge>,
) {
    // Its sides: along the footprint's edges, each in trapezoids.
    for (p, q, m) in outward(rings) {
        if !facing(frame, m) {
            continue;
        }
        let Some((t0, t1)) = frame.clip(p, q) else {
            continue;
        };
        let traps = flight_traps(frame, f, p, q, (t0, t1));
        for (k, tr) in traps.iter().enumerate() {
            if let Some(depth) = vertical_depth(tr.u0, tr.d0, tr.u1, tr.d1) {
                covers.extend(Cover::new(trap_ring(tr), depth));
            }
            edges.push(Edge {
                solid,
                p: Vec2::new(tr.u0, tr.top),
                q: Vec2::new(tr.u1, tr.top),
                dp: tr.d0,
                dq: tr.d1,
                plane: None,
            });
            edges.push(Edge {
                solid,
                p: Vec2::new(tr.u0, tr.b0),
                q: Vec2::new(tr.u1, tr.b1),
                dp: tr.d0,
                dq: tr.d1,
                plane: None,
            });
            // Where the next steps up: the riser's line on the side.
            if let Some(next) = traps.get(k + 1)
                && (next.top - tr.top).abs() > EPS
            {
                edges.push(Edge {
                    solid,
                    p: Vec2::new(tr.u1, js_min(tr.top, next.top)),
                    q: Vec2::new(tr.u1, js_max(tr.top, next.top)),
                    dp: tr.d1,
                    dq: tr.d1,
                    plane: None,
                });
            }
        }
        // The corners' vertical edges, never where the view clips the side.
        if let Some(first) = traps.first()
            && t0 <= 1e-12
        {
            edges.push(Edge {
                solid,
                p: Vec2::new(first.u0, first.b0),
                q: Vec2::new(first.u0, first.top),
                dp: first.d0,
                dq: first.d0,
                plane: None,
            });
        }
        if let Some(last) = traps.last()
            && t1 >= 1.0 - 1e-12
        {
            edges.push(Edge {
                solid,
                p: Vec2::new(last.u1, last.b1),
                q: Vec2::new(last.u1, last.top),
                dp: last.d1,
                dq: last.d1,
                plane: None,
            });
        }
    }
    // Its risers: across it where its top steps, each looking down the flight.
    let across = Vec2::new(-f.dir.y, f.dir.x);
    for w in f.steps.windows(2) {
        let ((_, low), (from, high)) = (w[0], w[1]);
        let m = if high > low {
            Vec2::new(-f.dir.x, -f.dir.y)
        } else {
            f.dir
        };
        if (high - low).abs() <= EPS || !facing(frame, m) {
            continue;
        }
        let at = Vec2::new(f.start.x + f.dir.x * from, f.start.y + f.dir.y * from);
        for (w0, w1) in through(rings, at, across, f.dir, -1e9, 1e9) {
            let (p, q) = (
                Vec2::new(at.x + across.x * w0, at.y + across.y * w0),
                Vec2::new(at.x + across.x * w1, at.y + across.y * w1),
            );
            let Some((t0, t1)) = frame.clip(p, q) else {
                continue;
            };
            let pt = |t: f64| Vec2::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t);
            let (a, b) = (pt(t0), pt(t1));
            let (u0, u1, d0, d1) = (frame.u(a), frame.u(b), frame.d(a), frame.d(b));
            let (z0, z1) = (js_min(low, high), js_max(low, high));
            if let Some(depth) = vertical_depth(u0, d0, u1, d1) {
                covers.extend(Cover::new(
                    vec![
                        Vec2::new(u0, z0),
                        Vec2::new(u1, z0),
                        Vec2::new(u1, z1),
                        Vec2::new(u0, z1),
                    ],
                    depth,
                ));
            }
            for z in [z0, z1] {
                edges.push(Edge {
                    solid,
                    p: Vec2::new(u0, z),
                    q: Vec2::new(u1, z),
                    dp: d0,
                    dq: d1,
                    plane: None,
                });
            }
            // Its ends where it meets the flight's sides, never where the view clips it.
            for (t, u, d) in [(t0, u0, d0), (t1, u1, d1)] {
                if t <= 1e-12 || t >= 1.0 - 1e-12 {
                    edges.push(Edge {
                        solid,
                        p: Vec2::new(u, z0),
                        q: Vec2::new(u, z1),
                        dp: d,
                        dq: d,
                        plane: None,
                    });
                }
            }
        }
    }
    // Its underside where its slope is over its floor: a plane looking down and back along the flight.
    let kl = f.dir.x * frame.look.x + f.dir.y * frame.look.y;
    if f.slope.abs() <= EPS || f.slope * kl >= -1e-9 {
        return;
    }
    let Some(kink) = f.kink() else {
        return;
    };
    // The footprint where s is past the kink (upwards of it along the slope).
    let up = if f.slope > 0.0 { 1.0 } else { -1.0 };
    let keep = |p: Vec2| up * (f.s(p) - kink);
    let mut plan: Vec<Vec<Vec2>> = rings.iter().map(|r| clip_ring(r, &keep)).collect();
    // Within the view: past the line, within its depth and length.
    let bounds: Vec<Box<dyn Fn(Vec2) -> f64>> = {
        let fr = *frame;
        let mut b: Vec<Box<dyn Fn(Vec2) -> f64>> = vec![
            Box::new(move |p| fr.d(p)),
            Box::new(move |p| fr.u(p)),
            Box::new(move |p| fr.length - fr.u(p)),
        ];
        if let Some(depth) = fr.depth {
            b.push(Box::new(move |p| depth - fr.d(p)));
        }
        b
    };
    for bound in &bounds {
        plan = plan.iter().map(|r| clip_ring(r, bound.as_ref())).collect();
    }
    let s0 = f.s(frame.origin);
    let k = frame.right.x * f.dir.x + frame.right.y * f.dir.y;
    let base = f.soffit + f.slope * s0;
    // d = (z − base − slope·k·u) / (slope·kl).
    let depth = (-base / (f.slope * kl), -k / kl, 1.0 / (f.slope * kl));
    for ring in plan.into_iter().filter(|r| r.len() >= 3) {
        let lifted: Vec<(Vec2, f64)> = ring
            .iter()
            .map(|p| {
                (
                    Vec2::new(frame.u(*p), f.soffit + f.slope * f.s(*p)),
                    frame.d(*p),
                )
            })
            .collect();
        covers.extend(Cover::new(lifted.iter().map(|l| l.0).collect(), depth));
        let n = lifted.len();
        for j in 0..n {
            let (a, b) = (lifted[j], lifted[(j + 1) % n]);
            edges.push(Edge {
                solid,
                p: a.0,
                q: b.0,
                dp: a.1,
                dq: b.1,
                plane: None,
            });
        }
    }
}

/// What bounds the view in plan: past the line, within its length and its depth; each not below 0 inside.
fn view_bounds(frame: &Frame) -> Vec<Box<dyn Fn(Vec2) -> f64>> {
    let fr = *frame;
    let mut b: Vec<Box<dyn Fn(Vec2) -> f64>> = vec![
        Box::new(move |p| fr.d(p)),
        Box::new(move |p| fr.u(p)),
        Box::new(move |p| fr.length - fr.u(p)),
    ];
    if let Some(depth) = fr.depth {
        b.push(Box::new(move |p| depth - fr.d(p)));
    }
    b
}

/// A sloped slab's plane in the section's frame: `z = k0 + ku·u + kd·d` at its top.
fn sloped_frame(frame: &Frame, s: &Sloped) -> (f64, f64, f64) {
    (
        s.z(frame.origin),
        s.dx * frame.right.x + s.dy * frame.right.y,
        s.dx * frame.look.x + s.dy * frame.look.y,
    )
}

/// A sloped slab's faces that look back at the viewer (docs/adr/8044 §5): its top where its plane rises away from the
/// viewer, else its underside where it rises towards them (none where it stands edge-on), with its footprint's edges
/// within the view; and the vertical faces on its edges along the roof's eaves line, with their top, their bottom and
/// a vertical edge at each corner (never where the view clips the face): covers, and their edges to draw.
fn sloped_faces(
    frame: &Frame,
    solid: usize,
    rings: &[Vec<Vec2>],
    s: &Sloped,
    covers: &mut Vec<Cover>,
    edges: &mut Vec<Edge>,
) {
    let Some(ring) = rings.first() else {
        return;
    };
    let (k0, ku, kd) = sloped_frame(frame, s);
    for (lift, shown) in [(0.0, kd > 1e-9), (-s.thickness, kd < -1e-9)] {
        if !shown {
            continue;
        }
        let mut plan = ring.clone();
        for bound in view_bounds(frame) {
            if plan.len() < 3 {
                break;
            }
            plan = clip_ring(&plan, bound.as_ref());
        }
        if plan.len() >= 3 {
            // d = (z − k0 − lift − ku·u) / kd.
            let depth = (-(k0 + lift) / kd, -ku / kd, 1.0 / kd);
            covers.extend(Cover::new(
                plan.iter()
                    .map(|p| Vec2::new(frame.u(*p), s.z(*p) + lift))
                    .collect(),
                depth,
            ));
        }
        let n = ring.len();
        for k in 0..n {
            let (p, q) = (ring[k], ring[(k + 1) % n]);
            let Some((t0, t1)) = frame.clip(p, q) else {
                continue;
            };
            let on = |t: f64| Vec2::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t);
            let (a, b) = (on(t0), on(t1));
            edges.push(Edge {
                solid,
                p: Vec2::new(frame.u(a), s.z(a) + lift),
                q: Vec2::new(frame.u(b), s.z(b) + lift),
                dp: frame.d(a),
                dq: frame.d(b),
                plane: None,
            });
        }
    }
    for (p, q, m) in outward(rings) {
        if !s.open(p, q) || !facing(frame, m) {
            continue;
        }
        let Some((t0, t1)) = frame.clip(p, q) else {
            continue;
        };
        let on = |t: f64| Vec2::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t);
        let (a, b) = (on(t0), on(t1));
        let (u0, u1, d0, d1) = (frame.u(a), frame.u(b), frame.d(a), frame.d(b));
        let (z0, z1) = (s.z(a), s.z(b));
        let t = s.thickness;
        let plane = vertical_depth(u0, d0, u1, d1);
        if let Some(depth) = plane {
            covers.extend(Cover::face(
                vec![
                    Vec2::new(u0, z0 - t),
                    Vec2::new(u1, z1 - t),
                    Vec2::new(u1, z1),
                    Vec2::new(u0, z0),
                ],
                depth,
                solid,
            ));
        }
        for lift in [0.0, -t] {
            edges.push(Edge {
                solid,
                p: Vec2::new(u0, z0 + lift),
                q: Vec2::new(u1, z1 + lift),
                dp: d0,
                dq: d1,
                plane,
            });
        }
        // A corner's vertical edge, never where the view clips the face.
        for (tt, u, d, z) in [(t0, u0, d0, z0), (t1, u1, d1, z1)] {
            if tt <= 1e-12 || tt >= 1.0 - 1e-12 {
                edges.push(Edge {
                    solid,
                    p: Vec2::new(u, z - t),
                    q: Vec2::new(u, z),
                    dp: d,
                    dq: d,
                    plane,
                });
            }
        }
    }
}

/// How far past the line a sloped slab's face looking back at the viewer is at `at` (`u`, `z`): its top, its
/// underside or a vertical face on the eaves line; none where none of them is.
fn sloped_seen_at(frame: &Frame, solid: &Solid, s: &Sloped, at: Vec2) -> Option<f64> {
    let rings: Vec<Vec<Vec2>> = solid
        .rings
        .iter()
        .map(|r| clean(r))
        .filter(|r| r.len() >= 3)
        .collect();
    let (k0, ku, kd) = sloped_frame(frame, s);
    let mut best: Option<f64> = None;
    let mut take = |d: f64| {
        if d >= -EPS
            && frame.depth.is_none_or(|depth| d <= depth + EPS)
            && best.is_none_or(|b| d < b)
        {
            best = Some(d);
        }
    };
    for (lift, shown) in [(0.0, kd > 1e-9), (-s.thickness, kd < -1e-9)] {
        if shown {
            let d = (at.y - k0 - lift - ku * at.x) / kd;
            if inside(&rings, frame.at(at.x, d)) {
                take(d);
            }
        }
    }
    for (p, q, m) in outward(&rings) {
        if !s.open(p, q) || !facing(frame, m) {
            continue;
        }
        let (u0, u1) = (frame.u(p), frame.u(q));
        if (u1 - u0).abs() <= EPS || at.x < js_min(u0, u1) - EPS || at.x > js_max(u0, u1) + EPS {
            continue;
        }
        let k = (at.x - u0) / (u1 - u0);
        let w = Vec2::new(p.x + (q.x - p.x) * k, p.y + (q.y - p.y) * k);
        let z = s.z(w);
        if at.y >= z - s.thickness - EPS && at.y <= z + EPS {
            take(frame.d(w));
        }
    }
    best
}

/// A ring cut to where `keep` is not below 0 (Sutherland–Hodgman; a convex ring stays one ring).
fn clip_ring(ring: &[Vec2], keep: &dyn Fn(Vec2) -> f64) -> Vec<Vec2> {
    let n = ring.len();
    let mut out = Vec::new();
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        let (kp, kq) = (keep(p), keep(q));
        if kp >= 0.0 {
            out.push(p);
        }
        if (kp >= 0.0) != (kq >= 0.0) {
            let t = kp / (kp - kq);
            out.push(Vec2::new(p.x + (q.x - p.x) * t, p.y + (q.y - p.y) * t));
        }
    }
    clean(&out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Vec2> {
        vec![
            Vec2::new(x0, y0),
            Vec2::new(x1, y0),
            Vec2::new(x1, y1),
            Vec2::new(x0, y1),
        ]
    }

    fn prism(ring: Vec<Vec2>, bottom: f64, top: f64) -> Solid {
        Solid {
            rings: vec![ring],
            height: Height::Prism { bottom, top },
            voids: Vec::new(),
            marks: Vec::new(),
            finish: None,
        }
    }

    fn has(ring: &[Vec2], x: f64, y: f64) -> bool {
        ring.iter()
            .any(|p| (p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9)
    }

    fn east() -> Frame {
        // The line runs east along y = 0, looking north.
        Frame::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), false, None).unwrap()
    }

    #[test]
    fn a_wall_across_the_line_is_cut_and_one_behind_is_seen_round_it() {
        let cut = prism(rect(2.0, -1.0, 2.25, 4.0), 0.0, 3.0);
        let far = prism(rect(0.0, 4.0, 10.0, 4.25), 0.0, 3.0);
        let s = section(&east(), &[cut, far]);
        assert_eq!(s.cut.len(), 1);
        assert!(has(&s.cut[0].ring, 2.0, 0.0) && has(&s.cut[0].ring, 2.25, 3.0));
        // The far wall's face at y = 4 is seen, its top either side of the cut wall's strip.
        let mut tops: Vec<(f64, f64)> = s
            .seen
            .iter()
            .filter(|e| e.solid == 1 && (e.a.y - 3.0).abs() < 1e-9 && (e.b.y - 3.0).abs() < 1e-9)
            .map(|e| (js_min(e.a.x, e.b.x), js_max(e.a.x, e.b.x)))
            .collect();
        tops.sort_by(|a, b| a.0.total_cmp(&b.0));
        assert_eq!(tops.len(), 2, "{tops:?}");
        // Hidden by what covers it or lies on its outline, give or take the nanometre the test of inside allows.
        assert!(
            (tops[0].0).abs() < 1e-7 && (tops[0].1 - 2.0).abs() < 1e-7,
            "{tops:?}"
        );
        assert!(
            (tops[1].0 - 2.25).abs() < 1e-7 && (tops[1].1 - 10.0).abs() < 1e-7,
            "{tops:?}"
        );
        assert!(s.seen.iter().all(|e| (e.depth - 4.0).abs() < 1e-9));
    }

    #[test]
    fn looking_the_other_way_flips_the_line_and_sees_nothing_behind() {
        let flipped = Frame::new(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), true, None).unwrap();
        let far = prism(rect(0.0, 4.0, 10.0, 4.25), 0.0, 3.0);
        let s = section(&flipped, &[far]);
        assert!(s.cut.is_empty() && s.seen.is_empty());
        // u runs from the line's east end westwards: a wall south of it is seen mirrored.
        let south = prism(rect(1.0, -3.25, 4.0, -3.0), 0.0, 3.0);
        let s = section(&flipped, &[south]);
        let us: Vec<f64> = s.seen.iter().flat_map(|e| [e.a.x, e.b.x]).collect();
        assert!(
            us.iter().all(|u| *u > 6.0 - 1e-9 && *u < 9.0 + 1e-9),
            "{us:?}"
        );
    }

    #[test]
    fn a_face_on_the_line_is_seen_not_cut() {
        let s = section(&east(), &[prism(rect(1.0, 0.0, 3.0, 0.25), 0.0, 3.0)]);
        assert!(s.cut.is_empty());
        assert!(!s.seen.is_empty());
    }

    #[test]
    fn a_window_leaves_a_gap_in_the_cut_and_its_outline_shows_its_reveals() {
        let mut wall = prism(rect(2.0, -1.0, 2.25, 4.0), 0.0, 3.0);
        wall.voids.push(Void {
            ring: rect(2.0, -0.6, 2.25, 0.6),
            bottom: 0.9,
            top: 2.1,
        });
        // Its outline on both faces of the wall: seen only where the gap lets it be.
        for x in [2.0, 2.25] {
            let c = [
                (Vec2::new(x, -0.6), 0.9),
                (Vec2::new(x, 0.6), 0.9),
                (Vec2::new(x, 0.6), 2.1),
                (Vec2::new(x, -0.6), 2.1),
            ];
            for k in 0..4 {
                wall.marks.push([c[k], c[(k + 1) % 4]]);
            }
        }
        let s = section(&east(), &[wall]);
        let mut rings: Vec<Vec<Vec2>> = s.cut.iter().map(|c| c.ring.clone()).collect();
        rings.sort_by(|a, b| a[0].y.total_cmp(&b[0].y));
        assert_eq!(rings.len(), 2);
        assert!(has(&rings[0], 2.0, 0.0) && has(&rings[0], 2.25, 0.9));
        assert!(has(&rings[1], 2.0, 2.1) && has(&rings[1], 2.25, 3.0));
        // The reveals: both faces' jamb lines from the sill to the head, 0.6 m past the line.
        let reveals: Vec<&Seen> = s
            .seen
            .iter()
            .filter(|e| (e.a.x - e.b.x).abs() < 1e-9 && (js_min(e.a.y, e.b.y) - 0.9).abs() < 1e-9)
            .collect();
        assert_eq!(reveals.len(), 2, "{:?}", s.seen);
        assert!(reveals.iter().all(|e| (e.depth - 0.6).abs() < 1e-9));
    }

    #[test]
    fn a_flight_along_the_line_cuts_as_steps_over_its_slope() {
        // Three risers of 0.2 over treads of 0.3 from x = 1 eastwards; its underside 0.1 under the steps' roots.
        let f = Flight {
            start: Vec2::new(1.0, 0.0),
            dir: Vec2::new(1.0, 0.0),
            steps: vec![(0.0, 0.2), (0.3, 0.4), (0.6, 0.6)],
            soffit: -0.1,
            slope: 0.2 / 0.3,
            floor: 0.0,
        };
        let solid = Solid {
            rings: vec![rect(1.0, -0.5, 1.9, 0.5)],
            height: Height::Flight(f),
            voids: Vec::new(),
            marks: Vec::new(),
            finish: None,
        };
        let s = section(&east(), &[solid]);
        assert_eq!(s.cut.len(), 1);
        let ring = &s.cut[0].ring;
        // Up the first riser, the treads, down the far end to the slope, back along the slope and the floor.
        for (x, y) in [
            (1.0, 0.0),
            (1.0, 0.2),
            (1.3, 0.2),
            (1.3, 0.4),
            (1.6, 0.6),
            (1.9, 0.6),
            (1.9, 0.5),
            (1.15, 0.0),
        ] {
            assert!(has(ring, x, y), "({x}, {y}) in {ring:?}");
        }
        // The slope from where it leaves the floor to the far end is one straight line: no corner on it.
        assert_eq!(ring.len(), 9, "{ring:?}");
    }

    #[test]
    fn a_flight_cut_across_on_a_riser_is_one_cut_with_no_nosing_through_it() {
        // The same steps running north from y = −0.3: the line (y = 0) is on the second riser, 0.2 to 0.4.
        let f = Flight {
            start: Vec2::new(5.0, -0.3),
            dir: Vec2::new(0.0, 1.0),
            steps: vec![(0.0, 0.2), (0.3, 0.4), (0.6, 0.6)],
            soffit: -0.1,
            slope: 0.2 / 0.3,
            floor: 0.0,
        };
        let solid = Solid {
            rings: vec![rect(4.5, -0.3, 5.5, 0.6)],
            height: Height::Flight(f),
            voids: Vec::new(),
            marks: Vec::new(),
            finish: None,
        };
        let s = section(&east(), &[solid]);
        assert_eq!(s.cut.len(), 1, "{:?}", s.cut);
        assert!(
            has(&s.cut[0].ring, 4.5, 0.4) && has(&s.cut[0].ring, 5.5, 0.4),
            "{:?}",
            s.cut
        );
        // The riser on the line is the cut's face: the nosing at its foot (0.2) is not drawn through the hatch.
        let level = |z: f64| {
            s.seen
                .iter()
                .filter(|e| (e.a.y - z).abs() < 1e-9 && (e.b.y - z).abs() < 1e-9)
                .count()
        };
        assert_eq!(level(0.2), 0, "{:?}", s.seen);
        // The next riser, 0.3 past the line and facing it, is seen: its nosing at 0.6.
        assert_eq!(level(0.6), 1, "{:?}", s.seen);
    }

    #[test]
    fn what_is_seen_at_a_point_is_the_nearest_face_looking_back() {
        // A wall's face at y 4 from 0 to 3 m up; a column 0.3 m in front of it between x 4.85 and 5.15.
        let wall = prism(rect(0.0, 4.0, 10.0, 4.25), 0.0, 3.0);
        let column = prism(rect(4.85, 3.7, 5.15, 4.0), 0.0, 3.0);
        let solids = [wall, column];
        let at = |u: f64, z: f64| seen_at(&east(), &solids, Vec2::new(u, z));
        let hit = |r: Option<(usize, f64)>, k: usize, d: f64| {
            r.is_some_and(|(i, e)| i == k && (e - d).abs() < 1e-9)
        };
        assert!(hit(at(2.0, 1.5), 0, 4.0), "{:?}", at(2.0, 1.5));
        assert!(hit(at(5.0, 1.5), 1, 3.7), "{:?}", at(5.0, 1.5));
        assert_eq!(at(2.0, 3.5), None, "over the wall");
        assert_eq!(at(11.0, 1.5), None, "past its end");
    }
}
