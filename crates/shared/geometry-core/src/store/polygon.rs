//! Çokgenle seç (docs/adr/0187 §2): what a drawn polygon holds, touches or
//! leaves out. The window's and the circle's rule (docs/adr/0029, 0141) for
//! any simple polygon: an object touches the polygon when an edge of it
//! meets the boundary, a point of it lies inside, or the polygon lies in its
//! area; an edge is inside when every piece of it between its meetings with
//! the boundary has its middle inside or on the boundary.

use super::{Item, Store, padded};
use crate::api::Op;
use crate::entity::{Shape, TextPlace, area_parts, entity_vertices, inside_polygon, is_multi_part};
use crate::geom::intersect::{Edge, closest_on_edge, intersect_edges, point_at, seg_seg};
use crate::geom::leader;
use crate::geometry::{Bounds, point_in_polygon, signed_area};
use crate::jsmath::{js_hypot, js_max, js_min};
use crate::op;
use crate::ops::edges::entity_edges;
use crate::predicates::orientation;
use crate::text::Font;
use crate::vec2::Vec2;

/// How a polygon selects: what lies wholly inside it, what it touches too,
/// or every visible object it does not touch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolygonMode {
    Inside,
    Crossing,
    Outside,
}

impl PolygonMode {
    /// The mode by its number on the WASM boundary: 0 inside, 1 crossing, 2 outside.
    pub fn of(n: u8) -> Self {
        match n {
            1 => PolygonMode::Crossing,
            2 => PolygonMode::Outside,
            _ => PolygonMode::Inside,
        }
    }
}

/// How near the boundary a point is on it, metres: far above the rounding
/// of a meeting point at map coordinates, far below anything drawn.
pub const ON_BOUNDARY: f64 = 1e-6;

/// Why a ring cannot select, in the words the tools say; none when it can:
/// fewer than three corners, no area, or a ring that crosses itself (an
/// edge meeting another that does not follow it, or turning back on the
/// one before).
pub fn ring_problem(ring: &[Vec2]) -> Option<&'static str> {
    let n = ring.len();
    if n < 3 {
        return Some("Çokgen için en az üç köşe gerekir; sonraki köşeyi gösterin.");
    }
    // Every corner on one line (exactly): no area, whatever else it does.
    let first = ring[0];
    if let Some(&other) = ring.iter().find(|p| **p != first)
        && ring.iter().all(|p| orientation(first, other, *p) == 0)
    {
        return Some(NO_AREA);
    }
    let side = |i: usize| (ring[i], ring[(i + 1) % n]);
    for i in 0..n {
        let (a, b) = side(i);
        let (_, c) = side((i + 1) % n);
        // An edge turning straight back on the one before overlaps it.
        let cross = (b.x - a.x) * (c.y - b.y) - (b.y - a.y) * (c.x - b.x);
        let dot = (b.x - a.x) * (c.x - b.x) + (b.y - a.y) * (c.y - b.y);
        if cross == 0.0 && dot < 0.0 {
            return Some(CROSSES_ITSELF);
        }
        for j in i + 2..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            let (c, d) = side(j);
            if seg_seg(a, b, c, d, 1e-9).is_some() {
                return Some(CROSSES_ITSELF);
            }
        }
    }
    let scale = ring
        .iter()
        .fold(1.0, |m, p| js_max(m, js_max(p.x.abs(), p.y.abs())));
    if !(signed_area(ring).abs() > 1e-12 * scale) {
        return Some(NO_AREA);
    }
    None
}

const NO_AREA: &str = "Çokgenin alanı yok; köşeleri bir doğru üzerinde olmayan noktalara verin.";
const CROSSES_ITSELF: &str =
    "Çokgen kendini kesiyor; son köşeyi geri alın (G) ya da kesmeyen bir köşe gösterin.";

/// A ring and its sides, as the tests ask them.
struct Ring<'a> {
    pts: &'a [Vec2],
    sides: Vec<Edge>,
}

impl Ring<'_> {
    fn new(pts: &[Vec2]) -> Ring<'_> {
        let n = pts.len();
        let sides = (0..n)
            .map(|i| Edge::Seg {
                a: pts[i],
                b: pts[(i + 1) % n],
            })
            .collect();
        Ring { pts, sides }
    }

    /// Inside, or on the boundary.
    fn holds(&self, p: Vec2) -> bool {
        point_in_polygon(p, self.pts)
            || self
                .sides
                .iter()
                .any(|s| closest_on_edge(s, p).d <= ON_BOUNDARY)
    }

    /// Where an edge meets the boundary, as parameters along it: crossings,
    /// touches, and the ring's corners lying on it (an edge along a side).
    fn meetings(&self, e: &Edge) -> Vec<f64> {
        let mut ts: Vec<f64> = self
            .sides
            .iter()
            .flat_map(|s| intersect_edges(e, s))
            .map(|h| h.t)
            .collect();
        for v in self.pts {
            let c = closest_on_edge(e, *v);
            if c.d <= ON_BOUNDARY {
                ts.push(c.t);
            }
        }
        ts
    }

    /// Whether the edge lies wholly inside or on the boundary.
    fn holds_edge(&self, e: &Edge) -> bool {
        if !self.holds(point_at(e, 0.0)) || !self.holds(point_at(e, 1.0)) {
            return false;
        }
        let mut ts = self.meetings(e);
        ts.push(0.0);
        ts.push(1.0);
        ts.retain(|t| t.is_finite());
        ts.sort_by(f64::total_cmp);
        ts.windows(2)
            .filter(|w| w[1] - w[0] > 1e-12)
            .all(|w| self.holds(point_at(e, (w[0] + w[1]) / 2.0)))
    }

    /// Whether the edge meets the boundary anywhere.
    fn meets(&self, e: &Edge) -> bool {
        !self.meetings(e).is_empty()
    }
}

/// The closed body a text, a table or a leader's note shows, by its corners.
fn body(e: &Shape, font: Font) -> Option<Vec<Vec2>> {
    match e {
        Shape::Text { .. } => TextPlace::of(e).map(|t| t.outline(font)),
        Shape::Table { .. } => Some(entity_vertices(e)).filter(|b| b.len() > 2),
        Shape::Leader { .. } => leader::note_place(e).map(|t| t.outline(font)),
        _ => None,
    }
}

fn closed(pts: &[Vec2]) -> Vec<Edge> {
    let n = pts.len();
    (0..n)
        .map(|i| Edge::Seg {
            a: pts[i],
            b: pts[(i + 1) % n],
        })
        .collect()
}

/// The edges and lone points an object is made of for the polygon: a
/// text's and a table's body by its sides, a leader's line and landing,
/// its note's body and its arrowhead's reach, a point and an unexpanded
/// insert by their place.
fn parts(e: &Shape, font: Font) -> (Vec<Edge>, Vec<Vec2>) {
    match e {
        Shape::Point { p, .. } | Shape::Insert { p, .. } => (Vec::new(), vec![*p]),
        Shape::Text { .. } | Shape::Table { .. } => (
            body(e, font).map(|b| closed(&b)).unwrap_or_default(),
            Vec::new(),
        ),
        Shape::Leader { .. } => {
            let mut edges = entity_edges(e);
            edges.extend(body(e, font).map(|b| closed(&b)).unwrap_or_default());
            let head = leader::layout_of(e)
                .map(|l| leader::head_reach(&l.head))
                .unwrap_or_default();
            (edges, head)
        }
        _ => {
            let edges = entity_edges(e);
            // A shape with no edges of its own (a degenerate one) by its corners.
            let lone = if edges.is_empty() {
                entity_vertices(e)
            } else {
                Vec::new()
            };
            (edges, lone)
        }
    }
}

/// Whether the polygon lies in the object's area (not in a hole): the
/// window's and the circle's rule for closed shapes and hatches.
fn holds_polygon(e: &Shape, at: Vec2, font: Font) -> bool {
    match e {
        Shape::Polygon { .. } => inside_polygon(e, at),
        Shape::Circle { c, r } => js_hypot(at.x - c.x, at.y - c.y) <= *r,
        Shape::Hatch { ring, holes, .. } => {
            point_in_polygon(at, ring) && !holes.iter().flatten().any(|h| point_in_polygon(at, h))
        }
        Shape::Text { .. } | Shape::Table { .. } | Shape::Leader { .. } => {
            body(e, font).is_some_and(|b| point_in_polygon(at, &b))
        }
        _ => false,
    }
}

fn infinite(s: &Shape) -> bool {
    matches!(s, Shape::Xline { .. } | Shape::Ray { .. })
}

/// Whether a shape lies wholly inside the ring (a multi-part one by all its parts).
fn inside(e: &Shape, ring: &Ring<'_>, font: Font) -> bool {
    if infinite(e) {
        return false;
    }
    if is_multi_part(e) {
        return area_parts(e).iter().all(|part| inside(part, ring, font));
    }
    let (edges, lone) = parts(e, font);
    if edges.is_empty() && lone.is_empty() {
        return false;
    }
    lone.iter().all(|p| ring.holds(*p)) && edges.iter().all(|ed| ring.holds_edge(ed))
}

/// Whether a shape touches the ring: an edge meets its boundary, a point of
/// it is inside, or the ring lies in its area.
fn touches(e: &Shape, ring: &Ring<'_>, font: Font) -> bool {
    if is_multi_part(e) {
        return area_parts(e).iter().any(|part| touches(part, ring, font));
    }
    let (edges, lone) = parts(e, font);
    lone.iter().any(|p| ring.holds(*p))
        || edges
            .iter()
            .any(|ed| ring.meets(ed) || ring.holds(point_at(ed, 0.0)))
        || holds_polygon(e, ring.pts[0], font)
}

impl Store {
    /// Çokgenle seç (docs/adr/0187 §2): the visible objects wholly inside a
    /// simple ring, touching it too (`Crossing`), or not touching it at all
    /// (`Outside`), in the document's order. An insert by its pieces, all of
    /// them inside or one of them touching. A ring [`ring_problem`] refuses
    /// selects nothing.
    pub fn in_polygon(&self, pts: &[Vec2], mode: PolygonMode) -> Vec<f64> {
        if ring_problem(pts).is_some() {
            return Vec::new();
        }
        let ring = Ring::new(pts);
        let mut q = Bounds {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        };
        for p in pts {
            q.min_x = js_min(q.min_x, p.x);
            q.min_y = js_min(q.min_y, p.y);
            q.max_x = js_max(q.max_x, p.x);
            q.max_y = js_max(q.max_y, p.y);
        }
        let q = padded(q, ON_BOUNDARY);
        let touching = |it: &Item| it.shapes().any(|s| touches(s, &ring, self.font));
        match mode {
            PolygonMode::Inside => self
                .candidates(&q)
                .into_iter()
                .filter(|it| {
                    self.flags(it).visible && it.shapes().all(|s| inside(s, &ring, self.font))
                })
                .map(|it| it.id)
                .collect(),
            PolygonMode::Crossing => self
                .candidates(&q)
                .into_iter()
                .filter(|it| self.flags(it).visible && touching(it))
                .map(|it| it.id)
                .collect(),
            PolygonMode::Outside => self
                .all_items()
                .into_iter()
                .filter(|it| {
                    if !self.flags(it).visible {
                        return false;
                    }
                    let b = &it.bounds;
                    let apart = !it.shapes().any(infinite)
                        && (b.max_x < q.min_x
                            || b.min_x > q.max_x
                            || b.max_y < q.min_y
                            || b.min_y > q.max_y);
                    apart || !touching(it)
                })
                .map(|it| it.id)
                .collect(),
        }
    }
}

pub(crate) static OPS: &[Op] = &[op!("selectionRingProblem", |ring: Vec<Vec2>| {
    ring_problem(&ring).map(str::to_owned)
})];

#[cfg(test)]
mod tests {
    use super::*;

    fn store(items: &[String]) -> Store {
        let mut s = Store::new();
        s.put_json(&format!("[{}]", items.join(","))).unwrap();
        s
    }

    fn line(id: u32, a: (f64, f64), b: (f64, f64)) -> String {
        format!(
            r#"{{"id":{id},"layerId":"a","attrs":{{}},"kind":"line","a":{{"x":{},"y":{}}},"b":{{"x":{},"y":{}}}}}"#,
            a.0, a.1, b.0, b.1
        )
    }

    fn point(id: u32, x: f64, y: f64) -> String {
        format!(
            r#"{{"id":{id},"layerId":"a","attrs":{{}},"kind":"point","p":{{"x":{x},"y":{y}}}}}"#
        )
    }

    fn square(id: u32, x: f64, y: f64, side: f64) -> String {
        format!(
            r#"{{"id":{id},"layerId":"a","attrs":{{}},"kind":"polygon","pts":[{{"x":{x},"y":{y}}},{{"x":{},"y":{y}}},{{"x":{},"y":{}}},{{"x":{x},"y":{}}}]}}"#,
            x + side,
            x + side,
            y + side,
            y + side
        )
    }

    fn pts(list: &[(f64, f64)]) -> Vec<Vec2> {
        list.iter().map(|&(x, y)| Vec2::new(x, y)).collect()
    }

    /// A U open at the top: (0,0) (30,0) (30,20) (20,20) (20,10) (10,10) (10,20) (0,20).
    fn u_shape() -> Vec<Vec2> {
        pts(&[
            (0.0, 0.0),
            (30.0, 0.0),
            (30.0, 20.0),
            (20.0, 20.0),
            (20.0, 10.0),
            (10.0, 10.0),
            (10.0, 20.0),
            (0.0, 20.0),
        ])
    }

    #[test]
    fn a_concave_polygon_holds_what_its_box_would_not() {
        let s = store(&[
            // Both ends in the U's arms, the middle across its notch: outside it.
            line(1, (5.0, 15.0), (25.0, 15.0)),
            // In the left arm.
            line(2, (2.0, 2.0), (8.0, 18.0)),
            // In the notch, above the U's bottom: touches nothing.
            point(3, 15.0, 15.0),
            // Along the bottom side: on the boundary, inside.
            line(4, (0.0, 0.0), (30.0, 0.0)),
            // Across the right side.
            line(5, (25.0, 5.0), (35.0, 5.0)),
            // A square around the whole U: the U lies in its area.
            square(6, -10.0, -10.0, 50.0),
            point(7, 100.0, 100.0),
        ]);
        let ring = u_shape();
        assert_eq!(s.in_polygon(&ring, PolygonMode::Inside), vec![2.0, 4.0]);
        assert_eq!(
            s.in_polygon(&ring, PolygonMode::Crossing),
            vec![1.0, 2.0, 4.0, 5.0, 6.0]
        );
        assert_eq!(s.in_polygon(&ring, PolygonMode::Outside), vec![3.0, 7.0]);
    }

    #[test]
    fn a_ring_that_cannot_select_is_said() {
        assert!(ring_problem(&pts(&[(0.0, 0.0), (1.0, 0.0)])).is_some());
        assert_eq!(
            ring_problem(&pts(&[(0.0, 0.0), (1.0, 0.0), (2.0, 0.0)])),
            Some(NO_AREA)
        );
        // A bow tie crosses itself.
        let bow = pts(&[(0.0, 0.0), (10.0, 10.0), (10.0, 0.0), (0.0, 10.0)]);
        assert_eq!(ring_problem(&bow), Some(CROSSES_ITSELF));
        // A spike turning back on itself.
        let spike = pts(&[(0.0, 0.0), (10.0, 0.0), (5.0, 0.0), (5.0, 5.0)]);
        assert_eq!(ring_problem(&spike), Some(CROSSES_ITSELF));
        assert_eq!(ring_problem(&u_shape()), None);
        let s = store(&[point(1, 5.0, 5.0)]);
        assert!(s.in_polygon(&bow, PolygonMode::Crossing).is_empty());
    }
}
