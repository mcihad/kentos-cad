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
pub mod array;
pub mod blocks;
pub mod blocks_define;
pub mod blocks_edit;
mod checks;
pub mod circle;
pub mod codes;
mod context;
pub mod create;
pub mod delete;
mod dimension;
pub mod edit;
pub mod elevation;
pub mod geometry;
pub mod layer_filter;
pub mod layers_filter;
pub mod layers_service;
pub mod layers_time;
pub mod line;
pub mod network_define;
pub mod point;
pub mod polygon;
pub mod polyline;
pub mod scenarios_edit;
pub mod set;
pub mod transform;

pub use context::ExecutionContext;
pub use kentos_contracts::{CommandError, CommandResult, CommandWarning};

use kentos_contracts::{
    CAD_ARC_CREATE, CAD_ARC_CREATE_VERSION, CAD_CIRCLE_CREATE, CAD_CIRCLE_CREATE_VERSION,
    CAD_ENTITIES_ARRAY, CAD_ENTITIES_ARRAY_VERSION, CAD_ENTITIES_DELETE,
    CAD_ENTITIES_DELETE_VERSION, CAD_ENTITIES_EDIT, CAD_ENTITIES_EDIT_VERSION, CAD_ENTITIES_SET,
    CAD_ENTITIES_SET_VERSION, CAD_ENTITIES_TRANSFORM, CAD_ENTITIES_TRANSFORM_VERSION,
    CAD_LINE_CREATE, CAD_LINE_CREATE_VERSION, CAD_POINT_CREATE, CAD_POINT_CREATE_VERSION,
    CAD_POLYGON_CREATE, CAD_POLYGON_CREATE_VERSION, CAD_POLYLINE_CREATE,
    CAD_POLYLINE_CREATE_VERSION,
};

/// The product commands the desktop runs, by name and version (docs/adr/0013).
/// Each has its module here ([`polygon`], [`line`], [`polyline`], [`delete`],
/// [`point`], [`circle`], [`arc`], [`transform`], [`edit`], [`array`], [`create`], [`set`]); `tests/catalog.rs` keeps the
/// list equal to the catalog's commands marked `desktop`, as the server's
/// `SERVER_COMMANDS` is kept to those marked `server`.
pub const DESKTOP_COMMANDS: &[(&str, u32)] = &[
    (CAD_POLYGON_CREATE, CAD_POLYGON_CREATE_VERSION),
    (CAD_LINE_CREATE, CAD_LINE_CREATE_VERSION),
    (CAD_POLYLINE_CREATE, CAD_POLYLINE_CREATE_VERSION),
    (CAD_ENTITIES_DELETE, CAD_ENTITIES_DELETE_VERSION),
    (CAD_POINT_CREATE, CAD_POINT_CREATE_VERSION),
    (CAD_CIRCLE_CREATE, CAD_CIRCLE_CREATE_VERSION),
    (CAD_ARC_CREATE, CAD_ARC_CREATE_VERSION),
    (CAD_ENTITIES_TRANSFORM, CAD_ENTITIES_TRANSFORM_VERSION),
    (CAD_ENTITIES_EDIT, CAD_ENTITIES_EDIT_VERSION),
    (CAD_ENTITIES_ARRAY, CAD_ENTITIES_ARRAY_VERSION),
    // The drawing tools' objects without a command of their own ([`create`], docs/adr/0057).
    (
        kentos_contracts::CAD_ENTITIES_CREATE,
        kentos_contracts::CAD_ENTITIES_CREATE_VERSION,
    ),
    // Öznitelikler's layer, colour, symbol, attributes and label ([`set`], docs/adr/0066).
    (CAD_ENTITIES_SET, CAD_ENTITIES_SET_VERSION),
    // Blok oluştur and the Bloklar panel ([`blocks_define`], [`blocks_edit`], docs/adr/0144).
    (
        kentos_contracts::CAD_BLOCKS_DEFINE,
        kentos_contracts::CAD_BLOCKS_DEFINE_VERSION,
    ),
    (
        kentos_contracts::CAD_BLOCKS_EDIT,
        kentos_contracts::CAD_BLOCKS_EDIT_VERSION,
    ),
    // Harita servisi and the service layers' menus ([`layers_service`], docs/adr/0208 §15).
    (
        kentos_contracts::CAD_LAYERS_SERVICE,
        kentos_contracts::CAD_LAYERS_SERVICE_VERSION,
    ),
    // Ağlar ([`network_define`], docs/adr/0209 §11).
    (
        kentos_contracts::CAD_NETWORK_DEFINE,
        kentos_contracts::CAD_NETWORK_DEFINE_VERSION,
    ),
    // Zaman ayarları and the scenarios ([`layers_time`], [`scenarios_edit`], docs/adr/0210 §11).
    (
        kentos_contracts::CAD_LAYERS_TIME,
        kentos_contracts::CAD_LAYERS_TIME_VERSION,
    ),
    (
        kentos_contracts::CAD_SCENARIOS_EDIT,
        kentos_contracts::CAD_SCENARIOS_EDIT_VERSION,
    ),
    // Katman süzgeci ([`layers_filter`], docs/adr/0211 §5).
    (
        kentos_contracts::CAD_LAYERS_FILTER,
        kentos_contracts::CAD_LAYERS_FILTER_VERSION,
    ),
];
