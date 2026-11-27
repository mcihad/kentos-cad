// The desktop's test suites, declared at the crate's root as lib.rs declares the program's modules (lib.rs
// includes this file: docs/adr/8054 §7, no source file past 2 000 lines). Their paths are lib.rs's, this file
// being in the same folder. A new test suite is declared here.

#[cfg(any(test, feature = "testing"))]
#[path = "tests/acceptance_flat_tests.rs"]
pub mod acceptance_flat_tests;
#[cfg(test)]
#[path = "tests/acceptance_gis_tests.rs"]
mod acceptance_gis_tests;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/acceptance_tests.rs"]
pub mod acceptance_tests;
#[cfg(test)]
#[path = "tests/measure_probe_tests.rs"]
mod measure_probe_tests;
#[cfg(test)]
#[path = "tests/coordinate_chain_tests.rs"]
mod coordinate_chain_tests;
#[cfg(test)]
#[path = "tests/semantic_tool_tests.rs"]
mod semantic_tool_tests;
#[cfg(test)]
#[path = "tests/annotative_sheet_tests.rs"]
mod annotative_sheet_tests;
#[cfg(test)]
#[path = "tests/dimension_set_sheet_tests.rs"]
mod dimension_set_sheet_tests;
#[cfg(test)]
#[path = "tests/symbols_sheet_tests.rs"]
mod symbols_sheet_tests;
#[cfg(test)]
#[path = "tests/wall_detail_sheet_tests.rs"]
mod wall_detail_sheet_tests;
#[cfg(test)]
#[path = "tests/text_outputs_tests.rs"]
mod text_outputs_tests;
#[cfg(test)]
#[path = "tests/address_scenes.rs"]
mod address_scenes;
#[cfg(test)]
#[path = "tests/option_contract_tests.rs"]
mod option_contract_tests;
#[cfg(test)]
#[path = "tests/alias_profile_scenes.rs"]
mod alias_profile_scenes;
#[cfg(test)]
#[path = "tests/keymap_tests.rs"]
mod keymap_tests;
#[cfg(test)]
#[path = "tests/import_decision_scenes.rs"]
mod import_decision_scenes;
#[cfg(test)]
#[path = "tests/appearance_tokens_tests.rs"]
mod appearance_tokens_tests;
#[cfg(test)]
#[path = "tests/assoc_array_scenes.rs"]
mod assoc_array_scenes;
#[cfg(test)]
#[path = "tests/constraint_follow_scenes.rs"]
mod constraint_follow_scenes;
#[cfg(test)]
#[path = "tests/constraint_scenes.rs"]
mod constraint_scenes;
#[cfg(test)]
#[path = "tests/constraint_ui_scenes.rs"]
mod constraint_ui_scenes;
#[cfg(test)]
#[path = "tests/attachment_scenes.rs"]
mod attachment_scenes;
#[cfg(test)]
#[path = "tests/attachments_tests.rs"]
mod attachments_tests;
#[cfg(test)]
#[path = "tests/audit_scenes.rs"]
mod audit_scenes;
#[cfg(test)]
#[path = "tests/band_scenes.rs"]
mod band_scenes;
#[cfg(test)]
#[path = "tests/block_attr_scenes.rs"]
mod block_attr_scenes;
#[cfg(test)]
#[path = "tests/block_dynamic_scenes.rs"]
mod block_dynamic_scenes;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/furniture_scenes.rs"]
pub mod furniture_scenes;
#[cfg(test)]
#[path = "tests/casework_scenes.rs"]
mod casework_scenes;
#[cfg(test)]
#[path = "tests/temporary_scenes.rs"]
mod temporary_scenes;
#[cfg(test)]
#[path = "tests/block_scenes.rs"]
mod block_scenes;
#[cfg(test)]
#[path = "tests/centerline_scenes.rs"]
mod centerline_scenes;
#[cfg(test)]
#[path = "tests/source_relation_scenes.rs"]
mod source_relation_scenes;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/chart_scenes.rs"]
pub mod chart_scenes;
#[cfg(test)]
#[path = "tests/cleanup_scenes.rs"]
mod cleanup_scenes;
#[cfg(test)]
#[path = "tests/clipboard_scenes.rs"]
mod clipboard_scenes;
#[cfg(test)]
#[path = "tests/cloud_scenes.rs"]
mod cloud_scenes;
#[cfg(test)]
#[path = "tests/coordinate_scenes.rs"]
mod coordinate_scenes;
#[cfg(test)]
#[path = "tests/curve_edit_scenes.rs"]
mod curve_edit_scenes;
#[cfg(test)]
#[path = "tests/curve_scenes.rs"]
mod curve_scenes;
#[cfg(test)]
#[path = "tests/dimension_follow_scenes.rs"]
mod dimension_follow_scenes;
#[cfg(test)]
#[path = "tests/dimension_mark_scenes.rs"]
mod dimension_mark_scenes;
#[cfg(test)]
#[path = "tests/facade_dimension_scenes.rs"]
mod facade_dimension_scenes;
#[cfg(test)]
#[path = "tests/dimension_layout_scenes.rs"]
mod dimension_layout_scenes;
#[cfg(test)]
#[path = "tests/annotative_scenes.rs"]
mod annotative_scenes;
#[cfg(test)]
#[path = "tests/paragraph_layout_scenes.rs"]
mod paragraph_layout_scenes;
#[cfg(test)]
#[path = "tests/text_field_scenes.rs"]
mod text_field_scenes;
#[cfg(test)]
#[path = "tests/multileader_scenes.rs"]
mod multileader_scenes;
#[cfg(test)]
#[path = "tests/table_cells_scenes.rs"]
mod table_cells_scenes;
#[cfg(test)]
#[path = "tests/dimension_option_scenes.rs"]
mod dimension_option_scenes;
#[cfg(test)]
#[path = "tests/dimension_scenes.rs"]
mod dimension_scenes;
#[cfg(test)]
#[path = "tests/divide_scenes.rs"]
mod divide_scenes;
#[cfg(test)]
#[path = "tests/edge_shift_scenes.rs"]
mod edge_shift_scenes;
#[cfg(test)]
#[path = "tests/area_cut_scenes.rs"]
mod area_cut_scenes;
#[cfg(test)]
#[path = "tests/select_typed_tests.rs"]
mod select_typed_tests;
#[cfg(test)]
#[path = "tests/edge_shift_bond_scenes.rs"]
mod edge_shift_bond_scenes;
#[cfg(test)]
#[path = "tests/edge_shift_junction_scenes.rs"]
mod edge_shift_junction_scenes;
#[cfg(test)]
#[path = "tests/topology_inner_scenes.rs"]
mod topology_inner_scenes;
#[cfg(test)]
#[path = "tests/arc_join_scenes.rs"]
mod arc_join_scenes;
#[cfg(test)]
#[path = "tests/elevation_scenes.rs"]
mod elevation_scenes;
#[cfg(test)]
#[path = "tests/cbs_storage_tests.rs"]
mod cbs_storage_tests;
#[cfg(test)]
#[path = "tests/elevation_tests.rs"]
mod elevation_tests;
#[cfg(test)]
#[path = "tests/explode_scenes.rs"]
mod explode_scenes;
#[cfg(any(test, feature = "testing"))]
#[path = "shell/files_testing.rs"]
pub mod files_testing;
#[cfg(any(test, feature = "testing"))]
#[path = "shell/window_testing.rs"]
pub mod window_testing;
#[cfg(any(test, feature = "testing"))]
#[path = "shell/render_testing.rs"]
pub mod render_testing;
#[cfg(test)]
#[path = "tests/hatch_scenes.rs"]
mod hatch_scenes;
#[cfg(test)]
#[path = "tests/hatch_boundary_scenes.rs"]
mod hatch_boundary_scenes;
#[cfg(test)]
#[path = "tests/hatch_pattern_scenes.rs"]
mod hatch_pattern_scenes;
#[cfg(test)]
#[path = "tests/hatch_look_scenes.rs"]
mod hatch_look_scenes;
#[cfg(test)]
#[path = "tests/history_scenes.rs"]
mod history_scenes;
#[cfg(test)]
#[path = "shell/icon_tour.rs"]
mod icon_tour;
#[cfg(test)]
#[path = "tests/image_scenes.rs"]
mod image_scenes;
#[cfg(test)]
#[path = "tests/impact_scenes.rs"]
mod impact_scenes;
#[cfg(test)]
#[path = "tests/imperial_scenes.rs"]
mod imperial_scenes;
#[cfg(test)]
#[path = "tests/input_scenes.rs"]
mod input_scenes;
#[cfg(test)]
#[path = "cad/label_probe.rs"]
mod label_probe;
#[cfg(test)]
#[path = "shell/inspector_probe.rs"]
mod inspector_probe;
#[cfg(test)]
#[path = "tests/inspector_screens.rs"]
mod inspector_screens;
#[cfg(test)]
#[path = "tests/label_scenes.rs"]
mod label_scenes;
#[cfg(test)]
#[path = "tests/dialog_size_tests.rs"]
mod dialog_size_tests;
#[cfg(test)]
#[path = "tests/constraint_screens.rs"]
mod constraint_screens;
#[cfg(test)]
#[path = "tests/form_fit_screens.rs"]
mod form_fit_screens;
#[cfg(test)]
#[path = "tests/scene3d_window_tests.rs"]
mod scene3d_window_tests;
#[cfg(test)]
#[path = "tests/room_emsal_tests.rs"]
mod room_emsal_tests;
#[cfg(test)]
#[path = "tests/zones_tests.rs"]
mod zones_tests;
#[cfg(test)]
#[path = "tests/zoning_parcel_tests.rs"]
mod zoning_parcel_tests;
#[cfg(test)]
#[path = "tests/road_project_regional_tests.rs"]
mod road_project_regional_tests;
#[cfg(test)]
#[path = "tests/catalog_carry_tests.rs"]
mod catalog_carry_tests;
#[cfg(test)]
#[path = "tests/survey_heights_tests.rs"]
mod survey_heights_tests;
#[cfg(test)]
#[path = "tests/features_reconcile_areas_tests.rs"]
mod features_reconcile_areas_tests;
#[cfg(test)]
#[path = "tests/scale_summary_tests.rs"]
mod scale_summary_tests;
#[cfg(test)]
#[path = "tests/features_window_tests.rs"]
mod features_window_tests;
#[cfg(test)]
#[path = "tests/grade_reference_tests.rs"]
mod grade_reference_tests;
#[cfg(test)]
#[path = "tests/grade_surface_tests.rs"]
mod grade_surface_tests;
#[cfg(test)]
#[path = "tests/grade_surface_relative_tests.rs"]
mod grade_surface_relative_tests;
#[cfg(test)]
#[path = "tests/grade_window_tests.rs"]
mod grade_window_tests;
#[cfg(test)]
#[path = "tests/clean_report_window_tests.rs"]
mod clean_report_window_tests;
#[cfg(test)]
#[path = "tests/exchange_check_window_tests.rs"]
mod exchange_check_window_tests;
#[cfg(test)]
#[path = "tests/project_graph_window_tests.rs"]
mod project_graph_window_tests;
#[cfg(test)]
#[path = "tests/recipe_layer_look_tests.rs"]
mod recipe_layer_look_tests;
#[cfg(test)]
#[path = "tests/recipe_layer_fields_tests.rs"]
mod recipe_layer_fields_tests;
#[cfg(test)]
#[path = "tests/recipe_layer_rules_tests.rs"]
mod recipe_layer_rules_tests;
#[cfg(test)]
#[path = "tests/features_reconcile_window_tests.rs"]
mod features_reconcile_window_tests;
#[cfg(test)]
#[path = "tests/features_reconcile_fill_tests.rs"]
mod features_reconcile_fill_tests;
#[cfg(test)]
#[path = "tests/features_reconcile_reapply_tests.rs"]
mod features_reconcile_reapply_tests;
#[cfg(test)]
#[path = "tests/impact_window_tests.rs"]
mod impact_window_tests;
#[cfg(test)]
#[path = "tests/impact_slope_tests.rs"]
mod impact_slope_tests;
#[cfg(test)]
#[path = "tests/impact_slope_classes_tests.rs"]
mod impact_slope_classes_tests;
#[cfg(test)]
#[path = "tests/impact_network_record_tests.rs"]
mod impact_network_record_tests;
#[cfg(test)]
#[path = "tests/brand_tests.rs"]
mod brand_tests;
#[cfg(test)]
#[path = "tests/reproject_window_tests.rs"]
mod reproject_window_tests;
#[cfg(test)]
#[path = "tests/catalog_report_window_tests.rs"]
mod catalog_report_window_tests;
#[cfg(test)]
#[path = "tests/section_check_window_tests.rs"]
mod section_check_window_tests;
#[cfg(test)]
#[path = "tests/sheet_contours_window_tests.rs"]
mod sheet_contours_window_tests;
#[cfg(test)]
#[path = "tests/stockpile_window_tests.rs"]
mod stockpile_window_tests;
#[cfg(test)]
#[path = "tests/map_menus_tests.rs"]
mod map_menus_tests;
#[cfg(test)]
#[path = "tests/platform_window_tests.rs"]
mod platform_window_tests;
#[cfg(test)]
#[path = "tests/terrain_cloud_window_tests.rs"]
mod terrain_cloud_window_tests;
#[cfg(test)]
#[path = "tests/drainage_window_tests.rs"]
mod drainage_window_tests;
#[cfg(test)]
#[path = "tests/extract_window_tests.rs"]
mod extract_window_tests;
#[cfg(test)]
#[path = "tests/surface_analysis_window_tests.rs"]
mod surface_analysis_window_tests;
#[cfg(test)]
#[path = "tests/material_zones_tests.rs"]
mod material_zones_tests;
#[cfg(test)]
#[path = "tests/compare_sections_tests.rs"]
mod compare_sections_tests;
#[cfg(test)]
#[path = "tests/dxf_attribute_names_tests.rs"]
mod dxf_attribute_names_tests;
#[cfg(test)]
#[path = "tests/quality_form_tests.rs"]
mod quality_form_tests;
#[cfg(test)]
#[path = "tests/terrain_window_tests.rs"]
mod terrain_window_tests;
#[cfg(test)]
#[path = "tests/layer_type_scenes.rs"]
mod layer_type_scenes;
#[cfg(test)]
#[path = "tests/new_layer_reveal_tests.rs"]
mod new_layer_reveal_tests;
#[cfg(test)]
#[path = "tests/layout_plan_tests.rs"]
mod layout_plan_tests;
#[cfg(test)]
#[path = "tests/layout_tests.rs"]
mod layout_tests;
#[cfg(test)]
#[path = "tests/leader_scenes.rs"]
mod leader_scenes;
#[cfg(test)]
#[path = "tests/lengthen_scenes.rs"]
mod lengthen_scenes;
#[cfg(test)]
#[path = "tests/line_type_scenes.rs"]
mod line_type_scenes;
#[cfg(test)]
#[path = "tests/log_plan_tests.rs"]
mod log_plan_tests;
#[cfg(test)]
#[path = "tests/look_scenes.rs"]
mod look_scenes;
#[cfg(test)]
#[path = "tests/match_scenes.rs"]
mod match_scenes;
#[cfg(test)]
#[path = "tests/message_log_tests.rs"]
mod message_log_tests;
#[cfg(test)]
#[path = "tests/method_scenes.rs"]
mod method_scenes;
#[cfg(test)]
#[path = "tests/mirror_scenes.rs"]
mod mirror_scenes;
#[cfg(test)]
#[path = "tests/fixture_library_tests.rs"]
mod fixture_library_tests;
#[cfg(test)]
#[path = "tests/open_report_tests.rs"]
mod open_report_tests;
#[cfg(test)]
#[path = "tests/selection_fit_tests.rs"]
mod selection_fit_tests;
#[cfg(test)]
#[path = "tests/module_screens.rs"]
mod module_screens;
#[cfg(test)]
#[path = "tests/module_catalog_tests.rs"]
mod module_catalog_tests;
#[cfg(test)]
#[path = "tests/scale_names_tests.rs"]
mod scale_names_tests;
#[cfg(test)]
#[path = "tests/module_tests.rs"]
mod module_tests;
#[cfg(test)]
#[path = "tests/multipart_scenes.rs"]
mod multipart_scenes;
#[cfg(test)]
#[path = "tests/typed_names_tests.rs"]
mod typed_names_tests;
#[cfg(test)]
#[path = "tests/nurbs_scenes.rs"]
mod nurbs_scenes;
#[cfg(test)]
#[path = "tests/offset_scenes.rs"]
mod offset_scenes;
#[cfg(test)]
#[path = "tests/overlap_tests.rs"]
mod overlap_tests;
#[cfg(test)]
#[path = "tests/parts_scenes.rs"]
mod parts_scenes;
#[cfg(test)]
#[path = "shell/perf/mod.rs"]
pub(crate) mod perf;
#[cfg(test)]
#[path = "tests/render_conformance_tests.rs"]
mod render_conformance_tests;
#[cfg(test)]
#[path = "tests/render_golden_tests.rs"]
mod render_golden_tests;
#[cfg(test)]
#[path = "tests/render_preview_tests.rs"]
mod render_preview_tests;
#[cfg(test)]
#[path = "tests/point_calc_scenes.rs"]
mod point_calc_scenes;
#[cfg(test)]
#[path = "tests/polyline_scenes.rs"]
mod polyline_scenes;
#[cfg(test)]
#[path = "tests/query_tests.rs"]
mod query_tests;
#[cfg(test)]
#[path = "tests/measure_box_tests.rs"]
mod measure_box_tests;
#[cfg(test)]
#[path = "gis/range_server.rs"]
mod range_server;
#[cfg(test)]
#[path = "tests/raster_scenes.rs"]
mod raster_scenes;
#[cfg(test)]
#[path = "tests/renderer_scenes.rs"]
mod renderer_scenes;
#[cfg(test)]
#[path = "tests/ribbon_tests.rs"]
mod ribbon_tests;
#[cfg(test)]
#[path = "shell/screens.rs"]
mod screens;
#[cfg(test)]
#[path = "tests/segment_scenes.rs"]
mod segment_scenes;
#[cfg(test)]
#[path = "tests/selection_scenes.rs"]
mod selection_scenes;
#[cfg(test)]
#[path = "tests/settings_look_tests.rs"]
mod settings_look_tests;
#[cfg(test)]
#[path = "tests/tool_layer_settings_tests.rs"]
mod tool_layer_settings_tests;
#[cfg(test)]
#[path = "tests/address_compare_scenes.rs"]
mod address_compare_scenes;
#[cfg(test)]
#[path = "tests/data_profile_tests.rs"]
mod data_profile_tests;
#[cfg(test)]
#[path = "tests/data_profile_names_tests.rs"]
mod data_profile_names_tests;
#[cfg(test)]
#[path = "tests/profile_scenes.rs"]
mod profile_scenes;
#[cfg(test)]
#[path = "tests/standard_scenes.rs"]
mod standard_scenes;
#[cfg(test)]
#[path = "tests/review_scenes.rs"]
mod review_scenes;
#[cfg(test)]
#[path = "tests/long_pattern_scenes.rs"]
mod long_pattern_scenes;
#[cfg(test)]
#[path = "tests/linetype_font_scenes.rs"]
mod linetype_font_scenes;
#[cfg(test)]
#[path = "tests/curve_dash_scenes.rs"]
mod curve_dash_scenes;
#[cfg(test)]
#[path = "tests/shx_text_scenes.rs"]
mod shx_text_scenes;
#[cfg(test)]
#[path = "tests/stretch_scenes.rs"]
mod stretch_scenes;
#[cfg(test)]
#[path = "tests/style_override_scenes.rs"]
mod style_override_scenes;
#[cfg(test)]
#[path = "tests/subref_scenes.rs"]
mod subref_scenes;
#[cfg(test)]
#[path = "tests/transform_scenes.rs"]
mod transform_scenes;
#[cfg(test)]
#[path = "tests/ucs_scenes.rs"]
mod ucs_scenes;
// Map services (docs/adr/0208): their tiles, connections' secrets and the device's cache.
#[cfg(test)]
#[path = "tests/distance_scenes.rs"]
mod distance_scenes;
#[cfg(test)]
#[path = "tests/hydrology_scenes.rs"]
mod hydrology_scenes;
#[cfg(test)]
#[path = "tests/interpolation_scenes.rs"]
mod interpolation_scenes;
#[cfg(test)]
#[path = "tests/layer_style_scenes.rs"]
mod layer_style_scenes;
#[cfg(test)]
#[path = "tests/multidim_scenes.rs"]
mod multidim_scenes;
#[cfg(test)]
#[path = "tests/raster_ops_scenes.rs"]
mod raster_ops_scenes;
#[cfg(test)]
#[path = "tests/raster_vector_scenes.rs"]
mod raster_vector_scenes;
#[cfg(test)]
#[path = "tests/remote_scenes.rs"]
mod remote_scenes;
#[cfg(test)]
#[path = "tests/service_scenes.rs"]
mod service_scenes;
#[cfg(test)]
#[path = "tests/snap_commit_tests.rs"]
mod snap_commit_tests;
#[cfg(test)]
#[path = "tests/typing_tests.rs"]
mod typing_tests;
#[cfg(test)]
#[path = "tests/snap_tests.rs"]
mod snap_tests;
#[cfg(any(test, feature = "testing"))]
#[path = "gis/data_layers_tests.rs"]
pub mod data_layers_tests;
// What the tests on a real PostgreSQL share: its address, a table of their own, a relay that cuts a COMMIT.
#[cfg(test)]
#[path = "gis/pg_testing.rs"]
mod pg_testing;
#[cfg(test)]
#[path = "gis/data_edits_tests.rs"]
mod data_edits_tests;
#[cfg(test)]
#[path = "gis/edit_commands_tests.rs"]
mod edit_commands_tests;
#[cfg(test)]
#[path = "gis/data_journal_tests.rs"]
mod data_journal_tests;
#[cfg(test)]
#[path = "gis/layer_export_memory_tests.rs"]
mod layer_export_memory_tests;
#[cfg(test)]
#[path = "gis/query_layer_tests.rs"]
mod query_layer_tests;
#[cfg(test)]
#[path = "gis/data_gone_tests.rs"]
mod data_gone_tests;
#[cfg(test)]
#[path = "gis/gpkg_open_tests.rs"]
mod gpkg_open_tests;
#[cfg(test)]
#[path = "gis/layer_migrate_tests.rs"]
mod layer_migrate_tests;
#[cfg(test)]
#[path = "gis/layer_migrate_many_tests.rs"]
mod layer_migrate_many_tests;
#[cfg(test)]
#[path = "gis/source_selection_tests.rs"]
mod source_selection_tests;
#[cfg(test)]
#[path = "gis/project_store_tests.rs"]
mod project_store_tests;
#[cfg(test)]
#[path = "gis/layer_export_tests.rs"]
mod layer_export_tests;
#[cfg(test)]
#[path = "gis/layer_classes_tests.rs"]
mod layer_classes_tests;
#[cfg(test)]
#[path = "gis/layer_classes_source_tests.rs"]
mod layer_classes_source_tests;
#[cfg(test)]
#[path = "tests/split_scenes.rs"]
mod split_scenes;
#[cfg(test)]
#[path = "tests/split_many_tests.rs"]
mod split_many_tests;
#[cfg(test)]
#[path = "tests/review_panels_tests.rs"]
mod review_panels_tests;
#[cfg(test)]
#[path = "tests/split_tests.rs"]
mod split_tests;
#[cfg(test)]
#[path = "tests/split_wall_tests.rs"]
mod split_wall_tests;
#[cfg(test)]
#[path = "tests/stationing_scenes.rs"]
mod stationing_scenes;
#[cfg(test)]
#[path = "tests/stats_scenes.rs"]
mod stats_scenes;
#[cfg(test)]
#[path = "tests/style_exchange_scenes.rs"]
mod style_exchange_scenes;
#[cfg(test)]
#[path = "tests/style_scenes.rs"]
mod style_scenes;
#[cfg(test)]
#[path = "tests/suitability_scenes.rs"]
mod suitability_scenes;
#[cfg(test)]
#[path = "tests/surface_scenes.rs"]
mod surface_scenes;
#[cfg(test)]
#[path = "tests/table_scenes.rs"]
mod table_scenes;
#[cfg(test)]
#[path = "tests/temporal_scenes.rs"]
mod temporal_scenes;
#[cfg(test)]
#[path = "tests/text_scenes.rs"]
mod text_scenes;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/tools_scenes.rs"]
pub mod tools_scenes;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/tools_screens.rs"]
pub mod tools_screens;
#[cfg(test)]
#[path = "tests/building_context_tests.rs"]
mod building_context_tests;
#[cfg(test)]
#[path = "tests/fixture_place_tests.rs"]
mod fixture_place_tests;
#[cfg(test)]
#[path = "tests/opening_codes_tests.rs"]
mod opening_codes_tests;
#[cfg(test)]
#[path = "tests/paiy_area_tests.rs"]
mod paiy_area_tests;
#[cfg(test)]
#[path = "tests/site_plan_tests.rs"]
mod site_plan_tests;
#[cfg(test)]
#[path = "tests/fire_escape_tests.rs"]
mod fire_escape_tests;
#[cfg(test)]
#[path = "tests/fire_hazard_tests.rs"]
mod fire_hazard_tests;
#[cfg(test)]
#[path = "tests/provisions_tests.rs"]
mod provisions_tests;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/permit_set_tests.rs"]
pub mod permit_set_tests;
#[cfg(test)]
#[path = "tests/roof_plan_set_tests.rs"]
mod roof_plan_set_tests;
#[cfg(test)]
#[path = "tests/room_list_tests.rs"]
mod room_list_tests;
#[cfg(test)]
#[path = "tests/joinery_list_tests.rs"]
mod joinery_list_tests;
#[cfg(test)]
#[path = "tests/renovation_legend_tests.rs"]
mod renovation_legend_tests;
#[cfg(test)]
#[path = "tests/renovation_set_tests.rs"]
mod renovation_set_tests;
#[cfg(test)]
#[path = "tests/permit_scope_tests.rs"]
mod permit_scope_tests;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/permit_package_tests.rs"]
pub mod permit_package_tests;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/bim_permit_tests.rs"]
pub mod bim_permit_tests;
#[cfg(test)]
#[path = "tests/wall_height_tests.rs"]
mod wall_height_tests;
#[cfg(test)]
#[path = "tests/topology_scenes.rs"]
mod topology_scenes;
#[cfg(test)]
#[path = "tests/large_scenes.rs"]
mod large_scenes;
#[cfg(test)]
#[path = "tests/topology_tests.rs"]
mod topology_tests;
#[cfg(test)]
#[path = "tests/transparent_tests.rs"]
mod transparent_tests;
#[cfg(test)]
#[path = "tests/map_crs_tests.rs"]
mod map_crs_tests;
#[cfg(test)]
#[path = "tests/ui_screens.rs"]
mod ui_screens;
#[cfg(test)]
#[path = "tests/ux_screens.rs"]
mod ux_screens;
#[cfg(test)]
#[path = "tests/view_scenes.rs"]
mod view_scenes;
#[cfg(test)]
#[path = "tests/form_scenes.rs"]
mod form_scenes;
#[cfg(test)]
#[path = "tests/form_access_scenes.rs"]
mod form_access_scenes;
#[cfg(test)]
#[path = "tests/window_layers_tests.rs"]
mod window_layers_tests;
#[cfg(test)]
#[path = "tests/wipeout_scenes.rs"]
mod wipeout_scenes;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/wall_scenes.rs"]
pub mod wall_scenes;
#[cfg(test)]
#[path = "tests/bond_drag_scenes.rs"]
mod bond_drag_scenes;
#[cfg(test)]
#[path = "tests/interior_scenes.rs"]
mod interior_scenes;
#[cfg(test)]
#[path = "tests/column_scenes.rs"]
mod column_scenes;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/walkthrough_scenes.rs"]
pub mod walkthrough_scenes;
#[cfg(test)]
#[path = "tests/quality_scenes.rs"]
mod quality_scenes;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/opening_scenes.rs"]
pub mod opening_scenes;
#[cfg(test)]
#[path = "tests/derived_elevation_scenes.rs"]
mod derived_elevation_scenes;
#[cfg(test)]
#[path = "tests/recess_scenes.rs"]
mod recess_scenes;
#[cfg(test)]
#[path = "tests/level_state_scenes.rs"]
mod level_state_scenes;
#[cfg(test)]
#[path = "tests/symbols_scenes.rs"]
mod symbols_scenes;
#[cfg(test)]
#[path = "tests/block_library_scenes.rs"]
mod block_library_scenes;
#[cfg(test)]
#[path = "tests/building_parts_scenes.rs"]
mod building_parts_scenes;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/program_scenes.rs"]
pub mod program_scenes;
#[cfg(test)]
#[path = "tests/view_recipe_scenes.rs"]
mod view_recipe_scenes;
#[cfg(test)]
#[path = "tests/view_detail_scenes.rs"]
mod view_detail_scenes;
#[cfg(test)]
#[path = "tests/view_template_scenes.rs"]
mod view_template_scenes;
#[cfg(test)]
#[path = "tests/view_crop_scenes.rs"]
mod view_crop_scenes;
#[cfg(test)]
#[path = "tests/navigator_scenes.rs"]
mod navigator_scenes;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/space_scenes.rs"]
pub mod space_scenes;
#[cfg(any(test, feature = "testing"))]
#[path = "tests/arch_scenes.rs"]
pub mod arch_scenes;
#[cfg(test)]
#[path = "tests/tag_scenes.rs"]
mod tag_scenes;
#[cfg(test)]
#[path = "tests/viewmark_scenes.rs"]
mod viewmark_scenes;
#[cfg(test)]
#[path = "tests/xref_scenes.rs"]
mod xref_scenes;
#[cfg(test)]
#[path = "tests/sheet_detail_scenes.rs"]
mod sheet_detail_scenes;
#[cfg(test)]
#[path = "tests/plot_style_scenes.rs"]
mod plot_style_scenes;
#[cfg(test)]
#[path = "tests/extract_tail_scenes.rs"]
mod extract_tail_scenes;
#[cfg(test)]
#[path = "tests/flatten_scenes.rs"]
mod flatten_scenes;
#[cfg(test)]
#[path = "tests/underlay_scenes.rs"]
mod underlay_scenes;
#[cfg(test)]
#[path = "tests/source_size_tests.rs"]
mod source_size_tests;
#[cfg(test)]
#[path = "tests/other_names_tests.rs"]
mod other_names_tests;
#[cfg(test)]
#[path = "tests/shell_boundary_tests.rs"]
mod shell_boundary_tests;
