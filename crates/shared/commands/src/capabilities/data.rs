//! What the commands of references, rasters, sensors, point clouds, meshes and views take and how they answer (docs/adr/0253 §7): one of the parts the table
//! was cut into, in its order (docs/adr/8054 §4).

use super::*;

pub(super) const ENTRIES: &[Entry] = &[
    // External references (docs/adr/0348).
    Entry {
        id: "cad.xrefs.edit",
        refusals: &[
            &[
                "xref_not_found",
                "nested_xref",
                "not_finite",
                "invalid_scale",
            ],
            LAYER,
            &[
                "invalid_file_path",
                "file_not_found",
                "import_unreadable",
                "pdf_page_not_found",
                "xref_cycle",
                "xref_empty",
                "block_refused",
            ],
            &["not_an_underlay", "invalid_underlay"],
            REVISION,
        ],
        warnings: &["objects_left_out", "layer_hidden", "pdf_left_out", "ties_break"],
        targets: none,
    },
    Entry {
        id: "cad.xrefs.list",
        refusals: &[],
        warnings: &[],
        targets: none,
    },
    // Sanal mozaik (docs/adr/0517).
    Entry {
        id: "cad.rasters.mosaic",
        refusals: &[
            &[
                "mosaic_empty",
                "mosaic_too_large",
                "invalid_file_path",
                "invalid_pixel_size",
            ],
            &[
                "file_not_found",
                "import_unreadable",
                "mosaic_unplaced",
                "mosaic_rotated",
                "mosaic_mismatch",
            ],
            &["file_exists", "crs_mismatch"],
            LAYER,
            REVISION,
        ],
        warnings: &["mosaic_resampled", "mosaic_undated", "layer_hidden"],
        targets: none,
    },
    // Rasteri projenin sistemine çevir (docs/adr/0644 §3).
    Entry {
        id: "cad.rasters.reproject",
        refusals: &[
            &[
                "invalid_file_path",
                "invalid_input",
                "file_not_found",
                "import_unreadable",
            ],
            &[
                "crs_unknown",
                "crs_mismatch",
                "crs_unreached",
                "file_exists",
            ],
            LAYER,
            REVISION,
        ],
        warnings: &["crs_overridden", "crs_unofficial", "layer_hidden"],
        targets: none,
    },
    // GeoPackage karoları (docs/adr/0556).
    Entry {
        id: "cad.rasters.gpkgread",
        refusals: &[
            &[
                "invalid_file_path",
                "file_not_found",
                "import_unreadable",
                "no_tile_table",
                "table_not_found",
            ],
            &[
                "zoom_not_found",
                "invalid_area",
                "tile_table_empty",
                "gpkg_tiles_too_large",
            ],
            &[
                "tile_unreadable",
                "tile_format_unsupported",
                "file_exists",
                "crs_mismatch",
                "file_unwritable",
            ],
            // docs/adr/0627 §3: a linked read writes no GeoTIFF and adds a raster.
            &["link_unusable"],
            LAYER,
            REVISION,
        ],
        warnings: &["crs_undeclared", "tiles_missing", "layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.rasters.gpkgwrite",
        refusals: &[
            &[
                "invalid_uid",
                "entity_not_found",
                "not_a_raster",
                "invalid_file_path",
                "invalid_table_name",
            ],
            &[
                "raster_unsupported",
                "raster_not_north_up",
                "gpkg_tiles_too_large",
                "unknown_asset",
            ],
            &[
                "file_not_found",
                "import_unreadable",
                "file_exists",
                "table_exists",
                "file_unwritable",
            ],
            // docs/adr/0627 §2: a height's tiles are no WebP.
            &["tile_format_unsupported"],
        ],
        warnings: &["nodata_lost", "precision_lost"],
        targets: none,
    },
    // Zamanda sorgu (docs/adr/0537).
    Entry {
        id: "cad.time.query",
        refusals: &[&[
            "no_such_step",
            "invalid_file_path",
            "file_not_found",
            "layer_not_found",
            "not_a_layer",
            "not_temporal",
            "invalid_moment",
            "invalid_period",
            "not_recorded",
            "state_changed",
        ]],
        warnings: &[],
        targets: none,
    },
    // Sensör gözlemleri (docs/adr/0646 §4, §5).
    Entry {
        id: "cad.sensors.ingest",
        refusals: &[&[
            "invalid_revision",
            "revision_conflict",
            "layer_not_found",
            "not_a_layer",
            "layer_locked",
            "not_temporal",
            "no_asset_field",
            "invalid_input",
            "invalid_source",
            "invalid_moment",
            "invalid_file_path",
            "file_not_found",
            "import_unreadable",
            "source_unreachable",
            "invalid_count",
        ]],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.sensors.latest",
        refusals: &[&[
            "layer_not_found",
            "not_a_layer",
            "not_temporal",
            "no_asset_field",
            "invalid_input",
            "invalid_moment",
            "not_recorded",
        ]],
        warnings: &[],
        targets: none,
    },
    // Nokta bulutunun bilgisi ve XYZ sor (docs/adr/0536).
    Entry {
        id: "cad.pointclouds.describe",
        refusals: &[&[
            "invalid_uid",
            "entity_not_found",
            "not_a_pointcloud",
            "too_many_cloud_points",
        ]],
        warnings: &["cloud_source_unreadable"],
        targets: || {
            vec![target(
                None,
                &["pointcloud"],
                &[],
                Some("not_a_pointcloud"),
                None,
            )]
        },
    },
    Entry {
        id: "cad.pointclouds.query",
        refusals: &[&[
            "invalid_uid",
            "entity_not_found",
            "not_a_pointcloud",
            "invalid_reach",
        ]],
        warnings: &["cloud_source_unreadable"],
        targets: || {
            vec![target(
                None,
                &["pointcloud"],
                &[],
                Some("not_a_pointcloud"),
                None,
            )]
        },
    },
    // Nokta bulutunun alan ve profil sorgusu, meshin bilgisi ve sorgusu (docs/adr/0552).
    Entry {
        id: "cad.pointclouds.region",
        refusals: &[&[
            "invalid_uid",
            "entity_not_found",
            "not_a_pointcloud",
            "invalid_cloud_area",
            "too_many_cloud_points",
        ]],
        warnings: &["cloud_source_unreadable"],
        targets: || {
            vec![target(
                None,
                &["pointcloud"],
                &[],
                Some("not_a_pointcloud"),
                None,
            )]
        },
    },
    Entry {
        id: "cad.pointclouds.profile",
        refusals: &[
            &[
                "invalid_uid",
                "entity_not_found",
                "not_a_pointcloud",
                "invalid_corridor",
                "too_many_stations",
            ],
            &["too_many_cloud_points", "too_many_listed_points"],
        ],
        warnings: &["cloud_source_unreadable"],
        targets: || {
            vec![target(
                None,
                &["pointcloud"],
                &[],
                Some("not_a_pointcloud"),
                None,
            )]
        },
    },
    Entry {
        id: "cad.meshes.describe",
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_mesh"]],
        warnings: &["mesh_source_unreadable"],
        targets: || vec![target(None, &["raster"], &[], Some("not_a_mesh"), None)],
    },
    Entry {
        id: "cad.meshes.query",
        refusals: &[&[
            "invalid_uid",
            "entity_not_found",
            "not_a_mesh",
            "mesh_variable_not_found",
            "too_many_mesh_values",
        ]],
        warnings: &["mesh_source_unreadable"],
        targets: || vec![target(None, &["raster"], &[], Some("not_a_mesh"), None)],
    },
    // Meshin alanı ve hattı (docs/adr/0645 §7).
    Entry {
        id: "cad.meshes.region",
        refusals: &[&[
            "invalid_uid",
            "entity_not_found",
            "not_a_mesh",
            "invalid_mesh_area",
            "mesh_variable_not_found",
            "too_many_mesh_values",
        ]],
        warnings: &["mesh_source_unreadable"],
        targets: || vec![target(None, &["raster"], &[], Some("not_a_mesh"), None)],
    },
    Entry {
        id: "cad.meshes.profile",
        refusals: &[&[
            "invalid_uid",
            "entity_not_found",
            "not_a_mesh",
            "invalid_mesh_path",
            "too_many_stations",
            "mesh_variable_not_found",
            "too_many_mesh_values",
        ]],
        warnings: &["mesh_source_unreadable"],
        targets: || vec![target(None, &["raster"], &[], Some("not_a_mesh"), None)],
    },
    // STAC keşfi ve veri hazırlama (docs/adr/0528).
    Entry {
        id: "cad.stac.search",
        refusals: &[
            &[
                "invalid_stac_endpoint",
                "invalid_stac_bbox",
                "invalid_stac_datetime",
                "invalid_stac_cloud_cover",
            ],
            &[
                "invalid_stac_limit",
                "invalid_stac_collection",
                "stac_next_foreign",
            ],
            &[
                "stac_unavailable",
                "stac_request_failed",
                "stac_response_unreadable",
            ],
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.stac.prepare",
        refusals: &[
            &[
                "invalid_stac_item",
                "stac_assets_empty",
                "stac_too_many_assets",
                "stac_asset_not_found",
            ],
            &[
                "stac_href_not_http",
                "stac_no_grid",
                "stac_unsupported_sample",
                "stac_multiband",
                "stac_qa_unknown",
                "stac_qa_is_data",
            ],
            &["crs_mismatch", "stac_local_missing", "invalid_group_name"],
            &[
                "invalid_file_path",
                "file_not_found",
                "import_unreadable",
                "crs_unreached",
            ],
            REVISION,
        ],
        warnings: &[
            "stac_signed_href_stripped",
            "stac_grid_differs",
            "stac_scale_differs",
            "stac_item_incomplete",
            "stac_qa_grid_differs",
            "stac_requester_pays",
            "crs_unofficial",
        ],
        targets: none,
    },
    // Raster oturt'un kaydı (docs/adr/0518).
    Entry {
        id: "cad.rasters.georef",
        refusals: &[
            &[
                "invalid_uid",
                "entity_not_found",
                "not_a_raster",
                "layer_locked",
                "invalid_control_point",
            ],
            &[
                "unknown_srid",
                // docs/adr/0594: the targets' system as a definition.
                "invalid_input",
                "crs_text_unreadable",
                "target_not_converted",
                "too_few_control_points",
                "control_points_duplicate",
            ],
            &[
                "control_points_collinear",
                "georef_singular",
                "invalid_pixel_size",
                "georef_grid",
            ],
            &[
                "invalid_file_path",
                "file_not_found",
                "import_unreadable",
                "unknown_asset",
                "georef_output_mismatch",
            ],
            REVISION,
        ],
        warnings: &["no_check_points", "source_not_read"],
        targets: || vec![target(None, &["raster"], &[], Some("not_a_raster"), None)],
    },
    // Resmin kaynağı (docs/adr/0350 §1).
    Entry {
        id: "cad.images.edit",
        refusals: &[
            &[
                "no_entities",
                "invalid_uid",
                "entity_not_found",
                "not_an_image",
                "layer_locked",
                "nothing_to_embed",
            ],
            &["invalid_file_path", "file_not_found", "import_unreadable"],
            REVISION,
        ],
        warnings: &["layer_locked"],
        targets: || vec![target(None, &["image"], &[], Some("not_an_image"), None)],
    },
    // Kırp (docs/adr/0351 §1).
    Entry {
        id: "cad.inserts.clip",
        refusals: &[
            &[
                "no_entities",
                "invalid_uid",
                "entity_not_found",
                "not_an_insert",
                "layer_locked",
                "invalid_clipping",
                "not_clipped",
            ],
            REVISION,
        ],
        warnings: &["layer_locked", "clip_empty"],
        targets: || vec![target(None, &["insert"], &[], Some("not_an_insert"), None)],
    },
    // Bağlı dosyaların durumu (docs/adr/0349 §2).
    Entry {
        id: "cad.assets.check",
        refusals: &[],
        warnings: &[],
        targets: none,
    },
    // Pafta seti (docs/adr/0347 §1).
    Entry {
        id: "cad.sheets.set",
        refusals: &[&["entity_not_found", "book_unreadable"]],
        warnings: &[],
        targets: none,
    },
    // Proje gezgini (docs/adr/0778).
    Entry {
        id: "cad.navigator.read",
        refusals: &[&["entity_not_found", "book_unreadable"]],
        warnings: &[],
        targets: none,
    },
    // Work templates (docs/adr/0346 §1).
    Entry {
        id: "cad.templates.apply",
        refusals: &[
            &[
                "unknown_template",
                "invalid_template_unit",
                "template_settings_conflict",
                "unit_needs_local",
                "book_unreadable",
            ],
            SHEET_CORE,
            REVISION,
        ],
        warnings: &["template_unit_kept"],
        targets: none,
    },
    // Data profiles (docs/adr/0541).
    Entry {
        id: "cad.profiles.apply",
        refusals: &[
            &[
                "unknown_profile",
                "invalid_profile",
                "unknown_mapping",
                "invalid_profile_mapping",
            ],
            REVISION,
        ],
        warnings: &[
            "profile_field_differs",
            "profile_domain_differs",
            "profile_layer_skipped",
        ],
        targets: none,
    },
    Entry {
        id: "cad.profiles.check",
        refusals: &[&[
            "unknown_profile",
            "invalid_profile",
            "unknown_mapping",
            "invalid_profile_mapping",
        ]],
        warnings: &[],
        targets: none,
    },
    // An institution's own profile (docs/adr/0558).
    Entry {
        id: "cad.profiles.import",
        refusals: &[
            &[
                "profile_source_missing",
                "invalid_file_path",
                "file_not_found",
            ],
            &[
                "invalid_profile",
                "unknown_profile_format",
                "unsupported_profile_version",
                "invalid_profile_mapping",
            ],
            &[
                "profile_name_taken",
                "profile_downgrade",
                "file_exists",
                "profile_exists",
                "file_unwritable",
            ],
            REVISION,
        ],
        warnings: &["mapping_unchecked", "profile_replaced", "profile_shadowed"],
        targets: none,
    },
    Entry {
        id: "cad.profiles.list",
        refusals: &[],
        warnings: &[],
        targets: none,
    },
    // The drawing's own profiles and mappings (docs/adr/0735).
    Entry {
        id: "cad.profiles.remove",
        refusals: &[&["unknown_profile", "unknown_mapping"], REVISION],
        warnings: &["profile_still_named"],
        targets: none,
    },
    Entry {
        id: "cad.profiles.migrate",
        refusals: &[
            &[
                "unknown_profile",
                "invalid_profile",
                "invalid_migration",
                "migration_gap",
                "unknown_mapping",
                "invalid_profile_mapping",
            ],
            &["layer_refused", "domain_exists"],
            REVISION,
        ],
        warnings: &[
            "migration_step_skipped",
            "profile_layer_skipped",
            "values_unfit",
        ],
        targets: none,
    },
    // Kurumdaki alan adları (plan/map-cad.md MAP-17, docs/adr/5180): read-only.
    Entry {
        id: "cad.profiles.names",
        refusals: &[&["unknown_profile", "invalid_profile", "unknown_mapping", "invalid_profile_mapping"]],
        warnings: &[],
        targets: none,
    },
    // Katalog geçiş raporu (plan/map-cad.md MAP-04, docs/adr/4940): read-only.
    Entry {
        id: "cad.profiles.compare",
        refusals: &[&["unknown_profile", "invalid_profile"]],
        warnings: &[],
        targets: none,
    },
    // Kataloğa taşı (plan/map-cad.md MAP-04, docs/adr/5340).
    Entry {
        id: "cad.profiles.carry",
        refusals: &[REVISION, &["unknown_profile", "invalid_profile", "layer_locked", "layer_refused", "nothing_to_set"]],
        warnings: &["catalog_objects_left", "profile_field_differs", "profile_domain_differs"],
        targets: none,
    },
    // Duvar birleşimleri (docs/adr/0340 §2): read-only.
    Entry {
        id: "cad.walls.joins",
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_wall"]],
        warnings: &[],
        targets: || vec![target(None, &["wall"], &[], Some("not_a_wall"), None)],
    },
];
