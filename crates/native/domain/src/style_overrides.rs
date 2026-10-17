//! The style fields an object sets apart by hand (docs/adr/0322 §5): after a step, each dimension and text of a named
//! style that the step changed has its `overrides` written from what the step did. A field the step changed is the
//! object's own when it differs from the style's value, and follows again when it equals it; a field the step left
//! keeps what it was. An object given another style starts again with none; one made keeps those it was made with (a
//! pasted copy's). A style changed later leaves the overrides where they are (`follow_text_style`,
//! `follow_dimension_style`). Fields not listed follow by the older rule, while they hold the style's old value: so
//! objects no step changed since (older files) follow as they did.

use std::collections::BTreeMap;
use std::sync::Arc;

use kentos_contracts::{
    DimensionEntity, Entity, TextEntity, TextLook, dimension_overrides, text_overrides,
};

use crate::document::Document;
use crate::history::Op;
use crate::identity::{Slot, Uuid};
use crate::store::Stored;

fn text_look(t: &TextEntity) -> TextLook {
    TextLook {
        face: t.face.clone(),
        width_factor: t.width_factor,
        height: t.height,
    }
}

impl Document {
    /// The updates that write the overrides of the styled objects `ops` made or changed.
    pub(crate) fn mark_overrides(&mut self, ops: &[Op]) -> Vec<Op> {
        // Each object the step made or changed: its slot, and how it was first (none: made by the step).
        let mut touched: BTreeMap<Uuid, (Slot, Option<Arc<Entity>>)> = BTreeMap::new();
        for op in ops {
            match op {
                // Made by the step: as it was made.
                Op::Add(s) => {
                    touched.insert(s.uid, (s.slot(), None));
                }
                Op::Update { before, after } => {
                    touched
                        .entry(after.uid)
                        .or_insert((after.slot(), Some(Arc::clone(&before.entity))));
                }
                Op::Remove(s) => {
                    touched.remove(&s.uid);
                }
                _ => {}
            }
        }
        let settings = self.settings();
        let scale = settings.plot_scale;
        let mut out = Vec::new();
        for (slot, before) in touched.into_values() {
            let Some(now) = self.store.get(slot) else {
                continue;
            };
            let Some(before) = before else {
                continue;
            };
            let next = match (&*now.entity, &*before) {
                (Entity::Dimension(d), was) => {
                    let style = d
                        .look
                        .dim_style
                        .as_ref()
                        .and_then(|id| settings.dimension_styles.iter().find(|s| &s.id == id));
                    let own = match (style, was) {
                        // No style (or one the project has not): nothing to set apart from.
                        (None, _) => Vec::new(),
                        (Some(style), Entity::Dimension(w))
                            if w.look.dim_style == d.look.dim_style =>
                        {
                            dimension_overrides(
                                (&w.look, w.height),
                                (&d.look, d.height),
                                style,
                                scale,
                            )
                        }
                        // Another style: it starts again.
                        _ => Vec::new(),
                    };
                    if own == d.look.overrides {
                        continue;
                    }
                    let mut d: DimensionEntity = d.clone();
                    d.look.overrides = own;
                    Entity::Dimension(d)
                }
                (Entity::Text(t), was) => {
                    let style = t
                        .face
                        .text_style
                        .as_ref()
                        .and_then(|id| settings.text_styles.iter().find(|s| &s.id == id));
                    let own = match (style, was) {
                        (None, _) => Vec::new(),
                        (Some(style), Entity::Text(w))
                            if w.face.text_style == t.face.text_style =>
                        {
                            text_overrides(&text_look(w), &text_look(t), style, scale)
                        }
                        _ => Vec::new(),
                    };
                    if own == t.face.overrides {
                        continue;
                    }
                    let mut t: TextEntity = t.clone();
                    t.face.overrides = own;
                    Entity::Text(t)
                }
                _ => continue,
            };
            out.push(Op::Update {
                before: now.clone(),
                after: Stored {
                    uid: now.uid,
                    entity: Arc::new(next),
                },
            });
        }
        for op in &out {
            self.apply_op(op);
        }
        out
    }
}
