//! The command registry's part of architecture's commands (docs/adr/8054 §9): `arch_commands!` calls `crate::arch_dispatch::arch_arm!($command, $then, module, InputType)`
//! for one of them, else gives `$else`. The registry (core/registry.rs) chains the parts; the handlers' names are
//! resolved where the registry calls it.

macro_rules! arch_commands {
    ($command:expr, $then:ident, $else:expr) => {
        match $command {
            kentos_commands::CAD_ARCH_IFC => {
                crate::arch_dispatch::arch_arm!(
                    $command,
                    $then,
                    arch_ifc,
                    kentos_commands::ExportIfcBuilding
                )
            }
            kentos_commands::CAD_WALLS_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, walls_create, WallsCreate)
            }
            kentos_commands::CAD_WALLS_RECTANGLE => {
                crate::arch_dispatch::arch_arm!($command, $then, walls_rectangle, WallsRectangle)
            }
            kentos_commands::CAD_WALLS_PARTITION => {
                crate::arch_dispatch::arch_arm!($command, $then, walls_partition, WallsPartition)
            }
            kentos_commands::CAD_WALLS_FROM_LINES => {
                crate::arch_dispatch::arch_arm!($command, $then, walls_from_lines, WallsFromLines)
            }
            kentos_commands::CAD_ARCH_RECOGNIZE => {
                crate::arch_dispatch::arch_arm!(
                    $command,
                    $then,
                    arch_recognize,
                    kentos_commands::ArchRecognize
                )
            }
            kentos_commands::CAD_WALLS_EDIT => {
                crate::arch_dispatch::arch_arm!($command, $then, walls_edit, WallsEdit)
            }
            kentos_commands::CAD_WALLS_JOINS => {
                crate::arch_dispatch::arch_arm!($command, $then, walls_joins, WallsJoins)
            }
            kentos_commands::CAD_OPENINGS_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, openings_create, OpeningsCreate)
            }
            kentos_commands::CAD_OPENINGS_EDIT => {
                crate::arch_dispatch::arch_arm!($command, $then, openings_edit, OpeningsEdit)
            }
            kentos_commands::CAD_OPENING_TYPES_EDIT => {
                crate::arch_dispatch::arch_arm!(
                    $command,
                    $then,
                    opening_types_edit,
                    kentos_commands::OpeningTypesEdit
                )
            }
            kentos_commands::CAD_MATERIALS_EDIT => {
                crate::arch_dispatch::arch_arm!(
                    $command,
                    $then,
                    materials_edit,
                    kentos_commands::MaterialsEdit
                )
            }
            kentos_commands::CAD_OPENINGS_QUERY => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                openings_query,
                kentos_commands::OpeningsQuery
            ),
            kentos_commands::CAD_MODULES_EDIT => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                modules_edit,
                kentos_commands::ModulesEdit
            ),
            kentos_commands::CAD_MODULES_QUERY => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                modules_query,
                kentos_commands::ModulesQuery
            ),
            kentos_commands::CAD_OPTIONS_EDIT => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                options_edit,
                kentos_commands::OptionsEdit
            ),
            kentos_commands::CAD_OPTIONS_QUERY => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                options_query,
                kentos_commands::OptionsQuery
            ),
            kentos_commands::CAD_ELEVATIONS_EDIT => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                elevations_edit,
                kentos_commands::ElevationsEdit
            ),
            kentos_commands::CAD_ELEVATIONS_QUERY => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                elevations_query,
                kentos_commands::ElevationsQuery
            ),
            kentos_commands::CAD_SECTIONS_EDIT => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                sections_edit,
                kentos_commands::BuildingSectionsEdit
            ),
            kentos_commands::CAD_SECTIONS_QUERY => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                sections_query,
                kentos_commands::BuildingSectionsQuery
            ),
            kentos_commands::CAD_OPENINGS_DISTANCES => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                openings_distances,
                kentos_commands::OpeningsDistancesInput
            ),
            kentos_commands::CAD_WALLS_DISTANCES => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                walls_distances,
                kentos_commands::WallsDistancesInput
            ),
            kentos_commands::CAD_ARCH_AUDIT => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                arch_audit,
                kentos_commands::ArchAudit
            ),
            kentos_commands::CAD_ARCH_EXCEPTIONS => {
                crate::arch_dispatch::arch_arm!(
                    $command,
                    $then,
                    arch_exceptions,
                    kentos_commands::ArchExceptions
                )
            }
            kentos_commands::CAD_CASEWORK_RUN => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                casework_run,
                kentos_commands::CaseworkRun
            ),
            kentos_commands::CAD_CASEWORK_EDIT => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                casework_edit,
                kentos_commands::CaseworkEdit
            ),
            kentos_commands::CAD_CASEWORK_QUERY => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                casework_query,
                kentos_commands::CaseworkQuery
            ),
            kentos_commands::CAD_LEVELS_ASSIGN => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                levels_assign,
                kentos_commands::LevelsAssign
            ),
            kentos_commands::CAD_NORTH_CREATE => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                north,
                kentos_commands::NorthCreate
            ),
            kentos_commands::CAD_REVISIONS_MARK => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                revisions,
                kentos_commands::RevisionsMark
            ),
            kentos_commands::CAD_LEGENDS_MATERIALS => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                legends,
                kentos_commands::LegendsMaterials
            ),
            kentos_commands::CAD_SPACES_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, spaces_create, SpacesCreate)
            }
            kentos_commands::CAD_SLABS_CREATE => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                slabs_create,
                kentos_commands::SlabsCreate
            ),
            kentos_commands::CAD_SLABS_EDIT => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                slabs_edit,
                kentos_commands::SlabsEdit
            ),
            kentos_commands::CAD_SPACES_DISCOVER => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                spaces_discover,
                kentos_commands::SpacesDiscover
            ),
            kentos_commands::CAD_SPACES_NUMBER => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                spaces_number,
                kentos_commands::SpacesNumber
            ),
            kentos_commands::CAD_SPACES_SEPARATORS => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                spaces_separators,
                kentos_commands::SpacesSeparators
            ),
            kentos_commands::CAD_ROOFS_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, roofs_create, RoofsCreate)
            }
            kentos_commands::CAD_ARCH_EDIT => {
                crate::arch_dispatch::arch_arm!($command, $then, arch_edit, ArchEdit)
            }
            kentos_commands::CAD_TAGS_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, tags_create, TagsCreate)
            }
            kentos_commands::CAD_TAGS_CHECK => {
                crate::arch_dispatch::arch_arm!($command, $then, tags_check, TagsCheck)
            }
            kentos_commands::CAD_TAGS_EDIT => {
                crate::arch_dispatch::arch_arm!($command, $then, tags_edit, TagsEdit)
            }
            kentos_commands::CAD_VIEWMARKS_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, viewmarks_create, ViewMarksCreate)
            }
            kentos_commands::CAD_VIEWMARKS_EDIT => {
                crate::arch_dispatch::arch_arm!($command, $then, viewmarks_edit, ViewMarksEdit)
            }
            kentos_commands::CAD_VIEWMARKS_REFERENCES => {
                crate::arch_dispatch::arch_arm!(
                    $command,
                    $then,
                    viewmarks_references,
                    ViewMarksReferences
                )
            }
            kentos_commands::CAD_SPACES_FINISH => {
                crate::arch_dispatch::arch_arm!($command, $then, spaces_finish, SpacesFinish)
            }
            kentos_commands::CAD_TEMPLATES_APPLY => {
                crate::arch_dispatch::arch_arm!($command, $then, templates_apply, TemplatesApply)
            }
            kentos_commands::CAD_SCHEDULES_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, schedules_create, SchedulesCreate)
            }
            kentos_commands::CAD_SCHEDULES_EDIT => {
                crate::arch_dispatch::arch_arm!($command, $then, schedules_edit, SchedulesEdit)
            }
            kentos_commands::CAD_SCHEDULES_QUERY => {
                crate::arch_dispatch::arch_arm!($command, $then, schedules_query, SchedulesQuery)
            }
            kentos_commands::CAD_SPACES_EDIT => {
                crate::arch_dispatch::arch_arm!($command, $then, spaces_edit, SpacesEdit)
            }
            kentos_commands::CAD_PROGRAM_EDIT => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                program_edit,
                kentos_commands::ProgramEdit
            ),
            kentos_commands::CAD_PROGRAM_CHECK => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                program_check,
                kentos_commands::ProgramCheck
            ),
            kentos_commands::CAD_SPACES_QUANTITIES => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                spaces_quantities,
                SpacesQuantities
            ),
            kentos_commands::CAD_VIEWS_EDIT => {
                crate::arch_dispatch::arch_arm!($command, $then, views_edit, ViewsEdit)
            }
            kentos_commands::CAD_REVIEW_NOTES_EDIT => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                review_notes,
                kentos_commands::ReviewNotesEdit
            ),
            kentos_commands::CAD_REVIEW_NOTES_LIST => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                review_notes_list,
                kentos_commands::ReviewNotesList
            ),
            // The building elements (moved from the core's part, docs/adr/8056 §1: they are architecture's).
            kentos_commands::CAD_COLUMNS_DISTANCES => crate::arch_dispatch::arch_arm!(
                $command,
                $then,
                columns_distances,
                kentos_commands::ColumnsDistancesInput
            ),
            kentos_commands::CAD_GRIDS_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, grids_create, GridsCreate)
            }
            kentos_commands::CAD_COLUMNS_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, columns_create, ColumnsCreate)
            }
            kentos_commands::CAD_STAIRS_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, stairs_create, StairsCreate)
            }
            kentos_commands::CAD_RAMPS_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, ramps_create, RampsCreate)
            }
            kentos_commands::CAD_BEAMS_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, beams_create, BeamsCreate)
            }
            kentos_commands::CAD_RAILINGS_CREATE => {
                crate::arch_dispatch::arch_arm!($command, $then, railings_create, RailingsCreate)
            }
            _ => $else,
        }
    };
}
pub(crate) use arch_commands;

/// One arm: the handler with `mod-arch` (docs/adr/8054 §11), else the answer that this build has no mimari commands.
/// The handler's module is never named in a build without it.
#[cfg(feature = "mod-arch")]
macro_rules! arch_arm {
    ($command:expr, $then:ident, $module:ident, $input:ty) => {
        $then!($module, $input)
    };
}
#[cfg(not(feature = "mod-arch"))]
macro_rules! arch_arm {
    ($command:expr, $then:ident, $module:ident, $input:ty) => {
        Err(crate::registry::not_built($command, "mimari"))
    };
}
pub(crate) use arch_arm;
