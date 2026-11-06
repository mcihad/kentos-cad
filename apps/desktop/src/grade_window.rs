//! Eğimle kot ver (docs/adr/1290; `tool.grade`): the selected line's, polyline's or area's vertices from one to another
//! take elevations along a constant grade, by `cad.entities.grade`. The window is the preview: the command's plan says
//! the range's horizontal length, the grade and the end's elevation; Uygula writes one undo step. Vertices are numbered
//! from 1, as Koordinat listesi numbers them; the grade is typed in percent.

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{CommandResult, EntitiesGrade, EntitiesGraded, Entity};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, entities_grade};
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Switch};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

pub const TITLE: &str = "Eğimle kot ver";
const APPLY: &str = "Uygula";
const CANCEL: &str = "Vazgeç";

#[derive(Clone, Debug, Default)]
pub struct State {
    pub uid: String,
    pub vertices: u32,
    /// From and to, numbered from 1.
    pub from: f64,
    pub to: f64,
    pub start: f64,
    /// By the end's elevation (else by the grade).
    pub by_end: bool,
    pub end: f64,
    /// Percent.
    pub percent: f64,
    pub plan: Option<Result<EntitiesGraded, String>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    From(f64),
    To(f64),
    Start(f64),
    ByEnd(bool),
    End(f64),
    Percent(f64),
    Apply,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Grade(e)
}

impl App {
    fn grade_input(&self) -> EntitiesGrade {
        let s = &self.grade;
        EntitiesGrade {
            uid: s.uid.clone(),
            from: (s.from.max(1.0) as u32) - 1,
            to: (s.to.max(1.0) as u32) - 1,
            start: Some(s.start),
            end: s.by_end.then_some(s.end),
            grade: (!s.by_end).then_some(s.percent / 100.0),
            expected_revision: None,
        }
    }

    fn grade_plan(&mut self) {
        let input = self.grade_input();
        let Some(doc) = self.document.as_mut() else { return };
        self.grade.plan = Some(match entities_grade::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            _ => Err("Önizleme alınamadı.".to_owned()),
        });
    }

    /// Eğimle kot ver (`tool.grade`): the one selected path, from its first vertex to its last.
    pub(crate) fn open_grade(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let picked = match self.selection.ids() {
            [one] => doc.model.get(*one).zip(doc.model.uid(*one)),
            _ => None,
        };
        let Some((e, uid)) = picked else {
            self.warn("Eğimle kot ver: önce bir çizgi, çoklu çizgi ya da alan seçin.");
            return Task::none();
        };
        let (n, z0) = match e {
            Entity::Line(l) => (2, l.za),
            Entity::Polyline(p) | Entity::Polygon(p) => (p.pts.len(), p.zs.as_ref().and_then(|z| z.first().copied().flatten())),
            _ => {
                self.warn("Eğimle kot ver: seçili nesne çizgi, çoklu çizgi ya da alan değil.");
                return Task::none();
            }
        };
        self.grade = State {
            uid: uid.to_string(),
            vertices: n as u32,
            from: 1.0,
            to: n as f64,
            start: z0.unwrap_or(0.0),
            by_end: false,
            end: z0.unwrap_or(0.0),
            percent: 2.0,
            plan: None,
        };
        self.grade_plan();
        self.dialog = Some(Dialog::Grade);
        Task::none()
    }

    pub(crate) fn grade_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::From(v) => self.grade.from = v.round(),
            Event::To(v) => self.grade.to = v.round(),
            Event::Start(v) => self.grade.start = v,
            Event::ByEnd(on) => self.grade.by_end = on,
            Event::End(v) => self.grade.end = v,
            Event::Percent(v) => self.grade.percent = v,
            Event::Apply => {
                let input = self.grade_input();
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                match entities_grade::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { output, .. } => {
                        self.say(
                            Level::Success,
                            format!(
                                "Eğimle kot verildi: {} köşe, %{:.3} eğim, {:.3} → {:.3} m (tek adımda geri alınır).",
                                output.zs.len(),
                                output.grade * 100.0,
                                output.start,
                                output.end
                            ),
                        );
                        self.grade = State::default();
                        self.dialog = None;
                    }
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message));
                    }
                    _ => {}
                }
                return Task::none();
            }
            Event::Close => {
                self.grade = State::default();
                self.dialog = None;
                return Task::none();
            }
        }
        self.grade_plan();
        Task::none()
    }

    pub(crate) fn grade_view(&self) -> Element<'_, Message> {
        let s = &self.grade;
        // One label column, every box the same width (the owner: a form's fields line up).
        const BOX: f32 = 140.0;
        let target: Element<'_, Message> = if s.by_end {
            NumberInput::new(s.end, |v| msg(Event::End(v))).step(0.1).decimals(3).point().width(BOX).into()
        } else {
            NumberInput::new(s.percent, |v| msg(Event::Percent(v))).step(0.1).decimals(3).point().width(BOX).into()
        };
        let form = Form::new()
            .label_width(130.0)
            .field("Başlangıç köşesi", NumberInput::new(s.from, |v| msg(Event::From(v))).step(1.0).decimals(0).width(BOX))
            .field("Bitiş köşesi", NumberInput::new(s.to, |v| msg(Event::To(v))).step(1.0).decimals(0).width(BOX))
            .help(format!("Köşeler 1'den {}'e, Koordinat listesindeki gibi.", s.vertices))
            .field("Başlangıç kotu (m)", NumberInput::new(s.start, |v| msg(Event::Start(v))).step(0.1).decimals(3).point().width(BOX))
            .row(Switch::new(s.by_end, |on| msg(Event::ByEnd(on))).label("Eğim yerine bitiş kotunu ver"))
            .field(if s.by_end { "Bitiş kotu (m)" } else { "Eğim (%)" }, target);
        let line = match &s.plan {
            Some(Ok(p)) => words::text_line(
                Kind::Ok,
                format!(
                    "Yatay uzunluk {:.3} m, eğim %{:.3}, kot {:.3} → {:.3} m; {} köşenin kotu değişecek.",
                    p.length,
                    p.grade * 100.0,
                    p.start,
                    p.end,
                    p.zs.len()
                ),
            ),
            Some(Err(why)) => words::text_line(Kind::Error, why.clone()),
            None => words::text_line(Kind::Info, "Önizleme yok."),
        };
        let about = words::text_line(
            Kind::Info,
            "Aradaki köşeler yol boyunca yatay uzaklıklarına göre (yaylı kenar yayı boyunca) kot alır; aralığın dışındakiler kotlarını korur.",
        );
        let body = Column::new().spacing(12).push(form).push(words::summary(vec![about, line]));
        let ready = matches!(s.plan, Some(Ok(_)));
        Frame::new(TITLE)
            .push(body)
            .aside(words::ghost(CANCEL, Some(msg(Event::Close))))
            .action(words::primary(APPLY, ready.then(|| msg(Event::Apply))))
            .width(560.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn grade_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(CANCEL) => Some(msg(Event::Close)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
