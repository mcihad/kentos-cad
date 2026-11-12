//! What the commands of mapping, cadastre, survey and road stakeout take and how they answer (docs/adr/0253 §7): one of the parts the table
//! was cut into, in its order (docs/adr/8054 §4).

use super::*;

pub(super) const ENTRIES: &[Entry] = &[
    // Eğimle kot ver (plan/map-cad.md MAP-06, docs/adr/1290).
    Entry {
        id: "cad.entities.grade",
        refusals: &[
            REVISION,
            &[
                "invalid_uid",
                "entity_not_found",
                "not_a_path",
                "wrong_place",
                "invalid_input",
                "elevation_missing",
                "no_intersection",
                "layer_locked",
            ],
        ],
        warnings: &[],
        targets: none,
    },
    // Referanstan kot ver (plan/map-cad.md MAP-06, docs/adr/3780).
    Entry {
        id: "cad.entities.raise",
        refusals: &[
            REVISION,
            &["invalid_uid", "entity_not_found", "not_a_path", "wrong_place", "invalid_input", "elevation_missing", "layer_locked"],
        ],
        warnings: &[],
        targets: none,
    },
    // Yüzeyden kot ver (plan/map-cad.md MAP-06, docs/adr/4180; the range's wrong_place, docs/adr/4580).
    Entry {
        id: "cad.entities.drape",
        refusals: &[
            REVISION,
            &[
                "not_a_road_project",
                "unknown_terrain",
                "invalid_input",
                "invalid_uid",
                "entity_not_found",
                "not_a_path",
                "wrong_place",
                "terrain_unbuildable",
                "off_surface",
                "layer_locked",
            ],
        ],
        warnings: &[],
        targets: none,
    },
    // Tarama uyumu denetimi (plan/map-cad.md MAP-13, docs/adr/1500).
    Entry {
        id: "cad.pointclouds.overlap",
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_pointcloud", "invalid_input", "layer_not_found", "too_many_cloud_points"]],
        warnings: &["cloud_source_unreadable"],
        targets: none,
    },
    // Bulutu konumla (plan/map-cad.md MAP-13, docs/adr/5100).
    Entry {
        id: "cad.pointclouds.place",
        refusals: &[
            &["invalid_uid", "entity_not_found", "not_a_pointcloud", "layer_locked", "not_finite", "invalid_input"],
            REVISION,
        ],
        warnings: &[],
        targets: || vec![target(None, &["pointcloud"], &[], Some("not_a_pointcloud"), None)],
    },
    // Tarama kaydı denetimi (plan/map-cad.md MAP-13, docs/adr/1800).
    // Tarama ağı dengelemesi (plan/map-cad.md MAP-13, docs/adr/4720); the registration's own refusals and warnings.
    Entry {
        id: "cad.pointclouds.network",
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_pointcloud", "invalid_input"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.pointclouds.register",
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_pointcloud", "invalid_input", "layer_not_found", "too_many_cloud_points"]],
        warnings: &["cloud_source_unreadable"],
        targets: none,
    },
    // Ortofoto doğruluk denetimi (plan/map-cad.md MAP-12, docs/adr/1480).
    Entry {
        id: "cad.orthophoto.check",
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_raster", "crs_mismatch", "layer_not_found", "invalid_input"]],
        warnings: &[],
        targets: none,
    },
    // İki ortofotonun karşılaştırılması (plan/map-cad.md MAP-12, docs/adr/1780).
    Entry {
        id: "cad.orthophoto.compare",
        // The seam's image file (docs/adr/2460): file_not_found.
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_raster", "crs_mismatch", "layer_not_found", "invalid_input", "file_not_found"]],
        warnings: &[],
        targets: none,
    },
    // Mozaik birleşme çizgisi (plan/map-cad.md MAP-12, docs/adr/4700).
    Entry {
        id: "cad.orthophoto.seamlines",
        // The radiometric seam's image files: invalid_file_path, file_not_found, import_unreadable.
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_raster", "crs_mismatch", "invalid_input", "layer_not_found", "layer_locked", "invalid_revision", "revision_conflict", "file_not_found"]],
        warnings: &[],
        targets: none,
    },
    // Birleşme çizgisi ağı ve ayak izi (plan/map-cad.md MAP-12, docs/adr/5260).
    Entry {
        id: "cad.orthophoto.network",
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_raster", "crs_mismatch", "invalid_input", "layer_not_found", "layer_locked", "invalid_revision", "revision_conflict"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.orthophoto.footprint",
        // The image's file: invalid_file_path, file_not_found, import_unreadable.
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_raster", "crs_mismatch", "invalid_input", "layer_not_found", "layer_locked", "invalid_revision", "revision_conflict", "file_not_found"]],
        warnings: &[],
        targets: none,
    },
    // Düzenleme dağıtım taslağı (plan/map-cad.md MAP-11, docs/adr/1460).
    Entry {
        id: "cad.readjustment.distribute",
        refusals: &[&["invalid_input", "area_mismatch", "duplicate_holder", "shares_not_whole", "needs_rule"]],
        // docs/adr/5080: an answer by value is no rule of the regulation.
        warnings: &["value_basis_not_a_rule"],
        targets: none,
    },
    // Düzenleme kapsamı (plan/map-cad.md MAP-11, docs/adr/2060).
    Entry {
        id: "cad.readjustment.scope",
        refusals: &[&["invalid_input", "invalid_uid", "entity_not_found", "not_an_area", "not_a_parcel"]],
        warnings: &[],
        targets: none,
    },
    // Ayırma çapı paftaları (plan/map-cad.md MAP-11, docs/adr/2680).
    Entry {
        id: "cad.readjustment.sheets",
        refusals: &[
            SHEET_CORE,
            REVISION,
            &["invalid_input", "invalid_uid", "entity_not_found", "not_an_area", "not_a_parcel"],
            &["no_partial_parcels", "no_identity", "duplicate_parcel", "layer_not_found", "invalid_group_name"],
            &["unknown_template", "unknown_variable", "invalid_options", "not_fixed_map", "separation_sheet_exists"],
            &["book_unreadable", "bad_asset"],
        ],
        // docs/adr/5080: a number outside that touches something wherever it goes.
        warnings: &["number_crowded"],
        targets: none,
    },
    // Senaryo ölçüleri (plan/map-cad.md MAP-10, docs/adr/1440; “Alternatif etkisi” until 2026-11-07).
    Entry {
        id: "cad.scenarios.impact",
        // invalid_input: the asked measures' values (docs/adr/2040); the slope's surface (docs/adr/4260).
        refusals: &[&[
            "invalid_input",
            "scenario_not_found",
            "nothing_to_compare",
            "not_a_road_project",
            "unknown_terrain",
            "terrain_unbuildable",
            // Docs/adr/5060: the access along a network.
            "unknown_network",
            "invalid_network",
            "tools_unavailable",
        ]],
        warnings: &[],
        targets: none,
    },
    // Ölçüleri çizime yaz (plan/map-cad.md MAP-10, docs/adr/5060).
    Entry {
        id: "cad.scenarios.record",
        // The comparison's own, then the table's layer and corner, the revision and the write's.
        refusals: &[&[
            "invalid_input",
            "scenario_not_found",
            "nothing_to_compare",
            "not_a_road_project",
            "unknown_terrain",
            "terrain_unbuildable",
            "unknown_network",
            "invalid_network",
            "tools_unavailable",
            "invalid_revision",
            "revision_conflict",
            "layer_locked",
        ]],
        warnings: &[],
        targets: none,
    },
    // İmar planı denetimi (plan/map-cad.md MAP-09, docs/adr/1420).
    Entry {
        id: "cad.zoning.check",
        refusals: &[&["invalid_input"]],
        warnings: &[],
        targets: none,
    },
    // Sembolü karardan ver (plan/map-cad.md MAP-09, docs/adr/4240).
    Entry {
        id: "cad.zoning.symbolize",
        refusals: &[
            REVISION,
            &[
                "invalid_input",
                "unknown_plan_symbol",
                "layer_not_found",
                "invalid_uid",
                "entity_not_found",
                "not_a_decision",
                "layer_locked",
            ],
        ],
        warnings: &["kept_symbol_ignored"],
        targets: none,
    },
    // Plan değişikliği taslağı (plan/map-cad.md MAP-09, docs/adr/5040).
    Entry {
        id: "cad.zoning.draft",
        refusals: &[
            REVISION,
            &[
                "invalid_input",
                "unknown_draft",
                "layer_not_found",
                "unknown_plan",
                "invalid_uid",
                "entity_not_found",
                "not_a_decision",
            ],
        ],
        warnings: &[],
        targets: none,
    },
    // Saha paketi (plan/map-cad.md MAP-19, docs/adr/1920).
    Entry {
        id: "cad.field.package",
        refusals: &[&[
            "invalid_field_package",
            "layer_not_found",
            "not_a_road_project",
            "unknown_terrain",
            "terrain_unbuildable",
            "invalid_file_path",
            "file_exists",
            "file_unwritable",
        ]],
        warnings: &[],
        targets: none,
    },
    // Uygulama kontrolü (plan/map-cad.md MAP-19, docs/adr/1700).
    Entry {
        id: "cad.asbuilt.check",
        refusals: &[&[
            "invalid_asbuilt_check",
            // The field package's (docs/adr/2420).
            "invalid_file_path",
            "file_not_found",
            "layer_not_found",
            "not_a_road_project",
            "unknown_terrain",
            "terrain_unbuildable",
        ]],
        warnings: &[],
        targets: none,
    },
    // İzleme incelemesi (plan/map-cad.md MAP-19, docs/adr/5220).
    Entry {
        id: "cad.asbuilt.review",
        refusals: &[REVISION, &["invalid_input", "unknown_monitoring_review", "invalid_asbuilt_check", "layer_not_found"]],
        warnings: &[],
        targets: none,
    },
    // Boru hattı denetimi (plan/map-cad.md MAP-16, docs/adr/1640).
    Entry {
        id: "cad.pipes.check",
        refusals: &[&[
            "invalid_pipe_check",
            "unknown_network",
            "not_a_pipe_network",
            "not_a_road_project",
            "unknown_terrain",
            "terrain_unbuildable",
            "invalid_network",
            "tools_unavailable",
        ]],
        warnings: &[],
        targets: none,
    },
    // Hat profili (plan/map-cad.md MAP-16, docs/adr/1860).
    Entry {
        id: "cad.pipes.profile",
        refusals: &[
            &[
                "invalid_pipe_check",
                "unknown_network",
                "not_a_pipe_network",
                "not_a_road_project",
                "unknown_terrain",
                "terrain_unbuildable",
                "invalid_network",
                "tools_unavailable",
                "invalid_pipe_run",
            ],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    // Parsel alan denetimi (plan/map-cad.md MAP-08, docs/adr/1400).
    Entry {
        id: "cad.parcels.check",
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_parcel"]],
        warnings: &[],
        targets: none,
    },
    // Sınır grafiği (plan/map-cad.md MAP-08, docs/adr/2000).
    Entry {
        id: "cad.parcels.boundaries",
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_parcel", "invalid_input"]],
        warnings: &[],
        targets: none,
    },
    // Parsel kaydı (plan/map-cad.md MAP-08, docs/adr/4220).
    Entry {
        id: "cad.parcels.record",
        refusals: &[
            REVISION,
            &[
                "invalid_input",
                "document_name_taken",
                "unknown_parcel_record",
                "parcel_closed",
                "invalid_uid",
                "entity_not_found",
                "not_a_parcel",
                "no_parcel_identity",
                "parcel_recorded",
                "parcel_identity_taken",
            ],
        ],
        warnings: &[],
        targets: none,
    },
    // Belgenin onayı (plan/map-cad.md MAP-08, docs/adr/5020).
    Entry {
        id: "cad.parcels.approve",
        refusals: &[
            REVISION,
            &[
                "invalid_input",
                "unknown_parcel_draft",
                "unknown_parcel_document",
                "document_approved",
                "parcel_closed",
                "parcel_recorded",
                "parcel_identity_taken",
            ],
        ],
        warnings: &[],
        targets: none,
    },
    // Taslağı geri çek (plan/map-cad.md MAP-08, docs/adr/5020).
    Entry {
        id: "cad.parcels.withdraw",
        refusals: &[REVISION, &["unknown_parcel_draft"]],
        warnings: &[],
        targets: none,
    },
    // Parsel kayıtları (plan/map-cad.md MAP-08, docs/adr/4220): read only, no refusal.
    Entry {
        id: "cad.parcels.records",
        refusals: &[],
        warnings: &[],
        targets: none,
    },
    // Topolojik temizlik, taşınma raporuyla (plan/map-cad.md MAP-07, docs/adr/1262).
    Entry {
        id: "cad.topology.clean",
        refusals: &[
            REVISION,
            &["invalid_input", "invalid_uid", "entity_not_found", "clean_beyond_acceptance"],
        ],
        warnings: &[],
        targets: none,
    },
    // GNSS baz ağı dengelemesi (plan/map-cad.md MAP-05, docs/adr/1380).
    Entry {
        id: "cad.survey.baselines",
        refusals: &[&[
            "invalid_input",
            "no_baselines",
            "duplicate_station",
            "unknown_station",
            "invalid_covariance",
            "datum_missing",
            "network_too_large",
            "network_disconnected",
        ]],
        warnings: &[],
        targets: none,
    },
    // Kodla çiz (plan/map-cad.md MAP-04, docs/adr/1360).
    Entry {
        id: "map.features.code",
        refusals: &[
            REVISION,
            &[
                "unknown_profile",
                "invalid_profile",
                "invalid_input",
                "invalid_uid",
                "entity_not_found",
                "not_a_point",
                "no_coded_points",
                "class_layer_missing",
                "layer_locked",
            ],
        ],
        warnings: &["unknown_code", "control_unknown", "string_too_short", "area_not_closed"],
        targets: none,
    },
    // The scales between the ground, the ellipsoid and the map (plan/map-cad.md MAP-02, docs/adr/1231).
    Entry {
        id: "cad.crs.factors",
        refusals: &[&["invalid_input", "no_system", "no_point_scale", "heights_unknown", "geoid_required", "geoid_missing"]],
        warnings: &["point_beyond_system", "points_outside"],
        targets: none,
    },
    // KentOS Harita's GNSS campaign archive (plan/map-cad.md MAP-03, docs/adr/1340).
    Entry {
        id: "map.survey.query",
        refusals: &[&["not_a_map_project", "invalid_uid", "entity_not_found"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "map.survey.edit",
        refusals: &[
            REVISION,
            &[
                "not_a_map_project",
                "unknown_survey_record",
                "unknown_work_item",
                "invalid_survey_text",
                "invalid_survey_value",
                "invalid_file_path",
                "file_not_found",
                "unreadable_gnss_file",
                "nothing_to_set",
                "survey_in_use",
                "unknown_frame",
                "no_observations",
                "no_ellipsoidal_height",
                "solution_excluded",
                "mark_mismatch",
                "method_count",
                "sigma_missing",
                "solutions_differ",
                "antenna_unknown",
                "antenna_eccentric",
                "already_accepted",
                "layer_not_found",
                "layer_locked",
                "no_entities",
                "invalid_uid",
                "entity_not_found",
                "survey_full",
                "epoch_unsupported",
                "velocity_missing",
                "geoid_missing",
                "no_system",
                "no_baselines",
                "datum_missing",
                "known_differ",
                "unknown_rule_class",
                "duplicate_station",
                "invalid_covariance",
                "network_too_large",
                "network_disconnected",
                "network_blunder",
                "network_test_failed",
                "session_blocks_missing",
                "session_covariance_invalid",
                "no_velocity_grid",
            ],
        ],
        warnings: &[
            "rinex_differs",
            "gnss_lines_skipped",
            "gvx_correlations_ignored",
            "gvx_session_incomplete",
            "baseline_text_stations",
            "network_rules_failed",
            "network_unchecked",
            "session_correlations_unused",
        ],
        targets: none,
    },
    // Candidates of map assets reconciled (plan/map-cad.md MAP-04, docs/adr/1760).
    Entry {
        id: "map.features.reconcile",
        refusals: &[
            REVISION,
            &[
                "unknown_profile",
                "invalid_profile",
                "invalid_input",
                "layer_not_found",
                "invalid_uid",
                "entity_not_found",
                "class_layer_missing",
                "layer_locked",
            ],
        ],
        warnings: &[],
        targets: none,
    },
    // The GNSS network adjusted from the archive (plan/map-cad.md MAP-03, MAP-05; docs/adr/1740).
    Entry {
        id: "map.survey.network",
        refusals: &[&[
            "not_a_map_project",
            "unknown_survey_record",
            "invalid_survey_value",
            "unknown_rule_class",
            "invalid_epoch",
            "no_baselines",
            "solutions_differ",
            "datum_missing",
            "known_differ",
            "unknown_frame",
            "epoch_unsupported",
            "duplicate_station",
            "invalid_covariance",
            "network_too_large",
            "network_disconnected",
            "session_blocks_missing",
            "session_covariance_invalid",
            "no_velocity_grid",
        ]],
        warnings: &["network_rules_failed", "network_unchecked", "session_correlations_unused"],
        targets: none,
    },
    // KentOS Harita's work structure (plan/map-cad.md MAP-01, docs/adr/1200).
    Entry {
        id: "map.work.query",
        refusals: &[&["not_a_map_project", "invalid_file_path", "invalid_uid", "entity_not_found"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "map.work.edit",
        refusals: &[
            REVISION,
            &[
                "not_a_map_project",
                "unknown_work_item",
                "unknown_derivation",
                "invalid_work_text",
                "layer_not_found",
                "layer_in_work_item",
                "nothing_to_set",
                "invalid_file_path",
                "file_not_found",
                "work_item_no_file",
                "no_entities",
                "invalid_uid",
                "entity_not_found",
                "already_derived",
                "own_content",
                "not_derived",
                "approval_stale",
                "unknown_profile",
                "invalid_profile",
                "unknown_profile_class",
                "approval_profile",
                "layer_refused",
                "layer_locked",
                "work_full",
            ],
        ],
        warnings: &[
            "source_content_changed",
            "link_not_followed",
            "profile_field_differs",
            "profile_domain_differs",
            "profile_layer_skipped",
        ],
        targets: none,
    },
    // An approved item's objects put back as approved (plan/map-cad.md MAP-01, docs/adr/4080).
    Entry {
        id: "map.work.restore",
        refusals: &[
            REVISION,
            &[
                "not_a_map_project",
                "unknown_work_item",
                "work_not_approved",
                "no_approved_values",
                "invalid_uid",
                "not_approved_object",
                "nothing_to_restore",
                "layer_not_found",
                "layer_locked",
                "layer_geometry_mismatch",
            ],
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    // KentOS Yol's LandXML export (plan/road.md Y10.2, docs/adr/0908 §6).
    Entry {
        id: "file.export.landxml",
        refusals: &[&[
            "not_a_road_project",
            "invalid_file_path",
            "file_exists",
            "unknown_terrain",
            "unknown_alignment",
            "invalid_landxml",
            "field_heights_unknown",
            "unproven_heights",
        ]],
        warnings: &["export_losses"],
        targets: none,
    },
    Entry {
        id: "road.alignments.import",
        refusals: &[
            &["not_a_road_project", "file_not_found", "import_unreadable", "landxml_no_alignment", "invalid_alignment"],
            REVISION,
        ],
        warnings: &["landxml_left"],
        targets: none,
    },
    Entry {
        id: "file.export.ifc",
        refusals: &[&["not_a_road_project", "invalid_file_path", "file_exists", "unknown_alignment", "invalid_ifc"]],
        warnings: &["export_losses"],
        targets: none,
    },
    // KentOS Mimari's building as IFC 4.3 (plan/architecture-advanced.md M47, docs/adr/8047).
    Entry {
        id: "cad.arch.ifc",
        refusals: &[&["invalid_file_path", "file_exists", "invalid_ifc"]],
        warnings: &["export_losses"],
        targets: none,
    },
    // Planı tanı: an exploded plan read as walls and openings (plan/architecture-advanced.md M48, docs/adr/8048).
    Entry {
        id: "cad.arch.recognize",
        refusals: &[
            OBJECTS,
            &[
                "not_finite",
                "recognize_option",
                "plan_unrecognized",
                "level_not_found",
                "invalid_wall",
                "opening_off_wall",
            ],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_locked", "plan_unrecognized", "layer_hidden"],
        targets: none,
    },
    // KentOS Yol's 3D scene (plan/road.md Y12.3, docs/adr/0910 §3).
    Entry {
        id: "file.export.gltf",
        refusals: &[&[
            "not_a_road_project",
            "invalid_file_path",
            "file_exists",
            "unknown_terrain",
            "unknown_corridor",
            "invalid_gltf",
        ]],
        warnings: &["export_losses"],
        targets: none,
    },
    // KentOS Yol's stakeout table (plan/road.md Y10, docs/adr/0908).
    Entry {
        id: "road.stakeout.query",
        refusals: &[&["not_a_road_project", "unknown_corridor", "invalid_stakeout", "stakeout_no_ground"]],
        warnings: &[
            "corridor_stale",
            "corridor_unsolved",
            "corridor_surface_gone",
            "stakeout_unsolved",
        ],
        targets: none,
    },
    Entry {
        id: "road.stakeout.send",
        refusals: &[&[
            "not_a_road_project",
            "unknown_corridor",
            "invalid_stakeout",
            "invalid_file_path",
            "file_exists",
            "invalid_stakeout_send",
            "field_heights_unknown",
        ]],
        warnings: &[
            "corridor_stale",
            "corridor_unsolved",
            "stakeout_unsolved",
            "stakeout_code_dropped",
            "export_losses",
        ],
        targets: none,
    },
    Entry {
        id: "road.sheets.plan",
        refusals: &[
            &["not_a_road_project", "unknown_alignment", "unknown_road_profile", "invalid_road_sheets", "unknown_template"],
            &["invalid_options", "not_fixed_map", "bad_drawing_info"],
            &["unknown_corridor", "invalid_label", "invalid_road_corridor", "no_terrain", "unknown_haul", "invalid_haul", "unknown_volume"],
            LAYER,
            REVISION,
        ],
        warnings: &["road_sheet_outside", "layer_hidden", "profile_stale"],
        targets: none,
    },
    Entry {
        id: "road.sheets.sections",
        refusals: &[
            &["not_a_road_project", "unknown_section_set", "invalid_label", "invalid_places", "no_terrain"],
            &["invalid_road_sheets", "unknown_template", "invalid_options", "not_fixed_map", "bad_drawing_info"],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden", "section_set_stale", "section_open", "section_ground_missing"],
        targets: none,
    },
    Entry {
        id: "road.alignments.fit",
        refusals: &[&["not_a_road_project", "entity_not_found", "invalid_alignment_fit"], REVISION],
        warnings: &["alignment_fit_deviation"],
        targets: none,
    },
    Entry {
        id: "road.profiles.fit",
        refusals: &[
            &["not_a_road_project", "unknown_alignment", "unknown_terrain", "entity_not_found", "invalid_profile_fit"],
            REVISION,
        ],
        warnings: &["profile_fit_deviation"],
        targets: none,
    },
    Entry {
        id: "road.volumes.history",
        refusals: &[&["not_a_road_project", "unknown_volume"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.alignments.impact",
        refusals: &[&["not_a_road_project", "unknown_alignment", "invalid_alignment", "invalid_alignment_impact"]],
        warnings: &["alignment_impact_unresolved", "alignment_impact_approved", "alignment_reverse_stale"],
        targets: none,
    },
    Entry {
        id: "road.profiles.impact",
        refusals: &[&["not_a_road_project", "unknown_road_profile", "invalid_road_profile", "profile_curves_overlap", "invalid_profile_impact"]],
        warnings: &["profile_impact_approved"],
        targets: none,
    },
    Entry {
        id: "road.volumes.states",
        refusals: &[&["not_a_road_project", "unknown_volume", "invalid_volume"]],
        warnings: &["volume_factor_physics", "volume_unfactored", "corridor_stale", "volume_gap", "volume_uncovered"],
        targets: none,
    },
    Entry {
        id: "road.volumes.ledger",
        refusals: &[&["invalid_volume_ledger", "not_a_road_project", "unknown_volume", "invalid_volume"]],
        warnings: &["corridor_stale", "volume_gap", "volume_uncovered", "volume_unfactored", "volume_changed"],
        targets: none,
    },
    Entry {
        id: "road.volumes.uncertainty",
        refusals: &[&["invalid_volume_uncertainty", "not_a_road_project", "unknown_volume", "invalid_volume"]],
        warnings: &[
            "volume_uncertainty_near_zero",
            "volume_uncertainty_over",
            "corridor_stale",
            "volume_uncovered",
            "volume_unfactored",
        ],
        targets: none,
    },
    Entry {
        id: "road.volumes.progress",
        refusals: &[&[
            "invalid_volume_progress",
            "not_a_road_project",
            "unknown_volume",
            "unknown_terrain",
            "unproven_heights",
            "invalid_volume",
        ]],
        warnings: &["heights_assumed", "volume_progress_uncovered", "volume_progress_backward"],
        targets: none,
    },
    Entry {
        id: "road.volumes.compare",
        refusals: &[&["invalid_volume_compare", "not_a_road_project", "unknown_volume"]],
        warnings: &["volume_compare_broken"],
        targets: none,
    },
    Entry {
        id: "road.volumes.approve",
        refusals: &[&["not_a_road_project", "unknown_volume", "volume_not_approved", "invalid_volume"], REVISION],
        warnings: &[],
        targets: none,
    },
];
