//! Kırp's frames shown on the screen only (docs/adr/0351 §1): each clipped insert whose frame is `show`, its boundary
//! placed by the core as its content is (`Blocks::clip_ring`). Kept with the drawing's journal (docs/adr/0742): a
//! change looks again at the objects it touched, a changed block definition or a journal that no longer reaches back
//! walks every object once more; the layers' visibility is asked when the frames are given.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::sync::Arc;

use kentos_commands::{BlockDefinition, ClipFrame, Entity};
use kentos_domain::{ChangeMark, Changes, Slot};
use kentos_interaction::Vec2;

/// The frames' rings, each in world coordinates.
type Rings = Arc<Vec<Vec<Vec2>>>;

/// What the frames were found from, and the frames.
struct Kept {
    session: u64,
    generation: u64,
    mark: ChangeMark,
    /// The definitions when found: the document keeps an unchanged one's `Arc`.
    blocks: Vec<Arc<BlockDefinition>>,
    /// Each shown-frame insert: its layer and its ring.
    clipped: BTreeMap<Slot, (String, Vec<Vec2>)>,
    rings: Rings,
}

/// The frames found for a drawing.
#[derive(Default)]
pub struct ClipFrames(RefCell<Option<Kept>>);

impl crate::app::App {
    pub(crate) fn clip_marks(&self) -> Rings {
        let Some(doc) = self.document.as_ref() else {
            return Arc::default();
        };
        let model = &doc.model;
        let generation = model.generation();
        let mut kept = self.clip_frames.0.borrow_mut();
        if let Some(k) = kept.as_ref()
            && k.session == doc.session
            && k.generation == generation
        {
            return k.rings.clone();
        }
        let store_blocks = self.spatial.store().blocks();
        let ring_of = |e: &Entity| -> Option<Vec<Vec2>> {
            matches!(e, Entity::Insert(i) if i.clipping.as_ref().is_some_and(|c| c.frame == ClipFrame::Show))
                .then(|| store_blocks.clip_ring(&kentos_native_application::geometry::shape(e)))
                .flatten()
                .map(|ring| ring.into_iter().map(|p| Vec2::new(p.x, p.y)).collect())
        };
        let same_blocks = |k: &Kept| {
            k.blocks.len() == model.blocks().len()
                && k.blocks
                    .iter()
                    .zip(model.blocks())
                    .all(|(a, b)| Arc::ptr_eq(a, b))
        };
        let slots = kept
            .as_ref()
            .filter(|k| k.session == doc.session && same_blocks(k))
            .map(|k| model.changes_since(k.mark));
        let mut clipped = match (kept.take(), slots) {
            (Some(mut k), Some(Changes::Slots(slots))) => {
                for &slot in slots {
                    match model
                        .get(slot)
                        .and_then(|e| ring_of(e).map(|r| (e.base().layer_id.clone(), r)))
                    {
                        Some(found) => {
                            k.clipped.insert(slot, found);
                        }
                        None => {
                            k.clipped.remove(&slot);
                        }
                    }
                }
                k.clipped
            }
            _ => model
                .entities()
                .filter_map(|e| {
                    ring_of(e).map(|r| (Slot(e.base().id), (e.base().layer_id.clone(), r)))
                })
                .collect(),
        };
        let layers = model.layers();
        let rings: Rings = Arc::new(
            clipped
                .values()
                .filter(|(layer, _)| layers.is_visible(layer))
                .map(|(_, ring)| ring.clone())
                .collect(),
        );
        *kept = Some(Kept {
            session: doc.session,
            generation,
            mark: model.change_mark(),
            blocks: model.blocks().to_vec(),
            clipped: std::mem::take(&mut clipped),
            rings: rings.clone(),
        });
        rings
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::app::App;
    use crate::underlay_scenes::{clip, plan};

    fn frame(kind: &str) -> serde_json::Value {
        json!({ "kind": "set", "boundary": { "kind": "rectangle", "a": { "x": 0.0, "y": 0.0 }, "b": { "x": 4.0, "y": 3.0 } },
                "frame": kind })
    }

    #[test]
    fn a_frame_follows_its_insert_its_layer_and_the_steps() {
        let (mut app, _) = App::boot(None);
        let uid = plan(&mut app);
        assert!(app.clip_marks().is_empty(), "not clipped yet");
        clip(&mut app, &uid, frame("show"));
        let shown = app.clip_marks();
        assert_eq!(shown.len(), 1, "its frame shows");
        assert_eq!(shown[0].len(), 4, "the rectangle's corners");
        clip(&mut app, &uid, frame("plot"));
        assert!(
            app.clip_marks().is_empty(),
            "a plotted frame is not the screen's"
        );
        clip(&mut app, &uid, frame("show"));
        assert_eq!(app.clip_marks().len(), 1);
        // The insert's layer hidden: its frame too.
        let layer = {
            let doc = app.document.as_ref().expect("a drawing");
            let slot = doc
                .model
                .slot_of(uid.parse().expect("a uid"))
                .expect("the insert");
            doc.model
                .get(slot)
                .expect("the insert")
                .base()
                .layer_id
                .clone()
        };
        let visible = |app: &mut App, on: bool| {
            let doc = app.document.as_mut().expect("a drawing");
            let answer = kentos_native_application::registry::run(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                "cad.layers.set",
                kentos_commands::CallOp::Execute,
                json!({ "operation": "visible", "layer": layer, "value": on }),
            );
            assert!(
                answer.as_ref().is_ok_and(|a| a["status"] == "completed"),
                "{answer:?}"
            );
        };
        visible(&mut app, false);
        assert!(
            app.clip_marks().is_empty(),
            "a hidden layer's frame does not show"
        );
        visible(&mut app, true);
        assert_eq!(app.clip_marks().len(), 1);
        // Undone, done again: the journal's slots are looked at again.
        let _ = app.run("edit.undo");
        assert!(app.clip_marks().is_empty(), "back to the plotted frame");
        let _ = app.run("edit.redo");
        assert_eq!(app.clip_marks().len(), 1, "shown again");
    }
}
