//! The programs' tests (docs/adr/9199 §4.2): what spans the modules — the surfaces of every program, the layer menu by
//! module and kind of layer, every command's handler, the usage traces, Yeni proje across modules — with every
//! module's parts registered as `kentos-cad` registers them. A module package's own windows are tested in its package,
//! the core's in the core.

// The core's modules under their own names (`crate::app` …), as these tests were written inside the core's crate.
#[allow(unused_imports)]
use kentos_desktop::*;

ctor::declarative::ctor! {
    #[ctor(unsafe)]
    fn register() {
        kentos_desktop_arch::register();
        kentos_desktop_map::register();
        kentos_desktop_road::register();
    }
}

mod layer_menu_tests;
mod layout_program_tests;
mod module_surface_tests;
mod program_tests;
mod scenes;
mod space_separator_tests;
mod wizard_tests;
