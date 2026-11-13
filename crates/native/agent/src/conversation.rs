//! A conversation with an agent, the same whichever provider answers it (docs/adr/8056 §4): a system prompt and
//! messages; a user's text, an assistant's text, tool calls and reasoning, the tools' results.
//!
//! The research note's first trap is reasoning sent back wrong: Anthropic's thinking blocks with their signatures,
//! OpenAI's and xAI's encrypted reasoning items, Gemini's thought signatures, the `reasoning_content` of DeepSeek, Kimi,
//! GLM and Qwen must reach the same provider again unchanged, or the request is refused or quietly answered worse. So an
//! assistant message from a provider keeps, beside its neutral parts, the provider's own record of the turn ([`Echo`]):
//! its reasoning items, thinking blocks and signatures with the text and calls as the provider worded them, kept
//! verbatim and tagged with the provider and model. It goes back as it came to that provider and model only; for any
//! other the turn is rebuilt from its parts without it. [`Conversation::for_profile`] gives a conversation without the
//! foreign ones, for a switch of agents in the middle of a chat.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::profiles::AgentProfile;
use crate::providers::ProviderKind;

/// The system prompt and the messages so far.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    /// The instructions every request starts with (empty: none).
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub messages: Vec<Message>,
}

/// Who a message is from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Assistant,
    /// The results of the assistant's tool calls just before it.
    Tool,
}

/// A message: its role and parts in order; an assistant's also the provider's record of it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub parts: Vec<Part>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    echo: Option<Echo>,
}

/// A part of a message.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Part {
    Text(String),
    /// An assistant's call of a tool.
    ToolCall(ToolCall),
    /// A tool's result, in a [`Role::Tool`] message.
    ToolResult(ToolResult),
    /// The assistant's reasoning, as far as the provider shows it; bound to the provider and model that produced it.
    Reasoning(Reasoning),
}

/// A call of a tool: the provider's id for it, the app's tool name and the input the model wrote.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub input: Value,
}

/// A tool's result for the call with the same id.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolResult {
    pub id: String,
    /// The app's tool name.
    pub name: String,
    /// What the tool gave; for an error, its message as a string.
    pub output: Value,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_error: bool,
}

/// What can be shown of an assistant's reasoning (a summary, the thinking text; often empty when the provider keeps it
/// to itself), tagged with the provider and model that produced it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reasoning {
    pub provider: ProviderKind,
    pub model: String,
    pub text: String,
}

/// The provider's own record of an assistant turn: its items (content blocks, output items, parts, the message) as JSON
/// text, verbatim and in order, with the provider and model that wrote them. Only the adapters read it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Echo {
    provider: ProviderKind,
    model: String,
    items: Vec<String>,
}

impl Echo {
    pub(crate) fn new(provider: ProviderKind, model: &str, items: Vec<String>) -> Echo {
        Echo {
            provider,
            model: model.to_owned(),
            items,
        }
    }

    pub fn provider(&self) -> ProviderKind {
        self.provider
    }

    pub fn model(&self) -> &str {
        &self.model
    }

    pub(crate) fn items(&self) -> &[String] {
        &self.items
    }

    fn belongs_to(&self, provider: ProviderKind, model: &str) -> bool {
        self.provider == provider && self.model == model
    }
}

impl ToolResult {
    /// A tool's answer as the desktop gives it: a value, or an error's message.
    pub fn from_answer(call: &ToolCall, answer: Result<Value, String>) -> ToolResult {
        let (output, is_error) = match answer {
            Ok(v) => (v, false),
            Err(message) => (Value::String(message), true),
        };
        ToolResult {
            id: call.id.clone(),
            name: call.name.clone(),
            output,
            is_error,
        }
    }

    /// The result as text, for providers that take a string: a string as it is, anything else as compact JSON; an error
    /// as `{"error": …}` where the provider has no error flag (`flagged`: it has one).
    pub(crate) fn text(&self, flagged: bool) -> String {
        if self.is_error && !flagged {
            return serde_json::json!({ "error": self.output }).to_string();
        }
        match &self.output {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        }
    }
}

impl Message {
    pub fn user(text: impl Into<String>) -> Message {
        Message {
            role: Role::User,
            parts: vec![Part::Text(text.into())],
            echo: None,
        }
    }

    /// An assistant message from its parts, with no provider's record (one the app writes, say).
    pub fn assistant(parts: Vec<Part>) -> Message {
        Message {
            role: Role::Assistant,
            parts,
            echo: None,
        }
    }

    pub(crate) fn from_provider(parts: Vec<Part>, echo: Echo) -> Message {
        Message {
            role: Role::Assistant,
            parts,
            echo: Some(echo),
        }
    }

    pub fn tool_results(results: Vec<ToolResult>) -> Message {
        Message {
            role: Role::Tool,
            parts: results.into_iter().map(Part::ToolResult).collect(),
            echo: None,
        }
    }

    /// The provider's record of this turn, when it has one.
    pub fn echo(&self) -> Option<&Echo> {
        self.echo.as_ref()
    }

    /// The record, when it is the given provider's and model's (only then does it go back verbatim).
    pub(crate) fn echo_for(&self, provider: ProviderKind, model: &str) -> Option<&Echo> {
        self.echo.as_ref().filter(|e| e.belongs_to(provider, model))
    }

    /// The message's text parts, joined.
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| match p {
                Part::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect()
    }

    pub fn tool_calls(&self) -> impl Iterator<Item = &ToolCall> {
        self.parts.iter().filter_map(|p| match p {
            Part::ToolCall(c) => Some(c),
            _ => None,
        })
    }

    pub fn tool_results_iter(&self) -> impl Iterator<Item = &ToolResult> {
        self.parts.iter().filter_map(|p| match p {
            Part::ToolResult(r) => Some(r),
            _ => None,
        })
    }

    /// The reasoning shown, joined.
    pub fn reasoning_text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|p| match p {
                Part::Reasoning(r) => Some(r.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Without what is bound to a provider and model other than these: their record and their reasoning.
    fn keep_own(&mut self, provider: ProviderKind, model: &str) {
        if self
            .echo
            .as_ref()
            .is_some_and(|e| !e.belongs_to(provider, model))
        {
            self.echo = None;
        }
        self.parts.retain(|p| match p {
            Part::Reasoning(r) => r.provider == provider && r.model == model,
            _ => true,
        });
    }
}

impl Conversation {
    pub fn new(system: impl Into<String>) -> Conversation {
        Conversation {
            system: system.into(),
            messages: Vec::new(),
        }
    }

    pub fn push_user(&mut self, text: impl Into<String>) {
        self.messages.push(Message::user(text));
    }

    /// The conversation as `profile`'s agent may take it: every record and reasoning of another provider or model
    /// left out (a switch of agents in the middle of a chat). Its own stay.
    pub fn for_profile(&self, profile: &AgentProfile) -> Conversation {
        self.keeping(profile.kind, &profile.model)
    }

    /// The same for a provider and model.
    pub fn keeping(&self, provider: ProviderKind, model: &str) -> Conversation {
        let mut out = self.clone();
        for m in &mut out.messages {
            m.keep_own(provider, model);
        }
        out
    }

    /// The conversation with no provider's record or reasoning at all: text, calls and results only.
    pub fn without_reasoning(&self) -> Conversation {
        let mut out = self.clone();
        for m in &mut out.messages {
            m.echo = None;
            m.parts.retain(|p| !matches!(p, Part::Reasoning(_)));
        }
        out
    }

    /// Whether the model is to speak next: the last message is the user's or the tools' results.
    pub fn awaits_reply(&self) -> bool {
        self.messages
            .last()
            .is_some_and(|m| m.role != Role::Assistant)
    }

    /// The last assistant message's text.
    pub fn last_answer(&self) -> Option<String> {
        self.messages
            .iter()
            .rev()
            .find(|m| m.role == Role::Assistant)
            .map(Message::text)
    }

    /// Every tool name the conversation's calls use, in order (for a turn's name map).
    pub(crate) fn tool_names(&self) -> impl Iterator<Item = &str> {
        self.messages
            .iter()
            .flat_map(|m| m.parts.iter())
            .filter_map(|p| match p {
                Part::ToolCall(c) => Some(c.name.as_str()),
                Part::ToolResult(r) => Some(r.name.as_str()),
                _ => None,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn turn(provider: ProviderKind, model: &str) -> Message {
        Message::from_provider(
            vec![
                Part::Reasoning(Reasoning {
                    provider,
                    model: model.into(),
                    text: "düşünce".into(),
                }),
                Part::Text("tamam".into()),
                Part::ToolCall(ToolCall {
                    id: "c1".into(),
                    name: "cad.run".into(),
                    input: json!({"mode": "plan"}),
                }),
            ],
            Echo::new(provider, model, vec!["{\"type\":\"thinking\"}".into()]),
        )
    }

    #[test]
    fn a_switch_keeps_the_new_agents_own_reasoning_and_drops_the_rest() {
        let mut c = Conversation::new("sistem");
        c.push_user("merhaba");
        c.messages
            .push(turn(ProviderKind::Anthropic, "claude-opus-5-5"));
        c.messages
            .push(turn(ProviderKind::DeepSeek, "deepseek-v4-pro"));
        let p = AgentProfile::new(ProviderKind::DeepSeek, "DeepSeek", "deepseek-v4-pro");
        let kept = c.for_profile(&p);
        assert!(kept.messages[1].echo().is_none());
        assert_eq!(kept.messages[1].reasoning_text(), "");
        assert_eq!(kept.messages[1].text(), "tamam");
        assert_eq!(kept.messages[1].tool_calls().count(), 1);
        assert_eq!(
            kept.messages[2].echo().map(Echo::provider),
            Some(ProviderKind::DeepSeek)
        );
        assert_eq!(kept.messages[2].reasoning_text(), "düşünce");
        // Another model of the same provider is foreign too.
        let other = c.keeping(ProviderKind::DeepSeek, "deepseek-flash");
        assert!(
            other
                .messages
                .iter()
                .all(|m| m.echo().is_none() && m.reasoning_text().is_empty())
        );
        let plain = c.without_reasoning();
        assert!(plain.messages.iter().all(|m| m.echo().is_none()));
        assert_eq!(plain.messages[2].tool_calls().count(), 1);
        // Kept and read back whole, records included.
        let text = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Conversation>(&text).unwrap(), c);
        assert!(!c.awaits_reply());
        assert_eq!(c.last_answer().as_deref(), Some("tamam"));
    }

    #[test]
    fn results_read_as_text_with_or_without_an_error_flag() {
        let call = ToolCall {
            id: "c".into(),
            name: "cad.run".into(),
            input: json!({}),
        };
        let ok = ToolResult::from_answer(&call, Ok(json!({"n": 2})));
        assert_eq!(ok.text(false), r#"{"n":2}"#);
        let text = ToolResult::from_answer(&call, Ok(json!("düz")));
        assert_eq!(text.text(false), "düz");
        let err = ToolResult::from_answer(&call, Err("yok".into()));
        assert!(err.is_error);
        assert_eq!(err.text(true), "yok");
        assert_eq!(err.text(false), r#"{"error":"yok"}"#);
    }
}
