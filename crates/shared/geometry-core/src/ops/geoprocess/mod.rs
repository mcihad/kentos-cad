//! Geometri işlemleri (docs/adr/0201): the processing tools' buffers,
//! clipping, overlays, dissolving, validity and repair, simplifying and
//! reprojecting of whole layers, over the core's overlay (areas exact with
//! their arcs, `geom::overlay`). An object is taken as areas, paths or points
//! (`class_of`); what the tools get back are shapes for their output layer.
//! The web calls the operations below (`OPS`) with whole objects; the
//! desktop's tools call the functions. The independent reference is
//! scripts/fixtures/geoprocess_cases.py.

pub mod buffer;
pub mod calls;
pub mod clip;
pub mod reproject;
pub mod validity;

use crate::entity::{Shape, area_parts, join_parts};
use crate::geom::arrangement::{Area, Rule, TOL, WindingIndex, edge_len};
use crate::geom::bulge::bulge_of_sweep;
use crate::geom::intersect::{Edge, closest_on_edge, point_at};
use crate::geom::overlay::overlay;
use crate::geom::region::{area_source, net_area, ring_edges};
use crate::jsmath::js_hypot;
use crate::ops::areas::areas_of_entity;
use crate::ops::edges::entity_edges;
use crate::ops::parts::one_area;
use crate::vec2::Vec2;

/// An object as the geometry tools take it (docs/adr/0201 §1).
#[derive(Clone, Debug, PartialEq)]
pub enum Class {
    /// Its areas with their holes: a multi-part area's parts, a circle, a whole ellipse, a closed curve.
    Areas(Vec<Area>),
    /// Its paths, each a run of edges end to start: a line, a polyline's parts, an arc, an ellipse's arc, an open curve.
    Paths(Vec<Vec<Edge>>),
    /// Its points (a multi-point object's every point).
    Points(Vec<Vec2>),
    /// Not taken: a text, a dimension, a block, a hatch, a construction line …
    None,
}

impl Class {
    pub fn is_none(&self) -> bool {
        matches!(self, Class::None)
    }
}

/// A path's edges as the tools take them: a straight edge whose ends are
/// one point (a repeated vertex) is no edge.
fn path_of(edges: Vec<Edge>) -> Vec<Edge> {
    edges
        .into_iter()
        .filter(|e| match *e {
            Edge::Seg { a, b } => js_hypot(b.x - a.x, b.y - a.y) > TOL,
            Edge::Arc { .. } => true,
        })
        .collect()
}

/// Paths without the empty ones.
fn paths(list: impl IntoIterator<Item = Vec<Edge>>) -> Class {
    Class::Paths(
        list.into_iter()
            .map(path_of)
            .filter(|p| !p.is_empty())
            .collect(),
    )
}

/// How the geometry tools take an object.
pub fn class_of(s: &Shape) -> Class {
    match s {
        Shape::Polygon { .. } | Shape::Circle { .. } => Class::Areas(areas_of_entity(s)),
        Shape::Ellipse { .. } | Shape::Spline { .. } => {
            let areas = areas_of_entity(s);
            if areas.is_empty() {
                paths([entity_edges(s)])
            } else {
                Class::Areas(areas)
            }
        }
        Shape::Line { .. } | Shape::Arc { .. } => paths([entity_edges(s)]),
        Shape::Polyline { .. } => paths(area_parts(s).iter().map(entity_edges)),
        Shape::Point { .. } => Class::Points(
            area_parts(s)
                .iter()
                .filter_map(|p| match p {
                    Shape::Point { p, .. } => Some(*p),
                    _ => None,
                })
                .collect(),
        ),
        _ => Class::None,
    }
}

/// The vertices an object is written with: an area's rings' (holes too), a
/// polyline's, a line's two, a point's; every part's.
pub fn written_vertices(s: &Shape) -> usize {
    area_parts(s)
        .iter()
        .map(|p| match p {
            Shape::Polygon { pts, holes, .. } => {
                pts.len() + holes.iter().flatten().map(|h| h.pts.len()).sum::<usize>()
            }
            Shape::Polyline { pts, .. } => pts.len(),
            Shape::Line { .. } => 2,
            Shape::Point { .. } => 1,
            _ => 0,
        })
        .sum()
}

/// Whether the object enters as chords within 0.1 mm of its curve (an ellipse, a curve; docs/adr/0149 §5.3).
pub fn is_chorded(s: &Shape) -> bool {
    matches!(s, Shape::Ellipse { .. } | Shape::Spline { .. })
}

/// Everything any of the areas covers: one source, so the overlay's nonzero
/// winding counts the areas over a point (an area's holes take its own
/// count away there).
pub fn union_all(areas: &[Area]) -> Vec<Area> {
    if areas.is_empty() {
        return Vec::new();
    }
    overlay(&[area_source(areas)], Rule::Any)
}

/// `from` less everything `cut` covers.
pub fn subtract_all(from: &[Area], cut: &[Area]) -> Vec<Area> {
    if from.is_empty() || cut.is_empty() {
        return union_all(from);
    }
    overlay(&[area_source(from), area_source(cut)], Rule::FirstNotOthers)
}

/// What `a` and `b` both cover (each a set of areas).
pub fn intersect_all(a: &[Area], b: &[Area]) -> Vec<Area> {
    if a.is_empty() || b.is_empty() {
        return Vec::new();
    }
    overlay(&[area_source(a), area_source(b)], Rule::All)
}

/// The areas' net area (m²).
pub fn areas_measure(areas: &[Area]) -> f64 {
    areas.iter().map(net_area).sum()
}

/// The paths' length (m).
pub fn paths_measure(paths: &[Vec<Edge>]) -> f64 {
    paths.iter().flatten().map(edge_len).sum()
}

/// A class's size: an area's net area, a path's length, a point set's count.
pub fn measure(c: &Class) -> f64 {
    match c {
        Class::Areas(a) => areas_measure(a),
        Class::Paths(p) => paths_measure(p),
        Class::Points(p) => p.len() as f64,
        Class::None => 0.0,
    }
}

/// The boundary edges of areas: outer rings and holes.
pub fn boundary_edges(areas: &[Area]) -> Vec<Edge> {
    let mut out = Vec::new();
    for a in areas {
        out.extend(ring_edges(&a.outer));
        for h in &a.holes {
            out.extend(ring_edges(h));
        }
    }
    out
}

/// A class as the shape the tools write: areas as one (multi-part) area,
/// paths as one (multi-part) polyline, points as one (multi-)point; none
/// when it is empty.
pub fn shape_of(c: &Class) -> Option<Shape> {
    match c {
        Class::Areas(areas) => one_area(areas).map(|e| e.shape),
        Class::Paths(paths) => {
            let shapes: Vec<Shape> = paths.iter().filter_map(|p| path_shape(p)).collect();
            join_parts(&shapes)
        }
        Class::Points(points) => {
            let shapes: Vec<Shape> = points
                .iter()
                .map(|&p| Shape::Point {
                    p,
                    z: None,
                    parts: None,
                })
                .collect();
            join_parts(&shapes)
        }
        Class::None => None,
    }
}

/// One path as a polyline: its vertices and, for arc edges, their bulges.
pub fn path_shape(path: &[Edge]) -> Option<Shape> {
    let first = path.first()?;
    let mut pts = vec![point_at(first, 0.0)];
    let mut bulges = Vec::with_capacity(path.len());
    for e in path {
        pts.push(point_at(e, 1.0));
        bulges.push(match *e {
            Edge::Seg { .. } => 0.0,
            Edge::Arc { sweep, .. } => bulge_of_sweep(sweep),
        });
    }
    let curved = bulges.iter().any(|b| *b != 0.0);
    Some(Shape::Polyline {
        pts,
        bulges: curved.then_some(bulges),
        holes: None,
        parts: None,
    })
}

/// Edges in their order cut into runs where one does not start at the end of the one before.
pub fn chain(edges: Vec<Edge>) -> Vec<Vec<Edge>> {
    let mut out: Vec<Vec<Edge>> = Vec::new();
    for e in edges {
        let start = point_at(&e, 0.0);
        match out.last_mut() {
            Some(run)
                if run.last().is_some_and(|l| {
                    let end = point_at(l, 1.0);
                    js_hypot(end.x - start.x, end.y - start.y) <= TOL
                }) =>
            {
                run.push(e);
            }
            _ => out.push(vec![e]),
        }
    }
    out
}

/// Where a point stands to a set of areas: inside, on its boundary (within the overlay's tolerance) or outside.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Inside,
    On,
    Outside,
}

/// The areas' boundary, indexed for where points stand.
pub struct Region {
    edges: Vec<Edge>,
}

impl Region {
    pub fn of(areas: &[Area]) -> Region {
        let src = area_source(areas);
        Region { edges: src.edges }
    }

    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Where `p` stands: on the boundary first, so a point there is the boundary's.
    pub fn place(&self, index: &WindingIndex<'_>, p: Vec2) -> Place {
        if self.edges.iter().any(|e| closest_on_edge(e, p).d <= TOL) {
            return Place::On;
        }
        if index.winding(p) != 0.0 {
            Place::Inside
        } else {
            Place::Outside
        }
    }
}
