//! Topological editing (docs/adr/0160): the objects around an edit whose
//! shared corners and edges go with it, while the mode is on (the web's
//! `tools/neighbours.ts`). What the edit changed and how the neighbours
//! follow are the core's (`topology_edit::changes`, `topology_edit::apply`);
//! the neighbours are the visible objects within 1 µm of the places the edit
//! changed, looked up in the geometry store (a locked layer's are counted,
//! never changed). Both platforms play
//! `fixtures/interaction/v1/topology-edit.json`. A host writing outside a
//! tool (Köşe tablosu, docs/adr/0172 §6) asks [`neighbours_in`] with its
//! drawing and store, and says [`lines`].

use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::topology_edit::Change;
use kentos_native_application::topology_follow;
pub use kentos_native_application::topology_follow::{Found, Neighbours};

use crate::Vec2;
use crate::log::Level;
use crate::spatial::Spatial;
use crate::tool::Context;

/// The visible objects sharing a corner with the places `at` might be
/// (lines, paths, areas; points with Noktalar da), the `edited` ones left
/// out, in the order the store finds them (the web's `neighboursAt`).
pub(crate) fn around(cx: &Context<'_>, at: &[Vec2], edited: &[Slot]) -> Found {
    topology_follow::around(
        cx.doc,
        cx.spatial.store(),
        cx.draft.topology_points,
        at,
        edited,
    )
}

/// `found` put right by `changes` (the core's `apply`): their writes and
/// shapes, the locked and invalid counts (the web's `putRight`).
pub(crate) fn put_right(cx: &Context<'_>, found: &Found, changes: &[Change]) -> Neighbours {
    topology_follow::put_right(cx.doc, cx.draft.topology_points, found, changes)
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
    neighbours_in(cx.doc, cx.spatial, cx.draft.topology_points, edits)
}

/// The neighbours the edited objects (each its slot, its shape before and
/// after) put right together in `doc`, found by `spatial` (in step with it),
/// for a host writing outside a tool while the mode is on (Köşe tablosu,
/// docs/adr/0172 §6); `points`: Noktalar da. `None` when nothing is shared.
pub fn neighbours_in(
    doc: &Document,
    spatial: &Spatial,
    points: bool,
    edits: &[(Slot, Shape, Shape)],
) -> Option<Neighbours> {
    topology_follow::neighbours(doc, spatial.store(), points, edits)
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
    for (level, text) in n.map(lines).unwrap_or_default() {
        cx.say(level, text);
    }
}

/// What to say of the neighbours once the edit is written, in order: how
/// many changed with it, how many could not.
pub fn lines(n: &Neighbours) -> Vec<(Level, String)> {
    let mut out = Vec::new();
    if !n.changes.is_empty() {
        out.push((
            Level::Info,
            format!(
                "Topolojik düzenleme: {} komşu nesne de değişti.",
                n.changes.len()
            ),
        ));
    }
    if n.locked > 0 {
        out.push((
            Level::Warn,
            format!(
                "Kilitli katmandaki {} komşu nesne değişmedi; ortak sınır ayrıldı.",
                n.locked
            ),
        ));
    }
    if n.invalid > 0 {
        out.push((
            Level::Warn,
            format!(
                "{} komşu nesne geçersiz kalacağı için değişmedi (açık yolda 2'den, halkada 3'ten az köşe).",
                n.invalid
            ),
        ));
    }
    out
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
