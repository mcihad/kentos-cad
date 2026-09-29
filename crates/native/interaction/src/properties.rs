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
};
use kentos_domain::{Document, Slot};
use kentos_native_application::geometry::{edit_geometry, shape};
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
