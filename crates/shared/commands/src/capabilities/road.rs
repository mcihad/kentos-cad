//! What the commands of road design take and how they answer (docs/adr/0253 §7): one of the parts the table
//! was cut into, in its order (docs/adr/8054 §4).

use super::*;

pub(super) const ENTRIES: &[Entry] = &[
    // KentOS Yol's terrain surfaces (plan/road.md Y2.3).
    Entry {
        id: "road.terrains.create",
        refusals: &[
            &["not_a_road_project", "invalid_terrain"],
            REVISION,
            &["terrain_unbuildable"],
        ],
        warnings: &["terrain_source_left"],
        targets: none,
    },
    Entry {
        id: "road.terrains.edit",
        refusals: &[
            &["not_a_road_project", "no_terrain", "no_id", "invalid_terrain"],
            REVISION,
            &["unknown_terrain", "terrain_unbuildable"],
        ],
        warnings: &["terrain_source_left"],
        targets: none,
    },
    // Sıfır hattı (plan/map-cad.md MAP-15, docs/adr/1620).
    Entry {
        id: "road.terrains.zeroline",
        refusals: &[
            &["not_a_road_project"],
            LAYER,
            REVISION,
            &["unknown_terrain", "invalid_analysis", "terrain_unbuildable", "no_zero_line"],
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    // Kazı ve dolgu bölgeleri (plan/map-cad.md MAP-15, docs/adr/1840).
    Entry {
        id: "road.terrains.cutfill",
        refusals: &[
            &["not_a_road_project"],
            LAYER,
            REVISION,
            // Docs/adr/4760: the sections' axis and a material's boundary missing.
            &["invalid_analysis", "unknown_terrain", "entity_not_found", "terrain_unbuildable", "no_cut_fill"],
        ],
        warnings: &["layer_hidden", "compare_uncovered"],
        targets: none,
    },
    Entry {
        id: "road.terrains.contours",
        refusals: &[
            &["road_drawings_full", "not_a_road_project", "invalid_contours"],
            LAYER,
            REVISION,
            &["unknown_terrain", "terrain_unbuildable", "no_contours"],
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    // Pafta eş yükselti eğrileri (plan/map-cad.md MAP-14, docs/adr/1820).
    Entry {
        id: "road.terrains.sheetcontours",
        refusals: &[
            &["not_a_road_project", "invalid_contours"],
            LAYER,
            REVISION,
            &["unknown_terrain", "terrain_unbuildable", "no_contours"],
        ],
        warnings: &["layer_hidden", "section_check_exceeded"],
        targets: none,
    },
    // Kesit kontrolü (plan/map-cad.md MAP-14, docs/adr/5120): read only.
    Entry {
        id: "road.terrains.sectioncheck",
        refusals: &[&[
            "invalid_uid",
            "entity_not_found",
            "not_a_height_point",
            "invalid_input",
            "layer_not_found",
            "invalid_contours",
        ]],
        warnings: &["section_check_exceeded", "section_spacing_exceeded"],
        targets: none,
    },
    Entry {
        id: "file.import.landxml",
        refusals: &[
            &["not_a_road_project", "file_not_found", "import_unreadable", "landxml_no_surface"],
            LAYER,
            &["invalid_terrain"],
            REVISION,
            &["terrain_unbuildable"],
        ],
        warnings: &["landxml_left", "layer_hidden"],
        targets: none,
    },
    Entry {
        id: "road.terrains.query",
        refusals: &[&["not_a_road_project", "unknown_terrain", "no_terrain", "invalid_places"]],
        warnings: &[],
        targets: none,
    },
    // KentOS Yol's horizontal alignments (plan/road.md Y3, docs/adr/0901).
    Entry {
        id: "road.alignments.create",
        refusals: &[
            &[
                "not_a_road_project",
                "invalid_alignment",
                "alignment_spirals_long",
                "alignment_curves_overlap",
                "invalid_stationing",
                "offset_cusp",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.alignments.edit",
        refusals: &[
            &[
                "not_a_road_project",
                "unknown_alignment",
                "invalid_alignment",
                "alignment_spirals_long",
                "alignment_curves_overlap",
                "invalid_stationing",
                "offset_cusp",
                "alignment_in_use",
            ],
            REVISION,
        ],
        warnings: &["equations_dropped"],
        targets: none,
    },
    Entry {
        id: "road.alignments.label",
        refusals: &[
            &["road_drawings_full", "not_a_road_project", "unknown_alignment", "invalid_label"],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "road.alignments.query",
        refusals: &[&[
            "not_a_road_project",
            "unknown_alignment",
            "invalid_places",
            "station_outside",
            "station_ambiguous",
        ]],
        warnings: &[],
        targets: none,
    },
    // KentOS Yol's vertical alignments (plan/road.md Y4, docs/adr/0902).
    Entry {
        id: "road.profiles.create",
        refusals: &[
            &[
                "not_a_road_project",
                "unknown_alignment",
                "unknown_terrain",
                "invalid_road_profile",
                "profile_curves_overlap",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.profiles.edit",
        refusals: &[
            &[
                "not_a_road_project",
                "unknown_road_profile",
                "unknown_terrain",
                "invalid_road_profile",
                "profile_curves_overlap",
                "profile_in_use",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.profiles.draw",
        refusals: &[
            &["road_drawings_full", "not_a_road_project", "unknown_road_profile", "invalid_label", "unknown_superelevation"],
            &["unknown_corridor", "invalid_road_corridor", "no_terrain", "unknown_haul", "invalid_haul", "unknown_volume"],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden", "profile_stale", "profile_surface_gone"],
        targets: none,
    },
    Entry {
        id: "road.profiles.query",
        refusals: &[&[
            "not_a_road_project",
            "unknown_road_profile",
            "invalid_places",
            "station_outside",
            "no_terrain",
        ]],
        warnings: &["profile_stale", "profile_surface_gone"],
        targets: none,
    },
    Entry {
        id: "road.alignments.check",
        refusals: &[&["not_a_road_project", "unknown_alignment", "invalid_design_check"]],
        warnings: &["design_rule_broken"],
        targets: none,
    },
    Entry {
        id: "road.profiles.check",
        refusals: &[&["not_a_road_project", "unknown_road_profile", "invalid_design_check"]],
        warnings: &["design_rule_broken", "profile_stale"],
        targets: none,
    },
    Entry {
        id: "road.profiles.lengths",
        refusals: &[&["invalid_profile_lengths", "not_a_road_project", "unknown_road_profile"]],
        warnings: &["profile_stale", "vertical_curve_short"],
        targets: none,
    },
    Entry {
        id: "road.profiles.sight",
        refusals: &[&["not_a_road_project", "unknown_road_profile", "invalid_sight"]],
        warnings: &["sight_short", "profile_stale"],
        targets: none,
    },
    Entry {
        id: "road.profiles.propose",
        refusals: &[&["not_a_road_project", "unknown_road_profile", "invalid_profile_proposal"]],
        warnings: &["profile_stale"],
        targets: none,
    },
    // KentOS Yol's cross sections and templates (plan/road.md Y5, docs/adr/0903).
    Entry {
        id: "road.templates.edit",
        refusals: &[
            &["not_a_road_project", "unknown_road_template", "invalid_road_template", "road_template_in_use"],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.sections.edit",
        refusals: &[
            &[
                "not_a_road_project",
                "unknown_section_set",
                "unknown_alignment",
                "unknown_road_profile",
                "unknown_road_template",
                "unknown_superelevation",
                "unknown_terrain",
                "invalid_section_set",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.sections.query",
        refusals: &[&["not_a_road_project", "unknown_section_set", "invalid_section_set", "invalid_places"]],
        warnings: &[
            "section_set_stale",
            "section_surface_gone",
            "section_lines_cross",
            "section_open",
            "section_ground_missing",
            "section_superelevation_gone",
            "pavement_unbuilt",
        ],
        targets: none,
    },
    Entry {
        id: "road.sections.draw",
        refusals: &[
            &[
                "road_drawings_full",
                "not_a_road_project",
                "unknown_section_set",
                "invalid_label",
                "invalid_section_set",
                "invalid_places",
                "no_terrain",
            ],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden", "section_set_stale", "section_open", "section_ground_missing"],
        targets: none,
    },
    // KentOS Yol's superelevation (plan/road.md Y6, docs/adr/0904).
    Entry {
        id: "road.superelevations.edit",
        refusals: &[
            &[
                "not_a_road_project",
                "unknown_superelevation",
                "unknown_alignment",
                "invalid_superelevation",
                "superelevation_in_use",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.superelevations.query",
        refusals: &[&[
            "not_a_road_project",
            "unknown_superelevation",
            "invalid_places",
            "station_outside",
            "invalid_superelevation",
        ]],
        warnings: &["superelevation_stale"],
        targets: none,
    },
    Entry {
        id: "road.superelevations.check",
        refusals: &[&[
            "not_a_road_project",
            "unknown_superelevation",
            "unknown_road_template",
            "invalid_superelevation_check",
        ]],
        warnings: &["superelevation_limit_broken"],
        targets: none,
    },
    Entry {
        id: "road.superelevations.widen",
        refusals: &[&[
            "invalid_widening",
            "not_a_road_project",
            "unknown_superelevation",
            "unknown_road_template",
        ]],
        warnings: &["widening_unturnable", "superelevation_stale"],
        targets: none,
    },
    Entry {
        id: "road.superelevations.transitions",
        refusals: &[&[
            "invalid_superelevation_transition",
            "not_a_road_project",
            "unknown_superelevation",
            "unknown_road_template",
        ]],
        warnings: &["superelevation_transition_conflict"],
        targets: none,
    },
    // KentOS Yol's corridors (plan/road.md Y7, docs/adr/0905).
    Entry {
        id: "road.corridors.edit",
        refusals: &[
            &[
                "not_a_road_project",
                "unknown_corridor",
                "unknown_alignment",
                "unknown_road_profile",
                "unknown_terrain",
                "unknown_superelevation",
                "unknown_road_template",
                "invalid_road_corridor",
                "corridor_in_use",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.corridors.draw",
        refusals: &[
            &[
                "road_drawings_full",
                "not_a_road_project",
                "unknown_corridor",
                "invalid_label",
                "invalid_road_corridor",
                "no_terrain",
                "layer_not_found",
                "not_a_layer",
                "layer_locked",
            ],
            REVISION,
        ],
        warnings: &["layer_hidden", "corridor_stale", "corridor_unsolved", "corridor_gap", "corridor_superelevation_gone"],
        targets: none,
    },
    Entry {
        id: "road.corridors.query",
        refusals: &[&["not_a_road_project", "unknown_corridor", "invalid_road_corridor"]],
        warnings: &[
            "corridor_stale",
            "corridor_unsolved",
            "corridor_folded",
            "corridor_gap",
            "corridor_surface_gone",
            "corridor_superelevation_gone",
            "corridor_sampling_left",
        ],
        targets: none,
    },
    Entry {
        id: "road.corridors.water",
        refusals: &[&["not_a_road_project", "unknown_corridor", "invalid_platform_water"]],
        warnings: &["corridor_flat", "corridor_stale", "corridor_unsolved", "corridor_folded"],
        targets: none,
    },
    Entry {
        id: "road.corridors.preflight",
        refusals: &[&["not_a_road_project", "unknown_corridor", "invalid_preflight"]],
        warnings: &["corridor_tight", "corridor_overlap", "corridor_seam", "corridor_folded", "corridor_stale", "corridor_unsolved", "corridor_gap", "corridor_surface_gone"],
        targets: none,
    },
    Entry {
        id: "road.corridors.protected",
        refusals: &[&["not_a_road_project", "unknown_corridor", "invalid_protected_check"]],
        warnings: &["corridor_protected", "corridor_stale", "corridor_unsolved", "corridor_gap", "corridor_surface_gone"],
        targets: none,
    },
    Entry {
        id: "road.corridors.daylights",
        refusals: &[&["not_a_road_project", "unknown_corridor", "invalid_daylight_check"]],
        warnings: &["corridor_daylight_crossings", "corridor_daylight_sensitive", "corridor_stale", "corridor_unsolved", "corridor_surface_gone"],
        targets: none,
    },
    // KentOS Yol's earthwork volumes (plan/road.md Y8, docs/adr/0906).
    Entry {
        id: "road.volumes.edit",
        refusals: &[
            &["not_a_road_project", "unknown_volume", "volume_approved", "volume_in_use", "unknown_corridor", "invalid_volume"],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.volumes.query",
        refusals: &[&["not_a_road_project", "unknown_volume", "invalid_volume"]],
        warnings: &[
            "corridor_stale",
            "volume_gap",
            "pavement_unbuilt",
            "volume_uncovered",
            "volume_unfactored",
            "volume_curve_past_centre",
            "volume_changed",
        ],
        targets: none,
    },
    // KentOS Yol's mass haul diagrams (plan/road.md Y9, docs/adr/0907).
    Entry {
        id: "road.hauls.edit",
        refusals: &[&["not_a_road_project", "unknown_haul", "unknown_volume", "invalid_haul"], REVISION],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.hauls.query",
        refusals: &[&["not_a_road_project", "unknown_haul", "invalid_haul"]],
        warnings: &["corridor_stale", "volume_gap", "haul_unbalanced", "haul_site_unconnected"],
        targets: none,
    },
    Entry {
        id: "road.hauls.approve",
        refusals: &[&["not_a_road_project", "unknown_haul", "haul_not_approved", "invalid_haul"], REVISION],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.hauls.network",
        refusals: &[&["not_a_road_project", "unknown_haul", "invalid_haul"]],
        warnings: &["haul_network_gap", "haul_site_unconnected"],
        targets: none,
    },
    Entry {
        id: "road.hauls.plan",
        refusals: &[&["not_a_road_project", "unknown_haul", "invalid_haul"]],
        warnings: &["haul_plan_infeasible", "corridor_stale", "volume_gap", "haul_plan_stale", "haul_site_unconnected"],
        targets: none,
    },
    Entry {
        id: "road.hauls.trucks",
        refusals: &[&["not_a_road_project", "unknown_haul", "invalid_haul", "invalid_haul_trucks", "haul_truck_stalls"]],
        warnings: &["haul_plan_infeasible", "corridor_stale", "volume_gap"],
        targets: none,
    },
    // KentOS Yol's guardrails (plan/road.md Y11.1, docs/adr/0909 §1).
    Entry {
        id: "road.guardrails.edit",
        refusals: &[&["not_a_road_project", "unknown_guardrail", "unknown_corridor", "invalid_guardrail"], REVISION],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.guardrails.query",
        refusals: &[&["not_a_road_project", "unknown_guardrail", "invalid_guardrail"]],
        warnings: &["guardrail_stale", "corridor_stale", "corridor_surface_gone", "corridor_unsolved"],
        targets: none,
    },
    Entry {
        id: "road.guardrails.draw",
        refusals: &[&["road_drawings_full", "not_a_road_project", "unknown_guardrail", "invalid_guardrail"], LAYER, REVISION],
        warnings: &["layer_hidden", "guardrail_stale", "corridor_stale", "corridor_unsolved"],
        targets: none,
    },
    // KentOS Yol's culverts (plan/road.md Y11.2, docs/adr/0909 §2).
    Entry {
        id: "road.culverts.edit",
        refusals: &[&["not_a_road_project", "unknown_culvert", "unknown_corridor", "invalid_culvert"], REVISION],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.culverts.query",
        refusals: &[&["not_a_road_project", "unknown_culvert", "invalid_culvert"]],
        warnings: &["culvert_stale", "culvert_cover", "corridor_stale", "corridor_unsolved"],
        targets: none,
    },
    Entry {
        id: "road.culverts.draw",
        refusals: &[&["road_drawings_full", "not_a_road_project", "unknown_culvert", "invalid_culvert"], LAYER, REVISION],
        warnings: &["layer_hidden", "culvert_stale", "culvert_cover", "corridor_stale", "corridor_unsolved"],
        targets: none,
    },
    Entry {
        id: "road.culverts.closure",
        refusals: &[&["not_a_road_project", "unknown_culvert", "invalid_culvert_hydraulics", "invalid_culvert"]],
        warnings: &["culvert_overtopped", "culvert_road_closed", "culvert_stale", "corridor_stale", "corridor_unsolved"],
        targets: none,
    },
    Entry {
        id: "road.culverts.hydraulics",
        refusals: &[&["not_a_road_project", "unknown_culvert", "invalid_culvert_hydraulics", "invalid_culvert"]],
        warnings: &["culvert_overtopped", "culvert_stale", "culvert_cover", "corridor_stale", "corridor_unsolved"],
        targets: none,
    },
    // KentOS Yol's retaining walls (plan/road.md Y11.3, docs/adr/0909 §3).
    Entry {
        id: "road.walls.edit",
        refusals: &[&["not_a_road_project", "unknown_retaining_wall", "unknown_corridor", "invalid_retaining_wall"], REVISION],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.walls.query",
        refusals: &[&["not_a_road_project", "unknown_retaining_wall", "invalid_retaining_wall"]],
        warnings: &["retaining_wall_stale", "retaining_wall_high", "corridor_stale", "corridor_unsolved"],
        targets: none,
    },
    Entry {
        id: "road.walls.draw",
        refusals: &[&["road_drawings_full", "not_a_road_project", "unknown_retaining_wall", "invalid_retaining_wall"], LAYER, REVISION],
        warnings: &["layer_hidden", "retaining_wall_stale", "retaining_wall_high", "corridor_stale", "corridor_unsolved"],
        targets: none,
    },
    // KentOS Yol's graded pads (plan/road.md Y14, docs/adr/0927).
    Entry {
        id: "road.platforms.edit",
        refusals: &[&["not_a_road_project", "unknown_platform", "unknown_terrain", "invalid_platform"], REVISION],
        warnings: &["platform_open"],
        targets: none,
    },
    Entry {
        id: "road.platforms.query",
        refusals: &[&["not_a_road_project", "unknown_platform", "invalid_platform"]],
        warnings: &["platform_open", "platform_no_balance"],
        targets: none,
    },
    Entry {
        id: "road.platforms.draw",
        refusals: &[&["road_drawings_full", "not_a_road_project", "unknown_platform", "invalid_platform"], LAYER, REVISION],
        warnings: &["layer_hidden", "platform_open"],
        targets: none,
    },
    // KentOS Yol's surface drainage (plan/road.md Y16, docs/adr/0929).
    Entry {
        id: "road.drainage.query",
        refusals: &[&[
            "not_a_road_project",
            "invalid_drainage",
            "unknown_terrain",
            "unknown_culvert",
            "terrain_unbuildable",
            "invalid_culvert",
        ]],
        warnings: &["drainage_catchment_empty", "culvert_no_roughness", "culvert_over_capacity"],
        targets: none,
    },
    Entry {
        id: "road.drainage.draw",
        refusals: &[
            &["not_a_road_project", "invalid_drainage", "unknown_terrain", "unknown_culvert", "terrain_unbuildable"],
            &["drainage_nothing"],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden", "drainage_catchment_empty", "culvert_no_roughness", "culvert_over_capacity"],
        targets: none,
    },
    // KentOS Yol's surface analysis (plan/road.md Y18, docs/adr/0916).
    Entry {
        id: "road.terrains.analysis",
        refusals: &[&["not_a_road_project", "invalid_analysis", "unknown_terrain", "terrain_unbuildable"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.terrains.compare",
        refusals: &[&["not_a_road_project", "invalid_analysis", "unknown_terrain", "entity_not_found", "unproven_heights", "terrain_unbuildable"]],
        warnings: &["heights_assumed", "compare_uncovered"],
        targets: none,
    },
    Entry {
        id: "road.terrains.quality",
        refusals: &[&[
            "not_a_road_project",
            "invalid_analysis",
            "unknown_terrain",
            "terrain_unbuildable",
            "layer_not_found",
            "entity_not_found",
        ]],
        warnings: &["terrain_checkpoints_far"],
        targets: none,
    },
    Entry {
        id: "road.terrains.repair",
        refusals: &[
            &["not_a_road_project", "invalid_repair", "unknown_terrain", "repair_breakline", "repair_not_a_point", "terrain_unbuildable"],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.terrains.coverage",
        refusals: &[&["invalid_coverage", "not_a_road_project", "unknown_terrain", "terrain_unbuildable", "entity_not_found"]],
        warnings: &["terrain_no_data"],
        targets: none,
    },
    Entry {
        id: "road.terrains.lowconfidence",
        refusals: &[&["invalid_coverage", "not_a_road_project", "unknown_terrain", "terrain_unbuildable"], LAYER, REVISION],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "road.terrains.density",
        refusals: &[&["invalid_point_density", "not_a_road_project", "unknown_terrain", "terrain_unbuildable", "entity_not_found", "invalid_coverage"]],
        warnings: &["terrain_density_short"],
        targets: none,
    },
    // A stockpile's volume over a base from its boundary (plan/map-cad.md MAP-15, docs/adr/3140).
    Entry {
        id: "road.terrains.stockpile",
        refusals: &[&["invalid_stockpile", "not_a_road_project", "unknown_terrain", "terrain_unbuildable", "entity_not_found", "invalid_coverage"]],
        warnings: &["terrain_no_data"],
        targets: none,
    },
    Entry {
        id: "road.terrains.sparse",
        refusals: &[
            &["invalid_point_density", "not_a_road_project", "unknown_terrain", "terrain_unbuildable", "entity_not_found", "invalid_coverage"],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    // KentOS Yol's design vehicle swept path (plan/road.md Y17, docs/adr/0915).
    Entry {
        id: "road.swept.query",
        refusals: &[&["invalid_swept", "not_a_road_project", "unknown_junction", "invalid_junction", "entity_not_found"]],
        warnings: &["swept_touches"],
        targets: none,
    },
    Entry {
        id: "road.swept.draw",
        refusals: &[&["invalid_swept", "not_a_road_project", "unknown_junction"], LAYER, REVISION],
        warnings: &["layer_hidden", "swept_touches"],
        targets: none,
    },
    // KentOS Yol's terrain from point clouds (plan/road.md Y15, docs/adr/0928).
    Entry {
        id: "road.terrains.cloud",
        refusals: &[
            &[
                "not_a_road_project",
                "entity_not_found",
                "not_a_pointcloud",
                "invalid_cloud_terrain",
                "cloud_class_not_ground",
                "unknown_terrain",
            ],
            LAYER,
            &[
                "invalid_heights",
                "heights_unknown",
                "geoid_required",
                "geoid_missing",
                "vertical_unsupported",
                "cloud_heights_differ",
                "too_many_cloud_points",
                "cloud_ground_empty",
                "invalid_terrain",
                "terrain_unbuildable",
            ],
            REVISION,
        ],
        warnings: &["layer_hidden", "cloud_class_above_ground", "cloud_source_unreadable"],
        targets: none,
    },
    // KentOS Yol's pay item quantities (plan/road.md Y11.4, docs/adr/0909 §4).
    Entry {
        id: "road.pay.edit",
        refusals: &[&["not_a_road_project", "no_pay_book", "invalid_pay_book"], REVISION],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.pay.query",
        refusals: &[&["not_a_road_project", "no_pay_book", "invalid_pay_book"]],
        warnings: &["pay_source_stale", "pay_source_unread"],
        targets: none,
    },
    // KentOS Yol's preliminary routes (plan/road.md Y12.1, docs/adr/0910 §1).
    Entry {
        id: "road.routes.find",
        refusals: &[&["not_a_road_project", "unknown_terrain", "invalid_route", "route_blocked", "route_unreached"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.routes.draw",
        refusals: &[&["not_a_road_project", "unknown_terrain", "invalid_route", "route_blocked", "route_unreached"], LAYER, REVISION],
        warnings: &["layer_hidden"],
        targets: none,
    },
    // KentOS Yol's junctions (plan/road.md Y12.2, docs/adr/0910 §2).
    Entry {
        id: "road.junctions.edit",
        refusals: &[&["not_a_road_project", "unknown_junction", "unknown_alignment", "invalid_junction"], REVISION],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.junctions.query",
        refusals: &[&["not_a_road_project", "unknown_junction", "invalid_junction"]],
        warnings: &["junction_stale", "junction_on_curve"],
        targets: none,
    },
    Entry {
        id: "road.junctions.draw",
        refusals: &[&["road_drawings_full", "not_a_road_project", "unknown_junction", "invalid_junction"], LAYER, REVISION],
        warnings: &["layer_hidden", "junction_stale", "junction_on_curve"],
        targets: none,
    },
    Entry {
        id: "road.junctions.clearance",
        refusals: &[&[
            "not_a_road_project",
            "unknown_junction",
            "invalid_junction",
            "unknown_road_profile",
            "invalid_road_profile",
            "profile_curves_overlap",
            "invalid_clearance",
        ]],
        warnings: &["junction_stale", "profile_stale", "junction_clearance_short"],
        targets: none,
    },
    Entry {
        id: "road.junctions.tie",
        refusals: &[
            &[
                "not_a_road_project",
                "unknown_junction",
                "invalid_junction",
                "unknown_road_profile",
                "invalid_road_profile",
            "profile_curves_overlap",
                "unknown_superelevation",
                "invalid_superelevation",
                "invalid_tie",
            ],
            REVISION,
        ],
        warnings: &["junction_stale", "profile_stale", "superelevation_stale", "junction_grade_change"],
        targets: none,
    },
    Entry {
        id: "road.junctions.overlap",
        refusals: &[&[
            "not_a_road_project",
            "unknown_junction",
            "invalid_junction",
            "unknown_corridor",
            "invalid_road_corridor",
            "invalid_overlap",
        ]],
        warnings: &["junction_stale", "corridor_stale", "junction_corridors_overlap"],
        targets: none,
    },
    // KentOS Yol's project tree (plan/road.md Y13, docs/adr/0911 §2).
    Entry {
        id: "road.project.query",
        refusals: &[&["not_a_road_project"]],
        warnings: &[],
        targets: none,
    },
    // İş grafiği (plan/map-cad.md MAP-20, docs/adr/1940).
    Entry {
        id: "road.project.graph",
        refusals: &[&["not_a_road_project"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "road.project.rebuild",
        refusals: &[
            &["not_a_road_project", "unknown_drawing", "drawing_source_gone", "drawing_edited", "road_drawings_full", "layer_locked"],
            // Docs/adr/4320: a drawing's surface that no longer builds, said by the drawing command's check.
            &["terrain_unbuildable"],
            REVISION,
        ],
        warnings: &["corridor_unsolved"],
        targets: none,
    },
    // A recipe's drawings drawn in another project (plan/map-cad.md MAP-20, docs/adr/3240): the drawing commands'
    // refusals come through with the recipe's drawing named; its layers by name, given or added (docs/adr/3640); a layer
    // found here keeping fields other than the recipe's is said (docs/adr/4860).
    Entry {
        id: "road.project.produce",
        refusals: &[
            &["not_a_road_project", "invalid_file_path", "file_not_found", "invalid_project_recipe", "unknown_drawing"],
            &["unknown_terrain", "layer_not_found", "layer_locked", "not_a_layer"],
            &["unknown_recipe_layer", "ambiguous_recipe_layer"],
            REVISION,
        ],
        warnings: &["recipe_other_system", "layer_hidden", "recipe_layer_created", "recipe_layer_fields", "recipe_layer_fields_added"],
        targets: none,
    },
    Entry {
        id: "road.project.edit",
        refusals: &[&["not_a_road_project", "nothing_to_set"], REVISION],
        warnings: &["drawings_rebuilt", "drawing_edited", "drawing_not_rebuilt"],
        targets: none,
    },
];
