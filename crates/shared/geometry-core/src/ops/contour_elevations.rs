//! Eğrilere kot ver (docs/adr/0234 §9): the curves a cut line crosses, in
//! the order it crosses them from its start, each given an elevation: the
//! first the first elevation, each next a step more. A curve's place is
//! its crossing nearest the start (the core's edge crossings, the cut's
//! parameter from 0 at the start to 1 at the end); curves at the same
//! place keep the input's order; a curve the cut does not cross keeps its
//! own elevations. One rule for both platforms: the web calls
//! `contourElevations`, the desktop this.

use std::cmp::Ordering;

use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::geom::intersect::{Edge, intersect_edges};
use crate::op;
use crate::ops::edges::entity_edges;
use crate::vec2::Vec2;

/// Each curve's place along the cut from `start` to `end`; none where it does not cross.
pub fn places(curves: &[Shape], start: Vec2, end: Vec2) -> Vec<Option<f64>> {
    let cut = Edge::Seg { a: start, b: end };
    curves
        .iter()
        .map(|s| {
            entity_edges(s)
                .iter()
                .flat_map(|e| intersect_edges(&cut, e))
                .map(|h| h.t)
                .filter(|t| t.is_finite())
                .fold(None, |best: Option<f64>, t| {
                    Some(best.map_or(t, |b| if t < b { t } else { b }))
                })
        })
        .collect()
}

/// Each curve's elevation: the k-th crossed (k from 0) `first + k · step`
/// in float64 in this order; none for a curve the cut does not cross.
pub fn contour_elevations(
    curves: &[Shape],
    start: Vec2,
    end: Vec2,
    first: f64,
    step: f64,
) -> Vec<Option<f64>> {
    let at = places(curves, start, end);
    let mut order: Vec<usize> = (0..curves.len()).filter(|&k| at[k].is_some()).collect();
    order.sort_by(|&a, &b| {
        at[a]
            .partial_cmp(&at[b])
            .unwrap_or(Ordering::Equal)
            .then(a.cmp(&b))
    });
    let mut out = vec![None; curves.len()];
    for (k, &c) in order.iter().enumerate() {
        out[c] = Some(first + k as f64 * step);
    }
    out
}

pub(crate) static OPS: &[Op] = &[op!(
    "contourElevations",
    |curves: Vec<Entity>, start: Vec2, end: Vec2, first: f64, step: f64| {
        let shapes: Vec<Shape> = curves.into_iter().map(|e| e.shape).collect();
        contour_elevations(&shapes, start, end, first, step)
    }
)];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::json::{FromJson, Json};

    /// The shared cases (fixtures/raster-vector/v1/cases.json's `elevations`),
    /// written by scripts/fixtures/raster_vector_cases.py from docs/adr/0234 §9
    /// with exact rationals, not from this code; the web runs them through WASM.
    #[test]
    fn as_the_shared_cases_say() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../fixtures/raster-vector/v1/cases.json"
        );
        let file = Json::parse(&std::fs::read_to_string(path).expect("the cases")).expect("JSON");
        let Json::Arr(cases) = file.get("elevations") else {
            panic!("elevations")
        };
        assert!(cases.len() >= 2);
        for c in cases {
            let Json::Arr(list) = c.get("curves") else {
                panic!("curves")
            };
            let curves: Vec<Shape> = list
                .iter()
                .map(|e| Entity::from_json(e).expect("a curve").shape)
                .collect();
            let p = |k: &str| Vec2::from_json(c.get(k)).expect("a point");
            let num = |j: &Json| match j {
                Json::Num(x) => Some(*x),
                _ => None,
            };
            let n = |k: &str| num(c.get(k)).expect("a number");
            let got = contour_elevations(&curves, p("start"), p("end"), n("first"), n("step"));
            let Json::Arr(want) = c.get("expect") else {
                panic!("expect")
            };
            let want: Vec<Option<f64>> = want.iter().map(num).collect();
            assert_eq!(got, want, "{:?}", c.get("name"));
        }
    }

    fn line(x: f64) -> Shape {
        Shape::Line {
            a: Vec2::new(x, 0.0),
            b: Vec2::new(x + 1.0, 10.0),
        }
    }

    #[test]
    fn in_the_order_the_cut_crosses_them() {
        // Three lines crossed at x ≈ 3.5, 1.5 and 5.5; one the cut misses; the
        // cut runs right to left, so the first crossed is the farthest right.
        let curves = [line(3.0), line(1.0), line(20.0), line(5.0)];
        let z = contour_elevations(
            &curves,
            Vec2::new(9.0, 5.0),
            Vec2::new(0.0, 5.0),
            100.0,
            -2.5,
        );
        assert_eq!(z, vec![Some(97.5), Some(95.0), None, Some(100.0)]);
        // A closed curve crossed twice counts once, at its nearer crossing.
        let ring = Shape::Polygon {
            pts: vec![
                Vec2::new(2.0, 4.0),
                Vec2::new(8.0, 4.0),
                Vec2::new(8.0, 6.0),
                Vec2::new(2.0, 6.0),
            ],
            bulges: None,
            holes: None,
            parts: None,
        };
        let z = contour_elevations(
            &[ring, line(5.0)],
            Vec2::new(0.0, 5.0),
            Vec2::new(9.0, 5.0),
            10.0,
            1.0,
        );
        assert_eq!(z, vec![Some(10.0), Some(11.0)]);
    }
}
