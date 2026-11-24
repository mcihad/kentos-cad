//! Where a drawn view goes when its Yer is left empty (plan/road.md Y85, docs/adr/0991): under everything drawn, from
//! the drawing's left edge, a gap of paper between, so the view never covers the plan's points and boxes. A fixed
//! guess ("0; -500", "300 m under the alignments") put a frame as tall as the sheet's profile map over them (Y84's first
//! sheet picture). Yol's profiles, sections and sheets and Harita's pipe profiles place their views so.

use crate::app::App;

/// Paper millimetres between everything drawn and a view put under it.
const GAP_MM: f64 = 20.0;

impl App {
    /// The lower left of a view that reaches `up` drawing units over it (its frame, the rows over the frame), so that
    /// its top stays `GAP_MM` of paper under the box of everything drawn, from the box's left edge; on an empty drawing
    /// under the origin.
    pub fn under_drawing(&mut self, up: f64) -> [f64; 2] {
        let Some(doc) = &self.document else {
            return [0.0, -up];
        };
        let gap = GAP_MM * doc.model.settings().plot_scale / 1000.0;
        self.spatial.sync(&doc.model);
        match self.spatial.extent() {
            Some(b) => [b.min_x, b.min_y - gap - up],
            None => [0.0, -gap - up],
        }
    }
}
