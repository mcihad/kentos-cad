//! Topolojik düzenleme's neighbours (docs/adr/0160, 0254 §8): the objects
//! around an edit whose shared corners and edges go with it. What the edit
//! changed and how the neighbours follow are the core's
//! (`topology_edit::changes`, `topology_edit::apply`); the neighbours are
//! the shown objects within 1 µm of the places the edit changed, looked up
//! in the geometry store (a locked layer's are counted, never changed).
//! Esnet's command finds them here; the tools' grips and the vertex table
//! ask the same (`kentos_interaction::neighbours`), so a tool and a command
//! follow the same neighbours.

use kentos_contracts::EntityEdit;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::bulge::has_bulges;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::topology_edit::{self, Change, Neighbour, SAME};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::vec2::Vec2;

use crate::geometry::{edit_geometry, shape};

/// The neighbours an edit puts right: their writes, their shapes for the
/// preview, how many are locked or would be left invalid.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Neighbours {
    pub changes: Vec<EntityEdit>,
    pub shapes: Vec<Shape>,
    pub locked: usize,
    pub invalid: usize,
}

/// A neighbour found: its slot, its shape and whether its layer is locked.
pub type Found = Vec<(Slot, Neighbour)>;

/// A store's id as a slot.
pub fn slot(id: f64) -> Option<Slot> {
    (id >= 0.0 && id <= f64::from(u32::MAX) && id.fract() == 0.0).then_some(Slot(id as u32))
}

/// The shown objects sharing a corner with the places `at` might be (lines,
/// paths, areas; points with `points`), the `edited` ones left out, in the
/// order the store finds them.
pub fn around(doc: &Document, store: &Store, points: bool, at: &[Vec2], edited: &[Slot]) -> Found {
    let mut seen = edited.to_vec();
    let mut found = Found::new();
    for p in at {
        let near = Bounds {
            min_x: p.x - SAME,
            min_y: p.y - SAME,
            max_x: p.x + SAME,
            max_y: p.y + SAME,
        };
        for s in store.in_rect(&near, true).into_iter().filter_map(slot) {
            if seen.contains(&s) {
                continue;
            }
            seen.push(s);
            let Some(e) = doc.get(s) else {
                continue;
            };
            let shape = shape(e);
            let shares = match shape {
                Shape::Line { .. } | Shape::Polyline { .. } | Shape::Polygon { .. } => true,
                Shape::Point { .. } => points,
                _ => false,
            };
            if shares {
                let locked = doc.layers().is_locked(&e.base().layer_id);
                found.push((s, Neighbour { shape, locked }));
            }
        }
    }
    // In the drawing's order, whatever the store's: a host's and a headless one's answer alike.
    found.sort_by_key(|(s, _)| *s);
    found
}

/// `found` put right by `changes` (the core's `apply`): their writes and
/// shapes, the locked and invalid counts.
pub fn put_right(doc: &Document, points: bool, found: &Found, changes: &[Change]) -> Neighbours {
    let list: Vec<Neighbour> = found.iter().map(|(_, n)| n.clone()).collect();
    let answer = topology_edit::apply(&list, changes, points);
    let mut out = Neighbours {
        locked: answer.locked,
        invalid: answer.invalid,
        ..Neighbours::default()
    };
    for (index, shape) in answer.edited {
        let Some(&(s, _)) = found.get(index) else {
            continue;
        };
        let shape = straight(shape);
        let (Some(uid), Some(geometry)) = (doc.uid(s), edit_geometry(shape.clone())) else {
            continue;
        };
        out.changes.push(EntityEdit::Update {
            uid: uid.to_string(),
            geometry,
        });
        out.shapes.push(shape);
    }
    out
}

/// A path whose bulges are all zero as the drawing keeps it: without them
/// (the core's `apply` gives every edge's).
fn straight(shape: Shape) -> Shape {
    match shape {
        Shape::Polyline {
            pts,
            bulges,
            holes,
            parts,
        } => Shape::Polyline {
            pts,
            bulges: bulges.filter(|b| has_bulges(Some(b))),
            holes,
            parts,
        },
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => Shape::Polygon {
            pts,
            bulges: bulges.filter(|b| has_bulges(Some(b))),
            holes,
            parts,
        },
        other => other,
    }
}

/// The neighbours the edited objects (each its slot, its shape before and
/// after) put right together in `doc`, found in `store` (in step with it);
/// `points`: Noktalar da. `None` when nothing is shared.
pub fn neighbours(
    doc: &Document,
    store: &Store,
    points: bool,
    edits: &[(Slot, Shape, Shape)],
) -> Option<Neighbours> {
    let changes: Vec<Change> = edits
        .iter()
        .filter(|(_, before, _)| points || !matches!(before, Shape::Point { .. }))
        .flat_map(|(_, before, after)| topology_edit::changes(before, after))
        .collect();
    if changes.is_empty() {
        return None;
    }
    let at: Vec<Vec2> = changes.iter().map(anchor).collect();
    let edited: Vec<Slot> = edits.iter().map(|(slot, ..)| *slot).collect();
    let found = around(doc, store, points, &at, &edited);
    (!found.is_empty()).then(|| put_right(doc, points, &found, &changes))
}

/// Where a change's neighbours have a corner: a move's and a removal's
/// vertex, an edge's first end.
pub fn anchor(c: &Change) -> Vec2 {
    match *c {
        Change::Move { at, .. } | Change::Remove { at, .. } => at,
        Change::Insert { a, .. } | Change::Bulge { a, .. } => a,
    }
}
