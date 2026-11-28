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
pub mod audit;
mod blocks;
mod calls;
pub mod casework;
mod changes;
mod constraint_follow;
pub mod constraints;
pub mod copy_links;
mod data_edits;
mod data_pending;
pub mod dimension_layout;
pub mod dimension_sets;
pub mod dimension_spans;
pub mod dimension_ties;
mod document;
mod edit;
pub mod etag;
pub mod exchange;
mod external;
mod followers;
pub mod format;
pub mod xref;
pub use followers::FollowerDelta;
mod form_follow;
mod related;
mod relation_follow;
mod relation_keys;
pub use related::{Related, Tie};
pub use relation_follow::RelationEffects;
pub use relation_keys::ParentKey;
pub mod arch_follow;
mod arch_modules;
pub mod arch_values;
pub mod color_plan;
mod deliveries;
pub mod derived_roles;
mod domains;
mod element_types;
pub mod escape;
pub mod fields;
pub mod fire_elements;
pub mod fire_use;
mod form_answers;
mod hash;
pub mod hatch_boundaries;
mod hatch_ties;
mod history;
mod identity;
mod import_metadata;
mod integrity;
pub mod joinery_list;
pub mod layer_purge;
pub mod layer_states;
mod layers;
pub mod leader_ties;
pub mod level_marks;
mod linked;
mod materials;
mod opening_cells;
pub mod opening_follow;
mod opening_types;
pub mod paiy_areas;
pub mod party_walls;
pub mod passages;
mod patch;
mod performance;
pub mod projections;
pub mod provisions;
pub mod renovation_legend;
mod review_notes;
mod road_alignments;
mod road_profiles;
mod road_sections;
mod room_height;
pub mod room_names;
pub mod room_openings;
mod room_walls;
pub mod room_width;
pub mod schedules;
pub mod separations;
pub mod site;
pub mod slab_bound;
pub mod slab_follow;
mod snapshot;
pub mod space_follow;
#[cfg(test)]
mod space_phase_tests;
mod step;
mod store;
mod structure_index;
mod style_overrides;
mod style_tables;
pub mod table_cells;
mod tag_follow;
pub mod tag_read;
mod template_settings;
pub mod text_fields;
pub mod title_facts;
pub mod under_stair;
mod underlays;
mod units;
pub mod vertical_links;
mod view_area;
pub mod viewmark_follow;
mod wall_candidates;
mod wall_height;
pub mod wall_insulation;
pub mod wall_pieces;
pub mod zoning;
pub use room_height::{CeilingFrom, FloorFrom, RoomHeight, RoomStructure};
pub use view_area::ViewArea;
pub use wall_height::WallTop;
pub mod arch_options;
mod lineage;
mod map_survey;
mod map_work;
mod parcel_register;
mod source_steps;
mod zoning_plan;

pub use calls::{REFUSED_CALLS, RefusedAs, RefusedCall};
pub use changes::{ChangeMark, Changes};
pub use constraint_follow::{Relax, cad_element};
pub use data_edits::{DELETE_ROWS, DataChange, DataChanges, DataWritten, Taken};
pub use data_pending::{PendingLayer, PendingRow};
pub use document::{Document, ObjectPolicy};
pub use edit::{Refusal, SlotsExhausted, labels};
pub use external::{External, ExternalMeta};
pub use hash::{SlotHasher, SlotMap};
pub use history::{Group, LayerInfo, Savepoint, ServedBy, Temporal, UNDO_LIMIT};
pub use identity::{Slot, Uuid, new_uid, v1_entity_uids};
pub use integrity::Fault;
pub use kentos_contracts as contracts;
pub use layers::{LayerTree, NewLayer, default_style};
pub use patch::{Patch, PatchRefused};
pub use step::Written;
pub use step::{BlockChange, IdChanges, ObjectChange, Sealed, StepReport};
pub use units::UnitState;
