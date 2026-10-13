//! What each command takes and how it answers (docs/adr/0253 §7): the
//! object kinds its operations apply to, the codes it refuses with and the
//! codes of its warnings. The catalog carries them (`CommandCapabilities`),
//! beside the meaning of every code (`codes::CODES`) and the codes any call
//! may answer with whatever its command (`PROTOCOL_CODES`).
//!
//! Kept true by tests: every code a shared case of `fixtures/commands/v1`
//! answers with is listed for its command, and a listed code no case reaches
//! is named with the reason (`kentos-native-application`, tests/capabilities.rs);
//! for the commands that pick objects by kind, an object of each kind is
//! tried: a listed kind is taken, another is refused with the listed code.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

/// What a command takes and how it answers.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandCapabilities {
    /// The object kinds its operations apply to; none when it takes no objects
    /// by kind (it names layers, styles, settings) or takes every kind.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<CommandTarget>>", optional))]
    pub targets: Vec<CommandTarget>,
    /// The codes it refuses with (`failed`, `conflict`), beside the
    /// protocol's own (`CommandCatalog.protocol`); `codes` says what each means.
    pub refusals: Vec<String>,
    /// The codes of its warnings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub warnings: Vec<String>,
}

/// The object kinds an operation of a command applies to.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", ts(export))]
pub struct CommandTarget {
    /// The operation or method (a transform's `type`, a corner's `kind`);
    /// none for the command as a whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub operation: Option<String>,
    /// The kinds (`Entity`'s `kind`: `line`, `polyline`, `polygon` …);
    /// `*` every kind but the ones `except` names.
    pub kinds: Vec<String>,
    /// With `*`: the kinds it does not take.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    #[cfg_attr(feature = "ts", ts(as = "Option<Vec<String>>", optional))]
    pub except: Vec<String>,
    /// The code an object of a kind it does not take is refused with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub refused_with: Option<String>,
    /// Turkish: what else decides.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub note: Option<String>,
}

/// Every object kind, as `Entity` tags it.
pub const KINDS: [&str; 19] = [
    "point",
    "line",
    "polyline",
    "polygon",
    "circle",
    "arc",
    "ellipse",
    "spline",
    "xline",
    "ray",
    "text",
    "dimension",
    "hatch",
    "insert",
    "leader",
    "table",
    "image",
    "raster",
    "pointcloud",
];

/// The codes any call may answer with, whatever its command: the
/// protocol's (docs/adr/0253 §4–§9).
pub const PROTOCOL_CODES: &[&str] = &[
    "invalid_call",
    "unknown_command",
    "server_command",
    "unknown_version",
    "invalid_input",
    "invalid_revision",
    "revision_conflict",
    "stale_state",
    "plan_changed",
    "request_reused",
    "integrity_failed",
    "document_changed",
    "already_committed",
    "unknown_job",
];

/// The revision's two, every writing command's.
const REVISION: &[&str] = &["invalid_revision", "revision_conflict"];
/// A new object's layer.
const LAYER: &[&str] = &["layer_not_found", "not_a_layer", "layer_locked"];
/// Objects named by persistent id.
const OBJECTS: &[&str] = &[
    "no_entities",
    "invalid_uid",
    "entity_not_found",
    "layer_locked",
];

struct Entry {
    id: &'static str,
    refusals: &'static [&'static [&'static str]],
    warnings: &'static [&'static str],
    targets: fn() -> Vec<CommandTarget>,
}

fn none() -> Vec<CommandTarget> {
    Vec::new()
}

fn target(
    operation: Option<&str>,
    kinds: &[&str],
    except: &[&str],
    refused_with: Option<&str>,
    note: Option<&str>,
) -> CommandTarget {
    CommandTarget {
        operation: operation.map(str::to_owned),
        kinds: kinds.iter().map(|k| (*k).to_owned()).collect(),
        except: except.iter().map(|k| (*k).to_owned()).collect(),
        refused_with: refused_with.map(str::to_owned),
        note: note.map(str::to_owned),
    }
}

/// The curves the core offsets (docs/adr/0246).
pub const OFFSET_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "circle", "arc", "ellipse", "xline", "ray",
];
/// The curves the core cuts (docs/adr/0250).
pub const TRIM_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "circle", "arc", "ellipse", "xline", "ray",
];
/// The curves the core grows (docs/adr/0250).
pub const EXTEND_KINDS: &[&str] = &["line", "polyline", "arc", "ellipse"];
/// The objects Kır breaks (docs/adr/0254 §3).
pub const BREAK_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "arc", "circle", "ellipse", "xline", "ray",
];
/// The objects a chain is made of (docs/adr/0254 §4).
pub const JOIN_KINDS: &[&str] = &["line", "arc", "polyline"];
/// The objects Böl divides (docs/adr/0254 §5).
pub const DIVIDE_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "arc", "circle", "ellipse", "spline",
];
/// The closed objects Çoklu tara hatches (docs/adr/0254 §7).
pub const CLOSED_KINDS: &[&str] = &["polygon", "circle", "ellipse", "spline"];
/// A new object's look, as `cad.entities.create` checks it.
const LOOK: &[&str] = &["invalid_line_weight"];

const ENTRIES: &[Entry] = &[
    Entry {
        id: "cad.polygon.create",
        refusals: &[
            &[
                "too_few_corners",
                "bulge_count",
                "not_finite",
                "invalid_line_weight",
            ],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.line.create",
        refusals: &[&["not_finite", "invalid_line_weight"], LAYER, REVISION],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.polyline.create",
        refusals: &[
            &[
                "too_few_points",
                "bulge_count",
                "not_finite",
                "invalid_polyline",
                "invalid_line_weight",
            ],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.entities.delete",
        refusals: &[OBJECTS, REVISION],
        warnings: &["layer_locked"],
        targets: || vec![target(None, &["*"], &[], None, None)],
    },
    Entry {
        id: "cad.point.create",
        refusals: &[&["not_finite"], LAYER, REVISION],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.circle.create",
        refusals: &[
            &["not_finite", "invalid_radius", "invalid_line_weight"],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.arc.create",
        refusals: &[
            &["not_finite", "invalid_radius", "invalid_line_weight"],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.entities.transform",
        refusals: &[
            OBJECTS,
            &[
                "not_finite",
                "invalid_factor",
                "invalid_axis",
                "invalid_align",
                "invalid_transform",
                "beyond_horizon",
                "invalid_links",
                "too_few_objects",
                "pointcloud_fixed",
                "raster_not_warped",
                "slots_exhausted",
            ],
            REVISION,
        ],
        warnings: &["layer_locked", "warp_curves", "warp_shapes", "rubber_bends"],
        targets: || {
            vec![
                target(
                    None,
                    &["*"],
                    &["pointcloud"],
                    Some("pointcloud_fixed"),
                    Some("Nokta bulutunun yeri dosyalarınındır."),
                ),
                target(
                    Some("projective"),
                    &["*"],
                    &["raster", "pointcloud"],
                    Some("raster_not_warped"),
                    None,
                ),
                target(
                    Some("rubbersheet"),
                    &["*"],
                    &["raster", "pointcloud"],
                    Some("raster_not_warped"),
                    None,
                ),
            ]
        },
    },
    Entry {
        id: "cad.entities.edit",
        refusals: &[
            &[
                "no_changes",
                "invalid_uid",
                "entity_not_found",
                "repeated_entity",
                "not_finite",
                "too_few_corners",
                "too_few_points",
                "part_holes",
                "invalid_radius",
                "invalid_path",
                "invalid_elevations",
                "invalid_measures",
                "invalid_polyline",
                "empty_text",
                "invalid_height",
                "invalid_width_factor",
                "invalid_arrow_size",
                "invalid_paragraph",
                "invalid_style",
                "unknown_style",
                "invalid_dimension",
                "invalid_refs",
                "invalid_hatch",
                "invalid_table",
                "not_a_table",
                "invalid_image",
                "unknown_asset",
                "invalid_raster",
                "invalid_pointcloud",
                "invalid_scale",
                "unknown_block",
                "invalid_line_weight",
                "slots_exhausted",
            ],
            LAYER,
            REVISION,
        ],
        warnings: &["elevation_lost", "measure_lost", "widths_lost"],
        targets: none,
    },
    Entry {
        id: "cad.entities.array",
        refusals: &[
            OBJECTS,
            &[
                "not_finite",
                "invalid_count",
                "invalid_spacing",
                "invalid_fill",
                "invalid_path",
                "pointcloud_fixed",
                "slots_exhausted",
            ],
            REVISION,
        ],
        warnings: &["layer_locked"],
        targets: || {
            vec![
                target(
                    None,
                    &["*"],
                    &["pointcloud"],
                    Some("pointcloud_fixed"),
                    None,
                ),
                target(
                    Some("path"),
                    &["line", "arc", "circle", "polyline"],
                    &[],
                    Some("invalid_path"),
                    Some("Yolun kendisi; dizilen nesneler bütün türlerden."),
                ),
            ]
        },
    },
    Entry {
        id: "cad.entities.create",
        refusals: &[
            &[
                "no_objects",
                "not_finite",
                "too_few_corners",
                "too_few_points",
                "invalid_polyline",
                "invalid_radius",
                "invalid_path",
                "empty_text",
                "invalid_height",
                "invalid_width_factor",
                "invalid_paragraph",
                "invalid_style",
                "unknown_style",
                "invalid_dimension",
                "invalid_refs",
                "no_such_place",
                "invalid_hatch",
                "invalid_table",
                "invalid_image",
                "unknown_asset",
                "invalid_raster",
                "invalid_pointcloud",
                "invalid_scale",
                "unknown_block",
                "invalid_link",
                "link_not_found",
                "invalid_attribute",
                "attribute_required",
                "entity_not_found",
                "slots_exhausted",
            ],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.entities.set",
        refusals: &[
            OBJECTS,
            &[
                "nothing_to_set",
                "layer_not_found",
                "not_a_layer",
                "invalid_line_weight",
                "invalid_attribute",
                "attribute_required",
                "invalid_attachments",
                "unknown_asset",
            ],
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: || vec![target(None, &["*"], &[], None, None)],
    },
    Entry {
        id: "cad.blocks.define",
        refusals: &[
            OBJECTS,
            &[
                "empty_name",
                "duplicate_block",
                "no_layer",
                "layer_not_found",
                "not_a_layer",
                "not_finite",
                "block_too_deep",
                "table_in_block",
                "image_in_block",
                "raster_in_block",
                "pointcloud_in_block",
                "block_refused",
                "slots_exhausted",
            ],
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: || {
            vec![target(
                None,
                &["*"],
                &["table", "image", "raster", "pointcloud"],
                None,
                Some(
                    "Tablo, resim, raster ve nokta bulutu blok tanımına girmez (table_in_block, image_in_block, raster_in_block, pointcloud_in_block).",
                ),
            )]
        },
    },
    Entry {
        id: "cad.blocks.edit",
        refusals: &[
            OBJECTS,
            &[
                "no_block",
                "unknown_block",
                "empty_name",
                "duplicate_block",
                "no_layer",
                "no_base",
                "not_finite",
                "block_in_use",
                "block_cycle",
                "no_attributes",
                "empty_tag",
                "duplicate_tag",
                "invalid_height",
                "table_in_block",
                "image_in_block",
                "raster_in_block",
                "block_refused",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.layers.service",
        refusals: &[
            &[
                "no_layer",
                "layer_not_found",
                "not_a_layer",
                "empty_name",
                "no_service",
                "no_feed",
                "service_and_feed",
                "invalid_service",
                "invalid_feed",
                "invalid_fields",
                "invalid_connection",
                "not_a_service_layer",
                "layer_has_objects",
                "unknown_connection",
                "layer_refused",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.network.define",
        refusals: &[
            &["no_network", "no_id", "invalid_network", "unknown_network"],
            REVISION,
        ],
        warnings: &["unknown_layer"],
        targets: none,
    },
    Entry {
        id: "cad.entities.offset",
        refusals: &[
            OBJECTS,
            &[
                "not_finite",
                "invalid_distance",
                "not_offsettable",
                "offset_failed",
                "layer_not_found",
                "not_a_layer",
            ],
            REVISION,
        ],
        warnings: &["layer_hidden", "offset_failed"],
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
            &["not_finite", "not_trimmable", "trim_failed"],
            REVISION,
        ],
        warnings: &["trim_failed"],
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
        refusals: &[OBJECTS, &["not_finite", "extend_failed"], REVISION],
        warnings: &["extend_failed"],
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
        targets: corner_targets,
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
            ],
            REVISION,
        ],
        warnings: &[
            "outside_window",
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
                    "Köşesi pencerede olan nesne esner; nokta bulutu esnemez. Listelenen nesne penceresinde köşesi yoksa outside_window uyarısıyla atlanır.",
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
        warnings: &["locked_skipped", "chain_stopped"],
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
        id: "cad.entities.divide",
        refusals: &[
            &[
                "invalid_uid",
                "invalid_count",
                "invalid_distance",
                "entity_not_found",
                "not_divisible",
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
            ],
            LAYER,
            LOOK,
            REVISION,
        ],
        warnings: &["not_closed", "layer_hidden"],
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
            &["not_a_dimension", "invalid_refs", "no_such_place"],
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
        id: "cad.dimensions.create",
        refusals: &[
            &[
                "not_finite",
                "invalid_height",
                "invalid_uid",
                "invalid_offset",
                "entity_not_found",
                "no_such_place",
                "not_measurable",
                "missing_elevation",
                "unknown_style",
                "no_dimension",
            ],
            LAYER,
            LOOK,
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: || {
            vec![
                target(
                    Some("radius"),
                    &["circle", "arc"],
                    &[],
                    Some("not_measurable"),
                    Some(
                        "Yarıçap, Çap ve Kırıklı yarıçap daireden, yaydan ya da çoklu çizginin ve alanın yay parçasından ölçülür; yay parçası olmayan çoklu çizgi ve alan not_measurable.",
                    ),
                ),
                target(
                    Some("arcLength"),
                    &["arc"],
                    &[],
                    Some("not_measurable"),
                    Some(
                        "Yay uzunluğu ve yayın açısı uçları olan yaydan ya da çoklu çizginin ve alanın yaylı kenarından ölçülür; tam daire not_measurable.",
                    ),
                ),
            ]
        },
    },
    Entry {
        id: "cad.addresses.define",
        refusals: &[
            &["no_locator", "no_id", "invalid_locator", "unknown_locator"],
            REVISION,
        ],
        warnings: &["unknown_layer"],
        targets: none,
    },
    Entry {
        id: "cad.layers.time",
        refusals: &[
            &[
                "layer_not_found",
                "not_a_layer",
                "service_layer",
                "invalid_time",
                "layer_refused",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.scenarios.edit",
        refusals: &[
            &[
                "empty_name",
                "invalid_name",
                "invalid_note",
                "duplicate_layer",
                "layer_not_found",
                "layer_locked",
                "not_a_base_layer",
                "no_scenario",
                "scenario_not_found",
                "layer_refused",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.layers.filter",
        refusals: &[
            &[
                "layer_not_found",
                "not_a_layer",
                "service_layer",
                "invalid_filter",
                "invalid_expression",
                "layer_refused",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.layers.compose",
        refusals: &[
            &[
                "layer_not_found",
                "not_a_layer",
                "invalid_compose",
                "mask_target_self",
                "mask_target_not_found",
                "mask_target_not_a_layer",
                "layer_refused",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.layers.info",
        refusals: &[
            &[
                "layer_not_found",
                "not_a_layer",
                "service_layer",
                "invalid_actions",
                "invalid_card",
                "invalid_template",
                "layer_refused",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.layers.labels",
        refusals: &[
            &[
                "layer_not_found",
                "not_a_layer",
                "service_layer",
                "invalid_labels",
                "invalid_expression",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.labels.pin",
        refusals: &[OBJECTS, &["invalid_pin", "unknown_class"], REVISION],
        warnings: &[],
        targets: || vec![target(None, &["*"], &[], None, None)],
    },
    Entry {
        id: "cad.layers.renderer",
        refusals: &[
            &[
                "layer_not_found",
                "not_a_layer",
                "service_layer",
                "invalid_renderer",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.layers.edit",
        refusals: &[
            &[
                "no_layer",
                "no_layers",
                "layer_not_found",
                "not_a_layer",
                "layer_locked",
                "service_layer",
                "empty_name",
                "invalid_color",
                "invalid_line_weight",
                "invalid_fields",
                "not_a_group",
                "move_into_itself",
                "nothing_to_set",
                "unknown_block",
                "layer_refused",
            ],
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.layers.set",
        refusals: &[
            &[
                "no_layer",
                "no_layers",
                "layer_not_found",
                "not_a_layer",
                "service_layer",
                "empty_name",
                "no_value",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.layers.states",
        refusals: &[
            &[
                "no_state",
                "state_not_found",
                "empty_name",
                "duplicate_name",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.styles.edit",
        refusals: &[
            &[
                "no_definition",
                "no_style",
                "style_not_found",
                "duplicate_style",
                "empty_name",
                "invalid_style",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.settings.set",
        refusals: &[
            &[
                "nothing_to_set",
                "empty_name",
                "invalid_project_type",
                "invalid_decimals",
                "invalid_scale",
                "unknown_srid",
                "second_needs_system",
                "second_is_own",
                "unit_needs_local",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.units.change",
        refusals: &[
            &["same_unit"],
            REVISION,
            &["unit_needs_local", "unit_mismatch", "layer_locked"],
        ],
        warnings: &["point_clouds_kept", "sheets_kept", "tolerances_kept"],
        targets: none,
    },
    Entry {
        id: "cad.sheets.edit",
        refusals: &[
            &[
                "no_operations",
                "invalid_operation",
                "book_unreadable",
                "unknown_sheet",
                "unknown_item",
                "unknown_master",
                "duplicate_id",
                "no_items",
                "mixed_owners",
                "item_locked",
                "ids_missing",
                "master_in_use",
                "no_master",
                "bad_order",
                "unknown_guide",
                "asset_in_use",
                "unknown_variant",
                "item_in_use",
                "bad_patch",
                "kind_change",
                "bad_group",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.changeset.apply",
        refusals: &[
            &[
                "no_steps",
                "nested_changeset",
                "side_effect_in_changeset",
                "unknown_command",
                "unknown_version",
                "invalid_input",
                "unknown_reference",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "file.import.dxf",
        refusals: &[
            &["invalid_file_path"],
            REVISION,
            &["file_not_found", "import_unreadable"],
            LAYER,
            &["nothing_to_import"],
        ],
        warnings: &[
            "unit_assumed",
            "unit_ignored",
            "block_units_differ",
            "import_losses",
        ],
        targets: none,
    },
    Entry {
        id: "file.export.dxf",
        refusals: &[&[
            "invalid_file_path",
            "file_exists",
            "layer_not_found",
            "nothing_to_export",
            "file_unwritable",
        ]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "file.export.pdf",
        refusals: &[&[
            "invalid_file_path",
            "file_exists",
            "no_sheets",
            "unknown_sheet",
            "book_unreadable",
            "pdf_unwritable",
            "file_unwritable",
        ]],
        warnings: &[],
        targets: none,
    },
];

fn corner_targets() -> Vec<CommandTarget> {
    vec![
        target(
            Some("vertex"),
            &["polyline", "polygon"],
            &[],
            Some("not_a_corner"),
            Some("İki düz kenar arasındaki köşe."),
        ),
        target(Some("lines"), &["line"], &[], Some("not_a_corner"), None),
        target(
            Some("all"),
            &["polyline", "polygon"],
            &[],
            Some("not_a_corner"),
            None,
        ),
    ]
}

/// What command `id` takes and how it answers; none for a command the
/// table does not know (the server's own: their answers are `ApiError`s).
pub fn capabilities_of(id: &str) -> Option<CommandCapabilities> {
    let entry = ENTRIES.iter().find(|e| e.id == id)?;
    let mut refusals: Vec<String> = Vec::new();
    for list in entry.refusals {
        for code in *list {
            if !refusals.iter().any(|r| r == code) {
                refusals.push((*code).to_owned());
            }
        }
    }
    Some(CommandCapabilities {
        targets: (entry.targets)(),
        refusals,
        warnings: entry.warnings.iter().map(|w| (*w).to_owned()).collect(),
    })
}

/// The ids the table lists, in its order.
pub fn listed() -> impl Iterator<Item = &'static str> {
    ENTRIES.iter().map(|e| e.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_listed_code_has_its_meaning() {
        for id in listed() {
            let c = capabilities_of(id).expect("listed");
            for code in c.refusals.iter().chain(&c.warnings) {
                assert!(
                    crate::codes::info(code).is_some(),
                    "{id}: {code} has no meaning in codes.rs"
                );
            }
            for t in &c.targets {
                for k in t.kinds.iter().chain(&t.except) {
                    assert!(k == "*" || KINDS.contains(&k.as_str()), "{id}: {k}");
                }
                if let Some(code) = &t.refused_with {
                    assert!(
                        c.refusals.contains(code),
                        "{id}: {code} refuses a kind but is not listed"
                    );
                }
            }
        }
        for code in PROTOCOL_CODES {
            assert!(crate::codes::info(code).is_some(), "{code}");
        }
    }
}
