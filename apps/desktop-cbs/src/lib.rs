//! KentOS GIS's (CBS) desktop package (docs/adr/9199 §4.14): its profile, Ağlar and Alt ağlar — the project's networks
//! built on the network thread, the network tools' questions answered there, Ağ denetimi's mark. The GIS work it shares
//! with the other modules (data layers, sources, processing, rasters, point clouds, forms, topology …) is the core's
//! (`kentos-desktop`'s gis/). It reaches the core alone, through the module interface (`module_parts`): its parts
//! (parts.rs) answer the shell, its state is among the modules' (`CbsState`), its messages and windows are the
//! module's (`Message::Module`, `Dialog::Module`), its `App` methods are its own traits (ext.rs). A program registers
//! the parts before its window starts ([`register`]); the module's tests register them before they run.

// The core's modules under their own names (`crate::app`, `crate::topology` …), as the module's files were written
// inside the core's crate; the module's own modules come first.
#[allow(unused_imports)]
use kentos_desktop::*;

/// Puts the module's parts among the program's (before the window starts).
pub fn register() {
    kentos_desktop::module_parts::register(&cbs_parts::Cbs);
}

#[cfg(test)]
ctor::declarative::ctor! {
    #[ctor(unsafe)]
    fn register_for_tests() {
        register();
    }
}

#[path = "state.rs"]
mod cbs;
#[path = "ext.rs"]
mod cbs_ext;
#[path = "parts.rs"]
mod cbs_parts;
#[path = "profile.rs"]
mod gis_profile;
#[path = "networks/mod.rs"]
mod networks;
#[path = "subnetworks_window.rs"]
mod subnetworks_window;

// The module's tests (docs/adr/9199 §4.14).
#[cfg(test)]
#[path = "tests/address_tab_tests.rs"]
mod address_tab_tests;
#[cfg(test)]
#[path = "tests/buffer_side_tests.rs"]
mod buffer_side_tests;
#[cfg(test)]
#[path = "tests/elevation_tests.rs"]
mod elevation_tests;
#[cfg(test)]
#[path = "tests/charts_tab_tests.rs"]
mod charts_tab_tests;
#[cfg(test)]
#[path = "tests/conditions_tab_tests.rs"]
mod conditions_tab_tests;
#[cfg(test)]
#[path = "tests/features_tab_tests.rs"]
mod features_tab_tests;
#[cfg(test)]
#[path = "tests/points_target_tab_tests.rs"]
mod points_target_tab_tests;
#[cfg(test)]
#[path = "tests/search_tab_tests.rs"]
mod search_tab_tests;
#[cfg(test)]
#[path = "tests/topology_municipal_tab_tests.rs"]
mod topology_municipal_tab_tests;
#[cfg(test)]
#[path = "tests/topology_object_tab_tests.rs"]
mod topology_object_tab_tests;
#[cfg(test)]
#[path = "tests/topology_source_tab_tests.rs"]
mod topology_source_tab_tests;
#[cfg(test)]
#[path = "tests/topology_tab_tests.rs"]
mod topology_tab_tests;
#[cfg(test)]
#[path = "tests/vertices_tab_tests.rs"]
mod vertices_tab_tests;
