//! Cihaza gönder (docs/adr/0169 §4, §6; the web's `ui/io/FieldSendDialog.ts`):
//! points written as an instrument's own coordinate file by the shared writer
//! (`kentos_formats::field::write`): Leica GSI-16 and GSI-8, Topcon GTS-7
//! points, Trimble JobXML, Nikon RAW or CSV. The window shows the file's
//! first lines and every point the format cannot carry, with why; nothing
//! is cut to fit. The points come from the drawing's selection, Aplikasyon's
//! table or Nokta editörü's rows; the drawing is not changed.

use std::sync::atomic::{AtomicUsize, Ordering};

use iced::widget::{Column, column, container, row, scrollable, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{Entity, FieldPoint, FieldWrite, FieldWriteFormat, FieldWriteOptions};
use kentos_interaction::Level;
use kentos_ui::icon::{Tone, icon};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, overlay};
use kentos_ui::{label, style};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message};
use crate::app::{App, Message};
use crate::icons::from_web;

/// The formats in the list's order: the format, its name, the file's
/// extension and the save dialog's filter.
const FORMATS: [(FieldWriteFormat, &str, &str, &str); 6] = [
    (FieldWriteFormat::Gsi16, "Leica GSI-16", "gsi", "Leica GSI"),
    (FieldWriteFormat::Gsi8, "Leica GSI-8", "gsi", "Leica GSI"),
    (
        FieldWriteFormat::Gts7,
        "Topcon GTS-7 (noktalar)",
        "xyz",
        "Topcon GTS-7 noktaları",
    ),
    (
        FieldWriteFormat::Jobxml,
        "Trimble JobXML",
        "jxl",
        "Trimble JobXML",
    ),
    (FieldWriteFormat::Nikon, "Nikon RAW", "raw", "Nikon RAW"),
    (
        FieldWriteFormat::Csv,
        "CSV (Ad, Y, X, Z, Kod)",
        "csv",
        "CSV",
    ),
];

/// The file's first lines the window shows.
const PREVIEW_LINES: usize = 14;
/// Points said in the summary; the rest are counted.
const SHOWN: usize = 5;

/// The last format chosen, kept for the session.
static LAST: AtomicUsize = AtomicUsize::new(0);

/// A drawing's point as the writer takes it: its name (its label, else its
/// Ad), Kod and elevation.
pub fn field_point(e: &Entity) -> Option<FieldPoint> {
    let Entity::Point(p) = e else {
        return None;
    };
    Some(FieldPoint {
        name: p
            .base
            .label
            .clone()
            .or_else(|| p.base.attrs.get("Ad").cloned())
            .unwrap_or_default(),
        east: p.p.x,
        north: p.p.y,
        elevation: p.z,
        code: p.base.attrs.get("Kod").filter(|c| !c.is_empty()).cloned(),
    })
}

#[derive(Debug, Clone)]
pub struct State {
    points: Vec<FieldPoint>,
    /// Where the points come from, as the window's head says it.
    from: String,
    format: usize,
    job: String,
    /// The time stamp JobXML carries, taken when the window opens.
    stamp: String,
    written: FieldWrite,
    writing: bool,
    status: Option<(bool, String)>,
    /// The window it was opened from (Aplikasyon), shown again when it closes.
    pub(super) back: Option<crate::app::Dialog>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Format(usize),
    Job(String),
    Run,
}

fn event(e: Event) -> Message {
    message(Exchange::FieldSend(e))
}

impl State {
    fn options(&self) -> FieldWriteOptions {
        FieldWriteOptions {
            format: FORMATS[self.format].0,
            job: self.job.clone(),
            stamp: self.stamp.clone(),
        }
    }

    /// The file again with the current choices.
    fn write(&mut self) {
        self.written = kentos_formats::field::write::write(&self.points, &self.options());
    }

    /// Whether the file can be saved.
    fn ready(&self) -> bool {
        !self.writing && self.written.error.is_none() && self.written.written > 0
    }
}

impl App {
    /// Opens the window over `points`; `from` says where they come from.
    pub(crate) fn open_field_send(
        &mut self,
        points: Vec<FieldPoint>,
        from: String,
    ) -> Task<Message> {
        let job = self
            .document
            .as_ref()
            .map_or("", |d| d.name())
            .trim_end_matches(".kcad")
            .trim_end_matches(".KCAD")
            .to_owned();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(0));
        let mut state = State {
            points,
            from,
            format: LAST.load(Ordering::Relaxed).min(FORMATS.len() - 1),
            job: if job.is_empty() {
                "KentOS".to_owned()
            } else {
                job
            },
            stamp: crate::cloud::local_time::iso_local(
                now,
                crate::cloud::local_time::Zone::system(),
            ),
            written: FieldWrite {
                text: String::new(),
                written: 0,
                skipped: Vec::new(),
                error: None,
            },
            writing: false,
            status: None,
            back: self.dialog.filter(|d| *d != crate::app::Dialog::Exchange),
        };
        state.write();
        self.open_window(Window::FieldSend(Box::new(state)));
        Task::none()
    }

    /// Cihaza gönder from the command: the drawing's selected points.
    pub(super) fn field_send_selection(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let points: Vec<FieldPoint> = self
            .selection
            .ids()
            .iter()
            .filter_map(|&slot| doc.model.get(slot))
            .filter_map(field_point)
            .collect();
        if points.is_empty() {
            self.warn("Seçili nokta yok: cihaza gönderilecek noktaları seçin ya da Nokta editöründen veya Aplikasyon penceresinden gönderin.");
            return Task::none();
        }
        let from = format!("Çizimde seçili {} nokta", points.len());
        self.open_field_send(points, from)
    }

    pub(super) fn field_send_event(&mut self, e: Event) -> Task<Message> {
        let Some(Window::FieldSend(s)) = &mut self.exchange else {
            return Task::none();
        };
        match e {
            Event::Format(i) => {
                s.format = i.min(FORMATS.len() - 1);
                LAST.store(s.format, Ordering::Relaxed);
                s.write();
            }
            Event::Job(job) => {
                s.job = job;
                s.write();
            }
            Event::Run => {
                if !s.ready() {
                    return Task::none();
                }
                s.writing = true;
                let (_, _, extension, filter) = FORMATS[s.format];
                let job = s.job.trim();
                let name = format!(
                    "{}.{extension}",
                    if job.is_empty() { "KentOS" } else { job }
                );
                let bytes = s.written.text.clone().into_bytes();
                return self.save_export(bytes, name, (filter, extension));
            }
        }
        Task::none()
    }

    pub(super) fn field_send_written(
        &mut self,
        outcome: Option<Result<String, String>>,
    ) -> Task<Message> {
        let Some(Window::FieldSend(s)) = &mut self.exchange else {
            return Task::none();
        };
        s.writing = false;
        match outcome {
            None => s.status = None,
            Some(Err(e)) => s.status = Some((true, format!("Yazılamadı: {e}"))),
            Some(Ok(name)) => {
                let (written, left) = (s.written.written, s.written.skipped.len());
                let format = FORMATS[s.format].1;
                let rest = if left > 0 {
                    format!(
                        " {left} nokta biçimde taşınamadığı için yazılmadı (nedenleri pencerede)."
                    )
                } else {
                    String::new()
                };
                self.say(
                    Level::Success,
                    format!("“{name}”: {written} nokta {format} olarak yazıldı.{rest}"),
                );
                self.close_exchange();
            }
        }
        Task::none()
    }

    pub(super) fn field_send_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let head = row![
            icon(from_web(Some("fieldSend")))
                .size(18.0)
                .tone(Tone::Accent),
            column![
                label::strong(format!("{} nokta", s.points.len())),
                label::caption(s.from.clone()),
            ]
            .spacing(2),
        ]
        .spacing(10)
        .align_y(Center);
        let pick = Select::new(
            FORMATS.iter().map(|(_, name, _, _)| Choice::new(*name)),
            Some(s.format),
            |i| event(Event::Format(i)),
        )
        .searchable(false);
        let job = kentos_ui::widget::focus_ring(
            text_input("İş adı", &s.job)
                .on_input(|t| event(Event::Job(t)))
                .padding([5, 8])
                .size(kentos_ui::theme::typography::body())
                .style(style::field::input),
        );
        let options = row![
            container(words::field("Biçim", pick, None)).width(Length::FillPortion(2)),
            container(words::field(
                "İş adı",
                job,
                Some("Trimble JobXML ve Nikon RAW dosyasına yazılır.".to_owned()),
            ))
            .width(Length::FillPortion(3)),
        ]
        .spacing(16);
        let w = &s.written;
        let preview: Element<'a, Message> = if w.error.is_some() || w.written == 0 {
            words::empty("Önizleme yok.")
        } else {
            let lines: Vec<&str> = w.text.lines().collect();
            // As high as the lines shown (as the web's box), at most 240.
            let rows = lines.len().min(PREVIEW_LINES) + usize::from(lines.len() > PREVIEW_LINES);
            let height = (rows as f32 * 16.0 + 30.0).min(240.0);
            let mut shown = lines
                .iter()
                .take(PREVIEW_LINES)
                .copied()
                .collect::<Vec<_>>()
                .join("\n");
            if lines.len() > PREVIEW_LINES {
                shown.push_str(&format!(
                    "\n… ve {} satır daha",
                    lines.len() - PREVIEW_LINES
                ));
            }
            // The lines as written: long GSI blocks scroll sideways, never wrap.
            let bar = || {
                iced::widget::scrollable::Scrollbar::new()
                    .width(6)
                    .scroller_width(6)
            };
            container(
                scrollable(
                    container(
                        label::mono(shown)
                            .size(12.0)
                            .wrapping(iced::widget::text::Wrapping::None),
                    )
                    .padding([8, 10]),
                )
                .direction(iced::widget::scrollable::Direction::Both {
                    vertical: bar(),
                    horizontal: bar(),
                })
                .width(Fill)
                .height(Length::Fixed(height)),
            )
            .width(Fill)
            .style(style::container::bordered)
            .into()
        };
        let mut lines = Vec::new();
        if let Some(e) = &w.error {
            lines.push(words::text_line(Line::Error, e.clone()));
        } else {
            lines.push(if w.written > 0 {
                words::text_line(
                    Line::Ok,
                    format!(
                        "{} nokta {} olarak yazılacak; değerler milimetreye yuvarlanır.",
                        w.written, FORMATS[s.format].1
                    ),
                )
            } else {
                words::text_line(Line::Warn, "Yazılacak nokta yok.")
            });
            if !w.skipped.is_empty() {
                let mut list = Column::new().spacing(2);
                for k in w.skipped.iter().take(SHOWN) {
                    list = list.push(label::caption(format!("• {}", k.problem)));
                }
                if w.skipped.len() > SHOWN {
                    list = list.push(label::caption(format!(
                        "• … ve {} nokta daha.",
                        w.skipped.len() - SHOWN
                    )));
                }
                lines.push(words::line(
                    Line::Warn,
                    column![
                        label::body(format!(
                            "{} nokta bu biçimde taşınamıyor; yazılmayacak:",
                            w.skipped.len()
                        )),
                        list
                    ]
                    .spacing(4),
                ));
            }
        }
        let mut body = Column::new()
            .spacing(12)
            .push(head)
            .push(options)
            .push(preview)
            .push(words::summary(lines));
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        overlay::blocking(
            Dialog::new("Cihaza gönder")
                // The body scrolls; the buttons stay in view whatever the window's height.
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "Kaydet…",
                    s.ready().then(|| event(Event::Run)),
                ))
                .width(760.0)
                .max_height(760.0),
        )
    }
}

#[cfg(test)]
impl State {
    /// The file as it stands.
    pub(super) fn written(&self) -> &FieldWrite {
        &self.written
    }

    /// The format chosen, by its place in the list.
    pub(super) fn format(&self) -> usize {
        self.format
    }

    /// Where the points come from.
    pub(super) fn from(&self) -> &str {
        &self.from
    }
}
