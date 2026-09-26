//! `cad.point.create` v1 (docs/adr/0032): one point object on a named layer,
//! as one undo step. The desktop's handler over the native document; the
//! web's is `apps/web/src/product/pointCreate.ts`. Both pass the shared cases
//! in `fixtures/commands/v1/cad.point.create.json`.
//!
//! The point tool calls it once per point it places. Its elevation and the
//! text beside it are optional (the spot elevation tool writes both).
//!
//! The checks, in order (the first that fails answers):
//! 1. the point finite, x before y, then the elevation when given;
//! 2. the expected revision, 3. the layer: every create command's
//!    (`checks.rs`), for the reasons `polygon.rs` gives.

use kentos_contracts::{
    CommandResult, CommandWarning, Entity, EntityBase, PointCreate, PointCreated, PointEntity,
    PointPlan,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &PointCreate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now, and the revision to expect for exactly
/// that; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &PointCreate) -> CommandResult<PointPlan> {
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: PointPlan {
                entity: point(input.clone(), 0),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the point as one undo step (“Ekle”, the
/// document's own `add`): into the open transaction or group, if one is.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PointCreate) -> CommandResult<PointCreated> {
    let warnings = match check(cx.doc, &input) {
        Ok(warnings) => warnings,
        Err(stop) => return stop.into(),
    };
    match checks::add(cx.doc, point(input, 0)) {
        Ok((id, uid, revision)) => CommandResult::Completed {
            output: PointCreated { uid, id, revision },
            warnings,
        },
        Err(error) => CommandResult::Failed { error },
    }
}

/// The checks in the contract's order; the warnings when the point may be written.
fn check(doc: &Document, input: &PointCreate) -> Result<Vec<CommandWarning>, Stop> {
    checks::point(input.p, "Noktanın", "p")?;
    if let Some(z) = input.z {
        checks::finite(z, "Kot", "Kotu sonlu bir sayıyla verin.", "z")?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    checks::layer(doc, &input.layer_id)
}

/// The point `input` describes, as the document stores it; `id` is the
/// document's to give (0 in a plan).
fn point(input: PointCreate, id: u32) -> Entity {
    Entity::Point(PointEntity {
        base: EntityBase {
            id,
            layer_id: input.layer_id,
            color: input.color,
            attrs: input.attrs.unwrap_or_default(),
            label: input.label,
            symbol: None,
        },
        p: input.p,
        z: input.z,
    })
}
