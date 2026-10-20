//! `cad.processing.batch` (docs/adr/0529, plan/gis.md CBS-24): one of İşlemler's tools or models run over a table of
//! rows of parameter values, each row the same run `cad.processing.run` makes, the whole one undo step written all or
//! not at all. `plan` runs it on a copy of the drawing. A stop asked while it runs (as a job, docs/adr/0253 §9) is
//! heard between the rows and takes back what the earlier rows did.

use kentos_commands::{CommandResult, ProcessingBatch, ProcessingBatched};

use crate::ExecutionContext;
use crate::jobs::Progress;
use crate::processing_host::{BatchEnd, BatchFlow, host};

/// The job's stop and progress, when the batch runs as one; none otherwise (it is never stopped).
struct Flow<'a>(Option<&'a Progress>);

impl BatchFlow for Flow<'_> {
    fn stopped(&self) -> bool {
        self.0.is_some_and(Progress::stopped)
    }

    fn progress(&self, done: u64, total: u64) {
        if let Some(job) = self.0 {
            job.set(done, total);
        }
    }
}

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
        BatchEnd::Ran(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        BatchEnd::Cancelled => CommandResult::Cancelled,
    }
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &ProcessingBatch,
) -> CommandResult<ProcessingBatched> {
    let flow = Flow(cx.job);
    let ran = host().and_then(|h| {
        h.check_batch(cx.doc, input)?;
        let mut copy = cx.doc.reading_copy();
        h.batch(&mut copy, input, &flow)
    });
    match ran {
        Ok(BatchEnd::Ran(mut output)) => {
            output.revision = cx.doc.revision().to_string();
            ended(BatchEnd::Ran(output))
        }
        Ok(end) => ended(end),
        Err(error) => CommandResult::Failed { error: *error },
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ProcessingBatch,
) -> CommandResult<ProcessingBatched> {
    let job = cx.job;
    let flow = Flow(job);
    let ran = host().and_then(|h| {
        h.check_batch(cx.doc, &input)?;
        h.batch(cx.doc, &input, &flow)
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
            tool: "attributes.calculate".into(),
            parameters: Default::default(),
            rows: vec![Default::default()],
            selection: Vec::new(),
            view: None,
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
