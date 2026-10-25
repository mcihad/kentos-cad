//! A binding's settings in the undo history (docs/adr/0636 §5): a layer bound to a source table keeps its key and its
//! part's area in the project's settings; a change of them is written in the step that changed the objects with it, so
//! undo gives the binding back as the objects were (the area's rows and the digest the layer matched).

use kentos_contracts::{SourceBase, SourceLink};

use crate::document::Document;
use crate::history::{Op, SourceState};

impl Document {
    /// The binding `id` and its reconciling base as they are now.
    pub fn source_state(&self, id: &str) -> (Option<SourceLink>, Option<SourceBase>) {
        (
            self.settings.sources.iter().find(|l| l.id == id).cloned(),
            self.settings
                .source_bases
                .iter()
                .find(|b| b.source == id)
                .cloned(),
        )
    }

    /// Records, into the open transaction (else a step of its own named `label`), that the binding `id` went from
    /// `before` (as [`Document::source_state`] gave it) to what it is now. Nothing when it did not change.
    pub fn record_source(
        &mut self,
        id: &str,
        before: (Option<SourceLink>, Option<SourceBase>),
        label: &str,
    ) {
        let now = self.source_state(id);
        if now == before {
            return;
        }
        self.record(
            vec![Op::Source {
                id: id.to_owned(),
                before: Box::new(SourceState {
                    link: before.0,
                    base: before.1,
                }),
                after: Box::new(SourceState {
                    link: now.0,
                    base: now.1,
                }),
            }],
            label,
        );
    }
}
