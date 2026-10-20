//! A text's fields and the text commands on the desktop (docs/adr/0325 §4), each by its command: Alan ekle
//! (`text.fieldInsert`: a window, the object picked on the drawing, `cad.fields.edit`'s insert at the text's end),
//! Alanları yenile (`text.fieldsUpdate`), a field's Alanı kaldır (Öznitelikler), Numarala (`text.number`: a window,
//! the selected texts in the order they were selected), Yazıları birleştir (`text.combine`), Satırlara ayır
//! (`text.split`) and Yazı denetimi (`text.check`: a window of `cad.texts.check`'s findings, each taking the view to
//! its text).

use std::fmt;

use iced::widget::{Column, button, container, row, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, Entity, FieldEdit, FieldFormat, FieldSource, FieldUnit, FieldsEdit, NumberPlace,
    ObjectValue, ProjectValue, TextEdit, TextFindingKind, TextsCheck, TextsChecked, TextsEdit,
};
use kentos_domain::{Slot, Uuid};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, fields_edit, texts_check, texts_edit};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::{Dialog as Frame, Segmented, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};

/// The windows' titles, which a trace names them by.
pub const FIELD_TITLE: &str = "Alan ekle";
pub const NUMBER_TITLE: &str = "Numarala";
pub const CHECK_TITLE: &str = "Yazı denetimi";

const OBJECT_VALUES: [ObjectValue; 9] = [
    ObjectValue::Area,
    ObjectValue::Perimeter,
    ObjectValue::Length,
    ObjectValue::Layer,
    ObjectValue::Label,
    ObjectValue::Attribute,
    ObjectValue::Text,
    ObjectValue::X,
    ObjectValue::Y,
];
const PROJECT_VALUES: [ProjectValue; 5] = [
    ProjectValue::Name,
    ProjectValue::Scale,
    ProjectValue::Unit,
    ProjectValue::Crs,
    ProjectValue::Variable,
];
const AREA_UNITS: [Option<FieldUnit>; 4] = [
    None,
    Some(FieldUnit::M2),
    Some(FieldUnit::Donum),
    Some(FieldUnit::Ha),
];
const LENGTH_UNITS: [Option<FieldUnit>; 5] = [
    None,
    Some(FieldUnit::Mm),
    Some(FieldUnit::Cm),
    Some(FieldUnit::M),
    Some(FieldUnit::Km),
];

/// Where a field's value comes from, as the window's switch says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum From {
    Object,
    Project,
}

impl fmt::Display for From {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            From::Object => "Nesne",
            From::Project => "Proje",
        })
    }
}

/// Where Numarala writes, as the window's switch says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Place(pub NumberPlace);

impl fmt::Display for Place {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            NumberPlace::Replace => "Yerine",
            NumberPlace::Prefix => "Başına",
            NumberPlace::Suffix => "Sonuna",
        })
    }
}

/// Alan ekle's window.
#[derive(Debug, Clone)]
pub struct FieldWindow {
    text: Slot,
    from: From,
    object: Option<Slot>,
    value: usize,
    name: String,
    decimals: String,
    unit: usize,
    comma: bool,
    prefix: String,
    suffix: String,
    /// Picking the object on the drawing: the selection to give back.
    picking: Option<Vec<Slot>>,
}

/// Numarala's window.
#[derive(Debug, Clone)]
pub struct NumberWindow {
    texts: Vec<Slot>,
    start: String,
    step: String,
    digits: String,
    prefix: String,
    suffix: String,
    place: Place,
}

/// The windows open.
#[derive(Debug, Clone, Default)]
pub struct TextTools {
    field: Option<FieldWindow>,
    number: Option<NumberWindow>,
    check: Option<TextsChecked>,
}

#[derive(Debug, Clone)]
pub enum Event {
    From(From),
    Value(usize),
    Name(String),
    Decimals(String),
    Unit(usize),
    Comma(bool),
    Prefix(String),
    Suffix(String),
    Pick,
    Insert,
    Start(String),
    Step(String),
    Digits(String),
    NumberPrefix(String),
    NumberSuffix(String),
    Place(Place),
    Number,
    /// A finding's text: the view goes to it, selected.
    Go(Slot),
    /// A text's field taken out (Öznitelikler).
    Remove(Slot, u32),
    Close,
}

fn msg(event: Event) -> Message {
    Message::TextTools(event)
}

/// What a command answered, as the app says it: its warnings, or its refusal; whether it was done.
fn answered<T>(app: &mut App, result: &CommandResult<T>) -> bool {
    match result {
        CommandResult::Completed { warnings, .. } => {
            for w in warnings {
                app.warn(w.message.clone());
            }
            true
        }
        CommandResult::Failed { error }
        | CommandResult::Conflict { error }
        | CommandResult::NeedsInput { error } => {
            app.warn(error.message.clone());
            false
        }
        CommandResult::Queued { .. } | CommandResult::Cancelled => false,
    }
}

impl FieldWindow {
    fn units(&self) -> &'static [Option<FieldUnit>] {
        match (self.from, OBJECT_VALUES.get(self.value)) {
            (From::Object, Some(ObjectValue::Area)) => &AREA_UNITS,
            (From::Object, Some(v)) if v.is_number() => &LENGTH_UNITS,
            _ => &[],
        }
    }

    fn named(&self) -> bool {
        match self.from {
            From::Object => OBJECT_VALUES.get(self.value) == Some(&ObjectValue::Attribute),
            From::Project => PROJECT_VALUES.get(self.value) == Some(&ProjectValue::Variable),
        }
    }

    /// The command's input; none while the object is not picked or a number does not read.
    fn input(&self, app: &App) -> Result<FieldsEdit, String> {
        let doc = app.document.as_ref().ok_or("Açık çizim yok.")?;
        let Some(Entity::Text(t)) = doc.model.get(self.text) else {
            return Err("Yazı çizimde yok.".into());
        };
        let uid = doc.model.uid(self.text).ok_or("Yazı çizimde yok.")?;
        let name = self.named().then(|| self.name.trim().to_owned());
        let source = match self.from {
            From::Object => {
                let object = self
                    .object
                    .and_then(|s| doc.model.uid(s))
                    .ok_or("Alanın nesnesini çizimden seçin.")?;
                FieldSource::Object {
                    uid: kentos_commands::EntityId(*object.as_bytes()),
                    value: OBJECT_VALUES[self.value],
                    name,
                }
            }
            From::Project => FieldSource::Project {
                value: PROJECT_VALUES[self.value],
                name,
            },
        };
        let decimals = match self.decimals.trim() {
            "" => None,
            d => Some(
                d.parse::<u8>()
                    .map_err(|_| "Basamak 0 ile 10 arasında bir tam sayı olmalı.")?,
            ),
        };
        let format = FieldFormat {
            decimals,
            unit: self.units().get(self.unit).copied().flatten(),
            comma: self.comma,
            prefix: self.prefix.clone(),
            suffix: self.suffix.clone(),
        };
        let at = t.text.chars().count() as u32;
        Ok(FieldsEdit {
            edit: FieldEdit::Insert {
                uid: uid.to_string(),
                start: at,
                end: at,
                source,
                format: (!format.is_plain()).then_some(format),
            },
            expected_revision: None,
        })
    }
}

impl App {
    /// The single text selected, or the text and one other object (the field's source).
    fn text_and_object(&self) -> Option<(Slot, Option<Slot>)> {
        let doc = self.document.as_ref()?;
        let ids = self.selection.ids();
        let is_text = |s: &Slot| matches!(doc.model.get(*s), Some(Entity::Text(_)));
        match ids {
            [one] if is_text(one) => Some((*one, None)),
            [a, b] if is_text(a) => Some((*a, Some(*b))),
            [a, b] if is_text(b) => Some((*b, Some(*a))),
            _ => None,
        }
    }

    /// The texts selected, in the order they were selected.
    fn selected_texts(&self) -> Vec<Slot> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        self.selection
            .ids()
            .iter()
            .copied()
            .filter(|s| matches!(doc.model.get(*s), Some(Entity::Text(_))))
            .collect()
    }

    /// The text commands by their ids; false for any other.
    pub(crate) fn text_tools_run(&mut self, id: &str) -> bool {
        match id {
            "text.fieldInsert" => self.open_field_insert(),
            "text.fieldsUpdate" => self.fields_update(),
            "text.number" => self.open_number(),
            "text.combine" => self.texts_edit_selected(true),
            "text.split" => self.texts_edit_selected(false),
            "text.check" => self.open_text_check(),
            _ => return false,
        }
        true
    }

    fn open_field_insert(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let Some((slot, object)) = self.text_and_object() else {
            self.warn(
                "Alan ekle: önce bir yazı seçin (alanın nesnesi de seçiliyse o kaynak olur).",
            );
            return;
        };
        let ends_open = self
            .document
            .as_ref()
            .and_then(|d| match d.model.get(slot) {
                Some(Entity::Text(t)) => t.text.chars().last(),
                _ => None,
            })
            .is_some_and(|c| !c.is_whitespace());
        self.text_tools.field = Some(FieldWindow {
            text: slot,
            from: From::Object,
            object,
            value: 0,
            name: String::new(),
            decimals: String::new(),
            unit: 0,
            comma: false,
            prefix: if ends_open { " ".into() } else { String::new() },
            suffix: String::new(),
            picking: None,
        });
        self.dialog = Some(Dialog::FieldInsert);
    }

    /// Alanları yenile: the selected texts' fields, or every text's, written again.
    fn fields_update(&mut self) {
        let texts = self.selected_texts();
        let uids = self.uids_of(&texts);
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let input = FieldsEdit {
            edit: FieldEdit::Update { uids },
            expected_revision: None,
        };
        let result = fields_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        if answered(self, &result)
            && let CommandResult::Completed { output, .. } = result
        {
            self.say(
                Level::Success,
                if output.changed.is_empty() {
                    "Alanlar güncel.".to_owned()
                } else {
                    format!("{} yazının alanları yenilendi.", output.changed.len())
                },
            );
        }
    }

    /// Yazıları birleştir (`combine`) or Satırlara ayır: the selected texts by `cad.texts.edit`, the result selected.
    fn texts_edit_selected(&mut self, combine: bool) {
        let texts = self.selected_texts();
        let uids = self.uids_of(&texts);
        let edit = if combine {
            if uids.len() < 2 {
                self.warn("Yazıları birleştir: en az iki yazı seçin.");
                return;
            }
            TextEdit::Combine { uids }
        } else {
            let [uid] = uids.as_slice() else {
                self.warn("Satırlara ayır: bir çok satırlı yazı seçin.");
                return;
            };
            TextEdit::Split { uid: uid.clone() }
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let result = texts_edit::execute(
            &mut ExecutionContext::new(&mut doc.model),
            TextsEdit {
                edit,
                expected_revision: None,
            },
        );
        if answered(self, &result)
            && let CommandResult::Completed { output, .. } = result
            && let Some(doc) = &self.document
        {
            let slots: Vec<Slot> = output
                .changed
                .iter()
                .chain(&output.added)
                .filter_map(|u| Uuid::parse_str(u).ok())
                .filter_map(|u| doc.model.slot_of(u))
                .collect();
            let said = if combine {
                format!("{} yazı birleştirildi.", output.removed.len() + 1)
            } else {
                format!("Yazı {} satıra ayrıldı.", output.added.len())
            };
            self.selection.set(slots);
            self.say(Level::Success, said);
        }
    }

    fn open_number(&mut self) {
        let texts = self.selected_texts();
        if texts.is_empty() {
            self.warn("Numarala: önce numaralanacak yazıları sırayla seçin.");
            return;
        }
        self.text_tools.number = Some(NumberWindow {
            texts,
            start: "1".into(),
            step: "1".into(),
            digits: "1".into(),
            prefix: String::new(),
            suffix: String::new(),
            place: Place(NumberPlace::Replace),
        });
        self.dialog = Some(Dialog::TextNumber);
    }

    fn number_input(&self) -> Result<TextsEdit, String> {
        let w = self.text_tools.number.as_ref().ok_or("")?;
        let whole = |t: &str, what: &str| {
            t.trim()
                .parse::<i64>()
                .map_err(|_| format!("{what} bir tam sayı olmalı."))
        };
        let digits = w
            .digits
            .trim()
            .parse::<u8>()
            .map_err(|_| "Basamak 1 ile 12 arasında olmalı.".to_owned())?;
        Ok(TextsEdit {
            edit: TextEdit::Number {
                uids: self.uids_of(&w.texts),
                start: whole(&w.start, "Başlangıç")?,
                step: whole(&w.step, "Artış")?,
                digits: Some(digits),
                prefix: w.prefix.clone(),
                suffix: w.suffix.clone(),
                place: w.place.0,
            },
            expected_revision: None,
        })
    }

    fn open_text_check(&mut self) {
        let texts = self.selected_texts();
        let uids = self.uids_of(&texts);
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return;
        };
        match texts_check::answer(&doc.model, &TextsCheck { uids }) {
            CommandResult::Completed { output, .. } => {
                self.text_tools.check = Some(output);
                self.dialog = Some(Dialog::TextCheck);
            }
            CommandResult::Failed { error } => self.warn(error.message),
            _ => {}
        }
    }

    /// Alan ekle's object picked on the drawing (or none: Esc): the window comes back. False when it was not picking.
    pub(crate) fn text_tools_picked(&mut self, keep: bool) -> bool {
        let Some(w) = self.text_tools.field.as_mut() else {
            return false;
        };
        let Some(before) = w.picking.take() else {
            return false;
        };
        if keep && let Some(picked) = self.selection.ids().first().copied() {
            w.object = Some(picked);
        }
        self.selection.set(before);
        self.dialog = Some(Dialog::FieldInsert);
        true
    }

    pub(crate) fn text_tools_event(&mut self, event: Event) -> Task<Message> {
        if let Some(w) = self.text_tools.field.as_mut() {
            match &event {
                Event::From(f) => {
                    w.from = *f;
                    w.value = 0;
                    w.unit = 0;
                }
                Event::Value(i) => {
                    w.value = *i;
                    w.unit = 0;
                }
                Event::Name(t) => w.name.clone_from(t),
                Event::Decimals(t) => w.decimals.clone_from(t),
                Event::Unit(i) => w.unit = *i,
                Event::Comma(on) => w.comma = *on,
                Event::Prefix(t) => w.prefix.clone_from(t),
                Event::Suffix(t) => w.suffix.clone_from(t),
                _ => {}
            }
        }
        if let Some(w) = self.text_tools.number.as_mut() {
            match &event {
                Event::Start(t) => w.start.clone_from(t),
                Event::Step(t) => w.step.clone_from(t),
                Event::Digits(t) => w.digits.clone_from(t),
                Event::NumberPrefix(t) => w.prefix.clone_from(t),
                Event::NumberSuffix(t) => w.suffix.clone_from(t),
                Event::Place(p) => w.place = *p,
                _ => {}
            }
        }
        match event {
            Event::Pick => {
                let before = vec![self.text_tools.field.as_ref().map_or(Slot(0), |w| w.text)];
                if let Some(w) = self.text_tools.field.as_mut() {
                    w.picking = Some(before);
                }
                self.dialog = None;
                self.selection.clear();
                self.field = None;
                self.snap = None;
                self.say(Level::Command, "Alan ekle: alanın nesnesi");
                self.session.run(Box::new(
                    kentos_interaction::pick_objects::PickObjects::one("Alanın nesnesi", None),
                ));
                self.with_tool(|s, cx| s.activate(cx));
            }
            Event::Insert => {
                let input = self.text_tools.field.as_ref().map(|w| w.input(self));
                match input {
                    Some(Ok(input)) => {
                        let Some(doc) = self.document.as_mut() else {
                            return Task::none();
                        };
                        let result =
                            fields_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
                        if answered(self, &result) {
                            self.say(Level::Success, "Alan eklendi.");
                            self.text_tools.field = None;
                            self.dialog = None;
                        }
                    }
                    Some(Err(why)) => self.warn(why),
                    None => {}
                }
            }
            Event::Number => match self.number_input() {
                Ok(input) => {
                    let Some(doc) = self.document.as_mut() else {
                        return Task::none();
                    };
                    let n = self.text_tools.number.as_ref().map_or(0, |w| w.texts.len());
                    let result =
                        texts_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
                    if answered(self, &result) {
                        self.say(Level::Success, format!("{n} yazı numaralandı."));
                        self.text_tools.number = None;
                        self.dialog = None;
                    }
                }
                Err(why) => self.warn(why),
            },
            Event::Go(slot) => {
                self.selection.set([slot]);
                return self.run("view.zoomSelection");
            }
            Event::Remove(slot, index) => {
                let uid = self.uids_of(&[slot]).pop().unwrap_or_default();
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let input = FieldsEdit {
                    edit: FieldEdit::Remove { uid, index },
                    expected_revision: None,
                };
                let result =
                    fields_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
                if answered(self, &result) {
                    self.say(Level::Success, "Alan kaldırıldı; değeri yazı olarak kaldı.");
                }
            }
            Event::Close => {
                self.text_tools = TextTools::default();
                self.dialog = None;
            }
            _ => {}
        }
        Task::none()
    }

    pub(crate) fn field_insert_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.text_tools.field else {
            return label::caption("").into();
        };
        let input =
            |placeholder: &'static str, value: &str, on: fn(String) -> Event, width: f32| {
                focus_ring(
                    text_input(placeholder, value)
                        .on_input(move |t| msg(on(t)))
                        .padding([5, 8])
                        .width(width)
                        .style(style::field::input),
                )
            };
        let doc = self.document.as_ref();
        let words_of = doc.and_then(|d| match d.model.get(w.text) {
            Some(Entity::Text(t)) => Some(t.text.replace('\n', " ⏎ ")),
            _ => None,
        });
        let mut body = Column::new().spacing(12).push(words::summary(vec![words::text_line(
            Kind::Info,
            format!(
                "“{}” yazısının sonuna bir alan eklenir; değeri kaynağı değişince aynı adımda yenilenir, okunamazsa #### olur.",
                words_of.unwrap_or_default()
            ),
        )]));
        let from = words::field(
            "Kaynak",
            Segmented::new([From::Object, From::Project], w.from, |f| {
                msg(Event::From(f))
            }),
            None,
        );
        let values: Vec<Choice> = match w.from {
            From::Object => OBJECT_VALUES
                .iter()
                .map(|v| Choice::new(v.label()))
                .collect(),
            From::Project => PROJECT_VALUES
                .iter()
                .map(|v| Choice::new(v.label()))
                .collect(),
        };
        let value = words::field(
            "Değer",
            Select::new(values, Some(w.value), |i| msg(Event::Value(i))),
            None,
        );
        let mut first = row![from, value].spacing(18).align_y(iced::Bottom);
        if w.from == From::Object {
            let object = w
                .object
                .and_then(|s| doc.and_then(|d| d.model.get(s)))
                .map_or("Seçilmedi".to_owned(), |e| {
                    let kind = crate::selecting::kind_title(e.kind());
                    match e.base().label.as_deref().filter(|l| !l.is_empty()) {
                        Some(l) => format!("{kind} “{l}”"),
                        None => kind.to_owned(),
                    }
                });
            let pick = button(
                row![icon(Icon::Target).size(14.0), label::body("Sahneden seç")]
                    .spacing(6)
                    .align_y(Center),
            )
            .on_press(msg(Event::Pick))
            .padding([5, 10])
            .style(style::button::secondary);
            first = first.push(words::field(
                "Nesne",
                row![label::body(object), pick].spacing(10).align_y(Center),
                None,
            ));
        }
        body = body.push(first);
        let mut second = row![].spacing(18).align_y(iced::Bottom);
        if w.named() {
            let what = if w.from == From::Object {
                "Özniteliğin adı"
            } else {
                "Değişkenin adı"
            };
            second = second.push(words::field(
                what,
                input("Ad", &w.name, Event::Name, 160.0),
                None,
            ));
        }
        let units = w.units();
        if !units.is_empty() {
            let choices: Vec<Choice> = units
                .iter()
                .map(|u| Choice::new(u.map_or("Projeninki", FieldUnit::name)))
                .collect();
            second = second
                .push(words::field(
                    "Basamak",
                    input("Projeninki", &w.decimals, Event::Decimals, 100.0),
                    None,
                ))
                .push(words::field(
                    "Birim",
                    Select::new(choices, Some(w.unit), |i| msg(Event::Unit(i))),
                    None,
                ))
                .push(words::check(
                    w.comma,
                    "Ondalık virgül",
                    Some(msg(Event::Comma(!w.comma))),
                ));
        }
        body = body.push(second);
        body = body.push(
            row![
                words::field("Ön ek", input("", &w.prefix, Event::Prefix, 160.0), None),
                words::field(
                    "Son ek",
                    input(" m²", &w.suffix, Event::Suffix, 160.0),
                    None
                ),
            ]
            .spacing(18),
        );
        // What it would write, by the command's plan.
        let preview = match w.input(self) {
            Ok(input) => match doc.map(|d| fields_edit::plan_in(&d.model, &input)) {
                Some(CommandResult::Completed { output, warnings }) => {
                    let after = output.texts.first().and_then(|e| match e {
                        Entity::Text(t) => Some(t.text.replace('\n', " ⏎ ")),
                        _ => None,
                    });
                    let mut lines = vec![words::text_line(
                        Kind::Info,
                        format!("Yazı olacak: “{}”", after.unwrap_or_default()),
                    )];
                    lines.extend(
                        warnings
                            .into_iter()
                            .map(|w| words::text_line(Kind::Warn, w.message)),
                    );
                    lines
                }
                Some(CommandResult::Failed { error }) => {
                    vec![words::text_line(Kind::Error, error.message)]
                }
                _ => Vec::new(),
            },
            Err(why) => vec![words::text_line(Kind::Warn, why)],
        };
        body = body.push(words::summary(preview));
        overlay::modal(
            Frame::new(FIELD_TITLE)
                .push(body)
                .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
                .action(words::primary(
                    "Ekle",
                    w.input(self).is_ok().then(|| msg(Event::Insert)),
                ))
                .width(640.0),
            msg(Event::Close),
        )
    }

    pub(crate) fn text_number_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.text_tools.number else {
            return label::caption("").into();
        };
        let input =
            |placeholder: &'static str, value: &str, on: fn(String) -> Event, width: f32| {
                focus_ring(
                    text_input(placeholder, value)
                        .on_input(move |t| msg(on(t)))
                        .padding([5, 8])
                        .width(width)
                        .style(style::field::input),
                )
            };
        let mut lines = vec![words::text_line(
            Kind::Info,
            format!(
                "Seçili {} yazı seçildikleri sırayla numaralanır.",
                w.texts.len()
            ),
        )];
        match self.number_input() {
            Ok(input) => match self
                .document
                .as_ref()
                .map(|d| texts_edit::plan_in(&d.model, &input))
            {
                Some(CommandResult::Completed { output, .. }) => {
                    let shown: Vec<String> = output
                        .texts
                        .iter()
                        .take(3)
                        .filter_map(|e| match e {
                            Entity::Text(t) => Some(format!("“{}”", t.text)),
                            _ => None,
                        })
                        .collect();
                    let more = if output.texts.len() > 3 { " …" } else { "" };
                    lines.push(words::text_line(
                        Kind::Info,
                        format!("Olacak: {}{more}", shown.join(", ")),
                    ));
                }
                Some(CommandResult::Failed { error }) => {
                    lines.push(words::text_line(Kind::Error, error.message))
                }
                _ => {}
            },
            Err(why) => lines.push(words::text_line(Kind::Warn, why)),
        }
        let body = Column::new()
            .spacing(12)
            .push(words::summary(lines))
            .push(
                row![
                    words::field("Başlangıç", input("1", &w.start, Event::Start, 90.0), None),
                    words::field("Artış", input("1", &w.step, Event::Step, 90.0), None),
                    words::field("Basamak", input("1", &w.digits, Event::Digits, 90.0), None),
                ]
                .spacing(18),
            )
            .push(
                row![
                    words::field(
                        "Ön ek",
                        input("K-", &w.prefix, Event::NumberPrefix, 140.0),
                        None
                    ),
                    words::field(
                        "Son ek",
                        input("", &w.suffix, Event::NumberSuffix, 140.0),
                        None
                    ),
                    words::field(
                        "Yer",
                        Segmented::new(
                            [
                                Place(NumberPlace::Replace),
                                Place(NumberPlace::Prefix),
                                Place(NumberPlace::Suffix)
                            ],
                            w.place,
                            |p| msg(Event::Place(p))
                        ),
                        None
                    ),
                ]
                .spacing(18)
                .align_y(iced::Bottom),
            );
        overlay::modal(
            Frame::new(NUMBER_TITLE)
                .push(body)
                .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
                .action(words::primary(
                    "Numarala",
                    self.number_input().is_ok().then(|| msg(Event::Number)),
                ))
                .width(560.0),
            msg(Event::Close),
        )
    }

    pub(crate) fn text_check_view(&self) -> Element<'_, Message> {
        let Some(found) = &self.text_tools.check else {
            return label::caption("").into();
        };
        let doc = self.document.as_ref();
        let kind_text = |k: TextFindingKind| match k {
            TextFindingKind::DoubleSpace => "Çift boşluk",
            TextFindingKind::RepeatedWord => "Yinelenen sözcük",
            TextFindingKind::Unbalanced => "Parantez, tırnak",
            TextFindingKind::BrokenField => "Kopuk alan",
        };
        let widths: [f32; 4] = [200.0, 130.0, 250.0, 70.0];
        let columns = ["Yazı", "Bulgu", "Açıklama", ""]
            .iter()
            .zip(widths)
            .map(|(h, w)| table::Column::new(*h).width(w));
        let body: Vec<table::Row<'_, Message>> = found
            .findings
            .iter()
            .filter_map(|f| {
                let slot = doc.and_then(|d| d.model.slot_of(Uuid::parse_str(&f.uid).ok()?))?;
                let words = match doc.and_then(|d| d.model.get(slot)) {
                    Some(Entity::Text(t)) if t.text.chars().count() > 28 => {
                        format!(
                            "{}…",
                            t.text
                                .chars()
                                .take(27)
                                .collect::<String>()
                                .replace('\n', " ⏎ ")
                        )
                    }
                    Some(Entity::Text(t)) => t.text.replace('\n', " ⏎ "),
                    _ => String::new(),
                };
                let go = button(label::caption("Göster"))
                    .on_press(msg(Event::Go(slot)))
                    .padding([2, 8])
                    .style(style::button::ghost);
                Some(table::Row::new([
                    label::caption(words).into(),
                    label::caption(kind_text(f.kind)).into(),
                    label::caption(f.words.clone()).into(),
                    go.into(),
                ]))
            })
            .collect();
        let summary = if found.findings.is_empty() {
            format!("{} yazı okundu; biçim sorunu yok.", found.texts)
        } else {
            format!(
                "{} yazıda {} bulgu. Sözlükle yazım denetimi yapılmaz: çift boşluk, yinelenen sözcük, parantez ve tırnak, kopuk alan aranır.",
                found.texts,
                found.findings.len()
            )
        };
        let list = container(
            Table::new(columns)
                .extend(body)
                .min_width(widths.iter().sum::<f32>() + 8.0 * 4.0 + 20.0)
                .height(260.0),
        )
        .style(style::container::field_box)
        .width(Fill);
        overlay::modal(
            Frame::new(CHECK_TITLE)
                .push(
                    Column::new()
                        .spacing(12)
                        .push(words::summary(vec![words::text_line(Kind::Info, summary)]))
                        .push(list),
                )
                .action(words::primary("Kapat", Some(msg(Event::Close))))
                .width(720.0),
            msg(Event::Close),
        )
    }
}
