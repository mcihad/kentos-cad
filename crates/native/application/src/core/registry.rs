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
    ProcessingBatch, ProcessingRun, ProcessingTools, RampsCreate, BeamsCreate, RoofsCreate, RailingsCreate, RastersGeoref, RastersMosaic,
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
use crate::parcel_drafts as parcel_approve;
use crate::asbuilt_review;
use crate::orthophoto_network::network as orthophoto_network;
use crate::orthophoto_network::outline as orthophoto_footprint;
use crate::parcel_drafts::withdraw as parcel_withdraw;
use crate::parcels_boundaries;
use crate::parcels_check;
use crate::pipes_check;
use crate::pipes_profile;
use crate::asbuilt_check;
use crate::field_package;
use crate::zoning_check;
use crate::zoning_symbolize;
use crate::zoning_draft;
use crate::scenarios_impact;
use crate::scenarios_record;
use crate::readjustment;
use crate::readjustment_scope;
use crate::readjustment_sheets;
use crate::orthophoto_check;
use crate::orthophoto_compare;
use crate::orthophoto_seamlines;
use crate::pointcloud_overlap;
use crate::pointcloud_network;
use crate::pointcloud_place;
use crate::profiles_compare;
use crate::profiles_carry;
use crate::profiles_names;
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
    linearref_define, file_import_landxml, road_alignments_create, road_scene, road_alignments_edit, road_alignments_label, road_alignments_query, road_profiles_create, road_profiles_draw, road_profiles_edit, road_profiles_query, road_profiles_sight, road_profiles_propose, road_alignments_check, road_profiles_check, road_profiles_lengths, road_profiles_impact, road_alignments_impact, road_sections_draw, road_sections_edit, road_sections_query, road_superelevations_edit, road_superelevations_query, road_superelevations_check, road_superelevations_widen, road_superelevations_transitions, road_corridors_draw, road_corridors_edit, road_corridors_query, road_corridors_water, road_corridors_daylights, road_corridors_protected, road_corridors_preflight, road_volumes_approve, road_volumes_edit, road_volumes_query, road_volumes_uncertainty, road_volumes_progress, road_volumes_compare, road_volumes_ledger, road_volumes_states, road_volumes_history, road_hauls_edit, road_hauls_query, road_hauls_plan, road_hauls_approve, road_hauls_network, road_hauls_trucks, road_guardrails_edit, road_guardrails_query, road_guardrails_draw, road_culverts_edit, road_culverts_query, road_culverts_draw, road_culverts_hydraulics, road_culverts_closure, road_walls_edit, road_walls_query, road_walls_draw, road_platforms_edit, road_platforms_query, road_platforms_draw, road_terrain_cloud, road_drainage, road_drainage_draw, road_swept, road_swept_draw, road_terrains_analysis, road_terrains_compare, road_terrains_quality, road_terrains_repair, road_terrains_coverage, road_terrains_coverage_draw, road_terrains_density, road_terrains_stockpile, road_terrains_sparse, road_pay_edit, road_pay_query, road_routes_find, road_routes_draw, road_junctions_edit, road_junctions_query, road_junctions_draw, road_junctions_clearance, road_junctions_tie, road_junctions_overlap, road_project_query, road_project_graph, road_project_produce, road_project_rebuild, road_project_edit, map_work, map_work_edit, map_work_restore, map_survey, map_survey_edit, map_survey_network, map_features_reconcile, road_alignments_import, road_alignments_fit, road_profiles_fit, road_ifc, arch_ifc, arch_recognize, road_landxml, road_stakeout, road_stakeout_send, road_sheets, road_sheets_sections, road_templates_edit, road_terrains_contours, road_terrains_sheet, road_terrains_section, road_terrains_zeroline, road_terrains_cutfill, road_terrains_create, road_terrains_edit, road_terrains_query, maplibre, network_define, north, nurbs, objects_derive, offset,
    openings_create, openings_distances, openings_edit, opening_types_edit, materials_edit, openings_query, modules_edit, modules_query, options_edit, options_query, elevations_edit, elevations_query, sections_edit, sections_query, parameters_edit, parcels, point,
    points_construct, polygon, polygon_regular, polyline, qml, ramps_create, beams_create, roofs_create, railings_create, rasters_georef,
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
        // The disciplines' parts, in turn (docs/adr/8054 §9): core, architecture, mapping, GIS, road.
        crate::core_dispatch::core_commands!(
            $command,
            $then,
            crate::arch_dispatch::arch_commands!(
                $command,
                $then,
                crate::map_dispatch::map_commands!(
                    $command,
                    $then,
                    crate::gis_dispatch::gis_commands!(
                        $command,
                        $then,
                        crate::road_dispatch::road_commands!($command, $then, Err(unknown($command)))
                    )
                )
            )
        )
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
