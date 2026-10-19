//! `cad.processing.run` (docs/adr/0508, plan/gis.md CBS-41): one of İşlemler's tools with explicit values, the
//! runner the desktop's İşlemler window runs, one undo step named by the tool. `plan` runs it on a copy of the
//! drawing: what it would write, by the same algorithm (the new objects' ids are the copy's).

use kentos_contracts::{CommandResult, ProcessingRan, ProcessingRun};

use crate::ExecutionContext;
use crate::processing_host::host;

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
    let ran = host().and_then(|h| {
        h.check(cx.doc, input)?;
        let mut copy = cx.doc.reading_copy();
        h.run(&mut copy, input, false)
    });
    match ran {
        Ok((mut output, warnings)) => {
            output.revision = cx.doc.revision().to_string();
            CommandResult::Completed { output, warnings }
        }
        Err(error) => CommandResult::Failed { error: *error },
    }
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: ProcessingRun,
) -> CommandResult<ProcessingRan> {
    // A plan through the protocol runs this on a reading copy: a sink's file is not written then (docs/adr/0532 §4).
    let dry = cx.dry();
    let ran = host().and_then(|h| {
        h.check(cx.doc, &input)?;
        h.run(cx.doc, &input, !dry)
    });
    match ran {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(error) => CommandResult::Failed { error: *error },
    }
}

#[cfg(test)]
mod tests {
    use kentos_contracts::{CommandResult, DocumentSnapshotV1, ProcessingRun, ProcessingTools};
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
