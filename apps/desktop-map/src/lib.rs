//! KentOS Harita's desktop package (docs/adr/9199): surveying, cadastre, zoning and orthophoto windows, Harita işi and
//! GNSS arşivi. It reaches the core (`kentos-desktop`) alone, through the module interface (`module_parts`): its
//! parts (parts.rs) answer the shell, its state is among the modules' (`MapState`), its messages and windows are the
//! module's (`Message::Module`, `Dialog::Module`), its `App` methods are its own traits (ext.rs). A program registers
//! the parts before its window starts ([`register`]); the module's tests register them before they run.

// The core's modules under their own names (`crate::app`, `crate::icons` …), as the module's files were written inside
// the core's crate; the module's own modules come first.
#[allow(unused_imports)]
use kentos_desktop::*;

/// Puts the module's parts among the program's (before the window starts).
pub fn register() {
    kentos_desktop::module_parts::register(&map_parts::Map);
}

#[cfg(test)]
ctor::declarative::ctor! {
    #[ctor(unsafe)]
    fn register_for_tests() {
        register();
    }
}

#[path = "asbuilt_chart.rs"]
mod asbuilt_chart;
#[path = "asbuilt_review_form.rs"]
mod asbuilt_review_form;
#[path = "asbuilt_trend.rs"]
mod asbuilt_trend;
#[path = "asbuilt_window.rs"]
mod asbuilt_window;
#[path = "baselines_window.rs"]
mod baselines_window;
#[path = "cloud_place_window.rs"]
mod cloud_place_window;
#[path = "cogo.rs"]
mod cogo;
#[path = "state.rs"]
mod map;
#[path = "ext.rs"]
mod map_ext;
#[path = "parts.rs"]
mod map_parts;
#[path = "profile.rs"]
mod map_profile;
#[path = "map_survey_network_rows.rs"]
mod map_survey_network_rows;
#[path = "map_survey_panel.rs"]
mod map_survey_panel;
#[path = "map_work_limit.rs"]
mod map_work_limit;
#[path = "map_work_panel.rs"]
mod map_work_panel;
#[path = "ortho_compare_window.rs"]
mod ortho_compare_window;
#[path = "ortho_network_section.rs"]
mod ortho_network_section;
#[path = "ortho_seamlines_window.rs"]
mod ortho_seamlines_window;
#[path = "ortho_window.rs"]
mod ortho_window;
#[path = "overlap_window.rs"]
mod overlap_window;
#[path = "parcel_boundaries_marks.rs"]
mod parcel_boundaries_marks;
#[path = "parcel_boundaries_window.rs"]
mod parcel_boundaries_window;
#[path = "parcel_register_window.rs"]
mod parcel_register_window;
#[path = "parcel_report_window.rs"]
mod parcel_report_window;
#[path = "parcels_check_window.rs"]
mod parcels_check_window;
#[path = "pipes_check_rules.rs"]
mod pipes_check_rules;
#[path = "pipes_check_window.rs"]
mod pipes_check_window;
#[path = "pipes_marks.rs"]
mod pipes_marks;
#[path = "readjustment_sheet.rs"]
mod readjustment_sheet;
#[path = "readjustment_window.rs"]
mod readjustment_window;
#[path = "register_window.rs"]
mod register_window;
#[path = "scan_network_window.rs"]
mod scan_network_window;
#[path = "survey_baseline_marks.rs"]
mod survey_baseline_marks;
#[path = "survey_network_window.rs"]
mod survey_network_window;
#[path = "zoning_draft_window.rs"]
mod zoning_draft_window;
#[path = "zoning_marks.rs"]
mod zoning_marks;
#[path = "zoning_symbols_window.rs"]
mod zoning_symbols_window;
#[path = "zoning_window.rs"]
mod zoning_window;

// The module's tests (docs/adr/9199 §4.2).
#[cfg(test)]
#[path = "tests/asbuilt_review_tests.rs"]
mod asbuilt_review_tests;
#[cfg(test)]
#[path = "tests/asbuilt_series_tests.rs"]
mod asbuilt_series_tests;
#[cfg(test)]
#[path = "tests/asbuilt_trend_tests.rs"]
mod asbuilt_trend_tests;
#[cfg(test)]
#[path = "tests/asbuilt_window_tests.rs"]
mod asbuilt_window_tests;
#[cfg(test)]
#[path = "tests/baselines_large_tests.rs"]
mod baselines_large_tests;
#[cfg(test)]
#[path = "tests/baselines_window_tests.rs"]
mod baselines_window_tests;
#[cfg(test)]
#[path = "tests/cloud_place_window_tests.rs"]
mod cloud_place_window_tests;
#[cfg(test)]
#[path = "tests/gnss_sessions_tests.rs"]
mod gnss_sessions_tests;
#[cfg(test)]
#[path = "tests/independent_baselines_tests.rs"]
mod independent_baselines_tests;
#[cfg(test)]
#[path = "tests/map_aids_tests.rs"]
mod map_aids_tests;
#[cfg(test)]
#[path = "tests/map_points_tests.rs"]
mod map_points_tests;
#[cfg(test)]
#[path = "tests/map_road_panel_tests.rs"]
mod map_road_panel_tests;
#[cfg(test)]
#[path = "tests/map_survey_limits_tests.rs"]
mod map_survey_limits_tests;
#[cfg(test)]
#[path = "tests/map_survey_network_rows_tests.rs"]
mod map_survey_network_rows_tests;
#[cfg(test)]
#[path = "tests/map_survey_panel_tests.rs"]
mod map_survey_panel_tests;
#[cfg(test)]
#[path = "tests/map_topology_tests.rs"]
mod map_topology_tests;
#[cfg(test)]
#[path = "tests/map_work_changes_tests.rs"]
mod map_work_changes_tests;
#[cfg(test)]
#[path = "tests/map_work_geometry_tests.rs"]
mod map_work_geometry_tests;
#[cfg(test)]
#[path = "tests/map_work_limit_tests.rs"]
mod map_work_limit_tests;
#[cfg(test)]
#[path = "tests/map_work_panel_tests.rs"]
mod map_work_panel_tests;
#[cfg(test)]
#[path = "tests/map_work_profile_tests.rs"]
mod map_work_profile_tests;
#[cfg(test)]
#[path = "tests/map_work_restore_tests.rs"]
mod map_work_restore_tests;
#[cfg(test)]
#[path = "tests/map_work_stamps_tests.rs"]
mod map_work_stamps_tests;
#[cfg(test)]
#[path = "tests/map_work_tools_tests.rs"]
mod map_work_tools_tests;
#[cfg(test)]
#[path = "tests/network_ellipses_tests.rs"]
mod network_ellipses_tests;
#[cfg(test)]
#[path = "tests/ortho_compare_window_tests.rs"]
mod ortho_compare_window_tests;
#[cfg(test)]
#[path = "tests/ortho_network_tests.rs"]
mod ortho_network_tests;
#[cfg(test)]
#[path = "tests/ortho_seam_polygons_tests.rs"]
mod ortho_seam_polygons_tests;
#[cfg(test)]
#[path = "tests/ortho_seamlines_window_tests.rs"]
mod ortho_seamlines_window_tests;
#[cfg(test)]
#[path = "tests/ortho_window_tests.rs"]
mod ortho_window_tests;
#[cfg(test)]
#[path = "tests/overlap_window_tests.rs"]
mod overlap_window_tests;
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
#[path = "tests/parcel_report_window_tests.rs"]
mod parcel_report_window_tests;
#[cfg(test)]
#[path = "tests/parcels_check_window_tests.rs"]
mod parcels_check_window_tests;
#[cfg(test)]
#[path = "tests/pipes_check_window_tests.rs"]
mod pipes_check_window_tests;
#[cfg(test)]
#[path = "tests/pipes_prerequisite_tests.rs"]
mod pipes_prerequisite_tests;
#[cfg(test)]
#[path = "tests/pipes_profile_walls_tests.rs"]
mod pipes_profile_walls_tests;
#[cfg(test)]
#[path = "tests/pipes_rules_tests.rs"]
mod pipes_rules_tests;
#[cfg(test)]
#[path = "tests/readjustment_window_tests.rs"]
mod readjustment_window_tests;
#[cfg(test)]
#[path = "tests/register_overwrite_tests.rs"]
mod register_overwrite_tests;
#[cfg(test)]
#[path = "tests/register_window_tests.rs"]
mod register_window_tests;
#[cfg(test)]
#[path = "tests/scan_network_window_tests.rs"]
mod scan_network_window_tests;
#[cfg(test)]
#[path = "tests/selection_roles_tests.rs"]
mod selection_roles_tests;
#[cfg(test)]
#[path = "tests/survey_epoch_tests.rs"]
mod survey_epoch_tests;
#[cfg(test)]
#[path = "tests/survey_network_window_tests.rs"]
mod survey_network_window_tests;
#[cfg(test)]
#[path = "tests/survey_region_tests.rs"]
mod survey_region_tests;
#[cfg(test)]
#[path = "tests/survey_velocity_tests.rs"]
mod survey_velocity_tests;
#[cfg(test)]
#[path = "tests/zoning_draft_window_tests.rs"]
mod zoning_draft_window_tests;
#[cfg(test)]
#[path = "tests/zoning_symbols_window_tests.rs"]
mod zoning_symbols_window_tests;
#[cfg(test)]
#[path = "tests/zoning_window_tests.rs"]
mod zoning_window_tests;
