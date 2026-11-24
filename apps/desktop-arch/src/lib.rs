//! KentOS Mimari's desktop package (docs/adr/9199 §4.11): wall styles, door and window types, element types, materials,
//! room modules, design options, Bina programı, Metraj, schedules, Mimari denetimi, the digital project's rules, the
//! delivery record, the underlay models, IFC export and comparison. It reaches the core (`kentos-desktop`) alone,
//! through the module interface (`module_parts`): its parts (parts.rs) answer the shell, its state is among the
//! modules' (`ArchState`), its messages and windows are the module's (`Message::Module`, `Dialog::Module`), its `App`
//! methods are its own traits (ext.rs). The tools that draw its objects (walls, doors, slabs, stairs) are the core's
//! (`kentos-interaction`). A program registers the parts before its window starts ([`register`]); the module's tests
//! register them before they run.

// The core's modules under their own names (`crate::app`, `crate::icons` …), as the module's files were written inside
// the core's crate; the module's own modules come first.
#[allow(unused_imports)]
use kentos_desktop::*;

/// Puts the module's parts among the program's (before the window starts).
pub fn register() {
    kentos_desktop::module_parts::register(&arch_parts::Architecture);
}

#[cfg(test)]
ctor::declarative::ctor! {
    #[ctor(unsafe)]
    fn register_for_tests() {
        register();
    }
}

#[path = "state.rs"]
mod arch;
#[path = "arch_audit/mod.rs"]
mod arch_audit;
#[path = "ext.rs"]
mod arch_ext;
#[path = "arch_ifc.rs"]
mod arch_ifc;
#[path = "parts.rs"]
mod arch_parts;
#[path = "profile.rs"]
mod arch_profile;
#[path = "arch_project.rs"]
mod arch_project;
#[path = "arch_start.rs"]
mod arch_start;
#[path = "architecture_template.rs"]
mod architecture_template;
#[path = "bcf_window.rs"]
mod bcf_window;
#[path = "deliveries_window.rs"]
mod deliveries_window;
#[path = "design_options_window.rs"]
mod design_options_window;
#[path = "digital_rules.rs"]
mod digital_rules;
#[path = "element_types_window.rs"]
mod element_types_window;
#[path = "ifc_compare_window.rs"]
mod ifc_compare_window;
#[path = "material_library.rs"]
mod material_library;
#[path = "materials_window.rs"]
mod materials_window;
#[path = "opening_types_window.rs"]
mod opening_types_window;
#[path = "package_review_window.rs"]
mod package_review_window;
#[path = "program_window.rs"]
mod program_window;
#[path = "quantities/mod.rs"]
mod quantities;
#[path = "room_modules_window.rs"]
mod room_modules_window;
#[path = "schedule_window.rs"]
mod schedule_window;
#[path = "tags_check.rs"]
mod tags_check;
#[path = "underlay_check_view.rs"]
mod underlay_check_view;
#[path = "underlays.rs"]
mod underlays;
#[path = "underlays_window.rs"]
mod underlays_window;
#[path = "viewmark_refs.rs"]
mod viewmark_refs;
#[path = "wall_joins.rs"]
mod wall_joins;
#[path = "wall_styles.rs"]
mod wall_styles;

// The module's tests (docs/adr/9199 §4.11).
#[cfg(test)]
#[path = "tests/arch_icons.rs"]
mod arch_icons;
#[cfg(test)]
#[path = "tests/arch_screens.rs"]
mod arch_screens;
#[cfg(test)]
#[path = "tests/program_window_scenes.rs"]
mod program_window_scenes;
#[cfg(test)]
#[path = "tests/quantities_scenes.rs"]
mod quantities_scenes;
#[cfg(test)]
#[path = "tests/wall_style_scenes.rs"]
mod wall_style_scenes;
#[cfg(test)]
#[path = "tests/render_conformance_tests.rs"]
mod render_conformance_tests;
#[cfg(test)]
#[path = "tests/arch_audit_exception_scenes.rs"]
mod arch_audit_exception_scenes;
#[cfg(test)]
#[path = "tests/arch_audit_scenes.rs"]
mod arch_audit_scenes;
#[cfg(test)]
#[path = "tests/arch_ifc_tests.rs"]
mod arch_ifc_tests;
#[cfg(test)]
#[path = "tests/arch_project_scenes.rs"]
mod arch_project_scenes;
#[cfg(test)]
#[path = "tests/architecture_ribbon_tests.rs"]
mod architecture_ribbon_tests;
#[cfg(test)]
#[path = "tests/basement_list_tests.rs"]
mod basement_list_tests;
#[cfg(test)]
#[path = "tests/boiler_room_tests.rs"]
mod boiler_room_tests;
#[cfg(test)]
#[path = "tests/building_plan_notes_tests.rs"]
mod building_plan_notes_tests;
#[cfg(test)]
#[path = "tests/compartment_tests.rs"]
mod compartment_tests;
#[cfg(test)]
#[path = "tests/covering_reaction_tests.rs"]
mod covering_reaction_tests;
#[cfg(test)]
#[path = "tests/cut_stone_tests.rs"]
mod cut_stone_tests;
#[cfg(test)]
#[path = "tests/dated_sizes_tests.rs"]
mod dated_sizes_tests;
#[cfg(test)]
#[path = "tests/declared_readings_tests.rs"]
mod declared_readings_tests;
#[cfg(test)]
#[path = "tests/delivered_model_tests.rs"]
mod delivered_model_tests;
#[cfg(test)]
#[path = "tests/deliveries_tests.rs"]
mod deliveries_tests;
#[cfg(test)]
#[path = "tests/design_option_scenes.rs"]
mod design_option_scenes;
#[cfg(test)]
#[path = "tests/digital_rules_tests.rs"]
mod digital_rules_tests;
#[cfg(test)]
#[path = "tests/door_clash_scenes.rs"]
mod door_clash_scenes;
#[cfg(test)]
#[path = "tests/double_door_tests.rs"]
mod double_door_tests;
#[cfg(test)]
#[path = "tests/element_types_tests.rs"]
mod element_types_tests;
#[cfg(test)]
#[path = "tests/emsal_limits_tests.rs"]
mod emsal_limits_tests;
#[cfg(test)]
#[path = "tests/finish_reaction_tests.rs"]
mod finish_reaction_tests;
#[cfg(test)]
#[path = "tests/grid_scenes.rs"]
mod grid_scenes;
#[cfg(test)]
#[path = "tests/hospital_tests.rs"]
mod hospital_tests;
#[cfg(test)]
#[path = "tests/hotel_room_tests.rs"]
mod hotel_room_tests;
#[cfg(test)]
#[path = "tests/ifc_revisions_tests.rs"]
mod ifc_revisions_tests;
#[cfg(test)]
#[path = "tests/ifc_viewer_tests.rs"]
mod ifc_viewer_tests;
#[cfg(test)]
#[path = "tests/lodging_escape_tests.rs"]
mod lodging_escape_tests;
#[cfg(test)]
#[path = "tests/material_library_scenes.rs"]
mod material_library_scenes;
#[cfg(test)]
#[path = "tests/material_scenes.rs"]
mod material_scenes;
#[cfg(test)]
#[path = "tests/module_flat_scenes.rs"]
mod module_flat_scenes;
#[cfg(test)]
#[path = "tests/module_scenes.rs"]
mod module_scenes;
#[cfg(test)]
#[path = "tests/module_storey_scenes.rs"]
mod module_storey_scenes;
#[cfg(test)]
#[path = "tests/multi_slide_scenes.rs"]
mod multi_slide_scenes;
#[cfg(test)]
#[path = "tests/opening_type_scenes.rs"]
mod opening_type_scenes;
#[cfg(test)]
#[path = "tests/package_review_tests.rs"]
mod package_review_tests;
#[cfg(test)]
#[path = "tests/party_wall_tests.rs"]
mod party_wall_tests;
#[cfg(test)]
#[path = "tests/performance_tests.rs"]
mod performance_tests;
#[cfg(test)]
#[path = "tests/permit_gate_tests.rs"]
mod permit_gate_tests;
#[cfg(test)]
#[path = "tests/plan_limits_tests.rs"]
mod plan_limits_tests;
#[cfg(test)]
#[path = "tests/plan_notes_tests.rs"]
mod plan_notes_tests;
#[cfg(test)]
#[path = "tests/projection_tests.rs"]
mod projection_tests;
#[cfg(test)]
#[path = "tests/ramp_scenes.rs"]
mod ramp_scenes;
#[cfg(test)]
#[path = "tests/regulation_audit_tests.rs"]
mod regulation_audit_tests;
#[cfg(test)]
#[path = "tests/regulation_dates_tests.rs"]
mod regulation_dates_tests;
#[cfg(test)]
#[path = "tests/resistance_tests.rs"]
mod resistance_tests;
#[cfg(test)]
#[path = "tests/roof_fire_tests.rs"]
mod roof_fire_tests;
#[cfg(test)]
#[path = "tests/roof_rules_tests.rs"]
mod roof_rules_tests;
#[cfg(test)]
#[path = "tests/scene3d_building_tests.rs"]
mod scene3d_building_tests;
#[cfg(test)]
#[path = "tests/scene3d_quality_tests.rs"]
mod scene3d_quality_tests;
#[cfg(test)]
#[path = "tests/schedule_extras_scenes.rs"]
mod schedule_extras_scenes;
#[cfg(test)]
#[path = "tests/shaft_tests.rs"]
mod shaft_tests;
#[cfg(test)]
#[path = "tests/slab_scenes.rs"]
mod slab_scenes;
#[cfg(test)]
#[path = "tests/slide_more_scenes.rs"]
mod slide_more_scenes;
#[cfg(test)]
#[path = "tests/special_door_scenes.rs"]
mod special_door_scenes;
#[cfg(test)]
#[path = "tests/stair_scenes.rs"]
mod stair_scenes;
#[cfg(test)]
#[path = "tests/stone_wall_tests.rs"]
mod stone_wall_tests;
#[cfg(test)]
#[path = "tests/storey_link_tests.rs"]
mod storey_link_tests;
#[cfg(test)]
#[path = "tests/under_stair_tests.rs"]
mod under_stair_tests;
#[cfg(test)]
#[path = "tests/underlay_check_tests.rs"]
mod underlay_check_tests;
#[cfg(test)]
#[path = "tests/underlays_tests.rs"]
mod underlays_tests;
#[cfg(test)]
#[path = "tests/unsaved_close_tests.rs"]
mod unsaved_close_tests;
#[cfg(test)]
#[path = "tests/use_class_tests.rs"]
mod use_class_tests;
#[cfg(test)]
#[path = "tests/wall_insulation_tests.rs"]
mod wall_insulation_tests;
#[cfg(test)]
#[path = "tests/zone_tests.rs"]
mod zone_tests;
#[cfg(test)]
#[path = "tests/ek_rules_tests.rs"]
mod ek_rules_tests;
#[cfg(test)]
#[path = "tests/mimari_dimension_layer_tests.rs"]
mod mimari_dimension_layer_tests;
#[cfg(test)]
#[path = "tests/mimari_fixture_tests.rs"]
mod mimari_fixture_tests;
#[cfg(test)]
#[path = "tests/building_ifc_reach_tests.rs"]
mod building_ifc_reach_tests;
