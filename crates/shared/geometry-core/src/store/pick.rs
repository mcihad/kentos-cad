//! Picking and selection on the store (`apps/web/src/viewport/picking.ts`), ported
//! rule for rule: points and edges before interiors, the smallest area
//! containing the cursor, window and crossing selection, the smallest
//! closed shape around a point. Candidates come in the document's order and
//! the TypeScript's comparisons decide, ties included.

use super::{Item, Store, padded};
use crate::text::Font;
use crate::entity::{
    Shape, dimension_geom, ellipse_geom, entity_area, entity_outline, inside_polygon,
    is_closed_outline, polygon_holes, polygon_ring, text_box,
};
use crate::geom::dimension::layout_dimension;
use crate::geom::ellipse::{ellipse_area, inside_ellipse, is_full_ellipse, tessellate_ellipse};
use crate::geom::intersect::{Edge, closest_on_edge, seg_seg};
use crate::geometry::{Bounds, point_in_polygon, signed_area};
use crate::jsmath::{PI, js_cmp, js_hypot, js_max, js_min};
use crate::ops::edges::entity_edges;
use crate::vec2::Vec2;

fn infinite(s: &Shape) -> bool {
    matches!(s, Shape::Xline { .. } | Shape::Ray { .. })
}

impl Store {
    /// Visible objects whose boxes come within `tol` of `p` (`PickIndex.near`).
    pub fn near(&self, p: Vec2, tol: f64) -> Vec<&Item> {
        let q = padded(
            Bounds {
                min_x: p.x,
                min_y: p.y,
                max_x: p.x,
                max_y: p.y,
            },
            tol,
        );
        self.candidates(&q)
            .into_iter()
            .filter(|it| {
                if !self.flags(it).visible {
                    return false;
                }
                // Infinite lines have no useful bounds; their distance test decides.
                if infinite(&it.shape) {
                    return true;
                }
                let b = &it.bounds;
                !(p.x < b.min_x - tol
                    || p.x > b.max_x + tol
                    || p.y < b.min_y - tol
                    || p.y > b.max_y + tol)
            })
            .collect()
    }

    /// Visible objects whose boxes overlap `r`, one id left out (`PickIndex.overlapping`).
    pub fn overlapping(&self, r: &Bounds, except: Option<f64>) -> Vec<&Item> {
        self.candidates(&padded(*r, 0.0))
            .into_iter()
            .filter(|it| {
                if Some(it.id) == except || !self.flags(it).visible {
                    return false;
                }
                if infinite(&it.shape) {
                    return true;
                }
                let b = &it.bounds;
                !(b.max_x < r.min_x || b.min_x > r.max_x || b.max_y < r.min_y || b.min_y > r.max_y)
            })
            .collect()
    }

    /// The most specific object at `p`: points and edges first, then the
    /// smallest area containing it (building before parcel before block).
    /// Layers with `pickInterior: false` are edge-pick only (`PickIndex.hit`).
    pub fn hit(&self, p: Vec2, tol: f64) -> Option<f64> {
        let mut edge: Option<(f64, f64)> = None;
        let mut area: Option<(f64, f64)> = None;
        for it in self.near(p, tol * 1.5) {
            let e = &it.shape;
            let d = edge_distance(e, p, self.font);
            let t = if matches!(e, Shape::Point { .. } | Shape::Text { .. }) {
                tol * 1.5
            } else {
                tol
            };
            if d <= t && edge.is_none_or(|(_, best)| d < best) {
                edge = Some((it.id, d));
            }
            if !self.flags(it).pick_interior {
                continue;
            }
            let a = match e {
                Shape::Hatch { ring, holes, .. }
                    if point_in_polygon(p, ring)
                        && !holes.iter().flatten().any(|h| point_in_polygon(p, h)) =>
                {
                    // Slightly smaller than its boundary so the hatch wins over the parcel it fills.
                    Some(entity_area(e).unwrap_or(0.0) * 0.999)
                }
                // Net area: a parcel with a building hole still loses to the building.
                Shape::Polygon { .. } if inside_polygon(e, p) => {
                    Some(entity_area(e).unwrap_or(0.0))
                }
                Shape::Circle { c, r } if js_hypot(p.x - c.x, p.y - c.y) < *r => Some(PI * r * r),
                Shape::Ellipse {
                    c,
                    major,
                    ratio,
                    t0,
                    t1,
                } => {
                    let g = ellipse_geom(*c, *major, *ratio, *t0, *t1);
                    (is_full_ellipse(&g) && inside_ellipse(&g, p)).then(|| ellipse_area(&g))
                }
                _ => None,
            };
            if let Some(a) = a
                && area.is_none_or(|(_, best)| a < best)
            {
                area = Some((it.id, a));
            }
        }
        edge.or(area).map(|(id, _)| id)
    }

    /// Objects whose edges come within `tol` of `p` (not points or text),
    /// nearest first and in the document's order among equals: the first
    /// one a caller's filter accepts is what `PickIndex.hitEdge` picked.
    pub fn hit_edge(&self, p: Vec2, tol: f64) -> Vec<(f64, f64)> {
        let mut out: Vec<(f64, f64)> = self
            .near(p, tol)
            .into_iter()
            .filter(|it| !matches!(it.shape, Shape::Point { .. } | Shape::Text { .. }))
            .map(|it| (it.id, edge_distance(&it.shape, p, self.font)))
            .filter(|&(_, d)| d <= tol)
            .collect();
        // Stable, and −0 equals 0 as in `d < best.d`: equal distances keep the document's order.
        crate::jsmath::stable_sort(&mut out, &mut |a, b| js_cmp(a.1, b.1));
        out
    }

    /// The smallest visible closed shape (polygon, circle, full ellipse, closed
    /// spline) containing `p`, with its ring: the boundary a hatch fills.
    pub fn enclosing(&self, p: Vec2) -> Option<(f64, Vec<Vec2>)> {
        let mut best: Option<(f64, Vec<Vec2>, f64)> = None;
        for it in self.near(p, 0.0) {
            let e = &it.shape;
            let ring = match e {
                Shape::Polygon { pts, bulges, .. } => {
                    inside_polygon(e, p).then(|| polygon_ring(pts, bulges.as_deref()))
                }
                Shape::Circle { .. } => Some(entity_outline(e, 96.0)),
                Shape::Ellipse {
                    c,
                    major,
                    ratio,
                    t0,
                    t1,
                } => {
                    let g = ellipse_geom(*c, *major, *ratio, *t0, *t1);
                    is_full_ellipse(&g).then(|| tessellate_ellipse(&g, 256.0))
                }
                Shape::Spline { closed: true, .. } => {
                    let mut r = entity_outline(e, 72.0);
                    r.pop();
                    Some(r)
                }
                _ => None,
            };
            let Some(ring) = ring else { continue };
            if !point_in_polygon(p, &ring) {
                continue;
            }
            let a = signed_area(&ring).abs();
            if best.as_ref().is_none_or(|b| a < b.2) {
                best = Some((it.id, ring, a));
            }
        }
        best.map(|(id, ring, _)| (id, ring))
    }

    /// Edges of every visible object overlapping `r`, one left out: boundaries for trim and extend.
    pub fn edges_in(&self, r: &Bounds, except: Option<f64>) -> Vec<Edge> {
        self.overlapping(r, except)
            .into_iter()
            .flat_map(|it| entity_edges(&it.shape))
            .collect()
    }

    /// Window (fully inside) or crossing (touching) selection, in the document's order.
    pub fn in_rect(&self, r: &Bounds, crossing: bool) -> Vec<f64> {
        let mut out = Vec::new();
        for it in self.candidates(&padded(*r, 0.0)) {
            if !self.flags(it).visible {
                continue;
            }
            let b = &it.bounds;
            // Infinite lines are never "inside" a window (as in AutoCAD); a crossing box catches them.
            let inf = infinite(&it.shape);
            let inside = !inf
                && b.min_x >= r.min_x
                && b.max_x <= r.max_x
                && b.min_y >= r.min_y
                && b.max_y <= r.max_y;
            if inside {
                out.push(it.id);
                continue;
            }
            if !crossing {
                continue;
            }
            let over = inf
                || (b.min_x <= r.max_x
                    && b.max_x >= r.min_x
                    && b.min_y <= r.max_y
                    && b.max_y >= r.min_y);
            if over && touches_rect(&it.shape, r) {
                out.push(it.id);
            }
        }
        out
    }
}

/// Distance from `p` to what can be clicked of an object: its edges, a
/// text's whole body, a dimension's value text.
pub fn edge_distance(e: &Shape, p: Vec2, font: Font) -> f64 {
    match e {
        Shape::Point { p: q, .. } => return js_hypot(p.x - q.x, p.y - q.y),
        Shape::Text {
            p: at,
            text,
            height,
            rotation,
        } => {
            // Anywhere on the text body counts as a hit.
            let b = text_box(*at, text, *height, *rotation, font);
            if point_in_polygon(p, &b) {
                return 0.0;
            }
            let mut d = js_hypot(p.x - at.x, p.y - at.y);
            for i in 0..4 {
                let side = Edge::Seg {
                    a: b[i],
                    b: b[(i + 1) % 4],
                };
                d = js_min(d, closest_on_edge(&side, p).d);
            }
            return d;
        }
        _ => {}
    }
    let mut d = f64::INFINITY;
    if let Shape::Dimension { height, .. } = e
        && let Some(l) = dimension_geom(e).and_then(|g| layout_dimension(&g))
    {
        // The value text is part of the dimension: clicking it selects the dimension.
        d = js_max(0.0, js_hypot(p.x - l.text_at.x, p.y - l.text_at.y) - height);
    }
    for ed in entity_edges(e) {
        d = js_min(d, closest_on_edge(&ed, p).d);
    }
    d
}

fn rect_corner(r: &Bounds, i: usize) -> Vec2 {
    let k = i % 4;
    Vec2::new(
        if k == 1 || k == 2 { r.max_x } else { r.min_x },
        if k >= 2 { r.max_y } else { r.min_y },
    )
}

/// Whether an object touches a box: a point of its outline inside, the box's
/// centre inside its area, or an edge (a hole's included) crossing the box.
pub fn touches_rect(e: &Shape, r: &Bounds) -> bool {
    let pts = entity_outline(e, 32.0);
    let in_r = |q: &Vec2| q.x >= r.min_x && q.x <= r.max_x && q.y >= r.min_y && q.y <= r.max_y;
    if pts.iter().any(in_r) {
        return true;
    }
    let centre = Vec2::new((r.min_x + r.max_x) / 2.0, (r.min_y + r.max_y) / 2.0);
    let inside = match e {
        Shape::Polygon { .. } => inside_polygon(e, centre),
        Shape::Hatch { ring, holes, .. } => {
            point_in_polygon(centre, ring)
                && !holes.iter().flatten().any(|h| point_in_polygon(centre, h))
        }
        _ => false,
    };
    if inside {
        return true;
    }
    // A window crossing a hole's boundary touches the polygon.
    for h in polygon_holes(e) {
        for i in 0..h.len() {
            for j in 0..4 {
                if seg_seg(
                    h[i],
                    h[(i + 1) % h.len()],
                    rect_corner(r, j),
                    rect_corner(r, j + 1),
                    1e-9,
                )
                .is_some()
                {
                    return true;
                }
            }
        }
    }
    let n = if is_closed_outline(e) {
        pts.len()
    } else {
        pts.len().saturating_sub(1)
    };
    for i in 0..n {
        for j in 0..4 {
            if seg_seg(
                pts[i],
                pts[(i + 1) % pts.len()],
                rect_corner(r, j),
                rect_corner(r, j + 1),
                1e-9,
            )
            .is_some()
            {
                return true;
            }
        }
    }
    false
}
