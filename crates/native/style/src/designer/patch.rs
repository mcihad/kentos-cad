//! Form patches turned into layers (the web's `applyPatch`), and the
//! JavaScript values the TypeScript leaned on: whole numbers written without
//! a point, truthiness, a template's text, `Number()`.

use kentos_style_core::js::number;
use serde_json::{Value, json};

use crate::classify::js_round;

// ── JavaScript's values, where the TypeScript leaned on them ──────────

/// A number as JSON writes it: whole numbers without a point, as JavaScript's.
pub fn js_value(x: f64) -> Value {
    if !x.is_finite() {
        return Value::Null;
    }
    if x.trunc() == x && x.abs() < 9_007_199_254_740_992.0 {
        // -0 is written 0, as JSON.stringify does.
        return Value::from(x as i64);
    }
    Value::from(x)
}

/// JavaScript's truthiness of a JSON value (an absent one is false).
pub fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(_) | Value::Object(_)) => true,
    }
}

/// A value as a template string writes it (`${v}`).
pub(super) fn js_text(v: Option<&Value>) -> String {
    match v {
        None => "undefined".into(),
        Some(Value::Null) => "null".into(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => number::to_string(n.as_f64().unwrap_or(f64::NAN)),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| match x {
                Value::Null => String::new(),
                x => js_text(Some(x)),
            })
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".into(),
    }
}

/// JavaScript's `Number(v)` for what a patch holds.
fn js_number_of(v: Option<&Value>) -> f64 {
    match v {
        None => f64::NAN,
        Some(Value::Null) => 0.0,
        Some(Value::Bool(b)) => f64::from(u8::from(*b)),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
        Some(Value::String(s)) => crate::classify::js_number(s),
        Some(_) => f64::NAN,
    }
}

// ── Form patches ───────────────────────────────────────────────────────

/// A form's change: fields and their new values, None for a field that goes
/// (JavaScript's undefined), and the helper keys the forms use (`offsetX`,
/// `jitterPct` …), in the order they were set.
pub type Patch = Vec<(String, Option<Value>)>;

/// A patch of one field.
pub fn set(key: &str, value: Value) -> Patch {
    vec![(key.to_owned(), Some(value))]
}

/// Turns a form patch (with the helper keys offsetX, jitterPct …) into a layer (`applyPatch`).
pub fn apply_patch(layer: &Value, patch: &[(String, Option<Value>)]) -> Value {
    let mut p: Vec<(String, Option<Value>)> = patch.to_vec();
    let has = |p: &[(String, Option<Value>)], k: &str| p.iter().any(|(x, _)| x == k);
    let get = |p: &[(String, Option<Value>)], k: &str| -> Option<Value> {
        p.iter().find(|(x, _)| x == k).and_then(|(_, v)| v.clone())
    };
    let take = |p: &mut Vec<(String, Option<Value>)>, k: &str| p.retain(|(x, _)| x != k);
    let put = |p: &mut Vec<(String, Option<Value>)>, k: &str, v: Option<Value>| match p
        .iter_mut()
        .find(|(x, _)| x == k)
    {
        Some(slot) => slot.1 = v,
        None => p.push((k.to_owned(), v)),
    };
    let pair_of = |v: Option<&Value>| -> [Option<Value>; 2] {
        match v.and_then(Value::as_array) {
            Some(a) => [a.first().cloned(), a.get(1).cloned()],
            None => [Some(json!(0)), Some(json!(0))],
        }
    };
    // `a ?? b`: b when a is absent or null.
    let or = |a: Option<Value>, b: Option<Value>| a.filter(|v| !v.is_null()).or(b);
    if has(&p, "shiftX") || has(&p, "shiftY") {
        let cs = pair_of(layer.get("shift"));
        let x = js_number_of(or(get(&p, "shiftX"), cs[0].clone()).as_ref());
        let y = js_number_of(or(get(&p, "shiftY"), cs[1].clone()).as_ref());
        let nonzero = |v: f64| v != 0.0 && !v.is_nan();
        let shift = (nonzero(x) || nonzero(y)).then(|| json!([js_value(x), js_value(y)]));
        put(&mut p, "shift", shift);
        take(&mut p, "shiftX");
        take(&mut p, "shiftY");
    }
    if has(&p, "offsetX") || has(&p, "offsetY") {
        let offset = pair_of(layer.get("offset"));
        let x = or(get(&p, "offsetX"), offset[0].clone()).unwrap_or(Value::Null);
        let y = or(get(&p, "offsetY"), offset[1].clone()).unwrap_or(Value::Null);
        put(&mut p, "offset", Some(json!([x, y])));
        take(&mut p, "offsetX");
        take(&mut p, "offsetY");
    }
    for (from, to) in [
        ("holePct", "hole"),
        ("teethDepthPct", "teethDepth"),
        ("jitterPct", "jitter"),
        ("coveragePct", "coverage"),
    ] {
        if has(&p, from) {
            let v = js_number_of(get(&p, from).as_ref()) / 100.0;
            put(&mut p, to, Some(js_value(v)));
            take(&mut p, from);
        }
    }
    if has(&p, "groupCount") || has(&p, "groupSpacing") {
        let group = layer.get("group").filter(|gr| gr.is_object());
        let g_count = group.and_then(|gr| gr.get("count")).cloned();
        let g_spacing = group.and_then(|gr| gr.get("spacing")).cloned();
        let (g_count, g_spacing) = if group.is_some() {
            (g_count, g_spacing)
        } else {
            (Some(json!(1)), Some(json!(1)))
        };
        let count = js_round(js_number_of(or(get(&p, "groupCount"), g_count).as_ref())).max(1.0);
        let spacing = js_number_of(or(get(&p, "groupSpacing"), g_spacing).as_ref());
        let value = (count > 1.0)
            .then(|| json!({ "count": js_value(count), "spacing": js_value(spacing) }));
        put(&mut p, "group", value);
        take(&mut p, "groupCount");
        take(&mut p, "groupSpacing");
    }
    if has(&p, "haloWidth") {
        let width = js_number_of(get(&p, "haloWidth").as_ref());
        let halo = match layer.get("halo") {
            Some(h @ Value::Object(_)) => {
                let mut h = h.clone();
                if let Some(o) = h.as_object_mut() {
                    o.insert("width".into(), js_value(width));
                }
                Some(h)
            }
            Some(Value::Null) => Some(Value::Null),
            _ => None,
        };
        put(&mut p, "halo", halo);
        take(&mut p, "haloWidth");
    }
    if p.iter()
        .any(|(k, v)| k == "dash" && matches!(v, Some(Value::Null)))
    {
        put(&mut p, "dash", None);
    }
    let mut out = layer.as_object().cloned().unwrap_or_default();
    for (k, v) in p {
        match v {
            Some(v) => {
                out.insert(k, v);
            }
            None => {
                out.remove(&k);
            }
        }
    }
    Value::Object(out)
}
