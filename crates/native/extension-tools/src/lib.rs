//! An extension's İşlemler tools run in its own Python process (plan/eklenti.md ADIM 19; docs/adr/8057 §7, §70): what
//! the desktop and the headless hosts share, so that a tool runs the same way wherever it is called (PY-TOOL-21).
//!
//! - [`plan`]: the tools a registry's extensions add, read from their descriptors without their code; what running
//!   each needs ([`Plan`]); how the extension version's process starts in its prepared environment ([`start_of`]).
//! - [`run_tool`]: one run in the version's process (`kentos_python_env::process::Pool`): its inputs as data, its
//!   progress, log and printed lines to İşlemler' feedback, Durdur asked of its code, how it ended read into İşlemler'
//!   answer and a [`Note`] for its host (the desktop writes the extension's log, the notice and the watchdog from it; a
//!   headless host the same log record, and says it to its caller).
//! - [`calls`]: what the code asks of KentOS while it runs: one of KentOS's built-in tools on a drawing of the call's
//!   own (docs/adr/8057 §47), never the user's.
//! - [`examples`]: a tool's examples run on drawings of their own and checked against what each expects (§48): the
//!   desktop's Örnekleri çalıştır, an agent's trial and `kentos kek test` (§74).
//! - [`headless`]: the headless hosts' provider (MCP, the Python library): the user's installed and enabled
//!   extensions' tools that declare `headless-python`, offered from a host's start and never over the desktop's.
//!
//! A separate process is not a sandbox (PY-RUN-19, 20): an extension runs with the user's rights; KentOS bounds only
//! what it gives the code from the drawing.

pub mod calls;
pub mod examples;
pub mod headless;
mod note;
pub mod plan;
mod run;
pub mod synthetic;

pub use note::Note;
pub use plan::{Library, Listed, Plan, Python, environment_of, read_tools, start_of};
pub use run::{answered, run_in_pool, run_tool};

#[cfg(all(test, unix))]
mod tests;
