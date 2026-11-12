//! What the commands of dimensions, texts, hatches, leaders and tables take and how they answer (docs/adr/0253 §7): one of the parts the table
//! was cut into, in its order (docs/adr/8054 §4).

use super::*;

pub(super) const ENTRIES: &[Entry] = &[
    Entry {
        id: "cad.entities.offset",
        refusals: &[
            OBJECTS,
            &[
                "not_finite",
                "invalid_distance",
                "invalid_tolerance",
                "not_offsettable",
                "offset_failed",
                "radius_collapse",
                "layer_not_found",
                "not_a_layer",
            ],
            REVISION,
        ],
        warnings: &[
            "layer_hidden",
            "offset_failed",
            "radius_collapse",
            "approximate_offset",
            "self_intersecting",
        ],
        targets: || {
            vec![target(
                None,
                OFFSET_KINDS,
                &[],
                Some("not_offsettable"),
                None,
            )]
        },
    },
    Entry {
        id: "cad.entities.trim",
        refusals: &[
            OBJECTS,
            &[
                "too_few_points",
                "not_finite",
                "not_trimmable",
                "trim_failed",
            ],
            REVISION,
        ],
        warnings: &["trim_failed", "layer_locked"],
        targets: || {
            vec![target(
                None,
                TRIM_KINDS,
                &[],
                Some("trim_failed"),
                Some(
                    "Delikli alan not_trimmable ile, çok parçalı nesne trim_failed ile reddedilir; bir yer ancak sınırlar onu kesiyorsa budanır.",
                ),
            )]
        },
    },
    Entry {
        id: "cad.entities.extend",
        refusals: &[
            OBJECTS,
            &["too_few_points", "not_finite", "extend_failed"],
            REVISION,
        ],
        warnings: &["extend_failed", "layer_locked"],
        targets: || {
            vec![target(
                None,
                EXTEND_KINDS,
                &[],
                Some("extend_failed"),
                Some("Açık uçlar uzar; bir uç ancak o doğrultuda bir sınıra ulaşıyorsa uzatılır."),
            )]
        },
    },
    Entry {
        id: "cad.entities.fillet",
        refusals: &[
            OBJECTS,
            &[
                "not_finite",
                "invalid_distance",
                "not_a_corner",
                "fillet_failed",
            ],
            REVISION,
        ],
        warnings: &["fillet_failed"],
        targets: fillet_targets,
    },
    Entry {
        id: "cad.entities.chamfer",
        refusals: &[
            OBJECTS,
            &[
                "not_finite",
                "invalid_distance",
                "not_a_corner",
                "chamfer_failed",
            ],
            REVISION,
        ],
        warnings: &["chamfer_failed"],
        targets: corner_targets,
    },
    Entry {
        id: "cad.entities.stretch",
        refusals: &[
            &[
                "not_finite",
                "no_entities",
                "invalid_uid",
                "entity_not_found",
                "nothing_to_stretch",
                "shared_boundary_locked",
            ],
            REVISION,
        ],
        warnings: &[
            "outside_window",
            "follows_object",
            "locked_skipped",
            "neighbour_locked",
            "neighbour_invalid",
        ],
        targets: || {
            vec![target(
                None,
                &["*"],
                &["pointcloud"],
                Some("nothing_to_stretch"),
                Some(
                    "Köşesi pencerede olan nesne esner; nokta bulutu esnemez. Listelenen nesne penceresinde köşesi yoksa outside_window, bağlı olduğu nesneyi izliyorsa (ilişkili tarama, bağlı yazı, bütün noktaları bağlı ölçü) follows_object uyarısıyla atlanır.",
                ),
            )]
        },
    },
    Entry {
        id: "cad.entities.break",
        refusals: &[
            &[
                "invalid_uid",
                "not_finite",
                "entity_not_found",
                "not_breakable",
                "layer_locked",
                "break_failed",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: || {
            vec![target(
                None,
                BREAK_KINDS,
                &[],
                Some("not_breakable"),
                Some(
                    "Delikli alan not_breakable ile reddedilir; noktalar nesnenin üstünde değilse ya da kapalı nesne tek noktada bölünmek istenirse break_failed.",
                ),
            )]
        },
    },
    Entry {
        id: "cad.entities.join",
        refusals: &[
            &[
                "no_entities",
                "invalid_uid",
                "invalid_distance",
                "not_finite",
                "entity_not_found",
                "not_joinable",
                "layer_locked",
                "nothing_to_join",
                "join_failed",
            ],
            REVISION,
        ],
        warnings: &[
            "locked_skipped",
            "chain_stopped",
            "not_joined",
            "data_conflict",
        ],
        targets: || {
            vec![
                target(
                    None,
                    JOIN_KINDS,
                    &[],
                    Some("nothing_to_join"),
                    Some(
                        "Uç uca buluşan çizgi, yay ve açık çoklu çizgiler birleşir; öbür türler zincire girmez.",
                    ),
                ),
                target(
                    Some("chain"),
                    JOIN_KINDS,
                    &[],
                    Some("not_joinable"),
                    Some("Zincirin tohumu çizgi, yay ya da açık çoklu çizgi olmalı."),
                ),
            ]
        },
    },
    Entry {
        id: "cad.entities.reverse",
        refusals: &[OBJECTS, &["not_reversible"], REVISION],
        warnings: &["not_reversible", "layer_locked"],
        targets: || {
            vec![target(
                None,
                REVERSE_KINDS,
                &[],
                Some("not_reversible"),
                Some(
                    "Yay, daire ve elips hep saat yönünün tersine gider; yönsüz türler gibi atlanır.",
                ),
            )]
        },
    },
    Entry {
        id: "cad.entities.lengthen",
        refusals: &[
            &[
                "invalid_uid",
                "not_finite",
                "invalid_change",
                "entity_not_found",
                "layer_locked",
                "not_lengthenable",
                "multi_part",
                "angle_needs_arc",
                "invalid_length",
                "arc_closes",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: || {
            vec![target(
                None,
                &["line", "arc", "ellipse", "polyline"],
                &[],
                Some("not_lengthenable"),
                Some(
                    "Elipsin yalnız yayı, çoklu çizginin yalnız açık ve tek parçalısı uzar ya da kısalır.",
                ),
            )]
        },
    },
    Entry {
        id: "cad.entities.divide",
        refusals: &[
            &[
                "invalid_uid",
                "invalid_count",
                "invalid_distance",
                "not_finite",
                "start_needs_step",
                "invalid_scale",
                "entity_not_found",
                "not_divisible",
                "unknown_block",
                "no_points",
                "too_many_points",
            ],
            LAYER,
            LOOK,
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: || vec![target(None, DIVIDE_KINDS, &[], Some("not_divisible"), None)],
    },
    Entry {
        id: "cad.entities.boundary",
        refusals: &[
            &[
                "not_finite",
                "no_entities",
                "invalid_uid",
                "entity_not_found",
                "layer_not_found",
                "not_enclosed",
            ],
            LAYER,
            LOOK,
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.entities.hatch",
        refusals: &[
            &[
                "unknown_pattern",
                "invalid_scale",
                "not_finite",
                "invalid_color",
                "no_entities",
                "invalid_uid",
                "entity_not_found",
                "layer_not_found",
                "not_enclosed",
                "not_closed",
                "pattern_too_dense",
                "invalid_region",
                "invalid_gap",
                "not_annotative",
                "invalid_transparency",
                "order_limit",
            ],
            LAYER,
            LOOK,
            REVISION,
        ],
        warnings: &[
            "not_closed",
            "layer_hidden",
            "gaps_closed",
            "islands_not_nested",
            "order_other_layer",
            "cutout_gone",
            "hatch_text_cycle",
        ],
        targets: || {
            vec![target(
                Some("objects"),
                CLOSED_KINDS,
                &[],
                Some("not_closed"),
                Some(
                    "Çoklu tara kapalı nesneleri tarar: alan (çok parçalının her parçası), daire, tam elips ve kapalı eğri.",
                ),
            )]
        },
    },
    Entry {
        id: "cad.dimensions.reassociate",
        refusals: &[
            OBJECTS,
            &[
                "not_a_dimension",
                "invalid_refs",
                "no_such_place",
                "piece_gone",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: || {
            vec![target(
                None,
                &["dimension"],
                &[],
                Some("not_a_dimension"),
                None,
            )]
        },
    },
    Entry {
        id: "cad.dimensions.edit",
        refusals: &[
            OBJECTS,
            &[
                "not_finite",
                "invalid_offset",
                "empty_text",
                "invalid_style",
                "invalid_dimension",
                "not_a_dimension",
                "not_straight",
                "not_parallel",
                "level_not_found",
            ],
            REVISION,
        ],
        warnings: &["no_crossing"],
        targets: || {
            vec![target(
                None,
                &["dimension"],
                &[],
                Some("not_a_dimension"),
                None,
            )]
        },
    },
    Entry {
        id: "cad.fields.edit",
        refusals: &[
            &["invalid_uid", "entity_not_found", "layer_locked"],
            &["not_a_text", "invalid_text_field", "field_not_found"],
            REVISION,
        ],
        warnings: &["field_broken"],
        targets: || vec![target(None, &["text"], &[], Some("not_a_text"), None)],
    },
    Entry {
        id: "cad.texts.edit",
        refusals: &[
            OBJECTS,
            &[
                "not_a_text",
                "empty_text",
                "too_few_objects",
                "not_splittable",
                "invalid_number",
                "not_a_path",
                "not_finite",
                "text_not_taken",
            ],
            REVISION,
        ],
        warnings: &["text_left_blank", "layer_locked", "field_broken"],
        targets: || vec![target(None, &["text"], &[], Some("not_a_text"), None)],
    },
    Entry {
        id: "cad.texts.check",
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_text"]],
        warnings: &[],
        targets: || vec![target(None, &["text"], &[], Some("not_a_text"), None)],
    },
    Entry {
        id: "cad.hatches.edit",
        refusals: &[
            OBJECTS,
            &[
                "not_a_hatch",
                "layer_not_found",
                "not_a_layer",
                "not_closed",
                "not_enclosed",
                "island_not_found",
                "invalid_region",
                "not_finite",
                "invalid_gap",
                "unknown_pattern",
                "invalid_scale",
                "not_annotative",
                "pattern_too_dense",
                "invalid_color",
            ],
            REVISION,
        ],
        warnings: &[
            "curves_approximated",
            "gaps_closed",
            "islands_not_nested",
            "layer_hidden",
        ],
        targets: || vec![target(None, &["hatch"], &[], Some("not_a_hatch"), None)],
    },
    Entry {
        id: "cad.leaders.edit",
        refusals: &[
            OBJECTS,
            &[
                "not_a_leader",
                "too_few_objects",
                "arm_not_found",
                "invalid_leader",
                "not_an_insert",
                "style_not_found",
                "no_such_place",
                "not_finite",
                "empty_text",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: || vec![target(None, &["leader"], &[], Some("not_a_leader"), None)],
    },
    Entry {
        id: "cad.tables.edit",
        refusals: &[
            OBJECTS,
            &[
                "not_a_table",
                "cell_out_of_range",
                "schedule_cell",
                "invalid_table",
                "not_finite",
                "no_source",
                "source_missing",
                "source_unreadable",
            ],
            REVISION,
        ],
        warnings: &[
            "source_missing",
            "source_unreadable",
            "source_unlinked",
            // Veri çıkarımı tablosunun yenilenmesi (docs/adr/1030 §6).
            "refresh_kept",
            "extraction_source_missing",
            "extraction_source_unreadable",
            "join_unusable",
            "extraction_hidden",
            "xref_unloaded",
            "xref_read_apart",
            "join_unmatched",
            "dimension_overrides",
            "extraction_too_deep",
        ],
        targets: || vec![target(None, &["table"], &[], Some("not_a_table"), None)],
    },
    Entry {
        id: "cad.fonts.report",
        refusals: &[&["empty_name", "invalid_name"]],
        warnings: &["font_substituted"],
        targets: none,
    },
    Entry {
        id: "cad.dimensions.query",
        refusals: &[&["invalid_uid", "entity_not_found", "not_a_dimension"]],
        warnings: &[],
        targets: || {
            vec![target(
                None,
                &["dimension"],
                &[],
                Some("not_a_dimension"),
                None,
            )]
        },
    },
    Entry {
        id: "cad.dimensions.create",
        refusals: &[
            &[
                "not_finite",
                "invalid_height",
                "invalid_uid",
                "invalid_offset",
                "entity_not_found",
                "no_such_place",
                "piece_gone",
                "not_measurable",
                "not_straight",
                "missing_elevation",
                "unknown_style",
                "no_dimension",
                "invalid_dimension",
                "level_not_found",
            ],
            LAYER,
            LOOK,
            REVISION,
        ],
        warnings: &["layer_hidden", "underlay_measure"],
        targets: || {
            vec![
                target(
                    Some("radius"),
                    &["circle", "arc", "polyline", "polygon", "opening", "space"],
                    &[],
                    Some("not_measurable"),
                    Some(
                        "Yarıçap, Çap ve Kırıklı yarıçap daireden, yaydan, kapının açılım yayından ya da çoklu çizginin, alanın ve mahalın analitik yay parçasından ölçülür; yay parçası olmayan sınır ve açıklık not_measurable.",
                    ),
                ),
                target(
                    Some("arcLength"),
                    &["arc", "polyline", "polygon", "opening", "space"],
                    &[],
                    Some("not_measurable"),
                    Some(
                        "Yay uzunluğu ve yayın açısı uçları olan yaydan, kapının açılım yayından ya da çoklu çizginin, alanın ve mahalın analitik yaylı kenarından ölçülür; tam daire ve yaysız sınır/açıklık not_measurable.",
                    ),
                ),
            ]
        },
    },
];
