//! The label engine's texts on the desktop (docs/adr/0212 §3.1): each
//! object's text for every class that labels it, worked out here and handed
//! to the geometry store, which places them. A layer's classes are its
//! rules' (a class labels an object its condition holds for), its single
//! label style's or, without one, its objects' kinds' defaults; a class's
//! text is its expression (İfadeyle seç's language) or its template filled
//! with the object's own label. A contour's class gives its height too (its
//! first vertex's). The web's is `apps/web/src/model/labelTexts.ts`.

use kentos_contracts::{Entity, LabelStyle, LabelsMode, LayerNode, LineLabelMode, default_label};
use kentos_expression::rows::{As, RowsInput, TEXT, evaluate_rows_on};
use kentos_expression::{Expr, compile};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::label_text::fill_template;

use crate::layer_filter::{kind_label, vertex_count};

/// An expression for a label (its text or its class's condition), compiled;
/// `$sıra` and `$ölçek` refused (a label's text does not change with the run
/// or the drawing's scale).
pub fn compile_label_expression(source: &str) -> Result<Expr, String> {
    let e = compile(source).map_err(|e| e.text())?;
    if e.needs.index {
        return Err("Etikette $sıra kullanılamaz.".into());
    }
    if e.needs.scale {
        return Err("Etikette $ölçek kullanılamaz.".into());
    }
    Ok(e)
}

/// One class as the desktop asks it.
#[derive(Clone, Debug)]
pub struct TextClass {
    pub when: Option<Expr>,
    pub text: Option<Expr>,
    pub template: Option<String>,
    pub contour: bool,
    /// Why the class labels nothing (its text or condition does not compile).
    pub error: Option<String>,
}

/// A layer's classes.
#[derive(Clone, Debug)]
pub enum LayerTexts {
    Off,
    /// `defaults`: its objects' kinds' defaults (one class each).
    Classes {
        classes: Vec<TextClass>,
        defaults: bool,
    },
}

fn text_class(style: &LabelStyle, when: Option<&str>) -> TextClass {
    let w = when.map(compile_label_expression).transpose();
    let t = style
        .text
        .as_deref()
        .map(compile_label_expression)
        .transpose();
    let error = w.as_ref().err().or(t.as_ref().err()).cloned();
    TextClass {
        when: w.ok().flatten(),
        text: t.ok().flatten(),
        template: style.template.clone(),
        contour: style.line == Some(LineLabelMode::Contour),
        error,
    }
}

/// A layer's classes as the desktop asks them.
pub fn layer_texts(node: Option<&LayerNode>) -> LayerTexts {
    let style = node.map(|n| &n.style);
    match style.and_then(|s| s.labels.as_ref()) {
        Some(l) if l.mode == LabelsMode::Off => LayerTexts::Off,
        Some(l) if l.mode == LabelsMode::Rules => LayerTexts::Classes {
            classes: l
                .classes
                .iter()
                .map(|c| text_class(&c.style, c.when.as_deref()))
                .collect(),
            defaults: false,
        },
        _ => match style.and_then(|s| s.label.as_ref()) {
            Some(single) => LayerTexts::Classes {
                classes: vec![text_class(single, None)],
                defaults: false,
            },
            None => LayerTexts::Classes {
                classes: Vec::new(),
                defaults: true,
            },
        },
    }
}

/// Kinds the engine labels: points and blocks, lines, areas.
fn labelled(e: &Entity) -> bool {
    !matches!(
        e,
        Entity::Text(_)
            | Entity::Dimension(_)
            | Entity::Leader(_)
            | Entity::Table(_)
            | Entity::Xline(_)
            | Entity::Ray(_)
    )
}

/// An object's kind as the defaults name it.
fn kind_name(e: &Entity) -> &'static str {
    match e {
        Entity::Polygon(_) => "polygon",
        Entity::Circle(_) => "circle",
        Entity::Point(_) => "point",
        Entity::Polyline(_) => "polyline",
        Entity::Line(_) => "line",
        _ => "",
    }
}

/// A contour's height: its first vertex's elevation, else NaN.
fn height_of(e: &Entity) -> f64 {
    match e {
        Entity::Polyline(p) => p.zs.iter().flatten().find_map(|z| *z).unwrap_or(f64::NAN),
        Entity::Line(l) => l.za.unwrap_or(f64::NAN),
        Entity::Point(p) => p.z.unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

/// One expression's values for the objects as `want` asks (`None` where
/// empty), the text values as text.
fn values<'s>(
    e: &Expr,
    list: &[&Entity],
    layer_name: &dyn Fn(&str) -> String,
    shape: &dyn Fn(usize) -> Option<&'s Shape>,
    want: As,
) -> Vec<Option<String>> {
    let needs = e.needs;
    let mut texts = String::new();
    let mut lens: Vec<i32> = Vec::with_capacity(list.len() * (e.fields.len() + 3));
    let mut numbers: Vec<f64> = Vec::new();
    let put = |texts: &mut String, lens: &mut Vec<i32>, v: Option<&str>| match v {
        None => lens.push(-1),
        Some(v) => {
            texts.push_str(v);
            lens.push(v.encode_utf16().count() as i32);
        }
    };
    for o in list {
        let base = o.base();
        for f in &e.fields {
            put(&mut texts, &mut lens, base.attrs.get(f).map(String::as_str));
        }
        if needs.label {
            put(&mut texts, &mut lens, base.label.as_deref());
        }
        if needs.layer {
            let name = layer_name(&base.layer_id);
            put(&mut texts, &mut lens, Some(&name));
        }
        if needs.kind {
            put(&mut texts, &mut lens, Some(kind_label(o)));
        }
        if needs.id {
            numbers.push(f64::from(base.id));
        }
        if needs.vertices {
            numbers.push(vertex_count(o).unwrap_or(f64::NAN));
        }
    }
    let input = RowsInput {
        n: list.len(),
        texts: &texts,
        text_lens: &lens,
        numbers: &numbers,
        measures: &[],
        scale: f64::NAN,
    };
    let Ok(column) = evaluate_rows_on(e, &input, want, shape) else {
        return vec![None; list.len()];
    };
    let units: Vec<u16> = column.texts.encode_utf16().collect();
    let mut at = 0usize;
    let mut k = 0usize;
    let mut out = Vec::with_capacity(list.len());
    for (i, kind) in column.kinds.iter().enumerate() {
        if *kind == TEXT {
            let len = column.text_lens.get(k).copied().unwrap_or(0) as usize;
            k += 1;
            let s = String::from_utf16_lossy(units.get(at..at + len).unwrap_or_default());
            at += len;
            out.push(Some(s));
        } else if *kind == kentos_expression::rows::BOOL {
            out.push((column.numbers.get(i) == Some(&1.0)).then(|| "true".to_owned()));
        } else if *kind == kentos_expression::rows::NUMBER {
            out.push(column.numbers.get(i).map(|x| x.to_string()));
        } else {
            out.push(None);
        }
    }
    out
}

/// The texts of `list` (one layer's objects): for each object, each class's
/// text (when its condition holds and the text is not empty), in the
/// classes' order, and its height (a contour class's). `shape(i)` is object
/// `i`'s shape for the geometry values.
pub fn object_texts<'s>(
    list: &[&Entity],
    t: &LayerTexts,
    layer_name: &dyn Fn(&str) -> String,
    shape: &dyn Fn(usize) -> Option<&'s Shape>,
) -> Vec<(Vec<(u16, String)>, f64)> {
    let mut out: Vec<(Vec<(u16, String)>, f64)> =
        list.iter().map(|_| (Vec::new(), f64::NAN)).collect();
    let LayerTexts::Classes { classes, defaults } = t else {
        return out;
    };
    if *defaults {
        for (i, e) in list.iter().enumerate() {
            let Some(label) = e.base().label.as_deref().filter(|l| !l.is_empty()) else {
                continue;
            };
            if let Some(st) = default_label(kind_name(e)) {
                out[i]
                    .0
                    .push((0, fill_template(st.template.as_deref(), label)));
            }
        }
        return out;
    }
    let mut contour = false;
    for (k, c) in classes.iter().enumerate() {
        if c.error.is_some() {
            continue;
        }
        contour |= c.contour;
        let met = c
            .when
            .as_ref()
            .map(|w| values(w, list, layer_name, shape, As::Bool));
        let texts = c
            .text
            .as_ref()
            .map(|x| values(x, list, layer_name, shape, As::Text));
        for (i, e) in list.iter().enumerate() {
            if !labelled(e) || met.as_ref().is_some_and(|m| m[i].is_none()) {
                continue;
            }
            let s = match &texts {
                Some(t) => t[i].clone().unwrap_or_default(),
                None => e
                    .base()
                    .label
                    .as_deref()
                    .map(|l| fill_template(c.template.as_deref(), l))
                    .unwrap_or_default(),
            };
            if !s.is_empty() {
                out[i].0.push((k as u16, s));
            }
        }
    }
    if contour {
        for (i, e) in list.iter().enumerate() {
            out[i].1 = height_of(e);
        }
    }
    out
}

/// The label engine's view of the layers as the geometry store reads it
/// (docs/adr/0212 §3.1): every layer drawn from its objects with its place in
/// the drawing order (the top first: the tree's first leaf is drawn last,
/// over the others), its point symbol's size and its label fields.
pub fn label_layers_json(leaves: &[&LayerNode]) -> String {
    let rows: Vec<serde_json::Value> = leaves
        .iter()
        .enumerate()
        .map(|(i, l)| {
            let mut row = serde_json::json!({ "id": l.id, "rank": i });
            if let Some(p) = &l.style.point {
                row["point"] = p.size.into();
            }
            if let Some(label) = &l.style.label {
                row["label"] = serde_json::to_value(label).unwrap_or_default();
            }
            if let Some(labels) = &l.style.labels {
                row["labels"] = serde_json::to_value(labels).unwrap_or_default();
            }
            row
        })
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// The kinds' default label styles as the geometry store reads them (`default_label`).
pub fn label_defaults_json() -> String {
    let map: serde_json::Map<String, serde_json::Value> =
        ["polygon", "circle", "point", "polyline", "line"]
            .iter()
            .filter_map(|k| {
                default_label(k)
                    .map(|s| ((*k).to_owned(), serde_json::to_value(s).unwrap_or_default()))
            })
            .collect();
    serde_json::Value::Object(map).to_string()
}

/// What a layer's texts are made with: its name (`$katman`), its single
/// label and its rules; when it changes, the layer's texts are made again.
pub fn texts_key(l: &LayerNode) -> String {
    serde_json::to_string(&(
        &l.name,
        &l.style.label,
        l.style.labels.as_ref().map(|x| (&x.mode, &x.classes)),
    ))
    .unwrap_or_default()
}
