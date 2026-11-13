//! The Responses API of OpenAI and xAI (docs/adr/8056 §4; the research note's §2, §8 and traps 8–11): `POST
//! {base}/responses` with a bearer key, typed server-sent events. Stateless: `store: false` (the provider would keep
//! every answer otherwise) with `include: ["reasoning.encrypted_content"]`, and every output item of an answer — the
//! encrypted reasoning items, the messages, the function calls with their ids — goes back verbatim in the next input.
//! Tools are flat (`name` at the top), never strict; `tool_choice` auto; no sampling parameter.

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

pub(crate) fn request(call: &Call<'_>) -> Request {
    let mut input: Vec<Json> = Vec::new();
    for m in &call.conversation.messages {
        match m.role {
            Role::User => {
                let text = m.text();
                if !text.is_empty() {
                    input.push(Obj::new().put("role", "user").put("content", text).into());
                }
            }
            Role::Assistant => match m.echo_for(call.profile.kind, &call.profile.model) {
                Some(echo) => input.extend(echo.items().iter().map(|i| Json::Raw(i.clone()))),
                None => {
                    // Another provider's turn, or the app's: its text and calls, no reasoning.
                    let text = m.text();
                    if !text.is_empty() {
                        input.push(
                            Obj::new()
                                .put("role", "assistant")
                                .put("content", text)
                                .into(),
                        );
                    }
                    for c in m.tool_calls() {
                        input.push(
                            Obj::new()
                                .put("type", "function_call")
                                .put("call_id", c.id.as_str())
                                .put("name", call.names.wire(&c.name))
                                .put("arguments", c.input.to_string())
                                .into(),
                        );
                    }
                }
            },
            Role::Tool => {
                for r in m.tool_results_iter() {
                    input.push(
                        Obj::new()
                            .put("type", "function_call_output")
                            .put("call_id", r.id.as_str())
                            .put("output", r.text(false))
                            .into(),
                    );
                }
            }
        }
    }
    let tools: Vec<Json> = call
        .tools
        .iter()
        .map(|t| {
            Obj::new()
                .put("type", "function")
                .put("name", call.names.wire(&t.name))
                .put("description", t.description.as_str())
                .put(
                    "parameters",
                    schema_for(Wire::OpenAiResponses, &t.input_schema),
                )
                .put("strict", false)
                .into()
        })
        .collect();
    let system = &call.conversation.system;
    let offered = !tools.is_empty();
    let body = Obj::new()
        .put("model", call.profile.model.as_str())
        .put_some(
            "instructions",
            (!system.is_empty()).then_some(system.as_str()),
        )
        .put("input", input)
        .put_some("tools", offered.then_some(tools))
        .put_some("tool_choice", offered.then_some("auto"))
        .put("store", false)
        .put("include", vec![Json::from("reasoning.encrypted_content")])
        .put("stream", true)
        .text();
    Request {
        url: format!("{}/responses", call.base),
        headers: Vec::new(),
        secrets: call
            .key
            .map(|k| vec![("authorization", format!("Bearer {k}"))])
            .unwrap_or_default(),
        body,
    }
}

/// Reads the typed events: text and reasoning deltas as they come, the finished output items, the end.
pub(crate) struct Reader {
    kind: ProviderKind,
    model: String,
    names: Arc<ToolNames>,
    who: &'static str,
    /// Finished items by their output index.
    items: BTreeMap<u64, Value>,
    /// The answer's output as `response.completed` gives it.
    output: Vec<Value>,
    incomplete: Option<String>,
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
            items: BTreeMap::new(),
            output: Vec::new(),
            incomplete: None,
            usage: None,
            done: false,
        }
    }

    fn end(&mut self, response: &Value) {
        if let Some(out) = response["output"].as_array() {
            self.output.clone_from(out);
        }
        let u = &response["usage"];
        if u.is_object() {
            self.usage = Some((count(&u["input_tokens"]), count(&u["output_tokens"])));
        }
        self.done = true;
    }
}

impl Decoder for Reader {
    fn event(&mut self, event: &SseEvent, live: &mut Vec<Live>) -> Result<(), Failure> {
        let Ok(v) = serde_json::from_str::<Value>(&event.data) else {
            return Ok(());
        };
        let kind = v["type"].as_str().unwrap_or(event.event.as_str());
        let delta = || v["delta"].as_str().unwrap_or_default().to_owned();
        match kind {
            "response.output_text.delta" | "response.refusal.delta" => {
                live.push(Live::Text(delta()))
            }
            "response.reasoning_summary_text.delta" | "response.reasoning_text.delta" => {
                live.push(Live::Reasoning(delta()));
            }
            "response.output_item.done" => {
                self.items
                    .insert(count(&v["output_index"]), v["item"].clone());
            }
            "response.completed" => self.end(&v["response"]),
            "response.incomplete" => {
                let reason = v["response"]["incomplete_details"]["reason"]
                    .as_str()
                    .unwrap_or("bilinmiyor");
                self.incomplete = Some(reason.to_owned());
                self.end(&v["response"]);
            }
            "response.failed" => {
                let e = &v["response"]["error"];
                let what = format!(
                    "{} ({})",
                    e["message"].as_str().unwrap_or("yanıt üretilemedi"),
                    e["code"].as_str().unwrap_or("failed")
                );
                return Err(Failure::in_stream(self.who, &what, None));
            }
            "error" => {
                let e = if v["error"].is_object() {
                    &v["error"]
                } else {
                    &v
                };
                let what = format!(
                    "{} ({})",
                    e["message"].as_str().unwrap_or("bilinmeyen hata"),
                    e["code"].as_str().or(e["type"].as_str()).unwrap_or("error")
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
        // The completed response's output is the whole answer; the items seen finishing stand in when it has fewer.
        let output: Vec<Value> = if self.output.len() >= self.items.len() {
            self.output
        } else {
            self.items.into_values().collect()
        };
        let mut parts = Vec::new();
        let mut items = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for item in output {
            match item["type"].as_str() {
                Some("reasoning") => {
                    let said = |field: &str| -> Vec<String> {
                        item[field]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(|s| s["text"].as_str().map(str::to_owned))
                            .collect()
                    };
                    let mut text = said("summary");
                    if text.is_empty() {
                        text = said("content");
                    }
                    parts.push(Part::Reasoning(Reasoning {
                        provider: self.kind,
                        model: self.model.clone(),
                        text: text.join("\n"),
                    }));
                }
                Some("message") => {
                    let text: String = item["content"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(|c| c["text"].as_str().or(c["refusal"].as_str()))
                        .collect();
                    if !text.is_empty() {
                        parts.push(Part::Text(text));
                    }
                }
                Some("function_call") => {
                    let arguments = item["arguments"].as_str().unwrap_or_default();
                    let input =
                        parse_input(arguments).ok_or_else(|| Failure::broken_call(self.who))?;
                    let id = item["call_id"].as_str().unwrap_or_default().to_owned();
                    let id = if id.is_empty() || !seen.insert(id.clone()) {
                        fresh_call_id()
                    } else {
                        id
                    };
                    parts.push(Part::ToolCall(ToolCall {
                        id,
                        name: self.names.app(item["name"].as_str().unwrap_or_default()),
                        input,
                    }));
                }
                _ => {}
            }
            items.push(item.to_string());
        }
        let stop = match self.incomplete.as_deref() {
            Some("max_output_tokens") => Stop::Length,
            _ => Stop::Done,
        };
        Ok(Reply {
            message: Message::from_provider(parts, Echo::new(self.kind, &self.model, items)),
            stop,
            usage: self.usage,
        })
    }
}
