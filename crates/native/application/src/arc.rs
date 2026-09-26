//! `cad.arc.create` v1 (docs/adr/0032): one circular arc on a named layer,
//! as the document stores it (centre, radius, counter-clockwise from `a0` to
//! `a1`), as one undo step. The desktop's handler over the native document;
//! the web's is `apps/web/src/product/arcCreate.ts`. Both pass the shared
//! cases in `fixtures/commands/v1/cad.arc.create.json`.
//!
//! The arc tool computes the arc of each of its methods through the shared
//! core (`kentos-geometry-core`) and gives the result here.
//!
//! The checks, in order (the first that fails answers):
//! 1. the centre finite (x before y), the radius, `a0` and `a1` finite, the
//!    radius above zero;
//! 2. the expected revision, 3. the layer: every create command's
//!    (`checks.rs`), for the reasons `polygon.rs` gives.
//!
//! The angles are stored as given: equal angles are a full turn, as the
//! document reads them, and no angle is refused for its size.

use kentos_contracts::{
    ArcCreate, ArcCreated, ArcEntity, ArcPlan, CommandResult, CommandWarning, Entity, EntityBase,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &ArcCreate) -> CommandResult<()> {
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
pub fn plan(cx: &ExecutionContext<'_>, input: &ArcCreate) -> CommandResult<ArcPlan> {
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: ArcPlan {
                entity: arc(input.clone(), 0),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the arc as one undo step (“Ekle”, the
/// document's own `add`): into the open transaction or group, if one is.
pub fn execute(cx: &mut ExecutionContext<'_>, input: ArcCreate) -> CommandResult<ArcCreated> {
    let warnings = match check(cx.doc, &input) {
        Ok(warnings) => warnings,
        Err(stop) => return stop.into(),
    };
    match checks::add(cx.doc, arc(input, 0)) {
        Ok((id, uid, revision)) => CommandResult::Completed {
            output: ArcCreated { uid, id, revision },
            warnings,
        },
        Err(error) => CommandResult::Failed { error },
    }
}

/// The checks in the contract's order; the warnings when the arc may be written.
fn check(doc: &Document, input: &ArcCreate) -> Result<Vec<CommandWarning>, Stop> {
    checks::point(input.c, "Merkezin", "c")?;
    checks::finite(input.r, "Yarıçap", "Yarıçapı sonlu bir sayıyla verin.", "r")?;
    checks::finite(
        input.a0,
        "Başlangıç açısı",
        "Açıyı sonlu bir sayıyla verin.",
        "a0",
    )?;
    checks::finite(
        input.a1,
        "Bitiş açısı",
        "Açıyı sonlu bir sayıyla verin.",
        "a1",
    )?;
    checks::radius(input.r)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    checks::layer(doc, &input.layer_id)
}

/// The arc `input` describes, as the document stores it; `id` is the
/// document's to give (0 in a plan).
fn arc(input: ArcCreate, id: u32) -> Entity {
    Entity::Arc(ArcEntity {
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
        a0: input.a0,
        a1: input.a1,
    })
}
