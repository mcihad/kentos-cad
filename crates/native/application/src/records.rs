//! `cad.records.create` v1 (docs/adr/0410 §3): records without a geometry, a table's rows, on a named
//! layer, as one undo step (“Kayıt ekle”). Each row's attributes go through the layer's fields
//! (`checks::object_attrs`); a form applied to the layer completes them in the same step, as it does
//! every write of the layer (`form_follow.rs`). The shared cases are
//! `fixtures/commands/v1/cad.records.create.json`.
//!
//! The checks, in order (the first that fails answers): the rows' count, the expected revision, the
//! layer (every create command's), each row's attributes in the rows' order.

use kentos_contracts::{
    CommandResult, CommandWarning, Entity, EntityBase, MAX_RECORDS, RecordEntity, RecordsCreate,
    RecordsCreated, RecordsPlan,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::forms::fail;

/// The undo step's name.
pub const STEP: &str = "Kayıt ekle";

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &RecordsCreate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok((warnings, _)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now, and the revision to expect for exactly that; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &RecordsCreate) -> CommandResult<RecordsPlan> {
    match check(cx.doc, input) {
        Ok((warnings, entities)) => CommandResult::Completed {
            output: RecordsPlan {
                entities,
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the records as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: RecordsCreate,
) -> CommandResult<RecordsCreated> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_RECORDS_CREATE,
        kentos_contracts::CAD_RECORDS_CREATE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: RecordsCreate,
) -> CommandResult<RecordsCreated> {
    let (warnings, entities) = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    cx.doc.name_step(STEP);
    let mut out = RecordsCreated {
        uids: Vec::with_capacity(entities.len()),
        ids: Vec::with_capacity(entities.len()),
        revision: String::new(),
    };
    for e in entities {
        match checks::add(cx.doc, e) {
            Ok((id, uid, revision)) => {
                out.ids.push(id);
                out.uids.push(uid);
                out.revision = revision;
            }
            Err(stop) => return stop.into(),
        }
    }
    CommandResult::Completed {
        output: out,
        warnings,
    }
}

/// The checks in the contract's order; the warnings and the records as they are written.
fn check(
    doc: &Document,
    input: &RecordsCreate,
) -> Result<(Vec<CommandWarning>, Vec<Entity>), Stop> {
    if input.rows.is_empty() || input.rows.len() > MAX_RECORDS {
        return Err(fail(
            codes::INVALID_COUNT,
            format!("Kayıt sayısı 1 ile {MAX_RECORDS} arasında olmalı."),
            "rows",
        ));
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    let mut entities = Vec::with_capacity(input.rows.len());
    for (i, row) in input.rows.iter().enumerate() {
        let attrs = checks::object_attrs(
            doc,
            &input.layer_id,
            row.attrs.as_ref(),
            &format!("rows.{i}.attrs"),
        )?;
        entities.push(Entity::Record(RecordEntity {
            base: EntityBase {
                look: Default::default(),
                id: 0,
                layer_id: input.layer_id.clone(),
                color: None,
                attrs: attrs.unwrap_or_default(),
                label: row.label.clone().filter(|l| !l.trim().is_empty()),
                symbol: None,
                line_weight: None,
                label_pins: Vec::new(),
                attachments: Vec::new(),
            },
        }));
    }
    Ok((warnings, entities))
}
