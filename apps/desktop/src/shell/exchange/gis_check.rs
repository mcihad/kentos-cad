//! GIS türevini denetle (docs/adr/0530 §4, 0634, plan/gis.md CBS-16): a derived dataset picked (a GeoPackage, a zip
//! of Shapefiles or GeoJSON files, a GeoJSON file) and every row of it compared, through `cad.gis.derivation`, with
//! the object it was made from in the open drawing: rows whose source changed or is gone (the derivative is old),
//! rows changed in the GIS, objects the dataset has no row for. The GIS's changes are the two ways out: **GIS'teki
//! değişiklikleri al** writes the edited rows to their sources through `cad.gis.takeback` (one undo step; with
//! Geometriyi de al their geometry too), **Bağı kopar** makes the tables the GIS's own data through
//! `cad.gis.detach` (the file changes, the drawing does not).

use std::path::PathBuf;

use iced::widget::Column;
use iced::widget::row;
use iced::{Element, Task};
use kentos_commands::{
    CommandResult, CommandWarning, DERIVATION_LIMIT_MAX, DerivationEntry, DerivationStatus,
    GisDerivation, GisDerived, GisDetach, GisTableRole, GisTakeback, TakebackPart, TakebackRow,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, gis_derivation, gis_detach, gis_takeback};
use kentos_ui::label;
use kentos_ui::widget::{Dialog, Switch, overlay};

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
    /// GIS'teki değişiklikleri al takes the rows' geometry too.
    pub geometry: bool,
    /// What the last way out did: an error or not, in words.
    pub said: Option<(bool, String)>,
}

/// The rows the window lists, at most.
const LISTED: usize = 30;

#[derive(Debug, Clone)]
pub enum Event {
    /// Başka dosya…: the file asked for.
    Pick,
    /// The file; none when the dialog was cancelled.
    Picked(Option<PathBuf>),
    Checked(Box<Found>),
    Geometry(bool),
    /// GIS'teki değişiklikleri al: the edited rows to their sources.
    TakeBack,
    /// Bağı kopar: the file's derived tables made the GIS's own.
    Detach,
}

fn event(e: Event) -> Message {
    message(Exchange::GisCheck(e))
}

fn role_name(role: GisTableRole) -> &'static str {
    match role {
        GisTableRole::Layer => "katman",
        GisTableRole::Blocks => "blok yerleştirmeleri",
        GisTableRole::Annotations => "yazılar",
        GisTableRole::Dimensions => "ölçüler",
        GisTableRole::Arch => "mimari nesneler",
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
                    limit: Some(DERIVATION_LIMIT_MAX),
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
                    .add_filter("GIS türevi", &["gpkg", "zip", "geojson", "json"])
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
            geometry: false,
            said: None,
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
            Event::Geometry(on) => {
                if let Some(Window::GisCheck(s)) = &mut self.exchange {
                    s.geometry = on;
                }
                Task::none()
            }
            Event::TakeBack => self.gis_take_back(),
            Event::Detach => self.gis_detach(),
        }
    }

    /// The edited rows written to their sources through `cad.gis.takeback`, then the file checked again.
    fn gis_take_back(&mut self) -> Task<Message> {
        let Some(Window::GisCheck(s)) = &self.exchange else {
            return Task::none();
        };
        let (Some(path), Some(Ok((out, _)))) = (&s.path, &s.found) else {
            return Task::none();
        };
        let take = if s.geometry {
            TakebackPart::Geometry
        } else {
            TakebackPart::Attributes
        };
        let rows: Vec<TakebackRow> = out
            .entries
            .iter()
            .filter(|e| e.status == DerivationStatus::Edited)
            .map(|e| TakebackRow {
                table: e.table.clone(),
                uid: e.uid.clone(),
                take,
            })
            .collect();
        let path = path.clone();
        let mut input = GisTakeback {
            path: path.display().to_string(),
            rows,
            expected_revision: None,
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        // A row whose GIS geometry its source cannot take (a curve made a point, an architectural object) is taken
        // with its attributes alone, asked again by the plan, rather than the whole refused.
        let mut attributes_only = 0;
        for _ in 0..input.rows.len() {
            let plan = gis_takeback::plan(&ExecutionContext::new(&mut doc.model), &input);
            let CommandResult::Failed { error } = plan else {
                break;
            };
            let row = error
                .path
                .as_deref()
                .and_then(|p| p.strip_prefix("rows."))
                .and_then(|i| i.parse::<usize>().ok());
            match (error.code.as_str(), row) {
                ("geometry_kind_mismatch" | "geometry_parametric", Some(i))
                    if input
                        .rows
                        .get(i)
                        .is_some_and(|r| r.take == TakebackPart::Geometry) =>
                {
                    input.rows[i].take = TakebackPart::Attributes;
                    attributes_only += 1;
                }
                _ => break,
            }
        }
        let said = match gis_takeback::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, warnings } => {
                let mut text = format!(
                    "{} satırdan {} nesne kaynağa yazıldı (tek geri alma adımı: “{}”).",
                    output.rows.len(),
                    output.changed,
                    gis_takeback::LABEL
                );
                if attributes_only > 0 {
                    text.push_str(&format!(
                        " {attributes_only} satırın GIS'teki geometrisi kaynağının türüne uymadığından yalnız öznitelikleri alındı."
                    ));
                }
                for w in &warnings {
                    text.push(' ');
                    text.push_str(&w.message);
                }
                self.announce(Level::Success, text.clone());
                (false, text)
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => (true, error.message),
            _ => (true, "Geri alma tamamlanmadı.".to_owned()),
        };
        if let Some(Window::GisCheck(s)) = &mut self.exchange {
            s.said = Some(said);
            s.found = None;
        }
        self.gis_check_run(path)
    }

    /// Every derived table of the file made the GIS's own data through `cad.gis.detach`.
    fn gis_detach(&mut self) -> Task<Message> {
        let Some(Window::GisCheck(s)) = &self.exchange else {
            return Task::none();
        };
        let Some(path) = s.path.clone() else {
            return Task::none();
        };
        let input = GisDetach {
            path: path.display().to_string(),
            tables: Vec::new(),
            reason: Some("Masaüstünde GIS türevini denetle penceresinden".to_owned()),
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let said = match gis_detach::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                let rows: u32 = output.tables.iter().map(|t| t.rows).sum();
                let text = format!(
                    "{} tablonun bağı koparıldı ({rows} satır): artık GIS'in kendi verisi, çizimle eşleşmez. Bağlanabilir ve içe aktarılabilir; satırları yeni nesnelerdir.",
                    output.tables.len()
                );
                self.announce(Level::Success, text.clone());
                (false, text)
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => (true, error.message),
            _ => (true, "Bağ koparılamadı.".to_owned()),
        };
        let failed = said.0;
        if let Some(Window::GisCheck(s)) = &mut self.exchange {
            s.said = Some(said);
            if !failed {
                s.found = Some(Err(
                    "Dosyada türev tablo kalmadı: bütün tabloların bağı koparıldı.".to_owned(),
                ));
            }
        }
        Task::none()
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
                    for e in out.entries.iter().take(LISTED) {
                        let (kind, text) = entry_line(e);
                        entries = entries.push(words::text_line(kind, text));
                    }
                    let more = out.unlisted as usize + out.entries.len().saturating_sub(LISTED);
                    if more > 0 {
                        entries = entries.push(words::text_line(
                            Line::Info,
                            format!("… {more} satır daha (komutta limit ile)."),
                        ));
                    }
                    body = body.push(words::field(
                        "Eşleşmeyenler",
                        entries,
                        Some("Eski satır: kaynak dışa aktarmadan sonra değişti ya da silindi. GIS'te değişmiş satır: kaynak nesne değişmedi; değişikliği kaynağa alabilir ya da tablonun bağını koparabilirsiniz. Türevde yok: nesne dışa aktarmadan sonra yapıldı.".to_owned()),
                    ));
                }
                if out.edited > 0 {
                    body = body.push(words::field(
                        "GIS'teki değişiklikler",
                        row![
                            Switch::new(s.geometry, |v| event(Event::Geometry(v))),
                            label::body("Geometriyi de al"),
                        ]
                        .spacing(8)
                        .align_y(iced::Alignment::Center),
                        Some(if s.geometry {
                            "Değişen sütunlar ve geometri alınır. Kaynağı eğri olan nesne (yay, daire, elips) GIS'teki kirişleri alır: analitik tanımı bırakılır, bağı kopar.".to_owned()
                        } else {
                            "Yalnız değişen sütunlar kaynağın özniteliklerine (yazının metni, bloğun dönüşü) yazılır; geometri olduğu gibi kalır.".to_owned()
                        }),
                    ));
                }
                for w in warnings {
                    lines.push(words::text_line(Line::Warn, w.message.clone()));
                }
            }
        }
        if let Some((error, text)) = &s.said {
            lines.push(words::text_line(
                if *error { Line::Error } else { Line::Ok },
                text.clone(),
            ));
        }
        body = body.push(words::summary(lines));
        let derived = matches!(&s.found, Some(Ok(_)));
        let edited = matches!(&s.found, Some(Ok((out, _))) if out.edited > 0);
        overlay::blocking(
            Dialog::new("GIS türevini denetle")
                .on_close(message(Exchange::Close))
                .scroll(body)
                .action(words::secondary("Kapat", Some(message(Exchange::Close))))
                .action(words::secondary(
                    "Bağı kopar",
                    derived.then(|| event(Event::Detach)),
                ))
                .action(words::secondary(
                    "GIS'teki değişiklikleri al",
                    edited.then(|| event(Event::TakeBack)),
                ))
                .action(words::primary("Başka dosya…", Some(event(Event::Pick))))
                .width(680.0)
                .max_height(760.0),
        )
    }
}
