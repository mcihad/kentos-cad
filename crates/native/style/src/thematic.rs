//! The thematic renderers' value rules for the window and the legend
//! (docs/adr/0213 §2; the web's `style/thematic.ts`): the style core's
//! ramps, shares, steps and sizes, and a symbol's JSON with its main colour
//! or its size changed (§2.10), as the core changes the symbol it draws. The
//! legend's rows rest on them (fixtures/style/v1/legend.json).

use kentos_style_core::js::number;
use kentos_style_core::style::thematic::{hex, parse_rgba, ramp_color};
use serde_json::{Value, json};

/// The ramp's colour at `t` as `#RRGGBB` or `#RRGGBBAA` (`rampAt`).
pub fn ramp_at(stops: &[String], t: f64) -> String {
    let s: Vec<[u8; 4]> = stops.iter().filter_map(|c| parse_rgba(c)).collect();
    if s.is_empty() {
        return "#000000".into();
    }
    hex(ramp_color(&s, t))
}

/// `String(x)`: a number in a field as the web's window writes it.
pub fn js_text(x: f64) -> String {
    number::to_string(x)
}

/// `String(Math.round(x * 100) / 100)`: a number on the legend.
pub fn legend_number(x: f64) -> String {
    number::to_string(kentos_geometry_core::jsmath::js_round(x * 100.0) / 100.0)
}

/// A class of breaks: `< b₀`, `b₀ – b₁`, `≥ bₙ`.
pub fn range_label(breaks: &[f64], i: usize) -> String {
    match (i, breaks.len()) {
        (_, 0) => String::new(),
        (0, _) => format!("< {}", legend_number(breaks[0])),
        (i, n) if i >= n => format!("≥ {}", legend_number(breaks[n - 1])),
        (i, _) => format!(
            "{} – {}",
            legend_number(breaks[i - 1]),
            legend_number(breaks[i])
        ),
    }
}

const OPEN: [&str; 6] = ["cross", "x", "line", "arrow", "chevron", "arc"];

fn kind(l: &Value) -> &str {
    l.get("type").and_then(Value::as_str).unwrap_or_default()
}

fn recolor_marker(m: &mut Value, c: &str) {
    match kind(m) {
        "shape" => {
            let open = OPEN.contains(&m.get("shape").and_then(Value::as_str).unwrap_or_default());
            m[if open { "stroke" } else { "fill" }] = json!(c);
        }
        "svg" => m["fill"] = json!(c),
        "text" => m["color"] = json!(c),
        _ => {}
    }
}

fn recolor_markers(marker: &mut Value, c: &str) {
    if let Some(layers) = marker.get_mut("layers").and_then(Value::as_array_mut) {
        for m in layers {
            recolor_marker(m, c);
        }
    }
}

/// The symbol with its main colour `c` (`withColor`).
pub fn with_color(symbol: &Value, c: &str) -> Value {
    let mut s = symbol.clone();
    let class = kind(symbol).to_owned();
    if let Some(layers) = s.get_mut("layers").and_then(Value::as_array_mut) {
        for l in layers {
            match (class.as_str(), kind(l)) {
                ("fill", "simpleFill" | "hatchFill" | "gradientFill") => l["color"] = json!(c),
                ("fill", "patternFill" | "centroidMarker") => {
                    if let Some(m) = l.get_mut("marker") {
                        recolor_markers(m, c);
                    }
                }
                ("line", "simpleLine") => l["color"] = json!(c),
                ("line", "markerLine") => {
                    if let Some(m) = l.get_mut("marker") {
                        recolor_markers(m, c);
                    }
                }
                ("marker", "shape" | "svg" | "text") => recolor_marker(l, c),
                _ => {}
            }
        }
    }
    s
}

const MM_PER_PX: f64 = 25.4 / 96.0;

fn convert(v: f64, from: Option<&str>, to: &str) -> f64 {
    match (from.unwrap_or("mm"), to) {
        ("mm", "px") => v / MM_PER_PX,
        ("px", "mm") => v * MM_PER_PX,
        _ => v,
    }
}

fn fixed_or(v: Option<&Value>, d: f64) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(d),
        Some(o @ Value::Object(_)) => o.get("fallback").and_then(Value::as_f64).unwrap_or(d),
        _ => d,
    }
}

fn is_mark(l: &Value) -> bool {
    matches!(kind(l), "shape" | "svg" | "text" | "raster")
}

/// A marker symbol's size in `unit` (its largest layer's; one without a size is 2).
pub fn marker_size(symbol: &Value, unit: &str) -> f64 {
    symbol
        .get("layers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|l| is_mark(l))
        .map(|l| {
            convert(
                fixed_or(l.get("size"), 2.0),
                l.get("unit").and_then(Value::as_str),
                unit,
            )
        })
        .fold(0.0, f64::max)
}

/// A line symbol's width in `unit`.
pub fn line_width(symbol: &Value, unit: &str) -> f64 {
    symbol
        .get("layers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|l| kind(l) == "simpleLine")
        .map(|l| {
            convert(
                fixed_or(l.get("width"), 0.0),
                l.get("unit").and_then(Value::as_str),
                unit,
            )
        })
        .fold(0.0, f64::max)
}

fn scale_mark(m: &mut Value, k: f64) {
    let size = fixed_or(m.get("size"), 2.0) * k;
    m["size"] = json!(size);
    if kind(m) == "shape"
        && let Some(h) = m.get("height").and_then(Value::as_f64)
    {
        m["height"] = json!(h * k);
    }
    if let Some([x, y]) = m.get("offset").and_then(Value::as_array).map(|o| {
        [
            o.first().and_then(Value::as_f64),
            o.get(1).and_then(Value::as_f64),
        ]
    }) && let (Some(x), Some(y)) = (x, y)
    {
        m["offset"] = json!([x * k, y * k]);
    }
}

/// The symbol scaled to `size` in `unit` (`withSize`).
pub fn with_size(symbol: &Value, size: f64, unit: &str) -> Value {
    let mut s = symbol.clone();
    match kind(symbol) {
        "marker" => {
            let now = marker_size(symbol, unit);
            if now > 0.0 {
                let k = size / now;
                for l in s
                    .get_mut("layers")
                    .and_then(Value::as_array_mut)
                    .into_iter()
                    .flatten()
                {
                    if is_mark(l) {
                        scale_mark(l, k);
                    }
                }
            }
        }
        "line" => {
            let now = line_width(symbol, unit);
            if now > 0.0 {
                let k = size / now;
                for l in s
                    .get_mut("layers")
                    .and_then(Value::as_array_mut)
                    .into_iter()
                    .flatten()
                {
                    match kind(l) {
                        "simpleLine" => {
                            let w = fixed_or(l.get("width"), 0.0) * k;
                            l["width"] = json!(w);
                            if let Some(o) = l.get("offset").and_then(Value::as_f64) {
                                l["offset"] = json!(o * k);
                            }
                        }
                        "markerLine" => {
                            for m in l
                                .get_mut("marker")
                                .and_then(|m| m.get_mut("layers"))
                                .and_then(Value::as_array_mut)
                                .into_iter()
                                .flatten()
                            {
                                scale_mark(m, k);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        _ => {}
    }
    s
}

/// İki değişkenli renk's ready-made grids by their corners (low-low, high X, high Y, high-high).
pub const BIVARIATE_SCHEMES: [(&str, &str, [&str; 4]); 3] = [
    (
        "pembeMavi",
        "Pembe – mavi",
        ["#E8E8E8", "#5AC8C8", "#BE64AC", "#3B4994"],
    ),
    (
        "yesilMor",
        "Yeşil – mor",
        ["#E8E8E8", "#73AE80", "#6C83B5", "#2A5A5B"],
    ),
    (
        "turuncuMavi",
        "Turuncu – mavi",
        ["#E8E8E8", "#E4ACAC", "#ACE4E4", "#5A5A9A"],
    ),
];

/// An n × n grid from its corners, mixed straight both ways (`bivariateColors`).
pub fn bivariate_colors(corners: &[&str; 4], n: usize) -> Vec<String> {
    let c: Vec<[u8; 4]> = corners
        .iter()
        .map(|x| parse_rgba(x).unwrap_or([0, 0, 0, 255]))
        .collect();
    let mut out = Vec::with_capacity(n * n);
    for j in 0..n {
        for i in 0..n {
            let u = if n > 1 {
                i as f64 / (n - 1) as f64
            } else {
                0.0
            };
            let w = if n > 1 {
                j as f64 / (n - 1) as f64
            } else {
                0.0
            };
            let mix = |k: usize| {
                let v = f64::from(c[0][k]) * (1.0 - u) * (1.0 - w)
                    + f64::from(c[1][k]) * u * (1.0 - w)
                    + f64::from(c[2][k]) * (1.0 - u) * w
                    + f64::from(c[3][k]) * u * w;
                kentos_geometry_core::jsmath::js_round(v) as u8
            };
            out.push(hex([mix(0), mix(1), mix(2), mix(3)]));
        }
    }
    out
}

/// Heat maps' ready-made ramps (`HEAT_RAMPS`).
pub const HEAT_RAMPS: [(&str, &str, &[&str]); 4] = [
    (
        "isi",
        "Isı",
        &[
            "#2B83BA00",
            "#2B83BA",
            "#ABDDA4",
            "#FFFFBF",
            "#FDAE61",
            "#D7191C",
        ],
    ),
    (
        "sariKirmizi",
        "Sarıdan kırmızıya",
        &["#FFF5B800", "#FFF5B8", "#FDB863", "#E66101", "#A50F15"],
    ),
    (
        "maviler",
        "Maviler",
        &["#E3EEF900", "#9ECAE1", "#4292C6", "#08306B"],
    ),
    (
        "griler",
        "Griler",
        &["#F0F0F000", "#BDBDBD", "#737373", "#252525"],
    ),
];
