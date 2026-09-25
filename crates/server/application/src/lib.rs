//! KentOS use cases (CLAUDE.md §14, §18): one implementation for every
//! entry point. HTTP, the admin CLI and later MCP and the worker call these
//! functions; none of them re-implements a rule. The actor always comes from
//! a verified session or token, never from the request body.

pub mod admin;
pub mod cad;
pub mod changes;
pub mod error;
pub mod events;
pub mod identity;
pub mod lifecycle;
pub mod projects;
pub mod tenancy;

pub use error::{AppError, AppResult};

/// The product commands this server runs, by name and version (docs/adr/0013).
/// The HTTP command route hands every envelope to [`changes::commit`], which
/// accepts exactly these; `tests/catalog.rs` keeps the list equal to the
/// catalog's commands marked `server`.
pub const SERVER_COMMANDS: &[(&str, u32)] = &[(
    kentos_contracts::PROJECT_CHANGES,
    kentos_contracts::PROJECT_CHANGES_VERSION,
)];
