//! The Chat Completions family (docs/adr/8056 §4; the research note's §3, §5–7, §9–12 and “Sağlayıcı profili”):
//! DeepSeek, Kimi, GLM, Mistral, Qwen, OpenRouter, Ollama, LM Studio and any compatible server, `POST
//! {base}/chat/completions` with a bearer key when there is one, data-only server-sent events ended by `[DONE]`. What
//! each does its own way is a [`Quirks`] profile: how usage is asked for, whether `tool_choice` is taken, GLM's
//! `tool_stream`, which reasoning goes back (`reasoning_content`, OpenRouter's `reasoning_details`, Mistral's thinking
//! chunks), Kimi's and Mistral's tool name in a result, Mistral's nine-character call ids. No sampling parameter.

use super::{Call, Request};
use crate::conversation::Role;
use crate::json::{Json, Obj};
use crate::providers::{ProviderKind, Wire};
use crate::tools::{fnv1a, schema_for};

pub(crate) use super::chat_reader::Reader;

/// What one provider of the family does its own way.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Quirks {
    /// `stream_options.include_usage`, without which no usage comes (compatible servers, DeepSeek, Kimi, Qwen, Ollama).
    pub stream_usage: bool,
    /// `tool_choice: "auto"` is sent (Ollama takes no `tool_choice`, LM Studio does not say it does).
    pub tool_choice: bool,
    /// `tool_stream: true`: GLM streams a call's arguments only when asked.
    pub tool_stream: bool,
    /// The assistant's `reasoning_content` goes back: DeepSeek refuses a tool request without every turn's (trap 14),
    /// Kimi, GLM and Qwen keep their thinking across tool calls with it.
    pub reasoning_content: bool,
    /// OpenRouter's `reasoning_details` go back (trap 35).
    pub reasoning_details: bool,
    /// Mistral's thinking comes as content chunks and goes back as them (trap 30).
    pub thinking_chunks: bool,
    /// A tool's result names its tool (Kimi, Mistral).
    pub tool_name: bool,
    /// A call id is nine letters and digits (Mistral, trap 29); another provider's ids are mapped to such.
    pub nine_char_ids: bool,
    /// An output budget; Kimi's thinking wants at least 16 000 (its default is not documented).
    pub max_tokens: Option<u64>,
}

pub(crate) fn quirks(kind: ProviderKind) -> Quirks {
    let base = Quirks {
        tool_choice: true,
        ..Quirks::default()
    };
    match kind {
        ProviderKind::DeepSeek => Quirks {
            stream_usage: true,
            reasoning_content: true,
            ..base
        },
        ProviderKind::Kimi => Quirks {
            stream_usage: true,
            reasoning_content: true,
            tool_name: true,
            max_tokens: Some(32_000),
            ..base
        },
        ProviderKind::Glm => Quirks {
            tool_stream: true,
            reasoning_content: true,
            ..base
        },
        ProviderKind::Mistral => Quirks {
            thinking_chunks: true,
            tool_name: true,
            nine_char_ids: true,
            ..base
        },
        ProviderKind::Qwen => Quirks {
            stream_usage: true,
            reasoning_content: true,
            ..base
        },
        ProviderKind::OpenRouter => Quirks {
            reasoning_details: true,
            ..base
        },
        ProviderKind::Ollama => Quirks {
            stream_usage: true,
            tool_choice: false,
            ..base
        },
        ProviderKind::LmStudio => Quirks {
            tool_choice: false,
            ..base
        },
        _ => Quirks {
            stream_usage: true,
            ..base
        },
    }
}

pub(crate) fn request(call: &Call<'_>) -> Request {
    let kind = call.profile.kind;
    let q = quirks(kind);
    let mut messages: Vec<Json> = Vec::new();
    let system = &call.conversation.system;
    if !system.is_empty() {
        messages.push(
            Obj::new()
                .put("role", "system")
                .put("content", system.as_str())
                .into(),
        );
    }
    for m in &call.conversation.messages {
        match m.role {
            Role::User => {
                let text = m.text();
                if !text.is_empty() {
                    messages.push(Obj::new().put("role", "user").put("content", text).into());
                }
            }
            Role::Assistant => match m.echo_for(kind, &call.profile.model) {
                Some(echo) => messages.extend(echo.items().iter().map(|i| Json::Raw(i.clone()))),
                None => {
                    // Another provider's turn, or the app's: its text and calls, no reasoning.
                    let text = m.text();
                    let calls: Vec<Json> = m
                        .tool_calls()
                        .map(|c| {
                            call_json(
                                &call_id(&q, &c.id),
                                &call.names.wire(&c.name),
                                &c.input.to_string(),
                            )
                        })
                        .collect();
                    if text.is_empty() && calls.is_empty() {
                        continue;
                    }
                    let offered = !calls.is_empty();
                    messages.push(
                        Obj::new()
                            .put("role", "assistant")
                            .put("content", text)
                            .put_some("tool_calls", offered.then_some(calls))
                            .into(),
                    );
                }
            },
            Role::Tool => {
                for r in m.tool_results_iter() {
                    messages.push(
                        Obj::new()
                            .put("role", "tool")
                            .put("tool_call_id", call_id(&q, &r.id))
                            .put_some("name", q.tool_name.then(|| call.names.wire(&r.name)))
                            .put("content", r.text(false))
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
            let function = Obj::new()
                .put("name", call.names.wire(&t.name))
                .put("description", t.description.as_str())
                .put(
                    "parameters",
                    schema_for(Wire::ChatCompletions, &t.input_schema),
                );
            Obj::new()
                .put("type", "function")
                .put("function", function)
                .into()
        })
        .collect();
    let offered = !tools.is_empty();
    let body = Obj::new()
        .put("model", call.profile.model.as_str())
        .put("messages", messages)
        .put_some("tools", offered.then_some(tools))
        .put_some("tool_choice", (offered && q.tool_choice).then_some("auto"))
        .put_some("tool_stream", (offered && q.tool_stream).then_some(true))
        .put_some("max_tokens", q.max_tokens)
        .put("stream", true)
        .put_some(
            "stream_options",
            q.stream_usage
                .then(|| Obj::new().put("include_usage", true)),
        )
        .text();
    let mut headers = Vec::new();
    if kind == ProviderKind::OpenRouter {
        // The application's name on the provider's usage page.
        headers.push(("x-openrouter-title", "KentOS CAD".to_owned()));
    }
    Request {
        url: format!("{}/chat/completions", call.base),
        headers,
        secrets: call
            .key
            .map(|k| vec![("authorization", format!("Bearer {k}"))])
            .unwrap_or_default(),
        body,
    }
}

/// A call as an assistant message carries it.
pub(crate) fn call_json(id: &str, name: &str, arguments: &str) -> Json {
    let function = Obj::new().put("name", name).put("arguments", arguments);
    Obj::new()
        .put("id", id)
        .put("type", "function")
        .put("function", function)
        .into()
}

/// A call's id as the provider takes it: Mistral's nine letters and digits (one of another form mapped the same way
/// for the call and its result), the others' as it is.
pub(crate) fn call_id(q: &Quirks, id: &str) -> String {
    let fits = id.len() == 9 && id.chars().all(|c| c.is_ascii_alphanumeric());
    if q.nine_char_ids && !fits {
        nine(id)
    } else {
        id.to_owned()
    }
}

/// Nine letters and digits from a hash of `seed`.
pub(crate) fn nine(seed: &str) -> String {
    const DIGITS: &[u8; 62] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let mut n = fnv1a(seed.as_bytes(), 9);
    (0..9)
        .map(|_| {
            let c = DIGITS[(n % 62) as usize];
            n /= 62;
            char::from(c)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mistral_ids_are_nine_letters_and_digits_and_others_stay() {
        let m = quirks(ProviderKind::Mistral);
        assert_eq!(call_id(&m, "D681PevKs"), "D681PevKs");
        let mapped = call_id(&m, "toolu_01A09q90qw90lq917835lq9");
        assert_eq!(mapped.len(), 9);
        assert!(mapped.chars().all(|c| c.is_ascii_alphanumeric()));
        assert_eq!(
            mapped,
            call_id(&m, "toolu_01A09q90qw90lq917835lq9"),
            "the call and its result agree"
        );
        assert_ne!(mapped, call_id(&m, "search:0"));
        assert_eq!(call_id(&quirks(ProviderKind::Kimi), "search:0"), "search:0");
    }
}
