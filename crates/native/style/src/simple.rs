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
