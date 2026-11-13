//! The command registry's part of the drawing's core commands (docs/adr/8054 §9): `core_commands!` calls `$then!(module, InputType)`
//! for one of them, else gives `$else`. The registry (core/registry.rs) chains the parts; the handlers' names are
//! resolved where the registry calls it.

macro_rules! core_commands {
    ($command:expr, $then:ident, $else:expr) => {
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
            kentos_commands::FILE_IMPORT_LANDXML => {
                crate::core_dispatch::terrain_arm!(
                    $command,
                    $then,
                    file_import_landxml,
                    kentos_commands::ImportLandxml
                )
            }
            kentos_commands::CAD_ENTITIES_GRADE => {
                $then!(entities_grade, kentos_commands::EntitiesGrade)
            }
            kentos_commands::CAD_ENTITIES_RAISE => {
                $then!(entities_raise, kentos_commands::EntitiesRaise)
            }
            kentos_commands::CAD_ENTITIES_DRAPE => {
                crate::core_dispatch::terrain_arm!(
                    $command,
                    $then,
                    entities_drape,
                    kentos_commands::EntitiesDrape
                )
            }
            kentos_commands::CAD_PROFILES_COMPARE => {
                $then!(profiles_compare, kentos_commands::ProfilesCompare)
            }
            kentos_commands::CAD_PROFILES_CARRY => {
                $then!(profiles_carry, kentos_commands::ProfilesCarry)
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
            kentos_commands::CAD_SHAPES_EDIT => $then!(shape_edits, kentos_commands::ShapesEdit),
            kentos_commands::CAD_NURBS_CREATE => $then!(nurbs, NurbsCreate),
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
            kentos_commands::CAD_SHEETS_SET => $then!(sheets_set, SheetsSet),
            kentos_commands::CAD_NAVIGATOR_READ => $then!(navigator, NavigatorRead),
            kentos_commands::CAD_XREFS_EDIT => $then!(xrefs_edit, XrefsEdit),
            kentos_commands::CAD_XREFS_LIST => $then!(xrefs_list, XrefsList),
            kentos_commands::CAD_ASSETS_CHECK => $then!(assets_check, AssetsCheck),
            kentos_commands::CAD_INSERTS_CLIP => $then!(inserts_clip, InsertsClip),
            kentos_commands::CAD_EXTRACT_RUN => $then!(extract_run, kentos_commands::ExtractRun),
            kentos_commands::CAD_EXTRACT_TABLE => $then!(extract_table, kentos_commands::ExtractTable),
            kentos_commands::CAD_ANNOTATIONS_SCALES => {
                $then!(annotations_scales, AnnotationsScales)
            }
            kentos_commands::CAD_ENTITIES_ORDER => $then!(entities_order, EntitiesOrder),
            kentos_commands::CAD_ENTITIES_FLATTEN => $then!(entities_flatten, EntitiesFlatten),
            kentos_commands::CAD_UCS_SET => $then!(ucs_set, UcsSet),
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
            kentos_commands::CAD_LEADERS_EDIT => $then!(leaders_edit, LeadersEdit),
            kentos_commands::CAD_HATCHES_EDIT => $then!(hatches_edit, HatchesEdit),
            kentos_commands::CAD_TABLES_EDIT => $then!(tables_edit, TablesEdit),
            kentos_commands::CAD_TEXTS_EDIT => $then!(texts_edit, TextsEdit),
            kentos_commands::CAD_TEXTS_CHECK => $then!(texts_check, TextsCheck),
            kentos_commands::CAD_LABELS_PIN => $then!(labels_pin, LabelsPin),
            kentos_commands::CAD_LABELS_REPORT => $then!(labels_report, LabelsReport),
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
            kentos_commands::CAD_PROFILES_NAMES => {
                $then!(profiles_names, kentos_commands::ProfilesNames)
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
            kentos_commands::FILE_IMPORT_CSV => $then!(file_import_csv, ImportCsv),
            kentos_commands::FILE_IMPORT_FGB => $then!(file_import_fgb, ImportFgb),
            kentos_commands::FILE_IMPORT_PARQUET => $then!(file_import_parquet, ImportParquet),
            kentos_commands::FILE_IMPORT_GML => $then!(file_import_gml, ImportGml),
            kentos_commands::FILE_IMPORT_SHP => $then!(file_import_shp, ImportShp),
            kentos_commands::FILE_IMPORT_GEOJSON => $then!(file_import_geojson, ImportGeojson),
            kentos_commands::FILE_IMPORT_KML => $then!(file_import_kml, ImportKml),
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
pub(crate) use core_commands;

/// A core command that drapes onto or reads a terrain surface (TIN): with road design or mapping (feature `terrain`,
/// docs/adr/8054 §11), else the answer that this build has none.
#[cfg(feature = "terrain")]
macro_rules! terrain_arm {
    ($command:expr, $then:ident, $module:ident, $input:ty) => {
        $then!($module, $input)
    };
}
#[cfg(not(feature = "terrain"))]
macro_rules! terrain_arm {
    ($command:expr, $then:ident, $module:ident, $input:ty) => {
        Err(crate::registry::not_built($command, "yol ya da harita"))
    };
}
pub(crate) use terrain_arm;
