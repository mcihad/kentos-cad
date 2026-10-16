//! Kayıtları denetle (docs/adr/0406 §1, `data.formAudit`): every record of a form's layer by its
//! rules, through `cad.forms.audit`.
//!
//! ```text
//! ┌ Kayıtları denetle · Bina hesabı ─────────────────────────────────┐
//! │ 9 kayıt denetlendi · 6 hatalı · 1 uyarılı        ☐ Uyarılar da   │
//! │ ⨯ B-3    “Kod” katmanda tekil olmalı; …  (1 sorun daha)  Git Aç  │
//! │ ⚠ B-10   Açıklama kısa görünüyor.                        Git Aç  │
//! │ [Hepsini seç]                       [Yeniden denetle] [Kapat]    │
//! └──────────────────────────────────────────────────────────────────┘
//! ```
//!
//! Hepsini seç selects the records listed; a row's Git shows it in the drawing, Aç opens it in
//! Formla düzenle (the window closes).

use iced::widget::{Column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{FormSeverity, FormsAudit, FormsAudited};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, forms_audit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Kayıtları denetle";
const CLOSE: &str = "Kapat";
const AGAIN: &str = "Yeniden denetle";
const SELECT_ALL: &str = "Hepsini seç";
const WARNINGS: &str = "Uyarılar da";

/// The window.
#[derive(Debug, Clone)]
pub struct Window {
    form: String,
    name: String,
    warnings: bool,
    answer: Option<FormsAudited>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Warnings(bool),
    Again,
    SelectAll,
    Show(String),
    Open(String),
    Close,
}

fn msg(event: Event) -> Message {
    Message::FormAudit(event)
}

impl App {
    /// Kayıtları denetle on `form`; on the active layer's form when none is given.
    pub(crate) fn open_form_audit(&mut self, form: Option<String>) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let active = doc.model.layers().active().to_owned();
        let form = form
            .or_else(|| self.form_of_layer(&active))
            .or_else(|| self.project_forms().first().map(|f| f.id.clone()));
        let Some(form) = form else {
            self.say(
                Level::Warn,
                "Projede form yok. Formlar penceresinden bir katmana form oluşturun.",
            );
            return Task::none();
        };
        let name = self
            .project_forms()
            .iter()
            .find(|f| f.id == form)
            .map_or_else(|| form.clone(), |f| f.name.clone());
        self.form_audit = Some(Window {
            form,
            name,
            warnings: false,
            answer: None,
        });
        self.dialog = Some(Dialog::FormAudit);
        self.run_form_audit();
        Task::none()
    }

    /// Asks `cad.forms.audit` again (the form's latest release, else its draft).
    fn run_form_audit(&mut self) {
        let Some(w) = self.form_audit.as_ref() else {
            return;
        };
        let draft = self
            .project_forms()
            .iter()
            .find(|f| f.id == w.form)
            .is_some_and(|f| f.releases.is_empty());
        let (moment, user) = self.form_moment();
        let input = FormsAudit {
            form: w.form.clone(),
            draft,
            warnings: w.warnings,
            limit: Some(10_000),
            moment,
            user,
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let result = forms_audit::plan(&ExecutionContext::new(&mut doc.model), &input);
        let answer = self.answered(result);
        if let Some(w) = self.form_audit.as_mut() {
            w.answer = answer;
        }
    }

    pub(crate) fn form_audit_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Warnings(on) => {
                if let Some(w) = self.form_audit.as_mut() {
                    w.warnings = on;
                }
                self.run_form_audit();
            }
            Event::Again => self.run_form_audit(),
            Event::SelectAll => {
                let slots: Vec<Slot> = self.audit_slots();
                let n = slots.len();
                self.selection.set(slots);
                self.say(Level::Info, format!("{n} kayıt seçildi."));
            }
            Event::Show(uid) => {
                if let Some(slot) = self.slot_of_text(&uid) {
                    self.selection.set([slot]);
                    self.zoom_selection();
                }
            }
            Event::Open(uid) => {
                let form = self.form_audit.as_ref().map(|w| w.form.clone());
                if let (Some(slot), Some(form)) = (self.slot_of_text(&uid), form) {
                    self.form_audit = None;
                    self.dialog = None;
                    self.selection.set([slot]);
                    return self.open_entry(slot, &form);
                }
            }
            Event::Close => {
                self.form_audit = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    fn slot_of_text(&self, uid: &str) -> Option<Slot> {
        let doc = self.document.as_ref()?;
        doc.model.slot_of(Uuid::parse_str(uid).ok()?)
    }

    /// The records listed, as slots.
    fn audit_slots(&self) -> Vec<Slot> {
        self.form_audit
            .as_ref()
            .and_then(|w| w.answer.as_ref())
            .map(|a| {
                a.records
                    .iter()
                    .filter_map(|r| self.slot_of_text(&r.uid))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A record's words in the list: its label, else its kind and the start of its id.
    fn audit_words(&self, uid: &str) -> String {
        let e = self
            .document
            .as_ref()
            .and_then(|doc| doc.model.get(self.slot_of_text(uid)?));
        match e {
            Some(e) => e.base().label.clone().unwrap_or_else(|| {
                format!(
                    "{} {}",
                    kentos_forms::kind_label(e),
                    uid.chars().take(8).collect::<String>()
                )
            }),
            None => uid.to_owned(),
        }
    }

    pub(crate) fn form_audit_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.form_audit else {
            return iced::widget::text("").into();
        };
        let mut body = Column::new().spacing(12);
        let warnings = words::check(
            w.warnings,
            WARNINGS,
            Some(msg(Event::Warnings(!w.warnings))),
        );
        match &w.answer {
            None => {
                body = body.push(words::text_line(Kind::Warn, "Denetim yapılamadı."));
            }
            Some(a) => {
                let mut head = format!("{} kayıt denetlendi · {} hatalı", a.checked, a.failing);
                if a.warned > 0 {
                    head.push_str(&format!(" · {} uyarılı", a.warned));
                }
                body = body.push(
                    row![
                        label::strong(head),
                        space::horizontal().width(Fill),
                        warnings
                    ]
                    .spacing(10)
                    .align_y(Center),
                );
                let mut list = Column::new().spacing(2).padding([2, 4]);
                if a.records.is_empty() {
                    list = list.push(words::text_line(
                        Kind::Ok,
                        "Bütün kayıtlar formun kurallarına uyuyor.",
                    ));
                }
                for r in a.records.iter().take(500) {
                    let error = r.issues.iter().find(|i| i.severity == FormSeverity::Error);
                    let first = error.or(r.issues.first());
                    let mut text = first.map_or_else(String::new, |i| i.message.clone());
                    if r.issues.len() > 1 {
                        text.push_str(&format!(" ({} sorun daha)", r.issues.len() - 1));
                    }
                    let kind = if error.is_some() {
                        Kind::Error
                    } else {
                        Kind::Warn
                    };
                    list = list.push(
                        row![
                            container(label::body(self.audit_words(&r.uid))).width(140),
                            container(words::text_line(kind, text)).width(Fill),
                            words::ghost("Git", Some(msg(Event::Show(r.uid.clone())))),
                            words::secondary("Aç", Some(msg(Event::Open(r.uid.clone())))),
                        ]
                        .spacing(8)
                        .align_y(Center),
                    );
                }
                if a.records.len() > 500 || a.more {
                    list = list.push(
                        label::caption(
                            "Liste kısaltıldı; Hepsini seç bütün sorunlu kayıtları seçer.",
                        )
                        .style(muted),
                    );
                }
                body = body.push(
                    container(scrollable(list).height(typography::scaled(300.0)))
                        .style(style::container::field_box)
                        .width(Fill),
                );
            }
        }
        let some = w.answer.as_ref().is_some_and(|a| !a.records.is_empty());
        body = body.push(
            row![
                words::secondary(SELECT_ALL, some.then(|| msg(Event::SelectAll))),
                space::horizontal().width(Fill),
                words::secondary(AGAIN, Some(msg(Event::Again))),
                words::primary(CLOSE, Some(msg(Event::Close))),
            ]
            .spacing(8)
            .align_y(Center),
        );
        overlay::modal(
            Frame::new(format!("{TITLE} · {}", w.name))
                .push(body)
                .width(760.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn form_audit_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.form_audit else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(AGAIN) => Some(msg(Event::Again)),
            Control::Press(SELECT_ALL) => Some(msg(Event::SelectAll)),
            Control::Press(WARNINGS) => Some(msg(Event::Warnings(!w.warnings))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

fn muted(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}
