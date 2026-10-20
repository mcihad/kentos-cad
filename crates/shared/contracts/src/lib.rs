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
//! The `schema` feature (also on by default) derives JSON Schema for every
//! contract type; the product command catalog (`catalog`, docs/adr/0013) is
//! built from it.
//!
//! `settings` (docs/adr/0023) is the typed settings schema and the rules every
//! host validates, resolves and stores settings by.
//!
//! `project_catalog` (docs/adr/0028) is the cloud's project catalog: project
//! types, catalog metadata, the lifecycle commands and the per-person lists.
//!
//! Rules (docs/adr/0002-contracts-fixtures.md):
//! - every stored or sent document carries `format` and `version`; readers
//!   reject versions they do not know instead of guessing;
//! - geometry is float64 and lossless (arcs, bulges, holes are kept);
//! - the style engine's own types (renderers, library items) are opaque JSON
//!   in v1 and become typed when the style core moves to Rust.

pub mod address;
pub mod annotation;
pub mod annotation_scale;
pub mod annotative;
pub mod api;
pub mod assoc_array;
pub mod arch;
pub mod opening;
pub mod space;
pub mod arch_elements;
pub mod arch_tags;
pub mod constraint_expr;
pub mod constraints;
pub mod attachment;
pub mod block_dynamic;
pub mod blocks;
pub mod cad;
pub mod cad_array;
pub mod cad_blocks;
pub mod cad_edit;
pub mod line_types;
pub mod assets;
pub mod images;
pub mod raster_mosaic;
pub mod inserts_clip;
pub mod raster_georef;
pub mod xref;
pub mod cad_properties;
pub mod cad_ucs;
pub mod cad_views;
pub mod join_view;
pub mod contingents;
pub mod layer_rules;
pub mod domains;
pub mod form_schema;
pub mod subtypes;
pub mod table_styles;
pub mod canonical;
pub mod catalog;
pub mod change_effects;
pub mod checkpoints;
pub mod cloud;
pub mod codes;
pub mod command;
pub mod command_call;
pub mod crs;
pub mod heights;
pub mod document;
pub mod encoding;
pub mod entity;
pub mod fields;
pub mod forms;
pub mod file_export;
pub mod file_gpkg;
pub mod file_pdf;
pub mod file_package;
pub mod file_shp;
pub mod file_sld;
pub mod file_csv;
pub mod file_atlas;
pub mod file_gis;
pub mod sources;
pub mod cad_stac;
pub mod file_import;
pub mod formats;
pub mod hatch;
pub mod identity;
pub mod image;
pub mod invitations;
pub mod job;
pub mod labels;
pub mod layer;
pub mod layer_compose;
pub mod layer_filter;
pub mod layer_info;
pub mod look;
pub mod network;
pub mod numeric;
pub mod nurbs;
pub mod pointcloud;
pub mod project_catalog;
pub mod project_files;
pub mod raster;
pub mod service;
pub mod schedule;
pub mod viewmark;
pub mod settings;
pub mod sheets;
pub mod style;
pub mod subref;
pub mod table;
pub mod table_cells;
pub mod table_formula;
pub mod text_field;
pub mod temporal;
pub mod topology;
pub mod ui_catalog;
pub mod units;
pub mod variables;

pub use address::*;
pub use annotation::*;
pub use annotation_scale::*;
pub use annotative::*;
pub use api::*;
pub use assoc_array::*;
pub use arch::*;
pub use opening::*;
pub use space::*;
pub use arch_elements::*;
pub use arch_tags::*;
pub use constraint_expr::*;
pub use constraints::*;
pub use attachment::*;
pub use cad::*;
pub use cad_array::*;
pub use block_dynamic::*;
pub use cad_blocks::*;
pub use text_field::*;
pub use cad_edit::*;
pub use line_types::*;
pub use change_effects::{CHANGE_EFFECTS, CatalogEffect, ChangeEffect, EffectPart, EffectUndo};
pub use assets::*;
pub use images::*;
pub use raster_mosaic::*;
pub use inserts_clip::*;
pub use raster_georef::*;
pub use xref::*;
pub use cad_properties::*;
pub use cad_ucs::*;
pub use cad_views::*;
pub use join_view::*;
pub use contingents::*;
pub use layer_rules::*;
pub use domains::*;
pub use form_schema::*;
pub use subtypes::*;
pub use table_styles::*;
pub use catalog::*;
pub use checkpoints::*;
pub use cloud::*;
pub use command::*;
pub use command_call::*;
pub use crs::*;
pub use heights::*;
pub use document::*;
pub use entity::*;
pub use fields::*;
pub use forms::*;
pub use file_export::*;
pub use file_gpkg::*;
pub use file_pdf::*;
pub use file_package::*;
pub use file_shp::*;
pub use file_sld::*;
pub use file_csv::*;
pub use file_atlas::*;
pub use file_gis::*;
pub use sources::*;
pub use cad_stac::*;
pub use file_import::*;
pub use formats::*;
pub use hatch::*;
pub use identity::*;
pub use image::*;
pub use invitations::*;
pub use job::*;
pub use labels::*;
pub use layer::*;
pub use layer_compose::*;
pub use layer_filter::*;
pub use layer_info::*;
pub use look::*;
pub use network::*;
pub use numeric::*;
pub use nurbs::*;
pub use pointcloud::*;
pub use project_catalog::*;
pub use project_files::*;
pub use raster::*;
pub use service::*;
pub use settings::*;
pub use sheets::*;
pub use style::*;
pub use subref::*;
pub mod dimension_options;
pub use dimension_options::*;
pub use schedule::*;
pub use viewmark::*;
pub use table::*;
pub use table_cells::*;
pub use temporal::*;
pub use topology::*;
pub use ui_catalog::*;
pub use variables::*;

/// Version of this set of contracts, reported by the API's health endpoint.
pub const CONTRACTS_VERSION: u32 = 1;
