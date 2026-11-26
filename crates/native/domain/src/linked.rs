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
//! A text linked without a scale is a point's name (docs/adr/9314): it is
//! written where it was put and moves as far as its point moves, unless the
//! step moved it too; its text is the point's name, and a point without a
//! name takes it away. Moved, turned or resized by hand it keeps its link; its
//! text retyped by hand, it loses it.
//!
//! The changes join the step: one undo takes them back with the rest, and
//! undo and redo replay what was recorded. The web's is `CadDocument`'s
//! `followLinks`; both run `fixtures/document-ops/v1/linked-texts.json`.

use std::collections::{BTreeMap, HashMap, HashSet};
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

/// Whether a text is a point's name: linked without a scale (docs/adr/9314).
fn names(text: &TextEntity) -> bool {
    text.label_of.is_some() && text.label_scale.is_none()
}

/// Where a point is; none for another object.
fn point_at(object: &Entity) -> Option<Vec2> {
    match object {
        Entity::Point(p) => Some(p.p),
        _ => None,
    }
}

/// A point's name `text` after a step that changed its point from `before` (none: the step made it) to `object`:
/// moved as far as the point, unless the step `moved` the text itself; its text the point's name. None when the
/// point has no name now: the text goes with it.
fn named(text: &TextEntity, before: Option<&Entity>, object: &Entity, moved: bool) -> Option<TextEntity> {
    let name = object.base().label.as_deref().filter(|l| !l.is_empty())?;
    let shift = match (before.and_then(point_at), point_at(object)) {
        (Some(a), Some(b)) if !moved => Vec2 { x: b.x - a.x, y: b.y - a.y },
        _ => Vec2 { x: 0.0, y: 0.0 },
    };
    Some(TextEntity {
        p: Vec2 { x: text.p.x + shift.x, y: text.p.y + shift.y },
        text: name.to_owned(),
        ..text.clone()
    })
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
        // The objects the step changed or removed, by persistent id, each as it was before the step; the linked texts
        // it edited itself, by slot, in the order they came; and, of those, the names it moved and the texts it
        // retyped (docs/adr/9314).
        let mut objects: Vec<Uuid> = Vec::new();
        let mut seen: HashSet<Uuid> = HashSet::new();
        let mut befores: HashMap<Uuid, Arc<Entity>> = HashMap::new();
        let mut texts: BTreeMap<Slot, ()> = BTreeMap::new();
        let mut moved: HashSet<Slot> = HashSet::new();
        let mut retyped: HashSet<Slot> = HashSet::new();
        for op in ops {
            let (stored, before) = match op {
                Op::Add(s) | Op::Remove(s) => (s, None),
                Op::Update { before, after } => (after, Some(before)),
                _ => continue,
            };
            if seen.insert(stored.uid) {
                objects.push(stored.uid);
            }
            if let Some(before) = before {
                befores.entry(stored.uid).or_insert_with(|| before.entity.clone());
            }
            if let (Some(before), Entity::Text(after)) = (before, &*stored.entity)
                && let Entity::Text(was) = &*before.entity
                && after.label_of.is_some()
                && edited(was, after)
            {
                texts.insert(stored.slot(), ());
                if was.p != after.p {
                    moved.insert(stored.slot());
                }
                if was.text != after.text {
                    retyped.insert(stored.slot());
                }
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
                    // A point's name retyped by hand in the step: a text of its own now (docs/adr/9314).
                    Some(_) if names(text) && retyped.contains(&slot) => unlinked(text),
                    Some(object) if names(text) => {
                        let before = befores.get(&uid).map(|b| &**b);
                        match named(text, before, &object.entity, moved.contains(&slot)) {
                            Some(next) => next,
                            // The point has no name now: its text goes.
                            None => {
                                out.push(Op::Remove(current.clone()));
                                continue;
                            }
                        }
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
            // A point's name moved, turned or resized by hand keeps following its point (docs/adr/9314).
            if let Entity::Text(text) = &*current.entity
                && text.label_of.is_some()
                && (!names(text) || retyped.contains(&slot))
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
