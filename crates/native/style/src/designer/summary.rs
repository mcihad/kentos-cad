//! A layer's one-line summary in the designer's list (the web's `summary`).

use kentos_style_core::js::number;
use serde_json::Value;

use super::patch::{js_text, truthy};
use super::{PLACEMENTS, SHAPES, choice_label, type_of};
use crate::classify::rounded;

// ── Summaries ──────────────────────────────────────────────────────────

/// A number to two decimals as the list writes it; “ƒ” for one from an expression.
fn num(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => rounded(n.as_f64().unwrap_or(f64::NAN), 2),
        _ => "ƒ".into(),
    }
}

fn unit_of(layer: &Value) -> &'static str {
    match layer.get("unit").and_then(Value::as_str) {
        Some("px") => "px",
        Some("m") => "m",
        _ => "mm",
    }
}

/// The suffix a count takes as Turkish reads it: 2'li, 3'lü, 6'lı, 9'lu, 10'lu, 20'li (`countSuffix`).
pub fn count_suffix(n: f64) -> &'static str {
    const ONES: [&str; 10] = ["", "li", "li", "lü", "lü", "li", "lı", "li", "li", "lu"];
    const TENS: [&str; 10] = ["lü", "lu", "li", "lu", "lı", "li", "lı", "li", "li", "lı"];
    let k = n.abs().trunc() as u64;
    if !k.is_multiple_of(10) {
        return ONES[(k % 10) as usize];
    }
    if !k.is_multiple_of(100) {
        return TENS[((k % 100) / 10) as usize];
    }
    if !k.is_multiple_of(1000) { "lü" } else { "li" }
}

/// A layer's line in the list: its colour, size or placement at a glance (`summary`).
pub fn summary(layer: &Value) -> String {
    let g = |k: &str| layer.get(k);
    let u = unit_of(layer);
    match type_of(layer) {
        "simpleFill" => match g("color") {
            Some(Value::String(c)) => c.clone(),
            _ => "ifadeden renk".into(),
        },
        "hatchFill" => format!(
            "{}° · {} {u} aralık",
            js_text(g("angle")),
            num(g("spacing"))
        ),
        "patternFill" => format!(
            "{} × {} {u}{}{}",
            num(g("spacingX")),
            num(g("spacingY")),
            if truthy(g("stagger")) {
                ", şaşırtmalı"
            } else {
                ""
            },
            if truthy(g("jitter")) {
                ", dağınık"
            } else {
                ""
            },
        ),
        "imageFill" => {
            if truthy(g("asset")) {
                format!("{} {u} döşeme", num(g("tileSize")))
            } else {
                "çizim seçilmedi".into()
            }
        }
        "centroidMarker" => {
            if g("position").and_then(Value::as_str) == Some("centroid") {
                "ağırlık merkezinde".into()
            } else {
                "alanın içinde".into()
            }
        }
        "simpleLine" => {
            let dashed = g("dash")
                .and_then(Value::as_array)
                .is_some_and(|d| !d.is_empty());
            let offset = match g("offset") {
                o if !truthy(o) => String::new(),
                Some(Value::Number(_)) => format!(", {} {u} kaydırılmış", num(g("offset"))),
                _ => ", ifadeyle kaydırılmış".into(),
            };
            format!(
                "{} {u}{}{offset}{}",
                num(g("width")),
                if dashed { ", kesikli" } else { "" },
                if truthy(g("wave")) { ", dalgalı" } else { "" },
            )
        }
        "markerLine" => {
            let placement = g("placement").and_then(Value::as_str).unwrap_or("");
            if placement == "interval" {
                let count = g("group")
                    .filter(|gr| truthy(Some(gr)))
                    .and_then(|gr| gr.get("count"))
                    .and_then(Value::as_f64)
                    .filter(|c| *c > 1.0);
                let group = count.map_or_else(String::new, |c| {
                    format!(", {}'{}", number::to_string(c), count_suffix(c))
                });
                format!("her {} {u}{group}", num(g("interval")))
            } else {
                choice_label(&PLACEMENTS, placement)
                    .unwrap_or("")
                    .to_owned()
            }
        }
        "shape" => {
            let shape = g("shape").and_then(Value::as_str);
            let name = shape
                .and_then(|s| choice_label(&SHAPES, s))
                .map_or_else(|| js_text(g("shape")), str::to_owned);
            format!("{name} · {} {u}", num(g("size")))
        }
        "svg" | "raster" => {
            if truthy(g("asset")) {
                format!("{} {u}", num(g("size")))
            } else {
                "çizim seçilmedi".into()
            }
        }
        "text" => match g("text") {
            Some(Value::String(t)) => format!("“{t}”"),
            _ => "öznitelikten".into(),
        },
        _ => String::new(),
    }
}
