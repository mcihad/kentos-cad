//! The command registry's part of GIS commands (docs/adr/8054 §9): `gis_commands!` calls `crate::gis_dispatch::gis_arm!($command, $then, module, InputType)`
//! for one of them, else gives `$else`. The registry (core/registry.rs) chains the parts; the handlers' names are
//! resolved where the registry calls it.

macro_rules! gis_commands {
    ($command:expr, $then:ident, $else:expr) => {
        match $command {
            kentos_commands::CAD_LAYERS_SERVICE => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_service, LayersService)
            }
            kentos_commands::CAD_NETWORK_DEFINE => {
                crate::gis_dispatch::gis_arm!($command, $then, network_define, NetworkDefine)
            }
            kentos_commands::CAD_LINEARREF_DEFINE => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    linearref_define,
                    kentos_commands::LinearRefDefine
                )
            }
            kentos_commands::CAD_TOPOLOGY_CLEAN => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    topology_clean,
                    kentos_commands::TopologyClean
                )
            }
            kentos_commands::CAD_SCENARIOS_IMPACT => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    scenarios_impact,
                    kentos_commands::ScenariosImpact
                )
            }
            kentos_commands::CAD_SCENARIOS_RECORD => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    scenarios_record,
                    kentos_commands::ScenariosRecord
                )
            }
            kentos_commands::CAD_SUBNETWORKS_FIND => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    subnetworks,
                    kentos_commands::SubnetworksFind
                )
            }
            kentos_commands::CAD_SUBNETWORKS_UPDATE => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    subnetworks_update,
                    kentos_commands::SubnetworksUpdate
                )
            }
            kentos_commands::CAD_SOURCES_DESCRIBE => {
                crate::gis_dispatch::gis_arm!($command, $then, sources_describe, SourcesDescribe)
            }
            kentos_commands::CAD_SOURCES_EDIT => {
                crate::gis_dispatch::gis_arm!($command, $then, sources_edit, SourcesEdit)
            }
            kentos_commands::CAD_SOURCES_SYNC => {
                crate::gis_dispatch::gis_arm!($command, $then, sources_sync, SourcesSync)
            }
            kentos_commands::CAD_SOURCES_RECONCILE => {
                crate::gis_dispatch::gis_arm!($command, $then, sources_reconcile, SourcesReconcile)
            }
            kentos_commands::CAD_SOURCES_RELATE => {
                crate::gis_dispatch::gis_arm!($command, $then, sources_relate, SourcesRelate)
            }
            kentos_commands::CAD_RECORDS_PAGE => {
                crate::gis_dispatch::gis_arm!($command, $then, records_page, RecordsPage)
            }
            kentos_commands::CAD_SOURCES_QUERY => {
                crate::gis_dispatch::gis_arm!($command, $then, sources_query, SourcesQuery)
            }
            kentos_commands::CAD_FIELDS_WRITE => {
                crate::gis_dispatch::gis_arm!($command, $then, fields_write, FieldsWrite)
            }
            kentos_commands::CAD_PROCESSING_TOOLS => {
                crate::gis_dispatch::gis_arm!($command, $then, processing_tools, ProcessingTools)
            }
            kentos_commands::CAD_PROCESSING_RUN => {
                crate::gis_dispatch::gis_arm!($command, $then, processing_run, ProcessingRun)
            }
            kentos_commands::CAD_PROCESSING_BATCH => {
                crate::gis_dispatch::gis_arm!($command, $then, processing_batch, ProcessingBatch)
            }
            kentos_commands::CAD_IMAGES_CLIP => crate::gis_dispatch::gis_arm!(
                $command,
                $then,
                images_clip,
                kentos_commands::ImagesClip
            ),
            kentos_commands::CAD_TIME_VERSION => crate::gis_dispatch::gis_arm!(
                $command,
                $then,
                time_version,
                kentos_commands::TimeVersion
            ),
            kentos_commands::CAD_IMAGES_EDIT => {
                crate::gis_dispatch::gis_arm!($command, $then, images_edit, ImagesEdit)
            }
            kentos_commands::CAD_RASTERS_MOSAIC => {
                crate::gis_dispatch::gis_arm!($command, $then, rasters_mosaic, RastersMosaic)
            }
            kentos_commands::CAD_RASTERS_REPROJECT => {
                crate::gis_dispatch::gis_arm!($command, $then, rasters_reproject, RastersReproject)
            }
            kentos_commands::CAD_RASTERS_GPKGREAD => {
                crate::gis_dispatch::gis_arm!($command, $then, gpkg_tiles_read, RastersGpkgRead)
            }
            kentos_commands::CAD_RASTERS_GPKGWRITE => {
                crate::gis_dispatch::gis_arm!($command, $then, gpkg_tiles_write, RastersGpkgWrite)
            }
            kentos_commands::CAD_STAC_SEARCH => {
                crate::gis_dispatch::gis_arm!($command, $then, stac_search, StacSearch)
            }
            kentos_commands::CAD_STAC_PREPARE => {
                crate::gis_dispatch::gis_arm!($command, $then, stac_prepare, StacPrepare)
            }
            kentos_commands::CAD_MESHES_DESCRIBE => {
                crate::gis_dispatch::gis_arm!($command, $then, meshes_describe, MeshesDescribe)
            }
            kentos_commands::CAD_MESHES_QUERY => {
                crate::gis_dispatch::gis_arm!($command, $then, meshes_query, MeshesQuery)
            }
            kentos_commands::CAD_MESHES_REGION => {
                crate::gis_dispatch::gis_arm!($command, $then, meshes_region, MeshesRegion)
            }
            kentos_commands::CAD_MESHES_PROFILE => {
                crate::gis_dispatch::gis_arm!($command, $then, meshes_profile, MeshesProfile)
            }
            kentos_commands::CAD_TIME_QUERY => {
                crate::gis_dispatch::gis_arm!($command, $then, time_query, TimeQuery)
            }
            kentos_commands::CAD_SENSORS_INGEST => {
                crate::gis_dispatch::gis_arm!($command, $then, sensors_ingest, SensorsIngest)
            }
            kentos_commands::CAD_SENSORS_LATEST => {
                crate::gis_dispatch::gis_arm!($command, $then, sensors_latest, SensorsLatest)
            }
            kentos_commands::CAD_RASTERS_GEOREF => {
                crate::gis_dispatch::gis_arm!($command, $then, rasters_georef, RastersGeoref)
            }
            kentos_commands::CAD_FORMS_EDIT => {
                crate::gis_dispatch::gis_arm!($command, $then, forms_edit, FormsEdit)
            }
            kentos_commands::CAD_FORMS_VALIDATE => {
                crate::gis_dispatch::gis_arm!($command, $then, forms_validate, FormsValidate)
            }
            kentos_commands::CAD_FORMS_SUBMIT => {
                crate::gis_dispatch::gis_arm!($command, $then, forms_submit, FormsSubmit)
            }
            kentos_commands::CAD_FORMS_FILL => {
                crate::gis_dispatch::gis_arm!($command, $then, forms_fill, FormsFill)
            }
            kentos_commands::CAD_FORMS_AUDIT => {
                crate::gis_dispatch::gis_arm!($command, $then, forms_audit, FormsAudit)
            }
            kentos_commands::CAD_FORMS_TEST => {
                crate::gis_dispatch::gis_arm!($command, $then, forms_test, FormsTest)
            }
            kentos_commands::CAD_FORMS_COMPARE => {
                crate::gis_dispatch::gis_arm!($command, $then, forms_compare, FormsCompare)
            }
            kentos_commands::CAD_FORMS_SOURCES => {
                crate::gis_dispatch::gis_arm!($command, $then, forms_sources, FormsSources)
            }
            kentos_commands::CAD_FORMS_TRANSITION => {
                crate::gis_dispatch::gis_arm!($command, $then, forms_transition, FormsTransition)
            }
            kentos_commands::CAD_DOMAINS_EDIT => {
                crate::gis_dispatch::gis_arm!($command, $then, domains_edit, DomainsEdit)
            }
            kentos_commands::CAD_DOMAINS_EXPORT => {
                crate::gis_dispatch::gis_arm!($command, $then, domains_export, DomainsExport)
            }
            kentos_commands::CAD_FIELDS_MIGRATE => {
                crate::gis_dispatch::gis_arm!($command, $then, fields_migrate, FieldsMigrate)
            }
            kentos_commands::CAD_ATTACHMENTS_INSPECT => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    attachments_inspect,
                    AttachmentsInspect
                )
            }
            kentos_commands::CAD_SCHEMA_DESCRIBE => {
                crate::gis_dispatch::gis_arm!($command, $then, schema_describe, SchemaDescribe)
            }
            kentos_commands::CAD_FORMS_EXPORT => {
                crate::gis_dispatch::gis_arm!($command, $then, forms_export, FormsExport)
            }
            kentos_commands::CAD_FORMS_IMPORT => {
                crate::gis_dispatch::gis_arm!($command, $then, forms_import, FormsImport)
            }
            kentos_commands::CAD_FORMS_RECOMPUTE => {
                crate::gis_dispatch::gis_arm!($command, $then, forms_recompute, FormsRecompute)
            }
            kentos_commands::CAD_RECORDS_CREATE => {
                crate::gis_dispatch::gis_arm!($command, $then, records, RecordsCreate)
            }
            kentos_commands::CAD_RECORDS_DUPLICATE => {
                crate::gis_dispatch::gis_arm!($command, $then, records_duplicate, RecordsDuplicate)
            }
            kentos_commands::CAD_RELATIONS_EDIT => {
                crate::gis_dispatch::gis_arm!($command, $then, relations_edit, RelationsEdit)
            }
            kentos_commands::CAD_JOINS_EDIT => {
                crate::gis_dispatch::gis_arm!($command, $then, joins_edit, JoinsEdit)
            }
            kentos_commands::CAD_JOINS_QUERY => {
                crate::gis_dispatch::gis_arm!($command, $then, joins_query, JoinsQuery)
            }
            kentos_commands::CAD_DATASETS_EDIT => {
                crate::gis_dispatch::gis_arm!($command, $then, datasets_edit, DatasetsEdit)
            }
            kentos_commands::CAD_DATASETS_DESCRIBE => {
                crate::gis_dispatch::gis_arm!($command, $then, datasets_describe, DatasetsDescribe)
            }
            kentos_commands::CAD_ADDRESSES_CHECK => {
                crate::gis_dispatch::gis_arm!($command, $then, addresses_check, AddressesCheck)
            }
            kentos_commands::CAD_ADDRESSES_COMPARE => {
                crate::gis_dispatch::gis_arm!($command, $then, addresses_compare, AddressesCompare)
            }
            kentos_commands::CAD_ADDRESSES_NORMALIZE => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    addresses_normalize,
                    AddressesNormalize
                )
            }
            kentos_commands::CAD_TOPOLOGY_CHECK => {
                crate::gis_dispatch::gis_arm!($command, $then, topology_check, TopologyCheck)
            }
            kentos_commands::CAD_TOPOLOGY_EXCEPTIONS => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    topology_exceptions,
                    TopologyExceptions
                )
            }
            kentos_commands::CAD_TOPOLOGY_REPAIR => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    topology_repair,
                    kentos_commands::TopologyRepair
                )
            }
            kentos_commands::CAD_FIELDS_EDIT => {
                crate::gis_dispatch::gis_arm!($command, $then, fields_edit, FieldsEdit)
            }
            kentos_commands::CAD_ADDRESSES_DEFINE => {
                crate::gis_dispatch::gis_arm!($command, $then, address_define, AddressesDefine)
            }
            kentos_commands::CAD_LAYERS_TIME => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_time, LayersTime)
            }
            kentos_commands::CAD_SCENARIOS_EDIT => {
                crate::gis_dispatch::gis_arm!($command, $then, scenarios_edit, ScenariosEdit)
            }
            kentos_commands::CAD_LAYERS_FILTER => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_filter, LayersFilter)
            }
            kentos_commands::CAD_LAYERS_CRS => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    layers_crs_assign,
                    kentos_commands::LayersCrs
                )
            }
            kentos_commands::CAD_LAYERS_REPROJECT => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    layers_reproject,
                    kentos_commands::LayersReproject
                )
            }
            kentos_commands::CAD_LAYERS_COMPOSE => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_compose, LayersCompose)
            }
            kentos_commands::CAD_LAYERS_INFO => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_info, LayersInfo)
            }
            kentos_commands::CAD_LAYERS_TABLE => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_table, LayersTable)
            }
            kentos_commands::CAD_LAYERS_SUBTYPES => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_subtypes, LayersSubtypes)
            }
            kentos_commands::CAD_LAYERS_CONTINGENTS => {
                crate::gis_dispatch::gis_arm!(
                    $command,
                    $then,
                    layers_contingents,
                    LayersContingents
                )
            }
            kentos_commands::CAD_LAYERS_RULES => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_rules, LayersRules)
            }
            kentos_commands::CAD_LAYERS_INDEXES => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_indexes, LayersIndexes)
            }
            kentos_commands::CAD_LAYERS_LABELS => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_labels, LayersLabels)
            }
            kentos_commands::CAD_LAYERS_RENDERER => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_renderer, LayersRenderer)
            }
            kentos_commands::CAD_LAYERS_EDIT => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_edit, LayersEdit)
            }
            kentos_commands::CAD_LAYERS_SET => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_set, LayersSet)
            }
            kentos_commands::CAD_LAYERS_STATES => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_states, LayersStates)
            }
            kentos_commands::CAD_LAYERS_FIND => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_find, LayersFind)
            }
            kentos_commands::CAD_FIELDS_USAGE => {
                crate::gis_dispatch::gis_arm!($command, $then, fields_usage, FieldsUsage)
            }
            kentos_commands::CAD_LAYERS_TRANSLATE => {
                crate::gis_dispatch::gis_arm!($command, $then, layers_translate, LayersTranslate)
            }
            kentos_commands::FILE_IMPORT_SLD => {
                crate::gis_dispatch::gis_arm!($command, $then, sld, ImportSld)
            }
            kentos_commands::FILE_IMPORT_QML => {
                crate::gis_dispatch::gis_arm!($command, $then, qml, ImportQml)
            }
            kentos_commands::FILE_IMPORT_MAPLIBRE => {
                crate::gis_dispatch::gis_arm!($command, $then, maplibre, ImportMaplibre)
            }
            kentos_commands::CAD_GIS_DERIVATION => {
                crate::gis_dispatch::gis_arm!($command, $then, gis_derivation, GisDerivation)
            }
            kentos_commands::CAD_GIS_TAKEBACK => {
                crate::gis_dispatch::gis_arm!($command, $then, gis_takeback, GisTakeback)
            }
            kentos_commands::CAD_GIS_DETACH => {
                crate::gis_dispatch::gis_arm!($command, $then, gis_detach, GisDetach)
            }
            _ => $else,
        }
    };
}
pub(crate) use gis_commands;

/// One arm: the handler with `mod-gis` (docs/adr/8054 §11), else the answer that this build has no CBS commands.
/// The handler's module is never named in a build without it.
#[cfg(feature = "mod-gis")]
macro_rules! gis_arm {
    ($command:expr, $then:ident, $module:ident, $input:ty) => {
        $then!($module, $input)
    };
}
#[cfg(not(feature = "mod-gis"))]
macro_rules! gis_arm {
    ($command:expr, $then:ident, $module:ident, $input:ty) => {
        Err(crate::registry::not_built($command, "CBS"))
    };
}
pub(crate) use gis_arm;
