//! Versioned KentOS contracts (CLAUDE.md §14): the shapes that cross a
//! boundary between the browser, WASM, the API and stored files. They are
//! defined once, here; `cargo test -p kentos-contracts` writes the matching
//! TypeScript types to `apps/web/src/contracts/generated/` (ts-rs), and the app's
//! internal types are checked against them at compile time
//! (`apps/web/src/contracts/contracts.test.ts`).
//!
//! The TypeScript generation is the `ts` feature (on by default, so
//! `cargo test` keeps the generated types current). Libraries and programs
//! that only need the Rust types (formats, the server, the desktop app)
//! depend on this crate with `default-features = false` and never build ts-rs.
//!
//! Rules (docs/adr/0002-contracts-fixtures.md):
//! - every stored or sent document carries `format` and `version`; readers
//!   reject versions they do not know instead of guessing;
//! - geometry is float64 and lossless (arcs, bulges, holes are kept);
//! - the style engine's own types (renderers, library items) are opaque JSON
//!   in v1 and become typed when the style core moves to Rust.

pub mod api;
pub mod cloud;
pub mod command;
pub mod document;
pub mod entity;
pub mod formats;
pub mod job;
pub mod layer;
pub mod numeric;
pub mod style;

pub use api::*;
pub use cloud::*;
pub use command::*;
pub use document::*;
pub use entity::*;
pub use formats::*;
pub use job::*;
pub use layer::*;
pub use numeric::*;
pub use style::*;

/// Version of this set of contracts, reported by the API's health endpoint.
pub const CONTRACTS_VERSION: u32 = 1;
