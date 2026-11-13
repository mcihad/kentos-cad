//! The in-app AI agent's provider layer (docs/adr/8056 §4): the user's own agents and their keys, the providers they
//! talk to, and a chat turn that lets a model call the app's tools. No UI here; the desktop shows the settings and the
//! chat window and runs the tools on the open drawing. The design follows the research note
//! docs/research/2026-11-13-yapay-zeka-saglayicilari.md (“Soyutlama önerisi”, “Tuzaklar”).
//!
//! - [`providers`]: the thirteen kinds of provider and what the settings window offers for each ([`Preset`]).
//! - [`profiles`]: the device's agents ([`ProfileStore`], a file readable by the user alone) and where a key comes
//!   from: the file, this session, the provider's environment variable.
//! - [`conversation`]: the chat, the same whichever provider answers it, with each provider's record of its turns
//!   kept to go back to it verbatim.
//! - [`tools`]: a tool as the app offers it ([`ToolSpec`]); names and schemas are mapped per provider inside.
//! - [`run`]: [`start_turn`] and its [`Turn`]: the events of a turn, the answers to its tool calls, cancelling.
//! - [`models`]: [`list_models`] and [`check`] (the settings window's “Bağlantıyı dene”).
//!
//! Every call runs on the crate's own tokio runtime (two threads named `kentos-ajan`) and is awaited through channels,
//! so any executor can wait on it. A key never shows in `Debug`, in an error or in a log.
#![forbid(unsafe_code)]
// What a provider sends must never crash the desktop.
#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod conversation;
pub mod failure;
mod http;
mod json;
pub mod models;
pub mod profiles;
pub mod providers;
pub mod run;
mod runtime;
mod sse;
pub mod tools;
mod wire;

pub use conversation::{Conversation, Echo, Message, Part, Reasoning, Role, ToolCall, ToolResult};
pub use failure::{Failure, FailureKind};
pub use models::{check, list_models};
pub use profiles::{AgentProfile, KeySource, ProfileField, ProfileStore};
pub use providers::{Preset, ProviderKind, Wire};
pub use run::{Event, MAX_TOOL_ROUNDS, Turn, start_turn};
pub use tools::ToolSpec;
