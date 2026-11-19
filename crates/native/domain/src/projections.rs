//! Çıkmalar (docs/adr/8152; Planlı Alanlar İmar Yönetmeliği 41): the parts of the rooms of the storeys over the ground
//! storey outside the footprint (taban alanı), each with how near it comes to each edge of the site plan's parcel and
//! how high its underside is; the limits are the standards package's.
//!
//! - **A projection** is a room's part outside the footprint's outline (the ground storeys' rooms, docs/adr/8032) on a
//!   storey higher than its building's ground storey; open (açık çıkma) when the room is outdoor (a balcony, a terrace
//!   by its use's brief), closed (kapalı çıkma) otherwise. A part under 1 cm² is no projection.
//! - **Each parcel edge:** its kind as the garden distances read it (a party edge in ayrık nizam is a side garden,
//!   docs/adr/8142), the distance it asks and the nearest the projection comes to it.
//! - **Its underside:** of the floor slab under its middle on its storey (its structure's underside less the layers
//!   under it, docs/adr/8040), over ±0.00, where the model's graded ground lies (EK-4 4.30); none with no slab drawn
//!   there.

use kentos_contracts::{BuildingLayout, Entity, SiteEdgeKind};
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region;
use kentos_geometry_core::vec2::Vec2;

use crate::Document;

/// A projection (docs/adr/8152): the room's persistent id and storey, whether it is open, its part outside the
/// footprint (area, m², and outline), each parcel edge with the distance it asks and the projection's nearest distance
/// to it, its storey's floor and its underside over ±0.00 (none: no slab drawn under it).
#[derive(Clone, Debug, PartialEq)]
pub struct Projection {
    pub room: String,
    pub level: Option<String>,
    pub open: bool,
    pub area: f64,
    pub shape: Vec<Area>,
    pub edges: Vec<ProjectionEdge>,
    pub floor: f64,
    pub underside: Option<f64>,
}

/// A parcel edge as a projection reads it: its place in the site plan (from 0), its kind as the gardens read it, the
/// distance it asks and the projection's nearest distance to it, m.
#[derive(Clone, Debug, PartialEq)]
pub struct ProjectionEdge {
    pub edge: usize,
    pub kind: SiteEdgeKind,
    pub required: f64,
    pub distance: f64,
}

/// The least area a projection has, m².
const LEAST: f64 = 1e-4;

/// A point's distance to a segment.
fn to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 { (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    (p.x - (a.x + dx * t)).hypot(p.y - (a.y + dy * t))
}

/// The nearest an outline comes to a segment: its corners to the segment and the segment's ends to its sides (the two
/// do not cross: the projection is inside the parcel).
fn nearest(shape: &[Area], a: Vec2, b: Vec2) -> f64 {
    let mut d = f64::INFINITY;
    for ring in shape.iter().flat_map(|x| std::iter::once(&x.outer).chain(&x.holes)) {
        let n = ring.pts.len();
        for k in 0..n {
            let (p, q) = (ring.pts[k], ring.pts[(k + 1) % n]);
            d = d.min(to_segment(p, a, b)).min(to_segment(a, p, q)).min(to_segment(b, p, q));
        }
    }
    d
}

/// A point inside an outline's largest part: its outer ring's centroid when inside it, else its first corner nudged in.
fn inner_point(shape: &[Area]) -> Option<Vec2> {
    let largest = shape.iter().max_by(|x, y| region::net_area(x).total_cmp(&region::net_area(y)))?;
    let pts = &largest.outer.pts;
    let (mut cx, mut cy, mut a2) = (0.0, 0.0, 0.0);
    for k in 0..pts.len() {
        let (p, q) = (pts[k], pts[(k + 1) % pts.len()]);
        let c = p.x * q.y - q.x * p.y;
        a2 += c;
        cx += (p.x + q.x) * c;
        cy += (p.y + q.y) * c;
    }
    if a2.abs() > 1e-12 {
        let c = Vec2::new(cx / (3.0 * a2), cy / (3.0 * a2));
        if region::inside_area(largest, c) {
            return Some(c);
        }
    }
    pts.first().copied()
}

impl Document {
    /// The rooms' parts outside the footprint on the storeys over the ground storey, in the drawing's order (docs/adr/
    /// 8152); none without the site plan's parcel or a footprint.
    pub fn projections(&self) -> Vec<Projection> {
        match self.settings().arch.as_ref().and_then(|a| a.site.clone()) {
            Some(site) => self.projections_for(&site),
            None => Vec::new(),
        }
    }

    /// The projections against `site`, a site plan not yet written (the Parsel tool's, docs/adr/8155).
    pub fn projections_for(&self, site: &kentos_contracts::SitePlan) -> Vec<Projection> {
        let Some(arch) = self.settings().arch.as_ref() else { return Vec::new() };
        let Some(report) = self.site_report_for(site) else { return Vec::new() };
        let counted = self.zoning_counted();
        if counted.footprint_shape.is_empty() {
            return Vec::new();
        }
        let detached = arch.layout == Some(BuildingLayout::Detached);
        let program = &arch.program;
        let elevation = |id: &str| arch.levels.iter().find(|l| l.id == id).map(|l| l.elevation);
        // Each storey's ground: its building's ground storey (docs/adr/8072 §4), else the footprint's lowest.
        let lowest = counted.grounds.iter().filter_map(|(g, _)| elevation(g)).reduce(f64::min);
        let ground_of = |level: &str| -> Option<f64> {
            program
                .building_of_level(level)
                .and_then(|b| program.ground_of(&b.id, &arch.levels))
                .and_then(|g| elevation(&g))
                .or(lowest)
        };
        // The outdoor rooms: a use whose brief is outdoor (docs/adr/8069).
        let outdoor: std::collections::BTreeSet<String> = self
            .entities_with_uids()
            .filter_map(|(uid, e)| match e {
                Entity::Space(s)
                    if s.space.usage.as_deref().and_then(|u| program.brief(u)).is_some_and(|b| b.outdoor) =>
                {
                    Some(uid.to_string())
                }
                _ => None,
            })
            .collect();
        let mut out = Vec::new();
        for r in &counted.rooms {
            let Some(level) = r.level.as_deref() else { continue };
            let (Some(floor), Some(ground)) = (elevation(level), ground_of(level)) else { continue };
            if floor <= ground + 1e-6 {
                continue;
            }
            let shape = region::subtract_areas(&r.shape, &counted.footprint_shape);
            let area: f64 = shape.iter().map(region::net_area).sum();
            if area < LEAST {
                continue;
            }
            let edges = report
                .edges
                .iter()
                .enumerate()
                .map(|(i, e)| ProjectionEdge {
                    edge: i,
                    kind: match e.kind {
                        SiteEdgeKind::Party if detached => SiteEdgeKind::Side,
                        k => k,
                    },
                    required: e.required,
                    distance: nearest(&shape, e.a, e.b),
                })
                .collect();
            let underside = inner_point(&shape).and_then(|at| {
                let at = kentos_contracts::Vec2 { x: at.x, y: at.y };
                let (s, f) = self.floor_slab_under(at, floor)?;
                Some(s.structure_underside_at(f, at) - s.layers.under_thickness())
            });
            out.push(Projection {
                room: r.uid.clone(),
                level: r.level.clone(),
                open: outdoor.contains(&r.uid),
                area,
                shape,
                edges,
                floor,
                underside,
            });
        }
        out
    }
}
