//! Edge-length labels for many shapes at once
//! (`apps/web/src/processing/builtin/edgeLengths.ts`, "Kenar uzunluklarını yaz"): every
//! edge of lines, polylines and polygons gets its length where
//! `ops::edge_labels` puts it, and an edge two shapes share is labelled once.
//! Shared means the same two ends on a millimetre grid and the same length in
//! millimetres, whichever way the edge runs (docs/adr/0008, S4).

use std::borrow::Cow;
use std::collections::HashSet;

use crate::api::Op;
use crate::api::json::Json;
use crate::entity::{Entity, Shape};
use crate::jsmath::js_round;
use crate::op;
use crate::ops::edge_labels::{EdgeLabel, edge_labels};
use crate::vec2::Vec2;

/// A path whose edges are labelled: a line's two ends, a polyline's or a
/// polygon's vertices (a polygon's holes are not labelled).
pub struct EdgePath<'a> {
    pub pts: Cow<'a, [Vec2]>,
    pub closed: bool,
    pub bulges: Option<&'a [f64]>,
}

/// The labelled path of an object, if it has one.
pub fn edge_path(s: &Shape) -> Option<EdgePath<'_>> {
    match s {
        Shape::Line { a, b } => Some(EdgePath {
            pts: Cow::Owned(vec![*a, *b]),
            closed: false,
            bulges: None,
        }),
        Shape::Polyline { pts, bulges, .. } | Shape::Polygon { pts, bulges, .. } => {
            Some(EdgePath {
                pts: Cow::Borrowed(pts),
                closed: matches!(s, Shape::Polygon { .. }),
                bulges: bulges.as_deref(),
            })
        }
        _ => None,
    }
}

/// A coordinate or a length on the millimetre grid, as a key: `Math.round(x
/// · 1000)` written as text made −0 and 0 one key, and every NaN one key.
fn mm(x: f64) -> u64 {
    let r = js_round(x * 1000.0);
    if r == 0.0 {
        0
    } else if r.is_nan() {
        f64::NAN.to_bits()
    } else {
        r.to_bits()
    }
}

/// One key per edge, whichever way it runs.
fn edge_key(a: Vec2, b: Vec2, length: f64) -> [u64; 5] {
    let ka = [mm(a.x), mm(a.y)];
    let kb = [mm(b.x), mm(b.y)];
    let (k1, k2) = if ka <= kb { (ka, kb) } else { (kb, ka) };
    [k1[0], k1[1], k2[0], k2[1], mm(length)]
}

/// Labels of every path in order, each with the index of its path; with
/// `shared`, an edge already labelled is skipped and counted.
pub fn edge_lengths(
    paths: &[EdgePath<'_>],
    height: f64,
    min_length: f64,
    inside: bool,
    shared: bool,
) -> (Vec<(usize, EdgeLabel)>, usize) {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    let mut skipped = 0;
    for (k, path) in paths.iter().enumerate() {
        let pts = &path.pts;
        let n = pts.len();
        for l in edge_labels(pts, path.closed, height, min_length, path.bulges, inside) {
            if shared && !seen.insert(edge_key(pts[l.index], pts[(l.index + 1) % n], l.length)) {
                skipped += 1;
                continue;
            }
            out.push((k, l));
        }
    }
    (out, skipped)
}

/// A label with its object's id, as `edgeLengthLabels` answers.
pub struct IdLabel {
    pub id: Option<f64>,
    pub p: Vec2,
    pub rotation: f64,
    pub length: f64,
}

crate::json_struct!(out IdLabel { id, p, rotation, length });

pub struct EdgeLengthLabels {
    pub labels: Vec<IdLabel>,
    pub skipped: usize,
}

crate::json_struct!(out EdgeLengthLabels { labels, skipped });

/// An object's id, from the fields the core keeps as they came.
fn id_of(e: &Entity) -> Option<f64> {
    e.rest.iter().find_map(|(k, v)| match (k.as_str(), v) {
        ("id", Json::Num(n)) => Some(*n),
        _ => None,
    })
}

pub(crate) static OPS: &[Op] = &[op!(
    "edgeLengthLabels",
    |entities: Vec<Entity>,
     height: f64,
     min_length: Option<f64>,
     side: Option<String>,
     shared: bool| {
        let mut ids = Vec::new();
        let mut paths = Vec::new();
        for e in &entities {
            if let Some(p) = edge_path(&e.shape) {
                ids.push(id_of(e));
                paths.push(p);
            }
        }
        let inside = side.as_deref() == Some("inside");
        let (labels, skipped) =
            edge_lengths(&paths, height, min_length.unwrap_or(0.0), inside, shared);
        EdgeLengthLabels {
            labels: labels
                .into_iter()
                .map(|(k, l)| IdLabel {
                    id: ids[k],
                    p: l.p,
                    rotation: l.rotation,
                    length: l.length,
                })
                .collect(),
            skipped,
        }
    }
)];

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn square(x: f64, y: f64, s: f64) -> Vec<Vec2> {
        vec![v(x, y), v(x + s, y), v(x + s, y + s), v(x, y + s)]
    }

    fn path(pts: &[Vec2], closed: bool) -> EdgePath<'_> {
        EdgePath {
            pts: Cow::Borrowed(pts),
            closed,
            bulges: None,
        }
    }

    #[test]
    fn a_shared_edge_is_labelled_once_whichever_way_it_runs() {
        let a = square(0.0, 0.0, 10.0);
        let b = square(10.0, 0.0, 10.0);
        // The shared edge again as a line drawn the other way, a millimetre off.
        let line = [v(10.0004, 10.0), v(10.0, 0.0)];
        let paths = [path(&a, true), path(&b, true), path(&line, false)];
        let (labels, skipped) = edge_lengths(&paths, 2.0, 0.0, false, true);
        assert_eq!((labels.len(), skipped), (7, 2));
        assert!(labels.iter().all(|(_, l)| l.length == 10.0));
        let (all, none) = edge_lengths(&paths, 2.0, 0.0, false, false);
        assert_eq!((all.len(), none), (9, 0));
    }

    #[test]
    fn keys_fold_negative_zero_and_millimetres() {
        assert_eq!(mm(-0.0001), mm(0.0001));
        assert_eq!(mm(-0.0), mm(0.0));
        assert_ne!(mm(0.0006), mm(0.0));
        assert_eq!(
            edge_key(v(1.0, 2.0), v(3.0, 4.0), 5.0),
            edge_key(v(3.0, 4.0), v(1.0, 2.0), 5.0)
        );
    }
}
