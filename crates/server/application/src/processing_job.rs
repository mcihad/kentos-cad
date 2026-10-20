//! An İşlemler tool, a file import or an atlas as a server job (docs/adr/0559, 0561; plan/gis.md CBS-39): what the
//! application layer decides without running anything, and the seam the work runs through.
//!
//! - **The request and the delta** are the contracts' (`kentos_contracts::processing_job`, shared with the process that
//!   runs the tool, which must not carry the server); re-exported here: the input checked when it is asked for
//!   ([`check_input`]), what a tool did ([`Processed`], [`ProcessFailure`]), what it changed ([`Delta`], [`delta`]).
//! - **The budget** ([`budget_of`]): the server's CPU and memory limits for one job, or smaller ones the request asks
//!   for.
//! - **The seam** ([`Processor`]): the server crate does not know the tools (they are the native crates', and the
//!   server may not use them: `scripts/arch/deps.mjs`). The host (`kentosd`) gives a processor that runs the tool in a
//!   process of its own with a CPU and memory budget; a test gives a fake. A processor gets the project's `.kcad` at
//!   the revision the worker read and answers with what it did: a [`Delta`] to commit or a file (a GeoPackage, the
//!   atlas's PDF).

use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, PoisonError};

use std::path::PathBuf;

use kentos_contracts::{
    ProcessingJobInput, SERVER_JOB_CPU_DEFAULT_SECONDS, SERVER_JOB_MEMORY_DEFAULT_MB,
};
use uuid::Uuid;

pub use kentos_contracts::processing_job::{
    Delta, DeltaRefusal, ImportRun, InputFault, JOB_CHANGES_MAX, PROCESSING_MEMORY_FLOOR_MB,
    PROCESSING_PARAMETERS_MAX, PROCESSING_SELECTION_MAX, ProcessFailure, Processed, RunRequest,
    check_atlas_input, check_import_input, check_input, delta,
};

use crate::jobs::JobLimits;

/// What one job may take: CPU seconds of the process the tool runs in, and its resident memory in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProcessingBudget {
    pub cpu_seconds: u32,
    pub memory_bytes: u64,
}

/// The budget of a request: the server's, or a smaller one the request asks for.
pub fn budget_of(input: &ProcessingJobInput, limits: &JobLimits) -> ProcessingBudget {
    budget_from(input.cpu_seconds, input.memory_mb, limits)
}

/// [`budget_of`] for any kind of job: the cpu and memory a request asked for, if it did.
pub fn budget_from(
    cpu_seconds: Option<u32>,
    memory_mb: Option<u32>,
    limits: &JobLimits,
) -> ProcessingBudget {
    let cpu = match cpu_seconds {
        Some(own) => own.min(limits.cpu_seconds),
        None => limits.cpu_seconds,
    };
    let mb = match memory_mb {
        Some(own) => own.min(limits.memory_mb),
        None => limits.memory_mb,
    };
    ProcessingBudget {
        cpu_seconds: cpu,
        memory_bytes: u64::from(mb) * 1024 * 1024,
    }
}

impl Default for ProcessingBudget {
    fn default() -> Self {
        ProcessingBudget {
            cpu_seconds: SERVER_JOB_CPU_DEFAULT_SECONDS,
            memory_bytes: u64::from(SERVER_JOB_MEMORY_DEFAULT_MB) * 1024 * 1024,
        }
    }
}

// ── The seam ────────────────────────────────────────────────────────────────

/// What a running tool says about itself: how far (0–1) and in a few words. The worker writes it at its beats, so the
/// people who poll the job see it.
#[derive(Debug, Default)]
pub struct JobProgress {
    said: Mutex<Option<(f32, String)>>,
}

impl JobProgress {
    pub fn set(&self, fraction: f32, text: impl Into<String>) {
        *self.said.lock().unwrap_or_else(PoisonError::into_inner) =
            Some((fraction.clamp(0.0, 1.0), text.into()));
    }

    /// What was last said; none before the first word.
    pub fn get(&self) -> Option<(f32, String)> {
        self.said
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// A staged file an import reads: where the object store keeps it (a file the processor links or copies, not bytes in
/// memory: it may be gigabytes), its name as its owner gave it and its size.
#[derive(Clone, Debug)]
pub struct StagedFile {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
}

/// What a processor is given.
pub struct ProcessingTask {
    pub job: Uuid,
    /// The project as one `.kcad` file at the revision the worker read.
    pub snapshot: Vec<u8>,
    /// What to run: a tool, an import, an atlas.
    pub request: RunRequest,
    /// An import's file.
    pub file: Option<StagedFile>,
    pub budget: ProcessingBudget,
}

impl ProcessingTask {
    /// The tool's request, when it is a tool the task runs.
    pub fn tool(&self) -> Option<&ProcessingJobInput> {
        match &self.request {
            RunRequest::Processing(input) => Some(input),
            _ => None,
        }
    }
}

/// The worker's side of a running processor: it says what it does, and it is told to stop.
#[derive(Clone)]
pub struct ProcessHooks {
    pub progress: Arc<JobProgress>,
    pub stop: Arc<AtomicBool>,
}

/// Runs an İşlemler tool, an import or an atlas for the server (docs/adr/0559 §3, 0561 §3): the host's, since the tools
/// and the commands are not the server's.
/// `process` blocks (the worker runs it on a blocking thread); it says its progress to `hooks.progress`, stops at
/// `hooks.stop`, and keeps the task's budget.
pub trait Processor: Send + Sync {
    fn process(
        &self,
        task: &ProcessingTask,
        hooks: &ProcessHooks,
    ) -> Result<Processed, ProcessFailure>;
}
