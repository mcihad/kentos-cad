//! Area algebra on the overlay engine (`apps/web/src/model/geom/region.ts`): union,
//! intersection, difference, splitting by lines, and the faces that line
//! work encloses. Areas are exact (arcs stay arcs) and may have holes.

use crate::api::Op;
use crate::geom::arrangement::{Area, Ring, Rule, Source, winding};
use crate::geom::bulge::{bulge_path_edges, bulge_ring_area, has_bulges, reverse_bulge_path};
use crate::geom::intersect::Edge;
use crate::geom::overlay::{FaceRing, face_rings, overlay};
use crate::jsmath::{js_cmp, stable_sort};
use crate::op;
use crate::vec2::Vec2;

pub fn ring_area(r: &Ring) -> f64 {
    bulge_ring_area(&r.pts, r.bulges.as_deref())
}

pub fn ring_edges(r: &Ring) -> Vec<Edge> {
    bulge_path_edges(&r.pts, r.bulges.as_deref(), true)
}

/// The ring running counter-clockwise (`ccw`) or clockwise.
pub fn orient_ring(r: &Ring, ccw: bool) -> Ring {
    if (ring_area(r) > 0.0) == ccw {
        return r.clone();
    }
    let rev = reverse_bulge_path(&r.pts, r.bulges.as_deref(), true);
    if has_bulges(rev.bulges.as_deref()) {
        Ring {
            pts: rev.pts,
            bulges: rev.bulges,
        }
    } else {
        Ring {
            pts: rev.pts,
            bulges: None,
        }
    }
}

/// Net area: outer ring minus holes (m²).
pub fn net_area(a: &Area) -> f64 {
    ring_area(&a.outer).abs() - a.holes.iter().fold(0.0, |s, h| s + ring_area(h).abs())
}

/// Whether p lies inside the area (inside the outer ring, outside every hole).
pub fn inside_area(a: &Area, p: Vec2) -> bool {
    if winding(&ring_edges(&a.outer), p) == 0.0 {
        return false;
    }
    !a.holes.iter().any(|h| winding(&ring_edges(h), p) != 0.0)
}

/// Overlay source of areas: outer rings counter-clockwise, holes clockwise.
pub fn area_source(list: &[Area]) -> Source {
    let mut edges = Vec::new();
    let mut points = Vec::new();
    for a in list {
        let mut rs = vec![orient_ring(&a.outer, true)];
        rs.extend(a.holes.iter().map(|h| orient_ring(h, false)));
        for r in rs {
            edges.extend(ring_edges(&r));
            points.extend_from_slice(&r.pts);
        }
    }
    Source {
        edges,
        points: Some(points),
        cut: None,
    }
}

/// Everything covered by any of the areas.
pub fn union_areas(list: &[Area]) -> Vec<Area> {
    if list.is_empty() {
        return Vec::new();
    }
    let sources: Vec<Source> = list
        .iter()
        .map(|a| area_source(std::slice::from_ref(a)))
        .collect();
    overlay(&sources, Rule::Any)
}

/// What all the areas have in common.
pub fn intersect_areas(list: &[Area]) -> Vec<Area> {
    if list.len() < 2 {
        return list.to_vec();
    }
    let sources: Vec<Source> = list
        .iter()
        .map(|a| area_source(std::slice::from_ref(a)))
        .collect();
    overlay(&sources, Rule::All)
}

/// `from` with everything covered by `cutters` removed.
pub fn subtract_areas(from: &[Area], cutters: &[Area]) -> Vec<Area> {
    if cutters.is_empty() {
        return from.to_vec();
    }
    overlay(
        &[area_source(from), area_source(cutters)],
        Rule::FirstNotOthers,
    )
}

/// The area cut along lines (a line has to cross the area, or meet another line, to cut).
pub fn split_area(a: &Area, cut: &Source) -> Vec<Area> {
    let cut = Source {
        edges: cut.edges.clone(),
        points: cut.points.clone(),
        cut: Some(true),
    };
    overlay(&[area_source(std::slice::from_ref(a)), cut], Rule::First)
}

/// Faces of line work, computed once and queried many times (hover previews).
pub struct FaceIndex {
    rings: Vec<FaceRing>,
    /// Indices of the bounded faces, smallest first.
    outers: Vec<usize>,
    /// Indices of the group outlines (area < 0).
    groups: Vec<usize>,
    /// The face each group outline belongs to (the smallest around a point just outside it).
    hosts: Vec<Option<usize>>,
}

fn in_box(r: &FaceRing, p: Vec2) -> bool {
    p.x >= r.bx.min_x && p.x <= r.bx.max_x && p.y >= r.bx.min_y && p.y <= r.bx.max_y
}

impl FaceIndex {
    pub fn new(lines: &[Source]) -> FaceIndex {
        let rings = face_rings(lines);
        let mut outers: Vec<usize> = (0..rings.len()).filter(|&i| rings[i].area > 0.0).collect();
        stable_sort(&mut outers, &mut |&a, &b| {
            js_cmp(rings[a].area - rings[b].area, 0.0)
        });
        let groups: Vec<usize> = (0..rings.len()).filter(|&i| rings[i].area < 0.0).collect();
        let hosts = groups
            .iter()
            .map(|&g| {
                let probe = rings[g].probe;
                outers
                    .iter()
                    .copied()
                    .find(|&o| in_box(&rings[o], probe) && rings[o].contains(probe))
            })
            .collect();
        FaceIndex {
            rings,
            outers,
            groups,
            hosts,
        }
    }

    fn holes_of(&self, outer: usize) -> Vec<Ring> {
        self.groups
            .iter()
            .zip(&self.hosts)
            .filter(|&(&g, &host)| {
                in_box(&self.rings[outer], self.rings[g].probe) && host == Some(outer)
            })
            .map(|(&g, _)| self.rings[g].ring.clone())
            .collect()
    }

    /// The face around `p`, or None outside every closed shape; with
    /// `islands`, closed groups inside the face become holes.
    pub fn at(&self, p: Vec2, islands: bool) -> Option<Area> {
        let outer = self
            .outers
            .iter()
            .copied()
            .find(|&o| in_box(&self.rings[o], p) && self.rings[o].contains(p))?;
        Some(Area {
            outer: self.rings[outer].ring.clone(),
            holes: if islands {
                self.holes_of(outer)
            } else {
                Vec::new()
            },
        })
    }

    /// Every bounded face, each with the groups inside it as holes.
    pub fn all(&self) -> Vec<Area> {
        self.outers
            .iter()
            .map(|&o| Area {
                outer: self.rings[o].ring.clone(),
                holes: self.holes_of(o),
            })
            .collect()
    }
}

pub(crate) static OPS: &[Op] = &[
    op!("ringArea", |r: Ring| ring_area(&r)),
    op!("ringEdges", |r: Ring| ring_edges(&r)),
    op!("orientRing", |r: Ring, ccw: bool| orient_ring(&r, ccw)),
    op!("netArea", |a: Area| net_area(&a)),
    op!("insideArea", |a: Area, p: Vec2| inside_area(&a, p)),
    op!("areaSource", |list: Vec<Area>| area_source(&list)),
    op!("unionAreas", |list: Vec<Area>| union_areas(&list)),
    op!("intersectAreas", |list: Vec<Area>| intersect_areas(&list)),
    op!("subtractAreas", |from: Vec<Area>, cutters: Vec<Area>| {
        subtract_areas(&from, &cutters)
    }),
    op!("splitArea", |a: Area, cut: Source| split_area(&a, &cut)),
    op!("faceAt", |lines: Vec<Source>,
                   p: Vec2,
                   islands: Option<bool>| {
        FaceIndex::new(&lines).at(p, islands.unwrap_or(true))
    }),
    op!("allFaces", |lines: Vec<Source>| FaceIndex::new(&lines)
        .all()),
    op!("overlay", |sources: Vec<Source>, rule: String| Rule::named(
        &rule
    )
    .map(|r| overlay(&sources, r))),
    op!("faceRings", |sources: Vec<Source>| face_rings(&sources)),
    op!("winding", |edges: Vec<Edge>, p: Vec2| winding(&edges, p)),
];
