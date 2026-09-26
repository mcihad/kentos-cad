//! What the tools that act on the edge under the cursor share: the web's
//! `EdgePickTool` (`apps/web/src/tools/edgeTools.ts`) without its DOM side
//! (docs/adr/0047).
//!
//! - The target is picked by its edge: the nearest object whose edge is
//!   within the pick aperture and that the tool edits (not on a locked
//!   layer, of the kinds it takes), hovered as the pointer moves.
//! - What a tool does is written through the product command
//!   `cad.entities.edit`: the objects by their persistent ids, the geometry
//!   the shared core computed, one undo step named after the tool. The
//!   command's refusal is the tool's warning.
//! - A replaced object is the first piece (its slot and persistent id); the
//!   others are new objects from it; attributes and the label survive a
//!   single piece.
//!
//! None of the geometry is computed here.

use kentos_contracts::{EditOperation, EntitiesEdit, EntitiesEdited, Entity, EntityEdit};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::store::tools::outline_paths;
use kentos_native_application::geometry::{edit_geometry, shape};
use kentos_native_application::{ExecutionContext, edit};

use crate::Vec2;
use crate::log::Level;
use crate::points;
use crate::tool::{Context, Pointer, Stroke, Tone};

/// The object under the pointer and where the pointer is on it (the web's `hover`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Hover {
    pub slot: Slot,
    /// The pointer's world point, before snapping (`p.raw`).
    pub at: Vec2,
}

/// Not on a locked layer, by itself or a group above it (the edge tools' `editable`).
pub(crate) fn unlocked(e: &Entity, doc: &Document) -> bool {
    !doc.layers().is_locked(&e.base().layer_id)
}

/// The object whose edge is under the pointer among those `editable` takes (the web's `pickEdge`).
pub(crate) fn pick(
    p: &Pointer,
    cx: &Context<'_>,
    editable: impl Fn(&Entity, &Document) -> bool,
) -> Option<Slot> {
    let doc = &*cx.doc;
    cx.spatial.pick_edge(p.raw, cx.pick_tolerance(), |slot| {
        doc.get(slot).is_some_and(|e| editable(e, doc))
    })
}

/// The pointer moved with no object picked yet: the edge under it is
/// hovered, highlighted as the select tool highlights (the web's `pointerMove`).
pub(crate) fn hover(
    p: &Pointer,
    cx: &mut Context<'_>,
    editable: impl Fn(&Entity, &Document) -> bool,
) -> Option<Hover> {
    let hit = pick(p, cx, editable);
    cx.selection.set_hover(hit);
    hit.map(|slot| Hover { slot, at: p.raw })
}

/// An object's persistent id as the command names it (every object has one; ADR 0014).
pub(crate) fn uid(doc: &Document, slot: Slot) -> String {
    doc.uid(slot).map(|u| u.to_string()).unwrap_or_default()
}

/// An object as the shared core's operations take it.
pub(crate) fn core(e: &Entity) -> CoreEntity {
    CoreEntity::new(shape(e))
}

/// Writes `changes` as one edit through `cad.entities.edit`; the command's
/// answer, or `None` when nothing was written (its refusal said).
pub(crate) fn write(
    operation: EditOperation,
    changes: Vec<EntityEdit>,
    cx: &mut Context<'_>,
) -> Option<EntitiesEdited> {
    let input = EntitiesEdit {
        operation,
        changes,
        expected_revision: None,
    };
    let result = edit::execute(&mut ExecutionContext::new(cx.doc), input);
    points::written(result, cx)
}

/// A shape the core computed as the command takes it. `None` only for a
/// dimension style or a hatch pattern the contract does not know.
pub(crate) fn geometry(shape: &Shape) -> Option<kentos_contracts::EntityGeometry> {
    edit_geometry(shape.clone())
}

/// Replaces the object at `slot` by `pieces` in one undo step (the web's
/// `EdgePickTool.replace`): the first piece is the object itself, the others
/// new objects from it; attributes and the label survive a single piece of
/// anything but a closed area. No piece deletes it. Whether it was written.
pub(crate) fn replace(
    operation: EditOperation,
    slot: Slot,
    pieces: &[Shape],
    cx: &mut Context<'_>,
) -> bool {
    let polygon = matches!(cx.doc.get(slot), Some(Entity::Polygon(_)));
    let keep = (pieces.len() == 1 && !polygon).then_some(true);
    let id = uid(cx.doc, slot);
    let mut changes = Vec::with_capacity(pieces.len().max(1));
    let mut geometries = pieces.iter().filter_map(geometry);
    match geometries.next() {
        Some(first) => changes.push(EntityEdit::Replace {
            uid: id.clone(),
            geometry: first,
            keep_data: keep,
        }),
        None => changes.push(EntityEdit::Remove { uid: id.clone() }),
    }
    for piece in geometries {
        changes.push(EntityEdit::Add {
            from: id.clone(),
            geometry: piece,
            keep_data: keep,
        });
    }
    let written = write(operation, changes, cx).is_some();
    let doc = &*cx.doc;
    cx.selection.retain(|s| doc.get(s).is_some());
    cx.selection.set_hover(None);
    written
}

/// Cutting open a closed area with holes would lose them: such a tool says
/// so instead (the web's `refuseHoled`). True when `e` was refused.
pub(crate) fn refuse_holed(e: &Entity, action: &str, cx: &mut Context<'_>) -> bool {
    let Entity::Polygon(p) = e else {
        return false;
    };
    if p.holes.as_ref().is_none_or(|h| h.is_empty()) {
        return false;
    }
    cx.say(
        Level::Warn,
        format!(
            "Adalı alanda {action} yapılamaz; iç halkalar kaybolurdu. Önce Patlat ile halkalarına ayırın ya da Alan böl kullanın."
        ),
    );
    true
}

/// What the web's `strokeGeometry` draws of a shape: its outline and its
/// holes as strokes, a point or a text as a 7 px mark (`outline_paths`, the
/// core's tessellation).
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Outline {
    pub strokes: Vec<Stroke>,
    pub marks: Vec<Vec2>,
}

impl Outline {
    pub fn of(shape: &Shape, dash: Option<[f32; 2]>, width: f32, tone: Tone) -> Self {
        let mut paths = Vec::new();
        outline_paths(shape, &mut paths);
        let mut out = Self::default();
        let mut at = 0;
        // `flags, n, x0, y0, …` per path: 0 open, 1 closed, 2 a marker.
        while at + 1 < paths.len() {
            let flags = paths[at];
            let n = paths[at + 1] as usize;
            let pts: Vec<Vec2> = (0..n)
                .filter_map(|k| {
                    let (x, y) = (*paths.get(at + 2 + 2 * k)?, *paths.get(at + 3 + 2 * k)?);
                    Some(Vec2::new(x, y))
                })
                .collect();
            at += 2 + 2 * n;
            if flags == 2.0 {
                out.marks.extend(pts.first());
            } else if pts.len() >= 2 {
                let mut stroke = Stroke::solid(pts, flags == 1.0).width(width).tone(tone);
                stroke.dash = dash;
                out.strokes.push(stroke);
            }
        }
        out
    }

    /// Adds another outline after this one.
    pub fn and(mut self, other: Outline) -> Self {
        self.strokes.extend(other.strokes);
        self.marks.extend(other.marks);
        self
    }
}
