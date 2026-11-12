//! A map project's work structure's undo step (plan/map-cad.md MAP-01, docs/adr/1200 §3). The structure lives in the
//! project's settings, which are otherwise outside the model's undo (`Document::set_settings`); its change is a step of
//! its own, or a part of the step it is written in (a relink also points rasters and images at the new file).

use std::sync::Arc;

use kentos_contracts::MapWork;

use crate::{Document, history::Op};

impl Document {
    /// The project's work structure; none in a project without one.
    pub fn map_work(&self) -> Option<&MapWork> {
        self.settings.map_work.as_ref()
    }

    /// Replaces the work structure as one undo step named `label` (inside an open transaction or group, part of it);
    /// an empty one is none. Nothing happens when it is the same.
    pub fn set_map_work(&mut self, next: Option<MapWork>, label: &str) {
        let next = next.filter(|w| !w.is_empty());
        if self.settings.map_work == next {
            return;
        }
        let before = Arc::new(std::mem::replace(
            &mut Arc::make_mut(&mut self.settings).map_work,
            next,
        ));
        let after = Arc::new(self.settings.map_work.clone());
        self.record(vec![Op::MapWork { before, after }], label);
    }
}
