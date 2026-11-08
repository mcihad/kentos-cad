//! Eğimle kot ver (docs/adr/1290; `tool.grade`): the selected line's, polyline's or area's vertices from one to another
//! take elevations along a constant grade, by `cad.entities.grade`. The window is the preview: the command's plan says
//! the range's horizontal length, the grade and the end's elevation; Uygula writes one undo step. Vertices are numbered
//! from 1, as Koordinat listesi numbers them; the grade is typed in percent. Sahneden seç takes the range by two clicks
//! on the drawing, each the path's vertex nearest it within the pick aperture (docs/adr/2180).

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{CommandResult, EntitiesGrade, EntitiesGraded, Entity};
use kentos_interaction::Level;
use kentos_interaction::pick::PickPoint;
use kentos_native_application::{ExecutionContext, entities_grade};
use kentos_ui::widget::{Dialog as Frame, Form, NumberInput, Switch};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

pub const TITLE: &str = "Eğimle kot ver";
const APPLY: &str = "Uygula";
const CANCEL: &str = "Vazgeç";
const PICK: &str = "Sahneden seç";

/// Which end of the range a click on the drawing gives (docs/adr/2180).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    Start,
    End,
}

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
    /// The end a click on the drawing is awaited for (Sahneden seç).
    pub picking: Option<Pick>,
}

#[derive(Clone, Debug)]
pub enum Event {
    From(f64),
    To(f64),
    Start(f64),
    ByEnd(bool),
    End(f64),
    Percent(f64),
    /// Sahneden seç: the range by two clicks, its start then its end.
    Pick,
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
            picking: None,
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
            Event::Pick => {
                self.start_grade_pick(Pick::Start);
                return Task::none();
            }
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

    /// The path's vertices as the window numbers them (from 0 here): a line's two ends, a polyline's or an area's outer
    /// ring's points.
    fn grade_vertices(&self) -> Vec<kentos_commands::Vec2> {
        let Some(doc) = self.document.as_ref() else { return Vec::new() };
        let Some(e) = kentos_domain::Uuid::parse_str(&self.grade.uid).ok().and_then(|u| doc.model.slot_of(u)).and_then(|s| doc.model.get(s))
        else {
            return Vec::new();
        };
        match e {
            Entity::Line(l) => vec![l.a, l.b],
            Entity::Polyline(p) | Entity::Polygon(p) => p.pts.clone(),
            _ => Vec::new(),
        }
    }

    /// Sahneden seç (docs/adr/2180): the window steps aside for a click on the drawing.
    fn start_grade_pick(&mut self, end: Pick) {
        self.grade.picking = Some(end);
        self.dialog = None;
        let asks = match end {
            Pick::Start => "Başlangıç köşesine tıklayın",
            Pick::End => "Bitiş köşesine tıklayın",
        };
        self.say(Level::Command, format!("{TITLE} › {asks}"));
        self.field = None;
        self.snap = None;
        self.session.run(Box::new(PickPoint::new(TITLE, asks)));
        self.with_tool(|s, cx| s.activate(cx));
    }

    /// The point Sahneden seç asked for (input.rs); false when it asked none. The path's vertex nearest it within the
    /// pick aperture is the range's start, then its end; the window opens again with the range previewed. Esc goes back
    /// to the window; a click by no vertex is said.
    pub(crate) fn grade_picked(&mut self, p: Option<kentos_interaction::Vec2>) -> bool {
        let Some(end) = self.grade.picking.take() else {
            return false;
        };
        self.dialog = Some(Dialog::Grade);
        let Some(p) = p else {
            return true;
        };
        let reach = self.draft.pick_aperture / self.viewport.camera.scale;
        let nearest = self
            .grade_vertices()
            .iter()
            .enumerate()
            .map(|(i, v)| (i, (v.x - p.x).hypot(v.y - p.y)))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i);
        let Some(i) = nearest else {
            self.warn(format!("{TITLE}: tıklanan yer seçili nesnenin bir köşesine yakın değil; köşenin üstüne tıklayın."));
            return true;
        };
        match end {
            Pick::Start => {
                self.grade.from = (i + 1) as f64;
                self.start_grade_pick(Pick::End);
            }
            Pick::End => {
                self.grade.to = (i + 1) as f64;
                self.grade_plan();
            }
        }
        true
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
        // The chosen vertices where they are, as the drawing writes coordinates (Y, X).
        let pts = self.grade_vertices();
        let f = self.format();
        let at = |n: f64| {
            let i = (n.max(1.0) as usize) - 1;
            pts.get(i).map_or(String::new(), |v| format!(" (Y {}, X {})", f.coord(v.x), f.coord(v.y)))
        };
        let form = Form::new()
            .label_width(130.0)
            .field(
                "Başlangıç köşesi",
                iced::widget::row![
                    NumberInput::new(s.from, |v| msg(Event::From(v))).step(1.0).decimals(0).width(BOX),
                    words::secondary(PICK, Some(msg(Event::Pick)))
                ]
                .spacing(8)
                .align_y(iced::Center),
            )
            .field("Bitiş köşesi", NumberInput::new(s.to, |v| msg(Event::To(v))).step(1.0).decimals(0).width(BOX))
            .help(format!(
                "Köşeler 1'den {}'e, Koordinat listesindeki gibi; Sahneden seç iki tıkla alır: başlangıç{}, bitiş{}.",
                s.vertices,
                at(s.from),
                at(s.to)
            ))
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
            Control::Press(PICK) => Some(msg(Event::Pick)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
