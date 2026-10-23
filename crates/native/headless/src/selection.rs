//! A session's explicit selection, shared by Python, MCP and the desktop's links.
//! This is view state: it never changes the document, revision, undo stack or saved file.
use std::collections::HashSet;
use kentos_domain::{Document, Slot, Uuid};
use serde_json::Value;
use crate::HeadlessError;

/// Resolve the complete list before changing anything. Duplicates keep their first place.
pub fn resolve(doc: &Document, given: &Value) -> Result<Vec<Slot>, HeadlessError> {
    let list = given.as_array().ok_or_else(|| {
        HeadlessError::new(
            "invalid_input",
            "uids bir nesne kimliği listesi olmalı; seçimi kaldırmak için [] verin.",
        )
    })?;
    let mut seen = HashSet::new();
    let mut slots = Vec::with_capacity(list.len());
    for value in list {
        let text = value.as_str().ok_or_else(|| {
            HeadlessError::new(
                "invalid_input",
                "uids listesinin her öğesi bir nesnenin UUID metni olmalı; seçim değişmedi.",
            )
        })?;
        let slot = Uuid::parse_str(text)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .ok_or_else(|| {
                HeadlessError::new(
                    "unknown_object",
                    format!("“{text}” kimlikli nesne çizimde yok; seçim değişmedi."),
                )
            })?;
        if seen.insert(slot) {
            slots.push(slot);
        }
    }
    Ok(slots)
}

/// Persistent ids for the still existing objects, in selection order.
pub fn uids(doc: &Document, slots: &[Slot]) -> Vec<String> {
    slots
        .iter()
        .filter_map(|s| doc.uid(*s))
        .map(|u| u.to_string())
        .collect()
}
