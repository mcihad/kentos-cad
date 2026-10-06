//! Faz 2's constructions (docs/adr/0140): the circle sector (Daire dilimi),
//! points between two points (Ara nokta), the point where two distances or
//! two bearings meet (Kesişim noktası), an angle at a vertex (Açı ölç) and
//! the next dimension of a chain or a baseline (Zincir ölçü, Baz ölçü).
//! The tools pick and preview; what the points make is computed here.

use crate::api::Op;
use crate::api::json::{ToJson, field};
use crate::geom::arc::norm_angle;
use crate::geom::bulge::{BulgePath, bulge_of_sweep};
use crate::geom::dimension::{DimensionGeom, layout_dimension};
use crate::geom::intersect::{circle_circle, line_line};
use crate::jsmath::{PI, TAU, atan2, cos, js_hypot, js_min, sin};
use crate::op;
use crate::vec2::Vec2;

/// Daire dilimi: the closed ring of a sector centred on `c`, of radius `r`,
/// swept counter-clockwise from `a0` to `a1` (radians from east): the
/// centre, the arc's start and end, the arc as the ring's middle edge. A
/// sweep of zero (or a whole turn) or a radius not above zero makes none.
pub fn sector(c: Vec2, r: f64, a0: f64, a1: f64) -> Option<BulgePath> {
    let sweep = norm_angle(a1 - a0);
    if !(r > 1e-9) || sweep < 1e-9 || TAU - sweep < 1e-9 {
        return None;
    }
    let at = |a: f64| Vec2::new(c.x + r * cos(a), c.y + r * sin(a));
    Some(BulgePath {
        pts: vec![c, at(a0), at(a0 + sweep)],
        bulges: Some(vec![0.0, bulge_of_sweep(sweep), 0.0]),
    })
}

/// How Ara nokta places its points on the line from `a` to `b`.
pub enum Between {
    /// The line in `n` equal parts: the n − 1 points between them.
    Parts(f64),
    /// At these distances from `a` towards `b` (metres; beyond `b` or
    /// before `a` on the line's extension too).
    Distances(Vec<f64>),
    /// At these fractions of the way from `a` to `b` (0 is `a`, 1 is `b`).
    Ratios(Vec<f64>),
}

impl crate::api::json::FromJson for Between {
    fn from_json(v: &crate::api::json::Json) -> Result<Between, String> {
        use crate::api::json::{Json, read_field};
        let Json::Obj(fields) = v else {
            return Err("{ parts } ya da { distances } ya da { ratios } bekleniyordu".into());
        };
        let has = |k: &str| fields.iter().any(|(f, _)| f == k);
        if has("parts") {
            Ok(Between::Parts(read_field(v, "parts")?))
        } else if has("distances") {
            Ok(Between::Distances(read_field(v, "distances")?))
        } else {
            Ok(Between::Ratios(read_field(v, "ratios")?))
        }
    }
}

/// Ara nokta: points on the line from `a` to `b`. None when the two points
/// fall together, or the count is not 2 to 10 000 parts.
pub fn points_between(a: Vec2, b: Vec2, how: &Between) -> Option<Vec<Vec2>> {
    let l = js_hypot(b.x - a.x, b.y - a.y);
    if l < 1e-9 {
        return None;
    }
    let at = |t: f64| Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
    match how {
        Between::Parts(n) => {
            let n = n.floor();
            if !(2.0..=10_000.0).contains(&n) {
                return None;
            }
            Some((1..n as usize).map(|k| at(k as f64 / n)).collect())
        }
        Between::Distances(ds) => Some(ds.iter().map(|d| at(d / l)).collect()),
        Between::Ratios(ts) => Some(ts.iter().map(|t| at(*t)).collect()),
    }
}

/// Kesişim noktası, İki uzaklık: the points `da` from `a` and `db` from `b`
/// (none, one where the circles touch, or two).
pub fn distance_distance(a: Vec2, da: f64, b: Vec2, db: f64) -> Vec<Vec2> {
    if !(da > 0.0) || !(db > 0.0) {
        return Vec::new();
    }
    circle_circle(a, da, b, db)
}

/// Kesişim noktası, İki doğrultu: where the line from `a` along the bearing
/// `ta` meets the line from `b` along `tb`. Bearings are survey bearings in
/// radians: from grid north, clockwise. None for parallel bearings or a
/// meeting behind either point.
pub fn bearing_bearing(a: Vec2, ta: f64, b: Vec2, tb: f64) -> Option<Vec2> {
    let dir = |t: f64| Vec2::new(sin(t), cos(t));
    let (u, w) = (dir(ta), dir(tb));
    let hit = line_line(
        a,
        Vec2::new(a.x + u.x, a.y + u.y),
        b,
        Vec2::new(b.x + w.x, b.y + w.y),
    )?;
    let p = hit.p;
    let ahead = |o: Vec2, d: Vec2| (p.x - o.x) * d.x + (p.y - o.y) * d.y > -1e-9;
    (ahead(a, u) && ahead(b, w)).then_some(p)
}

/// An angle at a vertex: counter-clockwise from the first arm to the
/// second (`sweep`, 0 to a whole turn), the smaller of it and its
/// explement (`inner`) and the larger (`outer`), in radians.
pub struct Angle {
    pub sweep: f64,
    pub inner: f64,
    pub outer: f64,
}

impl ToJson for Angle {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "sweep", &self.sweep);
        field(out, &mut first, "inner", &self.inner);
        field(out, &mut first, "outer", &self.outer);
        out.push('}');
    }
}

/// Açı ölç: the angle at `vertex` between the arms through `p1` and `p2`;
/// None when an arm has no length.
pub fn angle_at(vertex: Vec2, p1: Vec2, p2: Vec2) -> Option<Angle> {
    let (v1, v2) = (
        Vec2::new(p1.x - vertex.x, p1.y - vertex.y),
        Vec2::new(p2.x - vertex.x, p2.y - vertex.y),
    );
    if js_hypot(v1.x, v1.y) < 1e-12 || js_hypot(v2.x, v2.y) < 1e-12 {
        return None;
    }
    let sweep = norm_angle(atan2(v2.y, v2.x) - atan2(v1.y, v1.x));
    let inner = js_min(sweep, TAU - sweep);
    Some(Angle {
        sweep,
        inner,
        outer: TAU - inner,
    })
}

/// The direction a straight dimension measures along, in degrees from
/// east, and its dimension line's distance to the left of `a`; None for
/// angular, radius and diameter dimensions or a degenerate one.
fn measuring(d: &DimensionGeom) -> Option<(f64, Vec2)> {
    let deg = match d.style.as_deref().unwrap_or("aligned") {
        "linear" => d.angle.unwrap_or(0.0),
        "aligned" => {
            let l = js_hypot(d.b.x - d.a.x, d.b.y - d.a.y);
            if l < 1e-9 {
                return None;
            }
            atan2(d.b.y - d.a.y, d.b.x - d.a.x) * 180.0 / PI
        }
        _ => return None,
    };
    layout_dimension(d)?;
    let t = deg * PI / 180.0;
    // Where the dimension line runs: a point of it (d1).
    let n = Vec2::new(-sin(t), cos(t));
    Some((
        deg,
        Vec2::new(d.a.x + n.x * d.offset, d.a.y + n.y * d.offset),
    ))
}

/// A linear dimension from `a` to `p` measuring along `deg`, its line through `line_at`.
fn linear_through(
    base: &DimensionGeom,
    deg: f64,
    line_at: Vec2,
    a: Vec2,
    p: Vec2,
    extra: f64,
) -> Option<DimensionGeom> {
    let t = deg * PI / 180.0;
    let n = Vec2::new(-sin(t), cos(t));
    let offset = (line_at.x - a.x) * n.x + (line_at.y - a.y) * n.y;
    let side = if offset < 0.0 { -1.0 } else { 1.0 };
    let d = DimensionGeom {
        a,
        b: p,
        offset: offset + side * extra,
        height: base.height,
        style: Some("linear".into()),
        angle: Some(deg),
        c: None,
        za: None,
        zb: None,
        look: base.look.clone(),
    };
    layout_dimension(&d).map(|_| d)
}

/// Zincir ölçü: the next dimension of a chain from `from` (the last
/// dimension's end) to `p`, measuring along the base dimension's direction
/// with its line on the base's line. None for a base that is not an aligned
/// or a linear dimension, or a new one of no length.
pub fn continue_dimension(base: &DimensionGeom, from: Vec2, p: Vec2) -> Option<DimensionGeom> {
    let (deg, line_at) = measuring(base)?;
    linear_through(base, deg, line_at, from, p, 0.0)
}

/// Baz ölçü: a dimension from the base dimension's first point to `p`,
/// along its direction, its line `level` steps of `spacing` metres further
/// out than the base's (away from the measured points). None as for
/// [`continue_dimension`], or a spacing not above zero.
pub fn baseline_dimension(
    base: &DimensionGeom,
    p: Vec2,
    level: f64,
    spacing: f64,
) -> Option<DimensionGeom> {
    if !(spacing > 0.0) {
        return None;
    }
    let (deg, line_at) = measuring(base)?;
    linear_through(base, deg, line_at, base.a, p, level * spacing)
}

pub(crate) static OPS: &[Op] = &[
    op!("sector", |c: Vec2, r: f64, a0: f64, a1: f64| sector(
        c, r, a0, a1
    )),
    op!("pointsBetween", |a: Vec2, b: Vec2, how: Between| {
        points_between(a, b, &how)
    }),
    op!("distanceDistance", |a: Vec2, da: f64, b: Vec2, db: f64| {
        distance_distance(a, da, b, db)
    }),
    op!("bearingBearing", |a: Vec2, ta: f64, b: Vec2, tb: f64| {
        bearing_bearing(a, ta, b, tb)
    }),
    op!("angleAt", |vertex: Vec2, p1: Vec2, p2: Vec2| angle_at(
        vertex, p1, p2
    )),
    op!("continueDimension", |base: DimensionGeom,
                              from: Vec2,
                              p: Vec2| {
        continue_dimension(&base, from, p)
    }),
    op!(
        "baselineDimension",
        |base: DimensionGeom, p: Vec2, level: f64, spacing: f64| {
            baseline_dimension(&base, p, level, spacing)
        }
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::bulge::bulge_at;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn close(a: Vec2, b: Vec2) -> bool {
        (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
    }

    #[test]
    fn a_quarter_sector_is_three_vertices_and_one_arc() {
        let s = sector(v(0.0, 0.0), 10.0, 0.0, PI / 2.0).expect("sector");
        assert!(close(s.pts[1], v(10.0, 0.0)) && close(s.pts[2], v(0.0, 10.0)));
        assert!((bulge_at(s.bulges.as_deref(), 1) - crate::jsmath::tan(PI / 8.0)).abs() < 1e-12);
        // The sweep runs counter-clockwise past east: 350° to 10° is 20°.
        let t = sector(v(0.0, 0.0), 1.0, 350f64.to_radians(), 10f64.to_radians()).expect("sector");
        assert!(
            (bulge_at(t.bulges.as_deref(), 1) - crate::jsmath::tan(20f64.to_radians() / 4.0)).abs()
                < 1e-12
        );
        assert!(sector(v(0.0, 0.0), 1.0, 1.0, 1.0).is_none());
        assert!(sector(v(0.0, 0.0), 0.0, 0.0, 1.0).is_none());
    }

    #[test]
    fn points_between_by_parts_distances_and_ratios() {
        let (a, b) = (v(0.0, 0.0), v(10.0, 0.0));
        let parts = points_between(a, b, &Between::Parts(4.0)).expect("parts");
        assert_eq!(parts, vec![v(2.5, 0.0), v(5.0, 0.0), v(7.5, 0.0)]);
        let d = points_between(a, b, &Between::Distances(vec![3.0, 12.0])).expect("distances");
        assert!(close(d[0], v(3.0, 0.0)) && close(d[1], v(12.0, 0.0)));
        let r = points_between(a, b, &Between::Ratios(vec![0.1])).expect("ratios");
        assert!(close(r[0], v(1.0, 0.0)));
        assert!(points_between(a, a, &Between::Parts(2.0)).is_none());
        assert!(points_between(a, b, &Between::Parts(1.0)).is_none());
    }

    #[test]
    fn two_distances_meet_at_two_points_or_none() {
        let hits = distance_distance(v(0.0, 0.0), 5.0, v(8.0, 0.0), 5.0);
        assert_eq!(hits.len(), 2);
        assert!(
            hits.iter().any(|p| close(*p, v(4.0, 3.0)))
                && hits.iter().any(|p| close(*p, v(4.0, -3.0)))
        );
        assert!(distance_distance(v(0.0, 0.0), 1.0, v(8.0, 0.0), 1.0).is_empty());
    }

    #[test]
    fn two_bearings_meet_ahead_of_both_points() {
        // From (0, 0) north-east (50 grad = 45°), from (10, 0) north-west (350 grad).
        let p = bearing_bearing(v(0.0, 0.0), PI / 4.0, v(10.0, 0.0), 7.0 * PI / 4.0).expect("meet");
        assert!(close(p, v(5.0, 5.0)));
        // Pointing away from each other: they meet behind, which is no answer.
        assert!(
            bearing_bearing(v(0.0, 0.0), 5.0 * PI / 4.0, v(10.0, 0.0), 3.0 * PI / 4.0).is_none()
        );
        assert!(bearing_bearing(v(0.0, 0.0), 0.0, v(10.0, 0.0), 0.0).is_none());
    }

    #[test]
    fn an_angle_and_its_explement() {
        let a = angle_at(v(0.0, 0.0), v(10.0, 0.0), v(0.0, 5.0)).expect("angle");
        assert!((a.sweep - PI / 2.0).abs() < 1e-12 && (a.inner - PI / 2.0).abs() < 1e-12);
        assert!((a.outer - 1.5 * PI).abs() < 1e-12);
        let b = angle_at(v(0.0, 0.0), v(0.0, 5.0), v(10.0, 0.0)).expect("angle");
        assert!((b.sweep - 1.5 * PI).abs() < 1e-12 && (b.inner - PI / 2.0).abs() < 1e-12);
        assert!(angle_at(v(0.0, 0.0), v(0.0, 0.0), v(1.0, 0.0)).is_none());
    }

    fn base() -> DimensionGeom {
        DimensionGeom {
            a: v(0.0, 0.0),
            b: v(10.0, 0.0),
            offset: 2.0,
            height: 0.5,
            style: None,
            angle: None,
            c: None,
            za: None,
            zb: None,
            look: Default::default(),
        }
    }

    #[test]
    fn a_chain_measures_along_the_base_on_its_line() {
        let next = continue_dimension(&base(), v(10.0, 0.0), v(16.0, -1.0)).expect("next");
        assert_eq!(next.style.as_deref(), Some("linear"));
        assert!((next.angle.expect("angle")).abs() < 1e-12);
        // Its line is the base's: 2 m left (north) of the base's first point, 2 m from (10, 0) too.
        assert!((next.offset - 2.0).abs() < 1e-12);
        let layout = layout_dimension(&next).expect("drawn");
        assert!((layout.value - 6.0).abs() < 1e-12, "measured along east");
        assert!((layout.d1.y - 2.0).abs() < 1e-12 && (layout.d2.y - 2.0).abs() < 1e-12);
        // A radius dimension is no base.
        let radius = DimensionGeom {
            style: Some("radius".into()),
            ..base()
        };
        assert!(continue_dimension(&radius, v(10.0, 0.0), v(16.0, 0.0)).is_none());
    }

    #[test]
    fn a_baseline_steps_out_from_the_base() {
        let second = baseline_dimension(&base(), v(18.0, 0.0), 1.0, 1.5).expect("second");
        assert_eq!(second.a, v(0.0, 0.0));
        assert!((second.offset - 3.5).abs() < 1e-12);
        let below = DimensionGeom {
            offset: -2.0,
            ..base()
        };
        let third = baseline_dimension(&below, v(25.0, 0.0), 2.0, 1.5).expect("third");
        assert!(
            (third.offset + 5.0).abs() < 1e-12,
            "away from the points, below"
        );
        assert!(baseline_dimension(&base(), v(18.0, 0.0), 1.0, 0.0).is_none());
    }
}
