//! The Şablon düzenleyici's form (docs/adr/0176 §4): its texts and choices,
//! the library item's fields and the template made from them, and back. The
//! rules (fixtures/style/v1/template-form.json, written from them by
//! scripts/fixtures/template_form_cases.py): Ad is trimmed and needed;
//! Kategori and the layer's Gruplar are split on “/”, each part trimmed, the
//! empty ones left out; Açıklama, Etiket and the point's Ad and Kod are
//! trimmed and, empty, not written; Yöntem only for a tool that has methods;
//! Katman (its name) is trimmed and needed; an empty choice is the default; a
//! weight is a number with a point or a comma, 0 to 100 mm; attribute rows
//! with neither a name nor a value are left out, a value without a name is
//! said by its row, a name written twice by its name; a text template needs
//! a Yükseklik above zero, a block template its Blok; the other tools' fields
//! are not written. Every problem in the order of the fields. The web's is
//! `model/templateForm.ts`.

use std::collections::BTreeMap;

use kentos_style_core::js::number;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::object_template::template_methods;

/// The form's texts and choices, as typed.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TemplateForm {
    pub name: String,
    /// The category path, “Kadastro / Sınırlar”.
    pub category: String,
    pub description: String,
    pub tool: String,
    /// The tool's method; empty: its first.
    pub method: String,
    /// The groups above the layer, “Kadastro / Tapu”, and its name.
    pub layer_groups: String,
    pub layer_name: String,
    /// The opened layer's look; empty: a new layer's.
    pub layer_color: String,
    pub layer_line_type: String,
    pub layer_weight: String,
    /// The objects' own colour, weight and symbol; empty: their layer's.
    pub color: String,
    pub weight: String,
    pub symbol: String,
    /// Attribute rows, name and value, as typed.
    pub attrs: Vec<(String, String)>,
    pub label: String,
    pub point_name: String,
    pub point_code: String,
    pub text_height: String,
    pub text_align: String,
    pub text_mask: bool,
    pub block: String,
}

impl TemplateForm {
    /// A new template's form: Kapalı alan, nothing else chosen.
    pub fn new() -> Self {
        Self {
            tool: "polygon".to_owned(),
            ..Self::default()
        }
    }
}

/// What a form makes: the library item's fields and its template.
#[derive(Clone, Debug, PartialEq)]
pub struct FormItem {
    pub name: String,
    pub path: Vec<String>,
    pub description: Option<String>,
    pub template: Value,
}

/// The text split on “/”, each part trimmed, the empty ones left out.
fn parts(text: &str) -> Vec<String> {
    text.split('/')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect()
}

/// A non-negative number with a point or a comma; none for anything else.
fn parse_number(text: &str) -> Option<f64> {
    let t = text.trim();
    let (whole, fraction) = match t.split_once(['.', ',']) {
        Some((w, f)) => (w, Some(f)),
        None => (t, None),
    };
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(whole) || fraction.is_some_and(|f| !digits(f)) {
        return None;
    }
    t.replace(',', ".").parse().ok()
}

/// The library item's fields and the template the form makes, or what is
/// wrong with it, in the order of the fields.
pub fn from_form(f: &TemplateForm) -> Result<FormItem, Vec<String>> {
    let mut issues = Vec::new();
    let name = f.name.trim();
    if name.is_empty() {
        issues.push("Şablonun adı boş; bir ad yazın.".to_owned());
    }
    let layer_name = f.layer_name.trim();
    if layer_name.is_empty() {
        issues.push("Katmanın adı boş; şablonun çizeceği katmanı yazın ya da seçin.".to_owned());
    }
    let mut weight_of = |text: &str, what: &str| {
        if text.trim().is_empty() {
            return None;
        }
        match parse_number(text).filter(|w| *w <= 100.0) {
            Some(w) => Some(w),
            None => {
                issues.push(format!("{what} 0 ile 100 mm arasında bir sayı olmalı."));
                None
            }
        }
    };
    let layer_weight = weight_of(&f.layer_weight, "Katmanın kalınlığı");
    let weight = weight_of(&f.weight, "Kalınlık");
    let mut attrs: BTreeMap<String, String> = BTreeMap::new();
    for (i, (key, value)) in f.attrs.iter().enumerate() {
        let key = key.trim();
        if key.is_empty() && value.trim().is_empty() {
            continue;
        }
        if key.is_empty() {
            issues.push(format!("{}. öznitelik satırının adı boş.", i + 1));
        } else if attrs.contains_key(key) {
            issues.push(format!("“{key}” özniteliği iki kez yazılmış."));
        } else {
            attrs.insert(key.to_owned(), value.clone());
        }
    }
    let mut text = None;
    if f.tool == "text" {
        match parse_number(&f.text_height).filter(|h| *h > 0.0) {
            None => issues.push("Yazının yüksekliği sıfırdan büyük bir sayı olmalı.".to_owned()),
            Some(h) => {
                let mut written = Map::new();
                written.insert("height".into(), Value::from(h));
                if !f.text_align.is_empty() {
                    written.insert("align".into(), Value::from(f.text_align.clone()));
                }
                if f.text_mask {
                    written.insert("mask".into(), Value::from(true));
                }
                text = Some(Value::Object(written));
            }
        }
    }
    let block = f.block.trim();
    if f.tool == "blockInsert" && block.is_empty() {
        issues.push("Blok şablonunun bloğu yok; yerleştirilecek bloğu seçin.".to_owned());
    }
    if !issues.is_empty() {
        return Err(issues);
    }
    let mut layer = Map::new();
    layer.insert("path".into(), Value::from(parts(&f.layer_groups)));
    layer.insert("name".into(), Value::from(layer_name));
    if !f.layer_color.is_empty() {
        layer.insert("color".into(), Value::from(f.layer_color.clone()));
    }
    if !f.layer_line_type.is_empty() {
        layer.insert("lineType".into(), Value::from(f.layer_line_type.clone()));
    }
    if let Some(w) = layer_weight {
        layer.insert("lineWeight".into(), Value::from(w));
    }
    let mut t = Map::new();
    t.insert("tool".into(), Value::from(f.tool.clone()));
    if !template_methods(&f.tool).is_empty() && !f.method.is_empty() {
        t.insert("method".into(), Value::from(f.method.clone()));
    }
    t.insert("layer".into(), Value::Object(layer));
    if !f.color.is_empty() {
        t.insert("color".into(), Value::from(f.color.clone()));
    }
    if let Some(w) = weight {
        t.insert("lineWeight".into(), Value::from(w));
    }
    if !f.symbol.is_empty() {
        t.insert("symbol".into(), Value::from(f.symbol.clone()));
    }
    if !attrs.is_empty() {
        let attrs: Map<String, Value> = attrs
            .into_iter()
            .map(|(k, v)| (k, Value::from(v)))
            .collect();
        t.insert("attrs".into(), Value::Object(attrs));
    }
    let label = f.label.trim();
    if !label.is_empty() {
        t.insert("label".into(), Value::from(label));
    }
    if f.tool == "point" {
        let mut point = Map::new();
        if !f.point_name.trim().is_empty() {
            point.insert("name".into(), Value::from(f.point_name.trim()));
        }
        if !f.point_code.trim().is_empty() {
            point.insert("code".into(), Value::from(f.point_code.trim()));
        }
        if !point.is_empty() {
            t.insert("point".into(), Value::Object(point));
        }
    }
    if let Some(text) = text {
        t.insert("text".into(), text);
    }
    if f.tool == "blockInsert" {
        t.insert("block".into(), Value::from(block));
    }
    let description = f.description.trim();
    Ok(FormItem {
        name: name.to_owned(),
        path: parts(&f.category),
        description: (!description.is_empty()).then(|| description.to_owned()),
        template: Value::Object(t),
    })
}

/// A library item's fields (`name`, `path`, `description`) and template as
/// its form shows them: paths joined with “ / ”, numbers as JavaScript writes
/// them, the attributes by name.
pub fn to_form(item: &Value) -> TemplateForm {
    let text = |v: Option<&Value>| v.and_then(Value::as_str).unwrap_or("").to_owned();
    let joined = |v: Option<&Value>| {
        v.and_then(Value::as_array)
            .map(|p| {
                p.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join(" / ")
            })
            .unwrap_or_default()
    };
    let num = |v: Option<&Value>| {
        v.and_then(Value::as_f64)
            .map(number::to_string)
            .unwrap_or_default()
    };
    let t = item.get("template");
    let field = |key: &str| t.and_then(|t| t.get(key));
    let layer = field("layer");
    let point = field("point");
    let written = field("text");
    TemplateForm {
        name: text(item.get("name")),
        category: joined(item.get("path")),
        description: text(item.get("description")),
        tool: text(field("tool")),
        method: text(field("method")),
        layer_groups: joined(layer.and_then(|l| l.get("path"))),
        layer_name: text(layer.and_then(|l| l.get("name"))),
        layer_color: text(layer.and_then(|l| l.get("color"))),
        layer_line_type: text(layer.and_then(|l| l.get("lineType"))),
        layer_weight: num(layer.and_then(|l| l.get("lineWeight"))),
        color: text(field("color")),
        weight: num(field("lineWeight")),
        symbol: text(field("symbol")),
        // By name: the JSON map is read into a sorted one.
        attrs: field("attrs")
            .and_then(Value::as_object)
            .map(|a| {
                a.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_owned()))
                    .collect::<BTreeMap<_, _>>()
                    .into_iter()
                    .collect()
            })
            .unwrap_or_default(),
        label: text(field("label")),
        point_name: text(point.and_then(|p| p.get("name"))),
        point_code: text(point.and_then(|p| p.get("code"))),
        text_height: num(written.and_then(|w| w.get("height"))),
        text_align: text(written.and_then(|w| w.get("align"))),
        text_mask: written
            .and_then(|w| w.get("mask"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        block: text(field("block")),
    }
}
