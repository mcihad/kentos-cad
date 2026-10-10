//! The geometry processing tools ask for, from the shared geometry store
//! (docs/adr/0008, S4; the web's `processing/geometry.ts`). A run puts the
//! objects it reads into a store of its own, as the web's page and worker
//! do, and tools ask it by id: the expressions' geometry values, corner
//! numbering, the texts beside numbered corners, edge-length labels. This
//! only packs and reads: no coordinate is computed here.

use kentos_contracts::{Entity, Vec2};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::spatial_query::Relation;
use kentos_geometry_core::processing::numbering::{CornerWalk, corner_text_at};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::processing::{CORNER_STRIDE, EDGE_LABEL_STRIDE};
pub use kentos_geometry_core::store::proximity::Measure;
use kentos_geometry_core::store::proximity::{
    NEAREST_STRIDE, NEIGHBOR_CORNER, NEIGHBOR_OVERLAP, NEIGHBOR_STRIDE,
};
use kentos_geometry_core::text::Font;
use kentos_geometry_core::vec2::Vec2 as CoreVec2;
use kentos_native_application::geometry::shape;

/// An object as the store takes it: its id, layer, whether it has a label,
/// and its shape (the desktop's one conversion, `geometry::shape`).
pub fn record(entity: &Entity) -> (f64, &str, bool, Shape) {
    let base = entity.base();
    let label = base.label.as_deref().is_some_and(|l| !l.is_empty());
    (
        f64::from(base.id),
        base.layer_id.as_str(),
        label,
        shape(entity),
    )
}

pub(crate) fn core(p: Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

pub(crate) fn plain(p: CoreVec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

fn ids(slots: &[Slot]) -> Vec<f64> {
    slots.iter().map(|s| f64::from(s.0)).collect()
}

/// A numbered corner as the core finds it; the numbering names it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoreCorner {
    pub p: Vec2,
    /// Unit vector away from the shape at this corner (for text placement).
    pub out: Vec2,
    /// k ≥ 0: the run's k-th new number (made where it first appears);
    /// −1 − j: existing point j's.
    pub refers: i64,
}

/// An edge-length label, with the object it belongs to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeLabel {
    pub id: Slot,
    /// The text's anchor (baseline centre).
    pub p: Vec2,
    /// Degrees counter-clockwise from east, always readable.
    pub rotation: f64,
    /// The edge's length (an arc's length for an arc edge).
    pub length: f64,
}

/// A target found for an input (docs/adr/0215 §2.1): their places in the two
/// lists, the distance, the two nearest points and the bearing from the first
/// to the second (radians, clockwise from north; NaN when 0 apart).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NearestFound {
    pub input: usize,
    pub target: usize,
    pub d: f64,
    pub a: Vec2,
    pub b: Vec2,
    pub bearing: f64,
}

/// How two areas neighbour (docs/adr/0215 §2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NeighborKind {
    Edge,
    Corner,
    Overlap,
}

/// A neighbour of an area: their places, how they neighbour, the shared
/// boundary (m) and the overlapping area (m²).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NeighborFound {
    pub area: usize,
    pub neighbor: usize,
    pub kind: NeighborKind,
    pub length: f64,
    pub overlap: f64,
}

/// The geometry of a run's inputs: a store of their objects, by id.
pub struct RunGeometry {
    store: Store,
}

impl RunGeometry {
    pub fn of<'a>(objects: impl IntoIterator<Item = &'a Entity>) -> Self {
        let mut store = Store::new();
        store.put_many(objects.into_iter().map(record));
        Self { store }
    }

    /// The store itself: the shapes of the run's objects by id.
    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Geometry values of these objects for expressions, six numbers each
    /// (`kentos_style_core::expr::rows`, `measures`).
    pub fn measures(&self, slots: &[Slot]) -> Vec<f64> {
        self.store.measures(&ids(slots))
    }

    /// Corner numbering of these objects in the given order (polygons: outer
    /// ring, then holes; polylines; other kinds have no corners);
    /// `existing`: numbered points a corner may take (with `shared`).
    pub fn number_corners(
        &self,
        slots: &[Slot],
        walk: &CornerWalk,
        existing: &[Vec2],
    ) -> Vec<CoreCorner> {
        let existing: Vec<CoreVec2> = existing.iter().copied().map(core).collect();
        self.store
            .number_corners(&ids(slots), walk, &existing)
            .chunks_exact(CORNER_STRIDE)
            .map(|r| CoreCorner {
                p: Vec2 { x: r[0], y: r[1] },
                out: Vec2 { x: r[2], y: r[3] },
                refers: r[4] as i64,
            })
            .collect()
    }

    /// Where the texts beside numbered corners go, `texts` measured in the drawing's typeface.
    pub fn corner_texts(
        &self,
        corners: &[(Vec2, Vec2)],
        texts: &[String],
        height: f64,
        font: &str,
    ) -> Vec<Vec2> {
        let font = Font::from_id(font);
        corners
            .iter()
            .zip(texts)
            .map(|((p, out), t)| plain(corner_text_at(core(*p), core(*out), t, height, font)))
            .collect()
    }

    /// Which inputs stand in the relation to which references (docs/adr/0200
    /// §1): (input position, reference position) pairs, inputs first, then
    /// references in their order; an object is never paired with itself.
    /// Ayrık is asked as Kesişen and turned round by the caller.
    pub fn relate_pairs(
        &self,
        inputs: &[Slot],
        references: &[Slot],
        relation: Relation,
        within: f64,
    ) -> Vec<(usize, usize)> {
        self.store
            .relate_pairs(&ids(inputs), &ids(references), relation, within)
            .chunks_exact(2)
            .map(|p| (p[0] as usize, p[1] as usize))
            .collect()
    }

    /// Each input's nearest targets (docs/adr/0215 §2.1): `k` of them (0:
    /// all) within `max` (infinite: no bound), by distance then the targets'
    /// order; an object is never its own target.
    pub fn nearest(
        &self,
        inputs: &[Slot],
        targets: &[Slot],
        k: usize,
        max: f64,
        measure: Measure,
    ) -> Vec<NearestFound> {
        self.store
            .nearest(&ids(inputs), &ids(targets), k, max, measure)
            .chunks_exact(NEAREST_STRIDE)
            .map(|r| NearestFound {
                input: r[0] as usize,
                target: r[1] as usize,
                d: r[2],
                a: Vec2 { x: r[3], y: r[4] },
                b: Vec2 { x: r[5], y: r[6] },
                bearing: r[7],
            })
            .collect()
    }

    /// The areas' neighbours (docs/adr/0215 §2.2), both ways round, in the list's order.
    pub fn neighbors(
        &self,
        slots: &[Slot],
        tolerance: f64,
        corners: bool,
        overlaps: bool,
    ) -> Vec<NeighborFound> {
        self.store
            .neighbors(&ids(slots), tolerance, corners, overlaps)
            .chunks_exact(NEIGHBOR_STRIDE)
            .map(|r| NeighborFound {
                area: r[0] as usize,
                neighbor: r[1] as usize,
                kind: if r[2] == NEIGHBOR_OVERLAP {
                    NeighborKind::Overlap
                } else if r[2] == NEIGHBOR_CORNER {
                    NeighborKind::Corner
                } else {
                    NeighborKind::Edge
                },
                length: r[3],
                overlap: r[4],
            })
            .collect()
    }

    /// Edge-length labels of these objects in order (lines, polylines,
    /// polygons), and how many shared edges were written once.
    pub fn edge_lengths(
        &self,
        slots: &[Slot],
        height: f64,
        min_length: f64,
        inside: bool,
        shared: bool,
    ) -> (Vec<EdgeLabel>, usize) {
        let r = self
            .store
            .edge_lengths(&ids(slots), height, min_length, inside, shared);
        let skipped = r.first().copied().unwrap_or(0.0) as usize;
        let labels = r
            .get(1..)
            .unwrap_or(&[])
            .chunks_exact(EDGE_LABEL_STRIDE)
            .map(|l| EdgeLabel {
                id: Slot(l[0] as u32),
                p: Vec2 { x: l[1], y: l[2] },
                rotation: l[3],
                length: l[4],
            })
            .collect();
        (labels, skipped)
    }
}
