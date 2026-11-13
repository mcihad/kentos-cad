//! The four wire formats (docs/adr/8056 §4; the research note's “Dört protokol uyarlayıcısı”): Anthropic Messages,
//! OpenAI Responses (OpenAI, xAI), Chat Completions with a quirk profile per provider (DeepSeek, Kimi, GLM, Mistral, Qwen,
//! OpenRouter, Ollama, LM Studio, any compatible server) and Gemini's own `streamGenerateContent`. Each builds a request
//! from the neutral conversation and turns its server-sent events into live text, the assistant's message and the
//! provider's record of it ([`crate::conversation::Echo`]).

pub(crate) mod anthropic;
pub(crate) mod chat;
mod chat_reader;
pub(crate) mod gemini;
pub(crate) mod responses;

use std::sync::Arc;

use serde_json::Value;

use crate::conversation::{Conversation, Message};
use crate::failure::Failure;
use crate::profiles::AgentProfile;
use crate::providers::{Preset, Wire};
use crate::sse::SseEvent;
use crate::tools::{ToolNames, ToolSpec};

/// What a request is built from.
pub(crate) struct Call<'a> {
    pub profile: &'a AgentProfile,
    pub preset: Preset,
    /// The address without its trailing `/` (`https://api.anthropic.com/v1`).
    pub base: &'a str,
    pub key: Option<&'a str>,
    pub conversation: &'a Conversation,
    pub tools: &'a [ToolSpec],
    pub names: &'a Arc<ToolNames>,
}

/// A request ready to send.
pub(crate) struct Request {
    pub url: String,
    pub headers: Vec<(&'static str, String)>,
    /// Headers carrying the key: sent marked sensitive, never printed.
    pub secrets: Vec<(&'static str, String)>,
    pub body: String,
}

/// What an answer gives while it streams.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Live {
    Text(String),
    Reasoning(String),
}

/// Why the model stopped, beyond calling tools (which the message's calls say).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stop {
    Done,
    /// The output budget ran out: the text is cut short.
    Length,
    /// The conversation no longer fits the model.
    ContextFull,
}

/// An answer read to its end.
#[derive(Clone, Debug)]
pub(crate) struct Reply {
    pub message: Message,
    pub stop: Stop,
    /// Input and output tokens of this answer.
    pub usage: Option<(u64, u64)>,
}

/// Reads one answer's events.
pub(crate) trait Decoder: Send {
    /// One event; text and reasoning as they come go to `live`. A provider's error inside the stream is a failure.
    fn event(&mut self, event: &SseEvent, live: &mut Vec<Live>) -> Result<(), Failure>;
    /// The stream said it is over (`message_stop`, `response.completed`, `[DONE]`).
    fn done(&self) -> bool;
    /// The stream ended: the assistant's message.
    fn finish(self: Box<Self>) -> Result<Reply, Failure>;
}

/// The request for `call` in its provider's format.
pub(crate) fn request(call: &Call<'_>) -> Request {
    match call.preset.wire {
        Wire::AnthropicMessages => anthropic::request(call),
        Wire::OpenAiResponses => responses::request(call),
        Wire::ChatCompletions => chat::request(call),
        Wire::GeminiNative => gemini::request(call),
    }
}

/// A reader for the answer to `call`.
pub(crate) fn decoder(call: &Call<'_>) -> Box<dyn Decoder> {
    let names = Arc::clone(call.names);
    let who = call.preset.name;
    let kind = call.profile.kind;
    let model = call.profile.model.clone();
    match call.preset.wire {
        Wire::AnthropicMessages => Box::new(anthropic::Reader::new(kind, model, names, who)),
        Wire::OpenAiResponses => Box::new(responses::Reader::new(kind, model, names, who)),
        Wire::ChatCompletions => Box::new(chat::Reader::new(kind, model, names, who)),
        Wire::GeminiNative => Box::new(gemini::Reader::new(kind, model, names, who)),
    }
}

/// A tool call's input from the text the model wrote: none is an empty object; not JSON is none.
pub(crate) fn parse_input(text: &str) -> Option<Value> {
    if text.trim().is_empty() {
        return Some(Value::Object(serde_json::Map::new()));
    }
    serde_json::from_str(text).ok()
}

/// An id for a call the provider gave none (or the same one twice).
pub(crate) fn fresh_call_id() -> String {
    format!("kentos-{}", uuid::Uuid::new_v4().simple())
}

/// Whether an id is one this crate gave (Gemini's older models name no call; their result names none either).
pub(crate) fn ours(id: &str) -> bool {
    id.starts_with("kentos-")
}

/// A number field as `u64`.
pub(crate) fn count(v: &Value) -> u64 {
    v.as_u64().unwrap_or(0)
}
