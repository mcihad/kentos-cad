//! Pafta seti (docs/adr/0347 §1; `arch.sheetSet`): the project's sheet book as a drawing set, as `cad.sheets.set` lists
//! it — the sheets by series, each map frame with its scale and the marks pointing at it — and the sheets the selected
//! objects' change affects, marked to print again. Paftaya git opens a sheet in the sheet mode.

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{AffectReason, SheetsSet, SheetsSetResult};
use kentos_ui::icon::icon;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Pafta seti";
const CLOSE: &str = "Kapat";
const GO: &str = "Paftaya git";

#[derive(Clone, Debug)]
pub struct Window {
    pub set: SheetsSetResult,
    /// The selected objects the affected sheets are of; none: no selection.
    pub chosen: usize,
}

#[derive(Clone, Debug)]
pub enum Event {
    Go(usize),
    Close,
}

fn msg(e: Event) -> Message {
    Message::SheetSet(e)
}

fn reason_words(r: AffectReason) -> &'static str {
    match r {
        AffectReason::Shows => "nesneyi gösteriyor",
        AffectReason::Marked => "işaret ona bakıyor",
        AffectReason::Unknown => "zemini bilinmiyor",
    }
}

impl App {
    /// Pafta seti (`arch.sheetSet`): the set, and the sheets the selection affects.
    pub(crate) fn open_sheet_set(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return;
        };
        let uids: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter_map(|s| doc.model.uid(*s))
            .map(|u| u.to_string())
            .collect();
        let chosen = uids.len();
        let input = SheetsSet {
            uids: (!uids.is_empty()).then_some(uids),
            ..Default::default()
        };
        match kentos_native_application::sheets_set::plan_in(&doc.model, &input) {
            kentos_commands::CommandResult::Completed { output, .. } => {
                self.sheet_set = Some(Window {
                    set: output,
                    chosen,
                });
                self.dialog = Some(Dialog::SheetSet);
            }
            kentos_commands::CommandResult::Failed { error } => self.warn(error.message),
            _ => {}
        }
    }

    pub(crate) fn sheet_set_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Go(k) => {
                self.sheet_set = None;
                self.dialog = None;
                return self.update(Message::Sheet(kentos_sheet_ui::Message::Tab(k + 1)));
            }
            Event::Close => {
                self.sheet_set = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn sheet_set_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.sheet_set else {
            return iced::widget::text("").into();
        };
        let affected = w.set.affected.as_deref().unwrap_or_default();
        let touched: Vec<&str> = {
            let mut v: Vec<&str> = affected.iter().map(|a| a.sheet.as_str()).collect();
            v.dedup();
            v
        };
        let sheets = w.set.sheets.len();
        let line = if w.set.affected.is_none() {
            words::text_line(
                Kind::Ok,
                format!(
                    "{sheets} pafta, {} seri. Etkilenen paftaları görmek için çizimde nesne seçip yeniden açın.",
                    w.set.series.len()
                ),
            )
        } else if touched.is_empty() {
            words::text_line(
                Kind::Ok,
                format!(
                    "{sheets} pafta; seçili {} nesne hiçbir paftada görünmüyor.",
                    w.chosen
                ),
            )
        } else {
            words::text_line(
                Kind::Warn,
                format!(
                    "{sheets} pafta; seçili {} nesnenin değişikliği {} paftayı etkiler: yeniden basılmalı.",
                    w.chosen,
                    touched.len()
                ),
            )
        };
        let mut list = Column::new().spacing(12);
        if w.set.sheets.is_empty() {
            list = list.push(label::caption(
                "Projede pafta yok: Mimari › Proje'den Mimari şablonu ile ya da pafta kipinde “+” ile pafta ekleyin.",
            ));
        }
        for series in &w.set.series {
            let head = if series.title.is_empty() {
                series.series.clone()
            } else {
                format!("{} {}", series.series, series.title)
            };
            let mut group = Column::new().spacing(6).push(label::strong(head));
            for id in &series.sheets {
                let Some((k, s)) = w.set.sheets.iter().enumerate().find(|(_, s)| &s.id == id)
                else {
                    continue;
                };
                let hit: Vec<_> = affected.iter().filter(|a| a.sheet == s.id).collect();
                let frames = if s.frames.is_empty() {
                    "Harita çerçevesi yok".to_owned()
                } else {
                    s.frames
                        .iter()
                        .map(|f| {
                            let scale = f
                                .scale
                                .map_or_else(|| "atlas".to_owned(), |n| format!("1:{n}"));
                            let marks = if f.marks.is_empty() {
                                String::new()
                            } else {
                                format!(", {} işaret", f.marks.len())
                            };
                            format!("{} ({scale}{marks})", f.name)
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                };
                let mut text = Column::new()
                    .spacing(2)
                    .push(label::body(s.name.clone()))
                    .push(label::caption(frames));
                if !hit.is_empty() {
                    let why: Vec<String> = hit
                        .iter()
                        .map(|a| {
                            let name = s
                                .frames
                                .iter()
                                .find(|f| f.item == a.item)
                                .map_or("", |f| f.name.as_str());
                            let reasons: Vec<&str> =
                                a.reasons.iter().map(|r| reason_words(*r)).collect();
                            format!("{name}: {}", reasons.join(", "))
                        })
                        .collect();
                    text = text.push(label::caption(format!(
                        "Yeniden basılacak — {}",
                        why.join("; ")
                    )));
                }
                let line = row![
                    icon(crate::icons::from_web(Some(if hit.is_empty() {
                        "sheet"
                    } else {
                        "warning"
                    })))
                    .size(18.0),
                    container(text).width(Fill),
                    button(label::body(GO))
                        .on_press(msg(Event::Go(k)))
                        .padding(kentos_ui::theme::metrics::action())
                        .style(style::button::secondary),
                ]
                .spacing(10)
                .align_y(Center);
                group = group.push(
                    container(line)
                        .padding([6, 10])
                        .style(style::container::field_box)
                        .width(Fill),
                );
            }
            list = list.push(group);
        }
        let body = Column::new()
            .spacing(12)
            .push(words::summary(vec![line]))
            .push(iced::widget::scrollable(list).height(420.0));
        Frame::new(TITLE)
            .push(body)
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(760.0)
            .modal(msg(Event::Close))
    }

    /// The window's buttons by their words (a trace's `dialog` step): the first sheet's.
    pub(crate) fn sheet_set_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.sheet_set.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(GO) => Some(msg(Event::Go(0))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
