//! `cad.circle.create` v1 (docs/adr/0032): one circle by its centre and
//! radius on a named layer, as one undo step. The desktop's handler over the
//! native document; the web's is `apps/web/src/product/circleCreate.ts`.
//! Both pass the shared cases in `fixtures/commands/v1/cad.circle.create.json`.
//!
//! The circle tool computes the circle of each of its methods through the
//! shared core (`kentos-geometry-core`) and gives the result here.
//!
//! The checks, in order (the first that fails answers):
//! 1. the centre finite (x before y), then the radius finite, then above zero;
//! 2. the expected revision, 3. the layer: every create command's
//!    (`checks.rs`), for the reasons `polygon.rs` gives.

use kentos_contracts::{
    CircleCreate, CircleCreated, CircleEntity, CirclePlan, CommandResult, CommandWarning, Entity,
    EntityBase,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &CircleCreate) -> CommandResult<()> {
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
pub fn plan(cx: &ExecutionContext<'_>, input: &CircleCreate) -> CommandResult<CirclePlan> {
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: CirclePlan {
                entity: circle(input.clone(), 0),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the circle as one undo step (“Ekle”, the
/// document's own `add`): into the open transaction or group, if one is.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CircleCreate) -> CommandResult<CircleCreated> {
    let warnings = match check(cx.doc, &input) {
        Ok(warnings) => warnings,
        Err(stop) => return stop.into(),
    };
    match checks::add(cx.doc, circle(input, 0)) {
        Ok((id, uid, revision)) => CommandResult::Completed {
            output: CircleCreated { uid, id, revision },
            warnings,
        },
        Err(error) => CommandResult::Failed { error },
    }
}

/// The checks in the contract's order; the warnings when the circle may be written.
fn check(doc: &Document, input: &CircleCreate) -> Result<Vec<CommandWarning>, Stop> {
    checks::point(input.c, "Merkezin", "c")?;
    checks::finite(input.r, "Yarıçap", "Yarıçapı sonlu bir sayıyla verin.", "r")?;
    checks::radius(input.r)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    checks::layer(doc, &input.layer_id)
}

/// The circle `input` describes, as the document stores it; `id` is the
/// document's to give (0 in a plan).
fn circle(input: CircleCreate, id: u32) -> Entity {
    Entity::Circle(CircleEntity {
        base: EntityBase {
            id,
            layer_id: input.layer_id,
            color: input.color,
            attrs: input.attrs.unwrap_or_default(),
            label: None,
            symbol: None,
        },
        c: input.c,
        r: input.r,
    })
}
