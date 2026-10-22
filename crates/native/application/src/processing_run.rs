//! `cad.processing.run` (docs/adr/0508, plan/gis.md CBS-41): one of İşlemler's tools with explicit values, the
//! runner the desktop's İşlemler window runs, one undo step named by the tool. `plan` runs it on a copy of the
//! drawing: what it would write, by the same algorithm (the new objects' ids are the copy's).

use kentos_commands::{CommandResult, FileEffect, ProcessingRan, ProcessingRun};

use crate::ExecutionContext;
use crate::processing_host::{JobFlow, RunEnd, host};

pub fn validate(cx: &ExecutionContext<'_>, input: &ProcessingRun) -> CommandResult<()> {
    match host().and_then(|h| h.check(cx.doc, input)) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(error) => CommandResult::Failed { error: *error },
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ProcessingRun) -> CommandResult<ProcessingRan> {
    let flow = JobFlow(cx.job);
    let ran = host().and_then(|h| {
        h.check(cx.doc, input)?;
        let mut copy = cx.doc.reading_copy();
        h.run(&mut copy, input, false, &flow)
    });
    match ran {
        Ok(RunEnd::Ran(mut output, warnings)) => {
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings }
        }
        Ok(RunEnd::Cancelled) => CommandResult::Cancelled,
        Err(error) => CommandResult::Failed { error: *error },
    }
}

/// The run as one recorded step of the document (docs/adr/0253 §1, docs/adr/0551 §6): it is in the journal, a
/// `requestId` makes it answered as before when sent again and keeps its answer, record included, in the project's
/// request ledger.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ProcessingRun,
) -> CommandResult<ProcessingRan> {
    // A plan through the protocol runs this on a reading copy: a sink's file is not written then (docs/adr/0532 §4).
    let dry = cx.dry();
    crate::call::recorded_with(
        cx,
        kentos_commands::CAD_PROCESSING_RUN,
        kentos_commands::CAD_PROCESSING_RUN_VERSION,
        input,
        move |cx, input| write_execute(cx, input, dry),
        |out: &ProcessingRan| file_effects(out.sink.as_ref()),
    )
}

/// The file a sink wrote, as the journal keeps a file command's write (docs/adr/0593 §4): its path, size and SHA-256.
/// A database is no file; a plan's passing copy is none either.
pub(crate) fn file_effects(sink: Option<&kentos_commands::GpkgExported>) -> Vec<FileEffect> {
    sink.filter(|s| !kentos_commands::is_database(&s.path) && !s.sha256.is_empty())
        .map(|s| FileEffect {
            path: s.path.clone(),
            bytes: s.bytes,
            sha256: s.sha256.clone(),
        })
        .into_iter()
        .collect()
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: ProcessingRun,
    dry: bool,
) -> CommandResult<ProcessingRan> {
    let flow = JobFlow(cx.job);
    let ran = host().and_then(|h| {
        h.check(cx.doc, &input)?;
        h.run(cx.doc, &input, !dry, &flow)
    });
    match ran {
        Ok(RunEnd::Ran(output, warnings)) => CommandResult::Completed { output, warnings },
        Ok(RunEnd::Cancelled) => CommandResult::Cancelled,
        Err(error) => CommandResult::Failed { error: *error },
    }
}

#[cfg(test)]
mod tests {
    use kentos_commands::{CommandResult, DocumentSnapshotV1, ProcessingRun, ProcessingTools};
    use kentos_domain::Document;

    use crate::ExecutionContext;

    /// Nothing registers İşlemler's tools in this crate's own tests: both commands are refused, nothing written.
    #[test]
    fn without_tools_both_commands_are_refused() {
        let mut doc = Document::from_snapshot(
            DocumentSnapshotV1::from_json(include_str!(
                "../../../../fixtures/processing/v1/parcels.kcad"
            ))
            .expect("reads"),
        )
        .expect("opens");
        let cx = ExecutionContext::new(&mut doc);
        let run = ProcessingRun {
            tool: "attributes.calculate".into(),
            parameters: Default::default(),
            selection: Vec::new(),
            view: None,
            sink: None,
            drawing: None,
        };
        let CommandResult::Failed { error } = super::validate(&cx, &run) else {
            panic!("refused");
        };
        assert_eq!(error.code, "tools_unavailable");
        let CommandResult::Failed { error } =
            crate::processing_tools::plan(&cx, &ProcessingTools::default())
        else {
            panic!("refused");
        };
        assert_eq!(error.code, "tools_unavailable");
    }
}
