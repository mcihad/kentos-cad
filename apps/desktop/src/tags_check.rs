//! Etiketleri denetle (docs/adr/0344 §2; `arch.tagsCheck`): what `cad.tags.check` finds — numbers given twice,
//! openings without a type or off their wall, tags whose object is gone, openings and spaces without a tag — each with
//! its words and objects. Göster brings them into view; the drawing is not changed.

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{CommandResult, TagFinding, TagsCheck};
use kentos_ui::icon::icon;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Etiketleri denetle";
const CLOSE: &str = "Kapat";
const REFRESH: &str = "Yeniden denetle";
const SHOW: &str = "Göster";
const MISSING: &str = "Numarasız mahalleri de denetle";

#[derive(Clone, Debug, Default)]
pub struct Window {
    pub findings: Vec<TagFinding>,
    pub missing_space_numbers: bool,
}

#[derive(Clone, Debug)]
pub enum Event {
    Show(usize),
    Refresh,
    Missing(bool),
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
    fn tags_check_read(&self, missing_space_numbers: bool) -> Option<Window> {
        let doc = self.document.as_ref()?;
        let input = TagsCheck {
            missing_space_numbers: Some(missing_space_numbers),
            ..Default::default()
        };
        match kentos_native_application::tags_check::plan_in(&doc.model, &input) {
            CommandResult::Completed { output, .. } => Some(Window {
                findings: output.findings,
                missing_space_numbers,
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
        self.tags_check = self.tags_check_read(true);
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
            Event::Refresh => {
                if let Some(w) = &self.tags_check {
                    self.tags_check = self.tags_check_read(w.missing_space_numbers);
                }
            }
            Event::Missing(on) => {
                if self.tags_check.is_some() {
                    self.tags_check = self.tags_check_read(on);
                }
            }
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
                "Seçili denetimlerde bulgu yok.".to_owned(),
            )])
        } else {
            let count = |code: &str| w.findings.iter().filter(|f| f.code == code).count();
            let parts: Vec<String> = [
                ("tag_duplicate_number", "yinelenen numara"),
                ("tag_missing_space_number", "numarasız mahal"),
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
                .padding(kentos_ui::theme::metrics::action())
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
        let body = Column::new()
            .spacing(12)
            .push(words::check(
                w.missing_space_numbers,
                MISSING,
                Some(msg(Event::Missing(!w.missing_space_numbers))),
            ))
            .push(label::caption(
                "Numara denetimi gizli ve kilitli katmanları da içerir. Çizim değiştirilmez.",
            ))
            .push(summary)
            .push(
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
            Control::Press(MISSING) => Some(msg(Event::Missing(
                !self.tags_check.as_ref().unwrap().missing_space_numbers,
            ))),
            Control::Press(SHOW) => Some(msg(Event::Show(0))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::app_with_drawing;

    fn app() -> App {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../fixtures/commands/v1/cad.tags.check.json"
        ))
        .unwrap();
        let setup = &fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| {
                c["name"]
                    .as_str()
                    .unwrap()
                    .starts_with("Numarasız mahal opt-in")
            })
            .unwrap()["setup"];
        let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&setup.to_string()).unwrap();
        let mut app = app_with_drawing();
        let drawing = crate::document::Document::new(snapshot, None).unwrap();
        let _ = app.update(Message::Opened(Some(Ok(Box::new(drawing)))));
        app
    }
    fn missing(app: &App) -> usize {
        app.tags_check
            .as_ref()
            .unwrap()
            .findings
            .iter()
            .filter(|f| f.code == "tag_missing_space_number")
            .count()
    }

    #[test]
    fn visible_check_includes_unnumbered_rooms_and_toggling_refreshing_closing_write_nothing() {
        let mut app = app();
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        app.open_tags_check();
        assert_eq!(missing(&app), 3);
        assert_eq!(app.dialog, Some(Dialog::TagsCheck));
        let _ = app.tags_check_event(Event::Missing(false));
        assert_eq!(missing(&app), 0);
        let _ = app.tags_check_event(Event::Refresh);
        assert!(!app.tags_check.as_ref().unwrap().missing_space_numbers);
        let _ = app.tags_check_event(Event::Missing(true));
        assert_eq!(missing(&app), 3);
        let _ = app.tags_check_event(Event::Close);
        let _ = app.tags_check_event(Event::Missing(true));
        let _ = app.tags_check_event(Event::Refresh);
        assert!(app.tags_check.is_none() && app.dialog.is_none());
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }

    #[test]
    fn refresh_reads_the_current_room_fields_without_renumbering_other_rooms() {
        let mut app = app();
        app.open_tags_check();
        let doc = &mut app.document.as_mut().unwrap().model;
        let mut entity = doc.get(kentos_domain::Slot(1)).unwrap().clone();
        let kentos_commands::Entity::Space(room) = &mut entity else {
            panic!("room")
        };
        room.space.number = Some("N-001".into());
        doc.update(kentos_domain::Slot(1), entity);
        let before = doc.to_snapshot_v2();
        let _ = app.tags_check_event(Event::Refresh);
        assert_eq!(missing(&app), 2);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
    }
}
