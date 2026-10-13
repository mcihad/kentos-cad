//! The style engine's data model as the core reads it (the types of
//! `model/style.ts`, docs/STYLE.md): symbols made of symbol layers, the
//! renderers that pick symbols for each object, and values that may come
//! from an expression per object. Symbols arrive as the JSON the app keeps
//! them in; an expression is compiled once, when a program is read, and a
//! value refers to it by index.
//!
//! Reading is lenient where the TypeScript was: a missing number is NaN, a
//! missing option takes its default, a layer of an unknown type is left out
//! of the drawing (the TypeScript skipped it too).

use std::collections::HashMap;

use kentos_geometry_core::api::json::Json;

use crate::expr::{Expr, Schema, Unit as ReadingUnit, compile, compile_with};

/// Size unit: paper mm at the plot scale (the default), screen px, metres.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Mm,
    Px,
    M,
}

/// A value fixed in the symbol or taken from an expression per object: the
/// expression by index (None: it does not compile, so the value is empty).
#[derive(Clone, Debug, PartialEq)]
pub enum Dd<T> {
    Fixed(T),
    Expr {
        expr: Option<usize>,
        fallback: Option<T>,
    },
}

/// The expressions of a program, compiled once each (equal sources share one).
///
/// Two kinds of expression meet in a program, and they speak different units
/// where they read geometry (`$alan`, `$uzunluk`, `$y` … docs/adr/0165 §2, the
/// 13 Ekim addendum):
/// - what a layer's renderer reads (a category's or a class's value, a rule's
///   condition, a colour's, a size's or a weight's number) is a reading, the
///   user compares it with what Öznitelikler shows: it speaks the program's
///   reading unit, the project's drawing unit (`reading_in`, `intern_reading`);
/// - what a symbol reads (a data-defined size, angle, distance, colour or
///   text, a geometry generator's steps) stays in metres whatever the project
///   (`intern`): a symbol is portable (Kitaplığım, `.kstil`), it is the same
///   symbol in every project it is dropped into, and the numbers it gives are
///   sizes and distances in its own unit (paper mm, px or metres).
#[derive(Default)]
pub struct Exprs {
    pub list: Vec<Option<Expr>>,
    index: HashMap<String, usize>,
    /// The readings compiled in the program's unit, apart from the metre ones.
    readings: HashMap<String, usize>,
    unit: ReadingUnit,
}

impl Exprs {
    /// Expressions whose renderer readings speak `unit` (metres: as `default`).
    pub fn reading_in(unit: ReadingUnit) -> Exprs {
        Exprs {
            unit,
            ..Exprs::default()
        }
    }

    /// A symbol's expression: geometry values in metres.
    pub fn intern(&mut self, source: &str) -> usize {
        if let Some(&i) = self.index.get(source) {
            return i;
        }
        self.list.push(compile(source).ok());
        self.index.insert(source.to_string(), self.list.len() - 1);
        self.list.len() - 1
    }

    /// A renderer's expression (its value, condition or weight): geometry
    /// values in the program's reading unit. In metres it is the symbol's own
    /// expression, one compile for both.
    pub fn intern_reading(&mut self, source: &str) -> usize {
        if self.unit == ReadingUnit::METRE {
            return self.intern(source);
        }
        if let Some(&i) = self.readings.get(source) {
            return i;
        }
        let schema = Schema {
            unit: self.unit,
            ..Schema::default()
        };
        self.list.push(compile_with(source, &schema).ok());
        self.readings
            .insert(source.to_string(), self.list.len() - 1);
        self.list.len() - 1
    }
}

fn num(v: &Json) -> f64 {
    match v {
        Json::Num(x) => *x,
        _ => f64::NAN,
    }
}

fn opt_num(v: &Json) -> Option<f64> {
    match v {
        Json::Num(x) => Some(*x),
        _ => None,
    }
}

fn opt_str(v: &Json) -> Option<String> {
    match v {
        Json::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn opt_bool(v: &Json) -> Option<bool> {
    match v {
        Json::Bool(b) => Some(*b),
        _ => None,
    }
}

fn pair(v: &Json) -> Option<[f64; 2]> {
    match v {
        Json::Arr(items) if items.len() == 2 => Some([num(&items[0]), num(&items[1])]),
        _ => None,
    }
}

fn numbers(v: &Json) -> Option<Vec<f64>> {
    match v {
        Json::Arr(items) => Some(items.iter().map(num).collect()),
        _ => None,
    }
}

/// A symbol layer's effects (docs/adr/0226 §2): the switched-on ones, the
/// first of each type; a missing value takes its default.
fn effects(v: &Json) -> Option<Effects> {
    let Json::Arr(items) = v else {
        return None;
    };
    let mut out = Effects::default();
    for e in items {
        if opt_bool(e.get("enabled")) == Some(false) {
            continue;
        }
        let n = |key: &str, default: f64| {
            opt_num(e.get(key))
                .filter(|x| x.is_finite())
                .unwrap_or(default)
        };
        let color = |default: &str| opt_str(e.get("color")).unwrap_or_else(|| default.to_owned());
        let shadow = |distance: f64, blur: f64| Shadow {
            distance: n("distance", distance),
            angle: n("angle", 135.0),
            blur: n("blur", blur),
            color: color("#000000"),
            opacity: n("opacity", 0.5),
        };
        let glow = |spread: f64, blur: f64, c: &str| Glow {
            spread: n("spread", spread),
            blur: n("blur", blur),
            color: color(c),
            opacity: n("opacity", 0.5),
        };
        match e.get("type") {
            Json::Str(t) if t == "dropShadow" && out.drop_shadow.is_none() => {
                out.drop_shadow = Some(shadow(1.0, 1.5));
            }
            Json::Str(t) if t == "outerGlow" && out.outer_glow.is_none() => {
                out.outer_glow = Some(glow(0.5, 1.5, "#FFFFFF"));
            }
            Json::Str(t) if t == "innerShadow" && out.inner_shadow.is_none() => {
                out.inner_shadow = Some(shadow(0.5, 1.0));
            }
            Json::Str(t) if t == "innerGlow" && out.inner_glow.is_none() => {
                out.inner_glow = Some(glow(0.5, 1.0, "#FFFFFF"));
            }
            Json::Str(t) if t == "blur" && out.blur.is_none() => out.blur = Some(n("blur", 1.0)),
            _ => {}
        }
    }
    (!out.is_empty()).then_some(out)
}

impl Exprs {
    /// A data-defined value: absent (None), fixed (`fixed` reads it) or an expression object.
    fn dd<T>(&mut self, v: &Json, fixed: impl Fn(&Json) -> Option<T>) -> Option<Dd<T>> {
        match v {
            Json::Null => None,
            Json::Obj(_) => Some(Dd::Expr {
                expr: match v.get("expr") {
                    Json::Str(s) => Some(self.intern(s)),
                    _ => None,
                },
                fallback: fixed(v.get("fallback")),
            }),
            v => fixed(v).map(Dd::Fixed).or(Some(Dd::Expr {
                expr: None,
                fallback: None,
            })),
        }
    }

    fn dd_num(&mut self, v: &Json) -> Option<Dd<f64>> {
        self.dd(v, opt_num)
    }

    fn dd_str(&mut self, v: &Json) -> Option<Dd<String>> {
        self.dd(v, opt_str)
    }

    fn dd_bool(&mut self, v: &Json) -> Option<Dd<bool>> {
        self.dd(v, opt_bool)
    }
}

// ── Symbol layers ──────────────────────────────────────────────────────

/// What every symbol layer has.
#[derive(Clone, Debug)]
pub struct Base {
    pub enabled: Option<Dd<bool>>,
    pub opacity: Option<f64>,
    pub unit: Unit,
    /// Its draw effects (docs/adr/0226 §2); None when it has none switched on.
    pub effects: Option<Effects>,
}

/// A shadow (Gölge, İç gölge; docs/adr/0226 §2): its distance and angle
/// (degrees, clockwise from north), blur, colour and opacity, in the
/// layer's unit.
#[derive(Clone, Debug, PartialEq)]
pub struct Shadow {
    pub distance: f64,
    pub angle: f64,
    pub blur: f64,
    pub color: String,
    pub opacity: f64,
}

/// A glow (Dış parıltı, İç parıltı): its spread, blur, colour and opacity.
#[derive(Clone, Debug, PartialEq)]
pub struct Glow {
    pub spread: f64,
    pub blur: f64,
    pub color: String,
    pub opacity: f64,
}

/// A symbol layer's draw effects, each at most once; their order is fixed
/// (docs/adr/0226 §2): shadow, outer glow, the layer (blurred by `blur`),
/// inner shadow, inner glow.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Effects {
    pub drop_shadow: Option<Shadow>,
    pub outer_glow: Option<Glow>,
    pub inner_shadow: Option<Shadow>,
    pub inner_glow: Option<Glow>,
    pub blur: Option<f64>,
}

impl Effects {
    fn is_empty(&self) -> bool {
        self.drop_shadow.is_none()
            && self.outer_glow.is_none()
            && self.inner_shadow.is_none()
            && self.inner_glow.is_none()
            && self.blur.is_none()
    }
}

/// A step of a geometry generator (docs/adr/0226 §1); its numbers may come
/// from an expression per object.
#[derive(Clone, Debug)]
pub enum Step {
    Centroid,
    PointOnSurface,
    /// `all`, `inner`, `first`, `last`, `ends`.
    Vertices(String),
    Boundary,
    Offset {
        distance: Option<Dd<f64>>,
    },
    Extend {
        start: Option<Dd<f64>>,
        end: Option<Dd<f64>>,
    },
    /// `side`: `both`, `left`, `right`.
    Buffer {
        distance: Option<Dd<f64>>,
        side: String,
    },
    Envelope,
    Hull,
    Circle {
        radius: Option<Dd<f64>>,
    },
    Wedge {
        direction: Option<Dd<f64>>,
        width: Option<Dd<f64>>,
        radius: Option<Dd<f64>>,
        inner: Option<Dd<f64>>,
    },
    Polygon {
        sides: Option<Dd<f64>>,
        radius: Option<Dd<f64>>,
        rotation: Option<Dd<f64>>,
    },
    Translate {
        dx: Option<Dd<f64>>,
        dy: Option<Dd<f64>>,
    },
    Rotate {
        angle: Option<Dd<f64>>,
    },
    Scale {
        x: Option<Dd<f64>>,
        y: Option<Dd<f64>>,
    },
    /// A step the engine does not know: its result is empty.
    Unknown,
}

/// Geometri üretici (docs/adr/0226 §1): the object's geometry through its
/// steps, drawn with its own symbol.
#[derive(Clone, Debug)]
pub struct GeometryGenerator {
    pub base: Base,
    pub steps: Vec<Step>,
    pub symbol: Option<Symbol>,
}

#[derive(Clone, Debug)]
pub enum MarkerKind {
    Shape {
        shape: String,
        height: Option<f64>,
        fill: Option<Dd<String>>,
        stroke: Option<Dd<String>>,
        stroke_width: Option<f64>,
        hole: Option<f64>,
        teeth: Option<f64>,
        teeth_depth: Option<f64>,
        sweep: Option<f64>,
    },
    Svg {
        asset: String,
        fill: Option<Dd<String>>,
        stroke: Option<Dd<String>>,
    },
    Text {
        text: Option<Dd<String>>,
        font: Option<String>,
        weight: Option<f64>,
        italic: Option<bool>,
        color: Option<Dd<String>>,
        halo: Option<(String, f64)>,
    },
    Raster {
        asset: String,
    },
}

#[derive(Clone, Debug)]
pub struct MarkerLayer {
    pub base: Base,
    pub size: Option<Dd<f64>>,
    pub rotation: Option<Dd<f64>>,
    pub offset: Option<[f64; 2]>,
    pub anchor: Option<String>,
    pub kind: MarkerKind,
}

/// A wavy line (`LineWave`), in the layer unit.
#[derive(Clone, Debug)]
pub struct Wave {
    pub shape: String,
    pub length: f64,
    pub amplitude: f64,
    pub spacing: Option<f64>,
    pub connect: Option<bool>,
    pub offset_along: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct SimpleLine {
    pub base: Base,
    pub color: Option<Dd<String>>,
    pub width: Option<Dd<f64>>,
    pub dash: Option<Vec<f64>>,
    pub dash_offset: Option<f64>,
    pub cap: Option<String>,
    pub join: Option<String>,
    pub offset: Option<Dd<f64>>,
    pub rings: Option<String>,
    pub wave: Option<Wave>,
    pub blur: Option<f64>,
    pub shift: Option<[f64; 2]>,
}

#[derive(Clone, Debug)]
pub struct MarkerLine {
    pub base: Base,
    /// The marker symbol's layers (None: a layer that is not a marker).
    pub marker: Vec<Option<MarkerLayer>>,
    pub placement: String,
    pub interval: Option<f64>,
    pub offset_along: Option<f64>,
    pub offset: Option<Dd<f64>>,
    pub rotate: Option<bool>,
    pub rings: Option<String>,
    /// Count and spacing.
    pub group: Option<(f64, f64)>,
}

#[derive(Clone, Debug)]
pub struct HatchFill {
    pub base: Base,
    pub angle: f64,
    pub spacing: f64,
    pub width: f64,
    pub color: Option<Dd<String>>,
    pub offset: Option<f64>,
    pub dash: Option<Vec<f64>>,
    pub dash_offset: Option<f64>,
    /// How much further along each line starts than the one before it
    /// (a hatch pattern's family, docs/adr/0186 §3).
    pub stagger: Option<f64>,
}

/// A gradient over the area (docs/adr/0186 §3): from `color` to `color2`,
/// `linear` along `angle` (degrees), `cylinder` (the second colour along
/// the middle) or `spherical` (the second colour in the middle); `inverted`
/// runs it the other way. Its frame is the area's own box.
#[derive(Clone, Debug)]
pub struct GradientFill {
    pub base: Base,
    pub color: Option<Dd<String>>,
    pub color2: Option<Dd<String>>,
    pub shape: String,
    pub angle: f64,
    pub inverted: bool,
}

#[derive(Clone, Debug)]
pub struct PatternFill {
    pub base: Base,
    pub marker: Vec<Option<MarkerLayer>>,
    pub spacing_x: f64,
    pub spacing_y: f64,
    pub stagger: Option<bool>,
    pub angle: Option<f64>,
    pub offset: Option<[f64; 2]>,
    pub jitter: Option<f64>,
    pub coverage: Option<f64>,
    pub seed: Option<f64>,
}

/// A symbol layer of any class; which ones a symbol draws depends on its
/// type (a fill symbol draws line layers on its edges).
#[derive(Clone, Debug)]
pub enum Layer {
    Marker(MarkerLayer),
    SimpleLine(SimpleLine),
    MarkerLine(MarkerLine),
    SimpleFill {
        base: Base,
        color: Option<Dd<String>>,
    },
    HatchFill(HatchFill),
    GradientFill(GradientFill),
    PatternFill(PatternFill),
    ImageFill {
        base: Base,
        asset: String,
        tile_size: f64,
        angle: Option<f64>,
    },
    CentroidMarker {
        base: Base,
        marker: Vec<Option<MarkerLayer>>,
        position: Option<String>,
    },
    GeometryGenerator(GeometryGenerator),
    /// A layer type the engine does not know.
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolType {
    Marker,
    Line,
    Fill,
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub kind: SymbolType,
    pub layers: Vec<Layer>,
}

impl Exprs {
    fn base(&mut self, v: &Json) -> Base {
        Base {
            enabled: self.dd_bool(v.get("enabled")),
            opacity: opt_num(v.get("opacity")),
            unit: match v.get("unit") {
                Json::Str(s) if s == "px" => Unit::Px,
                Json::Str(s) if s == "m" => Unit::M,
                _ => Unit::Mm,
            },
            effects: effects(v.get("effects")),
        }
    }

    /// A geometry generator's steps (docs/adr/0226 §1), their expressions interned here.
    pub fn steps(&mut self, v: &Json) -> Vec<Step> {
        match v {
            Json::Arr(items) => items.iter().map(|s| self.step(s)).collect(),
            _ => Vec::new(),
        }
    }

    /// A geometry generator's step.
    fn step(&mut self, v: &Json) -> Step {
        let mut n = |key: &str| self.dd_num(v.get(key));
        match v.get("kind") {
            Json::Str(k) => match k.as_str() {
                "centroid" => Step::Centroid,
                "pointOnSurface" => Step::PointOnSurface,
                "vertices" => {
                    Step::Vertices(opt_str(v.get("which")).unwrap_or_else(|| "all".to_owned()))
                }
                "boundary" => Step::Boundary,
                "offset" => Step::Offset {
                    distance: n("distance"),
                },
                "extend" => Step::Extend {
                    start: n("start"),
                    end: n("end"),
                },
                "buffer" => Step::Buffer {
                    distance: n("distance"),
                    side: opt_str(v.get("side")).unwrap_or_else(|| "both".to_owned()),
                },
                "envelope" => Step::Envelope,
                "hull" => Step::Hull,
                "circle" => Step::Circle {
                    radius: n("radius"),
                },
                "wedge" => Step::Wedge {
                    direction: n("direction"),
                    width: n("width"),
                    radius: n("radius"),
                    inner: n("inner"),
                },
                "polygon" => Step::Polygon {
                    sides: n("sides"),
                    radius: n("radius"),
                    rotation: n("rotation"),
                },
                "translate" => Step::Translate {
                    dx: n("dx"),
                    dy: n("dy"),
                },
                "rotate" => Step::Rotate { angle: n("angle") },
                "scale" => Step::Scale {
                    x: n("x"),
                    y: n("y"),
                },
                _ => Step::Unknown,
            },
            _ => Step::Unknown,
        }
    }

    fn marker_layer(&mut self, v: &Json) -> Option<MarkerLayer> {
        let kind = match v.get("type") {
            Json::Str(t) if t == "shape" => MarkerKind::Shape {
                shape: opt_str(v.get("shape")).unwrap_or_default(),
                height: opt_num(v.get("height")),
                fill: self.dd_str(v.get("fill")),
                stroke: self.dd_str(v.get("stroke")),
                stroke_width: opt_num(v.get("strokeWidth")),
                hole: opt_num(v.get("hole")),
                teeth: opt_num(v.get("teeth")),
                teeth_depth: opt_num(v.get("teethDepth")),
                sweep: opt_num(v.get("sweep")),
            },
            Json::Str(t) if t == "svg" => MarkerKind::Svg {
                asset: opt_str(v.get("asset")).unwrap_or_default(),
                fill: self.dd_str(v.get("fill")),
                stroke: self.dd_str(v.get("stroke")),
            },
            Json::Str(t) if t == "text" => MarkerKind::Text {
                text: self.dd_str(v.get("text")),
                font: opt_str(v.get("font")),
                weight: opt_num(v.get("weight")),
                italic: opt_bool(v.get("italic")),
                color: self.dd_str(v.get("color")),
                halo: match v.get("halo") {
                    h @ Json::Obj(_) => Some((
                        opt_str(h.get("color")).unwrap_or_default(),
                        num(h.get("width")),
                    )),
                    _ => None,
                },
            },
            Json::Str(t) if t == "raster" => MarkerKind::Raster {
                asset: opt_str(v.get("asset")).unwrap_or_default(),
            },
            _ => return None,
        };
        Some(MarkerLayer {
            base: self.base(v),
            size: self.dd_num(v.get("size")),
            rotation: self.dd_num(v.get("rotation")),
            offset: pair(v.get("offset")),
            anchor: opt_str(v.get("anchor")),
            kind,
        })
    }

    /// A marker symbol's layers (inside a marker line, a pattern, a centroid marker).
    fn marker_symbol(&mut self, v: &Json) -> Vec<Option<MarkerLayer>> {
        match v.get("layers") {
            Json::Arr(items) => items.iter().map(|l| self.marker_layer(l)).collect(),
            _ => Vec::new(),
        }
    }

    pub fn layer(&mut self, v: &Json) -> Layer {
        let base = self.base(v);
        match v.get("type") {
            Json::Str(t) => match t.as_str() {
                "shape" | "svg" | "text" | "raster" => {
                    self.marker_layer(v).map_or(Layer::Unknown, Layer::Marker)
                }
                "simpleLine" => Layer::SimpleLine(SimpleLine {
                    base,
                    color: self.dd_str(v.get("color")),
                    width: self.dd_num(v.get("width")),
                    dash: numbers(v.get("dash")),
                    dash_offset: opt_num(v.get("dashOffset")),
                    cap: opt_str(v.get("cap")),
                    join: opt_str(v.get("join")),
                    offset: self.dd_num(v.get("offset")),
                    rings: opt_str(v.get("rings")),
                    wave: match v.get("wave") {
                        w @ Json::Obj(_) => Some(Wave {
                            shape: opt_str(w.get("shape")).unwrap_or_default(),
                            length: num(w.get("length")),
                            amplitude: num(w.get("amplitude")),
                            spacing: opt_num(w.get("spacing")),
                            connect: opt_bool(w.get("connect")),
                            offset_along: opt_num(w.get("offsetAlong")),
                        }),
                        _ => None,
                    },
                    blur: opt_num(v.get("blur")),
                    shift: pair(v.get("shift")),
                }),
                "markerLine" => Layer::MarkerLine(MarkerLine {
                    base,
                    marker: self.marker_symbol(v.get("marker")),
                    placement: opt_str(v.get("placement")).unwrap_or_default(),
                    interval: opt_num(v.get("interval")),
                    offset_along: opt_num(v.get("offsetAlong")),
                    offset: self.dd_num(v.get("offset")),
                    rotate: opt_bool(v.get("rotate")),
                    rings: opt_str(v.get("rings")),
                    group: match v.get("group") {
                        g @ Json::Obj(_) => Some((num(g.get("count")), num(g.get("spacing")))),
                        _ => None,
                    },
                }),
                "simpleFill" => Layer::SimpleFill {
                    base,
                    color: self.dd_str(v.get("color")),
                },
                "hatchFill" => Layer::HatchFill(HatchFill {
                    base,
                    angle: num(v.get("angle")),
                    spacing: num(v.get("spacing")),
                    width: num(v.get("width")),
                    color: self.dd_str(v.get("color")),
                    offset: opt_num(v.get("offset")),
                    dash: numbers(v.get("dash")),
                    dash_offset: opt_num(v.get("dashOffset")),
                    stagger: opt_num(v.get("stagger")),
                }),
                "gradientFill" => Layer::GradientFill(GradientFill {
                    base,
                    color: self.dd_str(v.get("color")),
                    color2: self.dd_str(v.get("color2")),
                    shape: opt_str(v.get("shape")).unwrap_or_else(|| "linear".to_owned()),
                    angle: opt_num(v.get("angle")).unwrap_or(0.0),
                    inverted: opt_bool(v.get("inverted")).unwrap_or(false),
                }),
                "patternFill" => Layer::PatternFill(PatternFill {
                    base,
                    marker: self.marker_symbol(v.get("marker")),
                    spacing_x: num(v.get("spacingX")),
                    spacing_y: num(v.get("spacingY")),
                    stagger: opt_bool(v.get("stagger")),
                    angle: opt_num(v.get("angle")),
                    offset: pair(v.get("offset")),
                    jitter: opt_num(v.get("jitter")),
                    coverage: opt_num(v.get("coverage")),
                    seed: opt_num(v.get("seed")),
                }),
                "imageFill" => Layer::ImageFill {
                    base,
                    asset: opt_str(v.get("asset")).unwrap_or_default(),
                    tile_size: num(v.get("tileSize")),
                    angle: opt_num(v.get("angle")),
                },
                "centroidMarker" => Layer::CentroidMarker {
                    base,
                    marker: self.marker_symbol(v.get("marker")),
                    position: opt_str(v.get("position")),
                },
                "geometryGenerator" => Layer::GeometryGenerator(GeometryGenerator {
                    base,
                    steps: self.steps(v.get("steps")),
                    symbol: self.symbol(v.get("symbol")),
                }),
                _ => Layer::Unknown,
            },
            _ => Layer::Unknown,
        }
    }

    /// A symbol, or None when the JSON is not one.
    pub fn symbol(&mut self, v: &Json) -> Option<Symbol> {
        let kind = match v.get("type") {
            Json::Str(t) if t == "marker" => SymbolType::Marker,
            Json::Str(t) if t == "line" => SymbolType::Line,
            Json::Str(t) if t == "fill" => SymbolType::Fill,
            _ => return None,
        };
        let layers = match v.get("layers") {
            Json::Arr(items) => items.iter().map(|l| self.layer(l)).collect(),
            _ => Vec::new(),
        };
        Some(Symbol { kind, layers })
    }
}

// ── Renderers ──────────────────────────────────────────────────────────

/// A symbol from the library (by its index in the program's table; None:
/// the library has no such symbol) or written in place.
#[derive(Clone, Debug)]
pub enum SymbolRef {
    Library(Option<usize>),
    Inline(usize),
}

/// What an object draws for each class of geometry.
#[derive(Clone, Debug, Default)]
pub struct SymbolSet {
    pub marker: Option<SymbolRef>,
    pub line: Option<SymbolRef>,
    pub fill: Option<SymbolRef>,
}

#[derive(Clone, Debug)]
pub struct Rule {
    /// The condition: None for every object; Some(None) when it does not compile.
    pub filter: Option<Option<usize>>,
    pub is_else: bool,
    pub min_scale: Option<f64>,
    pub max_scale: Option<f64>,
    pub symbols: Option<SymbolSet>,
    pub children: Vec<Rule>,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct Category {
    pub value: String,
    pub symbols: SymbolSet,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct Class {
    pub min: f64,
    pub max: f64,
    pub symbols: SymbolSet,
}

/// A value of a thematic renderer's list (Nokta yoğunluğu, Grafik): its
/// expression and colour (docs/adr/0213 §2.4, §2.5).
#[derive(Clone, Debug)]
pub struct Field {
    pub expr: Option<usize>,
    pub color: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChartKind {
    Pie,
    Bar,
    Stacked,
}

/// A pie's diameter from its total (docs/adr/0213 §2.5).
#[derive(Clone, Copy, Debug)]
pub struct SizeBy {
    pub min_value: f64,
    pub max_value: f64,
    pub min_size: f64,
    pub max_size: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    Ring,
    Rings,
    Grid,
}

/// A colour of a ramp, a grid or a list: red, green, blue, alpha.
pub type Rgba = [u8; 4];

#[derive(Clone, Debug)]
pub enum Renderer {
    Single(SymbolSet),
    Categorized {
        expr: Option<usize>,
        categories: Vec<Category>,
        other: Option<SymbolSet>,
    },
    Graduated {
        expr: Option<usize>,
        classes: Vec<Class>,
    },
    Rules(Vec<Rule>),
    /// Sürekli renk (docs/adr/0213 §2.1).
    Unclassed {
        expr: Option<usize>,
        min: f64,
        max: f64,
        ramp: Vec<Rgba>,
        symbols: SymbolSet,
        other: Option<SymbolSet>,
    },
    /// Orantılı sembol (§2.2): `exponent` 0.5 (Alan), 1 (Yarıçap), 0.57 (Flannery).
    Proportional {
        expr: Option<usize>,
        min_value: f64,
        max_value: f64,
        min_size: f64,
        max_size: f64,
        unit: Unit,
        exponent: f64,
        symbols: SymbolSet,
        other: Option<SymbolSet>,
    },
    /// İki değişkenli renk (§2.3): `colors[j · n + i]`.
    Bivariate {
        expr_x: Option<usize>,
        expr_y: Option<usize>,
        breaks_x: Vec<f64>,
        breaks_y: Vec<f64>,
        colors: Vec<Option<Rgba>>,
        symbols: SymbolSet,
        other: Option<SymbolSet>,
    },
    /// Nokta yoğunluğu (§2.4).
    DotDensity {
        fields: Vec<Field>,
        dot_value: f64,
        dot_size: f64,
        unit: Unit,
        seed: u64,
        symbols: Option<SymbolSet>,
    },
    /// Grafik (§2.5); `max_value` NaN when not given.
    Chart {
        kind: ChartKind,
        fields: Vec<Field>,
        size: f64,
        unit: Unit,
        size_by: Option<SizeBy>,
        max_value: f64,
        bar_width: Option<f64>,
        outline: Option<(String, f64)>,
        symbols: Option<SymbolSet>,
    },
    /// Isı haritası (§2.6); `max` None: dynamic.
    Heatmap {
        radius: f64,
        unit: Unit,
        weight: Option<usize>,
        max: Option<f64>,
        ramp: Vec<Rgba>,
        quality: u32,
        opacity: f64,
    },
    /// Kümeleme (§2.7); `inner` draws single points and the other objects.
    Cluster {
        distance: f64,
        unit: Unit,
        symbol: Option<SymbolRef>,
        count: bool,
        grow: bool,
        inner: Option<Box<Renderer>>,
    },
    /// Yayma (§2.8).
    Displacement {
        tolerance: f64,
        unit: Unit,
        placement: Placement,
        spacing: f64,
        center: Option<SymbolRef>,
        circle: Option<(String, f64)>,
        inner: Option<Box<Renderer>>,
    },
    /// Ters alan (§2.9); `merge`: overlaps stay empty (non-zero winding).
    Inverted {
        symbols: SymbolSet,
        merge: bool,
    },
}

impl Renderer {
    /// What its build depends on beyond the objects (docs/adr/0213 §3): the
    /// view's scale, and a box around the view.
    pub fn view_needs(&self) -> ViewNeeds {
        match self {
            Renderer::Heatmap { .. } => ViewNeeds {
                scale: true,
                frame: Some(Frame::Heat),
            },
            Renderer::Cluster { unit, .. } => ViewNeeds {
                scale: *unit != Unit::M,
                frame: None,
            },
            Renderer::Displacement { unit, .. } => ViewNeeds {
                scale: *unit != Unit::M,
                frame: None,
            },
            Renderer::Chart { unit, .. } => ViewNeeds {
                scale: *unit == Unit::Px,
                frame: None,
            },
            Renderer::Inverted { .. } => ViewNeeds {
                scale: false,
                frame: Some(Frame::Construction),
            },
            _ => ViewNeeds::default(),
        }
    }

    /// Whether its objects are drawn together (a layer built whole, never in parts).
    pub fn whole(&self) -> bool {
        matches!(
            self,
            Renderer::Heatmap { .. }
                | Renderer::Cluster { .. }
                | Renderer::Displacement { .. }
                | Renderer::Inverted { .. }
        )
    }
}

/// The box a view-dependent build takes (docs/adr/0213 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Frame {
    /// The view and half of it on every side.
    Heat,
    /// The construction lines' box: the view and three times its size.
    Construction,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewNeeds {
    /// The view's scale, rounded to quarter octaves.
    pub scale: bool,
    pub frame: Option<Frame>,
}

/// The symbols of a program: library symbols by id, and those written in place.
#[derive(Default)]
pub struct Symbols {
    pub list: Vec<Symbol>,
    by_id: HashMap<String, usize>,
}

impl Symbols {
    pub fn get(&self, r: &SymbolRef) -> Option<&Symbol> {
        match r {
            SymbolRef::Library(i) => i.and_then(|i| self.list.get(i)),
            SymbolRef::Inline(i) => self.list.get(*i),
        }
    }
}

/// Reads symbols, sets and renderers into one program's tables.
pub struct Reader<'a> {
    pub exprs: &'a mut Exprs,
    pub symbols: &'a mut Symbols,
}

impl Reader<'_> {
    /// The library's symbols, by id (a program holds the ones its layer uses).
    pub fn library(&mut self, v: &Json) {
        if let Json::Obj(fields) = v {
            for (id, s) in fields {
                if let Some(sym) = self.exprs.symbol(s) {
                    self.symbols.list.push(sym);
                    self.symbols
                        .by_id
                        .insert(id.clone(), self.symbols.list.len() - 1);
                }
            }
        }
    }

    pub fn symbol_ref(&mut self, v: &Json) -> Option<SymbolRef> {
        match v {
            Json::Obj(_) => match v.get("ref") {
                Json::Str(id) => Some(SymbolRef::Library(self.symbols.by_id.get(id).copied())),
                _ => {
                    let sym = self.exprs.symbol(v)?;
                    self.symbols.list.push(sym);
                    Some(SymbolRef::Inline(self.symbols.list.len() - 1))
                }
            },
            _ => None,
        }
    }

    /// A library symbol by id, as an object's own symbol refers to it.
    pub fn library_ref(&self, id: &str) -> SymbolRef {
        SymbolRef::Library(self.symbols.by_id.get(id).copied())
    }

    pub fn set(&mut self, v: &Json) -> SymbolSet {
        SymbolSet {
            marker: self.symbol_ref(v.get("marker")),
            line: self.symbol_ref(v.get("line")),
            fill: self.symbol_ref(v.get("fill")),
        }
    }

    fn opt_set(&mut self, v: &Json) -> Option<SymbolSet> {
        matches!(v, Json::Obj(_)).then(|| self.set(v))
    }

    fn rule(&mut self, v: &Json) -> Rule {
        Rule {
            filter: match v.get("filter") {
                Json::Str(s) if !s.is_empty() => Some(Some(self.exprs.intern_reading(s))),
                _ => None,
            },
            is_else: opt_bool(v.get("isElse")).unwrap_or(false),
            min_scale: opt_num(v.get("minScale")),
            max_scale: opt_num(v.get("maxScale")),
            symbols: self.opt_set(v.get("symbols")),
            children: match v.get("children") {
                Json::Arr(items) => items.iter().map(|r| self.rule(r)).collect(),
                _ => Vec::new(),
            },
            enabled: opt_bool(v.get("enabled")) != Some(false),
        }
    }

    /// A renderer's expression: a reading in the program's unit (`Exprs`).
    fn expr(&mut self, v: &Json) -> Option<usize> {
        match v {
            Json::Str(s) => Some(self.exprs.intern_reading(s)),
            _ => None,
        }
    }

    /// A layer renderer, or None when the JSON is not one.
    pub fn renderer(&mut self, v: &Json) -> Option<Renderer> {
        let Json::Str(t) = v.get("type") else {
            return None;
        };
        Some(match t.as_str() {
            "single" => Renderer::Single(self.set(v.get("symbols"))),
            "categorized" => Renderer::Categorized {
                expr: self.expr(v.get("expr")),
                categories: match v.get("categories") {
                    Json::Arr(items) => items
                        .iter()
                        .map(|c| Category {
                            value: opt_str(c.get("value")).unwrap_or_default(),
                            symbols: self.set(c.get("symbols")),
                            enabled: opt_bool(c.get("enabled")) != Some(false),
                        })
                        .collect(),
                    _ => Vec::new(),
                },
                other: self.opt_set(v.get("other")),
            },
            "graduated" => Renderer::Graduated {
                expr: self.expr(v.get("expr")),
                classes: match v.get("classes") {
                    Json::Arr(items) => items
                        .iter()
                        .map(|c| Class {
                            min: num(c.get("min")),
                            max: num(c.get("max")),
                            symbols: self.set(c.get("symbols")),
                        })
                        .collect(),
                    _ => Vec::new(),
                },
            },
            "rules" => Renderer::Rules(match v.get("rules") {
                Json::Arr(items) => items.iter().map(|r| self.rule(r)).collect(),
                _ => Vec::new(),
            }),
            "unclassed" => Renderer::Unclassed {
                expr: self.expr(v.get("expr")),
                min: num(v.get("min")),
                max: num(v.get("max")),
                ramp: ramp(v.get("ramp")),
                symbols: self.set(v.get("symbols")),
                other: self.opt_set(v.get("other")),
            },
            "proportional" => Renderer::Proportional {
                expr: self.expr(v.get("expr")),
                min_value: num(v.get("minValue")),
                max_value: num(v.get("maxValue")),
                min_size: num(v.get("minSize")),
                max_size: num(v.get("maxSize")),
                unit: size_unit(v.get("unit")),
                exponent: match v.get("scaling") {
                    Json::Str(s) if s == "radius" => 1.0,
                    Json::Str(s) if s == "flannery" => 0.57,
                    _ => 0.5,
                },
                symbols: self.set(v.get("symbols")),
                other: self.opt_set(v.get("other")),
            },
            "bivariate" => Renderer::Bivariate {
                expr_x: self.expr(v.get("exprX")),
                expr_y: self.expr(v.get("exprY")),
                breaks_x: numbers(v.get("breaksX")).unwrap_or_default(),
                breaks_y: numbers(v.get("breaksY")).unwrap_or_default(),
                colors: match v.get("colors") {
                    Json::Arr(items) => items
                        .iter()
                        .map(|c| match c {
                            Json::Str(s) => super::thematic::parse_rgba(s),
                            _ => None,
                        })
                        .collect(),
                    _ => Vec::new(),
                },
                symbols: self.set(v.get("symbols")),
                other: self.opt_set(v.get("other")),
            },
            "dotDensity" => Renderer::DotDensity {
                fields: self.fields(v.get("fields")),
                dot_value: num(v.get("dotValue")),
                dot_size: opt_num(v.get("dotSize")).unwrap_or(1.0),
                unit: size_unit(v.get("unit")),
                seed: match v.get("seed") {
                    Json::Num(x) if x.is_finite() && *x >= 0.0 => *x as u64,
                    _ => 0,
                },
                symbols: self.opt_set(v.get("symbols")),
            },
            "chart" => Renderer::Chart {
                kind: match v.get("kind") {
                    Json::Str(s) if s == "bar" => ChartKind::Bar,
                    Json::Str(s) if s == "stacked" => ChartKind::Stacked,
                    _ => ChartKind::Pie,
                },
                fields: self.fields(v.get("fields")),
                size: num(v.get("size")),
                unit: size_unit(v.get("unit")),
                size_by: match v.get("sizeBy") {
                    s @ Json::Obj(_) => Some(SizeBy {
                        min_value: num(s.get("minValue")),
                        max_value: num(s.get("maxValue")),
                        min_size: num(s.get("minSize")),
                        max_size: num(s.get("maxSize")),
                    }),
                    _ => None,
                },
                max_value: num(v.get("maxValue")),
                bar_width: opt_num(v.get("barWidth")),
                outline: stroke_of(v.get("outline")),
                symbols: self.opt_set(v.get("symbols")),
            },
            "heatmap" => Renderer::Heatmap {
                radius: num(v.get("radius")),
                unit: distance_unit(v.get("unit")),
                weight: match v.get("weight") {
                    Json::Str(s) if !s.trim().is_empty() => Some(self.exprs.intern_reading(s)),
                    _ => None,
                },
                max: opt_num(v.get("max")).filter(|m| *m > 0.0),
                ramp: ramp(v.get("ramp")),
                quality: match v.get("quality") {
                    Json::Num(q) if q.is_finite() => {
                        kentos_geometry_core::jsmath::js_round(*q).clamp(1.0, 5.0) as u32
                    }
                    _ => 2,
                },
                opacity: opt_num(v.get("opacity")).unwrap_or(1.0),
            },
            "cluster" => Renderer::Cluster {
                distance: num(v.get("distance")),
                unit: distance_unit(v.get("unit")),
                symbol: self.symbol_ref(v.get("symbol")),
                count: opt_bool(v.get("count")) != Some(false),
                grow: opt_bool(v.get("grow")) == Some(true),
                inner: self.inner(v.get("renderer")),
            },
            "displacement" => Renderer::Displacement {
                tolerance: num(v.get("tolerance")),
                unit: distance_unit(v.get("unit")),
                placement: match v.get("placement") {
                    Json::Str(s) if s == "rings" => Placement::Rings,
                    Json::Str(s) if s == "grid" => Placement::Grid,
                    _ => Placement::Ring,
                },
                spacing: opt_num(v.get("spacing")).unwrap_or(0.0),
                center: self.symbol_ref(v.get("center")),
                circle: stroke_of(v.get("circle")),
                inner: self.inner(v.get("renderer")),
            },
            "inverted" => Renderer::Inverted {
                symbols: self.set(v.get("symbols")),
                merge: opt_bool(v.get("merge")) == Some(true),
            },
            _ => return None,
        })
    }

    /// The list of a dot density or a chart.
    fn fields(&mut self, v: &Json) -> Vec<Field> {
        match v {
            Json::Arr(items) => items
                .iter()
                .map(|f| Field {
                    expr: self.expr(f.get("expr")),
                    color: opt_str(f.get("color")).unwrap_or_default(),
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// A cluster's or a displacement's renderer of single points: one that
    /// draws objects one by one (the wrappers do not nest).
    fn inner(&mut self, v: &Json) -> Option<Box<Renderer>> {
        let r = self.renderer(v)?;
        (!r.whole() && !matches!(r, Renderer::DotDensity { .. } | Renderer::Chart { .. }))
            .then(|| Box::new(r))
    }
}

/// A ramp's colours (unreadable ones left out).
fn ramp(v: &Json) -> Vec<Rgba> {
    match v {
        Json::Arr(items) => items
            .iter()
            .filter_map(|c| match c {
                Json::Str(s) => super::thematic::parse_rgba(s),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A size's unit: paper mm (the default) or screen px.
fn size_unit(v: &Json) -> Unit {
    match v {
        Json::Str(s) if s == "px" => Unit::Px,
        _ => Unit::Mm,
    }
}

/// A distance's unit: screen px (the default) or metres.
fn distance_unit(v: &Json) -> Unit {
    match v {
        Json::Str(s) if s == "m" => Unit::M,
        _ => Unit::Px,
    }
}

/// `{ color, width }`.
fn stroke_of(v: &Json) -> Option<(String, f64)> {
    match v {
        Json::Obj(_) => Some((
            opt_str(v.get("color")).unwrap_or_default(),
            opt_num(v.get("width")).unwrap_or(0.0),
        )),
        _ => None,
    }
}
