//! KentOS Yol's desktop package (docs/adr/9199): alignments, profiles, sections, superelevation, corridors, volumes and
//! hauls, stakeout, road sheets and views, the structures (guardrails, culverts, retaining walls, junctions), and
//! LandXML, IFC and glTF. It reaches the core (`kentos-desktop`) alone, through the module interface (`module_parts`):
//! its parts (parts.rs) answer the shell, its state is among the modules' (`RoadState`), its messages and windows are
//! the module's (`Message::Module`, `Dialog::Module`), its `App` methods are its own traits (ext.rs). The terrain work it
//! shares with Harita is the core's (terrain_work.rs). A program registers the parts before its window starts
//! ([`register`]); the module's tests register them before they run.

// The core's modules under their own names (`crate::app`, `crate::icons` …), as the module's files were written inside
// the core's crate; the module's own modules come first.
#[allow(unused_imports)]
use kentos_desktop::*;

/// Puts the module's parts among the program's (before the window starts).
pub fn register() {
    kentos_desktop::module_parts::register(&road_parts::Road);
}

#[cfg(test)]
ctor::declarative::ctor! {
    #[ctor(unsafe)]
    fn register_for_tests() {
        register();
    }
}

#[path = "alignment_layer.rs"]
mod alignment_layer;
#[path = "alignment_window.rs"]
mod alignment_window;
#[path = "corridor_window.rs"]
mod corridor_window;
#[path = "culvert_window.rs"]
mod culvert_window;
#[path = "design_check_window.rs"]
mod design_check_window;
#[path = "guardrail_window.rs"]
mod guardrail_window;
#[path = "haul_fleet_window.rs"]
mod haul_fleet_window;
#[path = "haul_plan_window.rs"]
mod haul_plan_window;
#[path = "haul_window.rs"]
mod haul_window;
#[path = "junction_window.rs"]
mod junction_window;
#[path = "pay_window.rs"]
mod pay_window;
#[path = "platform_water_window.rs"]
mod platform_water_window;
#[path = "profile_lengths_window.rs"]
mod profile_lengths_window;
#[path = "profile_window.rs"]
mod profile_window;
#[path = "retaining_wall_window.rs"]
mod retaining_wall_window;
#[path = "state.rs"]
mod road;
#[path = "road_alignment_fit_window.rs"]
mod road_alignment_fit_window;
#[path = "road_exchange.rs"]
mod road_exchange;
#[path = "ext.rs"]
mod road_ext;
#[path = "parts.rs"]
mod road_parts;
#[path = "road_place.rs"]
mod road_place;
#[path = "profile.rs"]
mod road_profile;
#[path = "road_profile_fit_window.rs"]
mod road_profile_fit_window;
#[path = "road_sheets_window.rs"]
mod road_sheets_window;
#[path = "road_views_window.rs"]
mod road_views_window;
#[path = "route_window.rs"]
mod route_window;
#[path = "section_window.rs"]
mod section_window;
#[path = "sight_window.rs"]
mod sight_window;
#[path = "stakeout_window.rs"]
mod stakeout_window;
#[path = "superelevation_window.rs"]
mod superelevation_window;
#[path = "swept_window.rs"]
mod swept_window;
#[path = "transition_window.rs"]
mod transition_window;
#[path = "volume_window.rs"]
mod volume_window;
#[path = "widening_window.rs"]
mod widening_window;

// The module's tests (docs/adr/9199 §4.6).
#[cfg(test)]
#[path = "tests/design_check_window_tests.rs"]
mod design_check_window_tests;
#[cfg(test)]
#[path = "tests/picture_tests.rs"]
mod picture_tests;
#[cfg(test)]
#[path = "tests/alignment_window_tests.rs"]
mod alignment_window_tests;
#[cfg(test)]
#[path = "tests/corridor_window_tests.rs"]
mod corridor_window_tests;
#[cfg(test)]
#[path = "tests/culvert_window_tests.rs"]
mod culvert_window_tests;
#[cfg(test)]
#[path = "tests/guardrail_window_tests.rs"]
mod guardrail_window_tests;
#[cfg(test)]
#[path = "tests/haul_fleet_window_tests.rs"]
mod haul_fleet_window_tests;
#[cfg(test)]
#[path = "tests/haul_plan_window_tests.rs"]
mod haul_plan_window_tests;
#[cfg(test)]
#[path = "tests/haul_window_tests.rs"]
mod haul_window_tests;
#[cfg(test)]
#[path = "tests/junction_window_tests.rs"]
mod junction_window_tests;
#[cfg(test)]
#[path = "tests/pay_window_tests.rs"]
mod pay_window_tests;
#[cfg(test)]
#[path = "tests/platform_water_window_tests.rs"]
mod platform_water_window_tests;
#[cfg(test)]
#[path = "tests/profile_lengths_window_tests.rs"]
mod profile_lengths_window_tests;
#[cfg(test)]
#[path = "tests/profile_window_tests.rs"]
mod profile_window_tests;
#[cfg(test)]
#[path = "tests/propose_window_tests.rs"]
mod propose_window_tests;
#[cfg(test)]
#[path = "tests/retaining_wall_window_tests.rs"]
mod retaining_wall_window_tests;
#[cfg(test)]
#[path = "tests/road_alignment_fit_window_tests.rs"]
mod road_alignment_fit_window_tests;
#[cfg(test)]
#[path = "tests/road_exchange_tests.rs"]
mod road_exchange_tests;
#[cfg(test)]
#[path = "tests/road_profile_fit_window_tests.rs"]
mod road_profile_fit_window_tests;
#[cfg(test)]
#[path = "tests/road_project_panel_tests.rs"]
mod road_project_panel_tests;
#[cfg(test)]
#[path = "tests/road_sheets_window_tests.rs"]
mod road_sheets_window_tests;
#[cfg(test)]
#[path = "tests/road_typing_tests.rs"]
mod road_typing_tests;
#[cfg(test)]
#[path = "tests/road_views_window_tests.rs"]
mod road_views_window_tests;
#[cfg(test)]
#[path = "tests/route_window_tests.rs"]
mod route_window_tests;
#[cfg(test)]
#[path = "tests/section_window_tests.rs"]
mod section_window_tests;
#[cfg(test)]
#[path = "tests/sight_window_tests.rs"]
mod sight_window_tests;
#[cfg(test)]
#[path = "tests/stakeout_window_tests.rs"]
mod stakeout_window_tests;
#[cfg(test)]
#[path = "tests/superelevation_window_tests.rs"]
mod superelevation_window_tests;
#[cfg(test)]
#[path = "tests/swept_window_tests.rs"]
mod swept_window_tests;
#[cfg(test)]
#[path = "tests/transition_window_tests.rs"]
mod transition_window_tests;
#[cfg(test)]
#[path = "tests/volume_window_tests.rs"]
mod volume_window_tests;
#[cfg(test)]
#[path = "tests/widening_window_tests.rs"]
mod widening_window_tests;
