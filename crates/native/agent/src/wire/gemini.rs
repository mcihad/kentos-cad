//! Gemini's own `streamGenerateContent` (docs/adr/8056 §4; the research note's §4 and traps 17–22): `POST
//! {base}/models/{model}:streamGenerateContent?alt=sse` with `x-goog-api-key` (never the key in the address), data-only
//! events without `[DONE]`, the end said by `finishReason`. Thought signatures are binding: every part of the model's
//! turn goes back verbatim, in order, as it came (a call's signature on its part, a text's on an empty part); a call
//! the model did not write (another provider's) carries the documented placeholder signature on its step's first
//! call. Schemas go as `parametersJsonSchema`, `functionCallingConfig` auto, no sampling parameter.

use std::sync::Arc;

use serde_json::{Map, Value};

use super::{Call, Decoder, Live, Reply, Request, Stop, count, fresh_call_id, ours};
use crate::conversation::{Echo, Message, Part, Reasoning, Role, ToolCall};
use crate::failure::Failure;
use crate::json::{Json, Obj};
use crate::providers::{ProviderKind, Wire};
use crate::sse::SseEvent;
use crate::tools::{ToolNames, schema_for};

/// The signature the provider documents for calls it did not write.
const FOREIGN_SIGNATURE: &str = "skip_thought_signature_validator";

pub(crate) fn request(call: &Call<'_>) -> Request {
    let mut contents: Vec<(&'static str, Vec<Json>)> = Vec::new();
    for m in &call.conversation.messages {
        let (role, parts) = match m.role {
            Role::User => {
                let text = m.text();
                let parts = if text.is_empty() {
                    Vec::new()
                } else {
                    vec![Obj::new().put("text", text).into()]
                };
                ("user", parts)
            }
            Role::Tool => (
                "user",
                m.tool_results_iter()
                    .map(|r| response_part(r, call))
                    .collect(),
            ),
            Role::Assistant => ("model", model_parts(m, call)),
        };
        if parts.is_empty() {
            continue;
        }
        match contents.last_mut() {
            Some((last, list)) if *last == role => list.extend(parts),
            _ => contents.push((role, parts)),
        }
    }
    let contents: Vec<Json> = contents
        .into_iter()
        .map(|(role, parts)| Obj::new().put("role", role).put("parts", parts).into())
        .collect();
    let declarations: Vec<Json> = call
        .tools
        .iter()
        .map(|t| {
            Obj::new()
                .put("name", call.names.wire(&t.name))
                .put("description", t.description.as_str())
                .put(
                    "parametersJsonSchema",
                    schema_for(Wire::GeminiNative, &t.input_schema),
                )
                .into()
        })
        .collect();
    let offered = !declarations.is_empty();
    let system = &call.conversation.system;
    let body = Obj::new()
        .put("contents", contents)
        .put_some(
            "systemInstruction",
            (!system.is_empty()).then(|| {
                Obj::new().put(
                    "parts",
                    vec![Json::from(Obj::new().put("text", system.as_str()))],
                )
            }),
        )
        .put_some(
            "tools",
            offered.then(|| {
                vec![Json::from(
                    Obj::new().put("functionDeclarations", declarations),
                )]
            }),
        )
        .put_some(
            "toolConfig",
            offered
                .then(|| Obj::new().put("functionCallingConfig", Obj::new().put("mode", "AUTO"))),
        )
        .text();
    // A listed model comes as `models/…`; a tuned one has its own path.
    let model = call.profile.model.trim();
    let path = if model.contains('/') {
        model.to_owned()
    } else {
        format!("models/{model}")
    };
    Request {
        url: format!("{}/{path}:streamGenerateContent?alt=sse", call.base),
        headers: Vec::new(),
        secrets: call
            .key
            .map(|k| vec![("x-goog-api-key", k.to_owned())])
            .unwrap_or_default(),
        body,
    }
}

fn model_parts(m: &Message, call: &Call<'_>) -> Vec<Json> {
    if let Some(echo) = m.echo_for(call.profile.kind, &call.profile.model) {
        return echo.items().iter().map(|i| Json::Raw(i.clone())).collect();
    }
    // Another provider's turn, or the app's: its text and calls, the first call signed as foreign.
    let mut out = Vec::new();
    let mut signed = false;
    for p in &m.parts {
        match p {
            Part::Text(t) if !t.is_empty() => out.push(Obj::new().put("text", t.as_str()).into()),
            Part::ToolCall(c) => {
                let function = Obj::new()
                    .put_some("id", (!ours(&c.id)).then_some(c.id.as_str()))
                    .put("name", call.names.wire(&c.name))
                    .put("args", Json::from(c.input.clone()));
                out.push(
                    Obj::new()
                        .put("functionCall", function)
                        .put_some("thoughtSignature", (!signed).then_some(FOREIGN_SIGNATURE))
                        .into(),
                );
                signed = true;
            }
            _ => {}
        }
    }
    out
}

/// A result as a `functionResponse` part: an object as it is, anything else under `output`, an error under `error`.
fn response_part(r: &crate::conversation::ToolResult, call: &Call<'_>) -> Json {
    let response = if r.is_error {
        let mut m = Map::new();
        m.insert("error".into(), r.output.clone());
        Value::Object(m)
    } else if r.output.is_object() {
        r.output.clone()
    } else {
        let mut m = Map::new();
        m.insert("output".into(), r.output.clone());
        Value::Object(m)
    };
    let function = Obj::new()
        .put_some("id", (!ours(&r.id)).then_some(r.id.as_str()))
        .put("name", call.names.wire(&r.name))
        .put("response", response);
    Obj::new().put("functionResponse", function).into()
}

/// Reads each `GenerateContentResponse`: its parts kept as they came, text and thoughts as they come, the finish
/// reason, the usage.
pub(crate) struct Reader {
    kind: ProviderKind,
    model: String,
    names: Arc<ToolNames>,
    who: &'static str,
    /// Every part, read and as its JSON text in its own key order.
    parts: Vec<(Value, String)>,
    finish: Option<String>,
    blocked: Option<String>,
    usage: Option<(u64, u64)>,
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
            parts: Vec::new(),
            finish: None,
            blocked: None,
            usage: None,
        }
    }
}

impl Decoder for Reader {
    fn event(&mut self, event: &SseEvent, live: &mut Vec<Live>) -> Result<(), Failure> {
        let Ok(v) = serde_json::from_str::<Value>(&event.data) else {
            return Ok(());
        };
        let e = &v["error"];
        if !e.is_null() {
            let what = format!(
                "{} ({})",
                e["message"].as_str().unwrap_or("bilinmeyen hata"),
                e["status"].as_str().unwrap_or("error")
            );
            return Err(Failure::in_stream(self.who, &what, None));
        }
        if let Some(reason) = v["promptFeedback"]["blockReason"].as_str() {
            self.blocked = Some(reason.to_owned());
        }
        let u = &v["usageMetadata"];
        if u.is_object() {
            let output = count(&u["candidatesTokenCount"]) + count(&u["thoughtsTokenCount"]);
            self.usage = Some((count(&u["promptTokenCount"]), output));
        }
        let candidate = &v["candidates"][0];
        // The parts again in their own order, to go back exactly as they came.
        let exact: Option<Json> = serde_json::from_str(&event.data).ok();
        let exact_parts = exact
            .as_ref()
            .and_then(|j| j.get("candidates"))
            .and_then(|c| c.items().first())
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .map(Json::items)
            .unwrap_or_default();
        if let Some(parts) = candidate["content"]["parts"].as_array() {
            for (i, part) in parts.iter().enumerate() {
                if let Some(text) = part["text"].as_str().filter(|t| !t.is_empty()) {
                    live.push(if part["thought"] == true {
                        Live::Reasoning(text.to_owned())
                    } else {
                        Live::Text(text.to_owned())
                    });
                }
                let raw = exact_parts
                    .get(i)
                    .map_or_else(|| part.to_string(), Json::text);
                self.parts.push((part.clone(), raw));
            }
        }
        if let Some(reason) = candidate["finishReason"].as_str() {
            self.finish = Some(reason.to_owned());
        }
        Ok(())
    }

    fn done(&self) -> bool {
        // Nothing follows the finish reason but the connection's end.
        false
    }

    fn finish(self: Box<Self>) -> Result<Reply, Failure> {
        if let Some(reason) = &self.blocked {
            return Err(Failure::bad_request(format!(
                "{} isteği güvenlik süzgecine takıldığı için yanıtlamadı ({reason}).",
                self.who
            )));
        }
        let Some(reason) = self.finish.as_deref() else {
            return Err(Failure::cut(self.who));
        };
        if matches!(
            reason,
            "MALFORMED_FUNCTION_CALL"
                | "UNEXPECTED_TOOL_CALL"
                | "TOO_MANY_TOOL_CALLS"
                | "MISSING_THOUGHT_SIGNATURE"
        ) {
            return Err(Failure::in_stream(
                self.who,
                &format!("geçersiz bir araç çağrısı üretti ({reason})"),
                None,
            ));
        }
        let stop = if reason == "MAX_TOKENS" {
            Stop::Length
        } else {
            Stop::Done
        };
        let mut parts: Vec<Part> = Vec::new();
        let mut items = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (part, raw) in self.parts {
            items.push(raw);
            if let Some(text) = part["text"].as_str().filter(|t| !t.is_empty()) {
                if part["thought"] == true {
                    parts.push(Part::Reasoning(Reasoning {
                        provider: self.kind,
                        model: self.model.clone(),
                        text: text.to_owned(),
                    }));
                } else if let Some(Part::Text(last)) = parts.last_mut() {
                    // A text the stream cut into pieces is one text.
                    last.push_str(text);
                } else {
                    parts.push(Part::Text(text.to_owned()));
                }
            }
            let f = &part["functionCall"];
            if f.is_object() {
                let id = f["id"].as_str().unwrap_or_default().to_owned();
                let id = if id.is_empty() || !seen.insert(id.clone()) {
                    fresh_call_id()
                } else {
                    id
                };
                let input = if f["args"].is_null() {
                    Value::Object(Map::new())
                } else {
                    f["args"].clone()
                };
                parts.push(Part::ToolCall(ToolCall {
                    id,
                    name: self.names.app(f["name"].as_str().unwrap_or_default()),
                    input,
                }));
            }
        }
        Ok(Reply {
            message: Message::from_provider(parts, Echo::new(self.kind, &self.model, items)),
            stop,
            usage: self.usage,
        })
    }
}
