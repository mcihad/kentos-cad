//! KentOS domain: the desktop's drawing document (docs/adr/0020, TODOS.md
//! DOM-02, TX-01), and later the server's.
//!
//! It is the native counterpart of the web's `CadDocument`
//! (apps/web/src/model/document.ts), not a facade over it or under it: the web
//! keeps its TypeScript document, the desktop owns this one. What makes them
//! agree is shared data, not shared code:
//!
//! - the versioned contracts (`kentos-contracts`: `DocumentSnapshotV1`,
//!   `Entity`, `LayerNode`) that both read and write;
//! - the operation fixtures in `fixtures/document-ops/v1`, which describe the
//!   web's behaviour step by step and which both run
//!   (`apps/web/src/model/documentOps.test.ts`, `tests/fixtures.rs`).
//!
//! Objects have a runtime slot (`u32`, the web's `Entity.id`) and a persistent
//! UUID (docs/adr/0014). Transactions are all or nothing, nested ones are
//! savepoints, every edit is one undo step (docs/adr/0003).
//!
//! Pure Rust, native only: no UI, GPU, database, async runtime or network
//! (scripts/arch/deps.mjs).
#![forbid(unsafe_code)]
// User data (a file, an edit) must never crash the document.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod document;
mod edit;
mod history;
mod identity;
mod layers;
mod snapshot;
mod store;

pub use document::Document;
pub use edit::{SlotsExhausted, labels};
pub use history::{Group, UNDO_LIMIT};
pub use identity::{Slot, Uuid, v1_entity_uids};
pub use kentos_contracts as contracts;
pub use layers::LayerTree;
