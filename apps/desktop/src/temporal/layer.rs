//! Zaman ayarları (docs/adr/0210 §10; the web's `ui/time/TimeLayerDialog.ts`):
//! Katman (the active one, or the one the layer tree's menu named),
//! Başlangıç alanı, Bitiş alanı (Yok: the objects are moments), Kimlik alanı
//! (Yok), Birikimli; the fields offered are the layer's own and its objects'
//! attribute names. What the values give is said as they are chosen: how
//! many objects have a time, how many have none, values that do not read as
//! a date, the range. Kaydet writes through `cad.layers.time` (one step
//! “Zaman ayarları”); Zamanı kaldır takes the setting away.

use iced::widget::{Column, container, row, text};
use iced::{Element, Fill, Task};
use kentos_contracts::{CommandResult, LayerTime, LayersTime};
use kentos_geometry_core::time::{self, Rule, Unit};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, layers_time};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, overlay};

use super::{Event as TimeEvent, msg as time_msg};
use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Zaman ayarları";
const SAVE: &str = "Kaydet";
const REMOVE: &str = "Zamanı kaldır";
const CANCEL: &str = "Vazgeç";
const CUMULATIVE: &str = "Birikimli (başlangıçtan sonra hep görünür)";
const NONE_END: &str = "Yok (anlık)";
const NONE: &str = "Yok";

/// The window: the layer and the setting as chosen (an empty name: none).
#[derive(Debug, Clone)]
pub struct Window {
    layer: String,
    start: String,
    end: String,
    key: String,
    cumulative: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Layer(usize),
    Start(usize),
    End(usize),
    Key(usize),
    Cumulative,
    Save,
    Remove,
    Cancel,
}

fn msg(event: Event) -> Message {
    time_msg(TimeEvent::Layer(event))
}

impl Window {
    /// The setting as chosen, or none without a start.
    fn chosen(&self) -> Option<LayerTime> {
        (!self.start.is_empty()).then(|| LayerTime {
            start: self.start.clone(),
            end: (!self.end.is_empty()).then(|| self.end.clone()),
            key: (!self.key.is_empty()).then(|| self.key.clone()),
            cumulative: self.cumulative,
        })
    }
}

impl App {
    /// The layers Zaman ayarları offers: those that hold objects, by path.
    fn time_layers(&self) -> Vec<String> {
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

    /// A layer's attribute names: its fields first, then its objects' other keys, in the order met.
    fn attribute_names(&self, layer: &str) -> Vec<String> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut out: Vec<String> = doc
            .model
            .layers()
            .get(layer)
            .map(|n| n.fields.iter().map(|f| f.name.clone()).collect())
            .unwrap_or_default();
        for e in doc.model.by_layer(layer) {
            for k in e.base().attrs.keys() {
                if !out.contains(k) {
                    out.push(k.clone());
                }
            }
        }
        out
    }

    /// The window's fields for `layer`: its own setting, else the first name that looks like a date's.
    fn time_layer_load(&self, layer: String) -> Window {
        let own = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer))
            .and_then(|n| n.time.clone());
        let names = self.attribute_names(&layer);
        let looks = |n: &str| {
            let n = n.to_lowercase();
            ["tarih", "baslangic", "başlangıç", "start", "date"]
                .iter()
                .any(|w| n.contains(w))
        };
        match own {
            Some(t) => Window {
                layer,
                start: t.start,
                end: t.end.unwrap_or_default(),
                key: t.key.unwrap_or_default(),
                cumulative: t.cumulative,
            },
            None => Window {
                start: names
                    .iter()
                    .find(|n| looks(n))
                    .or_else(|| names.first())
                    .cloned()
                    .unwrap_or_default(),
                layer,
                end: String::new(),
                key: String::new(),
                cumulative: false,
            },
        }
    }

    /// Zaman ayarları (`time.layer`): the window over `layer`, else the active layer.
    pub(crate) fn open_time_layer(&mut self, layer: Option<String>) {
        let layers = self.time_layers();
        if layers.is_empty() {
            self.warn("Çizimde nesne tutan katman yok.");
            return;
        }
        let active = self
            .document
            .as_ref()
            .map(|d| d.model.layers().active().to_owned())
            .unwrap_or_default();
        let want = layer.unwrap_or(active);
        let layer = if layers.contains(&want) {
            want
        } else {
            layers[0].clone()
        };
        self.time.layer = Some(self.time_layer_load(layer));
        self.dialog = Some(Dialog::TimeLayer);
    }

    pub(crate) fn time_layer_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.time.layer.clone() else {
            return Task::none();
        };
        let names = self.attribute_names(&w.layer);
        let named = |i: usize| names.get(i).cloned().unwrap_or_default();
        let mut next = w.clone();
        match event {
            Event::Layer(i) => {
                if let Some(id) = self.time_layers().get(i).cloned() {
                    next = self.time_layer_load(id);
                }
            }
            Event::Start(i) => next.start = named(i),
            // The first item of Bitiş and Kimlik is Yok.
            Event::End(i) => next.end = if i == 0 { String::new() } else { named(i - 1) },
            Event::Key(i) => next.key = if i == 0 { String::new() } else { named(i - 1) },
            Event::Cumulative => next.cumulative = !next.cumulative,
            Event::Save => {
                if let Some(t) = w.chosen() {
                    self.write_time(&w.layer, Some(t));
                }
                return Task::none();
            }
            Event::Remove => {
                self.write_time(&w.layer, None);
                return Task::none();
            }
            Event::Cancel => {
                self.time.layer = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.time.layer = Some(next);
        Task::none()
    }

    /// Writes a layer's time setting through `cad.layers.time`, says what it did and closes the window.
    fn write_time(&mut self, layer: &str, time: Option<LayerTime>) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let model = &mut doc.model;
        let name = model
            .layers()
            .get(layer)
            .map_or_else(|| layer.to_owned(), |n| n.name.clone());
        let input = LayersTime {
            layer: layer.to_owned(),
            time: time.clone(),
            expected_revision: None,
        };
        match layers_time::execute(&mut ExecutionContext::new(model), input) {
            CommandResult::Completed { output, .. } => {
                let said = match (&time, output.changed) {
                    (_, false) => format!("“{name}” katmanının zaman ayarı zaten böyle."),
                    (Some(t), true) => format!(
                        "“{name}” katmanı zamansal: {}{}.",
                        t.start,
                        match (&t.end, t.cumulative) {
                            (Some(e), _) => format!(" – {e}"),
                            (None, true) => " (birikimli)".to_owned(),
                            (None, false) => " (anlık)".to_owned(),
                        }
                    ),
                    (None, true) => format!("“{name}” katmanının zamanı kaldırıldı."),
                };
                self.say(
                    if output.changed {
                        Level::Success
                    } else {
                        Level::Info
                    },
                    said,
                );
                self.time.layer = None;
                self.dialog = None;
                if let Some(doc) = &self.document {
                    self.spatial.sync(&doc.model);
                }
                self.time_refresh();
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            _ => {}
        }
    }

    /// What the chosen setting gives, line by line, and whether Kaydet may write.
    fn time_layer_summary(&self, w: &Window) -> (bool, Vec<(Kind, String)>) {
        let Some(t) = w.chosen() else {
            let words = if self.attribute_names(&w.layer).is_empty() {
                "Katmanın nesnelerinde öznitelik yok; önce tarihleri bir alana yazın."
            } else {
                "Nesnelerin başlangıç tarihini tutan alanı seçin."
            };
            return (false, vec![(Kind::Info, words.to_owned())]);
        };
        if let Some(problem) = t.problem() {
            return (false, vec![(Kind::Warn, format!("Ayar: {problem}."))]);
        }
        let Some(doc) = &self.document else {
            return (false, Vec::new());
        };
        let list: Vec<_> = doc.model.by_layer(&w.layer).collect();
        let (_, s) = time::layer_times(
            Rule {
                ranged: t.end.is_some(),
                cumulative: t.cumulative,
            },
            list.iter().map(|e| {
                let a = &e.base().attrs;
                (
                    a.get(&t.start).map(String::as_str),
                    t.end.as_ref().and_then(|k| a.get(k)).map(String::as_str),
                )
            }),
        );
        let range = s.extent.map_or_else(String::new, |(a, b)| {
            format!(
                "; kapsam {} – {}",
                time::show(a, Unit::Day),
                time::show(b, Unit::Day)
            )
        });
        let mut lines = vec![(
            if s.timed > 0 { Kind::Ok } else { Kind::Warn },
            format!(
                "{} nesne: {} zamanlı, {} zamansız{range}.",
                list.len(),
                s.timed,
                s.timeless
            ),
        )];
        if s.unreadable > 0 {
            lines.push((
                Kind::Warn,
                format!(
                    "{} değer tarih olarak okunamadı (05.03.2024, 2024-03-05 ya da 2024-03-05T14:30 gibi yazılmalı); o nesnelerin o ucu boş sayılır.",
                    s.unreadable
                ),
            ));
        }
        if t.end.is_some() && t.cumulative {
            lines.push((
                Kind::Info,
                "Birikimli: nesne başlangıcından sonra hep görünür, bitişi göz ardı edilir."
                    .to_owned(),
            ));
        }
        (true, lines)
    }

    pub(crate) fn time_layer_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.time.layer, &self.document) else {
            return text("").into();
        };
        let layers = self.time_layers();
        let names = self.attribute_names(&w.layer);
        let picked = |v: &str| names.iter().position(|n| n == v);
        let layer_choices: Vec<Choice> = layers
            .iter()
            .map(|id| Choice::new(self.layer_path(id)))
            .collect();
        let layer = Select::new(
            layer_choices,
            layers.iter().position(|id| *id == w.layer),
            |i| msg(Event::Layer(i)),
        );
        let start_choices: Vec<Choice> = names.iter().map(|n| Choice::new(n.clone())).collect();
        let start = Select::new(start_choices, picked(&w.start), |i| msg(Event::Start(i)))
            .placeholder("Seçin");
        let with_none = |first: &str| -> Vec<Choice> {
            std::iter::once(Choice::new(first.to_owned()))
                .chain(names.iter().map(|n| Choice::new(n.clone())))
                .collect()
        };
        let end = Select::new(
            with_none(NONE_END),
            if w.end.is_empty() {
                Some(0)
            } else {
                picked(&w.end).map(|i| i + 1)
            },
            |i| msg(Event::End(i)),
        );
        let key = Select::new(
            with_none(NONE),
            if w.key.is_empty() {
                Some(0)
            } else {
                picked(&w.key).map(|i| i + 1)
            },
            |i| msg(Event::Key(i)),
        );
        let (ready, said) = self.time_layer_summary(w);
        let lines = said
            .into_iter()
            .map(|(kind, line)| words::text_line(kind, line))
            .collect();
        let has_time = doc
            .model
            .layers()
            .get(&w.layer)
            .is_some_and(|n| n.time.is_some());
        let body = Column::new()
            .spacing(12)
            .push(container(words::field("Katman", layer, None)).width(Fill))
            .push(
                row![
                    container(words::field("Başlangıç alanı", start, None)).width(Fill),
                    container(words::field("Bitiş alanı", end, None)).width(Fill),
                    container(words::field("Kimlik alanı", key, None)).width(Fill),
                ]
                .spacing(10),
            )
            .push(words::field(
                "Gösterim",
                words::check(w.cumulative, CUMULATIVE, Some(msg(Event::Cumulative))),
                None,
            ))
            .push(words::summary(lines));
        overlay::modal(
            Frame::new(TITLE)
                .push(body)
                .action(words::secondary(
                    REMOVE,
                    has_time.then(|| msg(Event::Remove)),
                ))
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(SAVE, ready.then(|| msg(Event::Save))))
                .width(640.0),
            msg(Event::Cancel),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step): a
    /// field's attribute by its name (`fill`), Birikimli, Kaydet, Zamanı kaldır, Vazgeç.
    pub(crate) fn time_layer_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.time.layer else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        let names = self.attribute_names(&w.layer);
        let index = |words: &str| {
            names
                .iter()
                .position(|n| n == words)
                .ok_or_else(|| format!("“{TITLE}” penceresinde “{words}” alanı yok"))
        };
        Ok(match control {
            Control::Fill("Katman", words) => {
                let layers = self.time_layers();
                let i = layers
                    .iter()
                    .position(|id| self.layer_path(id) == words)
                    .ok_or_else(|| format!("“{TITLE}” penceresinde “{words}” katmanı yok"))?;
                Some(msg(Event::Layer(i)))
            }
            Control::Fill("Başlangıç alanı", words) => Some(msg(Event::Start(index(words)?))),
            Control::Fill("Bitiş alanı", words) => Some(msg(Event::End(if words == NONE_END {
                0
            } else {
                index(words)? + 1
            }))),
            Control::Fill("Kimlik alanı", words) => Some(msg(Event::Key(if words == NONE {
                0
            } else {
                index(words)? + 1
            }))),
            Control::Check(CUMULATIVE, on) => (w.cumulative != on).then(|| msg(Event::Cumulative)),
            Control::Press(SAVE) => w.chosen().is_some().then(|| msg(Event::Save)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
