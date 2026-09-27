//! The number fields of the designer's forms (the web's `numberInput`
//! calls in `layerForms.ts`): each field's label, unit, step and bounds, the
//! value it shows and the patch it writes. The forms place the fields; ↑ and
//! ↓ step the one holding the keyboard by the same table.

use kentos_native_style::classify::{js_number, js_round, rounded};
use kentos_native_style::designer::{Patch, js_value, set, type_of};
use serde_json::{Value, json};

/// What stands after a number: the layer's unit, a fixed one, or none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Layer,
    Fixed(&'static str),
    None,
}

/// A number field.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spec {
    pub label: &'static str,
    pub unit: Unit,
    /// ↑ and ↓ change it by this (×10 with Shift).
    pub step: f64,
    pub min: f64,
    pub max: f64,
    /// Shown when the layer has no value.
    pub default: f64,
}

const INF: f64 = f64::INFINITY;

const fn spec(label: &'static str, unit: Unit, step: f64, min: f64, max: f64, default: f64) -> Spec {
    Spec {
        label,
        unit,
        step,
        min,
        max,
        default,
    }
}

/// A length in the layer's unit, any sign.
const fn length(label: &'static str, default: f64) -> Spec {
    spec(label, Unit::Layer, 0.1, -INF, INF, default)
}

/// A length in the layer's unit, at least `min`.
const fn size(label: &'static str, min: f64, default: f64) -> Spec {
    spec(label, Unit::Layer, 0.1, min, INF, default)
}

const fn degrees(label: &'static str, default: f64) -> Spec {
    spec(label, Unit::Fixed("°"), 5.0, -INF, INF, default)
}

/// The number field `key` of a layer, as its form shows it.
pub fn spec_of(layer: &Value, key: &str) -> Option<Spec> {
    let t = type_of(layer);
    let marker = matches!(t, "shape" | "svg" | "raster" | "text");
    Some(match (t, key) {
        (_, "transparency") => spec("Saydamlık", Unit::Fixed("%"), 5.0, 0.0, 100.0, 0.0),
        ("hatchFill", "angle") => degrees("Açı", 0.0),
        ("hatchFill", "spacing") => size("Aralık", 0.01, 0.0),
        ("hatchFill", "width") => size("Kalınlık", 0.0, 0.0),
        ("hatchFill", "offset") => length("Kaydırma", 0.0),
        ("hatchFill" | "simpleLine", "dashOffset") => length("Kesik kaydırma", 0.0),
        ("patternFill", "spacingX") => size("Aralık Y (sağa)", 0.01, 0.0),
        ("patternFill", "spacingY") => size("Aralık X (yukarı)", 0.01, 0.0),
        ("patternFill" | "imageFill", "angle") => degrees("Açı", 0.0),
        ("patternFill", "offsetX") => length("Kaydırma Y", 0.0),
        ("patternFill", "offsetY") => length("Kaydırma X", 0.0),
        ("patternFill", "jitterPct") => spec("Dağınıklık", Unit::Fixed("%"), 5.0, 0.0, 100.0, 0.0),
        ("patternFill", "coveragePct") => {
            spec("Doluluk", Unit::Fixed("%"), 5.0, 0.0, 100.0, 100.0)
        }
        ("patternFill", "seed") => spec("Rastgele tohum", Unit::None, 1.0, -INF, INF, 0.0),
        ("imageFill", "tileSize") => size("Döşeme genişliği", 0.01, 0.0),
        ("simpleLine", "width") => size("Kalınlık", 0.0, 0.25),
        ("simpleLine" | "markerLine", "offset") => length("Kaydırma", 0.0),
        ("simpleLine", "blur") => size("Yumuşatma", 0.0, 0.0),
        ("simpleLine", "shiftX") => length("Gölge sağa", 0.0),
        ("simpleLine", "shiftY") => length("Gölge yukarı", 0.0),
        ("simpleLine", "wave.length") => size("Dalga boyu", 0.01, 5.0),
        ("simpleLine", "wave.amplitude") => size("Genlik", 0.0, 0.8),
        ("simpleLine", "wave.spacing") => size("Tekrar", 0.01, 5.0),
        ("simpleLine", "wave.offsetAlong") => size("İlk dalga", 0.0, 0.0),
        ("markerLine", "interval") => size("Aralık", 0.01, 6.0),
        ("markerLine", "offsetAlong") => length("İlk uzaklık", 0.0),
        ("markerLine", "groupCount") => spec("Grup", Unit::Fixed("adet"), 1.0, 1.0, 20.0, 1.0),
        ("markerLine", "groupSpacing") => size("Grup aralığı", 0.0, 1.0),
        ("shape", "size") if layer.get("shape").and_then(Value::as_str) == Some("rectangle") => {
            size("Genişlik", 0.0, 3.0)
        }
        ("shape", "size") => size("Boyut", 0.0, 3.0),
        ("shape", "height") => size("Yükseklik", 0.0, 3.0),
        ("shape", "strokeWidth") => size("Çizgi kalınlığı", 0.0, 0.2),
        ("shape", "teeth") => spec("Diş sayısı", Unit::Fixed("adet"), 1.0, 3.0, 64.0, 12.0),
        ("shape", "teethDepthPct") => {
            spec("Diş derinliği", Unit::Fixed("%"), 1.0, 2.0, 60.0, 20.0)
        }
        ("shape", "sweep") => spec("Açıklık", Unit::Fixed("°"), 5.0, 1.0, 360.0, 180.0),
        ("shape", "holePct") => spec("Delik", Unit::Fixed("%"), 5.0, 0.0, 95.0, 0.0),
        ("svg" | "raster", "size") => size("Genişlik", 0.0, 5.0),
        ("text", "size") => size("Harf yüksekliği", 0.0, 3.0),
        ("text", "haloWidth") => size("Hale kalınlığı", 0.0, 0.3),
        (_, "rotation") if marker => degrees("Döndürme", 0.0),
        (_, "offsetX") if marker => length("Kaydırma Y", 0.0),
        (_, "offsetY") if marker => length("Kaydırma X", 0.0),
        _ => return None,
    })
}

/// Every number field a layer's form may show, for ↑ and ↓ to find the one with the keyboard.
pub const KEYS: [&str; 34] = [
    "transparency",
    "angle",
    "spacing",
    "width",
    "offset",
    "dashOffset",
    "spacingX",
    "spacingY",
    "offsetX",
    "offsetY",
    "jitterPct",
    "coveragePct",
    "seed",
    "tileSize",
    "blur",
    "shiftX",
    "shiftY",
    "wave.length",
    "wave.amplitude",
    "wave.spacing",
    "wave.offsetAlong",
    "interval",
    "offsetAlong",
    "groupCount",
    "groupSpacing",
    "size",
    "height",
    "strokeWidth",
    "teeth",
    "teethDepthPct",
    "sweep",
    "holePct",
    "haloWidth",
    "rotation",
];

fn number(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64)
}

fn element(v: Option<&Value>, i: usize) -> Option<f64> {
    v.and_then(Value::as_array)
        .and_then(|a| a.get(i))
        .and_then(Value::as_f64)
}

/// The value a field shows; None when the layer gives none (its default is shown)
/// or when it comes from an expression.
pub fn read(layer: &Value, key: &str) -> Option<f64> {
    let g = |k: &str| layer.get(k);
    match key {
        "transparency" => Some(js_round(
            (1.0 - number(g("opacity")).unwrap_or(1.0)) * 100.0,
        )),
        "offsetX" => element(g("offset"), 0),
        "offsetY" => element(g("offset"), 1),
        "shiftX" => element(g("shift"), 0),
        "shiftY" => element(g("shift"), 1),
        "jitterPct" => number(g("jitter")).map(|v| js_round(v * 100.0)),
        "coveragePct" => number(g("coverage")).map(|v| js_round(v * 100.0)),
        "holePct" => number(g("hole")).map(|v| js_round(v * 100.0)),
        "teethDepthPct" => number(g("teethDepth")).map(|v| js_round(v * 100.0)),
        "groupCount" => g("group").and_then(|gr| number(gr.get("count"))),
        "groupSpacing" => g("group").and_then(|gr| number(gr.get("spacing"))),
        "haloWidth" => g("halo").and_then(|h| number(h.get("width"))),
        "wave.spacing" => g("wave").and_then(|w| number(w.get("spacing")).or(number(w.get("length")))),
        k if k.starts_with("wave.") => g("wave").and_then(|w| number(w.get(&k[5..]))),
        // A rectangle's height follows its width until it is given.
        "height" => number(g("height")).or(number(g("size"))),
        k => number(g(k)),
    }
}

/// The value a field shows, its default when the layer gives none.
pub fn shown(layer: &Value, key: &str) -> f64 {
    read(layer, key)
        .or_else(|| spec_of(layer, key).map(|s| s.default))
        .unwrap_or(0.0)
}

/// A number as the field writes it: six decimals at most (`fmt`).
pub fn text_of(v: f64) -> String {
    rounded(v, 6)
}

/// What the field's text reads as (`parseNum`): a comma for the point, and
/// nothing for a blank field (the web's `Number("")` wrote 0 while a field
/// was being cleared).
pub fn parse(text: &str) -> Option<f64> {
    let t = kentos_processing::text::js_trim(text);
    if t.is_empty() {
        return None;
    }
    let v = js_number(&t.replacen(',', ".", 1));
    v.is_finite().then_some(v)
}

/// A value kept inside a field's bounds.
pub fn clamp(spec: &Spec, v: f64) -> f64 {
    v.max(spec.min).min(spec.max)
}

/// The patch a field writes with value `v` (already inside its bounds).
pub fn write(layer: &Value, key: &str, v: f64) -> Patch {
    match key {
        "transparency" => set("opacity", js_value(1.0 - v.clamp(0.0, 100.0) / 100.0)),
        k if k.starts_with("wave.") => {
            let mut wave = layer.get("wave").cloned().unwrap_or_else(|| json!({}));
            if let Some(o) = wave.as_object_mut() {
                o.insert(k[5..].to_owned(), js_value(v));
            }
            set("wave", wave)
        }
        k => set(k, js_value(v)),
    }
}

/// ↑ or ↓ on a field whose text is `text` (`numberInput`'s keys): a step up
/// or down from what the text reads (0 when nothing), ten with Shift, kept
/// inside the bounds.
pub fn stepped(spec: &Spec, text: &str, up: bool, shift: bool) -> f64 {
    let step = spec.step * if shift { 10.0 } else { 1.0 };
    let from = parse(text).unwrap_or(0.0);
    let to = from + if up { step } else { -step };
    clamp(spec, js_round(to * 1e6) / 1e6)
}
