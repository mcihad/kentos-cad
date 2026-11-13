//! Anthropic's Messages API (docs/adr/8056 §4; the research note's §1 and traps 1–7): `POST {base}/messages` with
//! `x-api-key` and `anthropic-version: 2026-10-21`, named server-sent events. No sampling parameter (that version
//! refuses them), `tool_choice` auto only (the 5.5 models refuse forcing), the conversation ends with the user's message
//! or the tools' results, results first in their user message and all of a turn's in one. Thinking and redacted
//! thinking blocks go back unchanged with their signatures; a tool's input goes back as the model wrote it, in its key
//! order, since a signature binds its block to everything before it.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value;

use super::{Call, Decoder, Live, Reply, Request, Stop, count, fresh_call_id, parse_input};
use crate::conversation::{Echo, Message, Part, Reasoning, Role, ToolCall};
use crate::failure::Failure;
use crate::json::{Json, Obj};
use crate::providers::{ProviderKind, Wire};
use crate::sse::SseEvent;
use crate::tools::{ToolNames, schema_for};

/// The API version every request names: the newest the research note found.
pub(crate) const VERSION: &str = "2026-10-21";
/// The output budget of an answer, which the API requires; thinking counts toward it.
const MAX_TOKENS: u64 = 32_000;

pub(crate) fn request(call: &Call<'_>) -> Request {
    let mut messages: Vec<(&'static str, Vec<Json>)> = Vec::new();
    for m in &call.conversation.messages {
        let (role, blocks) = match m.role {
            Role::User => ("user", user_blocks(m)),
            Role::Tool => ("user", result_blocks(m)),
            Role::Assistant => ("assistant", assistant_blocks(m, call)),
        };
        if blocks.is_empty() {
            continue;
        }
        // Results and a user's words after them make one user message, results first.
        match messages.last_mut() {
            Some((last, list)) if *last == role => list.extend(blocks),
            _ => messages.push((role, blocks)),
        }
    }
    let messages: Vec<Json> = messages
        .into_iter()
        .map(|(role, blocks)| Obj::new().put("role", role).put("content", blocks).into())
        .collect();
    let tools: Vec<Json> = call
        .tools
        .iter()
        .map(|t| {
            Obj::new()
                .put("name", call.names.wire(&t.name))
                .put("description", t.description.as_str())
                .put(
                    "input_schema",
                    schema_for(Wire::AnthropicMessages, &t.input_schema),
                )
                .into()
        })
        .collect();
    let system = &call.conversation.system;
    let body = Obj::new()
        .put("model", call.profile.model.as_str())
        .put("max_tokens", MAX_TOKENS)
        .put_some("system", (!system.is_empty()).then_some(system.as_str()))
        .put("messages", messages)
        .put_some(
            "tool_choice",
            (!tools.is_empty()).then(|| Obj::new().put("type", "auto")),
        )
        .put_some("tools", (!tools.is_empty()).then_some(tools))
        .put("stream", true)
        .text();
    Request {
        url: format!("{}/messages", call.base),
        headers: vec![("anthropic-version", VERSION.to_owned())],
        secrets: call
            .key
            .map(|k| vec![("x-api-key", k.to_owned())])
            .unwrap_or_default(),
        body,
    }
}

fn user_blocks(m: &Message) -> Vec<Json> {
    m.parts
        .iter()
        .filter_map(|p| match p {
            Part::Text(t) if !t.is_empty() => Some(text_block(t)),
            _ => None,
        })
        .collect()
}

fn result_blocks(m: &Message) -> Vec<Json> {
    m.tool_results_iter()
        .map(|r| {
            let text = r.text(true);
            Obj::new()
                .put("type", "tool_result")
                .put("tool_use_id", block_id(&r.id))
                .put_some("content", (!text.is_empty()).then_some(text))
                .put_some("is_error", r.is_error.then_some(true))
                .into()
        })
        .collect()
}

fn assistant_blocks(m: &Message, call: &Call<'_>) -> Vec<Json> {
    if let Some(echo) = m.echo_for(call.profile.kind, &call.profile.model) {
        return echo.items().iter().map(|i| Json::Raw(i.clone())).collect();
    }
    // Another provider's turn, or the app's: its text and calls, no reasoning.
    m.parts
        .iter()
        .filter_map(|p| match p {
            Part::Text(t) if !t.is_empty() => Some(text_block(t)),
            Part::ToolCall(c) => Some(
                Obj::new()
                    .put("type", "tool_use")
                    .put("id", block_id(&c.id))
                    .put("name", call.names.wire(&c.name))
                    .put("input", Json::from(c.input.clone()))
                    .into(),
            ),
            _ => None,
        })
        .collect()
}

fn text_block(text: &str) -> Json {
    Obj::new().put("type", "text").put("text", text).into()
}

/// A call's id as a block takes it (`^[a-zA-Z0-9_-]+$`): another provider's (`search:0`) mapped the same way for the
/// call and its result.
fn block_id(id: &str) -> String {
    let out: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        "toolu_kentos".to_owned()
    } else {
        out
    }
}

/// A content block as it streams.
enum Block {
    Text(String),
    Thinking {
        text: String,
        signature: String,
    },
    /// Redacted thinking and blocks of kinds not known here: back as they came.
    Whole(Value),
    ToolUse {
        id: String,
        name: String,
        input: String,
    },
}

/// Reads the named events: `message_start`, `content_block_start|delta|stop`, `message_delta`, `message_stop`, `ping`,
/// `error`; others are passed over.
pub(crate) struct Reader {
    kind: ProviderKind,
    model: String,
    names: Arc<ToolNames>,
    who: &'static str,
    blocks: BTreeMap<u64, Block>,
    stop_reason: Option<String>,
    input_tokens: u64,
    output_tokens: u64,
    counted: bool,
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
            blocks: BTreeMap::new(),
            stop_reason: None,
            input_tokens: 0,
            output_tokens: 0,
            counted: false,
            done: false,
        }
    }

    fn usage(&mut self, u: &Value) {
        if u.is_object() {
            let input = count(&u["input_tokens"])
                + count(&u["cache_creation_input_tokens"])
                + count(&u["cache_read_input_tokens"]);
            // `message_delta` counts are running totals.
            self.input_tokens = self.input_tokens.max(input);
            self.output_tokens = self.output_tokens.max(count(&u["output_tokens"]));
            self.counted = true;
        }
    }
}

impl Decoder for Reader {
    fn event(&mut self, event: &SseEvent, live: &mut Vec<Live>) -> Result<(), Failure> {
        let Ok(v) = serde_json::from_str::<Value>(&event.data) else {
            return Ok(());
        };
        let kind = v["type"].as_str().unwrap_or(event.event.as_str());
        match kind {
            "message_start" => self.usage(&v["message"]["usage"]),
            "content_block_start" => {
                let index = count(&v["index"]);
                let b = &v["content_block"];
                let block = match b["type"].as_str() {
                    Some("text") => Block::Text(b["text"].as_str().unwrap_or_default().to_owned()),
                    Some("thinking") => Block::Thinking {
                        text: b["thinking"].as_str().unwrap_or_default().to_owned(),
                        signature: b["signature"].as_str().unwrap_or_default().to_owned(),
                    },
                    Some("tool_use") => Block::ToolUse {
                        id: b["id"].as_str().unwrap_or_default().to_owned(),
                        name: b["name"].as_str().unwrap_or_default().to_owned(),
                        input: String::new(),
                    },
                    _ => Block::Whole(b.clone()),
                };
                self.blocks.insert(index, block);
            }
            "content_block_delta" => {
                let d = &v["delta"];
                let piece = |field: &str| d[field].as_str().unwrap_or_default().to_owned();
                match (self.blocks.get_mut(&count(&v["index"])), d["type"].as_str()) {
                    (Some(Block::Text(t)), Some("text_delta")) => {
                        let p = piece("text");
                        t.push_str(&p);
                        live.push(Live::Text(p));
                    }
                    (Some(Block::Thinking { text, .. }), Some("thinking_delta")) => {
                        let p = piece("thinking");
                        text.push_str(&p);
                        live.push(Live::Reasoning(p));
                    }
                    (Some(Block::Thinking { signature, .. }), Some("signature_delta")) => {
                        signature.push_str(&piece("signature"));
                    }
                    (Some(Block::ToolUse { input, .. }), Some("input_json_delta")) => {
                        input.push_str(&piece("partial_json"));
                    }
                    _ => {}
                }
            }
            "message_delta" => {
                if let Some(r) = v["delta"]["stop_reason"].as_str() {
                    self.stop_reason = Some(r.to_owned());
                }
                self.usage(&v["usage"]);
            }
            "message_stop" => self.done = true,
            "error" => {
                let e = &v["error"];
                let what = format!(
                    "{} ({})",
                    e["message"].as_str().unwrap_or("bilinmeyen hata"),
                    e["type"].as_str().unwrap_or("error")
                );
                return Err(Failure::in_stream(self.who, &what, None));
            }
            _ => {}
        }
        Ok(())
    }

    fn done(&self) -> bool {
        self.done
    }

    fn finish(self: Box<Self>) -> Result<Reply, Failure> {
        if !self.done {
            return Err(Failure::cut(self.who));
        }
        let mut parts = Vec::new();
        let mut items = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for block in self.blocks.into_values() {
            match block {
                Block::Text(t) => {
                    // An empty text block is refused when sent back.
                    if !t.is_empty() {
                        items.push(text_block(&t).text());
                        parts.push(Part::Text(t));
                    }
                }
                Block::Thinking { text, signature } => {
                    items.push(
                        Obj::new()
                            .put("type", "thinking")
                            .put("thinking", text.as_str())
                            .put("signature", signature)
                            .text(),
                    );
                    parts.push(Part::Reasoning(Reasoning {
                        provider: self.kind,
                        model: self.model.clone(),
                        text,
                    }));
                }
                Block::Whole(v) => {
                    if v["type"] == "redacted_thinking" {
                        parts.push(Part::Reasoning(Reasoning {
                            provider: self.kind,
                            model: self.model.clone(),
                            text: String::new(),
                        }));
                    }
                    items.push(v.to_string());
                }
                Block::ToolUse { id, name, input } => {
                    let value =
                        parse_input(&input).ok_or_else(|| Failure::broken_call(self.who))?;
                    let id = if id.is_empty() || !seen.insert(id.clone()) {
                        fresh_call_id()
                    } else {
                        id
                    };
                    let raw = if input.trim().is_empty() {
                        "{}".to_owned()
                    } else {
                        input
                    };
                    items.push(
                        Obj::new()
                            .put("type", "tool_use")
                            .put("id", id.as_str())
                            .put("name", name.as_str())
                            .put("input", Json::Raw(raw))
                            .text(),
                    );
                    parts.push(Part::ToolCall(ToolCall {
                        id,
                        name: self.names.app(&name),
                        input: value,
                    }));
                }
            }
        }
        let stop = match self.stop_reason.as_deref() {
            Some("max_tokens") => Stop::Length,
            Some("model_context_window_exceeded") => Stop::ContextFull,
            _ => Stop::Done,
        };
        Ok(Reply {
            message: Message::from_provider(parts, Echo::new(self.kind, &self.model, items)),
            stop,
            usage: self
                .counted
                .then_some((self.input_tokens, self.output_tokens)),
        })
    }
}
