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

mod annotative;
pub mod arrays;
pub mod constraints;
mod constraint_follow;
pub mod audit;
mod blocks;
mod calls;
mod changes;
pub mod dimension_ties;
mod document;
mod edit;
pub mod etag;
pub mod exchange;
pub mod xref;
mod external;
mod followers;
mod form_follow;
mod relation_follow;
mod related;
pub use related::{Related, Tie};
pub use relation_follow::RelationEffects;
pub mod fields;
mod hash;
pub mod hatch_boundaries;
mod hatch_ties;
mod history;
mod import_metadata;
mod template_settings;
mod identity;
mod integrity;
pub mod layer_purge;
pub mod layer_states;
mod layers;
pub mod leader_ties;
pub mod opening_follow;
pub mod space_follow;
pub mod arch_follow;
pub mod arch_values;
pub mod tag_read;
mod tag_follow;
pub mod viewmark_follow;
pub mod schedules;
mod linked;
mod patch;
mod snapshot;
mod step;
mod store;
mod wall_candidates;
mod style_overrides;
pub mod table_cells;
pub mod text_fields;
mod units;
mod domains;
mod source_steps;
mod lineage;

pub use calls::{REFUSED_CALLS, RefusedAs, RefusedCall};
pub use changes::{ChangeMark, Changes};
pub use constraint_follow::{Relax, cad_element};
pub use document::{Document, ObjectPolicy};
pub use step::Written;
pub use edit::{Refusal, SlotsExhausted, labels};
pub use external::{External, ExternalMeta};
pub use hash::{SlotHasher, SlotMap};
pub use history::{Group, LayerInfo, Savepoint, ServedBy, Temporal, UNDO_LIMIT};
pub use identity::{Slot, Uuid, v1_entity_uids};
pub use integrity::Fault;
pub use kentos_contracts as contracts;
pub use layers::{LayerTree, NewLayer, default_style};
pub use patch::{Patch, PatchRefused};
pub use step::{BlockChange, IdChanges, ObjectChange, Sealed, StepReport};
pub use units::UnitState;
