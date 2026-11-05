//! 2B düzleme indir (docs/adr/1061 §4; `cad.flatten`): the selection's elevations removed or set to one level, and
//! with Ölçüleri de kaldır their measures, through `cad.entities.flatten`. The window is the preview: the command's
//! plan says how many objects and vertices change and between which elevations they are; Uygula writes one undo step.

use iced::widget::{Column, row};
use iced::{Center, Element, Task};
use kentos_commands::{CommandResult, EntitiesFlatten, EntitiesFlattened};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, entities_flatten};
use kentos_ui::label;
use kentos_ui::widget::{Dialog as Frame, NumberInput, Switch};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

pub const TITLE: &str = "2B düzleme indir";
const APPLY: &str = "Uygula";
const CANCEL: &str = "Vazgeç";

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The selection's persistent ids when the window opened.
    pub uids: Vec<String>,
    /// Kota getir: the level every elevation becomes; none: Kotları kaldır.
    pub plane: Option<f64>,
    pub measures: bool,
    /// The command's plan for the choices, or why it is refused.
    pub plan: Option<Result<EntitiesFlattened, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    /// Kota getir on or off.
    Level(bool),
    Plane(f64),
    Measures(bool),
    Apply,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Flatten(e)
}

impl App {
    fn flatten_input(&self) -> EntitiesFlatten {
        EntitiesFlatten {
            uids: self.flatten.uids.clone(),
            plane: self.flatten.plane,
            measures: self.flatten.measures,
            expected_revision: None,
        }
    }

    /// The plan for the window's choices (nothing written).
    fn flatten_plan(&mut self) {
        let input = self.flatten_input();
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        self.flatten.plan = Some(
            match entities_flatten::plan(&ExecutionContext::new(&mut doc.model), &input) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Önizleme alınamadı.".to_owned()),
            },
        );
    }

    /// 2B düzleme indir (`cad.flatten`): the selection, previewed.
    pub(crate) fn open_flatten(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let uids: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| doc.model.uid(*s))
            .map(|u| u.to_string())
            .collect();
        if uids.is_empty() {
            self.warn("2B düzleme indir: önce kotları kaldırılacak nesneleri seçin.");
            return Task::none();
        }
        self.flatten = State {
            uids,
            ..State::default()
        };
        self.flatten_plan();
        self.dialog = Some(Dialog::Flatten);
        Task::none()
    }

    pub(crate) fn flatten_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Level(on) => {
                self.flatten.plane = on.then_some(self.flatten.plane.unwrap_or(0.0));
                self.flatten_plan();
            }
            Event::Plane(v) => {
                if self.flatten.plane.is_some() {
                    self.flatten.plane = Some(v);
                    self.flatten_plan();
                }
            }
            Event::Measures(on) => {
                self.flatten.measures = on;
                self.flatten_plan();
            }
            Event::Apply => {
                let input = self.flatten_input();
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                match entities_flatten::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { output, warnings } => {
                        self.say(
                            Level::Success,
                            format!(
                                "2B düzleme indirildi: {} nesne, {} köşe kotu, {} ölçü (tek adımda geri alınır).",
                                output.changed.len(),
                                output.vertices,
                                output.measures
                            ),
                        );
                        for w in warnings {
                            self.warn(format!("{TITLE}: {}", w.message));
                        }
                        self.flatten = State::default();
                        self.dialog = None;
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message))
                    }
                    _ => {}
                }
            }
            Event::Close => {
                self.flatten = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn flatten_view(&self) -> Element<'_, Message> {
        let s = &self.flatten;
        let format = self.format();
        let fmt = |z: f64| format.length(z);
        let line = match &s.plan {
            Some(Ok(p)) => {
                let mut words_ = format!(
                    "{} nesne değişecek: {} köşenin kotu",
                    p.changed.len(),
                    p.vertices
                );
                words_.push_str(if s.plane.is_some() {
                    " düzlemin kotuna gelecek"
                } else {
                    " kaldırılacak"
                });
                if let (Some(lo), Some(hi)) = (p.z_min, p.z_max) {
                    words_.push_str(&format!(" (şimdi {} ile {} arasında)", fmt(lo), fmt(hi)));
                }
                if p.measures > 0 {
                    words_.push_str(&format!("; {} ölçü (M) kaldırılacak", p.measures));
                }
                words_.push('.');
                words::text_line(Kind::Warn, words_)
            }
            Some(Err(why)) => words::text_line(Kind::Info, why.clone()),
            None => words::text_line(Kind::Info, "Önizleme yok."),
        };
        let about = words::text_line(
            Kind::Info,
            "Açılan çizimin kotları ve ölçüleri kendiliğinden silinmez; bu işlem açıkça, önizlemeyle ve tek geri alma adımıyla yapılır. Kotu olmayan köşe kotsuz kalır.",
        );
        let level = row![
            Switch::new(s.plane.is_some(), |on| msg(Event::Level(on)))
                .label("Kotları bir düzleme getir"),
            NumberInput::new(s.plane.unwrap_or(0.0), |v| msg(Event::Plane(v)))
                .step(0.5)
                .decimals(3)
                .point()
                .width(120),
        ]
        .spacing(16)
        .align_y(Center);
        let body = Column::new()
            .spacing(12)
            .push(words::summary(vec![about, line]))
            .push(label::caption(format!("{} nesne seçili", s.uids.len())))
            .push(level)
            .push(
                Switch::new(s.measures, |on| msg(Event::Measures(on)))
                    .label("Ölçüleri (M) de kaldır"),
            );
        let ready = matches!(s.plan, Some(Ok(_)));
        Frame::new(TITLE)
            .push(body)
            .aside(words::ghost(CANCEL, Some(msg(Event::Close))))
            .action(words::primary(APPLY, ready.then(|| msg(Event::Apply))))
            .width(560.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn flatten_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(CANCEL) => Some(msg(Event::Close)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
