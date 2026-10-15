//! A layer's simple look as symbols (the web's `style/fromLayer.ts`): its
//! colour, line type, weight, fill and point symbol, so layers without a
//! renderer go through the same drawing path as styled ones. Line types are
//! paper millimetres like CAD linetypes; point symbols stay in screen pixels.
//! A hatch object carries its own pattern, drawn as a fill symbol.
//!
//! The symbols are JSON, as the style core reads them; `fixtures/style/v1/
//! batches.json` holds both platforms to the same sets.

use kentos_contracts::{
    GradientShape, HatchEntity, HatchPattern, HatchPatternType, LayerStyle, LineType, PointSymbol,
};
use kentos_geometry_core::entity::HatchPattern as CorePattern;
use kentos_geometry_core::geom::hatch_pattern::{FamilyPaint, PatternLine, paints};
use serde_json::{Value, json};

/// Dash patterns of the layer line types, paper mm (`LINE_TYPE_DASH`).
pub fn line_type_dash(line_type: LineType) -> Option<&'static [f64]> {
    match line_type {
        LineType::Continuous => None,
        LineType::Dashed => Some(&[3.0, 1.5]),
        LineType::Dashdot => Some(&[5.0, 1.2, 0.6, 1.2]),
        LineType::Dotted => Some(&[0.6, 1.2]),
    }
}

fn point_shape(symbol: PointSymbol) -> &'static str {
    match symbol {
        PointSymbol::Ring => "ring",
        PointSymbol::Cross => "cross",
        PointSymbol::Triangle => "triangle",
    }
}

/// The simple line layer (`lineSymbolOf`'s only layer), with its id.
fn line_layer(id: &str, color: &str, line_type: LineType, weight: f64) -> Value {
    json!({
        "id": id,
        "type": "simpleLine",
        "color": color,
        "width": weight,
        "dash": line_type_dash(line_type),
        "unit": "mm",
        "cap": "butt",
        "join": "miter",
    })
}

/// A leader's look (docs/adr/0146 §5, the web's `leaderSymbolsOf`): its
/// lines as its layer's simple line in its own colour and weight, its
/// filled arrowhead or dot solid in that colour.
pub fn leader_symbols_of(style: &LayerStyle, color: &str, weight: f64, hairlines: bool) -> Value {
    let weight = if hairlines { 0.0 } else { weight };
    json!({
        "line": line_symbol_of(color, style.line_type, weight),
        "fill": { "type": "fill", "layers": [{ "id": "s", "type": "simpleFill", "color": color }] },
    })
}

/// A line symbol of a colour, line type and weight in mm (`lineSymbolOf`).
pub fn line_symbol_of(color: &str, line_type: LineType, weight: f64) -> Value {
    json!({ "type": "line", "layers": [line_layer("l", color, line_type, weight)] })
}

/// The symbols of a layer's simple look (`symbolsOfLayerStyle`); `color` is
/// the object's own colour when it has one, `weight` its own line weight
/// (mm; docs/adr/0139). `hairlines`: line weights hidden (Kalınlık off),
/// every line one pixel.
pub fn symbols_of_layer_style(
    style: &LayerStyle,
    color: &str,
    weight: f64,
    hairlines: bool,
) -> Value {
    let weight = if hairlines { 0.0 } else { weight };
    let line = line_symbol_of(color, style.line_type, weight);
    let mut fill_layers = Vec::with_capacity(2);
    if let Some(fill) = &style.fill {
        fill_layers.push(json!({ "id": "f", "type": "simpleFill", "color": fill }));
    }
    fill_layers.push(line_layer("o", color, style.line_type, weight));
    let (symbol, size) = style
        .point
        .as_ref()
        .map_or((PointSymbol::Ring, 7.0), |p| (p.symbol, p.size));
    let marker = json!({
        "type": "marker",
        "layers": [{
            "id": "p",
            "type": "shape",
            "shape": point_shape(symbol),
            "size": size,
            "unit": "px",
            "stroke": color,
            "strokeWidth": 1.3,
            "fill": null,
        }],
    });
    json!({
        "line": line,
        "fill": { "type": "fill", "layers": fill_layers },
        "marker": marker,
    })
}

/// An area's mask (docs/adr/0310 §2): its layer's symbols, but its fill the drawing's ground (`paper`, opaque) and its
/// outline only when its frame is drawn, continuous in its own colour and weight.
pub fn symbols_of_wipeout(
    style: &LayerStyle,
    color: &str,
    weight: f64,
    hairlines: bool,
    frame: bool,
) -> Value {
    let mut symbols = symbols_of_layer_style(style, color, weight, hairlines);
    let weight = if hairlines { 0.0 } else { weight };
    let mut layers = vec![json!({ "id": "w", "type": "simpleFill", "color": "paper" })];
    if frame {
        layers.push(line_layer("o", color, LineType::Continuous, weight));
    }
    symbols["fill"] = json!({ "type": "fill", "layers": layers });
    symbols
}

/// An object's own stroke (docs/adr/0260 §2, §3): its line type over its
/// layer's, its pattern's scale and its opacity (1 − its transparency).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    pub line_type: Option<LineType>,
    pub scale: f64,
    pub opacity: f64,
}

impl Stroke {
    /// An object's stroke from its look.
    pub fn of(look: &kentos_contracts::ObjectLook) -> Self {
        Self {
            line_type: look.line_type,
            scale: look.line_type_scale.unwrap_or(1.0),
            opacity: 1.0 - look.transparency.unwrap_or(0.0),
        }
    }

    /// Whether it changes nothing of its layer's look.
    pub fn plain(&self) -> bool {
        self.line_type.is_none() && self.scale == 1.0 && self.opacity == 1.0
    }

    /// Its parts as a key: line type, scale and opacity bits.
    pub fn key(&self) -> (u8, u64, u64) {
        (
            self.line_type.map_or(0, |t| 1 + t as u8),
            self.scale.to_bits(),
            self.opacity.to_bits(),
        )
    }

    /// `symbols` (a set of symbols, or one symbol) with this stroke: every
    /// layer's dash scaled, its opacity set. A plain stroke leaves them as they are.
    pub fn apply(&self, mut symbols: Value) -> Value {
        if self.scale == 1.0 && self.opacity == 1.0 {
            return symbols;
        }
        fn walk(v: &mut Value, s: &Stroke) {
            match v {
                Value::Object(map) => {
                    if map.contains_key("type") && map.contains_key("id") {
                        if let Some(Value::Array(dash)) = map.get_mut("dash")
                            && s.scale != 1.0
                        {
                            for d in dash.iter_mut() {
                                if let Some(x) = d.as_f64() {
                                    *d = json!(x * s.scale);
                                }
                            }
                        }
                        if s.opacity != 1.0 {
                            let own = map.get("opacity").and_then(Value::as_f64).unwrap_or(1.0);
                            map.insert("opacity".into(), json!(own * s.opacity));
                        }
                    }
                    for (_, x) in map.iter_mut() {
                        walk(x, s);
                    }
                }
                Value::Array(list) => list.iter_mut().for_each(|x| walk(x, s)),
                _ => {}
            }
        }
        walk(&mut symbols, self);
        symbols
    }
}

/// An object's simple look (docs/adr/0260): its layer's symbols in its own
/// colour, weight, line type, pattern scale and opacity. A plain stroke gives
/// exactly [`symbols_of_layer_style`]'s symbols.
pub fn symbols_of_object(
    style: &LayerStyle,
    color: &str,
    weight: f64,
    hairlines: bool,
    stroke: Stroke,
) -> Value {
    let symbols = match stroke.line_type {
        Some(t) if t != style.line_type => {
            let mut own = style.clone();
            own.line_type = t;
            symbols_of_layer_style(&own, color, weight, hairlines)
        }
        _ => symbols_of_layer_style(style, color, weight, hairlines),
    };
    stroke.apply(symbols)
}

/// [`symbols_of_object`] with the project's line types (docs/adr/0313 §3): the object's own built-in type over its
/// layer's, then `pattern` (the one it draws with, decided by its caller) and the project's scale, then its stroke's
/// scale and opacity.
pub fn symbols_of_object_in(
    style: &LayerStyle,
    color: &str,
    weight: f64,
    hairlines: bool,
    stroke: Stroke,
    pattern: Option<&str>,
    lines: &crate::line_types::LineTypes,
) -> Value {
    let symbols = match stroke.line_type {
        Some(t) if t != style.line_type => {
            let mut own = style.clone();
            own.line_type = t;
            symbols_of_layer_style(&own, color, weight, hairlines)
        }
        _ => symbols_of_layer_style(style, color, weight, hairlines),
    };
    stroke.apply(lines.apply(pattern, symbols))
}

/// A hatch object's own pattern (`hatchSymbolOf`): solid at 45 %, a
/// gradient (docs/adr/0186 §3), or one hatch fill a family, as the geometry
/// core gives the families (spacing in metres; a user-defined pattern's
/// lines as they always were).
pub fn hatch_symbol_of(hatch: &HatchEntity, color: &str) -> Value {
    let p = &hatch.pattern;
    match p.kind {
        HatchPatternType::Solid => json!({
            "type": "fill",
            "layers": [{ "id": "s", "type": "simpleFill", "color": color, "opacity": 0.45 }],
        }),
        HatchPatternType::Gradient => {
            let g = p.gradient.as_ref();
            let shape = match g.map(|g| g.shape) {
                Some(GradientShape::Cylinder) => "cylinder",
                Some(GradientShape::Spherical) => "spherical",
                _ => "linear",
            };
            json!({
                "type": "fill",
                "layers": [{
                    "id": "g",
                    "type": "gradientFill",
                    "color": color,
                    "color2": g.map_or("#FFFFFF", |g| g.color2.as_str()),
                    "shape": shape,
                    "angle": p.angle,
                    "inverted": g.is_some_and(|g| g.inverted),
                }],
            })
        }
        _ => {
            let layers: Vec<Value> = paints(&core_pattern(p))
                .iter()
                .enumerate()
                .map(|(i, f)| family_layer(&format!("h{}", i + 1), f, color))
                .collect();
            json!({ "type": "fill", "layers": layers })
        }
    }
}

/// A family as a hatch fill layer: its phases, dashes and stagger only when it has them.
fn family_layer(id: &str, f: &FamilyPaint, color: &str) -> Value {
    let mut layer = json!({
        "id": id,
        "type": "hatchFill",
        "angle": f.angle,
        "spacing": f.spacing,
        "width": 0,
        "color": color,
        "unit": "m",
    });
    if let Some(o) = layer.as_object_mut() {
        if f.offset != 0.0 {
            o.insert("offset".into(), json!(f.offset));
        }
        if let Some(dash) = &f.dash {
            o.insert("dash".into(), json!(dash));
            o.insert("dashOffset".into(), json!(f.dash_offset));
        }
        if f.stagger != 0.0 {
            o.insert("stagger".into(), json!(f.stagger));
        }
    }
    layer
}

/// A hatch's pattern as the geometry core holds it (its families too).
fn core_pattern(p: &HatchPattern) -> CorePattern {
    let kind = match p.kind {
        HatchPatternType::Solid => "solid",
        HatchPatternType::Lines => "lines",
        HatchPatternType::Cross => "cross",
        HatchPatternType::Pattern => "pattern",
        HatchPatternType::Gradient => "gradient",
    };
    CorePattern {
        name: p.name.clone(),
        scale: p.scale,
        lines: p.lines.as_ref().map(|ls| {
            ls.iter()
                .map(|l| PatternLine {
                    angle: l.angle,
                    origin: l.origin,
                    offset: l.offset,
                    dashes: (!l.dashes.is_empty()).then(|| l.dashes.clone()),
                })
                .collect()
        }),
        ..CorePattern::user(kind, p.angle, p.spacing)
    }
}
