//! Sürdür (docs/adr/0173 §4), one for both platforms (the web through
//! WASM): a line or an open polyline continued from one of its ends by a
//! drawn path. The drawn path starts at that end. From the last end its
//! points follow the object's; from the first they go before them turned
//! (their order and their arcs' sides reversed), so the object keeps its
//! direction. A line becomes a polyline. The independent reference is
//! `scripts/fixtures/reshape_cases.py`.

use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::geom::bulge::{bulge_at, clean_bulge_path, segment_tangent};
use crate::jsmath::js_hypot;
use crate::op;
use crate::vec2::Vec2;

/// A drawn point this near the previous one is the same point.
const SAME: f64 = 1e-9;

/// A path's two ends and the directions out of them: from the first
/// backwards, from the last onwards (along an end arc's tangent).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ends {
    pub first: Vec2,
    pub last: Vec2,
    /// None on an end segment of no length.
    pub out_first: Option<Vec2>,
    pub out_last: Option<Vec2>,
}

crate::json_struct!(Ends {
    first,
    last,
    out_first => "outFirst",
    out_last => "outLast"
});

/// A line's or an open polyline's vertices and one bulge per vertex.
fn path_of(e: &Shape) -> Option<(Vec<Vec2>, Vec<f64>)> {
    match e {
        Shape::Line { a, b } => Some((vec![*a, *b], vec![0.0, 0.0])),
        Shape::Polyline {
            pts,
            bulges,
            holes: None,
        } if pts.len() >= 2 => {
            let b = (0..pts.len())
                .map(|i| bulge_at(bulges.as_deref(), i))
                .collect();
            Some((pts.clone(), b))
        }
        _ => None,
    }
}

/// A unit direction, none for a segment of no length.
fn unit(v: Vec2, a: Vec2, b: Vec2) -> Option<Vec2> {
    (js_hypot(b.x - a.x, b.y - a.y) > SAME).then_some(v)
}

/// The ends of a line or an open polyline and the directions out of them;
/// none for any other object.
pub fn path_ends(e: &Shape) -> Option<Ends> {
    let (pts, b) = path_of(e)?;
    let n = pts.len();
    let onwards = segment_tangent(pts[n - 2], pts[n - 1], b[n - 2], true);
    let into = segment_tangent(pts[0], pts[1], b[0], false);
    Some(Ends {
        first: pts[0],
        last: pts[n - 1],
        out_first: unit(Vec2::new(-into.x, -into.y), pts[0], pts[1]),
        out_last: unit(onwards, pts[n - 2], pts[n - 1]),
    })
}

/// The object continued from its first end (`from_first`) or its last by
/// the drawn path `drawn` (its first point the end itself), with one bulge
/// per drawn segment. None when the object is no line or open polyline, or
/// nothing is drawn beyond the end.
pub fn continue_path(e: &Shape, from_first: bool, drawn: &[Vec2], bulges: &[f64]) -> Option<Shape> {
    let (pts, b) = path_of(e)?;
    let n0 = pts.len();
    let k = drawn.len().checked_sub(1).filter(|&k| k > 0)?;
    let seg = |i: usize| bulges.get(i).copied().unwrap_or(0.0);
    let (all, all_b) = if from_first {
        // The drawn path turned: its last point first; segment i runs from
        // drawn[k − i] to drawn[k − i − 1], drawn segment k − 1 − i turned.
        let mut all: Vec<Vec2> = drawn[1..].iter().rev().copied().collect();
        let mut all_b: Vec<f64> = (0..k).map(|i| -seg(k - 1 - i)).collect();
        all.extend(pts);
        all_b.extend(b);
        (all, all_b)
    } else {
        // The last vertex's bulge is the first drawn segment's.
        let n = pts.len();
        let mut all = pts;
        let mut all_b = b;
        all_b[n - 1] = seg(0);
        all.extend_from_slice(&drawn[1..]);
        all_b.extend((1..k).map(seg));
        all_b.push(0.0);
        (all, all_b)
    };
    let clean = clean_bulge_path(&all, Some(&all_b), false, SAME);
    (clean.pts.len() > n0).then_some(Shape::Polyline {
        pts: clean.pts,
        bulges: clean.bulges,
        holes: None,
    })
}

pub(crate) static OPS: &[Op] = &[
    op!("pathEnds", |e: Entity| path_ends(&e.shape)),
    op!("continuePath", |e: Entity,
                         from_first: bool,
                         drawn: Vec<Vec2>,
                         bulges: Vec<f64>| {
        continue_path(&e.shape, from_first, &drawn, &bulges).map(Entity::new)
    }),
];
