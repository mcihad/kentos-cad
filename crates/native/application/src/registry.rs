//! The product commands by name (docs/adr/0013, 0253 §4): each command's
//! input read as its own type, run by its handler, its `CommandResult`
//! written back as the catalog's JSON. The protocol (`call.rs`), the change
//! set (`changeset.rs`), the headless host, Python and MCP reach a command
//! through here, so they all run the same handler the tools run.

use crate::{
    attachments_inspect, domains_edit, domains_export, fields_migrate, forms_export, forms_import,
    schema_describe,
};
use crate::{
    block_params, blocks_freeze, blocks_proxy, blocks_query, blocks_replace, file_import_block,
    file_import_dwg,
};
use kentos_commands::{
    AddressesDefine, AnnotationsScales, ArcCreate, ArraysEdit, BlocksDefine, BlocksEdit, CallOp,
    ChangesetApply, CircleCreate, CrsTransform, CurvesConstruct, CurvesEdit, DimensionsCreate,
    DimensionsEdit, DimensionsQuery, DimensionsReassociate, DrawingAudit, EntitiesArray,
    EntitiesBoundary, EntitiesBreak, EntitiesChamfer, EntitiesCleanup, EntitiesCopy,
    EntitiesCreate, EntitiesDelete, EntitiesDivide, EntitiesEdit, EntitiesExplode, EntitiesFillet,
    EntitiesHatch, EntitiesJoin, EntitiesLengthen, EntitiesMatch, EntitiesOffset, EntitiesOrder, EntitiesFlatten,
    EntitiesPaste, EntitiesPick, EntitiesRelate, EntitiesReverse, EntitiesSelect,
    EntitiesSetProperties, EntitiesStretch, EntitiesTransform, EntitiesTrim, EntitiesVertex,
    FieldsEdit, FieldsUsage, FontsReport, GisDerivation, GisDetach, GisTakeback, HatchPatternsEdit,
    HatchesEdit, ImportCsv, ImportDxf, ImportFgb, ImportGeojson, ImportGml, ImportGpkg, ImportKml,
    ImportMaplibre, ImportParquet, ImportPdf, ImportQml, ImportShp, ImportSld, LabelsPin,
    LabelsReport, LayersCompose, LayersContingents, LayersEdit, LayersFilter, LayersFind,
    LayersIndexes, LayersInfo, LayersLabels, LayersRenderer, LayersRules, LayersService, LayersSet,
    LayersStates, LayersSubtypes, LayersTable, LayersTime, LayersTranslate, LeadersEdit,
    LibraryEdit, LineCreate, LineTypesEdit, NetworkDefine, NurbsCreate, PointCreate,
    PointsConstruct, PolygonCreate, PolylineCreate, RectangleCreate, RegularPolygonCreate,
    ScenariosEdit, SettingsSet, SheetsCreate, SheetsEdit, SheetsFit, SnapFind, StandardsApply,
    StandardsCheck, StylesEdit, StylesImpact, TablesEdit, TextsCheck, TextsEdit, UcsSet,
    UnitsChange, ViewsEdit, WallsCreate, WallsEdit, WallsFromLines, WallsPartition, WallsRectangle,
    codes,
};
use kentos_commands::{
    ArchEdit, AssetsCheck, AttachmentsInspect, BlocksFreeze, BlocksInsert, BlocksParameters,
    BlocksProxy, BlocksQuery, BlocksReplace, ColumnsCreate, ConstraintsEdit, ConstraintsQuery,
    DomainsEdit, DomainsExport, FieldsMigrate, FieldsWrite, FormsExport, FormsImport, GridsCreate,
    ImagesEdit, ImportBlock, InsertsClip, OpeningsCreate, OpeningsEdit, ParametersEdit,
    ProcessingBatch, ProcessingRun, ProcessingTools, RampsCreate, BeamsCreate, RastersGeoref, RastersMosaic,
    RastersReproject, RecordsPage, SchedulesCreate, SchedulesEdit, SchedulesQuery, SchemaDescribe, SheetsSet, NavigatorRead,
    SourcesDescribe, SourcesEdit, SourcesQuery, SourcesReconcile, SourcesRelate, SourcesSync,
    SpacesCreate, SpacesEdit, SpacesFinish, SpacesQuantities, StairsCreate, TagsCheck, TagsCreate,
    TagsEdit, TemplatesApply, ViewMarksCreate, ViewMarksEdit, ViewMarksReferences, WallsJoins,
    XrefsEdit, XrefsList,
};
use serde_json::Value;
use kentos_commands::{DimensionSetsCreateInput, DimensionSetsEditInput, DimensionSetsQueryInput};
use crate::{dimension_sets_create, dimension_sets_edit, dimension_sets_query};

use crate::entities_select::isolation as entities_isolation;
use crate::addresses_check;
use crate::crs_factors;
use crate::map_features;
use crate::survey_baselines;
use crate::entities_grade;
use crate::entities_drape;
use crate::entities_raise;
use crate::topology_clean;
use crate::parcel_register;
use crate::parcel_register::list_cmd as parcel_register_list;
use crate::parcels_boundaries;
use crate::parcels_check;
use crate::pipes_check;
use crate::pipes_profile;
use crate::asbuilt_check;
use crate::field_package;
use crate::zoning_check;
use crate::zoning_symbolize;
use crate::scenarios_impact;
use crate::readjustment;
use crate::readjustment_scope;
use crate::readjustment_sheets;
use crate::orthophoto_check;
use crate::orthophoto_compare;
use crate::orthophoto_seamlines;
use crate::pointcloud_overlap;
use crate::pointcloud_register;
use crate::addresses_compare;
use crate::addresses_normalize;
use crate::blocks_insert;
use crate::blocks_find;
use crate::blocks_convert;
use crate::arrays_adopt;
use crate::arrays_find;
use crate::drawing_inspect;
use crate::donut_create;
use crate::drawing_compare;
use crate::fields_write;
use crate::gpkg_tiles::{read as gpkg_tiles_read, write as gpkg_tiles_write};
use crate::layers_crs::{assign as layers_crs_assign, reproject as layers_reproject};
use crate::mesh_areas::{profile as meshes_profile, region as meshes_region};
use crate::meshes::{describe as meshes_describe, query as meshes_query};
use crate::pointcloud_areas::{profile as pointclouds_profile, region as pointclouds_region};
use crate::pointclouds::{describe as pointclouds_describe, query as pointclouds_query};
use crate::profile_files::{
    import as profiles_import, list as profiles_list, migrate as profiles_migrate,
    remove as profiles_remove,
};
use crate::profiles::{apply as profiles_apply, check as profiles_check};
use crate::records_page;
use crate::sources::{
    describe as sources_describe, edit as sources_edit, reconcile as sources_reconcile,
    relate as sources_relate, sync as sources_sync,
};
use crate::sources_query;
use crate::standards_install;
use crate::standards_exceptions;
use crate::drawing_repair;
use crate::drawing_take;
use crate::review_notes;
use crate::review_notes::list_cmd as review_notes_list;
use crate::standards_palettes;
use crate::standards::{apply as standards_apply, check as standards_check, conflicts as standards_conflicts, resolve as standards_resolve};
use crate::time_query;
use crate::{sensors_ingest, sensors_latest};
use crate::spaces_separators;
use crate::spaces_number;
use crate::{
    DESKTOP_COMMANDS, ExecutionContext, address_define, adjoin_cmd, annotations_scales,
    annotations_write, arc, arch_audit, arch_edit, arch_exceptions, areas, array, arrays_edit,
    assets_check, blocks_define, blocks_edit, casework_edit, casework_query, casework_run,
    changeset, circle, columns_create, columns_distances, constraints_edit, constraints_query,
    constructions, create, crs_transform, curves_construct, curves_edit, delete, dimensions_create,
    dimensions_edit, dimensions_interior, dimensions_query, dimensions_reassociate, divide,
    drawing_audit, edit, entities_boundary, entities_break, entities_chamfer, entities_cleanup,
    entities_copy, entities_explode, entities_extend, entities_fillet, entities_hatch,
    entities_join, entities_lengthen, entities_match, entities_order, entities_flatten, entities_paste,
    entities_pick, entities_relate, entities_reverse, entities_select, entities_trim,
    entities_vertex, fields_edit, fields_usage, file_import, file_import_csv, file_import_fgb,
    file_import_geojson, file_import_gml, file_import_gpkg, file_import_kml, file_import_parquet,
    file_import_pdf, file_import_shp, fonts_report, gis_derivation, gis_detach, gis_takeback,
    grids_create, hatch_patterns_edit, hatches_edit, images_clip, images_edit, inserts_clip,
    labels_pin, labels_report, layers_compose, layers_contingents, layers_edit, layers_filter,
    layers_find, layers_indexes, layers_info, layers_labels, layers_renderer, layers_rules,
    layers_service, layers_set, layers_states, layers_subtypes, layers_table, layers_time,
    layers_translate, leaders_edit, legends, levels_assign, library_edit, line, line_types_edit,
    linearref_define, file_import_landxml, road_alignments_create, road_scene, road_alignments_edit, road_alignments_label, road_alignments_query, road_profiles_create, road_profiles_draw, road_profiles_edit, road_profiles_query, road_profiles_sight, road_profiles_propose, road_alignments_check, road_profiles_check, road_profiles_lengths, road_profiles_impact, road_alignments_impact, road_sections_draw, road_sections_edit, road_sections_query, road_superelevations_edit, road_superelevations_query, road_superelevations_check, road_superelevations_widen, road_superelevations_transitions, road_corridors_draw, road_corridors_edit, road_corridors_query, road_corridors_water, road_corridors_daylights, road_corridors_protected, road_corridors_preflight, road_volumes_approve, road_volumes_edit, road_volumes_query, road_volumes_uncertainty, road_volumes_progress, road_volumes_compare, road_volumes_ledger, road_volumes_states, road_volumes_history, road_hauls_edit, road_hauls_query, road_hauls_plan, road_hauls_approve, road_hauls_network, road_hauls_trucks, road_guardrails_edit, road_guardrails_query, road_guardrails_draw, road_culverts_edit, road_culverts_query, road_culverts_draw, road_culverts_hydraulics, road_culverts_closure, road_walls_edit, road_walls_query, road_walls_draw, road_platforms_edit, road_platforms_query, road_platforms_draw, road_terrain_cloud, road_drainage, road_drainage_draw, road_swept, road_swept_draw, road_terrains_analysis, road_terrains_compare, road_terrains_quality, road_terrains_repair, road_terrains_coverage, road_terrains_coverage_draw, road_terrains_density, road_terrains_stockpile, road_terrains_sparse, road_pay_edit, road_pay_query, road_routes_find, road_routes_draw, road_junctions_edit, road_junctions_query, road_junctions_draw, road_junctions_clearance, road_junctions_tie, road_junctions_overlap, road_project_query, road_project_graph, road_project_produce, road_project_rebuild, road_project_edit, map_work, map_work_edit, map_work_restore, map_survey, map_survey_edit, map_survey_network, map_features_reconcile, road_alignments_import, road_alignments_fit, road_profiles_fit, road_ifc, road_landxml, road_stakeout, road_stakeout_send, road_sheets, road_sheets_sections, road_templates_edit, road_terrains_contours, road_terrains_sheet, road_terrains_zeroline, road_terrains_cutfill, road_terrains_create, road_terrains_edit, road_terrains_query, maplibre, network_define, north, nurbs, objects_derive, offset,
    openings_create, openings_distances, openings_edit, opening_types_edit, materials_edit, openings_query, modules_edit, modules_query, options_edit, options_query, elevations_edit, elevations_query, sections_edit, sections_query, parameters_edit, parcels, point,
    points_construct, polygon, polygon_regular, polyline, qml, ramps_create, beams_create, rasters_georef,
    rasters_mosaic, rasters_reproject, rectangle_create, revcloud_create, revisions, roads_edit,
    scenarios_edit, extract_run, extract_table, schedules_create, schedules_edit, schedules_query, sector_create, set,
    settings_set, shape_edits, sheets_create, sheets_edit, sheets_fit, sheets_levels, sheets_details, plot_styles_import, sheets_set, navigator,
    slabs_create, slabs_edit, sld, snap_find, spaces_create, spaces_discover, spaces_edit, program_check, program_edit,
    spaces_finish, spaces_quantities, stairs_create, stretch, styles_edit, styles_impact,
    survey_adjust, survey_points, tables_edit, tags_check, tags_create, tags_edit, templates_apply,
    texts_check, texts_edit, time_version, transform, ucs_set, units_change, viewmarks_create,
    viewmarks_edit, viewmarks_references, views_edit, walls_create, walls_distances, walls_edit,
    walls_from_lines, walls_joins, walls_partition, walls_rectangle, xrefs_edit, xrefs_list,
};
use crate::{datasets_describe, datasets_edit, joins_edit, joins_query};
use crate::{
    forms_audit, forms_compare, forms_edit, forms_fill, forms_recompute, forms_sources,
    forms_submit, forms_test, forms_transition, forms_validate, records, records_duplicate,
    relations_edit,
};
use crate::{processing_batch, processing_run, processing_tools};
use crate::{stac_prepare, stac_search};
use crate::{subnetworks, subnetworks_update};
use crate::{topology_check, topology_exceptions, topology_repair};
use kentos_commands::AddressesCheck;
use kentos_commands::AddressesCompare;
use kentos_commands::AddressesNormalize;
use kentos_commands::DrawingCompare;
use kentos_commands::{DatasetsDescribe, DatasetsEdit, JoinsEdit, JoinsQuery};
use kentos_commands::{
    FormsAudit, FormsCompare, FormsEdit, FormsFill, FormsRecompute, FormsSources, FormsSubmit,
    FormsTest, FormsTransition, FormsValidate, RecordsCreate, RecordsDuplicate, RelationsEdit,
};
use kentos_commands::{
    MeshesDescribe, MeshesProfile, MeshesQuery, MeshesRegion, PointCloudsDescribe,
    PointCloudsProfile, PointCloudsQuery, PointCloudsRegion, TimeQuery,
};
use kentos_commands::{SensorsIngest, SensorsLatest};
use kentos_commands::{RastersGpkgRead, RastersGpkgWrite};
use kentos_commands::{StacPrepare, StacSearch};
use kentos_commands::{TopologyCheck, TopologyExceptions};

/// Why a command was not run: not a refusal of the command (that is its
/// `CommandResult`), but a command this host does not run or input that is
/// not its type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unrun {
    /// `unknown_command`, `invalid_input`.
    pub code: &'static str,
    pub message: String,
}

/// The version of `command` this host runs; none for a command it does not run.
pub fn version_of(command: &str) -> Option<u32> {
    DESKTOP_COMMANDS
        .iter()
        .find(|(name, _)| *name == command)
        .map(|(_, v)| *v)
}

fn unknown(command: &str) -> Unrun {
    Unrun {
        code: codes::UNKNOWN_COMMAND,
        message: format!("“{command}” bu çizimde çalışan bir komut değil."),
    }
}

fn unreadable(command: &str, e: &serde_json::Error) -> Unrun {
    Unrun {
        code: codes::INVALID_INPUT,
        message: format!("{command} girdisi okunamadı: {e}. Kataloğun girdi şemasına bakın."),
    }
}

// Drawing style records stay permissive for legacy KCAD files. A direct
// standards command must enforce the package's stricter resource contract
// before serde drops unrecognized fields during typed deserialization.
fn check_standard_resource_input(command: &str, input: &Value) -> Result<(), Unrun> {
    if command == kentos_commands::CAD_STANDARDS_CHECK {
        if let Some(resources) = input.get("resources").filter(|value| !value.is_null()) {
            kentos_commands::check_resource_fields(resources).map_err(|message| Unrun {
                code: codes::INVALID_INPUT,
                message: format!("{command} girdisi okunamadı: {message}"),
            })?;
        }
    }
    Ok(())
}

fn unwritable(command: &str, e: &serde_json::Error) -> Unrun {
    Unrun {
        code: codes::INVALID_INPUT,
        message: format!("{command} sonucu JSON'a yazılamadı: {e}"),
    }
}

/// Calls `$then!(module, InputType)` for `command`, or answers that it is unknown.
macro_rules! by_command {
    ($command:expr, $then:ident) => {
        match $command {
            kentos_commands::CAD_POLYGON_CREATE => $then!(polygon, PolygonCreate),
            kentos_commands::CAD_LINE_CREATE => $then!(line, LineCreate),
            kentos_commands::CAD_POLYLINE_CREATE => $then!(polyline, PolylineCreate),
            kentos_commands::CAD_ENTITIES_DELETE => $then!(delete, EntitiesDelete),
            kentos_commands::CAD_POINT_CREATE => $then!(point, PointCreate),
            kentos_commands::CAD_DONUT_CREATE => $then!(donut_create, kentos_commands::DonutCreate),
            kentos_commands::CAD_CIRCLE_CREATE => $then!(circle, CircleCreate),
            kentos_commands::CAD_ARC_CREATE => $then!(arc, ArcCreate),
            kentos_commands::CAD_ENTITIES_TRANSFORM => $then!(transform, EntitiesTransform),
            kentos_commands::CAD_ENTITIES_EDIT => $then!(edit, EntitiesEdit),
            kentos_commands::CAD_ENTITIES_ARRAY => $then!(array, EntitiesArray),
            kentos_commands::CAD_ENTITIES_CREATE => $then!(create, EntitiesCreate),
            kentos_commands::CAD_ENTITIES_SET => $then!(set, EntitiesSetProperties),
            kentos_commands::CAD_BLOCKS_DEFINE => $then!(blocks_define, BlocksDefine),
            kentos_commands::CAD_BLOCKS_EDIT => $then!(blocks_edit, BlocksEdit),
            kentos_commands::CAD_LAYERS_SERVICE => $then!(layers_service, LayersService),
            kentos_commands::CAD_NETWORK_DEFINE => $then!(network_define, NetworkDefine),
            kentos_commands::CAD_LINEARREF_DEFINE => {
                $then!(linearref_define, kentos_commands::LinearRefDefine)
            }
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
            kentos_commands::ROAD_TERRAINS_ZEROLINE => {
                $then!(road_terrains_zeroline, kentos_commands::TerrainsZeroLine)
            }
            kentos_commands::ROAD_TERRAINS_CUTFILL => {
                $then!(road_terrains_cutfill, kentos_commands::TerrainsCutFill)
            }
            kentos_commands::FILE_IMPORT_LANDXML => {
                $then!(file_import_landxml, kentos_commands::ImportLandxml)
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
            kentos_commands::MAP_WORK_QUERY => {
                $then!(map_work, kentos_commands::MapWorkQuery)
            }
            kentos_commands::MAP_WORK_EDIT => {
                $then!(map_work_edit, kentos_commands::MapWorkEdit)
            }
            kentos_commands::MAP_WORK_RESTORE => {
                $then!(map_work_restore, kentos_commands::MapWorkRestore)
            }
            kentos_commands::CAD_CRS_FACTORS => {
                $then!(crs_factors, kentos_commands::CrsFactors)
            }
            kentos_commands::MAP_FEATURES_CODE => {
                $then!(map_features, kentos_commands::FeaturesCode)
            }
            kentos_commands::CAD_SURVEY_BASELINES => {
                $then!(survey_baselines, kentos_commands::SurveyBaselines)
            }
            kentos_commands::CAD_ENTITIES_GRADE => {
                $then!(entities_grade, kentos_commands::EntitiesGrade)
            }
            kentos_commands::CAD_ENTITIES_RAISE => {
                $then!(entities_raise, kentos_commands::EntitiesRaise)
            }
            kentos_commands::CAD_ENTITIES_DRAPE => {
                $then!(entities_drape, kentos_commands::EntitiesDrape)
            }
            kentos_commands::CAD_TOPOLOGY_CLEAN => {
                $then!(topology_clean, kentos_commands::TopologyClean)
            }
            kentos_commands::CAD_PARCELS_CHECK => {
                $then!(parcels_check, kentos_commands::ParcelsCheck)
            }
            kentos_commands::CAD_PARCELS_BOUNDARIES => {
                $then!(parcels_boundaries, kentos_commands::ParcelsBoundaries)
            }
            kentos_commands::CAD_PARCELS_RECORD => $then!(parcel_register, kentos_commands::ParcelsRecord),
            kentos_commands::CAD_PARCELS_RECORDS => $then!(parcel_register_list, kentos_commands::ParcelsRecords),
            kentos_commands::CAD_PIPES_CHECK => {
                $then!(pipes_check, kentos_commands::PipesCheck)
            }
            kentos_commands::CAD_PIPES_PROFILE => {
                $then!(pipes_profile, kentos_commands::PipesProfile)
            }
            kentos_commands::CAD_ASBUILT_CHECK => {
                $then!(asbuilt_check, kentos_commands::AsbuiltCheck)
            }
            kentos_commands::CAD_FIELD_PACKAGE => {
                $then!(field_package, kentos_commands::FieldPackage)
            }
            kentos_commands::CAD_ZONING_CHECK => {
                $then!(zoning_check, kentos_commands::ZoningCheck)
            }
            kentos_commands::CAD_ZONING_SYMBOLIZE => {
                $then!(zoning_symbolize, kentos_commands::ZoningSymbolize)
            }
            kentos_commands::CAD_SCENARIOS_IMPACT => {
                $then!(scenarios_impact, kentos_commands::ScenariosImpact)
            }
            kentos_commands::CAD_READJUSTMENT_DISTRIBUTE => {
                $then!(readjustment, kentos_commands::ReadjustmentDistribute)
            }
            kentos_commands::CAD_READJUSTMENT_SCOPE => {
                $then!(readjustment_scope, kentos_commands::ReadjustmentScope)
            }
            kentos_commands::CAD_READJUSTMENT_SHEETS => {
                $then!(readjustment_sheets, kentos_commands::ReadjustmentSheets)
            }
            kentos_commands::CAD_ORTHOPHOTO_CHECK => {
                $then!(orthophoto_check, kentos_commands::OrthophotoCheck)
            }
            kentos_commands::CAD_ORTHOPHOTO_COMPARE => {
                $then!(orthophoto_compare, kentos_commands::OrthophotoCompare)
            }
            kentos_commands::CAD_ORTHOPHOTO_SEAMLINES => {
                $then!(orthophoto_seamlines, kentos_commands::OrthophotoSeamlines)
            }
            kentos_commands::CAD_POINTCLOUDS_OVERLAP => {
                $then!(pointcloud_overlap, kentos_commands::PointCloudsOverlap)
            }
            kentos_commands::CAD_POINTCLOUDS_REGISTER => {
                $then!(pointcloud_register, kentos_commands::PointCloudsRegister)
            }
            kentos_commands::MAP_SURVEY_QUERY => {
                $then!(map_survey, kentos_commands::MapSurveyQuery)
            }
            kentos_commands::MAP_SURVEY_EDIT => {
                $then!(map_survey_edit, kentos_commands::MapSurveyEdit)
            }
            kentos_commands::MAP_SURVEY_NETWORK => {
                $then!(map_survey_network, kentos_commands::MapSurveyNetwork)
            }
            kentos_commands::MAP_FEATURES_RECONCILE => {
                $then!(map_features_reconcile, kentos_commands::FeaturesReconcile)
            }
            kentos_commands::ROAD_STAKEOUT_QUERY => {
                $then!(road_stakeout, kentos_commands::StakeoutQuery)
            }
            kentos_commands::FILE_EXPORT_LANDXML => {
                $then!(road_landxml, kentos_commands::ExportLandxml)
            }
            kentos_commands::FILE_EXPORT_IFC => {
                $then!(road_ifc, kentos_commands::ExportIfc)
            }
            kentos_commands::FILE_EXPORT_GLTF => {
                $then!(road_scene, kentos_commands::ExportGltf)
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
            kentos_commands::CAD_SUBNETWORKS_FIND => {
                $then!(subnetworks, kentos_commands::SubnetworksFind)
            }
            kentos_commands::CAD_SUBNETWORKS_UPDATE => {
                $then!(subnetworks_update, kentos_commands::SubnetworksUpdate)
            }
            kentos_commands::CAD_ENTITIES_OFFSET => $then!(offset, EntitiesOffset),
            kentos_commands::CAD_POINTS_CONSTRUCT => $then!(points_construct, PointsConstruct),
            kentos_commands::CAD_CURVES_CONSTRUCT => $then!(curves_construct, CurvesConstruct),
            kentos_commands::CAD_SNAP_FIND => $then!(snap_find, SnapFind),
            kentos_commands::CAD_ENTITIES_ISOLATE => $then!(entities_isolation, kentos_commands::EntitiesIsolate),
            kentos_commands::CAD_ENTITIES_SELECT => $then!(entities_select, EntitiesSelect),
            kentos_commands::CAD_ENTITIES_PICK => $then!(entities_pick, EntitiesPick),
            kentos_commands::CAD_ENTITIES_RELATE => $then!(entities_relate, EntitiesRelate),
            kentos_commands::CAD_CRS_TRANSFORM => $then!(crs_transform, CrsTransform),
            kentos_commands::CAD_SOURCES_DESCRIBE => $then!(sources_describe, SourcesDescribe),
            kentos_commands::CAD_SOURCES_EDIT => $then!(sources_edit, SourcesEdit),
            kentos_commands::CAD_SOURCES_SYNC => $then!(sources_sync, SourcesSync),
            kentos_commands::CAD_SOURCES_RECONCILE => $then!(sources_reconcile, SourcesReconcile),
            kentos_commands::CAD_SOURCES_RELATE => $then!(sources_relate, SourcesRelate),
            kentos_commands::CAD_RECORDS_PAGE => $then!(records_page, RecordsPage),
            kentos_commands::CAD_SOURCES_QUERY => $then!(sources_query, SourcesQuery),
            kentos_commands::CAD_FIELDS_WRITE => $then!(fields_write, FieldsWrite),
            kentos_commands::CAD_PROCESSING_TOOLS => $then!(processing_tools, ProcessingTools),
            kentos_commands::CAD_PROCESSING_RUN => $then!(processing_run, ProcessingRun),
            kentos_commands::CAD_PROCESSING_BATCH => $then!(processing_batch, ProcessingBatch),
            kentos_commands::CAD_ENTITIES_VERTEX => $then!(entities_vertex, EntitiesVertex),
            kentos_commands::CAD_RECTANGLE_CREATE => $then!(rectangle_create, RectangleCreate),
            kentos_commands::CAD_POLYGON_REGULAR => $then!(polygon_regular, RegularPolygonCreate),
            kentos_commands::CAD_SECTOR_CREATE => {
                $then!(sector_create, kentos_commands::SectorCreate)
            }
            kentos_commands::CAD_REVCLOUD_CREATE => {
                $then!(revcloud_create, kentos_commands::RevcloudCreate)
            }
            kentos_commands::CAD_ENTITIES_ADJOIN => {
                $then!(adjoin_cmd, kentos_commands::EntitiesAdjoin)
            }
            kentos_commands::CAD_PARCELS_CREATE => $then!(parcels, kentos_commands::ParcelsCreate),
            kentos_commands::CAD_CONSTRUCTIONS_CREATE => {
                $then!(constructions, kentos_commands::ConstructionsCreate)
            }
            kentos_commands::CAD_AREAS_EDIT => $then!(areas, kentos_commands::AreasEdit),
            kentos_commands::CAD_ANNOTATIONS_WRITE => {
                $then!(annotations_write, kentos_commands::AnnotationsWrite)
            }
            kentos_commands::CAD_OBJECTS_DERIVE => {
                $then!(objects_derive, kentos_commands::ObjectsDerive)
            }
            kentos_commands::CAD_ROADS_EDIT => $then!(roads_edit, kentos_commands::RoadsEdit),
            kentos_commands::CAD_IMAGES_CLIP => $then!(images_clip, kentos_commands::ImagesClip),
            kentos_commands::CAD_TIME_VERSION => $then!(time_version, kentos_commands::TimeVersion),
            kentos_commands::CAD_SURVEY_POINTS => {
                $then!(survey_points, kentos_commands::SurveyPoints)
            }
            kentos_commands::CAD_SURVEY_ADJUST => {
                $then!(survey_adjust, kentos_commands::SurveyAdjust)
            }
            kentos_commands::CAD_SHAPES_EDIT => $then!(shape_edits, kentos_commands::ShapesEdit),
            kentos_commands::CAD_NURBS_CREATE => $then!(nurbs, NurbsCreate),
            kentos_commands::CAD_WALLS_CREATE => $then!(walls_create, WallsCreate),
            kentos_commands::CAD_WALLS_RECTANGLE => $then!(walls_rectangle, WallsRectangle),
            kentos_commands::CAD_WALLS_PARTITION => $then!(walls_partition, WallsPartition),
            kentos_commands::CAD_WALLS_FROM_LINES => $then!(walls_from_lines, WallsFromLines),
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
            kentos_commands::CAD_COLUMNS_DISTANCES => $then!(columns_distances, kentos_commands::ColumnsDistancesInput),
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
            kentos_commands::CAD_GRIDS_CREATE => $then!(grids_create, GridsCreate),
            kentos_commands::CAD_COLUMNS_CREATE => $then!(columns_create, ColumnsCreate),
            kentos_commands::CAD_STAIRS_CREATE => $then!(stairs_create, StairsCreate),
            kentos_commands::CAD_RAMPS_CREATE => $then!(ramps_create, RampsCreate),
            kentos_commands::CAD_BEAMS_CREATE => $then!(beams_create, BeamsCreate),
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
            kentos_commands::CAD_SHEETS_SET => $then!(sheets_set, SheetsSet),
            kentos_commands::CAD_NAVIGATOR_READ => $then!(navigator, NavigatorRead),
            kentos_commands::CAD_XREFS_EDIT => $then!(xrefs_edit, XrefsEdit),
            kentos_commands::CAD_XREFS_LIST => $then!(xrefs_list, XrefsList),
            kentos_commands::CAD_ASSETS_CHECK => $then!(assets_check, AssetsCheck),
            kentos_commands::CAD_IMAGES_EDIT => $then!(images_edit, ImagesEdit),
            kentos_commands::CAD_RASTERS_MOSAIC => $then!(rasters_mosaic, RastersMosaic),
            kentos_commands::CAD_RASTERS_REPROJECT => {
                $then!(rasters_reproject, RastersReproject)
            }
            kentos_commands::CAD_RASTERS_GPKGREAD => $then!(gpkg_tiles_read, RastersGpkgRead),
            kentos_commands::CAD_RASTERS_GPKGWRITE => $then!(gpkg_tiles_write, RastersGpkgWrite),
            kentos_commands::CAD_STAC_SEARCH => $then!(stac_search, StacSearch),
            kentos_commands::CAD_STAC_PREPARE => $then!(stac_prepare, StacPrepare),
            kentos_commands::CAD_POINTCLOUDS_DESCRIBE => {
                $then!(pointclouds_describe, PointCloudsDescribe)
            }
            kentos_commands::CAD_POINTCLOUDS_QUERY => $then!(pointclouds_query, PointCloudsQuery),
            kentos_commands::CAD_POINTCLOUDS_REGION => {
                $then!(pointclouds_region, PointCloudsRegion)
            }
            kentos_commands::CAD_POINTCLOUDS_PROFILE => {
                $then!(pointclouds_profile, PointCloudsProfile)
            }
            kentos_commands::CAD_MESHES_DESCRIBE => $then!(meshes_describe, MeshesDescribe),
            kentos_commands::CAD_MESHES_QUERY => $then!(meshes_query, MeshesQuery),
            kentos_commands::CAD_MESHES_REGION => $then!(meshes_region, MeshesRegion),
            kentos_commands::CAD_MESHES_PROFILE => $then!(meshes_profile, MeshesProfile),
            kentos_commands::CAD_TIME_QUERY => $then!(time_query, TimeQuery),
            kentos_commands::CAD_SENSORS_INGEST => $then!(sensors_ingest, SensorsIngest),
            kentos_commands::CAD_SENSORS_LATEST => $then!(sensors_latest, SensorsLatest),
            kentos_commands::CAD_INSERTS_CLIP => $then!(inserts_clip, InsertsClip),
            kentos_commands::CAD_RASTERS_GEOREF => $then!(rasters_georef, RastersGeoref),
            kentos_commands::CAD_SCHEDULES_CREATE => $then!(schedules_create, SchedulesCreate),
            kentos_commands::CAD_EXTRACT_RUN => $then!(extract_run, kentos_commands::ExtractRun),
            kentos_commands::CAD_EXTRACT_TABLE => $then!(extract_table, kentos_commands::ExtractTable),
            kentos_commands::CAD_SCHEDULES_EDIT => $then!(schedules_edit, SchedulesEdit),
            kentos_commands::CAD_SCHEDULES_QUERY => $then!(schedules_query, SchedulesQuery),
            kentos_commands::CAD_SPACES_EDIT => $then!(spaces_edit, SpacesEdit),
            kentos_commands::CAD_PROGRAM_EDIT => $then!(program_edit, kentos_commands::ProgramEdit),
            kentos_commands::CAD_PROGRAM_CHECK => $then!(program_check, kentos_commands::ProgramCheck),
            kentos_commands::CAD_SPACES_QUANTITIES => $then!(spaces_quantities, SpacesQuantities),
            kentos_commands::CAD_ANNOTATIONS_SCALES => {
                $then!(annotations_scales, AnnotationsScales)
            }
            kentos_commands::CAD_ENTITIES_ORDER => $then!(entities_order, EntitiesOrder),
            kentos_commands::CAD_ENTITIES_FLATTEN => $then!(entities_flatten, EntitiesFlatten),
            kentos_commands::CAD_UCS_SET => $then!(ucs_set, UcsSet),
            kentos_commands::CAD_VIEWS_EDIT => $then!(views_edit, ViewsEdit),
            kentos_commands::CAD_FORMS_EDIT => $then!(forms_edit, FormsEdit),
            kentos_commands::CAD_FORMS_VALIDATE => $then!(forms_validate, FormsValidate),
            kentos_commands::CAD_FORMS_SUBMIT => $then!(forms_submit, FormsSubmit),
            kentos_commands::CAD_FORMS_FILL => $then!(forms_fill, FormsFill),
            kentos_commands::CAD_FORMS_AUDIT => $then!(forms_audit, FormsAudit),
            kentos_commands::CAD_FORMS_TEST => $then!(forms_test, FormsTest),
            kentos_commands::CAD_FORMS_COMPARE => $then!(forms_compare, FormsCompare),
            kentos_commands::CAD_FORMS_SOURCES => $then!(forms_sources, FormsSources),
            kentos_commands::CAD_FORMS_TRANSITION => $then!(forms_transition, FormsTransition),
            kentos_commands::CAD_DOMAINS_EDIT => $then!(domains_edit, DomainsEdit),
            kentos_commands::CAD_DOMAINS_EXPORT => $then!(domains_export, DomainsExport),
            kentos_commands::CAD_FIELDS_MIGRATE => $then!(fields_migrate, FieldsMigrate),
            kentos_commands::CAD_ATTACHMENTS_INSPECT => {
                $then!(attachments_inspect, AttachmentsInspect)
            }
            kentos_commands::CAD_SCHEMA_DESCRIBE => $then!(schema_describe, SchemaDescribe),
            kentos_commands::CAD_FORMS_EXPORT => $then!(forms_export, FormsExport),
            kentos_commands::CAD_FORMS_IMPORT => $then!(forms_import, FormsImport),
            kentos_commands::CAD_FORMS_RECOMPUTE => $then!(forms_recompute, FormsRecompute),
            kentos_commands::CAD_RECORDS_CREATE => $then!(records, RecordsCreate),
            kentos_commands::CAD_RECORDS_DUPLICATE => $then!(records_duplicate, RecordsDuplicate),
            kentos_commands::CAD_RELATIONS_EDIT => $then!(relations_edit, RelationsEdit),
            kentos_commands::CAD_JOINS_EDIT => $then!(joins_edit, JoinsEdit),
            kentos_commands::CAD_JOINS_QUERY => $then!(joins_query, JoinsQuery),
            kentos_commands::CAD_DATASETS_EDIT => $then!(datasets_edit, DatasetsEdit),
            kentos_commands::CAD_DATASETS_DESCRIBE => $then!(datasets_describe, DatasetsDescribe),
            kentos_commands::CAD_ENTITIES_TRIM => $then!(entities_trim, EntitiesTrim),
            kentos_commands::CAD_ENTITIES_EXTEND => $then!(entities_extend, EntitiesTrim),
            kentos_commands::CAD_ENTITIES_FILLET => $then!(entities_fillet, EntitiesFillet),
            kentos_commands::CAD_ENTITIES_CHAMFER => $then!(entities_chamfer, EntitiesChamfer),
            kentos_commands::CAD_ENTITIES_STRETCH => $then!(stretch, EntitiesStretch),
            kentos_commands::CAD_ENTITIES_BREAK => $then!(entities_break, EntitiesBreak),
            kentos_commands::CAD_ENTITIES_JOIN => $then!(entities_join, EntitiesJoin),
            kentos_commands::CAD_ENTITIES_REVERSE => $then!(entities_reverse, EntitiesReverse),
            kentos_commands::CAD_ENTITIES_LENGTHEN => $then!(entities_lengthen, EntitiesLengthen),
            kentos_commands::CAD_DRAWING_INSPECT => $then!(drawing_inspect, kentos_commands::DrawingInspect),
            kentos_commands::CAD_ARRAYS_FIND => $then!(arrays_find, kentos_commands::ArraysFind),
            kentos_commands::CAD_ARRAYS_ADOPT => $then!(arrays_adopt, kentos_commands::ArraysAdopt),
            kentos_commands::CAD_ARRAYS_EDIT => $then!(arrays_edit, ArraysEdit),
            kentos_commands::CAD_CONSTRAINTS_EDIT => $then!(constraints_edit, ConstraintsEdit),
            kentos_commands::CAD_CONSTRAINTS_QUERY => $then!(constraints_query, ConstraintsQuery),
            kentos_commands::CAD_PARAMETERS_EDIT => $then!(parameters_edit, ParametersEdit),
            kentos_commands::CAD_ENTITIES_EXPLODE => $then!(entities_explode, EntitiesExplode),
            kentos_commands::CAD_CURVES_EDIT => $then!(curves_edit, CurvesEdit),
            kentos_commands::CAD_ENTITIES_MATCH => $then!(entities_match, EntitiesMatch),
            kentos_commands::CAD_ENTITIES_CLEANUP => $then!(entities_cleanup, EntitiesCleanup),
            kentos_commands::CAD_DRAWING_AUDIT => $then!(drawing_audit, DrawingAudit),
            kentos_commands::CAD_DRAWING_COMPARE => $then!(drawing_compare, DrawingCompare),
            kentos_commands::CAD_DRAWING_TAKE => $then!(drawing_take, kentos_commands::DrawingTake),
            kentos_commands::CAD_DRAWING_REPAIR => $then!(drawing_repair, kentos_commands::DrawingRepair),
            kentos_commands::CAD_REVIEW_NOTES_EDIT => $then!(review_notes, kentos_commands::ReviewNotesEdit),
            kentos_commands::CAD_REVIEW_NOTES_LIST => $then!(review_notes_list, kentos_commands::ReviewNotesList),
            kentos_commands::CAD_ADDRESSES_CHECK => $then!(addresses_check, AddressesCheck),
            kentos_commands::CAD_ADDRESSES_COMPARE => $then!(addresses_compare, AddressesCompare),
            kentos_commands::CAD_ADDRESSES_NORMALIZE => {
                $then!(addresses_normalize, AddressesNormalize)
            }
            kentos_commands::CAD_TOPOLOGY_CHECK => $then!(topology_check, TopologyCheck),
            kentos_commands::CAD_TOPOLOGY_EXCEPTIONS => {
                $then!(topology_exceptions, TopologyExceptions)
            }
            kentos_commands::CAD_TOPOLOGY_REPAIR => {
                $then!(topology_repair, kentos_commands::TopologyRepair)
            }
            kentos_commands::CAD_ENTITIES_COPY => $then!(entities_copy, EntitiesCopy),
            kentos_commands::CAD_ENTITIES_PASTE => $then!(entities_paste, EntitiesPaste),
            kentos_commands::CAD_ENTITIES_DIVIDE => $then!(divide, EntitiesDivide),
            kentos_commands::CAD_ENTITIES_BOUNDARY => $then!(entities_boundary, EntitiesBoundary),
            kentos_commands::CAD_ENTITIES_HATCH => $then!(entities_hatch, EntitiesHatch),
            kentos_commands::CAD_DIMENSIONS_REASSOCIATE => {
                $then!(dimensions_reassociate, DimensionsReassociate)
            }
            kentos_commands::CAD_DIMENSIONS_CREATE => $then!(dimensions_create, DimensionsCreate),
            kentos_commands::CAD_DIMENSIONS_INTERIOR => $then!(dimensions_interior, kentos_commands::DimensionsInterior),
            kentos_commands::CAD_DIMENSION_SETS_CREATE => $then!(dimension_sets_create, DimensionSetsCreateInput),
            kentos_commands::CAD_DIMENSION_SETS_EDIT => $then!(dimension_sets_edit, DimensionSetsEditInput),
            kentos_commands::CAD_DIMENSION_SETS_QUERY => $then!(dimension_sets_query, DimensionSetsQueryInput),
            kentos_commands::CAD_DIMENSIONS_EDIT => $then!(dimensions_edit, DimensionsEdit),
            kentos_commands::CAD_DIMENSIONS_QUERY => $then!(dimensions_query, DimensionsQuery),
            kentos_commands::CAD_FONTS_REPORT => $then!(fonts_report, FontsReport),
            kentos_commands::CAD_FIELDS_EDIT => $then!(fields_edit, FieldsEdit),
            kentos_commands::CAD_LEADERS_EDIT => $then!(leaders_edit, LeadersEdit),
            kentos_commands::CAD_HATCHES_EDIT => $then!(hatches_edit, HatchesEdit),
            kentos_commands::CAD_TABLES_EDIT => $then!(tables_edit, TablesEdit),
            kentos_commands::CAD_TEXTS_EDIT => $then!(texts_edit, TextsEdit),
            kentos_commands::CAD_TEXTS_CHECK => $then!(texts_check, TextsCheck),
            kentos_commands::CAD_ADDRESSES_DEFINE => $then!(address_define, AddressesDefine),
            kentos_commands::CAD_LAYERS_TIME => $then!(layers_time, LayersTime),
            kentos_commands::CAD_SCENARIOS_EDIT => $then!(scenarios_edit, ScenariosEdit),
            kentos_commands::CAD_LAYERS_FILTER => $then!(layers_filter, LayersFilter),
            kentos_commands::CAD_LAYERS_CRS => {
                $then!(layers_crs_assign, kentos_commands::LayersCrs)
            }
            kentos_commands::CAD_LAYERS_REPROJECT => {
                $then!(layers_reproject, kentos_commands::LayersReproject)
            }
            kentos_commands::CAD_LAYERS_COMPOSE => $then!(layers_compose, LayersCompose),
            kentos_commands::CAD_LAYERS_INFO => $then!(layers_info, LayersInfo),
            kentos_commands::CAD_LAYERS_TABLE => $then!(layers_table, LayersTable),
            kentos_commands::CAD_LAYERS_SUBTYPES => $then!(layers_subtypes, LayersSubtypes),
            kentos_commands::CAD_LAYERS_CONTINGENTS => {
                $then!(layers_contingents, LayersContingents)
            }
            kentos_commands::CAD_LAYERS_RULES => $then!(layers_rules, LayersRules),
            kentos_commands::CAD_LAYERS_INDEXES => $then!(layers_indexes, LayersIndexes),
            kentos_commands::CAD_LAYERS_LABELS => $then!(layers_labels, LayersLabels),
            kentos_commands::CAD_LABELS_PIN => $then!(labels_pin, LabelsPin),
            kentos_commands::CAD_LABELS_REPORT => $then!(labels_report, LabelsReport),
            kentos_commands::CAD_LAYERS_RENDERER => $then!(layers_renderer, LayersRenderer),
            kentos_commands::CAD_LAYERS_EDIT => $then!(layers_edit, LayersEdit),
            kentos_commands::CAD_LAYERS_SET => $then!(layers_set, LayersSet),
            kentos_commands::CAD_LAYERS_STATES => $then!(layers_states, LayersStates),
            kentos_commands::CAD_LAYERS_FIND => $then!(layers_find, LayersFind),
            kentos_commands::CAD_FIELDS_USAGE => $then!(fields_usage, FieldsUsage),
            kentos_commands::CAD_LAYERS_TRANSLATE => $then!(layers_translate, LayersTranslate),
            kentos_commands::CAD_STANDARDS_CHECK => $then!(standards_check, StandardsCheck),
            kentos_commands::CAD_STANDARDS_CONFLICTS => $then!(standards_conflicts, kentos_commands::StandardsConflicts),
            kentos_commands::CAD_STANDARDS_INSTALL => $then!(standards_install, kentos_commands::StandardsInstall),
            kentos_commands::CAD_STANDARDS_EXCEPTIONS => {
                $then!(standards_exceptions, kentos_commands::StandardsExceptions)
            }
            kentos_commands::CAD_STANDARDS_RESOLVE => $then!(standards_resolve, kentos_commands::StandardsResolve),
            kentos_commands::CAD_STANDARDS_PALETTES => $then!(standards_palettes, kentos_commands::StandardsPalettes),
            kentos_commands::CAD_STANDARDS_APPLY => $then!(standards_apply, StandardsApply),
            kentos_commands::CAD_PROFILES_APPLY => {
                $then!(profiles_apply, kentos_commands::ProfilesApply)
            }
            kentos_commands::CAD_PROFILES_CHECK => {
                $then!(profiles_check, kentos_commands::ProfilesCheck)
            }
            kentos_commands::CAD_PROFILES_IMPORT => {
                $then!(profiles_import, kentos_commands::ProfilesImport)
            }
            kentos_commands::CAD_PROFILES_LIST => {
                $then!(profiles_list, kentos_commands::ProfilesList)
            }
            kentos_commands::CAD_PROFILES_MIGRATE => {
                $then!(profiles_migrate, kentos_commands::ProfilesMigrate)
            }
            kentos_commands::CAD_PROFILES_REMOVE => {
                $then!(profiles_remove, kentos_commands::ProfilesRemove)
            }
            kentos_commands::CAD_LINE_TYPES_EDIT => $then!(line_types_edit, LineTypesEdit),
            kentos_commands::CAD_HATCH_PATTERNS_EDIT => {
                $then!(hatch_patterns_edit, HatchPatternsEdit)
            }
            kentos_commands::CAD_LIBRARY_EDIT => $then!(library_edit, LibraryEdit),
            kentos_commands::CAD_STYLES_IMPACT => $then!(styles_impact, StylesImpact),
            kentos_commands::CAD_BLOCKS_INSERT => $then!(blocks_insert, BlocksInsert),
            kentos_commands::CAD_BLOCKS_REPLACE => $then!(blocks_replace, BlocksReplace),
            kentos_commands::CAD_BLOCKS_QUERY => $then!(blocks_query, BlocksQuery),
            kentos_commands::CAD_BLOCKS_FIND => $then!(blocks_find, kentos_commands::BlocksFind),
            kentos_commands::CAD_BLOCKS_CONVERT => $then!(blocks_convert, kentos_commands::BlocksConvert),
            kentos_commands::CAD_BLOCKS_PARAMETERS => $then!(block_params, BlocksParameters),
            kentos_commands::CAD_BLOCKS_PROXY => $then!(blocks_proxy, BlocksProxy),
            kentos_commands::CAD_BLOCKS_FREEZE => $then!(blocks_freeze, BlocksFreeze),
            kentos_commands::FILE_IMPORT_BLOCK => $then!(file_import_block, ImportBlock),
            kentos_commands::CAD_STYLES_EDIT => $then!(styles_edit, StylesEdit),
            kentos_commands::CAD_SETTINGS_SET => $then!(settings_set, SettingsSet),
            kentos_commands::CAD_UNITS_CHANGE => $then!(units_change, UnitsChange),
            kentos_commands::FILE_IMPORT_DXF => $then!(file_import, ImportDxf),
            kentos_commands::FILE_IMPORT_DWG => $then!(file_import_dwg, ImportDxf),
            kentos_commands::FILE_IMPORT_GPKG => $then!(file_import_gpkg, ImportGpkg),
            kentos_commands::FILE_IMPORT_SLD => $then!(sld, ImportSld),
            kentos_commands::FILE_IMPORT_QML => $then!(qml, ImportQml),
            kentos_commands::FILE_IMPORT_MAPLIBRE => $then!(maplibre, ImportMaplibre),
            kentos_commands::FILE_IMPORT_CSV => $then!(file_import_csv, ImportCsv),
            kentos_commands::FILE_IMPORT_FGB => $then!(file_import_fgb, ImportFgb),
            kentos_commands::FILE_IMPORT_PARQUET => $then!(file_import_parquet, ImportParquet),
            kentos_commands::FILE_IMPORT_GML => $then!(file_import_gml, ImportGml),
            kentos_commands::FILE_IMPORT_SHP => $then!(file_import_shp, ImportShp),
            kentos_commands::FILE_IMPORT_GEOJSON => $then!(file_import_geojson, ImportGeojson),
            kentos_commands::FILE_IMPORT_KML => $then!(file_import_kml, ImportKml),
            kentos_commands::CAD_GIS_DERIVATION => $then!(gis_derivation, GisDerivation),
            kentos_commands::CAD_GIS_TAKEBACK => $then!(gis_takeback, GisTakeback),
            kentos_commands::CAD_GIS_DETACH => $then!(gis_detach, GisDetach),
            kentos_commands::FILE_IMPORT_PDF => $then!(file_import_pdf, ImportPdf),
            kentos_commands::CAD_SHEETS_CREATE => $then!(sheets_create, SheetsCreate),
            kentos_commands::CAD_SHEETS_FIT => $then!(sheets_fit, SheetsFit),
            kentos_commands::CAD_SHEETS_LEVELS => $then!(sheets_levels, kentos_commands::SheetsLevels),
            kentos_commands::CAD_SHEETS_DETAILS => $then!(sheets_details, kentos_commands::SheetsDetails),
            kentos_commands::CAD_PLOTSTYLES_IMPORT => $then!(plot_styles_import, kentos_commands::PlotStylesImport),
            kentos_commands::CAD_SHEETS_EDIT => $then!(sheets_edit, SheetsEdit),
            kentos_commands::CAD_CHANGESET_APPLY => $then!(changeset, ChangesetApply),
            other => Err(unknown(other)),
        }
    };
}

/// `input` read as `command`'s input and written back: its defaults filled
/// in, its fields in the type's form (what the journal keeps and a
/// request's fingerprint is made of).
pub fn normalize(command: &str, input: Value) -> Result<Value, Unrun> {
    check_standard_resource_input(command, &input)?;
    macro_rules! normal {
        ($module:ident, $input:ty) => {{
            let typed: $input =
                serde_json::from_value(input).map_err(|e| unreadable(command, &e))?;
            serde_json::to_value(&typed).map_err(|e| unwritable(command, &e))
        }};
    }
    by_command!(command, normal)
}

/// Runs `op` of `command` with `input` in `cx`: the command's whole answer
/// as the catalog writes it (`status`-tagged).
pub fn run(
    cx: &mut ExecutionContext<'_>,
    command: &str,
    op: CallOp,
    input: Value,
) -> Result<Value, Unrun> {
    check_standard_resource_input(command, &input)?;
    macro_rules! go {
        ($module:ident, $input:ty) => {{
            let typed: $input =
                serde_json::from_value(input).map_err(|e| unreadable(command, &e))?;
            match op {
                CallOp::Validate => serde_json::to_value($module::validate(cx, &typed)),
                CallOp::Plan => serde_json::to_value($module::plan(cx, &typed)),
                CallOp::Execute => serde_json::to_value($module::execute(cx, typed)),
            }
            .map_err(|e| unwritable(command, &e))
        }};
    }
    by_command!(command, go)
}
