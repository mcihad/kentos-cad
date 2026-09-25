//! What compiling a symbol on a geometry produces (the types of
//! `style/primitives.ts`): drawing primitives in world coordinates,
//! independent of any backend. Every size is already converted: "world"
//! is metres (from mm or m), "px" screen pixels drawn by the shader.
//!
//! Styles are written as the JSON the page reads them in (the same fields
//! in the same order), and that text is also what merges equal styles into
//! one batch (`key`), as `JSON.stringify` did in the TypeScript.

use std::fmt::Write;

use kentos_geometry_core::Vec2;
use kentos_geometry_core::api::json::{write_number, write_str};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimUnit {
    World,
    Px,
}

impl PrimUnit {
    fn name(self) -> &'static str {
        match self {
            PrimUnit::World => "world",
            PrimUnit::Px => "px",
        }
    }
}

/// Paint shared by one batch of strokes.
#[derive(Clone, Debug, PartialEq)]
pub struct StrokeStyle {
    pub color: String,
    pub opacity: f64,
    pub width: f64,
    pub unit: PrimUnit,
    /// On/off lengths in `unit`, or None.
    pub dash: Option<Vec<f64>>,
    pub dash_offset: f64,
    pub cap: String,
    pub join: String,
    /// Soft edge width in `unit` (0 = crisp).
    pub blur: f64,
    /// Symbol layer order: lower draws first.
    pub level: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FillPaint {
    Solid {
        color: String,
        opacity: f64,
        level: f64,
    },
    Hatch {
        color: String,
        opacity: f64,
        /// Radians, counter-clockwise from east.
        angle: f64,
        spacing: f64,
        width: f64,
        offset: f64,
        dash: Option<Vec<f64>>,
        dash_offset: f64,
        unit: PrimUnit,
        level: f64,
    },
    /// One shape on a grid, drawn by the shader: pattern fills of shape markers.
    Pattern {
        mark: MarkerStyle,
        size: [f64; 2],
        stagger: bool,
        angle: f64,
        offset: [f64; 2],
        jitter: f64,
        coverage: f64,
        seed: f64,
        opacity: f64,
        unit: PrimUnit,
        level: f64,
    },
    /// A tile repeated over the area: a marker pattern or an image asset.
    Tile {
        tile: Tile,
        size: [f64; 2],
        angle: f64,
        offset: [f64; 2],
        opacity: f64,
        unit: PrimUnit,
        level: f64,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Tile {
    Asset(String),
    /// Markers at the tile's grid points (and half-shifted when staggered).
    Markers {
        markers: Vec<MarkerStyle>,
        stagger: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum Look {
    Shape {
        shape: String,
        /// Width, and height for rectangles, in `unit`.
        size: f64,
        height: f64,
        fill: Option<String>,
        stroke: Option<String>,
        stroke_width: f64,
        /// Hole (share of the radius), teeth, arc opening (radians), tooth depth.
        params: [f64; 4],
    },
    Svg {
        asset: String,
        size: f64,
        fill: Option<String>,
        stroke: Option<String>,
    },
    Raster {
        asset: String,
        size: f64,
    },
    Text {
        text: String,
        /// Letter height in `unit`.
        size: f64,
        font: String,
        weight: f64,
        italic: bool,
        color: String,
        halo: Option<(String, f64)>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Common {
    pub unit: PrimUnit,
    pub opacity: f64,
    /// Shift in `unit` before rotation (x right, y up).
    pub offset: [f64; 2],
    pub anchor: String,
    /// Extra rotation, radians (added to the placement angle).
    pub rotation: f64,
    pub level: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MarkerStyle {
    pub look: Look,
    pub common: Common,
}

impl MarkerStyle {
    pub fn is_text(&self) -> bool {
        matches!(self.look, Look::Text { .. })
    }

    pub fn is_shape(&self) -> bool {
        matches!(self.look, Look::Shape { .. })
    }

    /// Width in `unit` (text: its letter height).
    pub fn size(&self) -> f64 {
        match &self.look {
            Look::Shape { size, .. }
            | Look::Svg { size, .. }
            | Look::Raster { size, .. }
            | Look::Text { size, .. } => *size,
        }
    }
}

/// Where compile sends its output.
pub trait Sink {
    fn stroke(&mut self, style: &StrokeStyle, path: &[Vec2], closed: bool);
    /// One area: outer ring first, holes after.
    fn fill(&mut self, paint: &FillPaint, rings: &[Vec<Vec2>]);
    /// `angle`: radians, the direction the marker faces (0 = east).
    fn marker(&mut self, style: &MarkerStyle, at: Vec2, angle: f64);
}

// ── JSON, as the page reads it and as `JSON.stringify` wrote it ─────────

/// A number as `JSON.stringify` writes it: NaN and ±∞ as null, −0 as 0.
pub fn num(out: &mut String, x: f64) {
    if !x.is_finite() {
        out.push_str("null");
    } else if x == 0.0 {
        out.push('0');
    } else {
        write_number(out, x);
    }
}

fn opt_str(out: &mut String, s: &Option<String>) {
    match s {
        Some(s) => write_str(out, s),
        None => out.push_str("null"),
    }
}

fn nums(out: &mut String, xs: &[f64]) {
    out.push('[');
    for (i, x) in xs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        num(out, *x);
    }
    out.push(']');
}

fn opt_nums(out: &mut String, xs: &Option<Vec<f64>>) {
    match xs {
        Some(xs) => nums(out, xs),
        None => out.push_str("null"),
    }
}

/// Writes `,"name":` (or `"name":` first) before a field.
struct Obj<'a> {
    out: &'a mut String,
    first: bool,
}

impl<'a> Obj<'a> {
    fn new(out: &'a mut String) -> Obj<'a> {
        out.push('{');
        Obj { out, first: true }
    }

    fn key(&mut self, name: &str) -> &mut String {
        if !self.first {
            self.out.push(',');
        }
        self.first = false;
        write_str(self.out, name);
        self.out.push(':');
        self.out
    }

    fn num(&mut self, name: &str, x: f64) {
        num(self.key(name), x);
    }

    fn str(&mut self, name: &str, s: &str) {
        write_str(self.key(name), s);
    }

    fn bool(&mut self, name: &str, b: bool) {
        self.key(name).push_str(if b { "true" } else { "false" });
    }

    fn end(self) {
        self.out.push('}');
    }
}

impl StrokeStyle {
    pub fn write_json(&self, out: &mut String) {
        let mut o = Obj::new(out);
        o.str("color", &self.color);
        o.num("opacity", self.opacity);
        o.num("width", self.width);
        o.str("unit", self.unit.name());
        opt_nums(o.key("dash"), &self.dash);
        o.num("dashOffset", self.dash_offset);
        o.str("cap", &self.cap);
        o.str("join", &self.join);
        o.num("blur", self.blur);
        o.num("level", self.level);
        o.end();
    }
}

impl Common {
    fn write_json(&self, out: &mut String, rotation: f64) {
        let mut o = Obj::new(out);
        o.str("unit", self.unit.name());
        o.num("opacity", self.opacity);
        nums(o.key("offset"), &self.offset);
        o.str("anchor", &self.anchor);
        o.num("rotation", rotation);
        o.num("level", self.level);
        o.end();
    }
}

impl MarkerStyle {
    /// The style's JSON; `key`: without the size, the height and the
    /// rotation, which vary per object without splitting a batch.
    fn write(&self, out: &mut String, key: bool) {
        let mut o = Obj::new(out);
        match &self.look {
            Look::Shape {
                shape,
                size,
                height,
                fill,
                stroke,
                stroke_width,
                params,
            } => {
                o.str("kind", "shape");
                o.str("shape", shape);
                if !key {
                    o.num("size", *size);
                    o.num("height", *height);
                }
                opt_str(o.key("fill"), fill);
                opt_str(o.key("stroke"), stroke);
                o.num("strokeWidth", *stroke_width);
                nums(o.key("params"), params);
            }
            Look::Svg {
                asset,
                size,
                fill,
                stroke,
            } => {
                o.str("kind", "svg");
                o.str("asset", asset);
                if !key {
                    o.num("size", *size);
                }
                opt_str(o.key("fill"), fill);
                opt_str(o.key("stroke"), stroke);
            }
            Look::Raster { asset, size } => {
                o.str("kind", "raster");
                o.str("asset", asset);
                if !key {
                    o.num("size", *size);
                }
            }
            Look::Text {
                text,
                size,
                font,
                weight,
                italic,
                color,
                halo,
            } => {
                o.str("kind", "text");
                o.str("text", text);
                if !key {
                    o.num("size", *size);
                }
                o.str("font", font);
                o.num("weight", *weight);
                o.bool("italic", *italic);
                o.str("color", color);
                let out = o.key("halo");
                match halo {
                    Some((c, w)) => {
                        let mut h = Obj::new(out);
                        h.str("color", c);
                        h.num("width", *w);
                        h.end();
                    }
                    None => out.push_str("null"),
                }
            }
        }
        let rotation = if key { 0.0 } else { self.common.rotation };
        self.common.write_json(o.key("common"), rotation);
        o.end();
    }

    pub fn write_json(&self, out: &mut String) {
        self.write(out, false);
    }

    /// What merges markers into one batch.
    pub fn key(&self) -> String {
        let mut out = String::from("m|");
        self.write(&mut out, true);
        out
    }
}

impl FillPaint {
    pub fn write_json(&self, out: &mut String) {
        let mut o = Obj::new(out);
        match self {
            FillPaint::Solid {
                color,
                opacity,
                level,
            } => {
                o.str("kind", "solid");
                o.str("color", color);
                o.num("opacity", *opacity);
                o.num("level", *level);
            }
            FillPaint::Hatch {
                color,
                opacity,
                angle,
                spacing,
                width,
                offset,
                dash,
                dash_offset,
                unit,
                level,
            } => {
                o.str("kind", "hatch");
                o.str("color", color);
                o.num("opacity", *opacity);
                o.num("angle", *angle);
                o.num("spacing", *spacing);
                o.num("width", *width);
                o.num("offset", *offset);
                opt_nums(o.key("dash"), dash);
                o.num("dashOffset", *dash_offset);
                o.str("unit", unit.name());
                o.num("level", *level);
            }
            FillPaint::Pattern {
                mark,
                size,
                stagger,
                angle,
                offset,
                jitter,
                coverage,
                seed,
                opacity,
                unit,
                level,
            } => {
                o.str("kind", "pattern");
                mark.write_json(o.key("mark"));
                nums(o.key("size"), size);
                o.bool("stagger", *stagger);
                o.num("angle", *angle);
                nums(o.key("offset"), offset);
                o.num("jitter", *jitter);
                o.num("coverage", *coverage);
                o.num("seed", *seed);
                o.num("opacity", *opacity);
                o.str("unit", unit.name());
                o.num("level", *level);
            }
            FillPaint::Tile {
                tile,
                size,
                angle,
                offset,
                opacity,
                unit,
                level,
            } => {
                o.str("kind", "tile");
                let out = o.key("tile");
                let mut t = Obj::new(out);
                match tile {
                    Tile::Asset(asset) => {
                        t.str("kind", "asset");
                        t.str("asset", asset);
                    }
                    Tile::Markers { markers, stagger } => {
                        t.str("kind", "markers");
                        let out = t.key("markers");
                        out.push('[');
                        for (i, m) in markers.iter().enumerate() {
                            if i > 0 {
                                out.push(',');
                            }
                            m.write_json(out);
                        }
                        out.push(']');
                        t.bool("stagger", *stagger);
                    }
                }
                t.end();
                nums(o.key("size"), size);
                o.num("angle", *angle);
                nums(o.key("offset"), offset);
                o.num("opacity", *opacity);
                o.str("unit", unit.name());
                o.num("level", *level);
            }
        }
        o.end();
    }

    pub fn level(&self) -> f64 {
        match self {
            FillPaint::Solid { level, .. }
            | FillPaint::Hatch { level, .. }
            | FillPaint::Pattern { level, .. }
            | FillPaint::Tile { level, .. } => *level,
        }
    }
}

/// A point as `{"x":…,"y":…}`.
pub fn point(out: &mut String, p: Vec2) {
    let mut o = Obj::new(out);
    o.num("x", p.x);
    o.num("y", p.y);
    o.end();
}

pub fn points(out: &mut String, pts: &[Vec2]) {
    out.push('[');
    for (i, p) in pts.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        point(out, *p);
    }
    out.push(']');
}

/// Collects primitives in order, as JSON (previews, legends and tests:
/// formerly the TypeScript `PrimitiveList`).
#[derive(Default)]
pub struct PrimitiveList {
    strokes: Vec<String>,
    fills: Vec<String>,
    markers: Vec<String>,
}

impl Sink for PrimitiveList {
    fn stroke(&mut self, style: &StrokeStyle, path: &[Vec2], closed: bool) {
        let mut out = String::new();
        let mut o = Obj::new(&mut out);
        style.write_json(o.key("style"));
        points(o.key("path"), path);
        o.bool("closed", closed);
        o.end();
        self.strokes.push(out);
    }

    fn fill(&mut self, paint: &FillPaint, rings: &[Vec<Vec2>]) {
        let mut out = String::new();
        let mut o = Obj::new(&mut out);
        paint.write_json(o.key("paint"));
        let r = o.key("rings");
        r.push('[');
        for (i, ring) in rings.iter().enumerate() {
            if i > 0 {
                r.push(',');
            }
            points(r, ring);
        }
        r.push(']');
        o.end();
        self.fills.push(out);
    }

    fn marker(&mut self, style: &MarkerStyle, at: Vec2, angle: f64) {
        let mut out = String::new();
        let mut o = Obj::new(&mut out);
        style.write_json(o.key("style"));
        point(o.key("at"), at);
        o.num("angle", angle);
        o.end();
        self.markers.push(out);
    }
}

impl PrimitiveList {
    /// `{"strokes":[…],"fills":[…],"markers":[…]}`.
    pub fn json(&self) -> String {
        let mut out = String::from("{");
        for (i, (name, list)) in [
            ("strokes", &self.strokes),
            ("fills", &self.fills),
            ("markers", &self.markers),
        ]
        .into_iter()
        .enumerate()
        {
            if i > 0 {
                out.push(',');
            }
            let _ = write!(out, "\"{name}\":[");
            out.push_str(&list.join(","));
            out.push(']');
        }
        out.push('}');
        out
    }
}
