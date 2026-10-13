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
//! Every `execute` runs through [`call::recorded`] (docs/adr/0253): one step
//! of the document, all or nothing (the parts outside the undo history too),
//! its integrity kept and a journal row left. [`call::call`] is the protocol
//! the headless host, Python, MCP and agents call by (expectations, request
//! ids, plans, summaries); [`registry`] finds a command by name; [`jobs`]
//! runs one on a reading copy.
//!
//! Pure Rust, native only: no UI, GPU, database, runtime or network
//! (scripts/arch/deps.mjs).
#![forbid(unsafe_code)]
// User input must never crash a command.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod address_define;
pub mod addresses;
pub mod annotation_follow;
pub mod arc;
pub mod array;
pub mod blocks;
pub mod blocks_define;
pub mod blocks_edit;
pub mod break_join;
pub mod call;
pub mod changeset;
mod checks;
pub mod circle;
pub mod codes;
mod context;
pub mod corner;
pub mod create;
mod creating;
pub mod delete;
mod dimension;
pub mod dimension_create;
pub mod dimensions_reassociate;
pub mod divide;
mod edge_commands;
pub mod edit;
pub mod elevation;
pub mod expression_unit;
pub mod geometry;
pub mod info_templates;
pub mod jobs;
pub mod label_texts;
pub mod labels_pin;
pub use break_join::{entities_break, entities_join};
pub use dimension_create::dimensions_create;
pub use edge_commands::{entities_chamfer, entities_extend, entities_fillet, entities_trim};
pub use region::{entities_boundary, entities_hatch};
pub mod entities_order;
pub mod file_import;
pub mod geometry_store;
pub mod import_apply;
pub mod layer_filter;
pub mod layers_compose;
pub mod layers_edit;
pub mod layers_filter;
pub mod layers_info;
pub mod layers_labels;
pub mod layers_renderer;
pub mod layers_service;
pub mod layers_set;
pub mod layers_states;
pub mod layers_time;
pub mod line;
pub mod network_define;
pub mod nurbs;
pub mod offset;
pub mod point;
pub mod polygon;
pub mod polyline;
pub mod region;
pub mod registry;
pub mod replay;
pub mod scenarios_edit;
pub mod set;
pub mod settings_set;
pub mod sheets_edit;
pub mod stretch;
pub mod style_tables;
pub mod styles_edit;
pub mod topology_follow;
pub mod transform;
pub mod trim;
pub mod ucs_set;
pub mod units_change;
pub mod widths;

pub use context::ExecutionContext;
pub use kentos_contracts::{CommandError, CommandResult, CommandWarning};

/// What a command's answer says: its refusal, or its warnings.
pub fn said_of<T>(result: CommandResult<T>) -> Vec<String> {
    match result {
        CommandResult::Completed { warnings, .. } => {
            warnings.into_iter().map(|w| w.message).collect()
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => vec![error.message],
        CommandResult::Queued { .. } | CommandResult::Cancelled => Vec::new(),
    }
}

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
    // The drawing's user coordinate system ([`ucs_set`], docs/adr/0261).
    (
        kentos_contracts::CAD_UCS_SET,
        kentos_contracts::CAD_UCS_SET_VERSION,
    ),
    // Öne getir and Arkaya gönder ([`entities_order`], docs/adr/0260 §5).
    (
        kentos_contracts::CAD_ENTITIES_ORDER,
        kentos_contracts::CAD_ENTITIES_ORDER_VERSION,
    ),
    // A NURBS curve by control points or through points ([`nurbs`], docs/adr/0259).
    (
        kentos_contracts::CAD_NURBS_CREATE,
        kentos_contracts::CAD_NURBS_CREATE_VERSION,
    ),
    // Ötele's semantic command ([`offset`], docs/adr/0246).
    (
        kentos_contracts::CAD_ENTITIES_OFFSET,
        kentos_contracts::CAD_ENTITIES_OFFSET_VERSION,
    ),
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
    // Adres verisi ([`address_define`], docs/adr/0229 §14).
    (
        kentos_contracts::CAD_ADDRESSES_DEFINE,
        kentos_contracts::CAD_ADDRESSES_DEFINE_VERSION,
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
    // Katman çizimi ([`layers_compose`], docs/adr/0226 §3, §4).
    (
        kentos_contracts::CAD_LAYERS_COMPOSE,
        kentos_contracts::CAD_LAYERS_COMPOSE_VERSION,
    ),
    // Bilgi kartı ve eylemler ([`layers_info`], docs/adr/0225 §4, §5).
    (
        kentos_contracts::CAD_LAYERS_INFO,
        kentos_contracts::CAD_LAYERS_INFO_VERSION,
    ),
    // Etiketler and the label tools ([`layers_labels`], [`labels_pin`], docs/adr/0212 §5).
    (
        kentos_contracts::CAD_LAYERS_LABELS,
        kentos_contracts::CAD_LAYERS_LABELS_VERSION,
    ),
    (
        kentos_contracts::CAD_LABELS_PIN,
        kentos_contracts::CAD_LABELS_PIN_VERSION,
    ),
    // Yeniden bağla ([`dimensions_reassociate`], docs/adr/0249 §3).
    (
        kentos_contracts::CAD_DIMENSIONS_REASSOCIATE,
        kentos_contracts::CAD_DIMENSIONS_REASSOCIATE_VERSION,
    ),
    // Ölçülendirme by what it measures ([`dimensions_create`], docs/adr/0255).
    (
        kentos_contracts::CAD_DIMENSIONS_CREATE,
        kentos_contracts::CAD_DIMENSIONS_CREATE_VERSION,
    ),
    // Buda, Uzat, Köşe yuvarla and Pah by their meaning (docs/adr/0250).
    (
        kentos_contracts::CAD_ENTITIES_TRIM,
        kentos_contracts::CAD_ENTITIES_TRIM_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_EXTEND,
        kentos_contracts::CAD_ENTITIES_EXTEND_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_FILLET,
        kentos_contracts::CAD_ENTITIES_FILLET_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_CHAMFER,
        kentos_contracts::CAD_ENTITIES_CHAMFER_VERSION,
    ),
    // Esnet, Kır, Birleştir, Böl, Alan oluştur and Tarama by their meaning (docs/adr/0254).
    (
        kentos_contracts::CAD_ENTITIES_STRETCH,
        kentos_contracts::CAD_ENTITIES_STRETCH_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_BREAK,
        kentos_contracts::CAD_ENTITIES_BREAK_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_JOIN,
        kentos_contracts::CAD_ENTITIES_JOIN_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_DIVIDE,
        kentos_contracts::CAD_ENTITIES_DIVIDE_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_BOUNDARY,
        kentos_contracts::CAD_ENTITIES_BOUNDARY_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_HATCH,
        kentos_contracts::CAD_ENTITIES_HATCH_VERSION,
    ),
    // Katman stili's renderers ([`layers_renderer`], docs/adr/0213 §5).
    (
        kentos_contracts::CAD_LAYERS_RENDERER,
        kentos_contracts::CAD_LAYERS_RENDERER_VERSION,
    ),
    // The layer tree, its own changes and its states, the text and dimension
    // styles, the project's settings ([`layers_edit`], [`layers_set`],
    // [`layers_states`], [`styles_edit`], [`settings_set`], docs/adr/0248).
    (
        kentos_contracts::CAD_LAYERS_EDIT,
        kentos_contracts::CAD_LAYERS_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_LAYERS_SET,
        kentos_contracts::CAD_LAYERS_SET_VERSION,
    ),
    (
        kentos_contracts::CAD_LAYERS_STATES,
        kentos_contracts::CAD_LAYERS_STATES_VERSION,
    ),
    (
        kentos_contracts::CAD_STYLES_EDIT,
        kentos_contracts::CAD_STYLES_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_SETTINGS_SET,
        kentos_contracts::CAD_SETTINGS_SET_VERSION,
    ),
    // A local project's unit, keeping the sizes or the numbers ([`units_change`], docs/adr/0256 §1).
    (
        kentos_contracts::CAD_UNITS_CHANGE,
        kentos_contracts::CAD_UNITS_CHANGE_VERSION,
    ),
    // A DXF into the drawing, its unit decided by one rule ([`file_import`], docs/adr/0257).
    (
        kentos_contracts::FILE_IMPORT_DXF,
        kentos_contracts::FILE_IMPORT_DXF_VERSION,
    ),
    // Pafta kitabı ([`sheets_edit`], docs/adr/0252 §2).
    (
        kentos_contracts::CAD_SHEETS_EDIT,
        kentos_contracts::CAD_SHEETS_EDIT_VERSION,
    ),
    // The change set: several commands as one change ([`changeset`], docs/adr/0253 §8).
    (
        kentos_contracts::CAD_CHANGESET_APPLY,
        kentos_contracts::CAD_CHANGESET_APPLY_VERSION,
    ),
];
