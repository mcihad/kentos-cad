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

/// A neighbour found: its slot, its shape and whether its layer is locked.
pub(crate) type Found = Vec<(Slot, Neighbour)>;

/// The visible objects sharing a corner with the places `at` might be
/// (lines, paths, areas; points with Noktalar da), the `edited` ones left
/// out, in the order the store finds them (the web's `neighboursAt`).
pub(crate) fn around(cx: &Context<'_>, at: &[Vec2], edited: &[Slot]) -> Found {
    let doc = &*cx.doc;
    let points = cx.draft.topology_points;
    let mut seen = edited.to_vec();
    let mut found = Found::new();
    for p in at {
        let near = cx.spatial.in_rect(
            Vec2::new(p.x - SAME, p.y - SAME),
            Vec2::new(p.x + SAME, p.y + SAME),
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
    found
}

/// `found` put right by `changes` (the core's `apply`): their writes and
/// shapes, the locked and invalid counts (the web's `putRight`).
pub(crate) fn put_right(cx: &Context<'_>, found: &Found, changes: &[Change]) -> Neighbours {
    let list: Vec<Neighbour> = found.iter().map(|(_, n)| n.clone()).collect();
    let answer = topology_edit::apply(&list, changes, cx.draft.topology_points);
    let mut out = Neighbours {
        locked: answer.locked,
        invalid: answer.invalid,
        ..Neighbours::default()
    };
    for (index, shape) in answer.edited {
        let Some(&(s, _)) = found.get(index) else {
            continue;
        };
        let (Some(uid), Some(geometry)) = (cx.doc.uid(s), edit_geometry(shape.clone())) else {
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

/// The neighbours the edited objects (each its slot, its shape before and
/// after) put right together; `None` while the mode is off or nothing is
/// shared. A point is a corner only with Noktalar da (the web's `neighboursOf`).
pub(crate) fn neighbours_of(
    cx: &Context<'_>,
    edits: &[(Slot, Shape, Shape)],
) -> Option<Neighbours> {
    if !cx.draft.topology {
        return None;
    }
    let points = cx.draft.topology_points;
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
    let found = around(cx, &at, &edited);
    (!found.is_empty()).then(|| put_right(cx, &found, &changes))
}

/// The neighbours the object at `slot` becoming `after` (from `before`)
/// puts right: a grip, the grip menu.
pub(crate) fn neighbours(
    cx: &Context<'_>,
    slot: Slot,
    before: &Shape,
    after: &Shape,
) -> Option<Neighbours> {
    neighbours_of(cx, &[(slot, before.clone(), after.clone())])
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

/// How many objects have a corner at `at`, the selected one at `own` among
/// them, while the mode is on (the grip's tag, docs/adr/0160 §5): the core
/// finds the corners as a move that stays in place would. `None` when only
/// `own` has one, for an object of a kind without corners, and for a point
/// without Noktalar da (the web's `cornerCount`).
pub(crate) fn corner_count(cx: &Context<'_>, own: Slot, at: Vec2) -> Option<usize> {
    use kentos_contracts::Entity;
    if !cx.draft.topology {
        return None;
    }
    let corners = match cx.doc.get(own)? {
        Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_) => true,
        Entity::Point(_) => cx.draft.topology_points,
        _ => false,
    };
    if !corners {
        return None;
    }
    let found = around(cx, &[at], &[own]);
    if found.is_empty() {
        return None;
    }
    let n = put_right(cx, &found, &[Change::Move { at, to: at }]);
    let count = 1 + n.shapes.len() + n.locked;
    (count > 1).then_some(count)
}

/// Where a change's neighbours have a corner: a move's and a removal's
/// vertex, an edge's first end.
pub(crate) fn anchor(c: &Change) -> Vec2 {
    match *c {
        Change::Move { at, .. } | Change::Remove { at, .. } => at,
        Change::Insert { a, .. } | Change::Bulge { a, .. } => a,
    }
}
