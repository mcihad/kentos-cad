//! Associative hatches in DXF (docs/adr/0333 §3): a hatch tied to objects that are written too is written associative
//! (71 1), each of its paths with its source objects' handles (97, 330s), and each source object with the hatch's
//! handle among its reactors ({ACAD_REACTORS}). Handles are taken for the hatches and their sources before the objects
//! are written, so a source written before its hatch already names it.
//!
//! The outer path's sources are the hatch's closed object, or the objects closing its face; a hole's are the islands
//! whose box is the hole's (within 2 % of its size): a hole a text leaves open, or one the islands do not match, has
//! none.

use std::collections::HashMap;

use kentos_contracts::{DxfWriteInput, Entity, EntityId, HatchEntity, Vec2};

use super::Handles;
use crate::math::{atan, atan2, cos, hypot, sin};

/// What the writer knows of the associative hatches before writing.
#[derive(Debug, Default)]
pub(super) struct Ties {
    /// The handle taken for an object, by object id.
    pub handles: HashMap<u32, u64>,
    /// The hatches a source object reacts to, by its id.
    pub reactors: HashMap<u32, Vec<u64>>,
    /// A tied hatch's paths' sources, by its id: the outer path's, then each hole's.
    pub sources: HashMap<u32, Vec<Vec<u64>>>,
}

/// A box `[min x, min y, max x, max y]` of points.
fn box_of(pts: impl IntoIterator<Item = Vec2>) -> Option<[f64; 4]> {
    let mut b: Option<[f64; 4]> = None;
    for p in pts {
        let c = b.get_or_insert([p.x, p.y, p.x, p.y]);
        *c = [c[0].min(p.x), c[1].min(p.y), c[2].max(p.x), c[3].max(p.y)];
    }
    b
}

/// A ring's points with its arcs (bulges) walked in 32 steps each: enough for a box within 2 %.
fn ring_points(pts: &[Vec2], bulges: Option<&[f64]>) -> Vec<Vec2> {
    let n = pts.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        out.push(a);
        let bulge = bulges.and_then(|bs| bs.get(i)).copied().unwrap_or(0.0);
        if bulge == 0.0 || n < 2 {
            continue;
        }
        // The arc from a to b turning 4·atan(bulge): its centre off the chord's middle.
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let sweep = 4.0 * atan(bulge);
        let k = (1.0 - bulge * bulge) / (4.0 * bulge);
        let c = Vec2 {
            x: (a.x + b.x) / 2.0 - k * dy,
            y: (a.y + b.y) / 2.0 + k * dx,
        };
        let (r, start) = (hypot(a.x - c.x, a.y - c.y), atan2(a.y - c.y, a.x - c.x));
        for s in 1..32 {
            let t = start + sweep * f64::from(s) / 32.0;
            out.push(Vec2 {
                x: c.x + r * cos(t),
                y: c.y + r * sin(t),
            });
        }
    }
    out
}

/// An island's box: its outline with its arcs, a circle's or a full ellipse's extent.
fn entity_box(e: &Entity) -> Option<[f64; 4]> {
    match e {
        Entity::Polygon(p) | Entity::Polyline(p) => {
            box_of(ring_points(&p.pts, p.bulges.as_deref()))
        }
        Entity::Circle(c) => Some([c.c.x - c.r, c.c.y - c.r, c.c.x + c.r, c.c.y + c.r]),
        Entity::Ellipse(e) => {
            let (mx, my) = (e.major.x, e.major.y);
            let (wx, wy) = (hypot(mx, e.ratio * my), hypot(my, e.ratio * mx));
            Some([e.c.x - wx, e.c.y - wy, e.c.x + wx, e.c.y + wy])
        }
        _ => None,
    }
}

/// Whether two boxes are one within 2 % of the larger's size.
fn same_box(a: [f64; 4], b: [f64; 4]) -> bool {
    let size = (a[2] - a[0])
        .max(a[3] - a[1])
        .max(b[2] - b[0])
        .max(b[3] - b[1]);
    let tol = (size * 0.02).max(1e-9);
    (0..4).all(|i| (a[i] - b[i]).abs() <= tol)
}

/// The hatch's paths as the writer writes them: its exact boundary's loops, else its ring and holes.
fn path_boxes(h: &HatchEntity) -> Vec<Option<[f64; 4]>> {
    match h.boundary.as_deref().filter(|b| !b.is_empty()) {
        Some(loops) => loops
            .iter()
            .map(|l| box_of(ring_points(&l.pts, l.bulges.as_deref())))
            .collect(),
        None => std::iter::once(box_of(h.ring.iter().copied()))
            .chain(h.holes.iter().flatten().map(|r| box_of(r.iter().copied())))
            .collect(),
    }
}

/// The handle taken for `e`, taking one the first time.
fn take(taken: &mut HashMap<u32, u64>, e: &Entity, handles: &mut Handles) -> u64 {
    *taken.entry(e.base().id).or_insert_with(|| handles.take())
}

/// The ties of the drawing's hatches whose objects are written too; their handles taken from `handles`.
pub(super) fn ties_of(input: &DxfWriteInput, handles: &mut Handles) -> Ties {
    let mut ties = Ties::default();
    if input.uids.is_empty() {
        return ties;
    }
    let by_uid: HashMap<EntityId, &Entity> = input
        .entities
        .iter()
        .filter_map(|e| input.uids.get(&e.base().id).map(|u| (*u, e)))
        .collect();
    for e in &input.entities {
        let Entity::Hatch(h) = e else {
            continue;
        };
        let Some(a) = &h.assoc else {
            continue;
        };
        // Sources written as one entity of their own with the common head (a part of a multi-part area is not).
        let source = |id: &EntityId| {
            by_uid.get(id).copied().filter(|e| match e {
                Entity::Polygon(p) | Entity::Polyline(p) => p.parts.is_none(),
                Entity::Line(_)
                | Entity::Circle(_)
                | Entity::Arc(_)
                | Entity::Ellipse(_)
                | Entity::Nurbs(_) => true,
                _ => false,
            })
        };
        let mine: Option<Vec<&Entity>> = match &a.outer {
            Some(o) => source(o).map(|e| vec![e]),
            None => a.bounds.iter().map(source).collect(),
        };
        // Every object of its outer path written: else it goes out following nothing.
        let Some(mine) = mine.filter(|m| !m.is_empty()) else {
            continue;
        };
        let islands: Vec<&Entity> = a.islands.iter().filter_map(source).collect();
        let me = take(&mut ties.handles, e, handles);
        let boxes = path_boxes(h);
        let mut paths: Vec<Vec<u64>> = Vec::with_capacity(boxes.len());
        let mut used: Vec<u32> = Vec::new();
        for (k, b) in boxes.iter().enumerate() {
            let list: Vec<&Entity> = if k == 0 {
                mine.clone()
            } else {
                islands
                    .iter()
                    .copied()
                    .filter(|i| !used.contains(&i.base().id))
                    .filter(
                        |i| matches!((b, entity_box(i)), (Some(b), Some(ib)) if same_box(*b, ib)),
                    )
                    .take(1)
                    .collect()
            };
            let mut handles_of = Vec::with_capacity(list.len());
            for s in list {
                used.push(s.base().id);
                let h = take(&mut ties.handles, s, handles);
                ties.reactors.entry(s.base().id).or_default().push(me);
                handles_of.push(h);
            }
            paths.push(handles_of);
        }
        ties.sources.insert(e.base().id, paths);
    }
    ties
}
