//! A geometry generator's steps in the designer (the web's `designerModel.ts`,
//! docs/adr/0226 §1): their names and menu order, how a new one starts, the
//! class a geometry is after them, a step's line in its list and the list's
//! edits. Steps stay the JSON they are saved as.

use serde_json::{Value, json};

use super::default_for_kind;
use super::summary::num;
use super::Choice;
use kentos_style_core::js::text::lower_tr;

/// The steps in the menu's order: points, lines, areas, then moves (`STEP_KINDS`).
pub const STEP_KINDS: [&str; 15] = [
    "centroid",
    "pointOnSurface",
    "vertices",
    "boundary",
    "offset",
    "extend",
    "buffer",
    "envelope",
    "hull",
    "circle",
    "wedge",
    "polygon",
    "translate",
    "rotate",
    "scale",
];

/// A step's name (`STEP_LABEL`).
pub fn step_label(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "centroid" => "Ağırlık merkezi",
        "pointOnSurface" => "İç nokta",
        "vertices" => "Köşeler",
        "boundary" => "Sınır",
        "offset" => "Paralel",
        "extend" => "Uzat",
        "buffer" => "Tampon",
        "envelope" => "Sınırlayıcı kutu",
        "hull" => "Dışbükey örtü",
        "circle" => "Daire",
        "wedge" => "Daire dilimi",
        "polygon" => "Düzgün çokgen",
        "translate" => "Öteleme",
        "rotate" => "Döndür",
        "scale" => "Ölçekle",
        _ => return None,
    })
}

pub const VERTEX_CHOICES: [Choice; 5] = [
    ("all", "Hepsi"),
    ("inner", "Uçlar dışında"),
    ("first", "İlk"),
    ("last", "Son"),
    ("ends", "İki uç"),
];

pub const BUFFER_SIDES: [Choice; 3] = [("both", "İki yan"), ("left", "Sol yan"), ("right", "Sağ yan")];

/// A new step of a kind, with numbers that show at once (`newStep`).
pub fn new_step(kind: &str) -> Value {
    match kind {
        "vertices" => json!({ "kind": kind, "which": "all" }),
        "offset" => json!({ "kind": kind, "distance": 1 }),
        "extend" => json!({ "kind": kind, "start": 0, "end": 2 }),
        "buffer" => json!({ "kind": kind, "distance": 2 }),
        "circle" => json!({ "kind": kind, "radius": 2 }),
        "wedge" => json!({ "kind": kind, "direction": 0, "width": 60, "radius": 5 }),
        "polygon" => json!({ "kind": kind, "sides": 6, "radius": 2 }),
        "translate" => json!({ "kind": kind, "dx": 1, "dy": 0 }),
        "rotate" => json!({ "kind": kind, "angle": 45 }),
        "scale" => json!({ "kind": kind, "x": 1.5 }),
        _ => json!({ "kind": kind }),
    }
}

/// The class a symbol of `kind` draws on (`CLASS_OF`): a fill symbol's areas,
/// a line symbol's lines, a marker symbol's points.
pub fn class_of(kind: &str) -> &'static str {
    match kind {
        "fill" => "area",
        "line" => "line",
        _ => "point",
    }
}

/// The class after a step of `kind` from `class`; None when it takes no such geometry.
fn class_after(kind: &str, class: &'static str) -> Option<&'static str> {
    match kind {
        "centroid" | "pointOnSurface" => Some("point"),
        "vertices" => (class != "point").then_some("point"),
        "boundary" => (class == "area").then_some("line"),
        "offset" => (class != "point").then_some("line"),
        "extend" => (class == "line").then_some("line"),
        "buffer" | "envelope" | "hull" | "circle" | "wedge" | "polygon" => Some("area"),
        _ => Some(class),
    }
}

/// The class a geometry of `input` is after `steps`; None when a step leaves nothing (`stepOutput`).
pub fn step_output(steps: &[Value], input: &'static str) -> Option<&'static str> {
    let mut class = Some(input);
    for st in steps {
        let c = class?;
        class = class_after(st.get("kind").and_then(Value::as_str).unwrap_or(""), c);
    }
    class
}

/// A number, or `fallback` when the step has none.
fn num_or(v: Option<&Value>, fallback: f64) -> String {
    num(Some(v.unwrap_or(&json!(fallback))))
}

/// A step's line in its list: its name and numbers in the layer's unit (`stepSummary`).
pub fn step_summary(step: &Value, unit: &str) -> String {
    let kind = step.get("kind").and_then(Value::as_str).unwrap_or("");
    let name = step_label(kind).unwrap_or(kind);
    let g = |k: &str| step.get(k);
    match kind {
        "vertices" => {
            let which = g("which").and_then(Value::as_str).unwrap_or("all");
            let label = VERTEX_CHOICES
                .iter()
                .find(|(v, _)| *v == which)
                .map_or(which, |(_, l)| l);
            format!("{name} · {label}")
        }
        "offset" => format!("{name} · {} {unit}", num(g("distance"))),
        "extend" => format!(
            "{name} · {} / {} {unit}",
            num_or(g("start"), 0.0),
            num_or(g("end"), 0.0)
        ),
        "buffer" => {
            let side = g("side").and_then(Value::as_str).filter(|s| *s != "both");
            let tail = side
                .and_then(|s| BUFFER_SIDES.iter().find(|(v, _)| *v == s))
                .map_or_else(String::new, |(_, l)| format!(", {}", lower_tr(l)));
            format!("{name} · {} {unit}{tail}", num(g("distance")))
        }
        "circle" => format!("{name} · {} {unit}", num(g("radius"))),
        "wedge" => format!("{name} · {}° · {} {unit}", num(g("width")), num(g("radius"))),
        "polygon" => format!("{name} · {} köşe · {} {unit}", num(g("sides")), num(g("radius"))),
        "translate" => format!(
            "{name} · {}, {} {unit}",
            num_or(g("dx"), 0.0),
            num_or(g("dy"), 0.0)
        ),
        "rotate" => format!("{name} · {}°", num(g("angle"))),
        "scale" => {
            let (x, y) = (g("x"), g("y"));
            let one = json!(1);
            format!(
                "{name} · {} × {}",
                num(Some(x.or(y).unwrap_or(&one))),
                num(Some(y.or(x).unwrap_or(&one)))
            )
        }
        _ => name.to_owned(),
    }
}

fn steps_of(layer: &Value) -> Vec<Value> {
    layer
        .get("steps")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn with_steps(layer: &Value, steps: Vec<Value>) -> Value {
    let mut out = layer.clone();
    if let Some(o) = out.as_object_mut() {
        o.insert("steps".into(), Value::Array(steps));
    }
    out
}

/// The generator with a new step of `kind` at the end (`addStep`).
pub fn add_step(layer: &Value, kind: &str) -> Value {
    let mut steps = steps_of(layer);
    steps.push(new_step(kind));
    with_steps(layer, steps)
}

/// The generator with step `i` swapped with its neighbour; None at that end (`moveStep`).
pub fn move_step(layer: &Value, i: usize, delta: i64) -> Option<Value> {
    let mut steps = steps_of(layer);
    let to = i as i64 + delta;
    if i >= steps.len() || to < 0 || to as usize >= steps.len() {
        return None;
    }
    steps.swap(i, to as usize);
    Some(with_steps(layer, steps))
}

/// The generator without step `i` (`removeStep`).
pub fn remove_step(layer: &Value, i: usize) -> Value {
    let mut steps = steps_of(layer);
    if i < steps.len() {
        steps.remove(i);
    }
    with_steps(layer, steps)
}

/// The generator with step `i` replaced (`putStep`).
pub fn put_step(layer: &Value, i: usize, step: Value) -> Value {
    let mut steps = steps_of(layer);
    if let Some(slot) = steps.get_mut(i) {
        *slot = step;
    }
    with_steps(layer, steps)
}

/// The generator drawing with a symbol of another class: that class's starting symbol (`setGeneratorKind`).
pub fn set_generator_kind(layer: &Value, kind: &str) -> Value {
    let now = layer
        .get("symbol")
        .and_then(|s| s.get("type"))
        .and_then(Value::as_str);
    if now == Some(kind) {
        return layer.clone();
    }
    let mut out = layer.clone();
    if let Some(o) = out.as_object_mut() {
        o.insert("symbol".into(), default_for_kind(kind));
    }
    out
}
