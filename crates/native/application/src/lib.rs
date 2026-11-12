//! KentOS native application: the desktop's product commands (docs/adr/0013,
//! 0022; TODOS.md CMD-04..07, §2.1 `native/application`), and later the
//! server's local ones.
//!
//! A product command is a typed, versioned operation from the catalog
//! (`kentos_commands::catalog`). This crate runs the ones marked `desktop`
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

#[path = "gis/address_define.rs"]

pub mod address_define;
#[path = "gis/addresses.rs"]
pub mod addresses;
#[path = "gis/addresses_check.rs"]
pub mod addresses_check;
#[path = "gis/addresses_compare.rs"]
pub mod addresses_compare;
#[path = "gis/addresses_normalize.rs"]
pub mod addresses_normalize;
#[path = "core/annotation_follow.rs"]
pub mod annotation_follow;
#[path = "core/annotations_scales.rs"]
pub mod annotations_scales;
#[path = "core/arc.rs"]
pub mod arc;
#[path = "core/array.rs"]
pub mod array;
#[path = "core/array_assoc.rs"]
mod array_assoc;
#[path = "core/arrays_edit.rs"]
pub mod arrays_edit;
#[path = "core/arrays_adopt.rs"]
pub mod arrays_adopt;
#[path = "core/arrays_find/mod.rs"]
pub mod arrays_find;
#[path = "core/drawing_inspect/mod.rs"]
pub mod drawing_inspect;
#[path = "core/constraint_copy.rs"]
pub mod constraint_copy;
#[path = "core/dimension_set_copy.rs"]
pub(crate) mod dimension_set_copy;
#[path = "core/constraint_follow.rs"]
pub mod constraint_follow;
#[cfg(test)]
#[path = "core/constraint_jobs_tests.rs"]
mod constraint_jobs_tests;
#[cfg(test)]
#[path = "core/told_tests.rs"]
mod told_tests;
#[path = "core/constraint_solve/mod.rs"]
pub mod constraint_solve;
#[path = "core/constraints_edit.rs"]
pub mod constraints_edit;
#[path = "core/constraints_query.rs"]
pub mod constraints_query;
#[path = "core/parameters_edit.rs"]
pub mod parameters_edit;
#[path = "core/audit_cmd.rs"]
pub mod audit_cmd;
#[path = "core/drawing_compare/mod.rs"]
pub mod drawing_compare;
#[path = "core/drawing_repair.rs"]
pub mod drawing_repair;
#[path = "core/drawing_take.rs"]
pub mod drawing_take;
#[path = "core/blocks.rs"]
pub mod blocks;
#[path = "core/blocks_define.rs"]
pub mod blocks_define;
#[path = "core/blocks_edit.rs"]
pub mod blocks_edit;
#[path = "core/blocks_insert.rs"]
pub mod blocks_insert;
#[path = "core/block_params.rs"]
pub mod block_params;
#[path = "core/block_variants.rs"]
pub mod block_variants;
#[path = "core/blocks_freeze.rs"]
pub mod blocks_freeze;
#[path = "core/blocks_proxy.rs"]
pub mod blocks_proxy;
#[path = "core/blocks_query.rs"]
pub mod blocks_query;
#[path = "core/blocks_find/mod.rs"]
pub mod blocks_find;
#[path = "core/blocks_convert/mod.rs"]
pub mod blocks_convert;
#[path = "core/blocks_replace/mod.rs"]
pub mod blocks_replace;
#[path = "core/blocks_sync.rs"]
pub mod blocks_sync;
#[path = "core/file_import_block.rs"]
pub mod file_import_block;
#[path = "core/file_import_gpkg.rs"]
pub mod file_import_gpkg;
#[path = "core/file_place.rs"]
pub mod file_place;
#[path = "core/file_import_pdf.rs"]
pub mod file_import_pdf;
#[path = "core/provider.rs"]
pub mod provider;
#[path = "gis/fields_write.rs"]
pub mod fields_write;
#[path = "core/file_digest.rs"]
pub mod file_digest;
#[path = "gis/processing_batch.rs"]
pub mod processing_batch;
#[path = "gis/processing_host.rs"]
pub mod processing_host;
#[path = "gis/processing_run.rs"]
pub mod processing_run;
#[path = "gis/processing_tools.rs"]
pub mod processing_tools;
#[path = "gis/source_places.rs"]
pub mod source_places;
#[path = "gis/csv_table.rs"]
pub mod csv_table;
#[path = "gis/fgb_table.rs"]
pub mod fgb_table;
#[path = "gis/gml_read.rs"]
pub mod gml_read;
#[path = "gis/gml_table.rs"]
pub mod gml_table;
#[path = "gis/gml_write.rs"]
pub mod gml_write;
#[path = "gis/xyz_table.rs"]
pub mod xyz_table;
#[path = "gis/gis_derivation.rs"]
pub mod gis_derivation;
#[path = "gis/gis_derive.rs"]
pub mod gis_derive;
#[path = "gis/gis_detach.rs"]
pub mod gis_detach;
#[path = "gis/gis_set.rs"]
pub mod gis_set;
#[path = "gis/gis_takeback.rs"]
pub mod gis_takeback;
#[path = "core/file_import_csv.rs"]
pub mod file_import_csv;
#[path = "core/file_import_fgb.rs"]
pub mod file_import_fgb;
#[path = "core/file_import_parquet.rs"]
pub mod file_import_parquet;
#[path = "core/file_import_geojson.rs"]
pub mod file_import_geojson;
#[path = "core/file_import_gis.rs"]
pub mod file_import_gis;
#[path = "core/file_import_gml.rs"]
pub mod file_import_gml;
#[path = "core/file_import_kml.rs"]
pub mod file_import_kml;
#[path = "core/file_import_shp.rs"]
pub mod file_import_shp;
#[path = "gis/shapefile.rs"]
pub mod shapefile;
#[path = "gis/maplibre.rs"]
pub mod maplibre;
#[path = "gis/qml.rs"]
pub mod qml;
#[path = "gis/sld.rs"]
pub mod sld;
#[path = "gis/sources/mod.rs"]
pub mod sources;
#[path = "gis/geopackage.rs"]
pub mod geopackage;
#[path = "core/import_merge.rs"]
pub mod import_merge;
#[path = "core/blocks_library.rs"]
pub mod blocks_library;
#[path = "core/break_join.rs"]
pub mod break_join;
#[path = "core/call.rs"]
pub mod call;
#[path = "core/changeset.rs"]
pub mod changeset;
#[path = "core/checks.rs"]
mod checks;
#[path = "core/circle.rs"]
pub mod circle;
#[path = "core/cleanup_cmd.rs"]
pub mod cleanup_cmd;
#[path = "core/clipboard_cmd.rs"]
pub mod clipboard_cmd;
#[path = "core/codes.rs"]
pub mod codes;
#[path = "core/context.rs"]
mod context;
#[path = "core/corner.rs"]
pub mod corner;
#[path = "core/create.rs"]
pub mod create;
#[path = "core/creating.rs"]
mod creating;
#[path = "core/curves_edit_cmd.rs"]
pub mod curves_edit_cmd;
#[path = "core/delete.rs"]
pub mod delete;
#[path = "core/dimension.rs"]
mod dimension;
#[path = "core/dimension_create.rs"]
pub mod dimension_create;
// İç ölçü zinciri (docs/adr/0711).
#[path = "core/dimension_interior.rs"]
pub mod dimension_interior;
#[path = "core/dimensions_edit.rs"]
pub mod dimensions_edit;
#[path = "core/dimensions_query.rs"]
pub mod dimensions_query;
#[path = "gis/fields_edit.rs"]
pub mod fields_edit;
#[path = "core/hatches_edit.rs"]
pub mod hatches_edit;
#[path = "core/hatch_notes.rs"]
mod hatch_notes;
#[path = "core/leaders_edit.rs"]
pub mod leaders_edit;
#[path = "core/fonts_report.rs"]
pub mod fonts_report;
#[path = "core/texts_check.rs"]
pub mod texts_check;
#[path = "arch/table_schedule.rs"]
pub mod table_schedule;
#[path = "core/tables_edit.rs"]
pub mod tables_edit;
/// Veri çıkarımı (docs/adr/1030).
#[path = "core/extract.rs"]
pub mod extract;
pub use extract::{run as extract_run, table as extract_table};
#[path = "core/texts_edit.rs"]
pub mod texts_edit;
#[path = "core/dimensions_reassociate.rs"]
pub mod dimensions_reassociate;
#[path = "core/dimension_sets.rs"]
pub mod dimension_sets;
pub use dimension_sets::{create as dimension_sets_create, edit as dimension_sets_edit, query as dimension_sets_query};
#[path = "core/divide.rs"]
pub mod divide;
#[path = "arch/edge_commands.rs"]
mod edge_commands;
#[path = "core/edit.rs"]
pub mod edit;
#[path = "arch/elevation.rs"]
pub mod elevation;
#[path = "core/exchange_write.rs"]
pub mod exchange_write;
#[path = "core/explode.rs"]
pub mod explode;
#[path = "core/expression_unit.rs"]
pub mod expression_unit;
#[path = "core/geometry.rs"]
pub mod geometry;
#[path = "arch/info_templates.rs"]
pub mod info_templates;
#[path = "core/jobs.rs"]
pub mod jobs;
#[path = "core/label_texts.rs"]
pub mod label_texts;
#[path = "core/labels_pin.rs"]
pub mod labels_pin;
#[path = "core/labels_report.rs"]
pub mod labels_report;
#[path = "core/lengthen.rs"]
pub mod lengthen;
#[path = "core/library_edit.rs"]
pub mod library_edit;
#[path = "core/reverse.rs"]
pub mod reverse;
#[path = "core/styles_impact.rs"]
pub mod styles_impact;
pub use audit_cmd::drawing_audit;
pub use break_join::{entities_break, entities_join};
pub use cleanup_cmd::entities_cleanup;
pub use clipboard_cmd::{entities_copy, entities_paste};
pub use curves_edit_cmd::curves_edit;
pub use dimension_create::dimensions_create;
pub use dimension_interior::dimensions_interior;
pub use edge_commands::{entities_chamfer, entities_extend, entities_fillet, entities_trim};
pub use explode::entities_explode;
pub use lengthen::entities_lengthen;
pub use match_props::entities_match;
pub use region::{entities_boundary, entities_hatch};
pub use reverse::entities_reverse;
#[path = "core/entities_order.rs"]
pub mod entities_order;
#[path = "core/entities_flatten.rs"]
pub mod entities_flatten;
#[path = "core/entities_pick.rs"]
pub mod entities_pick;
#[path = "core/entities_relate.rs"]
pub mod entities_relate;
#[path = "core/crs_models.rs"]
pub mod crs_models;
/// The grids a project names found beside its file (docs/adr/3580).
#[path = "map/project_grids.rs"]
pub mod project_grids;
#[path = "core/crs_transform.rs"]
pub mod crs_transform;
#[path = "core/entities_select/mod.rs"]
pub mod entities_select;
#[path = "core/entities_vertex.rs"]
pub mod entities_vertex;
#[path = "core/shapes.rs"]
pub mod shapes;
#[path = "arch/area_rules.rs"]
mod area_rules;
#[path = "core/adjoin_cmd.rs"]
pub mod adjoin_cmd;
#[path = "map/parcels.rs"]
pub mod parcels;
#[path = "gis/parquet_table.rs"]
pub mod parquet_table;
#[path = "gis/parquet_write.rs"]
pub mod parquet_write;
#[path = "core/annotations_write.rs"]
pub mod annotations_write;
#[path = "core/areas.rs"]
pub mod areas;
#[path = "core/objects_derive.rs"]
pub mod objects_derive;
#[path = "gis/images_clip.rs"]
pub mod images_clip;
#[path = "gis/time_version.rs"]
pub mod time_version;
#[path = "road/roads_edit.rs"]
pub mod roads_edit;
#[path = "map/survey_adjust.rs"]
pub mod survey_adjust;
#[path = "map/survey_points.rs"]
pub mod survey_points;
#[path = "map/survey_heights.rs"]
pub mod survey_heights;
#[path = "arch/box_grid.rs"]
pub mod box_grid;
#[path = "gis/network_ellipses.rs"]
pub mod network_ellipses;
#[path = "core/shape_edits.rs"]
pub mod shape_edits;
#[path = "core/shx_text.rs"]
pub mod shx_text;
#[path = "core/shape_split.rs"]
mod shape_split;
#[path = "core/text_curves.rs"]
pub mod text_curves;
#[path = "core/annotation_labels.rs"]
mod annotation_labels;
#[path = "core/constructions.rs"]
pub mod constructions;
#[path = "core/donut_create.rs"]
pub mod donut_create;
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
/// `cad.sector.create` as the registry runs it ([`shapes`]).
pub mod sector_create {
    pub use crate::shapes::{
        execute_sector as execute, plan_sector as plan, validate_sector as validate,
    };
}
/// `cad.revcloud.create` as the registry runs it ([`shapes`]).
pub mod revcloud_create {
    pub use crate::shapes::{
        execute_revcloud as execute, plan_revcloud as plan, validate_revcloud as validate,
    };
}
#[path = "core/curves_construct.rs"]
pub mod curves_construct;
#[path = "core/cad_layouts.rs"]
pub mod cad_layouts;
#[path = "core/cad_paper.rs"]
mod cad_paper;
#[path = "core/cad_source.rs"]
mod cad_source;
#[path = "core/file_import.rs"]
pub mod file_import;
#[path = "core/file_import_dwg.rs"]
pub mod file_import_dwg;
#[path = "core/geometry_store.rs"]
pub mod geometry_store;
#[path = "map/grid_refs.rs"]
pub(crate) mod grid_refs;
#[path = "core/viewport_scope.rs"]
pub mod viewport_scope;
#[path = "core/import_apply.rs"]
pub mod import_apply;
#[path = "gis/layer_filter.rs"]
pub mod layer_filter;
#[path = "gis/layers_compose.rs"]
pub mod layers_compose;
#[path = "gis/layers_edit.rs"]
pub mod layers_edit;
#[path = "gis/layers_filter.rs"]
pub mod layers_filter;
#[path = "core/field_policy.rs"]
pub mod field_policy;
#[path = "gis/attachments_inspect.rs"]
pub mod attachments_inspect;
#[path = "gis/schema_describe.rs"]
pub mod schema_describe;
#[path = "gis/fields_usage.rs"]
pub mod fields_usage;
#[path = "gis/layers_find.rs"]
pub mod layers_find;
#[path = "gis/layers_info.rs"]
pub mod layers_info;
#[path = "gis/layers_contingents.rs"]
pub mod layers_contingents;
#[path = "gis/layers_indexes.rs"]
pub mod layers_indexes;
#[path = "gis/layers_rules.rs"]
pub mod layers_rules;
#[path = "gis/layers_subtypes.rs"]
pub mod layers_subtypes;
#[path = "gis/layers_table.rs"]
pub mod layers_table;
#[path = "gis/layers_labels.rs"]
pub mod layers_labels;
#[path = "gis/layers_renderer.rs"]
pub mod layers_renderer;
#[path = "gis/layers_service.rs"]
pub mod layers_service;
#[path = "gis/layers_set.rs"]
pub mod layers_set;
#[path = "gis/layers_states.rs"]
pub mod layers_states;
#[path = "gis/layers_time.rs"]
pub mod layers_time;
#[path = "gis/layers_translate.rs"]
pub mod layers_translate;
#[path = "core/line.rs"]
pub mod line;
#[path = "core/hatch_patterns_edit.rs"]
pub mod hatch_patterns_edit;
#[path = "core/line_types_edit.rs"]
pub mod line_types_edit;
#[path = "core/match_props.rs"]
pub mod match_props;
#[path = "gis/network_define.rs"]
pub mod network_define;
#[path = "gis/event_follow.rs"]
pub mod event_follow;
#[path = "gis/linearref_define.rs"]
pub mod linearref_define;
#[path = "core/file_import_landxml.rs"]
pub mod file_import_landxml;
#[path = "road/road_terrains.rs"]
pub mod road_terrains;
#[path = "road/road_alignments.rs"]
pub mod road_alignments;
#[path = "road/road_alignments_create.rs"]
pub mod road_alignments_create;
#[path = "road/road_alignments_edit.rs"]
pub mod road_alignments_edit;
#[path = "road/road_alignments_label.rs"]
pub mod road_alignments_label;
#[path = "road/road_alignments_query.rs"]
pub mod road_alignments_query;
#[path = "road/road_plan_axes.rs"]
pub mod road_plan_axes;
#[path = "road/road_profiles.rs"]
pub mod road_profiles;
#[path = "road/road_profiles_create.rs"]
pub mod road_profiles_create;
#[path = "road/road_profiles_draw.rs"]
pub mod road_profiles_draw;
#[path = "road/road_profiles_edit.rs"]
pub mod road_profiles_edit;
#[path = "road/road_profiles_query.rs"]
pub mod road_profiles_query;
#[path = "road/road_profiles_sight.rs"]
pub mod road_profiles_sight;
#[path = "road/road_profiles_propose.rs"]
pub mod road_profiles_propose;
#[path = "road/road_alignments_check.rs"]
pub mod road_alignments_check;
#[path = "road/road_profiles_check.rs"]
pub mod road_profiles_check;
#[path = "road/road_profiles_lengths.rs"]
pub mod road_profiles_lengths;
#[path = "road/road_corridor_bands.rs"]
pub mod road_corridor_bands;
#[path = "road/road_earthwork_bands.rs"]
pub mod road_earthwork_bands;
#[path = "road/road_profile_splits.rs"]
pub mod road_profile_splits;
#[path = "road/road_corridors.rs"]
pub mod road_corridors;
#[path = "road/road_corridors_daylights.rs"]
pub mod road_corridors_daylights;
#[path = "road/road_corridors_preflight.rs"]
pub mod road_corridors_preflight;
#[path = "road/road_corridors_protected.rs"]
pub mod road_corridors_protected;
#[path = "road/road_corridors_water.rs"]
pub mod road_corridors_water;
#[path = "road/road_drawings.rs"]
pub(crate) mod road_drawings;
#[path = "road/road_project.rs"]
pub(crate) mod road_project;
pub use road_project::outdated_drawings;
#[path = "road/road_project_graph.rs"]
pub mod road_project_graph;
#[path = "road/road_project_produce.rs"]
pub mod road_project_produce;
#[path = "road/road_recipe_fields.rs"]
pub(crate) mod road_recipe_fields;
#[path = "road/road_project_query.rs"]
pub mod road_project_query;
#[path = "road/road_project_rebuild.rs"]
pub mod road_project_rebuild;
#[path = "road/road_project_regional.rs"]
pub(crate) mod road_project_regional;
#[path = "road/road_project_edit.rs"]
pub mod road_project_edit;
#[path = "map/map_work.rs"]
pub mod map_work;
#[path = "map/crs_factors.rs"]
pub mod crs_factors;
#[path = "map/map_features.rs"]
pub mod map_features;
#[path = "map/survey_baselines.rs"]
pub mod survey_baselines;
#[path = "core/entities_grade.rs"]
pub mod entities_grade;
#[path = "core/entities_raise.rs"]
pub mod entities_raise;
#[path = "core/entities_drape.rs"]
pub mod entities_drape;
#[path = "gis/topology_clean.rs"]
pub mod topology_clean;
#[path = "map/parcel_drafts.rs"]
pub mod parcel_drafts;
#[path = "map/parcel_register.rs"]
pub mod parcel_register;
#[path = "map/parcels_boundaries.rs"]
pub mod parcels_boundaries;
#[path = "map/parcels_check.rs"]
pub mod parcels_check;
#[path = "map/pipes_check.rs"]
pub mod pipes_check;
#[path = "map/pipes_clearance.rs"]
mod pipes_clearance;
#[path = "map/pipes_profile.rs"]
pub mod pipes_profile;
#[path = "map/pipes_rules.rs"]
mod pipes_rules;
#[path = "map/asbuilt_check.rs"]
pub mod asbuilt_check;
#[path = "map/asbuilt_review.rs"]
pub mod asbuilt_review;
#[path = "map/asbuilt_series.rs"]
pub(crate) mod asbuilt_series;
#[path = "map/asbuilt_package.rs"]
mod asbuilt_package;
#[path = "map/field_package.rs"]
pub mod field_package;
#[path = "map/zoning_check.rs"]
pub mod zoning_check;
#[path = "map/zoning_symbolize.rs"]
pub mod zoning_symbolize;
#[path = "map/zoning_draft.rs"]
pub mod zoning_draft;
#[path = "gis/scenarios_impact.rs"]
pub mod scenarios_impact;
#[path = "gis/scenarios_impact_network.rs"]
mod scenarios_impact_network;
#[path = "gis/scenarios_record.rs"]
pub mod scenarios_record;
#[path = "map/readjustment.rs"]
pub mod readjustment;
#[path = "map/readjustment_scope.rs"]
pub mod readjustment_scope;
#[path = "map/readjustment_sheets.rs"]
pub mod readjustment_sheets;
#[path = "map/separation_number_clear.rs"]
pub mod separation_number_clear;
#[path = "map/orthophoto_check.rs"]
pub mod orthophoto_check;
#[path = "map/orthophoto_compare.rs"]
pub mod orthophoto_compare;
#[path = "map/orthophoto_seamlines.rs"]
pub mod orthophoto_seamlines;
#[path = "map/orthophoto_network.rs"]
pub mod orthophoto_network;
#[path = "map/orthophoto_mosaic.rs"]
mod orthophoto_mosaic;
#[path = "map/orthophoto_seams.rs"]
mod orthophoto_seams;
#[path = "map/pointcloud_overlap.rs"]
pub mod pointcloud_overlap;
#[path = "map/pointcloud_network.rs"]
pub mod pointcloud_network;
#[path = "map/pointcloud_place.rs"]
pub mod pointcloud_place;
#[path = "core/profiles_compare.rs"]
pub mod profiles_compare;
#[path = "core/profiles_carry.rs"]
pub mod profiles_carry;
#[path = "core/profiles_names.rs"]
pub mod profiles_names;
#[path = "map/pointcloud_register.rs"]
pub mod pointcloud_register;
#[path = "map/map_work_edit.rs"]
pub mod map_work_edit;
#[path = "map/map_work_profile.rs"]
pub mod map_work_profile;
#[path = "map/map_work_restore.rs"]
pub mod map_work_restore;
#[path = "map/map_survey.rs"]
pub mod map_survey;
#[path = "map/map_survey_edit.rs"]
pub mod map_survey_edit;
#[path = "map/map_survey_network.rs"]
pub mod map_survey_network;
#[path = "map/map_survey_places.rs"]
pub mod map_survey_places;
#[path = "map/map_survey_sessions.rs"]
pub(crate) mod map_survey_sessions;
#[path = "map/map_features_reconcile.rs"]
pub mod map_features_reconcile;
#[path = "road/road_corridors_draw.rs"]
pub mod road_corridors_draw;
#[path = "road/road_corridors_edit.rs"]
pub mod road_corridors_edit;
#[path = "road/road_corridors_query.rs"]
pub mod road_corridors_query;
#[path = "road/road_hauls.rs"]
pub mod road_hauls;
#[path = "road/road_hauls_edit.rs"]
pub mod road_hauls_edit;
#[path = "road/road_hauls_query.rs"]
pub mod road_hauls_query;
#[path = "road/road_hauls_approve.rs"]
pub mod road_hauls_approve;
#[path = "road/road_hauls_network.rs"]
pub mod road_hauls_network;
#[path = "road/road_hauls_plan.rs"]
pub mod road_hauls_plan;
#[path = "road/road_hauls_trucks.rs"]
pub mod road_hauls_trucks;
#[path = "road/road_guardrails.rs"]
pub mod road_guardrails;
#[path = "road/road_guardrails_draw.rs"]
pub mod road_guardrails_draw;
#[path = "road/road_guardrails_edit.rs"]
pub mod road_guardrails_edit;
#[path = "road/road_guardrails_query.rs"]
pub mod road_guardrails_query;
#[path = "road/road_culverts.rs"]
pub mod road_culverts;
#[path = "road/road_culverts_draw.rs"]
pub mod road_culverts_draw;
#[path = "road/road_culverts_closure.rs"]
pub mod road_culverts_closure;
#[path = "road/road_culverts_hydraulics.rs"]
pub mod road_culverts_hydraulics;
#[path = "road/road_culverts_edit.rs"]
pub mod road_culverts_edit;
#[path = "road/road_culverts_query.rs"]
pub mod road_culverts_query;
#[path = "road/road_junctions.rs"]
pub mod road_junctions;
#[path = "road/road_junctions_clearance.rs"]
pub mod road_junctions_clearance;
#[path = "road/road_junctions_tie.rs"]
pub mod road_junctions_tie;
#[path = "road/road_junctions_overlap.rs"]
pub mod road_junctions_overlap;
#[path = "road/road_junctions_draw.rs"]
pub mod road_junctions_draw;
#[path = "road/road_junctions_edit.rs"]
pub mod road_junctions_edit;
#[path = "road/road_junctions_query.rs"]
pub mod road_junctions_query;
#[path = "road/road_pay.rs"]
pub mod road_pay;
#[path = "road/road_routes.rs"]
pub mod road_routes;
#[path = "road/road_routes_draw.rs"]
pub mod road_routes_draw;
#[path = "road/road_routes_find.rs"]
pub mod road_routes_find;
#[path = "road/road_pay_edit.rs"]
pub mod road_pay_edit;
#[path = "road/road_pay_query.rs"]
pub mod road_pay_query;
#[path = "road/road_walls.rs"]
pub mod road_walls;
#[path = "road/road_walls_draw.rs"]
pub mod road_walls_draw;
#[path = "road/road_walls_edit.rs"]
pub mod road_walls_edit;
#[path = "road/road_walls_query.rs"]
pub mod road_walls_query;
#[path = "road/road_platforms.rs"]
pub mod road_platforms;
#[path = "road/road_platforms_draw.rs"]
pub mod road_platforms_draw;
#[path = "road/road_platforms_edit.rs"]
pub mod road_platforms_edit;
#[path = "road/road_terrain_cloud.rs"]
pub mod road_terrain_cloud;
#[path = "road/road_heights.rs"]
pub(crate) mod road_heights;
#[path = "road/road_field.rs"]
pub(crate) mod road_field;
#[path = "road/road_drainage.rs"]
pub mod road_drainage;
#[path = "road/road_drainage_draw.rs"]
pub mod road_drainage_draw;
#[path = "road/road_swept.rs"]
pub mod road_swept;
#[path = "road/road_swept_draw.rs"]
pub mod road_swept_draw;
#[path = "road/road_terrains_analysis.rs"]
pub mod road_terrains_analysis;
#[path = "road/road_terrains_coverage.rs"]
pub mod road_terrains_coverage;
#[path = "road/road_terrains_coverage_draw.rs"]
pub mod road_terrains_coverage_draw;
#[path = "road/road_terrains_density.rs"]
pub mod road_terrains_density;
#[path = "road/road_terrains_stockpile.rs"]
pub mod road_terrains_stockpile;
#[path = "road/road_terrains_sparse.rs"]
pub mod road_terrains_sparse;
#[path = "road/road_terrains_compare.rs"]
pub mod road_terrains_compare;
#[path = "road/road_terrains_quality.rs"]
pub mod road_terrains_quality;
#[path = "road/road_terrains_repair.rs"]
pub mod road_terrains_repair;
#[path = "road/road_platforms_query.rs"]
pub mod road_platforms_query;
#[path = "road/road_alignments_import.rs"]
pub mod road_alignments_import;
#[path = "road/road_alignments_fit.rs"]
pub mod road_alignments_fit;
#[path = "road/road_profiles_fit.rs"]
pub mod road_profiles_fit;
#[path = "road/road_ifc.rs"]
pub mod road_ifc;
#[path = "road/road_scene.rs"]
pub mod road_scene;
#[path = "road/road_landxml.rs"]
pub mod road_landxml;
#[path = "road/road_sources.rs"]
pub mod road_sources;
#[path = "road/road_stakeout.rs"]
pub mod road_stakeout;
#[path = "road/road_stakeout_send.rs"]
pub mod road_stakeout_send;
#[path = "road/road_sheets.rs"]
pub mod road_sheets;
#[path = "road/road_sheets_sections.rs"]
pub mod road_sheets_sections;
#[path = "road/road_volumes.rs"]
pub mod road_volumes;
#[path = "road/road_volumes_approve.rs"]
pub mod road_volumes_approve;
#[path = "road/road_volumes_uncertainty.rs"]
pub mod road_volumes_uncertainty;
#[path = "road/road_volumes_progress.rs"]
pub mod road_volumes_progress;
#[path = "road/road_volumes_compare.rs"]
pub mod road_volumes_compare;
#[path = "road/road_volumes_history.rs"]
pub mod road_volumes_history;
#[path = "road/road_volumes_ledger.rs"]
pub mod road_volumes_ledger;
#[path = "road/road_volumes_states.rs"]
pub mod road_volumes_states;
#[path = "road/road_profiles_impact.rs"]
pub mod road_profiles_impact;
#[path = "road/road_alignments_impact.rs"]
pub mod road_alignments_impact;
#[path = "road/road_volumes_edit.rs"]
pub mod road_volumes_edit;
#[path = "road/road_volumes_query.rs"]
pub mod road_volumes_query;
#[path = "road/road_sections.rs"]
pub mod road_sections;
#[path = "road/road_sections_draw.rs"]
pub mod road_sections_draw;
#[path = "road/road_sections_edit.rs"]
pub mod road_sections_edit;
#[path = "road/road_sections_query.rs"]
pub mod road_sections_query;
#[path = "road/road_superelevations.rs"]
pub mod road_superelevations;
#[path = "road/road_superelevations_check.rs"]
pub mod road_superelevations_check;
#[path = "road/road_superelevations_transitions.rs"]
pub mod road_superelevations_transitions;
#[path = "road/road_superelevations_widen.rs"]
pub mod road_superelevations_widen;
#[path = "road/road_superelevations_edit.rs"]
pub mod road_superelevations_edit;
#[path = "road/road_superelevations_query.rs"]
pub mod road_superelevations_query;
#[path = "road/road_templates_edit.rs"]
pub mod road_templates_edit;
#[path = "road/road_terrains_contours.rs"]
pub mod road_terrains_contours;
#[path = "road/road_terrains_section.rs"]
pub mod road_terrains_section;
#[path = "road/road_terrains_sheet.rs"]
pub mod road_terrains_sheet;
#[path = "road/road_terrains_zeroline.rs"]
pub mod road_terrains_zeroline;
#[path = "road/road_terrains_cutfill.rs"]
pub mod road_terrains_cutfill;
#[path = "road/road_terrains_cutfill_sections.rs"]
mod road_terrains_cutfill_sections;
#[path = "road/road_terrains_create.rs"]
pub mod road_terrains_create;
#[path = "road/road_terrains_edit.rs"]
pub mod road_terrains_edit;
#[path = "road/road_terrains_query.rs"]
pub mod road_terrains_query;
#[path = "gis/lrs_events.rs"]
pub mod lrs_events;
#[path = "core/nurbs.rs"]
pub mod nurbs;
#[path = "arch/walls.rs"]
pub mod walls;
#[path = "arch/walls_create.rs"]
pub mod walls_create;
#[path = "arch/walls_rectangle.rs"]
pub mod walls_rectangle;
#[path = "arch/walls_partition.rs"]
pub mod walls_partition;
#[path = "arch/openings.rs"]
pub(crate) mod openings;
#[path = "arch/openings_create.rs"]
pub mod openings_create;
#[path = "arch/opening_placement.rs"]
pub mod opening_placement;
#[path = "arch/openings_edit.rs"]
pub mod openings_edit;
#[path = "arch/opening_distances.rs"]
pub mod opening_distances;
// Kapı ve pencere tipleri (docs/adr/0772).
#[path = "arch/opening_codes.rs"]
pub mod opening_codes;
#[path = "arch/opening_types.rs"]
pub(crate) mod opening_types;
#[path = "arch/door_audit.rs"]
pub(crate) mod door_audit;
#[path = "core/fixture_audit.rs"]
pub mod fixture_audit;
#[path = "arch/opening_types_edit.rs"]
pub mod opening_types_edit;
#[path = "arch/materials_edit.rs"]
pub mod materials_edit;
#[path = "arch/material_quantities.rs"]
pub mod material_quantities;
#[path = "arch/openings_query.rs"]
pub mod openings_query;
#[path = "arch/modules_edit.rs"]
pub mod modules_edit;
#[path = "arch/modules_query.rs"]
pub mod modules_query;
#[path = "arch/options_edit.rs"]
pub mod options_edit;
#[path = "arch/options_query.rs"]
pub mod options_query;
// Türetilmiş çizim: oda iç görünüşü (docs/adr/0799).
#[path = "arch/elevations.rs"]
pub mod elevations;
#[path = "arch/elevations_edit.rs"]
pub mod elevations_edit;
#[path = "arch/elevations_query.rs"]
pub mod elevations_query;
#[path = "arch/sections.rs"]
pub mod sections;
#[path = "arch/sections_edit.rs"]
pub mod sections_edit;
#[path = "arch/sections_query.rs"]
pub mod sections_query;
#[path = "arch/wall_distances.rs"]
pub mod wall_distances;
// Binanın IFC 4.3'ü: kentos-ifc'in bağdaştırıcısı (docs/adr/8047).
#[path = "arch/arch_ifc.rs"]
pub mod arch_ifc;
#[path = "arch/arch_recognize.rs"]
pub mod arch_recognize;
// Dijital proje denetimi: EK-9'un modelin cevapladığı soruları (docs/adr/8047 §9).
#[path = "arch/digital_audit.rs"]
pub mod digital_audit;
// Kolonun geçici ölçüleri (docs/adr/0712).
#[path = "arch/column_distances.rs"]
pub mod column_distances;
pub use column_distances::distances as columns_distances;
// Mimari denetim (docs/adr/0703).
#[path = "arch/arch_audit.rs"]
pub mod arch_audit;
#[path = "arch/arch_bonds.rs"]
pub mod arch_bonds;
// Harita bağı: sınır kenarının referansa kalıcı uzaklığı (docs/adr/2580).
#[path = "map/map_bonds.rs"]
pub(crate) mod map_bonds;
// Paralel kaydır'ın referansı ve Bu uzaklığı koru (docs/adr/2580).
#[path = "arch/edge_shift_reference.rs"]
pub(crate) mod edge_shift_reference;
#[path = "arch/arch_exceptions.rs"]
pub mod arch_exceptions;
pub use opening_distances::distances as openings_distances;
pub use wall_distances::distances as walls_distances;
#[path = "arch/spaces.rs"]
pub mod spaces;
#[path = "arch/spaces_create.rs"]
pub mod spaces_create;
// Döşeme ve Döşemeyi düzenle (docs/adr/0722).
#[path = "arch/roofs.rs"]
pub mod roofs;
#[path = "arch/slabs.rs"]
pub mod slabs;
#[path = "arch/slabs_create.rs"]
pub mod slabs_create;
#[path = "arch/slabs_edit.rs"]
pub mod slabs_edit;
#[path = "arch/slab_quantities.rs"]
pub mod slab_quantities;
#[path = "core/table_export.rs"]
pub mod table_export;
#[path = "arch/spaces_discover.rs"]
pub mod spaces_discover;
#[path = "arch/spaces_separators.rs"]
pub mod spaces_separators;
#[path = "arch/spaces_number.rs"]
pub mod spaces_number;
#[path = "arch/arch_elements.rs"]
pub mod arch_elements;
#[path = "arch/arch_edit.rs"]
pub mod arch_edit;
#[path = "arch/casework_run.rs"]
pub mod casework_run;
#[path = "arch/casework_edit.rs"]
pub mod casework_edit;
pub use casework_edit::query as casework_query;
#[path = "arch/legends.rs"]
pub mod legends;
#[path = "arch/levels_assign.rs"]
pub mod levels_assign;
#[path = "core/library_symbols.rs"]
mod library_symbols;
#[path = "arch/north.rs"]
pub mod north;
#[path = "arch/review_notes.rs"]
pub mod review_notes;
#[path = "core/standards_install.rs"]
pub mod standards_install;
#[path = "core/standards_exceptions.rs"]
pub mod standards_exceptions;
#[path = "core/standards_palettes.rs"]
pub mod standards_palettes;
#[path = "core/standards_install_library.rs"]
mod standards_install_library;
#[path = "core/standard_libraries.rs"]
pub mod standard_libraries;
#[path = "core/appearance_origin.rs"]
pub mod appearance_origin;
#[path = "arch/revisions.rs"]
pub mod revisions;
pub use arch_elements::{
    beams_create, columns_create, grids_create, railings_create, ramps_create, roofs_create, stairs_create,
};
#[path = "arch/tags.rs"]
pub mod tags;
#[path = "arch/schedules.rs"]
pub mod schedules;
pub use tags::{check_tags as tags_check, create as tags_create, edit as tags_edit};
pub use schedules::{create as schedules_create, edit as schedules_edit, query as schedules_query};
#[path = "arch/viewmarks.rs"]
pub mod viewmarks;
#[path = "arch/spaces_finish.rs"]
pub mod spaces_finish;
#[path = "core/navigator.rs"]
pub mod navigator;
#[path = "core/sheets_set.rs"]
pub mod sheets_set;
#[path = "core/sheets_sources.rs"]
pub mod sheets_sources;
#[path = "core/pdf_underlay.rs"]
pub mod pdf_underlay;
#[path = "core/xrefs.rs"]
pub mod xrefs;
#[path = "core/xrefs_bind.rs"]
mod xrefs_bind;
#[path = "core/xrefs_review.rs"]
pub mod xrefs_review;
#[path = "gis/links.rs"]
pub mod links;
#[path = "core/assets_check.rs"]
pub mod assets_check;
#[path = "gis/images_edit.rs"]
pub mod images_edit;
#[path = "gis/gpkg_profile.rs"]
pub mod gpkg_profile;
#[path = "gis/gpkg_tiles.rs"]
pub mod gpkg_tiles;
#[path = "gis/rasters_mosaic.rs"]
pub mod rasters_mosaic;
#[path = "gis/rasters_reproject.rs"]
pub mod rasters_reproject;
#[path = "gis/stac_host.rs"]
pub mod stac_host;
#[path = "gis/remote_host.rs"]
pub mod remote_host;
#[path = "gis/multidim_source.rs"]
pub mod multidim_source;
#[path = "gis/mesh_areas.rs"]
pub mod mesh_areas;
#[path = "gis/stac_prepare.rs"]
pub mod stac_prepare;
#[path = "gis/stac_search.rs"]
pub mod stac_search;
#[path = "core/inserts_clip.rs"]
pub mod inserts_clip;
#[path = "gis/raster_files.rs"]
pub mod raster_files;
#[path = "gis/rasters_georef.rs"]
pub mod rasters_georef;
#[path = "arch/templates_apply.rs"]
pub mod templates_apply;
#[path = "arch/template_review.rs"]
mod template_review;
pub use xrefs::{edit as xrefs_edit, list as xrefs_list};
pub use viewmarks::{create as viewmarks_create, edit as viewmarks_edit, references as viewmarks_references};
#[path = "arch/spaces_edit.rs"]
pub mod spaces_edit;
#[path = "arch/program_check.rs"]
pub mod program_check;
#[path = "arch/regulation_audit.rs"]
pub mod regulation_audit;
#[path = "arch/fire_audit.rs"]
pub mod fire_audit;
#[path = "arch/fire_marks.rs"]
pub mod fire_marks;
#[path = "arch/permit_set.rs"]
pub mod permit_set;
#[path = "arch/provisions_audit.rs"]
pub mod provisions_audit;
#[path = "arch/joinery_audit.rs"]
pub mod joinery_audit;
#[path = "arch/roof_audit.rs"]
pub mod roof_audit;
#[path = "arch/railing_audit.rs"]
pub mod railing_audit;
#[path = "arch/railing_voids.rs"]
pub mod railing_voids;
#[path = "core/block_values.rs"]
pub mod block_values;
#[path = "arch/elevator_cabins.rs"]
pub mod elevator_cabins;
#[path = "arch/parking_audit.rs"]
pub mod parking_audit;
#[path = "arch/program_edit.rs"]
pub mod program_edit;
#[path = "arch/spaces_quantities.rs"]
pub mod spaces_quantities;
#[path = "arch/walls_edit.rs"]
pub mod walls_edit;
#[path = "arch/walls_joins.rs"]
pub mod walls_joins;
#[path = "arch/wall_reference.rs"]
pub(crate) mod wall_reference;
#[path = "arch/walls_from_lines.rs"]
pub mod walls_from_lines;
#[path = "core/offset.rs"]
pub mod offset;
#[path = "core/point.rs"]
pub mod point;
#[path = "map/pointclouds.rs"]
pub mod pointclouds;
#[path = "map/pointcloud_areas.rs"]
pub mod pointcloud_areas;
#[path = "gis/meshes.rs"]
pub mod meshes;
#[path = "gis/source_bytes.rs"]
mod source_bytes;
#[path = "gis/sensors_ingest.rs"]
pub mod sensors_ingest;
#[path = "gis/sensors_latest.rs"]
pub mod sensors_latest;
#[path = "gis/time_export.rs"]
pub mod time_export;
#[path = "gis/time_filter.rs"]
mod time_filter;
#[path = "gis/time_query.rs"]
pub mod time_query;
#[path = "core/points_construct.rs"]
pub mod points_construct;
#[path = "core/polygon.rs"]
pub mod polygon;
#[path = "core/polyline.rs"]
pub mod polyline;
#[path = "core/python_call.rs"]
pub mod python_call;
#[path = "core/region.rs"]
pub mod region;
#[path = "arch/dispatch.rs"]
mod arch_dispatch;
#[path = "core/dispatch.rs"]
mod core_dispatch;
#[path = "gis/dispatch.rs"]
mod gis_dispatch;
#[path = "map/dispatch.rs"]
mod map_dispatch;
#[path = "road/dispatch.rs"]
mod road_dispatch;
#[path = "core/registry.rs"]
pub mod registry;
#[path = "core/replay.rs"]
pub mod replay;
#[path = "gis/scenarios_edit.rs"]
pub mod scenarios_edit;
#[path = "core/set.rs"]
pub mod set;
#[path = "core/settings_set.rs"]
pub mod settings_set;
#[path = "core/sheets_edit.rs"]
pub mod sheets_edit;
#[path = "core/sheets_create.rs"]
pub mod sheets_create;
#[path = "core/sheets_fit.rs"]
pub mod sheets_fit;
#[path = "core/sheets_details.rs"]
pub mod sheets_details;
#[path = "core/plot_styles_import.rs"]
pub mod plot_styles_import;
#[path = "core/sheets_levels.rs"]
pub mod sheets_levels;
#[path = "core/snap_find.rs"]
pub mod snap_find;
#[path = "core/profile_files.rs"]
pub mod profile_files;
#[path = "core/profiles.rs"]
pub mod profiles;
#[path = "core/standards.rs"]
pub mod standards;
#[path = "core/stretch.rs"]
pub mod stretch;
#[path = "gis/subnetwork_follow.rs"]
pub mod subnetwork_follow;
#[path = "gis/subnetworks.rs"]
pub mod subnetworks;
#[path = "gis/subnetworks_update.rs"]
pub mod subnetworks_update;
#[path = "core/style_tables.rs"]
pub mod style_tables;
#[path = "core/styles_edit.rs"]
pub mod styles_edit;
#[path = "gis/topology_check.rs"]
pub mod topology_check;
#[path = "gis/topology_exceptions.rs"]
pub mod topology_exceptions;
#[path = "gis/topology_repair.rs"]
pub mod topology_repair;
#[path = "gis/topology_follow.rs"]
pub mod topology_follow;
#[path = "core/transform.rs"]
pub mod transform;
#[path = "core/trim.rs"]
pub mod trim;
#[path = "core/ucs_set.rs"]
pub mod ucs_set;
#[path = "core/units_change.rs"]
pub mod units_change;
#[path = "arch/views_edit.rs"]
pub mod views_edit;
#[path = "gis/forms.rs"]
pub mod forms;
#[path = "gis/forms_audit.rs"]
pub mod forms_audit;
#[path = "gis/records.rs"]
pub mod records;
#[path = "gis/records_page.rs"]
pub mod records_page;
#[path = "gis/sources_query.rs"]
pub mod sources_query;
#[path = "gis/layers_crs.rs"]
pub mod layers_crs;
#[path = "gis/layers_reproject.rs"]
mod layers_reproject;
#[path = "gis/records_duplicate.rs"]
pub mod records_duplicate;
#[path = "gis/forms_compare.rs"]
pub mod forms_compare;
#[path = "gis/forms_sources.rs"]
pub mod forms_sources;
#[path = "gis/forms_edit.rs"]
pub mod forms_edit;
#[path = "gis/forms_fill.rs"]
pub mod forms_fill;
#[path = "gis/forms_submit.rs"]
pub mod forms_submit;
#[path = "gis/forms_recompute.rs"]
pub mod forms_recompute;
#[path = "gis/forms_test.rs"]
pub mod forms_test;
#[path = "gis/forms_transition.rs"]
pub mod forms_transition;
#[path = "gis/domains_edit.rs"]
pub mod domains_edit;
#[path = "gis/domains_export.rs"]
pub mod domains_export;
#[path = "gis/fields_migrate.rs"]
pub mod fields_migrate;
#[path = "gis/forms_export.rs"]
pub mod forms_export;
#[path = "gis/forms_import.rs"]
pub mod forms_import;
#[path = "gis/forms_validate.rs"]
pub mod forms_validate;
#[path = "gis/datasets_describe.rs"]
pub mod datasets_describe;
#[path = "gis/datasets_edit.rs"]
pub mod datasets_edit;
#[path = "gis/joins_edit.rs"]
pub mod joins_edit;
#[path = "gis/joins_query.rs"]
pub mod joins_query;
#[path = "gis/relations_edit.rs"]
pub mod relations_edit;
#[path = "core/widths.rs"]
pub mod widths;

pub use context::ExecutionContext;
pub use kentos_commands::{CommandError, CommandResult, CommandWarning};

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

use kentos_commands::{
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
    (kentos_commands::CAD_DIMENSION_SETS_CREATE, kentos_commands::CAD_DIMENSION_SETS_VERSION),
    (kentos_commands::CAD_DIMENSION_SETS_EDIT, kentos_commands::CAD_DIMENSION_SETS_VERSION),
    (kentos_commands::CAD_DIMENSION_SETS_QUERY, kentos_commands::CAD_DIMENSION_SETS_VERSION),
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
        kentos_commands::CAD_UCS_SET,
        kentos_commands::CAD_UCS_SET_VERSION,
    ),
    // The project's named views ([`views_edit`], docs/adr/0285).
    (
        kentos_commands::CAD_VIEWS_EDIT,
        kentos_commands::CAD_VIEWS_EDIT_VERSION,
    ),
    // The project's forms ([`forms_edit`], [`forms_validate`], [`forms_submit`], docs/adr/0401).
    (
        kentos_commands::CAD_FORMS_EDIT,
        kentos_commands::CAD_FORMS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_VALIDATE,
        kentos_commands::CAD_FORMS_VALIDATE_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_AUDIT,
        kentos_commands::CAD_FORMS_AUDIT_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_TEST,
        kentos_commands::CAD_FORMS_TEST_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_COMPARE,
        kentos_commands::CAD_FORMS_COMPARE_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_SOURCES,
        kentos_commands::CAD_FORMS_SOURCES_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_TRANSITION,
        kentos_commands::CAD_FORMS_TRANSITION_VERSION,
    ),
    (
        kentos_commands::CAD_DOMAINS_EDIT,
        kentos_commands::CAD_DOMAINS_EDIT_VERSION,
    ),
    // The value lists as a package and a layer's values in their canonical texts (docs/adr/0439 §7, §8).
    (
        kentos_commands::CAD_DOMAINS_EXPORT,
        kentos_commands::CAD_DOMAINS_EXPORT_VERSION,
    ),
    (
        kentos_commands::CAD_FIELDS_MIGRATE,
        kentos_commands::CAD_FIELDS_MIGRATE_VERSION,
    ),
    // What each attachment is, read from its bytes ([`attachments_inspect`], docs/adr/0442 §7).
    (
        kentos_commands::CAD_ATTACHMENTS_INSPECT,
        kentos_commands::CAD_ATTACHMENTS_INSPECT_VERSION,
    ),
    // The data schema a page at a time ([`schema_describe`], docs/adr/0443 §1).
    (
        kentos_commands::CAD_SCHEMA_DESCRIBE,
        kentos_commands::CAD_SCHEMA_DESCRIBE_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_EXPORT,
        kentos_commands::CAD_FORMS_EXPORT_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_IMPORT,
        kentos_commands::CAD_FORMS_IMPORT_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_RECOMPUTE,
        kentos_commands::CAD_FORMS_RECOMPUTE_VERSION,
    ),
    // Koşullu biçim ([`layers_table`], docs/adr/0413).
    (
        kentos_commands::CAD_LAYERS_TABLE,
        kentos_commands::CAD_LAYERS_TABLE_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_SUBTYPES,
        kentos_commands::CAD_LAYERS_SUBTYPES_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_CONTINGENTS,
        kentos_commands::CAD_LAYERS_CONTINGENTS_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_RULES,
        kentos_commands::CAD_LAYERS_RULES_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_INDEXES,
        kentos_commands::CAD_LAYERS_INDEXES_VERSION,
    ),
    // Records without a geometry ([`records`], docs/adr/0410).
    (
        kentos_commands::CAD_RECORDS_CREATE,
        kentos_commands::CAD_RECORDS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_RECORDS_DUPLICATE,
        kentos_commands::CAD_RECORDS_DUPLICATE_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_SUBMIT,
        kentos_commands::CAD_FORMS_SUBMIT_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_FILL,
        kentos_commands::CAD_FORMS_FILL_VERSION,
    ),
    // The project's relations ([`relations_edit`], docs/adr/0405).
    (
        kentos_commands::CAD_RELATIONS_EDIT,
        kentos_commands::CAD_RELATIONS_EDIT_VERSION,
    ),
    // The project's join views kept and read ([`joins_edit`], [`joins_query`], docs/adr/0527).
    (
        kentos_commands::CAD_JOINS_EDIT,
        kentos_commands::CAD_JOINS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_JOINS_QUERY,
        kentos_commands::CAD_JOINS_QUERY_VERSION,
    ),
    // A layer as a dataset ([`datasets_edit`], [`datasets_describe`], docs/adr/0633).
    (
        kentos_commands::CAD_DATASETS_EDIT,
        kentos_commands::CAD_DATASETS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_DATASETS_DESCRIBE,
        kentos_commands::CAD_DATASETS_DESCRIBE_VERSION,
    ),
    // Taramanın sınırı, adaları ve ilişkisi ([`hatches_edit`], docs/adr/0329 §4).
    (
        kentos_commands::CAD_HATCHES_EDIT,
        kentos_commands::CAD_HATCHES_EDIT_VERSION,
    ),
    // Çok kollu kılavuz ([`leaders_edit`], docs/adr/0326 §5).
    (
        kentos_commands::CAD_LEADERS_EDIT,
        kentos_commands::CAD_LEADERS_EDIT_VERSION,
    ),
    // Tablonun hücreleri, formülleri, alanları ve dosyası ([`tables_edit`], docs/adr/0327).
    (
        kentos_commands::CAD_TABLES_EDIT,
        kentos_commands::CAD_TABLES_EDIT_VERSION,
    ),
    // Alanlar and the text commands ([`fields_edit`], [`texts_edit`], [`texts_check`], docs/adr/0325 §3).
    (
        kentos_commands::CAD_FIELDS_EDIT,
        kentos_commands::CAD_FIELDS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_TEXTS_EDIT,
        kentos_commands::CAD_TEXTS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_TEXTS_CHECK,
        kentos_commands::CAD_TEXTS_CHECK_VERSION,
    ),
    // Yazı tipleri ([`fonts_report`], docs/adr/0324 §5).
    (
        kentos_commands::CAD_FONTS_REPORT,
        kentos_commands::CAD_FONTS_REPORT_VERSION,
    ),
    // Ölçekli açıklamalar ([`annotations_scales`], docs/adr/0323 §4).
    (
        kentos_commands::CAD_ANNOTATIONS_SCALES,
        kentos_commands::CAD_ANNOTATIONS_SCALES_VERSION,
    ),
    // 2B düzleme indir ([`entities_flatten`], docs/adr/1061 §4).
    (
        kentos_commands::CAD_ENTITIES_FLATTEN,
        kentos_commands::CAD_ENTITIES_FLATTEN_VERSION,
    ),
    // Öne getir and Arkaya gönder ([`entities_order`], docs/adr/0260 §5).
    (
        kentos_commands::CAD_ENTITIES_ORDER,
        kentos_commands::CAD_ENTITIES_ORDER_VERSION,
    ),
    // A NURBS curve by control points or through points ([`nurbs`], docs/adr/0259).
    (
        kentos_commands::CAD_NURBS_CREATE,
        kentos_commands::CAD_NURBS_CREATE_VERSION,
    ),
    // Aks, Kolon, Merdiven, Rampa and Mimari nesneyi düzenle ([`arch_elements`], [`arch_edit`], docs/adr/0343).
    (
        kentos_commands::CAD_GRIDS_CREATE,
        kentos_commands::CAD_GRIDS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_COLUMNS_CREATE,
        kentos_commands::CAD_COLUMNS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_STAIRS_CREATE,
        kentos_commands::CAD_STAIRS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_RAMPS_CREATE,
        kentos_commands::CAD_RAMPS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_BEAMS_CREATE,
        kentos_commands::CAD_BEAMS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_ROOFS_CREATE,
        kentos_commands::CAD_ROOFS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_RAILINGS_CREATE,
        kentos_commands::CAD_RAILINGS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_ARCH_EDIT,
        kentos_commands::CAD_ARCH_EDIT_VERSION,
    ),
    // Etiket, Etiketleri denetle, Çizelge and Çizelgeyi düzenle ([`tags`], [`schedules`], docs/adr/0344).
    (
        kentos_commands::CAD_TAGS_CREATE,
        kentos_commands::CAD_TAGS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_TAGS_CHECK,
        kentos_commands::CAD_TAGS_CHECK_VERSION,
    ),
    (
        kentos_commands::CAD_TAGS_EDIT,
        kentos_commands::CAD_TAGS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_SCHEDULES_CREATE,
        kentos_commands::CAD_SCHEDULES_CREATE_VERSION,
    ),
    (kentos_commands::CAD_EXTRACT_RUN, kentos_commands::CAD_EXTRACT_RUN_VERSION),
    (kentos_commands::CAD_EXTRACT_TABLE, kentos_commands::CAD_EXTRACT_TABLE_VERSION),
    (
        kentos_commands::CAD_SCHEDULES_EDIT,
        kentos_commands::CAD_SCHEDULES_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_SCHEDULES_QUERY,
        kentos_commands::CAD_SCHEDULES_QUERY_VERSION,
    ),
    // Görünüm işareti, Görünüm işaretini düzenle, Başvurular and Döşeme ve tavan ([`viewmarks`], [`spaces_finish`],
    // docs/adr/0345).
    (
        kentos_commands::CAD_VIEWMARKS_CREATE,
        kentos_commands::CAD_VIEWMARKS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_VIEWMARKS_EDIT,
        kentos_commands::CAD_VIEWMARKS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_VIEWMARKS_REFERENCES,
        kentos_commands::CAD_VIEWMARKS_REFERENCES_VERSION,
    ),
    (
        kentos_commands::CAD_SPACES_FINISH,
        kentos_commands::CAD_SPACES_FINISH_VERSION,
    ),
    // Mimari şablonu ([`templates_apply`], docs/adr/0346).
    (
        kentos_commands::CAD_TEMPLATES_APPLY,
        kentos_commands::CAD_TEMPLATES_APPLY_VERSION,
    ),
    // Pafta seti ([`sheets_set`], docs/adr/0347).
    (
        kentos_commands::CAD_SHEETS_SET,
        kentos_commands::CAD_SHEETS_SET_VERSION,
    ),
    // Proje gezgini ([`navigator`], docs/adr/0778).
    (
        kentos_commands::CAD_NAVIGATOR_READ,
        kentos_commands::CAD_NAVIGATOR_READ_VERSION,
    ),
    // Dış başvurular ([`xrefs`], docs/adr/0348).
    (
        kentos_commands::CAD_XREFS_EDIT,
        kentos_commands::CAD_XREFS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_XREFS_LIST,
        kentos_commands::CAD_XREFS_LIST_VERSION,
    ),
    // Kırp ([`inserts_clip`], docs/adr/0351 §1).
    (
        kentos_commands::CAD_INSERTS_CLIP,
        kentos_commands::CAD_INSERTS_CLIP_VERSION,
    ),
    // Bağlı dosyaların durumu ([`assets_check`], docs/adr/0349 §2).
    (
        kentos_commands::CAD_ASSETS_CHECK,
        kentos_commands::CAD_ASSETS_CHECK_VERSION,
    ),
    // Resmin kaynağı ([`images_edit`], docs/adr/0350 §1).
    (
        kentos_commands::CAD_IMAGES_EDIT,
        kentos_commands::CAD_IMAGES_EDIT_VERSION,
    ),
    // STAC keşfi ve veri hazırlama ([`stac_search`], [`stac_prepare`], docs/adr/0528).
    (
        kentos_commands::CAD_STAC_SEARCH,
        kentos_commands::CAD_STAC_SEARCH_VERSION,
    ),
    (
        kentos_commands::CAD_STAC_PREPARE,
        kentos_commands::CAD_STAC_PREPARE_VERSION,
    ),
    // Zamanda sorgu ([`time_query`], docs/adr/0537).
    (
        kentos_commands::CAD_TIME_QUERY,
        kentos_commands::CAD_TIME_QUERY_VERSION,
    ),
    // Sensör gözlemleri ([`sensors_ingest`], [`sensors_latest`], docs/adr/0646).
    (
        kentos_commands::CAD_SENSORS_INGEST,
        kentos_commands::CAD_SENSORS_INGEST_VERSION,
    ),
    (
        kentos_commands::CAD_SENSORS_LATEST,
        kentos_commands::CAD_SENSORS_LATEST_VERSION,
    ),
    // Nokta bulutunun bilgisi ve XYZ sor ([`pointclouds`], docs/adr/0536).
    (
        kentos_commands::CAD_POINTCLOUDS_DESCRIBE,
        kentos_commands::CAD_POINTCLOUDS_DESCRIBE_VERSION,
    ),
    (
        kentos_commands::CAD_POINTCLOUDS_QUERY,
        kentos_commands::CAD_POINTCLOUDS_QUERY_VERSION,
    ),
    // Nokta bulutunun alan ve profil sorgusu, meshin bilgisi ve sorgusu ([`pointcloud_areas`], [`meshes`], docs/adr/0552).
    (
        kentos_commands::CAD_POINTCLOUDS_REGION,
        kentos_commands::CAD_POINTCLOUDS_REGION_VERSION,
    ),
    (
        kentos_commands::CAD_POINTCLOUDS_PROFILE,
        kentos_commands::CAD_POINTCLOUDS_PROFILE_VERSION,
    ),
    (
        kentos_commands::CAD_MESHES_DESCRIBE,
        kentos_commands::CAD_MESHES_DESCRIBE_VERSION,
    ),
    (
        kentos_commands::CAD_MESHES_QUERY,
        kentos_commands::CAD_MESHES_QUERY_VERSION,
    ),
    // Meshin alanı ve hattı ([`mesh_areas`], docs/adr/0645 §7).
    (
        kentos_commands::CAD_MESHES_REGION,
        kentos_commands::CAD_MESHES_REGION_VERSION,
    ),
    (
        kentos_commands::CAD_MESHES_PROFILE,
        kentos_commands::CAD_MESHES_PROFILE_VERSION,
    ),
    // Sanal mozaik ([`rasters_mosaic`], docs/adr/0517).
    (
        kentos_commands::CAD_RASTERS_MOSAIC,
        kentos_commands::CAD_RASTERS_MOSAIC_VERSION,
    ),
    // Rasteri projenin sistemine çevir ([`rasters_reproject`], docs/adr/0644 §3).
    (
        kentos_commands::CAD_RASTERS_REPROJECT,
        kentos_commands::CAD_RASTERS_REPROJECT_VERSION,
    ),
    // GeoPackage karoları ([`gpkg_tiles`], docs/adr/0556).
    (
        kentos_commands::CAD_RASTERS_GPKGREAD,
        kentos_commands::CAD_RASTERS_GPKGREAD_VERSION,
    ),
    (
        kentos_commands::CAD_RASTERS_GPKGWRITE,
        kentos_commands::CAD_RASTERS_GPKGWRITE_VERSION,
    ),
    // Raster oturt'un kaydı ([`rasters_georef`], docs/adr/0518).
    (
        kentos_commands::CAD_RASTERS_GEOREF,
        kentos_commands::CAD_RASTERS_GEOREF_VERSION,
    ),
    // Mahal, Mahali düzenle and Metraj ([`spaces_create`], [`spaces_edit`], [`spaces_quantities`], docs/adr/0342).
    (kentos_commands::CAD_SPACES_DISCOVER, kentos_commands::CAD_SPACES_DISCOVER_VERSION),
    (kentos_commands::CAD_SPACES_NUMBER,kentos_commands::CAD_SPACES_NUMBER_VERSION),
    (kentos_commands::CAD_SPACES_SEPARATORS,kentos_commands::CAD_SPACES_SEPARATORS_VERSION),
    (
        kentos_commands::CAD_SPACES_CREATE,
        kentos_commands::CAD_SPACES_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_SPACES_EDIT,
        kentos_commands::CAD_SPACES_EDIT_VERSION,
    ),
    // Bina programı ve denetimi ([`program_edit`], [`program_check`], docs/adr/0773).
    (
        kentos_commands::CAD_PROGRAM_EDIT,
        kentos_commands::CAD_PROGRAM_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_PROGRAM_CHECK,
        kentos_commands::CAD_PROGRAM_CHECK_VERSION,
    ),
    (
        kentos_commands::CAD_SPACES_QUANTITIES,
        kentos_commands::CAD_SPACES_QUANTITIES_VERSION,
    ),
    // Döşeme and Döşemeyi düzenle ([`slabs_create`], [`slabs_edit`], docs/adr/0722).
    (kentos_commands::CAD_SLABS_CREATE, kentos_commands::CAD_SLABS_CREATE_VERSION),
    (kentos_commands::CAD_SLABS_EDIT, kentos_commands::CAD_SLABS_EDIT_VERSION),
    // Kapı, Pencere, Boşluk and Açıklığı düzenle ([`openings_create`], [`openings_edit`], docs/adr/0341).
    (
        kentos_commands::CAD_OPENINGS_CREATE,
        kentos_commands::CAD_OPENINGS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_OPENINGS_EDIT,
        kentos_commands::CAD_OPENINGS_EDIT_VERSION,
    ),
    // Kapı ve pencere tipleri ([`opening_types_edit`], [`openings_query`], docs/adr/0772).
    (
        kentos_commands::CAD_OPENING_TYPES_EDIT,
        kentos_commands::CAD_OPENING_TYPES_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_OPENINGS_QUERY,
        kentos_commands::CAD_OPENINGS_QUERY_VERSION,
    ),
    // Malzemeler ([`materials_edit`], docs/adr/0785).
    (kentos_commands::CAD_MATERIALS_EDIT, kentos_commands::CAD_MATERIALS_EDIT_VERSION),
    // Oda modülleri ([`modules_edit`], [`modules_query`], docs/adr/0775).
    (kentos_commands::CAD_MODULES_EDIT, kentos_commands::CAD_MODULES_EDIT_VERSION),
    (kentos_commands::CAD_MODULES_QUERY, kentos_commands::CAD_MODULES_QUERY_VERSION),
    // Tasarım seçenekleri ([`options_edit`], [`options_query`], docs/adr/0788).
    (kentos_commands::CAD_OPTIONS_EDIT, kentos_commands::CAD_OPTIONS_EDIT_VERSION),
    (kentos_commands::CAD_OPTIONS_QUERY, kentos_commands::CAD_OPTIONS_QUERY_VERSION),
    // Oda iç görünüşü ([`elevations_edit`], [`elevations_query`], docs/adr/0799).
    (
        kentos_commands::CAD_ELEVATIONS_EDIT,
        kentos_commands::CAD_ELEVATIONS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_ELEVATIONS_QUERY,
        kentos_commands::CAD_ELEVATIONS_QUERY_VERSION,
    ),
    // Kesit çıkar ([`sections_edit`], [`sections_query`], docs/adr/8042).
    (
        kentos_commands::CAD_SECTIONS_EDIT,
        kentos_commands::CAD_SECTIONS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_SECTIONS_QUERY,
        kentos_commands::CAD_SECTIONS_QUERY_VERSION,
    ),
    // Açıklığın geçici ölçüleri ([`opening_distances`], docs/adr/0701).
    (
        kentos_commands::CAD_OPENINGS_DISTANCES,
        kentos_commands::CAD_OPENINGS_DISTANCES_VERSION,
    ),
    // Duvarın geçici ölçüleri ([`wall_distances`], docs/adr/0710).
    (
        kentos_commands::CAD_WALLS_DISTANCES,
        kentos_commands::CAD_WALLS_DISTANCES_VERSION,
    ),
    // Kolonun geçici ölçüleri ([`column_distances`], docs/adr/0712).
    (
        kentos_commands::CAD_COLUMNS_DISTANCES,
        kentos_commands::CAD_COLUMNS_DISTANCES_VERSION,
    ),
    // Mimari denetim ([`arch_audit`], docs/adr/0703).
    (
        kentos_commands::CAD_ARCH_AUDIT,
        kentos_commands::CAD_ARCH_AUDIT_VERSION,
    ),
    // Denetim istisnası ([`arch_exceptions`], docs/adr/0732 §3).
    (
        kentos_commands::CAD_ARCH_EXCEPTIONS,
        kentos_commands::CAD_ARCH_EXCEPTIONS_VERSION,
    ),
    // Mutfak dizisi ([`casework_run`], docs/adr/0706, 0727).
    (
        kentos_commands::CAD_CASEWORK_RUN,
        kentos_commands::CAD_CASEWORK_RUN_VERSION,
    ),
    // Mutfak dizisini düzenle ve oku ([`casework_edit`], docs/adr/0727 §6).
    (
        kentos_commands::CAD_CASEWORK_EDIT,
        kentos_commands::CAD_CASEWORK_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_CASEWORK_QUERY,
        kentos_commands::CAD_CASEWORK_QUERY_VERSION,
    ),
    // Kat ver ([`levels_assign`], docs/adr/0717).
    (
        kentos_commands::CAD_LEVELS_ASSIGN,
        kentos_commands::CAD_LEVELS_ASSIGN_VERSION,
    ),
    // Kuzey oku, Revizyon işaretle and Malzeme lejandı ([`north`], [`revisions`], [`legends`], docs/adr/0721).
    (
        kentos_commands::CAD_NORTH_CREATE,
        kentos_commands::CAD_NORTH_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_REVISIONS_MARK,
        kentos_commands::CAD_REVISIONS_MARK_VERSION,
    ),
    (
        kentos_commands::CAD_LEGENDS_MATERIALS,
        kentos_commands::CAD_LEGENDS_MATERIALS_VERSION,
    ),
    // Duvar, Çizgiden duvar, Duvarı düzenle and Duvar birleşimleri ([`walls_create`], [`walls_from_lines`], [`walls_edit`],
    // [`walls_joins`], docs/adr/0339, 0340).
    (
        kentos_commands::CAD_WALLS_CREATE,
        kentos_commands::CAD_WALLS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_WALLS_RECTANGLE,
        kentos_commands::CAD_WALLS_RECTANGLE_VERSION,
    ),
    (kentos_commands::CAD_WALLS_PARTITION, kentos_commands::CAD_WALLS_PARTITION_VERSION),
    (
        kentos_commands::CAD_WALLS_FROM_LINES,
        kentos_commands::CAD_WALLS_FROM_LINES_VERSION,
    ),
    (
        kentos_commands::CAD_ARCH_RECOGNIZE,
        kentos_commands::CAD_ARCH_RECOGNIZE_VERSION,
    ),
    (
        kentos_commands::CAD_WALLS_EDIT,
        kentos_commands::CAD_WALLS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_WALLS_JOINS,
        kentos_commands::CAD_WALLS_JOINS_VERSION,
    ),
    (kentos_commands::CAD_DONUT_CREATE, kentos_commands::CAD_DONUT_CREATE_VERSION),
    // Rectangles and regular polygons ([`shapes`], docs/adr/0290).
    (
        kentos_commands::CAD_RECTANGLE_CREATE,
        kentos_commands::CAD_RECTANGLE_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_POLYGON_REGULAR,
        kentos_commands::CAD_POLYGON_REGULAR_VERSION,
    ),
    (kentos_commands::CAD_SECTOR_CREATE, kentos_commands::CAD_SECTOR_CREATE_VERSION),
    (kentos_commands::CAD_REVCLOUD_CREATE, kentos_commands::CAD_REVCLOUD_CREATE_VERSION),
    (kentos_commands::CAD_ENTITIES_ADJOIN, kentos_commands::CAD_ENTITIES_ADJOIN_VERSION),
    (kentos_commands::CAD_PARCELS_CREATE, kentos_commands::CAD_PARCELS_CREATE_VERSION),
    (kentos_commands::CAD_CONSTRUCTIONS_CREATE, kentos_commands::CAD_CONSTRUCTIONS_CREATE_VERSION),
    (kentos_commands::CAD_AREAS_EDIT, kentos_commands::CAD_AREAS_EDIT_VERSION),
    (kentos_commands::CAD_ANNOTATIONS_WRITE, kentos_commands::CAD_ANNOTATIONS_WRITE_VERSION),
    (kentos_commands::CAD_OBJECTS_DERIVE, kentos_commands::CAD_OBJECTS_DERIVE_VERSION),
    (kentos_commands::CAD_ROADS_EDIT, kentos_commands::CAD_ROADS_EDIT_VERSION),
    (kentos_commands::CAD_IMAGES_CLIP, kentos_commands::CAD_IMAGES_CLIP_VERSION),
    (kentos_commands::CAD_TIME_VERSION, kentos_commands::CAD_TIME_VERSION_VERSION),
    (kentos_commands::CAD_SURVEY_POINTS, kentos_commands::CAD_SURVEY_POINTS_VERSION),
    (kentos_commands::CAD_SURVEY_ADJUST, kentos_commands::CAD_SURVEY_ADJUST_VERSION),
    (kentos_commands::CAD_SHAPES_EDIT, kentos_commands::CAD_SHAPES_EDIT_VERSION),
    // A path's vertices and edges ([`entities_vertex`], docs/adr/0279).
    (
        kentos_commands::CAD_ENTITIES_VERTEX,
        kentos_commands::CAD_ENTITIES_VERTEX_VERSION,
    ),
    // What a click can mean and the sub-object it falls on ([`entities_pick`], docs/adr/0276).
    (
        kentos_commands::CAD_ENTITIES_PICK,
        kentos_commands::CAD_ENTITIES_PICK_VERSION,
    ),
    // The standard DE-9IM relations ([`entities_relate`], docs/adr/0501).
    (
        kentos_commands::CAD_ENTITIES_RELATE,
        kentos_commands::CAD_ENTITIES_RELATE_VERSION,
    ),
    // Points between coordinate systems with their heights, and how ([`crs_transform`], docs/adr/0519).
    (
        kentos_commands::CAD_CRS_TRANSFORM,
        kentos_commands::CAD_CRS_TRANSFORM_VERSION,
    ),
    // The selection as one explicit query ([`entities_select`], docs/adr/0275).
    (
        kentos_commands::CAD_ENTITIES_ISOLATE,
        kentos_commands::CAD_ENTITIES_ISOLATE_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_SELECT,
        kentos_commands::CAD_ENTITIES_SELECT_VERSION,
    ),
    // The object snap at a point, with its object's place ([`snap_find`], docs/adr/0272).
    (
        kentos_commands::CAD_SNAP_FIND,
        kentos_commands::CAD_SNAP_FIND_VERSION,
    ),
    // The drafting aids' points by what they are made of ([`points_construct`], docs/adr/0270).
    (
        kentos_commands::CAD_POINTS_CONSTRUCT,
        kentos_commands::CAD_POINTS_CONSTRUCT_VERSION,
    ),
    // Daire's, Yay's and Elips's methods by what they are made of ([`curves_construct`], docs/adr/0289).
    (
        kentos_commands::CAD_CURVES_CONSTRUCT,
        kentos_commands::CAD_CURVES_CONSTRUCT_VERSION,
    ),
    // Ötele's semantic command ([`offset`], docs/adr/0246).
    (
        kentos_commands::CAD_ENTITIES_OFFSET,
        kentos_commands::CAD_ENTITIES_OFFSET_VERSION,
    ),
    // The drawing tools' objects without a command of their own ([`create`], docs/adr/0057).
    (
        kentos_commands::CAD_ENTITIES_CREATE,
        kentos_commands::CAD_ENTITIES_CREATE_VERSION,
    ),
    // Öznitelikler's layer, colour, symbol, attributes and label ([`set`], docs/adr/0066).
    (CAD_ENTITIES_SET, CAD_ENTITIES_SET_VERSION),
    // Blok oluştur and the Bloklar panel ([`blocks_define`], [`blocks_edit`], docs/adr/0144).
    (
        kentos_commands::CAD_BLOCKS_DEFINE,
        kentos_commands::CAD_BLOCKS_DEFINE_VERSION,
    ),
    (
        kentos_commands::CAD_BLOCKS_EDIT,
        kentos_commands::CAD_BLOCKS_EDIT_VERSION,
    ),
    // Harita servisi and the service layers' menus ([`layers_service`], docs/adr/0208 §15).
    (
        kentos_commands::CAD_LAYERS_SERVICE,
        kentos_commands::CAD_LAYERS_SERVICE_VERSION,
    ),
    // Ağlar ([`network_define`], docs/adr/0209 §11).
    (
        kentos_commands::CAD_NETWORK_DEFINE,
        kentos_commands::CAD_NETWORK_DEFINE_VERSION,
    ),
    // Olay bağı ([`linearref_define`], docs/adr/0569 §2).
    (
        kentos_commands::CAD_LINEARREF_DEFINE,
        kentos_commands::CAD_LINEARREF_DEFINE_VERSION,
    ),
    // KentOS Yol's terrain surfaces ([`road_terrains_create`], [`road_terrains_edit`], [`road_terrains_query`],
    // plan/road.md Y2.3).
    (
        kentos_commands::ROAD_TERRAINS_CREATE,
        kentos_commands::ROAD_TERRAINS_CREATE_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_EDIT,
        kentos_commands::ROAD_TERRAINS_EDIT_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_QUERY,
        kentos_commands::ROAD_TERRAINS_QUERY_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_CONTOURS,
        kentos_commands::ROAD_TERRAINS_CONTOURS_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_SHEET_CONTOURS,
        kentos_commands::ROAD_TERRAINS_SHEET_CONTOURS_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_SECTIONCHECK,
        kentos_commands::ROAD_TERRAINS_SECTIONCHECK_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_ZEROLINE,
        kentos_commands::ROAD_TERRAINS_ZEROLINE_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_CUTFILL,
        kentos_commands::ROAD_TERRAINS_CUTFILL_VERSION,
    ),
    (
        kentos_commands::FILE_IMPORT_LANDXML,
        kentos_commands::FILE_IMPORT_LANDXML_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_CREATE,
        kentos_commands::ROAD_ALIGNMENTS_CREATE_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_EDIT,
        kentos_commands::ROAD_ALIGNMENTS_EDIT_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_LABEL,
        kentos_commands::ROAD_ALIGNMENTS_LABEL_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_QUERY,
        kentos_commands::ROAD_ALIGNMENTS_QUERY_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_CREATE,
        kentos_commands::ROAD_PROFILES_CREATE_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_EDIT,
        kentos_commands::ROAD_PROFILES_EDIT_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_DRAW,
        kentos_commands::ROAD_PROFILES_DRAW_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_SIGHT,
        kentos_commands::ROAD_PROFILES_SIGHT_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_PROPOSE,
        kentos_commands::ROAD_PROFILES_PROPOSE_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_CHECK,
        kentos_commands::ROAD_ALIGNMENTS_CHECK_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_CHECK,
        kentos_commands::ROAD_PROFILES_CHECK_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_LENGTHS,
        kentos_commands::ROAD_PROFILES_LENGTHS_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_QUERY,
        kentos_commands::ROAD_PROFILES_QUERY_VERSION,
    ),
    (
        kentos_commands::ROAD_TEMPLATES_EDIT,
        kentos_commands::ROAD_TEMPLATES_EDIT_VERSION,
    ),
    (
        kentos_commands::ROAD_SECTIONS_EDIT,
        kentos_commands::ROAD_SECTIONS_EDIT_VERSION,
    ),
    (
        kentos_commands::ROAD_SECTIONS_QUERY,
        kentos_commands::ROAD_SECTIONS_QUERY_VERSION,
    ),
    (
        kentos_commands::ROAD_SECTIONS_DRAW,
        kentos_commands::ROAD_SECTIONS_DRAW_VERSION,
    ),
    (
        kentos_commands::ROAD_SUPERELEVATIONS_EDIT,
        kentos_commands::ROAD_SUPERELEVATIONS_EDIT_VERSION,
    ),
    (
        kentos_commands::ROAD_SUPERELEVATIONS_QUERY,
        kentos_commands::ROAD_SUPERELEVATIONS_QUERY_VERSION,
    ),
    (
        kentos_commands::ROAD_SUPERELEVATIONS_CHECK,
        kentos_commands::ROAD_SUPERELEVATIONS_CHECK_VERSION,
    ),
    (
        kentos_commands::ROAD_SUPERELEVATIONS_WIDEN,
        kentos_commands::ROAD_SUPERELEVATIONS_WIDEN_VERSION,
    ),
    (
        kentos_commands::ROAD_SUPERELEVATIONS_TRANSITIONS,
        kentos_commands::ROAD_SUPERELEVATIONS_TRANSITIONS_VERSION,
    ),
    (kentos_commands::ROAD_CORRIDORS_EDIT, kentos_commands::ROAD_CORRIDORS_EDIT_VERSION),
    (kentos_commands::ROAD_CORRIDORS_QUERY, kentos_commands::ROAD_CORRIDORS_QUERY_VERSION),
    (kentos_commands::ROAD_CORRIDORS_WATER, kentos_commands::ROAD_CORRIDORS_WATER_VERSION),
    (kentos_commands::ROAD_CORRIDORS_DAYLIGHTS, kentos_commands::ROAD_CORRIDORS_DAYLIGHTS_VERSION),
    (kentos_commands::ROAD_CORRIDORS_PROTECTED, kentos_commands::ROAD_CORRIDORS_PROTECTED_VERSION),
    (kentos_commands::ROAD_CORRIDORS_PREFLIGHT, kentos_commands::ROAD_CORRIDORS_PREFLIGHT_VERSION),
    (kentos_commands::ROAD_CORRIDORS_DRAW, kentos_commands::ROAD_CORRIDORS_DRAW_VERSION),
    (kentos_commands::ROAD_VOLUMES_EDIT, kentos_commands::ROAD_VOLUMES_EDIT_VERSION),
    (kentos_commands::ROAD_VOLUMES_QUERY, kentos_commands::ROAD_VOLUMES_QUERY_VERSION),
    (kentos_commands::ROAD_VOLUMES_APPROVE, kentos_commands::ROAD_VOLUMES_APPROVE_VERSION),
    (kentos_commands::ROAD_VOLUMES_UNCERTAINTY, kentos_commands::ROAD_VOLUMES_UNCERTAINTY_VERSION),
    (kentos_commands::ROAD_VOLUMES_PROGRESS, kentos_commands::ROAD_VOLUMES_PROGRESS_VERSION),
    (kentos_commands::ROAD_VOLUMES_COMPARE, kentos_commands::ROAD_VOLUMES_COMPARE_VERSION),
    (kentos_commands::ROAD_VOLUMES_LEDGER, kentos_commands::ROAD_VOLUMES_LEDGER_VERSION),
    (kentos_commands::ROAD_VOLUMES_STATES, kentos_commands::ROAD_VOLUMES_STATES_VERSION),
    (kentos_commands::ROAD_PROFILES_IMPACT, kentos_commands::ROAD_PROFILES_IMPACT_VERSION),
    (kentos_commands::ROAD_ALIGNMENTS_IMPACT, kentos_commands::ROAD_ALIGNMENTS_IMPACT_VERSION),
    (kentos_commands::ROAD_VOLUMES_HISTORY, kentos_commands::ROAD_VOLUMES_HISTORY_VERSION),
    (kentos_commands::ROAD_HAULS_EDIT, kentos_commands::ROAD_HAULS_EDIT_VERSION),
    (kentos_commands::ROAD_HAULS_QUERY, kentos_commands::ROAD_HAULS_QUERY_VERSION),
    (kentos_commands::ROAD_HAULS_PLAN, kentos_commands::ROAD_HAULS_PLAN_VERSION),
    (kentos_commands::ROAD_HAULS_APPROVE, kentos_commands::ROAD_HAULS_APPROVE_VERSION),
    (kentos_commands::ROAD_HAULS_NETWORK, kentos_commands::ROAD_HAULS_NETWORK_VERSION),
    (kentos_commands::ROAD_HAULS_TRUCKS, kentos_commands::ROAD_HAULS_TRUCKS_VERSION),
    (kentos_commands::ROAD_GUARDRAILS_EDIT, kentos_commands::ROAD_GUARDRAILS_EDIT_VERSION),
    (kentos_commands::ROAD_GUARDRAILS_QUERY, kentos_commands::ROAD_GUARDRAILS_QUERY_VERSION),
    (kentos_commands::ROAD_GUARDRAILS_DRAW, kentos_commands::ROAD_GUARDRAILS_DRAW_VERSION),
    (kentos_commands::ROAD_CULVERTS_EDIT, kentos_commands::ROAD_CULVERTS_EDIT_VERSION),
    (kentos_commands::ROAD_CULVERTS_QUERY, kentos_commands::ROAD_CULVERTS_QUERY_VERSION),
    (kentos_commands::ROAD_CULVERTS_DRAW, kentos_commands::ROAD_CULVERTS_DRAW_VERSION),
    (kentos_commands::ROAD_CULVERTS_HYDRAULICS, kentos_commands::ROAD_CULVERTS_HYDRAULICS_VERSION),
    (kentos_commands::ROAD_CULVERTS_CLOSURE, kentos_commands::ROAD_CULVERTS_CLOSURE_VERSION),
    (kentos_commands::ROAD_WALLS_EDIT, kentos_commands::ROAD_WALLS_EDIT_VERSION),
    (kentos_commands::ROAD_WALLS_QUERY, kentos_commands::ROAD_WALLS_QUERY_VERSION),
    (kentos_commands::ROAD_WALLS_DRAW, kentos_commands::ROAD_WALLS_DRAW_VERSION),
    (kentos_commands::ROAD_PLATFORMS_EDIT, kentos_commands::ROAD_PLATFORMS_EDIT_VERSION),
    (kentos_commands::ROAD_TERRAINS_FROM_CLOUD, kentos_commands::ROAD_TERRAINS_FROM_CLOUD_VERSION),
    (kentos_commands::ROAD_DRAINAGE_QUERY, kentos_commands::ROAD_DRAINAGE_QUERY_VERSION),
    (kentos_commands::ROAD_DRAINAGE_DRAW, kentos_commands::ROAD_DRAINAGE_DRAW_VERSION),
    (kentos_commands::ROAD_SWEPT_QUERY, kentos_commands::ROAD_SWEPT_QUERY_VERSION),
    (kentos_commands::ROAD_SWEPT_DRAW, kentos_commands::ROAD_SWEPT_DRAW_VERSION),
    (kentos_commands::ROAD_TERRAINS_ANALYSIS, kentos_commands::ROAD_TERRAINS_ANALYSIS_VERSION),
    (kentos_commands::ROAD_TERRAINS_COMPARE, kentos_commands::ROAD_TERRAINS_COMPARE_VERSION),
    (kentos_commands::ROAD_TERRAINS_QUALITY, kentos_commands::ROAD_TERRAINS_QUALITY_VERSION),
    (kentos_commands::ROAD_TERRAINS_REPAIR, kentos_commands::ROAD_TERRAINS_REPAIR_VERSION),
    (kentos_commands::ROAD_TERRAINS_COVERAGE, kentos_commands::ROAD_TERRAINS_COVERAGE_VERSION),
    (kentos_commands::ROAD_TERRAINS_LOW_CONFIDENCE_DRAW, kentos_commands::ROAD_TERRAINS_LOW_CONFIDENCE_DRAW_VERSION),
    (kentos_commands::ROAD_TERRAINS_DENSITY, kentos_commands::ROAD_TERRAINS_DENSITY_VERSION),
    (kentos_commands::ROAD_TERRAINS_STOCKPILE, kentos_commands::ROAD_TERRAINS_STOCKPILE_VERSION),
    (kentos_commands::ROAD_TERRAINS_SPARSE_DRAW, kentos_commands::ROAD_TERRAINS_SPARSE_DRAW_VERSION),
    (kentos_commands::ROAD_PLATFORMS_QUERY, kentos_commands::ROAD_PLATFORMS_QUERY_VERSION),
    (kentos_commands::ROAD_PLATFORMS_DRAW, kentos_commands::ROAD_PLATFORMS_DRAW_VERSION),
    (kentos_commands::ROAD_PAY_EDIT, kentos_commands::ROAD_PAY_EDIT_VERSION),
    (kentos_commands::ROAD_PAY_QUERY, kentos_commands::ROAD_PAY_QUERY_VERSION),
    (kentos_commands::ROAD_ROUTES_FIND, kentos_commands::ROAD_ROUTES_FIND_VERSION),
    (kentos_commands::ROAD_ROUTES_DRAW, kentos_commands::ROAD_ROUTES_DRAW_VERSION),
    (kentos_commands::ROAD_JUNCTIONS_EDIT, kentos_commands::ROAD_JUNCTIONS_EDIT_VERSION),
    (kentos_commands::ROAD_JUNCTIONS_QUERY, kentos_commands::ROAD_JUNCTIONS_QUERY_VERSION),
    (kentos_commands::ROAD_JUNCTIONS_DRAW, kentos_commands::ROAD_JUNCTIONS_DRAW_VERSION),
    (kentos_commands::ROAD_JUNCTIONS_CLEARANCE, kentos_commands::ROAD_JUNCTIONS_CLEARANCE_VERSION),
    (kentos_commands::ROAD_JUNCTIONS_TIE, kentos_commands::ROAD_JUNCTIONS_TIE_VERSION),
    (kentos_commands::ROAD_JUNCTIONS_OVERLAP, kentos_commands::ROAD_JUNCTIONS_OVERLAP_VERSION),
    (kentos_commands::ROAD_PROJECT_QUERY, kentos_commands::ROAD_PROJECT_QUERY_VERSION),
    (kentos_commands::ROAD_PROJECT_GRAPH, kentos_commands::ROAD_PROJECT_GRAPH_VERSION),
    (kentos_commands::ROAD_PROJECT_REBUILD, kentos_commands::ROAD_PROJECT_REBUILD_VERSION),
    (kentos_commands::ROAD_PROJECT_PRODUCE, kentos_commands::ROAD_PROJECT_PRODUCE_VERSION),
    (kentos_commands::ROAD_PROJECT_EDIT, kentos_commands::ROAD_PROJECT_EDIT_VERSION),
    // KentOS Harita's work structure (plan/map-cad.md MAP-01, docs/adr/1200).
    (kentos_commands::MAP_WORK_QUERY, kentos_commands::MAP_WORK_QUERY_VERSION),
    (kentos_commands::MAP_WORK_EDIT, kentos_commands::MAP_WORK_EDIT_VERSION),
    // An approved item's objects put back as approved (plan/map-cad.md MAP-01, docs/adr/4080).
    (kentos_commands::MAP_WORK_RESTORE, kentos_commands::MAP_WORK_RESTORE_VERSION),
    // The scales between the ground, the ellipsoid and the map (plan/map-cad.md MAP-02, docs/adr/1231).
    (kentos_commands::CAD_CRS_FACTORS, kentos_commands::CAD_CRS_FACTORS_VERSION),
    // Kodla çiz (plan/map-cad.md MAP-04, docs/adr/1360).
    (kentos_commands::MAP_FEATURES_CODE, kentos_commands::MAP_FEATURES_CODE_VERSION),
    // GNSS baz ağı dengelemesi (plan/map-cad.md MAP-05, docs/adr/1380).
    (kentos_commands::CAD_SURVEY_BASELINES, kentos_commands::CAD_SURVEY_BASELINES_VERSION),
    // Eğimle kot ver (plan/map-cad.md MAP-06, docs/adr/1290).
    (kentos_commands::CAD_ENTITIES_GRADE, kentos_commands::CAD_ENTITIES_GRADE_VERSION),
    // Referanstan kot ver (plan/map-cad.md MAP-06, docs/adr/3780).
    (kentos_commands::CAD_ENTITIES_RAISE, kentos_commands::CAD_ENTITIES_RAISE_VERSION),
    // Yüzeyden kot ver (plan/map-cad.md MAP-06, docs/adr/4180).
    (kentos_commands::CAD_ENTITIES_DRAPE, kentos_commands::CAD_ENTITIES_DRAPE_VERSION),
    (kentos_commands::CAD_TOPOLOGY_CLEAN, kentos_commands::CAD_TOPOLOGY_CLEAN_VERSION),
    (kentos_commands::CAD_PARCELS_CHECK, kentos_commands::CAD_PARCELS_CHECK_VERSION),
    (kentos_commands::CAD_PARCELS_BOUNDARIES, kentos_commands::CAD_PARCELS_BOUNDARIES_VERSION),
    (kentos_commands::CAD_PARCELS_RECORD, kentos_commands::CAD_PARCELS_RECORD_VERSION),
    (kentos_commands::CAD_PARCELS_RECORDS, kentos_commands::CAD_PARCELS_RECORDS_VERSION),
    (kentos_commands::CAD_PARCELS_APPROVE, kentos_commands::CAD_PARCELS_APPROVE_VERSION),
    (kentos_commands::CAD_PARCELS_WITHDRAW, kentos_commands::CAD_PARCELS_WITHDRAW_VERSION),
    (kentos_commands::CAD_PIPES_CHECK, kentos_commands::CAD_PIPES_CHECK_VERSION),
    (kentos_commands::CAD_PIPES_PROFILE, kentos_commands::CAD_PIPES_PROFILE_VERSION),
    (kentos_commands::CAD_ASBUILT_CHECK, kentos_commands::CAD_ASBUILT_CHECK_VERSION),
    // İzleme incelemesi (plan/map-cad.md MAP-19, docs/adr/5220).
    (kentos_commands::CAD_ASBUILT_REVIEW, kentos_commands::CAD_ASBUILT_REVIEW_VERSION),
    (kentos_commands::CAD_FIELD_PACKAGE, kentos_commands::CAD_FIELD_PACKAGE_VERSION),
    (kentos_commands::CAD_ZONING_CHECK, kentos_commands::CAD_ZONING_CHECK_VERSION),
    (kentos_commands::CAD_ZONING_SYMBOLIZE, kentos_commands::CAD_ZONING_SYMBOLIZE_VERSION),
    (kentos_commands::CAD_ZONING_DRAFT, kentos_commands::CAD_ZONING_DRAFT_VERSION),
    (kentos_commands::CAD_SCENARIOS_IMPACT, kentos_commands::CAD_SCENARIOS_IMPACT_VERSION),
    (kentos_commands::CAD_SCENARIOS_RECORD, kentos_commands::CAD_SCENARIOS_RECORD_VERSION),
    (kentos_commands::CAD_READJUSTMENT_DISTRIBUTE, kentos_commands::CAD_READJUSTMENT_DISTRIBUTE_VERSION),
    (kentos_commands::CAD_READJUSTMENT_SCOPE, kentos_commands::CAD_READJUSTMENT_SCOPE_VERSION),
    (kentos_commands::CAD_READJUSTMENT_SHEETS, kentos_commands::CAD_READJUSTMENT_SHEETS_VERSION),
    (kentos_commands::CAD_ORTHOPHOTO_CHECK, kentos_commands::CAD_ORTHOPHOTO_CHECK_VERSION),
    (kentos_commands::CAD_ORTHOPHOTO_COMPARE, kentos_commands::CAD_ORTHOPHOTO_COMPARE_VERSION),
    (kentos_commands::CAD_ORTHOPHOTO_SEAMLINES, kentos_commands::CAD_ORTHOPHOTO_SEAMLINES_VERSION),
    // Birleşme çizgisi ağı ve ayak izi (plan/map-cad.md MAP-12, docs/adr/5260).
    (kentos_commands::CAD_ORTHOPHOTO_NETWORK, kentos_commands::CAD_ORTHOPHOTO_NETWORK_VERSION),
    (kentos_commands::CAD_ORTHOPHOTO_FOOTPRINT, kentos_commands::CAD_ORTHOPHOTO_FOOTPRINT_VERSION),
    (kentos_commands::CAD_POINTCLOUDS_OVERLAP, kentos_commands::CAD_POINTCLOUDS_OVERLAP_VERSION),
    (kentos_commands::CAD_POINTCLOUDS_REGISTER, kentos_commands::CAD_POINTCLOUDS_REGISTER_VERSION),
    (kentos_commands::CAD_POINTCLOUDS_NETWORK, kentos_commands::CAD_POINTCLOUDS_NETWORK_VERSION),
    (kentos_commands::CAD_POINTCLOUDS_PLACE, kentos_commands::CAD_POINTCLOUDS_PLACE_VERSION),
    // The catalog's migration report (plan/map-cad.md MAP-04, docs/adr/4940).
    (kentos_commands::CAD_PROFILES_COMPARE, kentos_commands::CAD_PROFILES_COMPARE_VERSION),
    (kentos_commands::CAD_PROFILES_CARRY, kentos_commands::CAD_PROFILES_CARRY_VERSION),
    // KentOS Harita's GNSS campaign archive (plan/map-cad.md MAP-03, docs/adr/1340).
    (
        kentos_commands::MAP_SURVEY_QUERY,
        kentos_commands::MAP_SURVEY_QUERY_VERSION,
    ),
    (
        kentos_commands::MAP_SURVEY_EDIT,
        kentos_commands::MAP_SURVEY_EDIT_VERSION,
    ),
    (
        kentos_commands::MAP_SURVEY_NETWORK,
        kentos_commands::MAP_SURVEY_NETWORK_VERSION,
    ),
    (
        kentos_commands::MAP_FEATURES_RECONCILE,
        kentos_commands::MAP_FEATURES_RECONCILE_VERSION,
    ),
    (
        kentos_commands::ROAD_STAKEOUT_QUERY,
        kentos_commands::ROAD_STAKEOUT_QUERY_VERSION,
    ),
    (
        kentos_commands::FILE_EXPORT_LANDXML,
        kentos_commands::FILE_EXPORT_LANDXML_VERSION,
    ),
    (
        kentos_commands::FILE_EXPORT_IFC,
        kentos_commands::FILE_EXPORT_IFC_VERSION,
    ),
    (
        kentos_commands::CAD_ARCH_IFC,
        kentos_commands::CAD_ARCH_IFC_VERSION,
    ),
    (
        kentos_commands::FILE_EXPORT_GLTF,
        kentos_commands::FILE_EXPORT_GLTF_VERSION,
    ),
    (
        kentos_commands::ROAD_STAKEOUT_SEND,
        kentos_commands::ROAD_STAKEOUT_SEND_VERSION,
    ),
    (
        kentos_commands::ROAD_SHEETS_PLAN,
        kentos_commands::ROAD_SHEETS_PLAN_VERSION,
    ),
    (
        kentos_commands::ROAD_SHEETS_SECTIONS,
        kentos_commands::ROAD_SHEETS_SECTIONS_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_FIT,
        kentos_commands::ROAD_ALIGNMENTS_FIT_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_FIT,
        kentos_commands::ROAD_PROFILES_FIT_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_IMPORT,
        kentos_commands::ROAD_ALIGNMENTS_IMPORT_VERSION,
    ),
    // Alt ağlar ([`subnetworks`], [`subnetworks_update`], docs/adr/0557 §2).
    (
        kentos_commands::CAD_SUBNETWORKS_FIND,
        kentos_commands::CAD_SUBNETWORKS_FIND_VERSION,
    ),
    (
        kentos_commands::CAD_SUBNETWORKS_UPDATE,
        kentos_commands::CAD_SUBNETWORKS_UPDATE_VERSION,
    ),
    // Topolojiyi denetle ve istisnalar ([`topology_check`], [`topology_exceptions`], docs/adr/0589).
    (
        kentos_commands::CAD_TOPOLOGY_CHECK,
        kentos_commands::CAD_TOPOLOGY_CHECK_VERSION,
    ),
    (
        kentos_commands::CAD_TOPOLOGY_EXCEPTIONS,
        kentos_commands::CAD_TOPOLOGY_EXCEPTIONS_VERSION,
    ),
    // Topoloji onarım tarifi ([`topology_repair`], docs/adr/1260 §4).
    (
        kentos_commands::CAD_TOPOLOGY_REPAIR,
        kentos_commands::CAD_TOPOLOGY_REPAIR_VERSION,
    ),
    // Adres verisini denetle ([`addresses_check`], docs/adr/0579).
    (
        kentos_commands::CAD_ADDRESSES_CHECK,
        kentos_commands::CAD_ADDRESSES_CHECK_VERSION,
    ),
    // Adres kaydıyla karşılaştır ([`addresses_compare`], docs/adr/0736).
    (
        kentos_commands::CAD_ADDRESSES_COMPARE,
        kentos_commands::CAD_ADDRESSES_COMPARE_VERSION,
    ),
    // Adres yazılışlarını düzelt ([`addresses_normalize`], docs/adr/0647 §2).
    (
        kentos_commands::CAD_ADDRESSES_NORMALIZE,
        kentos_commands::CAD_ADDRESSES_NORMALIZE_VERSION,
    ),
    // Adres verisi ([`address_define`], docs/adr/0229 §14).
    (
        kentos_commands::CAD_ADDRESSES_DEFINE,
        kentos_commands::CAD_ADDRESSES_DEFINE_VERSION,
    ),
    // Zaman ayarları and the scenarios ([`layers_time`], [`scenarios_edit`], docs/adr/0210 §11).
    (
        kentos_commands::CAD_LAYERS_TIME,
        kentos_commands::CAD_LAYERS_TIME_VERSION,
    ),
    (
        kentos_commands::CAD_SCENARIOS_EDIT,
        kentos_commands::CAD_SCENARIOS_EDIT_VERSION,
    ),
    // Katman süzgeci ([`layers_filter`], docs/adr/0211 §5).
    (
        kentos_commands::CAD_LAYERS_FILTER,
        kentos_commands::CAD_LAYERS_FILTER_VERSION,
    ),
    // Katman çizimi ([`layers_compose`], docs/adr/0226 §3, §4).
    (
        kentos_commands::CAD_LAYERS_COMPOSE,
        kentos_commands::CAD_LAYERS_COMPOSE_VERSION,
    ),
    // Bilgi kartı ve eylemler ([`layers_info`], docs/adr/0225 §4, §5).
    (
        kentos_commands::CAD_LAYERS_INFO,
        kentos_commands::CAD_LAYERS_INFO_VERSION,
    ),
    // Etiketler and the label tools ([`layers_labels`], [`labels_pin`], docs/adr/0212 §5).
    (
        kentos_commands::CAD_LAYERS_LABELS,
        kentos_commands::CAD_LAYERS_LABELS_VERSION,
    ),
    (
        kentos_commands::CAD_LABELS_PIN,
        kentos_commands::CAD_LABELS_PIN_VERSION,
    ),
    // Etiketleri denetle ([`labels_report`], docs/adr/0533 §6).
    (
        kentos_commands::CAD_LABELS_REPORT,
        kentos_commands::CAD_LABELS_REPORT_VERSION,
    ),
    // Yeniden bağla ([`dimensions_reassociate`], docs/adr/0249 §3).
    (
        kentos_commands::CAD_DIMENSIONS_REASSOCIATE,
        kentos_commands::CAD_DIMENSIONS_REASSOCIATE_VERSION,
    ),
    // Ölçülendirme by what it measures ([`dimensions_create`], docs/adr/0255).
    (
        kentos_commands::CAD_DIMENSIONS_CREATE,
        kentos_commands::CAD_DIMENSIONS_CREATE_VERSION,
    ),
    // İç ölçü zinciri ([`dimensions_interior`], docs/adr/0711).
    (
        kentos_commands::CAD_DIMENSIONS_INTERIOR,
        kentos_commands::CAD_DIMENSIONS_INTERIOR_VERSION,
    ),
    // A dimension's spacing, breaks, moved value, written value and options ([`dimensions_edit`], docs/adr/0321 §4).
    (
        kentos_commands::CAD_DIMENSIONS_EDIT,
        kentos_commands::CAD_DIMENSIONS_EDIT_VERSION,
    ),
    // What dimensions measure and show ([`dimensions_query`], docs/adr/0321 §3).
    (
        kentos_commands::CAD_DIMENSIONS_QUERY,
        kentos_commands::CAD_DIMENSIONS_QUERY_VERSION,
    ),
    // Buda, Uzat, Köşe yuvarla and Pah by their meaning (docs/adr/0250).
    (
        kentos_commands::CAD_ENTITIES_TRIM,
        kentos_commands::CAD_ENTITIES_TRIM_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_EXTEND,
        kentos_commands::CAD_ENTITIES_EXTEND_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_FILLET,
        kentos_commands::CAD_ENTITIES_FILLET_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_CHAMFER,
        kentos_commands::CAD_ENTITIES_CHAMFER_VERSION,
    ),
    // Esnet, Kır, Birleştir, Böl, Alan oluştur and Tarama by their meaning (docs/adr/0254).
    (
        kentos_commands::CAD_ENTITIES_STRETCH,
        kentos_commands::CAD_ENTITIES_STRETCH_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_BREAK,
        kentos_commands::CAD_ENTITIES_BREAK_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_JOIN,
        kentos_commands::CAD_ENTITIES_JOIN_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_REVERSE,
        kentos_commands::CAD_ENTITIES_REVERSE_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_LENGTHEN,
        kentos_commands::CAD_ENTITIES_LENGTHEN_VERSION,
    ),
    (kentos_commands::CAD_DRAWING_INSPECT, kentos_commands::CAD_DRAWING_INSPECT_VERSION),
    (kentos_commands::CAD_ARRAYS_FIND, kentos_commands::CAD_ARRAYS_FIND_VERSION),
    (
        kentos_commands::CAD_ARRAYS_ADOPT,
        kentos_commands::CAD_ARRAYS_ADOPT_VERSION,
    ),
    (
        kentos_commands::CAD_ARRAYS_EDIT,
        kentos_commands::CAD_ARRAYS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_CONSTRAINTS_EDIT,
        kentos_commands::CAD_CONSTRAINTS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_CONSTRAINTS_QUERY,
        kentos_commands::CAD_CONSTRAINTS_QUERY_VERSION,
    ),
    (
        kentos_commands::CAD_PARAMETERS_EDIT,
        kentos_commands::CAD_PARAMETERS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_EXPLODE,
        kentos_commands::CAD_ENTITIES_EXPLODE_VERSION,
    ),
    (
        kentos_commands::CAD_CURVES_EDIT,
        kentos_commands::CAD_CURVES_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_MATCH,
        kentos_commands::CAD_ENTITIES_MATCH_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_CLEANUP,
        kentos_commands::CAD_ENTITIES_CLEANUP_VERSION,
    ),
    (
        kentos_commands::CAD_DRAWING_AUDIT,
        kentos_commands::CAD_DRAWING_AUDIT_VERSION,
    ),
    // İki revizyonu karşılaştır ([`drawing_compare`], docs/adr/0576).
    (
        kentos_commands::CAD_DRAWING_COMPARE,
        kentos_commands::CAD_DRAWING_COMPARE_VERSION,
    ),
    (
        kentos_commands::CAD_DRAWING_TAKE,
        kentos_commands::CAD_DRAWING_TAKE_VERSION,
    ),
    (
        kentos_commands::CAD_DRAWING_REPAIR,
        kentos_commands::CAD_DRAWING_REPAIR_VERSION,
    ),
    (
        kentos_commands::CAD_REVIEW_NOTES_EDIT,
        kentos_commands::CAD_REVIEW_NOTES_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_REVIEW_NOTES_LIST,
        kentos_commands::CAD_REVIEW_NOTES_LIST_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_COPY,
        kentos_commands::CAD_ENTITIES_COPY_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_PASTE,
        kentos_commands::CAD_ENTITIES_PASTE_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_DIVIDE,
        kentos_commands::CAD_ENTITIES_DIVIDE_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_BOUNDARY,
        kentos_commands::CAD_ENTITIES_BOUNDARY_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_HATCH,
        kentos_commands::CAD_ENTITIES_HATCH_VERSION,
    ),
    // Katman stili's renderers ([`layers_renderer`], docs/adr/0213 §5).
    (
        kentos_commands::CAD_LAYERS_RENDERER,
        kentos_commands::CAD_LAYERS_RENDERER_VERSION,
    ),
    // The layer tree, its own changes and its states, the text and dimension
    // styles, the project's settings ([`layers_edit`], [`layers_set`],
    // [`layers_states`], [`styles_edit`], [`settings_set`], docs/adr/0248).
    (
        kentos_commands::CAD_LAYERS_EDIT,
        kentos_commands::CAD_LAYERS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_SET,
        kentos_commands::CAD_LAYERS_SET_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_STATES,
        kentos_commands::CAD_LAYERS_STATES_VERSION,
    ),
    // Where a layer's field is named ([`fields_usage`], docs/adr/0414).
    (
        kentos_commands::CAD_FIELDS_USAGE,
        kentos_commands::CAD_FIELDS_USAGE_VERSION,
    ),
    // The layers found by a name pattern and their properties, and layers
    // translated to a standard's ([`layers_find`], [`layers_translate`], docs/adr/0311).
    (
        kentos_commands::CAD_LAYERS_FIND,
        kentos_commands::CAD_LAYERS_FIND_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_TRANSLATE,
        kentos_commands::CAD_LAYERS_TRANSLATE_VERSION,
    ),
    // A drawing standard checked and applied ([`standards`], docs/adr/0312).
    (
        kentos_commands::CAD_STANDARDS_CHECK,
        kentos_commands::CAD_STANDARDS_CHECK_VERSION,
    ),
    (
        kentos_commands::CAD_STANDARDS_CONFLICTS,
        kentos_commands::CAD_STANDARDS_CONFLICTS_VERSION,
    ),
    (
        kentos_commands::CAD_STANDARDS_INSTALL,
        kentos_commands::CAD_STANDARDS_INSTALL_VERSION,
    ),
    // Kabul edilmiş standart bulguları ([`standards_exceptions`], docs/adr/1129).
    (
        kentos_commands::CAD_STANDARDS_EXCEPTIONS,
        kentos_commands::CAD_STANDARDS_EXCEPTIONS_VERSION,
    ),
    (
        kentos_commands::CAD_STANDARDS_RESOLVE,
        kentos_commands::CAD_STANDARDS_RESOLVE_VERSION,
    ),
    (
        kentos_commands::CAD_STANDARDS_PALETTES,
        kentos_commands::CAD_STANDARDS_PALETTES_VERSION,
    ),
    (
        kentos_commands::CAD_STANDARDS_APPLY,
        kentos_commands::CAD_STANDARDS_APPLY_VERSION,
    ),
    // A data profile applied and checked ([`profiles`], docs/adr/0541).
    (
        kentos_commands::CAD_PROFILES_APPLY,
        kentos_commands::CAD_PROFILES_APPLY_VERSION,
    ),
    (
        kentos_commands::CAD_PROFILES_CHECK,
        kentos_commands::CAD_PROFILES_CHECK_VERSION,
    ),
    // An institution's own profile imported, listed, migrated and taken out of the drawing ([`profile_files`],
    // docs/adr/0558, 0735).
    (
        kentos_commands::CAD_PROFILES_IMPORT,
        kentos_commands::CAD_PROFILES_IMPORT_VERSION,
    ),
    (
        kentos_commands::CAD_PROFILES_LIST,
        kentos_commands::CAD_PROFILES_LIST_VERSION,
    ),
    (
        kentos_commands::CAD_PROFILES_MIGRATE,
        kentos_commands::CAD_PROFILES_MIGRATE_VERSION,
    ),
    (
        kentos_commands::CAD_PROFILES_REMOVE,
        kentos_commands::CAD_PROFILES_REMOVE_VERSION,
    ),
    // The fields' names in an institution by a mapping's form of names ([`profiles_names`], docs/adr/5180).
    (
        kentos_commands::CAD_PROFILES_NAMES,
        kentos_commands::CAD_PROFILES_NAMES_VERSION,
    ),
    // The project's line types with patterns ([`line_types_edit`], docs/adr/0313).
    (
        kentos_commands::CAD_LINE_TYPES_EDIT,
        kentos_commands::CAD_LINE_TYPES_EDIT_VERSION,
    ),
    // The project's own hatch patterns ([`hatch_patterns_edit`], docs/adr/0331 §1).
    (
        kentos_commands::CAD_HATCH_PATTERNS_EDIT,
        kentos_commands::CAD_HATCH_PATTERNS_EDIT_VERSION,
    ),
    // Blocks from a drawing file ([`file_import_block`], docs/adr/0316 §3).
    (
        kentos_commands::FILE_IMPORT_BLOCK,
        kentos_commands::FILE_IMPORT_BLOCK_VERSION,
    ),
    // A block placed ([`blocks_insert`], docs/adr/0315 §4).
    (
        kentos_commands::CAD_BLOCKS_INSERT,
        kentos_commands::CAD_BLOCKS_INSERT_VERSION,
    ),
    // A block's inserts placing another, and the inserts counted, found and scheduled ([`blocks_replace`],
    // [`blocks_query`], docs/adr/0317 §3).
    (
        kentos_commands::CAD_BLOCKS_REPLACE,
        kentos_commands::CAD_BLOCKS_REPLACE_VERSION,
    ),
    (
        kentos_commands::CAD_BLOCKS_QUERY,
        kentos_commands::CAD_BLOCKS_QUERY_VERSION,
    ),
    // A dynamic block's inserts' parameter values ([`block_params`], docs/adr/0318 §3).
    (kentos_commands::CAD_BLOCKS_FIND, kentos_commands::CAD_BLOCKS_FIND_VERSION),
    (kentos_commands::CAD_BLOCKS_CONVERT, kentos_commands::CAD_BLOCKS_CONVERT_VERSION),
    (
        kentos_commands::CAD_BLOCKS_PARAMETERS,
        kentos_commands::CAD_BLOCKS_PARAMETERS_VERSION,
    ),
    // What an insert draws, and a dynamic block's inserts made plain ([`block_variants`], docs/adr/0319 §4).
    (
        kentos_commands::CAD_BLOCKS_PROXY,
        kentos_commands::CAD_BLOCKS_PROXY_VERSION,
    ),
    (
        kentos_commands::CAD_BLOCKS_FREEZE,
        kentos_commands::CAD_BLOCKS_FREEZE_VERSION,
    ),
    // The project's style library and a change's reach ([`library_edit`], [`styles_impact`], docs/adr/0314).
    (
        kentos_commands::CAD_LIBRARY_EDIT,
        kentos_commands::CAD_LIBRARY_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_STYLES_IMPACT,
        kentos_commands::CAD_STYLES_IMPACT_VERSION,
    ),
    (
        kentos_commands::CAD_STYLES_EDIT,
        kentos_commands::CAD_STYLES_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_SETTINGS_SET,
        kentos_commands::CAD_SETTINGS_SET_VERSION,
    ),
    // A local project's unit, keeping the sizes or the numbers ([`units_change`], docs/adr/0256 §1).
    (
        kentos_commands::CAD_UNITS_CHANGE,
        kentos_commands::CAD_UNITS_CHANGE_VERSION,
    ),
    // A DXF into the drawing, its unit decided by one rule ([`file_import`], docs/adr/0257).
    (
        kentos_commands::FILE_IMPORT_DXF,
        kentos_commands::FILE_IMPORT_DXF_VERSION,
    ),
    (kentos_commands::FILE_IMPORT_DWG, kentos_commands::FILE_IMPORT_DWG_VERSION),
    // Toplu yazma of the attribute table ([`fields_write`], docs/adr/0507).
    (
        kentos_commands::CAD_FIELDS_WRITE,
        kentos_commands::CAD_FIELDS_WRITE_VERSION,
    ),
    (
        kentos_commands::CAD_PROCESSING_TOOLS,
        kentos_commands::CAD_PROCESSING_TOOLS_VERSION,
    ),
    (
        kentos_commands::CAD_PROCESSING_RUN,
        kentos_commands::CAD_PROCESSING_RUN_VERSION,
    ),
    // İşlemler's tool or model over a table of rows (docs/adr/0529).
    (
        kentos_commands::CAD_PROCESSING_BATCH,
        kentos_commands::CAD_PROCESSING_BATCH_VERSION,
    ),
    // Bound layers ([`sources`], docs/adr/0503).
    (
        kentos_commands::CAD_SOURCES_DESCRIBE,
        kentos_commands::CAD_SOURCES_DESCRIBE_VERSION,
    ),
    (
        kentos_commands::CAD_SOURCES_EDIT,
        kentos_commands::CAD_SOURCES_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_SOURCES_SYNC,
        kentos_commands::CAD_SOURCES_SYNC_VERSION,
    ),
    // Both sides of a bound layer changed ([`sources::reconcile`], docs/adr/0538).
    (
        kentos_commands::CAD_SOURCES_RECONCILE,
        kentos_commands::CAD_SOURCES_RECONCILE_VERSION,
    ),
    // A many-to-many reference in a bound GeoPackage ([`sources::relate`], docs/adr/0532).
    (
        kentos_commands::CAD_SOURCES_RELATE,
        kentos_commands::CAD_SOURCES_RELATE_VERSION,
    ),
    // A layer's records, or a relation's, a page at a time ([`records_page`], docs/adr/0532).
    (
        kentos_commands::CAD_RECORDS_PAGE,
        kentos_commands::CAD_RECORDS_PAGE_VERSION,
    ),
    // A layer's source system and its objects moved from it ([`layers_crs`], docs/adr/0583).
    (
        kentos_commands::CAD_LAYERS_CRS,
        kentos_commands::CAD_LAYERS_CRS_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_REPROJECT,
        kentos_commands::CAD_LAYERS_REPROJECT_VERSION,
    ),
    // A data set asked where it is: the box, the columns and the filter sent to the source ([`sources_query`], docs/adr/0581).
    (
        kentos_commands::CAD_SOURCES_QUERY,
        kentos_commands::CAD_SOURCES_QUERY_VERSION,
    ),
    // A GeoPackage into the drawing ([`file_import_gpkg`], docs/adr/0502).
    (
        kentos_commands::FILE_IMPORT_GPKG,
        kentos_commands::FILE_IMPORT_GPKG_VERSION,
    ),
    // A CSV's rows into the drawing ([`file_import_csv`], docs/adr/0522).
    (
        kentos_commands::FILE_IMPORT_CSV,
        kentos_commands::FILE_IMPORT_CSV_VERSION,
    ),
    // A FlatGeobuf's features into the drawing ([`file_import_fgb`], docs/adr/0544).
    (
        kentos_commands::FILE_IMPORT_FGB,
        kentos_commands::FILE_IMPORT_FGB_VERSION,
    ),
    // A GeoParquet's rows into the drawing ([`file_import_parquet`], docs/adr/0628).
    (
        kentos_commands::FILE_IMPORT_PARQUET,
        kentos_commands::FILE_IMPORT_PARQUET_VERSION,
    ),
    // A GML's features into the drawing ([`file_import_gml`], docs/adr/0545).
    (
        kentos_commands::FILE_IMPORT_GML,
        kentos_commands::FILE_IMPORT_GML_VERSION,
    ),
    // A Shapefile, a GeoJSON file and a KML/KMZ into the drawing ([`file_import_gis`], docs/adr/0563).
    (
        kentos_commands::FILE_IMPORT_SHP,
        kentos_commands::FILE_IMPORT_SHP_VERSION,
    ),
    (
        kentos_commands::FILE_IMPORT_GEOJSON,
        kentos_commands::FILE_IMPORT_GEOJSON_VERSION,
    ),
    (
        kentos_commands::FILE_IMPORT_KML,
        kentos_commands::FILE_IMPORT_KML_VERSION,
    ),
    // A derived GIS dataset against the drawing ([`gis_derivation`], docs/adr/0530).
    (
        kentos_commands::CAD_GIS_DERIVATION,
        kentos_commands::CAD_GIS_DERIVATION_VERSION,
    ),
    // What a GIS changed in a derived dataset written back ([`gis_takeback`], docs/adr/0634 §1).
    (
        kentos_commands::CAD_GIS_TAKEBACK,
        kentos_commands::CAD_GIS_TAKEBACK_VERSION,
    ),
    // A derived table made the GIS's own data ([`gis_detach`], docs/adr/0634 §2).
    (
        kentos_commands::CAD_GIS_DETACH,
        kentos_commands::CAD_GIS_DETACH_VERSION,
    ),
    // An SLD's look made a layer's ([`sld`], docs/adr/0516).
    (
        kentos_commands::FILE_IMPORT_SLD,
        kentos_commands::FILE_IMPORT_SLD_VERSION,
    ),
    // A QML's look made a layer's ([`qml`], docs/adr/0555).
    (
        kentos_commands::FILE_IMPORT_QML,
        kentos_commands::FILE_IMPORT_QML_VERSION,
    ),
    // A MapLibre style's look made the layers' ([`maplibre`], docs/adr/0560).
    (
        kentos_commands::FILE_IMPORT_MAPLIBRE,
        kentos_commands::FILE_IMPORT_MAPLIBRE_VERSION,
    ),
    // A PDF page into the drawing ([`file_import_pdf`], docs/adr/0352).
    (
        kentos_commands::FILE_IMPORT_PDF,
        kentos_commands::FILE_IMPORT_PDF_VERSION,
    ),
    (kentos_commands::CAD_SHEETS_CREATE, kentos_commands::CAD_SHEETS_CREATE_VERSION),
    (kentos_commands::CAD_SHEETS_FIT, kentos_commands::CAD_SHEETS_FIT_VERSION),
    // Kat planı paftaları ([`sheets_levels`], docs/adr/0718).
    (kentos_commands::CAD_SHEETS_LEVELS, kentos_commands::CAD_SHEETS_LEVELS_VERSION),
    // Detay paftaları ([`sheets_details`], docs/adr/1090 §2).
    (kentos_commands::CAD_SHEETS_DETAILS, kentos_commands::CAD_SHEETS_DETAILS_VERSION),
    // Baskı stili tablosu içe aktar ([`plot_styles_import`], docs/adr/1130).
    (kentos_commands::CAD_PLOTSTYLES_IMPORT, kentos_commands::CAD_PLOTSTYLES_IMPORT_VERSION),
    // Pafta kitabı ([`sheets_edit`], docs/adr/0252 §2).
    (
        kentos_commands::CAD_SHEETS_EDIT,
        kentos_commands::CAD_SHEETS_EDIT_VERSION,
    ),
    // The change set: several commands as one change ([`changeset`], docs/adr/0253 §8).
    (
        kentos_commands::CAD_CHANGESET_APPLY,
        kentos_commands::CAD_CHANGESET_APPLY_VERSION,
    ),
];

/// Opening external CAD files into a new native drawing.
#[path = "core/cad_open.rs"]
pub mod cad_open;
/// The drawing scale an opened CAD drawing takes from its texts (docs/adr/0626 §5).
#[path = "core/opened_scale.rs"]
pub mod opened_scale;
