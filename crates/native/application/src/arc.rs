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
//!    radius above zero; then the line weight from 0 to 100 mm when given;
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
                entity: arc(settled(cx.doc, input.clone()), 0),
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
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_ARC_CREATE,
        kentos_contracts::CAD_ARC_CREATE_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(cx: &mut ExecutionContext<'_>, input: ArcCreate) -> CommandResult<ArcCreated> {
    let warnings = match check(cx.doc, &input) {
        Ok(warnings) => warnings,
        Err(stop) => return stop.into(),
    };
    match checks::add(cx.doc, arc(settled(cx.doc, input), 0)) {
        Ok((id, uid, revision)) => CommandResult::Completed {
            output: ArcCreated { uid, id, revision },
            warnings,
        },
        Err(stop) => stop.into(),
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
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    checks::object_attrs(doc, &input.layer_id, input.attrs.as_ref(), "attrs")?;
    Ok(warnings)
}

/// The arc `input` describes, as the document stores it; `id` is the
/// document's to give (0 in a plan).
fn arc(input: ArcCreate, id: u32) -> Entity {
    Entity::Arc(ArcEntity {
        base: EntityBase {
            look: kentos_contracts::ObjectLook {
                line_type: input.line_type,
                ..Default::default()
            },
            id,
            layer_id: input.layer_id,
            color: input.color,
            attrs: input.attrs.unwrap_or_default(),
            label: input.label,
            symbol: input.symbol,
            line_weight: input.line_weight,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        c: input.c,
        r: input.r,
        a0: input.a0,
        a1: input.a1,
    })
}

/// `input` with the attributes the object is written with: canonical, the fields' defaults filled
/// (`checks::object_attrs`; the checks refused what it refuses).
fn settled(doc: &Document, mut input: ArcCreate) -> ArcCreate {
    input.attrs = checks::object_attrs(doc, &input.layer_id, input.attrs.as_ref(), "attrs")
        .ok()
        .flatten();
    input
}
