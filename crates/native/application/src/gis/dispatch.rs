//! The command registry's part of GIS commands (docs/adr/8054 §9): `gis_commands!` calls `$then!(module, InputType)`
//! for one of them, else gives `$else`. The registry (core/registry.rs) chains the parts; the handlers' names are
//! resolved where the registry calls it.

macro_rules! gis_commands {
    ($command:expr, $then:ident, $else:expr) => {
        match $command {
            kentos_commands::CAD_LAYERS_SERVICE => $then!(layers_service, LayersService),
            kentos_commands::CAD_NETWORK_DEFINE => $then!(network_define, NetworkDefine),
            kentos_commands::CAD_LINEARREF_DEFINE => {
                $then!(linearref_define, kentos_commands::LinearRefDefine)
            }
            kentos_commands::CAD_TOPOLOGY_CLEAN => {
                $then!(topology_clean, kentos_commands::TopologyClean)
            }
            kentos_commands::CAD_SCENARIOS_IMPACT => {
                $then!(scenarios_impact, kentos_commands::ScenariosImpact)
            }
            kentos_commands::CAD_SCENARIOS_RECORD => {
                $then!(scenarios_record, kentos_commands::ScenariosRecord)
            }
            kentos_commands::CAD_SUBNETWORKS_FIND => {
                $then!(subnetworks, kentos_commands::SubnetworksFind)
            }
            kentos_commands::CAD_SUBNETWORKS_UPDATE => {
                $then!(subnetworks_update, kentos_commands::SubnetworksUpdate)
            }
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
            kentos_commands::CAD_IMAGES_CLIP => $then!(images_clip, kentos_commands::ImagesClip),
            kentos_commands::CAD_TIME_VERSION => $then!(time_version, kentos_commands::TimeVersion),
            kentos_commands::CAD_IMAGES_EDIT => $then!(images_edit, ImagesEdit),
            kentos_commands::CAD_RASTERS_MOSAIC => $then!(rasters_mosaic, RastersMosaic),
            kentos_commands::CAD_RASTERS_REPROJECT => {
                $then!(rasters_reproject, RastersReproject)
            }
            kentos_commands::CAD_RASTERS_GPKGREAD => $then!(gpkg_tiles_read, RastersGpkgRead),
            kentos_commands::CAD_RASTERS_GPKGWRITE => $then!(gpkg_tiles_write, RastersGpkgWrite),
            kentos_commands::CAD_STAC_SEARCH => $then!(stac_search, StacSearch),
            kentos_commands::CAD_STAC_PREPARE => $then!(stac_prepare, StacPrepare),
            kentos_commands::CAD_MESHES_DESCRIBE => $then!(meshes_describe, MeshesDescribe),
            kentos_commands::CAD_MESHES_QUERY => $then!(meshes_query, MeshesQuery),
            kentos_commands::CAD_MESHES_REGION => $then!(meshes_region, MeshesRegion),
            kentos_commands::CAD_MESHES_PROFILE => $then!(meshes_profile, MeshesProfile),
            kentos_commands::CAD_TIME_QUERY => $then!(time_query, TimeQuery),
            kentos_commands::CAD_SENSORS_INGEST => $then!(sensors_ingest, SensorsIngest),
            kentos_commands::CAD_SENSORS_LATEST => $then!(sensors_latest, SensorsLatest),
            kentos_commands::CAD_RASTERS_GEOREF => $then!(rasters_georef, RastersGeoref),
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
            kentos_commands::CAD_FIELDS_EDIT => $then!(fields_edit, FieldsEdit),
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
            kentos_commands::CAD_LAYERS_RENDERER => $then!(layers_renderer, LayersRenderer),
            kentos_commands::CAD_LAYERS_EDIT => $then!(layers_edit, LayersEdit),
            kentos_commands::CAD_LAYERS_SET => $then!(layers_set, LayersSet),
            kentos_commands::CAD_LAYERS_STATES => $then!(layers_states, LayersStates),
            kentos_commands::CAD_LAYERS_FIND => $then!(layers_find, LayersFind),
            kentos_commands::CAD_FIELDS_USAGE => $then!(fields_usage, FieldsUsage),
            kentos_commands::CAD_LAYERS_TRANSLATE => $then!(layers_translate, LayersTranslate),
            kentos_commands::FILE_IMPORT_SLD => $then!(sld, ImportSld),
            kentos_commands::FILE_IMPORT_QML => $then!(qml, ImportQml),
            kentos_commands::FILE_IMPORT_MAPLIBRE => $then!(maplibre, ImportMaplibre),
            kentos_commands::CAD_GIS_DERIVATION => $then!(gis_derivation, GisDerivation),
            kentos_commands::CAD_GIS_TAKEBACK => $then!(gis_takeback, GisTakeback),
            kentos_commands::CAD_GIS_DETACH => $then!(gis_detach, GisDetach),
            _ => $else,
        }
    };
}
pub(crate) use gis_commands;
