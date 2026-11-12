//! Selection requests of both Python and MCP use the window's same state and native resolver.
use kentos_domain::Document;
use kentos_headless::HeadlessError;
use kentos_interaction::Selection;
use serde_json::{Value, json};

pub(super) fn answer(
    doc: &Document,
    selected: &mut Selection,
    method: &str,
    params: &Value,
) -> Result<Value, HeadlessError> {
    if method == "select" {
        let slots = kentos_headless::selection::resolve(doc, &params["uids"])?;
        selected.set(slots);
        Ok(json!(selected.len()))
    } else {
        Ok(json!(kentos_headless::selection::uids(doc, selected.ids())))
    }
}
