//! The semantic commands that make new objects (Böl, Alan oluştur, Tarama;
//! docs/adr/0254): they compute the objects and write them through
//! `cad.entities.create` ([`crate::create`]), so the layer's and the look's
//! checks, the undo step and the journal are that command's. A refusal
//! about an object's look (`objects[2].color`) is said at the look's own
//! field (`color`): the input names the look once, for every object.

use kentos_contracts::{
    CommandError, CommandResult, CreateOperation, EntitiesCreate, EntitiesCreatePlan,
    EntitiesCreated, EntityGeometry, NewLook, NewObject,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::create;

/// The objects `geometries` with `look`, on `layer_id`, as `cad.entities.create` takes them.
pub(crate) fn input(
    layer_id: &str,
    geometries: Vec<EntityGeometry>,
    look: &NewLook,
    operation: CreateOperation,
) -> EntitiesCreate {
    input_as(layer_id, geometries, look, Some(operation))
}

/// [`input`] with the step's operation, or none (the step “Ekle”).
pub(crate) fn input_as(
    layer_id: &str,
    geometries: Vec<EntityGeometry>,
    look: &NewLook,
    operation: Option<CreateOperation>,
) -> EntitiesCreate {
    EntitiesCreate {
        layer_id: layer_id.to_owned(),
        objects: geometries
            .into_iter()
            .map(|geometry| NewObject {
                line_type: None,
                geometry,
                color: look.color.clone(),
                line_weight: look.line_weight,
                attrs: look.attrs.clone(),
                label: look.label.clone(),
                symbol: look.symbol.clone(),
                label_of: None,
                label_scale: None,
            })
            .collect(),
        operation,
        expected_revision: None,
    }
}

/// An error at an object's field said at the look's (`objects[2].color` → `color`).
fn at_look(mut error: CommandError) -> CommandError {
    if let Some(path) = &error.path
        && let Some(rest) = path.strip_prefix("objects[")
        && let Some((_, field)) = rest.split_once("].")
    {
        error.path = Some(field.to_owned());
    }
    error
}

fn relay<T>(result: CommandResult<T>) -> CommandResult<T> {
    match result {
        CommandResult::Failed { error } => CommandResult::Failed {
            error: at_look(error),
        },
        CommandResult::Conflict { error } => CommandResult::Conflict {
            error: at_look(error),
        },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput {
            error: at_look(error),
        },
        other => other,
    }
}

/// What writing `objects` would write now.
pub(crate) fn plan(doc: &Document, objects: &EntitiesCreate) -> CommandResult<EntitiesCreatePlan> {
    relay(create::plan_in(doc, objects))
}

/// Writes `objects` as one undo step (inside the command's own step).
pub(crate) fn execute(
    cx: &mut ExecutionContext<'_>,
    objects: EntitiesCreate,
) -> CommandResult<EntitiesCreated> {
    relay(create::execute(cx, objects))
}

/// Checks `objects` against the document, writing nothing.
pub(crate) fn validate(cx: &ExecutionContext<'_>, objects: &EntitiesCreate) -> CommandResult<()> {
    relay(create::validate(cx, objects))
}

/// Warnings of the command's own put before the creation's.
pub(crate) fn with_warnings<T>(
    result: CommandResult<T>,
    first: Vec<kentos_contracts::CommandWarning>,
) -> CommandResult<T> {
    match result {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output,
            warnings: [first, warnings].concat(),
        },
        other => other,
    }
}
