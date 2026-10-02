//! Topological editing (docs/adr/0160): the objects around an edit whose
//! shared corners and edges go with it, while the mode is on (the web's
//! `tools/neighbours.ts`). What the edit changed and how the neighbours
//! follow are the core's (`topology_edit::changes`, `topology_edit::apply`);
//! the neighbours are the visible objects within 1 µm of the places the edit
//! changed, looked up in the geometry store (a locked layer's are counted,
//! never changed). Both platforms play
//! `fixtures/interaction/v1/topology-edit.json`.

use kentos_contracts::EntityEdit;
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::topology_edit::{self, Change, Neighbour, SAME};
use kentos_native_application::geometry::{edit_geometry, shape};

use crate::Vec2;
use crate::log::Level;
use crate::tool::Context;

/// The neighbours an edit puts right: their writes, their shapes for the
/// preview, how many are locked or would be left invalid.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Neighbours {
    pub changes: Vec<EntityEdit>,
    pub shapes: Vec<Shape>,
    pub locked: usize,
    pub invalid: usize,
}

/// The neighbours the object at `slot` becoming `after` (from `before`)
/// puts right; `None` while the mode is off or nothing is shared.
pub(crate) fn neighbours(
    cx: &Context<'_>,
    slot: Slot,
    before: &Shape,
    after: &Shape,
) -> Option<Neighbours> {
    if !cx.draft.topology {
        return None;
    }
    let points = cx.draft.topology_points;
    // A point is a corner only with Noktalar da.
    if matches!(before, Shape::Point { .. }) && !points {
        return None;
    }
    let changes = topology_edit::changes(before, after);
    if changes.is_empty() {
        return None;
    }
    let doc = &*cx.doc;
    let mut seen = vec![slot];
    let mut found: Vec<(Slot, Neighbour)> = Vec::new();
    for at in changes.iter().map(anchor) {
        let near = cx.spatial.in_rect(
            Vec2::new(at.x - SAME, at.y - SAME),
            Vec2::new(at.x + SAME, at.y + SAME),
            true,
        );
        for s in near {
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
    if found.is_empty() {
        return None;
    }
    let list: Vec<Neighbour> = found.iter().map(|(_, n)| n.clone()).collect();
    let answer = topology_edit::apply(&list, &changes, points);
    let mut out = Neighbours {
        locked: answer.locked,
        invalid: answer.invalid,
        ..Neighbours::default()
    };
    for (index, shape) in answer.edited {
        let Some(&(s, _)) = found.get(index) else {
            continue;
        };
        let (Some(uid), Some(geometry)) = (doc.uid(s), edit_geometry(shape.clone())) else {
            continue;
        };
        out.changes.push(EntityEdit::Update {
            uid: uid.to_string(),
            geometry,
        });
        out.shapes.push(shape);
    }
    Some(out)
}

/// Says what the neighbours did, once the edit is written: how many changed
/// with it, how many could not (the web's `sayNeighbours`).
pub(crate) fn say(n: Option<&Neighbours>, cx: &mut Context<'_>) {
    let Some(n) = n else {
        return;
    };
    if !n.changes.is_empty() {
        cx.say(
            Level::Info,
            format!(
                "Topolojik düzenleme: {} komşu nesne de değişti.",
                n.changes.len()
            ),
        );
    }
    if n.locked > 0 {
        cx.say(
            Level::Warn,
            format!(
                "Kilitli katmandaki {} komşu nesne değişmedi; ortak sınır ayrıldı.",
                n.locked
            ),
        );
    }
    if n.invalid > 0 {
        cx.say(
            Level::Warn,
            format!(
                "{} komşu nesne geçersiz kalacağı için değişmedi (açık yolda 2'den, halkada 3'ten az köşe).",
                n.invalid
            ),
        );
    }
}

/// Where a change's neighbours have a corner: a move's and a removal's
/// vertex, an edge's first end.
fn anchor(c: &Change) -> Vec2 {
    match *c {
        Change::Move { at, .. } | Change::Remove { at, .. } => at,
        Change::Insert { a, .. } | Change::Bulge { a, .. } => a,
    }
}
