//! View marks read their targets (docs/adr/0345 §2). A mark's bubble writes its number on top and its target's number
//! below: a named view's name, or the drawing number of the sheet its map frame is on (`drawingInfo.number`, “A-301”;
//! its name while it has none, docs/adr/0721 §1), so renumbering the set renumbers the marks. The targets are project
//! settings and the
//! sheet book, neither an undo step, so the words are derived: written again
//!
//! - at a step's end, for the marks the step added or changed (inside the step);
//! - outside any step, in place, when the named views or the book change, and after an undo or a redo: every mark whose
//!   words or break no longer hold. Such a write is no undo step of its own; the changes reach the readers that follow
//!   the document (`changes_since`) like any other.
//!
//! A target gone leaves the last words and `broken: targetGone`; no target writes “—” below.
//! `scripts/fixtures/viewmark_command_cases.py` writes the same without KentOS's code.

use std::collections::BTreeSet;
use std::sync::Arc;

use kentos_contracts::{
    Entity, NO_TARGET, ProjectSheets, ViewMarkBreak, ViewMarkFields, ViewMarkLabel, ViewTarget, view_key,
};

use crate::document::Document;
use crate::history::Op;
use crate::identity::Uuid;
use crate::store::Stored;

/// A sheet of the book as the marks read it: its id, name and drawing number and its map frames' ids and names, in
/// order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SheetFrames {
    pub id: String,
    pub name: String,
    /// Its drawing number (`drawingInfo.number`), empty when it has none.
    pub number: String,
    pub frames: Vec<(String, String)>,
}

impl SheetFrames {
    /// What a mark's bubble writes below for this sheet (docs/adr/0721 §1): its drawing number, else its name.
    pub fn label(&self) -> &str {
        if self.number.is_empty() { &self.name } else { &self.number }
    }
}

/// The book's sheets and their map frames, in the book's order (the sheet core's `kentos.sheet/1` JSON, read as a
/// JSON value: `sheets[].{id, name, drawingInfo.number, items[].{id, name, kind.type}}`).
pub fn sheet_frames(sheets: Option<&ProjectSheets>) -> Vec<SheetFrames> {
    let Some(list) = sheets.and_then(|s| s.book.get("sheets")).and_then(|v| v.as_array()) else {
        return Vec::new();
    };
    let text = |v: &serde_json::Value, k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_owned();
    list.iter()
        .map(|s| SheetFrames {
            id: text(s, "id"),
            name: text(s, "name"),
            number: s.get("drawingInfo").map(|d| text(d, "number")).unwrap_or_default().trim().to_owned(),
            frames: s
                .get("items")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
                .filter(|i| i.get("kind").and_then(|k| k.get("type")).and_then(|t| t.as_str()) == Some("map"))
                .map(|i| (text(i, "id"), text(i, "name")))
                .collect(),
        })
        .collect()
}

/// Where a target is: its bottom words, and a sheet frame's order on its sheet (from 1); none: it is gone.
pub fn target_place(doc: &Document, sheets: &[SheetFrames], t: &ViewTarget) -> Option<(String, Option<u32>)> {
    match (&t.view, &t.sheet, &t.item) {
        (Some(id), None, None) => doc
            .settings()
            .named_views
            .iter()
            .find(|v| view_key(v) == *id)
            .map(|v| (v.name.clone(), None)),
        (None, Some(sheet), Some(item)) => {
            let s = sheets.iter().find(|s| s.id == *sheet)?;
            let k = s.frames.iter().position(|(id, _)| id == item)?;
            Some((s.label().to_owned(), Some(u32::try_from(k + 1).unwrap_or(u32::MAX))))
        }
        _ => None,
    }
}

/// `m` with its words and break written from its target.
pub fn mark_settled(doc: &Document, sheets: &[SheetFrames], m: &ViewMarkFields) -> ViewMarkFields {
    let mut out = m.clone();
    match &m.target {
        None => {
            out.words = ViewMarkLabel {
                top: m.mark.clone(),
                bottom: NO_TARGET.to_owned(),
            };
            out.broken = None;
        }
        Some(t) => match target_place(doc, sheets, t) {
            Some((bottom, order)) => {
                let top = if m.mark.is_empty() {
                    order.map(|k| k.to_string()).unwrap_or_default()
                } else {
                    m.mark.clone()
                };
                out.words = ViewMarkLabel { top, bottom };
                out.broken = None;
            }
            None => out.broken = Some(ViewMarkBreak::TargetGone),
        },
    }
    out
}

impl Document {
    /// The book's sheets and frames, as the marks read them.
    pub fn sheet_frames(&self) -> Vec<SheetFrames> {
        sheet_frames(self.sheets())
    }

    /// The updates that write `slots`' marks again from their targets; none for one that already holds.
    fn viewmark_updates(&self, slots: impl IntoIterator<Item = crate::identity::Slot>) -> Vec<Op> {
        let sheets = self.sheet_frames();
        let mut out = Vec::new();
        for slot in slots {
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            let Entity::ViewMark(e) = &*current.entity else {
                continue;
            };
            let settled = mark_settled(self, &sheets, &e.mark);
            if settled != e.mark {
                let mut x = e.clone();
                x.mark = settled;
                out.push(Op::Update {
                    after: Stored {
                        uid: current.uid,
                        entity: Arc::new(Entity::ViewMark(x)),
                    },
                    before: current,
                });
            }
        }
        out
    }

    /// What writes the marks `ops` added or changed from their targets, applied: the step takes them in.
    pub(crate) fn follow_viewmarks(&mut self, ops: &[Op]) -> Vec<Op> {
        if !self.store.has_viewmarks() {
            return Vec::new();
        }
        let own: BTreeSet<Uuid> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Add(s) | Op::Update { after: s, .. } if matches!(&*s.entity, Entity::ViewMark(_)) => {
                    Some(s.uid)
                }
                _ => None,
            })
            .collect();
        let slots: Vec<_> = own.iter().filter_map(|u| self.store.slot_of(*u)).collect();
        let out = self.viewmark_updates(slots);
        for op in &out {
            self.apply_op(op);
        }
        out
    }

    /// Every mark written again from its target in place, outside the undo history: after the named views or the book
    /// changed, an undo or a redo. Returns how many changed.
    pub(crate) fn refresh_viewmarks(&mut self) -> usize {
        if !self.store.has_viewmarks() {
            return 0;
        }
        let slots: Vec<_> = self.store.viewmark_slots().collect();
        let ops = self.viewmark_updates(slots);
        for op in &ops {
            self.apply_op(op);
        }
        ops.len()
    }
}
