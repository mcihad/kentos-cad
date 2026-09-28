//! KentOS's headless command host (TODOS.md §14–15, docs/adr/0130): a
//! drawing without a window, and everything a program may do with it
//! through the product command system.
//!
//! - [`Session`]: a drawing opened from a `.kcad` file (v2, or v1 JSON) or
//!   started as a new project, saved as `.kcad` v2 the way the desktop saves.
//! - [`Session::run`]: a catalog command by name, `validate`, `plan` or
//!   `execute`, its input and answer as JSON (the catalog's schemas). The
//!   commands are the desktop's own handlers (`kentos_native_application`),
//!   so a script, an agent and the window write alike, one undo step each.
//! - [`catalog`]: every command with its schemas, hosts and examples; the
//!   ones the server runs are listed too, and refused here with the reason.
//! - Queries ([`Session::layers`], [`Session::entities`], [`Session::entity`])
//!   and measures ([`Session::measure`]) read without writing.
//! - [`rpc::call`]: the same by name over a document another host owns (the
//!   desktop's open drawing, asked by its Python console).
//!
//! JSON crosses the boundary on purpose: the Python SDK (`kentos.cad`) and
//! the MCP server speak the catalog's own wire form, which the web and the
//! server speak too. Objects are named by their persistent ids (UUID text),
//! never by their slots, which mean nothing once a drawing is closed.
#![forbid(unsafe_code)]
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod dispatch;
mod error;
mod files;
mod query;
pub mod rpc;
mod session;

pub use dispatch::{DESKTOP, Op};
pub use error::HeadlessError;
pub use query::{Measure, Page};
pub use session::{NewDrawing, Session};

/// The command catalog as JSON: every product command, its version, title,
/// summary, hosts, permissions, input and output schemas and examples
/// (`kentos_contracts::catalog`, the file the web and the server read).
pub fn catalog() -> serde_json::Value {
    serde_json::to_value(kentos_contracts::catalog::catalog()).unwrap_or(serde_json::Value::Null)
}
