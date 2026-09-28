//! A layer's simple look as symbols (the web's `style/fromLayer.ts`): its
//! colour, line type, weight, fill and point symbol, so layers without a
//! renderer go through the same drawing path as styled ones. Line types are
//! paper millimetres like CAD linetypes; point symbols stay in screen pixels.
//! A hatch object carries its own pattern, drawn as a fill symbol.
//!
//! The symbols are JSON, as the style core reads them; `fixtures/style/v1/
//! batches.json` holds both platforms to the same sets.

use kentos_contracts::{HatchEntity, HatchPatternType, LayerStyle, LineType, PointSymbol};
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

/// A hatch object's own pattern, solid or lines (spacing in metres) (`hatchSymbolOf`).
pub fn hatch_symbol_of(hatch: &HatchEntity, color: &str) -> Value {
    let p = &hatch.pattern;
    let lines = |id: &str, angle: f64| {
        json!({
            "id": id,
            "type": "hatchFill",
            "angle": angle,
            "spacing": p.spacing,
            "width": 0,
            "color": color,
            "unit": "m",
        })
    };
    match p.kind {
        HatchPatternType::Solid => json!({
            "type": "fill",
            "layers": [{ "id": "s", "type": "simpleFill", "color": color, "opacity": 0.45 }],
        }),
        HatchPatternType::Lines => json!({ "type": "fill", "layers": [lines("h1", p.angle)] }),
        HatchPatternType::Cross => json!({
            "type": "fill",
            "layers": [lines("h1", p.angle), lines("h2", p.angle + 90.0)],
        }),
    }
}
