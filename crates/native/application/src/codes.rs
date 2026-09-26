//! The stable codes of the drawing commands' answers (`CommandError.code`,
//! `CommandWarning.code`; docs/adr/0022, 0027, 0029). Programs branch on
//! these, never on the Turkish message.

/// A closed area with fewer than 3 corners (`cad.polygon.create`).
pub const TOO_FEW_CORNERS: &str = "too_few_corners";
/// A polyline with fewer than 2 points (`cad.polyline.create`).
pub const TOO_FEW_POINTS: &str = "too_few_points";
/// A coordinate or a bulge that is NaN or ±∞.
pub const NOT_FINITE: &str = "not_finite";
/// Bulges given, but not one per edge.
pub const BULGE_COUNT: &str = "bulge_count";
pub const INVALID_REVISION: &str = "invalid_revision";
pub const REVISION_CONFLICT: &str = "revision_conflict";
pub const LAYER_NOT_FOUND: &str = "layer_not_found";
pub const NOT_A_LAYER: &str = "not_a_layer";
pub const LAYER_LOCKED: &str = "layer_locked";
/// The desktop only: every slot (`u32`) of the document has been given out.
pub const SLOTS_EXHAUSTED: &str = "slots_exhausted";
/// No object named (`cad.entities.delete`).
pub const NO_ENTITIES: &str = "no_entities";
/// A persistent id that is not lowercase UUID text with hyphens.
pub const INVALID_UID: &str = "invalid_uid";
/// A persistent id no object of the document has.
pub const ENTITY_NOT_FOUND: &str = "entity_not_found";
/// A warning: the layer is hidden, by itself or a group above it.
pub const LAYER_HIDDEN: &str = "layer_hidden";
