//! Ölçekli açıklamalar on the selection (docs/adr/0323 §4): the ribbon's Ölçekli yap, Ölçekliliği kaldır and Ölçeği
//! sıfırla run `cad.annotations.scales` on the selected texts, dimensions and leaders; what it answered is said. Their
//! scales are Öznitelikler's Ölçekler; a scale's own place is the tool Ölçekteki yeri
//! (`kentos_interaction::annotative`).

use kentos_commands::{AnnotationsScales, AnnotativeOperation, CommandResult};
use kentos_native_application::{ExecutionContext, annotations_scales};

use crate::app::App;

impl App {
    /// `operation` on the selection's annotations.
    pub(crate) fn annotative_selection(&mut self, operation: AnnotativeOperation) {
        let what = annotations_scales::label(operation);
        if self.selection.is_empty() {
            self.warn(format!(
                "{what}: önce yazıları, ölçüleri ya da kılavuzları seçin."
            ));
            return;
        }
        let ids = self.selection.ids().to_vec();
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let input = AnnotationsScales {
            operation,
            uids: kentos_interaction::properties::uids_of(&doc.model, &ids),
            scales: Vec::new(),
            scale: None,
            point: None,
            rotation: None,
            offset: None,
            text_shift: None,
            expected_revision: None,
        };
        let result = annotations_scales::execute(&mut ExecutionContext::new(&mut doc.model), input);
        match result {
            CommandResult::Completed { output, warnings } => {
                let n = output.changed.len();
                if n == 0 {
                    self.output(format!("{what}: değişen nesne yok."));
                } else {
                    let scale = doc.model.settings().plot_scale;
                    self.output(match operation {
                        AnnotativeOperation::Make => format!(
                            "{what}: {n} nesne {} ölçeğinde ölçekli; diğer ölçekleri Öznitelikler › Diğer ölçekler'den ekleyin.",
                            kentos_interaction::annotative::ratio(scale)
                        ),
                        _ => format!("{what}: {n} nesne."),
                    });
                }
                for w in warnings {
                    self.warn(w.message);
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }
}
