//! KentOS native application: the desktop's product commands (docs/adr/0013,
//! 0022; TODOS.md CMD-04..07, §2.1 `native/application`), and later the
//! server's local ones.
//!
//! A product command is a typed, versioned operation from the catalog
//! (`kentos_contracts::catalog`). This crate runs the ones marked `desktop`
//! over the native document ([`kentos_domain::Document`]); the web runs its
//! own TypeScript handlers over `CadDocument` (apps/web/src/product). Neither
//! calls the other. What holds them together is shared data:
//!
//! - the contract types (input, output, plan, [`CommandResult`]);
//! - the cases in `fixtures/commands/v1`, which both run
//!   (`tests/fixtures.rs`, `apps/web/src/product/fixtures.test.ts`).
//!
//! Every command has the same flow (CMD-04): `validate` and `plan` write
//! nothing (not the document, its revision, dirty flag or history); `execute`
//! checks again, refuses with `conflict` when the input's expected revision is
//! not the document's, and writes one undo step through the document's own
//! edits. The answer is always a [`CommandResult`] (CMD-05). Whatever the
//! interface knows implicitly (the active layer, the current colour) is in the
//! input (CMD-07); a command never reads interface state.
//!
//! Pure Rust, native only: no UI, GPU, database, runtime or network
//! (scripts/arch/deps.mjs).
#![forbid(unsafe_code)]
// User input must never crash a command.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod arc;
mod checks;
pub mod circle;
pub mod codes;
mod context;
pub mod delete;
pub mod line;
pub mod point;
pub mod polygon;
pub mod polyline;

pub use context::ExecutionContext;
pub use kentos_contracts::{CommandError, CommandResult, CommandWarning};

use kentos_contracts::{
    CAD_ARC_CREATE, CAD_ARC_CREATE_VERSION, CAD_CIRCLE_CREATE, CAD_CIRCLE_CREATE_VERSION,
    CAD_ENTITIES_DELETE, CAD_ENTITIES_DELETE_VERSION, CAD_LINE_CREATE, CAD_LINE_CREATE_VERSION,
    CAD_POINT_CREATE, CAD_POINT_CREATE_VERSION, CAD_POLYGON_CREATE, CAD_POLYGON_CREATE_VERSION,
    CAD_POLYLINE_CREATE, CAD_POLYLINE_CREATE_VERSION,
};

/// The product commands the desktop runs, by name and version (docs/adr/0013).
/// Each has its module here ([`polygon`], [`line`], [`polyline`], [`delete`],
/// [`point`], [`circle`], [`arc`]); `tests/catalog.rs` keeps the list equal to
/// the catalog's commands marked `desktop`, as the server's `SERVER_COMMANDS`
/// is kept to those marked `server`.
pub const DESKTOP_COMMANDS: &[(&str, u32)] = &[
    (CAD_POLYGON_CREATE, CAD_POLYGON_CREATE_VERSION),
    (CAD_LINE_CREATE, CAD_LINE_CREATE_VERSION),
    (CAD_POLYLINE_CREATE, CAD_POLYLINE_CREATE_VERSION),
    (CAD_ENTITIES_DELETE, CAD_ENTITIES_DELETE_VERSION),
    (CAD_POINT_CREATE, CAD_POINT_CREATE_VERSION),
    (CAD_CIRCLE_CREATE, CAD_CIRCLE_CREATE_VERSION),
    (CAD_ARC_CREATE, CAD_ARC_CREATE_VERSION),
];
