//! A template is reviewed before it changes an existing drawing (ADR 0659).
use crate::app::{App, Dialog, Message};
use crate::arch;
use iced::Element;
use iced::widget::{Column, button};
use kentos_commands::{TemplateSettingsMode, TemplatesApplied, TemplatesApply};
use kentos_native_application::{ExecutionContext, templates_apply};
use kentos_ui::widget::{Choice, Dialog as Frame, Select};
use kentos_ui::{label, style};

#[derive(Clone, Debug)]
pub(crate) struct Window {
    pub(crate) session: u64,
    pub(crate) input: TemplatesApply,
    pub(crate) plan: TemplatesApplied,
    notice: Option<String>,
}
#[derive(Clone, Debug)]
pub(crate) enum Event {
    Mode {
        session: u64,
        mode: TemplateSettingsMode,
    },
    Apply {
        session: u64,
        input: TemplatesApply,
    },
}
impl App {
    pub(crate) fn apply_arch_template(&mut self) {
        self.prepare_arch_template(TemplateSettingsMode::Keep, None);
    }
    fn prepare_arch_template(&mut self, mode: TemplateSettingsMode, notice: Option<String>) {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return;
        };
        let input = TemplatesApply {
            name: "mimari".into(),
            unit: (mode == TemplateSettingsMode::Template && doc.settings().has_system())
                .then_some(kentos_commands::DrawingUnit::M),
            settings_mode: Some(mode),
            expected_revision: Some(doc.model.revision().to_string()),
        };
        let session = doc.session;
        let result = templates_apply::plan_in(&doc.model, &input);
        let Some(plan) = self.answered(result) else {
            return;
        };
        self.arch.arch_template = Some(Window {
            session,
            input,
            plan,
            notice,
        });
        self.dialog = Some(Dialog::Arch(arch::ArchDialogKind::ArchTemplate));
    }
    pub(crate) fn architecture_template_event(&mut self, event: Event) {
        if self.dialog != Some(Dialog::Arch(arch::ArchDialogKind::ArchTemplate)) {
            return;
        }
        match event {
            Event::Mode { session, mode } => {
                if self.document.as_ref().is_some_and(|d| d.session == session)
                    && self
                        .arch
                        .arch_template
                        .as_ref()
                        .is_some_and(|w| w.session == session)
                {
                    self.prepare_arch_template(mode, None);
                }
            }
            Event::Apply { session, input } => {
                if self.document.as_ref().is_none_or(|d| d.session != session)
                    || self
                        .arch
                        .arch_template
                        .as_ref()
                        .is_none_or(|w| w.session != session || w.input != input)
                {
                    self.warn("Şablon önizlemesi bu çizime ait değil; yeniden açın.");
                    return;
                }
                if let Some(why) = self.tree_locked() {
                    self.warn(why);
                    return;
                }
                let doc = self.document.as_mut().unwrap();
                if input.expected_revision.as_deref()
                    != Some(doc.model.revision().to_string().as_str())
                {
                    self.prepare_arch_template(input.settings_mode.unwrap_or(TemplateSettingsMode::Keep),
                        Some("Çizim değişti; hiçbir şey yazılmadı. Güncellenen farkı inceleyip yeniden uygulayın.".into()));
                    return;
                }
                let previous_revision = doc.model.revision().to_string();
                let result =
                    templates_apply::execute(&mut ExecutionContext::new(&mut doc.model), input);
                if let Some(out) = self.answered(result) {
                    self.arch.arch_template = None;
                    self.dialog = None;
                    let count = out.layers.len()
                        + out.styles.len()
                        + out.blocks.len()
                        + out.wall_styles.len()
                        + out.levels.len()
                        + out.sheets.len();
                    if out.revision == previous_revision {
                        self.say(
                            kentos_interaction::Level::Info,
                            "Şablon zaten uygulanmış; değişiklik yok.",
                        );
                        return;
                    }
                    self.say(kentos_interaction::Level::Success,
                        format!("Mimari şablonu uygulandı: {count} parça eklendi; birim {}, çizim ölçeği {}. Tek adımda geri alınabilir.",
                            out.unit.mark(), kentos_project::wizard::scale_text(out.review.as_ref().map_or(100., |r| r.plot_scale))));
                }
            }
        }
    }
    pub(crate) fn architecture_template_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.arch.arch_template else {
            return label::body("Önizleme yok.").into();
        };
        let Some(r) = &w.plan.review else {
            return label::body("Şablon farkı okunamadı.").into();
        };
        let session = w.session;
        // The package's own words, read from it (docs/adr/8007): the texts said cm / 1:50 by hand.
        let package = Package::read();
        let choices = [
            Choice::new("Mevcut birim ve çizim ölçeğini koru"),
            Choice::new(format!(
                "Ofis paketinin ayarlarını uygula — {}",
                package.settings
            )),
        ];
        let pick = Select::new(
            choices,
            Some(usize::from(r.mode == TemplateSettingsMode::Template)),
            move |i| {
                Message::Arch(arch::Event::ArchTemplate(Event::Mode {
                    session,
                    mode: if i == 0 {
                        TemplateSettingsMode::Keep
                    } else {
                        TemplateSettingsMode::Template
                    },
                }))
            },
        );
        let mut body = Column::new()
            .width(iced::Length::Fill)
            .spacing(10)
            .push(label::heading("Uygulanacak ayarlar"))
            .push(label::body(format!(
                "Birim: {} → {}",
                r.previous_unit.mark(),
                w.plan.unit.mark()
            )))
            .push(label::body(format!(
                "Çizim ölçeği: {} → {}",
                kentos_project::wizard::scale_text(r.previous_plot_scale),
                kentos_project::wizard::scale_text(r.plot_scale)
            )))
            .push(label::body(format!(
                "Mahal yöntemi: {} → {}",
                r.previous_space_method
                    .map_or("belirtilmemiş", |m| m.label()),
                r.space_method.label()
            )))
            .push(label::heading("Eklenecek parçalar"));
        for (kind, names) in [
            ("Katman", &w.plan.layers),
            ("Yazı / ölçü stili", &w.plan.styles),
            ("Blok", &w.plan.blocks),
            ("Duvar stili", &w.plan.wall_styles),
            ("Kat", &w.plan.levels),
            ("Pafta", &w.plan.sheets),
        ] {
            body = body.push(label::body(format!(
                "{kind} ({}): {}",
                names.len(),
                if names.is_empty() {
                    "yok".into()
                } else {
                    names.join(", ")
                }
            )));
        }
        if !r.preserved.is_empty() {
            body = body.push(label::heading("Korunacak mevcut tanımlar"));
            for part in &r.preserved {
                let kind = match part.kind.as_str() {
                    "layer" => "Katman",
                    "textStyle" => "Yazı stili",
                    "dimensionStyle" => "Ölçü stili",
                    "block" => "Blok",
                    "wallStyle" => "Duvar stili",
                    "level" => "Kat",
                    "sheet" => "Pafta",
                    _ => "Tanım",
                };
                let key = match part.matched_by.as_str() {
                    "id" => "aynı kimlik",
                    "idAndName" => "aynı kimlik ve ad",
                    _ => "aynı ad",
                };
                body = body.push(label::body(format!(
                    "{kind}: {} — {key}; mevcut değer korunur.",
                    part.name
                )));
            }
        }
        if let Some(notice) = &w.notice {
            body = body.push(label::body(notice.as_str()));
        }
        let locked = self.tree_locked();
        if let Some(why) = locked {
            body = body.push(label::body(why));
        }
        Frame::new("Mimari şablonunu gözden geçir")
            .push(label::muted(format!("Ofis paketi {}, {} kat ve {} A3 pafta içerir. Model geometrisi yeniden ölçeklenmez. Mevcut tanım, yazı tipi ve katman durumları korunur.", package.settings, package.levels, package.sheets)))
            .push(pick).scroll(body)
            .push(label::caption(format!("Tüm paket tek geri alma adımıdır. Paftalar kendi ölçekleriyle eklenir: {}.", package.sheet_scales)))
            .action(button(label::body("Vazgeç"))
.padding(kentos_ui::theme::metrics::action()).style(style::button::secondary).on_press(Message::DialogClosed))
            .action(button(label::body("Uygula"))
.padding(kentos_ui::theme::metrics::action()).style(style::button::primary).on_press_maybe(locked.is_none().then(|| {
                Message::Arch(arch::Event::ArchTemplate(Event::Apply { session, input: w.input.clone() }))
            })))
            .width(650.).max_height(650.)
.modal(Message::DialogClosed)
    }
}

/// What the window says of KentOS's Mimari package, from its data.
pub(crate) struct Package {
    /// Its unit and plot scale: “m / 1:100”.
    pub(crate) settings: String,
    pub(crate) levels: usize,
    pub(crate) sheets: usize,
    /// Its sheets' scales, each once in order: “1:100 ve 1:20”.
    pub(crate) sheet_scales: String,
}

impl Package {
    pub(crate) fn read() -> Self {
        let Some(t) = kentos_commands::builtin_template("mimari") else {
            return Self {
                settings: String::new(),
                levels: 0,
                sheets: 0,
                sheet_scales: String::new(),
            };
        };
        let mut scales: Vec<String> = Vec::new();
        for s in &t.sheets {
            let text = kentos_project::wizard::scale_text(f64::from(s.scale));
            if !scales.contains(&text) {
                scales.push(text);
            }
        }
        let sheet_scales = match scales.split_last() {
            Some((last, rest)) if !rest.is_empty() => format!("{} ve {last}", rest.join(", ")),
            Some((last, _)) => last.clone(),
            None => String::new(),
        };
        Self {
            settings: format!(
                "{} / {}",
                t.unit.mark(),
                kentos_project::wizard::scale_text(t.plot_scale)
            ),
            levels: t.levels.len(),
            sheets: t.sheets.len(),
            sheet_scales,
        }
    }
}

#[cfg(test)]
#[path = "../tests/architecture_template_tests.rs"]
mod tests;
