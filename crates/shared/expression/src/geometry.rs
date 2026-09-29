//! Geometry values read directly from the geometry (docs/adr/0100 §3), for
//! hosts that hold geometry-core shapes: length, area, anchor, centroid,
//! bounding box, corners. Each is computed only when an expression reads it,
//! and an object's centroid, box and anchor once per batch however many of
//! their values the expression reads (`Shapes`).
//!
//! Length, area and anchor are the geometry core's (`entity_length`,
//! `entity_area`, `entity_anchor`: the same values the web's `measures`
//! gives). The centroid of an area is its own: the rings' first moments
//! relative to the first vertex (projected coordinates of millions of metres
//! would cancel away the decimals otherwise), a bulged edge's circular
//! segment exactly, holes subtracted.

use std::cell::RefCell;

use kentos_geometry_core::entity::{
    Shape, area_parts, entity_anchor, entity_area, entity_bounds, entity_length, is_multi_part,
};
use kentos_geometry_core::geom::bulge::{bulge_arc, bulge_at, segment_mid};
use kentos_geometry_core::geometry::{Bounds, is_empty_bounds};
use kentos_geometry_core::jsmath::sin;
use kentos_geometry_core::vec2::Vec2;

use crate::exec::Slot;
use crate::host::Geometry;

/// One object's geometry value (None: it has none).
pub fn value(s: &Shape, what: Geometry) -> Option<f64> {
    match what {
        Geometry::Length => entity_length(s),
        Geometry::Area => entity_area(s),
        Geometry::AnchorY => entity_anchor(s).map(|p| p.x),
        Geometry::AnchorX => entity_anchor(s).map(|p| p.y),
        Geometry::CentroidY => centroid(s).map(|p| p.x),
        Geometry::CentroidX => centroid(s).map(|p| p.y),
        Geometry::Vertices => vertices(s),
        box_value => boxed(bounds(s), box_value),
    }
}

/// A value of the bounding box (Y is east, the code's x).
fn boxed(b: Option<Bounds>, what: Geometry) -> Option<f64> {
    let b = b?;
    Some(match what {
        Geometry::MinY => b.min_x,
        Geometry::MaxY => b.max_x,
        Geometry::MinX => b.min_y,
        Geometry::MaxX => b.max_y,
        Geometry::Width => b.max_x - b.min_x,
        Geometry::Height => b.max_y - b.min_y,
        _ => return None,
    })
}

/// Corners of a path or area, holes included, every part of a multi-part
/// area (docs/adr/0143) (the web's `vertexCount`).
pub fn vertices(s: &Shape) -> Option<f64> {
    let n = match s {
        Shape::Polyline { pts, .. } | Shape::Spline { pts, .. } => pts.len(),
        Shape::Polygon { .. } => area_parts(s)
            .iter()
            .map(|part| match part {
                Shape::Polygon { pts, holes, .. } => {
                    pts.len() + holes.iter().flatten().map(|h| h.pts.len()).sum::<usize>()
                }
                _ => 0,
            })
            .sum(),
        Shape::Line { .. } => 2,
        Shape::Point { .. } => 1,
        _ => return None,
    };
    Some(n as f64)
}

/// The bounding box (the geometry core's; text measured in its default face).
pub fn bounds(s: &Shape) -> Option<Bounds> {
    let b = entity_bounds(s);
    (!is_empty_bounds(&b)).then_some(b)
}

/// A ring's signed area and first moments about `o`, a bulged edge's
/// circular segment included (signed as the geometry core's area signs it).
fn moments(pts: &[Vec2], bulges: Option<&[f64]>, o: Vec2) -> (f64, f64, f64) {
    let n = pts.len();
    let (mut a, mut mx, mut my) = (0.0, 0.0, 0.0);
    if n < 2 {
        return (a, mx, my);
    }
    for i in 0..n {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        let (px, py, qx, qy) = (p.x - o.x, p.y - o.y, q.x - o.x, q.y - o.y);
        let cross = px * qy - qx * py;
        a += cross / 2.0;
        mx += (px + qx) * cross / 6.0;
        my += (py + qy) * cross / 6.0;
        if let Some(arc) = bulge_arc(p, q, bulge_at(bulges, i)) {
            let t = arc.sweep.abs();
            let lens = t - sin(t);
            if lens <= 0.0 || arc.r <= 0.0 || lens.is_nan() || arc.r.is_nan() {
                continue;
            }
            // The segment between chord and arc: its area, signed as the
            // sweep, and its centroid on the bisector at 4r·sin³(θ/2)/(3(θ − sin θ)).
            let s = arc.r * arc.r / 2.0 * (arc.sweep - sin(arc.sweep));
            let half = sin(t / 2.0);
            let d = 4.0 * arc.r * half * half * half / (3.0 * lens);
            let m = segment_mid(p, q, bulge_at(bulges, i));
            let (ux, uy) = ((m.x - arc.c.x) / arc.r, (m.y - arc.c.y) / arc.r);
            let (gx, gy) = (arc.c.x - o.x + ux * d, arc.c.y - o.y + uy * d);
            a += s;
            mx += s * gx;
            my += s * gy;
        }
    }
    (a, mx, my)
}

/// The centroid of an outer ring with holes (areas taken positive, holes
/// removed), or None when the area is empty.
fn area_centroid<'r>(
    outer: (&'r [Vec2], Option<&'r [f64]>),
    holes: impl Iterator<Item = (&'r [Vec2], Option<&'r [f64]>)>,
) -> Option<Vec2> {
    let o = *outer.0.first()?;
    let (a, mx, my) = moments(outer.0, outer.1, o);
    let sign = if a < 0.0 { -1.0 } else { 1.0 };
    let (mut area, mut sx, mut sy) = (a * sign, mx * sign, my * sign);
    for (pts, bulges) in holes {
        let (a, mx, my) = moments(pts, bulges, o);
        let sign = if a < 0.0 { -1.0 } else { 1.0 };
        area -= a * sign;
        sx -= mx * sign;
        sy -= my * sign;
    }
    if area == 0.0 || !area.is_finite() {
        return None;
    }
    Some(Vec2::new(o.x + sx / area, o.y + sy / area))
}

/// The centroid: of the area for an area (holes removed), a circle and a
/// whole ellipse; the anchor for everything else and for an empty area.
pub fn centroid(s: &Shape) -> Option<Vec2> {
    // A multi-part area's is its parts' centroids weighted by their areas (docs/adr/0143).
    if is_multi_part(s) {
        let (mut total, mut sx, mut sy) = (0.0, 0.0, 0.0);
        for part in area_parts(s).iter() {
            let (Some(c), Some(a)) = (centroid(part), entity_area(part)) else {
                continue;
            };
            total += a;
            sx += a * c.x;
            sy += a * c.y;
        }
        if total > 0.0 && total.is_finite() {
            return Some(Vec2::new(sx / total, sy / total));
        }
        return entity_anchor(s);
    }
    let area = match s {
        Shape::Polygon {
            pts, bulges, holes, ..
        } => area_centroid(
            (pts, bulges.as_deref()),
            holes
                .iter()
                .flatten()
                .map(|h| (h.pts.as_slice(), h.bulges.as_deref())),
        ),
        Shape::Hatch { ring, holes, .. } => area_centroid(
            (ring, None),
            holes.iter().flatten().map(|h| (h.as_slice(), None)),
        ),
        Shape::Circle { c, .. } => Some(*c),
        // A whole ellipse has an area (`entity_area`), an arc of one has none.
        Shape::Ellipse { c, .. } if entity_area(s).is_some() => Some(*c),
        _ => None,
    };
    area.or_else(|| entity_anchor(s))
}

/// Groups of values computed together for a batch.
#[derive(Clone, Copy, PartialEq)]
enum Group {
    Anchor,
    Centroid,
    Box,
}

#[derive(Default)]
struct Cache {
    /// The batch the groups below belong to (`start`, count).
    batch: Option<(usize, usize)>,
    anchors: Option<Vec<Option<Vec2>>>,
    centroids: Option<Vec<Option<Vec2>>>,
    boxes: Option<Vec<Option<Bounds>>>,
}

/// Geometry values of a host's shapes, a batch at a time. `shape(i)` is
/// object `i`'s shape (None: it has no geometry).
pub struct Shapes<'s, F: Fn(usize) -> Option<&'s Shape>> {
    shape: F,
    cache: RefCell<Cache>,
    _shapes: std::marker::PhantomData<&'s Shape>,
}

impl<'s, F: Fn(usize) -> Option<&'s Shape>> Shapes<'s, F> {
    pub fn new(shape: F) -> Self {
        Shapes {
            shape,
            cache: RefCell::new(Cache::default()),
            _shapes: std::marker::PhantomData,
        }
    }

    /// Writes `what` for objects `start .. start + slot.len()`.
    pub fn fill(&self, what: Geometry, start: usize, mut slot: Slot<'_, '_>) {
        let n = slot.len();
        let group = match what {
            Geometry::AnchorY | Geometry::AnchorX => Group::Anchor,
            Geometry::CentroidY | Geometry::CentroidX => Group::Centroid,
            Geometry::MinY
            | Geometry::MaxY
            | Geometry::MinX
            | Geometry::MaxX
            | Geometry::Width
            | Geometry::Height => Group::Box,
            // Read once per batch anyway: one register each.
            Geometry::Length | Geometry::Area | Geometry::Vertices => {
                for i in 0..n {
                    slot.number(i, (self.shape)(start + i).and_then(|s| value(s, what)));
                }
                return;
            }
        };
        let Ok(mut cache) = self.cache.try_borrow_mut() else {
            // Asked while filling (a host that nests): computed without the cache.
            for i in 0..n {
                slot.number(i, (self.shape)(start + i).and_then(|s| value(s, what)));
            }
            return;
        };
        if cache.batch != Some((start, n)) {
            *cache = Cache {
                batch: Some((start, n)),
                ..Cache::default()
            };
        }
        let shape = |i: usize| (self.shape)(start + i);
        match group {
            Group::Anchor => {
                let anchors = cache.anchors.get_or_insert_with(|| {
                    (0..n).map(|i| shape(i).and_then(entity_anchor)).collect()
                });
                for (i, p) in anchors.iter().enumerate() {
                    slot.number(
                        i,
                        p.map(|p| if what == Geometry::AnchorY { p.x } else { p.y }),
                    );
                }
            }
            Group::Centroid => {
                let centroids = cache
                    .centroids
                    .get_or_insert_with(|| (0..n).map(|i| shape(i).and_then(centroid)).collect());
                for (i, p) in centroids.iter().enumerate() {
                    slot.number(
                        i,
                        p.map(|p| {
                            if what == Geometry::CentroidY {
                                p.x
                            } else {
                                p.y
                            }
                        }),
                    );
                }
            }
            Group::Box => {
                let boxes = cache
                    .boxes
                    .get_or_insert_with(|| (0..n).map(|i| shape(i).and_then(bounds)).collect());
                for (i, b) in boxes.iter().enumerate() {
                    slot.number(i, boxed(*b, what));
                }
            }
        }
    }
}
