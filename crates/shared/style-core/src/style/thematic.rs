//! The value rules of the thematic renderers (docs/adr/0213 §2): a ramp's
//! colour at a share, a value's share of a range, a size from it, a value's
//! class among breaks, and a symbol with its main colour or its size
//! changed (Sürekli renk, İki değişkenli renk, Orantılı sembol).

use kentos_geometry_core::jsmath::{js_max, js_min, js_round, pow};

use super::compile::MM_PER_PX;
use super::model::{
    Dd, Layer, MarkerKind, MarkerLayer, Rgba, SimpleLine, Symbol, SymbolType, Unit,
};
use super::place::positive;

/// `#RRGGBB` or `#RRGGBBAA`.
pub fn parse_rgba(s: &str) -> Option<Rgba> {
    let hex = s.trim().strip_prefix('#')?;
    if !(hex.len() == 6 || hex.len() == 8) || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok();
    Some([
        byte(0)?,
        byte(2)?,
        byte(4)?,
        if hex.len() == 8 { byte(6)? } else { 255 },
    ])
}

/// `#RRGGBB` (opaque) or `#RRGGBBAA`.
pub fn hex(c: Rgba) -> String {
    if c[3] == 255 {
        format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
    } else {
        format!("#{:02X}{:02X}{:02X}{:02X}", c[0], c[1], c[2], c[3])
    }
}

/// The ramp's colour at `t` (0–1): equally spaced stops, each channel mixed
/// straight between the two stops around `t` and rounded half up (the
/// window's `rampColors`). An empty ramp is black; one stop is itself.
pub fn ramp_color(stops: &[Rgba], t: f64) -> Rgba {
    match stops {
        [] => [0, 0, 0, 255],
        [one] => *one,
        _ => {
            let t = if t.is_nan() {
                0.0
            } else {
                js_min(1.0, js_max(0.0, t))
            };
            let at = t * (stops.len() - 1) as f64;
            let k = (at.floor() as usize).min(stops.len() - 2);
            let f = at - k as f64;
            let (a, b) = (stops[k], stops[k + 1]);
            let mix = |i: usize| {
                let v = f64::from(a[i]) + (f64::from(b[i]) - f64::from(a[i])) * f;
                js_round(v).clamp(0.0, 255.0) as u8
            };
            [mix(0), mix(1), mix(2), mix(3)]
        }
    }
}

/// A value's share of `[min, max]`, clipped to 0–1; 0 when the range is empty.
pub fn share(v: f64, min: f64, max: f64) -> f64 {
    if !positive(max - min) {
        return 0.0;
    }
    js_min(1.0, js_max(0.0, (v - min) / (max - min)))
}

/// The share's step of 256 (`round(255 t)`): the colours and sizes a layer draws with.
pub fn step(t: f64) -> u8 {
    js_round(255.0 * js_min(1.0, js_max(0.0, t))) as u8
}

/// Orantılı sembol's size at share `t`: `min + (max − min) · tᵉ` (QGIS's size assistant).
pub fn size_at(t: f64, min: f64, max: f64, exponent: f64) -> f64 {
    let t = js_min(1.0, js_max(0.0, t));
    min + (max - min) * if exponent == 1.0 { t } else { pow(t, exponent) }
}

/// A value's class among ascending breaks: how many breaks it reaches (`v ≥ b`).
pub fn class_of(v: f64, breaks: &[f64]) -> usize {
    breaks.iter().filter(|&&b| v >= b).count()
}

/// A shape drawn by its outline (its colour is its stroke's): the shader's `isOpen`.
pub fn open_shape(shape: &str) -> bool {
    matches!(shape, "cross" | "x" | "line" | "arrow" | "chevron" | "arc")
}

fn recolor_marker(m: &mut MarkerLayer, c: &str) {
    match &mut m.kind {
        MarkerKind::Shape {
            shape,
            fill,
            stroke,
            ..
        } => {
            if open_shape(shape) {
                *stroke = Some(Dd::Fixed(c.to_owned()));
            } else {
                *fill = Some(Dd::Fixed(c.to_owned()));
            }
        }
        MarkerKind::Svg { fill, .. } => *fill = Some(Dd::Fixed(c.to_owned())),
        MarkerKind::Text { color, .. } => *color = Some(Dd::Fixed(c.to_owned())),
        MarkerKind::Raster { .. } => {}
    }
}

fn recolor_markers(layers: &mut [Option<MarkerLayer>], c: &str) {
    for m in layers.iter_mut().flatten() {
        recolor_marker(m, c);
    }
}

/// The symbol with its **main colour** `c` (docs/adr/0213 §2.10): a fill
/// symbol's fills (not its edges), a line symbol's lines and the marks along
/// them, a marker symbol's shapes, drawings and texts.
pub fn with_color(symbol: &Symbol, c: &str) -> Symbol {
    let mut s = symbol.clone();
    for layer in &mut s.layers {
        match (symbol.kind, layer) {
            (SymbolType::Fill, Layer::SimpleFill { color, .. }) => {
                *color = Some(Dd::Fixed(c.to_owned()))
            }
            (SymbolType::Fill, Layer::HatchFill(h)) => h.color = Some(Dd::Fixed(c.to_owned())),
            (SymbolType::Fill, Layer::GradientFill(g)) => g.color = Some(Dd::Fixed(c.to_owned())),
            (SymbolType::Fill, Layer::PatternFill(p)) => recolor_markers(&mut p.marker, c),
            (SymbolType::Fill, Layer::CentroidMarker { marker, .. }) => recolor_markers(marker, c),
            (SymbolType::Line, Layer::SimpleLine(l)) => l.color = Some(Dd::Fixed(c.to_owned())),
            (SymbolType::Line, Layer::MarkerLine(l)) => recolor_markers(&mut l.marker, c),
            (SymbolType::Marker, Layer::Marker(m)) => recolor_marker(m, c),
            _ => {}
        }
    }
    s
}

/// A length in `from` as `to` (paper mm and screen px; metres stay as they are).
fn convert(v: f64, from: Unit, to: Unit) -> f64 {
    match (from, to) {
        (Unit::Mm, Unit::Px) => v / MM_PER_PX,
        (Unit::Px, Unit::Mm) => v * MM_PER_PX,
        _ => v,
    }
}

fn fixed_or(v: &Option<Dd<f64>>, default: f64) -> f64 {
    match v {
        Some(Dd::Fixed(x)) => *x,
        Some(Dd::Expr { fallback, .. }) => fallback.unwrap_or(default),
        None => default,
    }
}

/// A marker symbol's size in `unit`: its largest layer's (a layer without a size draws 2).
pub fn marker_size(layers: &[&MarkerLayer], unit: Unit) -> f64 {
    layers
        .iter()
        .map(|m| convert(fixed_or(&m.size, 2.0), m.base.unit, unit))
        .fold(0.0, js_max)
}

/// A line symbol's width in `unit`: its widest line's.
pub fn line_width(symbol: &Symbol, unit: Unit) -> f64 {
    symbol
        .layers
        .iter()
        .filter_map(|l| match l {
            Layer::SimpleLine(l) => Some(convert(fixed_or(&l.width, 0.0), l.base.unit, unit)),
            _ => None,
        })
        .fold(0.0, js_max)
}

fn scale_marker(m: &mut MarkerLayer, k: f64) {
    m.size = Some(Dd::Fixed(fixed_or(&m.size, 2.0) * k));
    if let MarkerKind::Shape {
        height: Some(h), ..
    } = &mut m.kind
    {
        *h *= k;
    }
    if let Some(o) = &mut m.offset {
        *o = [o[0] * k, o[1] * k];
    }
}

/// The symbol scaled so its size is `size` in `unit` (QGIS's `setSize` and
/// `setWidth`): a marker symbol's layers grow with their offsets, a line
/// symbol's widths and offsets (and the marks along it); strokes of shapes
/// keep their widths. A symbol without a size is returned as it is.
pub fn with_size(symbol: &Symbol, size: f64, unit: Unit) -> Symbol {
    let mut s = symbol.clone();
    match symbol.kind {
        SymbolType::Marker => {
            let layers: Vec<&MarkerLayer> = symbol
                .layers
                .iter()
                .filter_map(|l| match l {
                    Layer::Marker(m) => Some(m),
                    _ => None,
                })
                .collect();
            let now = marker_size(&layers, unit);
            if !positive(now) {
                return s;
            }
            let k = size / now;
            for l in &mut s.layers {
                if let Layer::Marker(m) = l {
                    scale_marker(m, k);
                }
            }
        }
        SymbolType::Line => {
            let now = line_width(symbol, unit);
            if !positive(now) {
                return s;
            }
            let k = size / now;
            for l in &mut s.layers {
                match l {
                    Layer::SimpleLine(SimpleLine { width, offset, .. }) => {
                        *width = Some(Dd::Fixed(fixed_or(width, 0.0) * k));
                        if let Some(Dd::Fixed(o)) = offset {
                            *o *= k;
                        }
                    }
                    Layer::MarkerLine(ml) => {
                        for m in ml.marker.iter_mut().flatten() {
                            scale_marker(m, k);
                        }
                    }
                    _ => {}
                }
            }
        }
        SymbolType::Fill => {}
    }
    s
}

/// A pie's or a dot's colour as the batches take it (`#RRGGBB` or with alpha); a bad one is the drawing's ink.
pub fn color_or_ink(c: &str) -> String {
    match parse_rgba(c) {
        Some(rgba) => hex(rgba),
        None => "ink".to_owned(),
    }
}
