//! Mimari › Proje's template and phase view (docs/adr/0346): Mimari şablonu applies the KentOS work template through
//! `cad.templates.apply`; Tadilat's four views are the project's `arch.phaseView`, written through `cad.settings.set`;
//! Mahal ▸ Renk planı's schemes the project's `arch.colorPlan` (docs/adr/8176), the same way.

use kentos_commands::{ColorPlan, ColorScheme, PhaseView, SettingsSet};
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;

use crate::app::App;

/// The module's `App` methods of this file, as an extension trait (docs/adr/9199 §3.5).
pub(crate) trait ArchProjectApp {
    fn phase_view(&self) -> PhaseView;
    fn set_phase_view(&mut self, name: &str);
    fn color_plan(&self) -> Option<ColorScheme>;
    fn set_color_plan(&mut self, name: &str);
}

/// A colour plan's scheme by its command's last word (`arch.colorPlan.usage`); `off`: none.
fn scheme_named(name: &str) -> Option<Option<ColorScheme>> {
    match name {
        "off" => Some(None),
        "usage" => Some(Some(ColorScheme::Usage)),
        "unit" => Some(Some(ColorScheme::Unit)),
        _ => None,
    }
}

/// The command's last word of `scheme` (none: `off`).
pub(crate) fn scheme_name(scheme: Option<ColorScheme>) -> &'static str {
    match scheme {
        None => "off",
        Some(ColorScheme::Usage) => "usage",
        Some(ColorScheme::Unit) => "unit",
    }
}

impl ArchProjectApp for App {
    /// The project's phase view.
    fn phase_view(&self) -> PhaseView{
        self.document
            .as_ref()
            .and_then(|d| d.model.settings().arch.as_ref().and_then(|a| a.phase_view))
            .unwrap_or_default()
    }

    /// Tadilat ▸ `name` (`arch.phase.all` …).
    fn set_phase_view(&mut self, name: &str){
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
            // What is drawn from now on takes the view's state (docs/adr/8039 §3): said, so it is never a surprise.
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

    /// The project's colour plan's scheme.
    fn color_plan(&self) -> Option<ColorScheme> {
        self.document.as_ref().and_then(|d| d.model.color_plan())
    }

    /// Mahal ▸ Renk planı ▸ `name` (`arch.colorPlan.usage` …): the plan's rooms filled by it; the scheme on now chosen
    /// again, or `off`: not.
    fn set_color_plan(&mut self, name: &str) {
        let Some(mut scheme) = scheme_named(name) else {
            return;
        };
        if scheme.is_some() && scheme == self.color_plan() {
            scheme = None;
        }
        let Some(doc) = self.document.as_mut() else {
            self.output("Açık çizim yok.");
            return;
        };
        let mut arch = doc.model.settings().arch.clone().unwrap_or_default();
        arch.color_plan = scheme.map(|scheme| ColorPlan { scheme });
        let result = kentos_native_application::settings_set::execute(
            &mut ExecutionContext::new(&mut doc.model),
            SettingsSet { arch: Some(arch), ..Default::default() },
        );
        if self.answered(result).is_some() {
            let words = match scheme {
                None => "Mahal renk planı kapandı.".to_owned(),
                Some(s) => {
                    let rows = self.document.as_ref().map_or(0, |d| d.model.color_legend(None).len());
                    format!(
                        "Mahal renk planı: {}. Mahaller {rows} renkle dolduruldu; Renk planı lejandı renklerin \
                         lejandını çizer, aynı şema yeniden seçilince plan kapanır.",
                        s.label()
                    )
                }
            };
            self.say(Level::Info, words);
            // A fill covers what lies under its rooms' layer (docs/adr/8176 §3.4): said with the layers, and how.
            let covered = self.document.as_ref().map_or_else(Vec::new, |d| d.model.covered_layers());
            if scheme.is_some() && !covered.is_empty() {
                let rows: Vec<String> = covered
                    .iter()
                    .map(|(rooms, under)| {
                        let names: Vec<String> = under.iter().take(4).map(|n| format!("“{n}”")).collect();
                        let tail = match (under.len(), under.len().saturating_sub(4)) {
                            (1, _) => "katmanının üstünde".to_owned(),
                            (_, 0) => "katmanlarının üstünde".to_owned(),
                            (_, more) => format!("katmanlarının ve {more} katmanın daha üstünde"),
                        };
                        format!("“{rooms}” katmanı {} {tail}", names.join(", "))
                    })
                    .collect();
                self.say(
                    Level::Warn,
                    format!(
                        "Renk planının dolgusu mahallerin katmanında çizilir: {}; dolgu onların mahallerin içindeki \
                         çizgilerini örter. Katmanlar'da mahal katmanını onların altına taşıyın.",
                        rows.join("; ")
                    ),
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::Message;

    /// Tadilat ▾ says what is drawn from now on takes (docs/adr/8039 §3): the view's state, never a surprise.
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

    /// Mahal ▸ Renk planı (docs/adr/8176): a scheme is the project's and said, its item checked; chosen again or Kapalı, off.
    #[test]
    fn the_colour_plan_s_schemes_are_set_checked_and_said() {
        use super::{ArchProjectApp, scheme_name};
        use kentos_commands::ColorScheme;
        let mut app = crate::files_testing::app_with_drawing();
        crate::files_testing::make_cad(&mut app);
        assert_eq!(scheme_name(app.color_plan()), "off");
        let _ = app.update(Message::Run("arch.colorPlan.usage"));
        assert_eq!(app.color_plan(), Some(ColorScheme::Usage));
        assert!(crate::files_testing::last_said(&app).starts_with("Mahal renk planı: Kullanım."), "a drawing with no room covers nothing");
        let _ = app.update(Message::Run("arch.colorPlan.unit"));
        assert_eq!(scheme_name(app.color_plan()), "unit");
        // The scheme on, chosen again: off.
        let _ = app.update(Message::Run("arch.colorPlan.unit"));
        assert_eq!(app.color_plan(), None);
        assert_eq!(crate::files_testing::last_said(&app), "Mahal renk planı kapandı.");
        let _ = app.update(Message::Run("arch.colorPlan.usage"));
        let _ = app.update(Message::Run("arch.colorPlan.off"));
        assert_eq!(app.color_plan(), None);
    }
}
