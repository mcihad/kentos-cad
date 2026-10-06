//! Nokta hesaplayıcı ekleri (docs/adr/0188): what the point calculator's
//! newer constructions compute, for both platforms (the web through the op
//! table): a path's point at a distance from its start and an offset square
//! to it (Obje üzerinde nokta, Km ve sapma), where a point stands against a
//! path, the km notation read and written, a slope distance's horizontal
//! (Mesafe ve eğim) and the bisector of an angle (Açıortay). The
//! independent reference is `scripts/fixtures/point_calc_cases.py`
//! (`fixtures/point-calc/v1/cases.json`).
//!
//! A path is `ops::path`'s: arc length along an object's edges, a curve's
//! (fit-point curve, ellipse) along the chords that keep within 0.1 mm of it
//! (docs/adr/0149 §5.3). On a curve the point is taken onto the curve and
//! its square from the curve's own direction there: a chord's would turn an
//! offset of metres by centimetres.

use crate::api::Op;
use crate::display::fixed;
use crate::entity::{Entity, Shape, ellipse_geom, is_multi_part};
use crate::geom::ellipse::{closest_param, ellipse_derivative, ellipse_point};
use crate::geom::intersect::Edge;
use crate::geom::spline::catmull_rom_beziers;
use crate::jsmath::{js_hypot, js_max};
use crate::op;
use crate::ops::edges::edge_length;
use crate::ops::path::{Path, nearest_s, path_of, point_at_s, tangent_at_s};
use crate::tools::point_text::js_trim;
use crate::vec2::Vec2;

/// A path's point at a distance from its start; none when the distance is
/// outside the path. `length` is the path's, for a refusal to say.
#[derive(Clone, Debug, PartialEq)]
pub struct Station {
    pub point: Option<Vec2>,
    pub length: f64,
}

crate::json_struct!(out Station { point, length });

/// Where a point stands against a path: the distance of its nearest point
/// from the start and how far it is from the path, the right of the way
/// positive.
#[derive(Clone, Debug, PartialEq)]
pub struct Reading {
    pub s: f64,
    pub offset: f64,
    pub length: f64,
}

crate::json_struct!(out Reading { s, offset, length });

/// A slope distance's horizontal and the height it rises over it.
#[derive(Clone, Debug, PartialEq)]
pub struct Slope {
    pub horizontal: f64,
    pub rise: f64,
}

crate::json_struct!(out Slope { horizontal, rise });

/// The path the calculator walks on an object (docs/adr/0188 §1): a line,
/// a polyline, an arc, a circle, an ellipse, a fit-point curve, or an
/// area's outer boundary, its holes left out; none for the other kinds and
/// for a multi-part object, whose parts do not run on into each other.
pub fn route_of(e: &Shape) -> Option<Shape> {
    if is_multi_part(e) {
        return None;
    }
    match e {
        Shape::Line { .. }
        | Shape::Polyline { .. }
        | Shape::Arc { .. }
        | Shape::Circle { .. }
        | Shape::Ellipse { .. }
        | Shape::Spline { .. } => Some(e.clone()),
        Shape::Polygon { pts, bulges, .. } => Some(Shape::Polygon {
            pts: pts.clone(),
            bulges: bulges.clone(),
            holes: None,
            parts: None,
        }),
        _ => None,
    }
}

/// An edge walked the other way.
fn reversed(e: &Edge) -> Edge {
    match *e {
        Edge::Seg { a, b } => Edge::Seg { a: b, b: a },
        Edge::Arc { c, r, a0, sweep } => Edge::Arc {
            c,
            r,
            a0: a0 + sweep,
            sweep: -sweep,
        },
    }
}

/// The path walked from its end: its edges in the other order and way, so a
/// vertex's square is the next edge's in that walk too.
fn turned(path: &Path) -> Path {
    let edges: Vec<Edge> = path.edges.iter().rev().map(reversed).collect();
    let mut cum = Vec::with_capacity(edges.len());
    let mut s = 0.0;
    for e in &edges {
        cum.push(s);
        s += edge_length(e);
    }
    Path {
        edges,
        cum,
        length: path.length,
        closed: path.closed,
    }
}

/// Where a fit-point curve's span is at `u`, from its first control point,
/// and its derivative and second derivative there.
fn bezier(c: &[Vec2; 4], u: f64) -> (Vec2, Vec2, Vec2) {
    let o = c[0];
    let p1 = Vec2::new(c[1].x - o.x, c[1].y - o.y);
    let p2 = Vec2::new(c[2].x - o.x, c[2].y - o.y);
    let p3 = Vec2::new(c[3].x - o.x, c[3].y - o.y);
    let v = 1.0 - u;
    let (b1, b2, b3) = (3.0 * v * v * u, 3.0 * v * u * u, u * u * u);
    let at = Vec2::new(
        b1 * p1.x + b2 * p2.x + b3 * p3.x,
        b1 * p1.y + b2 * p2.y + b3 * p3.y,
    );
    // B′ = 3(1−u)²(p1−p0) + 6(1−u)u(p2−p1) + 3u²(p3−p2), p0 the origin.
    let (d0, d1, d2) = (3.0 * v * v, 6.0 * v * u, 3.0 * u * u);
    let d = Vec2::new(
        d0 * p1.x + d1 * (p2.x - p1.x) + d2 * (p3.x - p2.x),
        d0 * p1.y + d1 * (p2.y - p1.y) + d2 * (p3.y - p2.y),
    );
    // B″ = 6(1−u)(p2 − 2p1 + p0) + 6u(p3 − 2p2 + p1).
    let dd = Vec2::new(
        6.0 * v * (p2.x - 2.0 * p1.x) + 6.0 * u * (p3.x - 2.0 * p2.x + p1.x),
        6.0 * v * (p2.y - 2.0 * p1.y) + 6.0 * u * (p3.y - 2.0 * p2.y + p1.y),
    );
    (at, d, dd)
}

/// The fit-point curve's point nearest `p` and its derivative there: each
/// span's nearest of 32 samples, then Newton's steps to the foot of the
/// perpendicular on the three spans whose samples come nearest (a foot by
/// a fit point may lie in either span), the nearest foot of them.
fn spline_foot(pts: &[Vec2], closed: bool, p: Vec2) -> Option<(Vec2, Vec2)> {
    const SAMPLES: usize = 32;
    let spans = catmull_rom_beziers(pts, closed);
    let mut nearest: Vec<(f64, usize, f64)> = spans
        .iter()
        .enumerate()
        .map(|(i, span)| {
            let o = span.ctrl[0];
            let (mut best, mut bu) = (f64::INFINITY, 0.0);
            for k in 0..=SAMPLES {
                let u = k as f64 / SAMPLES as f64;
                let (at, _, _) = bezier(&span.ctrl, u);
                let d = js_hypot(o.x + at.x - p.x, o.y + at.y - p.y);
                if d < best {
                    (best, bu) = (d, u);
                }
            }
            (best, i, bu)
        })
        .collect();
    nearest.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut best: Option<(f64, Vec2, Vec2)> = None;
    for &(_, i, start) in nearest.iter().take(3) {
        let c = &spans[i].ctrl;
        let o = c[0];
        let (px, py) = (p.x - o.x, p.y - o.y);
        let mut u = start;
        for _ in 0..16 {
            let (at, d, dd) = bezier(c, u);
            let (ex, ey) = (at.x - px, at.y - py);
            let f = ex * d.x + ey * d.y;
            let df = d.x * d.x + d.y * d.y + ex * dd.x + ey * dd.y;
            if !(df > 0.0) {
                break;
            }
            let next = (u - f / df).clamp(0.0, 1.0);
            let done = (next - u).abs() < 1e-15;
            u = next;
            if done {
                break;
            }
        }
        let (at, d, _) = bezier(c, u);
        let gap = js_hypot(at.x - px, at.y - py);
        if best.is_none_or(|(g, _, _)| gap < g) {
            best = Some((gap, Vec2::new(o.x + at.x, o.y + at.y), d));
        }
    }
    best.map(|(_, at, d)| (at, d))
}

/// On a curve, its own point nearest `p` and its derivative there; none for
/// the other kinds, whose edges are their own.
fn curve_foot(e: &Shape, p: Vec2) -> Option<(Vec2, Vec2)> {
    match e {
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = ellipse_geom(*c, *major, *ratio, *t0, *t1);
            let t = closest_param(&g, p);
            Some((ellipse_point(&g, t), ellipse_derivative(&g, t)))
        }
        Shape::Spline { pts, closed } => spline_foot(pts, *closed, p),
        _ => None,
    }
}

/// A curve's derivative as a unit direction turned the way the path runs
/// there (`along`, its chord's).
fn way_of(d: Vec2, along: Vec2) -> Option<Vec2> {
    let l = js_hypot(d.x, d.y);
    if !(l > 0.0) {
        return None;
    }
    let way = if d.x * along.x + d.y * along.y < 0.0 {
        -1.0
    } else {
        1.0
    };
    Some(Vec2::new(d.x / l * way, d.y / l * way))
}

/// A route walked from one end (docs/adr/0188 §1, 0189 §2): the path its
/// points are taken along and the object it is drawn from, whose own curve
/// gives a curve's points and directions.
pub struct Walk {
    shape: Shape,
    path: Path,
}

impl Walk {
    /// The object's route walked from its start, or from its end when
    /// `from_end`; none for an object with no route (`route_of`).
    pub fn new(e: &Shape, from_end: bool) -> Option<Walk> {
        let shape = route_of(e)?;
        let forward = path_of(&shape)?;
        let path = if from_end { turned(&forward) } else { forward };
        Some(Walk { shape, path })
    }

    /// Its length along the route (along a curve's chords, within 0.1 mm of the curve's own).
    pub fn length(&self) -> f64 {
        self.path.length
    }

    /// Whether it comes round to its start.
    pub fn closed(&self) -> bool {
        self.path.closed
    }

    /// The point `s` along it and the unit direction it runs there: at a
    /// vertex the next edge's in the walk; round a closed path its length is
    /// its start again. On a curve the point is on the curve itself (the
    /// chords' is within 0.1 mm of it) and the direction is the curve's.
    pub fn frame(&self, s: f64) -> (Vec2, Vec2) {
        let q = point_at_s(&self.path, s);
        let t = tangent_at_s(&self.path, s);
        curve_foot(&self.shape, q)
            .and_then(|(p, d)| Some((p, way_of(d, t)?)))
            .unwrap_or((q, t))
    }
}

/// The point `s` along the path from its start (from its end when
/// `from_end`) and `offset` square to it there, the right of the way
/// positive. At a vertex the square is the next edge's in the walk; round a
/// closed path its length is its start again. None outside 0…length (the
/// caller says the length), or for an object with no route (`route_of`).
pub fn station(e: &Shape, from_end: bool, s: f64, offset: f64) -> Station {
    let Some(walk) = Walk::new(e, from_end) else {
        return Station {
            point: None,
            length: 0.0,
        };
    };
    let length = walk.length();
    if !(s >= 0.0 && s <= length && offset.is_finite()) {
        return Station {
            point: None,
            length,
        };
    }
    let (p, t) = walk.frame(s);
    Station {
        point: Some(Vec2::new(p.x + t.y * offset, p.y - t.x * offset)),
        length,
    }
}

/// Where `p` stands against the path walked from its start (from its end
/// when `from_end`): the distance of its nearest point and how far it is
/// from it, the right of the way positive (outside a corner, the distance
/// to the corner). None for an object with no route (`route_of`).
pub fn reading(e: &Shape, from_end: bool, p: Vec2) -> Option<Reading> {
    let path = path_of(&route_of(e)?)?;
    // On a curve the foot is the curve's own: a chord's way would slide a foot metres away by millimetres.
    let (s, q, t) = match curve_foot(e, p) {
        Some((foot, d)) => {
            let s = nearest_s(&path, foot);
            let along = tangent_at_s(&path, s);
            (s, foot, way_of(d, along).unwrap_or(along))
        }
        None => {
            let s = nearest_s(&path, p);
            (s, point_at_s(&path, s), tangent_at_s(&path, s))
        }
    };
    let away = js_hypot(p.x - q.x, p.y - q.y);
    // (p − q) · right, right = (t.y, −t.x).
    let side = (p.x - q.x) * t.y - (p.y - q.y) * t.x;
    let offset = if side < 0.0 { -away } else { away };
    Some(if from_end {
        Reading {
            s: path.length - s,
            offset: 0.0 - offset,
            length: path.length,
        }
    } else {
        Reading {
            s,
            offset,
            length: path.length,
        }
    })
}

/// `[0-9]+` alone.
fn digits(t: &str) -> bool {
    !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit())
}

/// `[0-9]+(\.[0-9]+)?` alone.
fn unsigned(t: &str) -> Option<f64> {
    let ok = match t.split_once('.') {
        Some((whole, fraction)) => digits(whole) && digits(fraction),
        None => digits(t),
    };
    if ok { t.parse().ok() } else { None }
}

/// A km typed (docs/adr/0188 §2): `k+mmm.mmm` (k whole kilometres, the
/// metres below a thousand) or metres (`-?[0-9]+(\.[0-9]+)?`), the spaces
/// round it dropped; ASCII digits only. None when it is neither.
pub fn km_value(text: &str) -> Option<f64> {
    let t = js_trim(text);
    if let Some((k, metres)) = t.split_once('+') {
        if !digits(k) {
            return None;
        }
        let metres = unsigned(metres)?;
        if metres >= 1000.0 {
            return None;
        }
        let k: f64 = k.parse().ok()?;
        return Some(k * 1000.0 + metres);
    }
    match t.strip_prefix('-') {
        Some(rest) => unsigned(rest).map(|v| -v),
        None => unsigned(t),
    }
}

/// A distance in metres written as km (`k+mmm.ddd`) with `decimals`, by the
/// display rule (docs/adr/0149): the metres that round to a thousand carry
/// into the kilometres; a minus before it.
pub fn km_text(value: f64, decimals: usize) -> String {
    let text = fixed(value, decimals);
    if !value.is_finite() {
        return text;
    }
    let (sign, body) = match text.strip_prefix('-') {
        Some(body) => ("-", body),
        None => ("", text.as_str()),
    };
    let (whole, fraction) = match body.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (body, None),
    };
    // Parted as text: every digit of a long one kept.
    let (k, metres) = if whole.len() > 3 {
        whole.split_at(whole.len() - 3)
    } else {
        ("0", whole)
    };
    match fraction {
        Some(f) => format!("{sign}{k}+{metres:0>3}.{f}"),
        None => format!("{sign}{k}+{metres:0>3}"),
    }
}

/// Mesafe ve eğim (docs/adr/0188 §4): a slope distance `s` at `percent`
/// slope as its horizontal `s / √(1 + (e/100)²)` and the rise over it.
pub fn slope(s: f64, percent: f64) -> Slope {
    let horizontal = s / js_hypot(1.0, percent / 100.0);
    Slope {
        horizontal,
        rise: horizontal * percent / 100.0,
    }
}

/// The unit direction of the inner bisector of the angle A–K–B; at a
/// straight angle square to K→A, to its left. None when K is on an arm's
/// point.
fn bisector_way(k: Vec2, a: Vec2, b: Vec2) -> Option<Vec2> {
    let (ka, kb) = (
        Vec2::new(a.x - k.x, a.y - k.y),
        Vec2::new(b.x - k.x, b.y - k.y),
    );
    let (la, lb) = (js_hypot(ka.x, ka.y), js_hypot(kb.x, kb.y));
    if !(la > 0.0 && lb > 0.0) {
        return None;
    }
    let (ua, ub) = (
        Vec2::new(ka.x / la, ka.y / la),
        Vec2::new(kb.x / lb, kb.y / lb),
    );
    let w = Vec2::new(ua.x + ub.x, ua.y + ub.y);
    let lw = js_hypot(w.x, w.y);
    if lw < 1e-12 {
        return Some(Vec2::new(-ua.y, ua.x));
    }
    Some(Vec2::new(w.x / lw, w.y / lw))
}

/// Açıortay (docs/adr/0188 §5): the point `d` from K along the bisector of
/// A–K–B (behind K when negative).
pub fn bisector(k: Vec2, a: Vec2, b: Vec2, d: f64) -> Option<Vec2> {
    let u = bisector_way(k, a, b)?;
    Some(Vec2::new(k.x + u.x * d, k.y + u.y * d))
}

/// The bisector's point nearest `p`, never behind K (a click).
pub fn bisector_nearest(k: Vec2, a: Vec2, b: Vec2, p: Vec2) -> Option<Vec2> {
    let u = bisector_way(k, a, b)?;
    let t = js_max(0.0, (p.x - k.x) * u.x + (p.y - k.y) * u.y);
    Some(Vec2::new(k.x + u.x * t, k.y + u.y * t))
}

pub(crate) static OPS: &[Op] = &[
    op!(
        "pointCalcStation",
        |e: Entity, from_end: bool, s: f64, offset: f64| station(&e.shape, from_end, s, offset)
    ),
    op!("pointCalcReading", |e: Entity, from_end: bool, p: Vec2| {
        reading(&e.shape, from_end, p)
    }),
    op!("kmValue", |text: String| km_value(&text)),
    op!("kmText", |value: f64, decimals: f64| km_text(
        value,
        if decimals.is_finite() && decimals > 0.0 {
            decimals as usize
        } else {
            0
        }
    )),
    op!("slopeHorizontal", |s: f64, percent: f64| slope(s, percent)),
    op!("bisectorPoint", |k: Vec2, a: Vec2, b: Vec2, d: f64| {
        bisector(k, a, b, d)
    }),
    op!("bisectorNearest", |k: Vec2, a: Vec2, b: Vec2, p: Vec2| {
        bisector_nearest(k, a, b, p)
    }),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn km_reads_its_two_forms_and_refuses_the_rest() {
        assert_eq!(km_value("1+250.5"), Some(1250.5));
        assert_eq!(km_value(" 0+000 "), Some(0.0));
        assert_eq!(km_value("-3"), Some(-3.0));
        for bad in [
            "1+1000", "1+", "+250", "1.5+250", "1+250,5", "abc", "", "-1+250", "1+-5",
        ] {
            assert_eq!(km_value(bad), None, "{bad}");
        }
        assert_eq!(km_text(999.9996, 3), "1+000.000");
        assert_eq!(km_text(-12.5, 3), "-0+012.500");
        assert_eq!(km_text(1_234_567.0, 0), "1234+567");
    }

    #[test]
    fn a_line_walked_from_its_end_has_its_right_on_the_other_side() {
        let line = Shape::Line {
            a: Vec2::new(0.0, 0.0),
            b: Vec2::new(10.0, 0.0),
        };
        assert_eq!(
            station(&line, false, 4.0, 2.0).point,
            Some(Vec2::new(4.0, -2.0))
        );
        assert_eq!(
            station(&line, true, 4.0, 2.0).point,
            Some(Vec2::new(6.0, 2.0))
        );
        assert_eq!(station(&line, false, 10.5, 0.0).point, None);
        let r = reading(&line, true, Vec2::new(3.0, 2.0)).unwrap();
        assert_eq!((r.s, r.offset), (7.0, 2.0));
    }
}
