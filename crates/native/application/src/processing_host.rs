//! İşlemler's tools behind the processing commands (docs/adr/0508): the processing crate sits above this one (its
//! tools write through these commands' document), so a host that has it registers it here, as a PostGIS provider is
//! registered (`provider::register_postgis`). Without one `cad.processing.tools` and `.run` are refused with
//! `tools_unavailable`.

use std::sync::OnceLock;

use kentos_contracts::{
    CommandError, ProcessingRan, ProcessingRun, ProcessingTools, ProcessingToolsListed, codes,
};
use kentos_domain::Document;

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
    /// The run on `doc`, written as one undo step named by the tool.
    fn run(
        &self,
        doc: &mut Document,
        input: &ProcessingRun,
    ) -> Result<ProcessingRan, Box<CommandError>>;
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
