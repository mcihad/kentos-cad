//! Vektör oturtma's warping of objects (docs/adr/0156 §4–§5), one for both
//! platforms (the web through WASM): a shape under a similarity, an affine
//! or a projective transform, given in the command's centred form, with its
//! paths' elevations. A similarity moves every kind as the modify tools do;
//! otherwise straight geometry maps exactly, curves become the exact ellipse
//! of their image (affine) or straight vertices within 0.1 mm (projective, and
//! a path's arc segments under any non-similar map), and texts, notes,
//! blocks, dimensions and hatch patterns keep their shape at their anchor's
//! derivative. The independent reference is `scripts/fixtures/warp_cases.py`.

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::entity::{HatchAssoc, Shape};
use crate::geom::affine::{Affine, translation};
use crate::geom::arc::{norm_angle, sweep};
use crate::geom::arrangement::Ring;
use crate::geom::bulge::bulge_arc;
use crate::geom::hatch_pattern::carried;
use crate::jsmath::{PI, TAU, atan2, cos, js_hypot, js_max, sin};
use crate::op;
use crate::ops::rubber::{Link, RubberError, Sheet};
use crate::ops::transform::transform_shape;
use crate::vec2::Vec2;

/// The largest chord error of straight vertices along a curve's image (m;
/// docs/adr/0149's bound).
pub const CHORD: f64 = 1e-4;
/// A projective transform's denominator at or below this: beyond its horizon.
pub const HORIZON: f64 = 1e-9;
/// The widest first step along a curve (radians of its parameter).
const STEP: f64 = PI / 8.0;
/// How many times a step is halved at most.
const DEPTH: u32 = 30;

/// A transform in the command's centred form (docs/adr/0156 §6): `from` the
/// source centre, `to` the target's, the numbers between the centred frames.
#[derive(Clone, Debug, PartialEq)]
pub enum Warp {
    /// x′ = to.x + a·x̄ − b·ȳ, y′ = to.y + b·x̄ + a·ȳ.
    Similarity {
        from: Vec2,
        to: Vec2,
        a: f64,
        b: f64,
    },
    /// x′ = to.x + a·x̄ + c·ȳ, y′ = to.y + b·x̄ + d·ȳ (`m` = [a, b, c, d]).
    Affine { from: Vec2, to: Vec2, m: [f64; 4] },
    /// x′ = to.x + (a1·x̄ + a2·ȳ + a3)/w, y′ = to.y + (b1·x̄ + b2·ȳ + b3)/w,
    /// w = c1·x̄ + c2·ȳ + 1 (`h` = [a1, a2, a3, b1, b2, b3, c1, c2]).
    Projective { from: Vec2, to: Vec2, h: [f64; 8] },
}

crate::json_tagged!(Warp, "kind",
    Similarity => "similarity" { from, to, a, b },
    Affine => "affine" { from, to, m },
    Projective => "projective" { from, to, h },
);

/// A point beyond a projective transform's horizon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Beyond;

/// J as [a, b, c, d]: its columns (a, b) and (c, d), the core's `Affine` order.
type Jacobian = [f64; 4];

impl Warp {
    fn ends(&self) -> (Vec2, Vec2) {
        match self {
            Warp::Similarity { from, to, .. }
            | Warp::Affine { from, to, .. }
            | Warp::Projective { from, to, .. } => (*from, *to),
        }
    }

    /// The linear part when the warp is linear (a similarity or an affine).
    fn linear(&self) -> Option<Jacobian> {
        match *self {
            Warp::Similarity { a, b, .. } => Some([a, b, -b, a]),
            Warp::Affine { m, .. } => Some(m),
            Warp::Projective { .. } => None,
        }
    }

    /// Whether every kind moves exactly, as the modify tools move it: the
    /// similarity kind, or an affine that turns, scales and perhaps mirrors.
    pub fn is_similarity(&self) -> bool {
        match *self {
            Warp::Similarity { .. } => true,
            Warp::Affine {
                m: [a, b, c, d], ..
            } => (a == d && b == -c) || (a == -d && b == c),
            Warp::Projective { .. } => false,
        }
    }

    /// The centred map at a centred point.
    fn centred(&self, x: f64, y: f64) -> Result<(f64, f64), Beyond> {
        match *self {
            Warp::Projective {
                h: [a1, a2, a3, b1, b2, b3, c1, c2],
                ..
            } => {
                let w = c1 * x + c2 * y + 1.0;
                if w > HORIZON {
                    Ok(((a1 * x + a2 * y + a3) / w, (b1 * x + b2 * y + b3) / w))
                } else {
                    Err(Beyond)
                }
            }
            _ => {
                let [a, b, c, d] = self.linear().unwrap_or([1.0, 0.0, 0.0, 1.0]);
                Ok((a * x + c * y, b * x + d * y))
            }
        }
    }

    /// The centred image of a point (`f(p) − to`).
    fn g(&self, p: Vec2) -> Result<(f64, f64), Beyond> {
        let (from, _) = self.ends();
        self.centred(p.x - from.x, p.y - from.y)
    }

    /// Where a point goes.
    pub fn point(&self, p: Vec2) -> Result<Vec2, Beyond> {
        let (gx, gy) = self.g(p)?;
        let (_, to) = self.ends();
        Ok(Vec2::new(to.x + gx, to.y + gy))
    }

    /// The derivative at a point.
    fn jacobian(&self, p: Vec2) -> Result<Jacobian, Beyond> {
        match *self {
            Warp::Projective {
                from,
                h: [a1, a2, a3, b1, b2, b3, c1, c2],
                ..
            } => {
                let (x, y) = (p.x - from.x, p.y - from.y);
                let w = c1 * x + c2 * y + 1.0;
                if w <= HORIZON {
                    return Err(Beyond);
                }
                let (n1, n2, w2) = (a1 * x + a2 * y + a3, b1 * x + b2 * y + b3, w * w);
                Ok([
                    (a1 * w - n1 * c1) / w2,
                    (b1 * w - n2 * c1) / w2,
                    (a2 * w - n1 * c2) / w2,
                    (b2 * w - n2 * c2) / w2,
                ])
            }
            _ => Ok(self.linear().unwrap_or([1.0, 0.0, 0.0, 1.0])),
        }
    }
}

/// What gives a point's image and the derivative there: a transform, or a
/// rubber sheet (docs/adr/0158).
trait Map {
    fn at(&self, p: Vec2) -> Result<Vec2, Beyond>;
    fn jac(&self, p: Vec2) -> Result<Jacobian, Beyond>;
}

impl Map for Warp {
    fn at(&self, p: Vec2) -> Result<Vec2, Beyond> {
        self.point(p)
    }
    fn jac(&self, p: Vec2) -> Result<Jacobian, Beyond> {
        self.jacobian(p)
    }
}

impl Map for Sheet {
    fn at(&self, p: Vec2) -> Result<Vec2, Beyond> {
        Ok(self.map(p))
    }
    fn jac(&self, p: Vec2) -> Result<Jacobian, Beyond> {
        Ok(self.jacobian(p))
    }
}

fn det(j: &Jacobian) -> f64 {
    j[0] * j[3] - j[1] * j[2]
}

fn lin(j: &Jacobian, v: Vec2) -> Vec2 {
    Vec2::new(j[0] * v.x + j[2] * v.y, j[1] * v.x + j[3] * v.y)
}

/// A shape warped: its geometry, its paths' elevations (outer, holes, then
/// each part's ring and holes; a line's two ends), whether a curve became
/// straight vertices, whether a text, note, block, dimension or hatch
/// pattern kept its shape.
#[derive(Clone, Debug, PartialEq)]
pub struct Warped {
    pub shape: Shape,
    pub zs: Vec<Vec<Option<f64>>>,
    pub curves: bool,
    pub kept: bool,
}

/// The parameters of straight vertices along a curve `point(t)` from `t0` to
/// `t1`, both ends included (docs/adr/0156 §4): equal steps of at most π/8,
/// each halved while its middle's image lies more than [`CHORD`] from the
/// chord of its ends' images.
fn densify(warp: &Warp, point: &dyn Fn(f64) -> Vec2, t0: f64, t1: f64) -> Result<Vec<f64>, Beyond> {
    let n = js_max(((t1 - t0).abs() / STEP).ceil(), 1.0) as usize;
    let mut out = vec![t0];
    for k in 0..n {
        let ta = t0 + (t1 - t0) * k as f64 / n as f64;
        let tb = t0 + (t1 - t0) * (k + 1) as f64 / n as f64;
        split(warp, point, ta, tb, 0, &mut out)?;
    }
    Ok(out)
}

fn split(
    warp: &Warp,
    point: &dyn Fn(f64) -> Vec2,
    ta: f64,
    tb: f64,
    depth: u32,
    out: &mut Vec<f64>,
) -> Result<(), Beyond> {
    let tm = (ta + tb) / 2.0;
    if depth < DEPTH && far(warp.g(point(ta))?, warp.g(point(tm))?, warp.g(point(tb))?) {
        split(warp, point, ta, tm, depth + 1, out)?;
        split(warp, point, tm, tb, depth + 1, out)
    } else {
        out.push(tb);
        Ok(())
    }
}

/// Whether `m` lies more than [`CHORD`] from the chord `a`–`b` (centred images).
fn far(a: (f64, f64), m: (f64, f64), b: (f64, f64)) -> bool {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let length = js_hypot(dx, dy);
    if length == 0.0 {
        return js_hypot(m.0 - a.0, m.1 - a.1) > CHORD;
    }
    (dx * (m.1 - a.1) - dy * (m.0 - a.0)).abs() / length > CHORD
}

/// A ring warped: its vertices, their elevations, whether arcs became straight vertices.
type WarpedRing = (Vec<Vec2>, Vec<Option<f64>>, bool);

/// A path's vertices and elevations under a non-similar warp: every vertex
/// by f, each arc segment's straight vertices between, which take its ends'
/// elevations linearly by angle (docs/adr/0142, rule 3).
fn warp_ring(
    warp: &Warp,
    pts: &[Vec2],
    bulges: &Option<Vec<f64>>,
    zs: Option<&Vec<Option<f64>>>,
    closed: bool,
) -> Result<WarpedRing, Beyond> {
    let n = pts.len();
    let z_at = |i: usize| zs.and_then(|z| z.get(i).copied().flatten());
    let edges = if closed { n } else { n.saturating_sub(1) };
    let (mut out, mut out_z, mut dense) = (Vec::new(), Vec::new(), false);
    for i in 0..n {
        out.push(warp.point(pts[i])?);
        out_z.push(z_at(i));
        if i >= edges {
            continue;
        }
        let j = (i + 1) % n;
        let bulge = bulges
            .as_ref()
            .and_then(|b| b.get(i))
            .copied()
            .unwrap_or(0.0);
        let Some(arc) = bulge_arc(pts[i], pts[j], bulge) else {
            continue;
        };
        dense = true;
        let at = |t: f64| {
            let a = arc.a0 + arc.sweep * t;
            Vec2::new(arc.c.x + arc.r * cos(a), arc.c.y + arc.r * sin(a))
        };
        let ts = densify(warp, &at, 0.0, 1.0)?;
        let (za, zb) = (z_at(i), z_at(j));
        for &t in &ts[1..ts.len() - 1] {
            out.push(warp.point(at(t))?);
            out_z.push(match (za, zb) {
                (Some(za), Some(zb)) => Some(za + (zb - za) * t),
                _ => None,
            });
        }
    }
    Ok((out, out_z, dense))
}

/// The ellipse of a curve's image (docs/adr/0156 §4): the curve is
/// c + M·cos t + N·sin t, A = J·[M N]; its major, ratio and axes.
struct Image {
    major: Vec2,
    ratio: f64,
    u1: Vec2,
    u2: Vec2,
    s1: f64,
    s2: f64,
}

fn image_of(j: &Jacobian, m: Vec2, n: Vec2) -> Image {
    let c1 = lin(j, m);
    let c2 = lin(j, n);
    let p = c1.x * c1.x + c2.x * c2.x;
    let s = c1.y * c1.y + c2.y * c2.y;
    let q = c1.x * c1.y + c2.x * c2.y;
    let phi = 0.5 * atan2(2.0 * q, p - s);
    let root = js_hypot((p - s) / 2.0, q);
    let s1 = ((p + s) / 2.0 + root).sqrt();
    let s2 = js_max((p + s) / 2.0 - root, 0.0).sqrt();
    let u1 = Vec2::new(cos(phi), sin(phi));
    Image {
        major: Vec2::new(s1 * u1.x, s1 * u1.y),
        ratio: s2 / s1,
        u1,
        u2: Vec2::new(-u1.y, u1.x),
        s1,
        s2,
    }
}

impl Image {
    /// The parameter of a point of the image, from its centre.
    fn param(&self, q: (f64, f64)) -> f64 {
        norm_angle(atan2(
            (q.0 * self.u2.x + q.1 * self.u2.y) / self.s2,
            (q.0 * self.u1.x + q.1 * self.u1.y) / self.s1,
        ))
    }
}

/// A text's height, turn (degrees) and width factor at J (docs/adr/0156 §5).
fn text_rule(j: &Jacobian, height: f64, rotation: f64, width: Option<f64>) -> (f64, f64, f64) {
    let rad = rotation * PI / 180.0;
    let lu = lin(j, Vec2::new(cos(rad), sin(rad)));
    let length = js_hypot(lu.x, lu.y);
    let d = det(j);
    let mut turn = atan2(lu.y, lu.x) * 180.0 / PI;
    if d < 0.0 {
        turn += 180.0;
    }
    turn = ((turn % 360.0) + 360.0) % 360.0;
    (
        height * d.abs() / length,
        turn,
        width.unwrap_or(1.0) * length * length / d.abs(),
    )
}

/// A shape under `warp` (docs/adr/0156 §4), with its paths' elevations `zs`
/// (as [`Warped::zs`]), or [`Beyond`] when a point of it is beyond a
/// projective transform's horizon.
pub fn warp_shape(shape: &Shape, zs: &[Vec<Option<f64>>], warp: &Warp) -> Result<Warped, Beyond> {
    if warp.is_similarity() {
        return Ok(similar(shape, zs, warp));
    }
    // A ray that runs to a projective transform's horizon.
    if let Shape::Ray { dir, .. } = shape
        && let Warp::Projective { h, .. } = warp
        && h[6] * dir.x + h[7] * dir.y < 0.0
    {
        return Err(Beyond);
    }
    if let Some(done) = common(shape, zs, warp)? {
        return Ok(done);
    }
    Ok(match shape {
        Shape::Polyline {
            pts,
            bulges,
            holes,
            parts,
        } => {
            let (pts2, z, dense) = warp_ring(warp, pts, bulges, zs.first(), false)?;
            let mut out_zs = vec![z];
            let mut curves = dense;
            // Every part of a multi-part polyline, its elevations after the first's (docs/adr/0174).
            let parts = match parts {
                Some(ps) => {
                    let mut out = Vec::with_capacity(ps.len());
                    for (k, part) in ps.iter().enumerate() {
                        let (p, z, d) =
                            warp_ring(warp, &part.pts, &part.bulges, zs.get(k + 1), false)?;
                        out_zs.push(z);
                        curves |= d;
                        out.push(crate::entity::Part {
                            pts: p,
                            bulges: if d { None } else { part.bulges.clone() },
                            holes: None,
                        });
                    }
                    Some(out)
                }
                None => None,
            };
            Warped {
                shape: Shape::Polyline {
                    pts: pts2,
                    bulges: if dense { None } else { bulges.clone() },
                    holes: holes.clone(),
                    parts,
                },
                zs: out_zs,
                curves,
                kept: false,
            }
        }
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            let mut k = 0;
            let mut out_zs = Vec::new();
            let mut dense = false;
            let mut ring = |pts: &[Vec2], bulges: &Option<Vec<f64>>| -> Result<Ring, Beyond> {
                let (p, z, d) = warp_ring(warp, pts, bulges, zs.get(k), true)?;
                k += 1;
                out_zs.push(z);
                dense |= d;
                Ok(Ring {
                    bulges: if d { None } else { bulges.clone() },
                    pts: p,
                })
            };
            let outer = ring(pts, bulges)?;
            let holes = match holes {
                Some(hs) => Some(
                    hs.iter()
                        .map(|h| ring(&h.pts, &h.bulges))
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                None => None,
            };
            let parts = match parts {
                Some(ps) => {
                    let mut out = Vec::with_capacity(ps.len());
                    for part in ps {
                        let own = ring(&part.pts, &part.bulges)?;
                        let holes = match &part.holes {
                            Some(hs) => Some(
                                hs.iter()
                                    .map(|h| ring(&h.pts, &h.bulges))
                                    .collect::<Result<Vec<_>, _>>()?,
                            ),
                            None => None,
                        };
                        out.push(crate::entity::Part {
                            pts: own.pts,
                            bulges: own.bulges,
                            holes,
                        });
                    }
                    Some(out)
                }
                None => None,
            };
            Warped {
                shape: Shape::Polygon {
                    pts: outer.pts,
                    bulges: outer.bulges,
                    holes,
                    parts,
                },
                zs: out_zs,
                curves: dense,
                kept: false,
            }
        }
        Shape::Circle { c, r } => curve(warp, *c, Vec2::new(*r, 0.0), Vec2::new(0.0, *r), None)?,
        Shape::Arc { c, r, a0, a1 } => curve(
            warp,
            *c,
            Vec2::new(*r, 0.0),
            Vec2::new(0.0, *r),
            Some((*a0, *a0 + sweep(*a0, *a1))),
        )?,
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let n = Vec2::new(-major.y * ratio, major.x * ratio);
            let sw = sweep(*t0, *t1);
            let span = (sw < TAU - 1e-12).then_some((*t0, *t0 + sw));
            curve(warp, *c, *major, n, span)?
        }
        // Every other kind is `common`'s.
        _ => Warped {
            shape: shape.clone(),
            zs: zs.to_vec(),
            curves: false,
            kept: false,
        },
    })
}

/// The kinds a transform and a rubber sheet move alike (docs/adr/0156
/// §4–§5, docs/adr/0158 §3): points, lines, a spline's fit points, an xline
/// or a ray by their point and J·dir; texts, notes, blocks, dimensions and
/// hatch patterns kept in shape at their anchor's derivative. None for a
/// path or a curve, which each map moves its own way.
fn common<M: Map + ?Sized>(
    shape: &Shape,
    zs: &[Vec<Option<f64>>],
    m: &M,
) -> Result<Option<Warped>, Beyond> {
    let kept = |shape: Shape| Warped {
        shape,
        zs: zs.to_vec(),
        curves: false,
        kept: true,
    };
    let plain = |shape: Shape| Warped {
        shape,
        zs: zs.to_vec(),
        curves: false,
        kept: false,
    };
    Ok(Some(match shape {
        Shape::Point { p, z, parts } => plain(Shape::Point {
            p: m.at(*p)?,
            z: *z,
            parts: match parts {
                Some(ps) => Some(
                    ps.iter()
                        .map(|q| {
                            Ok(crate::entity::PointPart {
                                p: m.at(q.p)?,
                                z: q.z,
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                ),
                None => None,
            },
        }),
        Shape::Line { a, b } => plain(Shape::Line {
            a: m.at(*a)?,
            b: m.at(*b)?,
        }),
        Shape::Spline { pts, closed } => plain(Shape::Spline {
            pts: pts
                .iter()
                .map(|&p| m.at(p))
                .collect::<Result<Vec<_>, _>>()?,
            closed: *closed,
        }),
        Shape::Xline { p, dir } | Shape::Ray { p, dir } => {
            let d = lin(&m.jac(*p)?, *dir);
            let l = js_hypot(d.x, d.y);
            let (p, dir) = (m.at(*p)?, Vec2::new(d.x / l, d.y / l));
            plain(if matches!(shape, Shape::Ray { .. }) {
                Shape::Ray { p, dir }
            } else {
                Shape::Xline { p, dir }
            })
        }
        Shape::Text {
            p,
            text,
            height: h0,
            rotation,
            align,
            width_factor,
            mask,
            box_width,
            line_spacing,
            runs,
            face,
        } => {
            let (height, rotation, width) = text_rule(&m.jac(*p)?, *h0, *rotation, *width_factor);
            // A multi-line text's box takes its letters' change of width (docs/adr/0182).
            let along = height * width / (h0 * width_factor.unwrap_or(1.0));
            kept(Shape::Text {
                p: m.at(*p)?,
                text: text.clone(),
                height,
                rotation,
                align: *align,
                width_factor: Some(width),
                mask: *mask,
                box_width: box_width.map(|w| w * along),
                line_spacing: *line_spacing,
                runs: runs.clone(),
                face: face.clone(),
            })
        }
        Shape::Leader {
            pts,
            text,
            height,
            rotation,
            arrow,
            mask,
        } => {
            let last = *pts.last().unwrap_or(&Vec2::new(0.0, 0.0));
            let (height, rotation, _) = text_rule(&m.jac(last)?, *height, *rotation, None);
            kept(Shape::Leader {
                pts: pts
                    .iter()
                    .map(|&p| m.at(p))
                    .collect::<Result<Vec<_>, _>>()?,
                text: text.clone(),
                height,
                rotation,
                arrow: arrow.clone(),
                mask: *mask,
            })
        }
        Shape::Insert { p, .. } => {
            // The modify tools' similarity at the point: scale √|det J|, turn
            // the angle of J's first column, mirrored when J mirrors.
            let at = m.at(*p)?;
            let mut out = transform_shape(shape, &nearest_similarity(&m.jac(*p)?));
            if let Shape::Insert { p, .. } = &mut out {
                *p = at;
            }
            kept(out)
        }
        // A picture keeps its shape under the similarity at its lower left corner, which
        // goes where the point goes; mirrored, its frame's new corner moves with it
        // (docs/adr/0192 §4).
        Shape::Image { p, .. } => {
            let at = m.at(*p)?;
            let s = nearest_similarity(&m.jac(*p)?);
            let from = crate::geom::affine::apply(&s, *p);
            let mut out = transform_shape(shape, &s);
            if let Shape::Image { p: corner, .. } = &mut out {
                *corner = Vec2::new(corner.x + at.x - from.x, corner.y + at.y - from.y);
            }
            kept(out)
        }
        Shape::Dimension {
            a,
            b,
            c,
            angle,
            style,
            ..
        } => {
            let mid = Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            let j = m.jac(mid)?;
            let (na, nb) = (m.at(*a)?, m.at(*b)?);
            let nc = match c {
                Some(c) => Some(m.at(*c)?),
                None => None,
            };
            let linear_angle = (style.as_deref() == Some("linear")).then(|| {
                let rad = angle.unwrap_or(0.0) * PI / 180.0;
                let d = lin(&j, Vec2::new(cos(rad), sin(rad)));
                atan2(d.y, d.x) * 180.0 / PI
            });
            let mut out = transform_shape(shape, &nearest_similarity(&j));
            if let Shape::Dimension {
                a,
                b,
                c,
                angle,
                style,
                ..
            } = &mut out
            {
                // The similarity's mirror swaps an angle's arms; the points are f's.
                let swapped =
                    matches!(style.as_deref(), Some("angular" | "arcLength")) && det(&j) < 0.0;
                (*a, *b) = if swapped { (nb, na) } else { (na, nb) };
                *c = nc;
                if let Some(l) = linear_angle {
                    *angle = Some(l);
                }
            }
            kept(out)
        }
        Shape::Hatch {
            ring,
            holes,
            pattern,
            assoc,
        } => {
            let n = ring.len().max(1) as f64;
            let mean = Vec2::new(
                ring.iter().map(|p| p.x).sum::<f64>() / n,
                ring.iter().map(|p| p.y).sum::<f64>() / n,
            );
            let j = m.jac(mean)?;
            let pattern = match pattern.kind.as_str() {
                // A pattern or a gradient by the similarity nearest J (docs/adr/0186 §8).
                "pattern" | "gradient" => carried(
                    pattern,
                    atan2(j[1], j[0]) * 180.0 / PI,
                    det(&j).abs().sqrt(),
                    det(&j) < 0.0,
                ),
                _ => {
                    let rad = pattern.angle * PI / 180.0;
                    let d = lin(&j, Vec2::new(cos(rad), sin(rad)));
                    let mut pattern = pattern.clone();
                    pattern.angle = (((atan2(d.y, d.x) * 180.0 / PI) % 180.0) + 180.0) % 180.0;
                    pattern.spacing *= det(&j).abs().sqrt();
                    pattern
                }
            };
            let map = |ps: &Vec<Vec2>| ps.iter().map(|&p| m.at(p)).collect::<Result<Vec<_>, _>>();
            kept(Shape::Hatch {
                ring: map(ring)?,
                holes: match holes {
                    Some(hs) => Some(hs.iter().map(map).collect::<Result<Vec<_>, _>>()?),
                    None => None,
                },
                pattern,
                assoc: match assoc {
                    Some(a) => Some(HatchAssoc {
                        seed: m.at(a.seed)?,
                        ..a.clone()
                    }),
                    None => None,
                },
            })
        }
        _ => return Ok(None),
    }))
}

/// The similarity of the modify tools nearest J for a block or a dimension:
/// scale √|det J|, turned as J's first column, mirrored when J mirrors.
fn nearest_similarity(j: &Jacobian) -> Affine {
    let s = det(j).abs().sqrt();
    let theta = atan2(j[1], j[0]);
    let (c, sn) = (s * cos(theta), s * sin(theta));
    if det(j) < 0.0 {
        [c, sn, sn, -c, 0.0, 0.0]
    } else {
        [c, sn, -sn, c, 0.0, 0.0]
    }
}

/// A circle, an arc or an ellipse (c + M·cos t + N·sin t; `span` the arc's
/// parameters, none for a full curve): under an affine warp the exact
/// ellipse of its image, under a projective one straight vertices.
fn curve(
    warp: &Warp,
    c: Vec2,
    m: Vec2,
    n: Vec2,
    span: Option<(f64, f64)>,
) -> Result<Warped, Beyond> {
    let point = |t: f64| {
        Vec2::new(
            c.x + m.x * cos(t) + n.x * sin(t),
            c.y + m.y * cos(t) + n.y * sin(t),
        )
    };
    if let Warp::Projective { .. } = warp {
        let (t0, t1) = span.unwrap_or((0.0, TAU));
        let ts = densify(warp, &point, t0, t1)?;
        let mut pts = ts
            .iter()
            .map(|&t| warp.point(point(t)))
            .collect::<Result<Vec<_>, _>>()?;
        if span.is_none()
            && let Some(&first) = pts.first()
            && let Some(last) = pts.last_mut()
        {
            *last = first;
        }
        return Ok(Warped {
            shape: Shape::Polyline {
                pts,
                bulges: None,
                holes: None,
                parts: None,
            },
            zs: Vec::new(),
            curves: true,
            kept: false,
        });
    }
    let j = warp.jacobian(c)?;
    let image = image_of(&j, m, n);
    let (t0, t1) = match span {
        None => (0.0, TAU),
        Some((a0, a1)) => {
            let gc = warp.g(c)?;
            let (s, e) = (warp.g(point(a0))?, warp.g(point(a1))?);
            let ts = image.param((s.0 - gc.0, s.1 - gc.1));
            let te = image.param((e.0 - gc.0, e.1 - gc.1));
            if det(&j) > 0.0 { (ts, te) } else { (te, ts) }
        }
    };
    Ok(Warped {
        shape: Shape::Ellipse {
            c: warp.point(c)?,
            major: image.major,
            ratio: image.ratio,
            t0,
            t1,
        },
        zs: Vec::new(),
        curves: false,
        kept: false,
    })
}

/// A similarity as the modify tools move a shape, in the centred frames
/// (translated to the source centre, the linear part, translated to the
/// target's: no national coordinate is multiplied); the elevations stay.
fn similar(shape: &Shape, zs: &[Vec<Option<f64>>], warp: &Warp) -> Warped {
    let (from, to) = warp.ends();
    let [a, b, c, d] = warp.linear().unwrap_or([1.0, 0.0, 0.0, 1.0]);
    let moved = transform_shape(shape, &translation(-from.x, -from.y));
    let turned = transform_shape(&moved, &[a, b, c, d, 0.0, 0.0]);
    Warped {
        shape: transform_shape(&turned, &translation(to.x, to.y)),
        zs: zs.to_vec(),
        curves: false,
        kept: false,
    }
}

/// Shapes warped together: theirs, their elevations and the counts, or the
/// first refused one's index (docs/adr/0156 §4).
pub struct WarpedAll(pub Result<(Vec<Warped>, usize, usize), usize>);

impl ToJson for WarpedAll {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok((all, curves, kept)) => {
                let shapes: Vec<Shape> = all.iter().map(|w| w.shape.clone()).collect();
                let zs: Vec<Vec<Vec<Option<f64>>>> = all.iter().map(|w| w.zs.clone()).collect();
                field(out, &mut first, "shapes", &shapes);
                field(out, &mut first, "zs", &zs);
                field(out, &mut first, "curves", &(*curves as f64));
                field(out, &mut first, "kept", &(*kept as f64));
            }
            Err(at) => {
                field(out, &mut first, "error", "beyond_horizon");
                field(out, &mut first, "at", &(*at as f64));
            }
        }
        out.push('}');
    }
}

/// Every shape under `warp`, with its elevations; how many curves became
/// straight vertices and how many shapes kept theirs; or the first refused.
pub fn warp_shapes(shapes: &[Shape], zs: &[Vec<Vec<Option<f64>>>], warp: &Warp) -> WarpedAll {
    let none = Vec::new();
    let mut out = Vec::with_capacity(shapes.len());
    let (mut curves, mut kept) = (0, 0);
    for (i, s) in shapes.iter().enumerate() {
        match warp_shape(s, zs.get(i).unwrap_or(&none), warp) {
            Ok(w) => {
                curves += usize::from(w.curves);
                kept += usize::from(w.kept);
                out.push(w);
            }
            Err(Beyond) => return WarpedAll(Err(i)),
        }
    }
    WarpedAll(Ok((out, curves, kept)))
}

// ── Kauçuk levha (docs/adr/0158 §3) ─────────────────────────────────────

/// A shape on a rubber sheet and how far it bends from its true image
/// (metres; [`bend_of`]). Only vertices move by the sheet: straight edges
/// stay straight and gain no vertex, an arc segment keeps its bulge, a
/// circle, an arc and an ellipse move by the nearest similarity at their
/// centre (their kind stays); the rest moves as a transform moves it, at its
/// anchor's derivative. Elevations stay with their vertices.
pub fn sheet_shape(shape: &Shape, zs: &[Vec<Option<f64>>], sheet: &Sheet) -> (Warped, f64) {
    let plain = |shape: Shape| Warped {
        shape,
        zs: zs.to_vec(),
        curves: false,
        kept: false,
    };
    let map = |ps: &[Vec2]| ps.iter().map(|&p| sheet.map(p)).collect::<Vec<_>>();
    let ring = |r: &Ring| Ring {
        pts: map(&r.pts),
        bulges: r.bulges.clone(),
    };
    let warped = match shape {
        Shape::Polyline {
            pts,
            bulges,
            holes,
            parts,
        } => plain(Shape::Polyline {
            pts: map(pts),
            bulges: bulges.clone(),
            // Only a polygon's holes move (as `transform_shape`).
            holes: holes.clone(),
            parts: parts.as_ref().map(|ps| {
                ps.iter()
                    .map(|part| crate::entity::Part {
                        pts: map(&part.pts),
                        bulges: part.bulges.clone(),
                        holes: None,
                    })
                    .collect()
            }),
        }),
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => plain(Shape::Polygon {
            pts: map(pts),
            bulges: bulges.clone(),
            holes: holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
            parts: parts.as_ref().map(|ps| {
                ps.iter()
                    .map(|part| crate::entity::Part {
                        pts: map(&part.pts),
                        bulges: part.bulges.clone(),
                        holes: part.holes.as_ref().map(|hs| hs.iter().map(ring).collect()),
                    })
                    .collect()
            }),
        }),
        Shape::Circle { c, .. } | Shape::Arc { c, .. } | Shape::Ellipse { c, .. } => {
            plain(near_similar(shape, *c, sheet))
        }
        _ => match common(shape, zs, sheet) {
            Ok(Some(w)) => w,
            // A sheet has no horizon, and `common` takes every other kind.
            _ => plain(shape.clone()),
        },
    };
    (warped, bend_of(shape, sheet))
}

/// A curve moved by the sheet's nearest similarity at its centre `c`, in
/// frames centred on `c` and on its image (no national coordinate is
/// multiplied).
fn near_similar(shape: &Shape, c: Vec2, sheet: &Sheet) -> Shape {
    let to = sheet.map(c);
    let moved = transform_shape(shape, &translation(-c.x, -c.y));
    let turned = transform_shape(&moved, &nearest_similarity(&sheet.jacobian(c)));
    transform_shape(&turned, &translation(to.x, to.y))
}

/// How far a shape on the sheet lies from its true image (docs/adr/0158
/// §3): the largest distance, metres, between the sheet's image of a point
/// and the kept shape's point at the same parameter. Straight edges and arc
/// segments (of lines, paths, rings, hatch rings and leaders) at a quarter,
/// a half and three quarters; a circle, an arc and an ellipse at the middles
/// of eight equal parts of their span. Points, splines, xlines, rays,
/// texts, blocks and dimensions: 0.
fn bend_of(shape: &Shape, sheet: &Sheet) -> f64 {
    let ring = |pts: &[Vec2], bulges: Option<&Vec<f64>>, closed: bool| {
        let n = pts.len();
        let edges = if closed { n } else { n.saturating_sub(1) };
        let mut worst: f64 = 0.0;
        for i in 0..edges {
            let j = (i + 1) % n;
            let bulge = bulges.and_then(|b| b.get(i)).copied().unwrap_or(0.0);
            worst = js_max(worst, edge_bend(pts[i], pts[j], bulge, sheet));
        }
        worst
    };
    match shape {
        Shape::Line { a, b } => edge_bend(*a, *b, 0.0, sheet),
        Shape::Polyline { pts, bulges, .. } => ring(pts, bulges.as_ref(), false),
        Shape::Leader { pts, .. } => ring(pts, None, false),
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => {
            let mut worst = ring(pts, bulges.as_ref(), true);
            for h in holes.iter().flatten() {
                worst = js_max(worst, ring(&h.pts, h.bulges.as_ref(), true));
            }
            for part in parts.iter().flatten() {
                worst = js_max(worst, ring(&part.pts, part.bulges.as_ref(), true));
                for h in part.holes.iter().flatten() {
                    worst = js_max(worst, ring(&h.pts, h.bulges.as_ref(), true));
                }
            }
            worst
        }
        Shape::Hatch { ring: r, holes, .. } => {
            let mut worst = ring(r, None, true);
            for h in holes.iter().flatten() {
                worst = js_max(worst, ring(h, None, true));
            }
            worst
        }
        Shape::Circle { c, r } => curve_bend(
            *c,
            Vec2::new(*r, 0.0),
            Vec2::new(0.0, *r),
            (0.0, TAU),
            sheet,
        ),
        Shape::Arc { c, r, a0, a1 } => curve_bend(
            *c,
            Vec2::new(*r, 0.0),
            Vec2::new(0.0, *r),
            (*a0, *a0 + sweep(*a0, *a1)),
            sheet,
        ),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => curve_bend(
            *c,
            *major,
            Vec2::new(-major.y * ratio, major.x * ratio),
            (*t0, *t0 + sweep(*t0, *t1)),
            sheet,
        ),
        _ => 0.0,
    }
}

/// The parameters a bend is measured at along an edge.
const QUARTERS: [f64; 3] = [0.25, 0.5, 0.75];

/// An edge's bend: the sheet's image of its point at each quarter against
/// the kept edge's (a straight edge between the images of its ends, or the
/// arc of the same bulge between them).
fn edge_bend(a: Vec2, b: Vec2, bulge: f64, sheet: &Sheet) -> f64 {
    let (na, nb) = (sheet.map(a), sheet.map(b));
    let mut worst: f64 = 0.0;
    match (bulge_arc(a, b, bulge), bulge_arc(na, nb, bulge)) {
        (Some(arc), Some(kept)) => {
            for t in QUARTERS {
                let (u, v) = (arc.a0 + arc.sweep * t, kept.a0 + kept.sweep * t);
                let real = sheet.map(Vec2::new(
                    arc.c.x + arc.r * cos(u),
                    arc.c.y + arc.r * sin(u),
                ));
                let here = Vec2::new(kept.c.x + kept.r * cos(v), kept.c.y + kept.r * sin(v));
                worst = js_max(worst, js_hypot(real.x - here.x, real.y - here.y));
            }
        }
        _ => {
            for t in QUARTERS {
                let real = sheet.map(Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
                let here = Vec2::new(na.x + (nb.x - na.x) * t, na.y + (nb.y - na.y) * t);
                worst = js_max(worst, js_hypot(real.x - here.x, real.y - here.y));
            }
        }
    }
    worst
}

/// A curve's bend (c + M·cos t + N·sin t over `span`): the sheet's image of
/// its point at the middles of eight equal parts against the nearest
/// similarity's image of it.
fn curve_bend(c: Vec2, m: Vec2, n: Vec2, span: (f64, f64), sheet: &Sheet) -> f64 {
    let [sa, sb, sc, sd, ..] = nearest_similarity(&sheet.jacobian(c));
    let to = sheet.map(c);
    let mut worst: f64 = 0.0;
    for k in 0..8 {
        let t = span.0 + (span.1 - span.0) * (f64::from(k) + 0.5) / 8.0;
        let v = Vec2::new(m.x * cos(t) + n.x * sin(t), m.y * cos(t) + n.y * sin(t));
        let real = sheet.map(Vec2::new(c.x + v.x, c.y + v.y));
        let here = Vec2::new(to.x + sa * v.x + sc * v.y, to.y + sb * v.x + sd * v.y);
        worst = js_max(worst, js_hypot(real.x - here.x, real.y - here.y));
    }
    worst
}

/// Shapes on a sheet together: theirs, their elevations, how many kept their
/// shape, how many bent more than [`CHORD`] and the largest bend; or why
/// there is no sheet (docs/adr/0158 §3–§4).
pub struct SheetAll(pub Result<(Vec<Warped>, usize, usize, f64), RubberError>);

impl ToJson for SheetAll {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match &self.0 {
            Ok((all, kept, bent, bend)) => {
                let shapes: Vec<Shape> = all.iter().map(|w| w.shape.clone()).collect();
                let zs: Vec<Vec<Vec<Option<f64>>>> = all.iter().map(|w| w.zs.clone()).collect();
                field(out, &mut first, "shapes", &shapes);
                field(out, &mut first, "zs", &zs);
                field(out, &mut first, "kept", &(*kept as f64));
                field(out, &mut first, "bent", &(*bent as f64));
                field(out, &mut first, "bend", bend);
            }
            Err(e) => field(out, &mut first, "error", &e.code()),
        }
        out.push('}');
    }
}

/// Every shape on the sheet through `links`, with its elevations.
pub fn sheet_shapes(shapes: &[Shape], zs: &[Vec<Vec<Option<f64>>>], links: &[Link]) -> SheetAll {
    let sheet = match Sheet::solve(links) {
        Ok(s) => s,
        Err(e) => return SheetAll(Err(e)),
    };
    let none = Vec::new();
    let mut out = Vec::with_capacity(shapes.len());
    let (mut kept, mut bent, mut bend) = (0, 0, 0.0_f64);
    for (i, s) in shapes.iter().enumerate() {
        let (w, b) = sheet_shape(s, zs.get(i).unwrap_or(&none), &sheet);
        kept += usize::from(w.kept);
        bent += usize::from(b > CHORD);
        bend = js_max(bend, b);
        out.push(w);
    }
    SheetAll(Ok((out, kept, bent, bend)))
}

pub(crate) static OPS: &[Op] = &[
    op!("warpShapes", |shapes: Vec<Shape>,
                       zs: Vec<Vec<Vec<Option<f64>>>>,
                       warp: Warp| {
        warp_shapes(&shapes, &zs, &warp)
    }),
    op!("rubberShapes", |shapes: Vec<Shape>,
                         zs: Vec<Vec<Vec<Option<f64>>>>,
                         links: Vec<Link>| {
        sheet_shapes(&shapes, &zs, &links)
    }),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::{FromJson, Json, to_string};

    /// The reference's cases (scripts/fixtures/warp_cases.py): every shape,
    /// its elevations and the counts, or the same refusal. Coordinates within
    /// the file's metres, every other number within its relative tolerance
    /// (of its size, at least 1), the rest exactly.
    #[test]
    fn every_case_is_warped_as_the_reference_warps_it() {
        let file = Json::parse(include_str!("../../../../../fixtures/fit/v1/warp.json"))
            .expect("warp.json reads");
        let tolerance = file.get("tolerance");
        let metres = f64::from_json(tolerance.get("metres")).expect("metres");
        let relative = f64::from_json(tolerance.get("relative")).expect("relative");
        let Json::Arr(cases) = file.get("cases") else {
            panic!("cases")
        };
        assert!(cases.len() >= 6, "{} cases", cases.len());
        let mut off = Vec::new();
        for case in cases {
            let name = String::from_json(case.get("name")).unwrap_or_default();
            let key = String::from_json(case.get("warp")).expect("a warp");
            let warp = Warp::from_json(file.get("warps").get(&key)).expect("the warp reads");
            let shapes = Vec::<Shape>::from_json(case.get("objects")).expect("objects read");
            let zs = Vec::<Vec<Vec<Option<f64>>>>::from_json(case.get("zs")).expect("zs read");
            let got = Json::parse(&to_string(&warp_shapes(&shapes, &zs, &warp))).expect("reads");
            if let Err(e) = close(&got, case.get("expected"), metres, relative, "") {
                off.push(format!("{name}: {e}"));
            }
        }
        assert!(
            off.is_empty(),
            "{} durum farklı:\n{}",
            off.len(),
            off.join("\n")
        );
    }

    /// The reference's sheets (scripts/fixtures/rubber_warp_cases.py; the
    /// sheet's map from the mpmath reference): every shape, its elevations,
    /// the counts and the largest bend, or the same refusal.
    #[test]
    fn every_case_on_a_sheet_is_as_the_reference_puts_it() {
        let file = Json::parse(include_str!(
            "../../../../../fixtures/fit/v1/rubber-warp.json"
        ))
        .expect("rubber-warp.json reads");
        let tolerance = file.get("tolerance");
        let metres = f64::from_json(tolerance.get("metres")).expect("metres");
        let relative = f64::from_json(tolerance.get("relative")).expect("relative");
        let Json::Arr(cases) = file.get("cases") else {
            panic!("cases")
        };
        assert!(cases.len() >= 3, "{} cases", cases.len());
        let mut off = Vec::new();
        for case in cases {
            let name = String::from_json(case.get("name")).unwrap_or_default();
            let key = String::from_json(case.get("sheet")).expect("a sheet");
            let links = Vec::<Link>::from_json(file.get("sheets").get(&key)).expect("links");
            let shapes = Vec::<Shape>::from_json(case.get("objects")).expect("objects read");
            let zs = Vec::<Vec<Vec<Option<f64>>>>::from_json(case.get("zs")).expect("zs read");
            let got = Json::parse(&to_string(&sheet_shapes(&shapes, &zs, &links))).expect("reads");
            if let Err(e) = close(&got, case.get("expected"), metres, relative, "") {
                off.push(format!("{name}: {e}"));
            }
        }
        assert!(
            off.is_empty(),
            "{} durum farklı:\n{}",
            off.len(),
            off.join("\n")
        );
    }

    fn close(a: &Json, e: &Json, metres: f64, relative: f64, path: &str) -> Result<(), String> {
        match (a, e) {
            (Json::Num(x), Json::Num(y)) => {
                // Coordinates and a sheet's bend are metres.
                let limit = if path.ends_with(".x") || path.ends_with(".y") || path == ".bend" {
                    metres
                } else {
                    relative * js_max(y.abs(), 1.0)
                };
                if (x - y).abs() <= limit {
                    Ok(())
                } else {
                    Err(format!("{path}: {x} ≠ {y} (fark {:e})", (x - y).abs()))
                }
            }
            (Json::Arr(xs), Json::Arr(ys)) => {
                if xs.len() != ys.len() {
                    return Err(format!("{path}: {} öğe ≠ {}", xs.len(), ys.len()));
                }
                for (i, (x, y)) in xs.iter().zip(ys).enumerate() {
                    close(x, y, metres, relative, &format!("{path}[{i}]"))?;
                }
                Ok(())
            }
            (Json::Obj(xs), Json::Obj(ys)) => {
                for (k, y) in ys {
                    close(a.get(k), y, metres, relative, &format!("{path}.{k}"))?;
                }
                match xs.iter().find(|(k, _)| !ys.iter().any(|(k2, _)| k2 == k)) {
                    Some((k, _)) => Err(format!("{path}.{k}: fazladan alan")),
                    None => Ok(()),
                }
            }
            _ if a == e => Ok(()),
            _ => Err(format!("{path}: {a:?} ≠ {e:?}")),
        }
    }

    #[test]
    fn a_similarity_moves_by_the_centred_frames() {
        let warp = Warp::Similarity {
            from: Vec2::new(100.0, 200.0),
            to: Vec2::new(487_000.0, 4_420_000.0),
            a: 0.0,
            b: 1.0,
        };
        let w = warp_shape(
            &Shape::Circle {
                c: Vec2::new(110.0, 200.0),
                r: 2.0,
            },
            &[],
            &warp,
        )
        .expect("no horizon");
        assert_eq!(
            w.shape,
            Shape::Circle {
                c: Vec2::new(487_000.0, 4_420_010.0),
                r: 2.0
            }
        );
    }
}
