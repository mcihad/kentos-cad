//! Etiketleri denetle (docs/adr/0344 §2; `arch.tagsCheck`): what `cad.tags.check` finds — numbers given twice,
//! openings without a type or off their wall, tags whose object is gone, openings and spaces without a tag — each with
//! its words and objects. Göster brings them into view; the drawing is not changed.

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{CommandResult, TagFinding, TagsCheck};
use kentos_ui::icon::icon;
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Etiketleri denetle";
const CLOSE: &str = "Kapat";
const REFRESH: &str = "Yeniden denetle";
const SHOW: &str = "Göster";

#[derive(Clone, Debug, Default)]
pub struct Window {
    pub findings: Vec<TagFinding>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Show(usize),
    Refresh,
    Close,
}

fn msg(e: Event) -> Message {
    Message::TagsCheck(e)
}

/// A finding's icon by its code.
fn icon_of(code: &str) -> &'static str {
    match code {
        "tag_untagged" => "tag",
        "tag_target_gone" => "tagsCheck",
        _ => "warning",
    }
}

impl App {
    fn tags_check_read(&self) -> Option<Window> {
        let doc = self.document.as_ref()?;
        match kentos_native_application::tags_check::plan_in(&doc.model, &TagsCheck::default()) {
            CommandResult::Completed { output, .. } => Some(Window {
                findings: output.findings,
            }),
            _ => None,
        }
    }

    /// Etiketleri denetle (`arch.tagsCheck`): the window.
    pub(crate) fn open_tags_check(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.tags_check = self.tags_check_read();
        self.dialog = Some(Dialog::TagsCheck);
    }

    pub(crate) fn tags_check_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Show(i) => {
                let Some(f) = self
                    .tags_check
                    .as_ref()
                    .and_then(|w| w.findings.get(i))
                    .cloned()
                else {
                    return Task::none();
                };
                let Some(doc) = self.document.as_ref() else {
                    return Task::none();
                };
                let ids: Vec<f64> = f
                    .uids
                    .iter()
                    .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                    .filter_map(|u| doc.model.slot_of(u))
                    .map(|s| f64::from(s.0))
                    .collect();
                self.zoom_to_objects(&ids);
            }
            Event::Refresh => self.tags_check = self.tags_check_read(),
            Event::Close => {
                self.tags_check = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn tags_check_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.tags_check else {
            return iced::widget::text("").into();
        };
        let summary = if w.findings.is_empty() {
            words::summary(vec![words::text_line(
                Kind::Ok,
                "Etiketler ve numaralar tutarlı: bulgu yok.".to_owned(),
            )])
        } else {
            let count = |code: &str| w.findings.iter().filter(|f| f.code == code).count();
            let parts: Vec<String> = [
                ("tag_duplicate_number", "yinelenen numara"),
                ("tag_missing_type", "türsüz açıklık"),
                ("tag_host_broken", "duvarı kopuk açıklık"),
                ("tag_target_gone", "nesnesiz etiket"),
                ("tag_untagged", "etiketsiz"),
            ]
            .iter()
            .filter_map(|(code, what)| {
                let n = count(code);
                (n > 0).then(|| format!("{n} {what}"))
            })
            .collect();
            words::summary(vec![words::text_line(
                Kind::Warn,
                format!("{} bulgu: {}.", w.findings.len(), parts.join(", ")),
            )])
        };
        let mut list = Column::new().spacing(8);
        for (i, f) in w.findings.iter().enumerate() {
            let text = Column::new()
                .spacing(2)
                .push(label::body(f.message.clone()))
                .push(label::caption(format!(
                    "Yer: {:.3}, {:.3}; {} nesne.",
                    f.at.x,
                    f.at.y,
                    f.uids.len()
                )));
            let show = button(label::body(SHOW))
                .on_press(msg(Event::Show(i)))
                .padding([4, 10])
                .style(style::button::secondary);
            list = list.push(
                container(
                    row![
                        icon(crate::icons::from_web(Some(icon_of(&f.code)))).size(18.0),
                        container(text).width(Fill),
                        show,
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
        overlay::modal(
            Frame::new(TITLE)
                .push(body)
                .action(words::secondary(REFRESH, Some(msg(Event::Refresh))))
                .action(words::primary(CLOSE, Some(msg(Event::Close))))
                .width(720.0),
            msg(Event::Close),
        )
    }

    /// The window's buttons by their words (a trace's `dialog` step): the first finding's Göster.
    pub(crate) fn tags_check_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.tags_check.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(REFRESH) => Some(msg(Event::Refresh)),
            Control::Press(SHOW) => Some(msg(Event::Show(0))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
