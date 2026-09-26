//! `cad.entities.create` v1 (docs/adr/0057): new objects of any kind on a
//! named layer, as one undo step named “Ekle” or after the drawing tool.
//! The desktop's handler over the native document; the web's is
//! `apps/web/src/product/entitiesCreate.ts`. Both pass the shared cases in
//! `fixtures/commands/v1/cad.entities.create.json`.
//!
//! Elips, Eğri, Yardımcı çizgi, Işın, Halka, Paralel çizgi, Dik in, Dik çık
//! and Böl compute the geometry with the shared core and write it here
//! (TODOS.md CMD-07); nothing is computed in this module.
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one object;
//! 2. every geometry, in order, by `cad.entities.edit`'s rules: enough
//!    points for its kind, every number finite, a circle's or an arc's
//!    radius above zero;
//! 3. the expected revision, then the layer (every create command's, `checks.rs`).

use kentos_contracts::{
    CommandResult, CommandWarning, CreateOperation, EntitiesCreate, EntitiesCreatePlan,
    EntitiesCreated, Entity, EntityBase,
};
use kentos_domain::{Document, labels};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;
use crate::edit::check_geometry;
use crate::geometry::entity_of;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesCreate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now: the objects as they would be stored, and
/// the revision to expect for exactly that; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &EntitiesCreate,
) -> CommandResult<EntitiesCreatePlan> {
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: EntitiesCreatePlan {
                entities: entities(input),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the objects in their order as one undo
/// step (the document's own `add_many`), into the open transaction or group
/// if one is. Nothing is written when the document has no slot left for them.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesCreate,
) -> CommandResult<EntitiesCreated> {
    let warnings = match check(cx.doc, &input) {
        Ok(warnings) => warnings,
        Err(stop) => return stop.into(),
    };
    let slots = match cx.doc.add_many(entities(&input), label(input.operation)) {
        Ok(slots) => slots,
        Err(full) => {
            return CommandResult::Failed {
                error: error(codes::SLOTS_EXHAUSTED, full.to_string(), None),
            };
        }
    };
    let doc = &*cx.doc;
    CommandResult::Completed {
        output: EntitiesCreated {
            created: slots
                .iter()
                .filter_map(|slot| doc.uid(*slot))
                .map(|uid| uid.to_string())
                .collect(),
            ids: slots.iter().map(|slot| slot.0).collect(),
            revision: doc.revision().to_string(),
        },
        warnings,
    }
}

/// The undo step's name: the drawing tool's when it has its own, else the
/// document's “Ekle” (docs/adr/0057).
pub fn label(operation: Option<CreateOperation>) -> &'static str {
    match operation {
        None => labels::ADD,
        Some(CreateOperation::Parallel) => "Paralel çizgi",
        Some(CreateOperation::PerpendicularIn) => "Dik in",
        Some(CreateOperation::PerpendicularOut) => "Dik çık",
        Some(CreateOperation::Divide) => "Böl",
    }
}

/// The checks in the contract's order: why nothing may be written, or the
/// warnings when it may.
fn check(doc: &Document, input: &EntitiesCreate) -> Result<Vec<CommandWarning>, Stop> {
    if input.objects.is_empty() {
        return Err(Stop::Failed(error(
            codes::NO_OBJECTS,
            "Eklenecek nesne verilmedi. En az bir nesne verin.".into(),
            Some("objects".into()),
        )));
    }
    for (i, object) in input.objects.iter().enumerate() {
        check_geometry(&object.geometry, "objects", i, "nesnenin")?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    checks::layer(doc, &input.layer_id)
}

/// The objects `input` describes, as the document stores them: slot 0 (given
/// when written), its own copies of every field.
fn entities(input: &EntitiesCreate) -> Vec<Entity> {
    input
        .objects
        .iter()
        .map(|object| {
            entity_of(
                &object.geometry,
                EntityBase {
                    id: 0,
                    layer_id: input.layer_id.clone(),
                    color: object.color.clone(),
                    attrs: object.attrs.clone().unwrap_or_default(),
                    label: object.label.clone(),
                    symbol: None,
                },
            )
        })
        .collect()
}
