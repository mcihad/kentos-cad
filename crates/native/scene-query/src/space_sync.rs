//! The spaces' state the geometry store draws them by, sent with the objects (`Spatial::reload`, `Spatial::sync`): the
//! spaces that have a tag, drawn without their own label (docs/adr/0344's 27 October addendum), and the colour plan's
//! fills (docs/adr/8176), drawn under their rings.

use kentos_domain::Document;

use super::Spatial;

impl Spatial {
    /// Sends the spaces that have a tag when they differ from those sent last, and the colour plan's fills: the store
    /// draws again only the spaces whose label or fill changed. Without a colour plan the fills are none at once.
    pub(super) fn sync_spaces(&mut self, doc: &Document) {
        let now = doc.tagged_spaces();
        if now != self.tagged_spaces {
            let ids: Vec<f64> = now.iter().map(|slot| f64::from(slot.0)).collect();
            self.store.set_tagged_spaces(&ids);
            self.tagged_spaces = now;
        }
        let fills: Vec<(f64, String)> = doc
            .space_fills()
            .into_iter()
            .map(|(id, color)| (f64::from(id), color.to_owned()))
            .collect();
        self.store.set_space_fills(&fills);
    }
}
