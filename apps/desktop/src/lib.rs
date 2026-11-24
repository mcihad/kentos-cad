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

// The test suites' declarations (docs/adr/8054 §7: lib.rs passed 2 000 lines).
include!("test_suites.rs");

#[path = "gis/address/mod.rs"]
mod address;
#[path = "shell/aids.rs"]
mod aids;
#[path = "shell/alias_profile.rs"]
mod alias_profile;
#[path = "shell/module_names.rs"]
mod module_names;
#[path = "shell/unsaved_close.rs"]
mod unsaved_close;
#[path = "shell/window_pick.rs"]
mod window_pick;
#[path = "shell/windows/mod.rs"]
mod windows;
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
#[path = "arch/viewmark_refs.rs"]
mod viewmark_refs;
#[path = "arch/digital_rules.rs"]
mod digital_rules;
#[path = "arch/deliveries_window.rs"]
mod deliveries_window;
#[path = "arch/underlays.rs"]
mod underlays;
#[path = "arch/underlays_window.rs"]
mod underlays_window;
#[path = "arch/underlay_check_view.rs"]
mod underlay_check_view;
#[path = "arch/ifc_compare_window.rs"]
mod ifc_compare_window;
#[path = "arch/bcf_window.rs"]
mod bcf_window;
#[path = "arch/package_review_window.rs"]
mod package_review_window;
#[path = "arch/element_types_window.rs"]
mod element_types_window;
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
#[path = "cad/grade_reference.rs"]
mod grade_reference;
#[path = "cad/grade_surface.rs"]
mod grade_surface;
#[path = "cad/grade_window.rs"]
mod grade_window;
#[path = "shell/clean_report_window.rs"]
mod clean_report_window;
#[path = "gis/impact_reach_marks.rs"]
mod impact_reach_marks;
#[path = "shell/exchange_check_window.rs"]
mod exchange_check_window;
#[path = "shell/overlap_layers_window.rs"]
mod overlap_layers_window;
#[path = "cad/project_graph_window.rs"]
mod project_graph_window;
#[path = "gis/features_reconcile_window.rs"]
mod features_reconcile_window;
#[path = "gis/features_reconcile_fill.rs"]
mod features_reconcile_fill;
#[path = "gis/impact_window.rs"]
mod impact_window;
#[path = "shell/catalog_report_window.rs"]
mod catalog_report_window;
#[path = "cad/section_check_window.rs"]
mod section_check_window;
#[path = "cad/sheet_contours_window.rs"]
mod sheet_contours_window;
#[path = "cad/stockpile_window.rs"]
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
pub mod app;
// Asistan: the user's own AI agent (docs/adr/8056 §4).
#[path = "shell/assistant/mod.rs"]
mod assistant;
// Eklentiler: the user's extension registry, its manager and what the extensions add (docs/adr/8057 §3).
#[path = "shell/extensions/mod.rs"]
mod extensions;
// Çalışan işler: the background work in one list, whoever started it (docs/adr/8057 §102).
#[path = "shell/works/mod.rs"]
mod works;

// Each discipline's part of the shell: its windows' state, messages and windows (docs/adr/8054 §6).
#[path = "arch/state.rs"]
mod arch;
#[path = "gis/state.rs"]
pub mod gis;

#[path = "shell/app_menu/mod.rs"]
pub mod app_menu;
#[path = "shell/appearance.rs"]
mod appearance;
#[path = "cad/constraint_relax.rs"]
mod constraint_relax;
#[path = "cad/constraint_view.rs"]
mod constraint_view;
#[path = "shell/attachments.rs"]
mod attachments;
#[path = "cad/bound.rs"]
mod bound;
#[path = "gis/attribute_table.rs"]
mod attribute_table;
#[path = "gis/attribute_values.rs"]
mod attribute_values;
#[path = "shell/band_ghosts.rs"]
mod band_ghosts;
#[path = "cad/block_attributes.rs"]
mod block_attributes;
#[path = "cad/block_card.rs"]
mod block_card;
#[path = "cad/block_parameter_groups.rs"]
mod block_parameter_groups;
#[path = "cad/block_type_values.rs"]
mod block_type_values;
#[path = "cad/block_editor/mod.rs"]
mod block_editor;
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
#[path = "cad/road_project_panel.rs"]
pub mod road_project_panel;
#[path = "shell/bottom.rs"]
pub mod bottom;
#[path = "gis/calc/mod.rs"]
pub mod calc;
#[path = "shell/catalog.rs"]
pub mod catalog;
#[path = "gis/charts/mod.rs"]
mod charts;
#[path = "cad/clipboard.rs"]
mod clipboard;
#[path = "shell/cloud/mod.rs"]
pub mod cloud;
#[path = "shell/command_bar.rs"]
mod command_bar;
#[path = "cad/crs.rs"]
mod crs;
#[path = "gis/data_compare.rs"]
mod data_compare;
#[path = "cad/clip_frames.rs"]
mod clip_frames;
#[path = "shell/doc_tabs.rs"]
mod doc_tabs;
#[path = "shell/document.rs"]
pub mod document;
#[path = "cad/draw_order.rs"]
mod draw_order;
#[path = "cad/drawing_exchange.rs"]
mod drawing_exchange;
#[path = "cad/drawing_fonts.rs"]
mod drawing_fonts;
#[path = "cad/drawing_menus.rs"]
pub mod drawing_menus;
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
#[path = "shell/exchange/mod.rs"]
pub mod exchange;
#[path = "gis/expression/mod.rs"]
mod expression;
#[path = "gis/features/mod.rs"]
mod features;
#[path = "cad/find_replace.rs"]
mod find_replace;
#[path = "shell/fixture_search.rs"]
mod fixture_search;
#[path = "shell/zoning_badge.rs"]
mod zoning_badge;
#[path = "shell/section_badge.rs"]
mod section_badge;
#[path = "arch/grids.rs"]
mod grids;
#[path = "cad/history/mod.rs"]
mod history;
#[path = "cad/hover_card.rs"]
pub mod hover_card;
#[path = "shell/icons.rs"]
pub mod icons;
#[path = "cad/info_card.rs"]
mod info_card;
#[path = "shell/input.rs"]
mod input;
#[path = "shell/keys.rs"]
pub mod keys;
#[path = "shell/keytips.rs"]
mod keytips;
#[path = "cad/label_picture.rs"]
mod label_picture;
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
#[path = "gis/layer_info.rs"]
mod layer_info;
#[path = "cad/layer_list.rs"]
pub mod layer_list;
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
#[path = "shell/approved_layer.rs"]
pub mod approved_layer;
#[path = "cad/platform_window.rs"]
mod platform_window;
#[path = "cad/terrain_cloud_window.rs"]
mod terrain_cloud_window;
#[path = "cad/drainage_window.rs"]
mod drainage_window;
#[path = "shell/propose_window.rs"]
pub mod propose_window;
#[path = "shell/picture.rs"]
pub mod picture;
#[path = "shell/road_dialog.rs"]
pub mod road_dialog;
#[path = "cad/surface_analysis_window.rs"]
mod surface_analysis_window;
#[path = "shell/scene3d_window.rs"]
mod scene3d_window;
#[path = "shell/scene3d_building.rs"]
mod scene3d_building;
#[path = "shell/ifc_viewer.rs"]
mod ifc_viewer;
#[path = "arch/arch_ifc.rs"]
mod arch_ifc;
#[path = "shell/ifc_facts.rs"]
mod ifc_facts;
#[path = "cad/terrain_layer.rs"]
mod terrain_layer;
#[path = "cad/terrain_edits.rs"]
mod terrain_edits;
#[path = "cad/terrain_window.rs"]
mod terrain_window;
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
#[path = "cad/layering.rs"]
mod layering;
#[path = "shell/layout.rs"]
pub mod layout;
#[path = "shell/layout_plan.rs"]
mod layout_plan;
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
#[path = "cad/map_marks.rs"]
mod map_marks;
#[path = "cad/marks.rs"]
pub mod marks;
#[path = "shell/message_log.rs"]
mod message_log;
#[path = "shell/modes.rs"]
mod modes;
#[path = "shell/selection_fit.rs"]
mod selection_fit;
#[path = "shell/open_report_window.rs"]
mod open_report_window;
#[path = "shell/module_surfaces.rs"]
pub mod module_surfaces;
#[path = "shell/layer_entries.rs"]
pub mod layer_entries;
#[path = "cad/view_place.rs"]
mod view_place;
#[path = "cad/terrain_work.rs"]
pub mod terrain_work;
#[path = "shell/numbers.rs"]
pub mod numbers;
#[path = "shell/module_parts.rs"]
pub mod module_parts;
#[path = "shell/module_profile.rs"]
pub mod module_profile;
#[path = "arch/profile.rs"]
mod arch_profile;
#[path = "gis/profile.rs"]
mod gis_profile;
#[path = "shell/navigation.rs"]
mod navigation;
#[path = "shell/navigation_cards.rs"]
mod navigation_cards;
#[path = "cad/object_info.rs"]
mod object_info;
#[path = "shell/opening.rs"]
pub mod opening;
#[path = "cad/paragraph_editor.rs"]
mod paragraph_editor;
#[path = "cad/photo_view.rs"]
mod photo_view;
#[path = "cad/pictures.rs"]
mod pictures;
#[path = "shell/point_calc.rs"]
pub mod point_calc;
#[path = "gis/pointclouds/mod.rs"]
pub mod pointclouds;
#[path = "gis/points/mod.rs"]
pub mod points;
#[path = "shell/preview.rs"]
mod preview;
#[path = "gis/processing/mod.rs"]
pub mod processing;
#[path = "shell/program.rs"]
pub mod program;
#[path = "shell/project/mod.rs"]
pub mod project;
#[path = "cad/properties/mod.rs"]
pub mod properties;
#[path = "shell/python/mod.rs"]
mod python;
#[path = "shell/python_personal.rs"]
mod python_personal;
#[path = "shell/python_setup.rs"]
mod python_setup;
#[path = "shell/standards.rs"]
mod standards;
#[path = "gis/rasters/mod.rs"]
mod rasters;
#[path = "shell/recent.rs"]
mod recent;
#[path = "shell/recovery.rs"]
pub mod recovery;
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
#[path = "shell/saving.rs"]
mod saving;
#[path = "shell/screen_scale.rs"]
mod screen_scale;
#[path = "cad/search/mod.rs"]
mod search;
#[path = "gis/layer_crs.rs"]
mod layer_crs;
#[path = "cad/map_crs.rs"]
mod map_crs;
#[path = "gis/second_crs.rs"]
pub mod second_crs;
#[path = "cad/selecting.rs"]
pub mod selecting;
#[path = "cad/selection_chip.rs"]
mod selection_chip;
#[path = "cad/selection_roles.rs"]
mod selection_roles;
#[path = "cad/selection_commands.rs"]
mod selection_commands;
#[path = "shell/settings.rs"]
pub mod settings;
#[path = "shell/settings_look.rs"]
mod settings_look;
#[path = "shell/settings_sections.rs"]
pub mod settings_sections;
#[path = "shell/settings_view.rs"]
mod settings_view;
#[path = "cad/style_impact.rs"]
mod style_impact;
#[path = "cad/sub_object.rs"]
mod sub_object;
#[path = "cad/typed_names.rs"]
mod typed_names;
#[path = "cad/ucs.rs"]
mod ucs;
#[path = "gis/networks/mod.rs"]
mod networks;
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
#[path = "cad/snap_cycle.rs"]
mod snap_cycle;
#[path = "cad/snap_menu.rs"]
mod snap_menu;
#[path = "shell/snapshot.rs"]
mod snapshot;
#[path = "gis/data_layers.rs"]
pub mod data_layers;
#[path = "gis/data_view.rs"]
mod data_view;
#[path = "gis/data_info.rs"]
mod data_info;
#[path = "gis/data_labels.rs"]
mod data_labels;
#[path = "gis/data_selection.rs"]
mod data_selection;
#[path = "gis/data_conflict.rs"]
mod data_conflict;
#[path = "gis/data_relink.rs"]
mod data_relink;
#[path = "gis/data_edits.rs"]
mod data_edits;
#[path = "gis/data_pending.rs"]
mod data_pending;
#[path = "gis/data_rows.rs"]
mod data_rows;
#[path = "gis/data_counts.rs"]
mod data_counts;
#[path = "gis/data_journal.rs"]
mod data_journal;
#[path = "gis/data_pick.rs"]
mod data_pick;
#[path = "gis/layer_classes.rs"]
mod layer_classes;
#[path = "gis/layer_export.rs"]
mod layer_export;
#[path = "gis/layer_export_extent.rs"]
mod layer_export_extent;
#[path = "gis/system_select.rs"]
mod system_select;
#[path = "gis/project_store.rs"]
mod project_store;
#[path = "gis/gpkg_open.rs"]
mod gpkg_open;
#[path = "gis/layer_create.rs"]
mod layer_create;
#[path = "gis/layer_migrate.rs"]
mod layer_migrate;
#[path = "gis/db_connections.rs"]
mod db_connections;
#[path = "gis/sources/mod.rs"]
pub mod sources;
#[path = "shell/split_revision/mod.rs"]
mod split_revision;
#[path = "cad/split_view.rs"]
mod split_view;
#[path = "shell/start.rs"]
mod start;
#[path = "gis/statistics/mod.rs"]
mod statistics;
#[path = "cad/style/mod.rs"]
pub mod style;
#[path = "cad/table_write.rs"]
pub mod table_write;
#[path = "cad/tables/mod.rs"]
mod tables;
#[path = "arch/template_editor.rs"]
pub mod template_editor;
#[path = "arch/template_members.rs"]
mod template_members;
#[path = "arch/templates.rs"]
mod templates;
#[path = "arch/templates_panel.rs"]
mod templates_panel;
#[path = "gis/temporal/mod.rs"]
mod temporal;
#[path = "cad/text_field.rs"]
mod text_field;
#[path = "cad/text_file.rs"]
mod text_file;
#[path = "shell/toasts.rs"]
mod toasts;
#[path = "gis/topology/mod.rs"]
pub mod topology;
#[path = "shell/traces/mod.rs"]
pub mod traces;
#[path = "cad/tracking.rs"]
mod tracking;
#[path = "shell/usage.rs"]
mod usage;
#[path = "cad/vertices/mod.rs"]
mod vertices;
#[path = "shell/view.rs"]
pub mod view;
#[path = "cad/view_commands.rs"]
mod view_commands;
#[path = "shell/viewport/mod.rs"]
pub mod viewport;
#[path = "cad/views.rs"]
mod views;
#[path = "cad/views_recipe.rs"]
mod views_recipe;
#[path = "gis/forms/mod.rs"]
mod forms;
#[path = "shell/active_level.rs"]
mod active_level;

#[path = "cad/scene_browse.rs"]
mod scene_browse;

mod launch;
pub use launch::{all_in_one, module_program, program_main};

#[cfg(target_os = "macos")]
#[path = "shell/accessibility.rs"]
mod accessibility;
