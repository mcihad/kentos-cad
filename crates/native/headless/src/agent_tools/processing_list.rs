//! İşlemler's tools as an agent reads them first (docs/adr/8057 §108): an index of each tool's id, name, category and
//! what it does, narrowed by words when asked; a tool's parameters come with `processing.describe`. The whole list with
//! every parameter took a conversation's budget at once: on the real window an agent asked to buffer and dissolve the
//! parcels read it, described two tools and was stopped at 90 900 of its 80 000 tokens before running anything.

use serde_json::{Map, Value, json};

use super::{Host, first_sentence};
use crate::HeadlessError;

/// What an extension's tool tells a model of when to use it, at most this many characters in the index.
const MODEL_WORDS: usize = 240;

/// `processing.list`: the tools of `cad.processing.tools` (its `query`, `category` and `models` passed on) as an index.
pub(super) fn list(host: &mut dyn Host, args: &Value) -> Result<Value, HeadlessError> {
    let mut input = Map::new();
    for key in ["query", "category"] {
        if let Some(words) = args
            .get(key)
            .and_then(Value::as_str)
            .filter(|w| !w.trim().is_empty())
        {
            input.insert(key.to_owned(), json!(words.trim()));
        }
    }
    if args.get("models").and_then(Value::as_bool) == Some(true) {
        input.insert("models".to_owned(), json!(true));
    }
    let narrowed = !input.is_empty();
    let words: Vec<String> = input
        .get("query")
        .and_then(Value::as_str)
        .map(|q| q.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default();
    let mut answer = super::processing_tools_with(host, Value::Object(input.clone()))?;
    let mut note = "Bir aracın parametrelerini ve çıktılarını processing.describe verir; çalıştırmadan önce okuyun.";
    // Words of two tools (“tampon birleştir”) name no one tool: each word's tools, as an agent asking for a chain
    // means (the real window: an agent asked for both and was told there were none).
    if count(&answer) == 0 && words.len() > 1 {
        let mut merged: Vec<Value> = Vec::new();
        for word in &words {
            let mut one = input.clone();
            one.insert("query".to_owned(), json!(word));
            let found = super::processing_tools_with(host, Value::Object(one))?;
            for tool in tools_of(&found) {
                if !merged.iter().any(|t| t.get("id") == tool.get("id")) {
                    merged.push(tool.clone());
                }
            }
        }
        if let Some(list) = answer.pointer_mut("/output/tools") {
            *list = Value::Array(merged);
        }
        note = "Bütün sözcükleri taşıyan bir araç yok; her sözcüğün araçları listelendi. Birkaçını zincirlemek için \
                processing.run'a bir model yazın.";
    }
    if count(&answer) == 0 && narrowed {
        note = "Eşleşen araç yok: başka bir sözcükle (tampon, kesişim, birleştir, alan, nokta …) ya da query'siz \
                arayın.";
    }
    if let Some(tools) = answer
        .pointer_mut("/output/tools")
        .and_then(Value::as_array_mut)
    {
        for tool in tools.iter_mut() {
            *tool = entry(tool);
        }
    }
    if let Some(map) = answer.as_object_mut() {
        map.insert("note".to_owned(), json!(note));
    }
    Ok(answer)
}

/// The tools an answer of `cad.processing.tools` lists.
fn tools_of(answer: &Value) -> &[Value] {
    answer
        .pointer("/output/tools")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn count(answer: &Value) -> usize {
    tools_of(answer).len()
}

/// A tool in the index: its id, name, category, its description's first sentence, what an extension's tool tells a
/// model and its extension's id.
fn entry(tool: &Value) -> Value {
    let mut out = Map::new();
    for key in ["id", "label", "category"] {
        if let Some(value) = tool.get(key) {
            out.insert(key.to_owned(), value.clone());
        }
    }
    if let Some(words) = tool.get("description").and_then(Value::as_str) {
        out.insert("description".to_owned(), json!(first_sentence(words)));
    }
    if let Some(words) = tool.get("modelDescription").and_then(Value::as_str) {
        let cut: String = words.chars().take(MODEL_WORDS).collect();
        out.insert("modelDescription".to_owned(), json!(cut));
    }
    if let Some(id) = tool.pointer("/extension/extension") {
        out.insert("extension".to_owned(), id.clone());
    }
    Value::Object(out)
}
