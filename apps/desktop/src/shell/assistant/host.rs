//! The assistant's tools on the drawing in front (docs/adr/8056 §4): the agents' tools (`kentos_headless::agent_tools`)
//! with this app as their host. A request goes through the agents link's core (`App::agent_request`), so the program's
//! offer, the fields the drawing hides and the log's “Ajan: …” lines are the link's; the assistant needs no link.

use kentos_headless::HeadlessError;
use kentos_headless::agent_tools::Host;
use serde_json::{Value, json};

use crate::app::App;

/// This app as the agents' tools' host.
pub(crate) struct AppHost<'a>(pub(crate) &'a mut App);

impl Host for AppHost<'_> {
    fn request(&mut self, method: &str, params: &Value) -> Result<Value, HeadlessError> {
        self.0
            .agent_request(&json!({ "method": method, "params": params }), "Asistan")
            .map_err(|(code, message)| HeadlessError::new(stable(&code), message))
    }

    fn not_offered(&self, command: &str) -> Option<String> {
        self.0.product_not_offered(command)
    }
}

/// A refusal's code as a host's stable one; another is the desktop's own (`desktop_refused`).
fn stable(code: &str) -> &'static str {
    const KNOWN: &[&str] = &[
        "not_offered",
        "busy",
        "no_document",
        "document_closed",
        "invalid_request",
        "invalid_input",
        "unknown_method",
        "unknown_command",
        "unknown_object",
    ];
    KNOWN
        .iter()
        .find(|k| **k == code)
        .copied()
        .unwrap_or("desktop_refused")
}
