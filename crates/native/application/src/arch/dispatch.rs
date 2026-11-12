//! The command registry's part of architecture's commands (docs/adr/8054 §9): `arch_commands!` calls `$then!(module, InputType)`
//! for one of them, else gives `$else`. The registry (core/registry.rs) chains the parts; the handlers' names are
//! resolved where the registry calls it.

macro_rules! arch_commands {
    ($command:expr, $then:ident, $else:expr) => {
        match $command {
            kentos_commands::CAD_ARCH_IFC => {
                $then!(arch_ifc, kentos_commands::ExportIfcBuilding)
            }
            kentos_commands::CAD_WALLS_CREATE => $then!(walls_create, WallsCreate),
            kentos_commands::CAD_WALLS_RECTANGLE => $then!(walls_rectangle, WallsRectangle),
            kentos_commands::CAD_WALLS_PARTITION => $then!(walls_partition, WallsPartition),
            kentos_commands::CAD_WALLS_FROM_LINES => $then!(walls_from_lines, WallsFromLines),
            kentos_commands::CAD_ARCH_RECOGNIZE => {
                $then!(arch_recognize, kentos_commands::ArchRecognize)
            }
            kentos_commands::CAD_WALLS_EDIT => $then!(walls_edit, WallsEdit),
            kentos_commands::CAD_WALLS_JOINS => $then!(walls_joins, WallsJoins),
            kentos_commands::CAD_OPENINGS_CREATE => $then!(openings_create, OpeningsCreate),
            kentos_commands::CAD_OPENINGS_EDIT => $then!(openings_edit, OpeningsEdit),
            kentos_commands::CAD_OPENING_TYPES_EDIT => {
                $then!(opening_types_edit, kentos_commands::OpeningTypesEdit)
            }
            kentos_commands::CAD_MATERIALS_EDIT => {
                $then!(materials_edit, kentos_commands::MaterialsEdit)
            }
            kentos_commands::CAD_OPENINGS_QUERY => $then!(openings_query, kentos_commands::OpeningsQuery),
            kentos_commands::CAD_MODULES_EDIT => $then!(modules_edit, kentos_commands::ModulesEdit),
            kentos_commands::CAD_MODULES_QUERY => $then!(modules_query, kentos_commands::ModulesQuery),
            kentos_commands::CAD_OPTIONS_EDIT => $then!(options_edit, kentos_commands::OptionsEdit),
            kentos_commands::CAD_OPTIONS_QUERY => $then!(options_query, kentos_commands::OptionsQuery),
            kentos_commands::CAD_ELEVATIONS_EDIT => $then!(elevations_edit, kentos_commands::ElevationsEdit),
            kentos_commands::CAD_ELEVATIONS_QUERY => $then!(elevations_query, kentos_commands::ElevationsQuery),
            kentos_commands::CAD_SECTIONS_EDIT => $then!(sections_edit, kentos_commands::BuildingSectionsEdit),
            kentos_commands::CAD_SECTIONS_QUERY => $then!(sections_query, kentos_commands::BuildingSectionsQuery),
            kentos_commands::CAD_OPENINGS_DISTANCES => $then!(openings_distances, kentos_commands::OpeningsDistancesInput),
            kentos_commands::CAD_WALLS_DISTANCES => $then!(walls_distances, kentos_commands::WallsDistancesInput),
            kentos_commands::CAD_ARCH_AUDIT => $then!(arch_audit, kentos_commands::ArchAudit),
            kentos_commands::CAD_ARCH_EXCEPTIONS => {
                $then!(arch_exceptions, kentos_commands::ArchExceptions)
            }
            kentos_commands::CAD_CASEWORK_RUN => $then!(casework_run, kentos_commands::CaseworkRun),
            kentos_commands::CAD_CASEWORK_EDIT => $then!(casework_edit, kentos_commands::CaseworkEdit),
            kentos_commands::CAD_CASEWORK_QUERY => $then!(casework_query, kentos_commands::CaseworkQuery),
            kentos_commands::CAD_LEVELS_ASSIGN => $then!(levels_assign, kentos_commands::LevelsAssign),
            kentos_commands::CAD_NORTH_CREATE => $then!(north, kentos_commands::NorthCreate),
            kentos_commands::CAD_REVISIONS_MARK => $then!(revisions, kentos_commands::RevisionsMark),
            kentos_commands::CAD_LEGENDS_MATERIALS => $then!(legends, kentos_commands::LegendsMaterials),
            kentos_commands::CAD_SPACES_CREATE => $then!(spaces_create, SpacesCreate),
            kentos_commands::CAD_SLABS_CREATE => $then!(slabs_create, kentos_commands::SlabsCreate),
            kentos_commands::CAD_SLABS_EDIT => $then!(slabs_edit, kentos_commands::SlabsEdit),
            kentos_commands::CAD_SPACES_DISCOVER => $then!(spaces_discover, kentos_commands::SpacesDiscover),
            kentos_commands::CAD_SPACES_NUMBER => $then!(spaces_number, kentos_commands::SpacesNumber),
            kentos_commands::CAD_SPACES_SEPARATORS => $then!(spaces_separators, kentos_commands::SpacesSeparators),
            kentos_commands::CAD_ROOFS_CREATE => $then!(roofs_create, RoofsCreate),
            kentos_commands::CAD_ARCH_EDIT => $then!(arch_edit, ArchEdit),
            kentos_commands::CAD_TAGS_CREATE => $then!(tags_create, TagsCreate),
            kentos_commands::CAD_TAGS_CHECK => $then!(tags_check, TagsCheck),
            kentos_commands::CAD_TAGS_EDIT => $then!(tags_edit, TagsEdit),
            kentos_commands::CAD_VIEWMARKS_CREATE => $then!(viewmarks_create, ViewMarksCreate),
            kentos_commands::CAD_VIEWMARKS_EDIT => $then!(viewmarks_edit, ViewMarksEdit),
            kentos_commands::CAD_VIEWMARKS_REFERENCES => {
                $then!(viewmarks_references, ViewMarksReferences)
            }
            kentos_commands::CAD_SPACES_FINISH => $then!(spaces_finish, SpacesFinish),
            kentos_commands::CAD_TEMPLATES_APPLY => $then!(templates_apply, TemplatesApply),
            kentos_commands::CAD_SCHEDULES_CREATE => $then!(schedules_create, SchedulesCreate),
            kentos_commands::CAD_SCHEDULES_EDIT => $then!(schedules_edit, SchedulesEdit),
            kentos_commands::CAD_SCHEDULES_QUERY => $then!(schedules_query, SchedulesQuery),
            kentos_commands::CAD_SPACES_EDIT => $then!(spaces_edit, SpacesEdit),
            kentos_commands::CAD_PROGRAM_EDIT => $then!(program_edit, kentos_commands::ProgramEdit),
            kentos_commands::CAD_PROGRAM_CHECK => $then!(program_check, kentos_commands::ProgramCheck),
            kentos_commands::CAD_SPACES_QUANTITIES => $then!(spaces_quantities, SpacesQuantities),
            kentos_commands::CAD_VIEWS_EDIT => $then!(views_edit, ViewsEdit),
            kentos_commands::CAD_REVIEW_NOTES_EDIT => $then!(review_notes, kentos_commands::ReviewNotesEdit),
            kentos_commands::CAD_REVIEW_NOTES_LIST => $then!(review_notes_list, kentos_commands::ReviewNotesList),
            _ => $else,
        }
    };
}
pub(crate) use arch_commands;
