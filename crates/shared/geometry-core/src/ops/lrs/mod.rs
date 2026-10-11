//! Doğrusal referanslama (docs/adr/0224): routes and the places along them.
//! A route is parts walked in order, each a path of vertices, bulges (an
//! arc edge) and a measure (M) a vertex, none where it has none. On an edge
//! with both ends measured M runs linearly along it: by length on a straight
//! edge, by angle on an arc. The measure direction is the route's: from its
//! first measured vertex to its last in walked order; when the last is
//! smaller, against the drawing. An offset or a distance is positive to the
//! right of the measure direction (Km yaz's way; a tool may turn it).
//!
//! The independent reference is `scripts/fixtures/lrs_cases.py`
//! (`fixtures/lrs/v1/cases.json`).

use crate::geom::bulge::bulge_arc;
use crate::geom::intersect::{Edge, closest_on_edge, point_at as edge_point};
use crate::geom::offset::{OffsetResult, offset_bulge_path};
use crate::jsmath::{cos, js_hypot, js_max, js_min, sin, tan};
use crate::vec2::Vec2;

/// A unit enum read and written as one of its words.
macro_rules! words {
    ($t:ident { $($v:ident => $w:literal),* $(,)? }) => {
        impl crate::api::json::FromJson for $t {
            fn from_json(v: &crate::api::json::Json) -> Result<$t, String> {
                match <String as crate::api::json::FromJson>::from_json(v)?.as_str() {
                    $($w => Ok($t::$v),)*
                    other => Err(format!("“{other}” bilinmiyor")),
                }
            }
        }
        impl crate::api::json::ToJson for $t {
            fn write_json(&self, out: &mut String) {
                match self {
                    $($t::$v => crate::api::json::ToJson::write_json($w, out),)*
                }
            }
        }
    };
}

/// Two places this near are one (m): a chain's joint, a vertex a
/// calibration point falls on, a piece's touching end.
pub const ON: f64 = 1e-6;

/// A part of a route: its vertices, its arcs' bulges and each vertex's
/// measure, none where it has none.
#[derive(Clone, Debug, PartialEq)]
pub struct RoutePart {
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
    pub ms: Vec<Option<f64>>,
}

crate::json_struct!(RoutePart { pts, bulges, ms });

/// A route: its parts in walked order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Route {
    pub parts: Vec<RoutePart>,
}

crate::json_struct!(Route { parts });

/// Why a route gives no place.
#[derive(Clone, Debug, PartialEq)]
pub enum Fault {
    /// No vertex of the route has a measure.
    NoMeasures,
    /// The measure is outside the route's, `least` to `most`.
    Outside { least: f64, most: f64 },
    /// A line event from and to one measure.
    Same,
    /// Fewer than two calibration points on the route.
    FewPoints,
    /// The offset path could not be made.
    Offset,
    /// An event names a route there is not.
    NoRoute,
    /// An event without a measure, or a line event without its from or to.
    NoMeasure,
}

crate::json_tagged!(Fault, "fault",
    NoMeasures => "noMeasures" {},
    Outside => "outside" { least, most },
    Same => "same" {},
    FewPoints => "fewPoints" {},
    Offset => "offset" {},
    NoRoute => "noRoute" {},
    NoMeasure => "noMeasure" {},
);

/// An edge of a route with its ends and their measures.
#[derive(Clone, Copy, Debug)]
struct Step {
    edge: Edge,
    a: Vec2,
    b: Vec2,
    ma: Option<f64>,
    mb: Option<f64>,
    len: f64,
}

impl Step {
    fn at(&self, t: f64) -> Vec2 {
        // The ends are the vertices themselves.
        if t == 0.0 {
            self.a
        } else if t == 1.0 {
            self.b
        } else {
            edge_point(&self.edge, t)
        }
    }

    /// The unit direction of travel at `t`.
    fn tangent(&self, t: f64) -> Vec2 {
        match self.edge {
            Edge::Seg { a, b } => {
                let l = js_hypot(b.x - a.x, b.y - a.y);
                Vec2::new((b.x - a.x) / l, (b.y - a.y) / l)
            }
            Edge::Arc { a0, sweep, .. } => {
                let ang = a0 + sweep * t;
                let s = if sweep > 0.0 { 1.0 } else { -1.0 };
                Vec2::new(-sin(ang) * s, cos(ang) * s)
            }
        }
    }

    fn m(&self, t: f64) -> Option<f64> {
        let (ma, mb) = (self.ma?, self.mb?);
        Some(if t == 0.0 {
            ma
        } else if t == 1.0 {
            mb
        } else {
            ma + (mb - ma) * t
        })
    }

    /// The bulge of the edge's piece from `t0` to `t1`.
    fn piece_bulge(&self, t0: f64, t1: f64) -> f64 {
        match self.edge {
            Edge::Arc { sweep, .. } => tan(sweep * (t1 - t0) / 4.0),
            Edge::Seg { .. } => 0.0,
        }
    }
}

/// A part's edges, in order.
fn steps(part: &RoutePart) -> Vec<Step> {
    let n = part.pts.len();
    (0..n.saturating_sub(1))
        .map(|i| {
            let (a, b) = (part.pts[i], part.pts[i + 1]);
            let bulge = part
                .bulges
                .as_ref()
                .and_then(|v| v.get(i))
                .copied()
                .unwrap_or(0.0);
            let (edge, len) = match bulge_arc(a, b, bulge) {
                Some(arc) => (
                    Edge::Arc {
                        c: arc.c,
                        r: arc.r,
                        a0: arc.a0,
                        sweep: arc.sweep,
                    },
                    arc.sweep.abs() * arc.r,
                ),
                None => (Edge::Seg { a, b }, js_hypot(b.x - a.x, b.y - a.y)),
            };
            let m = |k: usize| part.ms.get(k).copied().flatten();
            Step {
                edge,
                a,
                b,
                ma: m(i),
                mb: m(i + 1),
                len,
            }
        })
        .collect()
}

/// +1 when the route's measures grow along the drawing, -1 when they fall.
pub fn direction(route: &Route) -> f64 {
    let mut measured = route.parts.iter().flat_map(|p| p.ms.iter().flatten());
    let Some(&first) = measured.next() else {
        return 1.0;
    };
    match measured.last() {
        Some(&last) if last < first => -1.0,
        _ => 1.0,
    }
}

/// The least and most measure of the route.
pub fn measure_range(route: &Route) -> Option<(f64, f64)> {
    route
        .parts
        .iter()
        .flat_map(|p| p.ms.iter().flatten().copied())
        .fold(None, |r, m| match r {
            None => Some((m, m)),
            Some((lo, hi)) => Some((js_min(lo, m), js_max(hi, m))),
        })
}

fn right_of(t: Vec2) -> Vec2 {
    Vec2::new(t.y, -t.x)
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    js_hypot(a.x - b.x, a.y - b.y)
}

/// Where a point is along a route: its closest place, the measure there
/// (none on an edge with an end without one), the signed distance (+ right
/// of the measure direction), the length walked to it (the gaps between
/// parts not counted), its part, edge and the fraction along the edge.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub m: Option<f64>,
    pub distance: f64,
    pub foot: Vec2,
    pub along: f64,
    pub part: usize,
    pub edge: usize,
    pub t: f64,
}

crate::json_struct!(Place {
    m,
    distance,
    foot,
    along,
    part,
    edge,
    t
});

/// A route made ready to walk: its edges in walked order with their part,
/// index in the part and the length walked before them, its measure
/// direction and range. Made once, asked many times.
pub struct Walk {
    steps: Vec<Step>,
    /// Each step's part and index in it, and the length walked before it.
    at: Vec<(usize, usize, f64)>,
    dir: f64,
    range: Option<(f64, f64)>,
}

impl Walk {
    pub fn new(route: &Route) -> Walk {
        let mut steps = Vec::new();
        let mut at = Vec::new();
        let mut along = 0.0;
        for (k, part) in route.parts.iter().enumerate() {
            for (i, s) in self::steps(part).into_iter().enumerate() {
                at.push((k, i, along));
                along += s.len;
                steps.push(s);
            }
        }
        Walk {
            steps,
            at,
            dir: direction(route),
            range: measure_range(route),
        }
    }

    fn outside(&self) -> Fault {
        match self.range {
            Some((least, most)) => Fault::Outside { least, most },
            None => Fault::NoMeasures,
        }
    }

    /// The place on step `k` closest to `p`, with its distance unsigned.
    fn place_on(&self, k: usize, p: Vec2) -> (f64, Place) {
        let s = &self.steps[k];
        let t = closest_on_edge(&s.edge, p).t;
        let foot = s.at(t);
        let d = js_hypot(p.x - foot.x, p.y - foot.y);
        let (part, edge, along) = self.at[k];
        let rn = right_of(s.tangent(t));
        let side = ((p.x - foot.x) * rn.x + (p.y - foot.y) * rn.y) * self.dir;
        (
            d,
            Place {
                m: s.m(t),
                distance: if side >= 0.0 { d } else { -d },
                foot,
                along: along + s.len * t,
                part,
                edge,
                t,
            },
        )
    }

    /// The distance from `p` to edge `k` (walked order), as [`Walk::locate`] measures it.
    fn distance_to(&self, k: usize, p: Vec2) -> f64 {
        let s = &self.steps[k];
        dist(p, s.at(closest_on_edge(&s.edge, p).t))
    }

    /// A line along the route: when every vertex is within `radius` of it,
    /// the measures at the first's and the last's places and the farthest
    /// vertex's distance; none when a place has no measure.
    pub fn locate_line(&self, pts: &[Vec2], radius: f64) -> Option<Along> {
        let (first, last) = (pts.first()?, pts.last()?);
        let mut far: f64 = 0.0;
        for p in pts {
            let d = self.locate(*p)?.distance.abs();
            if d > radius {
                return None;
            }
            far = js_max(far, d);
        }
        Some(Along {
            from: self.locate(*first)?.m?,
            to: self.locate(*last)?.m?,
            distance: far,
        })
    }

    /// The closest place of the route to `p` (the first in walked order of
    /// equally close ones); none for a route without an edge.
    pub fn locate(&self, p: Vec2) -> Option<Place> {
        let mut best: Option<(f64, Place)> = None;
        for k in 0..self.steps.len() {
            let (d, place) = self.place_on(k, p);
            if best.as_ref().is_none_or(|(bd, _)| d < *bd) {
                best = Some((d, place));
            }
        }
        best.map(|(_, place)| place)
    }

    /// The first place in walked order whose measure is `m` (on an edge of
    /// equal ends, its start), moved square to the route by `offset` (+
    /// right of the measure direction).
    pub fn point_at(&self, m: f64, offset: f64) -> Result<Vec2, Fault> {
        for s in &self.steps {
            let (Some(ma), Some(mb)) = (s.ma, s.mb) else {
                continue;
            };
            if js_min(ma, mb) <= m && m <= js_max(ma, mb) {
                let t = if ma == mb { 0.0 } else { (m - ma) / (mb - ma) };
                let foot = s.at(t);
                if offset == 0.0 {
                    return Ok(foot);
                }
                let rn = right_of(s.tangent(t));
                let d = offset * self.dir;
                return Ok(Vec2::new(foot.x + rn.x * d, foot.y + rn.y * d));
            }
        }
        Err(self.outside())
    }

    /// The first place in walked order whose measure is `m` (on an edge of
    /// equal ends, its start), the unit direction the measures run there
    /// (the route's measure direction) and the length walked to it; none
    /// outside the route's measures (Km yaz by measure, docs/adr/0224 §6).
    pub fn frame_at(&self, m: f64) -> Option<(Vec2, Vec2, f64)> {
        for (k, s) in self.steps.iter().enumerate() {
            let (Some(ma), Some(mb)) = (s.ma, s.mb) else {
                continue;
            };
            if js_min(ma, mb) <= m && m <= js_max(ma, mb) {
                let t = if ma == mb { 0.0 } else { (m - ma) / (mb - ma) };
                let tan = s.tangent(t);
                let along = self.at[k].2 + s.len * t;
                return Some((
                    s.at(t),
                    Vec2::new(tan.x * self.dir, tan.y * self.dir),
                    along,
                ));
            }
        }
        None
    }

    /// The route's least and most measure.
    pub fn range(&self) -> Option<(f64, f64)> {
        self.range
    }

    /// The route's measures, first and last in walked order.
    pub fn ends(&self) -> Option<(f64, f64)> {
        let mut ms = self.steps.iter().flat_map(|s| [s.ma, s.mb]).flatten();
        let first = ms.next()?;
        Some((first, ms.last().unwrap_or(first)))
    }

    /// The route's places with a measure between `from` and `to`: each
    /// edge's piece, touching pieces one path, with their measures; offset
    /// (+ right of the measure direction) as the core offsets a path; the
    /// pieces run from `from` to `to`: in walked order when that is the
    /// measure direction's, else reversed.
    pub fn stretch(&self, from: f64, to: f64, offset: f64) -> Result<Vec<RoutePart>, Fault> {
        if from == to {
            return Err(Fault::Same);
        }
        let (lo, hi) = (js_min(from, to), js_max(from, to));
        let mut parts: Vec<RoutePart> = Vec::new();
        // The open path, which the next edge's piece may go on.
        let mut open: Option<usize> = None;
        for (k, s) in self.steps.iter().enumerate() {
            if self.at[k].1 == 0 {
                open = None;
            }
            let (Some(ma), Some(mb)) = (s.ma, s.mb) else {
                open = None;
                continue;
            };
            let (t0, t1) = if ma == mb {
                if !(lo <= ma && ma <= hi) {
                    open = None;
                    continue;
                }
                (0.0, 1.0)
            } else {
                let ta = (lo - ma) / (mb - ma);
                let tb = (hi - ma) / (mb - ma);
                (js_max(js_min(ta, tb), 0.0), js_min(js_max(ta, tb), 1.0))
            };
            if t1 - t0 <= 0.0 {
                open = None;
                continue;
            }
            let (p0, p1) = (s.at(t0), s.at(t1));
            let bulge = s.piece_bulge(t0, t1);
            let (m0, m1) = (s.m(t0), s.m(t1));
            match open {
                Some(q) if t0 == 0.0 => {
                    let q = &mut parts[q];
                    q.pts.push(p1);
                    q.bulges.get_or_insert_with(Vec::new).push(bulge);
                    q.ms.push(m1);
                }
                _ => {
                    parts.push(RoutePart {
                        pts: vec![p0, p1],
                        bulges: Some(vec![bulge]),
                        ms: vec![m0, m1],
                    });
                    open = Some(parts.len() - 1);
                }
            }
            if t1 != 1.0 {
                open = None;
            }
        }
        if parts.is_empty() {
            return Err(self.outside());
        }
        if offset != 0.0 {
            for q in &mut parts {
                let bulges = q.bulges.clone().unwrap_or_default();
                match offset_bulge_path(&q.pts, &bulges, -offset * self.dir, false) {
                    OffsetResult::Path { pts, bulges } if pts.len() == q.pts.len() => {
                        q.pts = pts;
                        q.bulges = Some(bulges);
                    }
                    _ => return Err(Fault::Offset),
                }
            }
        }
        for q in &mut parts {
            if q.bulges
                .as_ref()
                .is_some_and(|b| b.iter().all(|&v| v == 0.0))
            {
                q.bulges = None;
            }
        }
        if (from > to) != (self.dir < 0.0) {
            parts.reverse();
            for q in &mut parts {
                reverse_part(q);
            }
        }
        Ok(parts)
    }
}

/// The closest place of the route to `p`; see [`Walk::locate`].
pub fn locate(route: &Route, p: Vec2) -> Option<Place> {
    Walk::new(route).locate(p)
}

/// The place of measure `m`, offset; see [`Walk::point_at`].
pub fn point_at(route: &Route, m: f64, offset: f64) -> Result<Vec2, Fault> {
    Walk::new(route).point_at(m, offset)
}

/// The places between two measures, offset; see [`Walk::stretch`].
pub fn stretch(route: &Route, from: f64, to: f64, offset: f64) -> Result<Vec<RoutePart>, Fault> {
    Walk::new(route).stretch(from, to, offset)
}

fn reverse_part(q: &mut RoutePart) {
    q.pts.reverse();
    q.ms.reverse();
    if let Some(b) = &mut q.bulges {
        b.reverse();
        for v in b.iter_mut() {
            *v = -*v;
        }
    }
}

/// The first, last, least and most measure of a route and how many
/// vertices have one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Summary {
    pub first: Option<f64>,
    pub last: Option<f64>,
    pub least: Option<f64>,
    pub most: Option<f64>,
    pub count: usize,
}

crate::json_struct!(Summary {
    first,
    last,
    least,
    most,
    count
});

pub fn summary(route: &Route) -> Summary {
    let ms: Vec<f64> = route
        .parts
        .iter()
        .flat_map(|p| p.ms.iter().flatten().copied())
        .collect();
    let range = measure_range(route);
    Summary {
        first: ms.first().copied(),
        last: ms.last().copied(),
        least: range.map(|r| r.0),
        most: range.map(|r| r.1),
        count: ms.len(),
    }
}

/// A line along a route: when every vertex is within `radius` of it, the
/// measures at the first's and the last's places and the farthest vertex's
/// distance.
#[derive(Clone, Debug, PartialEq)]
pub struct Along {
    pub from: f64,
    pub to: f64,
    pub distance: f64,
}

crate::json_struct!(Along { from, to, distance });

pub fn locate_line(route: &Route, pts: &[Vec2], radius: f64) -> Option<Along> {
    Walk::new(route).locate_line(pts, radius)
}

mod area;
mod calibrate;
mod calls;
mod create;
mod index;
mod overlay;

pub use area::{Inside, locate_area};
pub use calibrate::{CalibrateRules, Calibrated, Mark, calibrate};
pub(crate) use calls::OPS;
pub use calls::{CalibrateOut, EventIn, EventOut, Located, calibrate_run, events_run, locate_run};
pub use create::{
    CreateRules, Created, LeftOut, Made, Method, Piece, PieceFault, Priority, create,
};
pub use index::{Near, RouteIndex};
pub use overlay::{Event, Overlaid, overlay};
