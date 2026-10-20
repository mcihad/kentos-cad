//! FlatGeobuf dışa aktar and FlatGeobuf içe aktar (docs/adr/0544, plan/gis.md CBS-05). Out: the window shows before
//! anything is written the features, the columns with their types, the geometry type, whether the spatial index is
//! written and what is left out; Dışa aktar… asks for the path and writes through `file.export.fgb`. In: the file
//! picked, the plan of `file.import.fgb` shown (with Projenin sistemine çevir when the file is in another system);
//! İçe aktar takes the features in one undo step. A script and the window do the same.

use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{
    CommandResult, CommandWarning, ExportFgb, FgbExported, GpkgImported, ImportFgb, ImportFormat,
};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, fgb_table, file_import_fgb};
use kentos_ui::widget::{Dialog, overlay};

use super::server_work;
use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

#[derive(Debug)]
pub struct ExportState {
    /// What the export would write, or why nothing.
    pub plan: Result<FgbExported, String>,
    pub status: Option<(bool, String)>,
}

#[derive(Debug)]
pub struct ImportState {
    pub path: PathBuf,
    pub reproject: bool,
    pub plan: Result<(GpkgImported, Vec<CommandWarning>), String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// Dışa aktar…: the path asked for.
    Save,
    /// The export's path; none when the dialog was cancelled.
    Saved(Option<PathBuf>),
    /// Written, or why not.
    Done(Box<Result<(FgbExported, Vec<CommandWarning>), String>>),
    /// The file to take in; none when the dialog was cancelled.
    Opened(Option<PathBuf>),
    Reproject,
    Run,
}

fn event(e: Event) -> Message {
    message(Exchange::Fgb(e))
}

impl App {
    fn fgb_plan(&self) -> Result<FgbExported, String> {
        let Some(doc) = &self.document else {
            return Err("Açık çizim yok.".into());
        };
        let input = ExportFgb {
            path: self.export_name(".fgb"),
            layers: Vec::new(),
            overwrite: true,
        };
        fgb_table::export(&doc.model, &input)
            .map(|m| m.output)
            .map_err(|(_, message, _)| message)
    }

    pub(super) fn fgb_export_open(&mut self) -> Task<Message> {
        let plan = self.fgb_plan();
        self.open_window(Window::FgbExport(ExportState { plan, status: None }));
        Task::none()
    }

    pub(super) fn fgb_import_pick(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(event(Event::Opened(Some(path.clone()))));
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("FlatGeobuf içe aktar")
                    .add_filter("FlatGeobuf", &["fgb"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |p| event(Event::Opened(p)),
        )
    }

    fn fgb_import_input(s: &ImportState) -> ImportFgb {
        ImportFgb {
            path: s.path.display().to_string(),
            layer: None,
            srid: None,
            reproject: s.reproject,
        }
    }

    /// What “Sunucuya içe aktar” asks of the server for the window as it stands (docs/adr/0561 §4): the command's
    /// input without its path, the file, the choices in words.
    pub(super) fn fgb_server_work(s: &ImportState) -> server_work::Work {
        server_work::Work::import(
            ImportFormat::Fgb,
            s.path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
            server_work::Source::Path(s.path.clone()),
            serde_json::to_value(Self::fgb_import_input(s)).unwrap_or_default(),
            s.reproject
                .then(|| "Başka sistemdeyse projenin sistemine çevrilir.".to_owned())
                .into_iter()
                .collect(),
        )
    }

    /// The import's plan for the window's choice, made on a copy of the drawing.
    fn fgb_import_replan(&mut self) {
        let Some(Window::FgbImport(s)) = &self.exchange else {
            return;
        };
        let input = Self::fgb_import_input(s);
        let plan = match &self.document {
            Some(doc) => {
                match file_import_fgb::plan(&ExecutionContext::new(&mut doc.model.clone()), &input)
                {
                    CommandResult::Completed { output, warnings } => Ok((output, warnings)),
                    CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                        Err(error.message)
                    }
                    _ => Err("FlatGeobuf okunamadı.".into()),
                }
            }
            None => Err("Açık çizim yok.".into()),
        };
        if let Some(Window::FgbImport(s)) = &mut self.exchange {
            s.plan = plan;
        }
    }

    pub(super) fn fgb_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Save => {
                if let Picker::File(path) = &self.picker {
                    return Task::done(event(Event::Saved(Some(path.clone()))));
                }
                let suggested = self.export_name(".fgb");
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("FlatGeobuf dışa aktar")
                            .set_file_name(suggested)
                            .add_filter("FlatGeobuf", &["fgb"])
                            .save_file()
                            .await?;
                        let mut path = file.path().to_path_buf();
                        if path.extension().is_none() {
                            path.set_extension("fgb");
                        }
                        Some(path)
                    },
                    |p| event(Event::Saved(p)),
                );
            }
            Event::Saved(None) | Event::Opened(None) => {}
            Event::Saved(Some(path)) => {
                let (Some(doc), Some(Window::FgbExport(s))) = (&self.document, &mut self.exchange)
                else {
                    return Task::none();
                };
                let input = ExportFgb {
                    path: path.display().to_string(),
                    layers: Vec::new(),
                    // The dialog asked before writing over.
                    overwrite: true,
                };
                s.status = Some((false, format!("{} yazılıyor…", path.display())));
                let model = doc.model.clone();
                return off_thread(
                    move || {
                        kentos_headless::file_commands::export_fgb(&model, &input)
                            .map_err(|e| e.message)
                    },
                    |r| Exchange::Fgb(Event::Done(Box::new(r))),
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
                        format!("{} yazıldı: {} nesne.", out.path, out.features),
                    );
                    for w in warnings {
                        self.say(Level::Warn, w.message);
                    }
                }
                Err(why) => {
                    if let Some(Window::FgbExport(s)) = &mut self.exchange {
                        s.status = Some((true, format!("FlatGeobuf yazılamadı. {why}")));
                    }
                }
            },
            Event::Opened(Some(path)) => {
                self.open_window(Window::FgbImport(ImportState {
                    path,
                    reproject: false,
                    plan: Err(String::new()),
                    status: None,
                }));
                self.fgb_import_replan();
            }
            Event::Reproject => {
                if let Some(Window::FgbImport(s)) = &mut self.exchange {
                    s.reproject = !s.reproject;
                }
                self.fgb_import_replan();
            }
            Event::Run => self.fgb_import_run(),
        }
        Task::none()
    }

    fn fgb_import_run(&mut self) {
        let (Some(Window::FgbImport(s)), Some(doc)) = (&self.exchange, &mut self.document) else {
            return;
        };
        let input = Self::fgb_import_input(s);
        let first = doc.model.next_slot();
        match file_import_fgb::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, warnings } => {
                let last = doc.model.next_slot();
                let slots: Vec<Slot> = (first..last)
                    .filter_map(|n| u32::try_from(n).ok())
                    .map(Slot)
                    .filter(|s| doc.model.get(*s).is_some())
                    .collect();
                self.close_exchange();
                self.zoom_to(&slots);
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    format!(
                        "“{}” alındı: {} nesne; Ctrl+Z geri alır.",
                        output.file, output.objects
                    ),
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                if let Some(Window::FgbImport(s)) = &mut self.exchange {
                    s.status = Some(error.message);
                }
            }
            _ => {}
        }
    }

    pub(super) fn fgb_export_view<'a>(&'a self, s: &'a ExportState) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
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
                        "{} nesne, geometri türü {}; sütunlar: {}",
                        out.features,
                        if out.geometry == "Unknown" {
                            "karışık"
                        } else {
                            out.geometry.as_str()
                        },
                        if cols.is_empty() {
                            "yok".to_owned()
                        } else {
                            cols.join(", ")
                        }
                    ),
                ));
                lines.push(words::text_line(
                    if out.indexed { Line::Ok } else { Line::Info },
                    if out.indexed {
                        "Mekânsal dizin yazılır: QGIS ve GDAL görünen kutuyu dosyanın tamamını okumadan bulur."
                    } else {
                        "Konumsuz kayıt olduğu için mekânsal dizin yazılmaz (GDAL'ın kuralı)."
                    },
                ));
                for l in &out.lost {
                    lines.push(words::text_line(Line::Warn, l.clone()));
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
        body = body.push(words::field(
            "Katmanlar",
            words::text_line(
                Line::Info,
                "Nesnesi olan bütün katmanlar tek dosyada; birden çok katmanda katman sütunu nesnenin katmanını söyler.",
            ),
            None,
        ));
        body = body.push(words::summary(lines));
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        overlay::blocking(
            Dialog::new("FlatGeobuf dışa aktar")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "Dışa aktar…",
                    ready.then(|| event(Event::Save)),
                ))
                .width(640.0)
                .max_height(720.0),
        )
    }

    pub(super) fn fgb_import_view<'a>(&'a self, s: &'a ImportState) -> Element<'a, Message> {
        let name = s
            .path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let size = std::fs::metadata(&s.path).map_or(0, |m| m.len());
        let mut body = Column::new().spacing(12).push(words::file_line(
            &name,
            format!("FlatGeobuf, {} KB", size.div_ceil(1024)),
        ));
        body = body.push(words::check(
            s.reproject,
            "Başka sistemdeyse projenin sistemine çevir",
            Some(event(Event::Reproject)),
        ));
        let mut lines = Vec::new();
        let ready = match &s.plan {
            Ok((out, warnings)) => {
                let records: u32 = out.tables.iter().map(|t| t.records).sum();
                let layer = out
                    .tables
                    .first()
                    .map_or_else(String::new, |t| format!(" “{}” katmanına", t.table));
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
                if let Some(srid) = out.srid {
                    lines.push(words::text_line(
                        Line::Info,
                        format!("Dosyanın sistemi EPSG:{srid}."),
                    ));
                }
                for w in warnings {
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
        let mut dialog = Dialog::new("FlatGeobuf içe aktar").scroll(body);
        if let Some(button) = self.server_work_button(Self::fgb_server_work(s)) {
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
                .max_height(720.0),
        )
    }
}
