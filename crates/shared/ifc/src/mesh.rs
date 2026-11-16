//! The building's bodies as triangles (docs/adr/8067; plan/bim-plan.md BIM-06, plan/ifc-uyum.md IFC-12): every element's
//! shapes as the IFC file writes them ([`crate::building::Building`]), its openings taken out of its walls, and every
//! space's volume, each with its element's id, class and storey. For a 3D view: the shapes stay the source and the
//! measure, the triangles only show them (plan K7).
//!
//! - **A prism** (an extrusion: a wall, a slab, a column, a beam, a stair's tread): its footprint's rings (even-odd, as the
//!   file writes them; an arc flattened at the drawing's step, docs/adr/8071 §1) triangulated for its caps, a quad up
//!   each edge. Where openings go through it, its cut body (`quantities::cut`, a Reference View's, docs/adr/8071 §4):
//!   its height cut at their sills and heads, each band's footprint the rings less the openings' footprints standing in
//!   it (the geometry core's overlay, arcs kept), so a window is a hole, its sill and head faces drawn.
//! - **Faces** (a roof's face): each face as the file has it, outward; a face not seen edge-on from above triangulated
//!   by its plan, a vertical one as a fan.
//! - **Heights** are over ±0.00: the building's ±0.00 and an element's storey's elevation added to its shapes' own.

use std::ops::Range;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::triangulate::triangulate_into;

use crate::building::{Building, Class, Opening, Shape, flat_rings, nest};

/// What a mesh is of.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Element(Class),
    /// A space's volume (a room).
    Space,
}

/// One element's or space's triangles.
#[derive(Clone, Debug, PartialEq)]
pub struct Mesh {
    /// Its KentOS object's id (the persistent id's 128 bits): the file's GlobalId, or the object it shows when that
    /// differs (a roof's chimney stack: the roof, docs/adr/8070 §3); none for an element made of the file's seed.
    pub id: Option<u128>,
    pub kind: Kind,
    /// Its storey, by index into [`Building::storeys`].
    pub storey: usize,
    /// Its name, as the file writes it.
    pub name: String,
    /// Metres; heights over ±0.00.
    pub vertices: Vec<[f64; 3]>,
    /// Counterclockwise seen from outside.
    pub triangles: Vec<[u32; 3]>,
}

impl Mesh {
    fn new(id: Option<u128>, kind: Kind, storey: usize, name: &str) -> Mesh {
        Mesh { id, kind, storey, name: name.to_owned(), vertices: Vec::new(), triangles: Vec::new() }
    }

    fn vertex(&mut self, p: [f64; 3]) -> u32 {
        self.vertices.push(p);
        (self.vertices.len() - 1) as u32
    }
}

/// Every element with a body and every space, in the building's order: the elements first.
pub fn meshes(b: &Building) -> Vec<Mesh> {
    let mut out = Vec::new();
    for (i, e) in b.elements.iter().enumerate() {
        let floor = b.zero + b.storeys.get(e.storey).map_or(0.0, |s| s.elevation);
        let cuts: Vec<&Opening> = b.openings.iter().filter(|o| o.host == i && o.ring.len() >= 3).collect();
        let mut m = Mesh::new(e.object.or(e.id), Kind::Element(e.class), e.storey, &e.name);
        // Its cut body (docs/adr/8071 §4): the Reference View's and the net volume's, each band's arcs flattened here.
        for shape in &crate::quantities::cut(&e.shapes, &cuts) {
            match shape {
                Shape::Prism { rings, bulges, bottom, top } => {
                    prism(&mut m, &flat_rings(rings, bulges), floor + bottom, floor + top)
                }
                Shape::Faces { vertices, faces } => polyhedron(&mut m, vertices, faces, floor),
            }
        }
        if !m.triangles.is_empty() {
            out.push(m);
        }
    }
    for s in &b.spaces {
        let floor = b.zero + b.storeys.get(s.storey).map_or(0.0, |x| x.elevation);
        let mut m = Mesh::new(s.id, Kind::Space, s.storey, s.long_name.as_deref().unwrap_or(&s.name));
        // Under a roof its pieces (docs/adr/8071 §11), else its prism.
        if s.body.is_empty() {
            prism(&mut m, &flat_rings(&s.rings, &s.bulges), floor + s.bottom, floor + s.top);
        } else {
            for shape in &s.body {
                if let Shape::Faces { vertices, faces } = shape {
                    polyhedron(&mut m, vertices, faces, floor);
                }
            }
        }
        if !m.triangles.is_empty() {
            out.push(m);
        }
    }
    out
}

fn v2(p: [f64; 2]) -> Vec2 {
    Vec2::new(p[0], p[1])
}

fn signed(pts: &[Vec2]) -> f64 {
    let n = pts.len();
    (0..n).map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y).sum::<f64>() / 2.0
}

/// `rings` (even-odd) as areas: outer rings counterclockwise, holes clockwise.
fn areas(rings: &[Vec<[f64; 2]>]) -> Vec<Area> {
    let ring = |r: &[[f64; 2]], ccw: bool| {
        let mut pts: Vec<Vec2> = r.iter().map(|p| v2(*p)).collect();
        if (signed(&pts) > 0.0) != ccw {
            pts.reverse();
        }
        Ring { pts, bulges: None }
    };
    nest(rings)
        .into_iter()
        .filter(|(o, _)| rings[*o].len() >= 3)
        .map(|(o, holes)| Area {
            outer: ring(&rings[o], true),
            holes: holes.into_iter().filter(|h| rings[*h].len() >= 3).map(|h| ring(&rings[h], false)).collect(),
        })
        .collect()
}

/// A prism of `rings` (straight, even-odd) between `bottom` and `top`.
fn prism(m: &mut Mesh, rings: &[Vec<[f64; 2]>], bottom: f64, top: f64) {
    if top - bottom <= 1e-9 {
        return;
    }
    for a in &areas(rings) {
        band_of(m, a, bottom, top);
    }
}

/// One area between two heights: its caps and its sides.
fn band_of(m: &mut Mesh, a: &Area, z0: f64, z1: f64) {
    let rings: Vec<&Vec<Vec2>> = std::iter::once(&a.outer.pts).chain(a.holes.iter().map(|h| &h.pts)).collect();
    let mut pts: Vec<Vec2> = Vec::new();
    let mut ranges: Vec<Range<usize>> = Vec::new();
    for r in &rings {
        let start = pts.len();
        pts.extend(r.iter().copied());
        ranges.push(start..pts.len());
    }
    let mut tri: Vec<u32> = Vec::new();
    triangulate_into(&pts, &ranges, &mut tri);
    // The top as triangulated (counterclockwise from over it), the bottom the other way round.
    let top: Vec<u32> = pts.iter().map(|p| m.vertex([p.x, p.y, z1])).collect();
    let bottom: Vec<u32> = pts.iter().map(|p| m.vertex([p.x, p.y, z0])).collect();
    for t in tri.chunks_exact(3) {
        let (i, j, k) = (t[0] as usize, t[1] as usize, t[2] as usize);
        let ccw = (pts[j].x - pts[i].x) * (pts[k].y - pts[i].y) - (pts[k].x - pts[i].x) * (pts[j].y - pts[i].y) > 0.0;
        let (j, k) = if ccw { (j, k) } else { (k, j) };
        m.triangles.push([top[i], top[j], top[k]]);
        m.triangles.push([bottom[i], bottom[k], bottom[j]]);
    }
    // Each edge's side, outward: an outer ring runs counterclockwise and a hole clockwise, so the outside is on the
    // right of every edge.
    for r in ranges {
        let n = r.len();
        for e in 0..n {
            let (i, j) = (r.start + e, r.start + (e + 1) % n);
            m.triangles.push([bottom[i], bottom[j], top[j]]);
            m.triangles.push([bottom[i], top[j], top[i]]);
        }
    }
}

/// A closed set of faces (`vertices` over `floor`), each outward.
fn polyhedron(m: &mut Mesh, vertices: &[[f64; 3]], faces: &[Vec<usize>], floor: f64) {
    let at: Vec<u32> = vertices.iter().map(|v| m.vertex([v[0], v[1], v[2] + floor])).collect();
    for face in faces.iter().filter(|f| f.len() >= 3 && f.iter().all(|k| *k < vertices.len())) {
        let plan: Vec<Vec2> = face.iter().map(|k| Vec2::new(vertices[*k][0], vertices[*k][1])).collect();
        if signed(&plan).abs() > 1e-9 {
            // Not edge-on from above: triangulated by its plan, in the face's own turn.
            let mut tri: Vec<u32> = Vec::new();
            let n = plan.len();
            triangulate_into(&plan, std::slice::from_ref(&(0..n)), &mut tri);
            let up = signed(&plan) > 0.0;
            for t in tri.chunks_exact(3) {
                let (i, j, k) = (t[0] as usize, t[1] as usize, t[2] as usize);
                let ccw = (plan[j].x - plan[i].x) * (plan[k].y - plan[i].y) - (plan[k].x - plan[i].x) * (plan[j].y - plan[i].y) > 0.0;
                let (j, k) = if ccw == up { (j, k) } else { (k, j) };
                m.triangles.push([at[face[i]], at[face[j]], at[face[k]]]);
            }
        } else {
            for k in 1..face.len() - 1 {
                m.triangles.push([at[face[0]], at[face[k]], at[face[k + 1]]]);
            }
        }
    }
}

#[cfg(test)]
mod tests;
