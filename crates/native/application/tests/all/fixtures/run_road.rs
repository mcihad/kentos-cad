//! The fixture runner's road commands (docs/adr/8054 §9): `road_cases!` calls `$then!(module, InputType)` for one
//! of them, else gives `$else`; run_op chains the parts.

macro_rules! road_cases {
    ($command:expr, $then:ident, $else:expr) => {
        match $command {
            kentos_commands::CAD_ROADS_EDIT => $then!(roads_edit, kentos_commands::RoadsEdit),
            kentos_commands::ROAD_TERRAINS_CREATE => {
                $then!(road_terrains_create, kentos_commands::TerrainsCreate)
            }
            kentos_commands::ROAD_TERRAINS_EDIT => {
                $then!(road_terrains_edit, kentos_commands::TerrainsEdit)
            }
            kentos_commands::ROAD_TERRAINS_QUERY => {
                $then!(road_terrains_query, kentos_commands::TerrainsQuery)
            }
            kentos_commands::ROAD_TERRAINS_CONTOURS => {
                $then!(road_terrains_contours, kentos_commands::TerrainsContours)
            }
            kentos_commands::ROAD_TERRAINS_SHEET_CONTOURS => {
                $then!(road_terrains_sheet, kentos_commands::TerrainsSheetContours)
            }
            kentos_commands::ROAD_TERRAINS_SECTIONCHECK => {
                $then!(road_terrains_section, kentos_commands::TerrainsSectionCheck)
            }
            kentos_commands::ROAD_TERRAINS_ZEROLINE => {
                $then!(road_terrains_zeroline, kentos_commands::TerrainsZeroLine)
            }
            kentos_commands::ROAD_TERRAINS_CUTFILL => {
                $then!(road_terrains_cutfill, kentos_commands::TerrainsCutFill)
            }
            kentos_commands::ROAD_ALIGNMENTS_CREATE => {
                $then!(road_alignments_create, kentos_commands::AlignmentsCreate)
            }
            kentos_commands::ROAD_ALIGNMENTS_EDIT => {
                $then!(road_alignments_edit, kentos_commands::AlignmentsEdit)
            }
            kentos_commands::ROAD_ALIGNMENTS_QUERY => {
                $then!(road_alignments_query, kentos_commands::AlignmentsQuery)
            }
            kentos_commands::ROAD_ALIGNMENTS_LABEL => {
                $then!(road_alignments_label, kentos_commands::AlignmentsLabel)
            }
            kentos_commands::ROAD_PROFILES_CREATE => {
                $then!(road_profiles_create, kentos_commands::ProfilesCreate)
            }
            kentos_commands::ROAD_PROFILES_EDIT => {
                $then!(road_profiles_edit, kentos_commands::ProfilesEdit)
            }
            kentos_commands::ROAD_PROFILES_QUERY => {
                $then!(road_profiles_query, kentos_commands::ProfilesQuery)
            }
            kentos_commands::ROAD_PROFILES_DRAW => {
                $then!(road_profiles_draw, kentos_commands::ProfilesDraw)
            }
            kentos_commands::ROAD_PROFILES_SIGHT => {
                $then!(road_profiles_sight, kentos_commands::ProfilesSight)
            }
            kentos_commands::ROAD_PROFILES_PROPOSE => {
                $then!(road_profiles_propose, kentos_commands::ProfilesPropose)
            }
            kentos_commands::ROAD_ALIGNMENTS_CHECK => {
                $then!(road_alignments_check, kentos_commands::AlignmentsCheck)
            }
            kentos_commands::ROAD_PROFILES_CHECK => {
                $then!(road_profiles_check, kentos_commands::RoadProfilesCheck)
            }
            kentos_commands::ROAD_PROFILES_LENGTHS => {
                $then!(road_profiles_lengths, kentos_commands::RoadProfilesLengths)
            }
            kentos_commands::ROAD_TEMPLATES_EDIT => {
                $then!(road_templates_edit, kentos_commands::TemplatesEdit)
            }
            kentos_commands::ROAD_SECTIONS_EDIT => {
                $then!(road_sections_edit, kentos_commands::SectionsEdit)
            }
            kentos_commands::ROAD_SECTIONS_QUERY => {
                $then!(road_sections_query, kentos_commands::SectionsQuery)
            }
            kentos_commands::ROAD_SECTIONS_DRAW => {
                $then!(road_sections_draw, kentos_commands::SectionsDraw)
            }
            kentos_commands::ROAD_SUPERELEVATIONS_EDIT => {
                $then!(road_superelevations_edit, kentos_commands::SuperelevationsEdit)
            }
            kentos_commands::ROAD_SUPERELEVATIONS_QUERY => {
                $then!(road_superelevations_query, kentos_commands::SuperelevationsQuery)
            }
            kentos_commands::ROAD_SUPERELEVATIONS_CHECK => {
                $then!(road_superelevations_check, kentos_commands::SuperelevationsCheck)
            }
            kentos_commands::ROAD_SUPERELEVATIONS_WIDEN => {
                $then!(road_superelevations_widen, kentos_commands::SuperelevationsWiden)
            }
            kentos_commands::ROAD_SUPERELEVATIONS_TRANSITIONS => {
                $then!(road_superelevations_transitions, kentos_commands::SuperelevationsTransitions)
            }
            kentos_commands::ROAD_CORRIDORS_EDIT => {
                $then!(road_corridors_edit, kentos_commands::CorridorsEdit)
            }
            kentos_commands::ROAD_CORRIDORS_QUERY => {
                $then!(road_corridors_query, kentos_commands::CorridorsQuery)
            }
            kentos_commands::ROAD_CORRIDORS_WATER => {
                $then!(road_corridors_water, kentos_commands::CorridorsWater)
            }
            kentos_commands::ROAD_CORRIDORS_DAYLIGHTS => {
                $then!(road_corridors_daylights, kentos_commands::CorridorsDaylights)
            }
            kentos_commands::ROAD_CORRIDORS_PROTECTED => {
                $then!(road_corridors_protected, kentos_commands::CorridorsProtected)
            }
            kentos_commands::ROAD_CORRIDORS_PREFLIGHT => {
                $then!(road_corridors_preflight, kentos_commands::CorridorsPreflight)
            }
            kentos_commands::ROAD_CORRIDORS_DRAW => {
                $then!(road_corridors_draw, kentos_commands::CorridorsDraw)
            }
            kentos_commands::ROAD_VOLUMES_EDIT => {
                $then!(road_volumes_edit, kentos_commands::VolumesEdit)
            }
            kentos_commands::ROAD_VOLUMES_QUERY => {
                $then!(road_volumes_query, kentos_commands::VolumesQuery)
            }
            kentos_commands::ROAD_VOLUMES_APPROVE => {
                $then!(road_volumes_approve, kentos_commands::VolumesApprove)
            }
            kentos_commands::ROAD_VOLUMES_UNCERTAINTY => {
                $then!(road_volumes_uncertainty, kentos_commands::VolumesUncertainty)
            }
            kentos_commands::ROAD_VOLUMES_PROGRESS => {
                $then!(road_volumes_progress, kentos_commands::VolumesProgress)
            }
            kentos_commands::ROAD_VOLUMES_COMPARE => {
                $then!(road_volumes_compare, kentos_commands::VolumesCompare)
            }
            kentos_commands::ROAD_VOLUMES_LEDGER => {
                $then!(road_volumes_ledger, kentos_commands::VolumesLedger)
            }
            kentos_commands::ROAD_VOLUMES_STATES => {
                $then!(road_volumes_states, kentos_commands::VolumesStates)
            }
            kentos_commands::ROAD_PROFILES_IMPACT => {
                $then!(road_profiles_impact, kentos_commands::ProfilesImpact)
            }
            kentos_commands::ROAD_ALIGNMENTS_IMPACT => {
                $then!(road_alignments_impact, kentos_commands::AlignmentsImpact)
            }
            kentos_commands::ROAD_VOLUMES_HISTORY => {
                $then!(road_volumes_history, kentos_commands::VolumesHistory)
            }
            kentos_commands::ROAD_HAULS_EDIT => {
                $then!(road_hauls_edit, kentos_commands::HaulsEdit)
            }
            kentos_commands::ROAD_HAULS_PLAN => {
                $then!(road_hauls_plan, kentos_commands::HaulsPlan)
            }
            kentos_commands::ROAD_HAULS_APPROVE => {
                $then!(road_hauls_approve, kentos_commands::HaulsApprove)
            }
            kentos_commands::ROAD_HAULS_NETWORK => {
                $then!(road_hauls_network, kentos_commands::HaulsNetwork)
            }
            kentos_commands::ROAD_HAULS_TRUCKS => {
                $then!(road_hauls_trucks, kentos_commands::HaulsTrucks)
            }
            kentos_commands::ROAD_HAULS_QUERY => {
                $then!(road_hauls_query, kentos_commands::HaulsQuery)
            }
            kentos_commands::ROAD_GUARDRAILS_EDIT => {
                $then!(road_guardrails_edit, kentos_commands::GuardrailsEdit)
            }
            kentos_commands::ROAD_GUARDRAILS_QUERY => {
                $then!(road_guardrails_query, kentos_commands::GuardrailsQuery)
            }
            kentos_commands::ROAD_GUARDRAILS_DRAW => {
                $then!(road_guardrails_draw, kentos_commands::GuardrailsDraw)
            }
            kentos_commands::ROAD_CULVERTS_EDIT => {
                $then!(road_culverts_edit, kentos_commands::CulvertsEdit)
            }
            kentos_commands::ROAD_CULVERTS_QUERY => {
                $then!(road_culverts_query, kentos_commands::CulvertsQuery)
            }
            kentos_commands::ROAD_CULVERTS_DRAW => {
                $then!(road_culverts_draw, kentos_commands::CulvertsDraw)
            }
            kentos_commands::ROAD_CULVERTS_HYDRAULICS => {
                $then!(road_culverts_hydraulics, kentos_commands::CulvertsHydraulics)
            }
            kentos_commands::ROAD_CULVERTS_CLOSURE => {
                $then!(road_culverts_closure, kentos_commands::CulvertsClosure)
            }
            kentos_commands::ROAD_WALLS_EDIT => {
                $then!(road_walls_edit, kentos_commands::RetainingWallsEdit)
            }
            kentos_commands::ROAD_WALLS_QUERY => {
                $then!(road_walls_query, kentos_commands::RetainingWallsQuery)
            }
            kentos_commands::ROAD_WALLS_DRAW => {
                $then!(road_walls_draw, kentos_commands::RetainingWallsDraw)
            }
            kentos_commands::ROAD_TERRAINS_FROM_CLOUD => {
                $then!(road_terrain_cloud, kentos_commands::TerrainsFromCloud)
            }
            kentos_commands::ROAD_DRAINAGE_QUERY => {
                $then!(road_drainage, kentos_commands::DrainageQuery)
            }
            kentos_commands::ROAD_DRAINAGE_DRAW => {
                $then!(road_drainage_draw, kentos_commands::DrainageDraw)
            }
            kentos_commands::ROAD_SWEPT_QUERY => {
                $then!(road_swept, kentos_commands::SweptQuery)
            }
            kentos_commands::ROAD_SWEPT_DRAW => {
                $then!(road_swept_draw, kentos_commands::SweptDraw)
            }
            kentos_commands::ROAD_TERRAINS_ANALYSIS => {
                $then!(road_terrains_analysis, kentos_commands::TerrainsAnalysis)
            }
            kentos_commands::ROAD_TERRAINS_COMPARE => {
                $then!(road_terrains_compare, kentos_commands::TerrainsCompare)
            }
            kentos_commands::ROAD_TERRAINS_QUALITY => {
                $then!(road_terrains_quality, kentos_commands::TerrainsQuality)
            }
            kentos_commands::ROAD_TERRAINS_REPAIR => {
                $then!(road_terrains_repair, kentos_commands::TerrainsRepair)
            }
            kentos_commands::ROAD_TERRAINS_COVERAGE => {
                $then!(road_terrains_coverage, kentos_commands::TerrainsCoverage)
            }
            kentos_commands::ROAD_TERRAINS_LOW_CONFIDENCE_DRAW => {
                $then!(road_terrains_coverage_draw, kentos_commands::TerrainsCoverageDraw)
            }
            kentos_commands::ROAD_TERRAINS_DENSITY => {
                $then!(road_terrains_density, kentos_commands::TerrainsDensity)
            }
            kentos_commands::ROAD_TERRAINS_STOCKPILE => {
                $then!(road_terrains_stockpile, kentos_commands::TerrainsStockpile)
            }
            kentos_commands::ROAD_TERRAINS_SPARSE_DRAW => {
                $then!(road_terrains_sparse, kentos_commands::TerrainsSparseDraw)
            }
            kentos_commands::ROAD_PLATFORMS_EDIT => {
                $then!(road_platforms_edit, kentos_commands::PlatformsEdit)
            }
            kentos_commands::ROAD_PLATFORMS_QUERY => {
                $then!(road_platforms_query, kentos_commands::PlatformsQuery)
            }
            kentos_commands::ROAD_PLATFORMS_DRAW => {
                $then!(road_platforms_draw, kentos_commands::PlatformsDraw)
            }
            kentos_commands::ROAD_PAY_EDIT => {
                $then!(road_pay_edit, kentos_commands::PayEdit)
            }
            kentos_commands::ROAD_PAY_QUERY => {
                $then!(road_pay_query, kentos_commands::PayQuery)
            }
            kentos_commands::ROAD_ROUTES_FIND => {
                $then!(road_routes_find, kentos_commands::RoutesFind)
            }
            kentos_commands::ROAD_ROUTES_DRAW => {
                $then!(road_routes_draw, kentos_commands::RoutesDraw)
            }
            kentos_commands::ROAD_JUNCTIONS_EDIT => {
                $then!(road_junctions_edit, kentos_commands::JunctionsEdit)
            }
            kentos_commands::ROAD_JUNCTIONS_QUERY => {
                $then!(road_junctions_query, kentos_commands::JunctionsQuery)
            }
            kentos_commands::ROAD_PROJECT_QUERY => {
                $then!(road_project_query, kentos_commands::RoadProjectQuery)
            }
            kentos_commands::ROAD_PROJECT_GRAPH => {
                $then!(road_project_graph, kentos_commands::RoadProjectGraphQuery)
            }
            kentos_commands::ROAD_PROJECT_REBUILD => {
                $then!(road_project_rebuild, kentos_commands::RoadProjectRebuild)
            }
            kentos_commands::ROAD_PROJECT_PRODUCE => {
                $then!(road_project_produce, kentos_commands::RoadProjectProduce)
            }
            kentos_commands::ROAD_PROJECT_EDIT => {
                $then!(road_project_edit, kentos_commands::RoadProjectEdit)
            }
            kentos_commands::ROAD_JUNCTIONS_DRAW => {
                $then!(road_junctions_draw, kentos_commands::JunctionsDraw)
            }
            kentos_commands::ROAD_JUNCTIONS_CLEARANCE => {
                $then!(road_junctions_clearance, kentos_commands::JunctionsClearance)
            }
            kentos_commands::ROAD_JUNCTIONS_TIE => {
                $then!(road_junctions_tie, kentos_commands::JunctionsTie)
            }
            kentos_commands::ROAD_JUNCTIONS_OVERLAP => {
                $then!(road_junctions_overlap, kentos_commands::JunctionsOverlap)
            }
            kentos_commands::ROAD_STAKEOUT_QUERY => {
                $then!(road_stakeout, kentos_commands::StakeoutQuery)
            }
            kentos_commands::FILE_EXPORT_LANDXML => {
                $then!(road_landxml, kentos_commands::ExportLandxml)
            }
            kentos_commands::FILE_EXPORT_GLTF => {
                $then!(road_scene, kentos_commands::ExportGltf)
            }
            kentos_commands::FILE_EXPORT_IFC => {
                $then!(road_ifc, kentos_commands::ExportIfc)
            }
            kentos_commands::ROAD_STAKEOUT_SEND => {
                $then!(road_stakeout_send, kentos_commands::StakeoutSend)
            }
            kentos_commands::ROAD_SHEETS_PLAN => {
                $then!(road_sheets, kentos_commands::RoadSheetsPlan)
            }
            kentos_commands::ROAD_SHEETS_SECTIONS => {
                $then!(road_sheets_sections, kentos_commands::RoadSheetsSections)
            }
            kentos_commands::ROAD_ALIGNMENTS_FIT => {
                $then!(road_alignments_fit, kentos_commands::AlignmentsFit)
            }
            kentos_commands::ROAD_PROFILES_FIT => {
                $then!(road_profiles_fit, kentos_commands::ProfilesFit)
            }
            kentos_commands::ROAD_ALIGNMENTS_IMPORT => {
                $then!(road_alignments_import, kentos_commands::AlignmentsImport)
            }
            _ => $else,
        }
    };
}
pub(super) use road_cases;
