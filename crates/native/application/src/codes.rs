//! The stable codes of the drawing commands' answers (`CommandError.code`,
//! `CommandWarning.code`; docs/adr/0022, 0027, 0029, 0032, 0037, 0047, 0066). Programs branch on
//! these, never on the Turkish message.

/// A closed area with fewer than 3 corners (`cad.polygon.create`).
pub const TOO_FEW_CORNERS: &str = "too_few_corners";
/// A polyline with fewer than 2 points (`cad.polyline.create`).
pub const TOO_FEW_POINTS: &str = "too_few_points";
/// A polyline's part with holes: only an area's parts have them (docs/adr/0174).
pub const PART_HOLES: &str = "part_holes";
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
/// An affine or projective transform whose linear part squashes the plane
/// (`cad.entities.transform`, docs/adr/0156).
pub const INVALID_TRANSFORM: &str = "invalid_transform";
/// A point of an object beyond a projective transform's horizon
/// (`cad.entities.transform`, docs/adr/0156).
pub const BEYOND_HORIZON: &str = "beyond_horizon";
/// Objects whose curves became straight vertices under a transform that is
/// not a similarity (`cad.entities.transform`, a warning).
pub const WARP_CURVES: &str = "warp_curves";
/// Texts, notes, blocks, dimensions and hatch patterns that kept their shape
/// under a transform that is not a similarity (`cad.entities.transform`, a warning).
pub const WARP_SHAPES: &str = "warp_shapes";
/// A rubber sheet's links that give no sheet: fewer than 3 or more than 1000,
/// two from one point, their sources on one line, no single solution
/// (`cad.entities.transform`, docs/adr/0158).
pub const INVALID_LINKS: &str = "invalid_links";
/// Shapes on a rubber sheet whose kept edges or curves lie over 0.1 mm from
/// their true image (`cad.entities.transform`, a warning, docs/adr/0158).
pub const RUBBER_BENDS: &str = "rubber_bends";
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
/// A new object's link to the object whose label it writes that cannot be
/// one: not a text's, `labelOf` without `labelScale` or the other way, an id
/// that is not a persistent id's text, a scale not finite and over 0
/// (`cad.entities.create`, docs/adr/0175 §4).
pub const INVALID_LINK: &str = "invalid_link";
/// A hatch's pattern or tie that cannot be one (docs/adr/0186 §1, §6): a
/// kind's field another kind's, a spacing, scale or family out of bounds, a
/// gradient's colour not `#RRGGBB`, a tie's seed not finite or an object
/// named twice (`cad.entities.create`, `cad.entities.edit`).
pub const INVALID_HATCH: &str = "invalid_hatch";
/// The object a linked text names is not in the drawing (`cad.entities.create`).
pub const LINK_NOT_FOUND: &str = "link_not_found";
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
/// `attributes` without its list (`cad.blocks.edit`, docs/adr/0144 §7).
pub const NO_ATTRIBUTES: &str = "no_attributes";
/// An attribute definition's tag empty or only white space (`cad.blocks.edit`).
pub const EMPTY_TAG: &str = "empty_tag";
/// An attribute definition's tag an earlier one of the list has (`cad.blocks.edit`).
pub const DUPLICATE_TAG: &str = "duplicate_tag";
/// An attribute definition's text height that is not a finite number above
/// zero (`cad.blocks.edit`).
pub const INVALID_HEIGHT: &str = "invalid_height";
/// A text's width factor not over 0 or over 100 (docs/adr/0145).
pub const INVALID_WIDTH_FACTOR: &str = "invalid_width_factor";
/// A multi-line text's box width, line spacing or letter formats out of their rules (docs/adr/0182 §6).
pub const INVALID_PARAGRAPH: &str = "invalid_paragraph";
/// A text's face or a dimension's look out of its rules: bold, italic or a
/// slant without a typeface, a slant or a size out of its bounds, too many
/// decimals, a prefix or suffix that cannot be written (docs/adr/0183 §9).
pub const INVALID_STYLE: &str = "invalid_style";
/// A text or dimension style id the project does not have (docs/adr/0183 §9).
pub const UNKNOWN_STYLE: &str = "unknown_style";
/// A dimension the core cannot draw, or a field its kind does not take
/// (a slope's elevations elsewhere, an ordinate's axis but 0 or 90; docs/adr/0147 §6).
pub const INVALID_DIMENSION: &str = "invalid_dimension";
/// A table's rows, columns, cells, merged ranges, alignments or source out
/// of their rules (docs/adr/0184 §6).
pub const INVALID_TABLE: &str = "invalid_table";
/// A table among the objects a block is defined from (docs/adr/0184 §1).
pub const TABLE_IN_BLOCK: &str = "table_in_block";
/// Tablo's edit (`table`, `tableUpdate`) changing an object that is not a
/// table, or into one that is not (docs/adr/0184 §6).
pub const NOT_A_TABLE: &str = "not_a_table";
/// A block rule the checks before it did not see, as the document said it
/// (never expected: the commands check the document's own rules first).
pub const BLOCK_REFUSED: &str = "block_refused";
