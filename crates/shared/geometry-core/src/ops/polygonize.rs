//! Toplu alan (docs/adr/0151): the regions line work closes, the label each
//! takes and what is reported. The regions are the faces of the shared face
//! index (Tarama's and İçine tıklayarak alan's), smallest first, with the
//! closed groups inside each as holes when `islands`; a label goes to the
//! smallest region whose outer ring holds it, unless it lies within 1 µm of
//! a ring (on a boundary); a region whose rings are an input area's is
//! marked with it, so it is not written again; and every free end of an
//! open path is listed (a region that does not close). The independent
//! reference is `scripts/fixtures/polygonize_cases.py`.

use crate::api::Op;
use crate::entity::{Entity, Shape, area_parts};
use crate::geom::arrangement::{Area, Ring};
use crate::geom::intersect::{Edge, closest_on_edge, point_at};
use crate::geom::region::FaceIndex;
use crate::geometry::Bounds;
use crate::jsmath::{js_hypot, js_max, js_min};
use crate::op;
use crate::ops::areas::line_source;
use crate::ops::edges::entity_edges;
use crate::store::rtree::{PackedTree, overlaps};
use crate::vec2::Vec2;

/// “The same place”, “on” and “touching”, metres: the elevation rules' (ADR 0142).
pub const TOUCH: f64 = 1e-6;

/// A label: where it is (a text's box middle, a point) and its value.
#[derive(Clone, Debug, PartialEq)]
pub struct PolyLabel {
    pub at: Vec2,
    pub value: String,
}

crate::json_struct!(PolyLabel { at, value });

/// A region: its area, the labels in it (their places in the input) and,
/// when its rings are an input area's, that area's place among the lines.
#[derive(Clone, Debug, PartialEq)]
pub struct PolyRegion {
    pub area: Area,
    pub labels: Vec<usize>,
    pub existing: Option<usize>,
}

crate::json_struct!(PolyRegion {
    area,
    labels,
    existing
});

#[derive(Clone, Debug, PartialEq)]
pub struct PolyResult {
    pub regions: Vec<PolyRegion>,
    /// The labels within 1 µm of a region's or a group's ring.
    pub on_boundary: Vec<usize>,
    /// The ends of open paths that touch nothing.
    pub free_ends: Vec<Vec2>,
}

crate::json_struct!(PolyResult {
    regions,
    on_boundary => "onBoundary",
    free_ends => "freeEnds"
});

/// The kinds that close regions (docs/adr/0151 §2): lines, polylines,
/// arcs, circles, ellipses, curves and areas; construction lines, leaders,
/// blocks, texts, dimensions, hatches and points do not.
fn bounds(shape: &Shape) -> bool {
    matches!(
        shape,
        Shape::Line { .. }
            | Shape::Polyline { .. }
            | Shape::Arc { .. }
            | Shape::Circle { .. }
            | Shape::Ellipse { .. }
            | Shape::Spline { .. }
            | Shape::Polygon { .. }
    )
}

/// Toplu alan's finding over `lines` (the line work, in the drawing's
/// order; other kinds are passed over) and `labels`.
pub fn polygonize(lines: &[Entity], labels: &[PolyLabel], islands: bool) -> PolyResult {
    let work: Vec<Entity> = lines.iter().filter(|e| bounds(&e.shape)).cloned().collect();
    let index = FaceIndex::new(&[line_source(&work)]);
    let mut regions: Vec<PolyRegion> = index
        .faces(islands)
        .into_iter()
        .map(|area| PolyRegion {
            area,
            labels: Vec::new(),
            existing: None,
        })
        .collect();
    let mut on_boundary = Vec::new();
    for (i, l) in labels.iter().enumerate() {
        if index.near_ring(l.at, TOUCH) {
            on_boundary.push(i);
        } else if let Some(k) = index.smallest_at(l.at) {
            regions[k].labels.push(i);
        }
    }
    let parts = existing_parts(lines);
    for region in &mut regions {
        let outer = turning(&region.area.outer);
        let Some(low) = lowest(&outer) else { continue };
        let holes: Vec<Vec<Corner>> = region.area.holes.iter().map(turning).collect();
        // The parts whose lowest corner is this one's (within 1 µm), then the same rings; the
        // first such area in the drawing's order.
        let from = parts.partition_point(|p| p.low.x < low.x - TOUCH);
        region.existing = parts[from..]
            .iter()
            .take_while(|p| p.low.x <= low.x + TOUCH)
            .filter(|p| {
                near(p.low, low) && same_ring(&p.outer, &outer) && same_holes(&p.holes, &holes)
            })
            .map(|p| p.area)
            .min();
    }
    PolyResult {
        regions,
        on_boundary,
        free_ends: free_ends(lines),
    }
}

/// A ring's corner: a vertex where the ring turns or meets an arc, and the
/// bulge of the edge it starts.
type Corner = (Vec2, f64);

/// A ring as its corners: a vertex that two straight edges pass straight
/// through, or that repeats the one before, is no corner.
fn turning(r: &Ring) -> Vec<Corner> {
    let n = r.pts.len();
    let bulge = |i: usize| {
        r.bulges
            .as_ref()
            .and_then(|b| b.get(i).copied())
            .unwrap_or(0.0)
    };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (p, q, s) = (r.pts[(i + n - 1) % n], r.pts[i], r.pts[(i + 1) % n]);
        if bulge((i + n - 1) % n) == 0.0 && bulge(i) == 0.0 {
            let (ux, uy, vx, vy) = (q.x - p.x, q.y - p.y, s.x - q.x, s.y - q.y);
            let l = js_hypot(ux, uy) * js_hypot(vx, vy);
            if l == 0.0 || ((ux * vy - uy * vx).abs() <= 1e-12 * l && ux * vx + uy * vy > 0.0) {
                continue;
            }
        }
        out.push((q, bulge(i)));
    }
    out
}

fn near(a: Vec2, b: Vec2) -> bool {
    js_hypot(a.x - b.x, a.y - b.y) <= TOUCH
}

/// Whether two rings have the same corners in the same order, whichever
/// corner each starts at and whichever way each runs (an edge's bulge
/// changes sign when it is run the other way).
fn same_ring(a: &[Corner], b: &[Corner]) -> bool {
    let n = a.len();
    if n != b.len() || n == 0 {
        return false;
    }
    let same_bulge = |x: f64, y: f64| (x - y).abs() <= 1e-9;
    (0..n).any(|k| {
        (0..n).all(|i| {
            let (p, bp) = a[i];
            let (q, bq) = b[(i + k) % n];
            near(p, q) && same_bulge(bp, bq)
        }) || (0..n).all(|i| {
            let (p, bp) = a[i];
            let q = b[(k + n - i) % n].0;
            // Run backwards, the edge from a[i] is b's edge into b[k − i], reversed.
            let bq = b[(k + 2 * n - i - 1) % n].1;
            near(p, q) && same_bulge(bp, -bq)
        })
    })
}

/// Whether two lists of holes are the same holes, in any order.
fn same_holes(a: &[Vec<Corner>], b: &[Vec<Corner>]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut used = vec![false; b.len()];
    a.iter().all(|h| {
        let hit = (0..b.len()).find(|&j| !used[j] && same_ring(h, &b[j]));
        if let Some(j) = hit {
            used[j] = true;
        }
        hit.is_some()
    })
}

/// A ring's lowest corner (x, then y).
fn lowest(corners: &[Corner]) -> Option<Vec2> {
    corners
        .iter()
        .map(|c| c.0)
        .min_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)))
}

/// A part of an input area: the area's place among the lines, its outer
/// ring's lowest corner, its corners and its holes'.
struct Part {
    area: usize,
    low: Vec2,
    outer: Vec<Corner>,
    holes: Vec<Vec<Corner>>,
}

/// Every part of every input area, by its lowest corner's x.
fn existing_parts(lines: &[Entity]) -> Vec<Part> {
    let mut out = Vec::new();
    for (i, e) in lines.iter().enumerate() {
        if !matches!(e.shape, Shape::Polygon { .. }) {
            continue;
        }
        for part in area_parts(&e.shape).iter() {
            let Shape::Polygon {
                pts, bulges, holes, ..
            } = part
            else {
                continue;
            };
            let outer = turning(&Ring {
                pts: pts.clone(),
                bulges: bulges.clone(),
            });
            let Some(low) = lowest(&outer) else { continue };
            out.push(Part {
                area: i,
                low,
                outer,
                holes: holes.iter().flatten().map(turning).collect(),
            });
        }
    }
    out.sort_by(|a, b| a.low.x.total_cmp(&b.low.x));
    out
}

fn edge_box(e: &Edge) -> Bounds {
    match *e {
        Edge::Seg { a, b } => Bounds {
            min_x: js_min(a.x, b.x),
            min_y: js_min(a.y, b.y),
            max_x: js_max(a.x, b.x),
            max_y: js_max(a.y, b.y),
        },
        Edge::Arc { c, r, .. } => Bounds {
            min_x: c.x - r,
            min_y: c.y - r,
            max_x: c.x + r,
            max_y: c.y + r,
        },
    }
}

fn around(p: Vec2, d: f64) -> Bounds {
    Bounds {
        min_x: p.x - d,
        min_y: p.y - d,
        max_x: p.x + d,
        max_y: p.y + d,
    }
}

/// An open path's two ends and its vertices, or none for a closed shape.
fn open_path(shape: &Shape) -> Option<([Vec2; 2], Vec<Vec2>)> {
    let edges = || entity_edges(shape);
    let ends = |edges: &[Edge]| -> Option<[Vec2; 2]> {
        Some([point_at(edges.first()?, 0.0), point_at(edges.last()?, 1.0)])
    };
    match shape {
        Shape::Polyline { pts, .. } => ends(&edges()).map(|e| (e, pts.clone())),
        Shape::Line { .. } | Shape::Arc { .. } => ends(&edges()).map(|e| (e, e.to_vec())),
        Shape::Spline { closed: false, .. } => ends(&edges()).map(|e| (e, e.to_vec())),
        Shape::Ellipse { .. } => {
            let edges = edges();
            let e = ends(&edges)?;
            // A whole ellipse comes back to its start: no ends.
            (!near(e[0], e[1])).then(|| (e, e.to_vec()))
        }
        _ => None,
    }
}

/// The ends of the open paths among `lines` that touch no other object
/// (within 1 µm of its edges) and no other vertex of their own path (the
/// free ends of ADR 0148 §5).
fn free_ends(lines: &[Entity]) -> Vec<Vec2> {
    // Part by part: a multi-part polyline's parts are paths of their own (docs/adr/0174).
    let parts: Vec<Shape> = lines
        .iter()
        .filter(|e| bounds(&e.shape))
        .flat_map(|e| area_parts(&e.shape).into_owned())
        .collect();
    let work: Vec<(usize, &Shape)> = parts.iter().enumerate().collect();
    let mut edges: Vec<(usize, Edge)> = Vec::new();
    for (o, s) in &work {
        for edge in entity_edges(s) {
            edges.push((*o, edge));
        }
    }
    let boxes: Vec<(u32, Bounds)> = edges
        .iter()
        .enumerate()
        .map(|(i, (_, e))| (i as u32, edge_box(e)))
        .collect();
    let tree = PackedTree::build(&boxes);
    let mut out = Vec::new();
    let mut hits = Vec::new();
    for (o, s) in &work {
        let Some((ends, own)) = open_path(s) else {
            continue;
        };
        for (k, end) in ends.iter().enumerate() {
            let own_index = if k == 0 { 0 } else { own.len() - 1 };
            if own
                .iter()
                .enumerate()
                .any(|(j, q)| j != own_index && near(*q, *end))
            {
                continue;
            }
            let reach = around(*end, TOUCH);
            hits.clear();
            tree.search(&reach, &mut hits);
            let touches = hits.iter().any(|&i| {
                let (other, edge) = &edges[i as usize];
                other != o
                    && overlaps(&edge_box(edge), &reach)
                    && closest_on_edge(edge, *end).d <= TOUCH
            });
            if !touches {
                out.push(*end);
            }
        }
    }
    out
}

pub(crate) static OPS: &[Op] =
    &[op!("polygonize", |lines: Vec<Entity>,
                         labels: Vec<PolyLabel>,
                         islands: bool| {
        polygonize(&lines, &labels, islands)
    })];
