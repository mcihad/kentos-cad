//! GeoParquet dışa aktar and GeoParquet içe aktar (docs/adr/0628, plan/gis.md CBS-05). Out: before anything is written
//! the window shows the rows and row groups, the columns with their Parquet types, the geometry types, the system and
//! what is left out, with the compression and the bbox column as choices; Dışa aktar… asks for the path and writes
//! through `file.export.parquet`. In: the file picked, the plan of `file.import.parquet` shown (with Projenin sistemine
//! çevir when the file is in another system, and Var olan katmanla birleştir); İçe aktar takes the rows in one undo
//! step. A script and the window do the same.

use std::fmt;
use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{
    CommandResult, CommandWarning, ExportParquet, GpkgImported, ImportFormat, ImportParquet,
    ParquetCompression, ParquetExported,
};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, file_import_parquet, parquet_write};
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Dialog, overlay};

use super::merge_form::{self, MergeEvent, MergeForm};
use super::server_work;
use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

/// The compression as the control names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Packing(pub ParquetCompression);

impl fmt::Display for Packing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            ParquetCompression::Snappy => "Snappy",
            ParquetCompression::None => "Sıkıştırmasız",
        })
    }
}

#[derive(Debug)]
pub struct ExportState {
    pub compression: ParquetCompression,
    /// Whether the `bbox` covering column is written.
    pub covering: bool,
    /// What the export would write, or why nothing.
    pub plan: Result<ParquetExported, String>,
    pub status: Option<(bool, String)>,
}

#[derive(Debug)]
pub struct ImportState {
    pub path: PathBuf,
    pub reproject: bool,
    /// Var olan katmanla birleştir (docs/adr/0628 §8).
    pub merge: MergeForm,
    pub plan: Result<(GpkgImported, Vec<CommandWarning>), String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Compression(Packing),
    Covering,
    /// Dışa aktar…: the path asked for.
    Save,
    /// The export's path; none when the dialog was cancelled.
    Saved(Option<PathBuf>),
    /// Written, or why not.
    Done(Box<Result<(ParquetExported, Vec<CommandWarning>), String>>),
    /// The file to take in; none when the dialog was cancelled.
    Opened(Option<PathBuf>),
    Reproject,
    Merge(MergeEvent),
    Run,
}

fn event(e: Event) -> Message {
    message(Exchange::Parquet(e))
}

/// The bytes of a file in words: KB under a megabyte, else MB with one decimal.
fn size_words(bytes: u64) -> String {
    if bytes < 1024 * 1024 {
        format!("{} KB", bytes.div_ceil(1024))
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

impl App {
    fn parquet_export_input(&self, path: String, s: &ExportState) -> ExportParquet {
        ExportParquet {
            path,
            layers: Vec::new(),
            // The dialog asked before writing over.
            overwrite: true,
            compression: s.compression,
            covering: s.covering,
            time: self.export_time(),
        }
    }

    /// What the window's choices would write, made in memory.
    pub(super) fn parquet_replan(&mut self) {
        let (Some(doc), Some(Window::ParquetExport(s))) = (&self.document, &self.exchange) else {
            return;
        };
        let input = self.parquet_export_input(self.export_name(".parquet"), s);
        let plan = parquet_write::export(&doc.model, &input)
            .map(|m| m.output)
            .map_err(|(_, message, _)| message);
        if let Some(Window::ParquetExport(s)) = &mut self.exchange {
            s.plan = plan;
        }
    }

    pub(super) fn parquet_export_open(&mut self) -> Task<Message> {
        self.open_window(Window::ParquetExport(ExportState {
            compression: ParquetCompression::Snappy,
            covering: true,
            plan: Err("Açık çizim yok.".into()),
            status: None,
        }));
        self.parquet_replan();
        Task::none()
    }

    pub(super) fn parquet_import_pick(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(event(Event::Opened(Some(path.clone()))));
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("GeoParquet içe aktar")
                    .add_filter("GeoParquet", &["parquet"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |p| event(Event::Opened(p)),
        )
    }

    fn parquet_import_input(s: &ImportState) -> ImportParquet {
        ImportParquet {
            path: s.path.display().to_string(),
            layer: s.merge.layer(),
            srid: None,
            reproject: s.reproject,
            merge: s.merge.input(),
            rejects: s.merge.rejects(),
        }
    }

    /// What “Sunucuya içe aktar” asks of the server for the window as it stands (docs/adr/0561 §4): the command's
    /// input without its path, the file, the choices in words.
    pub(super) fn parquet_server_work(s: &ImportState) -> server_work::Work {
        server_work::Work::import(
            ImportFormat::Parquet,
            s.path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
            server_work::Source::Path(s.path.clone()),
            {
                // The rows refused stay on the server's side: no file of this computer is written.
                let mut input = Self::parquet_import_input(s);
                input.rejects = None;
                serde_json::to_value(input).unwrap_or_default()
            },
            s.reproject
                .then(|| "Başka sistemdeyse projenin sistemine çevrilir.".to_owned())
                .into_iter()
                .collect(),
        )
    }

    /// The import's plan for the window's choices, made on a copy of the drawing.
    fn parquet_import_replan(&mut self) {
        let Some(Window::ParquetImport(s)) = &self.exchange else {
            return;
        };
        let input = Self::parquet_import_input(s);
        let plan = match &self.document {
            Some(doc) => match file_import_parquet::plan(
                &ExecutionContext::new(&mut doc.model.clone()),
                &input,
            ) {
                CommandResult::Completed { output, warnings } => Ok((output, warnings)),
                CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                    Err(error.message)
                }
                _ => Err("GeoParquet okunamadı.".into()),
            },
            None => Err("Açık çizim yok.".into()),
        };
        if let (Some(Window::ParquetImport(s)), Some(doc)) = (&mut self.exchange, &self.document) {
            s.merge.offer(&doc.model);
            if let Ok((out, _)) = &plan {
                let made: Vec<&str> = out.layers.iter().map(|l| l.id.as_str()).collect();
                s.merge.learn(
                    &doc.model,
                    out.tables.iter().map(|t| t.layer.as_str()),
                    &made,
                );
            }
            s.plan = plan;
        }
    }

    pub(super) fn parquet_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Compression(p) => {
                if let Some(Window::ParquetExport(s)) = &mut self.exchange {
                    s.compression = p.0;
                }
                self.parquet_replan();
            }
            Event::Covering => {
                if let Some(Window::ParquetExport(s)) = &mut self.exchange {
                    s.covering = !s.covering;
                }
                self.parquet_replan();
            }
            Event::Save => {
                if let Picker::File(path) = &self.picker {
                    return Task::done(event(Event::Saved(Some(path.clone()))));
                }
                let suggested = self.export_name(".parquet");
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("GeoParquet dışa aktar")
                            .set_file_name(suggested)
                            .add_filter("GeoParquet", &["parquet"])
                            .save_file()
                            .await?;
                        let mut path = file.path().to_path_buf();
                        if path.extension().is_none() {
                            path.set_extension("parquet");
                        }
                        Some(path)
                    },
                    |p| event(Event::Saved(p)),
                );
            }
            Event::Saved(None) | Event::Opened(None) => {}
            Event::Saved(Some(path)) => {
                let Some(Window::ParquetExport(s)) = &self.exchange else {
                    return Task::none();
                };
                let input = self.parquet_export_input(path.display().to_string(), s);
                let (Some(doc), Some(Window::ParquetExport(s))) =
                    (&self.document, &mut self.exchange)
                else {
                    return Task::none();
                };
                s.status = Some((false, format!("{} yazılıyor…", path.display())));
                let model = doc.model.clone();
                return off_thread(
                    move || {
                        kentos_headless::file_commands::export_parquet(&model, &input)
                            .map_err(|e| e.message)
                    },
                    |r| Exchange::Parquet(Event::Done(Box::new(r))),
                );
            }
            Event::Done(done) => match *done {
                Ok((out, warnings)) => {
                    self.close_exchange();
                    self.announce(
                        if warnings.is_empty() {
                            Level::Success
                        } else {
                            Level::Warn
                        },
                        format!(
                            "{} yazıldı: {} satır, {}.",
                            out.path,
                            out.rows,
                            size_words(out.bytes)
                        ),
                    );
                    for w in warnings {
                        self.say(Level::Warn, w.message);
                    }
                }
                Err(why) => {
                    if let Some(Window::ParquetExport(s)) = &mut self.exchange {
                        s.status = Some((true, format!("GeoParquet yazılamadı. {why}")));
                    }
                }
            },
            Event::Opened(Some(path)) => {
                self.open_window(Window::ParquetImport(ImportState {
                    merge: MergeForm::new(&path, true),
                    path,
                    reproject: false,
                    plan: Err(String::new()),
                    status: None,
                }));
                self.parquet_import_replan();
            }
            Event::Reproject => {
                if let Some(Window::ParquetImport(s)) = &mut self.exchange {
                    s.reproject = !s.reproject;
                }
                self.parquet_import_replan();
            }
            Event::Merge(e) => {
                if let Some(Window::ParquetImport(s)) = &mut self.exchange {
                    s.merge.apply(e);
                }
                self.parquet_import_replan();
            }
            Event::Run => self.parquet_import_run(),
        }
        Task::none()
    }

    fn parquet_import_run(&mut self) {
        let (Some(Window::ParquetImport(s)), Some(doc)) = (&self.exchange, &mut self.document)
        else {
            return;
        };
        let input = Self::parquet_import_input(s);
        let first = doc.model.next_slot();
        match file_import_parquet::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, warnings } => {
                let last = doc.model.next_slot();
                let slots: Vec<Slot> = (first..last)
                    .filter_map(|n| u32::try_from(n).ok())
                    .map(Slot)
                    .filter(|s| doc.model.get(*s).is_some())
                    .collect();
                self.close_exchange();
                if !slots.is_empty() {
                    self.zoom_to(&slots);
                }
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    merge_form::taken_words(&output.file, output.objects, &output.merged),
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                if let Some(Window::ParquetImport(s)) = &mut self.exchange {
                    s.status = Some(error.message);
                }
            }
            _ => {}
        }
    }

    pub(super) fn parquet_export_view<'a>(&'a self, s: &'a ExportState) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        body = body.push(words::field(
            "Katmanlar",
            words::text_line(
                Line::Info,
                "Nesnesi olan bütün katmanlar tek dosyada; birden çok katmanda katman sütunu nesnenin katmanını söyler.",
            ),
            None,
        ));
        body = body.push(words::field(
            "Sıkıştırma",
            Segmented::new(
                [ParquetCompression::Snappy, ParquetCompression::None].map(Packing),
                Packing(s.compression),
                |p| event(Event::Compression(p)),
            ),
            Some(
                match s.compression {
                    ParquetCompression::Snappy => {
                        "Parquet'in olağan sıkıştırması; GDAL, QGIS, DuckDB ve pandas okur."
                    }
                    ParquetCompression::None => {
                        "Sayfalar olduğu gibi yazılır: dosya büyür, her okuyucu açar."
                    }
                }
                .to_owned(),
            ),
        ));
        body = body.push(words::check(
            s.covering,
            "Kutu sütununu (bbox) yaz",
            Some(event(Event::Covering)),
        ));
        let mut lines = Vec::new();
        let ready = match &s.plan {
            Ok(out) => {
                let cols: Vec<String> = out
                    .columns
                    .iter()
                    .zip(&out.types)
                    .map(|(c, t)| format!("{c} ({t})"))
                    .collect();
                lines.push(words::text_line(
                    Line::Info,
                    format!(
                        "{} satır, {} satır grubu; geometri türleri: {}.",
                        out.rows,
                        out.row_groups,
                        if out.geometry_types.is_empty() {
                            "yok".to_owned()
                        } else {
                            out.geometry_types.join(", ")
                        }
                    ),
                ));
                lines.push(words::text_line(
                    Line::Info,
                    format!("Sütunlar: {}.", cols.join(", ")),
                ));
                lines.push(words::text_line(
                    Line::Info,
                    match out.srid {
                        Some(srid) => format!(
                            "Sistem EPSG:{srid}, PROJJSON olarak; yaklaşık boy {}.",
                            size_words(out.bytes)
                        ),
                        None => format!("Yaklaşık boy {}.", size_words(out.bytes)),
                    },
                ));
                if s.covering {
                    lines.push(words::text_line(
                        Line::Ok,
                        "Kutu sütunu yazılır: GeoParquet 1.1 okuyucuları görünen alanın dışındaki satır gruplarını okumadan atlar.",
                    ));
                }
                for l in &out.lost {
                    lines.push(words::text_line(Line::Warn, format!("{l}.")));
                }
                if out.lost.is_empty() {
                    lines.push(words::text_line(
                        Line::Ok,
                        "Her nesne ve değer olduğu gibi yazılır.",
                    ));
                }
                true
            }
            Err(why) => {
                lines.push(words::text_line(Line::Warn, why.clone()));
                false
            }
        };
        if let Some(row) = self.export_time_row() {
            body = body.push(row);
        }
        body = body.push(words::summary(lines));
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        overlay::blocking(
            Dialog::new("GeoParquet dışa aktar")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "Dışa aktar…",
                    ready.then(|| event(Event::Save)),
                ))
                .width(640.0)
                .max_height(720.0)
                .on_close(Some(message(Exchange::Close))),
        )
    }

    pub(super) fn parquet_import_view<'a>(&'a self, s: &'a ImportState) -> Element<'a, Message> {
        let name = s
            .path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let size = std::fs::metadata(&s.path).map_or(0, |m| m.len());
        let mut body = Column::new().spacing(12).push(words::file_line(
            &name,
            format!("GeoParquet, {}", size_words(size)),
        ));
        body = body.push(words::check(
            s.reproject,
            "Başka sistemdeyse projenin sistemine çevir",
            Some(event(Event::Reproject)),
        ));
        body = body.push(s.merge.view(|e| event(Event::Merge(e))));
        let mut lines = Vec::new();
        let ready = match &s.plan {
            Ok((out, warnings)) => {
                let records: u32 = out.tables.iter().map(|t| t.records).sum();
                // A layer the import makes is named by the plan; one the drawing has, by its path.
                let layer = out.tables.first().map_or_else(String::new, |t| {
                    let name = out
                        .layers
                        .iter()
                        .find(|l| l.id == t.layer)
                        .map_or_else(|| self.layer_path(&t.layer), |l| l.name.clone());
                    format!(" “{name}” katmanına")
                });
                if out.merged.is_empty() {
                    lines.push(words::text_line(
                        Line::Ok,
                        format!(
                            "{} nesne{}{layer} alınacak; tek adımda, Ctrl+Z geri alır.",
                            out.objects,
                            if records > 0 {
                                format!(" ({records} geometrisiz kayıt)")
                            } else {
                                String::new()
                            }
                        ),
                    ));
                } else {
                    lines.push(words::text_line(Line::Ok, merge_form::MERGE_LINE));
                }
                lines.extend(merge_form::report_lines(&out.merged, |id| {
                    self.layer_path(id)
                }));
                if let Some(srid) = out.srid {
                    lines.push(words::text_line(
                        Line::Info,
                        format!("Dosyanın sistemi EPSG:{srid}."),
                    ));
                }
                for w in warnings
                    .iter()
                    .filter(|w| !merge_form::said_by_merge(w, &out.merged))
                {
                    lines.push(words::text_line(Line::Warn, w.message.clone()));
                }
                true
            }
            Err(why) => {
                if !why.is_empty() {
                    lines.push(words::text_line(Line::Warn, why.clone()));
                }
                false
            }
        };
        body = body.push(words::summary(lines));
        if let Some(text) = &s.status {
            body = body.push(words::text_line(Line::Error, text.clone()));
        }
        let mut dialog = Dialog::new("GeoParquet içe aktar").scroll(body);
        if let Some(button) = self.server_work_button(Self::parquet_server_work(s)) {
            dialog = dialog.action(button);
        }
        overlay::blocking(
            dialog
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "İçe aktar",
                    ready.then(|| event(Event::Run)),
                ))
                .width(640.0)
                .max_height(720.0)
                .on_close(Some(message(Exchange::Close))),
        )
    }
}
