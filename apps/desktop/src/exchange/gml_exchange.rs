//! GML dışa aktar and GML içe aktar (docs/adr/0545, plan/gis.md CBS-05). Out: the window shows before anything is
//! written the feature types (one for each layer) with their geometry and properties, the schema file that goes
//! beside and what is left out; Dışa aktar… asks for the path and writes through `file.export.gml`. In: the file
//! picked, the plan of `file.import.gml` shown (the feature types and the layers they go onto, with Projenin
//! sistemine çevir when the file is in another system); İçe aktar takes the features in one undo step. A script
//! and the window do the same.

use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{
    CommandResult, CommandWarning, ExportGml, GmlExported, GpkgImported, ImportFormat, ImportGml,
};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, file_import_gml, gml_table};
use kentos_ui::widget::{Dialog, overlay};

use super::merge_form::{self, MergeEvent, MergeForm};
use super::server_work;
use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

#[derive(Debug)]
pub struct ExportState {
    /// Whether the schema (`.xsd`) is written beside the GML.
    pub schema: bool,
    /// What the export would write, or why nothing.
    pub plan: Result<GmlExported, String>,
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
    /// Dışa aktar…: the path asked for.
    Save,
    /// The export's path; none when the dialog was cancelled.
    Saved(Option<PathBuf>),
    /// Written, or why not.
    Done(Box<Result<(GmlExported, Vec<CommandWarning>), String>>),
    /// The schema file beside the GML on or off.
    Schema,
    /// The file to take in; none when the dialog was cancelled.
    Opened(Option<PathBuf>),
    Reproject,
    Merge(MergeEvent),
    Run,
}

fn event(e: Event) -> Message {
    message(Exchange::Gml(e))
}

impl App {
    fn gml_plan(&self, schema: bool) -> Result<GmlExported, String> {
        let Some(doc) = &self.document else {
            return Err("Açık çizim yok.".into());
        };
        let input = ExportGml {
            path: self.export_name(".gml"),
            layers: Vec::new(),
            schema,
            overwrite: true,
        };
        gml_table::export(&doc.model, &input)
            .map(|m| m.output)
            .map_err(|(_, message, _)| message)
    }

    pub(super) fn gml_export_open(&mut self) -> Task<Message> {
        let plan = self.gml_plan(true);
        self.open_window(Window::GmlExport(ExportState {
            schema: true,
            plan,
            status: None,
        }));
        Task::none()
    }

    pub(super) fn gml_import_pick(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(event(Event::Opened(Some(path.clone()))));
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("GML içe aktar")
                    .add_filter("GML", &["gml", "xml"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |p| event(Event::Opened(p)),
        )
    }

    fn gml_import_input(s: &ImportState) -> ImportGml {
        ImportGml {
            path: s.path.display().to_string(),
            tables: Vec::new(),
            layers: s.merge.layers_map(),
            srid: None,
            reproject: s.reproject,
            merge: s.merge.input(),
            rejects: s.merge.rejects(),
        }
    }

    /// What “Sunucuya içe aktar” asks of the server for the window as it stands (docs/adr/0561 §4).
    pub(super) fn gml_server_work(s: &ImportState) -> server_work::Work {
        server_work::Work::import(
            ImportFormat::Gml,
            s.path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
            server_work::Source::Path(s.path.clone()),
            {
                // The rows refused stay on the server's side: no file of this computer is written.
                let mut input = Self::gml_import_input(s);
                input.rejects = None;
                serde_json::to_value(input).unwrap_or_default()
            },
            s.reproject
                .then(|| "Başka sistemdeyse projenin sistemine çevrilir.".to_owned())
                .into_iter()
                .collect(),
        )
    }

    /// The import's plan for the window's choice, made on a copy of the drawing.
    fn gml_import_replan(&mut self) {
        let Some(Window::GmlImport(s)) = &self.exchange else {
            return;
        };
        let input = Self::gml_import_input(s);
        let plan = match &self.document {
            Some(doc) => {
                match file_import_gml::plan(&ExecutionContext::new(&mut doc.model.clone()), &input)
                {
                    CommandResult::Completed { output, warnings } => Ok((output, warnings)),
                    CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                        Err(error.message)
                    }
                    _ => Err("GML okunamadı.".into()),
                }
            }
            None => Err("Açık çizim yok.".into()),
        };
        if let (Some(Window::GmlImport(s)), Some(doc)) = (&mut self.exchange, &self.document) {
            s.merge.offer(&doc.model);
            if let Ok((out, _)) = &plan {
                let made: Vec<&str> = out.layers.iter().map(|l| l.id.as_str()).collect();
                s.merge.learn(&doc.model, out.tables.iter().map(|t| t.layer.as_str()), &made);
                s.merge.learn_tables(out.tables.iter().map(|t| t.table.clone()));
            }
            s.plan = plan;
        }
    }

    pub(super) fn gml_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Save => {
                if let Picker::File(path) = &self.picker {
                    return Task::done(event(Event::Saved(Some(path.clone()))));
                }
                let suggested = self.export_name(".gml");
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("GML dışa aktar")
                            .set_file_name(suggested)
                            .add_filter("GML", &["gml"])
                            .save_file()
                            .await?;
                        let mut path = file.path().to_path_buf();
                        if path.extension().is_none() {
                            path.set_extension("gml");
                        }
                        Some(path)
                    },
                    |p| event(Event::Saved(p)),
                );
            }
            Event::Saved(None) | Event::Opened(None) => {}
            Event::Saved(Some(path)) => {
                let (Some(doc), Some(Window::GmlExport(s))) = (&self.document, &mut self.exchange)
                else {
                    return Task::none();
                };
                let input = ExportGml {
                    path: path.display().to_string(),
                    layers: Vec::new(),
                    schema: s.schema,
                    // The dialog asked before writing over.
                    overwrite: true,
                };
                s.status = Some((false, format!("{} yazılıyor…", path.display())));
                let model = doc.model.clone();
                return off_thread(
                    move || {
                        kentos_headless::file_commands::export_gml(&model, &input)
                            .map_err(|e| e.message)
                    },
                    |r| Exchange::Gml(Event::Done(Box::new(r))),
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
                    if let Some(Window::GmlExport(s)) = &mut self.exchange {
                        s.status = Some((true, format!("GML yazılamadı. {why}")));
                    }
                }
            },
            Event::Schema => {
                let schema = match &mut self.exchange {
                    Some(Window::GmlExport(s)) => {
                        s.schema = !s.schema;
                        s.schema
                    }
                    _ => return Task::none(),
                };
                let plan = self.gml_plan(schema);
                if let Some(Window::GmlExport(s)) = &mut self.exchange {
                    s.plan = plan;
                }
            }
            Event::Opened(Some(path)) => {
                self.open_window(Window::GmlImport(ImportState {
                    merge: MergeForm::new(&path, true),
                    path,
                    reproject: false,
                    plan: Err(String::new()),
                    status: None,
                }));
                self.gml_import_replan();
            }
            Event::Reproject => {
                if let Some(Window::GmlImport(s)) = &mut self.exchange {
                    s.reproject = !s.reproject;
                }
                self.gml_import_replan();
            }
            Event::Merge(e) => {
                if let Some(Window::GmlImport(s)) = &mut self.exchange {
                    s.merge.apply(e);
                }
                self.gml_import_replan();
            }
            Event::Run => self.gml_import_run(),
        }
        Task::none()
    }

    fn gml_import_run(&mut self) {
        let (Some(Window::GmlImport(s)), Some(doc)) = (&self.exchange, &mut self.document) else {
            return;
        };
        let input = Self::gml_import_input(s);
        let first = doc.model.next_slot();
        match file_import_gml::execute(&mut ExecutionContext::new(&mut doc.model), input) {
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
                if let Some(Window::GmlImport(s)) = &mut self.exchange {
                    s.status = Some(error.message);
                }
            }
            _ => {}
        }
    }

    pub(super) fn gml_export_view<'a>(&'a self, s: &'a ExportState) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        let mut lines = Vec::new();
        let ready = match &s.plan {
            Ok(out) => {
                lines.push(words::text_line(
                    Line::Info,
                    format!(
                        "{} nesne, {} nesne türü (her katman bir tür).",
                        out.features,
                        out.layers.len()
                    ),
                ));
                for l in &out.layers {
                    let cols: Vec<String> = l
                        .columns
                        .iter()
                        .zip(&l.types)
                        .map(|(c, t)| format!("{c} ({})", t.trim_start_matches("xs:")))
                        .collect();
                    lines.push(words::text_line(
                        Line::Info,
                        format!(
                            "{}: {} nesne, geometri {}; özellikler: {}",
                            l.name,
                            l.features,
                            match l.geometry.trim_end_matches("PropertyType") {
                                "Geometry" => "karışık",
                                other => other,
                            },
                            if cols.is_empty() {
                                "yok".to_owned()
                            } else {
                                cols.join(", ")
                            }
                        ),
                    ));
                }
                if let Some(x) = out.files.iter().find(|f| f.name.ends_with(".xsd")) {
                    lines.push(words::text_line(
                        Line::Ok,
                        format!(
                            "Özelliklerin türleri {} şemasında: GDAL ve QGIS türleri oradan okur.",
                            x.name
                        ),
                    ));
                }
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
                "Nesnesi olan bütün katmanlar tek dosyada; koordinatlar projenin sistemindedir ve sistemin EPSG eksen sırasıyla yazılır (TUREF dilimleri ve enlem boylam: kuzey önce).",
            ),
            None,
        ));
        body = body.push(words::check(
            s.schema,
            "Yanına uygulama şemasını (.xsd) yaz",
            Some(event(Event::Schema)),
        ));
        body = body.push(words::summary(lines));
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        overlay::blocking(
            Dialog::new("GML dışa aktar")
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "Dışa aktar…",
                    ready.then(|| event(Event::Save)),
                ))
                .width(660.0)
                .max_height(720.0),
        )
    }

    pub(super) fn gml_import_view<'a>(&'a self, s: &'a ImportState) -> Element<'a, Message> {
        let name = s
            .path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let size = std::fs::metadata(&s.path).map_or(0, |m| m.len());
        let mut body = Column::new().spacing(12).push(words::file_line(
            &name,
            format!("GML, {} KB", size.div_ceil(1024)),
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
                if out.merged.is_empty() {
                    lines.push(words::text_line(
                        Line::Ok,
                        format!(
                            "{} nesne alınacak{}; tek adımda, Ctrl+Z geri alır.",
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
                for t in &out.tables {
                    lines.push(words::text_line(
                        Line::Info,
                        format!("“{}” → “{}” katmanı: {} nesne", t.table, t.layer, t.objects),
                    ));
                }
                lines.extend(merge_form::report_lines(&out.merged, |id| self.layer_path(id)));
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
        let mut dialog = Dialog::new("GML içe aktar").scroll(body);
        if let Some(button) = self.server_work_button(Self::gml_server_work(s)) {
            dialog = dialog.action(button);
        }
        overlay::blocking(
            dialog
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "İçe aktar",
                    ready.then(|| event(Event::Run)),
                ))
                .width(660.0)
                .max_height(720.0),
        )
    }
}
