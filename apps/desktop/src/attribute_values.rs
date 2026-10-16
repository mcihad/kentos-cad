//! Öznitelik değerleri (the web's `ui/blocks/AttributeValuesDialog.ts`,
//! docs/adr/0144 §7): Blok ekle's question before it places a block with
//! attribute definitions (`ViewChange::AttributeValues`). One field per
//! attribute, named by its prompt (else its tag), its default already in
//! it. Yerleştir gives the tool the values that are not empty and not the
//! default (an attribute left at its default follows it, as an insert
//! without a value shows it); Vazgeç, Esc or a click beside it drops the
//! point and the tool waits for the next.
//!
//! The options (docs/adr/0317 §1): a constant, a preset and a field attribute are not asked (the command fills a
//! field); a verified one has a second field, Yerleştir waiting until the two are the same; a multiline one is a
//! text box of lines.

use std::collections::BTreeMap;

use iced::widget::{Column, Id, text, text_editor, text_input};
use iced::{Element, Task};
use kentos_contracts::BlockId;
use kentos_interaction::js_trim;
use kentos_ui::style;
use kentos_ui::widget::{Dialog as Frame, focus_ring, overlay};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const VALUES_TITLE: &str = "Öznitelik değerleri";
const PLACE: &str = "Yerleştir";
const CANCEL: &str = "Vazgeç";

/// One attribute's field: its tag, words, default and what is in it.
#[derive(Debug)]
struct Field {
    tag: String,
    label: String,
    default: String,
    text: String,
    /// A verified attribute's second field and its words.
    again: Option<String>,
    again_label: String,
    /// A multiline attribute's lines.
    lines: Option<text_editor::Content>,
}

impl Field {
    fn value(&self) -> String {
        self.lines.as_ref().map_or_else(|| self.text.clone(), |c| {
            c.text().trim_end_matches('\n').to_owned()
        })
    }
}

/// The window: the block, and its attributes' fields.
#[derive(Debug)]
pub struct Window {
    name: String,
    fields: Vec<Field>,
    /// The window opened: its first field takes the keyboard.
    focus: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Text(usize, String),
    /// A verified attribute's second field.
    Again(usize, String),
    /// A multiline attribute's text box.
    Lines(usize, text_editor::Action),
    Place,
    Close,
}

fn msg(event: Event) -> Message {
    Message::AttributeValues(event)
}

/// The first field, which takes the keyboard when the window opens (the
/// trace player follows it: traces/command_line.rs).
pub(crate) const FIRST_FIELD: &str = "attribute-value-0";

fn field_id(i: usize) -> Id {
    if i == 0 {
        Id::new(FIRST_FIELD)
    } else {
        Id::from(format!("attribute-value-{i}"))
    }
}

impl Window {
    /// The values to write: those not empty and not the default, trimmed.
    fn values(&self) -> BTreeMap<String, String> {
        self.fields
            .iter()
            .filter_map(|f| {
                let value = f.value();
                let v = js_trim(&value);
                (!v.is_empty() && v != js_trim(&f.default)).then(|| (f.tag.clone(), v.to_owned()))
            })
            .collect()
    }

    /// The first verified attribute whose two fields differ.
    fn unverified(&self) -> Option<&str> {
        self.fields
            .iter()
            .find(|f| f.again.as_deref().is_some_and(|a| js_trim(a) != js_trim(&f.text)))
            .map(|f| f.label.as_str())
    }
}

impl App {
    /// Blok ekle's point for `block` (input.rs): the window asks its attributes' values.
    pub(crate) fn open_attribute_values(&mut self, block: BlockId) {
        let Some(b) = self.document.as_ref().and_then(|d| d.model.block(block)) else {
            return;
        };
        let fields = b
            .attributes
            .iter()
            // Not asked: a constant, a preset and a field attribute (docs/adr/0317 §1).
            .filter(|a| !a.options.constant && !a.options.preset && a.options.field.is_none())
            .map(|a| {
                let label = a
                    .prompt
                    .as_deref()
                    .map(js_trim)
                    .filter(|p| !p.is_empty())
                    .unwrap_or(&a.tag)
                    .to_owned();
                let default = a.value.clone().unwrap_or_default();
                Field {
                    tag: a.tag.clone(),
                    again_label: format!("{label} (doğrulayın)"),
                    label,
                    text: default.clone(),
                    again: a.options.verify.then(String::new),
                    lines: a
                        .options
                        .multiline
                        .then(|| text_editor::Content::with_text(&default)),
                    default,
                }
            })
            .collect();
        self.attribute_values = Some(Window {
            name: b.name.clone(),
            fields,
            focus: true,
        });
        self.dialog = Some(Dialog::AttributeValues);
    }

    pub(crate) fn attribute_values_event(&mut self, event: Event) {
        let Some(w) = self.attribute_values.as_mut() else {
            return;
        };
        match event {
            Event::Text(i, t) => {
                if let Some(f) = w.fields.get_mut(i) {
                    f.text = t;
                }
            }
            Event::Again(i, t) => {
                if let Some(f) = w.fields.get_mut(i) {
                    f.again = Some(t);
                }
            }
            Event::Lines(i, action) => {
                if let Some(c) = w.fields.get_mut(i).and_then(|f| f.lines.as_mut()) {
                    c.perform(action);
                }
            }
            Event::Place => {
                if w.unverified().is_some() {
                    return;
                }
                let values = w.values();
                self.attribute_values = None;
                self.dialog = None;
                self.with_tool(|s, cx| s.values_given(Some(&values), cx));
            }
            Event::Close => self.attribute_values_closed(),
        }
    }

    /// The window went (Vazgeç, Esc, a click beside it): the point is dropped.
    pub(crate) fn attribute_values_closed(&mut self) {
        if self.attribute_values.take().is_none() {
            return;
        }
        self.dialog = None;
        self.with_tool(|s, cx| s.values_given(None, cx));
    }

    /// The window opened: the keyboard to its first field, its text chosen.
    pub(crate) fn attribute_values_tasks(&mut self) -> Task<Message> {
        let Some(w) = self.attribute_values.as_mut() else {
            return Task::none();
        };
        if !std::mem::take(&mut w.focus) || w.fields.is_empty() {
            return Task::none();
        }
        Task::batch([
            iced::widget::operation::focus(field_id(0)),
            iced::widget::operation::select_all(field_id(0)),
        ])
    }

    /// Öznitelik değerleri (the web's `openAttributeValuesDialog`).
    pub(crate) fn attribute_values_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.attribute_values else {
            return text("").into();
        };
        let mut body = Column::new()
            .spacing(12)
            .push(words::summary(vec![words::text_line(
                Kind::Info,
                format!(
                    "“{}” bloğunun yerleştirmesi: boş bırakılan öznitelik varsayılanını yazar.",
                    w.name
                ),
            )]));
        for (i, f) in w.fields.iter().enumerate() {
            let control: Element<'_, Message> = match &f.lines {
                Some(content) => focus_ring(
                    text_editor(content)
                        .id(field_id(i))
                        .on_action(move |a| msg(Event::Lines(i, a)))
                        .height(72)
                        .padding([5, 8]),
                )
                .into(),
                None => focus_ring(
                    text_input(&f.default, &f.text)
                        .id(field_id(i))
                        .on_input(move |t| msg(Event::Text(i, t)))
                        .on_submit(msg(Event::Place))
                        .padding([5, 8])
                        .style(style::field::input),
                )
                .into(),
            };
            body = body.push(words::field(&f.label, control, None));
            if let Some(again) = &f.again {
                let input = text_input("Aynı değeri yeniden yazın", again)
                    .on_input(move |t| msg(Event::Again(i, t)))
                    .on_submit(msg(Event::Place))
                    .padding([5, 8])
                    .style(style::field::input);
                body = body.push(words::field(&f.again_label, focus_ring(input), None));
            }
        }
        let unverified = w.unverified();
        if let Some(label) = unverified {
            body = body.push(words::summary(vec![words::text_line(
                Kind::Warn,
                format!("“{label}” iki kez aynı yazılmadı; yerleştirmeden önce doğrulayın."),
            )]));
        }
        overlay::modal(
            Frame::new(VALUES_TITLE)
                .push(body)
                .action(words::secondary(CANCEL, Some(msg(Event::Close))))
                .action(words::primary(
                    PLACE,
                    unverified.is_none().then(|| msg(Event::Place)),
                ))
                .width(420.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step): the
    /// fields by their prompts, Yerleştir and Vazgeç.
    pub(crate) fn attribute_values_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.attribute_values else {
            return Err("Öznitelik değerleri penceresi açık değil".to_owned());
        };
        Ok(match control {
            Control::Fill(label, t) => {
                let i = w
                    .fields
                    .iter()
                    .position(|f| f.label == label)
                    .ok_or_else(|| format!("“{VALUES_TITLE}” penceresinde {control} yok"))?;
                Some(msg(Event::Text(i, t.to_owned())))
            }
            Control::Press(PLACE) => Some(msg(Event::Place)),
            Control::Press(CANCEL) => Some(msg(Event::Close)),
            other => return Err(format!("“{VALUES_TITLE}” penceresinde {other} yok")),
        })
    }
}
