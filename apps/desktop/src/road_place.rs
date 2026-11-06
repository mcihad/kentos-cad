//! Where a profile drawing goes when its Yer is left empty (plan/road.md Y85, docs/adr/0991): under everything drawn,
//! from the drawing's left edge, a gap of paper between. Civil 3D asks for the view's lower left corner; Autodesk's
//! plan–profile exercise picks it outside the project area so the views do not cover it. A fixed guess ("0; -500",
//! "300 m under the alignments") put a frame as tall as the sheet's profile map over the plan's points and boxes
//! (Y84's first sheet picture).

use crate::app::App;

/// The mass haul diagrams whose volume's corridor is on `alignment` (docs/adr/0995): their ids and names, for KGM's
/// earthwork rows.
pub(crate) fn hauls_on(doc: Option<&kentos_domain::Document>, alignment: Option<&str>) -> Vec<(String, String)> {
    let (Some(doc), Some(alignment)) = (doc, alignment) else { return Vec::new() };
    doc.road_hauls()
        .iter()
        .filter(|h| {
            doc.road_volumes()
                .iter()
                .find(|v| v.id == h.volume)
                .and_then(|v| doc.road_corridors().iter().find(|c| c.id == v.corridor))
                .is_some_and(|c| c.alignment == alignment)
        })
        .map(|h| (h.id.clone(), h.name.clone()))
        .collect()
}

/// Paper millimetres between everything drawn and a view put under it.
const GAP_MM: f64 = 20.0;

impl App {
    /// The lower left of a view that reaches `up` drawing units over it (its frame, the rows over the frame), so that
    /// its top stays `GAP_MM` of paper under the box of everything drawn, from the box's left edge; on an empty drawing
    /// under the origin.
    pub(crate) fn under_drawing(&mut self, up: f64) -> [f64; 2] {
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
