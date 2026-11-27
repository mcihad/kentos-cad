//! Açılış raporu (usage review UR-014): what opening a DWG, DXF or NCZ file as a drawing of its own left out,
//! changed or only says, kept with the drawing while it is open. It comes up by itself when something was left out
//! or changed, and comes back with Dosya › Açılış raporu; a clean file says nothing more than the command line did.
//! The rows are the reader's own report (`ImportReport`'s notes and skipped items), never parsed from its words.

use iced::Element;
use iced::widget::Column;
use kentos_commands::{ImportReport, ReportItem, ReportSeverity};
use kentos_ui::widget::Dialog as Frame;

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::road_dialog::{Col, col, num, sheet_chosen};
use crate::traces::Control;

pub const TITLE: &str = "Açılış raporu";
const CLOSE: &str = "Kapat";

/// How an item of the report came through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fate {
    /// Not taken into the drawing.
    Left,
    /// Taken, but not as it was (a dimension exploded, a hatch no longer associative).
    Changed,
    /// Said only.
    Said,
}

impl Fate {
    fn word(self) -> &'static str {
        match self {
            Fate::Left => "Alınmadı",
            Fate::Changed => "Değiştirildi",
            Fate::Said => "Bilgi",
        }
    }
}

/// An opened file's report: its name and its items, the worst first.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OpenReport {
    pub file: String,
    /// When the file was opened, milliseconds since 1970 (docs/adr/9342): the report is of that opening.
    pub opened: u64,
    pub items: Vec<(Fate, ReportItem)>,
    /// The row pressed: its whole words under the table, to read and copy (UR2-024).
    pub chosen: Option<usize>,
}

impl OpenReport {
    /// The reader's report as rows: a skipped item is left out; a note as its severity says (an error left out, a
    /// warning changed, else said).
    pub fn of(file: String, report: &ImportReport) -> Self {
        let mut items: Vec<(Fate, ReportItem)> = report
            .skipped
            .iter()
            .map(|i| (Fate::Left, i.clone()))
            .chain(report.notes.iter().map(|i| {
                let fate = match i.severity {
                    Some(ReportSeverity::Error) => Fate::Left,
                    Some(ReportSeverity::Warning) => Fate::Changed,
                    _ => Fate::Said,
                };
                (fate, i.clone())
            }))
            .collect();
        items.sort_by_key(|(fate, _)| *fate);
        // The first row chosen: what was left out is read at once, whole.
        let chosen = (!items.is_empty()).then_some(0);
        let opened = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(0));
        Self { file, opened, items, chosen }
    }

    /// Whether anything was left out or changed: the window comes up by itself.
    pub fn lossy(&self) -> bool {
        self.items.iter().any(|(fate, _)| *fate != Fate::Said)
    }

    fn count(&self, fate: Fate) -> u32 {
        self.items
            .iter()
            .filter(|(f, _)| *f == fate)
            .map(|(_, i)| i.count.max(1))
            .sum()
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Close,
    /// A row pressed.
    Choose(usize),
    /// The chosen row's words to the clipboard.
    Copy,
}

fn msg(e: Event) -> Message {
    Message::OpenReport(e)
}

impl App {
    /// The open drawing's report, when it came from a DWG, DXF or NCZ file.
    pub(crate) fn open_report(&self) -> Option<&OpenReport> {
        let session = self.document.as_ref()?.session;
        self.open_reports.get(&session)
    }

    /// Dosya › Açılış raporu.
    pub(crate) fn show_open_report(&mut self) {
        if self.open_report().is_some() {
            self.dialog = Some(Dialog::OpenReport);
        } else {
            self.output(
                "Bu çizim bir DWG, DXF ya da NCZ dosyasından açılmadı; açılış raporu yalnız onlarda tutulur.",
            );
        }
    }

    pub(crate) fn open_report_event(&mut self, e: Event) -> iced::Task<Message> {
        match e {
            Event::Copy => {
                let words = self
                    .open_report()
                    .and_then(|r| r.chosen.and_then(|i| r.items.get(i)))
                    .map(|(_, item)| format!("{}: {}", item.what, item.reason));
                return words.map_or_else(iced::Task::none, iced::clipboard::write);
            }
            Event::Close => {
                if self.dialog == Some(Dialog::OpenReport) {
                    self.dialog = None;
                }
            }
            Event::Choose(i) => {
                let session = self.document.as_ref().map(|d| d.session);
                if let Some(r) = session.and_then(|s| self.open_reports.get_mut(&s)) {
                    r.chosen = Some(i);
                }
            }
        }
        iced::Task::none()
    }

    pub(crate) fn open_report_view(&self) -> Element<'_, Message> {
        let mut lines = Vec::new();
        let mut rows = Vec::new();
        match self.open_report() {
            Some(r) => {
                let (left, changed) = (r.count(Fate::Left), r.count(Fate::Changed));
                lines.push(words::text_line(
                    Kind::Info,
                    format!(
                        "“{}” ayrı bir çizim olarak açıldı ({}); rapor bu açılışındır, kaynak dosya değişmedi.",
                        r.file,
                        crate::cloud::local_time::stamp(r.opened, crate::cloud::local_time::Zone::system())
                    ),
                ));
                if left > 0 {
                    lines.push(words::text_line(
                        Kind::Error,
                        format!("{left} öğe çizime alınmadı: KCAD olarak kaydedilen çizimde bulunmaz."),
                    ));
                }
                if changed > 0 {
                    lines.push(words::text_line(
                        Kind::Warn,
                        format!(
                            "{changed} öğe değiştirilerek alındı: görünüşü korunur, kaynaktaki gibi düzenlenmez."
                        ),
                    ));
                }
                if left == 0 && changed == 0 {
                    lines.push(words::text_line(Kind::Ok, "Bütün öğeler olduğu gibi alındı."));
                }
                for (fate, item) in &r.items {
                    rows.push(vec![
                        fate.word().to_owned(),
                        item.what.clone(),
                        item.count.to_string(),
                        item.reason.clone(),
                    ]);
                }
            }
            None => lines.push(words::text_line(Kind::Info, "Açık çizimin açılış raporu yok.")),
        }
        // The reason takes what the window leaves (UR2-024: cut with 500 px free beside it); the chosen row's item and
        // reason are written whole under the table, selectable.
        let chosen = self.open_report().and_then(|r| r.chosen.and_then(|i| r.items.get(i).map(|(_, item)| (i, item))));
        let table = sheet_chosen(
            vec![
                col("Durum", 100.0),
                col("Öğe", 190.0),
                num("Adet", 60.0),
                Col { title: "Açıklama", width: None, number: false },
            ],
            rows,
            "Raporlanacak bir şey yok.",
            320.0,
            chosen.map(|(i, _)| i),
            |i| msg(Event::Choose(i)),
        );
        let mut body = Column::new().spacing(12).push(words::summary(lines)).push(table);
        if let Some((_, item)) = chosen {
            body = body.push(
                iced::widget::row![
                    kentos_ui::label::body(format!("{}: {}", item.what, item.reason)).width(iced::Fill),
                    words::ghost("Kopyala", Some(msg(Event::Copy))),
                ]
                .spacing(12)
                .align_y(iced::Center),
            );
        }
        Frame::new(TITLE)
            .push(body)
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(820.0)
            .modal(msg(Event::Close))
    }

    pub(crate) fn open_report_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
