//! The plane geometry file formats need, written here because this crate
//! depends on nothing but the contracts (the analytic core lives in
//! `kentos-geometry-core`, which the formats do not link): 2D affine maps
//! with exact identity and quarter turns, DXF's object coordinate systems,
//! an ellipse from any parameterisation (the image of a circle under a
//! non-uniform map), and arc sampling for boundaries the model keeps as
//! point rings (hatches: 72 segments per turn, as the app does).

use kentos_contracts::Vec2;

use crate::math::{atan, atan2, cos, hypot, norm_angle, sin, sin_cos_deg, TAU};

pub const fn v(x: f64, y: f64) -> Vec2 {
    Vec2 { x, y }
}

pub fn dist(a: Vec2, b: Vec2) -> f64 {
    hypot(b.x - a.x, b.y - a.y)
}

pub fn finite(p: Vec2) -> bool {
    p.x.is_finite() && p.y.is_finite()
}

/// 2D affine map as the app writes it: x' = a·x + c·y + e, y' = b·x + d·y + f.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tf {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

/// A map that keeps shapes: uniform scale, a rotation (radians, of the x axis) and maybe a mirror.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Similarity {
    pub scale: f64,
    pub angle: f64,
    pub mirror: bool,
}

impl Tf {
    pub const IDENTITY: Tf = Tf { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    pub fn translate(dx: f64, dy: f64) -> Tf {
        Tf { e: dx, f: dy, ..Tf::IDENTITY }
    }

    /// Rotation by `deg` degrees counter-clockwise (exact for quarter turns).
    pub fn rotate_deg(deg: f64) -> Tf {
        let (s, c) = sin_cos_deg(deg);
        Tf { a: c, b: s, c: -s, d: c, e: 0.0, f: 0.0 }
    }

    pub fn scale(sx: f64, sy: f64) -> Tf {
        Tf { a: sx, d: sy, ..Tf::IDENTITY }
    }

    pub fn is_identity(&self) -> bool {
        *self == Tf::IDENTITY
    }

    pub fn apply(&self, p: Vec2) -> Vec2 {
        if self.is_identity() {
            return p;
        }
        v(self.a * p.x + self.c * p.y + self.e, self.b * p.x + self.d * p.y + self.f)
    }

    /// The linear part only (for directions and axis vectors).
    pub fn linear(&self, p: Vec2) -> Vec2 {
        v(self.a * p.x + self.c * p.y, self.b * p.x + self.d * p.y)
    }

    /// First `self`, then `outer`.
    pub fn then(&self, outer: &Tf) -> Tf {
        if self.is_identity() {
            return *outer;
        }
        if outer.is_identity() {
            return *self;
        }
        let o = outer;
        Tf {
            a: o.a * self.a + o.c * self.b,
            b: o.b * self.a + o.d * self.b,
            c: o.a * self.c + o.c * self.d,
            d: o.b * self.c + o.d * self.d,
            e: o.a * self.e + o.c * self.f + o.e,
            f: o.b * self.e + o.d * self.f + o.f,
        }
    }

    pub fn det(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// The map as scale, rotation and mirror, when it keeps shapes (to 1e-10).
    pub fn similarity(&self) -> Option<Similarity> {
        let (ux, uy, vx, vy) = (self.a, self.b, self.c, self.d);
        let lu = hypot(ux, uy);
        let lv = hypot(vx, vy);
        if !(lu > 0.0 && lv > 0.0) {
            return None;
        }
        let dot = ux * vx + uy * vy;
        if dot.abs() > 1e-10 * lu * lv || (lu - lv).abs() > 1e-10 * lu.max(lv) {
            return None;
        }
        Some(Similarity { scale: lu, angle: if uy == 0.0 && ux > 0.0 { 0.0 } else { atan2(uy, ux) }, mirror: self.det() < 0.0 })
    }
}

/// DXF's arbitrary axis algorithm: the object coordinate system of an
/// extrusion direction, as unit x and y axes and the normal (all in world
/// coordinates). None for a zero or non-finite direction.
pub fn ocs_axes(n: [f64; 3]) -> Option<([f64; 3], [f64; 3], [f64; 3])> {
    let l = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
    if !(l > 0.0) || !l.is_finite() {
        return None;
    }
    let n = [n[0] / l, n[1] / l, n[2] / l];
    let limit = 1.0 / 64.0;
    let ax = if n[0].abs() < limit && n[1].abs() < limit {
        // Wy × N
        [n[2], 0.0, -n[0]]
    } else {
        // Wz × N
        [-n[1], n[0], 0.0]
    };
    let la = (ax[0] * ax[0] + ax[1] * ax[1] + ax[2] * ax[2]).sqrt();
    let ax = [ax[0] / la, ax[1] / la, ax[2] / la];
    let ay = [n[1] * ax[2] - n[2] * ax[1], n[2] * ax[0] - n[0] * ax[2], n[0] * ax[1] - n[1] * ax[0]];
    let ly = (ay[0] * ay[0] + ay[1] * ay[1] + ay[2] * ay[2]).sqrt();
    Some((ax, [ay[0] / ly, ay[1] / ly, ay[2] / ly], n))
}

/// The world-plane map of an entity's object coordinates at `elevation`
/// (Z dropped): exactly the identity for the usual +Z extrusion, a mirror
/// of x for (0, 0, −1).
pub fn ocs_tf(n: [f64; 3], elevation: f64) -> Option<Tf> {
    if n == [0.0, 0.0, 1.0] {
        return Some(Tf::IDENTITY);
    }
    let (ax, ay, nz) = ocs_axes(n)?;
    Some(Tf { a: ax[0], b: ax[1], c: ay[0], d: ay[1], e: elevation * nz[0], f: elevation * nz[1] })
}

/// An ellipse in the model's form: centre, major axis, minor/major ratio ≤ 1, parameters t0 → t1 (equal: whole).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EllipseParts {
    pub c: Vec2,
    pub major: Vec2,
    pub ratio: f64,
    pub t0: f64,
    pub t1: f64,
}

/// The ellipse P(t) = c + u·cos t + v·sin t for t from t0 to t1 (u and v
/// any two independent vectors, the image of a circle or of another
/// ellipse under an affine map), in the model's form: counter-clockwise
/// parameter, principal axes, the longer one first. None when it is flat.
pub fn ellipse_from(c: Vec2, u: Vec2, mut vv: Vec2, mut t0: f64, mut t1: f64, full: bool) -> Option<EllipseParts> {
    if u.x * vv.y - u.y * vv.x < 0.0 {
        // Clockwise: t → −t turns it counter-clockwise.
        vv = v(-vv.x, -vv.y);
        (t0, t1) = (-t1, -t0);
    }
    let uu = u.x * u.x + u.y * u.y;
    let vvv = vv.x * vv.x + vv.y * vv.y;
    let uv = u.x * vv.x + u.y * vv.y;
    let mut phi = if uv == 0.0 && uu >= vvv { 0.0 } else { 0.5 * atan2(2.0 * uv, uu - vvv) };
    let (s, co) = (sin(phi), cos(phi));
    let mut a = v(u.x * co + vv.x * s, u.y * co + vv.y * s);
    let mut b = v(-u.x * s + vv.x * co, -u.y * s + vv.y * co);
    if hypot(a.x, a.y) < hypot(b.x, b.y) {
        // The other axis is the major one: a quarter turn of the parameter.
        (a, b) = (b, v(-a.x, -a.y));
        phi += TAU / 4.0;
    }
    let la = hypot(a.x, a.y);
    let lb = hypot(b.x, b.y);
    if !(la > 0.0) || !(lb > 1e-12 * la) || !finite(a) {
        return None;
    }
    let ratio = (lb / la).min(1.0);
    let (t0, t1) = if full { (0.0, 0.0) } else { (norm_angle(t0 - phi), norm_angle(t1 - phi)) };
    Some(EllipseParts { c, major: a, ratio, t0, t1 })
}

/// Segments per full turn when an arc becomes points (the app's hatch boundaries).
pub const ARC_SEGMENTS: f64 = 72.0;

/// Points of an arc after its start: from `a0` turning `sweep` radians
/// (negative: clockwise), ending exactly at `end` when given.
pub fn arc_points(c: Vec2, r: f64, a0: f64, sweep: f64, end: Option<Vec2>, out: &mut Vec<Vec2>) {
    let n = ((sweep.abs() / TAU) * ARC_SEGMENTS).ceil().max(1.0) as usize;
    for i in 1..=n {
        if i == n
            && let Some(e) = end
        {
            out.push(e);
            break;
        }
        let t = a0 + sweep * (i as f64) / (n as f64);
        out.push(v(c.x + r * cos(t), c.y + r * sin(t)));
    }
}

/// A bulged segment a → b (bulge = tan(θ/4), counter-clockwise positive) as
/// its circle: centre, radius, start angle and signed sweep; None if straight.
pub fn bulge_arc(a: Vec2, b: Vec2, bulge: f64) -> Option<(Vec2, f64, f64, f64)> {
    if bulge == 0.0 || !bulge.is_finite() {
        return None;
    }
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l = hypot(dx, dy);
    if !(l > 0.0) {
        return None;
    }
    let f = (1.0 - bulge * bulge) / (4.0 * bulge);
    let c = v((a.x + b.x) / 2.0 - f * dy, (a.y + b.y) / 2.0 + f * dx);
    let r = l * (1.0 + bulge * bulge) / (4.0 * bulge.abs());
    Some((c, r, atan2(a.y - c.y, a.x - c.x), 4.0 * atan(bulge)))
}

/// A ring of vertices with bulges as points (arcs sampled), without repeating the first point.
pub fn bulge_ring_points(pts: &[Vec2], bulges: &[f64]) -> Vec<Vec2> {
    let n = pts.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        out.push(a);
        if let Some((c, r, a0, sweep)) = bulge_arc(a, b, bulges.get(i).copied().unwrap_or(0.0)) {
            arc_points(c, r, a0, sweep, Some(b), &mut out);
            // The end point is the next vertex, pushed on its own turn.
            out.pop();
        }
    }
    out
}

/// Twice the signed area of a ring (positive counter-clockwise), accumulated from the first point.
pub fn signed_area2(ring: &[Vec2]) -> f64 {
    let Some(&o) = ring.first() else { return 0.0 };
    let mut s = 0.0;
    for i in 1..ring.len().saturating_sub(1) {
        let (p, q) = (ring[i], ring[i + 1]);
        s += (p.x - o.x) * (q.y - o.y) - (q.x - o.x) * (p.y - o.y);
    }
    s
}

/// The app's spline (src/model/geom/spline.ts: centripetal Catmull-Rom
/// through `pts`, evaluated with Barry–Goldman, phantom end points mirrored
/// for open curves) as its cubic Bézier spans [start, control, control, end].
/// Every span is a cubic polynomial, so this is the same curve, not a fit:
/// the inner controls sit a third of the span's knot interval along the
/// curve's derivative at each end. Fewer than three points are drawn by the
/// app as straight segments, and come out as straight spans.
pub fn catmull_rom_beziers(pts: &[Vec2], closed: bool) -> Vec<[Vec2; 4]> {
    let n = pts.len();
    let straight = |a: Vec2, b: Vec2| [a, v(a.x + (b.x - a.x) / 3.0, a.y + (b.y - a.y) / 3.0), v(b.x - (b.x - a.x) / 3.0, b.y - (b.y - a.y) / 3.0), b];
    if n < 3 {
        return pts.windows(2).map(|w| straight(w[0], w[1])).collect();
    }
    let at = |i: isize| -> Vec2 {
        let last = n as isize - 1;
        if closed {
            pts[i.rem_euclid(n as isize) as usize]
        } else if i < 0 {
            v(2.0 * pts[0].x - pts[1].x, 2.0 * pts[0].y - pts[1].y)
        } else if i > last {
            v(2.0 * pts[n - 1].x - pts[n - 2].x, 2.0 * pts[n - 1].y - pts[n - 2].y)
        } else {
            pts[i as usize]
        }
    };
    // Centripetal knot spacing, with the app's floor for coincident points.
    let knot = |a: Vec2, b: Vec2| {
        let d = dist(a, b).sqrt();
        if d > 0.0 { d } else { 1e-6 }
    };
    let spans = if closed { n } else { n - 1 };
    let mut out = Vec::with_capacity(spans);
    for i in 0..spans as isize {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let (d01, d12, d23) = (knot(p0, p1), knot(p1, p2), knot(p2, p3));
        // dC/dt at t1 and t2 from the differences (small numbers even at TM coordinates).
        let (u01, u12, u23) = (v(p1.x - p0.x, p1.y - p0.y), v(p2.x - p1.x, p2.y - p1.y), v(p3.x - p2.x, p3.y - p2.y));
        let (k0, k1) = (d12 / (d01 * (d01 + d12)), d01 / (d12 * (d01 + d12)));
        let m1 = v(u01.x * k0 + u12.x * k1, u01.y * k0 + u12.y * k1);
        let (k2, k3) = (d23 / (d12 * (d12 + d23)), d12 / (d23 * (d12 + d23)));
        let m2 = v(u12.x * k2 + u23.x * k3, u12.y * k2 + u23.y * k3);
        let h = d12 / 3.0;
        out.push([p1, v(p1.x + m1.x * h, p1.y + m1.y * h), v(p2.x - m2.x * h, p2.y - m2.y * h), p2]);
    }
    out
}

/// Even-odd point in ring test.
pub fn inside(p: Vec2, ring: &[Vec2]) -> bool {
    let mut odd = false;
    let n = ring.len();
    let mut j = n.wrapping_sub(1);
    for i in 0..n {
        let (a, b) = (ring[i], ring[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            odd = !odd;
        }
        j = i;
    }
    odd
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::PI;

    #[test]
    fn identity_and_quarter_turns_are_exact() {
        let p = v(452345.123, 4412345.678);
        assert_eq!(Tf::IDENTITY.apply(p), p);
        let r = Tf::rotate_deg(90.0).then(&Tf::translate(10.0, 20.0));
        assert_eq!(r.apply(v(1.0, 0.0)), v(10.0, 21.0));
        let s = r.similarity().expect("similar");
        assert_eq!((s.scale, s.mirror), (1.0, false));
        assert!((s.angle - PI / 2.0).abs() < 1e-15);
        assert_eq!(Tf::scale(2.0, 3.0).similarity(), None);
        assert!(Tf::scale(-2.0, 2.0).similarity().is_some_and(|s| s.mirror && s.scale == 2.0));
    }

    #[test]
    fn the_arbitrary_axis_algorithm() {
        assert_eq!(ocs_tf([0.0, 0.0, 1.0], 5.0), Some(Tf::IDENTITY));
        // The common mirrored case: x flips, y stays.
        let m = ocs_tf([0.0, 0.0, -1.0], 0.0).expect("ocs");
        assert_eq!(m.apply(v(3.0, 4.0)), v(-3.0, 4.0));
        // A tilted normal: axes stay unit and orthogonal to it.
        let (ax, ay, n) = ocs_axes([1.0, 1.0, 1.0]).expect("axes");
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        assert!(dot(ax, n).abs() < 1e-15 && dot(ay, n).abs() < 1e-15 && dot(ax, ay).abs() < 1e-15);
        assert!(ocs_axes([0.0, 0.0, 0.0]).is_none());
    }

    #[test]
    fn an_ellipse_from_a_stretched_circle() {
        // A unit circle scaled 3 × 1: major along x, ratio 1/3, whole.
        let e = ellipse_from(v(0.0, 0.0), v(3.0, 0.0), v(0.0, 1.0), 0.0, 0.0, true).expect("ellipse");
        assert_eq!((e.major, e.ratio, e.t0, e.t1), (v(3.0, 0.0), 1.0 / 3.0, 0.0, 0.0));
        // Taller than wide: the major axis turns to y, the parameter by a quarter.
        let e = ellipse_from(v(0.0, 0.0), v(1.0, 0.0), v(0.0, 2.0), 0.0, PI / 2.0, false).expect("ellipse");
        assert!((e.major.x).abs() < 1e-15 && (e.major.y - 2.0).abs() < 1e-15 && (e.ratio - 0.5).abs() < 1e-15);
        // The point at the old t = π/2 is the major end: new t1 = 0.
        assert!(e.t1.abs() < 1e-15 || (e.t1 - TAU).abs() < 1e-15, "{e:?}");
        // Mirrored (clockwise) parameterisation: the arc keeps its points.
        let e = ellipse_from(v(0.0, 0.0), v(2.0, 0.0), v(0.0, -1.0), 0.0, PI / 2.0, false).expect("ellipse");
        let at = |t: f64| {
            let m = v(-e.major.y * e.ratio, e.major.x * e.ratio);
            v(e.c.x + e.major.x * cos(t) + m.x * sin(t), e.c.y + e.major.y * cos(t) + m.y * sin(t))
        };
        let (p0, p1) = (at(e.t0), at(e.t1));
        // Old t = 0 → (2, 0), old t = π/2 → (0, −1); the model's arc runs from (0, −1) to (2, 0).
        assert!(dist(p0, v(0.0, -1.0)) < 1e-12 && dist(p1, v(2.0, 0.0)) < 1e-12, "{p0:?} {p1:?}");
    }

    /// spline.ts's evaluation, line for line.
    fn barry_goldman(p: [Vec2; 4], t: f64) -> Vec2 {
        let knot = |a: Vec2, b: Vec2| {
            let d = dist(a, b).sqrt();
            if d > 0.0 { d } else { 1e-6 }
        };
        let t0 = 0.0;
        let t1 = t0 + knot(p[0], p[1]);
        let t2 = t1 + knot(p[1], p[2]);
        let t3 = t2 + knot(p[2], p[3]);
        let lerp = |a: Vec2, b: Vec2, ta: f64, tb: f64| {
            let d = tb - ta;
            let (u, w) = ((tb - t1 - t * (t2 - t1)) / d, (t1 + t * (t2 - t1) - ta) / d);
            v(a.x * u + b.x * w, a.y * u + b.y * w)
        };
        let (a1, a2, a3) = (lerp(p[0], p[1], t0, t1), lerp(p[1], p[2], t1, t2), lerp(p[2], p[3], t2, t3));
        let (b1, b2) = (lerp(a1, a2, t0, t2), lerp(a2, a3, t1, t3));
        lerp(b1, b2, t1, t2)
    }

    fn bezier(b: &[Vec2; 4], s: f64) -> Vec2 {
        let r = 1.0 - s;
        let (c0, c1, c2, c3) = (r * r * r, 3.0 * r * r * s, 3.0 * r * s * s, s * s * s);
        v(b[0].x * c0 + b[1].x * c1 + b[2].x * c2 + b[3].x * c3, b[0].y * c0 + b[1].y * c1 + b[2].y * c2 + b[3].y * c3)
    }

    #[test]
    fn catmull_rom_spans_are_the_apps_curve() {
        // Uneven spacing, a sharp turn and TM-sized coordinates.
        let base = v(452000.0, 4412000.0);
        let local = [v(0.0, 0.0), v(10.0, 2.0), v(12.0, 15.0), v(40.0, 16.0), v(41.0, 0.0)];
        let pts: Vec<Vec2> = local.iter().map(|p| v(base.x + p.x, base.y + p.y)).collect();
        for closed in [false, true] {
            let spans = catmull_rom_beziers(&pts, closed);
            assert_eq!(spans.len(), if closed { 5 } else { 4 });
            let n = pts.len() as isize;
            let at = |i: isize| -> Vec2 {
                if closed {
                    pts[i.rem_euclid(n) as usize]
                } else if i < 0 {
                    v(2.0 * pts[0].x - pts[1].x, 2.0 * pts[0].y - pts[1].y)
                } else if i >= n {
                    v(2.0 * pts[4].x - pts[3].x, 2.0 * pts[4].y - pts[3].y)
                } else {
                    pts[i as usize]
                }
            };
            for (i, b) in spans.iter().enumerate() {
                let i = i as isize;
                // The spans join at the points.
                assert_eq!((b[0], b[3]), (at(i), at(i + 1)));
                for k in 0..=8 {
                    let s = k as f64 / 8.0;
                    let want = barry_goldman([at(i - 1), at(i), at(i + 1), at(i + 2)], s);
                    let got = bezier(b, s);
                    assert!(dist(want, got) < 1e-8, "span {i} s {s}: {want:?} {got:?}");
                }
            }
        }
        // Two points: one straight span.
        let two = catmull_rom_beziers(&pts[..2], false);
        assert_eq!(two.len(), 1);
        assert!(dist(bezier(&two[0], 0.5), v(base.x + 5.0, base.y + 1.0)) < 1e-9);
        assert!(catmull_rom_beziers(&pts[..1], false).is_empty());
    }

    #[test]
    fn bulges_and_rings() {
        // A half circle from (0,0) to (2,0) bulging down (bulge 1, counter-clockwise).
        let (c, r, a0, sweep) = bulge_arc(v(0.0, 0.0), v(2.0, 0.0), 1.0).expect("arc");
        assert!(dist(c, v(1.0, 0.0)) < 1e-15 && (r - 1.0).abs() < 1e-15 && (sweep - PI).abs() < 1e-15);
        assert!((a0 - PI).abs() < 1e-15);
        let ring = bulge_ring_points(&[v(0.0, 0.0), v(2.0, 0.0)], &[1.0, 1.0]);
        assert_eq!(ring.len(), 72);
        assert!((signed_area2(&ring) / 2.0 - PI).abs() < 0.01);
        assert!(inside(v(1.0, 0.5), &ring) && !inside(v(3.0, 0.0), &ring));
    }
}
