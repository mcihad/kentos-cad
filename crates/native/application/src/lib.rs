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
pub mod annotations_scales;
pub mod arc;
pub mod array;
mod array_assoc;
pub mod arrays_edit;
pub mod constraint_copy;
pub mod constraint_follow;
#[cfg(test)]
mod constraint_jobs_tests;
pub mod constraint_solve;
pub mod constraints_edit;
pub mod constraints_query;
pub mod parameters_edit;
pub mod audit_cmd;
pub mod blocks;
pub mod blocks_define;
pub mod blocks_edit;
pub mod blocks_insert;
pub mod block_params;
pub mod block_variants;
pub mod blocks_freeze;
pub mod blocks_proxy;
pub mod blocks_query;
pub mod blocks_replace;
pub mod blocks_sync;
pub mod file_import_block;
pub mod file_import_gpkg;
pub mod file_import_pdf;
pub mod provider;
pub mod fields_write;
pub mod processing_batch;
pub mod processing_host;
pub mod processing_run;
pub mod processing_tools;
pub mod source_places;
pub mod csv_table;
pub mod gis_derivation;
pub mod gis_derive;
pub mod file_import_csv;
pub mod shapefile;
pub mod sld;
pub mod sources;
pub mod geopackage;
pub mod blocks_library;
pub mod break_join;
pub mod call;
pub mod changeset;
mod checks;
pub mod circle;
pub mod cleanup_cmd;
pub mod clipboard_cmd;
pub mod codes;
mod context;
pub mod corner;
pub mod create;
mod creating;
pub mod curves_edit_cmd;
pub mod delete;
mod dimension;
pub mod dimension_create;
pub mod dimensions_edit;
pub mod dimensions_query;
pub mod fields_edit;
pub mod hatches_edit;
mod hatch_notes;
pub mod leaders_edit;
pub mod fonts_report;
pub mod texts_check;
pub mod table_schedule;
pub mod tables_edit;
pub mod texts_edit;
pub mod dimensions_reassociate;
pub mod divide;
mod edge_commands;
pub mod edit;
pub mod elevation;
pub mod exchange_write;
pub mod explode;
pub mod expression_unit;
pub mod geometry;
pub mod info_templates;
pub mod jobs;
pub mod label_texts;
pub mod labels_pin;
pub mod lengthen;
pub mod library_edit;
pub mod reverse;
pub mod styles_impact;
pub use audit_cmd::drawing_audit;
pub use break_join::{entities_break, entities_join};
pub use cleanup_cmd::entities_cleanup;
pub use clipboard_cmd::{entities_copy, entities_paste};
pub use curves_edit_cmd::curves_edit;
pub use dimension_create::dimensions_create;
pub use edge_commands::{entities_chamfer, entities_extend, entities_fillet, entities_trim};
pub use explode::entities_explode;
pub use lengthen::entities_lengthen;
pub use match_props::entities_match;
pub use region::{entities_boundary, entities_hatch};
pub use reverse::entities_reverse;
pub mod entities_order;
pub mod entities_pick;
pub mod entities_relate;
pub mod crs_transform;
pub mod entities_select;
pub mod entities_vertex;
pub mod shapes;
/// `cad.rectangle.create` as the registry runs it ([`shapes`]).
pub mod rectangle_create {
    pub use crate::shapes::{
        execute_rectangle as execute, plan_rectangle as plan, validate_rectangle as validate,
    };
}
/// `cad.polygon.regular` as the registry runs it ([`shapes`]).
pub mod polygon_regular {
    pub use crate::shapes::{
        execute_regular as execute, plan_regular as plan, validate_regular as validate,
    };
}
pub mod curves_construct;
pub mod file_import;
pub mod geometry_store;
pub mod import_apply;
pub mod layer_filter;
pub mod layers_compose;
pub mod layers_edit;
pub mod layers_filter;
pub mod field_policy;
pub mod attachments_inspect;
pub mod schema_describe;
pub mod fields_usage;
pub mod layers_find;
pub mod layers_info;
pub mod layers_contingents;
pub mod layers_rules;
pub mod layers_subtypes;
pub mod layers_table;
pub mod layers_labels;
pub mod layers_renderer;
pub mod layers_service;
pub mod layers_set;
pub mod layers_states;
pub mod layers_time;
pub mod layers_translate;
pub mod line;
pub mod hatch_patterns_edit;
pub mod line_types_edit;
pub mod match_props;
pub mod network_define;
pub mod nurbs;
pub mod walls;
pub mod walls_create;
pub(crate) mod openings;
pub mod openings_create;
pub mod openings_edit;
pub mod spaces;
pub mod spaces_create;
pub mod arch_elements;
pub mod arch_edit;
pub use arch_elements::{columns_create, grids_create, ramps_create, stairs_create};
pub mod tags;
pub mod schedules;
pub use tags::{check_tags as tags_check, create as tags_create, edit as tags_edit};
pub use schedules::{create as schedules_create, edit as schedules_edit};
pub mod viewmarks;
pub mod spaces_finish;
pub mod sheets_set;
pub mod pdf_underlay;
pub mod xrefs;
mod xrefs_bind;
pub mod links;
pub mod assets_check;
pub mod images_edit;
pub mod rasters_mosaic;
pub mod stac_host;
pub mod stac_prepare;
pub mod stac_search;
pub mod inserts_clip;
pub mod raster_files;
pub mod rasters_georef;
pub mod templates_apply;
pub use xrefs::{edit as xrefs_edit, list as xrefs_list};
pub use viewmarks::{create as viewmarks_create, edit as viewmarks_edit, references as viewmarks_references};
pub mod spaces_edit;
pub mod spaces_quantities;
pub mod walls_edit;
pub mod walls_joins;
pub mod walls_from_lines;
pub mod offset;
pub mod point;
pub mod points_construct;
pub mod polygon;
pub mod polyline;
pub mod python_call;
pub mod region;
pub mod registry;
pub mod replay;
pub mod scenarios_edit;
pub mod set;
pub mod settings_set;
pub mod sheets_edit;
pub mod snap_find;
pub mod standards;
pub mod stretch;
pub mod style_tables;
pub mod styles_edit;
pub mod topology_follow;
pub mod transform;
pub mod trim;
pub mod ucs_set;
pub mod units_change;
pub mod views_edit;
pub mod forms;
pub mod forms_audit;
pub mod records;
pub mod records_duplicate;
pub mod forms_compare;
pub mod forms_edit;
pub mod forms_fill;
pub mod forms_submit;
pub mod forms_recompute;
pub mod forms_test;
pub mod forms_transition;
pub mod domains_edit;
pub mod domains_export;
pub mod fields_migrate;
pub mod forms_export;
pub mod forms_import;
pub mod forms_validate;
pub mod joins_edit;
pub mod joins_query;
pub mod relations_edit;
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
    // The project's named views ([`views_edit`], docs/adr/0285).
    (
        kentos_contracts::CAD_VIEWS_EDIT,
        kentos_contracts::CAD_VIEWS_EDIT_VERSION,
    ),
    // The project's forms ([`forms_edit`], [`forms_validate`], [`forms_submit`], docs/adr/0401).
    (
        kentos_contracts::CAD_FORMS_EDIT,
        kentos_contracts::CAD_FORMS_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_FORMS_VALIDATE,
        kentos_contracts::CAD_FORMS_VALIDATE_VERSION,
    ),
    (
        kentos_contracts::CAD_FORMS_AUDIT,
        kentos_contracts::CAD_FORMS_AUDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_FORMS_TEST,
        kentos_contracts::CAD_FORMS_TEST_VERSION,
    ),
    (
        kentos_contracts::CAD_FORMS_COMPARE,
        kentos_contracts::CAD_FORMS_COMPARE_VERSION,
    ),
    (
        kentos_contracts::CAD_FORMS_TRANSITION,
        kentos_contracts::CAD_FORMS_TRANSITION_VERSION,
    ),
    (
        kentos_contracts::CAD_DOMAINS_EDIT,
        kentos_contracts::CAD_DOMAINS_EDIT_VERSION,
    ),
    // The value lists as a package and a layer's values in their canonical texts (docs/adr/0439 §7, §8).
    (
        kentos_contracts::CAD_DOMAINS_EXPORT,
        kentos_contracts::CAD_DOMAINS_EXPORT_VERSION,
    ),
    (
        kentos_contracts::CAD_FIELDS_MIGRATE,
        kentos_contracts::CAD_FIELDS_MIGRATE_VERSION,
    ),
    // What each attachment is, read from its bytes ([`attachments_inspect`], docs/adr/0442 §7).
    (
        kentos_contracts::CAD_ATTACHMENTS_INSPECT,
        kentos_contracts::CAD_ATTACHMENTS_INSPECT_VERSION,
    ),
    // The data schema a page at a time ([`schema_describe`], docs/adr/0443 §1).
    (
        kentos_contracts::CAD_SCHEMA_DESCRIBE,
        kentos_contracts::CAD_SCHEMA_DESCRIBE_VERSION,
    ),
    (
        kentos_contracts::CAD_FORMS_EXPORT,
        kentos_contracts::CAD_FORMS_EXPORT_VERSION,
    ),
    (
        kentos_contracts::CAD_FORMS_IMPORT,
        kentos_contracts::CAD_FORMS_IMPORT_VERSION,
    ),
    (
        kentos_contracts::CAD_FORMS_RECOMPUTE,
        kentos_contracts::CAD_FORMS_RECOMPUTE_VERSION,
    ),
    // Koşullu biçim ([`layers_table`], docs/adr/0413).
    (
        kentos_contracts::CAD_LAYERS_TABLE,
        kentos_contracts::CAD_LAYERS_TABLE_VERSION,
    ),
    (
        kentos_contracts::CAD_LAYERS_SUBTYPES,
        kentos_contracts::CAD_LAYERS_SUBTYPES_VERSION,
    ),
    (
        kentos_contracts::CAD_LAYERS_CONTINGENTS,
        kentos_contracts::CAD_LAYERS_CONTINGENTS_VERSION,
    ),
    (
        kentos_contracts::CAD_LAYERS_RULES,
        kentos_contracts::CAD_LAYERS_RULES_VERSION,
    ),
    // Records without a geometry ([`records`], docs/adr/0410).
    (
        kentos_contracts::CAD_RECORDS_CREATE,
        kentos_contracts::CAD_RECORDS_CREATE_VERSION,
    ),
    (
        kentos_contracts::CAD_RECORDS_DUPLICATE,
        kentos_contracts::CAD_RECORDS_DUPLICATE_VERSION,
    ),
    (
        kentos_contracts::CAD_FORMS_SUBMIT,
        kentos_contracts::CAD_FORMS_SUBMIT_VERSION,
    ),
    (
        kentos_contracts::CAD_FORMS_FILL,
        kentos_contracts::CAD_FORMS_FILL_VERSION,
    ),
    // The project's relations ([`relations_edit`], docs/adr/0405).
    (
        kentos_contracts::CAD_RELATIONS_EDIT,
        kentos_contracts::CAD_RELATIONS_EDIT_VERSION,
    ),
    // The project's join views kept and read ([`joins_edit`], [`joins_query`], docs/adr/0527).
    (
        kentos_contracts::CAD_JOINS_EDIT,
        kentos_contracts::CAD_JOINS_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_JOINS_QUERY,
        kentos_contracts::CAD_JOINS_QUERY_VERSION,
    ),
    // Taramanın sınırı, adaları ve ilişkisi ([`hatches_edit`], docs/adr/0329 §4).
    (
        kentos_contracts::CAD_HATCHES_EDIT,
        kentos_contracts::CAD_HATCHES_EDIT_VERSION,
    ),
    // Çok kollu kılavuz ([`leaders_edit`], docs/adr/0326 §5).
    (
        kentos_contracts::CAD_LEADERS_EDIT,
        kentos_contracts::CAD_LEADERS_EDIT_VERSION,
    ),
    // Tablonun hücreleri, formülleri, alanları ve dosyası ([`tables_edit`], docs/adr/0327).
    (
        kentos_contracts::CAD_TABLES_EDIT,
        kentos_contracts::CAD_TABLES_EDIT_VERSION,
    ),
    // Alanlar and the text commands ([`fields_edit`], [`texts_edit`], [`texts_check`], docs/adr/0325 §3).
    (
        kentos_contracts::CAD_FIELDS_EDIT,
        kentos_contracts::CAD_FIELDS_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_TEXTS_EDIT,
        kentos_contracts::CAD_TEXTS_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_TEXTS_CHECK,
        kentos_contracts::CAD_TEXTS_CHECK_VERSION,
    ),
    // Yazı tipleri ([`fonts_report`], docs/adr/0324 §5).
    (
        kentos_contracts::CAD_FONTS_REPORT,
        kentos_contracts::CAD_FONTS_REPORT_VERSION,
    ),
    // Ölçekli açıklamalar ([`annotations_scales`], docs/adr/0323 §4).
    (
        kentos_contracts::CAD_ANNOTATIONS_SCALES,
        kentos_contracts::CAD_ANNOTATIONS_SCALES_VERSION,
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
    // Aks, Kolon, Merdiven, Rampa and Mimari nesneyi düzenle ([`arch_elements`], [`arch_edit`], docs/adr/0343).
    (
        kentos_contracts::CAD_GRIDS_CREATE,
        kentos_contracts::CAD_GRIDS_CREATE_VERSION,
    ),
    (
        kentos_contracts::CAD_COLUMNS_CREATE,
        kentos_contracts::CAD_COLUMNS_CREATE_VERSION,
    ),
    (
        kentos_contracts::CAD_STAIRS_CREATE,
        kentos_contracts::CAD_STAIRS_CREATE_VERSION,
    ),
    (
        kentos_contracts::CAD_RAMPS_CREATE,
        kentos_contracts::CAD_RAMPS_CREATE_VERSION,
    ),
    (
        kentos_contracts::CAD_ARCH_EDIT,
        kentos_contracts::CAD_ARCH_EDIT_VERSION,
    ),
    // Etiket, Etiketleri denetle, Çizelge and Çizelgeyi düzenle ([`tags`], [`schedules`], docs/adr/0344).
    (
        kentos_contracts::CAD_TAGS_CREATE,
        kentos_contracts::CAD_TAGS_CREATE_VERSION,
    ),
    (
        kentos_contracts::CAD_TAGS_CHECK,
        kentos_contracts::CAD_TAGS_CHECK_VERSION,
    ),
    (
        kentos_contracts::CAD_TAGS_EDIT,
        kentos_contracts::CAD_TAGS_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_SCHEDULES_CREATE,
        kentos_contracts::CAD_SCHEDULES_CREATE_VERSION,
    ),
    (
        kentos_contracts::CAD_SCHEDULES_EDIT,
        kentos_contracts::CAD_SCHEDULES_EDIT_VERSION,
    ),
    // Görünüm işareti, Görünüm işaretini düzenle, Başvurular and Döşeme ve tavan ([`viewmarks`], [`spaces_finish`],
    // docs/adr/0345).
    (
        kentos_contracts::CAD_VIEWMARKS_CREATE,
        kentos_contracts::CAD_VIEWMARKS_CREATE_VERSION,
    ),
    (
        kentos_contracts::CAD_VIEWMARKS_EDIT,
        kentos_contracts::CAD_VIEWMARKS_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_VIEWMARKS_REFERENCES,
        kentos_contracts::CAD_VIEWMARKS_REFERENCES_VERSION,
    ),
    (
        kentos_contracts::CAD_SPACES_FINISH,
        kentos_contracts::CAD_SPACES_FINISH_VERSION,
    ),
    // Mimari şablonu ([`templates_apply`], docs/adr/0346).
    (
        kentos_contracts::CAD_TEMPLATES_APPLY,
        kentos_contracts::CAD_TEMPLATES_APPLY_VERSION,
    ),
    // Pafta seti ([`sheets_set`], docs/adr/0347).
    (
        kentos_contracts::CAD_SHEETS_SET,
        kentos_contracts::CAD_SHEETS_SET_VERSION,
    ),
    // Dış başvurular ([`xrefs`], docs/adr/0348).
    (
        kentos_contracts::CAD_XREFS_EDIT,
        kentos_contracts::CAD_XREFS_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_XREFS_LIST,
        kentos_contracts::CAD_XREFS_LIST_VERSION,
    ),
    // Kırp ([`inserts_clip`], docs/adr/0351 §1).
    (
        kentos_contracts::CAD_INSERTS_CLIP,
        kentos_contracts::CAD_INSERTS_CLIP_VERSION,
    ),
    // Bağlı dosyaların durumu ([`assets_check`], docs/adr/0349 §2).
    (
        kentos_contracts::CAD_ASSETS_CHECK,
        kentos_contracts::CAD_ASSETS_CHECK_VERSION,
    ),
    // Resmin kaynağı ([`images_edit`], docs/adr/0350 §1).
    (
        kentos_contracts::CAD_IMAGES_EDIT,
        kentos_contracts::CAD_IMAGES_EDIT_VERSION,
    ),
    // STAC keşfi ve veri hazırlama ([`stac_search`], [`stac_prepare`], docs/adr/0528).
    (
        kentos_contracts::CAD_STAC_SEARCH,
        kentos_contracts::CAD_STAC_SEARCH_VERSION,
    ),
    (
        kentos_contracts::CAD_STAC_PREPARE,
        kentos_contracts::CAD_STAC_PREPARE_VERSION,
    ),
    // Sanal mozaik ([`rasters_mosaic`], docs/adr/0517).
    (
        kentos_contracts::CAD_RASTERS_MOSAIC,
        kentos_contracts::CAD_RASTERS_MOSAIC_VERSION,
    ),
    // Raster oturt'un kaydı ([`rasters_georef`], docs/adr/0518).
    (
        kentos_contracts::CAD_RASTERS_GEOREF,
        kentos_contracts::CAD_RASTERS_GEOREF_VERSION,
    ),
    // Mahal, Mahali düzenle and Metraj ([`spaces_create`], [`spaces_edit`], [`spaces_quantities`], docs/adr/0342).
    (
        kentos_contracts::CAD_SPACES_CREATE,
        kentos_contracts::CAD_SPACES_CREATE_VERSION,
    ),
    (
        kentos_contracts::CAD_SPACES_EDIT,
        kentos_contracts::CAD_SPACES_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_SPACES_QUANTITIES,
        kentos_contracts::CAD_SPACES_QUANTITIES_VERSION,
    ),
    // Kapı, Pencere, Boşluk and Açıklığı düzenle ([`openings_create`], [`openings_edit`], docs/adr/0341).
    (
        kentos_contracts::CAD_OPENINGS_CREATE,
        kentos_contracts::CAD_OPENINGS_CREATE_VERSION,
    ),
    (
        kentos_contracts::CAD_OPENINGS_EDIT,
        kentos_contracts::CAD_OPENINGS_EDIT_VERSION,
    ),
    // Duvar, Çizgiden duvar, Duvarı düzenle and Duvar birleşimleri ([`walls_create`], [`walls_from_lines`], [`walls_edit`],
    // [`walls_joins`], docs/adr/0339, 0340).
    (
        kentos_contracts::CAD_WALLS_CREATE,
        kentos_contracts::CAD_WALLS_CREATE_VERSION,
    ),
    (
        kentos_contracts::CAD_WALLS_FROM_LINES,
        kentos_contracts::CAD_WALLS_FROM_LINES_VERSION,
    ),
    (
        kentos_contracts::CAD_WALLS_EDIT,
        kentos_contracts::CAD_WALLS_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_WALLS_JOINS,
        kentos_contracts::CAD_WALLS_JOINS_VERSION,
    ),
    // Rectangles and regular polygons ([`shapes`], docs/adr/0290).
    (
        kentos_contracts::CAD_RECTANGLE_CREATE,
        kentos_contracts::CAD_RECTANGLE_CREATE_VERSION,
    ),
    (
        kentos_contracts::CAD_POLYGON_REGULAR,
        kentos_contracts::CAD_POLYGON_REGULAR_VERSION,
    ),
    // A path's vertices and edges ([`entities_vertex`], docs/adr/0279).
    (
        kentos_contracts::CAD_ENTITIES_VERTEX,
        kentos_contracts::CAD_ENTITIES_VERTEX_VERSION,
    ),
    // What a click can mean and the sub-object it falls on ([`entities_pick`], docs/adr/0276).
    (
        kentos_contracts::CAD_ENTITIES_PICK,
        kentos_contracts::CAD_ENTITIES_PICK_VERSION,
    ),
    // The standard DE-9IM relations ([`entities_relate`], docs/adr/0501).
    (
        kentos_contracts::CAD_ENTITIES_RELATE,
        kentos_contracts::CAD_ENTITIES_RELATE_VERSION,
    ),
    // Points between coordinate systems with their heights, and how ([`crs_transform`], docs/adr/0519).
    (
        kentos_contracts::CAD_CRS_TRANSFORM,
        kentos_contracts::CAD_CRS_TRANSFORM_VERSION,
    ),
    // The selection as one explicit query ([`entities_select`], docs/adr/0275).
    (
        kentos_contracts::CAD_ENTITIES_SELECT,
        kentos_contracts::CAD_ENTITIES_SELECT_VERSION,
    ),
    // The object snap at a point, with its object's place ([`snap_find`], docs/adr/0272).
    (
        kentos_contracts::CAD_SNAP_FIND,
        kentos_contracts::CAD_SNAP_FIND_VERSION,
    ),
    // The drafting aids' points by what they are made of ([`points_construct`], docs/adr/0270).
    (
        kentos_contracts::CAD_POINTS_CONSTRUCT,
        kentos_contracts::CAD_POINTS_CONSTRUCT_VERSION,
    ),
    // Daire's, Yay's and Elips's methods by what they are made of ([`curves_construct`], docs/adr/0289).
    (
        kentos_contracts::CAD_CURVES_CONSTRUCT,
        kentos_contracts::CAD_CURVES_CONSTRUCT_VERSION,
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
    // A dimension's spacing, breaks, moved value, written value and options ([`dimensions_edit`], docs/adr/0321 §4).
    (
        kentos_contracts::CAD_DIMENSIONS_EDIT,
        kentos_contracts::CAD_DIMENSIONS_EDIT_VERSION,
    ),
    // What dimensions measure and show ([`dimensions_query`], docs/adr/0321 §3).
    (
        kentos_contracts::CAD_DIMENSIONS_QUERY,
        kentos_contracts::CAD_DIMENSIONS_QUERY_VERSION,
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
        kentos_contracts::CAD_ENTITIES_REVERSE,
        kentos_contracts::CAD_ENTITIES_REVERSE_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_LENGTHEN,
        kentos_contracts::CAD_ENTITIES_LENGTHEN_VERSION,
    ),
    (
        kentos_contracts::CAD_ARRAYS_EDIT,
        kentos_contracts::CAD_ARRAYS_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_CONSTRAINTS_EDIT,
        kentos_contracts::CAD_CONSTRAINTS_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_CONSTRAINTS_QUERY,
        kentos_contracts::CAD_CONSTRAINTS_QUERY_VERSION,
    ),
    (
        kentos_contracts::CAD_PARAMETERS_EDIT,
        kentos_contracts::CAD_PARAMETERS_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_EXPLODE,
        kentos_contracts::CAD_ENTITIES_EXPLODE_VERSION,
    ),
    (
        kentos_contracts::CAD_CURVES_EDIT,
        kentos_contracts::CAD_CURVES_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_MATCH,
        kentos_contracts::CAD_ENTITIES_MATCH_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_CLEANUP,
        kentos_contracts::CAD_ENTITIES_CLEANUP_VERSION,
    ),
    (
        kentos_contracts::CAD_DRAWING_AUDIT,
        kentos_contracts::CAD_DRAWING_AUDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_COPY,
        kentos_contracts::CAD_ENTITIES_COPY_VERSION,
    ),
    (
        kentos_contracts::CAD_ENTITIES_PASTE,
        kentos_contracts::CAD_ENTITIES_PASTE_VERSION,
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
    // Where a layer's field is named ([`fields_usage`], docs/adr/0414).
    (
        kentos_contracts::CAD_FIELDS_USAGE,
        kentos_contracts::CAD_FIELDS_USAGE_VERSION,
    ),
    // The layers found by a name pattern and their properties, and layers
    // translated to a standard's ([`layers_find`], [`layers_translate`], docs/adr/0311).
    (
        kentos_contracts::CAD_LAYERS_FIND,
        kentos_contracts::CAD_LAYERS_FIND_VERSION,
    ),
    (
        kentos_contracts::CAD_LAYERS_TRANSLATE,
        kentos_contracts::CAD_LAYERS_TRANSLATE_VERSION,
    ),
    // A drawing standard checked and applied ([`standards`], docs/adr/0312).
    (
        kentos_contracts::CAD_STANDARDS_CHECK,
        kentos_contracts::CAD_STANDARDS_CHECK_VERSION,
    ),
    (
        kentos_contracts::CAD_STANDARDS_APPLY,
        kentos_contracts::CAD_STANDARDS_APPLY_VERSION,
    ),
    // The project's line types with patterns ([`line_types_edit`], docs/adr/0313).
    (
        kentos_contracts::CAD_LINE_TYPES_EDIT,
        kentos_contracts::CAD_LINE_TYPES_EDIT_VERSION,
    ),
    // The project's own hatch patterns ([`hatch_patterns_edit`], docs/adr/0331 §1).
    (
        kentos_contracts::CAD_HATCH_PATTERNS_EDIT,
        kentos_contracts::CAD_HATCH_PATTERNS_EDIT_VERSION,
    ),
    // Blocks from a drawing file ([`file_import_block`], docs/adr/0316 §3).
    (
        kentos_contracts::FILE_IMPORT_BLOCK,
        kentos_contracts::FILE_IMPORT_BLOCK_VERSION,
    ),
    // A block placed ([`blocks_insert`], docs/adr/0315 §4).
    (
        kentos_contracts::CAD_BLOCKS_INSERT,
        kentos_contracts::CAD_BLOCKS_INSERT_VERSION,
    ),
    // A block's inserts placing another, and the inserts counted, found and scheduled ([`blocks_replace`],
    // [`blocks_query`], docs/adr/0317 §3).
    (
        kentos_contracts::CAD_BLOCKS_REPLACE,
        kentos_contracts::CAD_BLOCKS_REPLACE_VERSION,
    ),
    (
        kentos_contracts::CAD_BLOCKS_QUERY,
        kentos_contracts::CAD_BLOCKS_QUERY_VERSION,
    ),
    // A dynamic block's inserts' parameter values ([`block_params`], docs/adr/0318 §3).
    (
        kentos_contracts::CAD_BLOCKS_PARAMETERS,
        kentos_contracts::CAD_BLOCKS_PARAMETERS_VERSION,
    ),
    // What an insert draws, and a dynamic block's inserts made plain ([`block_variants`], docs/adr/0319 §4).
    (
        kentos_contracts::CAD_BLOCKS_PROXY,
        kentos_contracts::CAD_BLOCKS_PROXY_VERSION,
    ),
    (
        kentos_contracts::CAD_BLOCKS_FREEZE,
        kentos_contracts::CAD_BLOCKS_FREEZE_VERSION,
    ),
    // The project's style library and a change's reach ([`library_edit`], [`styles_impact`], docs/adr/0314).
    (
        kentos_contracts::CAD_LIBRARY_EDIT,
        kentos_contracts::CAD_LIBRARY_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_STYLES_IMPACT,
        kentos_contracts::CAD_STYLES_IMPACT_VERSION,
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
    // Toplu yazma of the attribute table ([`fields_write`], docs/adr/0507).
    (
        kentos_contracts::CAD_FIELDS_WRITE,
        kentos_contracts::CAD_FIELDS_WRITE_VERSION,
    ),
    (
        kentos_contracts::CAD_PROCESSING_TOOLS,
        kentos_contracts::CAD_PROCESSING_TOOLS_VERSION,
    ),
    (
        kentos_contracts::CAD_PROCESSING_RUN,
        kentos_contracts::CAD_PROCESSING_RUN_VERSION,
    ),
    // İşlemler's tool or model over a table of rows (docs/adr/0529).
    (
        kentos_contracts::CAD_PROCESSING_BATCH,
        kentos_contracts::CAD_PROCESSING_BATCH_VERSION,
    ),
    // Bound layers ([`sources`], docs/adr/0503).
    (
        kentos_contracts::CAD_SOURCES_DESCRIBE,
        kentos_contracts::CAD_SOURCES_DESCRIBE_VERSION,
    ),
    (
        kentos_contracts::CAD_SOURCES_EDIT,
        kentos_contracts::CAD_SOURCES_EDIT_VERSION,
    ),
    (
        kentos_contracts::CAD_SOURCES_SYNC,
        kentos_contracts::CAD_SOURCES_SYNC_VERSION,
    ),
    // A GeoPackage into the drawing ([`file_import_gpkg`], docs/adr/0502).
    (
        kentos_contracts::FILE_IMPORT_GPKG,
        kentos_contracts::FILE_IMPORT_GPKG_VERSION,
    ),
    // A CSV's rows into the drawing ([`file_import_csv`], docs/adr/0522).
    (
        kentos_contracts::FILE_IMPORT_CSV,
        kentos_contracts::FILE_IMPORT_CSV_VERSION,
    ),
    // A derived GIS dataset against the drawing ([`gis_derivation`], docs/adr/0530).
    (
        kentos_contracts::CAD_GIS_DERIVATION,
        kentos_contracts::CAD_GIS_DERIVATION_VERSION,
    ),
    // An SLD's look made a layer's ([`sld`], docs/adr/0516).
    (
        kentos_contracts::FILE_IMPORT_SLD,
        kentos_contracts::FILE_IMPORT_SLD_VERSION,
    ),
    // A PDF page into the drawing ([`file_import_pdf`], docs/adr/0352).
    (
        kentos_contracts::FILE_IMPORT_PDF,
        kentos_contracts::FILE_IMPORT_PDF_VERSION,
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
