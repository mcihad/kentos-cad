//! Birleşimleri denetle (docs/adr/0340 §6; `arch.wallJoinCheck`): the drawing's walls that meet and do not join, from
//! `cad.walls.joins`, each with what is wrong and what resolves it. Göster brings its walls into view; Öneriyi uygula
//! runs the fix the command gives (an end's join, through `cad.walls.edit`) in one undo step, and the list is read
//! again.

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{CommandResult, WallJoinIssue, WallsJoins};
use kentos_interaction::Level;
use kentos_ui::icon::icon;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Birleşimleri denetle";
const CLOSE: &str = "Kapat";
const REFRESH: &str = "Yeniden denetle";
const SHOW: &str = "Göster";
const APPLY: &str = "Öneriyi uygula";

#[derive(Clone, Debug, Default)]
pub struct Window {
    pub issues: Vec<WallJoinIssue>,
    /// How many joints the walls have.
    pub joints: usize,
}

#[derive(Clone, Debug)]
pub enum Event {
    Show(usize),
    Apply(usize),
    Refresh,
    Close,
}

fn msg(e: Event) -> Message {
    Message::WallJoins(e)
}

impl App {
    /// The drawing's joins read again: the window's list, or none without a drawing.
    fn wall_joins_read(&self) -> Option<Window> {
        let doc = self.document.as_ref()?;
        let mut copy = doc.model.reading_copy();
        let result = kentos_native_application::walls_joins::plan(
            &kentos_native_application::ExecutionContext::new(&mut copy),
            &WallsJoins::default(),
        );
        match result {
            CommandResult::Completed { output, .. } => Some(Window {
                issues: output.issues,
                joints: output.joints.len(),
            }),
            _ => None,
        }
    }

    /// Birleşimleri denetle (`arch.wallJoinCheck`): the window.
    pub(crate) fn open_wall_joins(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.wall_joins = self.wall_joins_read();
        self.dialog = Some(Dialog::WallJoins);
    }

    pub(crate) fn wall_joins_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Show(i) => {
                let Some(issue) = self
                    .wall_joins
                    .as_ref()
                    .and_then(|w| w.issues.get(i))
                    .cloned()
                else {
                    return Task::none();
                };
                let Some(doc) = self.document.as_ref() else {
                    return Task::none();
                };
                let ids: Vec<f64> = issue
                    .uids
                    .iter()
                    .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                    .filter_map(|u| doc.model.slot_of(u))
                    .map(|s| f64::from(s.0))
                    .collect();
                self.zoom_to_objects(&ids);
            }
            Event::Apply(i) => {
                let Some(fix) = self
                    .wall_joins
                    .as_ref()
                    .and_then(|w| w.issues.get(i))
                    .and_then(|x| x.fix.clone())
                else {
                    return Task::none();
                };
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let result = kentos_native_application::walls_edit::execute(
                    &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                    fix,
                );
                match result {
                    CommandResult::Completed { .. } => {
                        self.announce(
                            Level::Success,
                            "Öneri uygulandı: duvarın ucunun birleşimi değişti.",
                        );
                    }
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error } => self.warn(error.message),
                    _ => {}
                }
                self.wall_joins = self.wall_joins_read();
            }
            Event::Refresh => self.wall_joins = self.wall_joins_read(),
            Event::Close => {
                self.wall_joins = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn wall_joins_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.wall_joins else {
            return iced::widget::text("").into();
        };
        let summary = if w.issues.is_empty() {
            words::summary(vec![words::text_line(
                Kind::Ok,
                format!("Bütün duvarlar birleşiyor: {} birleşim.", w.joints),
            )])
        } else {
            words::summary(vec![words::text_line(
                Kind::Warn,
                format!(
                    "{} birleşim; {} yerde duvarlar buluşuyor ama birleşmiyor.",
                    w.joints,
                    w.issues.len()
                ),
            )])
        };
        let mut list = Column::new().spacing(8);
        for (i, issue) in w.issues.iter().enumerate() {
            let text = Column::new()
                .spacing(2)
                .push(label::body(issue.message.clone()))
                .push(label::caption(format!(
                    "{} Yer: {:.3}, {:.3}; {} duvar.",
                    issue.repair,
                    issue.at.x,
                    issue.at.y,
                    issue.uids.len()
                )));
            let show = button(label::body(SHOW))
                .on_press(msg(Event::Show(i)))
                .padding(kentos_ui::theme::metrics::action())
                .style(style::button::secondary);
            let apply = button(label::body(APPLY))
                .on_press_maybe(issue.fix.is_some().then(|| msg(Event::Apply(i))))
                .padding(kentos_ui::theme::metrics::action())
                .style(style::button::secondary);
            list = list.push(
                container(
                    row![
                        icon(crate::icons::from_web(Some("wallJoinCheck"))).size(18.0),
                        container(text).width(Fill),
                        show,
                        apply,
                    ]
                    .spacing(10)
                    .align_y(Center),
                )
                .padding([8, 10])
                .style(style::container::field_box)
                .width(Fill),
            );
        }
        let body = Column::new().spacing(12).push(summary).push(
            iced::widget::scrollable(list)
                .height(iced::Length::Shrink)
                .height(360.0),
        );
        Frame::new(TITLE)
            .push(body)
            .action(words::secondary(REFRESH, Some(msg(Event::Refresh))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(720.0)
            .modal(msg(Event::Close))
    }

    /// The window's buttons by their words (a trace's `dialog` step): the first issue's Göster and Öneriyi uygula.
    pub(crate) fn wall_joins_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.wall_joins.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(REFRESH) => Some(msg(Event::Refresh)),
            Control::Press(SHOW) => Some(msg(Event::Show(0))),
            Control::Press(APPLY) => Some(msg(Event::Apply(0))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
