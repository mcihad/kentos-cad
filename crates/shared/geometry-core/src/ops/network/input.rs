//! A network's input from the drawing's shapes (docs/adr/0209 §3): an edge
//! layer's lines, polylines (each part its own edge) and arcs; a junction
//! layer's points (each point of a multi-point object). Other kinds are not
//! taken and are said by their kind. The geometry store gives the shapes by
//! id (`from_store`), so the apps pass ids and the values they evaluated.

use super::graph::{EdgeIn, JunctionIn, Role};
use crate::entity::{Shape, area_parts};
use crate::geom::bulge::bulge_path_edges;
use crate::geom::intersect::Edge;
use crate::ops::edges::entity_edges;
use crate::store::Store;
use crate::vec2::Vec2;

/// What the app evaluated for an edge object: its direction field's value, its costs' fields' values and whether
/// the closed edges' expression holds.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EdgeValues {
    pub direction: Option<String>,
    pub costs: Vec<Option<String>>,
    pub closed: bool,
}

/// What the app gave for a junction object: its layer's role and whether its closed expression holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JunctionValues {
    pub role: Role,
    pub closed: bool,
}

/// The edge paths of a shape: a line's, each polyline part's, an arc's; none for another kind.
pub fn paths_of(shape: &Shape) -> Option<Vec<Vec<Edge>>> {
    match shape {
        Shape::Line { .. } | Shape::Arc { .. } => Some(vec![entity_edges(shape)]),
        Shape::Polyline { .. } => Some(
            area_parts(shape)
                .iter()
                .filter_map(|p| match p {
                    Shape::Polyline { pts, bulges, .. } if pts.len() >= 2 => {
                        Some(bulge_path_edges(pts, bulges.as_deref(), false))
                    }
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    }
}

/// The points of a shape: a point's, each point of a multi-point object; none for another kind.
pub fn points_of(shape: &Shape) -> Option<Vec<Vec2>> {
    match shape {
        Shape::Point { .. } => Some(
            area_parts(shape)
                .iter()
                .filter_map(|p| match p {
                    Shape::Point { p, .. } => Some(*p),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    }
}

/// A shape's kind as the apps name it (`line`, `polygon` …), for what was not taken.
pub fn kind_of(shape: &Shape) -> &'static str {
    match shape {
        Shape::Point { .. } => "point",
        Shape::Line { .. } => "line",
        Shape::Polyline { .. } => "polyline",
        Shape::Polygon { .. } => "polygon",
        Shape::Circle { .. } => "circle",
        Shape::Arc { .. } => "arc",
        Shape::Ellipse { .. } => "ellipse",
        Shape::Spline { .. } => "spline",
        Shape::Text { .. } => "text",
        _ => "other",
    }
}

/// The network's input from `store`: the edge objects `edges` with their values, the junction objects `junctions`
/// with theirs (each list's values in its order), and the objects not taken with their kinds.
pub fn from_store(
    store: &Store,
    edges: &[(f64, EdgeValues)],
    junctions: &[(f64, JunctionValues)],
) -> (Vec<EdgeIn>, Vec<JunctionIn>, Vec<(f64, &'static str)>) {
    let mut skipped = Vec::new();
    let mut edge_in = Vec::with_capacity(edges.len());
    for (id, v) in edges {
        let Some(item) = store.get(*id) else { continue };
        match paths_of(&item.shape) {
            Some(paths) => {
                for (part, path) in paths.into_iter().enumerate() {
                    if path.is_empty() {
                        continue;
                    }
                    edge_in.push(EdgeIn {
                        id: *id,
                        part: part as u32,
                        path,
                        direction: v.direction.clone(),
                        costs: v.costs.clone(),
                        closed: v.closed,
                    });
                }
            }
            None => skipped.push((*id, kind_of(&item.shape))),
        }
    }
    let mut junction_in = Vec::new();
    for (id, v) in junctions {
        let Some(item) = store.get(*id) else { continue };
        match points_of(&item.shape) {
            Some(points) => junction_in.extend(points.into_iter().map(|p| JunctionIn {
                id: *id,
                p,
                role: v.role,
                closed: v.closed,
            })),
            None => skipped.push((*id, kind_of(&item.shape))),
        }
    }
    (edge_in, junction_in, skipped)
}
