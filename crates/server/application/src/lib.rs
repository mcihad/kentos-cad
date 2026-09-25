//! KentOS use cases (CLAUDE.md §14, §18): one implementation for every
//! entry point. HTTP, the admin CLI and later MCP and the worker call these
//! functions; none of them re-implements a rule. The actor always comes from
//! a verified session or token, never from the request body, and every use
//! case on a project starts from the one access check (`access.rs`,
//! docs/adr/0015).

pub mod access;
pub mod admin;
pub mod cad;
pub mod changes;
pub mod commands;
pub mod error;
pub mod events;
mod idempotency;
pub mod identity;
pub mod lifecycle;
pub mod projects;
pub mod sharing;
pub mod tenancy;

pub use error::{AppError, AppResult};

/// The product commands this server runs, by name and version (docs/adr/0013).
/// The HTTP command route hands every envelope to [`commands::run`], which
/// accepts exactly these; `tests/catalog.rs` keeps the list equal to the
/// catalog's commands marked `server`.
pub const SERVER_COMMANDS: &[(&str, u32)] = &[
    (
        kentos_contracts::PROJECT_CHANGES,
        kentos_contracts::PROJECT_CHANGES_VERSION,
    ),
    (
        kentos_contracts::PROJECT_SHARE,
        kentos_contracts::PROJECT_SHARE_VERSION,
    ),
    (
        kentos_contracts::PROJECT_ACCESS_REVOKE,
        kentos_contracts::PROJECT_ACCESS_REVOKE_VERSION,
    ),
];
