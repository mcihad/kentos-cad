//! Lengths and areas in another system's plane (docs/adr/0167 §2): a path
//! or an area's rings, given in the project's system, measured where the
//! second system draws them. Each straight segment's ends, and each arc
//! segment as straight pieces whose sagitta is at most 0.1 mm (docs/adr/0149's
//! bound), are taken into the second system; the lengths are the straight
//! pieces' sums there, the areas their shoelace areas about each ring's first
//! point, the holes taken from the outer ring. A geographic system has no
//! plane; neither has the Pseudo-Mercator, whose scale is another at every
//! latitude. The independent reference is
//! `scripts/fixtures/crs_measure_cases.py` (PROJ).

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::geom::bulge::{bulge_arc, bulge_at};
use crate::jsmath::{acos, cos, js_hypot, js_max, sin};
use crate::op;
use crate::vec2::Vec2;

use super::{System, transform};

/// The largest sagitta of an arc's straight pieces (m; docs/adr/0149's bound).
pub const CHORD: f64 = 1e-4;

/// A path or a ring: its vertices and its segments' bulges (DXF's; none, or
/// a missing one, straight), as an area's rings are.
pub use crate::geom::arrangement::Ring;

/// What a path or an area measures in a plane.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaneMeasures {
    /// The path's length, or the rings' perimeter, outer and holes (m).
    pub length: f64,
    /// The area's: its outer ring's less its holes' (m²); 0 for a path.
    pub area: f64,
}

/// Why nothing was measured in a system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoPlane {
    /// Latitudes and longitudes: no plane (measures on the ellipsoid are `HYB-14`'s).
    Geographic,
    /// The Pseudo-Mercator: its scale is another at every latitude.
    Mercator,
    /// A point the system's projection does not reach.
    Unreachable,
}

impl NoPlane {
    /// As the reference writes it.
    pub fn as_str(self) -> &'static str {
        match self {
            NoPlane::Geographic => "geographic",
            NoPlane::Mercator => "mercator",
            NoPlane::Unreachable => "unreachable",
        }
    }
}

/// The arc segment from `a` to `b` as straight pieces' ends, `a` first and
/// `b` last: `n = max(1, ⌈|θ| / (2·acos(1 − CHORD/r))⌉)` equal pieces, one
/// when the radius is not above `CHORD`; a straight segment is its ends.
pub fn arc_pieces(a: Vec2, b: Vec2, bulge: f64) -> Vec<Vec2> {
    let Some(arc) = bulge_arc(a, b, bulge) else {
        return vec![a, b];
    };
    let n = if arc.r <= CHORD {
        1
    } else {
        let step = 2.0 * acos(1.0 - CHORD / arc.r);
        js_max(libm::ceil(arc.sweep.abs() / step), 1.0) as usize
    };
    let mut out = Vec::with_capacity(n + 1);
    out.push(a);
    for i in 1..n {
        let t = arc.a0 + arc.sweep * i as f64 / n as f64;
        out.push(Vec2::new(
            arc.c.x + arc.r * cos(t),
            arc.c.y + arc.r * sin(t),
        ));
    }
    out.push(b);
    out
}

/// Every point of a path or a ring, arcs in pieces; a ring's first point is
/// not repeated at its end.
fn ring_points(ring: &Ring, closed: bool) -> Vec<Vec2> {
    let pts = &ring.pts;
    let Some(&first) = pts.first() else {
        return Vec::new();
    };
    let mut out = vec![first];
    let count = if closed {
        pts.len()
    } else {
        pts.len().saturating_sub(1)
    };
    for i in 0..count {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        out.extend(
            arc_pieces(a, b, bulge_at(ring.bulges.as_deref(), i))
                .into_iter()
                .skip(1),
        );
    }
    if closed {
        out.pop();
    }
    out
}

/// The straight pieces' length; a ring's closing piece too.
fn length(points: &[Vec2], closed: bool) -> f64 {
    let mut total = 0.0;
    for w in points.windows(2) {
        total += js_hypot(w[1].x - w[0].x, w[1].y - w[0].y);
    }
    if closed && let (Some(first), Some(last)) = (points.first(), points.last()) {
        total += js_hypot(first.x - last.x, first.y - last.y);
    }
    total
}

/// The shoelace area about the ring's first point: the grid's millions do
/// not take the area's digits.
fn area(points: &[Vec2]) -> f64 {
    let Some(&o) = points.first() else {
        return 0.0;
    };
    let mut twice = 0.0;
    for (i, p) in points.iter().enumerate() {
        let q = points[(i + 1) % points.len()];
        twice += (p.x - o.x) * (q.y - o.y) - (q.x - o.x) * (p.y - o.y);
    }
    twice.abs() / 2.0
}

/// A path (`closed` false: the first ring) or an area's rings (the first the
/// outer, the others its holes), given in `from`, measured in `to`'s plane.
pub fn plane_measures(
    from: &System,
    to: &System,
    rings: &[Ring],
    closed: bool,
) -> Result<PlaneMeasures, NoPlane> {
    match to {
        System::Geographic { .. } => return Err(NoPlane::Geographic),
        System::Mercator {} => return Err(NoPlane::Mercator),
        System::Tm { .. } => {}
    }
    let mut measures = PlaneMeasures {
        length: 0.0,
        area: 0.0,
    };
    let rings = if closed {
        rings
    } else {
        &rings[..rings.len().min(1)]
    };
    for (i, ring) in rings.iter().enumerate() {
        let moved = ring_points(ring, closed)
            .into_iter()
            .map(|p| transform(from, to, p).map(|t| t.point))
            .collect::<Option<Vec<Vec2>>>()
            .ok_or(NoPlane::Unreachable)?;
        measures.length += length(&moved, closed);
        if closed {
            let a = area(&moved);
            measures.area += if i == 0 { a } else { -a };
        }
    }
    Ok(measures)
}

/// The op's answer: the measures, or why there are none.
struct Answer(Result<PlaneMeasures, NoPlane>);

impl ToJson for Answer {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        match self.0 {
            Ok(m) => {
                field(out, &mut first, "length", &m.length);
                field(out, &mut first, "area", &m.area);
            }
            Err(why) => field(out, &mut first, "why", why.as_str()),
        }
        out.push('}');
    }
}

pub(crate) static OPS: &[Op] = &[op!(
    "crsPlaneMeasures",
    |from: System, to: System, rings: Vec<Ring>, closed: bool| {
        Answer(plane_measures(&from, &to, &rings, closed))
    }
)];
