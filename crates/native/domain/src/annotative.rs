//! The drawing as a sheet's map at one scale draws it (docs/adr/0323 §2): every
//! annotative text, dimension and leader as its scale says (`at_scale`), those
//! not taking part in it left out; every other object as it is. A reading copy:
//! no undo history, the same revision and generation.

use std::sync::Arc;

use kentos_contracts::{AtScale, annotative_of, at_scale};

use crate::document::Document;
use crate::store::Stored;

impl Document {
    /// Whether an object of the drawing is annotative.
    pub fn has_annotative(&self) -> bool {
        self.store
            .iter()
            .any(|s| annotative_of(&s.entity).is_some())
    }

    /// The drawing as a map at 1:`scale` draws it; none when it is the drawing itself (no
    /// annotative object draws otherwise there).
    pub fn at_scale(&self, scale: f64) -> Option<Document> {
        let changes: Vec<(Stored, AtScale)> = self
            .store
            .iter()
            .filter(|s| annotative_of(&s.entity).is_some())
            .filter_map(|s| match at_scale(&s.entity, scale) {
                AtScale::Own => None,
                other => Some((s.clone(), other)),
            })
            .collect();
        if changes.is_empty() {
            return None;
        }
        let mut copy = self.reading_copy();
        for (stored, how) in changes {
            match how {
                AtScale::Hidden => {
                    copy.store.remove(stored.slot());
                }
                AtScale::Drawn(e) => copy.store.put(Stored {
                    uid: stored.uid,
                    entity: Arc::new(*e),
                }),
                AtScale::Own => {}
            }
        }
        Some(copy)
    }
}
