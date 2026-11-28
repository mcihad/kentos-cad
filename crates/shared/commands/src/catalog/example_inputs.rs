//! The catalog's examples against their commands' input types (docs/adr/0013): each example's input must parse as
//! its own command's input type. The table of types is apart from the catalog's other tests (mod.rs) so that neither
//! file grows past the source size limit (CLAUDE.md §8); a new command adds its arm here.

use crate::{EntitiesDelete, LineCreate, PolygonCreate, PolylineCreate};
use crate::{ProjectAccessRevoke, ProjectChanges, ProjectShare};

/// Whether the example `input` of the command `id` parses as the command's input type; a command without an arm
/// stops the test, so that a new command's examples are always read.
pub(super) fn parsed(id: &str, input: &serde_json::Value) -> serde_json::Result<()> {
    match id {
        crate::PROJECT_CHANGES => {
            serde_json::from_value::<ProjectChanges>(input.clone()).map(|_| ())
        }
        crate::PROJECT_SHARE => serde_json::from_value::<ProjectShare>(input.clone()).map(|_| ()),
        crate::PROJECT_ACCESS_REVOKE => {
            serde_json::from_value::<ProjectAccessRevoke>(input.clone()).map(|_| ())
        }
        // The catalog and lifecycle commands (docs/adr/0028).
        crate::PROJECT_CREATE => {
            serde_json::from_value::<crate::ProjectCreate>(input.clone()).map(|_| ())
        }
        crate::PROJECT_RENAME => {
            serde_json::from_value::<crate::ProjectRename>(input.clone()).map(|_| ())
        }
        crate::PROJECT_METADATA_UPDATE => {
            serde_json::from_value::<crate::ProjectMetadataUpdate>(input.clone()).map(|_| ())
        }
        crate::PROJECT_DUPLICATE => {
            serde_json::from_value::<crate::ProjectDuplicate>(input.clone()).map(|_| ())
        }
        crate::PROJECT_ARCHIVE
        | crate::PROJECT_UNARCHIVE
        | crate::PROJECT_TRASH
        | crate::PROJECT_RESTORE => {
            serde_json::from_value::<crate::EmptyInput>(input.clone()).map(|_| ())
        }
        crate::PROJECT_PURGE => {
            serde_json::from_value::<crate::ProjectPurge>(input.clone()).map(|_| ())
        }
        crate::PROJECT_FAVORITE => {
            serde_json::from_value::<crate::ProjectFavorite>(input.clone()).map(|_| ())
        }
        crate::PROJECT_FILE_COMMIT => {
            serde_json::from_value::<crate::FileCommit>(input.clone()).map(|_| ())
        }
        crate::PROJECT_CONVERT => {
            serde_json::from_value::<crate::ProjectConvert>(input.clone()).map(|_| ())
        }
        crate::PROJECT_IMPORT => {
            serde_json::from_value::<crate::ProjectImport>(input.clone()).map(|_| ())
        }
        crate::PROJECT_INVITE => {
            serde_json::from_value::<crate::ProjectInvite>(input.clone()).map(|_| ())
        }
        crate::PROJECT_INVITATION_REVOKE => {
            serde_json::from_value::<crate::InvitationRevoke>(input.clone()).map(|_| ())
        }
        crate::PROJECT_CHECKPOINT_CREATE => {
            serde_json::from_value::<crate::CheckpointCreate>(input.clone()).map(|_| ())
        }
        crate::PROJECT_CHECKPOINT_DELETE => {
            serde_json::from_value::<crate::CheckpointDelete>(input.clone()).map(|_| ())
        }
        crate::PROJECT_CHECKPOINT_RESTORE => {
            serde_json::from_value::<crate::CheckpointRestore>(input.clone()).map(|_| ())
        }
        crate::CAD_POLYGON_CREATE => {
            serde_json::from_value::<PolygonCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_LINE_CREATE => serde_json::from_value::<LineCreate>(input.clone()).map(|_| ()),
        crate::CAD_POLYLINE_CREATE => {
            serde_json::from_value::<PolylineCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_DELETE => {
            serde_json::from_value::<EntitiesDelete>(input.clone()).map(|_| ())
        }
        crate::CAD_POINT_CREATE => {
            serde_json::from_value::<crate::PointCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_CIRCLE_CREATE => {
            serde_json::from_value::<crate::CircleCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_ARC_CREATE => {
            serde_json::from_value::<crate::ArcCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_TRANSFORM => {
            serde_json::from_value::<crate::EntitiesTransform>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_EDIT => {
            serde_json::from_value::<crate::EntitiesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_ARRAY => {
            serde_json::from_value::<crate::EntitiesArray>(input.clone()).map(|_| ())
        }
        crate::CAD_DRAWING_INSPECT => {
            serde_json::from_value::<crate::DrawingInspect>(input.clone()).map(|_| ())
        }
        crate::CAD_ARRAYS_FIND => {
            serde_json::from_value::<crate::ArraysFind>(input.clone()).map(|_| ())
        }
        crate::CAD_ARRAYS_ADOPT => {
            serde_json::from_value::<crate::ArraysAdopt>(input.clone()).map(|_| ())
        }
        crate::CAD_ARRAYS_EDIT => {
            serde_json::from_value::<crate::ArraysEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_CONSTRAINTS_EDIT => {
            serde_json::from_value::<crate::ConstraintsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_CONSTRAINTS_QUERY => {
            serde_json::from_value::<crate::ConstraintsQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_PARAMETERS_EDIT => {
            serde_json::from_value::<crate::ParametersEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_EXPLODE => {
            serde_json::from_value::<crate::EntitiesExplode>(input.clone()).map(|_| ())
        }
        crate::CAD_CURVES_EDIT => {
            serde_json::from_value::<crate::CurvesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_MATCH => {
            serde_json::from_value::<crate::EntitiesMatch>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_CLEANUP => {
            serde_json::from_value::<crate::EntitiesCleanup>(input.clone()).map(|_| ())
        }
        crate::CAD_DRAWING_AUDIT => {
            serde_json::from_value::<crate::DrawingAudit>(input.clone()).map(|_| ())
        }
        crate::CAD_DRAWING_COMPARE => {
            serde_json::from_value::<crate::DrawingCompare>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_COPY => {
            serde_json::from_value::<crate::EntitiesCopy>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_PASTE => {
            serde_json::from_value::<crate::EntitiesPaste>(input.clone()).map(|_| ())
        }
        crate::CAD_NURBS_CREATE => {
            serde_json::from_value::<crate::NurbsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_WALLS_CREATE => {
            serde_json::from_value::<crate::WallsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_WALLS_RECTANGLE => {
            serde_json::from_value::<crate::WallsRectangle>(input.clone()).map(|_| ())
        }
        crate::CAD_WALLS_FROM_LINES => {
            serde_json::from_value::<crate::WallsFromLines>(input.clone()).map(|_| ())
        }
        crate::CAD_ARCH_RECOGNIZE => {
            serde_json::from_value::<crate::ArchRecognize>(input.clone()).map(|_| ())
        }
        crate::CAD_WALLS_EDIT => {
            serde_json::from_value::<crate::WallsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_WALLS_JOINS => {
            serde_json::from_value::<crate::WallsJoins>(input.clone()).map(|_| ())
        }
        crate::CAD_WALLS_DISTANCES => {
            serde_json::from_value::<crate::WallsDistancesInput>(input.clone()).map(|_| ())
        }
        crate::CAD_COLUMNS_DISTANCES => {
            serde_json::from_value::<crate::ColumnsDistancesInput>(input.clone()).map(|_| ())
        }
        crate::CAD_DIMENSIONS_INTERIOR => {
            serde_json::from_value::<crate::DimensionsInterior>(input.clone()).map(|_| ())
        }
        crate::CAD_OPENINGS_CREATE => {
            serde_json::from_value::<crate::OpeningsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_SPACES_CREATE => {
            serde_json::from_value::<crate::SpacesCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_SLABS_CREATE => {
            serde_json::from_value::<crate::SlabsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_SLABS_EDIT => {
            serde_json::from_value::<crate::SlabsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_SPACES_DISCOVER => {
            serde_json::from_value::<crate::SpacesDiscover>(input.clone()).map(|_| ())
        }
        crate::CAD_SPACES_NUMBER => {
            serde_json::from_value::<crate::SpacesNumber>(input.clone()).map(|_| ())
        }
        crate::CAD_SPACES_SEPARATORS => {
            serde_json::from_value::<crate::SpacesSeparators>(input.clone()).map(|_| ())
        }
        crate::CAD_GRIDS_CREATE => {
            serde_json::from_value::<crate::GridsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_COLUMNS_CREATE => {
            serde_json::from_value::<crate::ColumnsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_STAIRS_CREATE => {
            serde_json::from_value::<crate::StairsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_RAMPS_CREATE => {
            serde_json::from_value::<crate::RampsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_BEAMS_CREATE => {
            serde_json::from_value::<crate::BeamsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_ROOFS_CREATE => {
            serde_json::from_value::<crate::RoofsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_RAILINGS_CREATE => {
            serde_json::from_value::<crate::RailingsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_MASSES_CREATE => {
            serde_json::from_value::<crate::MassesCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_ARCH_EDIT => {
            serde_json::from_value::<crate::ArchEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_TAGS_CREATE => {
            serde_json::from_value::<crate::TagsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_TAGS_CHECK => {
            serde_json::from_value::<crate::TagsCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_TAGS_EDIT => {
            serde_json::from_value::<crate::TagsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_VIEWMARKS_CREATE => {
            serde_json::from_value::<crate::ViewMarksCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_VIEWMARKS_EDIT => {
            serde_json::from_value::<crate::ViewMarksEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_VIEWMARKS_REFERENCES => {
            serde_json::from_value::<crate::ViewMarksReferences>(input.clone()).map(|_| ())
        }
        crate::CAD_SPACES_FINISH => {
            serde_json::from_value::<crate::SpacesFinish>(input.clone()).map(|_| ())
        }
        crate::CAD_TEMPLATES_APPLY => {
            serde_json::from_value::<crate::TemplatesApply>(input.clone()).map(|_| ())
        }
        crate::CAD_PROFILES_APPLY => {
            serde_json::from_value::<crate::ProfilesApply>(input.clone()).map(|_| ())
        }
        crate::CAD_PROFILES_CHECK => {
            serde_json::from_value::<crate::ProfilesCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_PROFILES_IMPORT => {
            serde_json::from_value::<crate::ProfilesImport>(input.clone()).map(|_| ())
        }
        crate::CAD_PROFILES_LIST => {
            serde_json::from_value::<crate::ProfilesList>(input.clone()).map(|_| ())
        }
        crate::CAD_PROFILES_MIGRATE => {
            serde_json::from_value::<crate::ProfilesMigrate>(input.clone()).map(|_| ())
        }
        crate::CAD_PROFILES_COMPARE => {
            serde_json::from_value::<crate::ProfilesCompare>(input.clone()).map(|_| ())
        }
        crate::CAD_PROFILES_CARRY => {
            serde_json::from_value::<crate::ProfilesCarry>(input.clone()).map(|_| ())
        }
        crate::CAD_PROFILES_REMOVE => {
            serde_json::from_value::<crate::ProfilesRemove>(input.clone()).map(|_| ())
        }
        crate::CAD_PROFILES_NAMES => {
            serde_json::from_value::<crate::ProfilesNames>(input.clone()).map(|_| ())
        }
        crate::CAD_SHEETS_SET => {
            serde_json::from_value::<crate::SheetsSet>(input.clone()).map(|_| ())
        }
        crate::CAD_NAVIGATOR_READ => {
            serde_json::from_value::<crate::NavigatorRead>(input.clone()).map(|_| ())
        }
        crate::CAD_XREFS_EDIT => {
            serde_json::from_value::<crate::XrefsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_XREFS_LIST => {
            serde_json::from_value::<crate::XrefsList>(input.clone()).map(|_| ())
        }
        crate::CAD_IMAGES_EDIT => {
            serde_json::from_value::<crate::ImagesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_RASTERS_GEOREF => {
            serde_json::from_value::<crate::RastersGeoref>(input.clone()).map(|_| ())
        }
        crate::CAD_RASTERS_MOSAIC => {
            serde_json::from_value::<crate::RastersMosaic>(input.clone()).map(|_| ())
        }
        crate::CAD_RASTERS_REPROJECT => {
            serde_json::from_value::<crate::RastersReproject>(input.clone()).map(|_| ())
        }
        crate::CAD_RASTERS_GPKGREAD => {
            serde_json::from_value::<crate::RastersGpkgRead>(input.clone()).map(|_| ())
        }
        crate::CAD_RASTERS_GPKGWRITE => {
            serde_json::from_value::<crate::RastersGpkgWrite>(input.clone()).map(|_| ())
        }
        crate::CAD_INSERTS_CLIP => {
            serde_json::from_value::<crate::InsertsClip>(input.clone()).map(|_| ())
        }
        crate::CAD_ASSETS_CHECK => {
            serde_json::from_value::<crate::AssetsCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_STAC_SEARCH => {
            serde_json::from_value::<crate::StacSearch>(input.clone()).map(|_| ())
        }
        crate::CAD_STAC_PREPARE => {
            serde_json::from_value::<crate::StacPrepare>(input.clone()).map(|_| ())
        }
        crate::CAD_TIME_QUERY => {
            serde_json::from_value::<crate::TimeQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_SENSORS_INGEST => {
            serde_json::from_value::<crate::SensorsIngest>(input.clone()).map(|_| ())
        }
        crate::CAD_SENSORS_LATEST => {
            serde_json::from_value::<crate::SensorsLatest>(input.clone()).map(|_| ())
        }
        crate::CAD_POINTCLOUDS_DESCRIBE => {
            serde_json::from_value::<crate::PointCloudsDescribe>(input.clone()).map(|_| ())
        }
        crate::CAD_POINTCLOUDS_QUERY => {
            serde_json::from_value::<crate::PointCloudsQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_POINTCLOUDS_REGION => {
            serde_json::from_value::<crate::PointCloudsRegion>(input.clone()).map(|_| ())
        }
        crate::CAD_POINTCLOUDS_PROFILE => {
            serde_json::from_value::<crate::PointCloudsProfile>(input.clone()).map(|_| ())
        }
        crate::CAD_MESHES_DESCRIBE => {
            serde_json::from_value::<crate::MeshesDescribe>(input.clone()).map(|_| ())
        }
        crate::CAD_MESHES_QUERY => {
            serde_json::from_value::<crate::MeshesQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_MESHES_REGION => {
            serde_json::from_value::<crate::MeshesRegion>(input.clone()).map(|_| ())
        }
        crate::CAD_MESHES_PROFILE => {
            serde_json::from_value::<crate::MeshesProfile>(input.clone()).map(|_| ())
        }
        crate::CAD_SCHEDULES_CREATE => {
            serde_json::from_value::<crate::SchedulesCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_EXTRACT_RUN => {
            serde_json::from_value::<crate::ExtractRun>(input.clone()).map(|_| ())
        }
        crate::CAD_EXTRACT_TABLE => {
            serde_json::from_value::<crate::ExtractTable>(input.clone()).map(|_| ())
        }
        crate::CAD_SCHEDULES_EDIT => {
            serde_json::from_value::<crate::SchedulesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_SPACES_EDIT => {
            serde_json::from_value::<crate::SpacesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_SPACES_QUANTITIES => {
            serde_json::from_value::<crate::SpacesQuantities>(input.clone()).map(|_| ())
        }
        crate::CAD_OPENINGS_EDIT => {
            serde_json::from_value::<crate::OpeningsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_OPENINGS_DISTANCES => {
            serde_json::from_value::<crate::OpeningsDistancesInput>(input.clone()).map(|_| ())
        }
        crate::CAD_OPENING_TYPES_EDIT => {
            serde_json::from_value::<crate::OpeningTypesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_ELEMENT_TYPES_EDIT => {
            serde_json::from_value::<crate::ElementTypesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_MODULES_EDIT => {
            serde_json::from_value::<crate::ModulesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_MATERIALS_EDIT => {
            serde_json::from_value::<crate::MaterialsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_MODULES_QUERY => {
            serde_json::from_value::<crate::ModulesQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_OPTIONS_EDIT => {
            serde_json::from_value::<crate::OptionsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_OPTIONS_QUERY => {
            serde_json::from_value::<crate::OptionsQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_ELEVATIONS_EDIT => {
            serde_json::from_value::<crate::ElevationsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_ELEVATIONS_QUERY => {
            serde_json::from_value::<crate::ElevationsQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_SECTIONS_EDIT => {
            serde_json::from_value::<crate::BuildingSectionsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_SECTIONS_QUERY => {
            serde_json::from_value::<crate::BuildingSectionsQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_OPENINGS_QUERY => {
            serde_json::from_value::<crate::OpeningsQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_ARCH_AUDIT => {
            serde_json::from_value::<crate::ArchAudit>(input.clone()).map(|_| ())
        }
        crate::CAD_ARCH_WALK => {
            serde_json::from_value::<crate::ArchWalk>(input.clone()).map(|_| ())
        }
        crate::CAD_ARCH_EXCEPTIONS => {
            serde_json::from_value::<crate::ArchExceptions>(input.clone()).map(|_| ())
        }
        crate::CAD_PROGRAM_EDIT => {
            serde_json::from_value::<crate::ProgramEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_PROGRAM_CHECK => {
            serde_json::from_value::<crate::ProgramCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_LEVELS_ASSIGN => {
            serde_json::from_value::<crate::LevelsAssign>(input.clone()).map(|_| ())
        }
        crate::CAD_ARCH_PERFORMANCE => {
            serde_json::from_value::<crate::ArchPerformance>(input.clone()).map(|_| ())
        }
        crate::CAD_FORM_ANSWER => {
            serde_json::from_value::<crate::ArchFormAnswers>(input.clone()).map(|_| ())
        }
        crate::CAD_UNDERLAY_EDIT => {
            serde_json::from_value::<crate::UnderlayEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_UNDERLAY_CHECK => {
            serde_json::from_value::<crate::UnderlayCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_DELIVERY_EDIT => {
            serde_json::from_value::<crate::DeliveryEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_ZONING_PARCEL => {
            serde_json::from_value::<crate::ZoningParcel>(input.clone()).map(|_| ())
        }
        crate::CAD_NORTH_CREATE => {
            serde_json::from_value::<crate::NorthCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_REVISIONS_MARK => {
            serde_json::from_value::<crate::RevisionsMark>(input.clone()).map(|_| ())
        }
        crate::CAD_LEGENDS_MATERIALS => {
            serde_json::from_value::<crate::LegendsMaterials>(input.clone()).map(|_| ())
        }
        crate::CAD_CASEWORK_RUN => {
            serde_json::from_value::<crate::CaseworkRun>(input.clone()).map(|_| ())
        }
        crate::CAD_CASEWORK_EDIT => {
            serde_json::from_value::<crate::CaseworkEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_CASEWORK_QUERY => {
            serde_json::from_value::<crate::CaseworkQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_FLATTEN => {
            serde_json::from_value::<crate::EntitiesFlatten>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_ORDER => {
            serde_json::from_value::<crate::EntitiesOrder>(input.clone()).map(|_| ())
        }
        crate::CAD_ANNOTATIONS_SCALES => {
            serde_json::from_value::<crate::AnnotationsScales>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_OFFSET => {
            serde_json::from_value::<crate::EntitiesOffset>(input.clone()).map(|_| ())
        }
        crate::CAD_DIMENSIONS_REASSOCIATE => {
            serde_json::from_value::<crate::DimensionsReassociate>(input.clone()).map(|_| ())
        }
        crate::CAD_DIMENSION_SETS_CREATE => {
            serde_json::from_value::<crate::DimensionSetsCreateInput>(input.clone()).map(|_| ())
        }
        crate::CAD_DIMENSION_SETS_EDIT => {
            serde_json::from_value::<crate::DimensionSetsEditInput>(input.clone()).map(|_| ())
        }
        crate::CAD_DIMENSION_SETS_QUERY => {
            serde_json::from_value::<crate::DimensionSetsQueryInput>(input.clone()).map(|_| ())
        }
        crate::CAD_DIMENSIONS_CREATE => {
            serde_json::from_value::<crate::DimensionsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_DIMENSIONS_EDIT => {
            serde_json::from_value::<crate::DimensionsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_DIMENSIONS_QUERY => {
            serde_json::from_value::<crate::DimensionsQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_FIELDS_EDIT => {
            serde_json::from_value::<crate::FieldsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_TABLES_EDIT => {
            serde_json::from_value::<crate::TablesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_LEADERS_EDIT => {
            serde_json::from_value::<crate::LeadersEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_HATCHES_EDIT => {
            serde_json::from_value::<crate::HatchesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_TEXTS_EDIT => {
            serde_json::from_value::<crate::TextsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_TEXTS_CHECK => {
            serde_json::from_value::<crate::TextsCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_FONTS_REPORT => {
            serde_json::from_value::<crate::FontsReport>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_TRIM | crate::CAD_ENTITIES_EXTEND => {
            serde_json::from_value::<crate::EntitiesTrim>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_FILLET => {
            serde_json::from_value::<crate::EntitiesFillet>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_CHAMFER => {
            serde_json::from_value::<crate::EntitiesChamfer>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_STRETCH => {
            serde_json::from_value::<crate::EntitiesStretch>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_BREAK => {
            serde_json::from_value::<crate::EntitiesBreak>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_JOIN => {
            serde_json::from_value::<crate::EntitiesJoin>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_REVERSE => {
            serde_json::from_value::<crate::EntitiesReverse>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_LENGTHEN => {
            serde_json::from_value::<crate::EntitiesLengthen>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_DIVIDE => {
            serde_json::from_value::<crate::EntitiesDivide>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_BOUNDARY => {
            serde_json::from_value::<crate::EntitiesBoundary>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_HATCH => {
            serde_json::from_value::<crate::EntitiesHatch>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_CREATE => {
            serde_json::from_value::<crate::EntitiesCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_SET => {
            serde_json::from_value::<crate::EntitiesSetProperties>(input.clone()).map(|_| ())
        }
        crate::CAD_BLOCKS_DEFINE => {
            serde_json::from_value::<crate::BlocksDefine>(input.clone()).map(|_| ())
        }
        crate::CAD_BLOCKS_EDIT => {
            serde_json::from_value::<crate::BlocksEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_BLOCKS_INSERT => {
            serde_json::from_value::<crate::BlocksInsert>(input.clone()).map(|_| ())
        }
        crate::CAD_BLOCKS_REPLACE => {
            serde_json::from_value::<crate::BlocksReplace>(input.clone()).map(|_| ())
        }
        crate::CAD_BLOCKS_QUERY => {
            serde_json::from_value::<crate::BlocksQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_BLOCKS_CONVERT => {
            serde_json::from_value::<crate::BlocksConvert>(input.clone()).map(|_| ())
        }
        crate::CAD_BLOCKS_FIND => {
            serde_json::from_value::<crate::BlocksFind>(input.clone()).map(|_| ())
        }
        crate::CAD_BLOCKS_PARAMETERS => {
            serde_json::from_value::<crate::BlocksParameters>(input.clone()).map(|_| ())
        }
        crate::CAD_BLOCKS_PROXY => {
            serde_json::from_value::<crate::BlocksProxy>(input.clone()).map(|_| ())
        }
        crate::CAD_BLOCKS_FREEZE => {
            serde_json::from_value::<crate::BlocksFreeze>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_BLOCK => {
            serde_json::from_value::<crate::ImportBlock>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_BLOCK => {
            serde_json::from_value::<crate::ExportBlock>(input.clone()).map(|_| ())
        }
        crate::FILE_RECOVER => {
            serde_json::from_value::<crate::RecoverKcad>(input.clone()).map(|_| ())
        }
        crate::FILE_EXCHANGE_CHECK => {
            serde_json::from_value::<crate::ExchangeCheck>(input.clone()).map(|_| ())
        }
        crate::FILE_EXCHANGE_MATRIX => {
            serde_json::from_value::<crate::ExchangeMatrix>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_SERVICE => {
            serde_json::from_value::<crate::LayersService>(input.clone()).map(|_| ())
        }
        crate::CAD_NETWORK_DEFINE => {
            serde_json::from_value::<crate::NetworkDefine>(input.clone()).map(|_| ())
        }
        crate::CAD_LINEARREF_DEFINE => {
            serde_json::from_value::<crate::LinearRefDefine>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_CREATE => {
            serde_json::from_value::<crate::TerrainsCreate>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_EDIT => {
            serde_json::from_value::<crate::TerrainsEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_QUERY => {
            serde_json::from_value::<crate::TerrainsQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_CONTOURS => {
            serde_json::from_value::<crate::TerrainsContours>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_SHEET_CONTOURS => {
            serde_json::from_value::<crate::TerrainsSheetContours>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_SECTIONCHECK => {
            serde_json::from_value::<crate::TerrainsSectionCheck>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_ZEROLINE => {
            serde_json::from_value::<crate::TerrainsZeroLine>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_CUTFILL => {
            serde_json::from_value::<crate::TerrainsCutFill>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_LANDXML => {
            serde_json::from_value::<crate::ImportLandxml>(input.clone()).map(|_| ())
        }
        crate::ROAD_ALIGNMENTS_CREATE => {
            serde_json::from_value::<crate::AlignmentsCreate>(input.clone()).map(|_| ())
        }
        crate::ROAD_ALIGNMENTS_EDIT => {
            serde_json::from_value::<crate::AlignmentsEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_ALIGNMENTS_QUERY => {
            serde_json::from_value::<crate::AlignmentsQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_ALIGNMENTS_LABEL => {
            serde_json::from_value::<crate::AlignmentsLabel>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROFILES_CREATE => {
            serde_json::from_value::<crate::ProfilesCreate>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROFILES_EDIT => {
            serde_json::from_value::<crate::ProfilesEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROFILES_QUERY => {
            serde_json::from_value::<crate::ProfilesQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROFILES_DRAW => {
            serde_json::from_value::<crate::ProfilesDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROFILES_SIGHT => {
            serde_json::from_value::<crate::ProfilesSight>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROFILES_PROPOSE => {
            serde_json::from_value::<crate::ProfilesPropose>(input.clone()).map(|_| ())
        }
        crate::ROAD_ALIGNMENTS_CHECK => {
            serde_json::from_value::<crate::AlignmentsCheck>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROFILES_CHECK => {
            serde_json::from_value::<crate::RoadProfilesCheck>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROFILES_LENGTHS => {
            serde_json::from_value::<crate::RoadProfilesLengths>(input.clone()).map(|_| ())
        }
        crate::ROAD_TEMPLATES_EDIT => {
            serde_json::from_value::<crate::TemplatesEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_SECTIONS_EDIT => {
            serde_json::from_value::<crate::SectionsEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_SECTIONS_QUERY => {
            serde_json::from_value::<crate::SectionsQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_SECTIONS_DRAW => {
            serde_json::from_value::<crate::SectionsDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_SUPERELEVATIONS_EDIT => {
            serde_json::from_value::<crate::SuperelevationsEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_SUPERELEVATIONS_QUERY => {
            serde_json::from_value::<crate::SuperelevationsQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_SUPERELEVATIONS_CHECK => {
            serde_json::from_value::<crate::SuperelevationsCheck>(input.clone()).map(|_| ())
        }
        crate::ROAD_SUPERELEVATIONS_WIDEN => {
            serde_json::from_value::<crate::SuperelevationsWiden>(input.clone()).map(|_| ())
        }
        crate::ROAD_SUPERELEVATIONS_TRANSITIONS => {
            serde_json::from_value::<crate::SuperelevationsTransitions>(input.clone()).map(|_| ())
        }
        crate::ROAD_CORRIDORS_EDIT => {
            serde_json::from_value::<crate::CorridorsEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_CORRIDORS_QUERY => {
            serde_json::from_value::<crate::CorridorsQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_CORRIDORS_DRAW => {
            serde_json::from_value::<crate::CorridorsDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_CORRIDORS_WATER => {
            serde_json::from_value::<crate::CorridorsWater>(input.clone()).map(|_| ())
        }
        crate::ROAD_CORRIDORS_DAYLIGHTS => {
            serde_json::from_value::<crate::CorridorsDaylights>(input.clone()).map(|_| ())
        }
        crate::ROAD_CORRIDORS_PROTECTED => {
            serde_json::from_value::<crate::CorridorsProtected>(input.clone()).map(|_| ())
        }
        crate::ROAD_CORRIDORS_PREFLIGHT => {
            serde_json::from_value::<crate::CorridorsPreflight>(input.clone()).map(|_| ())
        }
        crate::ROAD_VOLUMES_EDIT => {
            serde_json::from_value::<crate::VolumesEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_VOLUMES_QUERY => {
            serde_json::from_value::<crate::VolumesQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_VOLUMES_APPROVE => {
            serde_json::from_value::<crate::VolumesApprove>(input.clone()).map(|_| ())
        }
        crate::ROAD_VOLUMES_UNCERTAINTY => {
            serde_json::from_value::<crate::VolumesUncertainty>(input.clone()).map(|_| ())
        }
        crate::ROAD_VOLUMES_PROGRESS => {
            serde_json::from_value::<crate::VolumesProgress>(input.clone()).map(|_| ())
        }
        crate::ROAD_VOLUMES_COMPARE => {
            serde_json::from_value::<crate::VolumesCompare>(input.clone()).map(|_| ())
        }
        crate::ROAD_VOLUMES_LEDGER => {
            serde_json::from_value::<crate::VolumesLedger>(input.clone()).map(|_| ())
        }
        crate::ROAD_VOLUMES_STATES => {
            serde_json::from_value::<crate::VolumesStates>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROFILES_IMPACT => {
            serde_json::from_value::<crate::ProfilesImpact>(input.clone()).map(|_| ())
        }
        crate::ROAD_ALIGNMENTS_IMPACT => {
            serde_json::from_value::<crate::AlignmentsImpact>(input.clone()).map(|_| ())
        }
        crate::ROAD_VOLUMES_HISTORY => {
            serde_json::from_value::<crate::VolumesHistory>(input.clone()).map(|_| ())
        }
        crate::ROAD_HAULS_EDIT => {
            serde_json::from_value::<crate::HaulsEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_HAULS_QUERY => {
            serde_json::from_value::<crate::HaulsQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_HAULS_PLAN => {
            serde_json::from_value::<crate::HaulsPlan>(input.clone()).map(|_| ())
        }
        crate::ROAD_HAULS_APPROVE => {
            serde_json::from_value::<crate::HaulsApprove>(input.clone()).map(|_| ())
        }
        crate::ROAD_HAULS_NETWORK => {
            serde_json::from_value::<crate::HaulsNetwork>(input.clone()).map(|_| ())
        }
        crate::ROAD_HAULS_TRUCKS => {
            serde_json::from_value::<crate::HaulsTrucks>(input.clone()).map(|_| ())
        }
        crate::ROAD_GUARDRAILS_EDIT => {
            serde_json::from_value::<crate::GuardrailsEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_GUARDRAILS_QUERY => {
            serde_json::from_value::<crate::GuardrailsQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_GUARDRAILS_DRAW => {
            serde_json::from_value::<crate::GuardrailsDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_CULVERTS_EDIT => {
            serde_json::from_value::<crate::CulvertsEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_CULVERTS_QUERY => {
            serde_json::from_value::<crate::CulvertsQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_CULVERTS_DRAW => {
            serde_json::from_value::<crate::CulvertsDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_CULVERTS_HYDRAULICS => {
            serde_json::from_value::<crate::CulvertsHydraulics>(input.clone()).map(|_| ())
        }
        crate::ROAD_CULVERTS_CLOSURE => {
            serde_json::from_value::<crate::CulvertsClosure>(input.clone()).map(|_| ())
        }
        crate::ROAD_PLATFORMS_EDIT => {
            serde_json::from_value::<crate::PlatformsEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_FROM_CLOUD => {
            serde_json::from_value::<crate::TerrainsFromCloud>(input.clone()).map(|_| ())
        }
        crate::ROAD_DRAINAGE_QUERY => {
            serde_json::from_value::<crate::DrainageQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_DRAINAGE_DRAW => {
            serde_json::from_value::<crate::DrainageDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_SWEPT_QUERY => {
            serde_json::from_value::<crate::SweptQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_ANALYSIS => {
            serde_json::from_value::<crate::TerrainsAnalysis>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_COMPARE => {
            serde_json::from_value::<crate::TerrainsCompare>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_QUALITY => {
            serde_json::from_value::<crate::TerrainsQuality>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_REPAIR => {
            serde_json::from_value::<crate::TerrainsRepair>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_COVERAGE => {
            serde_json::from_value::<crate::TerrainsCoverage>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_LOW_CONFIDENCE_DRAW => {
            serde_json::from_value::<crate::TerrainsCoverageDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_DENSITY => {
            serde_json::from_value::<crate::TerrainsDensity>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_STOCKPILE => {
            serde_json::from_value::<crate::TerrainsStockpile>(input.clone()).map(|_| ())
        }
        crate::ROAD_TERRAINS_SPARSE_DRAW => {
            serde_json::from_value::<crate::TerrainsSparseDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_SWEPT_DRAW => {
            serde_json::from_value::<crate::SweptDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_PLATFORMS_QUERY => {
            serde_json::from_value::<crate::PlatformsQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_PLATFORMS_DRAW => {
            serde_json::from_value::<crate::PlatformsDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_WALLS_EDIT => {
            serde_json::from_value::<crate::RetainingWallsEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_WALLS_QUERY => {
            serde_json::from_value::<crate::RetainingWallsQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_WALLS_DRAW => {
            serde_json::from_value::<crate::RetainingWallsDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_PAY_EDIT => serde_json::from_value::<crate::PayEdit>(input.clone()).map(|_| ()),
        crate::ROAD_PAY_QUERY => {
            serde_json::from_value::<crate::PayQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_ROUTES_FIND => {
            serde_json::from_value::<crate::RoutesFind>(input.clone()).map(|_| ())
        }
        crate::ROAD_ROUTES_DRAW => {
            serde_json::from_value::<crate::RoutesDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_JUNCTIONS_EDIT => {
            serde_json::from_value::<crate::JunctionsEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_JUNCTIONS_QUERY => {
            serde_json::from_value::<crate::JunctionsQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_JUNCTIONS_CLEARANCE => {
            serde_json::from_value::<crate::JunctionsClearance>(input.clone()).map(|_| ())
        }
        crate::ROAD_JUNCTIONS_TIE => {
            serde_json::from_value::<crate::JunctionsTie>(input.clone()).map(|_| ())
        }
        crate::ROAD_JUNCTIONS_OVERLAP => {
            serde_json::from_value::<crate::JunctionsOverlap>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROJECT_QUERY => {
            serde_json::from_value::<crate::RoadProjectQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROJECT_GRAPH => {
            serde_json::from_value::<crate::RoadProjectGraphQuery>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROJECT_REBUILD => {
            serde_json::from_value::<crate::RoadProjectRebuild>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROJECT_PRODUCE => {
            serde_json::from_value::<crate::RoadProjectProduce>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROJECT_EDIT => {
            serde_json::from_value::<crate::RoadProjectEdit>(input.clone()).map(|_| ())
        }
        crate::MAP_WORK_QUERY => {
            serde_json::from_value::<crate::MapWorkQuery>(input.clone()).map(|_| ())
        }
        crate::MAP_WORK_EDIT => {
            serde_json::from_value::<crate::MapWorkEdit>(input.clone()).map(|_| ())
        }
        crate::MAP_WORK_RESTORE => {
            serde_json::from_value::<crate::MapWorkRestore>(input.clone()).map(|_| ())
        }
        crate::CAD_CRS_FACTORS => {
            serde_json::from_value::<crate::CrsFactors>(input.clone()).map(|_| ())
        }
        crate::MAP_FEATURES_CODE => {
            serde_json::from_value::<crate::FeaturesCode>(input.clone()).map(|_| ())
        }
        crate::CAD_SURVEY_BASELINES => {
            serde_json::from_value::<crate::SurveyBaselines>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_GRADE => {
            serde_json::from_value::<crate::EntitiesGrade>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_RAISE => {
            serde_json::from_value::<crate::EntitiesRaise>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_DRAPE => {
            serde_json::from_value::<crate::EntitiesDrape>(input.clone()).map(|_| ())
        }
        crate::CAD_TOPOLOGY_CLEAN => {
            serde_json::from_value::<crate::TopologyClean>(input.clone()).map(|_| ())
        }
        crate::CAD_PARCELS_CHECK => {
            serde_json::from_value::<crate::ParcelsCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_PARCELS_BOUNDARIES => {
            serde_json::from_value::<crate::ParcelsBoundaries>(input.clone()).map(|_| ())
        }
        crate::CAD_PARCELS_RECORD => {
            serde_json::from_value::<crate::ParcelsRecord>(input.clone()).map(|_| ())
        }
        crate::CAD_PARCELS_RECORDS => {
            serde_json::from_value::<crate::ParcelsRecords>(input.clone()).map(|_| ())
        }
        crate::CAD_PARCELS_REPORT => {
            serde_json::from_value::<crate::ParcelsReport>(input.clone()).map(|_| ())
        }
        crate::CAD_PARCELS_TABLE => {
            serde_json::from_value::<crate::ParcelsTable>(input.clone()).map(|_| ())
        }
        crate::CAD_PARCELS_APPROVE => {
            serde_json::from_value::<crate::ParcelsApprove>(input.clone()).map(|_| ())
        }
        crate::CAD_ASBUILT_REVIEW => {
            serde_json::from_value::<crate::AsbuiltReview>(input.clone()).map(|_| ())
        }
        crate::CAD_PARCELS_WITHDRAW => {
            serde_json::from_value::<crate::ParcelsWithdraw>(input.clone()).map(|_| ())
        }
        crate::CAD_PIPES_CHECK => {
            serde_json::from_value::<crate::PipesCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_PIPES_PROFILE => {
            serde_json::from_value::<crate::PipesProfile>(input.clone()).map(|_| ())
        }
        crate::CAD_ASBUILT_CHECK => {
            serde_json::from_value::<crate::AsbuiltCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_FIELD_PACKAGE => {
            serde_json::from_value::<crate::FieldPackage>(input.clone()).map(|_| ())
        }
        crate::CAD_ZONING_CHECK => {
            serde_json::from_value::<crate::ZoningCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_ZONING_SYMBOLIZE => {
            serde_json::from_value::<crate::ZoningSymbolize>(input.clone()).map(|_| ())
        }
        crate::CAD_ZONING_DRAFT => {
            serde_json::from_value::<crate::ZoningDraft>(input.clone()).map(|_| ())
        }
        crate::CAD_SCENARIOS_IMPACT => {
            serde_json::from_value::<crate::ScenariosImpact>(input.clone()).map(|_| ())
        }
        crate::CAD_SCENARIOS_RECORD => {
            serde_json::from_value::<crate::ScenariosRecord>(input.clone()).map(|_| ())
        }
        crate::CAD_READJUSTMENT_DISTRIBUTE => {
            serde_json::from_value::<crate::ReadjustmentDistribute>(input.clone()).map(|_| ())
        }
        crate::CAD_READJUSTMENT_SCOPE => {
            serde_json::from_value::<crate::ReadjustmentScope>(input.clone()).map(|_| ())
        }
        crate::CAD_READJUSTMENT_SHEETS => {
            serde_json::from_value::<crate::ReadjustmentSheets>(input.clone()).map(|_| ())
        }
        crate::CAD_ORTHOPHOTO_CHECK => {
            serde_json::from_value::<crate::OrthophotoCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_POINTCLOUDS_REGISTER => {
            serde_json::from_value::<crate::PointCloudsRegister>(input.clone()).map(|_| ())
        }
        crate::CAD_POINTCLOUDS_NETWORK => {
            serde_json::from_value::<crate::PointCloudsNetwork>(input.clone()).map(|_| ())
        }
        crate::CAD_POINTCLOUDS_PLACE => {
            serde_json::from_value::<crate::PointCloudsPlace>(input.clone()).map(|_| ())
        }
        crate::CAD_ORTHOPHOTO_COMPARE => {
            serde_json::from_value::<crate::OrthophotoCompare>(input.clone()).map(|_| ())
        }
        crate::CAD_ORTHOPHOTO_SEAMLINES => {
            serde_json::from_value::<crate::OrthophotoSeamlines>(input.clone()).map(|_| ())
        }
        crate::CAD_ORTHOPHOTO_NETWORK => {
            serde_json::from_value::<crate::OrthophotoNetwork>(input.clone()).map(|_| ())
        }
        crate::CAD_ORTHOPHOTO_FOOTPRINT => {
            serde_json::from_value::<crate::OrthophotoFootprint>(input.clone()).map(|_| ())
        }
        crate::CAD_POINTCLOUDS_OVERLAP => {
            serde_json::from_value::<crate::PointCloudsOverlap>(input.clone()).map(|_| ())
        }
        crate::MAP_SURVEY_QUERY => {
            serde_json::from_value::<crate::MapSurveyQuery>(input.clone()).map(|_| ())
        }
        crate::MAP_SURVEY_NETWORK => {
            serde_json::from_value::<crate::MapSurveyNetwork>(input.clone()).map(|_| ())
        }
        crate::MAP_FEATURES_RECONCILE => {
            serde_json::from_value::<crate::FeaturesReconcile>(input.clone()).map(|_| ())
        }
        crate::MAP_SURVEY_EDIT => {
            serde_json::from_value::<crate::MapSurveyEdit>(input.clone()).map(|_| ())
        }
        crate::ROAD_JUNCTIONS_DRAW => {
            serde_json::from_value::<crate::JunctionsDraw>(input.clone()).map(|_| ())
        }
        crate::ROAD_STAKEOUT_QUERY => {
            serde_json::from_value::<crate::StakeoutQuery>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_LANDXML => {
            serde_json::from_value::<crate::ExportLandxml>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_IFC => {
            serde_json::from_value::<crate::ExportIfc>(input.clone()).map(|_| ())
        }
        crate::CAD_ARCH_IFC => {
            serde_json::from_value::<crate::ExportIfcBuilding>(input.clone()).map(|_| ())
        }
        crate::CAD_IFC_INSPECT => {
            serde_json::from_value::<crate::IfcInspect>(input.clone()).map(|_| ())
        }
        crate::CAD_IFC_COMPARE => {
            serde_json::from_value::<crate::IfcCompare>(input.clone()).map(|_| ())
        }
        crate::CAD_BCF_WRITE => {
            serde_json::from_value::<crate::BcfWrite>(input.clone()).map(|_| ())
        }
        crate::CAD_BCF_READ => serde_json::from_value::<crate::BcfRead>(input.clone()).map(|_| ()),
        crate::FILE_EXPORT_GLTF => {
            serde_json::from_value::<crate::ExportGltf>(input.clone()).map(|_| ())
        }
        crate::ROAD_STAKEOUT_SEND => {
            serde_json::from_value::<crate::StakeoutSend>(input.clone()).map(|_| ())
        }
        crate::ROAD_SHEETS_PLAN => {
            serde_json::from_value::<crate::RoadSheetsPlan>(input.clone()).map(|_| ())
        }
        crate::ROAD_SHEETS_SECTIONS => {
            serde_json::from_value::<crate::RoadSheetsSections>(input.clone()).map(|_| ())
        }
        crate::ROAD_ALIGNMENTS_FIT => {
            serde_json::from_value::<crate::AlignmentsFit>(input.clone()).map(|_| ())
        }
        crate::ROAD_PROFILES_FIT => {
            serde_json::from_value::<crate::ProfilesFit>(input.clone()).map(|_| ())
        }
        crate::ROAD_ALIGNMENTS_IMPORT => {
            serde_json::from_value::<crate::AlignmentsImport>(input.clone()).map(|_| ())
        }
        crate::CAD_SUBNETWORKS_FIND => {
            serde_json::from_value::<crate::SubnetworksFind>(input.clone()).map(|_| ())
        }
        crate::CAD_SUBNETWORKS_UPDATE => {
            serde_json::from_value::<crate::SubnetworksUpdate>(input.clone()).map(|_| ())
        }
        crate::CAD_ADDRESSES_DEFINE => {
            serde_json::from_value::<crate::AddressesDefine>(input.clone()).map(|_| ())
        }
        crate::CAD_ADDRESSES_CHECK => {
            serde_json::from_value::<crate::AddressesCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_ADDRESSES_COMPARE => {
            serde_json::from_value::<crate::AddressesCompare>(input.clone()).map(|_| ())
        }
        crate::CAD_ADDRESSES_NORMALIZE => {
            serde_json::from_value::<crate::AddressesNormalize>(input.clone()).map(|_| ())
        }
        crate::CAD_TOPOLOGY_CHECK => {
            serde_json::from_value::<crate::TopologyCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_TOPOLOGY_EXCEPTIONS => {
            serde_json::from_value::<crate::TopologyExceptions>(input.clone()).map(|_| ())
        }
        crate::CAD_TOPOLOGY_REPAIR => {
            serde_json::from_value::<crate::TopologyRepair>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_TIME => {
            serde_json::from_value::<crate::LayersTime>(input.clone()).map(|_| ())
        }
        crate::CAD_SCENARIOS_EDIT => {
            serde_json::from_value::<crate::ScenariosEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_FILTER => {
            serde_json::from_value::<crate::LayersFilter>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_DATA => {
            serde_json::from_value::<crate::LayersData>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_ROWS => {
            serde_json::from_value::<crate::LayersRows>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_CLASSES => {
            serde_json::from_value::<crate::LayersClasses>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_EXPORT => {
            serde_json::from_value::<crate::LayersExport>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_CREATE => {
            serde_json::from_value::<crate::LayersCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_COLUMNS => {
            serde_json::from_value::<crate::LayersColumns>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_MIGRATE => {
            serde_json::from_value::<crate::LayersMigrate>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_SESSION => {
            serde_json::from_value::<crate::LayersSession>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_CRS => {
            serde_json::from_value::<crate::LayersCrs>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_REPROJECT => {
            serde_json::from_value::<crate::LayersReproject>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_TABLE => {
            serde_json::from_value::<crate::LayersTable>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_SUBTYPES => {
            serde_json::from_value::<crate::LayersSubtypes>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_CONTINGENTS => {
            serde_json::from_value::<crate::LayersContingents>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_RULES => {
            serde_json::from_value::<crate::LayersRules>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_INDEXES => {
            serde_json::from_value::<crate::LayersIndexes>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_COMPOSE => {
            serde_json::from_value::<crate::LayersCompose>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_INFO => {
            serde_json::from_value::<crate::LayersInfo>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_LABELS => {
            serde_json::from_value::<crate::LayersLabels>(input.clone()).map(|_| ())
        }
        crate::CAD_LABELS_PIN => {
            serde_json::from_value::<crate::LabelsPin>(input.clone()).map(|_| ())
        }
        crate::CAD_LABELS_REPORT => {
            serde_json::from_value::<crate::LabelsReport>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_RENDERER => {
            serde_json::from_value::<crate::LayersRenderer>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_EDIT => {
            serde_json::from_value::<crate::LayersEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_SET => {
            serde_json::from_value::<crate::LayersSet>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_STATES => {
            serde_json::from_value::<crate::LayersStates>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_FIND => {
            serde_json::from_value::<crate::LayersFind>(input.clone()).map(|_| ())
        }
        crate::CAD_FIELDS_USAGE => {
            serde_json::from_value::<crate::FieldsUsage>(input.clone()).map(|_| ())
        }
        crate::CAD_LAYERS_TRANSLATE => {
            serde_json::from_value::<crate::LayersTranslate>(input.clone()).map(|_| ())
        }
        crate::CAD_STANDARDS_PALETTES => {
            serde_json::from_value::<crate::StandardsPalettes>(input.clone()).map(|_| ())
        }
        crate::CAD_STANDARDS_RESOLVE => {
            serde_json::from_value::<crate::StandardsResolve>(input.clone()).map(|_| ())
        }
        crate::CAD_STANDARDS_CONFLICTS => {
            serde_json::from_value::<crate::StandardsConflicts>(input.clone()).map(|_| ())
        }
        crate::CAD_STANDARDS_INSTALL => {
            serde_json::from_value::<crate::StandardsInstall>(input.clone()).map(|_| ())
        }
        crate::CAD_STANDARDS_EXCEPTIONS => {
            serde_json::from_value::<crate::StandardsExceptions>(input.clone()).map(|_| ())
        }
        crate::CAD_DRAWING_REPAIR => {
            serde_json::from_value::<crate::DrawingRepair>(input.clone()).map(|_| ())
        }
        crate::CAD_DRAWING_TAKE => {
            serde_json::from_value::<crate::DrawingTake>(input.clone()).map(|_| ())
        }
        crate::CAD_REVIEW_NOTES_EDIT => {
            serde_json::from_value::<crate::ReviewNotesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_REVIEW_NOTES_LIST => {
            serde_json::from_value::<crate::ReviewNotesList>(input.clone()).map(|_| ())
        }
        crate::CAD_STANDARDS_CHECK => {
            serde_json::from_value::<crate::StandardsCheck>(input.clone()).map(|_| ())
        }
        crate::CAD_LINE_TYPES_EDIT => {
            serde_json::from_value::<crate::LineTypesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_HATCH_PATTERNS_EDIT => {
            serde_json::from_value::<crate::HatchPatternsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_LIBRARY_EDIT => {
            serde_json::from_value::<crate::LibraryEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_STYLES_IMPACT => {
            serde_json::from_value::<crate::StylesImpact>(input.clone()).map(|_| ())
        }
        crate::CAD_STANDARDS_APPLY => {
            serde_json::from_value::<crate::StandardsApply>(input.clone()).map(|_| ())
        }
        crate::CAD_STYLES_EDIT => {
            serde_json::from_value::<crate::StylesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_SETTINGS_SET => {
            serde_json::from_value::<crate::SettingsSet>(input.clone()).map(|_| ())
        }
        crate::CAD_UCS_SET => serde_json::from_value::<crate::UcsSet>(input.clone()).map(|_| ()),
        crate::CAD_VIEWS_EDIT => {
            serde_json::from_value::<crate::ViewsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_FORMS_EDIT => {
            serde_json::from_value::<crate::FormsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_FORMS_VALIDATE => {
            serde_json::from_value::<crate::FormsValidate>(input.clone()).map(|_| ())
        }
        crate::CAD_FORMS_SUBMIT => {
            serde_json::from_value::<crate::FormsSubmit>(input.clone()).map(|_| ())
        }
        crate::CAD_FORMS_FILL => {
            serde_json::from_value::<crate::FormsFill>(input.clone()).map(|_| ())
        }
        crate::CAD_JOINS_EDIT => {
            serde_json::from_value::<crate::JoinsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_JOINS_QUERY => {
            serde_json::from_value::<crate::JoinsQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_DATASETS_EDIT => {
            serde_json::from_value::<crate::DatasetsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_DATASETS_DESCRIBE => {
            serde_json::from_value::<crate::DatasetsDescribe>(input.clone()).map(|_| ())
        }
        crate::CAD_RELATIONS_EDIT => {
            serde_json::from_value::<crate::RelationsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_FORMS_AUDIT => {
            serde_json::from_value::<crate::FormsAudit>(input.clone()).map(|_| ())
        }
        crate::CAD_FORMS_TEST => {
            serde_json::from_value::<crate::FormsTest>(input.clone()).map(|_| ())
        }
        crate::CAD_DOMAINS_EDIT => {
            serde_json::from_value::<crate::DomainsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_DOMAINS_EXPORT => {
            serde_json::from_value::<crate::DomainsExport>(input.clone()).map(|_| ())
        }
        crate::CAD_FIELDS_MIGRATE => {
            serde_json::from_value::<crate::FieldsMigrate>(input.clone()).map(|_| ())
        }
        crate::CAD_ATTACHMENTS_INSPECT => {
            serde_json::from_value::<crate::AttachmentsInspect>(input.clone()).map(|_| ())
        }
        crate::CAD_SCHEMA_DESCRIBE => {
            serde_json::from_value::<crate::SchemaDescribe>(input.clone()).map(|_| ())
        }
        crate::CAD_FORMS_EXPORT => {
            serde_json::from_value::<crate::FormsExport>(input.clone()).map(|_| ())
        }
        crate::CAD_FORMS_IMPORT => {
            serde_json::from_value::<crate::FormsImport>(input.clone()).map(|_| ())
        }
        crate::CAD_FORMS_TRANSITION => {
            serde_json::from_value::<crate::FormsTransition>(input.clone()).map(|_| ())
        }
        crate::CAD_FORMS_COMPARE => {
            serde_json::from_value::<crate::FormsCompare>(input.clone()).map(|_| ())
        }
        crate::CAD_FORMS_SOURCES => {
            serde_json::from_value::<crate::FormsSources>(input.clone()).map(|_| ())
        }
        crate::CAD_FORMS_RECOMPUTE => {
            serde_json::from_value::<crate::FormsRecompute>(input.clone()).map(|_| ())
        }
        crate::CAD_RECORDS_CREATE => {
            serde_json::from_value::<crate::RecordsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_RECORDS_DUPLICATE => {
            serde_json::from_value::<crate::RecordsDuplicate>(input.clone()).map(|_| ())
        }
        crate::CAD_SHEETS_LEVELS => {
            serde_json::from_value::<crate::SheetsLevels>(input.clone()).map(|_| ())
        }
        crate::CAD_SHEETS_DETAILS => {
            serde_json::from_value::<crate::SheetsDetails>(input.clone()).map(|_| ())
        }
        crate::CAD_PLOTSTYLES_IMPORT => {
            serde_json::from_value::<crate::PlotStylesImport>(input.clone()).map(|_| ())
        }
        crate::CAD_SHEETS_CREATE => {
            serde_json::from_value::<crate::SheetsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_SHEETS_FIT => {
            serde_json::from_value::<crate::SheetsFit>(input.clone()).map(|_| ())
        }
        crate::CAD_SHEETS_EDIT => {
            serde_json::from_value::<crate::SheetsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_UNITS_CHANGE => {
            serde_json::from_value::<crate::UnitsChange>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_DXF | crate::FILE_IMPORT_DWG => {
            serde_json::from_value::<crate::ImportDxf>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_PDF => {
            serde_json::from_value::<crate::ImportPdf>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_GPKG => {
            serde_json::from_value::<crate::ImportGpkg>(input.clone()).map(|_| ())
        }
        crate::CAD_GIS_DERIVATION => {
            serde_json::from_value::<crate::GisDerivation>(input.clone()).map(|_| ())
        }
        crate::CAD_GIS_DETACH => {
            serde_json::from_value::<crate::GisDetach>(input.clone()).map(|_| ())
        }
        crate::CAD_GIS_TAKEBACK => {
            serde_json::from_value::<crate::GisTakeback>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_GIS => {
            serde_json::from_value::<crate::ExportGis>(input.clone()).map(|_| ())
        }
        crate::CAD_SOURCES_DESCRIBE => {
            serde_json::from_value::<crate::SourcesDescribe>(input.clone()).map(|_| ())
        }
        crate::CAD_SOURCES_EDIT => {
            serde_json::from_value::<crate::SourcesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_SOURCES_SYNC => {
            serde_json::from_value::<crate::SourcesSync>(input.clone()).map(|_| ())
        }
        crate::CAD_SOURCES_RELATE => {
            serde_json::from_value::<crate::SourcesRelate>(input.clone()).map(|_| ())
        }
        crate::CAD_SOURCES_RECONCILE => {
            serde_json::from_value::<crate::SourcesReconcile>(input.clone()).map(|_| ())
        }
        crate::CAD_RECORDS_PAGE => {
            serde_json::from_value::<crate::RecordsPage>(input.clone()).map(|_| ())
        }
        crate::CAD_SOURCES_QUERY => {
            serde_json::from_value::<crate::SourcesQuery>(input.clone()).map(|_| ())
        }
        crate::CAD_FIELDS_WRITE => {
            serde_json::from_value::<crate::FieldsWrite>(input.clone()).map(|_| ())
        }
        crate::CAD_PROCESSING_TOOLS => {
            serde_json::from_value::<crate::ProcessingTools>(input.clone()).map(|_| ())
        }
        crate::CAD_PROCESSING_RUN => {
            serde_json::from_value::<crate::ProcessingRun>(input.clone()).map(|_| ())
        }
        crate::CAD_PROCESSING_BATCH => {
            serde_json::from_value::<crate::ProcessingBatch>(input.clone()).map(|_| ())
        }
        crate::CAD_CHANGESET_APPLY => {
            serde_json::from_value::<crate::ChangesetApply>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_DXF => {
            serde_json::from_value::<crate::ExportDxf>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_DWG => {
            serde_json::from_value::<crate::ExportDwg>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_GPKG => {
            serde_json::from_value::<crate::ExportGpkg>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_SHP => {
            serde_json::from_value::<crate::ExportShp>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_CSV => {
            serde_json::from_value::<crate::ExportCsv>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_ATLAS => {
            serde_json::from_value::<crate::ExportAtlas>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_CSV => {
            serde_json::from_value::<crate::ImportCsv>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_FGB => {
            serde_json::from_value::<crate::ExportFgb>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_FGB => {
            serde_json::from_value::<crate::ImportFgb>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_PARQUET => {
            serde_json::from_value::<crate::ExportParquet>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_PARQUET => {
            serde_json::from_value::<crate::ImportParquet>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_SHP => {
            serde_json::from_value::<crate::ImportShp>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_GEOJSON => {
            serde_json::from_value::<crate::ImportGeojson>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_KML => {
            serde_json::from_value::<crate::ImportKml>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_GML => {
            serde_json::from_value::<crate::ExportGml>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_GML => {
            serde_json::from_value::<crate::ImportGml>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_XYZ => {
            serde_json::from_value::<crate::ExportXyz>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_TABLE => {
            serde_json::from_value::<crate::ExportTable>(input.clone()).map(|_| ())
        }
        crate::CAD_SCHEDULES_QUERY => {
            serde_json::from_value::<crate::SchedulesQuery>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_SLD => {
            serde_json::from_value::<crate::ExportSld>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_SLD => {
            serde_json::from_value::<crate::ImportSld>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_QML => {
            serde_json::from_value::<crate::ExportQml>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_QML => {
            serde_json::from_value::<crate::ImportQml>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_MAPLIBRE => {
            serde_json::from_value::<crate::ExportMaplibre>(input.clone()).map(|_| ())
        }
        crate::FILE_IMPORT_MAPLIBRE => {
            serde_json::from_value::<crate::ImportMaplibre>(input.clone()).map(|_| ())
        }
        crate::FILE_PACKAGE => {
            serde_json::from_value::<crate::PackageProject>(input.clone()).map(|_| ())
        }
        crate::FILE_EXPORT_PDF => {
            serde_json::from_value::<crate::ExportPdf>(input.clone()).map(|_| ())
        }
        crate::FILE_PREFLIGHT_SHEETS => {
            serde_json::from_value::<crate::PreflightSheets>(input.clone()).map(|_| ())
        }
        crate::FILE_PUBLISH_SHEETS => {
            serde_json::from_value::<crate::PublishSheets>(input.clone()).map(|_| ())
        }
        crate::FILE_PUBLISH_STATUS => {
            serde_json::from_value::<crate::PublishStatus>(input.clone()).map(|_| ())
        }
        crate::FILE_REVIEW_PACKAGE => {
            serde_json::from_value::<crate::ReviewPackage>(input.clone()).map(|_| ())
        }
        crate::CAD_SNAP_FIND => {
            serde_json::from_value::<crate::SnapFind>(input.clone()).map(|_| ())
        }
        crate::CAD_DONUT_CREATE => {
            serde_json::from_value::<crate::DonutCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_RECTANGLE_CREATE => {
            serde_json::from_value::<crate::RectangleCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_POLYGON_REGULAR => {
            serde_json::from_value::<crate::RegularPolygonCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_SECTOR_CREATE => {
            serde_json::from_value::<crate::SectorCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_SHAPES_EDIT => {
            serde_json::from_value::<crate::ShapesEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_OBJECTS_DERIVE => {
            serde_json::from_value::<crate::ObjectsDerive>(input.clone()).map(|_| ())
        }
        crate::CAD_ROADS_EDIT => {
            serde_json::from_value::<crate::RoadsEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_IMAGES_CLIP => {
            serde_json::from_value::<crate::ImagesClip>(input.clone()).map(|_| ())
        }
        crate::CAD_TIME_VERSION => {
            serde_json::from_value::<crate::TimeVersion>(input.clone()).map(|_| ())
        }
        crate::CAD_SURVEY_POINTS => {
            serde_json::from_value::<crate::SurveyPoints>(input.clone()).map(|_| ())
        }
        crate::CAD_SURVEY_ADJUST => {
            serde_json::from_value::<crate::SurveyAdjust>(input.clone()).map(|_| ())
        }
        crate::CAD_ANNOTATIONS_WRITE => {
            serde_json::from_value::<crate::AnnotationsWrite>(input.clone()).map(|_| ())
        }
        crate::CAD_AREAS_EDIT => {
            serde_json::from_value::<crate::AreasEdit>(input.clone()).map(|_| ())
        }
        crate::CAD_CONSTRUCTIONS_CREATE => {
            serde_json::from_value::<crate::ConstructionsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_PARCELS_CREATE => {
            serde_json::from_value::<crate::ParcelsCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_ADJOIN => {
            serde_json::from_value::<crate::EntitiesAdjoin>(input.clone()).map(|_| ())
        }
        crate::CAD_REVCLOUD_CREATE => {
            serde_json::from_value::<crate::RevcloudCreate>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_VERTEX => {
            serde_json::from_value::<crate::EntitiesVertex>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_PICK => {
            serde_json::from_value::<crate::EntitiesPick>(input.clone()).map(|_| ())
        }
        crate::CAD_CRS_TRANSFORM => {
            serde_json::from_value::<crate::CrsTransform>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_RELATE => {
            serde_json::from_value::<crate::EntitiesRelate>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_SELECT => {
            serde_json::from_value::<crate::EntitiesSelect>(input.clone()).map(|_| ())
        }
        crate::CAD_ENTITIES_ISOLATE => {
            serde_json::from_value::<crate::EntitiesIsolate>(input.clone()).map(|_| ())
        }
        crate::CAD_POINTS_CONSTRUCT => {
            serde_json::from_value::<crate::PointsConstruct>(input.clone()).map(|_| ())
        }
        crate::CAD_CURVES_CONSTRUCT => {
            serde_json::from_value::<crate::CurvesConstruct>(input.clone()).map(|_| ())
        }
        other => panic!("{other}: add its input type to this test"),
    }
}
