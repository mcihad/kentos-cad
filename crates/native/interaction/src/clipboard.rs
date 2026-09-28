//! The clipboard (docs/adr/0056): what Kes and Panoya kopyala put aside for
//! Yapıştır, as the web's `Clipboard` (`apps/web/src/app/clipboard.ts`) and
//! its commands (`app/commands.ts`: `edit.cut`, `edit.copy`,
//! `edit.pasteOriginal`; `tools/editTools.ts`: `pasteEntities`) do it.
//!
//! - It is the session's (CLAUDE.md §4.4), not the drawing's: it outlives
//!   the drawing on screen, so objects go from one drawing to another, and it
//!   stays in the app. The system clipboard is not used, as on the web.
//! - It holds copies of the objects without their ids, in the order they
//!   were selected, and a base point: the lower left corner of their box.
//! - Kes deletes through the product command `cad.entities.delete`
//!   (docs/adr/0029) inside one undo step “Kes”, as the web's command does
//!   since f860b5f. Yapıştır writes through `cad.entities.create` and
//!   `cad.entities.set` (for the symbols) in one undo step “Yapıştır”, as
//!   the web's since 7a5ca22; what either refuses takes all of it back and
//!   is said. Objects on locked layers are not cut. Pasted objects are new
//!   ones with new persistent ids (docs/adr/0014), however often they are
//!   pasted; each keeps its layer when the drawing has it (a layer, not a
//!   group) unlocked, else goes to the active layer; a hidden layer that
//!   takes some is said once.

use std::convert::Infallible;

use kentos_contracts::{
    CommandResult, EntitiesCreate, EntitiesDelete, EntitiesSetProperties, Entity, EntityGeometry,
    LayerNodeType, NewObject, PropertiesOperation,
};
use kentos_domain::Slot;
use kentos_geometry_core::geom::affine::translation;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::transform::transform_shape;
use kentos_native_application::geometry::{edit_geometry, shape, with_shape};
use kentos_native_application::{ExecutionContext, codes, create, delete, set};

use crate::Vec2;
use crate::log::Level;
use crate::tool::Context;

/// The undo steps' names (the web's).
pub const CUT_LABEL: &str = "Kes";
pub const PASTE_LABEL: &str = "Yapıştır";

/// Copies of objects and their base point.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clipboard {
    items: Vec<Entity>,
    base: Vec2,
}

impl Clipboard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Puts copies of `entities` aside (what was there goes), with the lower
    /// left corner of their box `extent` as the base point; the origin when
    /// there is no box.
    pub fn set(&mut self, entities: Vec<Entity>, extent: Option<Bounds>) {
        self.base = match extent {
            Some(b) if !entities.is_empty() => Vec2::new(b.min_x, b.min_y),
            _ => Vec2::new(0.0, 0.0),
        };
        self.items = entities
            .into_iter()
            .map(|mut e| {
                // A copy is nobody's: the document gives it a slot when it is pasted.
                e.base_mut().id = 0;
                e
            })
            .collect();
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// The objects, in the order they were selected.
    pub fn items(&self) -> &[Entity] {
        &self.items
    }

    /// Where the objects are placed from: the lower left corner of their box.
    pub fn base(&self) -> Vec2 {
        self.base
    }
}

/// The selected objects that are in the drawing, in the order they were selected.
fn selected(cx: &Context<'_>) -> Vec<(Slot, Entity)> {
    cx.selection
        .ids()
        .iter()
        .filter_map(|slot| cx.doc.get(*slot).map(|e| (*slot, e.clone())))
        .collect()
}

/// The box around these objects, as the geometry store gives it (the web's `view.extent`).
fn extent(cx: &Context<'_>, slots: &[Slot]) -> Option<Bounds> {
    let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
    cx.spatial.store().extent(Some(&ids))
}

/// Panoya kopyala (`edit.copy`): the selected objects go to the clipboard;
/// the drawing does not change. How many; none, and nothing said, with no
/// selection (the web's command is off then).
pub fn copy(clipboard: &mut Clipboard, cx: &mut Context<'_>) -> usize {
    let chosen = selected(cx);
    if chosen.is_empty() {
        return 0;
    }
    let slots: Vec<Slot> = chosen.iter().map(|(s, _)| *s).collect();
    let n = chosen.len();
    clipboard.set(
        chosen.into_iter().map(|(_, e)| e).collect(),
        extent(cx, &slots),
    );
    cx.say(Level::Success, format!("{n} nesne panoya kopyalandı."));
    n
}

/// Kes (`edit.cut`): the selected objects on unlocked layers go to the
/// clipboard and leave the drawing in one undo step “Kes”. Those on locked
/// layers stay, selected, and a warning counts them; when all are locked the
/// clipboard keeps what it had. How many were cut.
pub fn cut(clipboard: &mut Clipboard, cx: &mut Context<'_>) -> usize {
    let chosen = selected(cx);
    let all = chosen.len();
    let layers = cx.doc.layers();
    let editable: Vec<(Slot, Entity)> = chosen
        .into_iter()
        .filter(|(_, e)| !layers.is_locked(&e.base().layer_id))
        .collect();
    if editable.len() < all {
        cx.say(
            Level::Warn,
            format!(
                "{} nesne kilitli katmanda olduğu için kesilmedi.",
                all - editable.len()
            ),
        );
    }
    if editable.is_empty() {
        return 0;
    }
    let slots: Vec<Slot> = editable.iter().map(|(s, _)| *s).collect();
    let n = editable.len();
    let base = extent(cx, &slots);
    // Deleted through cad.entities.delete (docs/adr/0029), as Sil deletes; the step keeps the name “Kes”.
    let input = EntitiesDelete {
        uids: slots
            .iter()
            .filter_map(|slot| cx.doc.uid(*slot))
            .map(|uid| uid.to_string())
            .collect(),
        expected_revision: None,
    };
    let Ok(result) = cx.doc.transact(CUT_LABEL, |doc| {
        Ok::<_, Infallible>(delete::execute(&mut ExecutionContext::new(doc), input))
    });
    if crate::points::written(result, cx).is_none() {
        return 0;
    }
    clipboard.set(editable.into_iter().map(|(_, e)| e).collect(), base);
    let doc = &*cx.doc;
    cx.selection.retain(|slot| doc.get(slot).is_some());
    cx.say(Level::Success, format!("{n} nesne panoya kesildi."));
    n
}

/// Özgün koordinatlara yapıştır (`edit.pasteOriginal`): the clipboard's
/// objects where they were copied, selected. The new slots.
pub fn paste_in_place(clipboard: &Clipboard, cx: &mut Context<'_>) -> Vec<Slot> {
    let slots = paste(clipboard.items(), 0.0, 0.0, cx);
    if !slots.is_empty() {
        cx.selection.set(slots.iter().copied());
    }
    slots
}

/// Adds copies of `items` moved by (`dx`, `dy`) in one undo step
/// “Yapıştır” (the web's `pasteEntities`); returns their slots. Each keeps
/// its layer when the drawing has it unlocked, else goes to the active
/// layer; when that is locked and some object would go there, nothing is
/// pasted and a warning says why. The geometry is the shared core's
/// translation of each object (`transform_shape`), as the web's store moves
/// its copies.
pub fn paste(items: &[Entity], dx: f64, dy: f64, cx: &mut Context<'_>) -> Vec<Slot> {
    let layers = cx.doc.layers();
    let active = layers.active().to_owned();
    // A layer (not a group) that is not locked (the web's since 7a5ca22).
    let usable = |id: &str| {
        layers
            .get(id)
            .is_some_and(|node| node.kind == LayerNodeType::Layer)
            && !layers.is_locked(id)
    };
    if layers.is_locked(&active) && items.iter().any(|e| !usable(&e.base().layer_id)) {
        let name = layers.get(&active).map_or("", |node| node.name.as_str());
        let line = format!("“{name}” katmanı kilitli; yapıştırılamadı.");
        cx.say(Level::Warn, line);
        return Vec::new();
    }
    let m = translation(dx, dy);
    // Each copy with its geometry as the commands take it.
    let moved: Vec<(Entity, EntityGeometry)> = items
        .iter()
        .filter_map(|e| {
            let mut copy = with_shape(e, transform_shape(&shape(e), &m))?;
            if !usable(&e.base().layer_id) {
                copy.base_mut().layer_id = active.clone();
            }
            let geometry = edit_geometry(shape(&copy))?;
            Some((copy, geometry))
        })
        .collect();
    let mut hidden: Vec<String> = Vec::new();
    let written = cx.doc.transact(PASTE_LABEL, |doc| {
        let mut out: Vec<Slot> = Vec::with_capacity(moved.len());
        // One create for each run of objects going to the same layer, in
        // their order, so their slots are in it too.
        for run in moved.chunk_by(|(a, _), (b, _)| a.base().layer_id == b.base().layer_id) {
            let layer = run[0].0.base().layer_id.clone();
            let objects = run
                .iter()
                .map(|(e, geometry)| NewObject {
                    geometry: geometry.clone(),
                    color: e.base().color.clone(),
                    attrs: Some(e.base().attrs.clone()),
                    label: e.base().label.clone(),
                })
                .collect();
            let input = EntitiesCreate {
                layer_id: layer.clone(),
                objects,
                operation: None,
                expected_revision: None,
            };
            match create::execute(&mut ExecutionContext::new(doc), input) {
                CommandResult::Completed { output, warnings } => {
                    if warnings.iter().any(|w| w.code == codes::LAYER_HIDDEN)
                        && !hidden.contains(&layer)
                    {
                        hidden.push(layer);
                    }
                    out.extend(output.ids.into_iter().map(Slot));
                }
                other => return Err(refusal(other)),
            }
        }
        // A new object carries no symbol: one set for each symbol, in the order they come.
        let mut symbols: Vec<(String, Vec<String>)> = Vec::new();
        for ((e, _), &slot) in moved.iter().zip(&out) {
            let (Some(symbol), Some(uid)) = (e.base().symbol.as_ref(), doc.uid(slot)) else {
                continue;
            };
            match symbols.iter_mut().find(|(s, _)| s == symbol) {
                Some((_, uids)) => uids.push(uid.to_string()),
                None => symbols.push((symbol.clone(), vec![uid.to_string()])),
            }
        }
        for (symbol, uids) in symbols {
            let input = EntitiesSetProperties {
                uids,
                layer_id: None,
                color: None,
                line_weight: None,
                symbol: Some(Some(symbol)),
                attrs: None,
                label: None,
                operation: PropertiesOperation::Symbol,
                expected_revision: None,
            };
            if let refused @ (CommandResult::Failed { .. }
            | CommandResult::Conflict { .. }
            | CommandResult::NeedsInput { .. }) =
                set::execute(&mut ExecutionContext::new(doc), input)
            {
                return Err(refusal(refused));
            }
        }
        Ok(out)
    });
    match written {
        Ok(slots) => {
            for id in &hidden {
                let name = cx
                    .doc
                    .layers()
                    .get(id)
                    .map_or(id.as_str(), |n| n.name.as_str());
                let line = format!("“{name}” katmanı gizli; yapıştırılan nesneler görünmeyecek.");
                cx.say(Level::Warn, line);
            }
            cx.say(
                Level::Success,
                format!("{} nesne yapıştırıldı.", slots.len()),
            );
            slots
        }
        Err(message) => {
            cx.say(Level::Warn, message);
            Vec::new()
        }
    }
}

/// A command's refusal as the paste says it.
fn refusal<T>(result: CommandResult<T>) -> String {
    match result {
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => error.message,
        _ => "Yapıştırılamadı.".to_owned(),
    }
}
