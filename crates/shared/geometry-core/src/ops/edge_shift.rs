//! Paralel kaydır (docs/adr/0191): an area's or a polyline's straight edge
//! moved parallel to itself, its two vertices sliding along the lines of its
//! neighbours (which lengthen or shorten), by a distance or as far as an
//! area's target size asks. The independent reference is
//! `scripts/fixtures/edge_shift_cases.py` (`fixtures/edge-shift/v1/cases.json`).
//!
//! A vertex the edge moves goes along its neighbour's line: by `d` along the
//! edge's normal it slides `d / (ê·n)` along the neighbour's direction `ê`;
//! an open polyline's free end goes square to the edge. So the area is
//! quadratic in `d`: the edge sweeps a trapezoid, `L·d + ½(λ_b − λ_a)·d²`,
//! `λ` each vertex's slide along the edge per unit of `d` (§3).

use crate::api::Op;
use crate::entity::{Entity, Shape, is_multi_part};
use crate::geom::arrangement::Ring;
use crate::geom::bulge::{bulge_at, bulge_path_edges, bulge_ring_area};
use crate::geom::intersect::closest_on_edge;
use crate::jsmath::js_hypot;
use crate::op;
use crate::vec2::Vec2;

/// Why an edge is not shifted, in the tool's words.
pub const NOT_PATH: &str =
    "Paralel kaydır alanın ya da çoklu çizginin düz kenarında çalışır; böyle bir kenara tıklayın.";
pub const CURVED: &str =
    "Kaydırılan kenar ya da komşusu yay; paralel kaydırma düz kenarlarla yapılır.";
pub const PARALLEL: &str = "Komşu kenar kaydırılan kenara paralel; köşe bulunamıyor.";
pub const PASSED: &str = "Kenar bu uzaklıkta komşusunu aşıyor; daha kısa bir uzaklık yazın.";
pub const NO_EDGE: &str = "Bu nesnede böyle bir kenar yok.";
pub const NO_AREA: &str = "Bu alana kenarı kaydırarak ulaşılamıyor.";
pub const NOT_AREA: &str = "Hedef alan yalnız alanlarda yazılır.";
pub const MULTI: &str = "Çok parçalı nesnenin kenarı kaydırılmaz; önce Parçalara ayır.";

/// A neighbour is parallel to the edge when the sine of the angle between
/// them is at most this.
const PARALLEL_SINE: f64 = 1e-12;

/// The rings of a shape an edge can be shifted on: an area's outer ring and
/// holes (closed), or a polyline's path.
struct Rings {
    rings: Vec<Ring>,
    closed: bool,
}

fn rings_of(shape: &Shape) -> Result<Rings, &'static str> {
    if is_multi_part(shape) {
        return Err(MULTI);
    }
    match shape {
        Shape::Polygon {
            pts, bulges, holes, ..
        } => {
            let mut rings = vec![Ring {
                pts: pts.clone(),
                bulges: bulges.clone(),
            }];
            rings.extend(holes.iter().flatten().cloned());
            Ok(Rings {
                rings,
                closed: true,
            })
        }
        Shape::Polyline { pts, bulges, .. } => Ok(Rings {
            rings: vec![Ring {
                pts: pts.clone(),
                bulges: bulges.clone(),
            }],
            closed: false,
        }),
        _ => Err(NOT_PATH),
    }
}

/// The shape with its rings replaced, everything else kept.
fn with_rings(shape: &Shape, mut rings: Vec<Ring>) -> Shape {
    match shape {
        Shape::Polygon { holes, parts, .. } => {
            let rest = rings.split_off(1.min(rings.len()));
            let outer = rings.pop().unwrap_or(Ring {
                pts: Vec::new(),
                bulges: None,
            });
            Shape::Polygon {
                pts: outer.pts,
                bulges: outer.bulges,
                holes: holes.as_ref().map(|_| rest),
                parts: parts.clone(),
            }
        }
        Shape::Polyline { holes, parts, .. } => {
            let path = rings.into_iter().next().unwrap_or(Ring {
                pts: Vec::new(),
                bulges: None,
            });
            Shape::Polyline {
                pts: path.pts,
                bulges: path.bulges,
                holes: holes.clone(),
                parts: parts.clone(),
            }
        }
        _ => shape.clone(),
    }
}

/// An edge and how it moves: its ends, unit direction and normal (away
/// from an area's inside, to the right of a polyline's way), its length,
/// and how each end moves per unit of the distance.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Motion {
    a: Vec2,
    b: Vec2,
    along: Vec2,
    normal: Vec2,
    length: f64,
    /// The start's move per unit of the distance, and its neighbour's
    /// vertex and length (none: a free end, moving square).
    va: Vec2,
    prev: Option<(Vec2, f64)>,
    vb: Vec2,
    next: Option<(Vec2, f64)>,
}

fn sub(p: Vec2, q: Vec2) -> Vec2 {
    Vec2::new(p.x - q.x, p.y - q.y)
}

fn dot(p: Vec2, q: Vec2) -> f64 {
    p.x * q.x + p.y * q.y
}

/// How the edge `at` of `shape` moves, or why it does not.
fn motion(shape: &Shape, ring: usize, edge: usize) -> Result<(Rings, Motion), &'static str> {
    let rings = rings_of(shape)?;
    let Some(r) = rings.rings.get(ring) else {
        return Err(NO_EDGE);
    };
    let pts = &r.pts;
    let bulges = r.bulges.as_deref();
    let n = pts.len();
    let edges = if rings.closed { n } else { n.saturating_sub(1) };
    if edge >= edges || n < 2 {
        return Err(NO_EDGE);
    }
    let (i, j) = (edge, (edge + 1) % n);
    if bulge_at(bulges, i) != 0.0 {
        return Err(CURVED);
    }
    let (a, b) = (pts[i], pts[j]);
    let t = sub(b, a);
    let length = js_hypot(t.x, t.y);
    if !(length > 0.0) {
        return Err(NO_EDGE);
    }
    let along = Vec2::new(t.x / length, t.y / length);
    let right = Vec2::new(along.y, -along.x);
    let normal = if rings.closed {
        let ccw = bulge_ring_area(pts, bulges) > 0.0;
        // An area's inside is to the left of its outer ring's edges when it
        // runs counter-clockwise, to the right of a hole's.
        let inside_left = if ring == 0 { ccw } else { !ccw };
        if inside_left {
            right
        } else {
            Vec2::new(-right.x, -right.y)
        }
    } else {
        right
    };
    // A vertex sliding along its neighbour's line: per unit of the distance
    // it moves `ê / (ê·n)`.
    let slide = |from: Vec2, to: Vec2| -> Result<(Vec2, f64), &'static str> {
        let d = sub(to, from);
        let l = js_hypot(d.x, d.y);
        if !(l > 0.0) {
            return Err(PARALLEL);
        }
        let e = Vec2::new(d.x / l, d.y / l);
        let s = dot(e, normal);
        if s.abs() <= PARALLEL_SINE {
            return Err(PARALLEL);
        }
        Ok((Vec2::new(e.x / s, e.y / s), l))
    };
    let (va, prev) = if rings.closed || i > 0 {
        let h = (i + n - 1) % n;
        if bulge_at(bulges, h) != 0.0 {
            return Err(CURVED);
        }
        let (v, l) = slide(pts[h], a)?;
        (v, Some((pts[h], l)))
    } else {
        (normal, None)
    };
    let (vb, next) = if rings.closed || j < n - 1 {
        let k = (j + 1) % n;
        if bulge_at(bulges, j) != 0.0 {
            return Err(CURVED);
        }
        let (v, l) = slide(pts[k], b)?;
        (v, Some((pts[k], l)))
    } else {
        (normal, None)
    };
    Ok((
        rings,
        Motion {
            a,
            b,
            along,
            normal,
            length,
            va,
            prev,
            vb,
            next,
        },
    ))
}

/// An area's size from its rings: the outer ring's less its holes'
/// (`measure::polygon_area`'s rule, on the shapes' rings).
fn rings_area(rings: &[Ring]) -> f64 {
    let size = |r: &Ring| bulge_ring_area(&r.pts, r.bulges.as_deref()).abs();
    rings.first().map_or(0.0, size) - rings.iter().skip(1).map(size).sum::<f64>()
}

/// The edge's two vertices moved by `d`, or `PASSED` when a neighbour or
/// the edge itself would shrink to nothing or turn round.
fn moved(m: &Motion, d: f64) -> Result<(Vec2, Vec2), &'static str> {
    let a = Vec2::new(m.a.x + m.va.x * d, m.a.y + m.va.y * d);
    let b = Vec2::new(m.b.x + m.vb.x * d, m.b.y + m.vb.y * d);
    if let Some((prev, _)) = m.prev
        && dot(sub(a, prev), sub(m.a, prev)) <= 0.0
    {
        return Err(PASSED);
    }
    if let Some((next, _)) = m.next
        && dot(sub(next, b), sub(next, m.b)) <= 0.0
    {
        return Err(PASSED);
    }
    if dot(sub(b, a), m.along) <= 0.0 {
        return Err(PASSED);
    }
    Ok((a, b))
}

/// The edge `edge` of ring `ring` (0: a polyline's path or an area's outer
/// ring; 1…: its holes) moved `d` along its normal: away from an area's
/// inside (so an area grows with a positive `d`), to the right of a
/// polyline's way (docs/adr/0191 §2).
pub fn shifted(shape: &Shape, ring: usize, edge: usize, d: f64) -> Result<Shape, String> {
    if !d.is_finite() {
        return Err(NO_EDGE.to_owned());
    }
    let (Rings { mut rings, .. }, m) = motion(shape, ring, edge).map_err(str::to_owned)?;
    let (a, b) = moved(&m, d).map_err(str::to_owned)?;
    let r = &mut rings[ring];
    let n = r.pts.len();
    r.pts[edge] = a;
    r.pts[(edge + 1) % n] = b;
    Ok(with_rings(shape, rings))
}

/// The distance the edge moves for the area to reach `target` m²
/// (docs/adr/0191 §3): the root of `A₀ + L·d + ½(λ_b − λ_a)·d² = target`
/// nearer 0; none when no shift reaches it or the shift is refused there.
pub fn for_area(shape: &Shape, ring: usize, edge: usize, target: f64) -> Result<f64, String> {
    let Shape::Polygon { .. } = shape else {
        return Err(NOT_AREA.to_owned());
    };
    let (rings, m) = motion(shape, ring, edge).map_err(str::to_owned)?;
    if !target.is_finite() {
        return Err(NO_AREA.to_owned());
    }
    let a0 = rings_area(&rings.rings);
    // Each end's slide along the edge per unit of the distance.
    let (la, lb) = (dot(m.va, m.along), dot(m.vb, m.along));
    let (q, p, c) = ((lb - la) / 2.0, m.length, a0 - target);
    let disc = p * p - 4.0 * q * c;
    if !(disc >= 0.0) {
        return Err(NO_AREA.to_owned());
    }
    // The root nearer 0, in the form that keeps its digits when q is small.
    let d = -2.0 * c / (p + disc.sqrt());
    moved(&m, d).map_err(str::to_owned)?;
    Ok(d)
}

/// An edge picked: where it is and how it moves (the tool's distance is
/// the cursor's along `normal` from `a`).
#[derive(Clone, Debug, PartialEq)]
pub struct Picked {
    pub ring: usize,
    pub edge: usize,
    pub a: Vec2,
    pub b: Vec2,
    pub normal: Vec2,
}

crate::json_struct!(out Picked {
    ring,
    edge,
    a,
    b,
    normal
});

/// The edge of `shape` nearest `p`, or why there is none to shift: a
/// multi-part object, another kind, an arc edge or a neighbour that stops it.
pub fn pick(shape: &Shape, p: Vec2) -> Result<Picked, String> {
    let rings = rings_of(shape).map_err(str::to_owned)?;
    let mut best: Option<(f64, usize, usize)> = None;
    for (ri, r) in rings.rings.iter().enumerate() {
        for (ei, e) in bulge_path_edges(&r.pts, r.bulges.as_deref(), rings.closed)
            .iter()
            .enumerate()
        {
            let d = closest_on_edge(e, p).d;
            if best.is_none_or(|(bd, ..)| d < bd) {
                best = Some((d, ri, ei));
            }
        }
    }
    let Some((_, ring, edge)) = best else {
        return Err(NO_EDGE.to_owned());
    };
    let (_, m) = motion(shape, ring, edge).map_err(str::to_owned)?;
    Ok(Picked {
        ring,
        edge,
        a: m.a,
        b: m.b,
        normal: m.normal,
    })
}

/// A shift as the op answers it: the object with its new geometry (its
/// other fields kept) and an area's size, or why not.
#[derive(Clone, Debug, PartialEq)]
pub struct Shift {
    pub entity: Option<Entity>,
    pub area: Option<f64>,
    pub problem: Option<String>,
}

crate::json_struct!(out Shift {
    entity,
    area,
    problem
});

/// The edge picked as the op answers it, or why it cannot be shifted.
#[derive(Clone, Debug, PartialEq)]
pub struct PickAnswer {
    pub picked: Option<Picked>,
    pub problem: Option<String>,
}

crate::json_struct!(out PickAnswer { picked, problem });

/// A target's distance as the op answers it.
#[derive(Clone, Debug, PartialEq)]
pub struct Reach {
    pub distance: Option<f64>,
    pub problem: Option<String>,
}

crate::json_struct!(out Reach { distance, problem });

/// An area's size (its holes taken off); none for a polyline.
pub fn area_of(shape: &Shape) -> Option<f64> {
    match shape {
        Shape::Polygon { .. } => {
            let rings = rings_of(shape).ok()?;
            Some(rings_area(&rings.rings))
        }
        _ => None,
    }
}

/// A ring's or an edge's index as the web gives it; one past any for a
/// negative, fractional or huge number, which no shape has.
fn index(v: f64) -> usize {
    if v >= 0.0 && v.fract() == 0.0 && v < 1e9 {
        v as usize
    } else {
        usize::MAX
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("edgeShift", |e: Entity, ring: f64, edge: f64, d: f64| {
        match shifted(&e.shape, index(ring), index(edge), d) {
            Ok(shape) => Shift {
                area: area_of(&shape),
                entity: Some(Entity {
                    shape,
                    rest: e.rest.clone(),
                }),
                problem: None,
            },
            Err(why) => Shift {
                entity: None,
                area: None,
                problem: Some(why),
            },
        }
    }),
    op!(
        "edgeShiftForArea",
        |e: Entity, ring: f64, edge: f64, target: f64| {
            match for_area(&e.shape, index(ring), index(edge), target) {
                Ok(d) => Reach {
                    distance: Some(d),
                    problem: None,
                },
                Err(why) => Reach {
                    distance: None,
                    problem: Some(why),
                },
            }
        }
    ),
    op!(
        "edgeShiftPick",
        |e: Entity, p: Vec2| match pick(&e.shape, p) {
            Ok(picked) => PickAnswer {
                picked: Some(picked),
                problem: None,
            },
            Err(why) => PickAnswer {
                picked: None,
                problem: Some(why),
            },
        }
    ),
];
