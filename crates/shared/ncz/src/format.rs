//! The Netcad NCZ drawing format, parsed: the records exactly as the
//! reference plugin reads them (README.md: a port of `ncz_pure.py` of Erdinç
//! Örsan ÜNAL's NCZ Reader 1.4.3, GPL-2.0-or-later). `emit` turns them into
//! the app's objects.
//!
//! Each function carries the name of the reference function it ports, so the
//! two read side by side. The arithmetic is the reference's operation for
//! operation — the same products, the same order, Python's `%`, `min` and
//! `max` — because that is what makes a double come out bit for bit the same
//! (the sines, cosines and arctangents are libm's, which agree with CPython's
//! to an ulp and with each other on every target; §23.4).
//!
//! WHERE IT DIFFERS ON PURPOSE:
//!
//! - TWO PASSES over the bytes instead of a list of every record: the
//!   reference collects everything and then finalises (drops the `S0` marks a
//!   smart object leaves and fills in layer names and colours from tables that
//!   may come after the geometry). The first pass reads those tables and notes
//!   whether a smart object exists; the second hands each record over already
//!   final and forgets it.
//! - THE COLLINEAR-VERTEX PASS of the rectangle test makes the same removals
//!   in the same order, found in O(n log n) rather than by rescanning the ring
//!   after each removal.
//! - WHAT THE REFERENCE DROPS IN SILENCE IS COUNTED: a record too short for its
//!   type, a coordinate outside the world, a text with no height.
//! - A BLOCK OF A TYPE IT DOES NOT KNOW IS SEARCHED FOR GEOMETRY, as a container
//!   is. Netcad 8 writes its settings as `name value` strings between the
//!   geometry; the reference's walk reads a letter of them as a block type and
//!   skips what that "block" spans — a third of a 72 MB Sivas UİP. A Netcad 5
//!   file reads exactly as before.
//! - A SMART OBJECT WITH NO RECTANGLE IS KEPT AS A POINT (the reference needs a
//!   rectangle of at least a millimetre; Netcad 8's notations are anchored at a
//!   point), and a Netcad 8 smart object's PROPERTIES are read: its class and
//!   values (`nizam=AYRIK`, `taks=0.4`), which `symbols` draws.
//! - THE LINE WIDTH IS READ: the float at +28 of every geometry record, in
//!   tenths of a millimetre.
//! - THE WALK RESUMES AFTER A NETCAD 8 SMART OBJECT'S PROPERTIES, which run 81
//!   bytes past the size its header declares.

use std::collections::BTreeSet;

use kentos_formats::math::{PI, atan2, cos, sin};
use kentos_formats::watch::Watch;

const LAYER_TABLE: u8 = 6;
const GEOMETRY: u8 = 21;
const GEOMETRY_EXTENDED: u8 = 22;
const VERSION: u8 = 25;
const NAMED_DATA: u8 = 28;
const EXTENDED_HEADER: usize = 28;

/// The watch is asked at least this often, in bytes walked.
const STRIDE: usize = 1 << 20;

/// `GEOMETRY_MINIMUM_BYTES`, or 0 for a type the reference has no entry for.
fn minimum_bytes(t: u8) -> usize {
    match t {
        1 => 87,
        2 => 39,
        3 => 74,
        4 => 120,
        5 => 94,
        6 => 95,
        7 => 113,
        9 => 24,
        10 => 124,
        11 => 82,
        12 | 13 => 122,
        15 => 90,
        _ => 0,
    }
}

/// `EXTENDED_HEADER_GEOMETRY_TYPES`.
fn extended_header_type(t: u8) -> bool {
    matches!(t, 1 | 4 | 5 | 6 | 7 | 9 | 10 | 13)
}

/// `EMBEDDED_GEOMETRY_CONTAINER_TYPES`.
fn container_type(t: u8) -> bool {
    matches!(t, 0 | 5 | 14 | 48 | 108 | 111 | 132 | 150 | 180)
}

// ── Python arithmetic ──────────────────────────────────────────────────

/// Python's `x % y` for floats: the sign of the divisor, and `+0.0` for an
/// exact zero (CPython `float_rem`). Rust's `%` is C's `fmod`, exact.
pub(crate) fn py_mod(x: f64, y: f64) -> f64 {
    let mut m = x % y;
    if m != 0.0 || m.is_nan() {
        if (y < 0.0) != (m < 0.0) {
            m += y;
        }
    } else {
        m = 0.0f64.copysign(y);
    }
    m
}

/// Python's two-argument `min`: the second only when it is strictly smaller,
/// which decides what a NaN does.
fn py_min(a: f64, b: f64) -> f64 {
    if b < a { b } else { a }
}

/// Python's two-argument `max`, likewise.
fn py_max(a: f64, b: f64) -> f64 {
    if b > a { b } else { a }
}

/// `180.0 / math.pi` and `math.pi / 180.0`, what the reference multiplies by
/// (and what `math.degrees` and `math.radians` multiply by).
pub(crate) const RAD_TO_DEG: f64 = 180.0 / PI;
const DEG_TO_RAD: f64 = PI / 180.0;

/// `_is_token_byte`.
fn token_byte(v: u8) -> bool {
    v.is_ascii_alphanumeric() || v == b'-' || v == b'_'
}

/// A byte that `str.strip()` takes as white space once decoded: the ASCII
/// controls 9–13 and 28–31, the space, and U+0085 and U+00A0 (what the
/// legacy decoding maps 0x85 and 0xA0 to).
fn py_space_byte(v: u8) -> bool {
    (9..=13).contains(&v) || (28..=32).contains(&v) || v == 0x85 || v == 0xA0
}

/// `_decode_legacy_char`: the six Turkish letters of Windows-1254 the
/// reference names, every other byte as Latin-1 (0x80–0x9F stay C1 controls).
pub(crate) fn legacy_char(b: u8) -> char {
    match b {
        221 => 'İ',
        222 => 'Ş',
        208 => 'Ğ',
        240 => 'ğ',
        253 => 'ı',
        254 => 'ş',
        _ => char::from(b),
    }
}

fn decode(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| legacy_char(b)).collect()
}

/// `str.strip('\x00 ')` on the bytes, which is the same thing: both are
/// single bytes in UTF-8 and in the legacy code page.
fn strip_nul_space(mut b: &[u8]) -> &[u8] {
    while let [0 | b' ', rest @ ..] = b {
        b = rest;
    }
    while let [rest @ .., 0 | b' '] = b {
        b = rest;
    }
    b
}

/// Whether every byte decodes to a character at or above 32, or a tab: the
/// reference's `all(ord(char) >= 32 or char == '\t' …)`.
fn printable_or_tab(b: &[u8]) -> bool {
    b.iter().all(|&c| c >= 32 || c == b'\t')
}

// ── What a read gives ──────────────────────────────────────────────────

/// The record kinds, in the reference's words (a user comparing a reading
/// with the plugin's layers reads the same).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Kind {
    #[default]
    Point,
    Line,
    Polyline,
    Polygon,
    Circle,
    Arc,
    Text,
    Symbol,
    Block,
    MapSheet,
    Triangle,
    SmartObject,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Point => "Point",
            Kind::Line => "Line",
            Kind::Polyline => "Polyline",
            Kind::Polygon => "Polygon",
            Kind::Circle => "Circle",
            Kind::Arc => "Arc",
            Kind::Text => "Text",
            Kind::Symbol => "Symbol",
            Kind::Block => "Block",
            Kind::MapSheet => "MapSheet",
            Kind::Triangle => "Triangle",
            Kind::SmartObject => "SmartObject",
        }
    }
}

/// One coordinate as the reference returns it: `x` EAST, `y` NORTH. The file
/// holds them the other way round (northing first, the Turkish X), and the
/// reference swaps them in `_coordinate`; so does this parser, in one place.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Coord {
    /// Easting, metres (the file's second number).
    pub x: f64,
    /// Northing, metres (the file's first number).
    pub y: f64,
    /// What the reference reads as a height (see `emit`).
    pub z: f64,
}

/// Which optional fields the reference set: the keywords its
/// `_append_entity` was called with. A field left out keeps its default.
pub mod field {
    pub const NAME: u16 = 1 << 0;
    pub const LABEL: u16 = 1 << 1;
    pub const TEXT_HEIGHT: u16 = 1 << 2;
    pub const ROTATION: u16 = 1 << 3;
    pub const BOX_WIDTH: u16 = 1 << 4;
    pub const BOX_HEIGHT: u16 = 1 << 5;
    pub const SCALE: u16 = 1 << 6;
    pub const GRID_X: u16 = 1 << 7;
    pub const GRID_Y: u16 = 1 << 8;
    pub const RADIUS: u16 = 1 << 9;
    pub const START_ANGLE: u16 = 1 << 10;
    pub const END_ANGLE: u16 = 1 << 11;
    pub const CLOSED: u16 = 1 << 12;
}

/// A Netcad 8 smart object's class ("akıllı nesne"), as the GUID in its
/// record names it: Planet's symbol tools write these.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmartClass {
    /// Not a Netcad 8 smart object: the reference's rectangle, or nothing.
    #[default]
    None,
    /// `Yerleşim`: order, storeys, garden distances.
    Settlement,
    /// `Yapılaşma`: TAKS, KAKS, Emsal, Hmax, Yençok.
    Construction,
    /// `Yol`: the road's width.
    Road,
    /// `Plan Notu`: a note in RTF.
    PlanNote,
    /// `Fonksiyon Adı`.
    FunctionName,
    /// A class this reader does not draw: a point with its properties.
    Other,
}

impl SmartClass {
    /// The words Netcad's menu uses.
    pub fn name(self) -> &'static str {
        match self {
            SmartClass::None => "",
            SmartClass::Settlement => "Yerleşim",
            SmartClass::Construction => "Yapılaşma",
            SmartClass::Road => "Yol",
            SmartClass::PlanNote => "Plan Notu",
            SmartClass::FunctionName => "Fonksiyon Adı",
            SmartClass::Other => "Akıllı Nesne",
        }
    }
}

/// One property of a Netcad 8 smart object: every value is TEXT in the
/// record (`12`, `0.4`, `AYRIK`, `True`), whatever its type byte says.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SmartProperty {
    /// Its own name: `nizam`, `taks`, `genislik`.
    pub name: String,
    /// Its value, as written.
    pub value: String,
    /// The label Netcad's property grid shows: `Nizam`, `Taks`.
    pub display: String,
    /// A value the user entered, not a setting of the symbol.
    pub user: bool,
    /// Switched off by its `chk…IsNull` companion: not drawn.
    pub null: bool,
}

/// One record, already final: its layer name and colour are what the
/// reference's `_finalize_entities` leaves in them.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entity {
    pub kind: Kind,
    pub layer_code: u8,
    /// Empty when no table names the code.
    pub layer_name: String,
    /// 0xAARRGGBB.
    pub color: Option<u32>,
    /// A point's name: its number.
    pub name: String,
    /// Text, symbol code, block or sheet name.
    pub label: String,
    /// Metres.
    pub text_height: f64,
    /// Degrees, as the reference computes it.
    pub rotation: f64,
    pub box_width: f64,
    pub box_height: f64,
    pub scale: f64,
    pub grid_x: f64,
    pub grid_y: f64,
    pub radius: f64,
    /// As stored: radians in every file seen so far.
    pub start_angle: f64,
    pub end_angle: f64,
    /// The pen width in TENTHS of a millimetre as the file holds it (7 is
    /// 0,70 mm); 0 is the thinnest line; a negative value is the layer's.
    pub line_width: f64,
    /// The reference read the height at +24, found 0 there and took the float
    /// at +28, which is the pen width: `coords[0].z` is then not a height.
    pub z_is_width: bool,
    pub closed: bool,
    /// `field` bits.
    pub set: u16,
    pub coords: Vec<Coord>,
    /// A Netcad 8 smart object's class and properties; the size and the
    /// rotation are `scale` and `rotation`.
    pub smart: SmartClass,
    pub properties: Vec<SmartProperty>,
}

/// Everything the reference returns besides its records, and what this port
/// counts that the reference let fall silently.
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    /// The layer tables, non-blank names only.
    pub layer_names: Vec<String>,
    /// LEX.ST2, 0xFFRRGGBB.
    pub layer_colors: Vec<u32>,
    /// `5.2.0.1035N`.
    pub version_name: String,
    /// TILED_XML's `SRS…`, as the reference cleans it.
    pub epsg: String,
    /// `ITRF / 3 / Zone 39`.
    pub projection_text: String,
    /// Whether an MPROJ block was read; then the three bytes below.
    pub mproj: bool,
    /// 1 geographic, 2 six-degree, 3 three-degree.
    pub projection: u8,
    /// 0 WGS-84, 1 ITRF, 4 ED50, 254 ED50-HGK.
    pub datum: u8,
    /// The zone byte: the central meridian for 3°.
    pub zone: u8,
    /// Geometry records of a type the reference does not read, by type byte.
    pub unsupported: [u64; 256],
    /// Geometry records of a type it reads that produced nothing: too short,
    /// a coordinate outside ±100 000 km, no text, no height, fewer than two
    /// points, no area. By type byte.
    pub dropped: [u64; 256],
    /// `S0` marks on layer 0 left out because the drawing has a smart object.
    pub smart_marks: u64,
    /// Blocks of a type the reference does not know, searched for geometry,
    /// and the records they held.
    pub swept_blocks: u64,
    pub swept_entities: u64,
    /// Smart objects kept as a point because they have no usable rectangle.
    pub point_smart_objects: u64,
    /// Netcad 8 smart objects read with their class and properties.
    pub planet_symbols: u64,
    /// The drawing has at least one smart object (the reference's own test).
    pub smart_object: bool,
}

impl Default for Header {
    fn default() -> Self {
        Self {
            layer_names: Vec::new(),
            layer_colors: Vec::new(),
            version_name: String::new(),
            epsg: String::new(),
            projection_text: String::new(),
            mproj: false,
            projection: 0,
            datum: 0,
            zone: 0,
            unsupported: [0; 256],
            dropped: [0; 256],
            smart_marks: 0,
            swept_blocks: 0,
            swept_entities: 0,
            point_smart_objects: 0,
            planet_symbols: 0,
            smart_object: false,
        }
    }
}

/// Receives the records in the reference's order.
pub trait Sink {
    /// The tables as they stand at the END of the file, before the first
    /// record: a layer's own colour, and what the file says its coordinates
    /// are in. `false` stops the read before any geometry.
    fn begin(&mut self, final_header: &Header) -> bool {
        let _ = final_header;
        true
    }

    /// One record; `false` stops the read.
    fn entity(&mut self, e: &Entity) -> bool;
}

/// How a read ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Every block was read.
    Complete,
    /// The watch said stop.
    Cancelled,
    /// The sink refused a record.
    Stopped,
}

/// `_to_argb`.
fn to_argb(r: u32, g: u32, b: u32) -> u32 {
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// `_normalize_layer_color`.
fn normalize_layer_color(argb: u32) -> u32 {
    let (red, green, blue) = ((argb >> 16) & 255, (argb >> 8) & 255, argb & 255);
    if red == 0 && green == 0 && blue <= 1 {
        return to_argb(0, 0, 0);
    }
    argb
}

/// `_geometry_color` against the tables in `h`.
fn geometry_color(h: &Header, layer_code: u8, color_code: u8) -> Option<u32> {
    match color_code {
        1 => return Some(to_argb(0, 0, 255)),
        255 => return Some(to_argb(255, 0, 0)),
        0 => {}
        _ => return None,
    }
    let n = h.layer_colors.len();
    let code = usize::from(layer_code);
    if code < n {
        return Some(normalize_layer_color(h.layer_colors[code]));
    }
    if code >= 1 && code - 1 < n {
        return Some(normalize_layer_color(h.layer_colors[code - 1]));
    }
    None
}

/// A layer's own colour: `_geometry_color(code, 0)`, what a record of colour
/// code 0 on it takes; none when no LEX.ST2 table covers it.
pub fn layer_color(h: &Header, layer_code: u8) -> Option<u32> {
    geometry_color(h, layer_code, 0)
}

/// `_layer_name` against the tables in `h`.
fn layer_name(h: &Header, layer_code: u8) -> &str {
    let n = h.layer_names.len();
    let code = usize::from(layer_code);
    if code < n {
        return &h.layer_names[code];
    }
    if code >= 1 && code - 1 < n {
        return &h.layer_names[code - 1];
    }
    ""
}

// ── The parser ─────────────────────────────────────────────────────────

/// Which of the two passes a parser runs.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Tables, the declared system, and whether a smart object exists.
    Header,
    /// Everything, each record handed to the sink.
    Entities,
}

/// Where a pass's walk sits in the whole read's progress (thousandths).
#[derive(Clone, Copy)]
struct Span {
    from: u64,
    width: u64,
}

/// `_NCZParser`, one pass of it. `fin` is the header the first pass left,
/// standing in for `_finalize_entities`.
struct Parser<'d, 'h> {
    data: &'d [u8],
    mode: Mode,
    header: &'h mut Header,
    sink: Option<&'h mut dyn Sink>,
    fin: Option<&'h Header>,
    span: Span,
    /// The one record buffer this parser reuses: a file of a million records
    /// allocates a million coordinate runs in the reference and one here.
    entity: Entity,
    /// The offset of the geometry record being read.
    record: usize,
    appended: u64,
    stopped: bool,
}

impl Parser<'_, '_> {
    fn size(&self) -> usize {
        self.data.len()
    }

    /// Asks the watch; `false` when it says stop.
    fn ask(&self, watch: &mut dyn Watch, cursor: usize) -> bool {
        let size = self.size().max(1) as u64;
        let done = self.span.from + (cursor as u64).min(size) * self.span.width / size;
        watch.step(done, crate::PROGRESS_TOTAL)
    }

    /// `_scan_blocks`.
    fn scan(&mut self, watch: &mut dyn Watch) -> Outcome {
        let size = self.size();
        let mut cursor = 0usize;
        let mut next_check = 0usize;
        while cursor + 5 < size {
            if cursor >= next_check {
                if !self.ask(watch, cursor) {
                    return Outcome::Cancelled;
                }
                next_check = cursor + STRIDE;
            }
            if self.stopped {
                return Outcome::Stopped;
            }
            let block_size = u64::from(self.u32(cursor + 1)) + 4;
            let total = block_size + 1;
            if block_size < 4 || cursor as u64 + total > size as u64 {
                cursor += 1;
                continue;
            }
            let bsize = block_size as usize;
            let typ = self.data[cursor];
            let mut advance = total;
            if typ == VERSION && self.header.version_name.is_empty() {
                self.header.version_name =
                    self.legacy_string(cursor + 6, usize::from(self.byte(cursor + 5)));
            } else if typ == NAMED_DATA {
                self.parse_named_data_block(cursor, bsize);
            } else if typ == LAYER_TABLE {
                self.parse_layer_table(cursor, bsize);
            } else if (typ == GEOMETRY || typ == GEOMETRY_EXTENDED) && bsize >= 7 {
                let ext = if typ == GEOMETRY_EXTENDED {
                    EXTENDED_HEADER
                } else {
                    0
                };
                self.parse_geometry(cursor, bsize, ext);
                advance = advance.max(self.smart_extent(cursor) as u64);
            } else if container_type(typ) {
                let done = self.parse_embedded_geometry(cursor, bsize, watch);
                if done != Outcome::Complete {
                    return done;
                }
            } else {
                // A type the reference does not know, searched as a container
                // is: Netcad 8's interleaved settings make the walk land on a
                // letter as a "block type", and what follows it is geometry.
                let before = self.appended;
                let done = self.parse_embedded_geometry(cursor, bsize, watch);
                if done != Outcome::Complete {
                    return done;
                }
                if self.appended != before && self.mode == Mode::Entities {
                    self.header.swept_blocks += 1;
                    self.header.swept_entities += self.appended - before;
                }
            }
            cursor += advance as usize;
        }
        if self.stopped {
            Outcome::Stopped
        } else {
            Outcome::Complete
        }
    }

    // ── bytes ──

    fn byte(&self, at: usize) -> u8 {
        self.data.get(at).copied().unwrap_or(0)
    }

    /// `_read_uint32`: little-endian, and (like `int.from_bytes` over a short
    /// slice) whatever bytes the file still has.
    fn u32(&self, at: usize) -> u32 {
        let mut v = 0u32;
        for k in 0..4 {
            if let Some(&b) = self.data.get(at.wrapping_add(k)) {
                v |= u32::from(b) << (8 * k);
            }
        }
        v
    }

    /// `_read_float64`; NaN past the end (a fuzzer's case, not a file's: every
    /// call site reads inside bytes the minimum-size test proved present).
    fn f64(&self, at: usize) -> f64 {
        match self.data.get(at..at.saturating_add(8)) {
            Some(b) if b.len() == 8 => {
                f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
            }
            _ => f64::NAN,
        }
    }

    /// `_read_float32`, widened exactly as `struct.unpack_from('<f')` widens it.
    fn f32(&self, at: usize) -> f64 {
        match self.data.get(at..at.saturating_add(4)) {
            Some(b) if b.len() == 4 => f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]])),
            _ => f64::NAN,
        }
    }

    /// The bytes `_read_legacy_string` decodes: at most `length` from
    /// `offset`, cut at the end of the file, trailing NULs stripped.
    fn legacy_bytes(&self, offset: usize, length: usize) -> &[u8] {
        if offset >= self.size() {
            return &[];
        }
        let end = self.size().min(offset.saturating_add(length));
        let mut last = end;
        while last > offset && self.data[last - 1] == 0 {
            last -= 1;
        }
        &self.data[offset..last]
    }

    /// `_read_legacy_string`.
    fn legacy_string(&self, offset: usize, length: usize) -> String {
        decode(self.legacy_bytes(offset, length))
    }

    /// `_read_positive_float`.
    fn positive_float(&self, offset: usize) -> Option<f64> {
        if offset > self.size() || self.size() - offset < 4 {
            return None;
        }
        let v = self.f32(offset);
        (v.is_finite() && v > 0.0 && v <= 100_000.0).then_some(v)
    }

    /// `_read_length_prefixed_text`.
    fn length_prefixed_text(&self, length_offset: usize, text_offset: usize) -> String {
        if length_offset >= self.size() || text_offset >= self.size() {
            return String::new();
        }
        let n = usize::from(self.data[length_offset]);
        if n == 0 || n > 240 || text_offset + n > self.size() {
            return String::new();
        }
        decode(strip_nul_space(self.legacy_bytes(text_offset, n)))
    }

    /// `_read_text_payload`.
    fn text_payload(&self, offset: usize, ext: usize) -> String {
        for (len, at) in [
            (offset + ext + 97, offset + ext + 98),
            (offset + ext + 86, offset + ext + 87),
            (offset + 97, offset + 98),
            (offset + 86, offset + 87),
        ] {
            let text = self.length_prefixed_text(len, at);
            if !text.is_empty() {
                return text;
            }
        }
        String::new()
    }

    /// `_read_length_prefixed_name`.
    fn length_prefixed_name(&self, start: usize, end: usize) -> String {
        let bounded = end.min(self.size());
        let stop_at = bounded.saturating_sub(2);
        for index in start..stop_at {
            let n = usize::from(self.data[index]);
            if n == 0 || n > 64 || index + 1 + n > bounded {
                continue;
            }
            let value = strip_nul_space(self.legacy_bytes(index + 1, n));
            if !value.is_empty() && printable_or_tab(value) {
                return decode(value);
            }
        }
        String::new()
    }

    /// `_read_ascii_token`.
    fn ascii_token(&self, start: usize, end: usize) -> String {
        let bounded = end.min(self.size());
        let mut cursor = start;
        while cursor < bounded {
            if !token_byte(self.data[cursor]) {
                cursor += 1;
                continue;
            }
            let from = cursor;
            while cursor < bounded && token_byte(self.data[cursor]) {
                cursor += 1;
            }
            if cursor - from >= 3 {
                return self.data[from..cursor]
                    .iter()
                    .map(|&b| char::from(b))
                    .collect();
            }
        }
        String::new()
    }

    /// `_read_plan_box_name`.
    fn plan_box_name(&self, offset: usize, block_size: usize) -> String {
        let end = self.size().min(offset + block_size + 1);
        let limit = if end >= 4 {
            offset.max(end - 4)
        } else {
            offset
        };
        for index in offset..limit {
            // `bytes.lower()` folds ASCII A–Z only.
            let low = |k: usize| self.data[index + k].to_ascii_lowercase();
            if low(0) != b'p' || low(1) != b'l' || low(2) != b'a' || low(3) != b'n' {
                continue;
            }
            let mut cursor = index + 4;
            while cursor < end && cursor - index < 32 && token_byte(self.data[cursor]) {
                cursor += 1;
            }
            if cursor <= index + 4 {
                continue;
            }
            if self.data[index + 4..cursor].iter().all(u8::is_ascii_digit) {
                return self.data[index..cursor]
                    .iter()
                    .map(|&b| char::from(b))
                    .collect();
            }
        }
        String::new()
    }

    /// `_read_epsg`: from the first `SRS` to the next `>`, which the reference
    /// looks for to the end of the FILE, not of the block; so does this.
    fn read_epsg(&self, offset: usize, max_length: usize) -> String {
        let tries = max_length.saturating_sub(3);
        for index in 0..tries {
            let p = offset + index;
            if p + 2 >= self.size() {
                break;
            }
            if &self.data[p..p + 3] != b"SRS" {
                continue;
            }
            let chars: String = self.data[p..]
                .iter()
                .take_while(|&&b| b != b'>')
                .map(|&b| legacy_char(b))
                .collect();
            return chars.replace("SRS:", "").replace('"', "");
        }
        String::new()
    }

    // ── Netcad 8 smart objects ──

    /// Where a Netcad 8 smart object really ends, from `offset`: past its
    /// property block, which the record's header does not count (it declares
    /// 81 bytes fewer); 0 for any other record. The walk resumes there, so
    /// the block's last bytes are never read as blocks of their own: a
    /// "block" found in a property's text would swallow the records after it.
    ///
    /// THE LAYOUT, worked out from 15 722 objects of a real UİP (nothing
    /// publishes it): at +94 the class GUID; at +110 −2 as an int32 and at
    /// +114 the record's length from +110; at +118 a version byte, the GUID
    /// again, four constant bytes, and at +139 the property count; the list
    /// ends four bytes before the record does.
    fn smart_extent(&self, offset: usize) -> usize {
        if offset + 143 > self.size() || self.byte(offset + 6) != 15 {
            return 0;
        }
        if self.u32(offset + 110) != 0xFFFF_FFFE {
            return 0;
        }
        if self.data[offset + 94..offset + 110] != self.data[offset + 119..offset + 135] {
            return 0;
        }
        if self.u32(offset + 139) > 4096 {
            return 0;
        }
        (self.size() - offset).min(110 + self.u32(offset + 114) as usize)
    }

    /// A length or a count in the property block: 7-bit groups, least
    /// significant first, as Delphi's streaming writes them.
    fn varint(rec: &[u8], at: &mut usize) -> Option<usize> {
        let mut value = 0usize;
        let mut shift = 0;
        while shift < 28 {
            let c = *rec.get(*at)?;
            *at += 1;
            value |= usize::from(c & 0x7F) << shift;
            if c < 0x80 {
                return Some(value);
            }
            shift += 7;
        }
        None
    }

    /// The property block after a smart object's geometry, into `e`; `false`
    /// when the record carries none (a Netcad 5 smart object) or does not
    /// hold together — then the record is read the reference's way. Each
    /// property: its name, a type byte, its value and the label Netcad shows
    /// for it (each a varint length and bytes), then seven flag bytes whose
    /// last says whether the value is the user's. Every value is text.
    fn smart_properties(&self, offset: usize, e: &mut Entity) -> bool {
        let end = self.smart_extent(offset);
        if end < 143 {
            return false;
        }
        let rec = &self.data[offset..offset + end];
        let count = self.u32(offset + 139) as usize;

        const CLASSES: [(&str, SmartClass); 5] = [
            ("89383f961207d34fba5a4e5f523cf239", SmartClass::Road),
            ("7b865f8e160c474cbc7b523f411216df", SmartClass::Settlement),
            ("3ed066ec376b7041ad0eebca2d350e10", SmartClass::Construction),
            ("f86f2b1e0c328441843d2c566b316d5c", SmartClass::PlanNote),
            ("89897e489d60c84388835aaf333c3074", SmartClass::FunctionName),
        ];
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let guid: String = rec[94..110]
            .iter()
            .flat_map(|&b| {
                [
                    char::from(HEX[usize::from(b >> 4)]),
                    char::from(HEX[usize::from(b & 15)]),
                ]
            })
            .collect();
        e.smart = CLASSES
            .iter()
            .find(|(id, _)| *id == guid)
            .map_or(SmartClass::Other, |(_, c)| *c);

        let text = |at: &mut usize| -> Option<Vec<u8>> {
            let n = Self::varint(rec, at)?;
            if n > rec.len() - *at {
                return None;
            }
            let out = rec[*at..*at + n].to_vec();
            *at += n;
            Some(out)
        };
        let mut raw: Vec<(Vec<u8>, SmartProperty)> = Vec::new();
        let mut at = 143usize;
        for _ in 0..count {
            let Some(name) = text(&mut at) else {
                return false;
            };
            if at >= rec.len() {
                return false;
            }
            at += 1; // the type byte: every value is text whatever it says
            let (Some(value), Some(display)) = (text(&mut at), text(&mut at)) else {
                return false;
            };
            if at + 7 > rec.len() {
                return false;
            }
            let user = rec[at + 6] == 1;
            at += 7;
            raw.push((
                name.clone(),
                SmartProperty {
                    name: String::from_utf8_lossy(&name).into_owned(),
                    value: String::from_utf8_lossy(&value).into_owned(),
                    display: String::from_utf8_lossy(&display).into_owned(),
                    user,
                    null: false,
                },
            ));
        }

        // `chkKatIsNull = True` switches `kat` off; the flags are not
        // properties of the symbol. `txtOn` pairs with `chkOnIsNull` and the
        // case differs (`yEncok`, `chkYEnCokIsNull`), so the match is ASCII
        // case-insensitive on the name without its `txt`. On the bytes, which
        // no file can make a panic of.
        let flag = |n: &[u8]| n.len() > 9 && n.starts_with(b"chk") && n.ends_with(b"IsNull");
        let nulls: Vec<Vec<u8>> = raw
            .iter()
            .filter(|(n, p)| flag(n) && p.value == "True")
            .map(|(n, _)| n[3..n.len() - 6].to_ascii_lowercase())
            .collect();
        for (n, mut prop) in raw {
            if flag(&n) {
                continue;
            }
            let lower = n.to_ascii_lowercase();
            let key = if lower.len() > 3 && lower.starts_with(b"txt") {
                &lower[3..]
            } else {
                &lower[..]
            };
            prop.null = nulls.iter().any(|x| x.as_slice() == key);
            e.properties.push(prop);
        }
        true
    }

    // ── values ──

    /// `_valid_xy`.
    fn valid_xy(x: f64, y: f64) -> bool {
        x.is_finite() && y.is_finite() && x.abs() <= 100_000_000.0 && y.abs() <= 100_000_000.0
    }

    /// `_coordinate`: the file's northing first, easting second, swapped.
    fn coordinate(raw_x: f64, raw_y: f64, z: f64) -> Coord {
        Coord {
            x: raw_y,
            y: raw_x,
            z,
        }
    }

    // ── blocks ──

    /// `_parse_named_data_block`.
    fn parse_named_data_block(&mut self, offset: usize, block_size: usize) {
        let block_end = self.size().min(offset + block_size + 1);
        if offset + 6 > block_end {
            return;
        }
        let name = self.legacy_string(offset + 6, usize::from(self.byte(offset + 5)));
        if name == "MPROJ" && offset + 22 <= block_end {
            let projection = self.data[offset + 16];
            let datum = self.data[offset + 17];
            let zone = self.data[offset + 21];
            let p = match projection {
                1 => "Geographic",
                2 => "6",
                3 => "3",
                _ => "Undefined",
            };
            let d = match datum {
                0 => "WGS-84",
                1 => "ITRF",
                4 => "ED50",
                254 => "ED50-HGK",
                _ => "Undefined",
            };
            self.header.projection_text = format!("{d} / {p} / Zone {zone}");
            self.header.mproj = true;
            self.header.projection = projection;
            self.header.datum = datum;
            self.header.zone = zone;
        } else if name == "TILED_XML" {
            self.header.epsg = self.read_epsg(offset, block_end - offset);
        } else if name == "LEX.ST2" && offset + 21 <= block_end {
            let count = usize::from(self.data[offset + 20]);
            for index in 0..count {
                let item = offset + 79 + index * 256;
                if item + 3 > block_end {
                    break;
                }
                let c = |k: usize| u32::from(self.data[item + k]);
                self.header.layer_colors.push(to_argb(c(0), c(1), c(2)));
            }
        }
    }

    /// `_parse_layer_table`.
    fn parse_layer_table(&mut self, offset: usize, block_size: usize) {
        let block_end = self.size().min(offset + block_size + 1);
        if offset + 18 > block_end {
            return;
        }
        let count = usize::from(self.data[offset + 16]) + usize::from(self.data[offset + 17]) * 256;
        for index in 0..count {
            let item = offset + 18 + index * 29;
            if item + 29 > block_end {
                break;
            }
            let n = usize::from(self.data[item + 4]);
            // `layer_name.strip()`: kept when anything but white space is left,
            // and kept as it was (the reference appends the unstripped name).
            let bytes = self.legacy_bytes(item + 5, n);
            if !bytes.iter().all(|&b| py_space_byte(b)) {
                let name = decode(bytes);
                self.header.layer_names.push(name);
            }
        }
    }

    /// `_parse_embedded_geometry`. The inner walk ends at `offset +
    /// block_size`, one byte short of where the outer walk says the block
    /// ends, as in the reference.
    fn parse_embedded_geometry(
        &mut self,
        offset: usize,
        block_size: usize,
        watch: &mut dyn Watch,
    ) -> Outcome {
        let mut cursor = offset + 5;
        let end = self.size().min(offset + block_size);
        let mut next_check = cursor + STRIDE;
        while cursor + 6 < end {
            if cursor >= next_check {
                if !self.ask(watch, cursor) {
                    return Outcome::Cancelled;
                }
                next_check = cursor + STRIDE;
            }
            if self.stopped {
                return Outcome::Stopped;
            }
            let t = self.data[cursor];
            let is_geometry = t == GEOMETRY || t == GEOMETRY_EXTENDED;
            if !is_geometry || self.data[cursor + 5] != self.data[cursor + 6] {
                cursor += 1;
                continue;
            }
            let inner = u64::from(self.u32(cursor + 1)) + 4;
            let total = inner + 1;
            if inner < 7 || cursor as u64 + total > end as u64 {
                cursor += 1;
                continue;
            }
            let ext = if t == GEOMETRY_EXTENDED {
                EXTENDED_HEADER
            } else {
                0
            };
            self.parse_geometry(cursor, inner as usize, ext);
            cursor += total.max(self.smart_extent(cursor) as u64) as usize;
        }
        Outcome::Complete
    }

    /// `_parse_geometry`.
    fn parse_geometry(&mut self, offset: usize, block_size: usize, ext: usize) {
        if block_size < 7 || offset + 6 >= self.size() {
            return;
        }
        let t = self.data[offset + 6];
        // The first pass needs one fact from the geometry: whether a smart
        // object exists. Everything else waits for the second.
        if self.mode == Mode::Header && t != 15 {
            return;
        }
        let mut minimum = minimum_bytes(t);
        if minimum != 0 {
            if extended_header_type(t) {
                minimum += ext;
            }
            if block_size + 1 < minimum {
                self.drop(t);
                return;
            }
        }
        let before = self.appended;
        self.record = offset;
        match t {
            1 => self.parse_point(offset, ext),
            2 => self.parse_line(offset, block_size),
            3 => self.parse_circle(offset),
            4 => self.parse_arc(offset, ext),
            5 => self.parse_text(offset, ext),
            6 => self.parse_symbol(offset, block_size, ext),
            7 => self.parse_multiline(offset, block_size, ext),
            9 => self.parse_compressed_curve(offset, block_size, ext),
            10 => self.parse_box(offset, block_size, ext),
            11 => self.parse_map_sheet(offset, block_size),
            12 => self.parse_triangle(offset),
            13 => self.parse_block_reference(offset, block_size, ext),
            15 => self.parse_smart_object(offset, block_size),
            _ => {
                if self.mode == Mode::Entities {
                    self.header.unsupported[usize::from(t)] += 1;
                }
                return;
            }
        }
        if self.appended == before {
            self.drop(t);
        }
    }

    fn drop(&mut self, t: u8) {
        if self.mode == Mode::Entities {
            self.header.dropped[usize::from(t)] += 1;
        }
    }

    // ── records ──

    /// `_parse_point`.
    fn parse_point(&mut self, offset: usize, ext: usize) {
        let layer_code = self.byte(offset + 7);
        let raw_x = self.f64(offset + 8);
        let raw_y = self.f64(offset + 16);
        let mut z = self.f32(offset + 24);
        let fallback = z == 0.0;
        if fallback {
            z = self.f32(offset + 28);
        }
        if !Self::valid_xy(raw_x, raw_y) {
            return;
        }
        let name = self.legacy_string(offset + ext + 87, usize::from(self.byte(offset + ext + 86)));
        self.begin(Kind::Point);
        self.entity.z_is_width = fallback;
        self.entity.coords.push(Self::coordinate(raw_x, raw_y, z));
        self.entity.name = name;
        self.entity.set |= field::NAME;
        self.append(layer_code, self.byte(offset + 37));
    }

    /// `_parse_line`.
    fn parse_line(&mut self, offset: usize, block_size: usize) {
        let layer_code = self.byte(offset + 7);
        let (raw_x1, raw_y1, z1) = (
            self.f64(offset + 8),
            self.f64(offset + 16),
            self.f32(offset + 24),
        );
        let raw_x2 = self.f64(offset + block_size - 19);
        let raw_y2 = self.f64(offset + block_size - 11);
        let z2 = self.f32(offset + block_size - 3);
        if !Self::valid_xy(raw_x1, raw_y1) || !Self::valid_xy(raw_x2, raw_y2) {
            return;
        }
        self.begin(Kind::Line);
        self.entity
            .coords
            .push(Self::coordinate(raw_x1, raw_y1, z1));
        self.entity
            .coords
            .push(Self::coordinate(raw_x2, raw_y2, z2));
        self.append(layer_code, self.byte(offset + 37));
    }

    /// `_parse_multiline`.
    fn parse_multiline(&mut self, offset: usize, block_size: usize, ext: usize) {
        let layer_code = self.byte(offset + 7);
        let text = self.legacy_string(offset + ext + 87, usize::from(self.byte(offset + ext + 86)));
        // Floor division of a non-negative number: the minimum-size test made
        // `block_size + 1` at least `113 + ext`.
        let point_count = (block_size + 1).saturating_sub(113 + ext) / 24;
        if point_count < 2 {
            return;
        }
        let block_end = self.size().min(offset + block_size + 1);
        self.begin(Kind::Polyline);
        self.entity.coords.reserve(point_count + 1);
        for index in 0..point_count {
            let at = index * 24 + (offset + ext + 113);
            if at + 24 > block_end {
                break;
            }
            let c = Self::coordinate(self.f64(at), self.f64(at + 8), self.f64(at + 16));
            self.entity.coords.push(c);
        }
        if self.entity.coords.len() < 2 {
            return;
        }
        let closed = nearly_closed(&self.entity.coords);
        let (first, last) = (
            self.entity.coords[0],
            self.entity.coords[self.entity.coords.len() - 1],
        );
        if closed && !same_coordinate(&first, &last) {
            self.entity.coords.push(first);
        }
        let metrics = box_metrics(&self.entity.coords);
        let e = &mut self.entity;
        e.kind = if closed {
            Kind::Polygon
        } else {
            Kind::Polyline
        };
        e.label = text;
        e.closed = closed;
        let (w, h, r) = metrics.unwrap_or((0.0, 0.0, 0.0));
        e.box_width = w;
        e.box_height = h;
        e.rotation = if metrics.is_some() { r } else { 0.0 };
        e.set |=
            field::LABEL | field::CLOSED | field::BOX_WIDTH | field::BOX_HEIGHT | field::ROTATION;
        self.append(layer_code, self.byte(offset + 37));
    }

    /// `_parse_compressed_curve`.
    fn parse_compressed_curve(&mut self, offset: usize, block_size: usize, ext: usize) {
        let origin_x = self.f64(offset + 8);
        let origin_y = self.f64(offset + 16);
        if !Self::valid_xy(origin_x, origin_y) {
            return;
        }
        let point_data = offset + ext + 122;
        let end_offset = offset + block_size + 1;
        if point_data + 8 > end_offset {
            return;
        }
        self.begin(Kind::Polyline);
        let mut invalid = 0u32;
        let mut at = point_data;
        while at < end_offset - 7 {
            let (dx, dy) = (self.f32(at), self.f32(at + 4));
            at += 18;
            let bad = |invalid: &mut u32, coords: &[Coord]| {
                *invalid += 1;
                !coords.is_empty() && *invalid >= 4
            };
            if !dx.is_finite() || !dy.is_finite() {
                if bad(&mut invalid, &self.entity.coords) {
                    break;
                }
                continue;
            }
            let (x, y) = (origin_x + dx, origin_y + dy);
            if !Self::valid_xy(x, y) {
                if bad(&mut invalid, &self.entity.coords) {
                    break;
                }
                continue;
            }
            invalid = 0;
            let here = Self::coordinate(x, y, 0.0);
            if let Some(prev) = self.entity.coords.last()
                && (prev.x - here.x).abs() < 0.0001
                && (prev.y - here.y).abs() < 0.0001
            {
                continue;
            }
            self.entity.coords.push(here);
        }
        if self.entity.coords.len() < 2 {
            return;
        }
        self.append(self.byte(offset + 7), self.byte(offset + 37));
    }

    /// `_parse_circle`.
    fn parse_circle(&mut self, offset: usize) {
        let layer_code = self.byte(offset + 7);
        let (raw_x, raw_y, z) = (
            self.f64(offset + 8),
            self.f64(offset + 16),
            self.f32(offset + 24),
        );
        if !Self::valid_xy(raw_x, raw_y) {
            return;
        }
        let (x2, x3) = (self.f64(offset + 50), self.f64(offset + 66));
        self.begin(Kind::Circle);
        self.entity.coords.push(Self::coordinate(raw_x, raw_y, z));
        self.entity.radius = (x2 - x3).abs() / 2.0;
        self.entity.set |= field::RADIUS;
        self.append(layer_code, self.byte(offset + 37));
    }

    /// `_parse_arc`.
    fn parse_arc(&mut self, offset: usize, ext: usize) {
        let layer_code = self.byte(offset + 7);
        let (raw_x, raw_y, z) = (
            self.f64(offset + 8),
            self.f64(offset + 16),
            self.f32(offset + 24),
        );
        if !Self::valid_xy(raw_x, raw_y) {
            return;
        }
        let radius = self.f64(offset + ext + 86);
        let start = self.f64(offset + ext + 104);
        let end = self.f64(offset + ext + 112);
        self.begin(Kind::Arc);
        self.entity.coords.push(Self::coordinate(raw_x, raw_y, z));
        self.entity.radius = radius;
        self.entity.start_angle = start;
        self.entity.end_angle = end;
        self.entity.set |= field::RADIUS | field::START_ANGLE | field::END_ANGLE;
        self.append(layer_code, self.byte(offset + 37));
    }

    /// `_parse_text`.
    fn parse_text(&mut self, offset: usize, ext: usize) {
        let layer_code = self.byte(offset + 7);
        let (raw_x, raw_y) = (self.f64(offset + 8), self.f64(offset + 16));
        let mut z = self.f32(offset + 24);
        let fallback = z == 0.0;
        if fallback {
            z = self.f32(offset + 28);
        }
        if !Self::valid_xy(raw_x, raw_y) {
            return;
        }
        let text = self.text_payload(offset, ext);
        if text.is_empty() {
            return;
        }
        let Some(height) = self
            .positive_float(offset + ext + 86)
            .or_else(|| self.positive_float(offset + 86))
        else {
            return;
        };
        let rotation = py_mod(self.f32(offset + ext + 90) * RAD_TO_DEG, 360.0);
        self.begin(Kind::Text);
        let e = &mut self.entity;
        e.z_is_width = fallback;
        e.coords.push(Self::coordinate(raw_x, raw_y, z));
        e.label = text;
        e.text_height = height;
        e.rotation = rotation;
        e.set |= field::LABEL | field::TEXT_HEIGHT | field::ROTATION;
        self.append(layer_code, self.byte(offset + 37));
    }

    /// `_parse_symbol`.
    fn parse_symbol(&mut self, offset: usize, block_size: usize, ext: usize) {
        let layer_code = self.byte(offset + 7);
        let (raw_x, raw_y, z) = (
            self.f64(offset + 8),
            self.f64(offset + 16),
            self.f32(offset + 24),
        );
        if !Self::valid_xy(raw_x, raw_y) {
            return;
        }
        let block_end = self.size().min(offset + block_size + 1);
        let mut symbol_offset = offset + ext + 94;
        if symbol_offset >= block_end {
            symbol_offset = offset + 94;
        }
        let code = if symbol_offset < block_end {
            self.data[symbol_offset]
        } else {
            0
        };
        let size = self
            .positive_float(offset + ext + 86)
            .or_else(|| self.positive_float(offset + 86));
        let rotation = py_mod(self.f32(offset + ext + 90) * RAD_TO_DEG, 360.0);
        self.begin(Kind::Symbol);
        let e = &mut self.entity;
        e.coords.push(Self::coordinate(raw_x, raw_y, z));
        e.label = format!("S{code}");
        e.text_height = size.unwrap_or(5.0);
        e.rotation = rotation;
        e.set |= field::LABEL | field::TEXT_HEIGHT | field::ROTATION;
        self.append(layer_code, self.byte(offset + 37));
    }

    /// `_parse_block_reference`.
    fn parse_block_reference(&mut self, offset: usize, block_size: usize, ext: usize) {
        let (raw_x, raw_y, z) = (
            self.f64(offset + 8),
            self.f64(offset + 16),
            self.f32(offset + 24),
        );
        if !Self::valid_xy(raw_x, raw_y) {
            return;
        }
        let name = self.length_prefixed_name(offset + ext + 86, offset + block_size + 1);
        let rotation = py_mod(self.f32(offset + ext + 118) * RAD_TO_DEG, 360.0);
        self.begin(Kind::Block);
        let e = &mut self.entity;
        e.coords.push(Self::coordinate(raw_x, raw_y, z));
        e.label = name;
        e.rotation = rotation;
        e.set |= field::LABEL | field::ROTATION;
        self.append(self.byte(offset + 7), self.byte(offset + 37));
    }

    /// `_parse_box`: a rectangle from its bottom-left corner, the far corner
    /// and a rotation stored in radians.
    fn parse_box(&mut self, offset: usize, block_size: usize, ext: usize) {
        let layer_code = self.byte(offset + 7);
        let (raw_x1, raw_y1) = (self.f64(offset + 8), self.f64(offset + 16));
        let (raw_x2, raw_y2) = (self.f64(offset + ext + 104), self.f64(offset + ext + 112));
        let rotation_radians = self.f32(offset + ext + 120);
        if !Self::valid_xy(raw_x1, raw_y1) || !Self::valid_xy(raw_x2, raw_y2) {
            return;
        }
        let width = (raw_x2 - raw_x1).abs();
        let height = (raw_y2 - raw_y1).abs();
        let rotation_degrees = py_mod(rotation_radians * RAD_TO_DEG, 360.0);
        let angle = rotation_degrees * DEG_TO_RAD;
        // A NaN rotation gives NaN corners, as `math.sin(nan)` does; the
        // mapping then refuses the record.
        let (s, c) = if angle.is_finite() {
            (sin(angle), cos(angle))
        } else {
            (f64::NAN, f64::NAN)
        };
        let (side_x, side_y, bottom_x, bottom_y) = (s, c, c, -s);
        let (p0x, p0y) = (raw_x1, raw_y1);
        let (p1x, p1y) = (p0x + bottom_x * width, p0y + bottom_y * width);
        let (p2x, p2y) = (p1x + side_x * height, p1y + side_y * height);
        let (p3x, p3y) = (p0x + side_x * height, p0y + side_y * height);
        let label = self.plan_box_name(offset, block_size);
        self.begin(Kind::Polygon);
        let e = &mut self.entity;
        for (x, y) in [(p0x, p0y), (p1x, p1y), (p2x, p2y), (p3x, p3y), (p0x, p0y)] {
            e.coords.push(Self::coordinate(x, y, 0.0));
        }
        e.closed = true;
        e.box_width = width;
        e.box_height = height;
        e.rotation = rotation_degrees;
        e.label = label;
        e.set |=
            field::CLOSED | field::BOX_WIDTH | field::BOX_HEIGHT | field::ROTATION | field::LABEL;
        self.append(layer_code, self.byte(offset + 37));
    }

    /// `_parse_map_sheet`: a pafta frame, axis-aligned, with its sheet name.
    fn parse_map_sheet(&mut self, offset: usize, block_size: usize) {
        let layer_code = self.byte(offset + 7);
        let (raw_x1, raw_y1) = (self.f64(offset + 50), self.f64(offset + 58));
        let (raw_x2, raw_y2) = (self.f64(offset + 66), self.f64(offset + 74));
        if !Self::valid_xy(raw_x1, raw_y1) || !Self::valid_xy(raw_x2, raw_y2) {
            return;
        }
        let (min_x, max_x) = (py_min(raw_x1, raw_x2), py_max(raw_x1, raw_x2));
        let (min_y, max_y) = (py_min(raw_y1, raw_y2), py_max(raw_y1, raw_y2));
        if (max_x - min_x).abs() < 0.001 || (max_y - min_y).abs() < 0.001 {
            return;
        }
        let sheet = self.length_prefixed_name(offset + 86, offset + block_size + 1);
        self.begin(Kind::MapSheet);
        let e = &mut self.entity;
        for (x, y) in [
            (min_x, min_y),
            (max_x, min_y),
            (max_x, max_y),
            (min_x, max_y),
            (min_x, min_y),
        ] {
            e.coords.push(Self::coordinate(x, y, 0.0));
        }
        e.closed = true;
        e.box_width = max_x - min_x;
        e.box_height = max_y - min_y;
        e.label = sheet;
        e.set |= field::CLOSED | field::BOX_WIDTH | field::BOX_HEIGHT | field::LABEL;
        self.append(layer_code, self.byte(offset + 37));
    }

    /// `_parse_triangle_vertex`.
    fn triangle_vertex(
        &self,
        offset: usize,
        xo: usize,
        yo: usize,
        zo: usize,
        has_z: bool,
    ) -> Option<Coord> {
        if offset + xo + 8 > self.size() || offset + yo + 8 > self.size() {
            return None;
        }
        let (x, y) = (self.f64(offset + xo), self.f64(offset + yo));
        let z = if has_z && offset + zo + 4 <= self.size() {
            self.f32(offset + zo)
        } else {
            0.0
        };
        Self::valid_xy(x, y).then(|| Self::coordinate(x, y, z))
    }

    /// `_parse_triangle`.
    fn parse_triangle(&mut self, offset: usize) {
        let (Some(a), Some(b), Some(c)) = (
            self.triangle_vertex(offset, 8, 16, 24, true),
            self.triangle_vertex(offset, 86, 94, 0, false),
            self.triangle_vertex(offset, 106, 114, 0, false),
        ) else {
            return;
        };
        let area2 = ((b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)).abs();
        if area2 <= 0.0001 {
            return;
        }
        self.begin(Kind::Triangle);
        self.entity.coords.extend([a, b, c]);
        self.append(self.byte(offset + 7), self.byte(offset + 37));
    }

    /// `_parse_smart_object`: Netcad's type 15, a rotated rectangle with a
    /// width, a height, a grid and a scale; its rotation in GRADS.
    fn parse_smart_object(&mut self, offset: usize, block_size: usize) {
        let layer_code = self.byte(offset + 7);
        let block_end = self.size().min(offset + block_size + 1);
        let (raw_x1, raw_y1) = (self.f64(offset + 8), self.f64(offset + 16));
        if !Self::valid_xy(raw_x1, raw_y1) {
            return;
        }
        let at_if = |end: usize, at: usize| {
            if offset + end <= block_end {
                self.f64(offset + at)
            } else {
                0.0
            }
        };
        let mut width = at_if(177, 169);
        let mut height = at_if(185, 177);
        let grid_x = at_if(193, 185);
        let grid_y = at_if(201, 193);
        let (raw_x2, raw_y2) = (self.f64(offset + 66), self.f64(offset + 74));
        if width <= 0.0 || height <= 0.0 {
            // No far corner either: the reference drops the object here; the
            // size stays zero, which keeps it as the point below.
            if Self::valid_xy(raw_x2, raw_y2) {
                width = (raw_x2 - raw_x1).abs();
                height = (raw_y2 - raw_y1).abs();
            } else {
                width = 0.0;
                height = 0.0;
            }
        }
        // NO RECTANGLE: a plan notation anchored at a point (no size, or a
        // size field holding something else: Netcad 8 writes 10^222 there for
        // some kinds). What the reference would have KEPT still decides the
        // `S0` rule, so that stays the reference's.
        let reference_keeps = !(width < 0.001 || height < 0.001);
        let rectangle =
            width >= 0.001 && height >= 0.001 && width <= 100_000_000.0 && height <= 100_000_000.0;
        let angle_grads = self.f32(offset + 82);
        let rotation_degrees = if angle_grads.is_finite() {
            py_mod(angle_grads * 0.9, 360.0)
        } else {
            0.0
        };
        let mut scale = self.f32(offset + 86);
        if !scale.is_finite() {
            scale = 0.0;
        }

        if self.mode == Mode::Header {
            // Only an object the reference itself keeps counts for the `S0` rule.
            if reference_keeps {
                self.header.smart_object = true;
            }
            self.appended += 1;
            return;
        }

        // A NETCAD 8 SMART OBJECT: a Planet symbol with its properties. Its
        // anchor, size and turn are all `symbols` needs to draw it.
        self.begin(Kind::SmartObject);
        let mut e = std::mem::take(&mut self.entity);
        let planet = self.smart_properties(offset, &mut e);
        self.entity = e;
        if planet {
            let e = &mut self.entity;
            e.coords.push(Self::coordinate(raw_x1, raw_y1, 0.0));
            e.rotation = rotation_degrees;
            e.scale = scale;
            e.label = e.smart.name().to_owned();
            e.set |= field::ROTATION | field::SCALE | field::LABEL;
            self.header.planet_symbols += 1;
            self.append(layer_code, self.byte(offset + 37));
            return;
        }

        const BASIC: &[u8] = b"BASIC";
        let whole = &self.data[offset..block_end];
        let label = if whole.windows(BASIC.len()).any(|w| w == BASIC) {
            "BASIC".to_owned()
        } else {
            self.ascii_token(offset + 145, block_end)
        };

        if !rectangle {
            self.begin(Kind::SmartObject);
            let e = &mut self.entity;
            e.coords.push(Self::coordinate(raw_x1, raw_y1, 0.0));
            e.rotation = rotation_degrees;
            e.scale = scale;
            e.grid_x = grid_x;
            e.grid_y = grid_y;
            e.label = label;
            e.set |= field::ROTATION | field::SCALE | field::GRID_X | field::GRID_Y | field::LABEL;
            self.header.point_smart_objects += 1;
            self.append(layer_code, self.byte(offset + 37));
            return;
        }

        let a = rotation_degrees * DEG_TO_RAD;
        let (s, c) = (sin(a), cos(a));
        let (bottom_x, bottom_y, side_x, side_y) = (s, c, c, -s);
        let (p0x, p0y) = (raw_x1, raw_y1);
        let (p1x, p1y) = (p0x + bottom_x * width, p0y + bottom_y * width);
        let (p2x, p2y) = (p1x + side_x * height, p1y + side_y * height);
        let (p3x, p3y) = (p0x + side_x * height, p0y + side_y * height);
        self.begin(Kind::SmartObject);
        let e = &mut self.entity;
        for (x, y) in [(p0x, p0y), (p1x, p1y), (p2x, p2y), (p3x, p3y), (p0x, p0y)] {
            e.coords.push(Self::coordinate(x, y, 0.0));
        }
        e.closed = true;
        e.box_width = width;
        e.box_height = height;
        e.rotation = rotation_degrees;
        e.scale = scale;
        e.grid_x = grid_x;
        e.grid_y = grid_y;
        e.label = label;
        e.set |= field::CLOSED
            | field::BOX_WIDTH
            | field::BOX_HEIGHT
            | field::ROTATION
            | field::SCALE
            | field::GRID_X
            | field::GRID_Y
            | field::LABEL;
        self.append(layer_code, self.byte(offset + 37));
    }

    // ── the record buffer ──

    /// A fresh record in the one buffer this parser reuses.
    fn begin(&mut self, kind: Kind) {
        let e = &mut self.entity;
        e.kind = kind;
        e.layer_code = 0;
        e.layer_name.clear();
        e.color = None;
        e.name.clear();
        e.label.clear();
        e.text_height = 0.0;
        e.rotation = 0.0;
        e.box_width = 0.0;
        e.box_height = 0.0;
        e.scale = 0.0;
        e.grid_x = 0.0;
        e.grid_y = 0.0;
        e.radius = 0.0;
        e.start_angle = 0.0;
        e.end_angle = 0.0;
        e.line_width = 0.0;
        e.z_is_width = false;
        e.closed = false;
        e.smart = SmartClass::None;
        e.properties.clear();
        e.set = 0;
        e.coords.clear();
    }

    /// `_append_entity`, then what `_finalize_entities` would do to it.
    fn append(&mut self, layer_code: u8, color_code: u8) {
        self.appended += 1;
        if self.mode != Mode::Entities {
            return;
        }
        let width = self.f32(self.record + 28);
        let name = layer_name(self.header, layer_code).to_owned();
        let color = geometry_color(self.header, layer_code, color_code);
        let e = &mut self.entity;
        e.layer_code = layer_code;
        e.line_width = width;
        e.layer_name = name;
        e.color = color;
        // `_finalize_entities`, which runs once the whole file is read. The
        // first pass has read it, so its tables are the final ones.
        if let Some(fin) = self.fin {
            if fin.smart_object && e.kind == Kind::Symbol && e.layer_code == 0 && e.label == "S0" {
                self.header.smart_marks += 1;
                return;
            }
            if e.layer_name.is_empty() {
                e.layer_name = layer_name(fin, layer_code).to_owned();
            }
            if e.color.is_none() {
                e.color = geometry_color(fin, layer_code, 0);
            }
        }
        if let Some(sink) = self.sink.as_mut()
            && !sink.entity(&self.entity)
        {
            self.stopped = true;
        }
    }
}

// ── the reference's geometric tests ────────────────────────────────────

/// `_same_coordinate`: within a millimetre on all three axes.
fn same_coordinate(a: &Coord, b: &Coord) -> bool {
    (a.x - b.x).abs() < 0.001 && (a.y - b.y).abs() < 0.001 && (a.z - b.z).abs() < 0.001
}

/// `_distance`, in three dimensions as the reference measures it.
fn distance(a: &Coord, b: &Coord) -> f64 {
    let (dx, dy, dz) = (a.x - b.x, a.y - b.y, a.z - b.z);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// `_is_nearly_closed`: the last point within a fifth of the shorter end edge
/// (at least five centimetres) of the first.
fn nearly_closed(c: &[Coord]) -> bool {
    if c.len() < 4 {
        return false;
    }
    let (first, last) = (&c[0], &c[c.len() - 1]);
    if same_coordinate(first, last) {
        return true;
    }
    if c.len() < 5 {
        return false;
    }
    let first_edge = distance(first, &c[1]);
    let last_edge = distance(&c[c.len() - 2], last);
    let gap = distance(first, last);
    let reference = py_min(first_edge, last_edge);
    if reference <= 0.001 {
        return false;
    }
    gap <= py_max(reference * 0.2, 0.05)
}

/// `_nearly_orthogonal`.
fn nearly_orthogonal(ax: f64, ay: f64, bx: f64, by: f64, a_len: f64, b_len: f64) -> bool {
    ((ax * bx + ay * by) / (a_len * b_len)).abs() <= 0.03
}

/// `_nearly_equal`.
fn nearly_equal(a: f64, b: f64) -> bool {
    let tolerance = py_max(py_max(a.abs(), b.abs()) * 0.02, 0.02);
    (a - b).abs() <= tolerance
}

/// `_simplify_collinear_ring`, to the four corners it leaves when there are
/// four; none when more survive.
///
/// THE SAME REMOVALS IN THE SAME ORDER, FOUND FASTER. The reference removes
/// the first removable vertex (a zero-length edge either side, or a turn
/// whose normalised cross product is at most 0.02) and scans again from the
/// start, until nothing is removable or four are left. A removal changes the
/// answer only for its two neighbours, so every vertex the last scan passed
/// stays unremovable unless it was one of them. Keeping the vertices whose
/// answer may have changed in an ordered set and taking the lowest index
/// each time visits exactly the vertices the reference would remove, in its
/// order, with its arithmetic, without its O(n²) rescans.
fn simplify_collinear_ring(points: &[Coord], count: usize) -> Option<[Coord; 4]> {
    if count < 4 {
        return None;
    }
    if count == 4 {
        return Some([points[0], points[1], points[2], points[3]]);
    }
    let mut prev: Vec<usize> = (0..count)
        .map(|i| if i == 0 { count - 1 } else { i - 1 })
        .collect();
    let mut next: Vec<usize> = (0..count)
        .map(|i| if i + 1 == count { 0 } else { i + 1 })
        .collect();
    let mut alive = vec![true; count];
    let mut pending: BTreeSet<usize> = (0..count).collect();
    let removable = |i: usize, prev: &[usize], next: &[usize]| {
        let (p, c, n) = (&points[prev[i]], &points[i], &points[next[i]]);
        let (ax, ay, bx, by) = (c.x - p.x, c.y - p.y, n.x - c.x, n.y - c.y);
        let a_len = (ax * ax + ay * ay).sqrt();
        let b_len = (bx * bx + by * by).sqrt();
        if a_len < 0.001 || b_len < 0.001 {
            return true;
        }
        (ax * by - ay * bx).abs() / (a_len * b_len) <= 0.02
    };
    let mut left = count;
    while left > 4 {
        let mut found = None;
        while let Some(v) = pending.pop_first() {
            if alive[v] && removable(v, &prev, &next) {
                found = Some(v);
                break;
            }
        }
        let Some(v) = found else { break };
        alive[v] = false;
        let (p, n) = (prev[v], next[v]);
        next[p] = n;
        prev[n] = p;
        pending.insert(p);
        pending.insert(n);
        left -= 1;
    }
    if left != 4 {
        return None;
    }
    let mut out = [Coord::default(); 4];
    let mut k = 0;
    for (i, point) in points.iter().enumerate().take(count) {
        if alive[i] && k < 4 {
            out[k] = *point;
            k += 1;
        }
    }
    Some(out)
}

/// `_box_metrics`: whether a closed run is a rectangle, and its sides and
/// rotation (degrees) when it is.
fn box_metrics(c: &[Coord]) -> Option<(f64, f64, f64)> {
    if c.len() < 5 {
        return None;
    }
    let mut count = c.len();
    if same_coordinate(&c[0], &c[c.len() - 1]) {
        count -= 1;
    }
    let u = simplify_collinear_ring(c, count)?;
    let e = [
        [u[1].x - u[0].x, u[1].y - u[0].y],
        [u[2].x - u[1].x, u[2].y - u[1].y],
        [u[3].x - u[2].x, u[3].y - u[2].y],
        [u[0].x - u[3].x, u[0].y - u[3].y],
    ];
    let lengths = e.map(|v| (v[0] * v[0] + v[1] * v[1]).sqrt());
    if lengths.iter().any(|&l| l < 0.001) {
        return None;
    }
    let opposite_equal =
        nearly_equal(lengths[0], lengths[2]) && nearly_equal(lengths[1], lengths[3]);
    let mut right_angles = true;
    for k in 0..4 {
        let j = (k + 1) % 4;
        right_angles =
            nearly_orthogonal(e[k][0], e[k][1], e[j][0], e[j][1], lengths[k], lengths[j])
                && right_angles;
    }
    if !opposite_equal || !right_angles {
        return None;
    }
    Some((
        lengths[0],
        lengths[1],
        py_mod(atan2(e[0][1], e[0][0]) * RAD_TO_DEG, 360.0),
    ))
}

// ── reading ────────────────────────────────────────────────────────────

/// Where the passes sit in the read's progress (thousandths of
/// `PROGRESS_TOTAL`): the tables' pass is quick, the records' is the read.
const HEADER_SPAN: Span = Span {
    from: 0,
    width: 100,
};
const ENTITY_SPAN: Span = Span {
    from: 100,
    width: 850,
};

/// Only the tables: the first of the two passes.
pub fn read_header(data: &[u8], header: &mut Header, watch: &mut dyn Watch) -> Outcome {
    *header = Header::default();
    let mut tables = Parser {
        data,
        mode: Mode::Header,
        header,
        sink: None,
        fin: None,
        span: HEADER_SPAN,
        entity: Entity::default(),
        record: 0,
        appended: 0,
        stopped: false,
    };
    tables.scan(watch)
}

/// Reads `data` the way the reference's `_NCZParser.parse` does (minus the
/// attribute tables, which `attributes` reads), handing every record to
/// `sink`, and fills `header`. Asks `watch` at least every megabyte.
pub fn read(
    data: &[u8],
    sink: &mut dyn Sink,
    header: &mut Header,
    watch: &mut dyn Watch,
) -> Outcome {
    // THE FIRST PASS: the tables as they stand at the end of the file, and
    // whether a smart object exists; the two things `_finalize_entities`
    // needs and one forward pass cannot know in time.
    let mut fin = Header::default();
    let first = read_header(data, &mut fin, watch);
    if first != Outcome::Complete {
        return first;
    }
    if !sink.begin(&fin) {
        return Outcome::Stopped;
    }
    // THE SECOND: the reference's own pass, the tables rebuilt as it meets
    // them so each record sees what the reference's saw, then finalised
    // against the first pass's.
    *header = Header::default();
    let second = {
        let mut records = Parser {
            data,
            mode: Mode::Entities,
            header,
            sink: Some(sink),
            fin: Some(&fin),
            span: ENTITY_SPAN,
            entity: Entity::default(),
            record: 0,
            appended: 0,
            stopped: false,
        };
        records.scan(watch)
    };
    header.smart_object = fin.smart_object;
    second
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn python_arithmetic() {
        assert_eq!(py_mod(-90.0, 360.0), 270.0);
        assert_eq!(py_mod(720.0, 360.0).to_bits(), 0.0f64.to_bits());
        assert_eq!(py_mod(-0.0, 360.0).to_bits(), 0.0f64.to_bits());
        assert!(py_mod(f64::NAN, 360.0).is_nan());
        assert_eq!(py_min(f64::NAN, 1.0).to_bits(), f64::NAN.to_bits());
        assert_eq!(py_min(1.0, f64::NAN), 1.0);
        assert_eq!(py_max(2.0, 3.0), 3.0);
    }

    #[test]
    fn the_legacy_code_page_is_latin_1_with_six_turkish_letters() {
        let bytes = [0xDD, 0xDE, 0xD0, 0xF0, 0xFD, 0xFE, 0xC7, 0x80, b'a'];
        assert_eq!(decode(&bytes), "İŞĞğışÇ\u{80}a");
        assert_eq!(strip_nul_space(b"\0 ab \0"), b"ab");
    }

    #[test]
    fn a_ring_of_many_collinear_points_is_a_box() {
        // 10 × 4 metres with every side split in five: 20 vertices, four corners.
        let mut ring = Vec::new();
        for i in 0..5 {
            ring.push(Coord {
                x: f64::from(i) * 2.0,
                y: 0.0,
                z: 0.0,
            });
        }
        for i in 0..5 {
            ring.push(Coord {
                x: 10.0,
                y: f64::from(i) * 0.8,
                z: 0.0,
            });
        }
        for i in 0..5 {
            ring.push(Coord {
                x: 10.0 - f64::from(i) * 2.0,
                y: 4.0,
                z: 0.0,
            });
        }
        for i in 0..5 {
            ring.push(Coord {
                x: 0.0,
                y: 4.0 - f64::from(i) * 0.8,
                z: 0.0,
            });
        }
        ring.push(ring[0]);
        let (w, h, r) = box_metrics(&ring).expect("a box");
        assert_eq!((w, h, r), (10.0, 4.0, 0.0));
        ring[7].x += 1.0;
        assert!(box_metrics(&ring).is_none());
    }
}
