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
//! | line | | a, b, za?, zb? | |
//! | polyline, polygon | n; m if bulges; e if elevations; h if holes, then per hole: hole flags, k, j if bulges, d if elevations; q if parts, then per part: part flags, n, m if bulges, e if elevations, h if holes and its holes as above | pts (2n), bulges (m), zs (e), per hole: pts (2k), bulges (j), zs (d); per part: pts, bulges, zs, its holes' | |
//! | circle | | c, r | |
//! | arc | | c, r, a0, a1 | |
//! | ellipse | | c, major, ratio, t0, t1 | |
//! | spline | n, closed (0 or 1) | pts (2n) | |
//! | xline, ray | | p, dir | |
//! | text | align if any (its place in `TextAlign::ALL`); r if runs, then per run: start, end, run flags; font if any (its place in `DrawingFont::ALL`) | p, height, rotation, width factor if any, label scale if linked, box width if any, line spacing if any, oblique if any | text, label's object if linked, each run's colour if it has one, text style if any |
//! | dimension | style if any; arrow if any (its place in `DimensionArrow::ALL`), decimals if any, unit if any (mm, cm, m), font if any | a, b, offset, height, angle if any, c if any, za if any, zb if any, arrow size, ext offset, ext beyond, text gap if any | text if any, dimension style if any, prefix if any, suffix if any |
//! | hatch | n, pattern type; h if holes, then k per hole | ring (2n), pattern angle, spacing, per hole: pts (2k) | |
//! | insert | | p, scale, rotation | block (its id as UUID text) |
//! | leader | n; arrow if any (its place in `LeaderArrow::ALL`) | pts (2n), height, rotation | text if any |
//! | table | n, m, r, then per cell row its length; q if merges, then row, col, rows, cols per range; a if aligns, then each its place in `TableAlign::ALL`; grid if any (its place in `TableGrid::ALL`); font if any; source if any: its kind (0 coordinates, 1 areas, 2 attributes, 3 file), then k objects, or a file's sheet flag (0 or 1) | p, rotation, height, rows (n), columns (m), oblique if any, frame if any | each cell (row by row), text style if any, the source's objects (UUID text) or the file's name and sheet |
//!
//! A point is two floats, x then y. Flags: 1 colour, 2 label, 4 symbol, 8
//! line weight (docs/adr/0139); a
//! kind's optional fields from bit 8 up, in the order the table names them
//! (point: z; line: za, zb; polyline and polygon: bulges, holes, zs; polygon:
//! parts; text: align, width factor, mask (no value; docs/adr/0145), link
//! (docs/adr/0175), box width, line spacing, runs (docs/adr/0182; a run's
//! flags: 1 bold, 2 italic, 4 underline, 8 raised, 16 lowered, 32 colour),
//! text style, font, bold (no value), italic (no value), oblique
//! (docs/adr/0183); dimension: text, style, angle, c, mask (no value), za, zb
//! (docs/adr/0147), dimension style, arrow, arrow size, ext offset, ext
//! beyond, text gap, centred (no value), decimals, unit, prefix, suffix, font
//! (docs/adr/0183);
//! hatch: holes; insert: mirror; leader: text, arrow, mask (no value;
//! docs/adr/0146); table: merges, aligns, header (no value), grid, text
//! style, font, bold (no value), italic (no value), oblique, source, frame
//! (docs/adr/0184)). A
//! hole's flags: 1 bulges, 2 elevations; a part's: 1 bulges, 2 elevations, 4
//! holes. Dimension styles and hatch pattern types are numbered in the
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

use std::collections::{BTreeMap, HashMap};

use kentos_contracts::{
    ArcEntity, AreaPart, BlockId, CircleEntity, ConstructionEntity, DimensionArrow,
    DimensionEntity, DimensionLook, DimensionStyle, DimensionTextPlace, DrawingFont, DrawingUnit,
    EllipseEntity, Entity, EntityBase, EntityId, HatchEntity, HatchPattern, HatchPatternType,
    InsertEntity, LeaderArrow, LeaderEntity, LineEntity, Paragraph, PathEntity, PointEntity,
    PointPart, RingGeometry, SplineEntity, TableAlign, TableEntity, TableGrid, TableSource,
    TextAlign, TextEntity, TextFace, TextRun, TextScript, Vec2,
};

use crate::error::{Code, KcadError};

/// The kinds, numbered as `kinds` holds them.
pub const KINDS: [&str; 16] = [
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
];

const COLOR: u32 = 1;
const LABEL: u32 = 2;
const SYMBOL: u32 = 4;
const WEIGHT: u32 = 8;
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

const DIMENSION_STYLES: [DimensionStyle; 10] = DimensionStyle::ALL;
const HATCH_PATTERNS: [HatchPatternType; 3] = [
    HatchPatternType::Solid,
    HatchPatternType::Lines,
    HatchPatternType::Cross,
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

impl Packer {
    fn int(&mut self, v: u32) {
        self.out.ints.push(v);
    }

    fn float(&mut self, x: f64) {
        self.out.floats.push(x);
    }

    fn point(&mut self, p: &Vec2) {
        let Vec2 { x, y } = *p;
        self.out.floats.extend([x, y]);
    }

    /// A list of points: its length, then its coordinates.
    fn points(&mut self, list: &[Vec2]) {
        self.int(count(list.len()));
        self.out.floats.reserve(list.len() * 2);
        for p in list {
            self.point(p);
        }
    }

    /// A list of numbers (bulges): its length, then the numbers.
    fn numbers(&mut self, list: &[f64]) {
        self.int(count(list.len()));
        self.out.floats.extend_from_slice(list);
    }

    /// A polygon's or a part's holes: their count, then each hole's flags,
    /// vertices, bulges and elevations.
    fn holes(&mut self, holes: &[RingGeometry]) {
        self.int(count(holes.len()));
        for RingGeometry { pts, bulges, zs } in holes {
            let mut hole = 0;
            if bulges.is_some() {
                hole |= HOLE_BULGES;
            }
            if zs.is_some() {
                hole |= HOLE_ELEVATIONS;
            }
            self.int(hole);
            self.points(pts);
            if let Some(b) = bulges {
                self.numbers(b);
            }
            if let Some(z) = zs {
                self.elevations(z);
            }
        }
    }

    /// A list of elevations: its length, then the numbers; NaN stands for a
    /// vertex without one.
    fn elevations(&mut self, list: &[Option<f64>]) {
        self.int(count(list.len()));
        self.out
            .floats
            .extend(list.iter().map(|z| z.unwrap_or(f64::NAN)));
    }

    fn text(&mut self, s: &str) {
        let start = self.out.text.len();
        self.out.text.extend(s.encode_utf16());
        let units = self.out.text.len() - start;
        self.out.text_lengths.push(count(units));
    }

    /// One object, its layer already a table index. Out of line: see `Encoder::object`.
    #[inline(never)]
    fn object(&mut self, entity: &Entity, uid: &EntityId, layer: u32) {
        self.out.kinds.push(kind_index(entity));
        self.out.uids.extend_from_slice(&uid.0);
        // Every field is named, so a field added to the contract cannot be left out unnoticed.
        let EntityBase {
            id: _slot,
            layer_id: _,
            color,
            attrs,
            label,
            symbol,
            line_weight,
        } = entity.base();
        let flags_at = self.out.ints.len() + 1;
        self.out.ints.extend([layer, 0, count(attrs.len())]);
        let mut flags = 0;
        if let Some(w) = line_weight {
            flags |= WEIGHT;
            self.float(*w);
        }
        for (bit, value) in [(COLOR, color), (LABEL, label), (SYMBOL, symbol)] {
            if let Some(v) = value {
                flags |= bit;
                self.text(v);
            }
        }
        for (key, value) in attrs {
            self.text(key);
            self.text(value);
        }
        flags |= self.geometry(entity);
        self.out.ints[flags_at] = flags;
    }

    /// The kind's own fields; returns their flags.
    fn geometry(&mut self, entity: &Entity) -> u32 {
        let mut flags = 0;
        match entity {
            Entity::Point(PointEntity {
                base: _,
                p,
                z,
                parts,
            }) => {
                self.point(p);
                if let Some(z) = z {
                    flags |= OPT[0];
                    self.float(*z);
                }
                // A multi-point object's other points (docs/adr/0174): each its flag, place and elevation.
                if let Some(parts) = parts {
                    flags |= OPT[1];
                    self.int(count(parts.len()));
                    for PointPart { p, z } in parts {
                        self.int(u32::from(z.is_some()));
                        self.point(p);
                        if let Some(z) = z {
                            self.float(*z);
                        }
                    }
                }
            }
            Entity::Line(LineEntity {
                base: _,
                a,
                b,
                za,
                zb,
            }) => {
                self.point(a);
                self.point(b);
                for (bit, z) in [(OPT[0], za), (OPT[1], zb)] {
                    if let Some(z) = z {
                        flags |= bit;
                        self.float(*z);
                    }
                }
            }
            Entity::Polyline(path) | Entity::Polygon(path) => {
                let PathEntity {
                    base: _,
                    pts,
                    bulges,
                    holes,
                    zs,
                    parts,
                } = path;
                self.points(pts);
                if let Some(b) = bulges {
                    flags |= OPT[0];
                    self.numbers(b);
                }
                if let Some(z) = zs {
                    flags |= OPT[2];
                    self.elevations(z);
                }
                if let Some(holes) = holes {
                    flags |= OPT[1];
                    self.holes(holes);
                }
                if let Some(parts) = parts {
                    flags |= OPT[3];
                    self.int(count(parts.len()));
                    for AreaPart {
                        pts,
                        bulges,
                        holes,
                        zs,
                    } in parts
                    {
                        let mut part = 0;
                        if bulges.is_some() {
                            part |= PART_BULGES;
                        }
                        if zs.is_some() {
                            part |= PART_ELEVATIONS;
                        }
                        if holes.is_some() {
                            part |= PART_HOLES;
                        }
                        self.int(part);
                        self.points(pts);
                        if let Some(b) = bulges {
                            self.numbers(b);
                        }
                        if let Some(z) = zs {
                            self.elevations(z);
                        }
                        if let Some(holes) = holes {
                            self.holes(holes);
                        }
                    }
                }
            }
            Entity::Circle(CircleEntity { base: _, c, r }) => {
                self.point(c);
                self.float(*r);
            }
            Entity::Arc(ArcEntity {
                base: _,
                c,
                r,
                a0,
                a1,
            }) => {
                self.point(c);
                self.out.floats.extend([*r, *a0, *a1]);
            }
            Entity::Ellipse(EllipseEntity {
                base: _,
                c,
                major,
                ratio,
                t0,
                t1,
            }) => {
                self.point(c);
                self.point(major);
                self.out.floats.extend([*ratio, *t0, *t1]);
            }
            Entity::Spline(SplineEntity {
                base: _,
                pts,
                closed,
            }) => {
                self.points(pts);
                self.int(u32::from(*closed));
            }
            Entity::Xline(line) | Entity::Ray(line) => {
                let ConstructionEntity { base: _, p, dir } = line;
                self.point(p);
                self.point(dir);
            }
            Entity::Text(TextEntity {
                base: _,
                p,
                text,
                height,
                rotation,
                align,
                width_factor,
                mask,
                label_of,
                label_scale,
                paragraph,
                face,
            }) => {
                self.point(p);
                self.out.floats.extend([*height, *rotation]);
                self.text(text);
                if let Some(a) = align {
                    flags |= OPT[0];
                    let at = TextAlign::ALL.iter().position(|x| x == a).unwrap_or(0);
                    self.int(count(at));
                }
                if let Some(w) = width_factor {
                    flags |= OPT[1];
                    self.float(*w);
                }
                if *mask {
                    flags |= OPT[2];
                }
                // A linked text's object, as its text, and scale (docs/adr/0175 §4); the codec checks the pair.
                if label_of.is_some() || label_scale.is_some() {
                    flags |= OPT[3];
                    self.text(&label_of.map(|u| u.to_text()).unwrap_or_default());
                    self.float(label_scale.unwrap_or(f64::NAN));
                }
                // A multi-line text's box, line spacing and runs (docs/adr/0182): each run its
                // range, its flags (`run_flags`) and, with RUN_COLOR, its colour.
                if let Some(w) = paragraph.box_width {
                    flags |= OPT[4];
                    self.float(w);
                }
                if let Some(s) = paragraph.line_spacing {
                    flags |= OPT[5];
                    self.float(s);
                }
                if !paragraph.runs.is_empty() {
                    flags |= OPT[6];
                    self.int(count(paragraph.runs.len()));
                    for r in &paragraph.runs {
                        self.int(r.start);
                        self.int(r.end);
                        self.int(run_flags(r));
                        if let Some(c) = &r.color {
                            self.text(c);
                        }
                    }
                }
                // Its face (docs/adr/0183 §2): the style's id, the typeface's place, bold and italic as
                // flags, the slant.
                if let Some(id) = &face.text_style {
                    flags |= OPT[7];
                    self.text(id);
                }
                if let Some(f) = face.font {
                    flags |= OPT[8];
                    self.int(font_place(f));
                }
                if face.bold {
                    flags |= OPT[9];
                }
                if face.italic {
                    flags |= OPT[10];
                }
                if let Some(o) = face.oblique {
                    flags |= OPT[11];
                    self.float(o);
                }
            }
            Entity::Dimension(DimensionEntity {
                base: _,
                a,
                b,
                offset,
                height,
                text,
                style,
                angle,
                c,
                mask,
                za,
                zb,
                look,
            }) => {
                self.point(a);
                self.point(b);
                self.out.floats.extend([*offset, *height]);
                if let Some(t) = text {
                    flags |= OPT[0];
                    self.text(t);
                }
                if let Some(s) = style {
                    flags |= OPT[1];
                    let at = DIMENSION_STYLES.iter().position(|x| x == s).unwrap_or(0);
                    self.int(count(at));
                }
                if let Some(x) = angle {
                    flags |= OPT[2];
                    self.float(*x);
                }
                if let Some(c) = c {
                    flags |= OPT[3];
                    self.point(c);
                }
                if *mask {
                    flags |= OPT[4];
                }
                if let Some(z) = za {
                    flags |= OPT[5];
                    self.float(*z);
                }
                if let Some(z) = zb {
                    flags |= OPT[6];
                    self.float(*z);
                }
                // Its look (docs/adr/0183 §3), field by field in the contract's order.
                if let Some(id) = &look.dim_style {
                    flags |= OPT[7];
                    self.text(id);
                }
                if let Some(a) = look.arrow {
                    flags |= OPT[8];
                    let at = DimensionArrow::ALL
                        .iter()
                        .position(|x| *x == a)
                        .unwrap_or(0);
                    self.int(count(at));
                }
                for (k, v) in [
                    (9, look.arrow_size),
                    (10, look.ext_offset),
                    (11, look.ext_beyond),
                    (12, look.text_gap),
                ] {
                    if let Some(x) = v {
                        flags |= OPT[k];
                        self.float(x);
                    }
                }
                if look.text_place.is_some() {
                    flags |= OPT[13];
                }
                if let Some(d) = look.decimals {
                    flags |= OPT[14];
                    self.int(d);
                }
                if let Some(u) = look.unit {
                    flags |= OPT[15];
                    self.int(count(UNITS.iter().position(|x| *x == u).unwrap_or(0)));
                }
                if let Some(t) = &look.prefix {
                    flags |= OPT[16];
                    self.text(t);
                }
                if let Some(t) = &look.suffix {
                    flags |= OPT[17];
                    self.text(t);
                }
                if let Some(f) = look.font {
                    flags |= OPT[18];
                    self.int(font_place(f));
                }
            }
            Entity::Hatch(HatchEntity {
                base: _,
                ring,
                holes,
                pattern,
            }) => {
                let HatchPattern {
                    kind,
                    angle,
                    spacing,
                } = pattern;
                self.points(ring);
                let at = HATCH_PATTERNS.iter().position(|x| x == kind).unwrap_or(0);
                self.int(count(at));
                self.out.floats.extend([*angle, *spacing]);
                if let Some(holes) = holes {
                    flags |= OPT[0];
                    self.int(count(holes.len()));
                    for hole in holes {
                        self.points(hole);
                    }
                }
            }
            Entity::Insert(InsertEntity {
                base: _,
                block,
                p,
                scale,
                rotation,
                mirror,
            }) => {
                self.point(p);
                self.out.floats.extend([*scale, *rotation]);
                self.text(&block.to_text());
                if *mirror {
                    flags |= OPT[0];
                }
            }
            Entity::Leader(LeaderEntity {
                base: _,
                pts,
                text,
                height,
                rotation,
                arrow,
                mask,
            }) => {
                self.points(pts);
                self.out.floats.extend([*height, *rotation]);
                if let Some(t) = text {
                    flags |= OPT[0];
                    self.text(t);
                }
                if let Some(a) = arrow {
                    flags |= OPT[1];
                    let at = LeaderArrow::ALL.iter().position(|x| x == a).unwrap_or(0);
                    self.int(count(at));
                }
                if *mask {
                    flags |= OPT[2];
                }
            }
            Entity::Table(TableEntity {
                base: _,
                p,
                rotation,
                height,
                rows,
                columns,
                cells,
                merges,
                aligns,
                header,
                grid,
                frame,
                face,
                source,
            }) => {
                self.point(p);
                self.out.floats.extend([*rotation, *height]);
                self.numbers(rows);
                self.numbers(columns);
                // Every cell row with its length: a ragged one reaches the encoder, which refuses it.
                self.int(count(cells.len()));
                for row in cells {
                    self.int(count(row.len()));
                    for words in row {
                        self.text(words);
                    }
                }
                if !merges.is_empty() {
                    flags |= OPT[0];
                    self.int(count(merges.len()));
                    for m in merges {
                        self.out.ints.extend([m.row, m.col, m.rows, m.cols]);
                    }
                }
                if let Some(list) = aligns {
                    flags |= OPT[1];
                    self.int(count(list.len()));
                    for a in list {
                        self.int(count(
                            TableAlign::ALL.iter().position(|x| x == a).unwrap_or(0),
                        ));
                    }
                }
                if *header {
                    flags |= OPT[2];
                }
                if let Some(g) = grid {
                    flags |= OPT[3];
                    self.int(count(
                        TableGrid::ALL.iter().position(|x| x == g).unwrap_or(0),
                    ));
                }
                if let Some(id) = &face.text_style {
                    flags |= OPT[4];
                    self.text(id);
                }
                if let Some(f) = face.font {
                    flags |= OPT[5];
                    self.int(font_place(f));
                }
                if face.bold {
                    flags |= OPT[6];
                }
                if face.italic {
                    flags |= OPT[7];
                }
                if let Some(o) = face.oblique {
                    flags |= OPT[8];
                    self.float(o);
                }
                // The frame's width (docs/adr/0184 §1) after the slant; its source's ints and texts are its own streams'.
                if let Some(f) = frame {
                    flags |= OPT[10];
                    self.float(*f);
                }
                if let Some(source) = source {
                    flags |= OPT[9];
                    match source {
                        TableSource::File { name, sheet } => {
                            self.int(3);
                            self.int(u32::from(sheet.is_some()));
                            self.text(name);
                            if let Some(s) = sheet {
                                self.text(s);
                            }
                        }
                        _ => {
                            let kind = match source {
                                TableSource::Coordinates { .. } => 0,
                                TableSource::Areas { .. } => 1,
                                _ => 2,
                            };
                            self.int(kind);
                            self.int(count(source.objects().len()));
                            for id in source.objects() {
                                self.text(&id.to_text());
                            }
                        }
                    }
                }
            }
        }
        flags
    }
}

/// Objects as columns, taking them: each is dropped once written, so a large
/// drawing is in memory about once, not twice. The layer table lists the
/// layer ids in the order the objects first use them.
pub fn pack(entities: Vec<Entity>, uids: Vec<EntityId>) -> Columns {
    let mut table: HashMap<String, u32> = HashMap::new();
    let mut order: Vec<&str> = Vec::new();
    for e in &entities {
        let id = e.base().layer_id.as_str();
        if !table.contains_key(id) {
            table.insert(id.to_owned(), count(order.len()));
            order.push(id);
        }
    }
    let mut packer = Packer {
        out: Columns::default(),
    };
    packer.int(count(order.len()));
    for id in &order {
        packer.text(id);
    }
    packer.out.kinds.reserve(entities.len());
    packer.out.uids.reserve(entities.len() * 16);
    for (entity, uid) in entities.into_iter().zip(uids) {
        let layer = table.get(&entity.base().layer_id).copied().unwrap_or(0);
        packer.object(&entity, &uid, layer);
    }
    packer.out
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

impl<'c> Cursor<'c> {
    fn new(cols: &'c Columns) -> Self {
        Self {
            cols,
            int: 0,
            float: 0,
            unit: 0,
            text: 0,
        }
    }

    fn int(&mut self) -> Result<u32, KcadError> {
        let v = *self
            .cols
            .ints
            .get(self.int)
            .ok_or_else(|| broken("tam sayılar erken bitti"))?;
        self.int += 1;
        Ok(v)
    }

    fn usize(&mut self) -> Result<usize, KcadError> {
        Ok(self.int()? as usize)
    }

    fn float(&mut self) -> Result<f64, KcadError> {
        let v = *self
            .cols
            .floats
            .get(self.float)
            .ok_or_else(|| broken("sayılar erken bitti"))?;
        self.float += 1;
        Ok(v)
    }

    fn point(&mut self) -> Result<Vec2, KcadError> {
        Ok(Vec2 {
            x: self.float()?,
            y: self.float()?,
        })
    }

    fn points(&mut self) -> Result<Vec<Vec2>, KcadError> {
        let n = self.usize()?;
        if n.saturating_mul(2) > self.cols.floats.len() - self.float {
            return Err(broken("nokta listesi sayılardan uzun"));
        }
        (0..n).map(|_| self.point()).collect()
    }

    fn numbers(&mut self) -> Result<Vec<f64>, KcadError> {
        let n = self.usize()?;
        let end = self
            .float
            .checked_add(n)
            .filter(|&end| end <= self.cols.floats.len())
            .ok_or_else(|| broken("sayı listesi sayılardan uzun"))?;
        let out = self.cols.floats[self.float..end].to_vec();
        self.float = end;
        Ok(out)
    }

    /// A polygon's or a part's holes (their count, then each hole's flags and lists).
    fn holes(&mut self) -> Result<Vec<RingGeometry>, KcadError> {
        let h = self.usize()?;
        if h > self.cols.ints.len() {
            return Err(broken("delik sayısı tam sayılardan fazla"));
        }
        let mut rings = Vec::with_capacity(h);
        for _ in 0..h {
            let hole = self.int()?;
            if hole & !(HOLE_BULGES | HOLE_ELEVATIONS) != 0 {
                return Err(broken(&format!("deliğin bayrakları {hole:#x}")));
            }
            rings.push(RingGeometry {
                pts: self.points()?,
                bulges: if hole & HOLE_BULGES != 0 {
                    Some(self.numbers()?)
                } else {
                    None
                },
                zs: if hole & HOLE_ELEVATIONS != 0 {
                    Some(self.elevations()?)
                } else {
                    None
                },
            });
        }
        Ok(rings)
    }

    /// A list of elevations (its length, then the numbers): NaN is a vertex without one.
    fn elevations(&mut self) -> Result<Vec<Option<f64>>, KcadError> {
        let n = self.usize()?;
        let end = self
            .float
            .checked_add(n)
            .filter(|&end| end <= self.cols.floats.len())
            .ok_or_else(|| broken("kot listesi sayılardan uzun"))?;
        let out = self.cols.floats[self.float..end]
            .iter()
            .map(|&z| (!z.is_nan()).then_some(z))
            .collect();
        self.float = end;
        Ok(out)
    }

    /// The next text; `where_` names it when it is not valid Unicode (a lone surrogate).
    fn text(&mut self, where_: impl FnOnce() -> String) -> Result<String, KcadError> {
        let units = *self
            .cols
            .text_lengths
            .get(self.text)
            .ok_or_else(|| broken("metinler erken bitti"))? as usize;
        let end = self
            .unit
            .checked_add(units)
            .filter(|&end| end <= self.cols.text.len())
            .ok_or_else(|| broken("metin uzunlukları metinden uzun"))?;
        let s = String::from_utf16(&self.cols.text[self.unit..end]).map_err(|_| {
            KcadError::unwritable(
                Code::InvalidUtf8,
                &where_(),
                "metinde eşi olmayan bir UTF-16 vekili var; KCAD 2 yalnız geçerli Unicode metin yazar",
            )
        })?;
        self.unit = end;
        self.text += 1;
        Ok(s)
    }

    /// The layer table at the head of the columns.
    fn table(&mut self) -> Result<Vec<String>, KcadError> {
        let n = self.usize()?;
        if n > self.cols.text_lengths.len() {
            return Err(broken("katman tablosu metinlerden uzun"));
        }
        (0..n)
            .map(|i| self.text(|| format!("katman {}", i + 1)))
            .collect()
    }

    fn at_end(&self) -> bool {
        self.int == self.cols.ints.len()
            && self.float == self.cols.floats.len()
            && self.unit == self.cols.text.len()
            && self.text == self.cols.text_lengths.len()
    }
}

/// The objects the columns hold, as the contract has them, with their
/// persistent ids; the slots are 1, 2, 3 … as a file reader gives them.
/// Columns that do not hold together are refused (`Code::BadColumns`); a
/// text that is not valid Unicode is refused with its place.
pub fn unpack(cols: &Columns) -> Result<(Vec<Entity>, Vec<EntityId>), KcadError> {
    let n = cols.kinds.len();
    if cols.uids.len() != n * 16 {
        return Err(broken("kimlikler nesne sayısıyla uyuşmuyor"));
    }
    let mut c = Cursor::new(cols);
    let table = c.table()?;
    let mut entities = Vec::with_capacity(n);
    let mut uids = Vec::with_capacity(n);
    for (i, &k) in cols.kinds.iter().enumerate() {
        entities.push(object(&mut c, &table, i, k)?);
        let mut id = [0u8; 16];
        id.copy_from_slice(&cols.uids[i * 16..i * 16 + 16]);
        uids.push(EntityId(id));
    }
    if !c.at_end() {
        return Err(broken("nesnelerden sonra fazladan değer var"));
    }
    Ok((entities, uids))
}

/// The `i`th object, of kind `k`. Out of line: see `Encoder::object`.
#[inline(never)]
fn object(c: &mut Cursor<'_>, table: &[String], i: usize, k: u8) -> Result<Entity, KcadError> {
    let kind = *KINDS
        .get(usize::from(k))
        .ok_or_else(|| broken(&format!("{}. nesnenin türü {k}", i + 1)))?;
    let place = |field: &str| format!("entities/{i} ({kind}) › {field}");
    let layer = table
        .get(c.usize()?)
        .ok_or_else(|| broken(&format!("{}. nesnenin katmanı tabloda yok", i + 1)))?
        .clone();
    let flags = c.int()?;
    let attr_count = c.usize()?;
    if attr_count > c.cols.text_lengths.len() {
        return Err(broken("öznitelik sayısı metinlerden fazla"));
    }
    let line_weight = (flags & WEIGHT != 0).then(|| c.float()).transpose()?;
    let color = (flags & COLOR != 0)
        .then(|| c.text(|| place("color")))
        .transpose()?;
    let label = (flags & LABEL != 0)
        .then(|| c.text(|| place("label")))
        .transpose()?;
    let symbol = (flags & SYMBOL != 0)
        .then(|| c.text(|| place("symbol")))
        .transpose()?;
    let mut attrs = BTreeMap::new();
    for _ in 0..attr_count {
        let key = c.text(|| place("attrs"))?;
        let value = c.text(|| place(&format!("attrs/{key}")))?;
        attrs.insert(key, value);
    }
    let base = EntityBase {
        id: count(i + 1),
        layer_id: layer,
        color,
        attrs,
        label,
        symbol,
        line_weight,
    };
    let known = COLOR | LABEL | SYMBOL | WEIGHT | allowed(k);
    if flags & !known != 0 {
        return Err(broken(&format!(
            "{}. nesnenin bayrakları {flags:#x}",
            i + 1
        )));
    }
    geometry(c, k, base, flags, &place)
}

/// The optional-field flags a kind may have.
fn allowed(kind: u8) -> u32 {
    match kind {
        0 => OPT[0] | OPT[1],
        1 => OPT[0] | OPT[1],
        // A polyline's parts are schema 17's (docs/adr/0174).
        2 => OPT[0] | OPT[1] | OPT[2] | OPT[3],
        3 => OPT[0] | OPT[1] | OPT[2] | OPT[3],
        // A text's alignment, width factor and mask, schema 18's link (docs/adr/0175 §4), schema 20's box,
        // spacing and runs, schema 21's face (docs/adr/0183); a leader's three.
        10 => OPT[..12].iter().fold(0, |m, b| m | b),
        14 => OPT[0] | OPT[1] | OPT[2],
        // A dimension: text, style, angle, c, schema 9's mask, za, zb (docs/adr/0147), schema 21's look.
        11 => OPT.iter().fold(0, |m, b| m | b),
        12 | 13 => OPT[0],
        // A table: merges, aligns, header, grid, its face, source, frame (docs/adr/0184).
        15 => OPT[..11].iter().fold(0, |m, b| m | b),
        _ => 0,
    }
}

fn geometry(
    c: &mut Cursor<'_>,
    kind: u8,
    base: EntityBase,
    flags: u32,
    place: &dyn Fn(&str) -> String,
) -> Result<Entity, KcadError> {
    let has = |bit: usize| flags & OPT[bit] != 0;
    Ok(match kind {
        0 => {
            let p = c.point()?;
            let z = if has(0) { Some(c.float()?) } else { None };
            let parts = if has(1) {
                let q = c.usize()?;
                if q > c.cols.ints.len() {
                    return Err(broken("nokta sayısı tam sayılardan fazla"));
                }
                let mut parts = Vec::with_capacity(q);
                for _ in 0..q {
                    let flag = c.int()?;
                    if flag > 1 {
                        return Err(broken(&format!("noktanın bayrağı {flag:#x}")));
                    }
                    let p = c.point()?;
                    let z = if flag == 1 { Some(c.float()?) } else { None };
                    parts.push(PointPart { p, z });
                }
                Some(parts)
            } else {
                None
            };
            Entity::Point(PointEntity { base, p, z, parts })
        }
        1 => Entity::Line(LineEntity {
            base,
            a: c.point()?,
            b: c.point()?,
            za: if has(0) { Some(c.float()?) } else { None },
            zb: if has(1) { Some(c.float()?) } else { None },
        }),
        2 | 3 => {
            let pts = c.points()?;
            let bulges = if has(0) { Some(c.numbers()?) } else { None };
            let zs = if has(2) { Some(c.elevations()?) } else { None };
            let holes = if has(1) { Some(c.holes()?) } else { None };
            let parts = if has(3) {
                let q = c.usize()?;
                if q > c.cols.ints.len() {
                    return Err(broken("parça sayısı tam sayılardan fazla"));
                }
                let mut parts = Vec::with_capacity(q);
                for _ in 0..q {
                    let part = c.int()?;
                    if part & !(PART_BULGES | PART_ELEVATIONS | PART_HOLES) != 0 {
                        return Err(broken(&format!("parçanın bayrakları {part:#x}")));
                    }
                    let pts = c.points()?;
                    let bulges = if part & PART_BULGES != 0 {
                        Some(c.numbers()?)
                    } else {
                        None
                    };
                    let zs = if part & PART_ELEVATIONS != 0 {
                        Some(c.elevations()?)
                    } else {
                        None
                    };
                    let holes = if part & PART_HOLES != 0 {
                        Some(c.holes()?)
                    } else {
                        None
                    };
                    parts.push(AreaPart {
                        pts,
                        bulges,
                        holes,
                        zs,
                    });
                }
                Some(parts)
            } else {
                None
            };
            let path = PathEntity {
                base,
                pts,
                bulges,
                holes,
                zs,
                parts,
            };
            if kind == 3 {
                Entity::Polygon(path)
            } else {
                Entity::Polyline(path)
            }
        }
        4 => Entity::Circle(CircleEntity {
            base,
            c: c.point()?,
            r: c.float()?,
        }),
        5 => Entity::Arc(ArcEntity {
            base,
            c: c.point()?,
            r: c.float()?,
            a0: c.float()?,
            a1: c.float()?,
        }),
        6 => Entity::Ellipse(EllipseEntity {
            base,
            c: c.point()?,
            major: c.point()?,
            ratio: c.float()?,
            t0: c.float()?,
            t1: c.float()?,
        }),
        7 => {
            let pts = c.points()?;
            let closed = match c.int()? {
                0 => false,
                1 => true,
                v => return Err(broken(&format!("eğrinin kapalılığı {v}"))),
            };
            Entity::Spline(SplineEntity { base, pts, closed })
        }
        8 | 9 => {
            let line = ConstructionEntity {
                base,
                p: c.point()?,
                dir: c.point()?,
            };
            if kind == 8 {
                Entity::Xline(line)
            } else {
                Entity::Ray(line)
            }
        }
        10 => {
            let (p, height, rotation) = (c.point()?, c.float()?, c.float()?);
            let text = c.text(|| place("text"))?;
            let align = if has(0) {
                let v = c.usize()?;
                Some(
                    *TextAlign::ALL
                        .get(v)
                        .ok_or_else(|| broken(&format!("yazı hizası {v}")))?,
                )
            } else {
                None
            };
            let width_factor = if has(1) { Some(c.float()?) } else { None };
            let (label_of, label_scale) = if has(3) {
                let of = c.text(|| place("labelOf"))?;
                let id = EntityId::parse(&of)
                    .ok_or_else(|| broken(&format!("bağlı yazının nesnesi “{of}”")))?;
                (Some(id), Some(c.float()?))
            } else {
                (None, None)
            };
            let box_width = if has(4) { Some(c.float()?) } else { None };
            let line_spacing = if has(5) { Some(c.float()?) } else { None };
            let runs = if has(6) {
                let n = c.usize()?;
                let mut runs = Vec::with_capacity(n.min(1024));
                for _ in 0..n {
                    let (start, end, bits) = (c.int()?, c.int()?, c.int()?);
                    let color = if bits & RUN_COLOR != 0 {
                        Some(c.text(|| place("runs"))?)
                    } else {
                        None
                    };
                    runs.push(TextRun {
                        start,
                        end,
                        bold: bits & RUN_BOLD != 0,
                        italic: bits & RUN_ITALIC != 0,
                        underline: bits & RUN_UNDERLINE != 0,
                        script: match bits & RUN_SCRIPT {
                            RUN_SUPER => Some(TextScript::Super),
                            RUN_SUB => Some(TextScript::Sub),
                            _ => None,
                        },
                        color,
                    });
                }
                runs
            } else {
                Vec::new()
            };
            let text_style = if has(7) {
                Some(c.text(|| place("textStyle"))?)
            } else {
                None
            };
            let font = if has(8) {
                Some(font_at(c.usize()?)?)
            } else {
                None
            };
            let oblique = if has(11) { Some(c.float()?) } else { None };
            Entity::Text(TextEntity {
                base,
                p,
                text,
                height,
                rotation,
                align,
                width_factor,
                mask: has(2),
                label_of,
                label_scale,
                paragraph: Paragraph {
                    box_width,
                    line_spacing,
                    runs,
                },
                face: TextFace {
                    text_style,
                    font,
                    bold: has(9),
                    italic: has(10),
                    oblique,
                },
            })
        }
        11 => {
            let (a, b) = (c.point()?, c.point()?);
            let (offset, height) = (c.float()?, c.float()?);
            let text = if has(0) {
                Some(c.text(|| place("text"))?)
            } else {
                None
            };
            let style = if has(1) {
                let v = c.usize()?;
                Some(
                    *DIMENSION_STYLES
                        .get(v)
                        .ok_or_else(|| broken(&format!("ölçü türü {v}")))?,
                )
            } else {
                None
            };
            let angle = if has(2) { Some(c.float()?) } else { None };
            let corner = if has(3) { Some(c.point()?) } else { None };
            let za = if has(5) { Some(c.float()?) } else { None };
            let zb = if has(6) { Some(c.float()?) } else { None };
            let dim_style = if has(7) {
                Some(c.text(|| place("dimStyle"))?)
            } else {
                None
            };
            let arrow = if has(8) {
                let v = c.usize()?;
                Some(
                    *DimensionArrow::ALL
                        .get(v)
                        .ok_or_else(|| broken(&format!("ölçü oku {v}")))?,
                )
            } else {
                None
            };
            let mut size = |k: usize| {
                if has(k) {
                    c.float().map(Some)
                } else {
                    Ok(None)
                }
            };
            let (arrow_size, ext_offset, ext_beyond, text_gap) =
                (size(9)?, size(10)?, size(11)?, size(12)?);
            let decimals = if has(14) { Some(c.int()?) } else { None };
            let unit = if has(15) {
                let v = c.usize()?;
                Some(
                    *UNITS
                        .get(v)
                        .ok_or_else(|| broken(&format!("ölçü birimi {v}")))?,
                )
            } else {
                None
            };
            let prefix = if has(16) {
                Some(c.text(|| place("prefix"))?)
            } else {
                None
            };
            let suffix = if has(17) {
                Some(c.text(|| place("suffix"))?)
            } else {
                None
            };
            let font = if has(18) {
                Some(font_at(c.usize()?)?)
            } else {
                None
            };
            Entity::Dimension(DimensionEntity {
                base,
                a,
                b,
                offset,
                height,
                text,
                style,
                angle,
                c: corner,
                mask: has(4),
                za,
                zb,
                look: DimensionLook {
                    dim_style,
                    arrow,
                    arrow_size,
                    ext_offset,
                    ext_beyond,
                    text_gap,
                    text_place: has(13).then_some(DimensionTextPlace::Centre),
                    decimals,
                    unit,
                    prefix,
                    suffix,
                    font,
                },
            })
        }
        12 => {
            let ring = c.points()?;
            let v = c.usize()?;
            let kind = *HATCH_PATTERNS
                .get(v)
                .ok_or_else(|| broken(&format!("tarama deseni {v}")))?;
            let (angle, spacing) = (c.float()?, c.float()?);
            let holes = if has(0) {
                let h = c.usize()?;
                if h > c.cols.ints.len() {
                    return Err(broken("delik sayısı tam sayılardan fazla"));
                }
                Some((0..h).map(|_| c.points()).collect::<Result<Vec<_>, _>>()?)
            } else {
                None
            };
            Entity::Hatch(HatchEntity {
                base,
                ring,
                holes,
                pattern: HatchPattern {
                    kind,
                    angle,
                    spacing,
                },
            })
        }
        15 => {
            let p = c.point()?;
            let (rotation, height) = (c.float()?, c.float()?);
            let rows = c.numbers()?;
            let columns = c.numbers()?;
            let r = c.usize()?;
            if r > c.cols.ints.len() {
                return Err(broken("tablonun satır sayısı tam sayılardan fazla"));
            }
            let mut cells = Vec::with_capacity(r);
            for i in 0..r {
                let len = c.usize()?;
                if len > c.cols.text_lengths.len() {
                    return Err(broken("tablonun hücre sayısı metinlerden fazla"));
                }
                cells.push(
                    (0..len)
                        .map(|j| c.text(|| place(&format!("cells/{i}/{j}"))))
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
            let merges = if has(0) {
                let q = c.usize()?;
                if q > c.cols.ints.len() {
                    return Err(broken("birleşik alan sayısı tam sayılardan fazla"));
                }
                (0..q)
                    .map(|_| {
                        Ok(kentos_contracts::CellRange {
                            row: c.int()?,
                            col: c.int()?,
                            rows: c.int()?,
                            cols: c.int()?,
                        })
                    })
                    .collect::<Result<Vec<_>, KcadError>>()?
            } else {
                Vec::new()
            };
            let aligns = if has(1) {
                let a = c.usize()?;
                if a > c.cols.ints.len() {
                    return Err(broken("hiza sayısı tam sayılardan fazla"));
                }
                Some(
                    (0..a)
                        .map(|_| {
                            let v = c.usize()?;
                            TableAlign::ALL
                                .get(v)
                                .copied()
                                .ok_or_else(|| broken(&format!("sütun hizası {v}")))
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                )
            } else {
                None
            };
            let grid = if has(3) {
                let v = c.usize()?;
                Some(
                    *TableGrid::ALL
                        .get(v)
                        .ok_or_else(|| broken(&format!("tablo çizgileri {v}")))?,
                )
            } else {
                None
            };
            let text_style = if has(4) {
                Some(c.text(|| place("textStyle"))?)
            } else {
                None
            };
            let font = if has(5) {
                Some(font_at(c.usize()?)?)
            } else {
                None
            };
            let oblique = if has(8) { Some(c.float()?) } else { None };
            let frame = if has(10) { Some(c.float()?) } else { None };
            let source = if has(9) {
                match c.int()? {
                    3 => {
                        let sheet = match c.int()? {
                            0 => false,
                            1 => true,
                            v => return Err(broken(&format!("kaynağın sayfa bayrağı {v}"))),
                        };
                        let name = c.text(|| place("source/name"))?;
                        let sheet = if sheet {
                            Some(c.text(|| place("source/sheet"))?)
                        } else {
                            None
                        };
                        Some(TableSource::File { name, sheet })
                    }
                    kind @ 0..=2 => {
                        let k = c.usize()?;
                        if k > c.cols.text_lengths.len() {
                            return Err(broken("kaynak nesne sayısı metinlerden fazla"));
                        }
                        let objects = (0..k)
                            .map(|_| {
                                let t = c.text(|| place("source/objects"))?;
                                EntityId::parse(&t).ok_or_else(|| {
                                    broken(&format!("kaynak nesnenin kimliği “{t}”"))
                                })
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        Some(match kind {
                            0 => TableSource::Coordinates { objects },
                            1 => TableSource::Areas { objects },
                            _ => TableSource::Attributes { objects },
                        })
                    }
                    v => return Err(broken(&format!("tablonun kaynağı {v}"))),
                }
            } else {
                None
            };
            Entity::Table(TableEntity {
                base,
                p,
                rotation,
                height,
                rows,
                columns,
                cells,
                merges,
                aligns,
                header: has(2),
                grid,
                frame,
                face: TextFace {
                    text_style,
                    font,
                    bold: has(6),
                    italic: has(7),
                    oblique,
                },
                source,
            })
        }
        13 => {
            let p = c.point()?;
            let (scale, rotation) = (c.float()?, c.float()?);
            let text = c.text(|| place("block"))?;
            let block =
                BlockId::parse(&text).ok_or_else(|| broken(&format!("blok kimliği “{text}”")))?;
            Entity::Insert(InsertEntity {
                base,
                block,
                p,
                scale,
                rotation,
                mirror: has(0),
            })
        }
        _ => {
            let pts = c.points()?;
            let (height, rotation) = (c.float()?, c.float()?);
            let text = if has(0) {
                Some(c.text(|| place("text"))?)
            } else {
                None
            };
            let arrow = if has(1) {
                let v = c.usize()?;
                Some(
                    *LeaderArrow::ALL
                        .get(v)
                        .ok_or_else(|| broken(&format!("kılavuz oku {v}")))?,
                )
            } else {
                None
            };
            Entity::Leader(LeaderEntity {
                base,
                pts,
                text,
                height,
                rotation,
                arrow,
                mask: has(2),
            })
        }
    })
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
    let mut x = Packer {
        out: Columns::default(),
    };
    let mut y = Packer {
        out: Columns::default(),
    };
    (0..a.len()).find(|&i| {
        x.out.clear();
        y.out.clear();
        x.object(&a[i], &a_uids[i], 0);
        y.object(&b[i], &b_uids[i], 0);
        let (p, q) = (&x.out, &y.out);
        a[i].base().layer_id != b[i].base().layer_id
            || p.kinds != q.kinds
            || p.uids != q.uids
            || p.ints != q.ints
            || !same_bits(&p.floats, &q.floats)
            || p.text != q.text
            || p.text_lengths != q.text_lengths
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(layer: &str) -> EntityBase {
        EntityBase {
            id: 7,
            layer_id: layer.into(),
            color: None,
            attrs: BTreeMap::new(),
            label: None,
            symbol: None,
            line_weight: None,
        }
    }

    fn id(n: u8) -> EntityId {
        let mut b = [0u8; 16];
        b[15] = n;
        b[6] = 0x70;
        EntityId(b)
    }

    #[test]
    fn a_point_is_laid_out_as_the_table_says() {
        let mut b = base("0");
        b.label = Some("P1".into());
        b.attrs.insert("Ad".into(), "Nokta".into());
        let p = Entity::Point(PointEntity {
            base: b,
            p: Vec2 { x: -0.0, y: 2.5 },
            z: Some(12.0),
            parts: None,
        });
        let cols = pack(vec![p], vec![id(1)]);
        assert_eq!(cols.kinds, [0]);
        assert_eq!(cols.ints, [1, 0, LABEL | OPT[0], 1]);
        assert_eq!(cols.floats[0].to_bits(), (-0.0f64).to_bits());
        assert_eq!(&cols.floats[1..], [2.5, 12.0]);
        let texts: Vec<String> = {
            let mut at = 0;
            cols.text_lengths
                .iter()
                .map(|&n| {
                    let s = String::from_utf16(&cols.text[at..at + n as usize]).unwrap_or_default();
                    at += n as usize;
                    s
                })
                .collect()
        };
        assert_eq!(texts, ["0", "P1", "Ad", "Nokta"]);
    }

    #[test]
    fn unpacking_gives_back_the_objects_and_refuses_what_does_not_hold_together() {
        let entities = vec![
            Entity::Line(LineEntity {
                base: base("a"),
                a: Vec2 { x: 1.0, y: 2.0 },
                b: Vec2 { x: 3.0, y: 4.0 },
                za: None,
                zb: None,
            }),
            Entity::Hatch(HatchEntity {
                base: base("b"),
                ring: vec![Vec2 { x: 0.0, y: 0.0 }; 3],
                holes: Some(vec![vec![Vec2 { x: 1.0, y: 1.0 }; 3]]),
                pattern: HatchPattern {
                    kind: HatchPatternType::Cross,
                    angle: 45.0,
                    spacing: 2.0,
                },
            }),
        ];
        let cols = pack(entities.clone(), vec![id(1), id(2)]);
        let (back, uids) = unpack(&cols).expect("unpacks");
        assert_eq!(uids, [id(1), id(2)]);
        assert_eq!(first_difference(&entities, &uids, &back, &uids), None);
        assert_eq!(differs(&cols, &back, &uids), None);
        // Slots are the reader's: 1, 2 …
        assert_eq!(back[1].base().id, 2);

        let mut short = cols.clone();
        short.floats.pop();
        assert_eq!(
            unpack(&short).map(|_| ()).unwrap_err().code,
            Code::BadColumns
        );
        let mut extra = cols.clone();
        extra.ints.push(0);
        assert_eq!(
            unpack(&extra).map(|_| ()).unwrap_err().code,
            Code::BadColumns
        );
        let mut kind = cols.clone();
        kind.kinds[0] = 13;
        assert_eq!(
            unpack(&kind).map(|_| ()).unwrap_err().code,
            Code::BadColumns
        );
        let mut lone = cols;
        lone.text[0] = 0xd800;
        let e = unpack(&lone).map(|_| ()).unwrap_err();
        assert_eq!(e.code, Code::InvalidUtf8);
    }

    #[test]
    fn elevations_are_laid_out_as_the_table_says() {
        let line = Entity::Line(LineEntity {
            base: base("0"),
            a: Vec2 { x: 1.0, y: 2.0 },
            b: Vec2 { x: 3.0, y: 4.0 },
            za: None,
            zb: Some(7.5),
        });
        let pts = vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 1.0, y: 1.0 },
            Vec2 { x: 2.0, y: 0.0 },
        ];
        let polygon = Entity::Polygon(PathEntity {
            base: base("0"),
            pts: pts.clone(),
            bulges: Some(vec![0.5]),
            holes: Some(vec![
                RingGeometry {
                    pts: pts.clone(),
                    bulges: None,
                    zs: Some(vec![None, Some(3.0), Some(-0.0)]),
                },
                RingGeometry {
                    pts,
                    bulges: Some(vec![0.25]),
                    zs: None,
                },
            ]),
            zs: Some(vec![Some(10.0), None, Some(12.0)]),
            parts: None,
        });
        let entities = vec![line, polygon];
        let uids = vec![id(1), id(2)];
        let cols = pack(entities.clone(), uids.clone());
        // The layer table (1); the line: layer, flags (zb), attributes; the polygon: layer, flags
        // (bulges, holes, elevations), attributes, n, m, e, h, then per hole its flags, k, j?, d?.
        assert_eq!(
            cols.ints,
            [
                1,
                0,
                OPT[1],
                0,
                0,
                OPT[0] | OPT[1] | OPT[2],
                0,
                3,
                1,
                3,
                2,
                HOLE_ELEVATIONS,
                3,
                3,
                HOLE_BULGES,
                3,
                1
            ]
        );
        // A vertex without an elevation is NaN, and only there.
        let nans: Vec<usize> = (0..cols.floats.len())
            .filter(|&i| cols.floats[i].is_nan())
            .collect();
        assert_eq!(nans, [13, 21]);
        assert_eq!(&cols.floats[..5], [1.0, 2.0, 3.0, 4.0, 7.5]);
        assert_eq!(&cols.floats[11..13], [0.5, 10.0]);
        assert_eq!(cols.floats[14], 12.0);
        assert_eq!(cols.floats[22], 3.0);
        assert_eq!(cols.floats[23].to_bits(), (-0.0f64).to_bits());

        let (back, back_uids) = unpack(&cols).expect("unpacks");
        assert_eq!(first_difference(&entities, &uids, &back, &back_uids), None);
        assert_eq!(differs(&cols, &back, &back_uids), None);
        let Entity::Polygon(p) = &back[1] else {
            panic!("a polygon")
        };
        assert_eq!(p.zs, Some(vec![Some(10.0), None, Some(12.0)]));
        let holes = p.holes.as_ref().expect("holes");
        assert_eq!(holes[0].zs.as_ref().map(Vec::len), Some(3));
        assert_eq!(holes[0].zs.as_ref().map(|z| z[0]), Some(None));
        assert_eq!(holes[1].zs, None);
        let Entity::Line(l) = &back[0] else {
            panic!("a line")
        };
        assert_eq!((l.za, l.zb), (None, Some(7.5)));

        // Every NaN is the same NaN: the page's engine may write another bit pattern for none.
        let mut other = cols.clone();
        other.floats[13] = f64::from_bits(0xfff8_0000_0000_0001);
        assert_eq!(differs(&other, &back, &back_uids), None);
        let (again, _) = unpack(&other).expect("unpacks");
        assert_eq!(
            first_difference(&back, &back_uids, &again, &back_uids),
            None
        );
        // A number of an elevation is a difference all the same.
        other.floats[14] = 12.5;
        assert!(differs(&other, &back, &back_uids).is_some());

        // Holes with flags no hole has, or lists running past the numbers, are refused.
        let mut flags = cols.clone();
        flags.ints[11] = 4;
        assert_eq!(
            unpack(&flags).map(|_| ()).unwrap_err().code,
            Code::BadColumns
        );
        let mut long = cols;
        long.ints[9] = 40;
        assert_eq!(
            unpack(&long).map(|_| ()).unwrap_err().code,
            Code::BadColumns
        );
    }

    #[test]
    fn a_changed_bit_or_field_is_a_difference() {
        let line = |x: f64| {
            Entity::Line(LineEntity {
                base: base("a"),
                a: Vec2 { x, y: 0.0 },
                b: Vec2 { x: 1.0, y: 1.0 },
                za: None,
                zb: None,
            })
        };
        let cols = pack(vec![line(0.0)], vec![id(1)]);
        assert_eq!(differs(&cols, &[line(0.0)], &[id(1)]), None);
        assert!(differs(&cols, &[line(-0.0)], &[id(1)]).is_some());
        assert!(differs(&cols, &[line(0.0)], &[id(2)]).is_some());
        assert_eq!(
            first_difference(&[line(0.0)], &[id(1)], &[line(-0.0)], &[id(1)]),
            Some(0)
        );
        let mut labelled = line(0.0);
        if let Entity::Line(l) = &mut labelled {
            l.base.label = Some("x".into());
        }
        assert!(differs(&cols, &[labelled], &[id(1)]).is_some());
    }
}
