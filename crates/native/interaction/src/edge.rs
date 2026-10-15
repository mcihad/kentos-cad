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
use crate::outlines;
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
    let kinds = cx.draft.select_kinds;
    // The selection filter's kinds, as every selecting step (docs/adr/0278).
    cx.spatial.pick_edge(p.raw, cx.pick_tolerance(), |slot| {
        doc.get(slot)
            .is_some_and(|e| crate::selectable::entity_allowed(kinds, e) && editable(e, doc))
    })
}

/// [`pick`] for a click: when nothing the tool takes is there but an object is, why it was left out is said
/// (docs/adr/0278): its layer is locked, or the selection filter holds not its kind.
pub(crate) fn pick_said(
    p: &Pointer,
    cx: &mut Context<'_>,
    editable: impl Fn(&Entity, &Document) -> bool,
) -> Option<Slot> {
    let picked = pick(p, cx, &editable);
    if picked.is_some() {
        return picked;
    }
    let under = cx.spatial.pick_edge(p.raw, cx.pick_tolerance(), |_| true)?;
    let e = cx.doc.get(under)?;
    let kind = crate::data_search::kind_name(e);
    if !crate::selectable::entity_allowed(cx.draft.select_kinds, e) {
        cx.say(
            Level::Info,
            format!("Seçim süzgeci tıklanan nesneyi ({kind}) dışarıda bıraktı."),
        );
    } else if !unlocked(e, cx.doc) && !editable(e, cx.doc) {
        let layer = &e.base().layer_id;
        let name = cx
            .doc
            .layers()
            .get(layer)
            .map_or(layer.clone(), |n| n.name.clone());
        cx.say(
            Level::Warn,
            format!(
                "“{name}” katmanı kilitli; {kind} düzenlenemez. Kilidi Katmanlar panelinden açın."
            ),
        );
    }
    None
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
        let (strokes, marks) = outlines::read(&paths, |pts, closed| {
            let mut stroke = Stroke::solid(pts, closed).width(width).tone(tone);
            stroke.dash = dash;
            stroke
        });
        Self { strokes, marks }
    }

    /// Adds another outline after this one.
    pub fn and(mut self, other: Outline) -> Self {
        self.strokes.extend(other.strokes);
        self.marks.extend(other.marks);
        self
    }
}

/// How many parts the polyline at `slot` has when it has more than one (docs/adr/0174, 0307): what Kır and Buda say
/// of a multi-part line, which stays one object.
pub fn line_parts(doc: &kentos_domain::Document, slot: kentos_domain::Slot) -> Option<usize> {
    match doc.get(slot)? {
        kentos_contracts::Entity::Polyline(p) => p
            .parts
            .as_ref()
            .filter(|parts| !parts.is_empty())
            .map(|parts| parts.len() + 1),
        _ => None,
    }
}
