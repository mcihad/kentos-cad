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

    /// The agents' extension workshop beside this app's extension folder (docs/adr/8057 §22).
    fn workshop(&self) -> Option<std::path::PathBuf> {
        self.0
            .extensions
            .root
            .as_deref()
            .and_then(kentos_extension_host::places::workshop_for)
    }

    /// Eklentiler's review of the package, as when the user opens a `.kek`: nothing is installed by it.
    fn review_package(&mut self, path: &std::path::Path) -> Result<Value, HeadlessError> {
        self.0.extensions.note = None;
        self.0.review_kek_file(path);
        if let Some((true, words)) = &self.0.extensions.note {
            return Err(HeadlessError::new("invalid_input", words.clone()));
        }
        let review = self.0.extensions.review.as_ref().map(|r| &r.review);
        Ok(json!({
            "opened": review.is_some(),
            "installable": review.is_some_and(|r| r.installable),
            "where": "Eklentiler › Gözden geçir ve kur",
        }))
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
