//! A drawing's objects as typed columns (docs/adr/0030, TODOS.md FILE-15):
//! how the browser hands a drawing between its page and its formats worker.
//! A JSON text of a large drawing, or a structured copy of every object,
//! took seconds and held the page; these are six typed arrays the two sides
//! pass without copying (transferable buffers), written and read by one loop
//! on each side. The page's side is `apps/web/src/io/columns.ts`, which
//! follows the same layout; the fixtures hold the two to each other. The
//! desktop compares a written drawing with the one read back through the
//! same packing (`first_difference`), every float bit for bit.
//!
//! Everything but the objects (name, settings, layers, styles …) stays the
//! contract's JSON: it is small, and the style engine's parts are JSON anyway.
//!
//! Six streams, each read front to back:
//!
//! - `kinds`: one byte per object, its index in [`KINDS`];
//! - `uids`: sixteen bytes per object, its persistent id;
//! - `ints` (u32): first the number of layer ids in the layer table, then per
//!   object its layer (table index), flags, attribute count and the kind's
//!   own counts;
//! - `floats` (f64, bit for bit: −0 stays −0): per object its own line
//!   weight when it has one, then the kind's numbers; NaN only in an
//!   elevation list, as a vertex without an elevation (below);
//! - `text` (UTF-16 code units, as JavaScript holds its strings) and
//!   `text_lengths` (code units of each text): first the layer table (layer
//!   ids in the order objects first use them), then per object its colour,
//!   label and symbol when it has them, its attributes as key, value pairs
//!   in key order (UTF-8 bytes, as the contract's map), then the kind's texts.
//!
//! | Kind | ints | floats | texts |
//! |---|---|---|---|
//! | every object | layer, flags, attributes | line weight? | colour?, label?, symbol?, key and value per attribute |
//! | point | | p, z? | |
//! | line | | a, b, za?, zb?, ma?, mb? | |
//! | polyline, polygon | n; m if bulges; e if elevations; v if measures; h if holes, then per hole: hole flags, k, j if bulges, d if elevations; q if parts, then per part: part flags, n, m if bulges, e if elevations, v if measures, h if holes and its holes as above; 2w if widths | pts (2n), bulges (m), zs (e), ms (v), per hole: pts (2k), bulges (j), zs (d); per part: pts, bulges, zs, ms, its holes'; widths (2w: each edge's start, end) | |
//! | circle | | c, r | |
//! | arc | | c, r, a0, a1 | |
//! | ellipse | | c, major, ratio, t0, t1 | |
//! | spline | n, closed (0 or 1) | pts (2n) | |
//! | nurbs | degree, n, m, curve flags (1 weights, 2 closed, 4 periodic, 8 fit, 16 fit tolerance, 32 start tangent, 64 end tangent), k if fit | ctrl (2n), knots (m), weights (n) if any, fit (2k) if any, fit tolerance if any, start and end tangents (2 each) if any | |
//! | xline, ray | | p, dir | |
//! | text | align if any (its place in `TextAlign::ALL`); r if runs, then per run: start, end, run flags; font if any (its place in `DrawingFont::ALL`); k if curved; b if its curve bends | p, height, rotation, width factor if any, label scale if linked, box width if any, line spacing if any, oblique if any, the curve's pts (2k) if curved, its bulges (b) if it bends | text, label's object if linked, each run's colour if it has one, text style if any, its overrides as a JSON list if any (option 14), its scales as JSON if any (option 15) |
//! | dimension | style if any; arrow if any (its place in `DimensionArrow::ALL`), decimals if any, unit if any (mm, cm, m), font if any; line flags if lines, then the dimension line's type and the extension lines' type if any (their places in `LINE_TYPES`); refs' mask if refs (bits 0–2 a, b, c; 3 the curve rule, 4 the edges rule, 3 and 4 the pair rule), then per reference its place's kind (`PLACE_KINDS`), its numbers and broken (0 or 1); its options, moved value, breaks, overrides and scales as one JSON text if more (bit 29) | a, b, offset, height, angle if any, c if any, za if any, zb if any, arrow size, ext offset, ext beyond, text gap if any; the dimension line's and the extension lines' weights if any | text if any, dimension style if any, prefix if any, suffix if any; the dimension line's, the extension lines' and the value's colours if any; per reference its object's id (UUID text) and why it broke if it did |
//! | hatch | n, pattern type; h if holes, then k per hole; f if families, then per family its dash count; gradient's shape and inverted (0 or 1) if any; i, k if tied | ring (2n), pattern angle, spacing, per hole: pts (2k); scale if any; per family: angle, origin, offset, dashes; seed if tied | name if any, gradient's second colour if any, tie's outer (empty for a face's tie), islands (i) and cutouts (k) ids (UUID text) if tied, then schema 69's boundary and the tie's bounds as one JSON object if any (docs/adr/0328) |
//! | insert | | p, scale, rotation | block (its id as UUID text); its moved attributes, parameters, clipping and level and phase (`arch`, option 5, docs/adr/0733 §1) as their texts if any |
//! | leader | n; arrow if any (its place in `LeaderArrow::ALL`) | pts (2n), height, rotation, arrow size if any | text if any, its scales as JSON if any (option 4) |
//! | table | n, m, r, then per cell row its length; q if merges, then row, col, rows, cols per range; a if aligns, then each its place in `TableAlign::ALL`; grid if any (its place in `TableGrid::ALL`); font if any; source if any: its kind (0 coordinates, 1 areas, 2 attributes, 3 file), then k objects, or a file's sheet flag (0 or 1) | p, rotation, height, rows (n), columns (m), oblique if any, frame if any | each cell (row by row), text style if any, the source's objects (UUID text) or the file's name and sheet, then schema 68's styles, formulas, fields, link and block schedule as one JSON object if any (docs/adr/0327) |
//! | image | n if clip | p, width, height, rotation, clip (2n) if clip, opacity if any | asset if any, file if any |
//! | raster | width, height, bands, sample (its place in `RASTER_SAMPLES`), srid; the look's kind (`RASTER_RENDERS`), its band count and bands, its stretch (`RASTER_STRETCHES`), its flags (`LOOK_*`); a dataset's dimension count and flags, each dimension's index, value count and flags | affine (6), opacity if any; the look's min, max, azimuth, altitude, z factor and nodata, each if any; each dimension's values | the look's ramp if any, its edges if any, asset if any, file if any, url if any; a dataset's variable, vector, mesh, each dimension's name and units; its georeference record as one JSON text if any (option 5, docs/adr/0518); its tile table as one JSON text if any (option 6, docs/adr/0627) |
//! | record | | | |
//! | wall | n, 1 if bulges | pts (2n), bulges (n) if any | the rest of its source (closed, width, justify, offset, style, components, caps, fill, height, arch, joins, cleanup) as one JSON object (docs/adr/0339, 0340) |
//! | space | | | its parts and the rest of its source (seed, method, number, name, usage, arch, height, finishes, broken) as one JSON object (docs/adr/0342) |
//! | opening | n | place (2n) | the rest of its source (host, at, anchor, width, form, leaves, operation, hand, side, swing, showSwing, frame, sill, head, depth, inset, style, number, broken) as one JSON object (docs/adr/0341, 0719) |
//! | grid, column, stair, ramp | | | its fields as one JSON object (docs/adr/0343) |
//! | tag | | | its fields (of, at, content, shape, size, leader, text, anchor, broken) as one JSON object (docs/adr/0344) |
//! | viewmark | | | its fields (form, points, radius, at, angle, flip, mark, target, size, label, broken) as one JSON object (docs/adr/0345) |
//! | slab | | | its fields (parts, role, thickness, offset, arch, material, edge, excluded) as one JSON object (docs/adr/0722) |
//! | pointcloud | srid, n (files); the look's kind (its place in `CloudRender::ALL`), its flags (`CLOUD_*`), the hidden classes' count and classes; per file its format (its place in `CloudFormat::ALL`) and source (1 asset, 2 file, 4 url; 8 added with a place, docs/adr/5100) | bounds (6), count, the look's size, opacity if any, the look's min and max if any; per file its count and bounds (6), its place (16) if any | the look's ramp if any; per file its asset, file or url |
//!
//! A point is two floats, x then y. Flags: 1 colour, 2 label, 4 symbol, 8
//! line weight (docs/adr/0139); a
//! kind's optional fields from bit 8 up, in the order the table names them
//! (point: z; line: za, zb, ma, mb (docs/adr/0224); polyline and polygon:
//! bulges, holes, zs, parts, ms (docs/adr/0224), closed (no value), widths
//! (docs/adr/0258; a polygon has neither, the columns carry them anyway so
//! that the encoder refuses them with their place); text: align, width factor, mask (no value; docs/adr/0145), link
//! (docs/adr/0175), box width, line spacing, runs (docs/adr/0182; a run's
//! flags: 1 bold, 2 italic, 4 underline, 8 raised, 16 lowered, 8 and 16 stacked (docs/adr/0324), 32 colour),
//! text style, font, bold (no value), italic (no value), oblique
//! (docs/adr/0183), curve, its bulges (docs/adr/0196), overrides (docs/adr/0322), scales (docs/adr/0323), layout
//! (docs/adr/0324), fields (docs/adr/0325); dimension: text, style, angle, c, mask (no value), za, zb
//! (docs/adr/0147), dimension style, arrow, arrow size, ext offset, ext
//! beyond, text gap, centred (no value), decimals, unit, prefix, suffix, font
//! (docs/adr/0183), and bit 27 (`LINES`) for its line fields (docs/adr/0205
//! §6): a line flags int follows (`LINE_*`: 1 dimension line colour, 2 its
//! weight, 4 its type, 8 extension lines' colour, 16 their weight, 32 their
//! type, 64 the value's colour), and bit 28 (`REFS`) for its references (docs/adr/0249 §4);
//! hatch: holes, name, scale, families, gradient, tie (docs/adr/0186), boundary or bounds (docs/adr/0328); insert: mirror, y factor (a float after the turn, docs/adr/0315 §3), moved attributes (a count, then each tag and point, docs/adr/0317 §1); leader: text, arrow, mask (no value;
//! docs/adr/0146), arrow size (docs/adr/0205 §7); table: merges, aligns, header (no value), grid, text
//! style, font, bold (no value), italic (no value), oblique, source, frame
//! (docs/adr/0184); image: mirror (no value), asset, file, clip, opacity
//! (docs/adr/0192); raster: asset, file, opacity (docs/adr/0204), url
//! (docs/adr/0207); pointcloud: opacity (docs/adr/0207)). A
//! hole's flags: 1 bulges, 2 elevations; a part's: 1 bulges, 2 elevations, 4
//! holes, 8 measures. Dimension styles and hatch pattern types are numbered in the
//! contract's order.
//!
//! Block definitions (docs/adr/0144) are not objects of the drawing: they
//! travel in the contract's JSON with the name, settings and layers.
//!
//! A multi-part area (docs/adr/0143) lays out its first part as every
//! polygon does, then its other parts, each as a polygon without flags of its
//! own would be: its flags, then its vertices, bulges, elevations and holes.
//!
//! Vertex elevations (docs/adr/0142): a line's ends are two optional floats.
//! A path's or a hole's `zs` is a list, its length (`e`, `d`) and then one
//! float per vertex; a vertex without an elevation is NaN, which an
//! elevation never is (a file holds finite numbers only), so the two cannot
//! meet: the reading maps NaN back to no elevation, and a NaN of any bit
//! pattern is the same in a comparison. The length is the vertex count in a
//! valid drawing; the columns carry it so that a list of another length
//! reaches the encoder, which refuses it with its place, instead of shifting
//! every number after it. A path's `zs` numbers follow its bulges and come
//! before its holes', though its flag (bit 10) is the last of the three.
//!
//! Vertex measures (docs/adr/0224) are laid out as elevations are: a line's
//! ends two optional floats after its elevations, a path's or a part's `ms`
//! a list after its `zs` (NaN a vertex without one). A polygon has none; the
//! columns carry one anyway so that the encoder refuses it with its place.

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{
    ArcEntity, AreaPart, BlockId, CircleEntity, ConstructionEntity, DimensionArrow,
    DimensionEntity, DimensionLook, DimensionRefs, DimensionStyle, DimensionTextPlace, DrawingFont,
    DrawingUnit, EllipseEntity, Entity, EntityBase, EntityId, GradientShape, HatchAssoc,
    HatchEntity, HatchGradient, HatchPattern, HatchPatternType, ImageEntity, ImageFields,
    InsertEntity, LeaderArrow, LeaderEntity, LineEntity, LineType, LookPart, Paragraph, PathEntity,
    PatternLine, PointEntity, PointPart, RasterEntity, RasterFields, RasterRender, RasterSample,
    RasterStretch, RingGeometry, SplineEntity, SubPlace, SubRef, TableAlign, TableEntity,
    TableGrid, TableSource, TextAlign, TextEntity, TextFace, TextRun, TextScript, Vec2,
};

use crate::error::{Code, KcadError};

/// A raster's samples, numbered as the columns hold them.
pub const RASTER_SAMPLES: [RasterSample; 8] = [
    RasterSample::U8,
    RasterSample::I8,
    RasterSample::U16,
    RasterSample::I16,
    RasterSample::U32,
    RasterSample::I32,
    RasterSample::F32,
    RasterSample::F64,
];

/// The kinds, numbered as `kinds` holds them.
/// A raster look's kinds, numbered as the columns hold them.
pub const RASTER_RENDERS: [RasterRender; 6] = [
    RasterRender::Rgb,
    RasterRender::Gray,
    RasterRender::Palette,
    RasterRender::Ramp,
    RasterRender::Hillshade,
    RasterRender::RampShade,
];

/// A raster look's stretches, numbered as the columns hold them.
pub const RASTER_STRETCHES: [RasterStretch; 4] = [
    RasterStretch::None,
    RasterStretch::MinMax,
    RasterStretch::Percent,
    RasterStretch::Manual,
];

/// A raster look's optional fields, as its flags in the columns.
const LOOK_MIN: u32 = 1;
const LOOK_MAX: u32 = 2;
const LOOK_RAMP: u32 = 4;
const LOOK_INVERT: u32 = 8;
const LOOK_AZIMUTH: u32 = 16;
const LOOK_ALTITUDE: u32 = 32;
const LOOK_Z_FACTOR: u32 = 64;
const LOOK_NODATA: u32 = 128;
const LOOK_NEAREST: u32 = 256;
/// A mesh's lines' colour (docs/adr/0243 §5): a text after the ramp.
const LOOK_EDGES: u32 = 512;

/// A point cloud's look's flags (docs/adr/0207 §5).
const CLOUD_MIN: u32 = 1;
const CLOUD_MAX: u32 = 2;
const CLOUD_RAMP: u32 = 4;
const CLOUD_INVERT: u32 = 8;
const CLOUD_RGB8: u32 = 16;
const CLOUD_METRES: u32 = 32;
const CLOUD_SQUARE: u32 = 64;
/// A point cloud file's source.
const SOURCE_ASSET: u32 = 1;
const SOURCE_FILE: u32 = 2;
const SOURCE_URL: u32 = 4;
/// A point cloud file with a place in the drawing (docs/adr/5100): its 16 numbers follow its bounds.
const SOURCE_POSE: u32 = 8;

pub const KINDS: [&str; 34] = [
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
    "nurbs",
    "record",
    "wall",
    "opening",
    "space",
    "grid",
    "column",
    "stair",
    "ramp",
    "tag",
    "viewmark",
    "slab",
    "beam",
    "roof",
    "railing",
];

const COLOR: u32 = 1;
const LABEL: u32 = 2;
const SYMBOL: u32 = 4;
const WEIGHT: u32 = 8;
/// An object's labels pinned by hand (docs/adr/0212 §2): after the symbol, how many, then each pin's
/// bits (`PIN_CLASS` …) and its class (a text), place (two floats) and turn (a float) as it has them.
const PINS: u32 = 16;
const PIN_CLASS: u32 = 1;
const PIN_AT: u32 = 2;
const PIN_ROTATION: u32 = 4;
const PIN_HIDDEN: u32 = 8;
/// An object's attachments (docs/adr/0225 §1): after the pins, how many, then each one's bits (its
/// sources `ATT_ASSET`, `ATT_FILE`, `ATT_URL` and `ATT_NOTE`), its name, then each source it has in
/// that order and its note. All are carried as they are: the encoder refuses what the contract's
/// rules refuse (no source, or two).
const ATTACHMENTS: u32 = 32;
const ATT_ASSET: u32 = 1;
const ATT_FILE: u32 = 2;
const ATT_URL: u32 = 4;
const ATT_NOTE: u32 = 8;
/// An object's look (docs/adr/0260): after the attachments, its bits (`LOOK_TYPE` …), then its line type
/// (its place in `LineType::ALL`), scale, transparency and order (floats) and its by-layer parts (a mask in
/// `LookPart::ALL`'s order) as it has them. An area's mask (docs/adr/0310) is two bits alone: `LOOK_WIPEOUT` and,
/// with its frame drawn, `LOOK_FRAME`. Its own line type with a pattern (docs/adr/0313 §1) is `LOOK_PATTERN`, the
/// name a text after the rest.
const LOOK: u32 = 64;
const LOOK_TYPE: u32 = 1;
const LOOK_SCALE: u32 = 2;
const LOOK_TRANSPARENCY: u32 = 4;
const LOOK_ORDER: u32 = 8;
const LOOK_BY_LAYER: u32 = 16;
const LOOK_WIPEOUT: u32 = 32;
const LOOK_FRAME: u32 = 64;
const LOOK_PATTERN: u32 = 128;
/// A kind's optional fields, in the order the module's table names them.
const OPT: [u32; 19] = [
    1 << 8,
    1 << 9,
    1 << 10,
    1 << 11,
    1 << 12,
    1 << 13,
    1 << 14,
    1 << 15,
    1 << 16,
    1 << 17,
    1 << 18,
    1 << 19,
    1 << 20,
    1 << 21,
    1 << 22,
    1 << 23,
    1 << 24,
    1 << 25,
    1 << 26,
];

/// A dimension's units in the columns, by their place here (docs/adr/0183).
const UNITS: [DrawingUnit; 3] = [DrawingUnit::Mm, DrawingUnit::Cm, DrawingUnit::M];

/// A dimension's line fields follow in a line flags int (docs/adr/0205 §6):
/// the kind's optional fields ran out of the flags' bits.
const LINES: u32 = 1 << 27;
const LINE_COLOR: u32 = 1;
const LINE_WEIGHT: u32 = 2;
const LINE_TYPE: u32 = 4;
const EXT_COLOR: u32 = 8;
const EXT_WEIGHT: u32 = 16;
const EXT_LINE_TYPE: u32 = 32;
const TEXT_COLOR: u32 = 64;
/// A dimension's references follow (docs/adr/0249 §4): a mask int (1 `a`, 2 `b`, 4 `c`), then per
/// reference its place's kind (its place in `PLACE_KINDS`, plus [`IN_BLOCK`] when it has a block path),
/// the kind's numbers (an `along` place's `t` a float), the path's length and numbers when it has one
/// (docs/adr/0263 §3) and whether it broke (0 or 1); its object's id (UUID text) and, when it broke, why.
const REFS: u32 = 1 << 28;
/// A dimension's options, moved value and breaks follow (docs/adr/0321): one text, their JSON
/// (`{"options": …, "textShift": …, "breaks": […]}`, only those it has).
const MORE: u32 = 1 << 29;
/// A reference's place kinds, numbered as the columns hold them, and how many numbers each has.
const PLACE_KINDS: [(&str, usize); 7] = [
    ("node", 1),
    ("start", 0),
    ("end", 0),
    ("center", 0),
    ("vertex", 3),
    ("mid", 3),
    ("along", 3),
];
/// Added to a reference's kind when it has a block path (docs/adr/0263 §3).
const IN_BLOCK: u32 = 8;
/// The line types, numbered as the columns hold them (the contract's order).
const LINE_TYPES: [LineType; 4] = [
    LineType::Continuous,
    LineType::Dashed,
    LineType::Dashdot,
    LineType::Dotted,
];

/// A line type's place in `LINE_TYPES`.
fn line_type_place(t: LineType) -> u32 {
    LINE_TYPES.iter().position(|x| *x == t).unwrap_or(0) as u32
}

/// A multi-line text's run's flags in the columns (docs/adr/0182): its
/// format, and whether its colour follows.
const RUN_BOLD: u32 = 1;
const RUN_ITALIC: u32 = 2;
const RUN_UNDERLINE: u32 = 4;
const RUN_SCRIPT: u32 = 8 | 16;
const RUN_SUPER: u32 = 8;
const RUN_SUB: u32 = 16;
const RUN_COLOR: u32 = 32;

fn run_flags(r: &TextRun) -> u32 {
    let mut bits = 0;
    if r.bold {
        bits |= RUN_BOLD;
    }
    if r.italic {
        bits |= RUN_ITALIC;
    }
    if r.underline {
        bits |= RUN_UNDERLINE;
    }
    bits |= match r.script {
        Some(TextScript::Super) => RUN_SUPER,
        Some(TextScript::Sub) => RUN_SUB,
        // Both script bits: a stacked fraction (docs/adr/0324 §1).
        Some(TextScript::Stack) => RUN_SCRIPT,
        None => 0,
    };
    if r.color.is_some() {
        bits |= RUN_COLOR;
    }
    bits
}
const HOLE_BULGES: u32 = 1;
const HOLE_ELEVATIONS: u32 = 2;
const PART_BULGES: u32 = 1;
const PART_ELEVATIONS: u32 = 2;
const PART_HOLES: u32 = 4;
const PART_MEASURES: u32 = 8;

const DIMENSION_STYLES: [DimensionStyle; 13] = DimensionStyle::ALL;
const HATCH_PATTERNS: [HatchPatternType; 5] = [
    HatchPatternType::Solid,
    HatchPatternType::Lines,
    HatchPatternType::Cross,
    HatchPatternType::Pattern,
    HatchPatternType::Gradient,
];
const GRADIENT_SHAPES: [GradientShape; 3] = [
    GradientShape::Linear,
    GradientShape::Cylinder,
    GradientShape::Spherical,
];

/// A drawing's objects as typed columns (see the module comment).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Columns {
    pub kinds: Vec<u8>,
    pub uids: Vec<u8>,
    pub ints: Vec<u32>,
    pub floats: Vec<f64>,
    pub text: Vec<u16>,
    pub text_lengths: Vec<u32>,
}

impl Columns {
    /// How many objects they hold.
    pub fn len(&self) -> usize {
        self.kinds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kinds.is_empty()
    }

    fn clear(&mut self) {
        self.kinds.clear();
        self.uids.clear();
        self.ints.clear();
        self.floats.clear();
        self.text.clear();
        self.text_lengths.clear();
    }
}

fn kind_index(entity: &Entity) -> u8 {
    match entity {
        Entity::Point(_) => 0,
        Entity::Line(_) => 1,
        Entity::Polyline(_) => 2,
        Entity::Polygon(_) => 3,
        Entity::Circle(_) => 4,
        Entity::Arc(_) => 5,
        Entity::Ellipse(_) => 6,
        Entity::Spline(_) => 7,
        Entity::Xline(_) => 8,
        Entity::Ray(_) => 9,
        Entity::Text(_) => 10,
        Entity::Dimension(_) => 11,
        Entity::Hatch(_) => 12,
        Entity::Insert(_) => 13,
        Entity::Leader(_) => 14,
        Entity::Table(_) => 15,
        Entity::Image(_) => 16,
        Entity::Raster(_) => 17,
        Entity::PointCloud(_) => 18,
        Entity::Nurbs(_) => 19,
        Entity::Record(_) => 20,
        Entity::Wall(_) => 21,
        Entity::Opening(_) => 22,
        Entity::Space(_) => 23,
        Entity::Grid(_) => 24,
        Entity::Column(_) => 25,
        Entity::Stair(_) => 26,
        Entity::Ramp(_) => 27,
        Entity::Tag(_) => 28,
        Entity::ViewMark(_) => 29,
        Entity::Slab(_) => 30,
        Entity::Beam(_) => 31,
        Entity::Roof(_) => 32,
        Entity::Railing(_) => 33,
    }
}

/// A count as the columns hold it. No list of a drawing comes near 2³² items
/// (the file allows 2²⁴); one that did would not pass the encoder anyway.
fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

// ── Writing the columns ─────────────────────────────────────────────────

/// Appends objects to columns; one field at a time, in the module table's order.
struct Packer {
    out: Columns,
}

// ── Reading the columns ─────────────────────────────────────────────────

/// Reads the streams front to back; any read past an end is a broken set of columns.
struct Cursor<'c> {
    cols: &'c Columns,
    int: usize,
    float: usize,
    unit: usize,
    text: usize,
}

/// A typeface's place in `DrawingFont::ALL` (docs/adr/0183).
fn font_place(f: DrawingFont) -> u32 {
    count(DrawingFont::ALL.iter().position(|x| *x == f).unwrap_or(0))
}

/// The typeface at its place; a place past them is broken columns.
fn font_at(v: usize) -> Result<DrawingFont, KcadError> {
    DrawingFont::ALL
        .get(v)
        .copied()
        .ok_or_else(|| broken(&format!("yazı tipi {v}")))
}

fn broken(what: &str) -> KcadError {
    KcadError::new(
        Code::BadColumns,
        format!(
            "Çizim biçim modülüne bozuk aktarıldı ({what}); dosya yazılmadı, eski dosyaya dokunulmadı. Bu bir yazılım hatasıdır: sayfayı yenileyip yeniden deneyin ve durumu bildirin."
        ),
    )
}

/// A leader's schema 67 fields (docs/adr/0326) as the columns carry them: one JSON object.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Multileader {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    arms: Vec<kentos_contracts::LeaderArm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tip: Option<kentos_contracts::SubRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    blocks: Vec<kentos_contracts::EntityId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    leader_style: Option<String>,
}

fn multileader_json(
    arms: &[kentos_contracts::LeaderArm],
    tip: Option<&kentos_contracts::SubRef>,
    blocks: &[kentos_contracts::EntityId],
    style: Option<&str>,
) -> String {
    serde_json::to_string(&Multileader {
        arms: arms.to_vec(),
        tip: tip.cloned(),
        blocks: blocks.to_vec(),
        leader_style: style.map(str::to_owned),
    })
    .unwrap_or_default()
}

/// A table's schema 68 fields (docs/adr/0327) as the columns carry them: one JSON object.
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TableCells {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    styles: Vec<kentos_contracts::CellStyle>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    formulas: Vec<kentos_contracts::CellFormula>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    fields: Vec<kentos_contracts::CellField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    link: Option<TableLink>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    schedule: Option<TableSource>,
    /// Its parts (docs/adr/1030 §7).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    breaks: Option<kentos_contracts::TableBreaks>,
    /// Its table style, by id (docs/adr/1126 §3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    table_style: Option<String>,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TableLink {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    digest: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    copy: Option<Vec<Vec<String>>>,
    #[serde(default, skip_serializing_if = "kentos_contracts::SourceOverrides::is_keep")]
    overrides: kentos_contracts::SourceOverrides,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    key_column: Option<u32>,
}

fn table_cells_json(
    styles: &[kentos_contracts::CellStyle],
    formulas: &[kentos_contracts::CellFormula],
    fields: &[kentos_contracts::CellField],
    source: Option<&TableSource>,
    breaks: Option<&kentos_contracts::TableBreaks>,
    table_style: Option<&str>,
) -> Option<String> {
    let link = match source {
        Some(TableSource::File {
            path,
            digest,
            copy,
            overrides,
            key_column,
            ..
        }) if path.is_some() || digest.is_some() || copy.is_some() || !overrides.is_keep() || key_column.is_some() => Some(TableLink {
            path: path.clone(),
            digest: digest.clone(),
            copy: copy.clone(),
            overrides: *overrides,
            key_column: *key_column,
        }),
        _ => None,
    };
    let schedule = source
        .filter(|s| {
            matches!(
                s,
                TableSource::Blocks { .. }
                    | TableSource::Schedule { .. }
                    | TableSource::Extraction(_)
                    | TableSource::Zoning {}
                    | TableSource::Parking {}
                    | TableSource::Shelter {}
            )
        })
        .cloned();
    if styles.is_empty()
        && formulas.is_empty()
        && fields.is_empty()
        && link.is_none()
        && schedule.is_none()
        && breaks.is_none()
        && table_style.is_none()
    {
        return None;
    }
    serde_json::to_string(&TableCells {
        styles: styles.to_vec(),
        formulas: formulas.to_vec(),
        fields: fields.to_vec(),
        link,
        schedule,
        breaks: breaks.copied(),
        table_style: table_style.map(str::to_owned),
    })
    .ok()
}

// ── Comparing ───────────────────────────────────────────────────────────

/// Whether two slices of floats are the same bit for bit (−0 is not 0). Every
/// NaN is the same as every other: the only NaN a set of columns may hold is
/// an elevation list's vertex without one (see the module comment), and which
/// bit pattern a page's engine gave it is no difference.
/// Out of line: called once per object (see `Encoder::object`).
#[inline(never)]
fn same_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.to_bits() == y.to_bits() || (x.is_nan() && y.is_nan()))
}

/// Where the objects `cols` hold differ from `entities` with `uids` (the
/// drawing a written file reads back to), or `None`: every field of every
/// object, every float bit for bit; the slots are not compared (a file does
/// not keep them). One object at a time, so no second copy of a large
/// drawing is made.
pub fn differs(cols: &Columns, entities: &[Entity], uids: &[EntityId]) -> Option<String> {
    if entities.len() != cols.len() || uids.len() != cols.len() {
        return Some(format!(
            "nesne sayısı: {} gönderildi, {} geri okundu",
            cols.len(),
            entities.len()
        ));
    }
    let mut c = Cursor::new(cols);
    let Ok(table) = c.table() else {
        return Some("katman tablosu".to_owned());
    };
    let index: HashMap<&str, u32> = table
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), count(i)))
        .collect();
    let mut scratch = Packer {
        out: Columns::default(),
    };
    for (i, (entity, uid)) in entities.iter().zip(uids).enumerate() {
        let Some(&layer) = index.get(entity.base().layer_id.as_str()) else {
            return Some(format!("nesne {}: katman", i + 1));
        };
        scratch.out.clear();
        scratch.object(entity, uid, layer);
        let s = &scratch.out;
        let (ints, floats) = (c.int + s.ints.len(), c.float + s.floats.len());
        let (units, texts) = (c.unit + s.text.len(), c.text + s.text_lengths.len());
        let same = cols.kinds[i] == s.kinds[0]
            && cols.uids[i * 16..i * 16 + 16] == s.uids[..]
            && cols.ints.get(c.int..ints) == Some(&s.ints[..])
            && cols
                .floats
                .get(c.float..floats)
                .is_some_and(|f| same_bits(f, &s.floats))
            && cols.text.get(c.unit..units) == Some(&s.text[..])
            && cols.text_lengths.get(c.text..texts) == Some(&s.text_lengths[..]);
        if !same {
            return Some(format!("nesne {} ({})", i + 1, entity.kind()));
        }
        (c.int, c.float, c.unit, c.text) = (ints, floats, units, texts);
    }
    (!c.at_end()).then(|| "nesnelerden sonra fazladan değer".to_owned())
}

/// The first object where two drawings' objects differ (every field, every
/// float bit for bit; not the slots), or `None`. Both sides are packed one
/// object at a time.
pub fn first_difference(
    a: &[Entity],
    a_uids: &[EntityId],
    b: &[Entity],
    b_uids: &[EntityId],
) -> Option<usize> {
    if a.len() != b.len() || a_uids.len() != a.len() || b_uids.len() != b.len() {
        return Some(a.len().min(b.len()));
    }
    let mut same = Same::default();
    (0..a.len()).find(|&i| same.differs(&a[i], &a_uids[i], &b[i], &b_uids[i]))
}

/// Two objects compared through their typed columns, the buffers kept from one pair to the next.
#[derive(Default)]
pub(crate) struct Same {
    x: Columns,
    y: Columns,
}

impl Same {
    /// Whether the objects differ: every field, every float bit for bit; the slots apart.
    pub(crate) fn differs(
        &mut self,
        a: &Entity,
        a_uid: &EntityId,
        b: &Entity,
        b_uid: &EntityId,
    ) -> bool {
        let mut x = Packer {
            out: std::mem::take(&mut self.x),
        };
        let mut y = Packer {
            out: std::mem::take(&mut self.y),
        };
        x.out.clear();
        y.out.clear();
        x.object(a, a_uid, 0);
        y.object(b, b_uid, 0);
        let (p, q) = (&x.out, &y.out);
        let differs = a.base().layer_id != b.base().layer_id
            || p.kinds != q.kinds
            || p.uids != q.uids
            || p.ints != q.ints
            || !same_bits(&p.floats, &q.floats)
            || p.text != q.text
            || p.text_lengths != q.text_lengths;
        self.x = x.out;
        self.y = y.out;
        differs
    }
}

#[cfg(test)]
mod tests;

mod pack;
pub use pack::*;

mod unpack;
pub use unpack::*;
