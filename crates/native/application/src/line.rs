//! `cad.line.create` v1 (docs/adr/0027): one straight line on a named layer,
//! as one undo step. The desktop's handler over the native document; the
//! web's is `apps/web/src/product/lineCreate.ts`. Both pass the shared cases
//! in `fixtures/commands/v1/cad.line.create.json`.
//!
//! The line tool draws a chain and calls this once per segment: each segment
//! is its own object and its own undo step, as the tool always wrote them.
//!
//! The checks, in order (the first that fails answers):
//! 1. both ends finite: `a` then `b`, x before y;
//! 2. the expected revision, 3. the layer: every create command's
//!    (`checks.rs`), for the reasons `polygon.rs` gives.
//!
//! A line whose ends coincide is written: geometric validity is not checked,
//! as for the closed area. The tool never gives one (a second click on the
//! same point adds nothing).

use kentos_contracts::{
    CommandResult, CommandWarning, Entity, EntityBase, LineCreate, LineCreated, LineEntity,
    LinePlan,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &LineCreate) -> CommandResult<()> {
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
pub fn plan(cx: &ExecutionContext<'_>, input: &LineCreate) -> CommandResult<LinePlan> {
    match check(cx.doc, input) {
        Ok(warnings) => CommandResult::Completed {
            output: LinePlan {
                entity: line(input.clone(), 0),
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the line as one undo step (“Ekle”, the
/// document's own `add`): into the open transaction or group, if one is.
pub fn execute(cx: &mut ExecutionContext<'_>, input: LineCreate) -> CommandResult<LineCreated> {
    let warnings = match check(cx.doc, &input) {
        Ok(warnings) => warnings,
        Err(stop) => return stop.into(),
    };
    match checks::add(cx.doc, line(input, 0)) {
        Ok((id, uid, revision)) => CommandResult::Completed {
            output: LineCreated { uid, id, revision },
            warnings,
        },
        Err(error) => CommandResult::Failed { error },
    }
}

/// The checks in the contract's order; the warnings when the line may be written.
fn check(doc: &Document, input: &LineCreate) -> Result<Vec<CommandWarning>, Stop> {
    for (end, p, whose) in [
        ("a", input.a, "Başlangıç noktasının"),
        ("b", input.b, "Bitiş noktasının"),
    ] {
        for (axis, value) in [("x", p.x), ("y", p.y)] {
            if !value.is_finite() {
                return Err(checks::not_finite(whose, axis, format!("{end}.{axis}")));
            }
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    checks::layer(doc, &input.layer_id)
}

/// The line `input` describes, as the document stores it; `id` is the
/// document's to give (0 in a plan).
fn line(input: LineCreate, id: u32) -> Entity {
    Entity::Line(LineEntity {
        base: EntityBase {
            id,
            layer_id: input.layer_id,
            color: input.color,
            attrs: input.attrs.unwrap_or_default(),
            label: None,
            symbol: None,
        },
        a: input.a,
        b: input.b,
    })
}
