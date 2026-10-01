//! How Öznitelikler and the text field over the drawing write (the web's
//! `ui/properties/write.ts`, docs/adr/0066): through product commands, with
//! the objects by persistent id and the value explicit in the input
//! (TODOS.md CMD-07). A layer, colour, symbol, attribute or label goes
//! through `cad.entities.set`; a value of the geometry (a point's Y, a text,
//! a hatch's spacing) through `cad.entities.edit`, operation `properties`,
//! the step “Değiştir”. The command's refusal or warnings are what the host
//! says, as warnings; the undo steps are named as before. The Kot rows
//! (docs/adr/0142) write the vertices' elevations through the same command's
//! operation `elevation`, the step “Kot ver”.

use kentos_contracts::{
    CommandResult, EditOperation, EntitiesEdit, EntitiesSetProperties, Entity, EntityEdit,
    LeaderEntity, TextAlign, TextEntity,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::TextPlace;
use kentos_geometry_core::text::Font;
use kentos_geometry_core::vec2::Vec2;
use kentos_native_application::geometry::{drawing_font, edit_geometry, shape};
use kentos_native_application::{ExecutionContext, edit, set};

use crate::elevation::{self, Change};

/// The persistent ids of the objects in these slots, as the commands name them.
pub fn uids_of(doc: &Document, slots: &[Slot]) -> Vec<String> {
    slots
        .iter()
        .filter_map(|&slot| doc.uid(slot))
        .map(|uid| uid.to_string())
        .collect()
}

/// Sets objects' layer, colour, symbol, attributes or label; what to say:
/// the command's refusal, or its warnings (a hidden layer).
pub fn set_properties(doc: &mut Document, input: EntitiesSetProperties) -> Vec<String> {
    said(set::execute(&mut ExecutionContext::new(doc), input))
}

/// The object at `slot` given `entity`'s geometry in its place, as the step
/// “Değiştir” (its layer, colour, attributes, label and symbol stay); what
/// to say: the command's refusal (a locked layer, an empty text).
pub fn set_geometry(doc: &mut Document, slot: Slot, entity: &Entity) -> Vec<String> {
    let (Some(uid), Some(geometry)) = (doc.uid(slot), edit_geometry(shape(entity))) else {
        return Vec::new();
    };
    let input = EntitiesEdit {
        operation: EditOperation::Properties,
        changes: vec![EntityEdit::Update {
            uid: uid.to_string(),
            geometry,
        }],
        expected_revision: None,
    };
    said(edit::execute(&mut ExecutionContext::new(doc), input))
}

/// The vertices' elevations of the lines, polylines, areas and points in
/// `slots` as `change` says (docs/adr/0142), through `cad.entities.edit`'s operation
/// `elevation`: each object's own geometry with its elevations explicit, one
/// undo step, “Kot ver”. What the change would leave as it is is not written
/// (no step for it), and nothing is written when nothing changes; what to say:
/// the command's refusal (a locked layer), or its warnings.
pub fn set_elevations(doc: &mut Document, slots: &[Slot], change: Change) -> Vec<String> {
    let changes: Vec<EntityEdit> = slots
        .iter()
        .filter_map(|&slot| {
            let e = doc.get(slot).filter(|e| elevation::takes(e))?;
            if elevation::leaves(e, change) {
                return None;
            }
            Some(EntityEdit::Update {
                uid: doc.uid(slot)?.to_string(),
                geometry: elevation::geometry_with(e, change)?,
            })
        })
        .collect();
    if changes.is_empty() {
        return Vec::new();
    }
    let input = EntitiesEdit {
        operation: EditOperation::Elevation,
        changes,
        expected_revision: None,
    };
    said(edit::execute(&mut ExecutionContext::new(doc), input))
}

/// Several objects, each given its entity's geometry in its place, in one
/// step “Değiştir” (Öznitelikler's rows over a selection, docs/adr/0145 §6);
/// nothing when none is given. What to say: the command's refusal.
pub fn set_geometries(doc: &mut Document, changes: &[(Slot, Entity)]) -> Vec<String> {
    let changes: Vec<EntityEdit> = changes
        .iter()
        .filter_map(|(slot, entity)| {
            Some(EntityEdit::Update {
                uid: doc.uid(*slot)?.to_string(),
                geometry: edit_geometry(shape(entity))?,
            })
        })
        .collect();
    if changes.is_empty() {
        return Vec::new();
    }
    let input = EntitiesEdit {
        operation: EditOperation::Properties,
        changes,
        expected_revision: None,
    };
    said(edit::execute(&mut ExecutionContext::new(doc), input))
}

/// Texts given new words in one step “Bul ve değiştir” (`cad.entities.edit`'s
/// `replaceText`, docs/adr/0145 §6), each keeping the rest of its geometry;
/// what to say: the command's refusal.
pub fn replace_texts(doc: &mut Document, words: &[(Slot, String)]) -> Vec<String> {
    let changes: Vec<EntityEdit> = words
        .iter()
        .filter_map(|(slot, text)| {
            let Some(Entity::Text(t)) = doc.get(*slot) else {
                return None;
            };
            let renamed = Entity::Text(TextEntity {
                text: text.clone(),
                ..t.clone()
            });
            Some(EntityEdit::Update {
                uid: doc.uid(*slot)?.to_string(),
                geometry: edit_geometry(shape(&renamed))?,
            })
        })
        .collect();
    if changes.is_empty() {
        return Vec::new();
    }
    let input = EntitiesEdit {
        operation: EditOperation::ReplaceText,
        changes,
        expected_revision: None,
    };
    said(edit::execute(&mut ExecutionContext::new(doc), input))
}

/// The texts in `slots` changed by `change`, those it changes, in one step
/// “Değiştir” (Öznitelikler's Hiza, Genişlik çarpanı and Zemin, docs/adr/0145
/// §6); what to say: the command's refusal.
fn change_texts(
    doc: &mut Document,
    slots: &[Slot],
    change: impl Fn(&TextEntity, Font) -> Option<TextEntity>,
) -> Vec<String> {
    let font = drawing_font(doc.settings().drawing_font);
    let changes: Vec<(Slot, Entity)> = slots
        .iter()
        .filter_map(|&slot| match doc.get(slot) {
            Some(Entity::Text(t)) => Some((slot, Entity::Text(change(t, font)?))),
            _ => None,
        })
        .collect();
    set_geometries(doc, &changes)
}

/// The texts in `slots` given the alignment `to` (none: the left of the
/// baseline), each where it is: its point moves to that alignment's point of
/// its box (the core's `TextPlace::realigned`). The web's `textRows` Hiza.
pub fn realign_texts(doc: &mut Document, slots: &[Slot], to: Option<TextAlign>) -> Vec<String> {
    change_texts(doc, slots, |t, font| {
        (t.align != to).then(|| {
            let place = TextPlace {
                p: Vec2::new(t.p.x, t.p.y),
                text: &t.text,
                height: t.height,
                rotation: t.rotation,
                align: t.align.and_then(core_align),
                width_factor: t.width_factor,
            };
            let p = place.realigned(to.and_then(core_align), font);
            TextEntity {
                p: kentos_contracts::Vec2 { x: p.x, y: p.y },
                align: to,
                ..t.clone()
            }
        })
    })
}

/// The texts in `slots` with the width factor `factor` about their points
/// (1: none); the command refuses one out of its range and says why.
pub fn set_text_width(doc: &mut Document, slots: &[Slot], factor: f64) -> Vec<String> {
    change_texts(doc, slots, |t, _| {
        (t.width_factor.unwrap_or(1.0) != factor).then(|| TextEntity {
            width_factor: Some(factor),
            ..t.clone()
        })
    })
}

/// The texts in `slots` with their mask on or off.
pub fn set_text_mask(doc: &mut Document, slots: &[Slot], mask: bool) -> Vec<String> {
    change_texts(doc, slots, |t, _| {
        (t.mask != mask).then(|| TextEntity { mask, ..t.clone() })
    })
}

/// The leaders in `slots`, each changed by `change` (none: it has the value
/// already and is left out), in one step “Değiştir” (Öznitelikler's Kılavuz
/// rows, docs/adr/0146 §7; the web's `leaderRows`); what to say: the
/// command's refusal.
pub fn change_leaders(
    doc: &mut Document,
    slots: &[Slot],
    change: impl Fn(&LeaderEntity) -> Option<LeaderEntity>,
) -> Vec<String> {
    let changes: Vec<(Slot, Entity)> = slots
        .iter()
        .filter_map(|&slot| match doc.get(slot) {
            Some(Entity::Leader(l)) => Some((slot, Entity::Leader(change(l)?))),
            _ => None,
        })
        .collect();
    set_geometries(doc, &changes)
}

/// The contract's alignment as the geometry core names it (the same names).
fn core_align(a: TextAlign) -> Option<kentos_geometry_core::text::TextAlign> {
    kentos_geometry_core::text::TextAlign::from_name(a.name())
}

/// A command's answer as the host says it: nothing but its warnings when it
/// completed, else why not.
fn said<T>(result: CommandResult<T>) -> Vec<String> {
    match result {
        CommandResult::Completed { warnings, .. } => {
            warnings.into_iter().map(|w| w.message).collect()
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => vec![error.message],
        CommandResult::Queued { .. } | CommandResult::Cancelled => Vec::new(),
    }
}
