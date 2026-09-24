//! The plane geometry file formats need. What the app itself computes comes
//! from the shared core (`kentos-geometry-core`, CLAUDE.md §14): a bulged
//! path as points (the app's hatch rings), ring area and point in ring.
//! What only a file reader needs is here: 2D affine maps with exact identity
//! and quarter turns, DXF's object coordinate systems, an ellipse from any
//! parameterisation (the image of a circle under a non-uniform map), and
//! arcs of DXF hatch boundaries sampled with the app's step (72 per turn).

use kentos_contracts::Vec2;
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::arc::DEFAULT_STEP;
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::geometry::{point_in_polygon, signed_area};

use crate::math::{TAU, atan2, cos, hypot, norm_angle, sin, sin_cos_deg};

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
    pub const IDENTITY: Tf = Tf {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    pub fn translate(dx: f64, dy: f64) -> Tf {
        Tf {
            e: dx,
            f: dy,
            ..Tf::IDENTITY
        }
    }

    /// Rotation by `deg` degrees counter-clockwise (exact for quarter turns).
    pub fn rotate_deg(deg: f64) -> Tf {
        let (s, c) = sin_cos_deg(deg);
        Tf {
            a: c,
            b: s,
            c: -s,
            d: c,
            e: 0.0,
            f: 0.0,
        }
    }

    pub fn scale(sx: f64, sy: f64) -> Tf {
        Tf {
            a: sx,
            d: sy,
            ..Tf::IDENTITY
        }
    }

    pub fn is_identity(&self) -> bool {
        *self == Tf::IDENTITY
    }

    pub fn apply(&self, p: Vec2) -> Vec2 {
        if self.is_identity() {
            return p;
        }
        v(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
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
        Some(Similarity {
            scale: lu,
            angle: if uy == 0.0 && ux > 0.0 {
                0.0
            } else {
                atan2(uy, ux)
            },
            mirror: self.det() < 0.0,
        })
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
    let ay = [
        n[1] * ax[2] - n[2] * ax[1],
        n[2] * ax[0] - n[0] * ax[2],
        n[0] * ax[1] - n[1] * ax[0],
    ];
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
    Some(Tf {
        a: ax[0],
        b: ax[1],
        c: ay[0],
        d: ay[1],
        e: elevation * nz[0],
        f: elevation * nz[1],
    })
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
pub fn ellipse_from(
    c: Vec2,
    u: Vec2,
    mut vv: Vec2,
    mut t0: f64,
    mut t1: f64,
    full: bool,
) -> Option<EllipseParts> {
    if u.x * vv.y - u.y * vv.x < 0.0 {
        // Clockwise: t → −t turns it counter-clockwise.
        vv = v(-vv.x, -vv.y);
        (t0, t1) = (-t1, -t0);
    }
    let uu = u.x * u.x + u.y * u.y;
    let vvv = vv.x * vv.x + vv.y * vv.y;
    let uv = u.x * vv.x + u.y * vv.y;
    let mut phi = if uv == 0.0 && uu >= vvv {
        0.0
    } else {
        0.5 * atan2(2.0 * uv, uu - vvv)
    };
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
    let (t0, t1) = if full {
        (0.0, 0.0)
    } else {
        (norm_angle(t0 - phi), norm_angle(t1 - phi))
    };
    Some(EllipseParts {
        c,
        major: a,
        ratio,
        t0,
        t1,
    })
}

/// Segments of an arc of `sweep` radians, by the app's rule (`tessellateArc`,
/// `bulgePathOutline`): 72 per turn, at least two.
pub fn arc_steps(sweep: f64) -> usize {
    (sweep.abs() / DEFAULT_STEP).ceil().max(2.0) as usize
}

/// Points of an arc after its start: from `a0` turning `sweep` radians
/// (negative: clockwise, as DXF hatch edges may run), ending exactly at
/// `end` when given.
pub fn arc_points(c: Vec2, r: f64, a0: f64, sweep: f64, end: Option<Vec2>, out: &mut Vec<Vec2>) {
    let n = arc_steps(sweep);
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

/// Points in the shared core's form.
pub fn to_core(pts: &[Vec2]) -> Vec<CoreVec2> {
    pts.iter().map(|p| CoreVec2::new(p.x, p.y)).collect()
}

/// Whether a bulge list has an arc, by the core's threshold (a bulge within
/// 10⁻¹² of zero is a straight segment to the app).
pub fn has_arcs(bulges: &[f64]) -> bool {
    kentos_geometry_core::geom::bulge::has_bulges(Some(bulges))
}

/// A vertex path with bulges as points, exactly as the app samples its own
/// (`polygonRing` → `bulgePathOutline`, in the shared core): a closed path
/// gives a ring without repeating its first point, an open one ends at its
/// last vertex.
pub fn bulge_path_points(pts: &[Vec2], bulges: &[f64], closed: bool) -> Vec<Vec2> {
    bulge_path_outline(&to_core(pts), Some(bulges), closed, DEFAULT_STEP)
        .into_iter()
        .map(|p| v(p.x, p.y))
        .collect()
}

/// Absolute area of a ring (the shared core's shoelace, taken from its first point).
pub fn ring_area(ring: &[CoreVec2]) -> f64 {
    signed_area(ring).abs()
}

/// Even-odd point in ring test (the shared core's).
pub fn ring_contains(ring: &[CoreVec2], p: Vec2) -> bool {
    point_in_polygon(CoreVec2::new(p.x, p.y), ring)
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
        assert!(
            Tf::scale(-2.0, 2.0)
                .similarity()
                .is_some_and(|s| s.mirror && s.scale == 2.0)
        );
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
        let e =
            ellipse_from(v(0.0, 0.0), v(3.0, 0.0), v(0.0, 1.0), 0.0, 0.0, true).expect("ellipse");
        assert_eq!(
            (e.major, e.ratio, e.t0, e.t1),
            (v(3.0, 0.0), 1.0 / 3.0, 0.0, 0.0)
        );
        // Taller than wide: the major axis turns to y, the parameter by a quarter.
        let e = ellipse_from(v(0.0, 0.0), v(1.0, 0.0), v(0.0, 2.0), 0.0, PI / 2.0, false)
            .expect("ellipse");
        assert!(
            (e.major.x).abs() < 1e-15
                && (e.major.y - 2.0).abs() < 1e-15
                && (e.ratio - 0.5).abs() < 1e-15
        );
        // The point at the old t = π/2 is the major end: new t1 = 0.
        assert!(e.t1.abs() < 1e-15 || (e.t1 - TAU).abs() < 1e-15, "{e:?}");
        // Mirrored (clockwise) parameterisation: the arc keeps its points.
        let e = ellipse_from(v(0.0, 0.0), v(2.0, 0.0), v(0.0, -1.0), 0.0, PI / 2.0, false)
            .expect("ellipse");
        let at = |t: f64| {
            let m = v(-e.major.y * e.ratio, e.major.x * e.ratio);
            v(
                e.c.x + e.major.x * cos(t) + m.x * sin(t),
                e.c.y + e.major.y * cos(t) + m.y * sin(t),
            )
        };
        let (p0, p1) = (at(e.t0), at(e.t1));
        // Old t = 0 → (2, 0), old t = π/2 → (0, −1); the model's arc runs from (0, −1) to (2, 0).
        assert!(
            dist(p0, v(0.0, -1.0)) < 1e-12 && dist(p1, v(2.0, 0.0)) < 1e-12,
            "{p0:?} {p1:?}"
        );
    }

    #[test]
    fn arcs_are_sampled_with_the_apps_step() {
        assert_eq!(
            (
                arc_steps(TAU),
                arc_steps(-PI / 2.0),
                arc_steps(1e-6),
                arc_steps(0.0)
            ),
            (72, 18, 2, 2)
        );
        // A clockwise quarter from (1, 0) to (0, −1): 18 points after the start, the last one exact.
        let mut out = Vec::new();
        arc_points(
            v(0.0, 0.0),
            1.0,
            0.0,
            -PI / 2.0,
            Some(v(0.0, -1.0)),
            &mut out,
        );
        assert_eq!(out.len(), 18);
        assert_eq!(out.last(), Some(&v(0.0, -1.0)));
        assert!(
            out.iter()
                .all(|p| (hypot(p.x, p.y) - 1.0).abs() < 1e-15 && p.x >= -1e-15 && p.y <= 1e-15)
        );
    }

    #[test]
    fn bulged_paths_and_rings_as_the_app_samples_them() {
        // A circle of two bulged halves (bulge 1: half a turn each), 36 segments per half.
        let ring = bulge_path_points(&[v(0.0, 0.0), v(2.0, 0.0)], &[1.0, 1.0], true);
        assert_eq!(ring.len(), 72);
        assert_eq!((ring[0], ring[36]), (v(0.0, 0.0), v(2.0, 0.0)));
        assert!(
            ring.iter()
                .all(|p| (hypot(p.x - 1.0, p.y) - 1.0).abs() < 1e-15)
        );
        let core = to_core(&ring);
        assert!((ring_area(&core) - PI).abs() < 0.01);
        assert!(ring_contains(&core, v(1.0, 0.5)) && !ring_contains(&core, v(3.0, 0.0)));
        // An open path ends at its last vertex; a bulge within 10⁻¹² of zero is straight.
        let open = bulge_path_points(
            &[v(0.0, 0.0), v(2.0, 0.0), v(2.0, 5.0)],
            &[-1.0, 1e-13],
            false,
        );
        assert_eq!((open.len(), open.last()), (38, Some(&v(2.0, 5.0))));
        assert!(has_arcs(&[0.0, -1.0]) && !has_arcs(&[0.0, 1e-13]) && !has_arcs(&[]));
    }
}
