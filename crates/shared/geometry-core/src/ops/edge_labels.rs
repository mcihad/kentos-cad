//! Placement of edge-length labels, "kenar ölçüleri" (`apps/web/src/model/ops/edgeLabels.ts`):
//! centred on each edge, lifted outside a ring (left of an open path) by a
//! gap proportional to the text height; arcs labelled with their length.

use crate::api::Op;
use crate::geom::bulge::{bulge_arc, bulge_at, bulge_ring_area, segment_mid};
use crate::jsmath::{PI, atan2, cos, js_hypot, js_max, sin};
use crate::op;
use crate::vec2::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeLabel {
    /// Text anchor (baseline centre).
    pub p: Vec2,
    /// Degrees CCW from east, always readable (−90° < r ≤ 90°).
    pub rotation: f64,
    /// Edge length in metres.
    pub length: f64,
    /// Segment index: the edge from pts[index] to pts[index + 1].
    pub index: usize,
}

crate::json_struct!(out EdgeLabel { p, rotation, length, index });

pub fn edge_labels(
    pts: &[Vec2],
    closed: bool,
    height: f64,
    min_length: f64,
    bulges: Option<&[f64]>,
    inside: bool,
) -> Vec<EdgeLabel> {
    let n = pts.len();
    let count = if closed { n } else { n.saturating_sub(1) };
    // For a CCW ring the outside is to the right of travel; "inside" (or right of an open path) flips it.
    let outside = (if closed {
        if bulge_ring_area(pts, bulges) > 0.0 {
            -1.0
        } else {
            1.0
        }
    } else {
        1.0
    }) * if inside { -1.0 } else { 1.0 };
    let mut out = Vec::new();
    for i in 0..count {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let chord = js_hypot(dx, dy);
        let arc = bulge_arc(a, b, bulge_at(bulges, i));
        let length = match arc {
            Some(arc) => arc.r * arc.sweep.abs(),
            None => chord,
        };
        if length < js_max(min_length, 1e-9) || chord < 1e-12 {
            continue;
        }
        let nx = (-dy / chord) * outside;
        let ny = (dx / chord) * outside;
        // The tangent at an arc's midpoint is parallel to its chord.
        let mid = segment_mid(a, b, bulge_at(bulges, i));
        let mut rotation = (atan2(dy, dx) * 180.0) / PI;
        if rotation > 90.0 {
            rotation -= 180.0;
        } else if rotation <= -90.0 {
            rotation += 180.0;
        }
        // Text grows from its baseline towards "up" (rotation + 90°): when up points
        // away from the edge the baseline sits a small gap outside; otherwise it must
        // clear a full text height so the glyphs stay off the line.
        let r = (rotation * PI) / 180.0;
        let grows_outward = nx * -sin(r) + ny * cos(r) > 0.0;
        let lift = if grows_outward {
            height * 0.4
        } else {
            height * 1.4
        };
        out.push(EdgeLabel {
            p: Vec2::new(mid.x + nx * lift, mid.y + ny * lift),
            rotation,
            length,
            index: i,
        });
    }
    out
}

pub(crate) static OPS: &[Op] = &[op!(
    "edgeLabels",
    |pts: Vec<Vec2>,
     closed: bool,
     height: f64,
     min_length: Option<f64>,
     bulges: Option<Vec<f64>>,
     side: Option<String>| edge_labels(
        &pts,
        closed,
        height,
        min_length.unwrap_or(0.0),
        bulges.as_deref(),
        side.as_deref() == Some("inside")
    )
)];
