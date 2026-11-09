//! Topolojik düzenleme's neighbours (docs/adr/0160, 0254 §8): the objects
//! around an edit whose shared corners and edges go with it. What the edit
//! changed and how the neighbours follow are the core's
//! (`topology_edit::changes`, `topology_edit::apply`); the neighbours are
//! the shown objects within 1 µm of the places the edit changed, looked up
//! in the geometry store (a locked layer's are counted, never changed).
//! Esnet's command finds them here; the tools' grips and the vertex table
//! ask the same (`kentos_interaction::neighbours`), so a tool and a command
//! follow the same neighbours.

use kentos_commands::EntityEdit;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::{Part, Shape};
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::geom::bulge::has_bulges;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::topology_edit::{self, Change, Neighbour, SAME};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::vec2::Vec2;

use crate::geometry::{edit_geometry, shape};

/// The neighbours an edit puts right: their writes, their shapes for the
/// preview, how many are locked or would be left invalid, and at how many
/// places inside the edges the edit changed an unlocked neighbour's vertex
/// went with its edge (docs/adr/3400).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Neighbours {
    pub changes: Vec<EntityEdit>,
    pub shapes: Vec<Shape>,
    pub locked: usize,
    pub invalid: usize,
    pub carried: usize,
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
/// (the core's `apply` gives every edge's), its holes' and parts' too.
pub(crate) fn straight(shape: Shape) -> Shape {
    let bent = |b: Option<Vec<f64>>| b.filter(|b| has_bulges(Some(b)));
    let rings = |hs: Option<Vec<Ring>>| {
        hs.map(|hs| {
            hs.into_iter()
                .map(|r| Ring {
                    pts: r.pts,
                    bulges: bent(r.bulges),
                })
                .collect()
        })
    };
    let parts_of = |ps: Option<Vec<Part>>| {
        ps.map(|ps| {
            ps.into_iter()
                .map(|p| Part {
                    pts: p.pts,
                    bulges: bent(p.bulges),
                    holes: rings(p.holes),
                })
                .collect()
        })
    };
    match shape {
        Shape::Polyline {
            pts,
            bulges,
            holes,
            parts,
            closed,
            widths,
        } => Shape::Polyline {
            pts,
            bulges: bent(bulges),
            holes: rings(holes),
            parts: parts_of(parts),
            closed,
            widths,
        },
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts,
        } => Shape::Polygon {
            pts,
            bulges: bent(bulges),
            holes: rings(holes),
            parts: parts_of(parts),
        },
        other => other,
    }
}

/// The neighbours the edited objects (each its slot, its shape before and
/// after) put right together in `doc`, found in `store` (in step with it);
/// `points`: Noktalar da. `None` when nothing is shared. The neighbours'
/// vertices inside an edge the edit changed go with it (docs/adr/3400,
/// [`inside_edges`]): Esnet, the grips, Köşe tablosu and the vertex command
/// alike.
pub fn neighbours(
    doc: &Document,
    store: &Store,
    points: bool,
    edits: &[(Slot, Shape, Shape)],
) -> Option<Neighbours> {
    let (extra, carried) = inside_edges(doc, store, points, edits, false);
    neighbours_with(doc, store, points, edits, &extra).map(|mut n| {
        n.carried = carried;
        n
    })
}

/// What the edges the edits changed carry (docs/adr/3400; the core's `topology_edit::carried`): the shown neighbours'
/// vertices strictly inside each edge as it was go to the same fraction of it as it is (moves), the neighbours' parts
/// along an edge whose bulge changed bend with it; and at how many places an unlocked neighbour has such a vertex (a
/// locked one's stays, as its corners do). With `edited_too` the other edited objects' vertices (as they were) count
/// as well, for a caller that puts them right in its own write ([`carry_edited`], Esnet); without it a place where an
/// edited object has a vertex is left, so that a T junction it shares is not split.
pub fn inside_edges(
    doc: &Document,
    store: &Store,
    points: bool,
    edits: &[(Slot, Shape, Shape)],
    edited_too: bool,
) -> (Vec<Change>, usize) {
    let edited: Vec<Slot> = edits.iter().map(|(slot, ..)| *slot).collect();
    let (mut extra, mut carried) = (Vec::new(), 0);
    for (own, before, after) in edits {
        if matches!(before, Shape::Point { .. }) {
            continue;
        }
        let edges = topology_edit::changed_edges(before, after);
        if edges.is_empty() {
            continue;
        }
        let mut found = Found::new();
        for &((a, b, bulge), _) in &edges {
            let near = match kentos_geometry_core::geom::bulge::bulge_arc(a, b, bulge) {
                Some(arc) if bulge != 0.0 => Bounds {
                    min_x: arc.c.x - arc.r,
                    min_y: arc.c.y - arc.r,
                    max_x: arc.c.x + arc.r,
                    max_y: arc.c.y + arc.r,
                },
                _ => Bounds { min_x: a.x.min(b.x), min_y: a.y.min(b.y), max_x: a.x.max(b.x), max_y: a.y.max(b.y) },
            };
            for (s, n) in within(doc, store, points, &near, &edited) {
                if !found.iter().any(|(f, _)| *f == s) {
                    found.push((s, n));
                }
            }
        }
        found.sort_by_key(|(s, _)| *s);
        let mut list: Vec<Neighbour> = found.into_iter().map(|(_, n)| n).collect();
        // The other edited objects as they were: their vertices inside the edge go too, or the place is left.
        let others: Vec<Neighbour> = edits
            .iter()
            .filter(|(s, ..)| s != own)
            .map(|(_, b, _)| Neighbour { shape: b.clone(), locked: false })
            .collect();
        let held: Vec<Vec2> = topology_edit::carried(&others, &edges, points)
            .iter()
            .filter_map(|c| match c {
                Change::Move { at, .. } => Some(*at),
                _ => None,
            })
            .collect();
        let kept = |c: &Change| edited_too || !matches!(c, Change::Move { at, .. } if held.iter().any(|h| h.x == at.x && h.y == at.y));
        if edited_too {
            list.extend(others);
        }
        extra.extend(topology_edit::carried(&list, &edges, points).into_iter().filter(|c| kept(c)));
        let unlocked: Vec<Neighbour> = list.into_iter().filter(|n| !n.locked).collect();
        carried += topology_edit::carried(&unlocked, &edges, points)
            .iter()
            .filter(|c| matches!(c, Change::Move { .. }) && kept(c))
            .count();
    }
    (extra, carried)
}

/// The edited objects the carried moves (`extra`, [`inside_edges`] with `edited_too`) put right: each one with a vertex
/// at a carried place, its shape as edited with that vertex moved (Esnet's other stretched objects sharing a T junction
/// with the edge). Points only with `points`.
pub fn carry_edited(edits: &[(Slot, Shape, Shape)], extra: &[Change], points: bool) -> Vec<(Slot, Shape)> {
    let moves: Vec<Change> = extra.iter().filter(|c| matches!(c, Change::Move { .. })).cloned().collect();
    if moves.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (slot, _, after) in edits {
        let list = [Neighbour { shape: after.clone(), locked: false }];
        if let Some((_, shape)) = topology_edit::apply(&list, &moves, points).edited.into_iter().next() {
            out.push((*slot, straight(shape)));
        }
    }
    out
}

/// [`neighbours`] with `extra` moves besides the edits' own: Paralel kaydır's vertices inside a shared edge
/// ([`along_edge`], docs/adr/2600).
pub fn neighbours_with(
    doc: &Document,
    store: &Store,
    points: bool,
    edits: &[(Slot, Shape, Shape)],
    extra: &[Change],
) -> Option<Neighbours> {
    let changes: Vec<Change> = edits
        .iter()
        .filter(|(_, before, _)| points || !matches!(before, Shape::Point { .. }))
        .flat_map(|(_, before, after)| topology_edit::changes(before, after))
        .chain(extra.iter().cloned())
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

/// Paralel kaydır's refusal when a neighbour's vertex inside the shared edge cannot go with it (docs/adr/2600).
pub const OFF_EDGE: &str = "Kenarın ortasındaki ortak köşe kaydırılan kenarın dışına düşüyor: kenar o köşeyi geçecek kadar kısalıyor. Daha kısa bir uzaklık yazın ya da Topolojik düzenlemeyi kapatın.";
/// The same inside a shared arc edge: an arc's shift does not carry a vertex on it.
pub const ON_ARC: &str = "Yaylı ortak kenarın ortasında komşu nesnenin köşesi var; yay kaydırılınca o köşe kenardan ayrılır. Topolojik düzenlemeyi kapatın ya da o köşeyi kaldırın.";

/// A ring's edge `edge`: its ends and bulge (ring 0 an area's outer ring or a path, 1… an area's holes).
fn ring_edge(s: &Shape, ring: usize, edge: usize) -> Option<(Vec2, Vec2, f64)> {
    let (pts, bulges, closed) = match (s, ring) {
        (Shape::Polygon { pts, bulges, .. }, 0) => (pts, bulges.as_deref(), true),
        (Shape::Polygon { holes, .. }, k) => {
            let h = holes.as_ref()?.get(k - 1)?;
            (&h.pts, h.bulges.as_deref(), true)
        }
        (Shape::Polyline { pts, bulges, closed, .. }, 0) => (pts, bulges.as_deref(), *closed),
        _ => return None,
    };
    let n = pts.len();
    if edge >= n || (!closed && edge + 1 >= n) {
        return None;
    }
    let bulge = bulges.and_then(|b| b.get(edge)).copied().unwrap_or(0.0);
    Some((pts[edge], pts[(edge + 1) % n], bulge))
}

/// The shown objects within [`SAME`] of `near` (lines, paths, areas; points with `points`), the `edited` ones left out,
/// in the drawing's order.
fn within(doc: &Document, store: &Store, points: bool, near: &Bounds, edited: &[Slot]) -> Found {
    let rect = Bounds {
        min_x: near.min_x - SAME,
        min_y: near.min_y - SAME,
        max_x: near.max_x + SAME,
        max_y: near.max_y + SAME,
    };
    let mut found = Found::new();
    for s in store.in_rect(&rect, true).into_iter().filter_map(slot) {
        if edited.contains(&s) || found.iter().any(|(f, _)| *f == s) {
            continue;
        }
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
    found.sort_by_key(|(s, _)| *s);
    found
}

/// What Paralel kaydır's shared edge carries (docs/adr/2600; ArcGIS Pro's “topologically connected edges stretch and
/// remain connected”): the shown neighbours' vertices strictly inside edge `edge` of ring `ring` of object `slot`
/// (`before`; `after` the object with the edge moved), a third object's corner on it (a T junction) or a neighbour's
/// own vertex along it, each moved square to the edge by as much as the edge moved, as moves for [`neighbours_with`];
/// and how many of these places an unlocked neighbour has (a locked one's stay, as its corners do). Refused, with the
/// input's path: a vertex the shortened edge no longer reaches ([`OFF_EDGE`]), a vertex inside an arc edge
/// ([`ON_ARC`]). Nothing inside: no moves.
#[allow(clippy::too_many_arguments)]
pub fn along_edge(
    doc: &Document,
    store: &Store,
    points: bool,
    slot: Slot,
    before: &Shape,
    after: &Shape,
    ring: usize,
    edge: usize,
) -> Result<(Vec<Change>, usize), (&'static str, &'static str)> {
    let (Some((a, b, bulge)), Some((a2, b2, _))) = (ring_edge(before, ring, edge), ring_edge(after, ring, edge)) else {
        return Ok((Vec::new(), 0));
    };
    if bulge != 0.0 {
        let Some(arc) = kentos_geometry_core::geom::bulge::bulge_arc(a, b, bulge) else {
            return Ok((Vec::new(), 0));
        };
        let near = Bounds {
            min_x: arc.c.x - arc.r,
            min_y: arc.c.y - arc.r,
            max_x: arc.c.x + arc.r,
            max_y: arc.c.y + arc.r,
        };
        let found = within(doc, store, points, &near, &[slot]);
        let list: Vec<Neighbour> = found.into_iter().map(|(_, n)| n).collect();
        return if topology_edit::inside_arc(&list, arc.c, arc.r, arc.a0, arc.sweep, points).is_empty() {
            Ok((Vec::new(), 0))
        } else {
            Err((ON_ARC, "work.at"))
        };
    }
    let near = Bounds {
        min_x: a.x.min(b.x),
        min_y: a.y.min(b.y),
        max_x: a.x.max(b.x),
        max_y: a.y.max(b.y),
    };
    let found = within(doc, store, points, &near, &[slot]);
    let list: Vec<Neighbour> = found.into_iter().map(|(_, n)| n).collect();
    let inside = topology_edit::inside_segment(&list, a, b, points);
    if inside.is_empty() {
        return Ok((Vec::new(), 0));
    }
    // The edge's move square to itself: the new line's distance from the old one along the old one's left normal.
    let l = (b.x - a.x).hypot(b.y - a.y);
    let n = Vec2::new(-(b.y - a.y) / l, (b.x - a.x) / l);
    let by = (a2.x - a.x) * n.x + (a2.y - a.y) * n.y;
    let (ex, ey) = (b2.x - a2.x, b2.y - a2.y);
    let l2 = ex.hypot(ey);
    let mut moves = Vec::with_capacity(inside.len());
    for v in inside {
        let to = Vec2::new(v.x + n.x * by, v.y + n.y * by);
        let along = ((to.x - a2.x) * ex + (to.y - a2.y) * ey) / l2;
        if !(along > SAME && l2 - along > SAME) {
            return Err((OFF_EDGE, "work.distance"));
        }
        moves.push(Change::Move { at: v, to });
    }
    let unlocked: Vec<Neighbour> = list.into_iter().filter(|n| !n.locked).collect();
    let carried = topology_edit::inside_segment(&unlocked, a, b, points).len();
    Ok((moves, carried))
}
