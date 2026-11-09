//! Mimari › Proje's template and phase view (docs/adr/0346): Mimari şablonu applies the KentOS work template through
//! `cad.templates.apply`; Tadilat's four views are the project's `arch.phaseView`, written through `cad.settings.set`.

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

    /// Tadilat ▸ `name` (`arch.phase.all` …).
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
            // What is drawn from now on takes the view's state (docs/adr/3039 §3): said, so it is never a surprise.
            let drawn = match view {
                PhaseView::All => String::new(),
                PhaseView::Existing | PhaseView::Demolition => {
                    " Bundan sonra çizilenler Mevcut olur.".to_owned()
                }
                PhaseView::Proposed | PhaseView::Renovation => {
                    " Bundan sonra çizilenler İlave olur.".to_owned()
                }
            };
            self.say(
                Level::Info,
                format!("Tadilat görünümü: {}.{drawn}", view.label()),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::Message;

    /// Tadilat ▾ says what is drawn from now on takes (docs/adr/3039 §3): the view's state, never a surprise.
    #[test]
    fn the_view_says_what_new_objects_take() {
        let mut app = crate::files_testing::app_with_drawing();
        crate::files_testing::make_cad(&mut app);
        for (id, said) in [
            (
                "arch.phase.proposed",
                "Tadilat görünümü: Yeni durum. Bundan sonra çizilenler İlave olur.",
            ),
            (
                "arch.phase.demolition",
                "Tadilat görünümü: Yıkım planı. Bundan sonra çizilenler Mevcut olur.",
            ),
            (
                "arch.phase.existing",
                "Tadilat görünümü: Mevcut durum. Bundan sonra çizilenler Mevcut olur.",
            ),
            (
                "arch.phase.renovation",
                "Tadilat görünümü: Tadilat projesi. Bundan sonra çizilenler İlave olur.",
            ),
            ("arch.phase.all", "Tadilat görünümü: Tümü."),
        ] {
            let _ = app.update(Message::Run(id));
            assert_eq!(crate::files_testing::last_said(&app), said, "{id}");
        }
    }
}
