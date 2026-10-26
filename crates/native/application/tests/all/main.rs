//! The application crate's integration tests as one binary: every module is a test
//! file of its own, the ones run alone (by `--test`) beside this folder.
//! One binary links once where each file was linked on its own.

mod array_assoc;
mod audit_cmd;
mod gpkg_tiles_files;
mod import_robust;
mod profile_files;
mod break_join_ties;
mod capabilities;
mod cad_sheet;
mod sheet_tools;
mod catalog;
mod change_effects;
mod cleanup_cmd;
mod clipboard_cmd;
mod cog;
mod create;
mod curves_edit;
mod delete;
mod divide;
mod edit;
mod entries;
mod fixtures;
mod forms_timing;
mod gml;
mod gml_timing;
mod info_templates;
mod templates_safe;
mod layer_types;
mod layers_find_cmd;
mod lengthen;
mod library_files;
mod match_props;
mod mosaic_files;
mod object_policy;
mod order_cmd;
mod polygon;
mod processing_sink;
mod protocol;
mod protocol_cases;
mod provider_timing;
mod python_calls;
mod reassociate;
mod reconcile;
mod source_relations;
mod sources;
mod sources_query;
mod standards_cmd;
mod rasters_georef;
mod stretch_ties;
mod layer_digest;
mod tables_cmd;
mod transform;
mod units;
mod wall_follow;
mod walls_rectangle;
mod walls_partition;
mod opening_placement;
mod xrefs;

mod viewport_identity;

mod drawing_set;

mod opening_array;

mod wall_contours;

mod wall_contour_mixed;
mod space_levels;
mod spaces_discover;
mod space_columns;
mod space_scope;
mod space_sources;
mod space_separators;

mod spaces_number;
