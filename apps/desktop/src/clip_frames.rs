//! Kırp's frames shown on the screen only (docs/adr/0351 §1): each clipped insert whose frame is `show`, its boundary
//! placed by the core as its content is (`Blocks::clip_ring`). Found again only when the drawing changes: the walk
//! over the objects is kept with the drawing's session and generation.

use std::cell::RefCell;
use std::sync::Arc;

use kentos_commands::{ClipFrame, Entity};
use kentos_interaction::Vec2;

/// The frames' rings, each in world coordinates.
type Rings = Arc<Vec<Vec<Vec2>>>;

/// The frames found for a drawing's session and generation.
#[derive(Default)]
pub struct ClipFrames(RefCell<Option<((u64, u64), Rings)>>);

impl crate::app::App {
    pub(crate) fn clip_marks(&self) -> Rings {
        let Some(doc) = self.document.as_ref() else {
            return Arc::default();
        };
        let key = (doc.session, doc.model.generation());
        if let Some((k, frames)) = self.clip_frames.0.borrow().as_ref()
            && *k == key
        {
            return frames.clone();
        }
        let layers = doc.model.layers();
        let blocks = self.spatial.store().blocks();
        let frames: Vec<Vec<Vec2>> = doc
            .model
            .entities()
            .filter(|e| {
                matches!(e, Entity::Insert(i) if i.clipping.as_ref().is_some_and(|c| c.frame == ClipFrame::Show))
                    && layers.is_visible(&e.base().layer_id)
            })
            .filter_map(|e| blocks.clip_ring(&kentos_native_application::geometry::shape(e)))
            .map(|ring| ring.into_iter().map(|p| Vec2::new(p.x, p.y)).collect())
            .collect();
        let frames = Arc::new(frames);
        *self.clip_frames.0.borrow_mut() = Some((key, frames.clone()));
        frames
    }
}
