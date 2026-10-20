//! A layer's look as a MapLibre GL style, version 8 (docs/adr/0560, plan/gis.md CBS-17): a style's `fill`, `line`,
//! `circle` and `symbol` layers over the source the layer's objects are in, written from KentOS's renderers
//! (`single`, `categorized`, `graduated`, `rules`) and read back into them. A categorized or graduated renderer whose
//! classes draw alike in structure becomes one style layer per symbol layer with `match` and `step` expressions in
//! its paint values; one whose classes differ (a width in metres, a dash, another symbol layer count) becomes one
//! style layer per class with a `filter`. Rules become filters, nested rules the parent's filter and the child's,
//! an ELSE rule the negation of its elders. What KentOS's style has and a MapLibre style has not (a hatch, a gradient,
//! a pattern, a marker line, an SVG or text marker, a shape but the circle, a data-defined value, an expression the
//! filter grammar cannot say) is named, never dropped in silence; read back, what MapLibre has and KentOS has not (a
//! zoom curve, a `case`, a label, a `fill-extrusion`) is named too.
//!
//! Units: a size in paper millimetres goes out in CSS pixels at 96 dpi (1 mm = 96/25.4 px; MapLibre has no paper
//! unit); screen pixels stay pixels; metres become a width that grows with the zoom (`interpolate`, exponential base
//! 2, the metre's length at the equator in Web Mercator, 512-pixel tiles), read back as metres. Scale denominators go
//! out as zoom levels at the OGC standard rendering pixel (0.28 mm), as SLD's do (docs/adr/0516). The style is
//! written for a GeoJSON or vector tile source; its fields are the objects' attributes, KentOS's `[Ad]` MapLibre's
//! `["get", "Ad"]`.

use std::fmt::Write as _;

use serde_json::{Map, Value, json};

pub use crate::sld::Kinds;
use crate::sld::{
    Filter, Lit, Op, colour, colour_with, field_of, filter_text, num, parse_filter, round3,
};

/// CSS pixels in a millimetre at 96 dpi.
pub const PX_PER_MM: f64 = 96.0 / 25.4;
/// The metres one CSS pixel covers at zoom 0 on the equator (Web Mercator, 512-pixel tiles).
const METRES_PER_PX_Z0: f64 = 78_271.516_964_020_48;
/// The scale denominator at zoom 0 at the OGC standard rendering pixel (0.28 mm).
const SCALE_Z0: f64 = 279_541_132.014_358_9;
/// The highest zoom a metre width is interpolated to.
const TOP_ZOOM: f64 = 24.0;

/// What the writer made: the style, what it could not say, the classes and rules written and the style layers.
#[derive(Clone, Debug, PartialEq)]
pub struct Written {
    pub json: String,
    pub lost: Vec<String>,
    /// Classes and rules written (a single symbol is one).
    pub rules: usize,
    /// Style layers written.
    pub layers: usize,
    /// The ids of the layers that had no style layer to write.
    pub empty: Vec<String>,
}

/// A KentOS layer as the writer takes it.
pub struct LayerIn<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub renderer: &'a Value,
    pub kinds: Kinds,
    /// The layer's own opacity (Katman çizimi), multiplied into its symbols'.
    pub opacity: f64,
    /// The style's source this layer's objects are in.
    pub source: &'a str,
    /// The vector tile layer of the source, when it is a vector source.
    pub source_layer: Option<&'a str>,
    /// The style's `label` (one label), when the layer has it.
    pub label: Option<&'a Value>,
    /// The layer's `labels` (how it is labelled), when it has them.
    pub labels: Option<&'a Value>,
}

fn round4(v: f64) -> f64 {
    (v * 10_000.0).round() / 10_000.0
}

/// A number as JSON: an integral one without a fraction.
fn n(v: f64) -> Value {
    if v == v.trunc() && v.abs() < 1e15 {
        json!(v as i64)
    } else {
        json!(v)
    }
}

// ─── Expressions ──────────────────────────────────────────────────────────────────────────────────────

fn as_num(f: &str) -> Value {
    json!(["to-number", ["get", f]])
}

fn as_text(f: &str) -> Value {
    json!(["to-string", ["get", f]])
}

fn has(f: &str) -> Value {
    json!(["has", f])
}

fn head(v: &Value) -> Option<&str> {
    v.as_array()?.first()?.as_str()
}

/// `all` of the parts: nested `all`s spliced, a `has` said twice said once, one part alone, none no filter.
fn all_of(parts: Vec<Value>) -> Option<Value> {
    let mut flat: Vec<Value> = Vec::new();
    for p in parts {
        let inner: Vec<Value> = match (head(&p), p.as_array()) {
            (Some("all"), Some(a)) => a[1..].to_vec(),
            _ => vec![p],
        };
        for q in inner {
            if head(&q) == Some("has") && flat.contains(&q) {
                continue;
            }
            flat.push(q);
        }
    }
    match flat.len() {
        0 => None,
        1 => flat.pop(),
        _ => {
            let mut a = vec![json!("all")];
            a.extend(flat);
            Some(Value::Array(a))
        }
    }
}

fn any_of(parts: Vec<Value>) -> Value {
    if parts.len() == 1 {
        return parts.into_iter().next().unwrap_or(Value::Null);
    }
    let mut a = vec![json!("any")];
    a.extend(parts);
    Value::Array(a)
}

fn op_name(op: Op) -> &'static str {
    match op {
        Op::Eq => "==",
        Op::Ne => "!=",
        Op::Lt => "<",
        Op::Le => "<=",
        Op::Gt => ">",
        Op::Ge => ">=",
    }
}

/// A LIKE pattern as an expression over the field's text, for the shapes it has as a prefix, a suffix, a
/// containment and an exact text; others (a wildcard inside, `_`) none.
fn like_expr(field: &str, pattern: &str) -> Option<Value> {
    let (start, rest) = match pattern.strip_prefix('%') {
        Some(r) => (true, r),
        None => (false, pattern),
    };
    let (end, body) = match rest.strip_suffix('%') {
        Some(b) => (true, b),
        None => (false, rest),
    };
    if body.contains(['%', '_', '\\']) || body.is_empty() {
        return None;
    }
    let text = as_text(field);
    let len = body.chars().count();
    let test = match (start, end) {
        (false, false) => json!(["==", text, body]),
        (false, true) => json!(["==", ["slice", text, 0, len], body]),
        (true, true) => json!(["!=", ["index-of", body, text], -1]),
        (true, false) => json!(["==", ["slice", text, ["-", ["length", text], len]], body]),
    };
    Some(json!(["all", has(field), test]))
}

/// A filter as a MapLibre expression, or none when the expression language cannot say it (a LIKE pattern with a
/// wildcard inside).
fn expr(f: &Filter) -> Option<Value> {
    Some(match f {
        Filter::Cmp(op, field, Lit::Num(x)) => {
            json!(["all", has(field), [op_name(*op), as_num(field), n(*x)]])
        }
        Filter::Cmp(op, field, Lit::Text(s)) => {
            json!(["all", has(field), [op_name(*op), as_text(field), s]])
        }
        Filter::Between(field, a, b) => json!([
            "all",
            has(field),
            [">=", as_num(field), n(*a)],
            ["<=", as_num(field), n(*b)]
        ]),
        Filter::Null(field) => json!(["any", ["!", has(field)], ["==", as_text(field), ""]]),
        Filter::Like(field, p) => like_expr(field, p)?,
        Filter::And(parts) => {
            let list: Option<Vec<Value>> = parts.iter().map(expr).collect();
            all_of(list?).unwrap_or(json!(true))
        }
        Filter::Or(parts) => {
            let list: Option<Vec<Value>> = parts.iter().map(expr).collect();
            any_of(list?)
        }
        Filter::Not(inner) => match &**inner {
            Filter::Null(field) => json!(["all", has(field), ["!=", as_text(field), ""]]),
            other => json!(["!", expr(other)?]),
        },
    })
}

// ─── Colours and lengths ──────────────────────────────────────────────────────────────────────────────

/// A colour and its opacity as CSS: `#rrggbb`, or `rgba(r,g,b,a)` when it is not opaque.
fn css_rgba(rgb: &str, a: f64) -> String {
    if a >= 0.9995 {
        return rgb.to_owned();
    }
    let h = &rgb[1..];
    let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    format!(
        "rgba({},{},{},{})",
        c(0),
        c(2),
        c(4),
        num(round3(a.clamp(0.0, 1.0)))
    )
}

/// A colour as CSS: a theme colour (`fg`, `fg-dim`, `ink`, `paper`, `label`) the paper's, as a sheet prints it
/// (docs/adr/0312 §5), any other as it is.
fn css(v: &Value) -> Option<String> {
    let text = v.as_str()?;
    let (rgb, a) = colour(&json!(kentos_contracts::print_color(text)))?;
    Some(css_rgba(&rgb, a))
}

/// A size in a layer's unit as the style draws it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Len {
    Px(f64),
    Metres(f64),
}

impl Len {
    fn of(v: f64, unit: &str) -> Len {
        match unit {
            "m" => Len::Metres(v),
            "px" => Len::Px(v),
            _ => Len::Px(v * PX_PER_MM),
        }
    }

    fn scaled(self, k: f64) -> Len {
        match self {
            Len::Px(p) => Len::Px(p * k),
            Len::Metres(m) => Len::Metres(m * k),
        }
    }

    fn zoomy(self) -> bool {
        matches!(self, Len::Metres(_))
    }

    fn json(self) -> Value {
        match self {
            Len::Px(p) => n(round4(p)),
            Len::Metres(m) => {
                // Exponential base 2 between two stops grows as 2^zoom exactly: the metre's pixels at every zoom.
                let w0 = m / METRES_PER_PX_Z0;
                json!([
                    "interpolate",
                    ["exponential", 2],
                    ["zoom"],
                    0,
                    w0,
                    TOP_ZOOM as i64,
                    w0 * libm::pow(2.0, TOP_ZOOM)
                ])
            }
        }
    }
}

/// A fixed value of a symbol layer's property, or why not.
fn fixed<'a>(layer: &'a Value, key: &str, lost: &mut Vec<String>, what: &str) -> Option<&'a Value> {
    let v = layer.get(key)?;
    if v.is_object() && v.get("expr").is_some() {
        let fallback = v.get("fallback").filter(|f| !f.is_null());
        lost.push(match fallback {
            Some(_) => format!(
                "{what}: “{key}” bir ifadeyle (veriye göre); MapLibre stiline yedek değeri yazıldı"
            ),
            None => {
                format!("{what}: “{key}” bir ifadeyle (veriye göre); MapLibre stiline yazılamadı")
            }
        });
        return fallback;
    }
    (!v.is_null()).then_some(v)
}

fn zoom_of_scale(denominator: f64) -> f64 {
    libm::log2(SCALE_Z0 / denominator)
}

fn round8(v: f64) -> f64 {
    (v * 1e8).round() / 1e8
}

// ─── Symbols ──────────────────────────────────────────────────────────────────────────────────────────

/// A style layer in the making: what a symbol layer of KentOS draws, in MapLibre's properties.
#[derive(Clone, Debug, PartialEq)]
struct Sl {
    /// MapLibre's layer type: `fill`, `line`, `circle`.
    kind: &'static str,
    /// The symbol it belongs to: `fill`, `line`, `marker`.
    symbol: &'static str,
    paint: Vec<(&'static str, Value)>,
    layout: Vec<(&'static str, Value)>,
    /// A width in metres: its value is a zoom expression, which no data expression can hold.
    zoomy: bool,
}

/// The opacity of a symbol layer: its symbol's and its own.
fn opacity_of(layer: &Value, sym: f64, lost: &mut Vec<String>, what: &str) -> f64 {
    if layer
        .get("effects")
        .and_then(Value::as_array)
        .is_some_and(|e| !e.is_empty())
    {
        lost.push(format!(
            "{what}: katmanın çizim efektleri MapLibre stiline yazılmadı"
        ));
    }
    sym * layer
        .get("opacity")
        .and_then(Value::as_f64)
        .map_or(1.0, |o| o.clamp(0.0, 1.0))
}

fn fill_sl(layer: &Value, sym: f64, lost: &mut Vec<String>, what: &str) -> Option<Sl> {
    let Some(color) = fixed(layer, "color", lost, what).and_then(css) else {
        lost.push(format!("{what}: dolgunun rengi CSS renklerinden değil"));
        return None;
    };
    let opacity = opacity_of(layer, sym, lost, what);
    Some(Sl {
        kind: "fill",
        symbol: "fill",
        paint: vec![
            ("fill-color", json!(color)),
            ("fill-opacity", n(round3(opacity))),
        ],
        layout: vec![],
        zoomy: false,
    })
}

fn line_sl(
    layer: &Value,
    symbol: &'static str,
    sym: f64,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<Sl> {
    let unit = layer.get("unit").and_then(Value::as_str).unwrap_or("mm");
    let Some(color) = fixed(layer, "color", lost, what).and_then(css) else {
        lost.push(format!("{what}: çizginin rengi CSS renklerinden değil"));
        return None;
    };
    let width = fixed(layer, "width", lost, what)
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    // A hairline is one pixel, as KentOS draws it.
    let w = if width <= 0.0 {
        Len::Px(1.0)
    } else {
        Len::of(width, unit)
    };
    let cap = match layer.get("cap").and_then(Value::as_str).unwrap_or("butt") {
        "round" => "round",
        "square" => "square",
        _ => "butt",
    };
    let join = match layer.get("join").and_then(Value::as_str).unwrap_or("miter") {
        "round" => "round",
        "bevel" => "bevel",
        _ => "miter",
    };
    if layer
        .get("rings")
        .and_then(Value::as_str)
        .is_some_and(|r| r != "all")
    {
        lost.push(format!(
            "{what}: kenar çizgisinin halka seçimi MapLibre stiline yazılmadı"
        ));
    }
    for (key, words) in [
        ("offset", "paralel kaydırması"),
        ("wave", "dalgası"),
        ("blur", "yumuşak kenarı"),
        ("shift", "gölge kaydırması"),
        ("dashOffset", "kesik başlangıcı"),
    ] {
        if layer
            .get(key)
            .is_some_and(|v| !v.is_null() && v != &json!(0))
        {
            lost.push(format!(
                "{what}: çizginin {words} MapLibre stiline yazılmadı"
            ));
        }
    }
    let opacity = opacity_of(layer, sym, lost, what);
    let mut paint = vec![
        ("line-color", json!(color)),
        ("line-width", w.json()),
        ("line-opacity", n(round3(opacity))),
    ];
    // MapLibre's dashes are in widths of the line.
    let dash: Vec<f64> = layer
        .get("dash")
        .and_then(Value::as_array)
        .map(|d| d.iter().filter_map(Value::as_f64).collect())
        .unwrap_or_default();
    if !dash.is_empty() {
        let unit_width = if width > 0.0 {
            Some(width)
        } else {
            match unit {
                "px" => Some(1.0),
                "mm" => Some(1.0 / PX_PER_MM),
                _ => None,
            }
        };
        match unit_width {
            Some(uw) => {
                let ratios: Vec<Value> = dash.iter().map(|d| n(round3(d / uw))).collect();
                paint.push(("line-dasharray", Value::Array(ratios)));
            }
            None => lost.push(format!(
                "{what}: kesikler çizgi kalınlığına oranlanamadı (kalınlık sıfır, birim metre); sürekli çizgi yazıldı"
            )),
        }
    }
    Some(Sl {
        kind: "line",
        symbol,
        paint,
        layout: vec![("line-cap", json!(cap)), ("line-join", json!(join))],
        zoomy: w.zoomy(),
    })
}

fn marker_sl(layer: &Value, sym: f64, lost: &mut Vec<String>, what: &str) -> Option<Sl> {
    let t = layer.get("type").and_then(Value::as_str).unwrap_or("");
    if t != "shape" {
        lost.push(format!(
            "{what}: “{t}” işareti MapLibre stiline yazılmadı (yalnız şekil işaretleri, daire olarak yazılır)"
        ));
        return None;
    }
    let shape = layer
        .get("shape")
        .and_then(Value::as_str)
        .unwrap_or("circle");
    // A ring is a circle with no fill: the circle layer says it.
    if shape != "circle" && shape != "ring" {
        lost.push(format!(
            "{what}: “{shape}” şekli MapLibre'nin `circle` katmanında yok; daire yazıldı"
        ));
    }
    let unit = layer.get("unit").and_then(Value::as_str).unwrap_or("mm");
    let size = fixed(layer, "size", lost, what)
        .and_then(Value::as_f64)
        .unwrap_or(2.0);
    let radius = Len::of(size, unit).scaled(0.5);
    let fill = fixed(layer, "fill", lost, what).and_then(css);
    let stroke = fixed(layer, "stroke", lost, what).and_then(css);
    let stroke_width = layer
        .get("strokeWidth")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let sw = Len::of(stroke_width, unit);
    for (key, words) in [("offset", "kaydırması"), ("hole", "deliği")] {
        let moved = layer.get(key).is_some_and(|v| match v {
            Value::Array(a) => a.iter().any(|x| x.as_f64().is_some_and(|x| x != 0.0)),
            Value::Number(x) => x.as_f64().is_some_and(|x| x != 0.0),
            _ => false,
        });
        if moved {
            lost.push(format!(
                "{what}: işaretin {words} MapLibre stiline yazılmadı"
            ));
        }
    }
    if layer.get("height").is_some_and(|v| !v.is_null()) {
        lost.push(format!(
            "{what}: işaretin yüksekliği MapLibre stiline yazılmadı"
        ));
    }
    let opacity = opacity_of(layer, sym, lost, what);
    let (circle_color, circle_opacity) = match fill {
        Some(c) => (c, opacity),
        None => ("#000000".to_owned(), 0.0),
    };
    let (stroke_color, stroke_opacity, stroke_w) = match stroke {
        Some(c) => (c, opacity, sw),
        None => ("#000000".to_owned(), 0.0, Len::Px(0.0)),
    };
    Some(Sl {
        kind: "circle",
        symbol: "marker",
        paint: vec![
            ("circle-radius", radius.json()),
            ("circle-color", json!(circle_color)),
            ("circle-opacity", n(round3(circle_opacity))),
            ("circle-stroke-color", json!(stroke_color)),
            ("circle-stroke-width", stroke_w.json()),
            ("circle-stroke-opacity", n(round3(stroke_opacity))),
        ],
        layout: vec![],
        zoomy: radius.zoomy() || stroke_w.zoomy(),
    })
}

/// A symbol's style layers, `key` the symbol's class (`fill`, `line`, `marker`).
fn symbol_sls(
    sym: &Value,
    key: &'static str,
    layer_opacity: f64,
    lost: &mut Vec<String>,
    what: &str,
) -> Vec<Sl> {
    let mut out = Vec::new();
    let opacity = layer_opacity
        * sym
            .get("opacity")
            .and_then(Value::as_f64)
            .map_or(1.0, |o| o.clamp(0.0, 1.0));
    if sym
        .get("effects")
        .and_then(Value::as_array)
        .is_some_and(|e| !e.is_empty())
    {
        lost.push(format!(
            "{what}: simgenin çizim efektleri MapLibre stiline yazılmadı"
        ));
    }
    let layers = sym
        .get("layers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for layer in &layers {
        if layer.get("enabled") == Some(&Value::Bool(false)) {
            continue;
        }
        let t = layer.get("type").and_then(Value::as_str).unwrap_or("");
        let made = match (key, t) {
            ("fill", "simpleFill") => fill_sl(layer, opacity, lost, what),
            ("fill", "simpleLine") => line_sl(layer, "fill", opacity, lost, what),
            ("line", "simpleLine") => line_sl(layer, "line", opacity, lost, what),
            ("marker", _) => marker_sl(layer, opacity, lost, what),
            (_, other) => {
                let words = match other {
                    "hatchFill" => "tarama dolgusu",
                    "gradientFill" => "degrade dolgu",
                    "patternFill" => "desen dolgusu",
                    "imageFill" => "resim dolgusu",
                    "markerLine" => "işaretli çizgi",
                    "geometryGenerator" => "geometri üretici",
                    "centroidMarker" => "merkez işareti",
                    "simpleFill" => "dolgu (çizgi simgesinde)",
                    _ => "katman",
                };
                lost.push(format!("{what}: {words} MapLibre stiline yazılmadı"));
                None
            }
        };
        out.extend(made);
    }
    out
}

/// What a layer's symbols are written for: the geometry its objects have and its opacity.
#[derive(Clone, Copy)]
struct Cx {
    kinds: Kinds,
    opacity: f64,
}

/// A symbol set's style layers for the kinds asked, its library references through `library`.
fn set_sls(
    set: Option<&Value>,
    cx: Cx,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
    what: &str,
) -> Vec<Sl> {
    let Some(set) = set else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, on) in [
        ("fill", cx.kinds.area),
        ("line", cx.kinds.line),
        ("marker", cx.kinds.point),
    ] {
        if !on {
            continue;
        }
        let Some(sym) = set.get(key) else {
            continue;
        };
        let sym = match sym.get("ref").and_then(Value::as_str) {
            Some(id) => match library(id) {
                Some(s) => s,
                None => {
                    lost.push(format!(
                        "{what}: kitaplıkta olmayan “{id}” simgesi yazılmadı"
                    ));
                    continue;
                }
            },
            None => sym.clone(),
        };
        let key: &'static str = match key {
            "fill" => "fill",
            "line" => "line",
            _ => "marker",
        };
        out.extend(symbol_sls(&sym, key, cx.opacity, lost, what));
    }
    out
}

// ─── Variants and the style layers ────────────────────────────────────────────────────────────────────

/// What draws together: a class's or a rule's style layers, where it draws.
struct Variant {
    label: String,
    test: Option<Filter>,
    sls: Vec<Sl>,
    min_zoom: Option<f64>,
    max_zoom: Option<f64>,
}

struct Emit<'a> {
    layer: &'a LayerIn<'a>,
    /// More than one kind of object: each style layer says which it draws.
    mixed: bool,
    layers: Vec<Value>,
    zoomed: bool,
}

impl Emit<'_> {
    fn geometry(&self, sl: &Sl) -> Option<Value> {
        if !self.mixed {
            return None;
        }
        match (sl.symbol, sl.kind) {
            ("fill", "line") => Some(json!(["==", ["geometry-type"], "Polygon"])),
            ("line", _) => Some(json!(["==", ["geometry-type"], "LineString"])),
            ("marker", _) => Some(json!(["==", ["geometry-type"], "Point"])),
            _ => None,
        }
    }

    fn push(
        &mut self,
        sl: &Sl,
        test: Option<Value>,
        zoom: (Option<f64>, Option<f64>),
        meta: Map<String, Value>,
    ) {
        let mut paint = Map::new();
        for (k, v) in &sl.paint {
            paint.insert((*k).to_owned(), v.clone());
        }
        let mut layout = Map::new();
        for (k, v) in &sl.layout {
            layout.insert((*k).to_owned(), v.clone());
        }
        let id = format!("{}/{}", self.layer.id, self.layers.len() + 1);
        let mut l = Map::new();
        l.insert("id".into(), json!(id));
        l.insert("type".into(), json!(sl.kind));
        l.insert("source".into(), json!(self.layer.source));
        if let Some(s) = self.layer.source_layer {
            l.insert("source-layer".into(), json!(s));
        }
        if let Some(f) = all_of(self.geometry(sl).into_iter().chain(test).collect()) {
            l.insert("filter".into(), f);
        }
        if let Some(z) = zoom.0 {
            l.insert("minzoom".into(), json!(round8(z.max(0.0))));
        }
        if let Some(z) = zoom.1 {
            l.insert("maxzoom".into(), json!(round8(z.min(24.0))));
        }
        if !layout.is_empty() {
            l.insert("layout".into(), Value::Object(layout));
        }
        l.insert("paint".into(), Value::Object(paint));
        let mut metadata = meta;
        metadata.insert("kentos:layer".into(), json!(self.layer.id));
        metadata.insert("kentos:symbol".into(), json!(sl.symbol));
        l.insert("metadata".into(), Value::Object(metadata));
        self.layers.push(Value::Object(l));
    }

    /// Every variant's style layers with its own filter.
    fn each(&mut self, variants: &[Variant], lost: &mut Vec<String>) {
        for v in variants {
            let test = match &v.test {
                Some(f) => match expr(f) {
                    Some(e) => Some(e),
                    None => {
                        lost.push(format!(
                            "“{}” sınıfının süzgeci MapLibre'nin ifadesine çevrilemedi (gibi'nin içinde joker ya da _); yazılmadı",
                            v.label
                        ));
                        continue;
                    }
                },
                None => None,
            };
            if v.min_zoom.is_some() || v.max_zoom.is_some() {
                self.zoomed = true;
            }
            for sl in &v.sls {
                let mut meta = Map::new();
                meta.insert("kentos:class".into(), json!(v.label));
                self.push(sl, test.clone(), (v.min_zoom, v.max_zoom), meta);
            }
        }
    }
}

/// Whether the variants' style layers have one structure: the same kinds and symbols, layouts and dashes, and
/// nothing in metres.
fn alike(variants: &[&Variant]) -> bool {
    let Some(first) = variants.first() else {
        return false;
    };
    variants.iter().all(|v| {
        v.min_zoom.is_none()
            && v.max_zoom.is_none()
            && v.sls.len() == first.sls.len()
            && v.sls.iter().zip(&first.sls).all(|(a, b)| {
                !a.zoomy
                    && a.kind == b.kind
                    && a.symbol == b.symbol
                    && a.layout == b.layout
                    && a.paint.iter().map(|p| p.0).eq(b.paint.iter().map(|p| p.0))
                    && a.paint.iter().find(|p| p.0 == "line-dasharray")
                        == b.paint.iter().find(|p| p.0 == "line-dasharray")
            })
    })
}

/// The style layers the classes make, one per symbol layer, each paint value that differs across the classes
/// what `pick` makes of the column of the classes' values (the other class last).
fn merged(
    emit: &mut Emit<'_>,
    variants: &[&Variant],
    pick: &dyn Fn(&[Value]) -> Value,
    filter: Option<Value>,
    meta: &Map<String, Value>,
) {
    let first = variants[0];
    for p in 0..first.sls.len() {
        let mut sl = first.sls[p].clone();
        for (k, (name, _)) in first.sls[p].paint.clone().iter().enumerate() {
            let col: Vec<Value> = variants
                .iter()
                .map(|v| v.sls[p].paint[k].1.clone())
                .collect();
            let value = if col.iter().all(|c| c == &col[0]) {
                col[0].clone()
            } else {
                pick(&col)
            };
            sl.paint[k] = (name, value);
        }
        emit.push(&sl, filter.clone(), (None, None), meta.clone());
    }
}

/// A `match` over the field's text: each value its output, the last output the other's.
fn match_of(field: &str, values: &[String], col: &[Value], fallback: &Value) -> Value {
    let mut a = vec![json!("match"), as_text(field)];
    for (v, c) in values.iter().zip(col) {
        a.push(json!(v));
        a.push(c.clone());
    }
    a.push(fallback.clone());
    Value::Array(a)
}

/// A `step` over the field's number: the first class's output, then each class's output from its lower bound.
fn step_of(field: &str, lows: &[f64], col: &[Value]) -> Value {
    let mut a = vec![json!("step"), as_num(field), col[0].clone()];
    for (lo, c) in lows.iter().zip(&col[1..]) {
        a.push(n(*lo));
        a.push(c.clone());
    }
    Value::Array(a)
}

fn rules_variants(
    rules: &[Value],
    parent: Option<&Filter>,
    cx: Cx,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
    out: &mut Vec<Variant>,
) {
    // The filters of the siblings before: an ELSE rule is none of them.
    let mut elders: Vec<Filter> = Vec::new();
    let mut unconditional = false;
    for r in rules {
        if r.get("enabled") == Some(&Value::Bool(false)) {
            continue;
        }
        let label = r
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("Kural")
            .to_owned();
        let what = format!("“{label}” kuralı");
        let is_else = r.get("isElse") == Some(&Value::Bool(true));
        let own = match r
            .get("filter")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
        {
            Some(src) => match parse_filter(src) {
                Some(f) => Some(f),
                None => {
                    lost.push(format!(
                        "{what}: “{src}” ifadesi MapLibre'nin süzgecine çevrilemedi; kural ve alt kuralları yazılmadı"
                    ));
                    continue;
                }
            },
            None => None,
        };
        let own = if is_else {
            if unconditional {
                continue;
            }
            (!elders.is_empty()).then(|| Filter::Not(Box::new(Filter::Or(elders.clone()))))
        } else {
            own
        };
        match (&own, is_else) {
            (Some(f), false) => elders.push(f.clone()),
            (None, false) => unconditional = true,
            _ => {}
        }
        let test = match (parent, own) {
            (Some(p), Some(o)) => Some(Filter::And(vec![p.clone(), o])),
            (Some(p), None) => Some(p.clone()),
            (None, o) => o,
        };
        let sls = set_sls(r.get("symbols"), cx, library, lost, &what);
        let (min_zoom, max_zoom) = (
            r.get("maxScale")
                .and_then(Value::as_f64)
                .filter(|s| *s > 0.0)
                .map(zoom_of_scale),
            r.get("minScale")
                .and_then(Value::as_f64)
                .filter(|s| *s > 0.0)
                .map(zoom_of_scale),
        );
        if !sls.is_empty() {
            out.push(Variant {
                label: label.clone(),
                test: test.clone(),
                sls,
                min_zoom,
                max_zoom,
            });
        }
        if let Some(children) = r.get("children").and_then(Value::as_array) {
            rules_variants(children, test.as_ref(), cx, library, lost, out);
        }
    }
}

// ─── Labels ───────────────────────────────────────────────────────────────────────────────────────────

/// A template's `{Field}` placeholders as a `concat` of the fields' text and the text between.
fn template_expr(template: &str) -> Value {
    let mut parts: Vec<Value> = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        let Some(close) = rest[open..].find('}') else {
            break;
        };
        if open > 0 {
            parts.push(json!(&rest[..open]));
        }
        let field = rest[open + 1..open + close].trim();
        parts.push(as_text(field));
        rest = &rest[open + close + 1..];
    }
    if !rest.is_empty() {
        parts.push(json!(rest));
    }
    match parts.len() {
        0 => json!(""),
        1 => parts.remove(0),
        _ => {
            let mut a = vec![json!("concat")];
            a.extend(parts);
            Value::Array(a)
        }
    }
}

/// The label class's text as an expression, or why not.
fn label_text(style: &Value) -> Result<Value, String> {
    if let Some(text) = style
        .get("text")
        .and_then(Value::as_str)
        .filter(|t| !t.trim().is_empty())
    {
        return match field_of(text) {
            Some(f) => Ok(as_text(&f)),
            None => Err(format!(
                "“{text}” ifadesi bir alan değil; MapLibre'nin `text-field`ine çevrilemedi"
            )),
        };
    }
    match style
        .get("template")
        .and_then(Value::as_str)
        .filter(|t| !t.is_empty())
    {
        Some(t) => Ok(template_expr(t)),
        None => Err("etiketin metni yok".into()),
    }
}

fn label_font(style: &Value) -> Value {
    let bold = style
        .get("weight")
        .and_then(Value::as_u64)
        .is_some_and(|w| w >= 600);
    let italic = style.get("italic") == Some(&Value::Bool(true));
    json!([match (bold, italic) {
        (true, _) => "Noto Sans Bold",
        (false, true) => "Noto Sans Italic",
        _ => "Noto Sans Regular",
    }])
}

/// The label classes of a layer: (name, condition, style).
fn label_classes<'a>(layer: &'a LayerIn<'a>) -> Vec<(String, Option<String>, &'a Value)> {
    let mode = layer
        .labels
        .and_then(|l| l.get("mode"))
        .and_then(Value::as_str)
        .unwrap_or("single");
    match mode {
        "off" => Vec::new(),
        "rules" => layer
            .labels
            .and_then(|l| l.get("classes"))
            .and_then(Value::as_array)
            .map(|cs| {
                cs.iter()
                    .filter_map(|c| {
                        Some((
                            c.get("name")?.as_str()?.to_owned(),
                            c.get("when").and_then(Value::as_str).map(str::to_owned),
                            c.get("style")?,
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        _ => layer
            .label
            .map(|s| vec![("Etiket".to_owned(), None, s)])
            .unwrap_or_default(),
    }
}

/// The settings of a label class MapLibre's symbol layer cannot hold.
const LABEL_UNMAPPED: [(&str, &str); 14] = [
    ("callout", "çağrı çizgisi"),
    ("background", "zemin"),
    ("shadow", "gölge"),
    ("stack", "yığma"),
    ("abbreviate", "kısaltma"),
    ("shrink", "küçültme"),
    ("duplicates", "yinelenenleri tek yazma"),
    ("mergeLines", "çizgileri birleştirme"),
    ("minScale", "ölçek alt sınırı"),
    ("maxScale", "ölçek üst sınırı"),
    ("minFeaturePx", "en küçük nesne boyu"),
    ("maxSize", "en büyük boy"),
    ("grow", "nesneye göre büyüme"),
    ("outside", "dışarıda çağrı çizgisiyle"),
];

fn label_layers(emit: &mut Emit<'_>, lost: &mut Vec<String>) {
    let layer = emit.layer;
    for (name, when, style) in label_classes(layer) {
        let what = format!("“{name}” etiketi");
        let text = match label_text(style) {
            Ok(t) => t,
            Err(why) => {
                lost.push(format!("{what}: {why}; etiket yazılmadı"));
                continue;
            }
        };
        let test = match when.as_deref().filter(|w| !w.trim().is_empty()) {
            Some(src) => match parse_filter(src).as_ref().and_then(expr) {
                Some(e) => Some(e),
                None => {
                    lost.push(format!(
                        "{what}: “{src}” koşulu MapLibre'nin ifadesine çevrilemedi; etiket yazılmadı"
                    ));
                    continue;
                }
            },
            None => None,
        };
        let unmapped: Vec<&str> = LABEL_UNMAPPED
            .iter()
            .filter(|(k, _)| style.get(*k).is_some_and(|v| !v.is_null()))
            .map(|(_, w)| *w)
            .collect();
        if !unmapped.is_empty() {
            lost.push(format!(
                "{what}: {} MapLibre'nin yazı katmanında yok; yazılmadı",
                unmapped.join(", ")
            ));
        }
        let size = style.get("size").and_then(Value::as_f64).unwrap_or(12.0);
        let color = style
            .get("color")
            .and_then(css_label)
            .unwrap_or_else(|| "#222222".to_owned());
        let halo = style.get("halo");
        let halo_w = halo
            .and_then(|h| h.get("width"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let halo_c = halo
            .and_then(|h| h.get("color"))
            .and_then(css_label)
            .unwrap_or_else(|| "#ffffff".to_owned());
        let kinds = layer.kinds;
        let mut per: Vec<(&'static str, bool)> = Vec::new();
        if kinds.area {
            per.push(("fill", true));
        }
        if kinds.line {
            per.push(("line", true));
        }
        if kinds.point {
            per.push(("marker", true));
        }
        for (symbol, _) in per {
            let mut layout = Map::new();
            layout.insert("text-field".into(), text.clone());
            layout.insert("text-font".into(), label_font(style));
            layout.insert("text-size".into(), n(round4(size)));
            if let Some(a) = style.get("align").and_then(Value::as_str) {
                layout.insert("text-justify".into(), json!(a));
            }
            match symbol {
                "line" => {
                    layout.insert("symbol-placement".into(), json!("line"));
                    if let Some(r) = style.get("repeat").and_then(Value::as_f64) {
                        layout.insert("symbol-spacing".into(), n(round4(r)));
                    }
                    let d = style.get("distance").and_then(Value::as_f64).unwrap_or(2.0);
                    match style.get("position").and_then(Value::as_str) {
                        Some("above") => {
                            layout.insert("text-offset".into(), json!([0, -round3(d / size)]));
                        }
                        Some("below") => {
                            layout.insert("text-offset".into(), json!([0, round3(d / size)]));
                        }
                        _ => {}
                    }
                    if let Some(m) = style.get("line").and_then(Value::as_str)
                        && m == "horizontal"
                    {
                        lost.push(format!(
                            "{what}: yatay çizgi etiketi MapLibre'de çizgiyi izler; çizgi boyunca yazıldı"
                        ));
                    }
                }
                "fill" => {
                    if matches!(
                        style.get("area").and_then(Value::as_str),
                        Some("perimeter" | "boundary")
                    ) {
                        lost.push(format!(
                            "{what}: alanın çevresi boyunca etiket MapLibre'de yok; alanın içine yazıldı"
                        ));
                    }
                }
                _ => {
                    let d = style.get("distance").and_then(Value::as_f64).unwrap_or(2.0);
                    if style.get("point").and_then(Value::as_str) == Some("center") {
                        layout.insert("text-anchor".into(), json!("center"));
                    } else {
                        layout.insert(
                            "text-variable-anchor".into(),
                            json!(["bottom-left", "top-left", "bottom-right", "top-right"]),
                        );
                        layout.insert("text-radial-offset".into(), n(round3(d / size)));
                    }
                }
            }
            if let Some(p) = style.get("priority").and_then(Value::as_u64) {
                layout.insert("symbol-sort-key".into(), json!(10 - p.min(10) as i64));
            }
            if style.get("overlap").and_then(Value::as_str) == Some("always") {
                layout.insert("text-allow-overlap".into(), json!(true));
            }
            let mut paint = Map::new();
            paint.insert("text-color".into(), json!(color));
            if halo_w > 0.0 {
                paint.insert("text-halo-color".into(), json!(halo_c));
                paint.insert("text-halo-width".into(), n(round4(halo_w)));
            }
            let geometry = if emit.mixed {
                Some(json!([
                    "==",
                    ["geometry-type"],
                    match symbol {
                        "fill" => "Polygon",
                        "line" => "LineString",
                        _ => "Point",
                    }
                ]))
            } else {
                None
            };
            let id = format!("{}/{}", layer.id, emit.layers.len() + 1);
            let mut l = Map::new();
            l.insert("id".into(), json!(id));
            l.insert("type".into(), json!("symbol"));
            l.insert("source".into(), json!(layer.source));
            if let Some(s) = layer.source_layer {
                l.insert("source-layer".into(), json!(s));
            }
            if let Some(f) = all_of(geometry.into_iter().chain(test.clone()).collect()) {
                l.insert("filter".into(), f);
            }
            l.insert("layout".into(), Value::Object(layout));
            l.insert("paint".into(), Value::Object(paint));
            l.insert(
                "metadata".into(),
                json!({ "kentos:layer": layer.id, "kentos:label": name }),
            );
            emit.layers.push(Value::Object(l));
        }
    }
}

/// A label's colour (hex; a theme name has no CSS colour) as CSS.
fn css_label(v: &Value) -> Option<String> {
    css(v)
}

// ─── Writing ──────────────────────────────────────────────────────────────────────────────────────────

/// The classes of a layer's renderer written into `emit`; how many classes and rules.
fn write_renderer(
    emit: &mut Emit<'_>,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
) -> usize {
    let renderer = emit.layer.renderer;
    let cx = Cx {
        kinds: emit.layer.kinds,
        opacity: emit.layer.opacity,
    };
    let kind = renderer.get("type").and_then(Value::as_str).unwrap_or("");
    match kind {
        "single" => {
            let sls = set_sls(renderer.get("symbols"), cx, library, lost, "Simge");
            if sls.is_empty() {
                return 0;
            }
            let v = Variant {
                label: emit.layer.name.to_owned(),
                test: None,
                sls,
                min_zoom: None,
                max_zoom: None,
            };
            emit.each(&[v], lost);
            1
        }
        "categorized" => write_categorized(emit, library, lost),
        "graduated" => write_graduated(emit, library, lost),
        "rules" => {
            let list = renderer
                .get("rules")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut variants = Vec::new();
            rules_variants(&list, None, cx, library, lost, &mut variants);
            let count = variants.len();
            emit.each(&variants, lost);
            count
        }
        other => {
            lost.push(format!(
                "“{other}” görünüşü MapLibre stiline yazılmadı (yalnız tek simge, kategorili, aralıklı ve kurallı görünüş yazılır)"
            ));
            0
        }
    }
}

fn write_categorized(
    emit: &mut Emit<'_>,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
) -> usize {
    let renderer = emit.layer.renderer;
    let cx = Cx {
        kinds: emit.layer.kinds,
        opacity: emit.layer.opacity,
    };
    let expr_text = renderer.get("expr").and_then(Value::as_str).unwrap_or("");
    let Some(field) = field_of(expr_text) else {
        lost.push(format!(
            "Kategorili görünüşün “{expr_text}” ifadesi bir alan değil; MapLibre'nin ifadesine çevrilemedi, sınıflar yazılmadı"
        ));
        return 0;
    };
    let mut values: Vec<String> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    let mut variants: Vec<Variant> = Vec::new();
    let mut disabled: Vec<String> = Vec::new();
    for c in renderer
        .get("categories")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
    {
        let value = c
            .get("value")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        let label = c
            .get("label")
            .and_then(Value::as_str)
            .filter(|l| !l.is_empty())
            .unwrap_or(&value)
            .to_owned();
        if c.get("enabled") == Some(&Value::Bool(false)) {
            disabled.push(value);
            continue;
        }
        let sls = set_sls(
            c.get("symbols"),
            cx,
            library,
            lost,
            &format!("“{label}” sınıfı"),
        );
        if sls.is_empty() {
            continue;
        }
        values.push(value.clone());
        labels.push(label.clone());
        variants.push(Variant {
            label,
            test: Some(Filter::Cmp(Op::Eq, field.clone(), Lit::Text(value))),
            sls,
            min_zoom: None,
            max_zoom: None,
        });
    }
    let other = renderer
        .get("other")
        .map(|o| set_sls(Some(o), cx, library, lost, "“Diğerleri” sınıfı"));
    let other = other.filter(|s| !s.is_empty());
    let count = variants.len() + usize::from(other.is_some());
    if count == 0 {
        return 0;
    }
    let all_values: Vec<String> = values.iter().chain(&disabled).cloned().collect();
    let unique = {
        let mut s = all_values.clone();
        s.sort();
        s.dedup();
        s.len() == all_values.len()
    };
    let other_variant = other.map(|sls| Variant {
        label: "Diğerleri".into(),
        test: None,
        sls,
        min_zoom: None,
        max_zoom: None,
    });
    let mut group: Vec<&Variant> = variants.iter().collect();
    if let Some(o) = &other_variant {
        group.push(o);
    }
    if disabled.is_empty() && unique && !variants.is_empty() && alike(&group) {
        let membership = other_variant
            .is_none()
            .then(|| json!(["match", as_text(&field), json!(values), true, false]));
        let mut meta = Map::new();
        let custom: Map<String, Value> = values
            .iter()
            .zip(&labels)
            .filter(|(v, l)| v != l)
            .map(|(v, l)| (v.clone(), json!(l)))
            .collect();
        if !custom.is_empty() {
            meta.insert("kentos:labels".into(), Value::Object(custom));
        }
        let field_c = field.clone();
        let values_c = values.clone();
        let has_other = other_variant.is_some();
        let pick = move |col: &[Value]| {
            let fallback = col[col.len() - 1].clone();
            let n_values = values_c.len();
            let cats = &col[..n_values];
            let fb = if has_other {
                fallback
            } else {
                cats[cats.len() - 1].clone()
            };
            match_of(&field_c, &values_c, cats, &fb)
        };
        merged(emit, &group, &pick, membership, &meta);
        return count;
    }
    // One style layer per class, each with its own filter; the other none of the values.
    let mut all = variants;
    if let Some(mut o) = other_variant {
        let any: Vec<Filter> = all_values
            .iter()
            .map(|v| Filter::Cmp(Op::Eq, field.clone(), Lit::Text(v.clone())))
            .collect();
        o.test = (!any.is_empty()).then(|| Filter::Not(Box::new(Filter::Or(any))));
        all.push(o);
    }
    emit.each(&all, lost);
    count
}

fn write_graduated(
    emit: &mut Emit<'_>,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
) -> usize {
    let renderer = emit.layer.renderer;
    let cx = Cx {
        kinds: emit.layer.kinds,
        opacity: emit.layer.opacity,
    };
    let expr_text = renderer.get("expr").and_then(Value::as_str).unwrap_or("");
    let Some(field) = field_of(expr_text) else {
        lost.push(format!(
            "Aralıklı görünüşün “{expr_text}” ifadesi bir alan değil; MapLibre'nin ifadesine çevrilemedi, sınıflar yazılmadı"
        ));
        return 0;
    };
    let classes = renderer
        .get("classes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let total = classes.len();
    let mut variants: Vec<Variant> = Vec::new();
    let mut bounds: Vec<(f64, f64)> = Vec::new();
    let mut labels: Vec<Value> = Vec::new();
    for (k, c) in classes.iter().enumerate() {
        let (Some(lo), Some(hi)) = (
            c.get("min").and_then(Value::as_f64),
            c.get("max").and_then(Value::as_f64),
        ) else {
            continue;
        };
        let default_label = format!("{} - {}", num(lo), num(hi));
        let label = c
            .get("label")
            .and_then(Value::as_str)
            .map_or_else(|| default_label.clone(), str::to_owned);
        let sls = set_sls(
            c.get("symbols"),
            cx,
            library,
            lost,
            &format!("“{label}” sınıfı"),
        );
        if sls.is_empty() {
            continue;
        }
        // min ≤ value < max; the last class takes its max too.
        let top = if k + 1 == total { Op::Le } else { Op::Lt };
        bounds.push((lo, hi));
        labels.push(json!(label));
        variants.push(Variant {
            label,
            test: Some(Filter::And(vec![
                Filter::Cmp(Op::Ge, field.clone(), Lit::Num(lo)),
                Filter::Cmp(top, field.clone(), Lit::Num(hi)),
            ])),
            sls,
            min_zoom: None,
            max_zoom: None,
        });
    }
    let count = variants.len();
    if count == 0 {
        return 0;
    }
    let contiguous = bounds.windows(2).all(|w| w[0].1 == w[1].0)
        && bounds.iter().all(|b| b.0 < b.1)
        && count == total;
    let group: Vec<&Variant> = variants.iter().collect();
    if contiguous && alike(&group) {
        let (first, last) = (bounds[0].0, bounds[count - 1].1);
        let filter = all_of(vec![
            has(&field),
            json!([">=", as_num(&field), n(first)]),
            json!(["<=", as_num(&field), n(last)]),
        ]);
        let lows: Vec<f64> = bounds.iter().skip(1).map(|b| b.0).collect();
        let default_labels: Vec<Value> = bounds
            .iter()
            .map(|b| json!(format!("{} - {}", num(b.0), num(b.1))))
            .collect();
        let mut meta = Map::new();
        if labels != default_labels {
            meta.insert("kentos:labels".into(), Value::Array(labels));
        }
        let field_c = field.clone();
        let pick = move |col: &[Value]| step_of(&field_c, &lows, col);
        merged(emit, &group, &pick, filter, &meta);
        return count;
    }
    emit.each(&variants, lost);
    count
}

/// The style of `layers` (bottom first), over `sources` (id → source), `library` giving the project library's
/// symbols by id; `glyphs` the glyph server's URL for the labels' fonts.
pub fn write(
    name: &str,
    layers: &[LayerIn<'_>],
    sources: &Map<String, Value>,
    glyphs: Option<&str>,
    library: &dyn Fn(&str) -> Option<Value>,
) -> Written {
    let mut lost = Vec::new();
    let mut out: Vec<Value> = Vec::new();
    let mut rules = 0usize;
    let mut zoomed = false;
    let mut texts = false;
    let mut empty: Vec<String> = Vec::new();
    for layer in layers {
        let k = layer.kinds;
        let mixed = usize::from(k.point) + usize::from(k.line) + usize::from(k.area) > 1;
        let mut emit = Emit {
            layer,
            mixed,
            layers: Vec::new(),
            zoomed: false,
        };
        let mut layer_lost = Vec::new();
        rules += write_renderer(&mut emit, library, &mut layer_lost);
        let before = emit.layers.len();
        label_layers(&mut emit, &mut layer_lost);
        texts |= emit.layers.len() > before;
        zoomed |= emit.zoomed;
        if emit.layers.is_empty() {
            empty.push(layer.id.to_owned());
        }
        let many = layers.len() > 1;
        lost.extend(layer_lost.into_iter().map(|l| {
            if many {
                format!("“{}” katmanı: {l}", layer.name)
            } else {
                l
            }
        }));
        out.extend(emit.layers);
    }
    if zoomed {
        lost.push(
            "ölçek aralıkları yakınlaştırma düzeyine yaklaşık çevrildi (OGC'nin 0,28 mm pikseli, ekvator, 512 pikselli karo)"
                .into(),
        );
    }
    if out
        .iter()
        .any(|l| uses_metres(l.get("paint").unwrap_or(&Value::Null)))
    {
        lost.push(
            "metre cinsinden boylar yakınlaştırmayla büyüyen piksel boylarına çevrildi (ekvatordaki ölçüyle; enlem arttıkça MapLibre'nin Mercator ölçeği metreden farklı olur)"
                .into(),
        );
    }
    let mut style = Map::new();
    style.insert("version".into(), json!(8));
    style.insert("name".into(), json!(name));
    style.insert(
        "metadata".into(),
        json!({ "kentos:format": "kentos-maplibre/1" }),
    );
    style.insert("sources".into(), Value::Object(sources.clone()));
    if texts && let Some(g) = glyphs {
        style.insert("glyphs".into(), json!(g));
    }
    let layers_written = out.len();
    style.insert("layers".into(), Value::Array(out));
    let mut text = String::new();
    let _ = writeln!(
        text,
        "{}",
        serde_json::to_string_pretty(&Value::Object(style)).unwrap_or_default()
    );
    Written {
        json: text,
        lost,
        rules,
        layers: layers_written,
        empty,
    }
}

fn uses_metres(paint: &Value) -> bool {
    paint.as_object().is_some_and(|p| {
        p.values().any(|v| {
            head(v) == Some("interpolate") && v.get(1).and_then(head) == Some("exponential")
        })
    })
}

// ─── Reading ──────────────────────────────────────────────────────────────────────────────────────────

pub use crate::sld::Read;

/// A layer of a style as the reader holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct StyleLayer {
    pub id: String,
    /// MapLibre's layer type.
    pub kind: String,
    pub source: Option<String>,
    pub source_layer: Option<String>,
    pub filter: Option<Value>,
    pub paint: Map<String, Value>,
    pub layout: Map<String, Value>,
    pub min_zoom: Option<f64>,
    pub max_zoom: Option<f64>,
    pub metadata: Map<String, Value>,
}

impl StyleLayer {
    /// The KentOS layer a KentOS style wrote this style layer for.
    pub fn kentos_layer(&self) -> Option<&str> {
        self.metadata.get("kentos:layer").and_then(Value::as_str)
    }
}

/// A style as the reader holds it.
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub name: Option<String>,
    pub layers: Vec<StyleLayer>,
}

fn object(v: &Value, key: &str) -> Map<String, Value> {
    v.get(key)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

/// A style's layers, or why the text is not a MapLibre style.
pub fn parse(text: &str) -> Result<Style, String> {
    let root: Value =
        serde_json::from_str(text).map_err(|_| "Dosya okunabilir bir JSON değil.".to_owned())?;
    let Some(obj) = root.as_object() else {
        return Err("Dosya bir MapLibre stili değil: kökü bir nesne değil.".into());
    };
    if let Some(v) = obj.get("version")
        && v != &json!(8)
    {
        return Err(format!(
            "Dosya bir MapLibre stili değil ya da sürümü okunamıyor: version {v}, yalnız 8 okunur."
        ));
    }
    let Some(list) = obj.get("layers").and_then(Value::as_array) else {
        return Err("Stilde katman listesi (layers) yok.".into());
    };
    let mut layers = Vec::new();
    for l in list {
        let (Some(id), Some(kind)) = (
            l.get("id").and_then(Value::as_str),
            l.get("type").and_then(Value::as_str),
        ) else {
            continue;
        };
        layers.push(StyleLayer {
            id: id.to_owned(),
            kind: kind.to_owned(),
            source: l.get("source").and_then(Value::as_str).map(str::to_owned),
            source_layer: l
                .get("source-layer")
                .and_then(Value::as_str)
                .map(str::to_owned),
            filter: l.get("filter").cloned(),
            paint: object(l, "paint"),
            layout: object(l, "layout"),
            min_zoom: l.get("minzoom").and_then(Value::as_f64),
            max_zoom: l.get("maxzoom").and_then(Value::as_f64),
            metadata: object(l, "metadata"),
        });
    }
    Ok(Style {
        name: obj.get("name").and_then(Value::as_str).map(str::to_owned),
        layers,
    })
}

// ── Expressions back into filters ─────────────────────────────────────────────

fn lit_of(v: &Value) -> Option<Lit> {
    match v {
        Value::Number(x) => x.as_f64().map(Lit::Num),
        Value::String(s) => Some(Lit::Text(s.clone())),
        Value::Bool(b) => Some(Lit::Text(b.to_string())),
        _ => None,
    }
}

/// The field an operand reads: `["get", f]`, alone or under a conversion.
fn operand_field(v: &Value) -> Option<String> {
    let a = v.as_array()?;
    match a.first()?.as_str()? {
        "get" if a.len() == 2 => a[1].as_str().map(str::to_owned),
        "to-number" | "to-string" | "number" | "string" if a.len() == 2 => operand_field(&a[1]),
        _ => None,
    }
}

fn flip(op: Op) -> Op {
    match op {
        Op::Lt => Op::Gt,
        Op::Le => Op::Ge,
        Op::Gt => Op::Lt,
        Op::Ge => Op::Le,
        other => other,
    }
}

fn op_of(name: &str) -> Option<Op> {
    Some(match name {
        "==" => Op::Eq,
        "!=" => Op::Ne,
        "<" => Op::Lt,
        "<=" => Op::Le,
        ">" => Op::Gt,
        ">=" => Op::Ge,
        _ => return None,
    })
}

/// The labels of a `match`/`in` as texts.
fn labels_of(v: &Value) -> Option<Vec<Lit>> {
    match v {
        // `["literal", [..]]` holds the labels; a plain array is the labels themselves.
        Value::Array(a) if a.len() == 2 && a[0] == json!("literal") => labels_of(&a[1]),
        Value::Array(a) => a.iter().map(lit_of).collect(),
        other => lit_of(other).map(|l| vec![l]),
    }
}

fn or_of_eq(field: &str, labels: Vec<Lit>) -> Filter {
    let mut parts: Vec<Filter> = labels
        .into_iter()
        .map(|l| Filter::Cmp(Op::Eq, field.to_owned(), l))
        .collect();
    if parts.len() == 1 {
        parts.remove(0)
    } else {
        Filter::Or(parts)
    }
}

/// The geometry types a filter part says (`Point`, `LineString`, `Polygon`), when it is one.
fn geometry_of(v: &Value) -> Option<Vec<String>> {
    let a = v.as_array()?;
    let kinds = |l: Vec<Lit>| -> Option<Vec<String>> {
        let mut out: Vec<String> = Vec::new();
        for l in l {
            let Lit::Text(t) = l else { return None };
            let t = t.trim_start_matches("Multi").to_owned();
            if !out.contains(&t) {
                out.push(t);
            }
        }
        Some(out)
    };
    let is_geom = |x: &Value| x == &json!(["geometry-type"]) || x == &json!("$type");
    match a.first()?.as_str()? {
        "==" if a.len() == 3 => {
            if is_geom(&a[1]) {
                kinds(vec![lit_of(&a[2])?])
            } else if is_geom(&a[2]) {
                kinds(vec![lit_of(&a[1])?])
            } else {
                None
            }
        }
        "match"
            if a.len() == 5 && is_geom(&a[1]) && a[3] == json!(true) && a[4] == json!(false) =>
        {
            kinds(labels_of(&a[2])?)
        }
        "in" if a.len() == 3 && is_geom(&a[1]) => kinds(labels_of(&a[2])?),
        "in" if a.len() > 2 && a[1] == json!("$type") => {
            kinds(a[2..].iter().map(lit_of).collect::<Option<Vec<_>>>()?)
        }
        _ => None,
    }
}

/// A MapLibre filter expression (the expression form or the legacy one) as a filter.
fn to_filter(v: &Value) -> Option<Filter> {
    let a = v.as_array()?;
    let name = a.first()?.as_str()?;
    if let Some(op) = op_of(name)
        && a.len() == 3
    {
        // The legacy form names the key by a bare string.
        let key = |x: &Value| {
            x.as_str()
                .filter(|k| !k.starts_with('$'))
                .map(str::to_owned)
        };
        let (field, lit_v, op) = if let Some(f) = operand_field(&a[1]) {
            (f, &a[2], op)
        } else if let Some(f) = operand_field(&a[2]) {
            (f, &a[1], flip(op))
        } else if let Some(f) = key(&a[1]) {
            (f, &a[2], op)
        } else {
            return like_of(v);
        };
        if lit_v.is_null() {
            return match op {
                Op::Eq => Some(Filter::Null(field)),
                Op::Ne => Some(Filter::Not(Box::new(Filter::Null(field)))),
                _ => None,
            };
        }
        return match lit_of(lit_v)? {
            Lit::Text(t) if t.is_empty() && op == Op::Eq => Some(Filter::Null(field)),
            Lit::Text(t) if t.is_empty() && op == Op::Ne => {
                Some(Filter::Not(Box::new(Filter::Null(field))))
            }
            l => Some(Filter::Cmp(op, field, l)),
        };
    }
    match name {
        "all" => {
            let mut parts: Vec<Filter> = Vec::new();
            for p in &a[1..] {
                match to_filter(p)? {
                    Filter::And(inner) => parts.extend(inner),
                    other => parts.push(other),
                }
            }
            Some(tidy(parts))
        }
        "any" => {
            // The pattern the writer gives a null: not there, or an empty text.
            if a.len() == 3
                && let Some(Filter::Null(f)) = null_pattern(&a[1], &a[2])
            {
                return Some(Filter::Null(f));
            }
            let parts: Option<Vec<Filter>> = a[1..].iter().map(to_filter).collect();
            let parts = parts?;
            Some(match parts.len() {
                1 => parts.into_iter().next()?,
                _ => Filter::Or(parts),
            })
        }
        "!" if a.len() == 2 => {
            if let Some(f) = has_of(&a[1]) {
                return Some(Filter::Null(f));
            }
            Some(Filter::Not(Box::new(to_filter(&a[1])?)))
        }
        "has" if a.len() == 2 => a[1]
            .as_str()
            .map(|f| Filter::Not(Box::new(Filter::Null(f.to_owned())))),
        "!has" if a.len() == 2 => a[1].as_str().map(|f| Filter::Null(f.to_owned())),
        "in" | "!in" => {
            let (field, labels) = if a.len() == 3 && operand_field(&a[1]).is_some() {
                (operand_field(&a[1])?, labels_of(&a[2])?)
            } else if a.len() >= 2 && a[1].as_str().is_some_and(|k| !k.starts_with('$')) {
                (
                    a[1].as_str()?.to_owned(),
                    a[2..].iter().map(lit_of).collect::<Option<Vec<_>>>()?,
                )
            } else {
                return None;
            };
            let f = or_of_eq(&field, labels);
            Some(if name == "!in" {
                Filter::Not(Box::new(f))
            } else {
                f
            })
        }
        "match" if a.len() == 5 => {
            let field = operand_field(&a[1])?;
            let f = or_of_eq(&field, labels_of(&a[2])?);
            match (&a[3], &a[4]) {
                (Value::Bool(true), Value::Bool(false)) => Some(f),
                (Value::Bool(false), Value::Bool(true)) => Some(Filter::Not(Box::new(f))),
                _ => None,
            }
        }
        _ => None,
    }
}

/// `["!", ["has", f]]`'s field.
fn has_of(v: &Value) -> Option<String> {
    let a = v.as_array()?;
    (a.len() == 2 && a[0] == json!("has")).then(|| a[1].as_str().map(str::to_owned))?
}

fn null_pattern(not_there: &Value, empty: &Value) -> Option<Filter> {
    let a = not_there.as_array()?;
    if a.len() != 2 || a[0] != json!("!") {
        return None;
    }
    let f = has_of(&a[1])?;
    let e = empty.as_array()?;
    (e.len() == 3
        && e[0] == json!("==")
        && operand_field(&e[1]).as_deref() == Some(&f)
        && e[2] == json!(""))
    .then_some(Filter::Null(f))
}

/// The LIKE patterns the writer gives a prefix, a suffix and a containment.
fn like_of(v: &Value) -> Option<Filter> {
    let a = v.as_array()?;
    if a.len() != 3 {
        return None;
    }
    let (op, x, y) = (a[0].as_str()?, &a[1], &a[2]);
    let slice = x.as_array()?;
    match (op, slice.first().and_then(Value::as_str)) {
        ("==", Some("slice")) => {
            let field = operand_field(slice.get(1)?)?;
            let body = y.as_str()?;
            let len = body.chars().count();
            if slice.len() == 4 && slice[2] == json!(0) && slice[3].as_u64()? as usize == len {
                return Some(Filter::Like(field, format!("{body}%")));
            }
            if slice.len() == 3 {
                let from = slice[2].as_array()?;
                if from.len() == 3
                    && from[0] == json!("-")
                    && from[2].as_u64()? as usize == len
                    && from[1].as_array()?.first().and_then(Value::as_str) == Some("length")
                {
                    return Some(Filter::Like(field, format!("%{body}")));
                }
            }
            None
        }
        ("!=", Some("index-of")) if y == &json!(-1) => {
            let body = slice.get(1)?.as_str()?;
            let field = operand_field(slice.get(2)?)?;
            Some(Filter::Like(field, format!("%{body}%")))
        }
        _ => None,
    }
}

fn mentions(f: &Filter, field: &str) -> bool {
    match f {
        Filter::Cmp(_, n, _) | Filter::Between(n, _, _) | Filter::Null(n) | Filter::Like(n, _) => {
            n == field
        }
        Filter::And(p) | Filter::Or(p) => p.iter().any(|x| mentions(x, field)),
        Filter::Not(i) => mentions(i, field),
    }
}

/// An `all`'s parts tidied: a `has` guard whose field another part reads dropped, the same part said twice said
/// once, a pair of bounds a Between.
fn tidy(parts: Vec<Filter>) -> Filter {
    let mut kept: Vec<Filter> = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        if let Filter::Not(inner) = p
            && let Filter::Null(field) = &**inner
            && parts.iter().enumerate().any(|(j, q)| {
                j != i
                    && mentions(q, field)
                    && !matches!(q, Filter::Not(x) if matches!(&**x, Filter::Null(_)))
            })
        {
            continue;
        }
        if !kept.contains(p) {
            kept.push(p.clone());
        }
    }
    if kept.len() == 2
        && let (Filter::Cmp(Op::Ge, f1, Lit::Num(a)), Filter::Cmp(Op::Le, f2, Lit::Num(b))) =
            (&kept[0], &kept[1])
        && f1 == f2
    {
        return Filter::Between(f1.clone(), *a, *b);
    }
    match kept.len() {
        1 => kept.remove(0),
        _ => Filter::And(kept),
    }
}

/// A layer's filter: what it says of the objects, and of their geometry.
struct FilterIn {
    filter: Option<Filter>,
    geometry: Option<Vec<String>>,
}

fn filter_in(v: Option<&Value>) -> Result<FilterIn, String> {
    let Some(v) = v else {
        return Ok(FilterIn {
            filter: None,
            geometry: None,
        });
    };
    if v == &json!(true) {
        return Ok(FilterIn {
            filter: None,
            geometry: None,
        });
    }
    let parts: Vec<&Value> = match v.as_array() {
        Some(a) if a.first() == Some(&json!("all")) => a[1..].iter().collect(),
        _ => vec![v],
    };
    let mut geometry: Option<Vec<String>> = None;
    let mut rest: Vec<Value> = Vec::new();
    for p in parts {
        match geometry_of(p) {
            Some(g) => geometry = Some(g),
            None => rest.push(p.clone()),
        }
    }
    let filter = match rest.len() {
        0 => None,
        _ => {
            let mut all = vec![json!("all")];
            all.extend(rest);
            Some(to_filter(&Value::Array(all)).ok_or_else(|| {
                "süzgeci KentOS'un ifadesine çevrilemedi (yalnız alanın değerinin karşılaştırmaları, boşluk, in, match ve all, any, ! okunur)"
                    .to_owned()
            })?)
        }
    };
    Ok(FilterIn { filter, geometry })
}

// ── Values ────────────────────────────────────────────────────────────────────

/// A CSS colour as a colour and its opacity.
fn css_in(text: &str) -> Option<(String, f64)> {
    let t = text.trim().to_ascii_lowercase();
    if t.starts_with('#') {
        return colour(&json!(t));
    }
    let named = |rgb: &str, a: f64| Some((rgb.to_owned(), a));
    match t.as_str() {
        "black" => return named("#000000", 1.0),
        "white" => return named("#ffffff", 1.0),
        "red" => return named("#ff0000", 1.0),
        "green" => return named("#008000", 1.0),
        "lime" => return named("#00ff00", 1.0),
        "blue" => return named("#0000ff", 1.0),
        "yellow" => return named("#ffff00", 1.0),
        "cyan" | "aqua" => return named("#00ffff", 1.0),
        "magenta" | "fuchsia" => return named("#ff00ff", 1.0),
        "gray" | "grey" => return named("#808080", 1.0),
        "orange" => return named("#ffa500", 1.0),
        "purple" => return named("#800080", 1.0),
        "pink" => return named("#ffc0cb", 1.0),
        "brown" => return named("#a52a2a", 1.0),
        "transparent" => return named("#000000", 0.0),
        _ => {}
    }
    let (func, args) = t.split_once('(')?;
    let args = args.strip_suffix(')')?;
    let nums: Vec<&str> = args
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .collect();
    let part = |s: &str, full: f64| -> Option<f64> {
        match s.strip_suffix('%') {
            Some(p) => p.parse::<f64>().ok().map(|p| p / 100.0 * full),
            None => s.trim_end_matches("deg").parse::<f64>().ok(),
        }
    };
    let alpha = |i: usize| -> Option<f64> {
        match nums.get(i) {
            Some(s) => part(s, 1.0).map(|a| a.clamp(0.0, 1.0)),
            None => Some(1.0),
        }
    };
    match func {
        "rgb" | "rgba" if nums.len() >= 3 => {
            let c = |i: usize| part(nums[i], 255.0).map(|x| x.round().clamp(0.0, 255.0) as u8);
            Some((
                format!("#{:02x}{:02x}{:02x}", c(0)?, c(1)?, c(2)?),
                alpha(3)?,
            ))
        }
        "hsl" | "hsla" if nums.len() >= 3 => {
            let h = part(nums[0], 360.0)?.rem_euclid(360.0) / 360.0;
            let s = part(nums[1], 1.0)?.clamp(0.0, 1.0);
            let l = part(nums[2], 1.0)?.clamp(0.0, 1.0);
            let q = if l < 0.5 {
                l * (1.0 + s)
            } else {
                l + s - l * s
            };
            let p = 2.0 * l - q;
            let channel = |mut t: f64| {
                if t < 0.0 {
                    t += 1.0;
                }
                if t > 1.0 {
                    t -= 1.0;
                }
                let v = if t < 1.0 / 6.0 {
                    p + (q - p) * 6.0 * t
                } else if t < 0.5 {
                    q
                } else if t < 2.0 / 3.0 {
                    p + (q - p) * (2.0 / 3.0 - t) * 6.0
                } else {
                    p
                };
                (v * 255.0).round().clamp(0.0, 255.0) as u8
            };
            Some((
                format!(
                    "#{:02x}{:02x}{:02x}",
                    channel(h + 1.0 / 3.0),
                    channel(h),
                    channel(h - 1.0 / 3.0)
                ),
                alpha(3)?,
            ))
        }
        _ => None,
    }
}

/// A paint or layout value as the reader holds it.
#[derive(Clone, Debug, PartialEq)]
enum Prop {
    Const(Value),
    /// `["match", input, …]`: the field's text and its arms.
    Match {
        field: String,
        arms: Vec<(Vec<String>, Value)>,
        fallback: Value,
    },
    /// `["step", input, …]`: the field's number, the first output and the stops.
    Step {
        field: String,
        base: Value,
        stops: Vec<(f64, Value)>,
    },
    /// A width in metres: the metres.
    Metres(f64),
}

fn prop_of(v: &Value, key: &str, lost: &mut Vec<String>, what: &str) -> Option<Prop> {
    let Some(a) = v.as_array() else {
        return Some(Prop::Const(v.clone()));
    };
    let Some(name) = a.first().and_then(Value::as_str) else {
        return Some(Prop::Const(v.clone()));
    };
    match name {
        "literal" => a.get(1).cloned().map(Prop::Const),
        "rgb" | "rgba" if a.len() >= 4 => {
            let c = |i: usize| a[i].as_f64().map(|x| x.round().clamp(0.0, 255.0) as u8);
            let alpha = a.get(4).and_then(Value::as_f64).unwrap_or(1.0);
            Some(Prop::Const(json!(css_rgba(
                &format!("#{:02x}{:02x}{:02x}", c(1)?, c(2)?, c(3)?),
                alpha
            ))))
        }
        "to-color" if a.len() == 2 && a[1].is_string() => Some(Prop::Const(a[1].clone())),
        "match" if a.len() >= 5 && a.len() % 2 == 1 => {
            let field = operand_field(&a[1])?;
            let mut arms = Vec::new();
            let mut i = 2;
            while i + 1 < a.len() {
                let labels: Vec<String> = labels_of(&a[i])?
                    .into_iter()
                    .map(|l| match l {
                        Lit::Num(x) => num(x),
                        Lit::Text(t) => t,
                    })
                    .collect();
                arms.push((labels, a[i + 1].clone()));
                i += 2;
            }
            Some(Prop::Match {
                field,
                arms,
                fallback: a[a.len() - 1].clone(),
            })
        }
        "step" if a.len() >= 3 && a.len() % 2 == 1 => {
            let field = operand_field(&a[1])?;
            let mut stops = Vec::new();
            let mut i = 3;
            while i + 1 < a.len() {
                stops.push((a[i].as_f64()?, a[i + 1].clone()));
                i += 2;
            }
            Some(Prop::Step {
                field,
                base: a[2].clone(),
                stops,
            })
        }
        "interpolate" if a.len() >= 5 && a[2] == json!(["zoom"]) => {
            let stops: Vec<(f64, f64)> = a[3..]
                .chunks(2)
                .filter_map(|c| Some((c.first()?.as_f64()?, c.get(1)?.as_f64()?)))
                .collect();
            let base2 = a[1].as_array().is_some_and(|b| {
                b.first() == Some(&json!("exponential"))
                    && b.get(1).and_then(Value::as_f64) == Some(2.0)
            });
            if base2 && stops.len() >= 2 && stops.len() * 2 == a.len() - 3 {
                let (z0, v0) = stops[0];
                let w0 = v0 / libm::pow(2.0, z0);
                if stops.iter().all(|(z, v)| {
                    (v - w0 * libm::pow(2.0, *z)).abs() <= 1e-6 * v.abs().max(w0.abs()).max(1e-300)
                }) {
                    return Some(Prop::Metres(w0 * METRES_PER_PX_Z0));
                }
            }
            lost.push(format!(
                "{what}: “{key}” yakınlaştırmaya göre değişen bir eğri; ilk durağının değeri alındı"
            ));
            a.get(4).cloned().map(Prop::Const)
        }
        other => {
            lost.push(format!(
                "{what}: “{key}” değerindeki “{other}” ifadesi KentOS'ta yok; değer alınmadı"
            ));
            None
        }
    }
}

/// Which class of a selector a value is read at.
#[derive(Clone, Debug)]
enum Class {
    Value(String),
    Other,
    /// A step's interval from this bound.
    Low(f64),
    /// A step's first interval.
    First,
}

impl Prop {
    fn at(&self, class: &Class) -> Value {
        match self {
            Prop::Const(v) => v.clone(),
            Prop::Metres(m) => json!({ "metres": m }),
            Prop::Match { arms, fallback, .. } => match class {
                Class::Value(s) => arms
                    .iter()
                    .find(|(l, _)| l.contains(s))
                    .map_or_else(|| fallback.clone(), |(_, v)| v.clone()),
                _ => fallback.clone(),
            },
            Prop::Step { base, stops, .. } => match class {
                Class::Low(x) => stops
                    .iter()
                    .rev()
                    .find(|(t, _)| t <= x)
                    .map_or_else(|| base.clone(), |(_, v)| v.clone()),
                _ => base.clone(),
            },
        }
    }
}

/// A layer the reader draws.
struct Raw<'a> {
    src: &'a StyleLayer,
    filter: Option<Filter>,
    geometry: Option<Vec<String>>,
    paint: Vec<(String, Prop)>,
    layout: Map<String, Value>,
    disabled: bool,
    min_scale: Option<f64>,
    max_scale: Option<f64>,
}

impl Raw<'_> {
    fn prop(&self, key: &str) -> Option<&Prop> {
        self.paint.iter().find(|(k, _)| k == key).map(|(_, p)| p)
    }

    fn value(&self, key: &str, class: &Class) -> Option<Value> {
        self.prop(key).map(|p| p.at(class))
    }

    fn number(&self, key: &str, class: &Class) -> Option<f64> {
        self.value(key, class)?.as_f64()
    }

    fn colour(&self, key: &str, class: &Class) -> Option<(String, f64)> {
        css_in(self.value(key, class)?.as_str()?)
    }

    /// A width: its pixels or its metres.
    fn length(&self, key: &str, class: &Class) -> Option<Len> {
        let v = self.value(key, class)?;
        match v.get("metres").and_then(Value::as_f64) {
            Some(m) => Some(Len::Metres(m)),
            None => v.as_f64().map(Len::Px),
        }
    }
}

fn scale_of_zoom(z: f64) -> f64 {
    (SCALE_Z0 / libm::pow(2.0, z)).round()
}

const READ_KEYS: [&str; 17] = [
    "fill-color",
    "fill-opacity",
    "fill-outline-color",
    "fill-antialias",
    "line-color",
    "line-width",
    "line-opacity",
    "line-dasharray",
    "circle-radius",
    "circle-color",
    "circle-opacity",
    "circle-stroke-color",
    "circle-stroke-width",
    "circle-stroke-opacity",
    "circle-pitch-alignment",
    "circle-pitch-scale",
    "fill-sort-key",
];

fn raw_of<'a>(l: &'a StyleLayer, lost: &mut Vec<String>) -> Option<Raw<'a>> {
    let what = format!("“{}” katmanı", l.id);
    if !matches!(l.kind.as_str(), "fill" | "line" | "circle") {
        match l.kind.as_str() {
            "symbol" => lost.push(format!(
                "{what}: etiket katmanı alınmadı; KentOS'ta etiketler ayrıdır"
            )),
            other => lost.push(format!(
                "{what}: “{other}” katmanı KentOS'un çizebileceği bir katman değil, alınmadı"
            )),
        }
        return None;
    }
    let fin = match filter_in(l.filter.as_ref()) {
        Ok(f) => f,
        Err(why) => {
            lost.push(format!("{what}: {why}; katman alınmadı"));
            return None;
        }
    };
    let mut paint = Vec::new();
    let mut ignored: Vec<&str> = Vec::new();
    for (k, v) in &l.paint {
        if !READ_KEYS.contains(&k.as_str()) {
            ignored.push(k);
            continue;
        }
        if let Some(p) = prop_of(v, k, lost, &what) {
            paint.push((k.clone(), p));
        }
    }
    // A layer whose colour is none KentOS can read would draw something else: it is left out.
    let colour_key = match l.kind.as_str() {
        "fill" => "fill-color",
        "line" => "line-color",
        _ => "circle-color",
    };
    if l.paint.contains_key(colour_key) && !paint.iter().any(|(k, _)| k == colour_key) {
        lost.push(format!(
            "{what}: “{colour_key}” okunamadığı için katman alınmadı"
        ));
        return None;
    }
    for (k, v) in &l.layout {
        if matches!(
            k.as_str(),
            "visibility"
                | "line-cap"
                | "line-join"
                | "line-miter-limit"
                | "line-round-limit"
                | "circle-sort-key"
                | "fill-sort-key"
                | "line-sort-key"
        ) {
            continue;
        }
        let _ = v;
        ignored.push(k);
    }
    if !ignored.is_empty() {
        ignored.sort_unstable();
        lost.push(format!(
            "{what}: {} özellikleri alınmadı",
            ignored.join(", ")
        ));
    }
    Some(Raw {
        src: l,
        filter: fin.filter,
        geometry: fin.geometry,
        paint,
        layout: l.layout.clone(),
        disabled: l.layout.get("visibility") == Some(&json!("none")),
        min_scale: l.max_zoom.map(scale_of_zoom),
        max_scale: l.min_zoom.filter(|z| *z > 0.0).map(scale_of_zoom),
    })
}

// ── Symbols ───────────────────────────────────────────────────────────────────

/// A symbol layer of KentOS read from a style layer, with its opacity.
struct Drawn {
    symbol: &'static str,
    layer: Value,
    opacity: f64,
}

fn kentos_colour(c: &(String, f64), opacity: f64) -> String {
    colour_with(&c.0, Some((c.1 * opacity).clamp(0.0, 1.0)))
}

fn len_parts(l: Len) -> (f64, &'static str) {
    match l {
        Len::Px(p) => (round4(p), "px"),
        Len::Metres(m) => (round4(m), "m"),
    }
}

/// The symbol layers one style layer draws at a class.
fn draw(raw: &Raw<'_>, class: &Class, others_fill: bool, lost: &mut Vec<String>) -> Vec<Drawn> {
    let what = format!("“{}” katmanı", raw.src.id);
    let hint = raw
        .src
        .metadata
        .get("kentos:symbol")
        .and_then(Value::as_str);
    let mut out = Vec::new();
    match raw.src.kind.as_str() {
        "fill" => {
            let colour = raw
                .colour("fill-color", class)
                .unwrap_or(("#000000".into(), 1.0));
            let opacity = raw
                .number("fill-opacity", class)
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);
            out.push(Drawn {
                symbol: "fill",
                layer: json!({ "type": "simpleFill", "color": kentos_colour(&colour, 1.0) }),
                opacity,
            });
            if let Some(o) = raw.colour("fill-outline-color", class) {
                out.push(Drawn {
                    symbol: "fill",
                    layer: json!({
                        "type": "simpleLine", "color": kentos_colour(&o, 1.0),
                        "width": 1, "unit": "px"
                    }),
                    opacity,
                });
            }
        }
        "line" => {
            let colour = raw
                .colour("line-color", class)
                .unwrap_or(("#000000".into(), 1.0));
            let opacity = raw
                .number("line-opacity", class)
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);
            let width = raw.length("line-width", class).unwrap_or(Len::Px(1.0));
            let (w, unit) = len_parts(width);
            let mut l = json!({
                "type": "simpleLine", "color": kentos_colour(&colour, 1.0),
                "width": w, "unit": unit,
            });
            if let Some(d) = raw.value("line-dasharray", class).and_then(|d| {
                d.as_array()
                    .map(|a| a.iter().filter_map(Value::as_f64).collect::<Vec<_>>())
            }) && !d.is_empty()
            {
                l["dash"] = json!(d.iter().map(|r| round4(r * w)).collect::<Vec<_>>());
            }
            l["cap"] = json!(match raw.layout.get("line-cap").and_then(Value::as_str) {
                Some("round") => "round",
                Some("square") => "square",
                _ => "butt",
            });
            l["join"] = json!(match raw.layout.get("line-join").and_then(Value::as_str) {
                Some("round") => "round",
                Some("bevel") => "bevel",
                _ => "miter",
            });
            let on_polygons = match (hint, &raw.geometry) {
                (Some("fill"), _) => true,
                (Some(_), _) => false,
                (None, Some(g)) => {
                    g.iter().any(|x| x == "Polygon") && !g.iter().any(|x| x == "LineString")
                }
                (None, None) => others_fill,
            };
            out.push(Drawn {
                symbol: if on_polygons { "fill" } else { "line" },
                layer: l,
                opacity,
            });
        }
        _ => {
            let radius = raw.length("circle-radius", class).unwrap_or(Len::Px(5.0));
            let (r, unit) = len_parts(radius);
            let fill_o = raw
                .number("circle-opacity", class)
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);
            let stroke_o = raw
                .number("circle-stroke-opacity", class)
                .unwrap_or(1.0)
                .clamp(0.0, 1.0);
            let fill = raw
                .colour("circle-color", class)
                .unwrap_or(("#000000".into(), 1.0));
            let stroke = raw
                .colour("circle-stroke-color", class)
                .unwrap_or(("#000000".into(), 1.0));
            let sw = raw.length("circle-stroke-width", class);
            let stroke_on = sw.is_some_and(|s| match s {
                Len::Px(p) => p > 0.0,
                Len::Metres(m) => m > 0.0,
            }) && stroke_o > 0.0;
            let mut m = json!({
                "type": "shape", "shape": "circle", "size": round4(r * 2.0), "unit": unit,
                "fill": if fill_o > 0.0 { json!(kentos_colour(&fill, 1.0)) } else { Value::Null },
            });
            if stroke_on && let Some(s) = sw {
                let (w, su) = len_parts(s);
                if su != unit {
                    lost.push(format!(
                        "{what}: çemberin kenar kalınlığının birimi boyununkinden başka; kalınlık alınmadı"
                    ));
                } else {
                    m["stroke"] = json!(kentos_colour(&stroke, 1.0));
                    m["strokeWidth"] = json!(w);
                }
            }
            // Fill and stroke may differ in opacity: baked into their colours when they do.
            let opacity = if fill_o > 0.0 && stroke_on && (fill_o - stroke_o).abs() > 1e-9 {
                m["fill"] = json!(kentos_colour(&fill, fill_o));
                m["stroke"] = json!(kentos_colour(&stroke, stroke_o));
                1.0
            } else if fill_o > 0.0 {
                fill_o
            } else {
                stroke_o
            };
            out.push(Drawn {
                symbol: "marker",
                layer: m,
                opacity,
            });
        }
    }
    out
}

/// A symbol set from drawn layers: one symbol of each class, an opacity common to its layers its own, else baked
/// into the layers' colours.
fn set_of(drawn: Vec<Drawn>, disabled: &[bool]) -> Value {
    let mut set = Map::new();
    for key in ["fill", "line", "marker"] {
        let mine: Vec<(usize, &Drawn)> = drawn
            .iter()
            .enumerate()
            .filter(|(_, d)| d.symbol == key)
            .collect();
        if mine.is_empty() {
            continue;
        }
        let common = mine[0].1.opacity;
        let same = mine.iter().all(|(_, d)| (d.opacity - common).abs() < 1e-9);
        let mut layers = Vec::new();
        for (k, (i, d)) in mine.iter().enumerate() {
            let mut l = d.layer.clone();
            l["id"] = json!(format!("{}{k}", &key[..1]));
            if !same && d.opacity < 1.0 {
                for field in ["color", "fill", "stroke"] {
                    if let Some(c) = l.get(field).and_then(Value::as_str).and_then(css_in) {
                        l[field] =
                            json!(colour_with(&c.0, Some((c.1 * d.opacity).clamp(0.0, 1.0))));
                    }
                }
            }
            if disabled.get(*i).copied().unwrap_or(false) {
                l["enabled"] = json!(false);
            }
            layers.push(l);
        }
        let mut sym = json!({ "type": key, "layers": layers });
        if same && common < 1.0 {
            sym["opacity"] = json!(round3(common));
        }
        set.insert(key.to_owned(), sym);
    }
    Value::Object(set)
}

// ── Renderers ─────────────────────────────────────────────────────────────────

/// The symbols of `raws` at `class`.
fn symbols_at(raws: &[&Raw<'_>], class: &Class, lost: &mut Vec<String>) -> Value {
    let fill_on = raws.iter().any(|r| r.src.kind == "fill");
    let mut drawn = Vec::new();
    let mut disabled = Vec::new();
    for r in raws {
        for d in draw(r, class, fill_on, lost) {
            drawn.push(d);
            disabled.push(r.disabled);
        }
    }
    set_of(drawn, &disabled)
}

/// What one selector is.
enum Selector {
    Categories(String),
    Steps(String),
}

fn selector_of(raws: &[Raw<'_>], lost: &mut Vec<String>) -> Option<Selector> {
    let mut found: Option<Selector> = None;
    for r in raws {
        for (k, p) in &r.paint {
            let (field, is_match) = match p {
                Prop::Match { field, .. } => (field, true),
                Prop::Step { field, .. } => (field, false),
                _ => continue,
            };
            match &found {
                None => {
                    found = Some(if is_match {
                        Selector::Categories(field.clone())
                    } else {
                        Selector::Steps(field.clone())
                    });
                }
                Some(Selector::Categories(f)) if is_match && f == field => {}
                Some(Selector::Steps(f)) if !is_match && f == field => {}
                Some(_) => lost.push(format!(
                    "“{}” katmanının “{k}” değeri başka bir alana ya da türe bağlı; yedek değeri alındı",
                    r.src.id
                )),
            }
        }
    }
    found
}

/// The values a layer's filter allows of `field` when it says exactly that, and the rest of the filter.
fn split_members(filter: &Option<Filter>, field: &str) -> (Option<Vec<String>>, Option<Filter>) {
    let members = |f: &Filter| -> Option<Vec<String>> {
        let lit = |p: &Filter| match p {
            Filter::Cmp(Op::Eq, n, l) if n == field => Some(match l {
                Lit::Num(x) => num(*x),
                Lit::Text(t) => t.clone(),
            }),
            _ => None,
        };
        match f {
            Filter::Or(p) => p.iter().map(lit).collect(),
            other => lit(other).map(|v| vec![v]),
        }
    };
    let Some(f) = filter else {
        return (None, None);
    };
    if let Some(m) = members(f) {
        return (Some(m), None);
    }
    if let Filter::And(parts) = f {
        for (i, p) in parts.iter().enumerate() {
            if let Some(m) = members(p) {
                let rest: Vec<Filter> = parts
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .map(|(_, q)| q.clone())
                    .collect();
                return (Some(m), Some(and_of(rest)));
            }
        }
    }
    (None, Some(f.clone()))
}

fn and_of(mut parts: Vec<Filter>) -> Filter {
    if parts.len() == 1 {
        parts.remove(0)
    } else {
        Filter::And(parts)
    }
}

/// The bounds a layer's filter puts on `field` (low, high, whether the high is inclusive) and the rest of it.
#[allow(clippy::type_complexity)]
fn split_bounds(
    filter: &Option<Filter>,
    field: &str,
) -> (Option<f64>, Option<(f64, bool)>, Option<Filter>) {
    let Some(f) = filter else {
        return (None, None, None);
    };
    let parts: Vec<&Filter> = match f {
        Filter::And(p) => p.iter().collect(),
        other => vec![other],
    };
    let (mut lo, mut hi) = (None, None);
    let mut rest: Vec<Filter> = Vec::new();
    for p in parts {
        match p {
            Filter::Cmp(Op::Ge, n, Lit::Num(x)) if n == field && lo.is_none() => lo = Some(*x),
            Filter::Cmp(Op::Le, n, Lit::Num(x)) if n == field && hi.is_none() => {
                hi = Some((*x, true))
            }
            Filter::Cmp(Op::Lt, n, Lit::Num(x)) if n == field && hi.is_none() => {
                hi = Some((*x, false));
            }
            Filter::Between(n, a, b) if n == field && lo.is_none() && hi.is_none() => {
                lo = Some(*a);
                hi = Some((*b, true));
            }
            // The guard of a field said to be there.
            Filter::Not(i) if matches!(&**i, Filter::Null(n) if n == field) => {}
            other => rest.push(other.clone()),
        }
    }
    (lo, hi, (!rest.is_empty()).then(|| and_of(rest)))
}

fn label_of(raw: &Raw<'_>, fallback: String) -> String {
    raw.src
        .metadata
        .get("kentos:class")
        .and_then(Value::as_str)
        .map_or(fallback, str::to_owned)
}

fn conj(a: Option<Filter>, b: Option<Filter>) -> Option<Filter> {
    match (a, b) {
        (Some(x), Some(y)) => Some(Filter::And(vec![x, y])),
        (x, None) => x,
        (None, y) => y,
    }
}

/// A rule of the reader's renderer.
struct RuleIn {
    label: String,
    filter: Option<Filter>,
    is_else: bool,
    min_scale: Option<f64>,
    max_scale: Option<f64>,
    symbols: Value,
}

fn rules_value(rules: Vec<RuleIn>) -> Value {
    let list: Vec<Value> = rules
        .into_iter()
        .enumerate()
        .map(|(i, r)| {
            let mut v =
                json!({ "id": format!("r{}", i + 1), "label": r.label, "symbols": r.symbols });
            if r.is_else {
                v["isElse"] = json!(true);
            } else if let Some(f) = &r.filter {
                v["filter"] = json!(filter_text(f));
            }
            if let Some(s) = r.min_scale {
                v["minScale"] = json!(s);
            }
            if let Some(s) = r.max_scale {
                v["maxScale"] = json!(s);
            }
            v
        })
        .collect();
    json!({ "type": "rules", "rules": list })
}

/// Rules of the same one-field values or ranges as a categorized or a graduated renderer, else none.
fn recognised(rules: &[RuleIn]) -> Option<Value> {
    if rules.len() < 2
        || rules
            .iter()
            .any(|r| r.min_scale.is_some() || r.max_scale.is_some())
    {
        return None;
    }
    // Categories: Eq of one field, an ELSE last.
    let mut field: Option<&str> = None;
    let mut cats = Vec::new();
    let mut other = None;
    let mut ok = true;
    for r in rules {
        match &r.filter {
            Some(Filter::Cmp(Op::Eq, f, lit)) if !r.is_else => {
                if field.is_some_and(|x| x != f) {
                    ok = false;
                    break;
                }
                field = Some(f);
                let value = match lit {
                    Lit::Num(x) => num(*x),
                    Lit::Text(t) => t.clone(),
                };
                cats.push(json!({ "value": value, "label": r.label, "symbols": r.symbols }));
            }
            None if r.is_else && other.is_none() => other = Some(r.symbols.clone()),
            _ => {
                ok = false;
                break;
            }
        }
    }
    if ok && let Some(f) = field {
        let mut v = json!({ "type": "categorized", "expr": format!("[{f}]"), "categories": cats });
        if let Some(o) = other {
            v["other"] = o;
        }
        return Some(v);
    }
    // Classes: min ≤ field < max, the last ≤ max, one field.
    let mut field: Option<&str> = None;
    let mut classes = Vec::new();
    let n = rules.len();
    for (k, r) in rules.iter().enumerate() {
        if r.is_else {
            return None;
        }
        let (f, lo, hi, closed) = match &r.filter {
            Some(Filter::And(p)) if p.len() == 2 => match (&p[0], &p[1]) {
                (Filter::Cmp(Op::Ge, f1, Lit::Num(a)), Filter::Cmp(op2, f2, Lit::Num(b)))
                    if f1 == f2 && matches!(op2, Op::Lt) =>
                {
                    (f1.as_str(), *a, *b, false)
                }
                _ => return None,
            },
            Some(Filter::Between(f, a, b)) => (f.as_str(), *a, *b, true),
            _ => return None,
        };
        // Only the last class may take its upper bound.
        if closed != (k + 1 == n) {
            return None;
        }
        if field.is_some_and(|x| x != f) {
            return None;
        }
        field = Some(f);
        classes.push(json!({ "min": lo, "max": hi, "label": r.label, "symbols": r.symbols }));
    }
    let field = field?;
    Some(json!({ "type": "graduated", "expr": format!("[{field}]"), "classes": classes }))
}

/// KentOS's renderer of the style layers `layers`, drawn in this order: one field's `match` or `step` becomes a
/// categorized or a graduated renderer, filters rules, none a single symbol.
pub fn renderer_of(layers: &[&StyleLayer]) -> Result<Read, String> {
    let mut lost = Vec::new();
    let raws: Vec<Raw<'_>> = layers.iter().filter_map(|l| raw_of(l, &mut lost)).collect();
    if raws.is_empty() {
        return Err(format!(
            "Stilde KentOS'un çizebileceği (fill, line, circle) bir katman yok.{}",
            if lost.is_empty() {
                String::new()
            } else {
                format!(" {}", lost.join("; "))
            }
        ));
    }
    if raws
        .iter()
        .any(|r| r.min_scale.is_some() || r.max_scale.is_some())
    {
        lost.push(
            "yakınlaştırma düzeyleri ölçek paydasına yaklaşık çevrildi (OGC'nin 0,28 mm pikseli, ekvator, 512 pikselli karo)"
                .into(),
        );
    }
    let selector = selector_of(&raws, &mut lost);
    let (renderer, rules) = match selector {
        None => generic(&raws, &mut lost),
        Some(Selector::Categories(field)) => categories(&raws, &field, &mut lost),
        Some(Selector::Steps(field)) => steps(&raws, &field, &mut lost),
    };
    if rules == 0 {
        return Err("Stilde KentOS'un çizebileceği bir sınıf ya da kural yok.".into());
    }
    Ok(Read {
        name: None,
        renderer,
        lost,
        rules,
    })
}

/// The layers in runs that draw together: the same filter, scale range.
fn runs<'a, 'b>(raws: &'b [Raw<'a>], key: &dyn Fn(&Raw<'a>) -> String) -> Vec<Vec<&'b Raw<'a>>> {
    let mut out: Vec<Vec<&Raw<'a>>> = Vec::new();
    let mut last: Option<String> = None;
    for r in raws {
        let k = key(r);
        if last.as_deref() == Some(k.as_str()) {
            if let Some(run) = out.last_mut() {
                run.push(r);
            }
        } else {
            out.push(vec![r]);
            last = Some(k);
        }
    }
    out
}

fn run_key(r: &Raw<'_>) -> String {
    format!(
        "{:?}|{:?}|{:?}",
        r.filter.as_ref().map(filter_text),
        r.min_scale,
        r.max_scale
    )
}

fn generic(raws: &[Raw<'_>], lost: &mut Vec<String>) -> (Value, usize) {
    let groups = runs(raws, &run_key);
    let mut rules: Vec<RuleIn> = Vec::new();
    for (i, g) in groups.iter().enumerate() {
        let first = g[0];
        let symbols = symbols_at(g, &Class::First, lost);
        let fallback = format!("Kural {}", i + 1);
        rules.push(RuleIn {
            label: label_of(first, fallback),
            filter: first.filter.clone(),
            is_else: false,
            min_scale: first.min_scale,
            max_scale: first.max_scale,
            symbols,
        });
    }
    if rules.len() == 1
        && rules[0].filter.is_none()
        && rules[0].min_scale.is_none()
        && rules[0].max_scale.is_none()
    {
        let symbols = rules.remove(0).symbols;
        return (json!({ "type": "single", "symbols": symbols }), 1);
    }
    let n = rules.len();
    match recognised(&rules) {
        Some(v) => (v, n),
        None => (rules_value(rules), n),
    }
}

fn categories(raws: &[Raw<'_>], field: &str, lost: &mut Vec<String>) -> (Value, usize) {
    let split: Vec<(Option<Vec<String>>, Option<Filter>)> = raws
        .iter()
        .map(|r| split_members(&r.filter, field))
        .collect();
    // Segments: runs of layers with the same members, rest of the filter and scale range.
    let mut segments: Vec<Vec<usize>> = Vec::new();
    for i in 0..raws.len() {
        let same = segments.last().is_some_and(|s| {
            let j = s[0];
            split[j] == split[i]
                && raws[j].min_scale == raws[i].min_scale
                && raws[j].max_scale == raws[i].max_scale
        });
        if same {
            if let Some(s) = segments.last_mut() {
                s.push(i);
            }
        } else {
            segments.push(vec![i]);
        }
    }
    let labels: Map<String, Value> = raws
        .iter()
        .find_map(|r| {
            r.src
                .metadata
                .get("kentos:labels")
                .and_then(Value::as_object)
                .cloned()
        })
        .unwrap_or_default();
    let label_of_value = |v: &str| -> String {
        labels
            .get(v)
            .and_then(Value::as_str)
            .map_or_else(|| v.to_owned(), str::to_owned)
    };
    let direct = segments.len() == 1
        && split[0].1.is_none()
        && raws[0].min_scale.is_none()
        && raws[0].max_scale.is_none();
    let mut cats: Vec<Value> = Vec::new();
    let mut other: Option<Value> = None;
    let mut rules: Vec<RuleIn> = Vec::new();
    let mut count = 0usize;
    for seg in &segments {
        let members = &split[seg[0]].0;
        let rest = split[seg[0]].1.clone();
        let layers: Vec<&Raw<'_>> = seg.iter().map(|i| &raws[*i]).collect();
        let mut values: Vec<String> = Vec::new();
        match members {
            Some(m) => values.extend(m.iter().cloned()),
            None => {
                for r in &layers {
                    for (_, p) in &r.paint {
                        if let Prop::Match { field: f, arms, .. } = p
                            && f == field
                        {
                            for (labels, _) in arms {
                                for l in labels {
                                    if !values.contains(l) {
                                        values.push(l.clone());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for v in &values {
            let symbols = symbols_at(&layers, &Class::Value(v.clone()), lost);
            let label = label_of_value(v);
            count += 1;
            if direct {
                cats.push(json!({ "value": v, "label": label, "symbols": symbols }));
            } else {
                let lit = match v.parse::<f64>() {
                    Ok(x) if num(x) == *v => Lit::Num(x),
                    _ => Lit::Text(v.clone()),
                };
                rules.push(RuleIn {
                    label,
                    filter: conj(
                        rest.clone(),
                        Some(Filter::Cmp(Op::Eq, field.to_owned(), lit)),
                    ),
                    is_else: false,
                    min_scale: layers[0].min_scale,
                    max_scale: layers[0].max_scale,
                    symbols,
                });
            }
        }
        if members.is_none() {
            let symbols = symbols_at(&layers, &Class::Other, lost);
            count += 1;
            if direct {
                other = Some(symbols);
            } else {
                let any: Vec<Filter> = values
                    .iter()
                    .map(|v| {
                        Filter::Cmp(
                            Op::Eq,
                            field.to_owned(),
                            match v.parse::<f64>() {
                                Ok(x) if num(x) == *v => Lit::Num(x),
                                _ => Lit::Text(v.clone()),
                            },
                        )
                    })
                    .collect();
                let none_of = (!any.is_empty()).then(|| Filter::Not(Box::new(Filter::Or(any))));
                let filter = conj(rest.clone(), none_of);
                rules.push(RuleIn {
                    label: "Diğerleri".into(),
                    is_else: filter.is_none(),
                    filter,
                    min_scale: layers[0].min_scale,
                    max_scale: layers[0].max_scale,
                    symbols,
                });
            }
        }
    }
    if direct {
        let mut v =
            json!({ "type": "categorized", "expr": format!("[{field}]"), "categories": cats });
        if let Some(o) = other {
            v["other"] = o;
        }
        (v, count)
    } else {
        (rules_value(rules), count)
    }
}

fn steps(raws: &[Raw<'_>], field: &str, lost: &mut Vec<String>) -> (Value, usize) {
    let mut cuts: Vec<f64> = Vec::new();
    for r in raws {
        for (_, p) in &r.paint {
            if let Prop::Step {
                field: f, stops, ..
            } = p
                && f == field
            {
                cuts.extend(stops.iter().map(|s| s.0));
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let split: Vec<_> = raws
        .iter()
        .map(|r| split_bounds(&r.filter, field))
        .collect();
    let mut segments: Vec<Vec<usize>> = Vec::new();
    for i in 0..raws.len() {
        let same = segments.last().is_some_and(|s| {
            let j = s[0];
            split[j] == split[i]
                && raws[j].min_scale == raws[i].min_scale
                && raws[j].max_scale == raws[i].max_scale
        });
        if same {
            if let Some(s) = segments.last_mut() {
                s.push(i);
            }
        } else {
            segments.push(vec![i]);
        }
    }
    let labels: Vec<String> = raws
        .iter()
        .find_map(|r| {
            r.src
                .metadata
                .get("kentos:labels")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .map(|l| l.as_str().unwrap_or("").to_owned())
                        .collect()
                })
        })
        .unwrap_or_default();
    let direct = segments.len() == 1
        && split[0].0.is_some()
        && split[0].1.is_some_and(|h| h.1)
        && split[0].2.is_none()
        && raws[0].min_scale.is_none()
        && raws[0].max_scale.is_none();
    let mut classes: Vec<Value> = Vec::new();
    let mut rules: Vec<RuleIn> = Vec::new();
    let mut count = 0usize;
    for seg in &segments {
        let (lo, hi, rest) = split[seg[0]].clone();
        let layers: Vec<&Raw<'_>> = seg.iter().map(|i| &raws[*i]).collect();
        // The interval edges inside the filter's bounds.
        let inside: Vec<f64> = cuts
            .iter()
            .copied()
            .filter(|c| lo.is_none_or(|l| *c > l) && hi.is_none_or(|h| *c < h.0))
            .collect();
        let mut edges: Vec<Option<f64>> = vec![lo];
        edges.extend(inside.iter().map(|c| Some(*c)));
        edges.push(hi.map(|h| h.0));
        for k in 0..edges.len() - 1 {
            let class = match edges[k] {
                Some(x) => Class::Low(x),
                None => Class::First,
            };
            let symbols = symbols_at(&layers, &class, lost);
            let (a, b) = (edges[k], edges[k + 1]);
            let default_label = match (a, b) {
                (Some(a), Some(b)) => format!("{} - {}", num(a), num(b)),
                (Some(a), None) => format!("≥ {}", num(a)),
                (None, Some(b)) => format!("< {}", num(b)),
                (None, None) => "Tümü".into(),
            };
            let label = labels
                .get(count)
                .filter(|l| !l.is_empty())
                .cloned()
                .unwrap_or(default_label);
            count += 1;
            if direct && let (Some(a), Some(b)) = (a, b) {
                classes.push(json!({ "min": a, "max": b, "label": label, "symbols": symbols }));
                continue;
            }
            let mut parts: Vec<Filter> = Vec::new();
            if let Some(a) = a {
                parts.push(Filter::Cmp(Op::Ge, field.to_owned(), Lit::Num(a)));
            }
            if let Some(b) = b {
                let last = k + 2 == edges.len();
                let op = if last && hi.is_some_and(|h| h.1) {
                    Op::Le
                } else {
                    Op::Lt
                };
                parts.push(Filter::Cmp(op, field.to_owned(), Lit::Num(b)));
            }
            let bound = (!parts.is_empty()).then(|| and_of(parts));
            rules.push(RuleIn {
                label,
                filter: conj(rest.clone(), bound),
                is_else: false,
                min_scale: layers[0].min_scale,
                max_scale: layers[0].max_scale,
                symbols,
            });
        }
    }
    if direct {
        (
            json!({ "type": "graduated", "expr": format!("[{field}]"), "classes": classes }),
            count,
        )
    } else {
        (rules_value(rules), count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<Value> {
        None
    }

    fn area() -> Kinds {
        Kinds {
            point: false,
            line: false,
            area: true,
        }
    }

    fn sources() -> Map<String, Value> {
        let mut m = Map::new();
        m.insert(
            "parsel".into(),
            json!({ "type": "geojson", "data": "parsel.geojson" }),
        );
        m
    }

    fn write_one(r: &Value, kinds: Kinds) -> Written {
        let l = LayerIn {
            id: "parsel",
            name: "Parsel",
            renderer: r,
            kinds,
            opacity: 1.0,
            source: "parsel",
            source_layer: None,
            label: None,
            labels: None,
        };
        write("Deneme", &[l], &sources(), None, &none)
    }

    fn fill(color: &str) -> Value {
        json!({ "fill": { "type": "fill", "layers": [
            { "id": "f", "type": "simpleFill", "color": color },
            { "id": "o", "type": "simpleLine", "color": "#232323", "width": 0.5, "unit": "mm" },
        ] } })
    }

    fn read_back(w: &Written) -> Read {
        let style = parse(&w.json).expect("parses");
        let all: Vec<&StyleLayer> = style.layers.iter().collect();
        renderer_of(&all).expect("reads")
    }

    #[test]
    fn a_categorized_look_is_one_layer_per_symbol_layer_with_a_match() {
        let r = json!({ "type": "categorized", "expr": "[kullanim]", "categories": [
            { "value": "Konut", "label": "Konut", "symbols": fill("#e31a1c") },
            { "value": "Ticaret", "label": "Ticari", "symbols": fill("#1f78b4") },
        ], "other": fill("#33a02c80") });
        let w = write_one(&r, area());
        assert_eq!(w.rules, 3);
        assert!(w.lost.is_empty(), "{:?}", w.lost);
        let style: Value = serde_json::from_str(&w.json).expect("json");
        let layers = style["layers"].as_array().expect("layers");
        assert_eq!(layers.len(), 2, "{}", w.json);
        assert_eq!(layers[0]["type"], "fill");
        assert_eq!(
            layers[0]["paint"]["fill-color"],
            json!([
                "match",
                ["to-string", ["get", "kullanim"]],
                "Konut",
                "#e31a1c",
                "Ticaret",
                "#1f78b4",
                "rgba(51,160,44,0.502)"
            ])
        );
        assert_eq!(layers[1]["type"], "line");
        assert_eq!(layers[1]["paint"]["line-width"], json!(1.8898));
        let back = read_back(&w);
        assert_eq!(back.renderer["type"], "categorized", "{:?}", back.lost);
        assert_eq!(back.renderer["expr"], "[kullanim]");
        assert_eq!(back.renderer["categories"][1]["label"], "Ticari");
        assert_eq!(
            back.renderer["categories"][1]["symbols"]["fill"]["layers"][0]["color"],
            "#1f78b4"
        );
        assert_eq!(
            back.renderer["other"]["fill"]["layers"][0]["color"],
            "#33a02c80"
        );
    }

    #[test]
    fn a_graduated_look_is_a_step_inside_its_bounds() {
        let r = json!({ "type": "graduated", "expr": "[alan]", "classes": [
            { "min": 0, "max": 100, "label": "Küçük", "symbols": fill("#fee5d9") },
            { "min": 100, "max": 500, "label": "Orta", "symbols": fill("#fb6a4a") },
            { "min": 500, "max": 2000, "label": "Büyük", "symbols": fill("#a50f15") },
        ] });
        let w = write_one(&r, area());
        let style: Value = serde_json::from_str(&w.json).expect("json");
        let l = &style["layers"][0];
        assert_eq!(
            l["paint"]["fill-color"],
            json!([
                "step",
                ["to-number", ["get", "alan"]],
                "#fee5d9",
                100,
                "#fb6a4a",
                500,
                "#a50f15"
            ])
        );
        assert_eq!(
            l["filter"],
            json!([
                "all",
                ["has", "alan"],
                [">=", ["to-number", ["get", "alan"]], 0],
                ["<=", ["to-number", ["get", "alan"]], 2000]
            ])
        );
        let back = read_back(&w);
        assert_eq!(back.renderer["type"], "graduated", "{:?}", back.lost);
        let classes = back.renderer["classes"].as_array().expect("classes");
        assert_eq!(classes.len(), 3);
        assert_eq!(classes[1]["min"], 100.0);
        assert_eq!(classes[2]["max"], 2000.0);
        assert_eq!(classes[0]["label"], "Küçük");
    }

    #[test]
    fn rules_become_filters_and_an_else_the_negation_of_its_elders() {
        let r = json!({ "type": "rules", "rules": [
            { "label": "Büyük", "filter": "[alan] > 500", "symbols": fill("#a50f15") },
            { "label": "Konut", "filter": "[kullanim] = 'Konut' ve [kat] >= 3", "symbols": fill("#fb6a4a"),
              "minScale": 1000, "maxScale": 25000 },
            { "label": "Diğer", "isElse": true, "symbols": fill("#cccccc") },
        ] });
        let w = write_one(&r, area());
        assert_eq!(w.rules, 3);
        let style: Value = serde_json::from_str(&w.json).expect("json");
        let layers = style["layers"].as_array().expect("layers");
        assert_eq!(layers.len(), 6);
        assert_eq!(
            layers[0]["filter"],
            json!([
                "all",
                ["has", "alan"],
                [">", ["to-number", ["get", "alan"]], 500]
            ])
        );
        assert!(layers[2]["minzoom"].is_number() && layers[2]["maxzoom"].is_number());
        assert_eq!(layers[4]["metadata"]["kentos:class"], "Diğer");
        let back = read_back(&w);
        assert_eq!(back.renderer["type"], "rules", "{:?}", back.lost);
        let rules = back.renderer["rules"].as_array().expect("rules");
        assert_eq!(rules.len(), 3);
        assert_eq!(rules[0]["filter"], "[alan] > 500");
        assert_eq!(rules[1]["filter"], "[kullanim] = 'Konut' ve [kat] >= 3");
        assert_eq!(rules[1]["minScale"], 1000.0);
        assert_eq!(rules[1]["maxScale"], 25000.0);
        assert_eq!(
            rules[2]["filter"],
            "değil ([alan] > 500 veya ([kullanim] = 'Konut' ve [kat] >= 3))"
        );
    }

    #[test]
    fn units_go_out_in_pixels_and_metres_grow_with_the_zoom() {
        let r = json!({ "type": "single", "symbols": { "line": { "type": "line", "layers": [
            { "id": "l", "type": "simpleLine", "color": "#112233", "width": 5, "unit": "m", "dash": [10, 5], "cap": "round" },
        ] }, "marker": { "type": "marker", "opacity": 0.5, "layers": [
            { "id": "p", "type": "shape", "shape": "star", "size": 6, "unit": "mm", "fill": "#ff0000", "stroke": "#000000", "strokeWidth": 1 },
        ] } } });
        let k = Kinds {
            point: true,
            line: true,
            area: false,
        };
        let w = write_one(&r, k);
        assert!(
            w.lost.iter().any(|l| l.contains("“star” şekli")),
            "{:?}",
            w.lost
        );
        assert!(
            w.lost.iter().any(|l| l.contains("metre cinsinden")),
            "{:?}",
            w.lost
        );
        let style: Value = serde_json::from_str(&w.json).expect("json");
        let layers = style["layers"].as_array().expect("layers");
        let line = &layers[0];
        assert_eq!(
            line["filter"],
            json!(["==", ["geometry-type"], "LineString"])
        );
        assert_eq!(line["paint"]["line-width"][0], "interpolate");
        assert_eq!(line["paint"]["line-dasharray"], json!([2, 1]));
        assert_eq!(line["layout"]["line-cap"], "round");
        let circle = &layers[1];
        assert_eq!(circle["filter"], json!(["==", ["geometry-type"], "Point"]));
        assert_eq!(circle["paint"]["circle-radius"], json!(11.3386));
        assert_eq!(circle["paint"]["circle-opacity"], json!(0.5));
        let back = read_back(&w);
        let l = &back.renderer["symbols"]["line"]["layers"][0];
        assert_eq!(l["unit"], "m");
        assert!((l["width"].as_f64().expect("w") - 5.0).abs() < 1e-3, "{l}");
        assert_eq!(l["dash"], json!([10.0, 5.0]));
        let m = &back.renderer["symbols"]["marker"];
        assert_eq!(m["opacity"], 0.5);
        assert_eq!(m["layers"][0]["unit"], "px");
        assert_eq!(m["layers"][0]["size"], 22.6772);
    }

    #[test]
    fn filters_turn_between_the_two_languages() {
        let f = parse_filter("[kat] > 3 ve [ad] gibi 'A%' veya [tur] boş değil").expect("parses");
        let e = expr(&f).expect("says");
        let back = to_filter(&e).expect("reads");
        assert_eq!(filter_text(&back), filter_text(&f));
        for src in [
            "[ad] gibi '%vadi%'",
            "[ad] gibi '%vadi'",
            "[tur] boş",
            "[x] arasında 1 ve 5",
        ] {
            let f = parse_filter(src).expect("parses");
            let back = to_filter(&expr(&f).expect("says")).expect("reads");
            assert_eq!(filter_text(&back), src);
        }
        assert!(expr(&parse_filter("[ad] gibi 'a%b'").expect("parses")).is_none());
    }

    #[test]
    fn a_handwritten_style_reads_in_its_own_forms() {
        let text = r##"{ "version": 8, "sources": {}, "layers": [
            { "id": "bg", "type": "background", "paint": { "background-color": "#fff" } },
            { "id": "kent", "type": "fill", "source": "s", "source-layer": "bina",
              "filter": ["==", "class", "konut"],
              "paint": { "fill-color": "hsl(120, 50%, 50%)", "fill-opacity": 0.8, "fill-outline-color": "#333" } },
            { "id": "kent-line", "type": "line", "source": "s", "source-layer": "bina",
              "filter": ["==", "class", "konut"],
              "paint": { "line-color": "rgb(0,0,255)", "line-width": ["interpolate", ["linear"], ["zoom"], 10, 1, 18, 4] } },
            { "id": "etiket", "type": "symbol", "source": "s", "source-layer": "bina", "layout": { "text-field": "{ad}" } }
        ] }"##;
        let style = parse(text).expect("parses");
        assert_eq!(style.layers.len(), 4);
        let mine: Vec<&StyleLayer> = style
            .layers
            .iter()
            .filter(|l| l.source_layer.as_deref() == Some("bina"))
            .collect();
        let read = renderer_of(&mine).expect("reads");
        assert_eq!(read.renderer["type"], "rules", "{:?}", read.lost);
        assert_eq!(read.renderer["rules"][0]["filter"], "[class] = 'konut'");
        let fill = &read.renderer["rules"][0]["symbols"]["fill"];
        // The outline and the line draw opaque, the fill at 0.8: the opacity goes into the fill's colour.
        assert_eq!(fill["opacity"], Value::Null);
        assert_eq!(fill["layers"][0]["color"], "#40bf40cc");
        assert_eq!(fill["layers"].as_array().map(Vec::len), Some(3));
        assert!(
            read.lost.iter().any(|l| l.contains("etiket katmanı")),
            "{:?}",
            read.lost
        );
        assert!(
            read.lost.iter().any(|l| l.contains("yakınlaştırmaya göre")),
            "{:?}",
            read.lost
        );
    }

    #[test]
    fn labels_become_symbol_layers() {
        let r = json!({ "type": "single", "symbols": fill("#cccccc") });
        let label = json!({ "placement": "inside", "size": 12, "template": "{ad} ({kat})", "weight": 600,
            "color": "#112233", "halo": { "width": 2, "color": "#ffffff" }, "callout": { "kind": "straight" } });
        let l = LayerIn {
            id: "parsel",
            name: "Parsel",
            renderer: &r,
            kinds: area(),
            opacity: 1.0,
            source: "parsel",
            source_layer: None,
            label: Some(&label),
            labels: None,
        };
        let w = write(
            "Deneme",
            &[l],
            &sources(),
            Some("https://example.org/{fontstack}/{range}.pbf"),
            &none,
        );
        let style: Value = serde_json::from_str(&w.json).expect("json");
        assert_eq!(
            style["glyphs"],
            "https://example.org/{fontstack}/{range}.pbf"
        );
        let sym = style["layers"]
            .as_array()
            .expect("l")
            .iter()
            .find(|l| l["type"] == "symbol")
            .expect("symbol");
        assert_eq!(
            sym["layout"]["text-field"],
            json!([
                "concat",
                ["to-string", ["get", "ad"]],
                " (",
                ["to-string", ["get", "kat"]],
                ")"
            ])
        );
        assert_eq!(sym["layout"]["text-font"], json!(["Noto Sans Bold"]));
        assert_eq!(sym["paint"]["text-halo-width"], json!(2));
        assert!(
            w.lost.iter().any(|l| l.contains("çağrı çizgisi")),
            "{:?}",
            w.lost
        );
    }

    /// With `KENTOS_MAPLIBRE_OUT=dir`, the styles above written there, for the official validator.
    #[test]
    fn the_styles_are_written_for_the_validator() {
        let Ok(dir) = std::env::var("KENTOS_MAPLIBRE_OUT") else {
            return;
        };
        let cat = json!({ "type": "categorized", "expr": "[kullanim]", "categories": [
            { "value": "Konut", "label": "Konut", "symbols": fill("#e31a1c") },
            { "value": "Ticaret", "label": "Ticari", "symbols": fill("#1f78b4") },
        ], "other": fill("#33a02c80") });
        let grad = json!({ "type": "graduated", "expr": "[alan]", "classes": [
            { "min": 0, "max": 100, "label": "Küçük", "symbols": fill("#fee5d9") },
            { "min": 100, "max": 2000, "label": "Büyük", "symbols": fill("#a50f15") },
        ] });
        let rules = json!({ "type": "rules", "rules": [
            { "label": "Büyük", "filter": "[alan] > 500", "symbols": fill("#a50f15") },
            { "label": "Konut", "filter": "[ad] gibi 'A%' veya [kat] arasında 3 ve 5", "symbols": fill("#fb6a4a"),
              "minScale": 1000, "maxScale": 25000 },
            { "label": "Diğer", "isElse": true, "symbols": fill("#cccccc") },
        ] });
        let marker = json!({ "type": "single", "symbols": { "line": { "type": "line", "layers": [
            { "id": "l", "type": "simpleLine", "color": "#112233", "width": 5, "unit": "m", "dash": [10, 5], "cap": "round" },
        ] }, "marker": { "type": "marker", "opacity": 0.5, "layers": [
            { "id": "p", "type": "shape", "shape": "circle", "size": 6, "unit": "mm", "fill": "#ff0000", "stroke": "#000000", "strokeWidth": 1 },
        ] } } });
        for (name, r, kinds) in [
            ("categorized", cat, area()),
            ("graduated", grad, area()),
            ("rules", rules, area()),
            (
                "marker",
                marker,
                Kinds {
                    point: true,
                    line: true,
                    area: false,
                },
            ),
        ] {
            let label = json!({ "placement": "inside", "size": 12, "template": "{ad}", "weight": 600, "halo": { "width": 2, "color": "#ffffff" } });
            let l = LayerIn {
                id: "parsel",
                name: "Parsel",
                renderer: &r,
                kinds,
                opacity: 1.0,
                source: "parsel",
                source_layer: None,
                label: Some(&label),
                labels: None,
            };
            let w = write(
                "Deneme",
                &[l],
                &sources(),
                Some("https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf"),
                &none,
            );
            std::fs::write(format!("{dir}/{name}.json"), w.json).expect("written");
        }
    }
}
