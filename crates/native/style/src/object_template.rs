//! Nesne şablonları (docs/adr/0176): a template is a drawing recipe for one kind
//! of object: the tool it is drawn with, the layer it goes on (found by
//! name, opened when the drawing lacks it), its colour, line weight, symbol,
//! attributes and label. Templates are the style library's third kind of item
//! (`kind: "template"`, [`crate::library`]). The web's rules are
//! `apps/web/src/model/objectTemplate.ts`; both pass `fixtures/style/v1/object-templates.json`.

use kentos_style_core::js::number;
use serde_json::{Map, Value};

/// The tools a template draws with, and their names as the interface says them.
pub const TEMPLATE_TOOLS: [(&str, &str); 9] = [
    ("point", "Nokta"),
    ("line", "Çizgi"),
    ("polyline", "Çoklu çizgi"),
    ("polygon", "Kapalı alan"),
    ("rectangle", "Dikdörtgen"),
    ("rectangle3", "Döndürülmüş dikdörtgen"),
    ("circle", "Daire"),
    ("text", "Yazı"),
    ("blockInsert", "Blok ekle"),
];

/// A tool's name as the interface says it; the id itself when it is not a template's tool.
pub fn tool_label(tool: &str) -> &str {
    TEMPLATE_TOOLS
        .iter()
        .find(|(id, _)| *id == tool)
        .map_or(tool, |(_, label)| label)
}

/// The methods a template may name, by tool (their options in the tool catalog);
/// a tool not here has none to choose.
pub fn template_methods(tool: &str) -> &'static [&'static str] {
    match tool {
        "circle" => &["2N", "3N", "TTY", "TTT"],
        _ => &[],
    }
}

/// The heaviest line weight, mm (`MAX_LINE_WEIGHT`).
const MAX_LINE_WEIGHT: f64 = 100.0;
const LINE_TYPES: [&str; 4] = ["continuous", "dashed", "dashdot", "dotted"];
/// A text's alignments but the left of the baseline, which is no value (docs/adr/0145).
const TEXT_ALIGNS: [&str; 11] = [
    "baselineCenter",
    "baselineRight",
    "bottomLeft",
    "bottomCenter",
    "bottomRight",
    "middleLeft",
    "middleCenter",
    "middleRight",
    "topLeft",
    "topCenter",
    "topRight",
];

/// A colour: hex with or without alpha, or a theme token (the style file's rule).
fn color_ok(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if matches!(lower.as_str(), "ink" | "paper" | "fg" | "fg-dim") {
        return true;
    }
    let Some(hex) = lower.strip_prefix('#') else {
        return false;
    };
    (hex.len() == 6 || hex.len() == 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Empty or only white space, as Unicode's `White_Space` has it.
fn blank(text: &str) -> bool {
    text.chars().all(char::is_whitespace)
}

fn weight_ok(v: &Value) -> bool {
    v.as_f64()
        .is_some_and(|w| w.is_finite() && (0.0..=MAX_LINE_WEIGHT).contains(&w))
}

/// A value as the web's `String(v)` writes it in a message.
fn shown(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_owned(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number::to_string(n.as_f64().unwrap_or(f64::NAN)),
        // An array's items joined by commas, a missing one empty.
        Value::Array(items) => items
            .iter()
            .map(|x| if x.is_null() { String::new() } else { shown(x) })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// What is wrong with a template, each problem with where it is (`where_`: “öğe 3
/// (Parsel sınırı)”), in the order the fields are read; none for a template the
/// app can draw with. A template read from a file or a library is untrusted data.
pub fn template_issues(template: &Value, where_: &str) -> Vec<String> {
    let Some(template) = template.as_object() else {
        return vec![format!("{where_}: şablon tanımı yok")];
    };
    let mut out = Vec::new();
    let mut say = |text: String| out.push(format!("{where_}: {text}"));
    let get = |key: &str| template.get(key);
    let tool = get("tool").and_then(Value::as_str).unwrap_or("");
    let known = TEMPLATE_TOOLS
        .iter()
        .find(|(id, _)| *id == tool)
        .map(|(_, l)| *l);
    if known.is_none() {
        say(format!(
            "bilinmeyen araç “{}”",
            get("tool").map_or_else(|| "undefined".to_owned(), shown)
        ));
    }
    if let (Some(method), Some(label)) = (get("method"), known) {
        let methods = template_methods(tool);
        if methods.is_empty() {
            say(format!("{label} aracının seçilecek yöntemi yok"));
        } else if !method.as_str().is_some_and(|m| methods.contains(&m)) {
            say(format!("“{}” yöntemi {label} aracında yok", shown(method)));
        }
    }
    match get("layer").and_then(Value::as_object) {
        None => say("katmanı yok".to_owned()),
        Some(layer) => {
            if layer.get("name").and_then(Value::as_str).is_none_or(blank) {
                say("katmanın adı boş".to_owned());
            }
            match layer.get("path").and_then(Value::as_array) {
                Some(path) if path.iter().all(Value::is_string) => {
                    if path.iter().filter_map(Value::as_str).any(blank) {
                        say("katmanın yolunda boş ad var".to_owned());
                    }
                }
                _ => say("katmanın yolu metin listesi olmalı".to_owned()),
            }
            if let Some(c) = layer.get("color")
                && !c.as_str().is_some_and(color_ok)
            {
                say(format!("katmanın rengi geçersiz “{}”", shown(c)));
            }
            if let Some(t) = layer.get("lineType")
                && !t.as_str().is_some_and(|t| LINE_TYPES.contains(&t))
            {
                say(format!("katmanın çizgi tipi bilinmiyor “{}”", shown(t)));
            }
            if layer.get("lineWeight").is_some_and(|w| !weight_ok(w)) {
                say("katmanın kalınlığı 0 ile 100 mm arasında olmalı".to_owned());
            }
        }
    }
    if let Some(c) = get("color")
        && !c.as_str().is_some_and(color_ok)
    {
        say(format!("renk geçersiz “{}”", shown(c)));
    }
    if get("lineWeight").is_some_and(|w| !weight_ok(w)) {
        say("kalınlık 0 ile 100 mm arasında olmalı".to_owned());
    }
    if get("symbol").is_some_and(|s| s.as_str().is_none_or(blank)) {
        say("sembolün kimliği boş".to_owned());
    }
    if let Some(attrs) = get("attrs") {
        match attrs.as_object() {
            None => say("öznitelikler ad ve metin değer olmalı".to_owned()),
            // By name: the web sorts them the same way.
            Some(attrs) => {
                for (name, value) in attrs {
                    if blank(name) {
                        say("öznitelik adı boş".to_owned());
                    }
                    if !value.is_string() {
                        say(format!("“{name}” özniteliğinin değeri metin olmalı"));
                    }
                }
            }
        }
    }
    if get("label").is_some_and(|l| !l.is_string()) {
        say("etiket metin olmalı".to_owned());
    }
    if let Some(point) = get("point") {
        let texts = |p: &Map<String, Value>| {
            ["name", "code"]
                .iter()
                .all(|k| p.get(*k).is_none_or(Value::is_string))
        };
        if tool != "point" {
            say("ad ve kod yalnız nokta şablonunda olur".to_owned());
        } else if !point.as_object().is_some_and(texts) {
            say("noktanın adı ve kodu metin olmalı".to_owned());
        }
    }
    if let Some(text) = get("text") {
        if tool != "text" {
            say("yazı bilgisi yalnız yazı şablonunda olur".to_owned());
        } else {
            match text.as_object() {
                None => say("yazı bilgisi yok".to_owned()),
                Some(text) => {
                    if !text
                        .get("height")
                        .and_then(Value::as_f64)
                        .is_some_and(|h| h.is_finite() && h > 0.0)
                    {
                        say("yazının yüksekliği sıfırdan büyük olmalı".to_owned());
                    }
                    if let Some(a) = text.get("align")
                        && !a.as_str().is_some_and(|a| TEXT_ALIGNS.contains(&a))
                    {
                        say(format!("bilinmeyen hiza “{}”", shown(a)));
                    }
                    if text.get("mask").is_some_and(|m| !m.is_boolean()) {
                        say("zemin açık ya da kapalı olmalı".to_owned());
                    }
                }
            }
        }
    }
    if tool == "blockInsert" {
        if get("block").and_then(Value::as_str).is_none_or(blank) {
            say("blok şablonunun bloğu yok".to_owned());
        }
    } else if get("block").is_some() {
        say("blok yalnız blok şablonunda olur".to_owned());
    }
    out
}

/// What a template's card and preview show (docs/adr/0176): its own symbol when
/// the library has it, else one made of the template's colour and line weight
/// (its layer's when it gives none) in its tool's shape: a dot for a point
/// or a block, a word for a text, a line for a line, an outline for an area.
/// The web's is `style/templateSymbol.ts`.
pub fn preview_symbol(template: &Value, library: &crate::library::StyleLibrary) -> Value {
    if let Some(own) = template
        .get("symbol")
        .and_then(Value::as_str)
        .and_then(|id| library.symbol(id))
    {
        return own.clone();
    }
    let layer = template.get("layer");
    let field = |key: &str| {
        template
            .get(key)
            .or_else(|| layer.and_then(|l| l.get(key)))
            .cloned()
    };
    let color = field("color").unwrap_or_else(|| Value::from("ink"));
    let width = field("lineWeight").unwrap_or_else(|| Value::from(0.35));
    match template.get("tool").and_then(Value::as_str).unwrap_or("") {
        "point" | "blockInsert" => serde_json::json!({ "type": "marker", "layers": [
            { "id": "p", "type": "shape", "shape": "circle", "size": 2.4, "fill": color }
        ] }),
        "text" => serde_json::json!({ "type": "marker", "layers": [
            { "id": "t", "type": "text", "text": "Abc", "size": 3.2, "color": color }
        ] }),
        "line" | "polyline" => serde_json::json!({ "type": "line", "layers": [
            { "id": "l", "type": "simpleLine", "color": color, "width": width }
        ] }),
        _ => serde_json::json!({ "type": "fill", "layers": [
            { "id": "l", "type": "simpleLine", "color": color, "width": width }
        ] }),
    }
}
