//! A drawing's requests by name (docs/adr/0132): what a Python script in the
//! desktop's console asks of the drawing open there, answered here so the
//! desktop and the headless session answer alike. Each request is a method
//! and its JSON parameters; the answer is JSON, or the host's refusal.
//!
//! - `run` `{command, op, input, version?}`: a catalog command (`crate::dispatch`);
//! - `summary`: the drawing as a whole (name, revision, dirty, objects,
//!   settings, active layer, undo and redo);
//! - `layers`: the layer tree;
//! - `entities` `{layer?, kinds?, bbox?, after?, limit?}`: a page of objects;
//!   `bbox` is `[min_x, min_y, max_x, max_y]`;
//! - `entity` `{uid}`, `measure` `{uid}`: one object, its measures.
//!
//! Undo, redo, groups and files belong to the host (the desktop keeps its
//! own history and its own Save); they are not requests here.

use kentos_contracts::Bounds;
use kentos_domain::Document;
use serde_json::{Value, json};

use crate::HeadlessError;
use crate::dispatch::{self, Op};
use crate::query::{self, Filter};

/// Answers `method` with `params` on `doc`.
pub fn call(doc: &mut Document, method: &str, params: &Value) -> Result<Value, HeadlessError> {
    match method {
        "run" => {
            let command = text(params, "command")?;
            let op = Op::parse(text(params, "op")?)?;
            let version = match params.get("version") {
                None | Some(Value::Null) => None,
                Some(v) => Some(
                    v.as_u64()
                        .and_then(|v| u32::try_from(v).ok())
                        .ok_or_else(|| invalid("version bir sürüm numarası olmalı"))?,
                ),
            };
            let input = params.get("input").cloned().unwrap_or(Value::Null);
            dispatch::run(doc, command, version, op, input)
        }
        "summary" => Ok(summary(doc)),
        "layers" => to_value(doc.layers().nodes()),
        "entities" => {
            let filter = Filter {
                layer: optional_text(params, "layer")?,
                kinds: match params.get("kinds") {
                    None | Some(Value::Null) => None,
                    Some(Value::Array(items)) => Some(
                        items
                            .iter()
                            .map(|k| k.as_str().map(str::to_owned))
                            .collect::<Option<Vec<_>>>()
                            .ok_or_else(|| invalid("kinds bir metin listesi olmalı"))?,
                    ),
                    Some(_) => return Err(invalid("kinds bir metin listesi olmalı")),
                },
                bbox: match params.get("bbox") {
                    None | Some(Value::Null) => None,
                    Some(v) => Some(bounds(v)?),
                },
                after: optional_text(params, "after")?,
                limit: match params.get("limit") {
                    None | Some(Value::Null) => None,
                    Some(v) => Some(
                        v.as_u64()
                            .and_then(|v| usize::try_from(v).ok())
                            .ok_or_else(|| invalid("limit bir sayı olmalı"))?,
                    ),
                },
            };
            to_value(&query::page(doc, &filter)?)
        }
        "entity" => {
            let uid = text(params, "uid")?;
            let slot = query::slot_of(doc, uid)?;
            let entity = doc.get(slot).ok_or_else(|| {
                HeadlessError::new(
                    "unknown_object",
                    format!("{uid} kimlikli nesne bu çizimde yok."),
                )
            })?;
            Ok(json!({ "uid": uid, "entity": entity }))
        }
        "measure" => to_value(&query::measure(doc, text(params, "uid")?)?),
        other => Err(HeadlessError::new(
            "unknown_method",
            format!(
                "“{other}” bir istek değil: run, summary, layers, entities, entity ya da measure."
            ),
        )),
    }
}

/// The drawing as a whole, without its objects: what `summary` answers.
pub fn summary(doc: &Document) -> Value {
    json!({
        "name": doc.name(),
        "revision": doc.revision().to_string(),
        "dirty": doc.is_dirty(),
        "objects": doc.len(),
        "settings": doc.settings(),
        "activeLayer": doc.layers().active(),
        "legacy": false,
        "canUndo": doc.can_undo(),
        "canRedo": doc.can_redo(),
    })
}

fn to_value<T: serde::Serialize + ?Sized>(value: &T) -> Result<Value, HeadlessError> {
    serde_json::to_value(value).map_err(|e| {
        HeadlessError::new("unwritable_result", format!("yanıt JSON'a yazılamadı: {e}"))
    })
}

fn invalid(message: &str) -> HeadlessError {
    HeadlessError::new("invalid_input", message)
}

fn text<'a>(params: &'a Value, name: &str) -> Result<&'a str, HeadlessError> {
    params.get(name).and_then(Value::as_str).ok_or_else(|| {
        HeadlessError::new("invalid_input", format!("{name} gerekli, metin olarak."))
    })
}

fn optional_text(params: &Value, name: &str) -> Result<Option<String>, HeadlessError> {
    match params.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(HeadlessError::new(
            "invalid_input",
            format!("{name} metin olmalı."),
        )),
    }
}

fn bounds(v: &Value) -> Result<Bounds, HeadlessError> {
    let n: Option<Vec<f64>> = v
        .as_array()
        .map(|items| items.iter().filter_map(Value::as_f64).collect());
    match n.as_deref() {
        Some(&[min_x, min_y, max_x, max_y]) => Ok(Bounds {
            min_x,
            min_y,
            max_x,
            max_y,
        }),
        _ => Err(invalid(
            "bbox dört sayı olmalı: [min_x, min_y, max_x, max_y]",
        )),
    }
}
