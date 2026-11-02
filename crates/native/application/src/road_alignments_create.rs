//! `road.alignments.create` v1 (plan/road.md Y3, docs/adr/0901): a horizontal alignment from PIs, or a parallel axis of
//! one, last in the road project's alignments; one undo step “Aks oluştur”. The shared cases are
//! `fixtures/commands/v1/road.alignments.create.json`.
//!
//! The checks, in the contract's order: a road project; the definition and the alignments with it; the geometry (the
//! PIs, the km, a parallel axis's cusp); the expected revision.

use kentos_commands::{AlignmentDef, AlignmentsCreate, AlignmentsWritten, CommandResult};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::road_alignments as common;

pub(crate) const STEP: &str = "Aks oluştur";

struct Checked {
    list: Vec<AlignmentDef>,
    id: String,
}

fn check(doc: &kentos_domain::Document, input: &AlignmentsCreate) -> Result<Checked, Stop> {
    common::road_project(doc)?;
    let own = common::alignments(doc);
    let id = input.id.clone().unwrap_or_else(|| {
        (1..)
            .map(|n| format!("g-{n}"))
            .find(|id| own.iter().all(|a| &a.id != id))
            .unwrap_or_default()
    });
    let def = AlignmentDef {
        id: id.clone(),
        name: input.name.clone(),
        revision: 1,
        pis: input.pis.clone(),
        offset_of: input.offset_of.clone(),
        stationing: input.stationing.clone().filter(|s| !s.is_empty()),
    };
    let mut list = own.to_vec();
    list.push(def);
    common::judged(&list, list.len() - 1, "")?;
    let last = list.len() - 1;
    common::build(&list, &list[last], "")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok(Checked { list, id })
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &AlignmentsCreate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: vec![],
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &AlignmentsCreate) -> CommandResult<AlignmentsWritten> {
    let c = match check(cx.doc, input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let mut copy = cx.doc.reading_copy();
    let mut planned = write(&mut ExecutionContext::new(&mut copy), c);
    if let CommandResult::Completed { output, .. } = &mut planned {
        output.revision = cx.doc.revision().to_string();
    }
    planned
}

/// Writes the alignment: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: AlignmentsCreate) -> CommandResult<AlignmentsWritten> {
    crate::call::recorded(
        cx,
        kentos_commands::ROAD_ALIGNMENTS_CREATE,
        kentos_commands::ROAD_ALIGNMENTS_CREATE_VERSION,
        input,
        write_execute,
    )
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<AlignmentsWritten> {
    // A parallel axis has no elements of its own to map.
    let built = c
        .list
        .iter()
        .find(|a| a.id == c.id && a.offset_of.is_none())
        .and_then(|d| common::build(&c.list, d, "").ok());
    let mapping = common::mapping_rows(None, built.as_ref().map(|b| &b.core));
    common::write(cx, c.list, STEP, Some(&c.id), mapping, vec![])
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: AlignmentsCreate) -> CommandResult<AlignmentsWritten> {
    match check(cx.doc, &input) {
        Ok(c) => write(cx, c),
        Err(stop) => stop.into(),
    }
}
