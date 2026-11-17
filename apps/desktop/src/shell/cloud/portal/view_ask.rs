//! A step's or a project file's question over Portal dosyaları (docs/adr/9001 §4): what the step does, the note it
//! takes (written before a refusal or a deficiency list can go), a decision's caution, the person to assign, the
//! server's refusal; Vazgeç and the confirming button, red outlined for a refusal (DESIGN.md §7.9.1).

use iced::widget::button;
use iced::widget::text::Wrapping;
use iced::widget::text_editor;
use iced::{Element, Length};
use kentos_commands::DossierAction;
use kentos_ui::theme::{metrics, typography};
use kentos_ui::widget::{Banner, Choice, Dialog as Frame, Form, Select};
use kentos_ui::{label, style};

use super::plan;
use super::{Asking, Event, Portal, Question, msg};
use crate::app::{App, Message};
use crate::exchange::words;

impl App {
    /// A step's or a project file's question over the window.
    pub(super) fn portal_ask_view<'a>(&'a self, p: &'a Portal) -> Option<Element<'a, Message>> {
        let a = p.asking.as_ref()?;
        let d = p.detail.as_ref().filter(|d| d.summary.id == a.dossier)?;
        let cancel = (!a.busy).then(|| msg(Event::AskClose));
        let mut form = Form::new().label_width(120.0);
        let (title, confirm, danger) = match &a.question {
            Question::Import { name, .. } => {
                form = form
                    .wide(label::body(plan::import_text(d, name)).wrapping(Wrapping::WordOrGlyph));
                ("Proje olarak aç".to_owned(), "Proje olarak aç", false)
            }
            Question::Step(DossierAction::Assign) => {
                let choices = d
                    .assignees
                    .iter()
                    .map(|x| Choice::new(x.display_name.clone()));
                form = form
                    .field(
                        "Sorumlu",
                        Select::new(choices, a.assignee, |i| msg(Event::AskAssignee(i))),
                    )
                    .required()
                    .help("Kurumda dosyalara bakabilen (en az düzenleyici) üyeler.");
                ("Sorumlu ata".to_owned(), "Ata", false)
            }
            Question::Step(action) => {
                let ask = plan::ask_of(*action, d)?;
                if let Some(lead) = &ask.lead {
                    form = form.wide(label::body(lead.clone()).wrapping(Wrapping::WordOrGlyph));
                }
                if let Some((need, name)) = ask.note {
                    form = self.portal_note(form, a, need, name, ask.note_hint);
                }
                if let Some(caution) = ask.caution {
                    form = form.wide(Banner::info(caution));
                }
                (ask.title, ask.confirm, ask.danger)
            }
        };
        if let Some(error) = &a.error {
            form = form.wide(Banner::error(error.clone()));
        }
        let go = (!a.busy).then(|| msg(Event::AskConfirm));
        let confirm: Element<'a, Message> = if danger {
            button(label::body(confirm))
                .on_press_maybe(go)
                .padding(metrics::action())
                .style(style::button::danger_outline)
                .into()
        } else {
            words::primary(confirm, go)
        };
        let mut frame = Frame::new(title)
            .key("portal-adim")
            .push(form)
            .action(words::secondary("Vazgeç", cancel))
            .action(confirm)
            .width(560.0);
        if a.busy {
            frame = frame.status(words::text_line(
                words::Kind::Info,
                "Sunucuya gönderiliyor…",
            ));
        }
        Some(frame.modal(msg(Event::AskClose)))
    }

    fn portal_note<'a>(
        &'a self,
        form: Form<'a, Message>,
        a: &'a Asking,
        need: plan::Note,
        name: &'static str,
        hint: Option<&'static str>,
    ) -> Form<'a, Message> {
        let required = need == plan::Note::Required;
        let editor = text_editor(&a.note)
            .placeholder(if required { "" } else { "İsteğe bağlı" })
            .on_action(|x| msg(Event::AskNote(x)))
            .height(Length::Fixed(typography::from_default(if required {
                112.0
            } else {
                66.0
            })))
            .size(typography::body())
            .padding([5, 8])
            .style(style::field::text_area);
        let mut form = form.field(name, editor);
        if required {
            form = form.required();
        }
        if let Some(h) = hint {
            form = form.help(h);
        }
        form.error(a.missing.as_deref())
    }
}
