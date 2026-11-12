//! The fixture runner's core commands (docs/adr/8054 §9): `core_cases!` calls `$then!(module, InputType)` for one
//! of them, else gives `$else`; run_op chains the parts.

macro_rules! core_cases {
    ($command:expr, $then:ident, $else:expr) => {
        match $command {
            kentos_commands::CAD_DIMENSION_SETS_CREATE => $then!(
                dimension_sets_create,
                kentos_commands::DimensionSetsCreateInput
            ),
            kentos_commands::CAD_DIMENSION_SETS_EDIT => {
                $then!(dimension_sets_edit, kentos_commands::DimensionSetsEditInput)
            }
            kentos_commands::CAD_DIMENSION_SETS_QUERY => $then!(
                dimension_sets_query,
                kentos_commands::DimensionSetsQueryInput
            ),
            CAD_POLYGON_CREATE => $then!(polygon, PolygonCreate),
            CAD_LINE_CREATE => $then!(line, LineCreate),
            CAD_POLYLINE_CREATE => $then!(polyline, PolylineCreate),
            CAD_ENTITIES_DELETE => $then!(delete, EntitiesDelete),
            CAD_POINT_CREATE => $then!(point, PointCreate),
            CAD_CIRCLE_CREATE => $then!(circle, CircleCreate),
            CAD_ARC_CREATE => $then!(arc, ArcCreate),
            CAD_ENTITIES_TRANSFORM => $then!(transform, EntitiesTransform),
            CAD_ENTITIES_EDIT => $then!(edit, EntitiesEdit),
            CAD_ENTITIES_ARRAY => $then!(array, EntitiesArray),
            kentos_commands::CAD_ENTITIES_OFFSET => $then!(offset, EntitiesOffset),
            kentos_commands::CAD_POINTS_CONSTRUCT => {
                $then!(points_construct, kentos_commands::PointsConstruct)
            }
            kentos_commands::CAD_CURVES_CONSTRUCT => {
                $then!(curves_construct, kentos_commands::CurvesConstruct)
            }
            kentos_commands::CAD_SNAP_FIND => {
                $then!(snap_find, kentos_commands::SnapFind)
            }
            kentos_commands::CAD_RECTANGLE_CREATE => {
                $then!(rectangle_create, kentos_commands::RectangleCreate)
            }
            kentos_commands::CAD_POLYGON_REGULAR => {
                $then!(polygon_regular, kentos_commands::RegularPolygonCreate)
            }
            kentos_commands::CAD_SECTOR_CREATE => $then!(sector_create, kentos_commands::SectorCreate),
            kentos_commands::CAD_REVCLOUD_CREATE => {
                $then!(revcloud_create, kentos_commands::RevcloudCreate)
            }
            kentos_commands::CAD_ENTITIES_ADJOIN => $then!(adjoin_cmd, kentos_commands::EntitiesAdjoin),
            kentos_commands::CAD_CONSTRUCTIONS_CREATE => {
                $then!(constructions, kentos_commands::ConstructionsCreate)
            }
            kentos_commands::CAD_AREAS_EDIT => $then!(areas, kentos_commands::AreasEdit),
            kentos_commands::CAD_ANNOTATIONS_WRITE => {
                $then!(annotations_write, kentos_commands::AnnotationsWrite)
            }
            kentos_commands::CAD_OBJECTS_DERIVE => $then!(objects_derive, kentos_commands::ObjectsDerive),
            kentos_commands::CAD_SHAPES_EDIT => $then!(shape_edits, kentos_commands::ShapesEdit),
            kentos_commands::CAD_ENTITIES_VERTEX => {
                $then!(entities_vertex, kentos_commands::EntitiesVertex)
            }
            kentos_commands::CAD_ENTITIES_PICK => {
                $then!(entities_pick, kentos_commands::EntitiesPick)
            }
            kentos_commands::CAD_CRS_TRANSFORM => {
                $then!(crs_transform, kentos_commands::CrsTransform)
            }
            kentos_commands::CAD_ENTITIES_RELATE => {
                $then!(entities_relate, kentos_commands::EntitiesRelate)
            }
            kentos_commands::CAD_ENTITIES_SELECT => {
                $then!(entities_select, kentos_commands::EntitiesSelect)
            }
            kentos_commands::CAD_ENTITIES_ISOLATE => $then!(isolation, kentos_commands::EntitiesIsolate),
            kentos_commands::CAD_ENTITIES_TRIM => $then!(entities_trim, EntitiesTrim),
            kentos_commands::CAD_ENTITIES_EXTEND => $then!(entities_extend, EntitiesTrim),
            kentos_commands::CAD_ENTITIES_FILLET => $then!(entities_fillet, EntitiesFillet),
            kentos_commands::CAD_ENTITIES_CHAMFER => $then!(entities_chamfer, EntitiesChamfer),
            kentos_commands::CAD_ENTITIES_STRETCH => $then!(stretch, EntitiesStretch),
            kentos_commands::CAD_ENTITIES_BREAK => $then!(entities_break, EntitiesBreak),
            kentos_commands::CAD_ENTITIES_JOIN => $then!(entities_join, EntitiesJoin),
            kentos_commands::CAD_ENTITIES_REVERSE => {
                $then!(entities_reverse, kentos_commands::EntitiesReverse)
            }
            kentos_commands::CAD_ENTITIES_LENGTHEN => {
                $then!(entities_lengthen, kentos_commands::EntitiesLengthen)
            }
            kentos_commands::CAD_ENTITIES_DIVIDE => $then!(divide, EntitiesDivide),
            kentos_commands::CAD_DRAWING_INSPECT => $then!(drawing_inspect, kentos_commands::DrawingInspect),
            kentos_commands::CAD_ARRAYS_FIND => {
                $then!(arrays_find, kentos_commands::ArraysFind)
            }
            kentos_commands::CAD_ARRAYS_ADOPT => {
                $then!(arrays_adopt, kentos_commands::ArraysAdopt)
            }
            kentos_commands::CAD_ARRAYS_EDIT => {
                $then!(arrays_edit, kentos_commands::ArraysEdit)
            }
            kentos_commands::CAD_CONSTRAINTS_EDIT => {
                $then!(constraints_edit, kentos_commands::ConstraintsEdit)
            }
            kentos_commands::CAD_CONSTRAINTS_QUERY => {
                $then!(constraints_query, kentos_commands::ConstraintsQuery)
            }
            kentos_commands::CAD_PARAMETERS_EDIT => {
                $then!(parameters_edit, kentos_commands::ParametersEdit)
            }
            kentos_commands::CAD_ENTITIES_EXPLODE => {
                $then!(entities_explode, kentos_commands::EntitiesExplode)
            }
            kentos_commands::CAD_ENTITIES_MATCH => {
                $then!(entities_match, kentos_commands::EntitiesMatch)
            }
            kentos_commands::CAD_ENTITIES_CLEANUP => {
                $then!(entities_cleanup, kentos_commands::EntitiesCleanup)
            }
            kentos_commands::CAD_ENTITIES_COPY => {
                $then!(entities_copy, kentos_commands::EntitiesCopy)
            }
            kentos_commands::CAD_ENTITIES_PASTE => {
                $then!(entities_paste, kentos_commands::EntitiesPaste)
            }
            kentos_commands::CAD_DRAWING_AUDIT => {
                $then!(drawing_audit, kentos_commands::DrawingAudit)
            }
            kentos_commands::CAD_BLOCKS_CONVERT => {
                $then!(blocks_convert, kentos_commands::BlocksConvert)
            }
            kentos_commands::CAD_BLOCKS_FIND => {
                $then!(blocks_find, kentos_commands::BlocksFind)
            }
            kentos_commands::CAD_DRAWING_COMPARE => {
                $then!(drawing_compare, kentos_commands::DrawingCompare)
            }
            kentos_commands::CAD_DRAWING_TAKE => {
                $then!(drawing_take, kentos_commands::DrawingTake)
            }
            kentos_commands::CAD_DRAWING_REPAIR => {
                use kentos_native_application::drawing_repair;
                $then!(drawing_repair, kentos_commands::DrawingRepair)
            }
            kentos_commands::CAD_STANDARDS_INSTALL => {
                use kentos_native_application::standards_install;
                $then!(standards_install, kentos_commands::StandardsInstall)
            }
            kentos_commands::CAD_STANDARDS_EXCEPTIONS => {
                use kentos_native_application::standards_exceptions;
                $then!(standards_exceptions, kentos_commands::StandardsExceptions)
            }
            kentos_commands::CAD_CURVES_EDIT => {
                $then!(curves_edit, kentos_commands::CurvesEdit)
            }
            kentos_commands::CAD_ENTITIES_BOUNDARY => {
                $then!(entities_boundary, EntitiesBoundary)
            }
            kentos_commands::CAD_ENTITIES_HATCH => $then!(entities_hatch, EntitiesHatch),
            kentos_commands::CAD_DIMENSIONS_REASSOCIATE => {
                $then!(dimensions_reassociate, DimensionsReassociate)
            }
            kentos_commands::CAD_DIMENSIONS_EDIT => {
                $then!(dimensions_edit, kentos_commands::DimensionsEdit)
            }
            kentos_commands::CAD_DIMENSIONS_QUERY => {
                $then!(dimensions_query, kentos_commands::DimensionsQuery)
            }
            kentos_commands::CAD_FONTS_REPORT => {
                $then!(fonts_report, kentos_commands::FontsReport)
            }
            kentos_commands::CAD_LEADERS_EDIT => {
                $then!(leaders_edit, kentos_commands::LeadersEdit)
            }
            kentos_commands::CAD_HATCHES_EDIT => {
                $then!(hatches_edit, kentos_commands::HatchesEdit)
            }
            kentos_commands::CAD_TABLES_EDIT => {
                $then!(tables_edit, kentos_commands::TablesEdit)
            }
            kentos_commands::CAD_TEXTS_EDIT => {
                $then!(texts_edit, kentos_commands::TextsEdit)
            }
            kentos_commands::CAD_TEXTS_CHECK => {
                $then!(texts_check, kentos_commands::TextsCheck)
            }
            kentos_commands::CAD_DIMENSIONS_CREATE => {
                $then!(dimensions_create, DimensionsCreate)
            }
            kentos_commands::CAD_DIMENSIONS_INTERIOR => {
                $then!(dimensions_interior, kentos_commands::DimensionsInterior)
            }
            kentos_commands::CAD_ENTITIES_CREATE => $then!(create, EntitiesCreate),
            kentos_commands::CAD_NURBS_CREATE => $then!(nurbs, NurbsCreate),
            kentos_commands::CAD_COLUMNS_DISTANCES => {
                $then!(columns_distances, kentos_commands::ColumnsDistancesInput)
            }
            kentos_commands::CAD_GRIDS_CREATE => {
                $then!(grids_create, kentos_commands::GridsCreate)
            }
            kentos_commands::CAD_COLUMNS_CREATE => {
                $then!(columns_create, kentos_commands::ColumnsCreate)
            }
            kentos_commands::CAD_STAIRS_CREATE => {
                $then!(stairs_create, kentos_commands::StairsCreate)
            }
            kentos_commands::CAD_RAMPS_CREATE => {
                $then!(ramps_create, kentos_commands::RampsCreate)
            }
            kentos_commands::CAD_BEAMS_CREATE => {
                $then!(beams_create, kentos_commands::BeamsCreate)
            }
            kentos_commands::CAD_RAILINGS_CREATE => {
                $then!(railings_create, kentos_commands::RailingsCreate)
            }
            kentos_commands::CAD_SHEETS_SET => {
                $then!(sheets_set, kentos_commands::SheetsSet)
            }
            kentos_commands::CAD_NAVIGATOR_READ => {
                $then!(navigator, kentos_commands::NavigatorRead)
            }
            kentos_commands::CAD_XREFS_EDIT => {
                $then!(xrefs_edit, kentos_commands::XrefsEdit)
            }
            kentos_commands::CAD_XREFS_LIST => {
                $then!(xrefs_list, kentos_commands::XrefsList)
            }
            kentos_commands::CAD_ASSETS_CHECK => {
                $then!(assets_check, kentos_commands::AssetsCheck)
            }
            kentos_commands::CAD_INSERTS_CLIP => {
                $then!(inserts_clip, kentos_commands::InsertsClip)
            }
            kentos_commands::CAD_EXTRACT_RUN => $then!(extract_run, kentos_commands::ExtractRun),
            kentos_commands::CAD_EXTRACT_TABLE => $then!(extract_table, kentos_commands::ExtractTable),
            kentos_commands::CAD_ENTITIES_ORDER => $then!(entities_order, EntitiesOrder),
            kentos_commands::CAD_ENTITIES_FLATTEN => $then!(entities_flatten, kentos_commands::EntitiesFlatten),
            kentos_commands::CAD_ANNOTATIONS_SCALES => $then!(annotations_scales, AnnotationsScales),
            CAD_ENTITIES_SET => $then!(set, EntitiesSetProperties),
            CAD_BLOCKS_DEFINE => $then!(blocks_define, BlocksDefine),
            CAD_BLOCKS_EDIT => $then!(blocks_edit, BlocksEdit),
            kentos_commands::CAD_BLOCKS_INSERT => {
                $then!(blocks_insert, kentos_commands::BlocksInsert)
            }
            kentos_commands::FILE_IMPORT_BLOCK => {
                $then!(file_import_block, kentos_commands::ImportBlock)
            }
            kentos_commands::CAD_BLOCKS_REPLACE => {
                $then!(blocks_replace, kentos_commands::BlocksReplace)
            }
            kentos_commands::CAD_BLOCKS_QUERY => {
                $then!(blocks_query, kentos_commands::BlocksQuery)
            }
            kentos_commands::CAD_BLOCKS_PARAMETERS => {
                $then!(block_params, kentos_commands::BlocksParameters)
            }
            kentos_commands::CAD_BLOCKS_PROXY => {
                $then!(blocks_proxy, kentos_commands::BlocksProxy)
            }
            kentos_commands::CAD_BLOCKS_FREEZE => {
                $then!(blocks_freeze, kentos_commands::BlocksFreeze)
            }
            kentos_commands::FILE_IMPORT_LANDXML => {
                $then!(file_import_landxml, kentos_commands::ImportLandxml)
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
            kentos_commands::CAD_PROFILES_COMPARE => {
                $then!(profiles_compare, kentos_commands::ProfilesCompare)
            }
            kentos_commands::CAD_PROFILES_CARRY => {
                $then!(profiles_carry, kentos_commands::ProfilesCarry)
            }
            kentos_commands::CAD_PROFILES_NAMES => {
                $then!(profiles_names, kentos_commands::ProfilesNames)
            }
            kentos_commands::CAD_LABELS_PIN => $then!(labels_pin, LabelsPin),
            kentos_commands::CAD_LABELS_REPORT => {
                $then!(labels_report, kentos_commands::LabelsReport)
            }
            kentos_commands::CAD_STANDARDS_CHECK => $then!(standards_check, StandardsCheck),
            kentos_commands::CAD_STANDARDS_CONFLICTS => $then!(standards_conflicts, kentos_commands::StandardsConflicts),
            kentos_commands::CAD_STANDARDS_RESOLVE => $then!(standards_resolve, kentos_commands::StandardsResolve),
            kentos_commands::CAD_STANDARDS_PALETTES => $then!(standards_palettes, kentos_commands::StandardsPalettes),
            kentos_commands::CAD_STANDARDS_APPLY => $then!(standards_apply, StandardsApply),
            kentos_commands::CAD_PROFILES_APPLY => $then!(profiles_apply, kentos_commands::ProfilesApply),
            kentos_commands::CAD_PROFILES_CHECK => $then!(profiles_check, kentos_commands::ProfilesCheck),
            kentos_commands::CAD_PROFILES_IMPORT => {
                $then!(profiles_import, kentos_commands::ProfilesImport)
            }
            kentos_commands::CAD_PROFILES_LIST => $then!(profiles_list, kentos_commands::ProfilesList),
            kentos_commands::CAD_PROFILES_MIGRATE => {
                $then!(profiles_migrate, kentos_commands::ProfilesMigrate)
            }
            kentos_commands::CAD_PROFILES_REMOVE => {
                $then!(profiles_remove, kentos_commands::ProfilesRemove)
            }
            kentos_commands::CAD_LINE_TYPES_EDIT => $then!(line_types_edit, LineTypesEdit),
            kentos_commands::CAD_HATCH_PATTERNS_EDIT => $then!(hatch_patterns_edit, HatchPatternsEdit),
            kentos_commands::CAD_LIBRARY_EDIT => $then!(library_edit, LibraryEdit),
            kentos_commands::CAD_STYLES_IMPACT => $then!(styles_impact, StylesImpact),
            kentos_commands::CAD_STYLES_EDIT => $then!(styles_edit, StylesEdit),
            kentos_commands::CAD_DONUT_CREATE => $then!(donut_create, kentos_commands::DonutCreate),
            kentos_commands::CAD_SETTINGS_SET => $then!(settings_set, SettingsSet),
            kentos_commands::CAD_UCS_SET => $then!(ucs_set, UcsSet),
            kentos_commands::CAD_UNITS_CHANGE => $then!(units_change, UnitsChange),
            kentos_commands::FILE_IMPORT_DXF => $then!(file_import, ImportDxf),
            kentos_commands::FILE_IMPORT_DWG => $then!(file_import_dwg, ImportDxf),
            kentos_commands::FILE_IMPORT_GPKG => $then!(file_import_gpkg, ImportGpkg),
            kentos_commands::FILE_IMPORT_CSV => {
                $then!(file_import_csv, kentos_commands::ImportCsv)
            }
            kentos_commands::FILE_IMPORT_FGB => {
                $then!(file_import_fgb, kentos_commands::ImportFgb)
            }
            kentos_commands::FILE_IMPORT_PARQUET => {
                $then!(file_import_parquet, kentos_commands::ImportParquet)
            }
            kentos_commands::FILE_IMPORT_GML => {
                $then!(file_import_gml, kentos_commands::ImportGml)
            }
            kentos_commands::FILE_IMPORT_SHP => {
                $then!(file_import_shp, kentos_commands::ImportShp)
            }
            kentos_commands::FILE_IMPORT_GEOJSON => {
                $then!(file_import_geojson, kentos_commands::ImportGeojson)
            }
            kentos_commands::FILE_IMPORT_KML => {
                $then!(file_import_kml, kentos_commands::ImportKml)
            }
            kentos_commands::FILE_IMPORT_PDF => $then!(file_import_pdf, ImportPdf),
            kentos_commands::CAD_SHEETS_CREATE => $then!(sheets_create, SheetsCreate),
            kentos_commands::CAD_SHEETS_FIT => $then!(sheets_fit, SheetsFit),
            kentos_commands::CAD_SHEETS_LEVELS => $then!(sheets_levels, kentos_commands::SheetsLevels),
            kentos_commands::CAD_SHEETS_DETAILS => $then!(sheets_details, kentos_commands::SheetsDetails),
            kentos_commands::CAD_PLOTSTYLES_IMPORT => $then!(plot_styles_import, kentos_commands::PlotStylesImport),
            kentos_commands::CAD_SHEETS_EDIT => $then!(sheets_edit, SheetsEdit),
            kentos_commands::CAD_CHANGESET_APPLY => $then!(changeset, ChangesetApply),
            _ => $else,
        }
    };
}
pub(super) use core_cases;
