//! Kesen çizgiyle iç ölçü zinciri (docs/adr/0711 §3): the faces a cut crosses, in order along it.
//!
//! A cut from `from` to `to` crosses each wall's chosen lines (its two faces, or every line between its components:
//! docs/adr/0340 §5) on their straight edges, and the columns: a rectangular one's sides, or a column's centre. A
//! crossing inside another source's body is a face no one sees (a corner's, a tee's, a column set in a wall) and is
//! left out; a curved face is counted, not measured. The places are sorted along the cut; of two within a nanometre
//! the first stays. Each consecutive pair is one dimension of the chain.

use crate::entity::Shape;
use crate::geom::bulge::bulge_arc;
use crate::geom::intersect::{line_circle_params, line_line};
use crate::jsmath::{TAU, atan2, js_hypot};
use crate::ops::subref::Place;
use crate::vec2::Vec2;

/// The shortest cut, in the drawing's length: a millimetre of a metre drawing.
pub const SHORTEST: f64 = 1e-3;
/// Two crossings nearer than this along the cut are one; a crossing this far inside another body is hidden.
pub const TIE: f64 = 1e-9;

/// The lines of a wall a cut measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Faces {
    /// Its two faces (its first and last line).
    Outer,
    /// Every line: the faces and the boundaries between its components (Archicad's “All Skins”).
    Skins,
}

/// What of a column a cut measures.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Columns {
    /// A rectangular column's sides; a round column's centre.
    Ends,
    /// Its centre.
    Centers,
    /// Nothing.
    None,
}

/// A point the chain measures: its source (the index in the list given), its place on it, where it is and how far
/// along the cut from `from`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reference {
    pub source: usize,
    pub place: Place,
    pub at: Vec2,
    pub s: f64,
}

/// The references in order along the cut, and how many curved faces it crossed and left out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Crossed {
    pub refs: Vec<Reference>,
    pub curved: usize,
}

/// A source's footprint, for the faces it hides: rings with holes (a wall's body, a rectangular column), or a circle.
enum Body {
    Rings(Vec<Vec<Vec2>>),
    Circle(Vec2, f64),
}

fn body_of(shape: &Shape) -> Option<Body> {
    match shape {
        Shape::Wall { pts, bulges, wall } => {
            let (outer, holes) = crate::geom::wall::body(pts, bulges.as_deref(), wall).ok()?;
            let mut rings = vec![outer];
            rings.extend(holes.unwrap_or_default());
            Some(Body::Rings(rings))
        }
        Shape::Column { at, column } if column.round => Some(Body::Circle(*at, column.width / 2.0)),
        Shape::Column { at, column } => Some(Body::Rings(vec![crate::geom::arch::column_corners(
            *at, column,
        )])),
        _ => None,
    }
}

/// How far `p` is from segment `a`–`b`.
fn off_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    js_hypot(p.x - (a.x + dx * t), p.y - (a.y + dy * t))
}

/// Whether `p` is inside the rings by the even-odd rule.
fn in_rings(p: Vec2, rings: &[Vec<Vec2>]) -> bool {
    let mut inside = false;
    for ring in rings {
        let n = ring.len();
        for i in 0..n {
            let (a, b) = (ring[i], ring[(i + 1) % n]);
            if (a.y > p.y) != (b.y > p.y) {
                let x = a.x + (p.y - a.y) / (b.y - a.y) * (b.x - a.x);
                if p.x < x {
                    inside = !inside;
                }
            }
        }
    }
    inside
}

/// Whether `p` is inside the body by more than [`TIE`].
fn hides(body: &Body, p: Vec2) -> bool {
    match body {
        Body::Circle(c, r) => js_hypot(p.x - c.x, p.y - c.y) < r - TIE,
        Body::Rings(rings) => {
            in_rings(p, rings)
                && rings.iter().all(|ring| {
                    let n = ring.len();
                    (0..n).all(|i| off_segment(p, ring[i], ring[(i + 1) % n]) > TIE)
                })
        }
    }
}

/// Whether the cut crosses the arc of the bulged edge `a`–`b` (bulge `k`) within its ends.
fn crosses_arc(from: Vec2, to: Vec2, a: Vec2, b: Vec2, k: f64) -> bool {
    let Some(arc) = bulge_arc(a, b, k) else {
        return false;
    };
    line_circle_params(from, to, arc.c, arc.r)
        .into_iter()
        .filter(|t| (0.0..=1.0).contains(t))
        .any(|t| {
            let p = Vec2::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
            let th = atan2(p.y - arc.c.y, p.x - arc.c.x);
            let mut d = if arc.sweep > 0.0 {
                th - arc.a0
            } else {
                arc.a0 - th
            };
            while d < 0.0 {
                d += TAU;
            }
            while d >= TAU {
                d -= TAU;
            }
            d <= arc.sweep.abs() + 1e-12
        })
}

/// Where the cut crosses the straight edge `a`–`b`, its ends counted: the point, the fraction along the cut and the
/// fraction along the edge. A parallel edge does not cross.
fn cross(from: Vec2, to: Vec2, a: Vec2, b: Vec2) -> Option<(Vec2, f64, f64)> {
    let h = line_line(from, to, a, b)?;
    ((0.0..=1.0).contains(&h.t) && (0.0..=1.0).contains(&h.u)).then_some((h.p, h.t, h.u))
}

/// The references the cut from `from` to `to` measures on `sources` (walls and columns; any other shape is passed
/// over), in order along it; why none can be (the cut too short).
pub fn references(
    sources: &[&Shape],
    from: Vec2,
    to: Vec2,
    faces: Faces,
    columns: Columns,
) -> Result<Crossed, String> {
    let length = js_hypot(to.x - from.x, to.y - from.y);
    if !(length >= SHORTEST) {
        return Err(
            "Kesen çizgi çok kısa; ölçülecek duvarların içinden geçen bir çizgi çizin.".into(),
        );
    }
    let u = Vec2::new((to.x - from.x) / length, (to.y - from.y) / length);
    let along = |p: Vec2| (p.x - from.x) * u.x + (p.y - from.y) * u.y;
    let bodies: Vec<Option<Body>> = sources.iter().map(|s| body_of(s)).collect();
    let hidden = |source: usize, p: Vec2| {
        bodies
            .iter()
            .enumerate()
            .any(|(j, b)| j != source && b.as_ref().is_some_and(|b| hides(b, p)))
    };
    let mut out = Crossed::default();
    let mut found: Vec<Reference> = Vec::new();
    for (source, shape) in sources.iter().enumerate() {
        match shape {
            Shape::Wall { pts, bulges, wall } => {
                let Ok(lines) = crate::geom::wall::lines(pts, bulges.as_deref(), wall) else {
                    continue;
                };
                let last = lines.len() - 1;
                let parts: Vec<usize> = match faces {
                    Faces::Outer => {
                        if last == 0 {
                            vec![0]
                        } else {
                            vec![0, last]
                        }
                    }
                    Faces::Skins => (0..lines.len()).collect(),
                };
                for part in parts {
                    let l = &lines[part];
                    let n = l.pts.len();
                    let edges = if wall.closed { n } else { n.saturating_sub(1) };
                    for index in 0..edges {
                        let (a, b, k) = (l.pts[index], l.pts[(index + 1) % n], l.bulges[index]);
                        if k != 0.0 {
                            if crosses_arc(from, to, a, b, k) {
                                out.curved += 1;
                            }
                            continue;
                        }
                        let Some((p, t, w)) = cross(from, to, a, b) else {
                            continue;
                        };
                        if hidden(source, p) {
                            continue;
                        }
                        found.push(Reference {
                            source,
                            place: Place::Along {
                                part,
                                ring: 0,
                                index,
                                t: w,
                            },
                            at: p,
                            s: t * length,
                        });
                    }
                }
            }
            Shape::Column { at, column } => {
                if columns == Columns::None {
                    continue;
                }
                let centre = Reference {
                    source,
                    place: Place::Center,
                    at: *at,
                    s: along(*at),
                };
                if column.round {
                    let r = column.width / 2.0;
                    let met = line_circle_params(from, to, *at, r)
                        .into_iter()
                        .any(|t| (0.0..=1.0).contains(&t))
                        || js_hypot(from.x - at.x, from.y - at.y) <= r;
                    if met {
                        found.push(centre);
                    }
                    continue;
                }
                let c = crate::geom::arch::column_corners(*at, column);
                let sides: Vec<(usize, Vec2, f64, f64)> = (0..c.len())
                    .filter_map(|index| {
                        cross(from, to, c[index], c[(index + 1) % c.len()])
                            .map(|(p, t, w)| (index, p, t, w))
                    })
                    .collect();
                match columns {
                    Columns::Ends => {
                        for (index, p, t, w) in sides {
                            if hidden(source, p) {
                                continue;
                            }
                            found.push(Reference {
                                source,
                                place: Place::Along {
                                    part: 0,
                                    ring: 0,
                                    index,
                                    t: w,
                                },
                                at: p,
                                s: t * length,
                            });
                        }
                    }
                    Columns::Centers => {
                        if !sides.is_empty() || in_rings(from, std::slice::from_ref(&c)) {
                            found.push(centre);
                        }
                    }
                    Columns::None => {}
                }
            }
            _ => {}
        }
    }
    let key = |r: &Reference| match r.place {
        Place::Along { part, index, .. } => (part, index),
        _ => (0, 0),
    };
    found.sort_by(|a, b| {
        a.s.total_cmp(&b.s)
            .then(a.source.cmp(&b.source))
            .then(key(a).cmp(&key(b)))
    });
    for r in found {
        if out.refs.last().is_none_or(|last| r.s - last.s > TIE) {
            out.refs.push(r);
        }
    }
    Ok(out)
}

/// The direction of the cut, degrees counter-clockwise from east: a linear dimension's angle.
pub fn angle_of(from: Vec2, to: Vec2) -> f64 {
    atan2(to.y - from.y, to.x - from.x) * 180.0 / crate::jsmath::PI
}
