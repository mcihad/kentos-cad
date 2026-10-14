//! The sub-object a Ctrl click selected (docs/adr/0276): its point marked on the drawing beside the Koordinat
//! listesi's marked vertices, and said again when Sıradakini seç moves to another object.

use kentos_domain::dimension_ties::{core_blocks, core_place};
use kentos_geometry_core::ops::subref::{point, point_in};
use kentos_interaction::Vec2;
use kentos_native_application::blocks::core_entity;

use crate::app::App;

impl App {
    /// The selected sub-object's point in the drawing; none without one.
    pub(crate) fn sub_object_point(&self) -> Option<Vec2> {
        let doc = self.document.as_ref()?;
        let (slot, sub) = self.selection.sub()?;
        let e = doc.model.get(slot)?;
        let core = core_entity(e);
        let place = core_place(sub.place);
        if sub.path.is_empty() {
            point(&core, place)
        } else {
            point_in(&core_blocks(doc.model.blocks()), &core, &sub.path, place)
        }
    }

    /// The points marked with a ring: the Koordinat listesi's selected vertices and the selected sub-object.
    pub(crate) fn marked_points(&self) -> Vec<Vec2> {
        let mut out = self.vertex_marks();
        out.extend(self.sub_object_point());
        out
    }

    /// Says the selected sub-object (after Sıradakini seç moved), when a Ctrl click chose it.
    pub(crate) fn say_sub_object(&mut self) {
        let Some(doc) = &self.document else {
            return;
        };
        if self.selection.sub().is_none() {
            return;
        }
        if let Some(line) = kentos_interaction::select::sub_line(&doc.model, &self.selection) {
            self.output(line);
        }
    }
}
