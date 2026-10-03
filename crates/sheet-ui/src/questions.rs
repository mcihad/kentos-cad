//! A template's questions (design §12, “kullanırken sorulacaklar”; the web's
//! `TemplateQuestions`): asked before a sheet is made from a template (Kullan
//! in the gallery, Yeni pafta), one field per question of its kind — a line,
//! several lines when the template's value has them, a number, a date,
//! yes/no — filled with the template's own value. What is left empty is
//! written ‹ad?› on the paper and the preflight says so; Değişkenler fills it
//! later. Enter makes the sheet (Ctrl+Enter in a text of several lines), Esc
//! leaves it; over the gallery, leaving goes back to it.

use iced::keyboard::Key;
use iced::keyboard::key::Named;
use iced::widget::text_editor::{self, Binding, KeyPress};
use iced::widget::{button, column, container, scrollable, text_editor as editor};
use iced::{Element, Fill, Length};
use kentos_sheet::model::{VarKind, VarValue};
use kentos_sheet::template::{PaperChoice, Template, VariableValue};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::{Dialog, InputField, Switch};

use crate::designer::{Designer, Dialog as Window, Effect};
use crate::message::Message;
use crate::variables::{shown, value_of};

/// The web's words (`QUESTION_TEXTS`).
pub mod texts {
    pub const TITLE: &str = "Paftanın bilgileri";
    pub fn intro(name: &str) -> String {
        format!(
            "“{name}” şablonu bunları soruyor. Boş bıraktıklarınız paftada ‹ad?› diye yazılır ve ön denetim hatırlatır; sonra Değişkenler'den doldurabilirsiniz."
        )
    }
    pub const MAKE: &str = "Paftayı oluştur";
    pub const CANCEL: &str = "Vazgeç";
    pub const EMPTY: &str = "boş";
    pub const DATE: &str = "yyyy-aa-gg";
}

/// The window: the template, where its sheet goes, and what is typed for each question.
#[derive(Debug)]
pub struct QuestionsDialog {
    pub(crate) template: Template,
    pub(crate) paper: Option<PaperChoice>,
    /// The undo step's name.
    pub(crate) label: String,
    /// Each question's text (a yes/no question's is unused).
    pub texts: Vec<String>,
    /// A question of several lines: its editor (none: a line).
    pub(crate) areas: Vec<Option<text_editor::Content>>,
    pub bools: Vec<bool>,
    pub problem: Option<String>,
}

#[derive(Clone, Debug)]
pub enum QuestionsMessage {
    Text(usize, String),
    /// A question of several lines edited.
    Edit(usize, text_editor::Action),
    Bool(usize, bool),
    /// Paftayı oluştur.
    Make,
}

impl QuestionsDialog {
    /// The questions, in the template's order.
    pub fn template_variables(&self) -> &[kentos_sheet::model::Variable] {
        &self.template.variables
    }

    /// Whether question `i` is a text of several lines (an editor rather than a line).
    pub fn is_area(&self, i: usize) -> bool {
        matches!(self.areas.get(i), Some(Some(_)))
    }
}

/// A text of several lines: Ctrl+Enter makes the sheet, Enter starts a line.
fn area_keys(press: KeyPress) -> Option<Binding<Message>> {
    let focused = matches!(press.status, text_editor::Status::Focused { .. });
    match press.key.as_ref() {
        Key::Named(Named::Enter) if focused && press.modifiers.command() => {
            Some(Binding::Custom(Message::Questions(QuestionsMessage::Make)))
        }
        _ => Binding::from_key_press(press),
    }
}

impl Designer {
    /// The template's questions, while they are asked.
    pub fn questions(&self) -> Option<&QuestionsDialog> {
        match &self.dialog {
            Some(Window::Questions(d)) => Some(d),
            _ => None,
        }
    }

    /// Kullan and Yeni pafta: the template's questions first; a template that asks nothing makes
    /// its sheet straight away (the gallery closes once it is made).
    pub(crate) fn ask_then_use(
        &mut self,
        t: &Template,
        paper: Option<PaperChoice>,
        label: String,
    ) -> Vec<Effect> {
        if t.variables.is_empty() {
            if self.use_template_with(t, paper, &label, Vec::new()) {
                self.gallery = None;
                return vec![Effect::ShowSheet];
            }
            return Vec::new();
        }
        let texts: Vec<String> = t.variables.iter().map(|v| shown(&v.value)).collect();
        let areas = t
            .variables
            .iter()
            .zip(&texts)
            .map(|(v, text)| {
                (v.kind == VarKind::Text && text.contains('\n'))
                    .then(|| text_editor::Content::with_text(text))
            })
            .collect();
        self.dialog = Some(Window::Questions(Box::new(QuestionsDialog {
            template: t.clone(),
            paper,
            label,
            texts,
            areas,
            bools: t
                .variables
                .iter()
                .map(|v| matches!(v.value, VarValue::Bool(true)))
                .collect(),
            problem: None,
        })));
        Vec::new()
    }

    pub(crate) fn questions_update(&mut self, m: QuestionsMessage) -> Vec<Effect> {
        let Some(Window::Questions(d)) = &mut self.dialog else {
            return Vec::new();
        };
        match m {
            QuestionsMessage::Text(i, text) => {
                if let Some(t) = d.texts.get_mut(i) {
                    *t = text;
                    d.problem = None;
                }
            }
            QuestionsMessage::Edit(i, action) => {
                if let (Some(Some(area)), Some(t)) = (d.areas.get_mut(i), d.texts.get_mut(i)) {
                    let edit = action.is_edit();
                    area.perform(action);
                    if edit {
                        *t = area.text();
                        d.problem = None;
                    }
                }
            }
            QuestionsMessage::Bool(i, on) => {
                if let Some(b) = d.bools.get_mut(i) {
                    *b = on;
                }
            }
            QuestionsMessage::Make => {
                let mut values = Vec::with_capacity(d.template.variables.len());
                for (i, v) in d.template.variables.iter().enumerate() {
                    let value = if v.kind == VarKind::Bool {
                        VarValue::Bool(d.bools.get(i).copied().unwrap_or(false))
                    } else {
                        let text = d.texts.get(i).map_or("", String::as_str);
                        // A text of several lines keeps its lines (the web's text area).
                        let text = if v.kind == VarKind::Text && text.contains('\n') {
                            text.trim_end_matches('\n')
                        } else {
                            text
                        };
                        match value_of(v.kind, text) {
                            Some(x) => x,
                            None => {
                                d.problem = Some(format!("“@{}” bir sayı olmalı.", v.name));
                                return Vec::new();
                            }
                        }
                    };
                    values.push(VariableValue {
                        name: v.name.clone(),
                        value,
                    });
                }
                let (t, paper, label) = (d.template.clone(), d.paper, d.label.clone());
                if self.use_template_with(&t, paper, &label, values) {
                    self.dialog = None;
                    self.gallery = None;
                    return vec![Effect::ShowSheet];
                }
            }
        }
        Vec::new()
    }

    pub(crate) fn questions_view<'a>(&'a self, d: &'a QuestionsDialog) -> Element<'a, Message> {
        let qm = Message::Questions;
        let mut list = column![].spacing(10);
        for (i, v) in d.template.variables.iter().enumerate() {
            let title = format!(
                "{} · @{}",
                if v.label.is_empty() {
                    &v.name
                } else {
                    &v.label
                },
                v.name
            );
            let input: Element<'a, Message> = match (v.kind, d.areas.get(i)) {
                (VarKind::Bool, _) => {
                    Switch::new(d.bools.get(i).copied().unwrap_or(false), move |on| {
                        qm(QuestionsMessage::Bool(i, on))
                    })
                    .into()
                }
                (_, Some(Some(area))) => editor(area)
                    .on_action(move |a| qm(QuestionsMessage::Edit(i, a)))
                    .key_binding(area_keys)
                    .height(Length::Fixed(typography::from_default(84.0)))
                    .size(typography::body())
                    .padding([5, 7])
                    .style(style::field::text_area)
                    .into(),
                (kind, _) => InputField::new(
                    if kind == VarKind::Date {
                        texts::DATE
                    } else {
                        texts::EMPTY
                    },
                    d.texts.get(i).map_or("", String::as_str),
                )
                .on_input(move |t| qm(QuestionsMessage::Text(i, t)))
                .on_submit(qm(QuestionsMessage::Make))
                .width(Fill)
                .into(),
            };
            list = list
                .push(column![label::caption(title).style(style::text::muted), input].spacing(2));
        }
        let mut body = column![
            label::body(texts::intro(&d.template.meta.name)).style(style::text::muted),
            scrollable(container(list).padding([0, 8])).height(Length::Shrink),
        ]
        .spacing(12);
        if let Some(p) = &d.problem {
            body = body.push(label::body(p.as_str()).style(style::text::danger));
        }
        Dialog::new(texts::TITLE)
            .push(container(body).max_height(520.0))
            .action(
                button(label::body(texts::CANCEL))
                    .on_press(Message::CloseDialog)
                    .padding([6, 14])
                    .style(style::button::secondary),
            )
            .action(
                button(label::body(texts::MAKE))
                    .on_press(qm(QuestionsMessage::Make))
                    .padding([6, 16])
                    .style(style::button::primary),
            )
            .width(520.0)
            .into()
    }
}
