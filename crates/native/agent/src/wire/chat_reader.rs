//! Reading a Chat Completions stream (docs/adr/8056 §4; the research note's “Akış ayrıştırıcısı”): `data:` chunks to
//! `[DONE]`; `choices` may be empty (the usage chunk) and `usage` may sit beside a choice (OpenRouter, DeepSeek) or in
//! it (Kimi); `content` and `reasoning_content` come as `null` between pieces; Mistral's content may be chunks with its
//! thinking; a call's `id` and name come with its first fragment, its arguments in fragments by `index` (or whole, an
//! object even, from Mistral); an error inside the stream is a top-level `error` or `finish_reason: "error"`. The
//! assistant's message is put back together as the provider would send it again (the echo).

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value;

use super::chat::{Quirks, call_json, nine, quirks};
use super::{Decoder, Live, Reply, Stop, count, fresh_call_id, parse_input};
use crate::conversation::{Echo, Message, Part, Reasoning, ToolCall};
use crate::failure::Failure;
use crate::json::{Json, Obj};
use crate::providers::ProviderKind;
use crate::sse::SseEvent;
use crate::tools::ToolNames;

/// A call as its fragments come.
#[derive(Default)]
struct Partial {
    id: String,
    name: String,
    arguments: String,
}

pub(crate) struct Reader {
    kind: ProviderKind,
    model: String,
    names: Arc<ToolNames>,
    who: &'static str,
    q: Quirks,
    text: String,
    reasoning: String,
    details: Vec<Value>,
    calls: BTreeMap<u64, Partial>,
    finish_reason: Option<String>,
    usage: Option<(u64, u64)>,
    done: bool,
}

impl Reader {
    pub(crate) fn new(
        kind: ProviderKind,
        model: String,
        names: Arc<ToolNames>,
        who: &'static str,
    ) -> Reader {
        Reader {
            kind,
            model,
            names,
            who,
            q: quirks(kind),
            text: String::new(),
            reasoning: String::new(),
            details: Vec::new(),
            calls: BTreeMap::new(),
            finish_reason: None,
            usage: None,
            done: false,
        }
    }

    fn take_text(&mut self, piece: &str, live: &mut Vec<Live>) {
        if !piece.is_empty() {
            self.text.push_str(piece);
            live.push(Live::Text(piece.to_owned()));
        }
    }

    fn take_reasoning(&mut self, piece: &str, live: &mut Vec<Live>) {
        if !piece.is_empty() {
            self.reasoning.push_str(piece);
            live.push(Live::Reasoning(piece.to_owned()));
        }
    }

    /// OpenRouter's reasoning details: pieces of one detail (same `index` and type) joined, others kept apart.
    fn take_detail(&mut self, d: &Value) {
        let index = d["index"].as_u64();
        let at = index.and_then(|i| {
            self.details
                .iter()
                .position(|x| x["index"].as_u64() == Some(i) && x["type"] == d["type"])
        });
        let (Some(at), Some(fields)) = (at, d.as_object()) else {
            self.details.push(d.clone());
            return;
        };
        let x = &mut self.details[at];
        for (field, value) in fields {
            if value.is_null() {
                continue;
            }
            let joined = matches!(field.as_str(), "text" | "summary" | "data")
                && value.is_string()
                && x.get(field.as_str()).is_some_and(Value::is_string);
            if joined {
                if let (Some(Value::String(have)), Some(piece)) =
                    (x.get_mut(field.as_str()), value.as_str())
                {
                    have.push_str(piece);
                }
            } else if let Some(obj) = x.as_object_mut() {
                obj.insert(field.clone(), value.clone());
            }
        }
    }
}

/// A chunk's error, when it carries one: `{"error": {"message", "code"}}` or `{"error": "…"}`.
fn error_of(v: &Value) -> Option<String> {
    match &v["error"] {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        e => {
            let message = e["message"].as_str().unwrap_or("bilinmeyen hata");
            let code = match &e["code"] {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                _ => String::new(),
            };
            Some(if code.is_empty() {
                message.to_owned()
            } else {
                format!("{message} ({code})")
            })
        }
    }
}

impl Decoder for Reader {
    fn event(&mut self, event: &SseEvent, live: &mut Vec<Live>) -> Result<(), Failure> {
        let data = event.data.trim();
        if data == "[DONE]" {
            self.done = true;
            return Ok(());
        }
        let Ok(v) = serde_json::from_str::<Value>(data) else {
            return Ok(());
        };
        if let Some(what) = error_of(&v) {
            return Err(Failure::in_stream(self.who, &what, None));
        }
        let choice = v["choices"]
            .as_array()
            .and_then(|c| c.first())
            .unwrap_or(&Value::Null);
        for u in [&v["usage"], &choice["usage"]] {
            if u.is_object() {
                self.usage = Some((count(&u["prompt_tokens"]), count(&u["completion_tokens"])));
            }
        }
        let d = &choice["delta"];
        match &d["content"] {
            Value::String(s) => self.take_text(s, live),
            Value::Array(chunks) => {
                for c in chunks {
                    match c["type"].as_str() {
                        Some("text") => {
                            self.take_text(c["text"].as_str().unwrap_or_default(), live)
                        }
                        Some("thinking") => match &c["thinking"] {
                            Value::String(s) => self.take_reasoning(s, live),
                            Value::Array(pieces) => {
                                for p in pieces {
                                    self.take_reasoning(
                                        p["text"].as_str().unwrap_or_default(),
                                        live,
                                    );
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        // DeepSeek, Kimi, GLM and Qwen say `reasoning_content`; OpenRouter and Ollama `reasoning`.
        let said = |field: &str| d[field].as_str().filter(|s| !s.is_empty());
        if let Some(r) = said("reasoning_content").or_else(|| said("reasoning")) {
            self.take_reasoning(r, live);
        }
        if let Some(list) = d["reasoning_details"].as_array() {
            for detail in list {
                self.take_detail(detail);
            }
        }
        if let Some(list) = d["tool_calls"].as_array() {
            for (at, tc) in list.iter().enumerate() {
                let index = tc["index"].as_u64().unwrap_or(at as u64);
                let call = self.calls.entry(index).or_default();
                if let Some(id) = tc["id"].as_str().filter(|s| !s.is_empty()) {
                    id.clone_into(&mut call.id);
                }
                if let Some(name) = tc["function"]["name"].as_str().filter(|s| !s.is_empty())
                    && call.name.is_empty()
                {
                    name.clone_into(&mut call.name);
                }
                match &tc["function"]["arguments"] {
                    Value::String(s) => call.arguments.push_str(s),
                    whole @ (Value::Object(_) | Value::Array(_)) => {
                        call.arguments = whole.to_string()
                    }
                    _ => {}
                }
            }
        }
        if let Some(reason) = choice["finish_reason"].as_str() {
            if reason == "error" {
                return Err(Failure::in_stream(
                    self.who,
                    "yanıt üretilemedi (error)",
                    None,
                ));
            }
            self.finish_reason = Some(reason.to_owned());
        }
        Ok(())
    }

    fn done(&self) -> bool {
        self.done
    }

    fn finish(self: Box<Self>) -> Result<Reply, Failure> {
        let reason = self.finish_reason.as_deref();
        if !self.done && reason.is_none() {
            return Err(Failure::cut(self.who));
        }
        // DeepSeek's and GLM's ways of saying the answer was cut on their side.
        if let Some(r @ ("insufficient_system_resource" | "network_error" | "aborted")) = reason {
            return Err(Failure::in_stream(
                self.who,
                &format!("yanıt yarıda kaldı ({r})"),
                None,
            ));
        }
        let stop = match reason {
            Some("length" | "model_length") => Stop::Length,
            Some("model_context_window_exceeded") => Stop::ContextFull,
            _ => Stop::Done,
        };
        let mut parts = Vec::new();
        if !self.reasoning.is_empty() {
            parts.push(Part::Reasoning(Reasoning {
                provider: self.kind,
                model: self.model.clone(),
                text: self.reasoning.clone(),
            }));
        }
        if !self.text.is_empty() {
            parts.push(Part::Text(self.text.clone()));
        }
        let mut wire_calls = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for call in self.calls.into_values() {
            let input =
                parse_input(&call.arguments).ok_or_else(|| Failure::broken_call(self.who))?;
            let id = if call.id.is_empty() || !seen.insert(call.id.clone()) {
                if self.q.nine_char_ids {
                    nine(&fresh_call_id())
                } else {
                    fresh_call_id()
                }
            } else {
                call.id
            };
            let arguments = if call.arguments.trim().is_empty() {
                "{}".to_owned()
            } else {
                call.arguments
            };
            wire_calls.push(call_json(&id, &call.name, &arguments));
            parts.push(Part::ToolCall(ToolCall {
                id,
                name: self.names.app(&call.name),
                input,
            }));
        }
        // The message as the provider would take it back.
        let content: Json = if self.q.thinking_chunks && !self.reasoning.is_empty() {
            let thinking = Obj::new().put("type", "thinking").put(
                "thinking",
                vec![Json::from(
                    Obj::new()
                        .put("type", "text")
                        .put("text", self.reasoning.as_str()),
                )],
            );
            let mut chunks = vec![Json::from(thinking)];
            if !self.text.is_empty() {
                chunks.push(
                    Obj::new()
                        .put("type", "text")
                        .put("text", self.text.as_str())
                        .into(),
                );
            }
            chunks.into()
        } else {
            self.text.as_str().into()
        };
        let reasoning = !self.reasoning.is_empty();
        let echo = Obj::new()
            .put("role", "assistant")
            .put("content", content)
            .put_some(
                "reasoning_content",
                (self.q.reasoning_content && reasoning).then(|| self.reasoning.clone()),
            )
            .put_some(
                "reasoning_details",
                (self.q.reasoning_details && !self.details.is_empty())
                    .then(|| Json::Array(self.details.iter().cloned().map(Json::from).collect())),
            )
            .put_some(
                "reasoning",
                (self.q.reasoning_details && self.details.is_empty() && reasoning)
                    .then(|| self.reasoning.clone()),
            )
            .put_some("tool_calls", (!wire_calls.is_empty()).then_some(wire_calls))
            .text();
        Ok(Reply {
            message: Message::from_provider(parts, Echo::new(self.kind, &self.model, vec![echo])),
            stop,
            usage: self.usage,
        })
    }
}
