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

use crate::expr::{Expr, compile};

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
#[derive(Default)]
pub struct Exprs {
    pub list: Vec<Option<Expr>>,
    index: HashMap<String, usize>,
}

impl Exprs {
    pub fn intern(&mut self, source: &str) -> usize {
        if let Some(&i) = self.index.get(source) {
            return i;
        }
        self.list.push(compile(source).ok());
        self.index.insert(source.to_string(), self.list.len() - 1);
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
                Json::Str(s) if !s.is_empty() => Some(Some(self.exprs.intern(s))),
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

    fn expr(&mut self, v: &Json) -> Option<usize> {
        match v {
            Json::Str(s) => Some(self.exprs.intern(s)),
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
            _ => return None,
        })
    }
}
