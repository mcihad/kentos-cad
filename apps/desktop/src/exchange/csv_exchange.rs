//! CSV dışa aktar and CSV içe aktar (docs/adr/0522, plan/gis.md CBS-05). Out: the geometry as WKT, as X and Y, or
//! none, and the separator are chosen; the window shows before anything is written the rows, the columns with their
//! GDAL types and what is left out; Dışa aktar… asks for the path and writes through `file.export.csv`. In: the file
//! picked, its columns are offered for the geometry (WKT, X and Y, none; found by their names first), the plan of
//! `file.import.csv` shown as the choice changes; İçe aktar takes the rows in one undo step. A script and the window do
//! the same.

use std::fmt;
use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_commands::{
    CommandResult, CommandWarning, CsvExported, CsvGeometry, CsvGeometryIn, CsvSeparator,
    ExportCsv, GpkgImported, ImportCsv, ImportFormat,
};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, csv_table, file_import_csv};
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, overlay};

use super::merge_form::{self, MergeEvent, MergeForm};
use super::server_work;
use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

/// The geometry's form, as the control names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Form(pub CsvGeometry);

impl fmt::Display for Form {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            CsvGeometry::Wkt => "WKT",
            CsvGeometry::Xy => "X ve Y",
            CsvGeometry::None => "Geometrisiz",
        })
    }
}

/// The separator, as the control names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sep(pub CsvSeparator);

impl fmt::Display for Sep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            CsvSeparator::Comma => "Virgül",
            CsvSeparator::Semicolon => "Noktalı virgül",
            CsvSeparator::Tab => "Sekme",
        })
    }
}

/// Where an imported CSV's geometry is, as the control names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InForm {
    Wkt,
    Xy,
    None,
}

impl fmt::Display for InForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            InForm::Wkt => "WKT sütunu",
            InForm::Xy => "X ve Y sütunları",
            InForm::None => "Geometrisiz",
        })
    }
}

#[derive(Debug)]
pub struct ExportState {
    pub geometry: CsvGeometry,
    pub separator: CsvSeparator,
    /// What would be written, or why nothing can be.
    pub plan: Result<CsvExported, String>,
    pub status: Option<(bool, String)>,
}

#[derive(Debug)]
pub struct ImportState {
    pub path: PathBuf,
    pub columns: Vec<String>,
    pub rows: usize,
    pub form: InForm,
    /// The WKT column; the X, Y and Z columns (Z's 0 is none).
    pub wkt: usize,
    pub x: usize,
    pub y: usize,
    pub z: usize,
    pub reproject: bool,
    /// Var olan katmanla birleştir (docs/adr/0628 §8).
    pub merge: MergeForm,
    /// What the import would take, or why not.
    pub plan: Result<(GpkgImported, Vec<CommandWarning>), String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Form(Form),
    Separator(Sep),
    Save,
    SavePicked(Option<PathBuf>),
    Written(Box<Result<(CsvExported, Vec<CommandWarning>), String>>),
    /// The CSV to take in; none when the dialog was cancelled.
    Picked(Option<PathBuf>),
    InForm(InForm),
    Wkt(usize),
    X(usize),
    Y(usize),
    Z(usize),
    Reproject,
    Merge(MergeEvent),
    Run,
}

fn event(e: Event) -> Message {
    message(Exchange::Csv(e))
}

impl ImportState {
    /// What “Sunucuya içe aktar” asks of the server for the window as it stands (docs/adr/0561 §4).
    pub(super) fn server_work(&self) -> server_work::Work {
        let mut chosen = vec![format!("Geometri: {}.", self.form)];
        if self.reproject {
            chosen.push("Başka sistemdeyse projenin sistemine çevrilir.".to_owned());
        }
        server_work::Work::import(
            ImportFormat::Csv,
            self.path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
            server_work::Source::Path(self.path.clone()),
            {
                // The rows refused stay on the server's side: no file of this computer is written.
                let mut input = self.input();
                input.rejects = None;
                serde_json::to_value(input).unwrap_or_default()
            },
            chosen,
        )
    }

    fn input(&self) -> ImportCsv {
        let col = |i: usize| self.columns.get(i).cloned().unwrap_or_default();
        ImportCsv {
            path: self.path.display().to_string(),
            geometry: Some(match self.form {
                InForm::Wkt => CsvGeometryIn::Wkt {
                    column: col(self.wkt),
                },
                InForm::Xy => CsvGeometryIn::Xy {
                    x: col(self.x),
                    y: col(self.y),
                    z: (self.z > 0).then(|| col(self.z - 1)),
                },
                InForm::None => CsvGeometryIn::None,
            }),
            layer: self.merge.layer(),
            srid: None,
            reproject: self.reproject,
            merge: self.merge.input(),
            rejects: self.merge.rejects(),
        }
    }
}

impl App {
    fn csv_plan(
        &self,
        geometry: CsvGeometry,
        separator: CsvSeparator,
    ) -> Result<CsvExported, String> {
        let Some(doc) = &self.document else {
            return Err("Açık çizim yok.".into());
        };
        let input = ExportCsv {
            path: String::new(),
            layers: Vec::new(),
            geometry,
            separator,
            overwrite: false,
        };
        csv_table::export(&doc.model, &input)
            .map(|m| m.output)
            .map_err(|(_, why, _)| why)
    }

    pub(super) fn csv_export_open(&mut self) -> Task<Message> {
        let plan = self.csv_plan(CsvGeometry::Wkt, CsvSeparator::Comma);
        self.open_window(Window::CsvExport(ExportState {
            geometry: CsvGeometry::Wkt,
            separator: CsvSeparator::Comma,
            plan,
            status: None,
        }));
        Task::none()
    }

    pub(super) fn csv_import_pick(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(event(Event::Picked(Some(path.clone()))));
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("CSV içe aktar")
                    .add_filter("CSV", &["csv", "txt"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |p| event(Event::Picked(p)),
        )
    }

    /// The import's plan for the window's choices.
    fn csv_import_replan(&mut self) {
        let Some(Window::CsvImport(s)) = &self.exchange else {
            return;
        };
        let mut input = s.input();
        // The plan writes no file: the rows refused are written by İçe aktar.
        input.rejects = None;
        let plan = match &self.document {
            // Executed on a copy: what the import would take, the drawing untouched.
            Some(doc) => match file_import_csv::execute(
                &mut ExecutionContext::new(&mut doc.model.clone()),
                input,
            ) {
                CommandResult::Completed { output, warnings } => Ok((output, warnings)),
                CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                    Err(error.message)
                }
                _ => Err("CSV okunamadı.".into()),
            },
            None => Err("Açık çizim yok.".into()),
        };
        if let (Some(Window::CsvImport(s)), Some(doc)) = (&mut self.exchange, &self.document) {
            s.merge.offer(&doc.model);
            if let Ok((out, _)) = &plan {
                let made: Vec<&str> = out.layers.iter().map(|l| l.id.as_str()).collect();
                s.merge.learn(&doc.model, out.tables.iter().map(|t| t.layer.as_str()), &made);
            }
            s.plan = plan;
        }
    }

    pub(super) fn csv_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Form(Form(geometry)) => {
                if let Some(Window::CsvExport(s)) = &self.exchange {
                    let plan = self.csv_plan(geometry, s.separator);
                    if let Some(Window::CsvExport(s)) = &mut self.exchange {
                        s.geometry = geometry;
                        s.plan = plan;
                    }
                }
            }
            Event::Separator(Sep(separator)) => {
                if let Some(Window::CsvExport(s)) = &mut self.exchange {
                    s.separator = separator;
                }
            }
            Event::Save => {
                if let Picker::File(path) = &self.picker {
                    return Task::done(event(Event::SavePicked(Some(path.clone()))));
                }
                let suggested = self.export_name(".csv");
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("CSV dışa aktar")
                            .add_filter("CSV", &["csv"])
                            .set_file_name(suggested)
                            .save_file()
                            .await?;
                        let mut path = file.path().to_path_buf();
                        if path.extension().is_none() {
                            path.set_extension("csv");
                        }
                        Some(path)
                    },
                    |p| event(Event::SavePicked(p)),
                );
            }
            Event::SavePicked(None) | Event::Picked(None) => {}
            Event::SavePicked(Some(path)) => {
                let (Some(doc), Some(Window::CsvExport(s))) = (&self.document, &mut self.exchange)
                else {
                    return Task::none();
                };
                let input = ExportCsv {
                    path: path.display().to_string(),
                    layers: Vec::new(),
                    geometry: s.geometry,
                    separator: s.separator,
                    // The save dialog asked about writing over it.
                    overwrite: true,
                };
                s.status = Some((false, format!("{} yazılıyor…", path.display())));
                let model = doc.model.clone();
                return off_thread(
                    move || {
                        kentos_headless::file_commands::export_csv(&model, &input)
                            .map_err(|e| e.message)
                    },
                    |r| Exchange::Csv(Event::Written(Box::new(r))),
                );
            }
            Event::Written(done) => match *done {
                Ok((out, warnings)) => {
                    self.close_exchange();
                    self.announce(
                        if warnings.is_empty() {
                            Level::Success
                        } else {
                            Level::Warn
                        },
                        format!(
                            "{} yazıldı: {} satır, {} sütun ({}).",
                            out.path,
                            out.rows,
                            out.columns.len(),
                            out.files
                                .iter()
                                .map(|f| f.name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ),
                    );
                    for w in warnings {
                        self.say(Level::Warn, w.message);
                    }
                }
                Err(why) => {
                    if let Some(Window::CsvExport(s)) = &mut self.exchange {
                        s.status = Some((true, format!("CSV yazılamadı. {why}")));
                    }
                }
            },
            Event::Picked(Some(path)) => {
                let read = std::fs::read(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|b| csv_table::columns(&b));
                let (columns, rows) = match read {
                    Ok(r) => r,
                    Err(why) => {
                        self.say(Level::Error, format!("{} açılamadı: {why}", path.display()));
                        return Task::none();
                    }
                };
                let at = |name: &str| columns.iter().position(|c| c == name).unwrap_or(0);
                let (form, wkt, x, y, z) = match csv_table::auto_geometry(&columns) {
                    CsvGeometryIn::Wkt { column } => (
                        InForm::Wkt,
                        at(&column),
                        0,
                        1.min(columns.len().saturating_sub(1)),
                        0,
                    ),
                    CsvGeometryIn::Xy { x, y, z } => {
                        (InForm::Xy, 0, at(&x), at(&y), z.map_or(0, |z| at(&z) + 1))
                    }
                    _ => (
                        InForm::None,
                        0,
                        0,
                        1.min(columns.len().saturating_sub(1)),
                        0,
                    ),
                };
                self.open_window(Window::CsvImport(ImportState {
                    merge: MergeForm::new(&path, true),
                    path,
                    columns,
                    rows,
                    form,
                    wkt,
                    x,
                    y,
                    z,
                    reproject: false,
                    plan: Err(String::new()),
                    status: None,
                }));
                self.csv_import_replan();
            }
            Event::InForm(f) => {
                if let Some(Window::CsvImport(s)) = &mut self.exchange {
                    s.form = f;
                }
                self.csv_import_replan();
            }
            Event::Wkt(i) | Event::X(i) | Event::Y(i) | Event::Z(i) => {
                if let Some(Window::CsvImport(s)) = &mut self.exchange {
                    match e {
                        Event::Wkt(_) => s.wkt = i,
                        Event::X(_) => s.x = i,
                        Event::Y(_) => s.y = i,
                        _ => s.z = i,
                    }
                }
                self.csv_import_replan();
            }
            Event::Reproject => {
                if let Some(Window::CsvImport(s)) = &mut self.exchange {
                    s.reproject = !s.reproject;
                }
                self.csv_import_replan();
            }
            Event::Merge(e) => {
                if let Some(Window::CsvImport(s)) = &mut self.exchange {
                    s.merge.apply(e);
                }
                self.csv_import_replan();
            }
            Event::Run => self.csv_import_run(),
        }
        Task::none()
    }

    fn csv_import_run(&mut self) {
        let (Some(Window::CsvImport(s)), Some(doc)) = (&self.exchange, &mut self.document) else {
            return;
        };
        let input = s.input();
        let first = doc.model.next_slot();
        let answer = file_import_csv::execute(&mut ExecutionContext::new(&mut doc.model), input);
        match answer {
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
                let records: u32 = output.tables.iter().map(|t| t.records).sum();
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    if output.merged.is_empty() {
                        format!(
                            "“{}” alındı: {} nesne{}; Ctrl+Z geri alır.",
                            output.file,
                            output.objects,
                            if records > 0 {
                                format!(" ({records} geometrisiz kayıt)")
                            } else {
                                String::new()
                            }
                        )
                    } else {
                        merge_form::taken_words(&output.file, output.objects, &output.merged)
                    },
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                if let Some(Window::CsvImport(s)) = &mut self.exchange {
                    s.status = Some(error.message);
                }
            }
            _ => {}
        }
    }

    pub(super) fn csv_export_view<'a>(&'a self, s: &'a ExportState) -> Element<'a, Message> {
        let mut body = Column::new().spacing(12);
        body = body.push(words::field(
            "Geometri",
            Segmented::new(
                [
                    Form(CsvGeometry::Wkt),
                    Form(CsvGeometry::Xy),
                    Form(CsvGeometry::None),
                ],
                Form(s.geometry),
                |f| event(Event::Form(f)),
            ),
            Some(match s.geometry {
                CsvGeometry::Wkt => "İlk sütun WKT: her nesne türü, yaylar CIRCULARSTRING olarak kesin. QGIS'in ayrılmış metin katmanı ve GDAL okur.",
                CsvGeometry::Xy => "Noktalar X (doğu), Y (kuzey) ve kotluysa Z sütunlarında; nokta olmayan nesneler yazılmaz.",
                CsvGeometry::None => "Yalnız öznitelikler; her nesne bir satır.",
            }
            .to_owned()),
        ));
        body = body.push(words::field(
            "Ayırıcı",
            Segmented::new(
                [
                    Sep(CsvSeparator::Comma),
                    Sep(CsvSeparator::Semicolon),
                    Sep(CsvSeparator::Tab),
                ],
                Sep(s.separator),
                |p| event(Event::Separator(p)),
            ),
            Some("Excel Türkçe ayarlarda noktalı virgülle ayrılmış dosyayı doğrudan açar; sayılarda ondalık ayırıcı her zaman noktadır.".to_owned()),
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
                    format!("{} satır; sütunlar: {}", out.rows, cols.join(", ")),
                ));
                lines.push(words::text_line(
                    Line::Info,
                    "Yanına .csvt (sütun türleri) ve projenin sistemi varsa .prj yazılır.",
                ));
                // The objects left out are in `lost`'s sentence.
                for l in &out.lost {
                    lines.push(words::text_line(Line::Warn, l.clone()));
                }
                if out.left.is_empty() && out.lost.is_empty() {
                    lines.push(words::text_line(
                        Line::Ok,
                        "Her nesne ve değer olduğu gibi yazılır.",
                    ));
                }
                out.rows > 0
            }
            Err(why) => {
                lines.push(words::text_line(Line::Warn, why.clone()));
                false
            }
        };
        body = body.push(words::summary(lines));
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        overlay::blocking(
            Dialog::new("CSV dışa aktar")
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

    pub(super) fn csv_import_view<'a>(&'a self, s: &'a ImportState) -> Element<'a, Message> {
        let name = s
            .path
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let mut body = Column::new().spacing(12).push(words::file_line(
            &name,
            format!("{} sütun, {} satır", s.columns.len(), s.rows),
        ));
        let pick = |at: usize, on: fn(usize) -> Event| {
            Select::new(
                s.columns.iter().map(|c| Choice::new(c.clone())),
                Some(at),
                move |i| event(on(i)),
            )
        };
        body = body.push(words::field(
            "Geometri",
            Segmented::new(
                [InForm::Wkt, InForm::Xy, InForm::None],
                s.form,
                |f| event(Event::InForm(f)),
            ),
            Some("Sütunların adlarından bulundu (WKT, geom; X ve Y, lon ve lat, doğu ve kuzey); değiştirilebilir.".to_owned()),
        ));
        match s.form {
            InForm::Wkt => {
                body = body.push(words::field("WKT sütunu", pick(s.wkt, Event::Wkt), None));
            }
            InForm::Xy => {
                body = body
                    .push(words::field("X (doğu)", pick(s.x, Event::X), None))
                    .push(words::field("Y (kuzey)", pick(s.y, Event::Y), None))
                    .push(words::field(
                        "Z (kot)",
                        Select::new(
                            std::iter::once(Choice::new("Yok".to_owned()))
                                .chain(s.columns.iter().map(|c| Choice::new(c.clone()))),
                            Some(s.z),
                            |i| event(Event::Z(i)),
                        ),
                        None,
                    ));
            }
            InForm::None => {}
        }
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
                            "{} nesne{} alınacak; tek adımda, Ctrl+Z geri alır.",
                            out.objects,
                            if records > 0 {
                                format!(" ve {records} geometrisiz kayıt")
                            } else {
                                String::new()
                            }
                        ),
                    ));
                } else {
                    lines.push(words::text_line(Line::Ok, merge_form::MERGE_LINE));
                }
                lines.extend(merge_form::report_lines(&out.merged, |id| self.layer_path(id)));
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
        let mut dialog = Dialog::new("CSV içe aktar").scroll(body);
        if let Some(button) = self.server_work_button(s.server_work()) {
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
