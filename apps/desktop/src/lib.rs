//! KentOS CAD's desktop (docs/adr/0010, 0017): native Iced with the KentOS UI
//! components, the shared Rust contracts and KentOS's own wgpu drawing area
//! (docs/adr/0019), as a library its programs run (docs/adr/0801):
//!
//! - `kentos-cad [--modul <id>] [dosya]`, every module at once: a drawing's
//!   ribbon follows its type and CAD work; `--modul mimari|harita|cbs` makes it
//!   that module's program. `kentos-cad snapshot çıktı.png [çizim.kcad]` draws
//!   the window into an image without opening it, `kentos-cad kullan <iz>`
//!   plays a usage scenario and pictures it ([`all_in_one`]);
//! - `kentos-mimari`, `kentos-harita`, `kentos-gis [dosya]`: one module's own
//!   program, the same library fixed to it ([`module_program`]).

#[cfg(test)]
#[path = "tests/acceptance_flat_tests.rs"]
mod acceptance_flat_tests;
#[cfg(test)]
#[path = "tests/acceptance_gis_tests.rs"]
mod acceptance_gis_tests;
#[cfg(test)]
#[path = "tests/acceptance_tests.rs"]
mod acceptance_tests;
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
#[path = "gis/address/mod.rs"]
mod address;
#[cfg(test)]
#[path = "tests/address_scenes.rs"]
mod address_scenes;
#[path = "shell/aids.rs"]
mod aids;
#[path = "shell/alias_profile.rs"]
mod alias_profile;
#[path = "shell/module_names.rs"]
mod module_names;
#[path = "shell/unsaved_close.rs"]
mod unsaved_close;
#[cfg(test)]
#[path = "tests/unsaved_close_tests.rs"]
mod unsaved_close_tests;
#[path = "shell/window_pick.rs"]
mod window_pick;
#[cfg(test)]
#[path = "tests/option_contract_tests.rs"]
mod option_contract_tests;
#[cfg(test)]
#[path = "tests/alias_profile_scenes.rs"]
mod alias_profile_scenes;
#[path = "cad/annotation_scale.rs"]
mod annotation_scale;
#[path = "cad/annotative.rs"]
mod annotative;
#[path = "cad/annotation_styles.rs"]
mod annotation_styles;
#[path = "arch/wall_joins.rs"]
mod wall_joins;
#[path = "arch/opening_types_window.rs"]
mod opening_types_window;
#[path = "arch/materials_window.rs"]
mod materials_window;
#[path = "arch/material_library.rs"]
mod material_library;
#[path = "arch/room_modules_window.rs"]
mod room_modules_window;
#[path = "arch/design_options_window.rs"]
mod design_options_window;
#[path = "arch/tags_check.rs"]
mod tags_check;
#[path = "arch/schedule_window.rs"]
mod schedule_window;
#[path = "arch/arch_audit/mod.rs"]
mod arch_audit;
#[path = "cad/cad_inspection.rs"]
mod cad_inspection;
#[path = "shell/open_health.rs"]
mod open_health;
#[path = "shell/office_palettes.rs"]
mod office_palettes;
#[path = "shell/recover_file.rs"]
mod recover_file;
#[cfg(test)]
#[path = "tests/import_decision_scenes.rs"]
mod import_decision_scenes;
#[path = "arch/viewmark_refs.rs"]
mod viewmark_refs;
#[path = "cad/sheet_set.rs"]
mod sheet_set;
#[path = "cad/xrefs_window.rs"]
mod xrefs_window;
#[path = "cad/sheet_details_window.rs"]
mod sheet_details_window;
#[path = "cad/plot_styles_window.rs"]
mod plot_styles_window;
#[path = "cad/flatten_window.rs"]
mod flatten_window;
#[path = "gis/reproject_window.rs"]
mod reproject_window;
#[path = "gis/features_window.rs"]
mod features_window;
#[path = "map/baselines_window.rs"]
mod baselines_window;
#[path = "road/grade_reference.rs"]
mod grade_reference;
#[path = "road/grade_surface.rs"]
mod grade_surface;
#[path = "road/grade_window.rs"]
mod grade_window;
#[path = "shell/clean_report_window.rs"]
mod clean_report_window;
#[path = "map/parcels_check_window.rs"]
mod parcels_check_window;
#[path = "gis/impact_reach_marks.rs"]
mod impact_reach_marks;
#[path = "map/parcel_boundaries_marks.rs"]
mod parcel_boundaries_marks;
#[path = "map/readjustment_sheet.rs"]
mod readjustment_sheet;
#[path = "map/parcel_boundaries_window.rs"]
mod parcel_boundaries_window;
#[path = "map/parcel_register_window.rs"]
mod parcel_register_window;
#[path = "map/pipes_check_window.rs"]
mod pipes_check_window;
#[path = "map/pipes_check_rules.rs"]
mod pipes_check_rules;
#[path = "shell/exchange_check_window.rs"]
mod exchange_check_window;
#[path = "map/asbuilt_window.rs"]
mod asbuilt_window;
#[path = "map/asbuilt_trend.rs"]
mod asbuilt_trend;
#[path = "map/asbuilt_chart.rs"]
mod asbuilt_chart;
#[path = "map/ortho_network_section.rs"]
mod ortho_network_section;
#[path = "map/asbuilt_review_form.rs"]
mod asbuilt_review_form;
#[path = "road/project_graph_window.rs"]
mod project_graph_window;
#[path = "map/survey_network_window.rs"]
mod survey_network_window;
#[path = "gis/features_reconcile_window.rs"]
mod features_reconcile_window;
#[path = "gis/features_reconcile_fill.rs"]
mod features_reconcile_fill;
#[path = "map/zoning_window.rs"]
mod zoning_window;
#[path = "map/zoning_symbols_window.rs"]
mod zoning_symbols_window;
#[path = "map/zoning_draft_window.rs"]
mod zoning_draft_window;
#[path = "map/zoning_marks.rs"]
mod zoning_marks;
#[path = "map/pipes_marks.rs"]
mod pipes_marks;
#[path = "map/survey_baseline_marks.rs"]
mod survey_baseline_marks;
#[path = "gis/impact_window.rs"]
mod impact_window;
#[path = "map/readjustment_window.rs"]
mod readjustment_window;
#[path = "map/ortho_compare_window.rs"]
mod ortho_compare_window;
#[path = "map/ortho_seamlines_window.rs"]
mod ortho_seamlines_window;
#[path = "map/ortho_window.rs"]
mod ortho_window;
#[path = "map/overlap_window.rs"]
mod overlap_window;
#[path = "map/register_window.rs"]
mod register_window;
#[path = "map/scan_network_window.rs"]
mod scan_network_window;
#[path = "map/cloud_place_window.rs"]
mod cloud_place_window;
#[path = "shell/catalog_report_window.rs"]
mod catalog_report_window;
#[path = "road/section_check_window.rs"]
mod section_check_window;
#[path = "road/sheet_contours_window.rs"]
mod sheet_contours_window;
#[path = "road/stockpile_window.rs"]
mod stockpile_window;
#[path = "cad/xrefs_review_view.rs"]
mod xrefs_review_view;
#[path = "cad/xrefs_watch.rs"]
mod xrefs_watch;
#[path = "shell/bond_notice.rs"]
mod bond_notice;
#[path = "arch/arch_project.rs"]
mod arch_project;
#[path = "arch/arch_start.rs"]
mod arch_start;
#[path = "arch/architecture_settings.rs"]
mod architecture_settings;
#[path = "arch/architecture_ribbon.rs"]
mod architecture_ribbon;
#[path = "arch/architecture_template.rs"]
mod architecture_template;
#[path = "arch/quantities/mod.rs"]
mod quantities;
#[path = "arch/wall_styles.rs"]
mod wall_styles;
#[path = "cad/table_style_window.rs"]
mod table_style_window;
#[path = "shell/app.rs"]
mod app;
// Asistan: the user's own AI agent (docs/adr/8056 §4).
#[path = "shell/assistant/mod.rs"]
mod assistant;

// Each discipline's part of the shell: its windows' state, messages and windows (docs/adr/8054 §6).
#[path = "arch/state.rs"]
mod arch;
#[path = "gis/state.rs"]
mod gis;
#[path = "map/state.rs"]
mod map;
#[path = "road/state.rs"]
mod road;

#[path = "shell/app_menu/mod.rs"]
mod app_menu;
#[path = "shell/appearance.rs"]
mod appearance;
#[cfg(test)]
#[path = "tests/appearance_tokens_tests.rs"]
mod appearance_tokens_tests;
#[cfg(test)]
#[path = "tests/assoc_array_scenes.rs"]
mod assoc_array_scenes;
#[path = "cad/constraint_relax.rs"]
mod constraint_relax;
#[path = "cad/constraint_view.rs"]
mod constraint_view;
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
#[path = "shell/attachments.rs"]
mod attachments;
#[path = "cad/bound.rs"]
mod bound;
#[cfg(test)]
#[path = "tests/attachments_tests.rs"]
mod attachments_tests;
#[path = "gis/attribute_table.rs"]
mod attribute_table;
#[path = "gis/attribute_values.rs"]
mod attribute_values;
#[cfg(test)]
#[path = "tests/audit_scenes.rs"]
mod audit_scenes;
#[path = "shell/band_ghosts.rs"]
mod band_ghosts;
#[cfg(test)]
#[path = "tests/band_scenes.rs"]
mod band_scenes;
#[cfg(test)]
#[path = "tests/block_attr_scenes.rs"]
mod block_attr_scenes;
#[path = "cad/block_attributes.rs"]
mod block_attributes;
#[path = "cad/block_card.rs"]
mod block_card;
#[path = "cad/block_parameter_groups.rs"]
mod block_parameter_groups;
#[path = "cad/block_type_values.rs"]
mod block_type_values;
#[cfg(test)]
#[path = "tests/block_dynamic_scenes.rs"]
mod block_dynamic_scenes;
#[cfg(test)]
#[path = "tests/furniture_scenes.rs"]
mod furniture_scenes;
#[cfg(test)]
#[path = "tests/casework_scenes.rs"]
mod casework_scenes;
#[cfg(test)]
#[path = "tests/temporary_scenes.rs"]
mod temporary_scenes;
#[cfg(test)]
#[path = "tests/arch_audit_scenes.rs"]
mod arch_audit_scenes;
#[cfg(test)]
#[path = "tests/arch_audit_exception_scenes.rs"]
mod arch_audit_exception_scenes;
#[cfg(test)]
#[path = "tests/schedule_extras_scenes.rs"]
mod schedule_extras_scenes;
#[path = "cad/block_editor/mod.rs"]
mod block_editor;
#[cfg(test)]
#[path = "tests/block_scenes.rs"]
mod block_scenes;
#[path = "cad/block_schedule.rs"]
mod block_schedule;
#[path = "cad/blocks.rs"]
mod blocks;
#[path = "cad/blocks_files.rs"]
mod blocks_files;
#[path = "cad/blocks_library.rs"]
mod blocks_library;
#[path = "cad/blocks_panel/mod.rs"]
mod blocks_panel;
#[path = "shell/navigator.rs"]
mod navigator;
#[path = "road/road_project_panel.rs"]
mod road_project_panel;
#[path = "map/map_work_panel.rs"]
mod map_work_panel;
#[path = "map/map_work_limit.rs"]
mod map_work_limit;
#[path = "map/map_survey_panel.rs"]
mod map_survey_panel;
#[path = "map/map_survey_network_rows.rs"]
mod map_survey_network_rows;
#[path = "shell/bottom.rs"]
mod bottom;
#[path = "gis/calc/mod.rs"]
mod calc;
#[path = "shell/catalog.rs"]
mod catalog;
#[cfg(test)]
#[path = "tests/centerline_scenes.rs"]
mod centerline_scenes;
#[cfg(test)]
#[path = "tests/source_relation_scenes.rs"]
mod source_relation_scenes;
#[cfg(test)]
#[path = "tests/chart_scenes.rs"]
mod chart_scenes;
#[path = "gis/charts/mod.rs"]
mod charts;
#[cfg(test)]
#[path = "tests/cleanup_scenes.rs"]
mod cleanup_scenes;
#[path = "cad/clipboard.rs"]
mod clipboard;
#[cfg(test)]
#[path = "tests/clipboard_scenes.rs"]
mod clipboard_scenes;
#[path = "shell/cloud/mod.rs"]
mod cloud;
#[cfg(test)]
#[path = "tests/cloud_scenes.rs"]
mod cloud_scenes;
#[path = "map/cogo.rs"]
mod cogo;
#[path = "shell/command_bar.rs"]
mod command_bar;
#[cfg(test)]
#[path = "tests/coordinate_scenes.rs"]
mod coordinate_scenes;
#[path = "cad/crs.rs"]
mod crs;
#[cfg(test)]
#[path = "tests/curve_edit_scenes.rs"]
mod curve_edit_scenes;
#[cfg(test)]
#[path = "tests/curve_scenes.rs"]
mod curve_scenes;
#[path = "gis/data_compare.rs"]
mod data_compare;
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
#[path = "cad/clip_frames.rs"]
mod clip_frames;
#[path = "shell/doc_tabs.rs"]
mod doc_tabs;
#[path = "shell/document.rs"]
mod document;
#[path = "cad/draw_order.rs"]
mod draw_order;
#[path = "cad/drawing_exchange.rs"]
mod drawing_exchange;
#[path = "cad/drawing_fonts.rs"]
mod drawing_fonts;
#[path = "cad/drawing_menus.rs"]
mod drawing_menus;
#[path = "gis/data_profile.rs"]
mod data_profile;
#[path = "gis/data_profile_names.rs"]
mod data_profile_names;
#[path = "gis/subnetworks_window.rs"]
mod subnetworks_window;
#[path = "gis/source_status.rs"]
mod source_status;
#[path = "gis/source_trouble.rs"]
mod source_trouble;
#[path = "cad/drawing_standard.rs"]
mod drawing_standard;
#[path = "shell/review_window.rs"]
mod review_window;
#[path = "shell/standard_install.rs"]
mod standard_install;
#[cfg(test)]
#[path = "tests/edge_shift_scenes.rs"]
mod edge_shift_scenes;
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
#[path = "tests/elevation_tests.rs"]
mod elevation_tests;
#[path = "shell/exchange/mod.rs"]
mod exchange;
#[cfg(test)]
#[path = "tests/explode_scenes.rs"]
mod explode_scenes;
#[path = "gis/expression/mod.rs"]
mod expression;
#[path = "gis/features/mod.rs"]
mod features;
#[cfg(test)]
#[path = "shell/files_testing.rs"]
mod files_testing;
#[path = "cad/find_replace.rs"]
mod find_replace;
#[path = "shell/fixture_search.rs"]
mod fixture_search;
#[path = "map/zoning_badge.rs"]
mod zoning_badge;
#[path = "arch/section_badge.rs"]
mod section_badge;
#[path = "arch/grids.rs"]
mod grids;
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
#[path = "cad/history/mod.rs"]
mod history;
#[cfg(test)]
#[path = "tests/history_scenes.rs"]
mod history_scenes;
#[path = "cad/hover_card.rs"]
mod hover_card;
#[cfg(test)]
#[path = "shell/icon_tour.rs"]
mod icon_tour;
#[path = "shell/icons.rs"]
mod icons;
#[cfg(test)]
#[path = "tests/image_scenes.rs"]
mod image_scenes;
#[cfg(test)]
#[path = "tests/impact_scenes.rs"]
mod impact_scenes;
#[cfg(test)]
#[path = "tests/imperial_scenes.rs"]
mod imperial_scenes;
#[path = "cad/info_card.rs"]
mod info_card;
#[path = "shell/input.rs"]
mod input;
#[cfg(test)]
#[path = "tests/input_scenes.rs"]
mod input_scenes;
#[path = "shell/keys.rs"]
mod keys;
#[path = "shell/keytips.rs"]
mod keytips;
#[path = "cad/label_picture.rs"]
mod label_picture;
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
#[path = "gis/labelling/mod.rs"]
mod labelling;
#[path = "cad/labels/mod.rs"]
mod labels;
#[path = "cad/layer_commands.rs"]
mod layer_commands;
#[path = "gis/layer_fields.rs"]
mod layer_fields;
#[path = "gis/layer_filters.rs"]
mod layer_filters;
#[path = "gis/dataset_window.rs"]
mod dataset_window;
#[path = "shell/dialog_sizes.rs"]
mod dialog_sizes;
#[cfg(test)]
#[path = "tests/dialog_size_tests.rs"]
mod dialog_size_tests;
#[cfg(test)]
#[path = "tests/constraint_screens.rs"]
mod constraint_screens;
#[cfg(test)]
#[path = "tests/form_fit_screens.rs"]
mod form_fit_screens;
#[path = "gis/layer_info.rs"]
mod layer_info;
#[path = "cad/layer_list.rs"]
mod layer_list;
#[path = "cad/font_report.rs"]
mod font_report;
#[path = "cad/text_tools.rs"]
mod text_tools;
#[path = "cad/leader_tools.rs"]
mod leader_tools;
#[path = "gis/layer_merge.rs"]
mod layer_merge;
#[path = "gis/joins_window.rs"]
mod joins_window;
#[path = "gis/event_links_window.rs"]
mod event_links_window;
#[path = "road/alignment_layer.rs"]
mod alignment_layer;
#[path = "shell/approved_layer.rs"]
mod approved_layer;
#[path = "road/alignment_window.rs"]
mod alignment_window;
#[path = "road/profile_lengths_window.rs"]
mod profile_lengths_window;
#[path = "road/profile_window.rs"]
mod profile_window;
#[path = "road/section_window.rs"]
mod section_window;
#[path = "road/superelevation_window.rs"]
mod superelevation_window;
#[path = "road/transition_window.rs"]
mod transition_window;
#[path = "road/widening_window.rs"]
mod widening_window;
#[path = "road/corridor_window.rs"]
mod corridor_window;
#[path = "road/volume_window.rs"]
mod volume_window;
#[path = "road/haul_window.rs"]
mod haul_window;
#[path = "road/stakeout_window.rs"]
mod stakeout_window;
#[path = "road/road_alignment_fit_window.rs"]
mod road_alignment_fit_window;
#[path = "road/guardrail_window.rs"]
mod guardrail_window;
#[path = "road/culvert_window.rs"]
mod culvert_window;
#[path = "road/retaining_wall_window.rs"]
mod retaining_wall_window;
#[path = "road/platform_window.rs"]
mod platform_window;
#[path = "road/terrain_cloud_window.rs"]
mod terrain_cloud_window;
#[path = "road/drainage_window.rs"]
mod drainage_window;
#[path = "arch/design_check_window.rs"]
mod design_check_window;
#[path = "road/platform_water_window.rs"]
mod platform_water_window;
#[path = "road/haul_fleet_window.rs"]
mod haul_fleet_window;
#[path = "road/haul_plan_window.rs"]
mod haul_plan_window;
#[path = "road/sight_window.rs"]
mod sight_window;
#[path = "shell/propose_window.rs"]
mod propose_window;
#[path = "cad/picture.rs"]
mod picture;
#[path = "road/road_dialog.rs"]
mod road_dialog;
#[path = "road/swept_window.rs"]
mod swept_window;
#[path = "road/surface_analysis_window.rs"]
mod surface_analysis_window;
#[path = "road/pay_window.rs"]
mod pay_window;
#[path = "road/route_window.rs"]
mod route_window;
#[path = "road/junction_window.rs"]
mod junction_window;
#[path = "shell/scene3d_window.rs"]
mod scene3d_window;
#[cfg(test)]
#[path = "tests/junction_window_tests.rs"]
mod junction_window_tests;
#[cfg(test)]
#[path = "tests/scene3d_window_tests.rs"]
mod scene3d_window_tests;
#[cfg(test)]
#[path = "tests/road_project_panel_tests.rs"]
mod road_project_panel_tests;
#[cfg(test)]
#[path = "tests/road_project_regional_tests.rs"]
mod road_project_regional_tests;
#[cfg(test)]
#[path = "tests/map_work_panel_tests.rs"]
mod map_work_panel_tests;
#[cfg(test)]
#[path = "tests/independent_baselines_tests.rs"]
mod independent_baselines_tests;
#[cfg(test)]
#[path = "tests/catalog_carry_tests.rs"]
mod catalog_carry_tests;
#[cfg(test)]
#[path = "tests/map_work_limit_tests.rs"]
mod map_work_limit_tests;
#[cfg(test)]
#[path = "tests/map_work_tools_tests.rs"]
mod map_work_tools_tests;
#[cfg(test)]
#[path = "tests/map_work_stamps_tests.rs"]
mod map_work_stamps_tests;
#[cfg(test)]
#[path = "tests/map_work_changes_tests.rs"]
mod map_work_changes_tests;
#[cfg(test)]
#[path = "tests/map_work_restore_tests.rs"]
mod map_work_restore_tests;
#[cfg(test)]
#[path = "tests/map_work_geometry_tests.rs"]
mod map_work_geometry_tests;
#[cfg(test)]
#[path = "tests/map_work_profile_tests.rs"]
mod map_work_profile_tests;
#[cfg(test)]
#[path = "tests/survey_heights_tests.rs"]
mod survey_heights_tests;
#[cfg(test)]
#[path = "tests/survey_epoch_tests.rs"]
mod survey_epoch_tests;
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
#[path = "tests/baselines_window_tests.rs"]
mod baselines_window_tests;
#[cfg(test)]
#[path = "tests/baselines_large_tests.rs"]
mod baselines_large_tests;
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
#[path = "tests/parcels_check_window_tests.rs"]
mod parcels_check_window_tests;
#[cfg(test)]
#[path = "tests/parcel_boundaries_marks_tests.rs"]
mod parcel_boundaries_marks_tests;
#[cfg(test)]
#[path = "tests/parcel_boundaries_window_tests.rs"]
mod parcel_boundaries_window_tests;
#[cfg(test)]
#[path = "tests/parcel_register_window_tests.rs"]
mod parcel_register_window_tests;
#[cfg(test)]
#[path = "tests/pipes_check_window_tests.rs"]
mod pipes_check_window_tests;
#[cfg(test)]
#[path = "tests/exchange_check_window_tests.rs"]
mod exchange_check_window_tests;
#[cfg(test)]
#[path = "tests/asbuilt_window_tests.rs"]
mod asbuilt_window_tests;
#[cfg(test)]
#[path = "tests/asbuilt_series_tests.rs"]
mod asbuilt_series_tests;
#[cfg(test)]
#[path = "tests/asbuilt_trend_tests.rs"]
mod asbuilt_trend_tests;
#[cfg(test)]
#[path = "tests/asbuilt_review_tests.rs"]
mod asbuilt_review_tests;
#[cfg(test)]
#[path = "tests/ortho_network_tests.rs"]
mod ortho_network_tests;
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
#[path = "tests/survey_network_window_tests.rs"]
mod survey_network_window_tests;
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
#[path = "tests/zoning_window_tests.rs"]
mod zoning_window_tests;
#[cfg(test)]
#[path = "tests/zoning_symbols_window_tests.rs"]
mod zoning_symbols_window_tests;
#[cfg(test)]
#[path = "tests/zoning_draft_window_tests.rs"]
mod zoning_draft_window_tests;
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
#[path = "tests/readjustment_window_tests.rs"]
mod readjustment_window_tests;
#[cfg(test)]
#[path = "tests/brand_tests.rs"]
mod brand_tests;
#[cfg(test)]
#[path = "tests/ortho_compare_window_tests.rs"]
mod ortho_compare_window_tests;
#[cfg(test)]
#[path = "tests/ortho_seamlines_window_tests.rs"]
mod ortho_seamlines_window_tests;
#[cfg(test)]
#[path = "tests/ortho_seam_polygons_tests.rs"]
mod ortho_seam_polygons_tests;
#[cfg(test)]
#[path = "tests/reproject_window_tests.rs"]
mod reproject_window_tests;
#[cfg(test)]
#[path = "tests/survey_velocity_tests.rs"]
mod survey_velocity_tests;
#[cfg(test)]
#[path = "tests/survey_region_tests.rs"]
mod survey_region_tests;
#[cfg(test)]
#[path = "tests/ortho_window_tests.rs"]
mod ortho_window_tests;
#[cfg(test)]
#[path = "tests/overlap_window_tests.rs"]
mod overlap_window_tests;
#[cfg(test)]
#[path = "tests/register_window_tests.rs"]
mod register_window_tests;
#[cfg(test)]
#[path = "tests/scan_network_window_tests.rs"]
mod scan_network_window_tests;
#[cfg(test)]
#[path = "tests/cloud_place_window_tests.rs"]
mod cloud_place_window_tests;
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
#[path = "tests/map_survey_panel_tests.rs"]
mod map_survey_panel_tests;
#[cfg(test)]
#[path = "tests/map_survey_network_rows_tests.rs"]
mod map_survey_network_rows_tests;
#[cfg(test)]
#[path = "tests/map_survey_limits_tests.rs"]
mod map_survey_limits_tests;
#[cfg(test)]
#[path = "tests/gnss_sessions_tests.rs"]
mod gnss_sessions_tests;
#[cfg(test)]
#[path = "tests/network_ellipses_tests.rs"]
mod network_ellipses_tests;
#[cfg(test)]
#[path = "tests/route_window_tests.rs"]
mod route_window_tests;
#[cfg(test)]
#[path = "tests/pay_window_tests.rs"]
mod pay_window_tests;
#[cfg(test)]
#[path = "tests/retaining_wall_window_tests.rs"]
mod retaining_wall_window_tests;
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
#[path = "tests/design_check_window_tests.rs"]
mod design_check_window_tests;
#[cfg(test)]
#[path = "tests/extract_window_tests.rs"]
mod extract_window_tests;
#[cfg(test)]
#[path = "tests/platform_water_window_tests.rs"]
mod platform_water_window_tests;
#[cfg(test)]
#[path = "tests/haul_fleet_window_tests.rs"]
mod haul_fleet_window_tests;
#[cfg(test)]
#[path = "tests/haul_plan_window_tests.rs"]
mod haul_plan_window_tests;
#[cfg(test)]
#[path = "tests/sight_window_tests.rs"]
mod sight_window_tests;
#[cfg(test)]
#[path = "tests/propose_window_tests.rs"]
mod propose_window_tests;
#[cfg(test)]
#[path = "tests/swept_window_tests.rs"]
mod swept_window_tests;
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
#[path = "tests/pipes_profile_walls_tests.rs"]
mod pipes_profile_walls_tests;
#[cfg(test)]
#[path = "tests/pipes_rules_tests.rs"]
mod pipes_rules_tests;
#[cfg(test)]
#[path = "tests/dxf_attribute_names_tests.rs"]
mod dxf_attribute_names_tests;
#[cfg(test)]
#[path = "tests/culvert_window_tests.rs"]
mod culvert_window_tests;
#[cfg(test)]
#[path = "tests/guardrail_window_tests.rs"]
mod guardrail_window_tests;
#[cfg(test)]
#[path = "tests/road_profile_fit_window_tests.rs"]
mod road_profile_fit_window_tests;
#[path = "road/road_profile_fit_window.rs"]
mod road_profile_fit_window;
#[cfg(test)]
#[path = "tests/road_alignment_fit_window_tests.rs"]
mod road_alignment_fit_window_tests;
#[path = "road/road_place.rs"]
mod road_place;
#[path = "road/road_sheets_window.rs"]
mod road_sheets_window;
#[path = "road/road_views_window.rs"]
mod road_views_window;
#[path = "road/road_exchange.rs"]
mod road_exchange;
#[path = "arch/arch_ifc.rs"]
mod arch_ifc;
#[cfg(test)]
#[path = "tests/alignment_window_tests.rs"]
mod alignment_window_tests;
#[cfg(test)]
#[path = "tests/profile_lengths_window_tests.rs"]
mod profile_lengths_window_tests;
#[cfg(test)]
#[path = "tests/picture_tests.rs"]
mod picture_tests;
#[cfg(test)]
#[path = "tests/profile_window_tests.rs"]
mod profile_window_tests;
#[cfg(test)]
#[path = "tests/section_window_tests.rs"]
mod section_window_tests;
#[cfg(test)]
#[path = "tests/superelevation_window_tests.rs"]
mod superelevation_window_tests;
#[cfg(test)]
#[path = "tests/transition_window_tests.rs"]
mod transition_window_tests;
#[cfg(test)]
#[path = "tests/widening_window_tests.rs"]
mod widening_window_tests;
#[cfg(test)]
#[path = "tests/corridor_window_tests.rs"]
mod corridor_window_tests;
#[cfg(test)]
#[path = "tests/volume_window_tests.rs"]
mod volume_window_tests;
#[cfg(test)]
#[path = "tests/haul_window_tests.rs"]
mod haul_window_tests;
#[cfg(test)]
#[path = "tests/stakeout_window_tests.rs"]
mod stakeout_window_tests;
#[cfg(test)]
#[path = "tests/road_sheets_window_tests.rs"]
mod road_sheets_window_tests;
#[cfg(test)]
#[path = "tests/road_views_window_tests.rs"]
mod road_views_window_tests;
#[cfg(test)]
#[path = "tests/road_exchange_tests.rs"]
mod road_exchange_tests;
#[cfg(test)]
#[path = "tests/arch_ifc_tests.rs"]
mod arch_ifc_tests;
#[cfg(test)]
#[path = "tests/quality_form_tests.rs"]
mod quality_form_tests;
#[path = "road/terrain_layer.rs"]
mod terrain_layer;
#[path = "road/terrain_edits.rs"]
mod terrain_edits;
#[path = "road/terrain_window.rs"]
mod terrain_window;
#[cfg(test)]
#[path = "tests/terrain_window_tests.rs"]
mod terrain_window_tests;
#[path = "arch/program_window.rs"]
mod program_window;
#[path = "cad/layer_order.rs"]
mod layer_order;
#[path = "cad/layer_purge.rs"]
mod layer_purge;
#[path = "gis/source_reconcile.rs"]
mod source_reconcile;
#[path = "gis/layer_maplibre.rs"]
mod layer_maplibre;
#[path = "gis/layer_qml.rs"]
mod layer_qml;
#[path = "gis/layer_sld.rs"]
mod layer_sld;
#[path = "gis/layer_style_file.rs"]
mod layer_style_file;
#[path = "cad/layer_snap.rs"]
mod layer_snap;
#[path = "cad/layer_states.rs"]
mod layer_states;
#[path = "gis/layer_translate.rs"]
mod layer_translate;
#[path = "cad/layer_tree.rs"]
mod layer_tree;
#[path = "gis/layer_types.rs"]
mod layer_types;
#[cfg(test)]
#[path = "tests/layer_type_scenes.rs"]
mod layer_type_scenes;
#[path = "cad/layering.rs"]
mod layering;
#[path = "shell/layout.rs"]
mod layout;
#[path = "shell/layout_plan.rs"]
mod layout_plan;
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
#[path = "cad/hatch_patterns_window.rs"]
mod hatch_patterns_window;
#[path = "cad/line_types_window.rs"]
mod line_types_window;
#[path = "cad/parameters_window/mod.rs"]
mod parameters_window;
#[path = "shell/locks.rs"]
mod locks;
#[path = "shell/log_plan.rs"]
mod log_plan;
#[cfg(test)]
#[path = "tests/log_plan_tests.rs"]
mod log_plan_tests;
#[cfg(test)]
#[path = "tests/look_scenes.rs"]
mod look_scenes;
#[path = "map/map_marks.rs"]
mod map_marks;
#[path = "cad/marks.rs"]
mod marks;
#[cfg(test)]
#[path = "tests/match_scenes.rs"]
mod match_scenes;
#[path = "shell/message_log.rs"]
mod message_log;
#[cfg(test)]
#[path = "tests/message_log_tests.rs"]
mod message_log_tests;
#[cfg(test)]
#[path = "tests/method_scenes.rs"]
mod method_scenes;
#[cfg(test)]
#[path = "tests/mirror_scenes.rs"]
mod mirror_scenes;
#[path = "shell/modes.rs"]
mod modes;
#[cfg(test)]
#[path = "tests/module_screens.rs"]
mod module_screens;
#[cfg(test)]
#[path = "tests/module_surface_tests.rs"]
mod module_surface_tests;
#[path = "shell/module_surfaces.rs"]
mod module_surfaces;
#[cfg(test)]
#[path = "tests/module_tests.rs"]
mod module_tests;
#[cfg(test)]
#[path = "tests/multipart_scenes.rs"]
mod multipart_scenes;
#[path = "shell/navigation.rs"]
mod navigation;
#[path = "shell/navigation_cards.rs"]
mod navigation_cards;
#[cfg(test)]
#[path = "tests/typed_names_tests.rs"]
mod typed_names_tests;
#[cfg(test)]
#[path = "tests/nurbs_scenes.rs"]
mod nurbs_scenes;
#[path = "cad/object_info.rs"]
mod object_info;
#[cfg(test)]
#[path = "tests/offset_scenes.rs"]
mod offset_scenes;
#[path = "shell/opening.rs"]
mod opening;
#[cfg(test)]
#[path = "tests/overlap_tests.rs"]
mod overlap_tests;
#[path = "cad/paragraph_editor.rs"]
mod paragraph_editor;
#[cfg(test)]
#[path = "tests/parts_scenes.rs"]
mod parts_scenes;
#[cfg(test)]
#[path = "shell/perf/mod.rs"]
pub(crate) mod perf;
#[path = "cad/photo_view.rs"]
mod photo_view;
#[path = "cad/pictures.rs"]
mod pictures;
#[path = "map/point_calc.rs"]
mod point_calc;
#[cfg(test)]
#[path = "tests/point_calc_scenes.rs"]
mod point_calc_scenes;
#[path = "gis/pointclouds/mod.rs"]
mod pointclouds;
#[path = "gis/points/mod.rs"]
mod points;
#[cfg(test)]
#[path = "tests/polyline_scenes.rs"]
mod polyline_scenes;
#[path = "shell/preview.rs"]
mod preview;
#[path = "gis/processing/mod.rs"]
mod processing;
#[path = "arch/program.rs"]
mod program;
#[path = "shell/project/mod.rs"]
mod project;
#[path = "cad/properties/mod.rs"]
mod properties;
#[path = "shell/python/mod.rs"]
mod python;
#[cfg(test)]
#[path = "tests/query_tests.rs"]
mod query_tests;
#[cfg(test)]
#[path = "gis/range_server.rs"]
mod range_server;
#[cfg(test)]
#[path = "tests/raster_scenes.rs"]
mod raster_scenes;
#[path = "gis/rasters/mod.rs"]
mod rasters;
#[path = "shell/recent.rs"]
mod recent;
#[path = "shell/recovery.rs"]
mod recovery;
#[cfg(test)]
#[path = "tests/renderer_scenes.rs"]
mod renderer_scenes;
#[path = "shell/ribbon_bar.rs"]
mod ribbon_bar;
#[path = "shell/ribbon_keys.rs"]
mod ribbon_keys;
#[path = "shell/ribbon_panels/mod.rs"]
mod ribbon_panels;
#[path = "shell/ribbon_plan.rs"]
mod ribbon_plan;
#[path = "shell/ribbon_search.rs"]
mod ribbon_search;
#[cfg(test)]
#[path = "tests/ribbon_tests.rs"]
mod ribbon_tests;
#[path = "shell/saving.rs"]
mod saving;
#[path = "shell/screen_scale.rs"]
mod screen_scale;
#[cfg(test)]
#[path = "shell/screens.rs"]
mod screens;
#[path = "cad/search/mod.rs"]
mod search;
#[path = "gis/layer_crs.rs"]
mod layer_crs;
#[path = "map/map_crs.rs"]
mod map_crs;
#[path = "gis/second_crs.rs"]
mod second_crs;
#[cfg(test)]
#[path = "tests/segment_scenes.rs"]
mod segment_scenes;
#[path = "cad/selecting.rs"]
mod selecting;
#[path = "cad/selection_chip.rs"]
mod selection_chip;
#[path = "cad/selection_roles.rs"]
mod selection_roles;
#[cfg(test)]
#[path = "tests/selection_roles_tests.rs"]
mod selection_roles_tests;
#[path = "cad/selection_commands.rs"]
mod selection_commands;
#[cfg(test)]
#[path = "tests/selection_scenes.rs"]
mod selection_scenes;
#[path = "shell/settings.rs"]
mod settings;
#[path = "shell/settings_look.rs"]
mod settings_look;
#[cfg(test)]
#[path = "tests/settings_look_tests.rs"]
mod settings_look_tests;
#[path = "shell/settings_sections.rs"]
mod settings_sections;
#[path = "shell/settings_view.rs"]
mod settings_view;
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
#[path = "cad/style_impact.rs"]
mod style_impact;
#[cfg(test)]
#[path = "tests/style_override_scenes.rs"]
mod style_override_scenes;
#[path = "cad/sub_object.rs"]
mod sub_object;
#[cfg(test)]
#[path = "tests/subref_scenes.rs"]
mod subref_scenes;
#[cfg(test)]
#[path = "tests/transform_scenes.rs"]
mod transform_scenes;
#[path = "cad/typed_names.rs"]
mod typed_names;
#[path = "cad/ucs.rs"]
mod ucs;
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
#[path = "gis/networks/mod.rs"]
mod networks;
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
#[path = "shell/services/mod.rs"]
mod services;
#[path = "cad/sheet_book.rs"]
mod sheet_book;
#[path = "cad/sheet_library.rs"]
mod sheet_library;
#[path = "cad/sheet_pdf.rs"]
mod sheet_pdf;
#[path = "cad/sheets/mod.rs"]
mod sheets;
#[path = "cad/sheet_model.rs"]
mod sheet_model;
#[path = "shell/shortcuts.rs"]
mod shortcuts;
#[cfg(test)]
#[path = "tests/snap_commit_tests.rs"]
mod snap_commit_tests;
#[cfg(test)]
#[path = "tests/typing_tests.rs"]
mod typing_tests;
#[path = "cad/snap_cycle.rs"]
mod snap_cycle;
#[path = "cad/snap_menu.rs"]
mod snap_menu;
#[cfg(test)]
#[path = "tests/snap_tests.rs"]
mod snap_tests;
#[path = "shell/snapshot.rs"]
mod snapshot;
#[path = "gis/db_connections.rs"]
mod db_connections;
#[path = "gis/sources/mod.rs"]
mod sources;
#[path = "shell/split_revision/mod.rs"]
mod split_revision;
#[cfg(test)]
#[path = "tests/split_scenes.rs"]
mod split_scenes;
#[cfg(test)]
#[path = "tests/split_many_tests.rs"]
mod split_many_tests;
#[cfg(test)]
#[path = "tests/split_tests.rs"]
mod split_tests;
#[path = "cad/split_view.rs"]
mod split_view;
#[cfg(test)]
#[path = "tests/split_wall_tests.rs"]
mod split_wall_tests;
#[path = "shell/start.rs"]
mod start;
#[cfg(test)]
#[path = "tests/stationing_scenes.rs"]
mod stationing_scenes;
#[path = "gis/statistics/mod.rs"]
mod statistics;
#[cfg(test)]
#[path = "tests/stats_scenes.rs"]
mod stats_scenes;
#[path = "cad/style/mod.rs"]
mod style;
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
#[path = "cad/table_write.rs"]
mod table_write;
#[path = "cad/tables/mod.rs"]
mod tables;
#[path = "arch/template_editor.rs"]
mod template_editor;
#[path = "arch/template_members.rs"]
mod template_members;
#[path = "arch/templates.rs"]
mod templates;
#[path = "arch/templates_panel.rs"]
mod templates_panel;
#[path = "gis/temporal/mod.rs"]
mod temporal;
#[cfg(test)]
#[path = "tests/temporal_scenes.rs"]
mod temporal_scenes;
#[path = "cad/text_field.rs"]
mod text_field;
#[path = "cad/text_file.rs"]
mod text_file;
#[cfg(test)]
#[path = "tests/text_scenes.rs"]
mod text_scenes;
#[path = "shell/toasts.rs"]
mod toasts;
#[cfg(test)]
#[path = "tests/tools_scenes.rs"]
mod tools_scenes;
#[cfg(test)]
#[path = "tests/tools_screens.rs"]
mod tools_screens;
#[cfg(test)]
#[path = "arch/arch_icons.rs"]
mod arch_icons;
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
#[path = "tests/regulation_audit_tests.rs"]
mod regulation_audit_tests;
#[cfg(test)]
#[path = "tests/fire_escape_tests.rs"]
mod fire_escape_tests;
#[cfg(test)]
#[path = "tests/provisions_tests.rs"]
mod provisions_tests;
#[cfg(test)]
#[path = "tests/permit_set_tests.rs"]
mod permit_set_tests;
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
#[path = "tests/wall_height_tests.rs"]
mod wall_height_tests;
#[path = "gis/topology/mod.rs"]
mod topology;
#[cfg(test)]
#[path = "tests/topology_scenes.rs"]
mod topology_scenes;
#[cfg(test)]
#[path = "tests/large_scenes.rs"]
mod large_scenes;
#[cfg(test)]
#[path = "tests/topology_tests.rs"]
mod topology_tests;
#[path = "shell/traces/mod.rs"]
mod traces;
#[path = "cad/tracking.rs"]
mod tracking;
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
#[path = "shell/usage.rs"]
mod usage;
#[path = "cad/vertices/mod.rs"]
mod vertices;
#[path = "shell/view.rs"]
mod view;
#[path = "cad/view_commands.rs"]
mod view_commands;
#[cfg(test)]
#[path = "tests/view_scenes.rs"]
mod view_scenes;
#[path = "shell/viewport/mod.rs"]
mod viewport;
#[path = "cad/views.rs"]
mod views;
#[path = "arch/views_recipe.rs"]
mod views_recipe;
#[path = "gis/forms/mod.rs"]
mod forms;
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
#[cfg(test)]
#[path = "tests/wall_scenes.rs"]
mod wall_scenes;
#[cfg(test)]
#[path = "tests/bond_drag_scenes.rs"]
mod bond_drag_scenes;
#[cfg(test)]
#[path = "tests/interior_scenes.rs"]
mod interior_scenes;
#[path = "arch/active_level.rs"]
mod active_level;
#[cfg(test)]
#[path = "tests/column_scenes.rs"]
mod column_scenes;
#[cfg(test)]
#[path = "tests/walkthrough_scenes.rs"]
mod walkthrough_scenes;
#[cfg(test)]
#[path = "tests/quality_scenes.rs"]
mod quality_scenes;
#[cfg(test)]
#[path = "tests/opening_scenes.rs"]
mod opening_scenes;
#[cfg(test)]
#[path = "tests/opening_type_scenes.rs"]
mod opening_type_scenes;
#[cfg(test)]
#[path = "tests/material_scenes.rs"]
mod material_scenes;
#[cfg(test)]
#[path = "tests/material_library_scenes.rs"]
mod material_library_scenes;
#[cfg(test)]
#[path = "tests/module_scenes.rs"]
mod module_scenes;
#[cfg(test)]
#[path = "tests/derived_elevation_scenes.rs"]
mod derived_elevation_scenes;
#[cfg(test)]
#[path = "tests/special_door_scenes.rs"]
mod special_door_scenes;
#[cfg(test)]
#[path = "tests/multi_slide_scenes.rs"]
mod multi_slide_scenes;
#[cfg(test)]
#[path = "tests/slide_more_scenes.rs"]
mod slide_more_scenes;
#[cfg(test)]
#[path = "tests/module_flat_scenes.rs"]
mod module_flat_scenes;
#[cfg(test)]
#[path = "tests/module_storey_scenes.rs"]
mod module_storey_scenes;
#[cfg(test)]
#[path = "tests/door_clash_scenes.rs"]
mod door_clash_scenes;
#[cfg(test)]
#[path = "tests/design_option_scenes.rs"]
mod design_option_scenes;
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
#[path = "tests/ramp_scenes.rs"]
mod ramp_scenes;
#[cfg(test)]
#[path = "tests/stair_scenes.rs"]
mod stair_scenes;
#[cfg(test)]
#[path = "tests/grid_scenes.rs"]
mod grid_scenes;
#[cfg(test)]
#[path = "tests/block_library_scenes.rs"]
mod block_library_scenes;
#[cfg(test)]
#[path = "tests/slab_scenes.rs"]
mod slab_scenes;
#[cfg(test)]
#[path = "tests/building_parts_scenes.rs"]
mod building_parts_scenes;
#[cfg(test)]
#[path = "tests/program_scenes.rs"]
mod program_scenes;
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
#[cfg(test)]
#[path = "tests/space_scenes.rs"]
mod space_scenes;
#[cfg(test)]
#[path = "tests/arch_scenes.rs"]
mod arch_scenes;
#[cfg(test)]
#[path = "tests/tag_scenes.rs"]
mod tag_scenes;
#[cfg(test)]
#[path = "tests/viewmark_scenes.rs"]
mod viewmark_scenes;
#[cfg(test)]
#[path = "tests/arch_project_scenes.rs"]
mod arch_project_scenes;
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
#[path = "tests/shell_boundary_tests.rs"]
mod shell_boundary_tests;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

use kentos_launch::Module;
use kentos_ui::theme::typography;

/// The exit code of a command line the program could not read.
const BAD_ARGUMENTS: u8 = 2;

/// The program's entry (docs/adr/0802 §1): by the name it was started under (`args`' first word), a
/// module's own program (`kentos-mimari`, a link to this one file) or `kentos-cad`, every module.
pub fn program_main(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let mut args = args.into_iter();
    let named = args.next();
    match named.as_deref().and_then(Module::of_program_name) {
        Some(module) => module_program(module, args),
        None => all_in_one(args),
    }
}

/// `kentos-cad`: its own words first (`snapshot`, `kullan`), then
/// `[--modul <id>] [dosya]` (kentos-launch's command line). A command line it
/// cannot read is said on stderr, with exit code 2.
pub fn all_in_one(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let args: Vec<OsString> = args.into_iter().collect();
    let words = || {
        args.iter()
            .skip(1)
            .map(|a| a.to_string_lossy().into_owned())
    };
    match args.first().and_then(|a| a.to_str()) {
        // `kentos-cad snapshot çıktı.png [çizim.kcad]`: the window without a window.
        Some("snapshot") => return said(snapshot::run(words())),
        // `kentos-cad kullan <iz>`: a usage scenario played and pictured step by step.
        Some("kullan") => return said(usage::run(words())),
        _ => {}
    }
    match kentos_launch::args::parse(args) {
        Ok(launch) => run(launch.module, launch.path),
        Err(why) => refused(&why),
    }
}

/// A module's own program (`kentos-mimari [dosya]`, the one file under the module's name): `--modul` may
/// name only this module, any other is refused with the program to start instead.
pub fn module_program(module: Module, args: impl IntoIterator<Item = OsString>) -> ExitCode {
    match program::fixed_launch(module, args) {
        Ok(path) => run(Some(module), path),
        Err(why) => refused(&why),
    }
}

/// A word's run, its error on stderr.
fn said(result: Result<(), String>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn refused(why: &str) -> ExitCode {
    eprintln!("{why}");
    ExitCode::from(BAD_ARGUMENTS)
}

/// The window: `program` the module it works as (none: every module, kentos-cad's
/// own), `path` the drawing it opens at once.
fn run(program: Option<Module>, path: Option<PathBuf>) -> ExitCode {
    // Embedded faces: nothing has to be installed on the machine.
    typography::load();
    // Bound layers on PostGIS (docs/adr/0505).
    kentos_postgis::register();
    kentos_processing::command::register_as("desktop");
    // STAC searches (docs/adr/0528 §2): over the cloud client's HTTP.
    kentos_native_application::stac_host::register(Box::new(kentos_cloud::stac::HttpFetch));
    kentos_native_application::remote_host::register(Box::new(kentos_cloud::range::HttpRemote));

    // Today's midnight for Yeni sürüm oluştur and Sona erdir is the device's (docs/adr/0210 §7).
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    kentos_interaction::time_version::LOCAL_OFFSET.store(
        cloud::local_time::Zone::system().offset_at(now),
        std::sync::atomic::Ordering::Relaxed,
    );

    let result = iced::application(
        // The user's settings file (docs/adr/0023); in memory where no configuration folder is known.
        move || {
            let settings = settings::Settings::config_dir()
                .map_or_else(settings::Settings::memory, |dir| {
                    settings::Settings::open(&dir, std::time::SystemTime::now())
                });
            // Recovery copies of unsaved work (docs/adr/0030): none when the data folder cannot be used.
            let recovery =
                match recovery::default_root().map(|root| recovery::Recovery::open(&root)) {
                    Some(Ok(recovery)) => recovery,
                    Some(Err(error)) => recovery::Recovery::unavailable(error),
                    None => recovery::Recovery::off(),
                };
            let (mut app, task) = app::App::start_in(program, path.clone(), settings, recovery);
            // Device drafts of cloud projects (docs/adr/0040, 0041), next to the recovery copies.
            app.cloud.drafts = cloud::default_drafts();
            // Local copies of cloud projects: they open without a connection (docs/adr/0043).
            app.cloud.replicas = cloud::default_replicas();
            // The sheets' books, by the project's key, beside them (docs/sheet/design.md §10).
            app.sheet_store = sheets::default_store();
            // The device's NTv2 grids for the projects' datum choices (docs/adr/0168 §4).
            app.arch.grids.library = grids::Library::device();
            // The forms designer's section library and kept drafts (docs/adr/0441 §1, §5).
            app.gis.form_files = forms::library::FormFiles::device();
            // The user's AI agents, in a file of their own readable by the user alone (docs/adr/8056 §4).
            app.assistant.store = kentos_agent::ProfileStore::device();
            app.assistant.refresh();
            if let Some(kept) = app.assistant.store.take_set_aside() {
                app.output(format!(
                    "Yapay zekâ ajanlarının dosyası okunamadı; {} olarak bir kenara alındı. Ajanları Uygulama ayarları › Yapay zekâ'dan yeniden tanımlayın.",
                    kept.display()
                ));
            }
            // The recent files, kept beside the program's other history.
            if let Some(folder) = recent::RecentFiles::default_folder() {
                app.recent = recent::RecentFiles::open(&folder);
                // Kaynaklar's folders, beside them (docs/adr/0199 §7).
                app.gis.sources.folders = sources::SourceFolders::open(&folder);
                // Bloklar's libraries, beside them (docs/adr/0726 §1).
                app.blocks_panel.library = blocks_library::Library::open(&folder);
                // Malzemeler's library folders, beside them (docs/adr/0791).
                app.arch.material_libraries = material_library::Libraries::open(&folder);
                // The Python tab's unsaved script, kept beside them (docs/adr/0136).
                app.python.script = python::script::Script::load(&folder);
                // Each processing tool's last values (islemler.json), beside them.
                app.gis.processing.memory = processing::memory::Memory::open(&folder);
                // The user's models (the designer's Kaydet) beside the built-in ones.
                let models = app.gis.processing.memory.models().to_vec();
                app.gis.processing.registry.set_user_models(models);
            }
            // Kitaplığım, the user's own symbols (kitaplik.kstil, docs/adr/0092).
            if let Some(folder) = style::user_library::default_folder()
                && let Some(problem) = app.styles.open_user_library(&folder)
            {
                app.warn(problem);
            }
            // The layout kept last time (yerlesim.json beside the settings, docs/adr/0115), each program's
            // own (docs/adr/0802).
            if let Some(dir) = settings::Settings::config_dir() {
                if let Some(problem) = app.arch.templates_panel.open_personal(&dir) {
                    app.warn(problem);
                }
                // The user's tool palette (arac-paleti.json, docs/adr/1016).
                if let Some(problem) = app.open_personal_tools(&dir) {
                    app.warn(problem);
                }
                // Each window's size (pencereler.json, docs/adr/0746).
                app.dialog_sizes = dialog_sizes::Keeper::open(&dir);
                app.layout = layout::Keeper::open_for(&dir, program);
                app.apply_layout();
            }
            // Without a drawing named, the start screen (when the preference wants it).
            if path.is_none() {
                app.start_at_launch();
            }
            // The server cell's first answer (the web asks once it is idle after start).
            let server = app.check_server_quietly();
            // The window's device pixels per logical pixel: the brand's pictures are chosen by it.
            let scale = iced::window::oldest().and_then(iced::window::scale_factor).map(app::Message::DeviceScale);
            (app, iced::Task::batch([task, server, scale]))
        },
        app::App::update,
        app::App::view,
    )
    .title(app::App::title)
    .theme(app::App::theme)
    .default_font(typography::ui())
    .subscription(app::App::subscription)
    .window(iced::window::Settings {
        icon: kentos_ui::widget::brand::window_icon(),
        size: iced::Size::new(1440.0, 900.0),
        // The smallest window the layouts are checked at: the ribbon fits (docs/adr/0051),
        // the windows keep their buttons in view.
        min_size: Some(iced::Size::new(1100.0, 650.0)),
        // A drawing with unsaved changes asks before the window closes.
        exit_on_close_request: false,
        // Each program is an application of its own to the desktop (docs/adr/0802): its window's class and
        // Wayland application id is its program's name, as its .desktop file is named (apps/desktop/packaging).
        #[cfg(target_os = "linux")]
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: program.map_or("kentos-cad", Module::program).to_owned(),
            ..Default::default()
        },
        ..iced::window::Settings::default()
    })
    .antialiasing(true)
    .run();
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
