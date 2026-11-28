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
    /// A template that asks nothing: only where its main map looks (docs/adr/9187).
    pub fn intro_extent(name: &str) -> String {
        format!("“{name}” şablonu bir şey sormuyor; ana haritasının nereye bakacağını seçin.")
    }
    pub const MAKE: &str = "Paftayı oluştur";
    pub const CANCEL: &str = "Vazgeç";
    pub const EMPTY: &str = "boş";
    pub const DATE: &str = "yyyy-aa-gg";
    /// Where the main map looks (docs/adr/9147).
    pub const EXTENT: &str = "Kapsam";
    pub const LAYER: &str = "Katman seçin";
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
    /// The answers make a sheet per level (Kat planı paftaları, docs/adr/0718), not one sheet.
    pub(crate) levels: bool,
    /// Where the main map looks (docs/adr/9147): the choice, the layer of a Katman choice (its place in the context's
    /// layer boxes), and what the choice gives with its sentence. None of it without a main map to place.
    pub extent: Option<crate::new_extent::Extent>,
    pub layer: Option<usize>,
    pub placed: (crate::new_extent::Placement, String),
    /// The placement's picture: the page, the main map's frame and the boxes it shows (docs/adr/9187).
    pub picture: Option<crate::new_extent::Picture>,
}

#[derive(Clone, Debug)]
pub enum QuestionsMessage {
    Text(usize, String),
    /// A question of several lines edited.
    Edit(usize, text_editor::Action),
    Bool(usize, bool),
    /// Kapsam: where the main map looks (docs/adr/9147).
    Extent(crate::new_extent::Extent),
    /// Kapsam's layer, by its place in the context's layer boxes.
    Layer(usize),
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

    /// Kullan and Yeni pafta: the template's questions first, with where its main map looks; a template that asks
    /// nothing and has no main map to place (a print calibration sheet) makes its sheet straight away (the gallery
    /// closes once it is made). One that asks nothing but has a main map asks Kapsam alone (docs/adr/9187).
    pub(crate) fn ask_then_use(
        &mut self,
        t: &Template,
        paper: Option<PaperChoice>,
        label: String,
    ) -> Vec<Effect> {
        if t.variables.is_empty() && !crate::new_extent::has_main_map(t) {
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
            levels: false,
            extent: crate::new_extent::has_main_map(t).then_some(crate::new_extent::Extent::Fixed),
            layer: None,
            placed: (crate::new_extent::Placement::default(), String::new()),
            picture: None,
        })));
        self.questions_place();
        Vec::new()
    }

    /// The questions' extent worked out again: the placement and its sentence (docs/adr/9147).
    fn questions_place(&mut self) {
        let ctx = self.ctx.clone();
        let Some(Window::Questions(d)) = &mut self.dialog else {
            return;
        };
        if let Some(extent) = d.extent {
            d.placed = crate::new_extent::place(&d.template, d.paper, &ctx, extent, d.layer);
            d.picture =
                crate::new_extent::picture(&d.template, d.paper, &ctx, extent, d.layer, d.placed.0);
        }
    }

    /// Kat planı paftaları's questions: the template's, asked once for every level's sheet (docs/adr/0718).
    pub(crate) fn ask_then_levels(&mut self, t: &Template) -> Vec<Effect> {
        let label = crate::level_sheets::STEP.to_owned();
        let effects = self.ask_then_use(t, None, label);
        if let Some(Window::Questions(d)) = &mut self.dialog {
            d.levels = true;
            // Every level's sheet fits the levels' ground (docs/adr/0718): no extent of its own.
            d.extent = None;
        }
        effects
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
            QuestionsMessage::Extent(extent) => {
                if d.extent.is_some() {
                    d.extent = Some(extent);
                    // A Katman choice starts on the first layer that holds objects.
                    if extent == crate::new_extent::Extent::Layer
                        && d.layer.is_none()
                        && !self.ctx.layer_bounds.is_empty()
                    {
                        d.layer = Some(0);
                    }
                    self.questions_place();
                }
            }
            QuestionsMessage::Layer(i) => {
                if i < self.ctx.layer_bounds.len() {
                    d.layer = Some(i);
                    self.questions_place();
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
                let place = d.extent.map(|_| d.placed.0);
                if d.levels {
                    let effects = self.make_level_sheets(&t, values);
                    if effects.contains(&Effect::ShowSheet) {
                        self.dialog = None;
                    }
                    return effects;
                }
                if self.use_template_placed(&t, paper, &label, values, place) {
                    self.dialog = None;
                    self.gallery = None;
                    return vec![Effect::ShowSheet];
                }
            }
        }
        Vec::new()
    }

    /// Kapsam: the four choices, a Katman choice's layer, and the sentence of what the choice gives.
    fn extent_view<'a>(
        &'a self,
        d: &'a QuestionsDialog,
        extent: crate::new_extent::Extent,
    ) -> Element<'a, Message> {
        use crate::new_extent::{EXTENTS, Extent};
        use kentos_ui::widget::segmented::Segmented;
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        struct Choice(Extent, &'static str);
        impl std::fmt::Display for Choice {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.1)
            }
        }
        let all: Vec<Choice> = EXTENTS.iter().map(|(e, w)| Choice(*e, w)).collect();
        let chosen = all
            .iter()
            .copied()
            .find(|c| c.0 == extent)
            .unwrap_or(all[3]);
        // A choice with nothing to fit is dimmed: no view, no layer with objects, no drawing.
        let ctx = &self.ctx;
        let can = |c: Choice| match c.0 {
            Extent::View => ctx.center.is_some() && ctx.view_size.is_some(),
            Extent::Layer => !ctx.layer_bounds.is_empty(),
            Extent::Drawing => ctx.drawing_bounds.is_some(),
            Extent::Fixed => true,
        };
        let choices = Segmented::new_with(
            all,
            chosen,
            |c: Choice| Message::Questions(QuestionsMessage::Extent(c.0)),
            can,
        )
        .width(Fill);
        let mut col = column![
            label::caption(texts::EXTENT).style(style::text::muted),
            choices
        ]
        .spacing(4);
        if extent == Extent::Layer {
            let names: Vec<kentos_ui::widget::select::Choice> = (0..self.ctx.layer_bounds.len())
                .map(|i| {
                    kentos_ui::widget::select::Choice::new(crate::new_extent::layer_name(
                        &self.ctx, i,
                    ))
                })
                .collect();
            col = col.push(
                kentos_ui::widget::select::Select::new(names, d.layer, |i| {
                    Message::Questions(QuestionsMessage::Layer(i))
                })
                .placeholder(texts::LAYER),
            );
        }
        // The words beside the picture of what they say (docs/adr/9187).
        let words = label::body(d.placed.1.as_str());
        match &d.picture {
            Some(p) => col
                .push(
                    iced::widget::row![
                        crate::extent_picture::picture(p),
                        iced::widget::container(words).width(Fill)
                    ]
                    .spacing(12)
                    .align_y(iced::Center),
                )
                .into(),
            None => col.push(words).into(),
        }
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
        // A template that asks nothing asks Kapsam alone (docs/adr/9187).
        let intro = if d.template.variables.is_empty() {
            texts::intro_extent(&d.template.meta.name)
        } else {
            texts::intro(&d.template.meta.name)
        };
        let mut body = column![label::body(intro).style(style::text::muted)].spacing(12);
        // Kapsam: where the main map looks, with what it gives (docs/adr/9147).
        if let Some(extent) = d.extent {
            body = body.push(self.extent_view(d, extent));
        }
        let mut body =
            body.push(scrollable(container(list).padding([0, 8])).height(Length::Shrink));
        if let Some(p) = &d.problem {
            body = body.push(label::body(p.as_str()).style(style::text::danger));
        }
        Dialog::new(texts::TITLE)
            .on_close(crate::Message::CloseDialog)
            .push(container(body).max_height(520.0))
            .action(
                button(label::body(texts::CANCEL))
                    .on_press(Message::CloseDialog)
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::secondary),
            )
            .action(
                button(label::body(texts::MAKE))
                    .on_press(qm(QuestionsMessage::Make))
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::primary),
            )
            .width(520.0)
            .into()
    }
}

#[cfg(test)]
#[path = "questions_tests.rs"]
mod questions_tests;
