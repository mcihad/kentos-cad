//! The geometry processing tools ask for, from the shared geometry store
//! (docs/adr/0008, S4; the web's `processing/geometry.ts`). A run puts the
//! objects it reads into a store of its own, as the web's page and worker
//! do, and tools ask it by id: the expressions' geometry values, corner
//! numbering, the texts beside numbered corners, edge-length labels. This
//! only packs and reads: no coordinate is computed here.

use kentos_contracts::{Entity, Vec2};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::processing::numbering::{CornerWalk, corner_text_at};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::processing::{CORNER_STRIDE, EDGE_LABEL_STRIDE};
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
