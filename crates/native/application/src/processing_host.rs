//! İşlemler's tools behind the processing commands (docs/adr/0508): the processing crate sits above this one (its
//! tools write through these commands' document), so a host that has it registers it here, as a PostGIS provider is
//! registered (`provider::register_postgis`). Without one `cad.processing.tools` and `.run` are refused with
//! `tools_unavailable`.

use std::sync::OnceLock;

use kentos_commands::{
    CommandError, CommandWarning, ProcessingBatch, ProcessingBatched, ProcessingRan, ProcessingRun,
    ProcessingTools, ProcessingToolsListed, codes,
};
use kentos_domain::Document;

/// What a batch asks of the command running it (docs/adr/0529 §1): whether it was asked to stop, asked between the
/// rows, and where it has come to.
pub trait BatchFlow {
    fn stopped(&self) -> bool;
    fn progress(&self, done: u64, total: u64);
}

/// The job's stop and progress, when a command runs as one; none otherwise (it is never stopped).
pub(crate) struct JobFlow<'a>(pub Option<&'a crate::jobs::Progress>);

impl BatchFlow for JobFlow<'_> {
    fn stopped(&self) -> bool {
        self.0.is_some_and(crate::jobs::Progress::stopped)
    }

    fn progress(&self, done: u64, total: u64) {
        if let Some(job) = self.0 {
            job.set(done, total);
        }
    }
}

/// How a run ended without being refused (docs/adr/0549 §3).
#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
pub enum RunEnd {
    /// It ran: its answer and warnings.
    Ran(ProcessingRan, Vec<CommandWarning>),
    /// It was stopped while its result went to a sink: nothing is written, the file is as it was.
    Cancelled,
}

/// How a batch ended without being refused.
#[allow(clippy::large_enum_variant)]
pub enum BatchEnd {
    /// Every row ran.
    Ran(ProcessingBatched),
    /// It was stopped between rows: what the earlier rows did is taken back, nothing is written.
    Cancelled,
}

/// İşlemler's tools as the commands reach them.
pub trait ToolHost: Send + Sync {
    /// The tools `input` asks for, with their parameters and this drawing's defaults.
    fn list(
        &self,
        doc: &Document,
        input: &ProcessingTools,
    ) -> Result<ProcessingToolsListed, Box<CommandError>>;
    /// The refusals a run would meet before it runs: an unknown tool, an id, a value.
    fn check(&self, doc: &Document, input: &ProcessingRun) -> Result<(), Box<CommandError>>;
    /// The run on `doc`, written as one undo step named by the tool; with a sink the run is made on a copy and its
    /// result written to the file (only when `write`: a plan writes nothing). Its warnings beside it. `flow` is asked
    /// between the sink's batches of rows and told how far they have come.
    fn run(
        &self,
        doc: &mut Document,
        input: &ProcessingRun,
        write: bool,
        flow: &dyn BatchFlow,
    ) -> Result<RunEnd, Box<CommandError>>;
    /// The refusals a batch would meet before it runs: its size, then each row's, as `check` gives them.
    fn check_batch(&self, doc: &Document, input: &ProcessingBatch)
    -> Result<(), Box<CommandError>>;
    /// The batch on `doc`: its rows one after another, one undo step, all or nothing; `flow` is asked between rows.
    fn batch(
        &self,
        doc: &mut Document,
        input: &ProcessingBatch,
        flow: &dyn BatchFlow,
    ) -> Result<BatchEnd, Box<CommandError>>;
}

static TOOLS: OnceLock<Box<dyn ToolHost>> = OnceLock::new();

/// Registers the host's tools; the first registration stays.
pub fn register_tools(host: Box<dyn ToolHost>) {
    let _ = TOOLS.set(host);
}

/// The host's tools, none where the host registered none.
pub fn tools() -> Option<&'static dyn ToolHost> {
    TOOLS.get().map(|b| b.as_ref())
}

/// The host's tools, or why the commands do not run here.
pub(crate) fn host() -> Result<&'static dyn ToolHost, Box<CommandError>> {
    tools().ok_or_else(|| Box::new(CommandError {
        code: codes::TOOLS_UNAVAILABLE.into(),
        message: "Bu ortamda işlem araçları yok: masaüstünde, başsız sunucuda, Python'da ya da MCP'de çalıştırın."
            .into(),
        ..CommandError::default()
    }))
}
