//! Linked texts (docs/adr/0175 §4): a text that writes an object's label
//! (`labelOf`, `labelScale`) keeps following it. Before a step is recorded
//! (`commit`), its changes are read:
//!
//! - an object the step changed has its linked texts written again by the
//!   label rule (`label_text_of`): from its label now, its layer's label
//!   style (or its kind's default), the project's typeface and the text's
//!   scale; a text the rule no longer writes (no label, out of the style's
//!   scale range, too small) stays where it is and loses its link;
//! - an object the step removed takes its linked texts with it;
//! - a linked text the step itself edited (its place, text, height, turn or
//!   alignment) loses its link, unless its object changed in the same step
//!   (then the rule writes it).
//!
//! The changes join the step: one undo takes them back with the rest, and
//! undo and redo replay what was recorded. The web's is `CadDocument`'s
//! `followLinks`; both run `fixtures/document-ops/v1/linked-texts.json`.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;

use kentos_contracts::{DrawingFont, Entity, TextAlign, TextEntity, Vec2, default_label};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Entity as CoreEntity;
use kentos_geometry_core::ops::label_text::{fill_template, label_text_of};
use kentos_geometry_core::text::Font;

use crate::document::Document;
use crate::history::Op;
use crate::identity::{Slot, Uuid};
use crate::store::Stored;

/// Whether `after` is the same linked text as `before` written elsewhere or
/// otherwise: its place, text, height, turn or alignment changed.
fn edited(before: &TextEntity, after: &TextEntity) -> bool {
    before.p != after.p
        || before.text != after.text
        || before.height != after.height
        || before.rotation != after.rotation
        || before.align != after.align
}

/// The drawing's typeface as the geometry core names it (the contract's own spelling).
pub(crate) fn font_of(font: Option<DrawingFont>) -> Font {
    font.and_then(|f| serde_json::to_value(f).ok())
        .and_then(|v| v.as_str().map(Font::from_id))
        .unwrap_or(Font::DEFAULT)
}

/// A text without its link: it stays as it is and follows nothing.
fn unlinked(text: &TextEntity) -> TextEntity {
    TextEntity {
        label_of: None,
        label_scale: None,
        ..text.clone()
    }
}

impl Document {
    /// The texts that write the label of the object in `slot`, in slot order.
    pub fn linked_texts(&self, slot: Slot) -> Vec<Slot> {
        self.uid(slot)
            .map(|uid| self.store.linked_to(uid).collect())
            .unwrap_or_default()
    }

    /// Whether a text writes the label of the object in `slot`: that text is
    /// its label now, so the drawing, the sheet and Etiketleri yazıya çevir
    /// leave its own out.
    pub fn has_linked_text(&self, slot: Slot) -> bool {
        self.uid(slot)
            .is_some_and(|uid| self.store.linked_to(uid).next().is_some())
    }

    /// The objects whose label a text writes, in slot order.
    pub fn text_labelled(&self) -> Vec<Slot> {
        let mut slots: Vec<Slot> = self
            .store
            .linked_objects()
            .filter_map(|uid| self.store.slot_of(uid))
            .collect();
        slots.sort_unstable();
        slots
    }

    /// The linked text `text` written again for its object `object` now, or
    /// none when the rule writes nothing (docs/adr/0175 §4).
    fn rewritten(&self, text: &TextEntity, object: &Entity) -> Option<TextEntity> {
        let label = object.base().label.as_deref().filter(|l| !l.is_empty())?;
        let style = self
            .layers()
            .get(&object.base().layer_id)
            .and_then(|l| l.style.label.clone())
            .or_else(|| default_label(object.kind()))?;
        let scale = text.label_scale?;
        // The geometry core reads the contract's own JSON for an object.
        let json = serde_json::to_string(object).ok()?;
        let core = CoreEntity::from_json(&Json::parse(&json).ok()?).ok()?;
        let font = font_of(self.settings().drawing_font);
        // The label engine places it alone (docs/adr/0212 §4), its layer's point symbol beside it.
        let point = self
            .layers()
            .get(&object.base().layer_id)
            .and_then(|l| l.style.point.as_ref())
            .map_or(0.0, |p| p.size);
        let style_json = Json::parse(&serde_json::to_string(&style).ok()?).ok()?;
        let filled = fill_template(style.template.as_deref(), label);
        let t = label_text_of(&core.shape, &filled, &style_json, scale, point, font)?;
        Some(TextEntity {
            p: Vec2 { x: t.p.x, y: t.p.y },
            text: t.text,
            height: t.height,
            rotation: t.rotation,
            align: TextAlign::ALL
                .into_iter()
                .find(|a| a.name() == t.align.name()),
            ..text.clone()
        })
    }

    /// What keeps the linked texts with their objects after `ops` (a step
    /// about to be recorded), applied: the step takes them in.
    pub(crate) fn follow_links(&mut self, ops: &[Op]) -> Vec<Op> {
        if !self.store.has_links() {
            return Vec::new();
        }
        // The objects the step changed or removed, by persistent id, and the
        // linked texts it edited itself, by slot, in the order they came.
        let mut objects: Vec<Uuid> = Vec::new();
        let mut seen: HashSet<Uuid> = HashSet::new();
        let mut texts: BTreeMap<Slot, ()> = BTreeMap::new();
        for op in ops {
            let (stored, before) = match op {
                Op::Add(s) | Op::Remove(s) => (s, None),
                Op::Update { before, after } => (after, Some(before)),
                _ => continue,
            };
            if seen.insert(stored.uid) {
                objects.push(stored.uid);
            }
            if let (Some(before), Entity::Text(after)) = (before, &*stored.entity)
                && let Entity::Text(was) = &*before.entity
                && after.label_of.is_some()
                && edited(was, after)
            {
                texts.insert(stored.slot(), ());
            }
        }
        let mut out = Vec::new();
        let mut done: HashSet<Slot> = HashSet::new();
        for uid in objects {
            let linked: Vec<Slot> = self.store.linked_to(uid).collect();
            for slot in linked {
                if !done.insert(slot) {
                    continue;
                }
                let Some(current) = self.store.get(slot).cloned() else {
                    continue;
                };
                let Entity::Text(text) = &*current.entity else {
                    continue;
                };
                let next = match self.store.slot_of(uid).and_then(|s| self.store.get(s)) {
                    // Its object is gone: so is the text.
                    None => {
                        out.push(Op::Remove(current.clone()));
                        continue;
                    }
                    Some(object) => self
                        .rewritten(text, &object.entity)
                        .unwrap_or_else(|| unlinked(text)),
                };
                if next != *text {
                    out.push(Op::Update {
                        after: Stored {
                            uid: current.uid,
                            entity: Arc::new(Entity::Text(next)),
                        },
                        before: current,
                    });
                }
            }
        }
        // A linked text edited by itself, its object unchanged: it follows nothing now.
        for slot in texts.into_keys() {
            if done.contains(&slot) {
                continue;
            }
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            if let Entity::Text(text) = &*current.entity
                && text.label_of.is_some()
            {
                out.push(Op::Update {
                    after: Stored {
                        uid: current.uid,
                        entity: Arc::new(Entity::Text(unlinked(text))),
                    },
                    before: current,
                });
            }
        }
        for op in &out {
            self.apply_op(op);
        }
        out
    }
}
