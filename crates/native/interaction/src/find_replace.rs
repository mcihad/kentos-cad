//! Bul ve değiştir's matches and write (docs/adr/0145 §6; the web's
//! `ui/text/findReplace.ts`): the texts in scope whose words the core's
//! rule changes (`text::edit::replace`), each with its layer and whether it
//! may be written. A text on a locked layer is listed but not written; one
//! that would become empty neither (a text is never empty). The changed
//! texts go in one step “Bul ve değiştir” (`cad.entities.edit`'s
//! `replaceText`). The window is the desktop's `find_replace`.

use kentos_contracts::Entity;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::text::edit::{Find, replace};

use crate::properties;

/// Why a match is not written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Blocked {
    /// Its layer is locked.
    Locked,
    /// It would become empty.
    Empty,
}

/// A text whose words change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub slot: Slot,
    /// Its layer's path, as Katmanlar names it.
    pub layer: String,
    pub old: String,
    /// What it becomes, trimmed.
    pub new: String,
    pub blocked: Option<Blocked>,
}

/// What is looked for.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Query {
    pub find: String,
    pub replace: String,
    pub how: Find,
}

/// The matches among the texts in `scope` (every text of the drawing when none), in the drawing's order.
pub fn matches(doc: &Document, scope: Option<&[Slot]>, q: &Query) -> Vec<Match> {
    if q.find.is_empty() {
        return Vec::new();
    }
    let texts: Vec<(Slot, &kentos_contracts::TextEntity)> = match scope {
        Some(slots) => slots
            .iter()
            .filter_map(|&s| match doc.get(s) {
                Some(Entity::Text(t)) => Some((s, t)),
                _ => None,
            })
            .collect(),
        None => doc
            .entities()
            .filter_map(|e| match e {
                Entity::Text(t) => Some((Slot(t.base.id), t)),
                _ => None,
            })
            .collect(),
    };
    let layers = doc.layers();
    texts
        .into_iter()
        .filter_map(|(slot, t)| {
            let next = replace(&t.text, &q.find, &q.replace, q.how)?;
            let next = next.trim().to_owned();
            if next == t.text {
                return None;
            }
            let blocked = if layers.is_locked(&t.base.layer_id) {
                Some(Blocked::Locked)
            } else if next.is_empty() {
                Some(Blocked::Empty)
            } else {
                None
            };
            Some(Match {
                slot,
                layer: layers.path(&t.base.layer_id),
                old: t.text.clone(),
                new: next,
                blocked,
            })
        })
        .collect()
}

/// The matches that may be written, written in one step “Bul ve değiştir”;
/// how many and what to say (the command's refusal).
pub fn write(doc: &mut Document, picked: &[Match]) -> (usize, Vec<String>) {
    let changes: Vec<(Slot, String)> = picked
        .iter()
        .filter(|m| m.blocked.is_none())
        .map(|m| (m.slot, m.new.clone()))
        .collect();
    let n = changes.len();
    if n == 0 {
        return (0, Vec::new());
    }
    let said = properties::replace_texts(doc, &changes);
    (if said.is_empty() { n } else { 0 }, said)
}
