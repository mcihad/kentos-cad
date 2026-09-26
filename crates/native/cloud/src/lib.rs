//! KentOS cloud client of the desktop (docs/adr/0040, TODOS.md SYNC-01): the
//! protocol the web speaks (apps/web/src/app/cloud/), through a native
//! adapter of its own. The web keeps its TypeScript client; what makes the
//! two agree is the server and the shared contracts (`kentos-contracts`),
//! not shared code.
//!
//! - [`Cloud`]: the HTTP routes, signed in with a local account; the session
//!   lives in memory only.
//! - [`open()`]: a project into a drawing — a database project object by
//!   object with each object's version, a file project from its newest revision.
//! - [`saving`]: a file project's next revision, and a drawing as a new
//!   cloud project of either kind.
//! - [`ProjectSync`]: a database project's changes as `project.changes`
//!   commands, the web's tracker rules, driven by the desktop; other
//!   editors' commits taken in from the events ([`follow`]); what is not sent
//!   yet kept on this device ([`drafts`]) and put back when the project opens again.
//!
//! Every call runs on the crate's own runtime (runtime.rs) and can be
//! awaited on any executor, Iced's included. No UI here.
#![forbid(unsafe_code)]
// What the server sends must never crash the desktop.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod api;
pub mod drafts;
pub mod failure;
pub mod follow;
pub mod open;
pub mod replica;
mod runtime;
pub mod saving;
pub mod sync;

pub use api::{CatalogQuery, Cloud, Download, Progress};
pub use drafts::{DraftKey, DraftStore, Loaded};
pub use failure::{ApiFailure, Failure};
pub use open::{Opened, Revision, Source, open};
pub use replica::{Ended, Kept, Replica, ReplicaError, ReplicaStore};
pub use saving::{
    Uploaded, conflicting_revision, project_create, save_revision, save_revision_watched,
    upload_new,
};
pub use sync::{
    After, BaseMeta, BaseObject, BaseSnapshot, BaseStep, Conflict, Draft, DraftChange, DraftMeta,
    Incoming, ProjectSync, Remote, Restored, SaveState, Taken,
};
