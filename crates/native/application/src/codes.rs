//! The stable codes of the drawing commands' answers (`CommandError.code`,
//! `CommandWarning.code`; docs/adr/0022, 0027, 0029, 0032, 0037, 0047, 0066). Programs branch on
//! these, never on the Turkish message.

/// A closed area with fewer than 3 corners (`cad.polygon.create`).
pub const TOO_FEW_CORNERS: &str = "too_few_corners";
/// A polyline with fewer than 2 points (`cad.polyline.create`).
pub const TOO_FEW_POINTS: &str = "too_few_points";
/// A coordinate, a bulge, a radius, an angle or an elevation that is NaN or ±∞.
pub const NOT_FINITE: &str = "not_finite";
/// A circle's or an arc's radius that is not above zero (`cad.circle.create`, `cad.arc.create`).
pub const INVALID_RADIUS: &str = "invalid_radius";
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
/// A scale factor that is not above zero (`cad.entities.transform`).
pub const INVALID_FACTOR: &str = "invalid_factor";
/// A mirror axis whose two points give it no direction (`cad.entities.transform`).
pub const INVALID_AXIS: &str = "invalid_axis";
/// No change given (`cad.entities.edit`).
pub const NO_CHANGES: &str = "no_changes";
/// One object changed by two changes of the same edit (`cad.entities.edit`).
pub const REPEATED_ENTITY: &str = "repeated_entity";
/// An alignment's second pair given by half, or its points within a nanometre
/// of the first pair's (`cad.entities.transform`).
pub const INVALID_ALIGN: &str = "invalid_align";
/// Rows and columns, or a polar array's count, out of their range (`cad.entities.array`).
pub const INVALID_COUNT: &str = "invalid_count";
/// A grid direction with more than one place and no spacing (`cad.entities.array`).
pub const INVALID_SPACING: &str = "invalid_spacing";
/// A polar array's fill of zero or past a full turn (`cad.entities.array`).
pub const INVALID_FILL: &str = "invalid_fill";
/// A path array's path that is not a line, an arc, a circle or a polyline,
/// or has no length (`cad.entities.array`, docs/adr/0140).
pub const INVALID_PATH: &str = "invalid_path";
/// A warning: an edit could not carry the elevations of an object's vertices
/// to what it wrote (docs/adr/0142).
pub const ELEVATION_LOST: &str = "elevation_lost";
/// Elevations written with a geometry that are not one for each vertex
/// (`cad.entities.edit`, docs/adr/0142).
pub const INVALID_ELEVATIONS: &str = "invalid_elevations";
/// No object given (`cad.entities.create`).
pub const NO_OBJECTS: &str = "no_objects";
/// A text whose text is empty or only white space (`cad.entities.edit`, `cad.entities.create`).
pub const EMPTY_TEXT: &str = "empty_text";
/// No property given (`cad.entities.set`).
pub const NOTHING_TO_SET: &str = "nothing_to_set";
/// An attribute name empty or only white space (`cad.entities.set`).
pub const INVALID_ATTRIBUTE: &str = "invalid_attribute";
/// A line weight that is not a number from 0 to 100 mm (`cad.entities.set`, docs/adr/0139).
pub const INVALID_LINE_WEIGHT: &str = "invalid_line_weight";
/// An insert's scale that is not above zero (its geometry, docs/adr/0144).
pub const INVALID_SCALE: &str = "invalid_scale";
/// A block the drawing does not define (an insert's geometry, `cad.blocks.edit`).
pub const UNKNOWN_BLOCK: &str = "unknown_block";
/// A block name empty or only white space (`cad.blocks.define`, `cad.blocks.edit`).
pub const EMPTY_NAME: &str = "empty_name";
/// A block name the drawing already has, Turkish case folded.
pub const DUPLICATE_BLOCK: &str = "duplicate_block";
/// A definition that would hold itself, directly or through others.
pub const BLOCK_CYCLE: &str = "block_cycle";
/// Blocks that would be nested deeper than 16 levels.
pub const BLOCK_TOO_DEEP: &str = "block_too_deep";
/// `replace` without the layer the insert goes on.
pub const NO_LAYER: &str = "no_layer";
/// No block given where the operation needs one (`cad.blocks.edit`).
pub const NO_BLOCK: &str = "no_block";
/// No base point given to `rebase` (`cad.blocks.edit`).
pub const NO_BASE: &str = "no_base";
/// A definition an insert uses, in the drawing or in another definition (`cad.blocks.edit`).
pub const BLOCK_IN_USE: &str = "block_in_use";
/// A block rule the checks before it did not see, as the document said it
/// (never expected: the commands check the document's own rules first).
pub const BLOCK_REFUSED: &str = "block_refused";
