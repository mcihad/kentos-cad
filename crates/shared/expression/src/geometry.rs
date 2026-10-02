//! Geometry values read directly from the geometry (docs/adr/0100 §3), for
//! hosts that hold geometry-core shapes: length, area, anchor, centroid,
//! bounding box, corners. Each is computed only when an expression reads it,
//! and an object's centroid, box and anchor once per batch however many of
//! their values the expression reads (`Shapes`).
//!
//! Length, area, anchor and centroid are the geometry core's (`entity_length`,
//! `entity_area`, `entity_anchor`: the same values the web's `measures`
//! gives; `geom::centroid::shape_centroid`, which the Ağırlık merkezi snap
//! uses too, docs/adr/0163).

use std::cell::RefCell;

use kentos_geometry_core::entity::{
    Shape, area_parts, entity_anchor, entity_area, entity_bounds, entity_length,
};
use kentos_geometry_core::geom::centroid::shape_centroid as centroid;
use kentos_geometry_core::geometry::{Bounds, is_empty_bounds};
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
