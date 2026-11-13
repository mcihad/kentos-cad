//! Which part of the product each command belongs to (docs/adr/8054 §9, 8056 §1): `registry::discipline_of` reads
//! the registry's five parts, so the desktop's agents link offers a module's program the core's commands and its own
//! discipline's. Every command the desktop runs belongs to one.

use kentos_native_application::DESKTOP_COMMANDS;
use kentos_native_application::registry::{Discipline, discipline_of};

#[test]
fn each_command_belongs_to_the_core_or_one_discipline() {
    assert_eq!(
        discipline_of(kentos_commands::CAD_POLYGON_CREATE),
        Some(Discipline::Core)
    );
    // The edge commands are every module's (they sat in arch/ until ADR 8054 §11).
    assert_eq!(
        discipline_of(kentos_commands::CAD_ENTITIES_FILLET),
        Some(Discipline::Core)
    );
    // A terrain surface's import is the core's: the registry's core part answers it.
    assert_eq!(
        discipline_of(kentos_commands::FILE_IMPORT_LANDXML),
        Some(Discipline::Core)
    );
    assert_eq!(
        discipline_of(kentos_commands::CAD_WALLS_CREATE),
        Some(Discipline::Arch)
    );
    // The building elements moved from the core's part to architecture's (docs/adr/8056 §1).
    assert_eq!(
        discipline_of(kentos_commands::CAD_GRIDS_CREATE),
        Some(Discipline::Arch)
    );
    assert_eq!(
        discipline_of(kentos_commands::CAD_RAILINGS_CREATE),
        Some(Discipline::Arch)
    );
    assert_eq!(
        discipline_of(kentos_commands::CAD_PARCELS_CREATE),
        Some(Discipline::Map)
    );
    assert_eq!(
        discipline_of(kentos_commands::CAD_LAYERS_CRS),
        Some(Discipline::Gis)
    );
    assert_eq!(
        discipline_of(kentos_commands::ROAD_ALIGNMENTS_CREATE),
        Some(Discipline::Road)
    );
    assert_eq!(discipline_of("cad.yok.boyle"), None);
    let lost: Vec<&str> = DESKTOP_COMMANDS
        .iter()
        .map(|(id, _)| *id)
        .filter(|id| discipline_of(id).is_none())
        .collect();
    assert!(
        lost.is_empty(),
        "commands the registry's parts do not answer: {lost:?}"
    );
}
