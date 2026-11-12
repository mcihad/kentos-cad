//! The fixture runner's arch commands (docs/adr/8054 §9): `arch_cases!` calls `$then!(module, InputType)` for one
//! of them, else gives `$else`; run_op chains the parts.

macro_rules! arch_cases {
    ($command:expr, $then:ident, $else:expr) => {
        match $command {
            kentos_commands::CAD_REVIEW_NOTES_EDIT => {
                $then!(review_notes, kentos_commands::ReviewNotesEdit)
            }
            kentos_commands::CAD_REVIEW_NOTES_LIST => {
                use kentos_native_application::review_notes::list_cmd as review_notes_list;
                $then!(review_notes_list, kentos_commands::ReviewNotesList)
            }
            kentos_commands::CAD_WALLS_CREATE => {
                $then!(walls_create, kentos_commands::WallsCreate)
            }
            kentos_commands::CAD_WALLS_RECTANGLE => {
                $then!(walls_rectangle, kentos_commands::WallsRectangle)
            }
            kentos_commands::CAD_WALLS_PARTITION => {
                $then!(walls_partition, kentos_commands::WallsPartition)
            }
            kentos_commands::CAD_WALLS_FROM_LINES => {
                $then!(walls_from_lines, kentos_commands::WallsFromLines)
            }
            kentos_commands::CAD_ARCH_RECOGNIZE => {
                $then!(arch_recognize, kentos_commands::ArchRecognize)
            }
            kentos_commands::CAD_WALLS_EDIT => {
                $then!(walls_edit, kentos_commands::WallsEdit)
            }
            kentos_commands::CAD_WALLS_JOINS => {
                $then!(walls_joins, kentos_commands::WallsJoins)
            }
            kentos_commands::CAD_OPENINGS_CREATE => {
                $then!(openings_create, kentos_commands::OpeningsCreate)
            }
            kentos_commands::CAD_OPENINGS_EDIT => {
                $then!(openings_edit, kentos_commands::OpeningsEdit)
            }
            kentos_commands::CAD_OPENINGS_DISTANCES => {
                $then!(openings_distances, kentos_commands::OpeningsDistancesInput)
            }
            // Kapı ve pencere tipleri (docs/adr/0772).
            kentos_commands::CAD_MATERIALS_EDIT => {
                $then!(materials_edit, kentos_commands::MaterialsEdit)
            }
            kentos_commands::CAD_OPENING_TYPES_EDIT => {
                $then!(opening_types_edit, kentos_commands::OpeningTypesEdit)
            }
            kentos_commands::CAD_OPENINGS_QUERY => {
                $then!(openings_query, kentos_commands::OpeningsQuery)
            }
            // Oda modülleri (docs/adr/0775).
            kentos_commands::CAD_MODULES_EDIT => $then!(modules_edit, kentos_commands::ModulesEdit),
            kentos_commands::CAD_MODULES_QUERY => $then!(modules_query, kentos_commands::ModulesQuery),
            // Tasarım seçenekleri (docs/adr/0788).
            kentos_commands::CAD_OPTIONS_EDIT => $then!(options_edit, kentos_commands::OptionsEdit),
            kentos_commands::CAD_OPTIONS_QUERY => $then!(options_query, kentos_commands::OptionsQuery),
            // Oda iç görünüşü (docs/adr/0799).
            kentos_commands::CAD_ELEVATIONS_EDIT => $then!(elevations_edit, kentos_commands::ElevationsEdit),
            kentos_commands::CAD_ELEVATIONS_QUERY => $then!(elevations_query, kentos_commands::ElevationsQuery),
            kentos_commands::CAD_SECTIONS_EDIT => $then!(sections_edit, kentos_commands::BuildingSectionsEdit),
            kentos_commands::CAD_SECTIONS_QUERY => $then!(sections_query, kentos_commands::BuildingSectionsQuery),
            kentos_commands::CAD_WALLS_DISTANCES => {
                $then!(walls_distances, kentos_commands::WallsDistancesInput)
            }
            kentos_commands::CAD_ARCH_AUDIT => {
                $then!(arch_audit, kentos_commands::ArchAudit)
            }
            kentos_commands::CAD_ARCH_EXCEPTIONS => {
                $then!(arch_exceptions, kentos_commands::ArchExceptions)
            }
            kentos_commands::CAD_CASEWORK_RUN => {
                $then!(casework_run, kentos_commands::CaseworkRun)
            }
            kentos_commands::CAD_CASEWORK_EDIT => {
                $then!(casework_edit, kentos_commands::CaseworkEdit)
            }
            kentos_commands::CAD_CASEWORK_QUERY => {
                $then!(casework_query, kentos_commands::CaseworkQuery)
            }
            kentos_commands::CAD_NORTH_CREATE => $then!(north, kentos_commands::NorthCreate),
            kentos_commands::CAD_REVISIONS_MARK => $then!(revisions, kentos_commands::RevisionsMark),
            kentos_commands::CAD_LEGENDS_MATERIALS => $then!(legends, kentos_commands::LegendsMaterials),
            kentos_commands::CAD_LEVELS_ASSIGN => {
                $then!(levels_assign, kentos_commands::LevelsAssign)
            }
            kentos_commands::CAD_SPACES_CREATE => {
                $then!(spaces_create, kentos_commands::SpacesCreate)
            }
            kentos_commands::CAD_SLABS_CREATE => $then!(slabs_create, kentos_commands::SlabsCreate),
            kentos_commands::CAD_SLABS_EDIT => $then!(slabs_edit, kentos_commands::SlabsEdit),
            kentos_commands::CAD_SPACES_DISCOVER => {
                $then!(spaces_discover, kentos_commands::SpacesDiscover)
            }
            kentos_commands::CAD_SPACES_NUMBER => {
                $then!(spaces_number, kentos_commands::SpacesNumber)
            }
            kentos_commands::CAD_SPACES_SEPARATORS => {
                $then!(spaces_separators,kentos_commands::SpacesSeparators)
            }
            kentos_commands::CAD_ROOFS_CREATE => {
                $then!(roofs_create, kentos_commands::RoofsCreate)
            }
            kentos_commands::CAD_ARCH_EDIT => {
                $then!(arch_edit, kentos_commands::ArchEdit)
            }
            kentos_commands::CAD_TAGS_CREATE => {
                $then!(tags_create, kentos_commands::TagsCreate)
            }
            kentos_commands::CAD_TAGS_CHECK => {
                $then!(tags_check, kentos_commands::TagsCheck)
            }
            kentos_commands::CAD_TAGS_EDIT => {
                $then!(tags_edit, kentos_commands::TagsEdit)
            }
            kentos_commands::CAD_VIEWMARKS_CREATE => {
                $then!(viewmarks_create, kentos_commands::ViewMarksCreate)
            }
            kentos_commands::CAD_VIEWMARKS_EDIT => {
                $then!(viewmarks_edit, kentos_commands::ViewMarksEdit)
            }
            kentos_commands::CAD_VIEWMARKS_REFERENCES => {
                $then!(viewmarks_references, kentos_commands::ViewMarksReferences)
            }
            kentos_commands::CAD_SPACES_FINISH => {
                $then!(spaces_finish, kentos_commands::SpacesFinish)
            }
            kentos_commands::CAD_TEMPLATES_APPLY => {
                $then!(templates_apply, kentos_commands::TemplatesApply)
            }
            kentos_commands::CAD_SCHEDULES_CREATE => {
                $then!(schedules_create, kentos_commands::SchedulesCreate)
            }
            kentos_commands::CAD_SCHEDULES_EDIT => {
                $then!(schedules_edit, kentos_commands::SchedulesEdit)
            }
            kentos_commands::CAD_SCHEDULES_QUERY => {
                $then!(schedules_query, kentos_commands::SchedulesQuery)
            }
            kentos_commands::CAD_SPACES_EDIT => {
                $then!(spaces_edit, kentos_commands::SpacesEdit)
            }
            kentos_commands::CAD_PROGRAM_EDIT => $then!(program_edit, kentos_commands::ProgramEdit),
            kentos_commands::CAD_PROGRAM_CHECK => $then!(program_check, kentos_commands::ProgramCheck),
            kentos_commands::CAD_SPACES_QUANTITIES => {
                $then!(spaces_quantities, kentos_commands::SpacesQuantities)
            }
            kentos_commands::CAD_ARCH_IFC => {
                $then!(arch_ifc, kentos_commands::ExportIfcBuilding)
            }
            kentos_commands::CAD_VIEWS_EDIT => $then!(views_edit, ViewsEdit),
            _ => $else,
        }
    };
}
pub(super) use arch_cases;
