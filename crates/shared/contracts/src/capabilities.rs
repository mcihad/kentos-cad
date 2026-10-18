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
pub const KINDS: [&str; 20] = [
    "point",
    "line",
    "polyline",
    "polygon",
    "circle",
    "arc",
    "ellipse",
    "spline",
    "nurbs",
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
    // Every write that changes constrained objects is followed by the constraints' solve (docs/adr/0337 §1, §2).
    "constraint_conflict",
    "constraint_singular",
    "not_converged",
    "constraints_removed",
    "constraints_relaxed",
];

/// The revision's two, every writing command's.
/// A new object's attributes by its layer's fields (TODOS.md FORM-O, ADR 0199 §2).
const ATTRS: &[&str] = &["invalid_attribute", "attribute_required"];
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

/// The objects a tangent circle touches (docs/adr/0289).
pub const TANGENT_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "arc", "circle", "xline", "ray",
];
/// The curves the core offsets (docs/adr/0246).
pub const OFFSET_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "circle", "arc", "ellipse", "spline", "nurbs", "xline", "ray",
];
/// The curves the core cuts (docs/adr/0250).
pub const TRIM_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "circle", "arc", "ellipse", "xline", "ray",
];
/// The curves the core grows (docs/adr/0250).
pub const EXTEND_KINDS: &[&str] = &["line", "polyline", "arc", "ellipse"];
/// The objects Kır breaks (docs/adr/0254 §3, 0296).
pub const BREAK_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "arc", "circle", "ellipse", "nurbs", "xline", "ray",
];
/// The objects a chain is made of (docs/adr/0254 §4).
pub const JOIN_KINDS: &[&str] = &["line", "arc", "polyline"];
/// The objects Yönü çevir turns (docs/adr/0296).
pub const REVERSE_KINDS: &[&str] = &["line", "polyline", "polygon", "spline", "nurbs"];
/// The objects Böl divides (docs/adr/0254 §5).
pub const DIVIDE_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "arc", "circle", "ellipse", "spline", "nurbs",
];
/// The objects a UCS can be aligned to (docs/adr/0261 §3).
pub const UCS_KINDS: &[&str] = &[
    "line", "polyline", "polygon", "hatch", "arc", "circle", "ellipse", "text", "insert", "point",
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
            ATTRS,
            // A value group's combination (docs/adr/0423 §2).
            &["invalid_combination"],
            // The layer's record rules and unique fields (docs/adr/0438 §1, §3).
            &["rule_broken", "not_unique"],
        ],
        warnings: &["layer_hidden", "layer_locked"],
        targets: none,
    },
    Entry {
        id: "cad.rectangle.create",
        refusals: &[
            &[
                "not_finite",
                "invalid_distance",
                "degenerate_shape",
                "invalid_corners",
                "invalid_line_weight",
            ],
            LAYER,
            REVISION,
            ATTRS,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.polygon.regular",
        refusals: &[
            &[
                "not_finite",
                "invalid_count",
                "degenerate_shape",
                "invalid_line_weight",
            ],
            LAYER,
            REVISION,
            ATTRS,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.line.create",
        refusals: &[
            &["not_finite", "invalid_line_weight"],
            LAYER,
            REVISION,
            ATTRS,
        ],
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
            ATTRS,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.entities.delete",
        refusals: &[
            OBJECTS,
            REVISION,
            &["relation_restricted", "relation_locked", "relation_too_large"],
        ],
        warnings: &["layer_locked"],
        targets: || vec![target(None, &["*"], &[], None, None)],
    },
    Entry {
        id: "cad.layers.rules",
        refusals: &[
            REVISION,
            &["layer_not_found", "not_a_layer", "service_layer"],
            &["invalid_rules"],
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.layers.contingents",
        refusals: &[
            REVISION,
            &["layer_not_found", "not_a_layer", "service_layer"],
            &["invalid_contingents"],
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.layers.subtypes",
        refusals: &[
            REVISION,
            &["layer_not_found", "not_a_layer", "service_layer"],
            &["invalid_subtypes"],
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.layers.table",
        refusals: &[
            &["invalid_table_styles"],
            REVISION,
            &["layer_not_found", "not_a_layer", "service_layer"],
            &["field_not_found", "invalid_expression"],
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.records.duplicate",
        refusals: &[
            &[
                "no_entities",
                "invalid_uid",
                "invalid_count",
                "entity_not_found",
                "layer_locked",
            ],
            REVISION,
        ],
        warnings: &["layer_locked", "unique_cleared"],
        targets: none,
    },
    Entry {
        id: "cad.records.create",
        refusals: &[&["invalid_count"], LAYER, REVISION, ATTRS],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.point.create",
        refusals: &[&["not_finite"], LAYER, REVISION, ATTRS],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.circle.create",
        refusals: &[
            &["not_finite", "invalid_radius", "invalid_line_weight"],
            LAYER,
            REVISION,
            ATTRS,
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
            ATTRS,
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
        warnings: &[
            "layer_locked",
            "warp_curves",
            "warp_shapes",
            "rubber_bends",
            "constraints_not_copied",
        ],
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
                "invalid_nurbs",
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
        warnings: &["layer_locked", "constraints_not_copied"],
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
        id: "cad.entities.copy",
        refusals: &[&[
            "no_entities",
            "invalid_uid",
            "not_finite",
            "entity_not_found",
        ]],
        warnings: &["constraints_not_copied"],
        targets: none,
    },
    Entry {
        id: "cad.entities.paste",
        refusals: &[
            &[
                "no_entities",
                "invalid_clip",
                "not_finite",
                "empty_name",
                "not_a_layer",
                "layer_locked",
            ],
            REVISION,
        ],
        warnings: &["units_differ", "layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.entities.explode",
        refusals: &[
            OBJECTS,
            &["invalid_density", "not_explodable", "slots_exhausted"],
            REVISION,
        ],
        warnings: &["layer_locked", "not_explodable"],
        targets: || {
            vec![target(
                None,
                &[
                    "polyline",
                    "polygon",
                    "circle",
                    "arc",
                    "ellipse",
                    "spline",
                    "nurbs",
                    "dimension",
                    "hatch",
                    "leader",
                    "table",
                    "insert",
                    "point",
                ],
                &[],
                Some("not_explodable"),
                Some("Noktanın yalnız çok parçalısı; taramanın yalnız desenlisi patlar."),
            )]
        },
    },
    Entry {
        id: "cad.curves.edit",
        refusals: &[
            &[
                "invalid_uid",
                "not_finite",
                "invalid_weight",
                "invalid_tolerance",
                "entity_not_found",
                "layer_locked",
                "not_a_curve",
                "multi_part",
                "edit_not_applicable",
                "no_fit_points",
                "invalid_index",
                "invalid_knot",
                "too_few_points",
                "coincident_points",
                "invalid_nurbs",
            ],
            REVISION,
        ],
        warnings: &["widths_lost"],
        targets: || {
            vec![target(
                None,
                &["polyline", "nurbs"],
                &[],
                Some("not_a_curve"),
                Some(
                    "Çoklu çizginin düzenlemeleri ile NURBS eğrisininkiler ayrı; öbürünün düzenlemesi edit_not_applicable.",
                ),
            )]
        },
    },
    Entry {
        id: "cad.drawing.audit",
        refusals: &[REVISION],
        warnings: &["layer_locked"],
        targets: none,
    },
    Entry {
        id: "cad.entities.cleanup",
        refusals: &[
            &[
                "invalid_uid",
                "no_entities",
                "not_finite",
                "invalid_tolerance",
                "entity_not_found",
                "layer_locked",
            ],
            REVISION,
        ],
        warnings: &["layer_locked"],
        targets: none,
    },
    Entry {
        id: "cad.entities.match",
        refusals: &[
            &[
                "invalid_uid",
                "no_entities",
                "no_properties",
                "entity_not_found",
                "layer_locked",
            ],
            REVISION,
        ],
        warnings: &["layer_locked", "layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.constraints.edit",
        refusals: &[
            &[
                "no_constraints",
                "invalid_uid",
                "invalid_constraint",
                "invalid_parameter",
                "parameter_expression",
                "entity_not_found",
                "element_not_found",
                "constraint_governed",
                "constraint_not_found",
                "parameter_in_use",
                "parameter_kind",
                "parameter_missing",
                "parameter_cycle",
                "parameter_range",
                "layer_locked",
                "constraint_conflict",
                "constraint_singular",
                "not_converged",
                "constraint_redundant",
                "shape_flipped",
            ],
            REVISION,
        ],
        warnings: &[],
        // A circle's part: a circle and an arc have one, another kind is refused for the part (docs/adr/0335 §1).
        targets: || vec![target(Some("circle"), &["circle", "arc"], &[], Some("element_not_found"), None)],
    },
    Entry {
        id: "cad.parameters.edit",
        refusals: &[
            &[
                "invalid_parameter",
                "parameter_expression",
                "parameter_not_found",
                "parameter_exists",
                "parameter_measured",
                "parameter_read",
                "parameter_missing",
                "parameter_cycle",
                "parameter_range",
                "invalid_constraint",
                "layer_locked",
                "constraint_conflict",
                "constraint_singular",
                "not_converged",
                "shape_flipped",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.constraints.query",
        refusals: &[&["invalid_uid"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.arrays.edit",
        refusals: &[
            &[
                "invalid_uid",
                "not_finite",
                "invalid_count",
                "invalid_spacing",
                "invalid_fill",
                "invalid_scale",
                "invalid_item",
                "array_not_found",
                "layer_locked",
                "item_removed",
                "unknown_block",
                "entity_not_found",
                "invalid_path",
                "slots_exhausted",
                "block_refused",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.entities.create",
        refusals: &[
            &[
                "no_objects",
                "invalid_transparency",
                "not_finite",
                "too_few_corners",
                "too_few_points",
                "invalid_polyline",
                "invalid_nurbs",
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
                "invalid_line_type_scale",
                "invalid_transparency",
                "invalid_wipeout",
                "invalid_attribute",
                "attribute_required",
                "invalid_attachments",
                "unknown_asset",
                "line_type_not_found",
                "constant_attribute",
                "form_rules",
                "invalid_combination",
                // docs/adr/0438 §1, §3: the layer's record rules and unique fields.
                "rule_broken",
                "not_unique",
            ],
            REVISION,
        ],
        warnings: &["layer_hidden", "subtype_misfit", "combination_unmet", "rule_unmet"],
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
                "block_too_large",
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
                "block_too_large",
                "no_attributes",
                "empty_tag",
                "duplicate_tag",
                "invalid_height",
                "table_in_block",
                "image_in_block",
                "raster_in_block",
                "block_refused",
                "no_library",
                "invalid_library",
                "no_units",
                "duplicate_piece",
                "invalid_attribute_options",
                "invalid_block_card",
                "attribute_not_movable",
                "invalid_field",
                "not_insert",
                "unknown_tag",
                "layer_locked",
                "invalid_dynamic_block",
            ],
            REVISION,
        ],
        warnings: &["layer_locked"],
        targets: none,
    },
    Entry {
        id: "cad.blocks.parameters",
        refusals: &[
            &["no_entities", "invalid_uid"],
            REVISION,
            &[
                "entity_not_found",
                "not_insert",
                "not_dynamic",
                "unknown_parameter",
                "derived_parameter",
                "invalid_parameter_value",
                "layer_locked",
            ],
        ],
        warnings: &["layer_locked"],
        targets: none,
    },
    Entry {
        id: "cad.blocks.proxy",
        refusals: &[
            &["no_entities", "invalid_uid"],
            &["entity_not_found", "not_insert"],
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.blocks.freeze",
        refusals: &[
            &["no_entities", "invalid_uid", "empty_name"],
            REVISION,
            &[
                "entity_not_found",
                "not_insert",
                "not_dynamic",
                "duplicate_block",
                "layer_locked",
            ],
        ],
        warnings: &["layer_locked"],
        targets: none,
    },
    Entry {
        id: "cad.blocks.replace",
        refusals: &[
            &["no_block", "same_block"],
            REVISION,
            &[
                "unknown_block",
                "no_entities",
                "invalid_uid",
                "entity_not_found",
                "not_insert",
                "layer_locked",
                "invalid_field",
            ],
        ],
        warnings: &["layer_locked"],
        targets: none,
    },
    Entry {
        id: "cad.blocks.query",
        refusals: &[&[
            "unknown_block",
            "layer_not_found",
            "invalid_condition",
            "unknown_tag",
        ]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.blocks.insert",
        refusals: &[
            &[
                "no_block",
                "not_finite",
                "invalid_scale",
                "unknown_block",
                "unknown_tag",
                "constant_attribute",
                "invalid_field",
                "not_dynamic",
                "unknown_parameter",
                "derived_parameter",
                "invalid_parameter_value",
                "slots_exhausted",
            ],
            LAYER,
            REVISION,
        ],
        warnings: &["layer_hidden", "insert_units_differ"],
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
        id: "cad.annotations.scales",
        refusals: &[
            OBJECTS,
            &[
                "no_scales",
                "no_scale",
                "invalid_scale",
                "no_value",
                "not_annotative",
                "own_scale",
                "scale_not_taken",
                "invalid_annotative",
            ],
            REVISION,
        ],
        warnings: &["layer_locked", "not_annotative"],
        targets: || vec![target(None, &["text", "dimension", "leader"], &[], Some("not_annotative"), None)],
    },
    Entry {
        id: "cad.entities.order",
        refusals: &[
            OBJECTS,
            &["order_limit", "invalid_reference", "not_same_layer"],
            REVISION,
        ],
        warnings: &["layer_locked"],
        targets: || vec![target(None, &["*"], &[], None, None)],
    },
    Entry {
        id: "cad.ucs.set",
        refusals: &[
            &[
                "not_finite",
                "far_point",
                "invalid_ucs_name",
                "same_points",
                "invalid_uid",
                "entity_not_found",
                "object_without_axis",
                "unknown_ucs",
                "too_many_ucs",
            ],
            REVISION,
        ],
        warnings: &["ucs_replaced"],
        targets: || {
            vec![target(
                Some("object"),
                UCS_KINDS,
                &[],
                Some("object_without_axis"),
                None,
            )]
        },
    },
    Entry {
        id: "cad.nurbs.create",
        refusals: &[
            &[
                "not_finite",
                "coincident_points",
                "invalid_nurbs",
                "invalid_line_weight",
            ],
            LAYER,
            REVISION,
            ATTRS,
        ],
        warnings: &["layer_hidden"],
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
                "not_a_dimension",
                "not_straight",
                "not_parallel",
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
            &["not_a_text", "empty_text", "too_few_objects", "not_splittable", "invalid_number"],
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
        warnings: &["curves_approximated", "gaps_closed", "islands_not_nested", "layer_hidden"],
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
                "invalid_table",
                "not_finite",
                "no_source",
                "source_missing",
                "source_unreadable",
            ],
            REVISION,
        ],
        warnings: &["source_missing", "source_unreadable", "source_unlinked"],
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
                "line_type_not_found",
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
                "layer_frozen",
                "active_frozen",
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
                "no_file",
                "invalid_states",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.fields.usage",
        refusals: &[&[
            "layer_not_found",
            "not_a_layer",
            "service_layer",
            "field_not_found",
        ]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.layers.find",
        refusals: &[&[
            "invalid_pattern",
            "invalid_color",
            "invalid_line_weight",
            "layer_not_found",
        ]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.linetypes.edit",
        refusals: &[
            &[
                "no_lin",
                "no_definition",
                "invalid_line_type",
                "empty_name",
                "line_type_not_found",
                "line_type_in_use",
                "invalid_line_type_scale",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.hatchpatterns.edit",
        refusals: &[
            &[
                "no_pat",
                "no_definition",
                "invalid_hatch_pattern",
                "empty_name",
                "hatch_pattern_not_found",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.library.edit",
        refusals: &[
            &[
                "no_item",
                "no_id",
                "no_kstil",
                "no_targets",
                "invalid_library_item",
                "invalid_kstil",
            ],
            REVISION,
            &[
                "library_item_exists",
                "library_item_not_found",
                "library_item_in_use",
                "not_a_symbol",
                "layer_not_found",
            ],
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.styles.impact",
        refusals: &[&[
            "empty_name",
            "layer_not_found",
            "style_not_found",
            "line_type_not_found",
        ]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.standards.check",
        refusals: &[&["no_standard", "standard_not_found", "invalid_standard"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.standards.apply",
        refusals: &[
            &[
                "no_standard",
                "standard_not_found",
                "invalid_standard",
                "layer_not_found",
                "not_a_group",
                "service_layer",
                "layer_locked",
            ],
            REVISION,
        ],
        warnings: &["layer_hidden"],
        targets: none,
    },
    Entry {
        id: "cad.layers.translate",
        refusals: &[
            &[
                "no_rules",
                "empty_name",
                "invalid_pattern",
                "invalid_color",
                "invalid_line_weight",
                "layer_not_found",
                "not_a_group",
                "service_layer",
                "layer_locked",
            ],
            REVISION,
        ],
        warnings: &["layer_hidden"],
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
                "no_library",
                "invalid_library",
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
                "style_needs_imperial",
            ],
            REVISION,
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.views.edit",
        refusals: &[
            &[
                "not_finite",
                "far_point",
                "invalid_view_size",
                "invalid_view_name",
            ],
            REVISION,
            &["unknown_view", "view_exists", "too_many_views"],
        ],
        warnings: &["view_replaced"],
        targets: none,
    },
    Entry {
        id: "cad.forms.edit",
        refusals: &[&["form_enforced", "form_withdrawn"], 
            &["invalid_form_id", "invalid_form_name", "invalid_form"],
            REVISION,
            &[
                "unknown_form",
                "form_exists",
                "too_many_forms",
                "layer_not_found",
                "not_a_layer",
                "form_draft_changed",
                "form_preflight",
                "form_unpublished",
                "form_item_unknown",
                "form_comment_unknown",
                "form_tests_failed",
            ],
        ],
        warnings: &["form_release_dropped", "form_breaking"],
        targets: none,
    },
    Entry {
        id: "cad.forms.validate",
        refusals: &[
            &["invalid_uid", "invalid_moment", "invalid_user"],
            &[
                "unknown_form",
                "form_unpublished",
                "entity_not_found",
                "form_layer_mismatch",
            ],
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.forms.submit",
        refusals: &[&["form_withdrawn"], 
            &["invalid_uid", "invalid_moment", "invalid_user"],
            REVISION,
            &[
                "unknown_form",
                "form_unpublished",
                "form_preflight",
                "entity_not_found",
                "form_layer_mismatch",
                "layer_locked",
                "form_invalid",
                // docs/adr/0438 §6, §7: the screen's release and record.
                "form_release_changed",
                "record_changed",
                // docs/adr/0440 §2: the children.
                "unknown_relation",
                "relation_mismatch",
                "relation_field_unknown",
            ],
        ],
        warnings: &["form_check_warning", "form_required", "form_layer_rule"],
        targets: none,
    },
    Entry {
        id: "cad.forms.audit",
        refusals: &[
            &["invalid_count", "invalid_moment", "invalid_user"],
            &["unknown_form", "form_unpublished", "form_preflight"],
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.forms.recompute",
        refusals: &[&["form_withdrawn"], &[
            "invalid_moment",
            "invalid_user",
            "invalid_revision",
            "revision_conflict",
            "unknown_form",
            "form_unpublished",
            "form_preflight",
            "layer_locked",
        ]],
        warnings: &["form_calc_failed"],
        targets: none,
    },
    Entry {
        id: "cad.domains.edit",
        refusals: &[&[
            "invalid_revision",
            "revision_conflict",
            "unknown_domain",
            "layer_not_found",
            "not_a_layer",
            "field_not_found",
            "domain_exists",
            "invalid_domain",
            "domain_mismatch",
            "invalid_recode",
            "domain_code_in_use",
            "domain_in_use",
            "layer_locked",
        ]],
        warnings: &["domain_values_outside"],
        targets: none,
    },
    Entry {
        id: "cad.domains.export",
        refusals: &[&["unknown_domain"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.fields.migrate",
        refusals: &[&[
            "invalid_revision",
            "revision_conflict",
            "layer_not_found",
            "not_a_layer",
            "field_not_found",
            "layer_locked",
        ]],
        warnings: &["values_unfit"],
        targets: none,
    },
    Entry {
        id: "cad.attachments.inspect",
        refusals: &[&["invalid_count", "invalid_uid", "entity_not_found", "invalid_folder"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.schema.describe",
        refusals: &[&["invalid_count", "invalid_cursor", "layer_not_found"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.forms.export",
        refusals: &[&["unknown_form", "form_unpublished"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.forms.import",
        refusals: &[&[
            "invalid_revision",
            "revision_conflict",
            "invalid_package",
            "layer_not_found",
            "not_a_layer",
            "invalid_form_id",
            "invalid_form_name",
            "form_exists",
            "too_many_forms",
            "invalid_map",
            "field_not_found",
            "field_kind_mismatch",
            "form_fields_missing",
        ]],
        warnings: &["domain_kept"],
        targets: none,
    },
    Entry {
        id: "cad.forms.transition",
        refusals: &[&["form_withdrawn"], &[
            "invalid_uid",
            "invalid_moment",
            "invalid_user",
            "invalid_revision",
            "revision_conflict",
            "unknown_form",
            "form_unpublished",
            "form_preflight",
            "no_workflow",
            "entity_not_found",
            "form_layer_mismatch",
            "layer_locked",
            "unknown_step",
            "step_not_allowed",
            "record_locked",
            "form_invalid",
            "reason_required",
        ]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.forms.compare",
        refusals: &[&["unknown_form", "form_unpublished", "unknown_release"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.forms.test",
        refusals: &[&[
            "unknown_form",
            "form_unpublished",
            "form_preflight",
            "unknown_test",
        ]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.relations.edit",
        refusals: &[
            &["invalid_relation"],
            REVISION,
            &[
                "unknown_relation",
                "relation_exists",
                "too_many_relations",
                "layer_not_found",
                "not_a_layer",
                "relation_field_unknown",
                // docs/adr/0440 §3: a key changed under links; compositions in a circle.
                "relation_in_use",
                "relation_cycle",
            ],
        ],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.forms.fill",
        refusals: &[&["form_withdrawn"], 
            &["no_entities", "invalid_uid", "invalid_moment", "invalid_user"],
            REVISION,
            &[
                "unknown_form",
                "form_unpublished",
                "form_preflight",
                "entity_not_found",
                "form_layer_mismatch",
                "layer_locked",
                "form_invalid",
            ],
        ],
        warnings: &["form_check_warning"],
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
                "bad_color",
                "bad_value",
                "out_of_range",
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
        id: "cad.sources.describe",
        refusals: &[&["invalid_file_path", "file_not_found", "import_unreadable", "invalid_address", "source_unreachable"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.sources.edit",
        refusals: &[
            &["invalid_file_path", "file_not_found", "import_unreadable", "table_not_found", "invalid_address", "source_unreachable"],
            LAYER,
            &["crs_mismatch", "entity_exists", "source_bound", "source_differs", "source_not_found", "source_changed", "too_many_sources", "invalid_extent"],
        ],
        warnings: &["crs_undeclared", "import_losses"],
        targets: none,
    },
    Entry {
        id: "cad.fields.write",
        refusals: &[
            LAYER,
            &["nothing_to_set", "invalid_attribute", "invalid_uid", "entity_not_found", "field_not_found"],
            &["invalid_expression", "invalid_values"],
        ],
        warnings: &["values_skipped"],
        targets: none,
    },
    Entry {
        id: "cad.processing.tools",
        refusals: &[&["tools_unavailable", "invalid_input"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.processing.run",
        refusals: &[&[
            "tools_unavailable",
            "unknown_tool",
            "invalid_uid",
            "entity_not_found",
            "invalid_parameters",
            "tool_failed",
            "invalid_file_path",
            "file_exists",
            "file_unwritable",
            "nothing_to_export",
        ]],
        warnings: &["export_losses"],
        targets: none,
    },
    Entry {
        id: "cad.sources.sync",
        refusals: &[
            &["source_not_found", "layer_not_found", "layer_locked", "source_gone", "import_unreadable", "table_not_found", "source_unreachable"],
            &["source_readonly", "source_changed", "geometry_mismatch", "file_unwritable", "pending_changes", "invalid_extent"],
        ],
        warnings: &["export_losses"],
        targets: none,
    },
    Entry {
        id: "file.import.gpkg",
        refusals: &[
            &["invalid_file_path", "file_not_found", "import_unreadable", "table_not_found"],
            LAYER,
            &["crs_mismatch", "entity_exists", "nothing_to_import"],
        ],
        warnings: &["crs_undeclared", "import_losses"],
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
        id: "cad.points.construct",
        refusals: &[&[
            "no_constructions",
            "not_finite",
            "invalid_distance",
            "invalid_uid",
            "entity_not_found",
            "no_such_place",
            "piece_gone",
            "not_an_end",
            "no_straight_edge",
            "same_points",
        ]],
        warnings: &[],
        targets: || {
            vec![
                target(
                    Some("extension"),
                    &["line", "polyline", "arc"],
                    &[],
                    Some("not_an_end"),
                    Some(
                        "Uzantı çizginin, açık çoklu çizginin ve yayın ucundan başlar; çoklu çizginin yaylı son kenarı da.",
                    ),
                ),
                target(
                    Some("parallel"),
                    &[
                        "line",
                        "polyline",
                        "polygon",
                        "xline",
                        "ray",
                        "leader",
                        "dimension",
                        "hatch",
                        "table",
                        "image",
                        "raster",
                        "pointcloud",
                    ],
                    &[],
                    Some("no_straight_edge"),
                    Some(
                        "Paralelin doğrultusu nesnenin near'a en yakın düz kenarından: çizgiler, yolların düz kenarları, taramanın, tablonun ve resmin çerçevesi; eğrinin ve elipsin kirişleri kenar sayılmaz.",
                    ),
                ),
            ]
        },
    },
    Entry {
        id: "cad.curves.construct",
        refusals: &[&[
            "no_constructions",
            "not_finite",
            "invalid_uid",
            "entity_not_found",
            "no_such_place",
            "piece_gone",
            "same_points",
            "collinear_points",
            "invalid_angle",
            "invalid_radius",
            "radius_too_small",
            "invalid_distance",
            "chord_too_long",
            "invalid_half_axis",
            "not_tangent_kind",
            "no_tangent_circle",
            "not_an_end",
        ]],
        warnings: &[],
        targets: || {
            vec![
                target(
                    Some("circleTangentTangentRadius"),
                    TANGENT_KINDS,
                    &[],
                    Some("not_tangent_kind"),
                    Some(
                        "Doğru ve çoklu çizginin düz kenarı bütün doğrusuyla, yay ve yaylı kenar bütün çemberiyle sayılır.",
                    ),
                ),
                target(
                    Some("circleTangents"),
                    TANGENT_KINDS,
                    &[],
                    Some("not_tangent_kind"),
                    Some(
                        "Doğru ve çoklu çizginin düz kenarı bütün doğrusuyla, yay ve yaylı kenar bütün çemberiyle sayılır.",
                    ),
                ),
                target(
                    Some("arcContinue"),
                    &["line", "polyline", "arc"],
                    &[],
                    Some("not_an_end"),
                    Some(
                        "Devam çizginin, çoklu çizginin ya da yayın bitişinden yönüyle, başlangıcından tersine sürer; kapalı çoklu çizginin başlangıcı yoktur.",
                    ),
                ),
            ]
        },
    },
    Entry {
        id: "cad.entities.vertex",
        refusals: &[&[
            "invalid_uid",
            "invalid_revision",
            "revision_conflict",
            "invalid_arc_edge",
            "invalid_polyline",
            "entity_not_found",
            "layer_locked",
            "not_a_polyline",
            "not_a_path",
            "wrong_place",
            "hole_edit",
            "no_such_place",
            "too_few_corners",
            "too_few_points",
        ]],
        warnings: &["neighbour_locked", "neighbour_invalid"],
        targets: || {
            vec![target(
                None,
                &["line", "polyline", "polygon"],
                &[],
                None,
                None,
            )]
        },
    },
    Entry {
        id: "cad.entities.pick",
        refusals: &[&["not_finite", "invalid_distance"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.entities.relate",
        refusals: &[&[
            "no_objects",
            "invalid_input",
            "invalid_pattern",
            "entity_not_found",
            "too_many_pairs",
        ]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.entities.select",
        refusals: &[&[
            "not_finite",
            "too_few_points",
            "too_few_corners",
            "invalid_ring",
            "invalid_radius",
            "invalid_distance",
            "too_many_points",
            "invalid_uid",
            "entity_not_found",
            "no_objects",
            "layer_not_found",
            "not_a_layer",
        ]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "cad.snap.find",
        refusals: &[&["not_finite", "invalid_aperture", "no_snap_kinds"]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "file.package",
        refusals: &[&["invalid_file_path", "file_exists", "file_unwritable"]],
        warnings: &["package_left"],
        targets: none,
    },
    Entry {
        id: "file.export.gpkg",
        refusals: &[&[
            "invalid_file_path",
            "file_exists",
            "layer_not_found",
            "nothing_to_export",
            "file_unwritable",
        ]],
        warnings: &["export_losses"],
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
        id: "file.export.block",
        refusals: &[&[
            "invalid_file_path",
            "file_exists",
            "no_block",
            "unknown_block",
            "invalid_uid",
            "entity_not_found",
            "nothing_to_export",
            "not_finite",
            "file_unwritable",
        ]],
        warnings: &[],
        targets: none,
    },
    Entry {
        id: "file.import.block",
        refusals: &[
            &["invalid_file_path"],
            &["file_not_found", "import_unreadable"],
            REVISION,
            &["nothing_to_import", "unknown_block", "block_refused"],
        ],
        warnings: &["objects_left_out"],
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
