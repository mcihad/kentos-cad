//! `cad.processing.batch` (docs/adr/0529, plan/gis.md CBS-24): one of İşlemler's tools or models run over a table of
//! rows of parameter values, each row the same run `cad.processing.run` makes, the whole one undo step written all or
//! not at all. `plan` runs it on a copy of the drawing. A stop asked while it runs (as a job, docs/adr/0253 §9) is
//! heard between the rows and takes back what the earlier rows did.

use kentos_commands::{CommandResult, ProcessingBatch, ProcessingBatched};

use crate::ExecutionContext;
use crate::processing_host::{BatchEnd, JobFlow, host};

pub fn validate(cx: &ExecutionContext<'_>, input: &ProcessingBatch) -> CommandResult<()> {
    match host().and_then(|h| h.check_batch(cx.doc, input)) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(error) => CommandResult::Failed { error: *error },
    }
}

fn ended(end: BatchEnd) -> CommandResult<ProcessingBatched> {
    match end {
        BatchEnd::Ran(output, warnings) => CommandResult::Completed { output, warnings },
        BatchEnd::Cancelled => CommandResult::Cancelled,
    }
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &ProcessingBatch,
) -> CommandResult<ProcessingBatched> {
    let flow = JobFlow(cx.job);
    let ran = host().and_then(|h| {
        h.check_batch(cx.doc, input)?;
        let mut copy = cx.doc.reading_copy();
        h.batch(&mut copy, input, false, &flow)
    });
    match ran {
        Ok(BatchEnd::Ran(mut output, warnings)) => {
            output.revision = cx.doc.revision().to_string();
            ended(BatchEnd::Ran(output, warnings))
        }
        Ok(end) => ended(end),
        Err(error) => CommandResult::Failed { error: *error },
    }
}

/// The batch as one recorded step of the document (docs/adr/0253 §1, docs/adr/0551 §6): in the journal, answered as
/// before for the same `requestId`, its answer (record included) in the project's request ledger.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ProcessingBatch,
) -> CommandResult<ProcessingBatched> {
    // A plan through the protocol runs this on a reading copy: a sink's file is not written then (docs/adr/0532 §4).
    let dry = cx.dry();
    crate::call::recorded_with(
        cx,
        kentos_commands::CAD_PROCESSING_BATCH,
        kentos_commands::CAD_PROCESSING_BATCH_VERSION,
        input,
        move |cx, input| write_execute(cx, input, dry),
        |out: &ProcessingBatched| crate::processing_run::file_effects(out.sink.as_ref()),
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: ProcessingBatch,
    dry: bool,
) -> CommandResult<ProcessingBatched> {
    let flow = JobFlow(cx.job);
    let ran = host().and_then(|h| {
        h.check_batch(cx.doc, &input)?;
        h.batch(cx.doc, &input, !dry, &flow)
    });
    match ran {
        Ok(end) => ended(end),
        Err(error) => CommandResult::Failed { error: *error },
    }
}

#[cfg(test)]
mod tests {
    use kentos_commands::{CommandResult, DocumentSnapshotV1, ProcessingBatch};
    use kentos_domain::Document;

    use crate::ExecutionContext;

    /// Nothing registers İşlemler's tools in this crate's own tests: the batch is refused, nothing written.
    #[test]
    fn without_tools_the_batch_is_refused() {
        let mut doc = Document::from_snapshot(
            DocumentSnapshotV1::from_json(include_str!(
                "../../../../fixtures/processing/v1/parcels.kcad"
            ))
            .expect("reads"),
        )
        .expect("opens");
        let cx = ExecutionContext::new(&mut doc);
        let batch = ProcessingBatch {
            environment: None,
            tool: "attributes.calculate".into(),
            parameters: Default::default(),
            rows: vec![Default::default()],
            selection: Vec::new(),
            view: None,
            drawing: None,
            on_error: Default::default(),
            sink: None,
        };
        let CommandResult::Failed { error } = super::validate(&cx, &batch) else {
            panic!("refused");
        };
        assert_eq!(error.code, "tools_unavailable");
        let CommandResult::Failed { error } = super::plan(&cx, &batch) else {
            panic!("refused");
        };
        assert_eq!(error.code, "tools_unavailable");
    }
}
