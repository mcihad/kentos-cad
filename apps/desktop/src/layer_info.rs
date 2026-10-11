//! Bilgi kartı ve eylemler (docs/adr/0225 §4–§6; the web's
//! `ui/layers/LayerInfoDialog.ts`): a layer's card (Başlık, Alanlar, Metin,
//! Ekleri göster, Eylemleri göster) and its actions (a row each: Ad and
//! Hedef, moved up and down, taken away), with the card and the actions of an
//! object of the layer (the one selected there, else its first) shown as they
//! change; a broken template is said. Kaydet writes through
//! `cad.layers.info` (one step “Bilgi kartı”).

use iced::widget::{Column, button, column, container, row, text_editor, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{
    CommandResult, InfoCard, LayerAction, LayerNodeType, LayersInfo, actions_problem,
};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, layers_info};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Bilgi kartı ve eylemler";
const TITLE_FIELD: &str = "Başlık";
const FIELDS: &str = "Alanlar";
const TEXT: &str = "Metin";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const ADD: &str = "Eylem ekle";
const NO_LAYER: &str = "Çizimde nesne tutan katman yok.";

/// The window: the layer and the card and actions as typed.
#[derive(Debug)]
pub struct Window {
    layer: String,
    title: String,
    fields: String,
    text: text_editor::Content,
    attachments: bool,
    actions_shown: bool,
    actions: Vec<LayerAction>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Layer(usize),
    Title(String),
    Fields(String),
    Text(text_editor::Action),
    Attachments,
    Actions,
    Add,
    Name(usize, String),
    Target(usize, String),
    Up(usize),
    Down(usize),
    Remove(usize),
    Save,
    Cancel,
}

fn msg(e: Event) -> Message {
    Message::LayerInfo(e)
}

impl Window {
    /// The card as typed: what is empty is not given; a card with nothing in it is none (the command says so too).
    fn card(&self) -> InfoCard {
        let fields: Vec<String> = self
            .fields
            .split(',')
            .map(|f| f.trim().to_owned())
            .filter(|f| !f.is_empty())
            .collect();
        let text = self.text.text().replace("\r\n", "\n");
        let text = text.strip_suffix('\n').map_or(text.clone(), str::to_owned);
        InfoCard {
            title: Some(self.title.trim().to_owned()).filter(|t| !t.is_empty()),
            fields,
            text: (!text.trim().is_empty()).then_some(text),
            attachments: self.attachments,
            actions: self.actions_shown,
        }
    }

    /// The actions as typed, a row with neither name nor target left out.
    fn trimmed(&self) -> Vec<LayerAction> {
        self.actions
            .iter()
            .map(|a| LayerAction {
                name: a.name.trim().to_owned(),
                target: a.target.trim().to_owned(),
            })
            .filter(|a| !a.name.is_empty() || !a.target.is_empty())
            .collect()
    }

    /// Why Kaydet may not write, each said as the command would.
    fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let actions = self.trimmed();
        if let Some(p) = actions_problem(&actions) {
            out.push(format!("Eylemler: {p}."));
        }
        let card = self.card();
        if let Some(p) = card.problem() {
            out.push(format!("Kart: {p}."));
        }
        for a in &actions {
            if let Err(why) = crate::object_info::compiled(&a.target) {
                out.push(format!("“{}” eyleminin hedefi: {why}", a.name));
            }
        }
        for (source, what) in [(&card.title, "Başlık"), (&card.text, "Metin")] {
            if let Some(source) = source
                && let Err(why) = crate::object_info::compiled(source)
            {
                out.push(format!("{what}: {why}"));
            }
        }
        out
    }
}

impl App {
    /// The layers a card may go on: those that hold objects, by id.
    fn info_layers(&self) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        doc.model
            .layers()
            .leaves()
            .into_iter()
            .filter(|l| l.service.is_none())
            .map(|l| l.id.clone())
            .collect()
    }

    /// The window loaded from `layer`'s card and actions.
    fn info_window(&self, layer: String) -> Window {
        let node = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer));
        let card = node.and_then(|n| n.card.clone()).unwrap_or_default();
        Window {
            title: card.title.clone().unwrap_or_default(),
            fields: card.fields.join(", "),
            text: text_editor::Content::with_text(card.text.as_deref().unwrap_or("")),
            attachments: card.attachments,
            actions_shown: card.actions,
            actions: node.map(|n| n.actions.clone()).unwrap_or_default(),
            layer,
        }
    }

    /// Bilgi kartı ve eylemler (`layer.info`): the window over `layer`, else the active layer, else the first one.
    pub(crate) fn open_layer_info(&mut self, layer: Option<String>) -> Task<Message> {
        let layers = self.info_layers();
        if layers.is_empty() {
            self.warn(NO_LAYER);
            return Task::none();
        }
        let active = self
            .document
            .as_ref()
            .map(|d| d.model.layers().active().to_owned())
            .unwrap_or_default();
        let wanted = layer.unwrap_or(active);
        let id = layers
            .iter()
            .find(|l| **l == wanted)
            .cloned()
            .unwrap_or_else(|| layers[0].clone());
        self.layer_info = Some(self.info_window(id));
        self.dialog = Some(Dialog::LayerInfo);
        iced::widget::operation::focus(title_id())
    }

    pub(crate) fn layer_info_event(&mut self, e: Event) -> Task<Message> {
        let Some(mut w) = self.layer_info.take() else {
            return Task::none();
        };
        match e {
            Event::Layer(i) => {
                if let Some(id) = self.info_layers().get(i).cloned() {
                    w = self.info_window(id);
                }
            }
            Event::Title(t) => w.title = t,
            Event::Fields(t) => w.fields = t,
            Event::Text(action) => w.text.perform(action),
            Event::Attachments => w.attachments = !w.attachments,
            Event::Actions => w.actions_shown = !w.actions_shown,
            Event::Add => w.actions.push(LayerAction::default()),
            Event::Name(i, t) => {
                if let Some(a) = w.actions.get_mut(i) {
                    a.name = t;
                }
            }
            Event::Target(i, t) => {
                if let Some(a) = w.actions.get_mut(i) {
                    a.target = t;
                }
            }
            Event::Up(i) if i > 0 && i < w.actions.len() => w.actions.swap(i, i - 1),
            Event::Down(i) if i + 1 < w.actions.len() => w.actions.swap(i, i + 1),
            Event::Up(_) | Event::Down(_) => {}
            Event::Remove(i) => {
                if i < w.actions.len() {
                    w.actions.remove(i);
                }
            }
            Event::Save => {
                if w.problems().is_empty() && self.write_layer_info(&w) {
                    self.dialog = None;
                    return Task::none();
                }
            }
            Event::Cancel => {
                self.dialog = None;
                return Task::none();
            }
        }
        self.layer_info = Some(w);
        Task::none()
    }

    /// Writes the card and the actions through `cad.layers.info` and says what it did; whether it wrote.
    fn write_layer_info(&mut self, w: &Window) -> bool {
        let actions = w.trimmed();
        let card = w.card();
        let input = LayersInfo {
            layer: w.layer.clone(),
            actions,
            card: (card != InfoCard::default()).then_some(card),
            expected_revision: None,
        };
        let name = self.layer_name(&w.layer);
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        match layers_info::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                if output.changed {
                    self.say(
                        Level::Success,
                        format!("“{name}” katmanının bilgi kartı ve eylemleri yazıldı."),
                    );
                } else {
                    self.say(
                        Level::Info,
                        format!("“{name}” katmanının bilgi kartı ve eylemleri zaten böyle."),
                    );
                }
                true
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                false
            }
            _ => false,
        }
    }

    /// The object the preview shows: the one selected on the layer, else its first.
    fn info_sample(&self, layer: &str) -> Option<Slot> {
        let doc = self.document.as_ref()?;
        self.selection
            .ids()
            .iter()
            .copied()
            .find(|s| {
                doc.model
                    .get(*s)
                    .is_some_and(|e| e.base().layer_id == layer)
            })
            .or_else(|| {
                doc.model
                    .by_layer_with_uids(layer)
                    .next()
                    .and_then(|(uid, _)| doc.model.slot_of(uid))
            })
    }

    pub(crate) fn layer_info_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.layer_info else {
            return label::caption("").into();
        };
        let layers = self.info_layers();
        let layer = Select::new(
            layers
                .iter()
                .map(|id| Choice::new(self.layer_path(id)))
                .collect::<Vec<_>>(),
            layers.iter().position(|id| *id == w.layer),
            |i| msg(Event::Layer(i)),
        );
        let input = |placeholder: &'static str, value: &str, on: fn(String) -> Event| {
            kentos_ui::widget::focus_ring(
                text_input(placeholder, value)
                    .on_input(move |t| msg(on(t)))
                    .padding([5, 8])
                    .size(typography::body())
                    .style(style::field::input),
            )
        };
        let title = kentos_ui::widget::focus_ring(
            text_input("Parsel {Ada}/{Parsel}", &w.title)
                .id(title_id())
                .on_input(|t| msg(Event::Title(t)))
                .padding([5, 8])
                .size(typography::body())
                .style(style::field::input),
        );
        let text = text_editor(&w.text)
            .placeholder("Alan: {yuvarla($alan, 2)} m²")
            .on_action(|a| msg(Event::Text(a)))
            .height(72)
            .padding(6)
            .size(typography::body());
        let mut list = Column::new().spacing(4);
        for (i, a) in w.actions.iter().enumerate() {
            let small = |glyph: &'static str, tip: &'static str, on: Option<Message>| {
                kentos_ui::widget::tip(
                    button(icon(from_web(Some(glyph))).size(14.0))
                        .on_press_maybe(on)
                        .padding(4)
                        .style(style::button::ghost),
                    kentos_ui::widget::Tip::new(tip),
                    iced::widget::tooltip::Position::Top,
                )
            };
            list = list.push(
                row![
                    container(kentos_ui::widget::focus_ring(
                        text_input("Ad", &a.name)
                            .on_input(move |t| msg(Event::Name(i, t)))
                            .padding([5, 8])
                            .size(typography::body())
                            .style(style::field::input),
                    ))
                    .width(Length::Fixed(140.0)),
                    container(kentos_ui::widget::focus_ring(
                        text_input("https://… {Ada} ya da dosya yolu", &a.target)
                            .on_input(move |t| msg(Event::Target(i, t)))
                            .padding([5, 8])
                            .size(typography::body())
                            .style(style::field::input),
                    ))
                    .width(Fill),
                    small("chevronUp", "Yukarı", (i > 0).then(|| msg(Event::Up(i)))),
                    small(
                        "chevronDown",
                        "Aşağı",
                        (i + 1 < w.actions.len()).then(|| msg(Event::Down(i)))
                    ),
                    small("close", "Kaldır", Some(msg(Event::Remove(i)))),
                ]
                .spacing(4)
                .align_y(Center),
            );
        }
        if w.actions.is_empty() {
            list = list.push(label::muted("Eylem yok."));
        }
        let add = button(
            row![icon(from_web(Some("plus"))).size(14.0), label::body(ADD)]
                .spacing(6)
                .align_y(Center),
        )
        .on_press(msg(Event::Add))
        .padding([3, 10])
        .style(style::button::secondary);
        let form = column![
            words::field(
                TITLE_FIELD,
                title,
                Some("Şablon: {…} arası bir ifadedir. Boşsa nesnenin türü ve etiketi.".to_owned())
            ),
            words::field(
                FIELDS,
                input("Malik, Nitelik", &w.fields, Event::Fields),
                Some("Virgülle ayırın; boşsa bütün öznitelikler.".to_owned())
            ),
            words::field(
                TEXT,
                text,
                Some(
                    "{{ ve }} düz ayraç; {$enlem}, {$boylam} nesnenin WGS 84’teki yeri.".to_owned()
                )
            ),
            row![
                words::check(
                    w.attachments,
                    "Ekleri göster",
                    Some(msg(Event::Attachments))
                ),
                words::check(
                    w.actions_shown,
                    "Eylemleri göster",
                    Some(msg(Event::Actions))
                ),
            ]
            .spacing(18),
            row![container(label::strong("Eylemler")).width(Fill), add].align_y(Center),
            list,
        ]
        .spacing(10)
        .width(Fill);
        let problems = w.problems();
        let preview = self.layer_info_preview(w);
        let lines = if problems.is_empty() {
            vec![words::text_line(
                Kind::Ok,
                "Önizleme katmanın bir nesnesinde.",
            )]
        } else {
            problems
                .into_iter()
                .map(|p| words::text_line(Kind::Warn, p))
                .collect()
        };
        let ready = w.problems().is_empty();
        let body = Column::new()
            .spacing(12)
            .push(container(words::field("Katman", layer, None)).width(Fill))
            .push(
                row![
                    form,
                    container(column![label::caption("Önizleme"), preview].spacing(4))
                        .width(Length::Fixed(260.0)),
                ]
                .spacing(12),
            )
            .push(words::summary(lines));
        overlay::modal(
            Frame::new(TITLE)
                .scroll(body)
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(SAVE, ready.then(|| msg(Event::Save))))
                .width(780.0)
                .max_height(820.0),
            msg(Event::Cancel),
        )
    }

    /// The sample object's card and actions with the window's draft.
    fn layer_info_preview(&self, w: &Window) -> Element<'_, Message> {
        let Some(e) = self
            .info_sample(&w.layer)
            .and_then(|s| self.document.as_ref().and_then(|d| d.model.get(s)))
        else {
            return label::muted("Katmanda nesne yok: önizleme yok.").into();
        };
        let card = w.card();
        let shown = self.object_card(e, Some(&card));
        let actions = if shown.actions {
            self.object_actions(e, Some(&w.trimmed()))
        } else {
            Vec::new()
        };
        crate::info_card::card_body(self, &shown, None, &actions, e.base().attachments.len())
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn layer_info_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.layer_info else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Fill(TITLE_FIELD, words) => Some(msg(Event::Title(words.to_owned()))),
            Control::Fill(FIELDS, words) => Some(msg(Event::Fields(words.to_owned()))),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(SAVE) => w.problems().is_empty().then(|| msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The layer kind check lives with the command: a group is refused there.
#[allow(dead_code)]
fn is_layer(kind: LayerNodeType) -> bool {
    kind == LayerNodeType::Layer
}

fn title_id() -> iced::widget::Id {
    iced::widget::Id::new("bilgi-karti-baslik")
}
