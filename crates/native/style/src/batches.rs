//! The page's half of a styled layer (the web's `render/styledBatches.ts`,
//! docs/STYLE.md §6): the style core merges equal styles, packs the geometry
//! origin-relative into float32 and orders the batches by symbol level; here
//! each batch gets its colours from the theme palette, its images atlas keys
//! (SVG and raster markers, text, pattern tiles), and how far it reaches past
//! its geometry, so a frame can skip it when it is out of view. The renderer
//! draws what this gives it; `fixtures/style/v1/batches.json` holds both
//! platforms to the same batches.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::ops::Range;

use kentos_style_core::js::number;
use kentos_style_core::style::batch::{Batches, TEXT_BOX};
use serde_json::{Value, json};

use crate::color::{StylePalette, parse_hex, resolve, rgba};
use crate::library::StyleLibrary;

/// Sizes in metres ("world", from paper mm at the plot scale) or screen pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    World,
    Px,
}

impl Unit {
    fn read(v: Option<&Value>) -> Unit {
        match v.and_then(Value::as_str) {
            Some("px") => Unit::Px,
            _ => Unit::World,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Unit::World => "world",
            Unit::Px => "px",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cap {
    Butt,
    Round,
    Square,
}

impl Cap {
    fn read(v: Option<&Value>) -> Cap {
        match v.and_then(Value::as_str) {
            Some("round") => Cap::Round,
            Some("square") => Cap::Square,
            _ => Cap::Butt,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Cap::Butt => "butt",
            Cap::Round => "round",
            Cap::Square => "square",
        }
    }
}

/// The shapes drawn from distance fields, in the shader's order (`SHAPE_IDS`,
/// `styled.layout.json` → `values.shape`).
pub const SHAPE_IDS: [&str; 20] = [
    "circle",
    "ring",
    "square",
    "rectangle",
    "diamond",
    "triangle",
    "pentagon",
    "hexagon",
    "octagon",
    "star",
    "cross",
    "x",
    "line",
    "arrow",
    "arrowhead",
    "chevron",
    "semicircle",
    "quartercircle",
    "gear",
    "arc",
];

/// A shape's index for the shader; an unknown one draws as a circle (`shapeId`).
pub fn shape_index(shape: &str) -> u32 {
    SHAPE_IDS
        .iter()
        .position(|s| *s == shape)
        .map_or(0, |i| i as u32)
}

/// Shapes a pattern inks with lines only (the tint's `OPEN_SHAPES`).
const OPEN_SHAPES: [&str; 5] = ["cross", "x", "line", "arrow", "chevron"];

/// Share of its box a shape covers when filled (the far-zoom tint of patterns).
fn filled_share(shape: &str) -> f64 {
    match shape {
        "circle" | "ring" => 0.785,
        "square" | "rectangle" => 1.0,
        "diamond" => 0.5,
        "triangle" => 0.43,
        "pentagon" => 0.6,
        "hexagon" => 0.65,
        "octagon" => 0.8,
        "star" => 0.35,
        "semicircle" => 0.39,
        "quartercircle" => 0.2,
        "arrowhead" => 0.4,
        "gear" => 0.7,
        _ => 0.6,
    }
}

/// Where the point sits in a marker's box: (0,0) centre, (0,0.5) top edge …
fn anchor_of(anchor: &str) -> [f64; 2] {
    match anchor {
        "top" => [0.0, 0.5],
        "bottom" => [0.0, -0.5],
        "left" => [-0.5, 0.0],
        "right" => [0.5, 0.0],
        "top-left" => [-0.5, 0.5],
        "top-right" => [0.5, 0.5],
        "bottom-left" => [-0.5, -0.5],
        "bottom-right" => [0.5, -0.5],
        _ => [0.0, 0.0],
    }
}

/// Text marker faces as the web names them (`FONT_FAMILY`); the regulation's
/// legends use Arial, Arial Black and Times (metric twins on Linux).
pub fn font_family(font: &str) -> Option<&'static str> {
    match font {
        "ui" => Some("Barlow, \"Segoe UI\", sans-serif"),
        "sans" => Some("Arial, \"Liberation Sans\", Arimo, Helvetica, sans-serif"),
        "black" => Some("\"Arial Black\", \"Arial\", \"Liberation Sans\", Arimo, sans-serif"),
        "narrow" => Some("\"Arial Narrow\", \"Liberation Sans Narrow\", Arial, sans-serif"),
        "serif" => Some("\"Times New Roman\", \"Liberation Serif\", Tinos, Times, serif"),
        "mono" => Some("\"IBM Plex Mono\", monospace"),
        _ => None,
    }
}

/// Metres of paper per CSS pixel at 96 dpi, in mm (`MM_PER_PX`).
const MM_PER_PX: f64 = 25.4 / 96.0;

/// Drawn where a symbol's SVG asset is missing from the library: a crossed box.
pub const MISSING_SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 24 24\" width=\"24\" height=\"24\"><rect x=\"2\" y=\"2\" width=\"20\" height=\"20\" fill=\"none\" stroke=\"#E0457B\" stroke-width=\"2\"/><path d=\"M4 4L20 20M20 4L4 20\" stroke=\"#E0457B\" stroke-width=\"2\"/></svg>";

/// What the atlas draws for an image, keyed by `key` (equal keys share one entry).
#[derive(Clone, Debug, PartialEq)]
pub enum AtlasImage {
    Svg {
        key: String,
        svg: String,
        width: f64,
        height: f64,
    },
    Raster {
        key: String,
        url: String,
        width: f64,
        height: f64,
    },
    Text {
        key: String,
        text: String,
        /// The web's CSS font stack; the desktop maps it to its faces.
        font: Option<String>,
        weight: f64,
        italic: bool,
        color: String,
        /// Halo colour and width as a share of the letter height.
        halo: Option<(String, f64)>,
    },
    /// Pattern tile: marker looks laid out on a grid cell of aspect w:h.
    Tile {
        key: String,
        aspect: f64,
        stagger: bool,
        draw: Vec<TileMark>,
    },
}

impl AtlasImage {
    pub fn key(&self) -> &str {
        match self {
            AtlasImage::Svg { key, .. }
            | AtlasImage::Raster { key, .. }
            | AtlasImage::Text { key, .. }
            | AtlasImage::Tile { key, .. } => key,
        }
    }
}

/// A marker inside a pattern tile, in tile fractions and tile-width units.
#[derive(Clone, Debug, PartialEq)]
pub struct TileMark {
    pub look: MarkerLook,
    /// Size as a fraction of the tile width.
    pub w: f64,
    pub h: f64,
    pub offset: [f64; 2],
    pub rotation: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MarkerLook {
    Shape {
        shape: String,
        fill: Option<[f64; 4]>,
        stroke: Option<[f64; 4]>,
        stroke_width: f64,
        params: [f64; 4],
    },
    /// An atlas image; `fit_height`: its size gives the height (text), else the width.
    Image { image: AtlasImage, fit_height: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub enum FillPaintBatch {
    Solid {
        color: [f64; 4],
    },
    Hatch {
        color: [f64; 4],
        angle: f64,
        spacing: f64,
        width: f64,
        offset: f64,
        dash: Option<Vec<f64>>,
        dash_offset: f64,
        /// A line's start along it past the one before (docs/adr/0186 §3).
        stagger: f64,
        unit: Unit,
    },
    /// A gradient (docs/adr/0186 §3): `shape` 0 linear, 1 cylinder, 2
    /// spherical; along `dir` (radians) from `from` to `to`, or from
    /// `centre` out to `radius` (metres from the batch's tile once folded).
    Gradient {
        color: [f64; 4],
        color2: [f64; 4],
        shape: u32,
        inverted: bool,
        dir: f64,
        from: f64,
        to: f64,
        centre: [f64; 2],
        radius: f64,
    },
    /// One shape on a grid computed per pixel; `tint` is the share of a cell
    /// the shape inks, used when cells are too small to draw.
    Pattern {
        shape: String,
        fill: Option<[f64; 4]>,
        stroke: Option<[f64; 4]>,
        stroke_width: f64,
        half: [f64; 2],
        mark_offset: [f64; 2],
        mark_rotation: f64,
        params: [f64; 4],
        size: [f64; 2],
        stagger: bool,
        angle: f64,
        offset: [f64; 2],
        jitter: f64,
        coverage: f64,
        seed: f64,
        tint: f64,
        opacity: f64,
        unit: Unit,
    },
    /// A tile from the atlas repeated over the area; `size` in `unit`.
    Tile {
        image: AtlasImage,
        size: [f64; 2],
        angle: f64,
        offset: [f64; 2],
        opacity: f64,
        unit: Unit,
    },
    /// A picture over its frame (docs/adr/0192 §3): `image` names its
    /// pixels (`asset:<id>`, `file:<path>`); its frame's lower left corner
    /// (metres from the batch's tile once folded), width and height, turn
    /// (radians) and whether the picture is upside down in it.
    Image {
        image: String,
        corner: [f64; 2],
        size: [f64; 2],
        angle: f64,
        mirror: bool,
        opacity: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum BatchKind {
    /// Thick, capped, dashed lines as instanced segments (6 floats each).
    Stroke {
        color: [f64; 4],
        width: f64,
        unit: Unit,
        dash: Option<Vec<f64>>,
        dash_offset: f64,
        cap: Cap,
        blur: f64,
    },
    /// Triangulated areas (2 floats a vertex) with a paint computed per pixel.
    Fill { paint: FillPaintBatch },
    /// Instanced markers (5 floats each: x, y, angle, width, height) sharing one look.
    Marker {
        unit: Unit,
        look: MarkerLook,
        offset: [f64; 2],
        anchor: [f64; 2],
        opacity: f64,
        /// Largest width and height among the instances, in `unit`.
        extent: [f64; 2],
    },
}

/// One GPU batch: its numbers in the layer's data, its look, and where it draws.
#[derive(Clone, Debug, PartialEq)]
pub struct StyledBatch {
    pub range: Range<usize>,
    pub kind: BatchKind,
    /// The symbol level it draws at, lowest first (the core's order).
    pub level: f64,
    /// What gathers objects into it in the core (`BatchSink::entry`): its
    /// kind, its style without what varies per object (a mark's size, height
    /// and turn) and its scale range. Two builds of parts of one layer give
    /// a batch the same key when the layer built whole draws them as one.
    pub key: u64,
    /// Origin-relative box of the geometry: min x, min y, max x, max y.
    pub bounds: [f64; 4],
    /// The origin of the tile its numbers are relative to, from the layers'
    /// origin (docs/adr/0157): [0, 0] around the anchor, a whole multiple of
    /// `TILE` far from it, exact in float32.
    pub origin: [f64; 2],
    /// How far drawing reaches past the geometry, in `reach_unit`.
    pub reach: f64,
    pub reach_unit: Unit,
    pub min_scale: Option<f64>,
    pub max_scale: Option<f64>,
}

/// A layer's batches in draw order and their numbers, one after another.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StyledLayer {
    pub data: Vec<f32>,
    pub batches: Vec<StyledBatch>,
}

/// What the colours and images come from.
pub struct DecodeOptions<'a> {
    pub palette: &'a StylePalette,
    /// The scale symbols were compiled at: relates px and paper mm inside pattern tiles.
    pub plot_scale: f64,
    pub library: &'a StyleLibrary,
}

// ── Reading the core's JSON ────────────────────────────────────────────

/// A number of the core's JSON; `null` (a NaN or ±∞ written) counts as 0, as the web's arithmetic has it.
fn n(v: &Value, k: &str) -> f64 {
    v.get(k).and_then(Value::as_f64).unwrap_or(0.0)
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

fn opt_s<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

fn pair(v: &Value, k: &str) -> [f64; 2] {
    let a = v.get(k).and_then(Value::as_array);
    let at = |i: usize| {
        a.and_then(|a| a.get(i))
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    };
    [at(0), at(1)]
}

fn quad(v: &Value, k: &str) -> [f64; 4] {
    let a = v.get(k).and_then(Value::as_array);
    let at = |i: usize| {
        a.and_then(|a| a.get(i))
            .and_then(Value::as_f64)
            .unwrap_or(0.0)
    };
    [at(0), at(1), at(2), at(3)]
}

/// A dash list cut to the eight values the shader takes.
fn dash8(v: &Value, k: &str) -> Option<Vec<f64>> {
    v.get(k).and_then(Value::as_array).map(|d| {
        d.iter()
            .take(8)
            .map(|x| x.as_f64().unwrap_or(0.0))
            .collect()
    })
}

/// `JSON.stringify` of a string.
fn js_str(out: &mut String, text: &str) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// `JSON.stringify` of a number (NaN and ±∞ as null).
fn js_num(out: &mut String, x: f64) {
    if x.is_finite() {
        out.push_str(&number::to_string(x));
    } else {
        out.push_str("null");
    }
}

fn js_nums(out: &mut String, xs: &[f64]) {
    out.push('[');
    for (i, x) in xs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        js_num(out, *x);
    }
    out.push(']');
}

/// `${x}` of a value that may be null in the web's template strings.
fn or_null(x: Option<&str>) -> &str {
    x.unwrap_or("null")
}

/// SVG colours given by the symbol (`applySvgParams`): `param(fill)` and
/// `param(stroke)`, each optionally followed by a default colour, and
/// `currentColor`.
pub fn apply_svg_params(svg: &str, fill: Option<&str>, stroke: Option<&str>) -> String {
    let lower = svg.to_ascii_lowercase();
    let bytes = svg.as_bytes();
    let lb = lower.as_bytes();
    let mut out = String::with_capacity(svg.len());
    let mut i = 0;
    let mut last = 0;
    let space = |b: u8| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'\x0c' | b'\x0b');
    while i < bytes.len() {
        if !lb[i..].starts_with(b"param(") {
            i += 1;
            continue;
        }
        let mut j = i + 6;
        while j < bytes.len() && space(bytes[j]) {
            j += 1;
        }
        let which_fill = if lb[j..].starts_with(b"fill") {
            j += 4;
            true
        } else if lb[j..].starts_with(b"stroke") {
            j += 6;
            false
        } else {
            i += 1;
            continue;
        };
        while j < bytes.len() && space(bytes[j]) {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b')' {
            i += 1;
            continue;
        }
        j += 1;
        // The optional default: whitespace, '#', three to eight hex digits (as many as there are).
        let mut default: Option<&str> = None;
        let mut k = j;
        while k < bytes.len() && space(bytes[k]) {
            k += 1;
        }
        if k > j && k < bytes.len() && bytes[k] == b'#' {
            let mut e = k + 1;
            while e < bytes.len() && e - (k + 1) < 8 && bytes[e].is_ascii_hexdigit() {
                e += 1;
            }
            if e - (k + 1) >= 3 {
                default = Some(&svg[k..e]);
                j = e;
            }
        }
        out.push_str(&svg[last..i]);
        let chosen = if which_fill { fill } else { stroke };
        out.push_str(chosen.or(default).unwrap_or("#000000"));
        i = j;
        last = j;
    }
    out.push_str(&svg[last..]);
    out.replace("currentColor", fill.or(stroke).unwrap_or("#000000"))
}

struct Looks<'a> {
    o: &'a DecodeOptions<'a>,
}

impl Looks<'_> {
    fn rgba(&self, color: &str, opacity: f64) -> [f64; 4] {
        rgba(color, opacity, self.o.palette)
    }

    fn resolved(&self, color: &str) -> String {
        resolve(color, self.o.palette).to_owned()
    }

    /// A marker's look (`Looks.look`).
    fn look(&self, style: &Value) -> MarkerLook {
        match s(style, "kind") {
            "text" => {
                let size = n(style, "size");
                let color = self.resolved(s(style, "color"));
                let halo = style
                    .get("halo")
                    .filter(|h| h.is_object())
                    .map(|h| (self.resolved(s(h, "color")), n(h, "width") / size.max(1e-9)));
                let font = s(style, "font");
                let weight = n(style, "weight");
                let face = if font == "sans" && weight >= 900.0 {
                    "black"
                } else {
                    font
                };
                let italic = style.get("italic").and_then(Value::as_bool) == Some(true);
                let text = s(style, "text").to_owned();
                let key = format!(
                    "t|{text}|{font}|{}|{italic}|{color}|{}",
                    number::to_string(weight),
                    halo.as_ref().map_or(String::new(), |(c, w)| format!(
                        "{c}/{}",
                        number::to_fixed(*w, 3)
                    ))
                );
                MarkerLook::Image {
                    image: AtlasImage::Text {
                        key,
                        text,
                        font: font_family(face).map(str::to_owned),
                        weight,
                        italic,
                        color,
                        halo,
                    },
                    fit_height: true,
                }
            }
            "svg" => MarkerLook::Image {
                image: self.asset_image(
                    s(style, "asset"),
                    opt_s(style, "fill"),
                    opt_s(style, "stroke"),
                ),
                fit_height: false,
            },
            "raster" => MarkerLook::Image {
                image: self.asset_image(s(style, "asset"), None, None),
                fit_height: false,
            },
            _ => MarkerLook::Shape {
                shape: shape_name(s(style, "shape")).to_owned(),
                fill: opt_s(style, "fill").map(|c| self.rgba(c, 1.0)),
                stroke: opt_s(style, "stroke").map(|c| self.rgba(c, 1.0)),
                stroke_width: n(style, "strokeWidth"),
                params: quad(style, "params"),
            },
        }
    }

    /// An SVG or raster asset of the library (`assetImage`); a crossed box when it is gone.
    fn asset_image(&self, id: &str, fill: Option<&str>, stroke: Option<&str>) -> AtlasImage {
        let Some(a) = self.o.library.asset(id) else {
            return AtlasImage::Svg {
                key: format!("missing|{id}"),
                svg: MISSING_SVG.to_owned(),
                width: 24.0,
                height: 24.0,
            };
        };
        let data = a.data().unwrap_or("");
        let (width, height) = a.size().unwrap_or((0.0, 0.0));
        let len = data.encode_utf16().count();
        if a.format() == Some("svg") {
            let fill = fill.map(|c| self.resolved(c));
            let stroke = stroke.map(|c| self.resolved(c));
            AtlasImage::Svg {
                key: format!(
                    "svg|{}|{}|{}|{len}",
                    a.id(),
                    or_null(fill.as_deref()),
                    or_null(stroke.as_deref())
                ),
                svg: apply_svg_params(data, fill.as_deref(), stroke.as_deref()),
                width,
                height,
            }
        } else {
            AtlasImage::Raster {
                key: format!("img|{}|{len}", a.id()),
                url: data.to_owned(),
                width,
                height,
            }
        }
    }

    /// A fill's paint (`Looks.paint`).
    fn paint(&self, p: &Value) -> FillPaintBatch {
        match s(p, "kind") {
            "hatch" => FillPaintBatch::Hatch {
                color: self.rgba(s(p, "color"), n(p, "opacity")),
                angle: n(p, "angle"),
                spacing: n(p, "spacing"),
                width: n(p, "width"),
                offset: n(p, "offset"),
                dash: dash8(p, "dash"),
                dash_offset: n(p, "dashOffset"),
                stagger: n(p, "stagger"),
                unit: Unit::read(p.get("unit")),
            },
            "gradient" => FillPaintBatch::Gradient {
                color: self.rgba(s(p, "color"), n(p, "opacity")),
                color2: self.rgba(s(p, "color2"), n(p, "opacity")),
                shape: n(p, "shape") as u32,
                inverted: p.get("inverted").and_then(Value::as_bool) == Some(true),
                dir: n(p, "dir"),
                from: n(p, "from"),
                to: n(p, "to"),
                centre: pair(p, "centre"),
                radius: n(p, "radius"),
            },
            "pattern" => {
                let m = p.get("mark").unwrap_or(&Value::Null);
                let common = m.get("common").unwrap_or(&Value::Null);
                let size = pair(p, "size");
                let msize = n(m, "size");
                let mheight = n(m, "height");
                let coverage = n(p, "coverage");
                FillPaintBatch::Pattern {
                    shape: shape_name(s(m, "shape")).to_owned(),
                    fill: opt_s(m, "fill").map(|c| self.rgba(c, 1.0)),
                    stroke: opt_s(m, "stroke").map(|c| self.rgba(c, 1.0)),
                    stroke_width: n(m, "strokeWidth"),
                    half: [
                        msize / 2.0,
                        if mheight != 0.0 { mheight } else { msize } / 2.0,
                    ],
                    mark_offset: pair(common, "offset"),
                    mark_rotation: n(common, "rotation"),
                    params: quad(m, "params"),
                    size,
                    stagger: p.get("stagger").and_then(Value::as_bool) == Some(true),
                    angle: n(p, "angle"),
                    offset: pair(p, "offset"),
                    jitter: n(p, "jitter"),
                    coverage,
                    seed: n(p, "seed"),
                    tint: pattern_tint(m, size[0] * size[1]) * coverage,
                    opacity: n(p, "opacity") * n(common, "opacity"),
                    unit: Unit::read(p.get("unit")),
                }
            }
            "image" => FillPaintBatch::Image {
                image: s(p, "image").to_owned(),
                corner: pair(p, "corner"),
                size: pair(p, "size"),
                angle: n(p, "angle"),
                mirror: p.get("mirror").and_then(Value::as_bool) == Some(true),
                opacity: n(p, "opacity"),
            },
            "tile" => {
                let tile = p.get("tile").unwrap_or(&Value::Null);
                let stagger = s(tile, "kind") == "markers"
                    && tile.get("stagger").and_then(Value::as_bool) == Some(true);
                let size = pair(p, "size");
                let unit = Unit::read(p.get("unit"));
                FillPaintBatch::Tile {
                    image: self.tile_image(tile, size, unit),
                    size: [size[0], size[1] * if stagger { 2.0 } else { 1.0 }],
                    angle: n(p, "angle"),
                    offset: pair(p, "offset"),
                    opacity: n(p, "opacity"),
                    unit,
                }
            }
            _ => FillPaintBatch::Solid {
                color: self.rgba(s(p, "color"), n(p, "opacity")),
            },
        }
    }

    /// A pattern tile: marker looks placed in a cell, sized relative to the tile width (`tileImage`).
    fn tile_image(&self, tile: &Value, size: [f64; 2], unit: Unit) -> AtlasImage {
        if s(tile, "kind") == "asset" {
            return self.asset_image(s(tile, "asset"), None, None);
        }
        let world_per_px = MM_PER_PX * self.o.plot_scale / 1000.0;
        let in_tile_unit = |v: f64, u: Unit| {
            if u == unit {
                v
            } else if u == Unit::Px {
                v * world_per_px
            } else {
                v / world_per_px
            }
        };
        let tw = size[0];
        let stagger = tile.get("stagger").and_then(Value::as_bool) == Some(true);
        let draw: Vec<TileMark> = tile
            .get("markers")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|m| {
                let common = m.get("common").unwrap_or(&Value::Null);
                let mu = Unit::read(common.get("unit"));
                let w = in_tile_unit(n(m, "size"), mu) / tw;
                let h_raw = if s(m, "kind") == "shape" {
                    n(m, "height")
                } else {
                    n(m, "size")
                };
                let h = in_tile_unit(h_raw, mu) / tw;
                let look = match self.look(m) {
                    MarkerLook::Shape {
                        shape,
                        fill,
                        stroke,
                        stroke_width,
                        params,
                    } => MarkerLook::Shape {
                        shape,
                        fill,
                        stroke,
                        stroke_width: in_tile_unit(stroke_width, mu) / tw,
                        params,
                    },
                    image => image,
                };
                let off = pair(common, "offset");
                TileMark {
                    look,
                    w,
                    h,
                    offset: [in_tile_unit(off[0], mu) / tw, in_tile_unit(off[1], mu) / tw],
                    rotation: n(common, "rotation"),
                }
            })
            .collect();
        // A staggered tile holds two rows (the second half-shifted).
        let aspect = (size[1] / size[0]) * if stagger { 2.0 } else { 1.0 };
        let mut key = String::from("tile|[");
        for (i, m) in draw.iter().enumerate() {
            if i > 0 {
                key.push(',');
            }
            write_tile_mark(&mut key, m);
        }
        key.push_str("]|");
        key.push_str(&number::to_fixed(aspect, 4));
        key.push('|');
        key.push_str(if stagger { "true" } else { "false" });
        AtlasImage::Tile {
            key,
            aspect,
            stagger,
            draw,
        }
    }
}

/// A shape name the shader knows, else a circle (`shapeId`).
fn shape_name(shape: &str) -> &str {
    if SHAPE_IDS.contains(&shape) {
        shape
    } else {
        "circle"
    }
}

/// How much of a cell a pattern shape inks, 0–1 (`patternTint`).
fn pattern_tint(m: &Value, cell_area: f64) -> f64 {
    let w = n(m, "size");
    let height = n(m, "height");
    let h = if height != 0.0 { height } else { w };
    let shape = s(m, "shape");
    let open = OPEN_SHAPES.contains(&shape);
    let params = quad(m, "params");
    let fill = opt_s(m, "fill").is_some_and(|f| !f.is_empty());
    let stroke = opt_s(m, "stroke").is_some_and(|f| !f.is_empty());
    let mut ink = 0.0;
    if fill && !open {
        ink = w * h * filled_share(shape) * (1.0 - params[0] * params[0]);
    } else if stroke || open {
        let big = w.max(h);
        ink = 3.2 * big * n(m, "strokeWidth").max(0.05 * big);
    }
    (ink / cell_area.max(1e-12)).min(1.0)
}

// ── The tile key: `JSON.stringify(draw)` ───────────────────────────────

fn write_rgba(out: &mut String, c: Option<&[f64; 4]>) {
    match c {
        Some(c) => js_nums(out, c),
        None => out.push_str("null"),
    }
}

fn write_image(out: &mut String, image: &AtlasImage) {
    match image {
        AtlasImage::Svg {
            key,
            svg,
            width,
            height,
        } => {
            out.push_str("{\"key\":");
            js_str(out, key);
            out.push_str(",\"kind\":\"svg\",\"svg\":");
            js_str(out, svg);
            out.push_str(",\"width\":");
            js_num(out, *width);
            out.push_str(",\"height\":");
            js_num(out, *height);
            out.push('}');
        }
        AtlasImage::Raster {
            key,
            url,
            width,
            height,
        } => {
            out.push_str("{\"key\":");
            js_str(out, key);
            out.push_str(",\"kind\":\"raster\",\"url\":");
            js_str(out, url);
            out.push_str(",\"width\":");
            js_num(out, *width);
            out.push_str(",\"height\":");
            js_num(out, *height);
            out.push('}');
        }
        AtlasImage::Text {
            key,
            text,
            font,
            weight,
            italic,
            color,
            halo,
        } => {
            out.push_str("{\"key\":");
            js_str(out, key);
            out.push_str(",\"kind\":\"text\",\"text\":");
            js_str(out, text);
            // An unknown face is `undefined` there, which JSON leaves out.
            if let Some(font) = font {
                out.push_str(",\"font\":");
                js_str(out, font);
            }
            out.push_str(",\"weight\":");
            js_num(out, *weight);
            out.push_str(if *italic {
                ",\"italic\":true"
            } else {
                ",\"italic\":false"
            });
            out.push_str(",\"color\":");
            js_str(out, color);
            out.push_str(",\"halo\":");
            match halo {
                Some((c, w)) => {
                    out.push_str("{\"color\":");
                    js_str(out, c);
                    out.push_str(",\"width\":");
                    js_num(out, *w);
                    out.push('}');
                }
                None => out.push_str("null"),
            }
            out.push('}');
        }
        AtlasImage::Tile {
            key,
            aspect,
            stagger,
            draw,
        } => {
            out.push_str("{\"key\":");
            js_str(out, key);
            out.push_str(",\"kind\":\"tile\",\"aspect\":");
            js_num(out, *aspect);
            out.push_str(if *stagger {
                ",\"stagger\":true"
            } else {
                ",\"stagger\":false"
            });
            out.push_str(",\"draw\":[");
            for (i, m) in draw.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_tile_mark(out, m);
            }
            out.push_str("]}");
        }
    }
}

fn write_look(out: &mut String, look: &MarkerLook) {
    match look {
        MarkerLook::Shape {
            shape,
            fill,
            stroke,
            stroke_width,
            params,
        } => {
            out.push_str("{\"kind\":\"shape\",\"shape\":");
            js_str(out, shape);
            out.push_str(",\"fill\":");
            write_rgba(out, fill.as_ref());
            out.push_str(",\"stroke\":");
            write_rgba(out, stroke.as_ref());
            out.push_str(",\"strokeWidth\":");
            js_num(out, *stroke_width);
            out.push_str(",\"params\":");
            js_nums(out, params);
            out.push('}');
        }
        MarkerLook::Image { image, fit_height } => {
            out.push_str("{\"kind\":\"image\",\"image\":");
            write_image(out, image);
            out.push_str(if *fit_height {
                ",\"fit\":\"height\"}"
            } else {
                ",\"fit\":\"width\"}"
            });
        }
    }
}

fn write_tile_mark(out: &mut String, m: &TileMark) {
    out.push_str("{\"look\":");
    write_look(out, &m.look);
    out.push_str(",\"w\":");
    js_num(out, m.w);
    out.push_str(",\"h\":");
    js_num(out, m.h);
    out.push_str(",\"offset\":");
    js_nums(out, &m.offset);
    out.push_str(",\"rotation\":");
    js_num(out, m.rotation);
    out.push('}');
}

// ── Decoding ───────────────────────────────────────────────────────────

/// The core's batches of a layer as GPU batches, in the core's (draw) order (`styledBatches`).
pub fn decode(out: Batches, o: &DecodeOptions) -> Result<StyledLayer, String> {
    let described: Value = serde_json::from_str(&out.json)
        .map_err(|e| format!("Stil çekirdeğinin yanıtı okunamadı: {e}"))?;
    let looks = Looks { o };
    let mut batches = Vec::new();
    for d in described.as_array().into_iter().flatten() {
        let from = n(d, "from") as usize;
        let len = n(d, "len") as usize;
        let range = from..(from + len).min(out.data.len());
        let bounds = quad(d, "bounds");
        let origin = pair(d, "origin");
        let (dw, dh) = (n(d, "w"), n(d, "h"));
        let style = d.get("style").unwrap_or(&Value::Null);
        let min_scale = d.get("minScale").and_then(Value::as_f64);
        let max_scale = d.get("maxScale").and_then(Value::as_f64);
        let (level, key) = order_of(d, style);
        let batch = match s(d, "kind") {
            "stroke" => {
                let unit = Unit::read(style.get("unit"));
                StyledBatch {
                    range,
                    kind: BatchKind::Stroke {
                        color: looks.rgba(s(style, "color"), n(style, "opacity")),
                        width: n(style, "width"),
                        unit,
                        dash: dash8(style, "dash"),
                        dash_offset: n(style, "dashOffset"),
                        cap: Cap::read(style.get("cap")),
                        blur: n(style, "blur"),
                    },
                    level,
                    key,
                    bounds,
                    origin,
                    reach: dw + 1.0,
                    reach_unit: unit,
                    min_scale,
                    max_scale,
                }
            }
            "fill" => StyledBatch {
                range,
                kind: BatchKind::Fill {
                    paint: fold(looks.paint(style), origin),
                },
                level,
                key,
                bounds,
                origin,
                reach: 0.0,
                reach_unit: Unit::World,
                min_scale,
                max_scale,
            },
            _ => {
                let common = style.get("common").unwrap_or(&Value::Null);
                let look = looks.look(style);
                // A text marker is as wide as its text; an image as its proportions say.
                let mut w = dw;
                if let MarkerLook::Image { image, .. } = &look {
                    match image {
                        AtlasImage::Text { text, .. } => {
                            w = w.max((dh / TEXT_BOX) * 0.62 * text.encode_utf16().count() as f64);
                        }
                        AtlasImage::Svg { width, height, .. }
                        | AtlasImage::Raster { width, height, .. } => {
                            let side = if dh != 0.0 { dh } else { dw };
                            w = w.max(side * (width / height.max(1e-9)));
                        }
                        AtlasImage::Tile { .. } => {}
                    }
                }
                let offset = pair(common, "offset");
                let unit = Unit::read(common.get("unit"));
                StyledBatch {
                    range,
                    kind: BatchKind::Marker {
                        unit,
                        look,
                        offset,
                        anchor: anchor_of(s(common, "anchor")),
                        opacity: n(common, "opacity"),
                        extent: [dw, dh],
                    },
                    level,
                    key,
                    bounds,
                    origin,
                    reach: w.max(dh).max(dw) * 1.5 + offset[0].hypot(offset[1]) + 2.0,
                    reach_unit: unit,
                    min_scale,
                    max_scale,
                }
            }
        };
        batches.push(batch);
    }
    Ok(StyledLayer {
        data: out.data,
        batches,
    })
}

/// A batch's level and key (`StyledBatch::level`, `key`) from the core's
/// description: a stroke's and a fill's style is its key as it is; a mark's
/// key leaves out its size and height and turns it to 0 (`MarkerStyle::key`),
/// as the description carries the first mark's.
fn order_of(d: &Value, style: &Value) -> (f64, u64) {
    let kind = s(d, "kind");
    let (level, keyed) = match kind {
        "stroke" | "fill" => (n(style, "level"), style.clone()),
        _ => {
            let mut keyed = style.clone();
            if let Some(o) = keyed.as_object_mut() {
                o.remove("size");
                o.remove("height");
                if let Some(common) = o.get_mut("common").and_then(Value::as_object_mut) {
                    common.insert("rotation".into(), json!(0.0));
                }
            }
            (
                n(style.get("common").unwrap_or(&Value::Null), "level"),
                keyed,
            )
        }
    };
    let mut hasher = DefaultHasher::new();
    kind.hash(&mut hasher);
    keyed.to_string().hash(&mut hasher);
    d.get("minScale").map(Value::to_string).hash(&mut hasher);
    d.get("maxScale").map(Value::to_string).hash(&mut hasher);
    // A style's batches in two tiles are two (docs/adr/0157); the anchor's
    // tile says none, and its keys stay as they were.
    if let Some(origin) = d.get("origin") {
        origin.to_string().hash(&mut hasher);
    }
    (level, hasher.finish())
}

/// A paint whose phase comes from the world (a hatch, a tile or a pattern
/// in metres) drawn from its tile's origin `o` rather than the anchor: its
/// offsets folded so that the shader, given positions from the tile, draws
/// the very pattern it would from the anchor (docs/adr/0157 §3). The rest
/// of the period is the whole: a remainder is never below zero.
fn fold(paint: FillPaintBatch, o: [f64; 2]) -> FillPaintBatch {
    use kentos_geometry_core::jsmath::{cos, sin};
    if o == [0.0, 0.0] {
        return paint;
    }
    let rest = |v: f64, period: f64| {
        if period > 0.0 {
            v.rem_euclid(period)
        } else {
            0.0
        }
    };
    match paint {
        FillPaintBatch::Hatch {
            color,
            angle,
            spacing,
            width,
            offset,
            dash,
            dash_offset,
            stagger,
            unit: Unit::World,
        } => {
            // The lines run along (cos, sin); the offset is across them. The tile's first
            // line is `m` lines past the anchor's: a staggered family's dashes start m
            // staggers further along there (docs/adr/0186 §3).
            let (c, s) = (cos(angle), sin(angle));
            let along = dash.as_deref().map_or(0.0, dash_period);
            let across = -s * o[0] + c * o[1];
            let m = if spacing > 0.0 {
                ((across - rest(across, spacing)) / spacing).round()
            } else {
                0.0
            };
            FillPaintBatch::Hatch {
                color,
                angle,
                spacing,
                width,
                offset: offset - rest(across, spacing),
                dash,
                dash_offset: dash_offset + rest(c * o[0] + s * o[1] - m * stagger, along),
                stagger,
                unit: Unit::World,
            }
        }
        // A gradient's frame from the tile: along its direction, and its middle.
        FillPaintBatch::Gradient {
            color,
            color2,
            shape,
            inverted,
            dir,
            from,
            to,
            centre,
            radius,
        } => {
            let shift = cos(dir) * o[0] + sin(dir) * o[1];
            FillPaintBatch::Gradient {
                color,
                color2,
                shape,
                inverted,
                dir,
                from: from - shift,
                to: to - shift,
                centre: [centre[0] - o[0], centre[1] - o[1]],
                radius,
            }
        }
        // A picture's frame from the tile (docs/adr/0192 §3).
        FillPaintBatch::Image {
            image,
            corner,
            size,
            angle,
            mirror,
            opacity,
        } => FillPaintBatch::Image {
            image,
            corner: [corner[0] - o[0], corner[1] - o[1]],
            size,
            angle,
            mirror,
            opacity,
        },
        FillPaintBatch::Tile {
            image,
            size,
            angle,
            offset,
            opacity,
            unit: Unit::World,
        } => {
            let (c, s) = (cos(angle), sin(angle));
            let turned = [c * o[0] + s * o[1], -s * o[0] + c * o[1]];
            FillPaintBatch::Tile {
                image,
                size,
                angle,
                offset: [
                    offset[0] - rest(turned[0], size[0]),
                    offset[1] - rest(turned[1], size[1]),
                ],
                opacity,
                unit: Unit::World,
            }
        }
        FillPaintBatch::Pattern {
            angle,
            offset,
            size,
            stagger,
            unit: Unit::World,
            shape,
            fill,
            stroke,
            stroke_width,
            half,
            mark_offset,
            mark_rotation,
            params,
            jitter,
            coverage,
            seed,
            tint,
            opacity,
        } => {
            let (c, s) = (cos(angle), sin(angle));
            let turned = [c * o[0] + s * o[1], -s * o[0] + c * o[1]];
            // Staggered rows alternate: two rows make the period, so a row keeps its kind.
            let rows = if stagger { 2.0 * size[1] } else { size[1] };
            FillPaintBatch::Pattern {
                angle,
                offset: [
                    offset[0] - rest(turned[0], size[0]),
                    offset[1] - rest(turned[1], rows),
                ],
                size,
                stagger,
                unit: Unit::World,
                shape,
                fill,
                stroke,
                stroke_width,
                half,
                mark_offset,
                mark_rotation,
                params,
                jitter,
                coverage,
                seed,
                tint,
                opacity,
            }
        }
        paint => paint,
    }
}

/// A dash pattern's length as the shader repeats it: an odd pattern twice,
/// cut to eight values (the renderer's `dash_values`, the web's `dashValues`).
fn dash_period(dash: &[f64]) -> f64 {
    let even: Vec<f64> = if dash.len() % 2 == 1 {
        dash.iter().chain(dash).copied().collect()
    } else {
        dash.to_vec()
    };
    let mut d = [0.0; 8];
    for (slot, v) in d.iter_mut().zip(even.iter()) {
        *slot = *v;
    }
    d.iter().sum()
}

/// Where a batch's kind draws within its level: fills, then lines, then marks (the core's `Kind`).
fn kind_rank(kind: &BatchKind) -> u8 {
    match kind {
        BatchKind::Fill { .. } => 0,
        BatchKind::Stroke { .. } => 1,
        BatchKind::Marker { .. } => 2,
    }
}

/// The draw order of one layer built in parts that follow one another in
/// document order (the desktop's parts of a large layer): `(part, batch)`
/// pairs in the order the layer built whole draws. The core orders a layer's
/// batches by level, then fills, lines and marks, then the order it first
/// met them; a part's batches are in that order among themselves. So a
/// batch's place in the whole layer is its level, its kind and where its key
/// is first met (the first part that has it, and its place there), and the
/// parts of one batch draw one after another, as its objects do.
pub fn merged_order(parts: &[&StyledLayer]) -> Vec<(usize, usize)> {
    let mut first: std::collections::HashMap<u64, (usize, usize)> =
        std::collections::HashMap::new();
    for (p, layer) in parts.iter().enumerate() {
        for (b, batch) in layer.batches.iter().enumerate() {
            first.entry(batch.key).or_insert((p, b));
        }
    }
    let mut all: Vec<(usize, usize)> = parts
        .iter()
        .enumerate()
        .flat_map(|(p, layer)| (0..layer.batches.len()).map(move |b| (p, b)))
        .collect();
    all.sort_by(|&(pa, ba), &(pb, bb)| {
        let (a, b) = (&parts[pa].batches[ba], &parts[pb].batches[bb]);
        a.level
            .total_cmp(&b.level)
            .then(kind_rank(&a.kind).cmp(&kind_rank(&b.kind)))
            .then(first[&a.key].cmp(&first[&b.key]))
            .then(pa.cmp(&pb))
    });
    all
}

// ── Checks a frame makes (`render/types.ts`) ────────────────────────────

/// Text smaller than this on screen (CSS px) is left out (`MIN_TEXT_PX`).
pub const MIN_TEXT_PX: f64 = 3.0;

impl StyledBatch {
    /// Whether the batch is drawn at the screen scale 1:`den` (`inScale`).
    pub fn in_scale(&self, den: f64) -> bool {
        !(self.min_scale.is_some_and(|m| den < m) || self.max_scale.is_some_and(|m| den > m))
    }

    /// Whether it can show in `view` (origin-relative box in metres) at
    /// `px_per_m` device pixels per metre (`batchInView`).
    pub fn in_view(&self, view: [f64; 4], px_per_m: f64, dpr: f64) -> bool {
        let r = match self.reach_unit {
            Unit::World => self.reach,
            Unit::Px => self.reach * dpr / px_per_m,
        };
        let [x0, y0, x1, y1] = self.bounds;
        x1 + r >= view[0] && x0 - r <= view[2] && y1 + r >= view[1] && y0 - r <= view[3]
    }

    /// The atlas image it draws with, if any (`batchImage`).
    pub fn image(&self) -> Option<&AtlasImage> {
        match &self.kind {
            BatchKind::Fill {
                paint: FillPaintBatch::Tile { image, .. },
            } => Some(image),
            BatchKind::Marker {
                look: MarkerLook::Image { image, .. },
                ..
            } => Some(image),
            _ => None,
        }
    }

    /// Device pixels its image is shown at: a tile's width, a text marker's
    /// height, an SVG or raster marker's width (`batchImagePx`).
    pub fn image_px(&self, px_per_m: f64, dpr: f64) -> f64 {
        let k = |unit: Unit| if unit == Unit::World { px_per_m } else { dpr };
        match &self.kind {
            BatchKind::Fill {
                paint: FillPaintBatch::Tile { size, unit, .. },
            } => size[0] * k(*unit),
            BatchKind::Marker {
                look: MarkerLook::Image { fit_height, .. },
                extent,
                unit,
                ..
            } => (if *fit_height { extent[1] } else { extent[0] }) * k(*unit),
            _ => 0.0,
        }
    }

    /// False for a text batch too small to read at this zoom (`batchLegible`).
    pub fn legible(&self, px_per_m: f64, dpr: f64) -> bool {
        match self.image() {
            Some(AtlasImage::Text { .. }) => self.image_px(px_per_m, dpr) >= MIN_TEXT_PX * dpr,
            _ => true,
        }
    }
}

// ── As the fixtures write batches (`batchesJson`) ───────────────────────

fn f32s(data: &[f32]) -> Value {
    Value::Array(data.iter().map(|x| json!(f64::from(*x))).collect())
}

fn rgba_json(c: Option<&[f64; 4]>) -> Value {
    c.map_or(Value::Null, |c| json!(c))
}

fn image_json(image: &AtlasImage) -> Value {
    let mut text = String::new();
    write_image(&mut text, image);
    serde_json::from_str(&text).unwrap_or(Value::Null)
}

fn look_json(look: &MarkerLook) -> Value {
    let mut text = String::new();
    write_look(&mut text, look);
    serde_json::from_str(&text).unwrap_or(Value::Null)
}

impl StyledLayer {
    /// The batches as `fixtures/style/v1/batches.json` writes them.
    pub fn to_json(&self) -> Value {
        Value::Array(self.batches.iter().map(|b| self.batch_json(b)).collect())
    }

    fn batch_json(&self, b: &StyledBatch) -> Value {
        let data = self.data.get(b.range.clone()).unwrap_or(&[]);
        let mut v = match &b.kind {
            BatchKind::Stroke {
                color,
                width,
                unit,
                dash,
                dash_offset,
                cap,
                blur,
            } => json!({
                "kind": "stroke",
                "segments": f32s(data),
                "color": color,
                "width": width,
                "unit": unit.name(),
                "dash": dash,
                "dashOffset": dash_offset,
                "cap": cap.name(),
                "blur": blur,
            }),
            BatchKind::Fill { paint } => json!({
                "kind": "fill",
                "positions": f32s(data),
                "paint": paint_json(paint),
            }),
            BatchKind::Marker {
                unit,
                look,
                offset,
                anchor,
                opacity,
                extent,
            } => json!({
                "kind": "marker",
                "instances": f32s(data),
                "unit": unit.name(),
                "look": look_json(look),
                "offset": offset,
                "anchor": anchor,
                "opacity": opacity,
                "extent": extent,
            }),
        };
        if let Value::Object(o) = &mut v {
            o.insert("bounds".into(), json!(b.bounds));
            if b.origin != [0.0, 0.0] {
                o.insert("origin".into(), json!(b.origin));
            }
            o.insert("reach".into(), json!(b.reach));
            o.insert("reachUnit".into(), json!(b.reach_unit.name()));
            if let Some(m) = b.min_scale {
                o.insert("minScale".into(), json!(m));
            }
            if let Some(m) = b.max_scale {
                o.insert("maxScale".into(), json!(m));
            }
        }
        v
    }
}

fn paint_json(p: &FillPaintBatch) -> Value {
    match p {
        FillPaintBatch::Solid { color } => json!({ "kind": "solid", "color": color }),
        FillPaintBatch::Hatch {
            color,
            angle,
            spacing,
            width,
            offset,
            dash,
            dash_offset,
            stagger,
            unit,
        } => {
            let mut v = json!({
                "kind": "hatch",
                "color": color,
                "angle": angle,
                "spacing": spacing,
                "width": width,
                "offset": offset,
                "dash": dash,
                "dashOffset": dash_offset,
                "unit": unit.name(),
            });
            // Only a staggered family has it (docs/adr/0186 §3).
            if *stagger != 0.0
                && let Some(o) = v.as_object_mut()
            {
                o.insert("stagger".into(), json!(stagger));
            }
            v
        }
        FillPaintBatch::Gradient {
            color,
            color2,
            shape,
            inverted,
            dir,
            from,
            to,
            centre,
            radius,
        } => json!({
            "kind": "gradient",
            "color": color,
            "color2": color2,
            "shape": shape,
            "inverted": inverted,
            "dir": dir,
            "from": from,
            "to": to,
            "centre": centre,
            "radius": radius,
        }),
        FillPaintBatch::Image {
            image,
            corner,
            size,
            angle,
            mirror,
            opacity,
        } => json!({
            "kind": "image",
            "image": image,
            "corner": corner,
            "size": size,
            "angle": angle,
            "mirror": mirror,
            "opacity": opacity,
        }),
        FillPaintBatch::Pattern {
            shape,
            fill,
            stroke,
            stroke_width,
            half,
            mark_offset,
            mark_rotation,
            params,
            size,
            stagger,
            angle,
            offset,
            jitter,
            coverage,
            seed,
            tint,
            opacity,
            unit,
        } => json!({
            "kind": "pattern",
            "shape": shape,
            "fill": rgba_json(fill.as_ref()),
            "stroke": rgba_json(stroke.as_ref()),
            "strokeWidth": stroke_width,
            "half": half,
            "markOffset": mark_offset,
            "markRotation": mark_rotation,
            "params": params,
            "size": size,
            "stagger": stagger,
            "angle": angle,
            "offset": offset,
            "jitter": jitter,
            "coverage": coverage,
            "seed": seed,
            "tint": tint,
            "opacity": opacity,
            "unit": unit.name(),
        }),
        FillPaintBatch::Tile {
            image,
            size,
            angle,
            offset,
            opacity,
            unit,
        } => json!({
            "kind": "tile",
            "image": image_json(image),
            "size": size,
            "angle": angle,
            "offset": offset,
            "opacity": opacity,
            "unit": unit.name(),
        }),
    }
}

/// A colour read straight from hex, for tests and legends.
pub fn hex_rgba(hex: &str) -> [f64; 4] {
    parse_hex(hex, 1.0)
}

#[cfg(test)]
mod fold_tests {
    //! Folding a paint's phase into its tile (docs/adr/0157 §3), checked on
    //! the shader's own formulas in float64: from a far tile the hatch line,
    //! the tile's cell and the pattern's cell fall where they fall from the
    //! anchor. The platforms' agreement is `fixtures/style/v1/batches.json`'s;
    //! this is the rule itself.
    use super::*;
    use kentos_geometry_core::jsmath::{cos, sin};

    /// The far tile of `fixtures/interaction/v1/vector-fit.kcad`'s local survey.
    const O: [f64; 2] = [-458_752.0, -4_390_912.0];
    /// Points from that tile's origin, to its corners.
    const POINTS: [[f64; 2]; 5] = [
        [-27_285.0, -27_285.0],
        [12.25, -40.5],
        [30_000.5, 1.0],
        [-5.0, 31_000.125],
        [32_767.0, -32_767.0],
    ];

    /// Whether two phases (in periods) are one modulo a whole period.
    fn same_phase(a: f64, b: f64) -> bool {
        let d = (a - b).rem_euclid(1.0);
        d < 1e-9 || 1.0 - d < 1e-9
    }

    fn from_anchor(p: [f64; 2]) -> [f64; 2] {
        [p[0] + O[0], p[1] + O[1]]
    }

    #[test]
    fn a_hatch_and_its_dashes_continue_from_their_tile() {
        let (angle, spacing, offset, dash_offset) = (0.6, 1.5, 0.25, 0.75);
        let dash = vec![2.0, 1.0, 0.5];
        let paint = FillPaintBatch::Hatch {
            color: [0.0; 4],
            angle,
            spacing,
            width: 0.1,
            offset,
            dash: Some(dash.clone()),
            dash_offset,
            stagger: 0.0,
            unit: Unit::World,
        };
        let FillPaintBatch::Hatch {
            offset: folded,
            dash_offset: dash_folded,
            ..
        } = fold(paint, O)
        else {
            panic!("a hatch stays a hatch");
        };
        let (c, s) = (cos(angle), sin(angle));
        // An odd pattern repeats twice: 7 m.
        let total = dash_period(&dash);
        assert_eq!(total, 7.0);
        for p in POINTS {
            let w = from_anchor(p);
            // Across the lines (`hatchFs`: dot(p, n) − offset over the spacing).
            assert!(same_phase(
                (-s * w[0] + c * w[1] - offset) / spacing,
                (-s * p[0] + c * p[1] - folded) / spacing
            ));
            // Along them (`dashCover`: dot(p, dir) + dash offset over the pattern).
            assert!(same_phase(
                (c * w[0] + s * w[1] + dash_offset) / total,
                (c * p[0] + s * p[1] + dash_folded) / total
            ));
        }
    }

    /// A staggered family (docs/adr/0186 §3): each line's dashes `stagger`
    /// further along than the line's before; the tile's line numbers start
    /// where its first line is, so its dash phase takes the lines before.
    #[test]
    fn a_staggered_hatch_continues_from_its_tile() {
        let (angle, spacing, offset, dash_offset, stagger) = (0.6, 1.5, 0.25, 0.75, 0.4);
        let dash = vec![2.0, 1.0, 0.5, 1.5];
        let paint = FillPaintBatch::Hatch {
            color: [0.0; 4],
            angle,
            spacing,
            width: 0.1,
            offset,
            dash: Some(dash.clone()),
            dash_offset,
            stagger,
            unit: Unit::World,
        };
        let FillPaintBatch::Hatch {
            offset: folded,
            dash_offset: dash_folded,
            ..
        } = fold(paint, O)
        else {
            panic!("a hatch stays a hatch");
        };
        let (c, s) = (cos(angle), sin(angle));
        let total = dash_period(&dash);
        // The shader's row: the nearest line, from the offset.
        let row = |q: [f64; 2], off: f64| ((-s * q[0] + c * q[1] - off) / spacing + 0.5).floor();
        for p in POINTS {
            let w = from_anchor(p);
            assert!(same_phase(
                (c * w[0] + s * w[1] - row(w, offset) * stagger + dash_offset) / total,
                (c * p[0] + s * p[1] - row(p, folded) * stagger + dash_folded) / total
            ));
        }
    }

    /// A gradient's frame moves with the tile: the same point, the same share.
    #[test]
    fn a_gradient_continues_from_its_tile() {
        let paint = FillPaintBatch::Gradient {
            color: [0.0; 4],
            color2: [1.0; 4],
            shape: 0,
            inverted: false,
            dir: 0.6,
            from: -4_390_000.0,
            to: -4_380_000.0,
            centre: [-458_000.0, -4_390_000.0],
            radius: 300.0,
        };
        let FillPaintBatch::Gradient {
            from, to, centre, ..
        } = fold(paint.clone(), O)
        else {
            panic!("a gradient stays a gradient");
        };
        let FillPaintBatch::Gradient {
            from: f0,
            to: t0,
            centre: c0,
            ..
        } = paint
        else {
            unreachable!()
        };
        let (c, s) = (cos(0.6), sin(0.6));
        for p in POINTS {
            let w = from_anchor(p);
            let t_far = (c * w[0] + s * w[1] - f0) / (t0 - f0);
            let t_near = (c * p[0] + s * p[1] - from) / (to - from);
            assert!((t_far - t_near).abs() < 1e-9);
            let d_far = ((w[0] - c0[0]).powi(2) + (w[1] - c0[1]).powi(2)).sqrt();
            let d_near = ((p[0] - centre[0]).powi(2) + (p[1] - centre[1]).powi(2)).sqrt();
            assert!((d_far - d_near).abs() < 1e-6);
        }
    }

    #[test]
    fn a_tile_and_a_staggered_pattern_continue_from_their_tile() {
        let (angle, size, offset) = (0.3, [5.0, 3.5], [0.4, -0.2]);
        let turned = |p: [f64; 2]| {
            let (c, s) = (cos(angle), sin(angle));
            [c * p[0] + s * p[1], -s * p[0] + c * p[1]]
        };
        let tile = FillPaintBatch::Tile {
            image: AtlasImage::Raster {
                key: "t".into(),
                url: String::new(),
                width: 1.0,
                height: 1.0,
            },
            size,
            angle,
            offset,
            opacity: 1.0,
            unit: Unit::World,
        };
        let FillPaintBatch::Tile { offset: folded, .. } = fold(tile, O) else {
            panic!("a tile stays a tile");
        };
        for p in POINTS {
            let (a, t) = (turned(from_anchor(p)), turned(p));
            for k in 0..2 {
                assert!(same_phase(
                    (a[k] - offset[k]) / size[k],
                    (t[k] - folded[k]) / size[k]
                ));
            }
        }
        let pattern = FillPaintBatch::Pattern {
            shape: "circle".into(),
            fill: None,
            stroke: None,
            stroke_width: 0.0,
            half: [0.5, 0.5],
            mark_offset: [0.0, 0.0],
            mark_rotation: 0.0,
            params: [0.0; 4],
            size,
            stagger: true,
            angle,
            offset,
            jitter: 0.0,
            coverage: 1.0,
            seed: 0.0,
            tint: 0.0,
            opacity: 1.0,
            unit: Unit::World,
        };
        let FillPaintBatch::Pattern { offset: folded, .. } = fold(pattern, O) else {
            panic!("a pattern stays a pattern");
        };
        // Staggered rows: two rows make the period, so a row keeps its kind.
        let period = [size[0], 2.0 * size[1]];
        for p in POINTS {
            let (a, t) = (turned(from_anchor(p)), turned(p));
            for k in 0..2 {
                assert!(same_phase(
                    (a[k] - offset[k]) / period[k],
                    (t[k] - folded[k]) / period[k]
                ));
            }
        }
    }

    #[test]
    fn screen_paints_and_the_anchor_s_tile_are_left_as_they_are() {
        let hatch = |unit| FillPaintBatch::Hatch {
            color: [0.0; 4],
            angle: 0.6,
            spacing: 1.5,
            width: 0.1,
            offset: 0.25,
            dash: None,
            dash_offset: 0.0,
            stagger: 0.0,
            unit,
        };
        assert_eq!(fold(hatch(Unit::Px), O), hatch(Unit::Px));
        assert_eq!(fold(hatch(Unit::World), [0.0, 0.0]), hatch(Unit::World));
        let solid = FillPaintBatch::Solid { color: [1.0; 4] };
        assert_eq!(fold(solid.clone(), O), solid);
    }
}
