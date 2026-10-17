//! What a selected associative dimension is bound to (docs/adr/0320 §4, plan/cad.md `CAD-ANNO-04`): for each of its
//! live references, the place on the object and the object's edges (the piece's, when the place is inside an
//! insert), for the drawing to show over the selection. Read from the document; nothing is written.

use kentos_contracts::{DimensionRefs, Entity};
use kentos_domain::dimension_ties::{core_blocks, core_place};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::jsmath::{cos, sin};
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::ops::subref::{piece, point};
use kentos_native_application::blocks::core_entity;

use crate::Vec2;

/// At most this many selected dimensions show their bonds: a large selection is not slowed down by them.
pub const MOST_DIMENSIONS: usize = 64;

/// One live reference of a selected dimension.
#[derive(Clone, Debug, PartialEq)]
pub struct Bond {
    /// The place on the object, where it is now.
    pub at: Vec2,
    /// The object's (or the piece's) edges, each a run of points (an arc in short chords).
    pub edges: Vec<Vec<Vec2>>,
}

/// The bonds of the selected dimensions among `slots` (the first [`MOST_DIMENSIONS`] of them).
pub fn bonds(doc: &Document, slots: &[Slot]) -> Vec<Bond> {
    let mut out = Vec::new();
    let mut blocks = None;
    let dims = slots
        .iter()
        .filter_map(|s| match doc.get(*s) {
            Some(Entity::Dimension(d)) => d.refs.as_deref(),
            _ => None,
        })
        .take(MOST_DIMENSIONS);
    for refs in dims {
        bonds_of(doc, refs, &mut blocks, &mut out);
    }
    out
}

fn bonds_of(
    doc: &Document,
    refs: &DimensionRefs,
    blocks: &mut Option<kentos_geometry_core::block::Blocks>,
    out: &mut Vec<Bond>,
) {
    for (_, r) in refs.each().filter(|(_, r)| r.live()) {
        let Some(e) = doc
            .slot_of(Uuid::from_bytes(r.uid.0))
            .and_then(|s| doc.get(s))
        else {
            continue;
        };
        let mut object = core_entity(e);
        if !r.path.is_empty() {
            let blocks = blocks.get_or_insert_with(|| core_blocks(doc.blocks()));
            let Some(p) = piece(blocks, &object, &r.path) else {
                continue;
            };
            object = p;
        }
        let Some(at) = point(&object, core_place(r.place)) else {
            continue;
        };
        let edges = entity_edges(&object.shape).iter().map(chords).collect();
        out.push(Bond { at, edges });
    }
}

/// An edge as a run of points: a segment's two ends, an arc's chords of at most 1° (a chord of 10° stood 5.7 mm
/// inside a 1.5 m circle, beside the circle the scene draws when zoomed in).
fn chords(edge: &Edge) -> Vec<Vec2> {
    match *edge {
        Edge::Seg { a, b } => vec![a, b],
        Edge::Arc { c, r, a0, sweep } => {
            let n = ((sweep.abs() / 1f64.to_radians()).ceil() as usize).clamp(2, 720);
            (0..=n)
                .map(|i| {
                    let t = a0 + sweep * i as f64 / n as f64;
                    Vec2::new(c.x + r * cos(t), c.y + r * sin(t))
                })
                .collect()
        }
    }
}
