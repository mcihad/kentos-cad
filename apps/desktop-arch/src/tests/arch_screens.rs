//! Mimari's pictures for the owner, as the core's `tools_screens` takes the core's (docs/adr/9199 §4.11): its windows
//! (styles, types, materials, modules, options, Bina programı, Metraj, schedules), Mimari denetimi and the scenes of
//! its objects in a Mimari project, in the dark and the light theme, at 1440×900 and 1100×650. Not a test of
//! correctness and not run by default; writes `.run/shots/arac-*`:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=duvar-stilleri cargo test -p kentos-desktop-arch arch_screens -- --ignored --nocapture
//! ```

use crate::tools_screens::{Pointed, Scene};

fn scenes() -> Vec<Scene> {
    let mut all = crate::opening_type_scenes::scenes();
    all.extend(crate::material_scenes::scenes());
    all.extend(crate::material_library_scenes::scenes());
    all.extend(crate::module_scenes::scenes());
    all.extend(crate::special_door_scenes::scenes());
    all.extend(crate::multi_slide_scenes::scenes());
    all.extend(crate::slide_more_scenes::scenes());
    all.extend(crate::module_flat_scenes::scenes());
    all.extend(crate::module_storey_scenes::scenes());
    all.extend(crate::door_clash_scenes::scenes());
    all.extend(crate::design_option_scenes::scenes());
    all.extend(crate::slab_scenes::scenes());
    all.extend(crate::ramp_scenes::scenes());
    all.extend(crate::stair_scenes::scenes());
    all.extend(crate::grid_scenes::scenes());
    all.extend(crate::arch_project_scenes::scenes());
    all.extend(crate::arch_audit_scenes::scenes());
    all.extend(crate::arch_audit_exception_scenes::scenes());
    all.extend(crate::schedule_extras_scenes::scenes());
    all.extend(crate::wall_style_scenes::scenes());
    all.extend(crate::program_window_scenes::scenes());
    all.extend(crate::quantities_scenes::scenes());
    all
}

fn pointed() -> Vec<Pointed> {
    let mut all = crate::arch_audit_scenes::pointed();
    all.extend(crate::arch_audit_exception_scenes::pointed());
    all.extend(crate::slab_scenes::pointed());
    all.extend(crate::ramp_scenes::pointed());
    all.extend(crate::stair_scenes::pointed());
    all.extend(crate::grid_scenes::pointed());
    all
}

#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    crate::tools_screens::take(scenes(), pointed());
}
