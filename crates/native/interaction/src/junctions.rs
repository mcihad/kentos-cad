//! Corners shared with the neighbours (docs/adr/0162 §4): while Topoloji is
//! on, a new area drawn by its outline is joined with the visible lines,
//! paths and areas it touches (points too with Noktalar da) corner by
//! corner: its corners on their edges go to them, theirs on its edges to
//! it, within 1 µm. The shared core finds them (`ops::adjoin::junctions`);
//! the neighbours are written through `cad.entities.edit` (`vertexAdd`,
//! their elevations carried along the edge) in the new area's undo step. A
//! locked layer's neighbour takes none and is counted. The web's is
//! `apps/web/src/tools/junctions.ts`; both play
//! `fixtures/interaction/v1/junctions.json`.

use kentos_contracts::{EditOperation, EntitiesEdit, EntityEdit};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::ops::adjoin::junctions;
use kentos_geometry_core::ops::topology_edit::{Neighbour, SAME};
use kentos_native_application::geometry::{edit_geometry, shape};
use kentos_native_application::{ExecutionContext, edit};

use crate::Vec2;
use crate::log::Level;
use crate::points;
use crate::tool::Context;

/// What joining a new area with its neighbours gives: the new area to
/// write, the neighbours' writes and what to say.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Joining {
    /// The new area's parts, the neighbours' corners on their edges added.
    pub areas: Vec<Area>,
    /// The neighbours given corners of the new area.
    pub changes: Vec<EntityEdit>,
    /// How many corners the new area took.
    pub taken: usize,
    /// How many corners the neighbours were given, all together.
    pub given: usize,
    /// How many neighbours would have been given one but lie on a locked layer.
    pub locked: usize,
}

/// The box of the areas' outer rings, generous on arcs (an arc stays within
/// twice its radius of its ends), and 1 µm round.
fn areas_box(areas: &[Area]) -> (Vec2, Vec2) {
    let mut min = Vec2::new(f64::INFINITY, f64::INFINITY);
    let mut max = Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY);
    for r in areas.iter().map(|a| &a.outer) {
        let n = r.pts.len();
        for (i, p) in r.pts.iter().enumerate() {
            let q = r.pts[(i + 1) % n];
            let k = r
                .bulges
                .as_ref()
                .and_then(|b| b.get(i))
                .copied()
                .unwrap_or(0.0)
                .abs();
            let chord = (q.x - p.x).hypot(q.y - p.y);
            let pad = SAME
                + if k > 0.0 {
                    chord * (1.0 + k * k) / (2.0 * k)
                } else {
                    0.0
                };
            min = Vec2::new(min.x.min(p.x - pad), min.y.min(p.y - pad));
            max = Vec2::new(max.x.max(p.x + pad), max.y.max(p.y + pad));
        }
    }
    (min, max)
}

/// The new area joined with its neighbours: the visible lines, paths and
/// areas near it (points with Noktalar da), a locked layer's counted.
/// `None` while Topoloji is off or nothing is shared.
pub(crate) fn join(cx: &Context<'_>, areas: &[Area]) -> Option<Joining> {
    if !cx.draft.topology || areas.is_empty() {
        return None;
    }
    let points = cx.draft.topology_points;
    let (min, max) = areas_box(areas);
    let doc = &*cx.doc;
    let mut found = Vec::new();
    let mut list = Vec::new();
    for s in cx.spatial.in_rect(min, max, true) {
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
            found.push(s);
            list.push(Neighbour { shape, locked });
        }
    }
    if list.is_empty() {
        return None;
    }
    let got = junctions(areas, &list, points);
    if got.taken == 0 && got.edited.is_empty() && got.locked == 0 {
        return None;
    }
    let changes = got
        .edited
        .into_iter()
        .filter_map(|(index, shape)| {
            let uid = doc.uid(*found.get(index)?)?;
            Some(EntityEdit::Update {
                uid: uid.to_string(),
                geometry: edit_geometry(shape)?,
            })
        })
        .collect();
    Some(Joining {
        areas: got.areas,
        changes,
        taken: got.taken,
        given: got.given,
        locked: got.locked,
    })
}

/// Writes the neighbours' corners into the open undo step; false when the
/// command refused (its message said).
pub(crate) fn write_neighbours(joining: &Joining, cx: &mut Context<'_>) -> bool {
    if joining.changes.is_empty() {
        return true;
    }
    let input = EntitiesEdit {
        operation: EditOperation::VertexAdd,
        changes: joining.changes.clone(),
        expected_revision: None,
    };
    let result = edit::execute(&mut ExecutionContext::new(cx.doc), input);
    points::written(result, cx).is_some()
}

/// Writes what `write` writes (the new area) and the neighbours' corners as
/// one undo step named `label` (the new area's own); when either is
/// refused, nothing stays. What `write` gave, or `None`.
pub(crate) fn with_joined<T>(
    cx: &mut Context<'_>,
    joining: Option<&Joining>,
    label: &str,
    write: impl FnOnce(&mut Context<'_>) -> Option<T>,
) -> Option<T> {
    let Some(joining) = joining.filter(|j| !j.changes.is_empty()) else {
        return write(cx);
    };
    let group = cx.doc.begin_group(label);
    let written = write(cx).filter(|_| write_neighbours(joining, cx));
    if written.is_some() {
        cx.doc.end_group(group);
    } else {
        cx.doc.cancel_group(group);
    }
    written
}

/// Says what joining did, once the new area is written.
pub(crate) fn say(joining: Option<&Joining>, cx: &mut Context<'_>) {
    let Some(j) = joining else {
        return;
    };
    if !j.changes.is_empty() {
        let text = format!(
            "Topolojik düzenleme: {} komşu nesneye yeni alanın {} köşesi eklendi.",
            j.changes.len(),
            j.given
        );
        cx.say(Level::Info, text);
    }
    if j.taken > 0 {
        let text = format!(
            "Topolojik düzenleme: yeni alana komşulardan {} köşe eklendi.",
            j.taken
        );
        cx.say(Level::Info, text);
    }
    if j.locked > 0 {
        let text = format!(
            "Kilitli katmandaki {} komşu nesneye köşe eklenmedi.",
            j.locked
        );
        cx.say(Level::Warn, text);
    }
}
