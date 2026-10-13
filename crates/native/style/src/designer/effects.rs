//! A symbol layer's draw effects in the designer (the web's `designerModel.ts`,
//! docs/adr/0226 §2): their names, how a new one starts, switching one on or
//! off, replacing one, which layers take them and their line in the list.
//! The list is kept in the drawn order: shadow, outer glow, blur, inner
//! shadow, inner glow.

use serde_json::{Value, json};

use kentos_style_core::js::text::lower_tr;

use super::list::LayerPath;

/// The effects in the order they are drawn (`EFFECT_TYPES`).
pub const EFFECT_TYPES: [&str; 5] = [
    "dropShadow",
    "outerGlow",
    "blur",
    "innerShadow",
    "innerGlow",
];

/// An effect's name (`EFFECT_LABEL`).
pub fn effect_label(kind: &str) -> &'static str {
    match kind {
        "dropShadow" => "Gölge",
        "outerGlow" => "Dış parıltı",
        "blur" => "Bulanıklaştır",
        "innerShadow" => "İç gölge",
        "innerGlow" => "İç parıltı",
        _ => "",
    }
}

/// A new effect of a type, as the style core reads a missing value (`newEffect`).
pub fn new_effect(kind: &str) -> Value {
    match kind {
        "dropShadow" => {
            json!({ "type": kind, "distance": 1, "angle": 135, "blur": 1.5, "color": "#000000", "opacity": 0.5 })
        }
        "innerShadow" => {
            json!({ "type": kind, "distance": 0.5, "angle": 135, "blur": 1, "color": "#000000", "opacity": 0.5 })
        }
        "outerGlow" => {
            json!({ "type": kind, "spread": 0.5, "blur": 1.5, "color": "#FFFFFF", "opacity": 0.5 })
        }
        "innerGlow" => {
            json!({ "type": kind, "spread": 0.5, "blur": 1, "color": "#FFFFFF", "opacity": 0.5 })
        }
        _ => json!({ "type": kind, "blur": 1 }),
    }
}

fn type_of(e: &Value) -> &str {
    e.get("type").and_then(Value::as_str).unwrap_or("")
}

fn effects_of(layer: &Value) -> Vec<Value> {
    layer
        .get("effects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// A layer's effect of a type, switched on or not (`effectOf`).
pub fn effect_of<'a>(layer: &'a Value, kind: &str) -> Option<&'a Value> {
    layer
        .get("effects")
        .and_then(Value::as_array)?
        .iter()
        .find(|e| type_of(e) == kind)
}

/// The layer with its effects in their drawn order; none as no list.
fn with_effects(layer: &Value, mut effects: Vec<Value>) -> Value {
    let place = |e: &Value| {
        EFFECT_TYPES
            .iter()
            .position(|t| *t == type_of(e))
            .unwrap_or(usize::MAX)
    };
    effects.sort_by_key(place);
    let mut out = layer.clone();
    if let Some(o) = out.as_object_mut() {
        o.remove("effects");
        if !effects.is_empty() {
            o.insert("effects".into(), Value::Array(effects));
        }
    }
    out
}

/// A layer with an effect switched on (added with its defaults, or its
/// `enabled` taken off) or off (taken away) (`setEffect`).
pub fn set_effect(layer: &Value, kind: &str, on: bool) -> Value {
    let rest: Vec<Value> = effects_of(layer)
        .into_iter()
        .filter(|e| type_of(e) != kind)
        .collect();
    if !on {
        return with_effects(layer, rest);
    }
    let effect = match effect_of(layer, kind) {
        Some(had) => {
            let mut kept = had.clone();
            if let Some(o) = kept.as_object_mut() {
                o.remove("enabled");
            }
            kept
        }
        None => new_effect(kind),
    };
    let mut all = rest;
    all.push(effect);
    with_effects(layer, all)
}

/// A layer with its effect of the same type replaced (`putEffect`).
pub fn put_effect(layer: &Value, effect: Value) -> Value {
    let kind = type_of(&effect).to_owned();
    let mut all: Vec<Value> = effects_of(layer)
        .into_iter()
        .filter(|e| type_of(e) != kind)
        .collect();
    all.push(effect);
    with_effects(layer, all)
}

/// Whether a layer at `p` takes effects: a symbol's own layers (a generator's
/// or a marker's child rows draw within it) (`takesEffects`).
pub fn takes_effects(p: LayerPath) -> bool {
    matches!(p, LayerPath::Top(_))
}

/// The effects switched on, as a list row says them: “gölge, iç parıltı” (`effectsSummary`).
pub fn effects_summary(layer: &Value) -> String {
    EFFECT_TYPES
        .iter()
        .filter(|t| {
            effect_of(layer, t).is_some_and(|e| e.get("enabled") != Some(&Value::Bool(false)))
        })
        .map(|t| lower_tr(effect_label(t)))
        .collect::<Vec<_>>()
        .join(", ")
}
