//! What another discipline's model says about where it stands (feature `geometry`; docs/adr/8102; plan/bim-plan.md
//! BIM-17): its storeys' elevations (`IfcBuildingStorey.Elevation`, above the building's ±0.00) and its grids' axes
//! (`IfcGrid` U, V and W axes, each an `IfcGridAxis` with its tag and curve) in the file's world, in metres. The
//! alignment check sets them beside the project's levels and grids. Nothing is guessed: a storey without an elevation
//! says none, an axis whose curve is no polyline or whose grid's placement does not resolve is left out.

use ifc_lite_core::{AttributeValue, DecodedEntity, EntityDecoder, SchemaVersion};

use super::graph::named;
use super::{Model, ReadError, guarded, header};

/// A storey: its object (an index into [`Model::objects`]), its name (its long name, else its name) and its
/// elevation above the building's ±0.00, metres; none when the file does not say it.
#[derive(Debug, Clone, PartialEq)]
pub struct Storey {
    pub object: usize,
    pub name: String,
    pub elevation: Option<f64>,
}

/// An axis of a grid: its grid (an index into [`Model::objects`]), its tag and its curve's first and last points in
/// the file's world, metres.
#[derive(Debug, Clone, PartialEq)]
pub struct GridAxis {
    pub grid: usize,
    pub tag: String,
    pub a: [f64; 3],
    pub b: [f64; 3],
}

/// A point's coordinates (`IfcCartesianPoint.Coordinates`), a missing one 0.
fn coordinates(schema: SchemaVersion, p: &DecodedEntity) -> Option<[f64; 3]> {
    let c = named(schema, p, "Coordinates").and_then(AttributeValue::as_list)?;
    let v: Vec<f64> = c.iter().filter_map(AttributeValue::as_float).collect();
    (!v.is_empty()).then(|| [v[0], v.get(1).copied().unwrap_or(0.0), v.get(2).copied().unwrap_or(0.0)])
}

impl Model {
    /// The storeys, in the file's order.
    pub fn storeys(&self, bytes: &[u8]) -> Result<Vec<Storey>, ReadError> {
        let schema = header::version(self.summary.family);
        guarded(|| {
            let mut decoder = EntityDecoder::with_arc_index(bytes, self.index.clone());
            let scale = decoder.length_unit_scale();
            let mut out = Vec::new();
            for (i, o) in self.objects.iter().enumerate() {
                if !schema.is_subtype_of(&o.class, "IfcBuildingStorey") {
                    continue;
                }
                let elevation = decoder
                    .decode_by_id(o.step)
                    .ok()
                    .and_then(|e| named(schema, &e, "Elevation").and_then(AttributeValue::as_float))
                    .map(|v| v * scale);
                let name = o
                    .long_name
                    .clone()
                    .filter(|n| !n.trim().is_empty())
                    .or_else(|| o.name.clone())
                    .unwrap_or_default();
                out.push(Storey { object: i, name, elevation });
            }
            Ok(out)
        })
    }

    /// The grids' axes, in the file's order: each grid's U axes, then its V and W axes.
    pub fn grid_axes(&self, bytes: &[u8]) -> Result<Vec<GridAxis>, ReadError> {
        let schema = header::version(self.summary.family);
        guarded(|| {
            let mut decoder = EntityDecoder::with_arc_index(bytes, self.index.clone());
            let scale = decoder.length_unit_scale();
            let mut out = Vec::new();
            for (i, o) in self.objects.iter().enumerate() {
                if !schema.is_subtype_of(&o.class, "IfcGrid") {
                    continue;
                }
                let Ok(grid) = decoder.decode_by_id(o.step) else { continue };
                // The grid's world placement, in the file's units; `$` is the world itself.
                let placed = ifc_lite_geometry::placement::resolve_object_placement(&grid, &mut decoder);
                let m = match placed {
                    Ok(Some(m)) => Some(m),
                    Ok(None) => None,
                    Err(_) => continue,
                };
                let world = |p: [f64; 3]| -> [f64; 3] {
                    let q = match &m {
                        Some(m) => [
                            m[(0, 0)] * p[0] + m[(0, 1)] * p[1] + m[(0, 2)] * p[2] + m[(0, 3)],
                            m[(1, 0)] * p[0] + m[(1, 1)] * p[1] + m[(1, 2)] * p[2] + m[(1, 3)],
                            m[(2, 0)] * p[0] + m[(2, 1)] * p[1] + m[(2, 2)] * p[2] + m[(2, 3)],
                        ],
                        None => p,
                    };
                    [q[0] * scale, q[1] * scale, q[2] * scale]
                };
                for list in ["UAxes", "VAxes", "WAxes"] {
                    let ids: Vec<u32> = named(schema, &grid, list)
                        .and_then(AttributeValue::as_list)
                        .map(|l| l.iter().filter_map(AttributeValue::as_entity_ref).collect())
                        .unwrap_or_default();
                    for id in ids {
                        let Ok(axis) = decoder.decode_by_id(id) else { continue };
                        let tag = named(schema, &axis, "AxisTag").and_then(AttributeValue::as_string).unwrap_or("").to_owned();
                        let Some(curve) = named(schema, &axis, "AxisCurve").and_then(AttributeValue::as_entity_ref) else {
                            continue;
                        };
                        let Ok(curve) = decoder.decode_by_id(curve) else { continue };
                        let ids: Vec<u32> = named(schema, &curve, "Points")
                            .and_then(AttributeValue::as_list)
                            .map(|l| l.iter().filter_map(AttributeValue::as_entity_ref).collect())
                            .unwrap_or_default();
                        let points: Vec<[f64; 3]> = ids
                            .into_iter()
                            .filter_map(|p| decoder.decode_by_id(p).ok())
                            .filter_map(|p| coordinates(schema, &p))
                            .collect();
                        if let (Some(a), Some(b)) = (points.first(), points.last())
                            && points.len() >= 2
                        {
                            out.push(GridAxis { grid: i, tag, a: world(*a), b: world(*b) });
                        }
                    }
                }
            }
            Ok(out)
        })
    }
}
