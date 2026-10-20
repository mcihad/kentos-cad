//! GIS türevini denetle (docs/adr/0530 §4, plan/gis.md CBS-16): a derived GeoPackage picked and every row of it
//! compared, through `cad.gis.derivation`, with the object it was made from in the open drawing: rows whose source
//! changed or is gone (the derivative is old), rows changed in the GIS (the drawing is the source and does not take
//! them), objects the dataset has no row for. The window reads and writes nothing of the drawing.

use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{
    CommandResult, CommandWarning, DerivationEntry, DerivationStatus, GisDerivation, GisDerived,
    GisTableRole,
};
use kentos_native_application::{ExecutionContext, gis_derivation};
use kentos_ui::widget::{Dialog, overlay};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

/// What the check found, or why it could not.
pub type Found = Result<(GisDerived, Vec<CommandWarning>), String>;

#[derive(Debug)]
pub struct State {
    pub path: Option<PathBuf>,
    /// The answer; none while it is being asked for.
    pub found: Option<Found>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// Başka dosya…: the file asked for.
    Pick,
    /// The file; none when the dialog was cancelled.
    Picked(Option<PathBuf>),
    Checked(Box<Found>),
}

fn event(e: Event) -> Message {
    message(Exchange::GisCheck(e))
}

fn role_name(role: GisTableRole) -> &'static str {
    match role {
        GisTableRole::Layer => "katman",
        GisTableRole::Blocks => "blok yerleştirmeleri",
        GisTableRole::Annotations => "yazılar",
    }
}

fn status_name(s: DerivationStatus) -> &'static str {
    match s {
        DerivationStatus::Current => "güncel",
        DerivationStatus::Stale => "eski",
        DerivationStatus::Edited => "GIS'te değişmiş",
        DerivationStatus::SourceGone => "kaynağı yok",
        DerivationStatus::NotExported => "türevde yok",
    }
}

fn entry_line(e: &DerivationEntry) -> (Line, String) {
    let place = match e.fid {
        Some(fid) => format!("{} #{fid}", e.table),
        None => e.table.clone(),
    };
    let kind = e
        .kind
        .as_deref()
        .map_or(String::new(), |k| format!(" ({k})"));
    let line = match e.status {
        DerivationStatus::Edited => Line::Info,
        DerivationStatus::Current => Line::Ok,
        _ => Line::Warn,
    };
    (
        line,
        format!("{place}{kind}: {} — {}", status_name(e.status), e.reason),
    )
}

impl App {
    fn gis_check_run(&self, path: PathBuf) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let mut model = doc.model.clone();
        off_thread(
            move || {
                let input = GisDerivation {
                    path: path.display().to_string(),
                    limit: Some(30),
                };
                match gis_derivation::execute(&mut ExecutionContext::new(&mut model), input) {
                    CommandResult::Completed { output, warnings } => Ok((output, warnings)),
                    CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                        Err(error.message)
                    }
                    _ => Err("Denetim tamamlanmadı.".to_owned()),
                }
            },
            |r| Exchange::GisCheck(Event::Checked(Box::new(r))),
        )
    }

    fn gis_check_pick(&self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(event(Event::Picked(Some(path.clone()))));
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("GIS türevini denetle")
                    .add_filter("GeoPackage", &["gpkg"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |p| event(Event::Picked(p)),
        )
    }

    pub(super) fn gis_check_open(&mut self) -> Task<Message> {
        self.open_window(Window::GisCheck(State {
            path: None,
            found: None,
        }));
        self.gis_check_pick()
    }

    pub(super) fn gis_check_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Pick => self.gis_check_pick(),
            Event::Picked(None) => {
                // Cancelled with nothing picked yet: the window has nothing to show.
                if matches!(&self.exchange, Some(Window::GisCheck(s)) if s.path.is_none()) {
                    self.close_exchange();
                }
                Task::none()
            }
            Event::Picked(Some(path)) => {
                if let Some(Window::GisCheck(s)) = &mut self.exchange {
                    s.path = Some(path.clone());
                    s.found = None;
                }
                self.gis_check_run(path)
            }
            Event::Checked(found) => {
                if let Some(Window::GisCheck(s)) = &mut self.exchange {
                    s.found = Some(*found);
                }
                Task::none()
            }
        }
    }

    pub(super) fn gis_check_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        let mut lines = Vec::new();
        if let Some(path) = &s.path {
            body = body.push(words::file_line(
                &path
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                path.display().to_string(),
            ));
        }
        match &s.found {
            None => lines.push(words::text_line(Line::Info, "Denetleniyor…")),
            Some(Err(why)) => lines.push(words::text_line(Line::Warn, why.clone())),
            Some(Ok((out, warnings))) => {
                lines.push(words::text_line(
                    Line::Info,
                    format!(
                        "Kaynak: “{}”, {} sürümü; {}, tolerans {} m{}.",
                        out.source.name,
                        out.source.revision,
                        out.algorithm,
                        out.tolerance,
                        if out.same_project {
                            ""
                        } else {
                            "; bu çizim başka bir proje"
                        }
                    ),
                ));
                let mut list = Column::new().spacing(4);
                for t in &out.tables {
                    let mut what = format!(
                        "{} — {}, {} satır: {} güncel",
                        t.table,
                        role_name(t.role),
                        t.rows,
                        t.current
                    );
                    for (n, word) in [
                        (t.stale, "eski"),
                        (t.edited, "GIS'te değişmiş"),
                        (t.source_gone, "kaynağı yok"),
                        (t.not_exported, "türevde yok"),
                    ] {
                        if n > 0 {
                            what.push_str(&format!(", {n} {word}"));
                        }
                    }
                    list = list.push(words::text_line(
                        if t.rows == t.current && t.not_exported == 0 {
                            Line::Ok
                        } else {
                            Line::Warn
                        },
                        what,
                    ));
                }
                body = body.push(words::field("Tablolar", list, None));
                let behind = out.stale + out.source_gone + out.not_exported;
                if behind == 0 && out.edited == 0 {
                    lines.push(words::text_line(
                        Line::Ok,
                        format!(
                            "Türev güncel: {} satırın hepsi kaynağıyla aynı.",
                            out.current
                        ),
                    ));
                } else {
                    let mut entries = Column::new().spacing(4);
                    for e in &out.entries {
                        let (kind, text) = entry_line(e);
                        entries = entries.push(words::text_line(kind, text));
                    }
                    if out.unlisted > 0 {
                        entries = entries.push(words::text_line(
                            Line::Info,
                            format!("… {} satır daha (komutta limit ile).", out.unlisted),
                        ));
                    }
                    body = body.push(words::field(
                        "Eşleşmeyenler",
                        entries,
                        Some("Eski satır: kaynak dışa aktarmadan sonra değişti ya da silindi. GIS'te değişmiş satır: kaynak nesne değişmedi, değişiklik çizime geri yazılmaz. Türevde yok: nesne dışa aktarmadan sonra yapıldı.".to_owned()),
                    ));
                }
                for w in warnings {
                    lines.push(words::text_line(Line::Warn, w.message.clone()));
                }
            }
        }
        body = body.push(words::summary(lines));
        overlay::blocking(
            Dialog::new("GIS türevini denetle")
                .scroll(body)
                .action(words::secondary("Kapat", Some(message(Exchange::Close))))
                .action(words::primary("Başka dosya…", Some(event(Event::Pick))))
                .width(680.0)
                .max_height(760.0),
        )
    }
}
