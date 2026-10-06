//! Selection beyond window and crossing (docs/adr/0141):
//!
//! - the closed shapes around a point, smallest first (İçeren alanı seç);
//! - the areas with a hole around a point, the innermost hole first
//!   (Deliği sil, Deliği doldur; docs/adr/0173 §5);
//! - what a fence crosses (Çitle seç);
//! - what a circle holds or touches (Daireyle seç);
//! - the objects lying far from the rest of the drawing (Kapsam denetimi).
//!
//! As with the window, only visible objects count. Answers keep the
//! document's order unless said otherwise.

use super::Store;
use super::pick::edge_distance;
use crate::entity::{
    Shape, TextPlace, area_parts, ellipse_geom, entity_area, entity_outline, inside_polygon,
    is_multi_part,
};
use crate::geom::ellipse::{ellipse_area, inside_ellipse, is_full_ellipse};
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges};
use crate::geom::leader;
use crate::geom::region::ring_area;
use crate::geometry::{Bounds, point_in_polygon, signed_area};
use crate::jsmath::{PI, js_cmp, js_hypot, js_max, js_min, stable_sort};
use crate::ops::edges::{entity_edges, entity_edges_in};
use crate::ops::holes::{HoleAt, hole_at, hole_ring};
use crate::text::Font;
use crate::vec2::Vec2;

/// Kapsam denetimi's reach, in sides of the drawing's core box (the box
/// of the middle half of the objects' centres, per axis).
pub const EXTENT_FACTOR: f64 = 10.0;
/// The core box's smallest side (m), so that a drawing of a few close
/// objects still has a reach.
pub const EXTENT_MIN_SIDE: f64 = 10.0;

fn infinite(s: &Shape) -> bool {
    matches!(s, Shape::Xline { .. } | Shape::Ray { .. })
}

impl Store {
    /// Visible closed shapes containing `p`, with their areas, smallest
    /// first: a polygon less its holes, a circle, a full ellipse, a closed
    /// spline. Equal areas keep the document's order. For a parcel inside a
    /// block inside a district the answer is parcel, block, district.
    pub fn containing(&self, p: Vec2) -> Vec<(f64, f64)> {
        let mut out: Vec<(f64, f64)> = Vec::new();
        for it in self.near(p, 0.0) {
            let e = &it.shape;
            let area = match e {
                Shape::Polygon { .. } if inside_polygon(e, p) => entity_area(e),
                Shape::Circle { c, r } if js_hypot(p.x - c.x, p.y - c.y) <= *r => Some(PI * r * r),
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
                Shape::Spline { closed: true, .. } => {
                    let mut ring = entity_outline(e, 72.0);
                    ring.pop();
                    point_in_polygon(p, &ring).then(|| signed_area(&ring).abs())
                }
                _ => None,
            };
            if let Some(a) = area {
                out.push((it.id, a));
            }
        }
        stable_sort(&mut out, &mut |a, b| js_cmp(a.1, b.1));
        out
    }

    /// Visible areas with a hole around `p`, each with the hole's part and
    /// place, the smallest hole first (for an area filling another's hole,
    /// its own hole before the other's). Equal holes keep the document's order.
    pub fn holes_at(&self, p: Vec2) -> Vec<(f64, HoleAt)> {
        let mut out: Vec<(f64, HoleAt, f64)> = Vec::new();
        for it in self.near(p, 0.0) {
            let (Some(at), Ok(ring)) = (hole_at(&it.shape, p), hole_ring(&it.shape, p)) else {
                continue;
            };
            out.push((it.id, at, ring_area(&ring).abs()));
        }
        stable_sort(&mut out, &mut |a, b| js_cmp(a.2, b.2));
        out.into_iter().map(|(id, at, _)| (id, at)).collect()
    }

    /// Visible objects the fence (an open path) crosses: one of its edges,
    /// a text's body, or a point within `tol` of the fence.
    pub fn in_fence(&self, fence: &[Vec2], tol: f64) -> Vec<f64> {
        if fence.len() < 2 {
            return Vec::new();
        }
        let mut q = Bounds {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        for p in fence {
            q.min_x = js_min(q.min_x, p.x);
            q.min_y = js_min(q.min_y, p.y);
            q.max_x = js_max(q.max_x, p.x);
            q.max_y = js_max(q.max_y, p.y);
        }
        let segs: Vec<Edge> = fence
            .windows(2)
            .map(|w| Edge::Seg { a: w[0], b: w[1] })
            .collect();
        self.candidates(&super::padded(q, tol))
            .into_iter()
            .filter(|it| {
                self.flags(it).visible
                    && it
                        .shapes()
                        .any(|s| crosses(s, fence, &segs, tol, self.font))
            })
            .map(|it| it.id)
            .collect()
    }

    /// Visible objects wholly inside the circle; with `crossing` also those
    /// it touches: an edge within it, or its centre inside a closed shape.
    /// Infinite lines are never inside, only touched.
    pub fn in_circle(&self, c: Vec2, r: f64, crossing: bool) -> Vec<f64> {
        if !(r > 0.0) {
            return Vec::new();
        }
        let q = Bounds {
            min_x: c.x - r,
            min_y: c.y - r,
            max_x: c.x + r,
            max_y: c.y + r,
        };
        let within = |p: &Vec2| js_hypot(p.x - c.x, p.y - c.y) <= r;
        let mut out = Vec::new();
        for it in self.candidates(&super::padded(q, 0.0)) {
            if !self.flags(it).visible {
                continue;
            }
            // An insert by its pieces (docs/adr/0144).
            let mut shapes = it.shapes();
            let inside = !shapes.clone().any(infinite) && {
                let b = &it.bounds;
                let corners = [
                    Vec2::new(b.min_x, b.min_y),
                    Vec2::new(b.max_x, b.min_y),
                    Vec2::new(b.max_x, b.max_y),
                    Vec2::new(b.min_x, b.max_y),
                ];
                corners.iter().all(within)
                    || shapes
                        .clone()
                        .all(|e| outline(e, self.font).iter().all(within))
            };
            if inside || (crossing && shapes.any(|e| touches_circle(e, c, r, self.font))) {
                out.push(it.id);
            }
        }
        out
    }

    /// Kapsam denetimi: visible objects lying far from the rest of the
    /// drawing (brought in with a wrong coordinate system, fallen to zero).
    /// The middle half of the objects' centres, per axis, is the drawing's
    /// core box; an object whose own box lies wholly beyond
    /// [`EXTENT_FACTOR`] times the core's longer side (at least
    /// [`EXTENT_MIN_SIDE`]) from it is far. Up to a quarter of the objects
    /// can be far and still be found. Fewer than four objects have no core;
    /// infinite lines are left out.
    pub fn extent_outliers(&self) -> Vec<f64> {
        let items: Vec<_> = self
            .all_items()
            .into_iter()
            .filter(|it| self.flags(it).visible && !infinite(&it.shape))
            .collect();
        if items.len() < 4 {
            return Vec::new();
        }
        let mut xs: Vec<f64> = items
            .iter()
            .map(|it| (it.bounds.min_x + it.bounds.max_x) / 2.0)
            .collect();
        let mut ys: Vec<f64> = items
            .iter()
            .map(|it| (it.bounds.min_y + it.bounds.max_y) / 2.0)
            .collect();
        xs.sort_by(f64::total_cmp);
        ys.sort_by(f64::total_cmp);
        // Nearest rank, halves up: the quartiles are objects' own centres.
        let quartile = |v: &[f64], k: usize| v[((v.len() - 1) * k + 2) / 4];
        let (x0, x1) = (quartile(&xs, 1), quartile(&xs, 3));
        let (y0, y1) = (quartile(&ys, 1), quartile(&ys, 3));
        let reach = EXTENT_FACTOR * js_max(js_max(x1 - x0, y1 - y0), EXTENT_MIN_SIDE);
        items
            .into_iter()
            .filter(|it| {
                let b = &it.bounds;
                b.max_x < x0 - reach
                    || b.min_x > x1 + reach
                    || b.max_y < y0 - reach
                    || b.min_y > y1 + reach
            })
            .map(|it| it.id)
            .collect()
    }
}

/// What an object shows of itself for "wholly inside": its outline, a
/// text's body.
fn outline(e: &Shape, font: Font) -> Vec<Vec2> {
    match e {
        Shape::Text { .. } => TextPlace::of(e)
            .map(|t| t.outline(font))
            .unwrap_or_default(),
        // A leader's line, landing, arrowhead and note's body (docs/adr/0146 §4).
        Shape::Leader { .. } => {
            let mut out = entity_outline(e, 64.0);
            if let Some(l) = leader::layout_of(e) {
                out.extend(leader::head_reach(&l.head));
            }
            out.extend(
                leader::note_place(e)
                    .map(|t| t.outline(font))
                    .unwrap_or_default(),
            );
            out
        }
        // Every part of a multi-part area: all of them must be inside (docs/adr/0143).
        _ if is_multi_part(e) => area_parts(e)
            .iter()
            .flat_map(|part| entity_outline(part, 64.0))
            .collect(),
        _ => entity_outline(e, 64.0),
    }
}

/// Whether the fence crosses the object: an edge, a text's body (crossed or
/// holding a fence point), a point within `tol`.
fn crosses(e: &Shape, fence: &[Vec2], segs: &[Edge], tol: f64, font: Font) -> bool {
    // A multi-part object when the fence crosses one of its parts (docs/adr/0143, 0174).
    if is_multi_part(e) {
        return area_parts(e)
            .iter()
            .any(|part| crosses(part, fence, segs, tol, font));
    }
    match e {
        Shape::Point { p, .. } | Shape::Insert { p, .. } => {
            segs.iter().any(|s| closest_on_edge(s, *p).d <= tol)
        }
        // Its body, as a text's: the fence crosses its outline or holds a point of it (docs/adr/0184 §2).
        Shape::Table { .. } => {
            let body = crate::entity::entity_vertices(e);
            fence.iter().any(|q| point_in_polygon(*q, &body))
                || segs.iter().any(|s| {
                    (0..body.len()).any(|i| {
                        let side = Edge::Seg {
                            a: body[i],
                            b: body[(i + 1) % body.len()],
                        };
                        !intersect_edges(s, &side).is_empty()
                    })
                })
        }
        Shape::Text { .. } | Shape::Leader { .. } => {
            let body = TextPlace::of(e)
                .or_else(|| leader::note_place(e))
                .map(|t| t.outline(font))
                .unwrap_or_default();
            // A leader also by its line and landing (docs/adr/0146 §4).
            if matches!(e, Shape::Leader { .. }) {
                let edges = entity_edges(e);
                if segs
                    .iter()
                    .any(|s| edges.iter().any(|ed| !intersect_edges(s, ed).is_empty()))
                {
                    return true;
                }
            }
            fence.iter().any(|q| point_in_polygon(*q, &body))
                || segs.iter().any(|s| {
                    (0..body.len()).any(|i| {
                        let side = Edge::Seg {
                            a: body[i],
                            b: body[(i + 1) % body.len()],
                        };
                        !intersect_edges(s, &side).is_empty()
                    })
                })
        }
        _ => {
            // A curve is split only where the fence can meet it (docs/adr/0149 §5.3).
            let mut area = crate::geometry::empty_bounds();
            for s in segs {
                if let Edge::Seg { a, b } = s {
                    for q in [a, b] {
                        area.min_x = js_min(area.min_x, q.x);
                        area.min_y = js_min(area.min_y, q.y);
                        area.max_x = js_max(area.max_x, q.x);
                        area.max_y = js_max(area.max_y, q.y);
                    }
                }
            }
            let edges = if segs.iter().all(|s| matches!(s, Edge::Seg { .. })) {
                entity_edges_in(e, &area)
            } else {
                entity_edges(e)
            };
            segs.iter()
                .any(|s| edges.iter().any(|ed| !intersect_edges(s, ed).is_empty()))
        }
    }
}

/// Whether a circle touches the object: an edge (a text's body, a point)
/// within it, or its centre inside a closed shape.
fn touches_circle(e: &Shape, c: Vec2, r: f64, font: Font) -> bool {
    if edge_distance(e, c, font) <= r {
        return true;
    }
    match e {
        Shape::Polygon { .. } => inside_polygon(e, c),
        Shape::Circle { c: o, r: ro } => js_hypot(c.x - o.x, c.y - o.y) <= *ro,
        Shape::Hatch { ring, holes, .. } => {
            point_in_polygon(c, ring) && !holes.iter().flatten().any(|h| point_in_polygon(c, h))
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(id: u32, x: f64, y: f64, side: f64) -> String {
        format!(
            r#"{{"id":{id},"layerId":"a","attrs":{{}},"kind":"polygon","pts":[{{"x":{x},"y":{y}}},{{"x":{},"y":{y}}},{{"x":{},"y":{}}},{{"x":{x},"y":{}}}]}}"#,
            x + side,
            x + side,
            y + side,
            y + side
        )
    }

    fn point(id: u32, x: f64, y: f64) -> String {
        format!(
            r#"{{"id":{id},"layerId":"a","attrs":{{}},"kind":"point","p":{{"x":{x},"y":{y}}}}}"#
        )
    }

    fn line(id: u32, a: (f64, f64), b: (f64, f64)) -> String {
        format!(
            r#"{{"id":{id},"layerId":"a","attrs":{{}},"kind":"line","a":{{"x":{},"y":{}}},"b":{{"x":{},"y":{}}}}}"#,
            a.0, a.1, b.0, b.1
        )
    }

    fn store(items: &[String]) -> Store {
        let mut s = Store::new();
        s.put_json(&format!("[{}]", items.join(","))).unwrap();
        s
    }

    #[test]
    fn the_shapes_around_a_point_come_smallest_first() {
        // District 100 m, block 40 m, parcel 10 m, all around (5, 5); a
        // square beside them and a line through the point do not count.
        let s = store(&[
            square(1, -50.0, -50.0, 100.0),
            square(2, 0.0, 0.0, 10.0),
            square(3, -10.0, -10.0, 40.0),
            square(4, 20.0, 20.0, 5.0),
            line(5, (0.0, 5.0), (10.0, 5.0)),
        ]);
        let got = s.containing(Vec2::new(5.0, 5.0));
        assert_eq!(got, vec![(2.0, 100.0), (3.0, 1600.0), (1.0, 10_000.0)]);
        assert!(s.containing(Vec2::new(500.0, 500.0)).is_empty());
    }

    #[test]
    fn a_hole_is_outside_its_polygon() {
        let s = store(&[
            r#"{"id":1,"layerId":"a","attrs":{},"kind":"polygon","pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}],"holes":[{"pts":[{"x":4,"y":4},{"x":6,"y":4},{"x":6,"y":6},{"x":4,"y":6}]}]}"#.to_owned(),
            square(2, 4.0, 4.0, 2.0),
        ]);
        // In the hole only the building (the hole's filling) holds the point.
        assert_eq!(s.containing(Vec2::new(5.0, 5.0)), vec![(2.0, 4.0)]);
        // Net area: 100 − 4.
        assert_eq!(s.containing(Vec2::new(1.0, 1.0)), vec![(1.0, 96.0)]);
    }

    #[test]
    fn the_holes_around_a_point_come_smallest_first() {
        // A district with a 40 m hole; in it a block with a 10 m hole and a
        // second part with a 2 m one; the point (5, 5) is in both big holes.
        let s = store(&[
            r#"{"id":1,"layerId":"a","attrs":{},"kind":"polygon","pts":[{"x":-50,"y":-50},{"x":50,"y":-50},{"x":50,"y":50},{"x":-50,"y":50}],"holes":[{"pts":[{"x":-10,"y":-10},{"x":30,"y":-10},{"x":30,"y":30},{"x":-10,"y":30}]}]}"#.to_owned(),
            r#"{"id":2,"layerId":"a","attrs":{},"kind":"polygon","pts":[{"x":-5,"y":-5},{"x":20,"y":-5},{"x":20,"y":20},{"x":-5,"y":20}],"holes":[{"pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}]}],"parts":[{"pts":[{"x":60,"y":0},{"x":70,"y":0},{"x":70,"y":10},{"x":60,"y":10}],"holes":[{"pts":[{"x":64,"y":4},{"x":66,"y":4},{"x":66,"y":6},{"x":64,"y":6}]}]}]}"#.to_owned(),
        ]);
        let at = |part, hole| HoleAt { part, hole };
        assert_eq!(
            s.holes_at(Vec2::new(5.0, 5.0)),
            vec![(2.0, at(0, 0)), (1.0, at(0, 0))]
        );
        // The second part's hole; the filled block holds no hole there.
        assert_eq!(s.holes_at(Vec2::new(65.0, 5.0)), vec![(2.0, at(1, 0))]);
        assert!(
            s.holes_at(Vec2::new(15.0, 15.0))
                .iter()
                .all(|(id, _)| *id == 1.0)
        );
        assert!(s.holes_at(Vec2::new(-30.0, -30.0)).is_empty());
    }

    #[test]
    fn a_fence_takes_what_it_crosses() {
        let s = store(&[
            line(1, (0.0, 0.0), (0.0, 10.0)),
            line(2, (5.0, 0.0), (5.0, 10.0)),
            line(3, (20.0, 0.0), (20.0, 10.0)),
            square(4, 8.0, 4.0, 2.0),
            point(5, 12.0, 5.05),
            point(6, 12.0, 7.0),
        ]);
        // A fence along y = 5 from x = −1 to 14: the two lines, the square's
        // edges, the point 5 cm off with a 10 cm tolerance; not the far line
        // nor the point 2 m away.
        let fence = [Vec2::new(-1.0, 5.0), Vec2::new(14.0, 5.0)];
        assert_eq!(s.in_fence(&fence, 0.1), vec![1.0, 2.0, 4.0, 5.0]);
        // A fence wholly inside the square crosses none of its edges.
        let inner = [Vec2::new(8.5, 5.0), Vec2::new(9.5, 5.0)];
        assert!(s.in_fence(&inner, 0.0).is_empty());
        assert!(s.in_fence(&fence[..1], 0.1).is_empty());
    }

    #[test]
    fn a_circle_holds_what_is_inside_and_touches_what_it_reaches() {
        let s = store(&[
            square(1, -1.0, -1.0, 2.0),
            line(2, (0.0, 0.0), (20.0, 0.0)),
            point(3, 3.0, 0.0),
            square(4, -100.0, -100.0, 200.0),
            point(5, 30.0, 30.0),
        ]);
        let c = Vec2::new(0.0, 0.0);
        // Radius 5: the 2 m square (corners √2 away) and the point 3 m away
        // are inside; the long line only crosses; the big square holds the
        // circle.
        assert_eq!(s.in_circle(c, 5.0, false), vec![1.0, 3.0]);
        assert_eq!(s.in_circle(c, 5.0, true), vec![1.0, 2.0, 3.0, 4.0]);
        assert!(s.in_circle(c, 0.0, true).is_empty());
    }

    #[test]
    fn far_objects_are_found_and_the_drawing_itself_is_not() {
        // A 10 × 10 block of 100 parcels 20 m apart at TM coordinates, one
        // parcel fallen to zero and a point 30 km away.
        let mut items: Vec<String> = (0..100)
            .map(|i| {
                square(
                    i + 1,
                    500_000.0 + f64::from(i % 10) * 20.0,
                    4_400_000.0 + f64::from(i / 10) * 20.0,
                    15.0,
                )
            })
            .collect();
        items.push(square(200, 0.0, 0.0, 15.0));
        items.push(point(201, 530_000.0, 4_400_000.0));
        // A suburb 1 km away is part of the drawing.
        items.push(square(202, 501_000.0, 4_400_000.0, 15.0));
        let s = store(&items);
        assert_eq!(s.extent_outliers(), vec![200.0, 201.0]);
        // Two towns 50 km apart, half the objects each: neither is far.
        let towns: Vec<String> = (0..20)
            .map(|i| {
                let x = if i < 10 { 500_000.0 } else { 550_000.0 };
                square(i + 1, x + f64::from(i % 10) * 20.0, 4_400_000.0, 15.0)
            })
            .collect();
        assert!(store(&towns).extent_outliers().is_empty());
        // Three objects have no core.
        assert!(store(&items[..3]).extent_outliers().is_empty());
    }
}
