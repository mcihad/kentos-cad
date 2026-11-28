//! The application crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.
// A build without every module (docs/adr/8054 §11) leaves the left-out disciplines' imports and helpers unused.
#![cfg_attr(
    not(all(
        feature = "mod-arch",
        feature = "mod-map",
        feature = "mod-gis",
        feature = "mod-road"
    )),
    allow(unused_imports, dead_code, unused_macros)
)]

#[cfg(feature = "mod-arch")]
mod beam_quantities;
#[cfg(feature = "mod-arch")]
mod delivery_limit;
mod element_types;
mod frame_joins;
#[cfg(feature = "mod-arch")]
mod ifc_revisions;
#[cfg(feature = "mod-arch")]
mod ifc_types;
#[cfg(feature = "mod-arch")]
mod railing_audit;
#[cfg(feature = "mod-arch")]
mod underlay_check;
// Runs shafts and rooms through the registry by name (docs/adr/8054 §11).
#[cfg(feature = "mod-arch")]
mod elevator_audit;
// Runs parking spaces through the registry by name (docs/adr/8054 §11).
#[cfg(feature = "mod-arch")]
mod parking_audit;
// Runs rooms through the registry by name (docs/adr/8054 §11).
#[cfg(feature = "mod-arch")]
mod arch_ifc;
#[cfg(feature = "mod-arch")]
mod blocks_areas;
mod color_plan;
#[cfg(feature = "mod-arch")]
mod dxf_rooms;
#[cfg(feature = "mod-arch")]
mod ifc_place;
mod ifc_required;
mod masses;
#[cfg(feature = "mod-arch")]
mod roof_sections;
#[cfg(feature = "mod-arch")]
mod sections;
mod shadows;
mod site_ground;
// Runs its plans are read into walls through the registry by name (docs/adr/8054 §11).
#[cfg(feature = "mod-arch")]
mod arch_recognize;
// Runs a building's walls and rooms through the registry by name (docs/adr/8054 §11).
mod blocks_find;
#[cfg(feature = "mod-arch")]
#[cfg(feature = "mod-arch")]
mod facades;
#[cfg(feature = "mod-map")]
mod pipes_profile_stop;
mod review_take;
// Runs map features reconciled through the registry by name (docs/adr/8054 §11).
#[cfg(feature = "mod-gis")]
mod address_spellings;
#[cfg(feature = "mod-gis")]
mod addresses_compare;
#[cfg(feature = "mod-arch")]
mod arch_bond_timing;
mod array_assoc;
mod arrays_adopt;
mod arrays_find;
mod audit_cmd;
mod block_formula_values;
mod block_nested_attribute_renames;
mod block_type_values;
mod block_unit_rescale;
mod blocks_convert;
mod break_join_ties;
mod cad_sheet;
mod capabilities;
mod catalog;
mod change_effects;
mod cleanup_cmd;
mod clipboard_cmd;
#[cfg(feature = "mod-road")]
mod design_speeds;
mod dimension_set_copy;
#[cfg(feature = "mod-road")]
mod gltf_files;
mod gpkg_field_rules;
mod gpkg_tiles_files;
mod gpkg_tiles_timing;
#[cfg(feature = "mod-road")]
mod ifc_files;
mod import_robust;
#[cfg(feature = "mod-road")]
mod landxml_files;
mod profile_files;
#[cfg(feature = "mod-map")]
mod reconcile_large;
mod sheet_tools;
#[cfg(feature = "mod-road")]
mod stakeout_send_files;
#[cfg(feature = "mod-road")]
mod superelevation_reverse;
// Runs walls and openings through the registry by name (docs/adr/8054 §11).
#[cfg(feature = "mod-arch")]
mod dimension_sets;
// Its house is drawn with walls (`wall_contours::drawing`).
#[cfg(feature = "mod-arch")]
mod dimension_layout;
// Its house is drawn with walls (`wall_contours::drawing`).
mod cog;
mod compare_timing;
mod create;
mod curves_edit;
mod delete;
#[cfg(feature = "mod-arch")]
mod dimension_spans;
mod divide;
mod edit;
mod entries;
mod extract_timing;
// The shared cases run in every build; a left-out discipline's are skipped there (docs/adr/8054 §11).
#[cfg(feature = "mod-gis")]
mod dataset_timing;
mod disciplines;
mod fixtures;
#[cfg(feature = "mod-gis")]
mod forms_timing;
mod gml;
mod gml_timing;
mod info_templates;
#[cfg(feature = "mod-gis")]
mod join_source_timing;
#[cfg(feature = "mod-gis")]
mod kinds_timing;
mod labels_report_draft;
mod layer_digest;
mod layer_indexes;
mod layer_types;
#[cfg(feature = "mod-gis")]
mod layers_find_cmd;
mod lengthen;
mod library_files;
mod match_props;
mod module_features;
#[cfg(feature = "mod-gis")]
mod mosaic_files;
mod object_policy;
mod object_source_timing;
mod order_cmd;
#[cfg(feature = "mod-gis")]
mod parquet_timing;
mod polygon;
mod processing_sink;
mod processing_sink_faults;
mod protocol;
mod protocol_cases;
mod provider_timing;
mod python_calls;
mod rasters_georef;
mod reassociate;
mod reconcile;
#[cfg(feature = "mod-gis")]
mod reproject_files;
#[cfg(feature = "mod-road")]
mod road_plan_axes;
#[cfg(feature = "mod-road")]
mod road_view_scales;
#[cfg(feature = "mod-gis")]
mod sensors_timing;
mod source_relations;
mod sources;
#[cfg(feature = "mod-gis")]
mod sources_query;
mod standards_cmd;
mod stretch_ties;
mod tables_cmd;
#[cfg(feature = "mod-arch")]
mod templates_safe;
#[cfg(feature = "mod-road")]
mod terrain_repair;
mod transform;
mod units;
#[cfg(feature = "mod-arch")]
mod wall_follow;
// Runs walls through the registry by name (docs/adr/8054 §11).
#[cfg(feature = "mod-arch")]
mod opening_placement;
#[cfg(feature = "mod-arch")]
mod wall_style_ties;
#[cfg(feature = "mod-arch")]
mod walls_partition;
#[cfg(feature = "mod-arch")]
mod walls_rectangle;
mod xrefs;

mod viewport_identity;

mod drawing_inspect;
mod drawing_set;

#[cfg(feature = "mod-arch")]
mod opening_array;
#[cfg(feature = "mod-arch")]
mod opening_types_timing;

#[cfg(feature = "mod-arch")]
mod wall_contours;

#[cfg(feature = "mod-arch")]
mod quantities_zero;
#[cfg(feature = "mod-arch")]
mod space_columns;
#[cfg(feature = "mod-arch")]
mod space_levels;
#[cfg(feature = "mod-arch")]
mod space_scope;
#[cfg(feature = "mod-arch")]
mod space_separators;
#[cfg(feature = "mod-arch")]
mod space_sources;
#[cfg(feature = "mod-arch")]
mod spaces_discover;
#[cfg(feature = "mod-arch")]
mod wall_contour_mixed;

#[cfg(feature = "mod-arch")]
mod spaces_number;

#[cfg(feature = "mod-arch")]
mod grid_forms;
#[cfg(feature = "mod-arch")]
mod tag_refresh;

mod curve_extend;

mod curve_ties;

mod area_curve_ties;

mod ellipse_constraints;

mod insert_constraints;
mod nurbs_constraints;

mod arc_edge_constraints;
mod parameter_dimensions;

mod office_profiles;
mod table_style_dxf;
mod text_bulk;

// Runs grids, walls and columns through the registry by name (docs/adr/8054 §11).
#[cfg(feature = "mod-arch")]
mod grid_bonds;

// Runs columns and walls through the registry by name (docs/adr/8054 §11).
#[cfg(feature = "mod-arch")]
mod column_bonds;

mod arc_bonds;

// Runs modules of walls and slabs through the registry by name (docs/adr/8054 §11).
#[cfg(feature = "mod-arch")]
mod module_kinds;

// A GeoPackage opened as a project (docs/adr/9116).
#[cfg(feature = "mod-gis")]
mod gpkg_open;
mod topology_sources;
mod write_receipts;
// A data layer's field counted by its source for İstatistik and Grafik (docs/adr/9181).
#[cfg(feature = "mod-gis")]
mod field_counts;
// A data layer's box and outline as its source has them now (docs/adr/9327).
mod data_extent;
// A condition naming a field the data layer lacks (docs/adr/9132).
mod layers_rows_fields;
// A data layer's columns added and its shape read again where a shared case cannot go (docs/adr/9318).
#[cfg(feature = "mod-gis")]
mod layers_columns;
// A relation by a parent key through the commands (docs/adr/9135).
mod relation_keys;
// A tool's table written with its columns' kinds (docs/adr/9136).
mod processing_typed_tables;
