//! Mimari › Proje's template and phase view (docs/adr/0346): Mimari şablonu applies the KentOS work template through
//! `cad.templates.apply`; Evre's four views are the project's `arch.phaseView`, written through `cad.settings.set`.

use kentos_commands::{PhaseView, SettingsSet};
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;

use crate::app::App;

impl App {
    /// The project's phase view.
    pub(crate) fn phase_view(&self) -> PhaseView {
        self.document
            .as_ref()
            .and_then(|d| d.model.settings().arch.as_ref().and_then(|a| a.phase_view))
            .unwrap_or_default()
    }

    /// Evre ▸ `name` (`arch.phase.all` …).
    pub(crate) fn set_phase_view(&mut self, name: &str) {
        let Some(view) = PhaseView::ALL.into_iter().find(|v| v.name() == name) else {
            return;
        };
        let Some(doc) = self.document.as_mut() else {
            self.output("Açık çizim yok.");
            return;
        };
        let mut arch = doc.model.settings().arch.clone().unwrap_or_default();
        arch.phase_view = (view != PhaseView::All).then_some(view);
        let result = kentos_native_application::settings_set::execute(
            &mut ExecutionContext::new(&mut doc.model),
            SettingsSet {
                arch: Some(arch),
                ..Default::default()
            },
        );
        if self.answered(result).is_some() {
            self.say(Level::Info, format!("Evre: {}.", view.label()));
        }
    }
}
