//! KentOS use cases (CLAUDE.md §14, §18): one implementation for every
//! entry point. HTTP, the admin CLI and later MCP and the worker call these
//! functions; none of them re-implements a rule. The actor always comes from
//! a verified session or token, never from the request body, and every use
//! case on a project starts from the one access check (`access.rs`,
//! docs/adr/0015).

pub mod access;
pub mod admin;
pub mod blobs;
pub mod cad;
pub mod catalog;
pub mod changes;
pub mod checkpoints;
pub mod commands;
pub mod convert;
pub mod duplicate;
pub mod error;
pub mod events;
pub mod files;
mod idempotency;
pub mod identity;
pub mod importing;
pub mod invitations;
mod journal;
pub mod lifecycle;
pub mod listing;
pub mod people;
pub mod projects;
pub mod restore;
pub mod sharing;
pub mod snapshot;
pub mod tenancy;

pub use error::{AppError, AppResult};

/// The product commands this server runs, by name and version (docs/adr/0013).
/// The HTTP command routes hand every envelope to [`commands::run`] (a
/// project's) or [`commands::run_in_tenant`] (`project.create`), which accept
/// exactly these; `tests/catalog.rs` keeps the list equal to the catalog's
/// commands marked `server`.
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
    // Invitations by link (docs/adr/0035).
    (
        kentos_contracts::PROJECT_INVITE,
        kentos_contracts::PROJECT_INVITE_VERSION,
    ),
    (
        kentos_contracts::PROJECT_INVITATION_REVOKE,
        kentos_contracts::PROJECT_INVITATION_REVOKE_VERSION,
    ),
    // The project catalog and lifecycle (docs/adr/0028).
    (
        kentos_contracts::PROJECT_CREATE,
        kentos_contracts::PROJECT_CREATE_VERSION,
    ),
    (
        kentos_contracts::PROJECT_RENAME,
        kentos_contracts::PROJECT_RENAME_VERSION,
    ),
    (
        kentos_contracts::PROJECT_METADATA_UPDATE,
        kentos_contracts::PROJECT_METADATA_UPDATE_VERSION,
    ),
    (
        kentos_contracts::PROJECT_DUPLICATE,
        kentos_contracts::PROJECT_DUPLICATE_VERSION,
    ),
    (
        kentos_contracts::PROJECT_ARCHIVE,
        kentos_contracts::PROJECT_ARCHIVE_VERSION,
    ),
    (
        kentos_contracts::PROJECT_UNARCHIVE,
        kentos_contracts::PROJECT_UNARCHIVE_VERSION,
    ),
    (
        kentos_contracts::PROJECT_TRASH,
        kentos_contracts::PROJECT_TRASH_VERSION,
    ),
    (
        kentos_contracts::PROJECT_RESTORE,
        kentos_contracts::PROJECT_RESTORE_VERSION,
    ),
    (
        kentos_contracts::PROJECT_PURGE,
        kentos_contracts::PROJECT_PURGE_VERSION,
    ),
    (
        kentos_contracts::PROJECT_FAVORITE,
        kentos_contracts::PROJECT_FAVORITE_VERSION,
    ),
    // File projects (docs/adr/0031).
    // Checkpoints (docs/adr/0034).
    (
        kentos_contracts::PROJECT_CHECKPOINT_CREATE,
        kentos_contracts::PROJECT_CHECKPOINT_CREATE_VERSION,
    ),
    (
        kentos_contracts::PROJECT_CHECKPOINT_DELETE,
        kentos_contracts::PROJECT_CHECKPOINT_DELETE_VERSION,
    ),
    (
        kentos_contracts::PROJECT_CHECKPOINT_RESTORE,
        kentos_contracts::PROJECT_CHECKPOINT_RESTORE_VERSION,
    ),
    (
        kentos_contracts::PROJECT_CONVERT,
        kentos_contracts::PROJECT_CONVERT_VERSION,
    ),
    (
        kentos_contracts::PROJECT_IMPORT,
        kentos_contracts::PROJECT_IMPORT_VERSION,
    ),
    (
        kentos_contracts::PROJECT_FILE_COMMIT,
        kentos_contracts::PROJECT_FILE_COMMIT_VERSION,
    ),
];
