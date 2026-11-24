//! The product commands the desktop runs, by name and version (docs/adr/0013): the table [`DESKTOP_COMMANDS`],
//! apart from the crate's index of modules (lib.rs) so that neither file grows past the source size limit
//! (CLAUDE.md §8). Each command has its module in the crate root; the comments name them.

use kentos_commands::{
    CAD_ARC_CREATE, CAD_ARC_CREATE_VERSION, CAD_CIRCLE_CREATE, CAD_CIRCLE_CREATE_VERSION,
    CAD_ENTITIES_ARRAY, CAD_ENTITIES_ARRAY_VERSION, CAD_ENTITIES_DELETE,
    CAD_ENTITIES_DELETE_VERSION, CAD_ENTITIES_EDIT, CAD_ENTITIES_EDIT_VERSION, CAD_ENTITIES_SET,
    CAD_ENTITIES_SET_VERSION, CAD_ENTITIES_TRANSFORM, CAD_ENTITIES_TRANSFORM_VERSION,
    CAD_LINE_CREATE, CAD_LINE_CREATE_VERSION, CAD_POINT_CREATE, CAD_POINT_CREATE_VERSION,
    CAD_POLYGON_CREATE, CAD_POLYGON_CREATE_VERSION, CAD_POLYLINE_CREATE,
    CAD_POLYLINE_CREATE_VERSION,
};

/// The product commands the desktop runs, by name and version (docs/adr/0013).
/// Each has its module in the crate ([`crate::polygon`], [`crate::line`], [`crate::polyline`], [`crate::delete`],
/// [`crate::point`], [`crate::circle`], [`crate::arc`], [`crate::transform`], [`crate::edit`], [`crate::array`],
/// [`crate::create`], [`crate::set`]); `tests/catalog.rs` keeps the
/// list equal to the catalog's commands marked `desktop`, as the server's
/// `SERVER_COMMANDS` is kept to those marked `server`.
pub const DESKTOP_COMMANDS: &[(&str, u32)] = &[
    (kentos_commands::CAD_DIMENSION_SETS_CREATE, kentos_commands::CAD_DIMENSION_SETS_VERSION),
    (kentos_commands::CAD_DIMENSION_SETS_EDIT, kentos_commands::CAD_DIMENSION_SETS_VERSION),
    (kentos_commands::CAD_DIMENSION_SETS_QUERY, kentos_commands::CAD_DIMENSION_SETS_VERSION),
    (CAD_POLYGON_CREATE, CAD_POLYGON_CREATE_VERSION),
    (CAD_LINE_CREATE, CAD_LINE_CREATE_VERSION),
    (CAD_POLYLINE_CREATE, CAD_POLYLINE_CREATE_VERSION),
    (CAD_ENTITIES_DELETE, CAD_ENTITIES_DELETE_VERSION),
    (CAD_POINT_CREATE, CAD_POINT_CREATE_VERSION),
    (CAD_CIRCLE_CREATE, CAD_CIRCLE_CREATE_VERSION),
    (CAD_ARC_CREATE, CAD_ARC_CREATE_VERSION),
    (CAD_ENTITIES_TRANSFORM, CAD_ENTITIES_TRANSFORM_VERSION),
    (CAD_ENTITIES_EDIT, CAD_ENTITIES_EDIT_VERSION),
    (CAD_ENTITIES_ARRAY, CAD_ENTITIES_ARRAY_VERSION),
    // The drawing's user coordinate system ([`ucs_set`], docs/adr/0261).
    (
        kentos_commands::CAD_UCS_SET,
        kentos_commands::CAD_UCS_SET_VERSION,
    ),
    // The project's named views ([`views_edit`], docs/adr/0285).
    (
        kentos_commands::CAD_VIEWS_EDIT,
        kentos_commands::CAD_VIEWS_EDIT_VERSION,
    ),
    // The project's forms ([`forms_edit`], [`forms_validate`], [`forms_submit`], docs/adr/0401).
    (
        kentos_commands::CAD_FORMS_EDIT,
        kentos_commands::CAD_FORMS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_VALIDATE,
        kentos_commands::CAD_FORMS_VALIDATE_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_AUDIT,
        kentos_commands::CAD_FORMS_AUDIT_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_TEST,
        kentos_commands::CAD_FORMS_TEST_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_COMPARE,
        kentos_commands::CAD_FORMS_COMPARE_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_SOURCES,
        kentos_commands::CAD_FORMS_SOURCES_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_TRANSITION,
        kentos_commands::CAD_FORMS_TRANSITION_VERSION,
    ),
    (
        kentos_commands::CAD_DOMAINS_EDIT,
        kentos_commands::CAD_DOMAINS_EDIT_VERSION,
    ),
    // The value lists as a package and a layer's values in their canonical texts (docs/adr/0439 §7, §8).
    (
        kentos_commands::CAD_DOMAINS_EXPORT,
        kentos_commands::CAD_DOMAINS_EXPORT_VERSION,
    ),
    (
        kentos_commands::CAD_FIELDS_MIGRATE,
        kentos_commands::CAD_FIELDS_MIGRATE_VERSION,
    ),
    // What each attachment is, read from its bytes ([`attachments_inspect`], docs/adr/0442 §7).
    (
        kentos_commands::CAD_ATTACHMENTS_INSPECT,
        kentos_commands::CAD_ATTACHMENTS_INSPECT_VERSION,
    ),
    // The data schema a page at a time ([`schema_describe`], docs/adr/0443 §1).
    (
        kentos_commands::CAD_SCHEMA_DESCRIBE,
        kentos_commands::CAD_SCHEMA_DESCRIBE_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_EXPORT,
        kentos_commands::CAD_FORMS_EXPORT_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_IMPORT,
        kentos_commands::CAD_FORMS_IMPORT_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_RECOMPUTE,
        kentos_commands::CAD_FORMS_RECOMPUTE_VERSION,
    ),
    // Koşullu biçim ([`layers_table`], docs/adr/0413).
    (
        kentos_commands::CAD_LAYERS_TABLE,
        kentos_commands::CAD_LAYERS_TABLE_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_SUBTYPES,
        kentos_commands::CAD_LAYERS_SUBTYPES_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_CONTINGENTS,
        kentos_commands::CAD_LAYERS_CONTINGENTS_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_RULES,
        kentos_commands::CAD_LAYERS_RULES_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_INDEXES,
        kentos_commands::CAD_LAYERS_INDEXES_VERSION,
    ),
    // Records without a geometry ([`records`], docs/adr/0410).
    (
        kentos_commands::CAD_RECORDS_CREATE,
        kentos_commands::CAD_RECORDS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_RECORDS_DUPLICATE,
        kentos_commands::CAD_RECORDS_DUPLICATE_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_SUBMIT,
        kentos_commands::CAD_FORMS_SUBMIT_VERSION,
    ),
    (
        kentos_commands::CAD_FORMS_FILL,
        kentos_commands::CAD_FORMS_FILL_VERSION,
    ),
    // The project's relations ([`relations_edit`], docs/adr/0405).
    (
        kentos_commands::CAD_RELATIONS_EDIT,
        kentos_commands::CAD_RELATIONS_EDIT_VERSION,
    ),
    // The project's join views kept and read ([`joins_edit`], [`joins_query`], docs/adr/0527).
    (
        kentos_commands::CAD_JOINS_EDIT,
        kentos_commands::CAD_JOINS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_JOINS_QUERY,
        kentos_commands::CAD_JOINS_QUERY_VERSION,
    ),
    // A layer as a dataset ([`datasets_edit`], [`datasets_describe`], docs/adr/0633).
    (
        kentos_commands::CAD_DATASETS_EDIT,
        kentos_commands::CAD_DATASETS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_DATASETS_DESCRIBE,
        kentos_commands::CAD_DATASETS_DESCRIBE_VERSION,
    ),
    // Taramanın sınırı, adaları ve ilişkisi ([`hatches_edit`], docs/adr/0329 §4).
    (
        kentos_commands::CAD_HATCHES_EDIT,
        kentos_commands::CAD_HATCHES_EDIT_VERSION,
    ),
    // Çok kollu kılavuz ([`leaders_edit`], docs/adr/0326 §5).
    (
        kentos_commands::CAD_LEADERS_EDIT,
        kentos_commands::CAD_LEADERS_EDIT_VERSION,
    ),
    // Tablonun hücreleri, formülleri, alanları ve dosyası ([`tables_edit`], docs/adr/0327).
    (
        kentos_commands::CAD_TABLES_EDIT,
        kentos_commands::CAD_TABLES_EDIT_VERSION,
    ),
    // Alanlar and the text commands ([`fields_edit`], [`texts_edit`], [`texts_check`], docs/adr/0325 §3).
    (
        kentos_commands::CAD_FIELDS_EDIT,
        kentos_commands::CAD_FIELDS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_TEXTS_EDIT,
        kentos_commands::CAD_TEXTS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_TEXTS_CHECK,
        kentos_commands::CAD_TEXTS_CHECK_VERSION,
    ),
    // Yazı tipleri ([`fonts_report`], docs/adr/0324 §5).
    (
        kentos_commands::CAD_FONTS_REPORT,
        kentos_commands::CAD_FONTS_REPORT_VERSION,
    ),
    // Ölçekli açıklamalar ([`annotations_scales`], docs/adr/0323 §4).
    (
        kentos_commands::CAD_ANNOTATIONS_SCALES,
        kentos_commands::CAD_ANNOTATIONS_SCALES_VERSION,
    ),
    // 2B düzleme indir ([`entities_flatten`], docs/adr/1061 §4).
    (
        kentos_commands::CAD_ENTITIES_FLATTEN,
        kentos_commands::CAD_ENTITIES_FLATTEN_VERSION,
    ),
    // Öne getir and Arkaya gönder ([`entities_order`], docs/adr/0260 §5).
    (
        kentos_commands::CAD_ENTITIES_ORDER,
        kentos_commands::CAD_ENTITIES_ORDER_VERSION,
    ),
    // A NURBS curve by control points or through points ([`nurbs`], docs/adr/0259).
    (
        kentos_commands::CAD_NURBS_CREATE,
        kentos_commands::CAD_NURBS_CREATE_VERSION,
    ),
    // Aks, Kolon, Merdiven, Rampa and Mimari nesneyi düzenle ([`arch_elements`], [`arch_edit`], docs/adr/0343).
    (
        kentos_commands::CAD_GRIDS_CREATE,
        kentos_commands::CAD_GRIDS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_COLUMNS_CREATE,
        kentos_commands::CAD_COLUMNS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_STAIRS_CREATE,
        kentos_commands::CAD_STAIRS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_RAMPS_CREATE,
        kentos_commands::CAD_RAMPS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_BEAMS_CREATE,
        kentos_commands::CAD_BEAMS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_ROOFS_CREATE,
        kentos_commands::CAD_ROOFS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_RAILINGS_CREATE,
        kentos_commands::CAD_RAILINGS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_ARCH_EDIT,
        kentos_commands::CAD_ARCH_EDIT_VERSION,
    ),
    // Etiket, Etiketleri denetle, Çizelge and Çizelgeyi düzenle ([`tags`], [`schedules`], docs/adr/0344).
    (
        kentos_commands::CAD_TAGS_CREATE,
        kentos_commands::CAD_TAGS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_TAGS_CHECK,
        kentos_commands::CAD_TAGS_CHECK_VERSION,
    ),
    (
        kentos_commands::CAD_TAGS_EDIT,
        kentos_commands::CAD_TAGS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_SCHEDULES_CREATE,
        kentos_commands::CAD_SCHEDULES_CREATE_VERSION,
    ),
    (kentos_commands::CAD_EXTRACT_RUN, kentos_commands::CAD_EXTRACT_RUN_VERSION),
    (kentos_commands::CAD_EXTRACT_TABLE, kentos_commands::CAD_EXTRACT_TABLE_VERSION),
    (
        kentos_commands::CAD_SCHEDULES_EDIT,
        kentos_commands::CAD_SCHEDULES_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_SCHEDULES_QUERY,
        kentos_commands::CAD_SCHEDULES_QUERY_VERSION,
    ),
    // Görünüm işareti, Görünüm işaretini düzenle, Başvurular and Döşeme ve tavan ([`viewmarks`], [`spaces_finish`],
    // docs/adr/0345).
    (
        kentos_commands::CAD_VIEWMARKS_CREATE,
        kentos_commands::CAD_VIEWMARKS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_VIEWMARKS_EDIT,
        kentos_commands::CAD_VIEWMARKS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_VIEWMARKS_REFERENCES,
        kentos_commands::CAD_VIEWMARKS_REFERENCES_VERSION,
    ),
    (
        kentos_commands::CAD_SPACES_FINISH,
        kentos_commands::CAD_SPACES_FINISH_VERSION,
    ),
    // Mimari şablonu ([`templates_apply`], docs/adr/0346).
    (
        kentos_commands::CAD_TEMPLATES_APPLY,
        kentos_commands::CAD_TEMPLATES_APPLY_VERSION,
    ),
    // Pafta seti ([`sheets_set`], docs/adr/0347).
    (
        kentos_commands::CAD_SHEETS_SET,
        kentos_commands::CAD_SHEETS_SET_VERSION,
    ),
    // Proje gezgini ([`navigator`], docs/adr/0778).
    (
        kentos_commands::CAD_NAVIGATOR_READ,
        kentos_commands::CAD_NAVIGATOR_READ_VERSION,
    ),
    // Dış başvurular ([`xrefs`], docs/adr/0348).
    (
        kentos_commands::CAD_XREFS_EDIT,
        kentos_commands::CAD_XREFS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_XREFS_LIST,
        kentos_commands::CAD_XREFS_LIST_VERSION,
    ),
    // Kırp ([`inserts_clip`], docs/adr/0351 §1).
    (
        kentos_commands::CAD_INSERTS_CLIP,
        kentos_commands::CAD_INSERTS_CLIP_VERSION,
    ),
    // Bağlı dosyaların durumu ([`assets_check`], docs/adr/0349 §2).
    (
        kentos_commands::CAD_ASSETS_CHECK,
        kentos_commands::CAD_ASSETS_CHECK_VERSION,
    ),
    // Resmin kaynağı ([`images_edit`], docs/adr/0350 §1).
    (
        kentos_commands::CAD_IMAGES_EDIT,
        kentos_commands::CAD_IMAGES_EDIT_VERSION,
    ),
    // STAC keşfi ve veri hazırlama ([`stac_search`], [`stac_prepare`], docs/adr/0528).
    (
        kentos_commands::CAD_STAC_SEARCH,
        kentos_commands::CAD_STAC_SEARCH_VERSION,
    ),
    (
        kentos_commands::CAD_STAC_PREPARE,
        kentos_commands::CAD_STAC_PREPARE_VERSION,
    ),
    // Zamanda sorgu ([`time_query`], docs/adr/0537).
    (
        kentos_commands::CAD_TIME_QUERY,
        kentos_commands::CAD_TIME_QUERY_VERSION,
    ),
    // Sensör gözlemleri ([`sensors_ingest`], [`sensors_latest`], docs/adr/0646).
    (
        kentos_commands::CAD_SENSORS_INGEST,
        kentos_commands::CAD_SENSORS_INGEST_VERSION,
    ),
    (
        kentos_commands::CAD_SENSORS_LATEST,
        kentos_commands::CAD_SENSORS_LATEST_VERSION,
    ),
    // Nokta bulutunun bilgisi ve XYZ sor ([`pointclouds`], docs/adr/0536).
    (
        kentos_commands::CAD_POINTCLOUDS_DESCRIBE,
        kentos_commands::CAD_POINTCLOUDS_DESCRIBE_VERSION,
    ),
    (
        kentos_commands::CAD_POINTCLOUDS_QUERY,
        kentos_commands::CAD_POINTCLOUDS_QUERY_VERSION,
    ),
    // Nokta bulutunun alan ve profil sorgusu, meshin bilgisi ve sorgusu ([`pointcloud_areas`], [`meshes`], docs/adr/0552).
    (
        kentos_commands::CAD_POINTCLOUDS_REGION,
        kentos_commands::CAD_POINTCLOUDS_REGION_VERSION,
    ),
    (
        kentos_commands::CAD_POINTCLOUDS_PROFILE,
        kentos_commands::CAD_POINTCLOUDS_PROFILE_VERSION,
    ),
    (
        kentos_commands::CAD_MESHES_DESCRIBE,
        kentos_commands::CAD_MESHES_DESCRIBE_VERSION,
    ),
    (
        kentos_commands::CAD_MESHES_QUERY,
        kentos_commands::CAD_MESHES_QUERY_VERSION,
    ),
    // Meshin alanı ve hattı ([`mesh_areas`], docs/adr/0645 §7).
    (
        kentos_commands::CAD_MESHES_REGION,
        kentos_commands::CAD_MESHES_REGION_VERSION,
    ),
    (
        kentos_commands::CAD_MESHES_PROFILE,
        kentos_commands::CAD_MESHES_PROFILE_VERSION,
    ),
    // Sanal mozaik ([`rasters_mosaic`], docs/adr/0517).
    (
        kentos_commands::CAD_RASTERS_MOSAIC,
        kentos_commands::CAD_RASTERS_MOSAIC_VERSION,
    ),
    // Rasteri projenin sistemine çevir ([`rasters_reproject`], docs/adr/0644 §3).
    (
        kentos_commands::CAD_RASTERS_REPROJECT,
        kentos_commands::CAD_RASTERS_REPROJECT_VERSION,
    ),
    // GeoPackage karoları ([`gpkg_tiles`], docs/adr/0556).
    (
        kentos_commands::CAD_RASTERS_GPKGREAD,
        kentos_commands::CAD_RASTERS_GPKGREAD_VERSION,
    ),
    (
        kentos_commands::CAD_RASTERS_GPKGWRITE,
        kentos_commands::CAD_RASTERS_GPKGWRITE_VERSION,
    ),
    // Raster oturt'un kaydı ([`rasters_georef`], docs/adr/0518).
    (
        kentos_commands::CAD_RASTERS_GEOREF,
        kentos_commands::CAD_RASTERS_GEOREF_VERSION,
    ),
    // Mahal, Mahali düzenle and Metraj ([`spaces_create`], [`spaces_edit`], [`spaces_quantities`], docs/adr/0342).
    (kentos_commands::CAD_SPACES_DISCOVER, kentos_commands::CAD_SPACES_DISCOVER_VERSION),
    (kentos_commands::CAD_SPACES_NUMBER,kentos_commands::CAD_SPACES_NUMBER_VERSION),
    (kentos_commands::CAD_SPACES_SEPARATORS,kentos_commands::CAD_SPACES_SEPARATORS_VERSION),
    (
        kentos_commands::CAD_SPACES_CREATE,
        kentos_commands::CAD_SPACES_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_SPACES_EDIT,
        kentos_commands::CAD_SPACES_EDIT_VERSION,
    ),
    // Bina programı ve denetimi ([`program_edit`], [`program_check`], docs/adr/0773).
    (
        kentos_commands::CAD_PROGRAM_EDIT,
        kentos_commands::CAD_PROGRAM_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_PROGRAM_CHECK,
        kentos_commands::CAD_PROGRAM_CHECK_VERSION,
    ),
    (
        kentos_commands::CAD_SPACES_QUANTITIES,
        kentos_commands::CAD_SPACES_QUANTITIES_VERSION,
    ),
    // Döşeme and Döşemeyi düzenle ([`slabs_create`], [`slabs_edit`], docs/adr/0722).
    (kentos_commands::CAD_SLABS_CREATE, kentos_commands::CAD_SLABS_CREATE_VERSION),
    (kentos_commands::CAD_SLABS_EDIT, kentos_commands::CAD_SLABS_EDIT_VERSION),
    // Kapı, Pencere, Boşluk and Açıklığı düzenle ([`openings_create`], [`openings_edit`], docs/adr/0341).
    (
        kentos_commands::CAD_OPENINGS_CREATE,
        kentos_commands::CAD_OPENINGS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_OPENINGS_EDIT,
        kentos_commands::CAD_OPENINGS_EDIT_VERSION,
    ),
    // Kapı ve pencere tipleri ([`opening_types_edit`], [`openings_query`], docs/adr/0772).
    (
        kentos_commands::CAD_OPENING_TYPES_EDIT,
        kentos_commands::CAD_OPENING_TYPES_EDIT_VERSION,
    ),
    // Eleman tipleri ([`element_types_edit`], docs/adr/8106).
    (
        kentos_commands::CAD_ELEMENT_TYPES_EDIT,
        kentos_commands::CAD_ELEMENT_TYPES_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_OPENINGS_QUERY,
        kentos_commands::CAD_OPENINGS_QUERY_VERSION,
    ),
    // Malzemeler ([`materials_edit`], docs/adr/0785).
    (kentos_commands::CAD_MATERIALS_EDIT, kentos_commands::CAD_MATERIALS_EDIT_VERSION),
    // Oda modülleri ([`modules_edit`], [`modules_query`], docs/adr/0775).
    (kentos_commands::CAD_MODULES_EDIT, kentos_commands::CAD_MODULES_EDIT_VERSION),
    (kentos_commands::CAD_MODULES_QUERY, kentos_commands::CAD_MODULES_QUERY_VERSION),
    // Tasarım seçenekleri ([`options_edit`], [`options_query`], docs/adr/0788).
    (kentos_commands::CAD_OPTIONS_EDIT, kentos_commands::CAD_OPTIONS_EDIT_VERSION),
    (kentos_commands::CAD_OPTIONS_QUERY, kentos_commands::CAD_OPTIONS_QUERY_VERSION),
    // Oda iç görünüşü ([`elevations_edit`], [`elevations_query`], docs/adr/0799).
    (
        kentos_commands::CAD_ELEVATIONS_EDIT,
        kentos_commands::CAD_ELEVATIONS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_ELEVATIONS_QUERY,
        kentos_commands::CAD_ELEVATIONS_QUERY_VERSION,
    ),
    // Kesit çıkar ([`sections_edit`], [`sections_query`], docs/adr/8042).
    (
        kentos_commands::CAD_SECTIONS_EDIT,
        kentos_commands::CAD_SECTIONS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_SECTIONS_QUERY,
        kentos_commands::CAD_SECTIONS_QUERY_VERSION,
    ),
    // Açıklığın geçici ölçüleri ([`opening_distances`], docs/adr/0701).
    (
        kentos_commands::CAD_OPENINGS_DISTANCES,
        kentos_commands::CAD_OPENINGS_DISTANCES_VERSION,
    ),
    // Duvarın geçici ölçüleri ([`wall_distances`], docs/adr/0710).
    (
        kentos_commands::CAD_WALLS_DISTANCES,
        kentos_commands::CAD_WALLS_DISTANCES_VERSION,
    ),
    // Kolonun geçici ölçüleri ([`column_distances`], docs/adr/0712).
    (
        kentos_commands::CAD_COLUMNS_DISTANCES,
        kentos_commands::CAD_COLUMNS_DISTANCES_VERSION,
    ),
    // Mimari denetim ([`arch_audit`], docs/adr/0703).
    (
        kentos_commands::CAD_ARCH_AUDIT,
        kentos_commands::CAD_ARCH_AUDIT_VERSION,
    ),
    (kentos_commands::CAD_ARCH_WALK, kentos_commands::CAD_ARCH_WALK_VERSION),
    // Denetim istisnası ([`arch_exceptions`], docs/adr/0732 §3).
    (
        kentos_commands::CAD_ARCH_EXCEPTIONS,
        kentos_commands::CAD_ARCH_EXCEPTIONS_VERSION,
    ),
    // Mutfak dizisi ([`casework_run`], docs/adr/0706, 0727).
    (
        kentos_commands::CAD_CASEWORK_RUN,
        kentos_commands::CAD_CASEWORK_RUN_VERSION,
    ),
    // Mutfak dizisini düzenle ve oku ([`casework_edit`], docs/adr/0727 §6).
    (
        kentos_commands::CAD_CASEWORK_EDIT,
        kentos_commands::CAD_CASEWORK_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_CASEWORK_QUERY,
        kentos_commands::CAD_CASEWORK_QUERY_VERSION,
    ),
    // Kat ver ([`levels_assign`], docs/adr/0717).
    (
        kentos_commands::CAD_LEVELS_ASSIGN,
        kentos_commands::CAD_LEVELS_ASSIGN_VERSION,
    ),
    // Yangın ve performans ([`arch_performance`], docs/adr/8068).
    (
        kentos_commands::CAD_ARCH_PERFORMANCE,
        kentos_commands::CAD_ARCH_PERFORMANCE_VERSION,
    ),
    // Kontrol formunun cevapları ([`arch_form_answers`], docs/adr/8077).
    (
        kentos_commands::CAD_FORM_ANSWER,
        kentos_commands::CAD_FORM_ANSWER_VERSION,
    ),
    // Teslim kaydı ([`delivery_edit`], docs/adr/8099).
    (
        kentos_commands::CAD_DELIVERY_EDIT,
        kentos_commands::CAD_DELIVERY_EDIT_VERSION,
    ),
    // Altlık model ([`underlay_edit`], docs/adr/8101).
    (
        kentos_commands::CAD_UNDERLAY_EDIT,
        kentos_commands::CAD_UNDERLAY_EDIT_VERSION,
    ),
    // Altlık modeli denetle ([`underlay_check`], docs/adr/8102).
    (
        kentos_commands::CAD_UNDERLAY_CHECK,
        kentos_commands::CAD_UNDERLAY_CHECK_VERSION,
    ),
    // Parselin plan kararları ([`zoning_parcel`], docs/adr/8072 §5).
    (
        kentos_commands::CAD_ZONING_PARCEL,
        kentos_commands::CAD_ZONING_PARCEL_VERSION,
    ),
    // Kuzey oku, Revizyon işaretle and Malzeme lejandı ([`north`], [`revisions`], [`legends`], docs/adr/0721).
    (
        kentos_commands::CAD_NORTH_CREATE,
        kentos_commands::CAD_NORTH_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_REVISIONS_MARK,
        kentos_commands::CAD_REVISIONS_MARK_VERSION,
    ),
    (
        kentos_commands::CAD_LEGENDS_MATERIALS,
        kentos_commands::CAD_LEGENDS_MATERIALS_VERSION,
    ),
    // Duvar, Çizgiden duvar, Duvarı düzenle and Duvar birleşimleri ([`walls_create`], [`walls_from_lines`], [`walls_edit`],
    // [`walls_joins`], docs/adr/0339, 0340).
    (
        kentos_commands::CAD_WALLS_CREATE,
        kentos_commands::CAD_WALLS_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_WALLS_RECTANGLE,
        kentos_commands::CAD_WALLS_RECTANGLE_VERSION,
    ),
    (kentos_commands::CAD_WALLS_PARTITION, kentos_commands::CAD_WALLS_PARTITION_VERSION),
    (
        kentos_commands::CAD_WALLS_FROM_LINES,
        kentos_commands::CAD_WALLS_FROM_LINES_VERSION,
    ),
    (
        kentos_commands::CAD_ARCH_RECOGNIZE,
        kentos_commands::CAD_ARCH_RECOGNIZE_VERSION,
    ),
    (
        kentos_commands::CAD_WALLS_EDIT,
        kentos_commands::CAD_WALLS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_WALLS_JOINS,
        kentos_commands::CAD_WALLS_JOINS_VERSION,
    ),
    (kentos_commands::CAD_DONUT_CREATE, kentos_commands::CAD_DONUT_CREATE_VERSION),
    // Rectangles and regular polygons ([`shapes`], docs/adr/0290).
    (
        kentos_commands::CAD_RECTANGLE_CREATE,
        kentos_commands::CAD_RECTANGLE_CREATE_VERSION,
    ),
    (
        kentos_commands::CAD_POLYGON_REGULAR,
        kentos_commands::CAD_POLYGON_REGULAR_VERSION,
    ),
    (kentos_commands::CAD_SECTOR_CREATE, kentos_commands::CAD_SECTOR_CREATE_VERSION),
    (kentos_commands::CAD_REVCLOUD_CREATE, kentos_commands::CAD_REVCLOUD_CREATE_VERSION),
    (kentos_commands::CAD_ENTITIES_ADJOIN, kentos_commands::CAD_ENTITIES_ADJOIN_VERSION),
    (kentos_commands::CAD_PARCELS_CREATE, kentos_commands::CAD_PARCELS_CREATE_VERSION),
    (kentos_commands::CAD_CONSTRUCTIONS_CREATE, kentos_commands::CAD_CONSTRUCTIONS_CREATE_VERSION),
    (kentos_commands::CAD_AREAS_EDIT, kentos_commands::CAD_AREAS_EDIT_VERSION),
    (kentos_commands::CAD_ANNOTATIONS_WRITE, kentos_commands::CAD_ANNOTATIONS_WRITE_VERSION),
    (kentos_commands::CAD_OBJECTS_DERIVE, kentos_commands::CAD_OBJECTS_DERIVE_VERSION),
    (kentos_commands::CAD_ROADS_EDIT, kentos_commands::CAD_ROADS_EDIT_VERSION),
    (kentos_commands::CAD_IMAGES_CLIP, kentos_commands::CAD_IMAGES_CLIP_VERSION),
    (kentos_commands::CAD_TIME_VERSION, kentos_commands::CAD_TIME_VERSION_VERSION),
    (kentos_commands::CAD_SURVEY_POINTS, kentos_commands::CAD_SURVEY_POINTS_VERSION),
    (kentos_commands::CAD_SURVEY_ADJUST, kentos_commands::CAD_SURVEY_ADJUST_VERSION),
    (kentos_commands::CAD_SHAPES_EDIT, kentos_commands::CAD_SHAPES_EDIT_VERSION),
    // A path's vertices and edges ([`entities_vertex`], docs/adr/0279).
    (
        kentos_commands::CAD_ENTITIES_VERTEX,
        kentos_commands::CAD_ENTITIES_VERTEX_VERSION,
    ),
    // What a click can mean and the sub-object it falls on ([`entities_pick`], docs/adr/0276).
    (
        kentos_commands::CAD_ENTITIES_PICK,
        kentos_commands::CAD_ENTITIES_PICK_VERSION,
    ),
    // The standard DE-9IM relations ([`entities_relate`], docs/adr/0501).
    (
        kentos_commands::CAD_ENTITIES_RELATE,
        kentos_commands::CAD_ENTITIES_RELATE_VERSION,
    ),
    // Points between coordinate systems with their heights, and how ([`crs_transform`], docs/adr/0519).
    (
        kentos_commands::CAD_CRS_TRANSFORM,
        kentos_commands::CAD_CRS_TRANSFORM_VERSION,
    ),
    // The selection as one explicit query ([`entities_select`], docs/adr/0275).
    (
        kentos_commands::CAD_ENTITIES_ISOLATE,
        kentos_commands::CAD_ENTITIES_ISOLATE_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_SELECT,
        kentos_commands::CAD_ENTITIES_SELECT_VERSION,
    ),
    // The object snap at a point, with its object's place ([`snap_find`], docs/adr/0272).
    (
        kentos_commands::CAD_SNAP_FIND,
        kentos_commands::CAD_SNAP_FIND_VERSION,
    ),
    // The drafting aids' points by what they are made of ([`points_construct`], docs/adr/0270).
    (
        kentos_commands::CAD_POINTS_CONSTRUCT,
        kentos_commands::CAD_POINTS_CONSTRUCT_VERSION,
    ),
    // Daire's, Yay's and Elips's methods by what they are made of ([`curves_construct`], docs/adr/0289).
    (
        kentos_commands::CAD_CURVES_CONSTRUCT,
        kentos_commands::CAD_CURVES_CONSTRUCT_VERSION,
    ),
    // Ötele's semantic command ([`offset`], docs/adr/0246).
    (
        kentos_commands::CAD_ENTITIES_OFFSET,
        kentos_commands::CAD_ENTITIES_OFFSET_VERSION,
    ),
    // The drawing tools' objects without a command of their own ([`create`], docs/adr/0057).
    (
        kentos_commands::CAD_ENTITIES_CREATE,
        kentos_commands::CAD_ENTITIES_CREATE_VERSION,
    ),
    // Öznitelikler's layer, colour, symbol, attributes and label ([`set`], docs/adr/0066).
    (CAD_ENTITIES_SET, CAD_ENTITIES_SET_VERSION),
    // Blok oluştur and the Bloklar panel ([`blocks_define`], [`blocks_edit`], docs/adr/0144).
    (
        kentos_commands::CAD_BLOCKS_DEFINE,
        kentos_commands::CAD_BLOCKS_DEFINE_VERSION,
    ),
    (
        kentos_commands::CAD_BLOCKS_EDIT,
        kentos_commands::CAD_BLOCKS_EDIT_VERSION,
    ),
    // Harita servisi and the service layers' menus ([`layers_service`], docs/adr/0208 §15).
    (
        kentos_commands::CAD_LAYERS_SERVICE,
        kentos_commands::CAD_LAYERS_SERVICE_VERSION,
    ),
    // Ağlar ([`network_define`], docs/adr/0209 §11).
    (
        kentos_commands::CAD_NETWORK_DEFINE,
        kentos_commands::CAD_NETWORK_DEFINE_VERSION,
    ),
    // Olay bağı ([`linearref_define`], docs/adr/0569 §2).
    (
        kentos_commands::CAD_LINEARREF_DEFINE,
        kentos_commands::CAD_LINEARREF_DEFINE_VERSION,
    ),
    // KentOS Yol's terrain surfaces ([`road_terrains_create`], [`road_terrains_edit`], [`road_terrains_query`],
    // plan/road.md Y2.3).
    (
        kentos_commands::ROAD_TERRAINS_CREATE,
        kentos_commands::ROAD_TERRAINS_CREATE_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_EDIT,
        kentos_commands::ROAD_TERRAINS_EDIT_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_QUERY,
        kentos_commands::ROAD_TERRAINS_QUERY_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_CONTOURS,
        kentos_commands::ROAD_TERRAINS_CONTOURS_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_SHEET_CONTOURS,
        kentos_commands::ROAD_TERRAINS_SHEET_CONTOURS_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_SECTIONCHECK,
        kentos_commands::ROAD_TERRAINS_SECTIONCHECK_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_ZEROLINE,
        kentos_commands::ROAD_TERRAINS_ZEROLINE_VERSION,
    ),
    (
        kentos_commands::ROAD_TERRAINS_CUTFILL,
        kentos_commands::ROAD_TERRAINS_CUTFILL_VERSION,
    ),
    (
        kentos_commands::FILE_IMPORT_LANDXML,
        kentos_commands::FILE_IMPORT_LANDXML_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_CREATE,
        kentos_commands::ROAD_ALIGNMENTS_CREATE_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_EDIT,
        kentos_commands::ROAD_ALIGNMENTS_EDIT_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_LABEL,
        kentos_commands::ROAD_ALIGNMENTS_LABEL_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_QUERY,
        kentos_commands::ROAD_ALIGNMENTS_QUERY_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_CREATE,
        kentos_commands::ROAD_PROFILES_CREATE_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_EDIT,
        kentos_commands::ROAD_PROFILES_EDIT_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_DRAW,
        kentos_commands::ROAD_PROFILES_DRAW_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_SIGHT,
        kentos_commands::ROAD_PROFILES_SIGHT_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_PROPOSE,
        kentos_commands::ROAD_PROFILES_PROPOSE_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_CHECK,
        kentos_commands::ROAD_ALIGNMENTS_CHECK_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_CHECK,
        kentos_commands::ROAD_PROFILES_CHECK_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_LENGTHS,
        kentos_commands::ROAD_PROFILES_LENGTHS_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_QUERY,
        kentos_commands::ROAD_PROFILES_QUERY_VERSION,
    ),
    (
        kentos_commands::ROAD_TEMPLATES_EDIT,
        kentos_commands::ROAD_TEMPLATES_EDIT_VERSION,
    ),
    (
        kentos_commands::ROAD_SECTIONS_EDIT,
        kentos_commands::ROAD_SECTIONS_EDIT_VERSION,
    ),
    (
        kentos_commands::ROAD_SECTIONS_QUERY,
        kentos_commands::ROAD_SECTIONS_QUERY_VERSION,
    ),
    (
        kentos_commands::ROAD_SECTIONS_DRAW,
        kentos_commands::ROAD_SECTIONS_DRAW_VERSION,
    ),
    (
        kentos_commands::ROAD_SUPERELEVATIONS_EDIT,
        kentos_commands::ROAD_SUPERELEVATIONS_EDIT_VERSION,
    ),
    (
        kentos_commands::ROAD_SUPERELEVATIONS_QUERY,
        kentos_commands::ROAD_SUPERELEVATIONS_QUERY_VERSION,
    ),
    (
        kentos_commands::ROAD_SUPERELEVATIONS_CHECK,
        kentos_commands::ROAD_SUPERELEVATIONS_CHECK_VERSION,
    ),
    (
        kentos_commands::ROAD_SUPERELEVATIONS_WIDEN,
        kentos_commands::ROAD_SUPERELEVATIONS_WIDEN_VERSION,
    ),
    (
        kentos_commands::ROAD_SUPERELEVATIONS_TRANSITIONS,
        kentos_commands::ROAD_SUPERELEVATIONS_TRANSITIONS_VERSION,
    ),
    (kentos_commands::ROAD_CORRIDORS_EDIT, kentos_commands::ROAD_CORRIDORS_EDIT_VERSION),
    (kentos_commands::ROAD_CORRIDORS_QUERY, kentos_commands::ROAD_CORRIDORS_QUERY_VERSION),
    (kentos_commands::ROAD_CORRIDORS_WATER, kentos_commands::ROAD_CORRIDORS_WATER_VERSION),
    (kentos_commands::ROAD_CORRIDORS_DAYLIGHTS, kentos_commands::ROAD_CORRIDORS_DAYLIGHTS_VERSION),
    (kentos_commands::ROAD_CORRIDORS_PROTECTED, kentos_commands::ROAD_CORRIDORS_PROTECTED_VERSION),
    (kentos_commands::ROAD_CORRIDORS_PREFLIGHT, kentos_commands::ROAD_CORRIDORS_PREFLIGHT_VERSION),
    (kentos_commands::ROAD_CORRIDORS_DRAW, kentos_commands::ROAD_CORRIDORS_DRAW_VERSION),
    (kentos_commands::ROAD_VOLUMES_EDIT, kentos_commands::ROAD_VOLUMES_EDIT_VERSION),
    (kentos_commands::ROAD_VOLUMES_QUERY, kentos_commands::ROAD_VOLUMES_QUERY_VERSION),
    (kentos_commands::ROAD_VOLUMES_APPROVE, kentos_commands::ROAD_VOLUMES_APPROVE_VERSION),
    (kentos_commands::ROAD_VOLUMES_UNCERTAINTY, kentos_commands::ROAD_VOLUMES_UNCERTAINTY_VERSION),
    (kentos_commands::ROAD_VOLUMES_PROGRESS, kentos_commands::ROAD_VOLUMES_PROGRESS_VERSION),
    (kentos_commands::ROAD_VOLUMES_COMPARE, kentos_commands::ROAD_VOLUMES_COMPARE_VERSION),
    (kentos_commands::ROAD_VOLUMES_LEDGER, kentos_commands::ROAD_VOLUMES_LEDGER_VERSION),
    (kentos_commands::ROAD_VOLUMES_STATES, kentos_commands::ROAD_VOLUMES_STATES_VERSION),
    (kentos_commands::ROAD_PROFILES_IMPACT, kentos_commands::ROAD_PROFILES_IMPACT_VERSION),
    (kentos_commands::ROAD_ALIGNMENTS_IMPACT, kentos_commands::ROAD_ALIGNMENTS_IMPACT_VERSION),
    (kentos_commands::ROAD_VOLUMES_HISTORY, kentos_commands::ROAD_VOLUMES_HISTORY_VERSION),
    (kentos_commands::ROAD_HAULS_EDIT, kentos_commands::ROAD_HAULS_EDIT_VERSION),
    (kentos_commands::ROAD_HAULS_QUERY, kentos_commands::ROAD_HAULS_QUERY_VERSION),
    (kentos_commands::ROAD_HAULS_PLAN, kentos_commands::ROAD_HAULS_PLAN_VERSION),
    (kentos_commands::ROAD_HAULS_APPROVE, kentos_commands::ROAD_HAULS_APPROVE_VERSION),
    (kentos_commands::ROAD_HAULS_NETWORK, kentos_commands::ROAD_HAULS_NETWORK_VERSION),
    (kentos_commands::ROAD_HAULS_TRUCKS, kentos_commands::ROAD_HAULS_TRUCKS_VERSION),
    (kentos_commands::ROAD_GUARDRAILS_EDIT, kentos_commands::ROAD_GUARDRAILS_EDIT_VERSION),
    (kentos_commands::ROAD_GUARDRAILS_QUERY, kentos_commands::ROAD_GUARDRAILS_QUERY_VERSION),
    (kentos_commands::ROAD_GUARDRAILS_DRAW, kentos_commands::ROAD_GUARDRAILS_DRAW_VERSION),
    (kentos_commands::ROAD_CULVERTS_EDIT, kentos_commands::ROAD_CULVERTS_EDIT_VERSION),
    (kentos_commands::ROAD_CULVERTS_QUERY, kentos_commands::ROAD_CULVERTS_QUERY_VERSION),
    (kentos_commands::ROAD_CULVERTS_DRAW, kentos_commands::ROAD_CULVERTS_DRAW_VERSION),
    (kentos_commands::ROAD_CULVERTS_HYDRAULICS, kentos_commands::ROAD_CULVERTS_HYDRAULICS_VERSION),
    (kentos_commands::ROAD_CULVERTS_CLOSURE, kentos_commands::ROAD_CULVERTS_CLOSURE_VERSION),
    (kentos_commands::ROAD_WALLS_EDIT, kentos_commands::ROAD_WALLS_EDIT_VERSION),
    (kentos_commands::ROAD_WALLS_QUERY, kentos_commands::ROAD_WALLS_QUERY_VERSION),
    (kentos_commands::ROAD_WALLS_DRAW, kentos_commands::ROAD_WALLS_DRAW_VERSION),
    (kentos_commands::ROAD_PLATFORMS_EDIT, kentos_commands::ROAD_PLATFORMS_EDIT_VERSION),
    (kentos_commands::ROAD_TERRAINS_FROM_CLOUD, kentos_commands::ROAD_TERRAINS_FROM_CLOUD_VERSION),
    (kentos_commands::ROAD_DRAINAGE_QUERY, kentos_commands::ROAD_DRAINAGE_QUERY_VERSION),
    (kentos_commands::ROAD_DRAINAGE_DRAW, kentos_commands::ROAD_DRAINAGE_DRAW_VERSION),
    (kentos_commands::ROAD_SWEPT_QUERY, kentos_commands::ROAD_SWEPT_QUERY_VERSION),
    (kentos_commands::ROAD_SWEPT_DRAW, kentos_commands::ROAD_SWEPT_DRAW_VERSION),
    (kentos_commands::ROAD_TERRAINS_ANALYSIS, kentos_commands::ROAD_TERRAINS_ANALYSIS_VERSION),
    (kentos_commands::ROAD_TERRAINS_COMPARE, kentos_commands::ROAD_TERRAINS_COMPARE_VERSION),
    (kentos_commands::ROAD_TERRAINS_QUALITY, kentos_commands::ROAD_TERRAINS_QUALITY_VERSION),
    (kentos_commands::ROAD_TERRAINS_REPAIR, kentos_commands::ROAD_TERRAINS_REPAIR_VERSION),
    (kentos_commands::ROAD_TERRAINS_COVERAGE, kentos_commands::ROAD_TERRAINS_COVERAGE_VERSION),
    (kentos_commands::ROAD_TERRAINS_LOW_CONFIDENCE_DRAW, kentos_commands::ROAD_TERRAINS_LOW_CONFIDENCE_DRAW_VERSION),
    (kentos_commands::ROAD_TERRAINS_DENSITY, kentos_commands::ROAD_TERRAINS_DENSITY_VERSION),
    (kentos_commands::ROAD_TERRAINS_STOCKPILE, kentos_commands::ROAD_TERRAINS_STOCKPILE_VERSION),
    (kentos_commands::ROAD_TERRAINS_SPARSE_DRAW, kentos_commands::ROAD_TERRAINS_SPARSE_DRAW_VERSION),
    (kentos_commands::ROAD_PLATFORMS_QUERY, kentos_commands::ROAD_PLATFORMS_QUERY_VERSION),
    (kentos_commands::ROAD_PLATFORMS_DRAW, kentos_commands::ROAD_PLATFORMS_DRAW_VERSION),
    (kentos_commands::ROAD_PAY_EDIT, kentos_commands::ROAD_PAY_EDIT_VERSION),
    (kentos_commands::ROAD_PAY_QUERY, kentos_commands::ROAD_PAY_QUERY_VERSION),
    (kentos_commands::ROAD_ROUTES_FIND, kentos_commands::ROAD_ROUTES_FIND_VERSION),
    (kentos_commands::ROAD_ROUTES_DRAW, kentos_commands::ROAD_ROUTES_DRAW_VERSION),
    (kentos_commands::ROAD_JUNCTIONS_EDIT, kentos_commands::ROAD_JUNCTIONS_EDIT_VERSION),
    (kentos_commands::ROAD_JUNCTIONS_QUERY, kentos_commands::ROAD_JUNCTIONS_QUERY_VERSION),
    (kentos_commands::ROAD_JUNCTIONS_DRAW, kentos_commands::ROAD_JUNCTIONS_DRAW_VERSION),
    (kentos_commands::ROAD_JUNCTIONS_CLEARANCE, kentos_commands::ROAD_JUNCTIONS_CLEARANCE_VERSION),
    (kentos_commands::ROAD_JUNCTIONS_TIE, kentos_commands::ROAD_JUNCTIONS_TIE_VERSION),
    (kentos_commands::ROAD_JUNCTIONS_OVERLAP, kentos_commands::ROAD_JUNCTIONS_OVERLAP_VERSION),
    (kentos_commands::ROAD_PROJECT_QUERY, kentos_commands::ROAD_PROJECT_QUERY_VERSION),
    (kentos_commands::ROAD_PROJECT_GRAPH, kentos_commands::ROAD_PROJECT_GRAPH_VERSION),
    (kentos_commands::ROAD_PROJECT_REBUILD, kentos_commands::ROAD_PROJECT_REBUILD_VERSION),
    (kentos_commands::ROAD_PROJECT_PRODUCE, kentos_commands::ROAD_PROJECT_PRODUCE_VERSION),
    (kentos_commands::ROAD_PROJECT_EDIT, kentos_commands::ROAD_PROJECT_EDIT_VERSION),
    // KentOS Harita's work structure (plan/map-cad.md MAP-01, docs/adr/1200).
    (kentos_commands::MAP_WORK_QUERY, kentos_commands::MAP_WORK_QUERY_VERSION),
    (kentos_commands::MAP_WORK_EDIT, kentos_commands::MAP_WORK_EDIT_VERSION),
    // An approved item's objects put back as approved (plan/map-cad.md MAP-01, docs/adr/4080).
    (kentos_commands::MAP_WORK_RESTORE, kentos_commands::MAP_WORK_RESTORE_VERSION),
    // The scales between the ground, the ellipsoid and the map (plan/map-cad.md MAP-02, docs/adr/1231).
    (kentos_commands::CAD_CRS_FACTORS, kentos_commands::CAD_CRS_FACTORS_VERSION),
    // Kodla çiz (plan/map-cad.md MAP-04, docs/adr/1360).
    (kentos_commands::MAP_FEATURES_CODE, kentos_commands::MAP_FEATURES_CODE_VERSION),
    // GNSS baz ağı dengelemesi (plan/map-cad.md MAP-05, docs/adr/1380).
    (kentos_commands::CAD_SURVEY_BASELINES, kentos_commands::CAD_SURVEY_BASELINES_VERSION),
    // Eğimle kot ver (plan/map-cad.md MAP-06, docs/adr/1290).
    (kentos_commands::CAD_ENTITIES_GRADE, kentos_commands::CAD_ENTITIES_GRADE_VERSION),
    // Referanstan kot ver (plan/map-cad.md MAP-06, docs/adr/3780).
    (kentos_commands::CAD_ENTITIES_RAISE, kentos_commands::CAD_ENTITIES_RAISE_VERSION),
    // Yüzeyden kot ver (plan/map-cad.md MAP-06, docs/adr/4180).
    (kentos_commands::CAD_ENTITIES_DRAPE, kentos_commands::CAD_ENTITIES_DRAPE_VERSION),
    (kentos_commands::CAD_TOPOLOGY_CLEAN, kentos_commands::CAD_TOPOLOGY_CLEAN_VERSION),
    (kentos_commands::CAD_PARCELS_CHECK, kentos_commands::CAD_PARCELS_CHECK_VERSION),
    (kentos_commands::CAD_PARCELS_BOUNDARIES, kentos_commands::CAD_PARCELS_BOUNDARIES_VERSION),
    (kentos_commands::CAD_PARCELS_RECORD, kentos_commands::CAD_PARCELS_RECORD_VERSION),
    (kentos_commands::CAD_PARCELS_RECORDS, kentos_commands::CAD_PARCELS_RECORDS_VERSION),
    // Parsel alan çizelgesi and its table ([`crate::parcels_report`], docs/adr/9306).
    (kentos_commands::CAD_PARCELS_REPORT, kentos_commands::CAD_PARCELS_REPORT_VERSION),
    (kentos_commands::CAD_PARCELS_TABLE, kentos_commands::CAD_PARCELS_TABLE_VERSION),
    (kentos_commands::CAD_PARCELS_APPROVE, kentos_commands::CAD_PARCELS_APPROVE_VERSION),
    (kentos_commands::CAD_PARCELS_WITHDRAW, kentos_commands::CAD_PARCELS_WITHDRAW_VERSION),
    (kentos_commands::CAD_PIPES_CHECK, kentos_commands::CAD_PIPES_CHECK_VERSION),
    (kentos_commands::CAD_PIPES_PROFILE, kentos_commands::CAD_PIPES_PROFILE_VERSION),
    (kentos_commands::CAD_ASBUILT_CHECK, kentos_commands::CAD_ASBUILT_CHECK_VERSION),
    // İzleme incelemesi (plan/map-cad.md MAP-19, docs/adr/5220).
    (kentos_commands::CAD_ASBUILT_REVIEW, kentos_commands::CAD_ASBUILT_REVIEW_VERSION),
    (kentos_commands::CAD_FIELD_PACKAGE, kentos_commands::CAD_FIELD_PACKAGE_VERSION),
    (kentos_commands::CAD_ZONING_CHECK, kentos_commands::CAD_ZONING_CHECK_VERSION),
    (kentos_commands::CAD_ZONING_SYMBOLIZE, kentos_commands::CAD_ZONING_SYMBOLIZE_VERSION),
    (kentos_commands::CAD_ZONING_DRAFT, kentos_commands::CAD_ZONING_DRAFT_VERSION),
    (kentos_commands::CAD_SCENARIOS_IMPACT, kentos_commands::CAD_SCENARIOS_IMPACT_VERSION),
    (kentos_commands::CAD_SCENARIOS_RECORD, kentos_commands::CAD_SCENARIOS_RECORD_VERSION),
    (kentos_commands::CAD_READJUSTMENT_DISTRIBUTE, kentos_commands::CAD_READJUSTMENT_DISTRIBUTE_VERSION),
    (kentos_commands::CAD_READJUSTMENT_SCOPE, kentos_commands::CAD_READJUSTMENT_SCOPE_VERSION),
    (kentos_commands::CAD_READJUSTMENT_SHEETS, kentos_commands::CAD_READJUSTMENT_SHEETS_VERSION),
    (kentos_commands::CAD_ORTHOPHOTO_CHECK, kentos_commands::CAD_ORTHOPHOTO_CHECK_VERSION),
    (kentos_commands::CAD_ORTHOPHOTO_COMPARE, kentos_commands::CAD_ORTHOPHOTO_COMPARE_VERSION),
    (kentos_commands::CAD_ORTHOPHOTO_SEAMLINES, kentos_commands::CAD_ORTHOPHOTO_SEAMLINES_VERSION),
    // Birleşme çizgisi ağı ve ayak izi (plan/map-cad.md MAP-12, docs/adr/5260).
    (kentos_commands::CAD_ORTHOPHOTO_NETWORK, kentos_commands::CAD_ORTHOPHOTO_NETWORK_VERSION),
    (kentos_commands::CAD_ORTHOPHOTO_FOOTPRINT, kentos_commands::CAD_ORTHOPHOTO_FOOTPRINT_VERSION),
    (kentos_commands::CAD_POINTCLOUDS_OVERLAP, kentos_commands::CAD_POINTCLOUDS_OVERLAP_VERSION),
    (kentos_commands::CAD_POINTCLOUDS_REGISTER, kentos_commands::CAD_POINTCLOUDS_REGISTER_VERSION),
    (kentos_commands::CAD_POINTCLOUDS_NETWORK, kentos_commands::CAD_POINTCLOUDS_NETWORK_VERSION),
    (kentos_commands::CAD_POINTCLOUDS_PLACE, kentos_commands::CAD_POINTCLOUDS_PLACE_VERSION),
    // The catalog's migration report (plan/map-cad.md MAP-04, docs/adr/4940).
    (kentos_commands::CAD_PROFILES_COMPARE, kentos_commands::CAD_PROFILES_COMPARE_VERSION),
    (kentos_commands::CAD_PROFILES_CARRY, kentos_commands::CAD_PROFILES_CARRY_VERSION),
    // KentOS Harita's GNSS campaign archive (plan/map-cad.md MAP-03, docs/adr/1340).
    (
        kentos_commands::MAP_SURVEY_QUERY,
        kentos_commands::MAP_SURVEY_QUERY_VERSION,
    ),
    (
        kentos_commands::MAP_SURVEY_EDIT,
        kentos_commands::MAP_SURVEY_EDIT_VERSION,
    ),
    (
        kentos_commands::MAP_SURVEY_NETWORK,
        kentos_commands::MAP_SURVEY_NETWORK_VERSION,
    ),
    (
        kentos_commands::MAP_FEATURES_RECONCILE,
        kentos_commands::MAP_FEATURES_RECONCILE_VERSION,
    ),
    (
        kentos_commands::ROAD_STAKEOUT_QUERY,
        kentos_commands::ROAD_STAKEOUT_QUERY_VERSION,
    ),
    (
        kentos_commands::FILE_EXPORT_LANDXML,
        kentos_commands::FILE_EXPORT_LANDXML_VERSION,
    ),
    (
        kentos_commands::FILE_EXPORT_IFC,
        kentos_commands::FILE_EXPORT_IFC_VERSION,
    ),
    (
        kentos_commands::CAD_ARCH_IFC,
        kentos_commands::CAD_ARCH_IFC_VERSION,
    ),
    (
        kentos_commands::CAD_IFC_INSPECT,
        kentos_commands::CAD_IFC_INSPECT_VERSION,
    ),
    (
        kentos_commands::CAD_IFC_COMPARE,
        kentos_commands::CAD_IFC_COMPARE_VERSION,
    ),
    (kentos_commands::CAD_BCF_WRITE, kentos_commands::CAD_BCF_WRITE_VERSION),
    (kentos_commands::CAD_BCF_READ, kentos_commands::CAD_BCF_READ_VERSION),
    (
        kentos_commands::FILE_EXPORT_GLTF,
        kentos_commands::FILE_EXPORT_GLTF_VERSION,
    ),
    (
        kentos_commands::ROAD_STAKEOUT_SEND,
        kentos_commands::ROAD_STAKEOUT_SEND_VERSION,
    ),
    (
        kentos_commands::ROAD_SHEETS_PLAN,
        kentos_commands::ROAD_SHEETS_PLAN_VERSION,
    ),
    (
        kentos_commands::ROAD_SHEETS_SECTIONS,
        kentos_commands::ROAD_SHEETS_SECTIONS_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_FIT,
        kentos_commands::ROAD_ALIGNMENTS_FIT_VERSION,
    ),
    (
        kentos_commands::ROAD_PROFILES_FIT,
        kentos_commands::ROAD_PROFILES_FIT_VERSION,
    ),
    (
        kentos_commands::ROAD_ALIGNMENTS_IMPORT,
        kentos_commands::ROAD_ALIGNMENTS_IMPORT_VERSION,
    ),
    // Alt ağlar ([`subnetworks`], [`subnetworks_update`], docs/adr/0557 §2).
    (
        kentos_commands::CAD_SUBNETWORKS_FIND,
        kentos_commands::CAD_SUBNETWORKS_FIND_VERSION,
    ),
    (
        kentos_commands::CAD_SUBNETWORKS_UPDATE,
        kentos_commands::CAD_SUBNETWORKS_UPDATE_VERSION,
    ),
    // Topolojiyi denetle ve istisnalar ([`topology_check`], [`topology_exceptions`], docs/adr/0589).
    (
        kentos_commands::CAD_TOPOLOGY_CHECK,
        kentos_commands::CAD_TOPOLOGY_CHECK_VERSION,
    ),
    (
        kentos_commands::CAD_TOPOLOGY_EXCEPTIONS,
        kentos_commands::CAD_TOPOLOGY_EXCEPTIONS_VERSION,
    ),
    // Topoloji onarım tarifi ([`topology_repair`], docs/adr/1260 §4).
    (
        kentos_commands::CAD_TOPOLOGY_REPAIR,
        kentos_commands::CAD_TOPOLOGY_REPAIR_VERSION,
    ),
    // Adres verisini denetle ([`addresses_check`], docs/adr/0579).
    (
        kentos_commands::CAD_ADDRESSES_CHECK,
        kentos_commands::CAD_ADDRESSES_CHECK_VERSION,
    ),
    // Adres kaydıyla karşılaştır ([`addresses_compare`], docs/adr/0736).
    (
        kentos_commands::CAD_ADDRESSES_COMPARE,
        kentos_commands::CAD_ADDRESSES_COMPARE_VERSION,
    ),
    // Adres yazılışlarını düzelt ([`addresses_normalize`], docs/adr/0647 §2).
    (
        kentos_commands::CAD_ADDRESSES_NORMALIZE,
        kentos_commands::CAD_ADDRESSES_NORMALIZE_VERSION,
    ),
    // Adres verisi ([`address_define`], docs/adr/0229 §14).
    (
        kentos_commands::CAD_ADDRESSES_DEFINE,
        kentos_commands::CAD_ADDRESSES_DEFINE_VERSION,
    ),
    // Zaman ayarları and the scenarios ([`layers_time`], [`scenarios_edit`], docs/adr/0210 §11).
    (
        kentos_commands::CAD_LAYERS_TIME,
        kentos_commands::CAD_LAYERS_TIME_VERSION,
    ),
    (
        kentos_commands::CAD_SCENARIOS_EDIT,
        kentos_commands::CAD_SCENARIOS_EDIT_VERSION,
    ),
    // Katman süzgeci ([`layers_filter`], docs/adr/0211 §5).
    (
        kentos_commands::CAD_LAYERS_FILTER,
        kentos_commands::CAD_LAYERS_FILTER_VERSION,
    ),
    // Katman çizimi ([`layers_compose`], docs/adr/0226 §3, §4).
    (
        kentos_commands::CAD_LAYERS_COMPOSE,
        kentos_commands::CAD_LAYERS_COMPOSE_VERSION,
    ),
    // Bilgi kartı ve eylemler ([`layers_info`], docs/adr/0225 §4, §5).
    (
        kentos_commands::CAD_LAYERS_INFO,
        kentos_commands::CAD_LAYERS_INFO_VERSION,
    ),
    // Etiketler and the label tools ([`layers_labels`], [`labels_pin`], docs/adr/0212 §5).
    (
        kentos_commands::CAD_LAYERS_LABELS,
        kentos_commands::CAD_LAYERS_LABELS_VERSION,
    ),
    (
        kentos_commands::CAD_LABELS_PIN,
        kentos_commands::CAD_LABELS_PIN_VERSION,
    ),
    // Etiketleri denetle ([`labels_report`], docs/adr/0533 §6).
    (
        kentos_commands::CAD_LABELS_REPORT,
        kentos_commands::CAD_LABELS_REPORT_VERSION,
    ),
    // Yeniden bağla ([`dimensions_reassociate`], docs/adr/0249 §3).
    (
        kentos_commands::CAD_DIMENSIONS_REASSOCIATE,
        kentos_commands::CAD_DIMENSIONS_REASSOCIATE_VERSION,
    ),
    // Ölçülendirme by what it measures ([`dimensions_create`], docs/adr/0255).
    (
        kentos_commands::CAD_DIMENSIONS_CREATE,
        kentos_commands::CAD_DIMENSIONS_CREATE_VERSION,
    ),
    // İç ölçü zinciri ([`dimensions_interior`], docs/adr/0711).
    (
        kentos_commands::CAD_DIMENSIONS_INTERIOR,
        kentos_commands::CAD_DIMENSIONS_INTERIOR_VERSION,
    ),
    // A dimension's spacing, breaks, moved value, written value and options ([`dimensions_edit`], docs/adr/0321 §4).
    (
        kentos_commands::CAD_DIMENSIONS_EDIT,
        kentos_commands::CAD_DIMENSIONS_EDIT_VERSION,
    ),
    // What dimensions measure and show ([`dimensions_query`], docs/adr/0321 §3).
    (
        kentos_commands::CAD_DIMENSIONS_QUERY,
        kentos_commands::CAD_DIMENSIONS_QUERY_VERSION,
    ),
    // Buda, Uzat, Köşe yuvarla and Pah by their meaning (docs/adr/0250).
    (
        kentos_commands::CAD_ENTITIES_TRIM,
        kentos_commands::CAD_ENTITIES_TRIM_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_EXTEND,
        kentos_commands::CAD_ENTITIES_EXTEND_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_FILLET,
        kentos_commands::CAD_ENTITIES_FILLET_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_CHAMFER,
        kentos_commands::CAD_ENTITIES_CHAMFER_VERSION,
    ),
    // Esnet, Kır, Birleştir, Böl, Alan oluştur and Tarama by their meaning (docs/adr/0254).
    (
        kentos_commands::CAD_ENTITIES_STRETCH,
        kentos_commands::CAD_ENTITIES_STRETCH_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_BREAK,
        kentos_commands::CAD_ENTITIES_BREAK_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_JOIN,
        kentos_commands::CAD_ENTITIES_JOIN_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_REVERSE,
        kentos_commands::CAD_ENTITIES_REVERSE_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_LENGTHEN,
        kentos_commands::CAD_ENTITIES_LENGTHEN_VERSION,
    ),
    (kentos_commands::CAD_DRAWING_INSPECT, kentos_commands::CAD_DRAWING_INSPECT_VERSION),
    (kentos_commands::CAD_ARRAYS_FIND, kentos_commands::CAD_ARRAYS_FIND_VERSION),
    (
        kentos_commands::CAD_ARRAYS_ADOPT,
        kentos_commands::CAD_ARRAYS_ADOPT_VERSION,
    ),
    (
        kentos_commands::CAD_ARRAYS_EDIT,
        kentos_commands::CAD_ARRAYS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_CONSTRAINTS_EDIT,
        kentos_commands::CAD_CONSTRAINTS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_CONSTRAINTS_QUERY,
        kentos_commands::CAD_CONSTRAINTS_QUERY_VERSION,
    ),
    (
        kentos_commands::CAD_PARAMETERS_EDIT,
        kentos_commands::CAD_PARAMETERS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_EXPLODE,
        kentos_commands::CAD_ENTITIES_EXPLODE_VERSION,
    ),
    (
        kentos_commands::CAD_CURVES_EDIT,
        kentos_commands::CAD_CURVES_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_MATCH,
        kentos_commands::CAD_ENTITIES_MATCH_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_CLEANUP,
        kentos_commands::CAD_ENTITIES_CLEANUP_VERSION,
    ),
    (
        kentos_commands::CAD_DRAWING_AUDIT,
        kentos_commands::CAD_DRAWING_AUDIT_VERSION,
    ),
    // İki revizyonu karşılaştır ([`drawing_compare`], docs/adr/0576).
    (
        kentos_commands::CAD_DRAWING_COMPARE,
        kentos_commands::CAD_DRAWING_COMPARE_VERSION,
    ),
    (
        kentos_commands::CAD_DRAWING_TAKE,
        kentos_commands::CAD_DRAWING_TAKE_VERSION,
    ),
    (
        kentos_commands::CAD_DRAWING_REPAIR,
        kentos_commands::CAD_DRAWING_REPAIR_VERSION,
    ),
    (
        kentos_commands::CAD_REVIEW_NOTES_EDIT,
        kentos_commands::CAD_REVIEW_NOTES_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_REVIEW_NOTES_LIST,
        kentos_commands::CAD_REVIEW_NOTES_LIST_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_COPY,
        kentos_commands::CAD_ENTITIES_COPY_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_PASTE,
        kentos_commands::CAD_ENTITIES_PASTE_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_DIVIDE,
        kentos_commands::CAD_ENTITIES_DIVIDE_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_BOUNDARY,
        kentos_commands::CAD_ENTITIES_BOUNDARY_VERSION,
    ),
    (
        kentos_commands::CAD_ENTITIES_HATCH,
        kentos_commands::CAD_ENTITIES_HATCH_VERSION,
    ),
    // Katman stili's renderers ([`layers_renderer`], docs/adr/0213 §5).
    (
        kentos_commands::CAD_LAYERS_RENDERER,
        kentos_commands::CAD_LAYERS_RENDERER_VERSION,
    ),
    // The layer tree, its own changes and its states, the text and dimension
    // styles, the project's settings ([`layers_edit`], [`layers_set`],
    // [`layers_states`], [`styles_edit`], [`settings_set`], docs/adr/0248).
    (
        kentos_commands::CAD_LAYERS_EDIT,
        kentos_commands::CAD_LAYERS_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_SET,
        kentos_commands::CAD_LAYERS_SET_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_STATES,
        kentos_commands::CAD_LAYERS_STATES_VERSION,
    ),
    // Where a layer's field is named ([`fields_usage`], docs/adr/0414).
    (
        kentos_commands::CAD_FIELDS_USAGE,
        kentos_commands::CAD_FIELDS_USAGE_VERSION,
    ),
    // The layers found by a name pattern and their properties, and layers
    // translated to a standard's ([`layers_find`], [`layers_translate`], docs/adr/0311).
    (
        kentos_commands::CAD_LAYERS_FIND,
        kentos_commands::CAD_LAYERS_FIND_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_TRANSLATE,
        kentos_commands::CAD_LAYERS_TRANSLATE_VERSION,
    ),
    // A drawing standard checked and applied ([`standards`], docs/adr/0312).
    (
        kentos_commands::CAD_STANDARDS_CHECK,
        kentos_commands::CAD_STANDARDS_CHECK_VERSION,
    ),
    (
        kentos_commands::CAD_STANDARDS_CONFLICTS,
        kentos_commands::CAD_STANDARDS_CONFLICTS_VERSION,
    ),
    (
        kentos_commands::CAD_STANDARDS_INSTALL,
        kentos_commands::CAD_STANDARDS_INSTALL_VERSION,
    ),
    // Kabul edilmiş standart bulguları ([`standards_exceptions`], docs/adr/1129).
    (
        kentos_commands::CAD_STANDARDS_EXCEPTIONS,
        kentos_commands::CAD_STANDARDS_EXCEPTIONS_VERSION,
    ),
    (
        kentos_commands::CAD_STANDARDS_RESOLVE,
        kentos_commands::CAD_STANDARDS_RESOLVE_VERSION,
    ),
    (
        kentos_commands::CAD_STANDARDS_PALETTES,
        kentos_commands::CAD_STANDARDS_PALETTES_VERSION,
    ),
    (
        kentos_commands::CAD_STANDARDS_APPLY,
        kentos_commands::CAD_STANDARDS_APPLY_VERSION,
    ),
    // A data profile applied and checked ([`profiles`], docs/adr/0541).
    (
        kentos_commands::CAD_PROFILES_APPLY,
        kentos_commands::CAD_PROFILES_APPLY_VERSION,
    ),
    (
        kentos_commands::CAD_PROFILES_CHECK,
        kentos_commands::CAD_PROFILES_CHECK_VERSION,
    ),
    // An institution's own profile imported, listed, migrated and taken out of the drawing ([`profile_files`],
    // docs/adr/0558, 0735).
    (
        kentos_commands::CAD_PROFILES_IMPORT,
        kentos_commands::CAD_PROFILES_IMPORT_VERSION,
    ),
    (
        kentos_commands::CAD_PROFILES_LIST,
        kentos_commands::CAD_PROFILES_LIST_VERSION,
    ),
    (
        kentos_commands::CAD_PROFILES_MIGRATE,
        kentos_commands::CAD_PROFILES_MIGRATE_VERSION,
    ),
    (
        kentos_commands::CAD_PROFILES_REMOVE,
        kentos_commands::CAD_PROFILES_REMOVE_VERSION,
    ),
    // The fields' names in an institution by a mapping's form of names ([`profiles_names`], docs/adr/5180).
    (
        kentos_commands::CAD_PROFILES_NAMES,
        kentos_commands::CAD_PROFILES_NAMES_VERSION,
    ),
    // The project's line types with patterns ([`line_types_edit`], docs/adr/0313).
    (
        kentos_commands::CAD_LINE_TYPES_EDIT,
        kentos_commands::CAD_LINE_TYPES_EDIT_VERSION,
    ),
    // The project's own hatch patterns ([`hatch_patterns_edit`], docs/adr/0331 §1).
    (
        kentos_commands::CAD_HATCH_PATTERNS_EDIT,
        kentos_commands::CAD_HATCH_PATTERNS_EDIT_VERSION,
    ),
    // Blocks from a drawing file ([`file_import_block`], docs/adr/0316 §3).
    (
        kentos_commands::FILE_IMPORT_BLOCK,
        kentos_commands::FILE_IMPORT_BLOCK_VERSION,
    ),
    // A block placed ([`blocks_insert`], docs/adr/0315 §4).
    (
        kentos_commands::CAD_BLOCKS_INSERT,
        kentos_commands::CAD_BLOCKS_INSERT_VERSION,
    ),
    // A block's inserts placing another, and the inserts counted, found and scheduled ([`blocks_replace`],
    // [`blocks_query`], docs/adr/0317 §3).
    (
        kentos_commands::CAD_BLOCKS_REPLACE,
        kentos_commands::CAD_BLOCKS_REPLACE_VERSION,
    ),
    (
        kentos_commands::CAD_BLOCKS_QUERY,
        kentos_commands::CAD_BLOCKS_QUERY_VERSION,
    ),
    // A dynamic block's inserts' parameter values ([`block_params`], docs/adr/0318 §3).
    (kentos_commands::CAD_BLOCKS_FIND, kentos_commands::CAD_BLOCKS_FIND_VERSION),
    (kentos_commands::CAD_BLOCKS_CONVERT, kentos_commands::CAD_BLOCKS_CONVERT_VERSION),
    (
        kentos_commands::CAD_BLOCKS_PARAMETERS,
        kentos_commands::CAD_BLOCKS_PARAMETERS_VERSION,
    ),
    // What an insert draws, and a dynamic block's inserts made plain ([`block_variants`], docs/adr/0319 §4).
    (
        kentos_commands::CAD_BLOCKS_PROXY,
        kentos_commands::CAD_BLOCKS_PROXY_VERSION,
    ),
    (
        kentos_commands::CAD_BLOCKS_FREEZE,
        kentos_commands::CAD_BLOCKS_FREEZE_VERSION,
    ),
    // The project's style library and a change's reach ([`library_edit`], [`styles_impact`], docs/adr/0314).
    (
        kentos_commands::CAD_LIBRARY_EDIT,
        kentos_commands::CAD_LIBRARY_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_STYLES_IMPACT,
        kentos_commands::CAD_STYLES_IMPACT_VERSION,
    ),
    (
        kentos_commands::CAD_STYLES_EDIT,
        kentos_commands::CAD_STYLES_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_SETTINGS_SET,
        kentos_commands::CAD_SETTINGS_SET_VERSION,
    ),
    // A local project's unit, keeping the sizes or the numbers ([`units_change`], docs/adr/0256 §1).
    (
        kentos_commands::CAD_UNITS_CHANGE,
        kentos_commands::CAD_UNITS_CHANGE_VERSION,
    ),
    // A DXF into the drawing, its unit decided by one rule ([`file_import`], docs/adr/0257).
    (
        kentos_commands::FILE_IMPORT_DXF,
        kentos_commands::FILE_IMPORT_DXF_VERSION,
    ),
    (kentos_commands::FILE_IMPORT_DWG, kentos_commands::FILE_IMPORT_DWG_VERSION),
    // Toplu yazma of the attribute table ([`fields_write`], docs/adr/0507).
    (
        kentos_commands::CAD_FIELDS_WRITE,
        kentos_commands::CAD_FIELDS_WRITE_VERSION,
    ),
    (
        kentos_commands::CAD_PROCESSING_TOOLS,
        kentos_commands::CAD_PROCESSING_TOOLS_VERSION,
    ),
    (
        kentos_commands::CAD_PROCESSING_RUN,
        kentos_commands::CAD_PROCESSING_RUN_VERSION,
    ),
    // İşlemler's tool or model over a table of rows (docs/adr/0529).
    (
        kentos_commands::CAD_PROCESSING_BATCH,
        kentos_commands::CAD_PROCESSING_BATCH_VERSION,
    ),
    // Bound layers ([`sources`], docs/adr/0503).
    (
        kentos_commands::CAD_SOURCES_DESCRIBE,
        kentos_commands::CAD_SOURCES_DESCRIBE_VERSION,
    ),
    (
        kentos_commands::CAD_SOURCES_EDIT,
        kentos_commands::CAD_SOURCES_EDIT_VERSION,
    ),
    (
        kentos_commands::CAD_SOURCES_SYNC,
        kentos_commands::CAD_SOURCES_SYNC_VERSION,
    ),
    // Both sides of a bound layer changed ([`sources::reconcile`], docs/adr/0538).
    (
        kentos_commands::CAD_SOURCES_RECONCILE,
        kentos_commands::CAD_SOURCES_RECONCILE_VERSION,
    ),
    // A many-to-many reference in a bound GeoPackage ([`sources::relate`], docs/adr/0532).
    (
        kentos_commands::CAD_SOURCES_RELATE,
        kentos_commands::CAD_SOURCES_RELATE_VERSION,
    ),
    // A layer's records, or a relation's, a page at a time ([`records_page`], docs/adr/0532).
    (
        kentos_commands::CAD_RECORDS_PAGE,
        kentos_commands::CAD_RECORDS_PAGE_VERSION,
    ),
    // A layer's source system and its objects moved from it ([`layers_crs`], docs/adr/0583).
    // A data layer added, changed or removed: its rows stay in the source ([`layers_data`], docs/adr/9101 §4).
    (
        kentos_commands::CAD_LAYERS_DATA,
        kentos_commands::CAD_LAYERS_DATA_VERSION,
    ),
    // A data layer's rows read from its source ([`layers_rows`], docs/adr/9102 §4).
    (
        kentos_commands::CAD_LAYERS_ROWS,
        kentos_commands::CAD_LAYERS_ROWS_VERSION,
    ),
    // A layer written to a GeoPackage or PostGIS table or a GeoJSON, Shapefile, CSV or KML file ([`layer_export`],
    // docs/adr/9106).
    (
        kentos_commands::CAD_LAYERS_EXPORT,
        kentos_commands::CAD_LAYERS_EXPORT_VERSION,
    ),
    // A new layer made as a new table of a data source ([`layers_create`], docs/adr/9107 §7).
    (
        kentos_commands::CAD_LAYERS_CREATE,
        kentos_commands::CAD_LAYERS_CREATE_VERSION,
    ),
    // A drawing layer moved into a data source ([`layers_migrate`], docs/adr/9107 §8).
    (
        kentos_commands::CAD_LAYERS_MIGRATE,
        kentos_commands::CAD_LAYERS_MIGRATE_VERSION,
    ),
    // A layer's style classes, counted by the drawing's rule ([`layer_classes`], docs/adr/9105 §2).
    (
        kentos_commands::CAD_LAYERS_CLASSES,
        kentos_commands::CAD_LAYERS_CLASSES_VERSION,
    ),
    // A data layer's edit session: opened, saved to its table, let go, closed ([`layers_session`], docs/adr/9103 §3).
    (
        kentos_commands::CAD_LAYERS_SESSION,
        kentos_commands::CAD_LAYERS_SESSION_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_CRS,
        kentos_commands::CAD_LAYERS_CRS_VERSION,
    ),
    (
        kentos_commands::CAD_LAYERS_REPROJECT,
        kentos_commands::CAD_LAYERS_REPROJECT_VERSION,
    ),
    // A data set asked where it is: the box, the columns and the filter sent to the source ([`sources_query`], docs/adr/0581).
    (
        kentos_commands::CAD_SOURCES_QUERY,
        kentos_commands::CAD_SOURCES_QUERY_VERSION,
    ),
    // A GeoPackage into the drawing ([`file_import_gpkg`], docs/adr/0502).
    (
        kentos_commands::FILE_IMPORT_GPKG,
        kentos_commands::FILE_IMPORT_GPKG_VERSION,
    ),
    // A CSV's rows into the drawing ([`file_import_csv`], docs/adr/0522).
    (
        kentos_commands::FILE_IMPORT_CSV,
        kentos_commands::FILE_IMPORT_CSV_VERSION,
    ),
    // A FlatGeobuf's features into the drawing ([`file_import_fgb`], docs/adr/0544).
    (
        kentos_commands::FILE_IMPORT_FGB,
        kentos_commands::FILE_IMPORT_FGB_VERSION,
    ),
    // A GeoParquet's rows into the drawing ([`file_import_parquet`], docs/adr/0628).
    (
        kentos_commands::FILE_IMPORT_PARQUET,
        kentos_commands::FILE_IMPORT_PARQUET_VERSION,
    ),
    // A GML's features into the drawing ([`file_import_gml`], docs/adr/0545).
    (
        kentos_commands::FILE_IMPORT_GML,
        kentos_commands::FILE_IMPORT_GML_VERSION,
    ),
    // A Shapefile, a GeoJSON file and a KML/KMZ into the drawing ([`file_import_gis`], docs/adr/0563).
    (
        kentos_commands::FILE_IMPORT_SHP,
        kentos_commands::FILE_IMPORT_SHP_VERSION,
    ),
    (
        kentos_commands::FILE_IMPORT_GEOJSON,
        kentos_commands::FILE_IMPORT_GEOJSON_VERSION,
    ),
    (
        kentos_commands::FILE_IMPORT_KML,
        kentos_commands::FILE_IMPORT_KML_VERSION,
    ),
    // A derived GIS dataset against the drawing ([`gis_derivation`], docs/adr/0530).
    (
        kentos_commands::CAD_GIS_DERIVATION,
        kentos_commands::CAD_GIS_DERIVATION_VERSION,
    ),
    // What a GIS changed in a derived dataset written back ([`gis_takeback`], docs/adr/0634 §1).
    (
        kentos_commands::CAD_GIS_TAKEBACK,
        kentos_commands::CAD_GIS_TAKEBACK_VERSION,
    ),
    // A derived table made the GIS's own data ([`gis_detach`], docs/adr/0634 §2).
    (
        kentos_commands::CAD_GIS_DETACH,
        kentos_commands::CAD_GIS_DETACH_VERSION,
    ),
    // An SLD's look made a layer's ([`sld`], docs/adr/0516).
    (
        kentos_commands::FILE_IMPORT_SLD,
        kentos_commands::FILE_IMPORT_SLD_VERSION,
    ),
    // A QML's look made a layer's ([`qml`], docs/adr/0555).
    (
        kentos_commands::FILE_IMPORT_QML,
        kentos_commands::FILE_IMPORT_QML_VERSION,
    ),
    // A MapLibre style's look made the layers' ([`maplibre`], docs/adr/0560).
    (
        kentos_commands::FILE_IMPORT_MAPLIBRE,
        kentos_commands::FILE_IMPORT_MAPLIBRE_VERSION,
    ),
    // A PDF page into the drawing ([`file_import_pdf`], docs/adr/0352).
    (
        kentos_commands::FILE_IMPORT_PDF,
        kentos_commands::FILE_IMPORT_PDF_VERSION,
    ),
    (kentos_commands::CAD_SHEETS_CREATE, kentos_commands::CAD_SHEETS_CREATE_VERSION),
    (kentos_commands::CAD_SHEETS_FIT, kentos_commands::CAD_SHEETS_FIT_VERSION),
    // Kat planı paftaları ([`sheets_levels`], docs/adr/0718).
    (kentos_commands::CAD_SHEETS_LEVELS, kentos_commands::CAD_SHEETS_LEVELS_VERSION),
    // Detay paftaları ([`sheets_details`], docs/adr/1090 §2).
    (kentos_commands::CAD_SHEETS_DETAILS, kentos_commands::CAD_SHEETS_DETAILS_VERSION),
    // Baskı stili tablosu içe aktar ([`plot_styles_import`], docs/adr/1130).
    (kentos_commands::CAD_PLOTSTYLES_IMPORT, kentos_commands::CAD_PLOTSTYLES_IMPORT_VERSION),
    // Pafta kitabı ([`sheets_edit`], docs/adr/0252 §2).
    (
        kentos_commands::CAD_SHEETS_EDIT,
        kentos_commands::CAD_SHEETS_EDIT_VERSION,
    ),
    // The change set: several commands as one change ([`changeset`], docs/adr/0253 §8).
    (
        kentos_commands::CAD_CHANGESET_APPLY,
        kentos_commands::CAD_CHANGESET_APPLY_VERSION,
    ),
];
