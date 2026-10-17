//! GeoPackage içe ve dışa aktar (docs/adr/0502, plan/gis.md CBS-03): the window shows a file's feature and attribute
//! tables with their rows and kinds and the file's coordinate system against the project's, and takes the chosen ones
//! with `file.import.gpkg`; a file in another system goes in only with Projenin sistemine çevir ticked (no silent
//! move). Dışa aktar asks where and writes the layers with `file.export.gpkg`. Both write through the commands, so a
//! script and the window do the same. Bağlı katman olarak aç (docs/adr/0503) binds each chosen table to a new layer
//! with `cad.sources.edit` instead: the layer is the table's working copy (bound.rs); a file in another system is not
//! bound, a bound layer is never moved.

use std::path::PathBuf;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_contracts::{
    CallOp, CommandResult, ExportGpkg, GpkgImported, SourcesEdit, SourcesEditOp,
};
use kentos_domain::Slot;
use kentos_geopackage::{DataType, GeoPackage};
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;
use kentos_ui::widget::{Dialog, overlay};
use serde_json::json;

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Picker, Window, message};
use crate::app::{App, Message};

/// A table of the file.
#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub features: bool,
    pub rows: u64,
    pub geometry: Option<String>,
}

#[derive(Debug)]
pub struct State {
    pub path: PathBuf,
    pub file: String,
    pub tables: Vec<Table>,
    pub chosen: Vec<bool>,
    /// The EPSG code the file declares, none for an undefined system.
    pub srid: Option<u32>,
    pub reproject: bool,
    /// Bağlı katman olarak aç: each table bound to its layer (`cad.sources.edit`).
    pub bind: bool,
    pub status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The file picked; whether the window opens with Bağlı katman olarak aç ticked.
    Picked(Option<PathBuf>, bool),
    Toggle(usize),
    Reproject,
    Bind,
    Run,
    Save(Option<PathBuf>),
}

fn event(e: Event) -> Message {
    message(Exchange::Gpkg(e))
}

/// The file's tables and system, or why it cannot be read.
pub fn describe(path: &std::path::Path) -> Result<State, String> {
    let gp = GeoPackage::open(path, false).map_err(|e| e.0)?;
    let mut tables = Vec::new();
    let mut srid = None;
    for l in gp.layers().map_err(|e| e.0)? {
        if !matches!(l.data_type, DataType::Features | DataType::Attributes) {
            continue;
        }
        if srid.is_none()
            && let Some(id) = l.srs_id.filter(|&id| id > 0)
            && let Some(s) = gp.srs(id).map_err(|e| e.0)?
            && s.organization.eq_ignore_ascii_case("EPSG")
        {
            srid = u32::try_from(s.organization_id).ok();
        }
        tables.push(Table {
            rows: gp.count(&l).map_err(|e| e.0)?,
            features: l.data_type == DataType::Features,
            geometry: l.geometry.as_ref().map(|g| g.type_name.clone()),
            name: l.name,
        });
    }
    let chosen = tables.iter().map(|t| t.rows > 0).collect();
    Ok(State {
        file: path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        ),
        path: path.to_path_buf(),
        tables,
        chosen,
        srid,
        reproject: false,
        bind: false,
        status: None,
    })
}

impl App {
    pub(super) fn gpkg_pick(&mut self, bind: bool) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            let path = path.clone();
            return Task::done(event(Event::Picked(Some(path), bind)));
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title(if bind {
                        "Kaynağa bağla"
                    } else {
                        "GeoPackage içe aktar"
                    })
                    .add_filter("GeoPackage", &["gpkg"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            move |p| event(Event::Picked(p, bind)),
        )
    }

    pub(super) fn gpkg_save(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            let path = path.clone();
            return Task::done(event(Event::Save(Some(path))));
        }
        let suggested = self.export_name(".gpkg");
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("GeoPackage dışa aktar")
                    .add_filter("GeoPackage", &["gpkg"])
                    .set_file_name(suggested)
                    .save_file()
                    .await?;
                let mut path = file.path().to_path_buf();
                if path.extension().is_none() {
                    path.set_extension("gpkg");
                }
                Some(path)
            },
            |p| event(Event::Save(p)),
        )
    }

    pub(super) fn gpkg_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Picked(None, _) | Event::Save(None) => {}
            Event::Picked(Some(path), bind) => match describe(&path) {
                Ok(state) => self.open_window(Window::Gpkg(State { bind, ..state })),
                Err(why) => self.say(Level::Error, format!("{} açılamadı: {why}", path.display())),
            },
            Event::Save(Some(path)) => self.gpkg_write(path),
            Event::Toggle(i) => {
                if let Some(Window::Gpkg(s)) = &mut self.exchange
                    && let Some(c) = s.chosen.get_mut(i)
                {
                    *c = !*c;
                }
            }
            Event::Reproject => {
                if let Some(Window::Gpkg(s)) = &mut self.exchange {
                    s.reproject = !s.reproject;
                }
            }
            Event::Bind => {
                if let Some(Window::Gpkg(s)) = &mut self.exchange {
                    s.bind = !s.bind;
                }
            }
            Event::Run => self.gpkg_run(),
        }
        Task::none()
    }

    fn gpkg_run(&mut self) {
        let Some(Window::Gpkg(s)) = &self.exchange else {
            return;
        };
        if s.bind {
            return self.gpkg_bind();
        }
        let tables: Vec<&str> = s
            .tables
            .iter()
            .zip(&s.chosen)
            .filter(|(_, c)| **c)
            .map(|(t, _)| t.name.as_str())
            .collect();
        let input = json!({ "path": s.path.display().to_string(), "tables": tables, "reproject": s.reproject });
        let Some(doc) = &mut self.document else {
            return;
        };
        let first = doc.model.next_slot();
        let answer = kentos_native_application::registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            "file.import.gpkg",
            CallOp::Execute,
            input,
        );
        let result: Option<CommandResult<GpkgImported>> =
            answer.ok().and_then(|v| serde_json::from_value(v).ok());
        match result {
            Some(CommandResult::Completed { output, warnings }) => {
                let last = doc.model.next_slot();
                let slots: Vec<Slot> = (first..last)
                    .filter_map(|n| u32::try_from(n).ok())
                    .map(Slot)
                    .filter(|s| doc.model.get(*s).is_some())
                    .collect();
                self.close_exchange();
                self.zoom_to(&slots);
                let records: u32 = output.tables.iter().map(|t| t.records).sum();
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    format!(
                        "“{}” alındı. {} tablodan {} nesne{}; Ctrl+Z geri alır.",
                        output.file,
                        output.tables.len(),
                        output.objects,
                        if records > 0 {
                            format!(" ({records} geometrisiz kayıt)")
                        } else {
                            String::new()
                        }
                    ),
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            Some(CommandResult::Failed { error } | CommandResult::Conflict { error }) => {
                if let Some(Window::Gpkg(s)) = &mut self.exchange {
                    s.status = Some((true, error.message));
                }
            }
            _ => {
                if let Some(Window::Gpkg(s)) = &mut self.exchange {
                    s.status = Some((
                        true,
                        "GeoPackage alınamadı: komutun cevabı okunamadı.".into(),
                    ));
                }
            }
        }
    }

    /// Bağlı katman olarak aç: each chosen table onto a new layer bound to it, one `cad.sources.edit` each.
    fn gpkg_bind(&mut self) {
        let Some(Window::Gpkg(s)) = &self.exchange else {
            return;
        };
        let path = s.path.display().to_string();
        let file = s.file.clone();
        let reproject = s.reproject;
        let tables: Vec<String> = s
            .tables
            .iter()
            .zip(&s.chosen)
            .filter(|(_, c)| **c)
            .map(|(t, _)| t.name.clone())
            .collect();
        let Some(doc) = &mut self.document else {
            return;
        };
        let first = doc.model.next_slot();
        let (mut bound, mut objects, mut warnings) = (Vec::new(), 0u32, Vec::new());
        for table in &tables {
            let input = SourcesEdit {
                edit: SourcesEditOp::Link {
                    path: path.clone(),
                    table: table.clone(),
                    layer: None,
                    reproject,
                },
            };
            match kentos_native_application::sources::edit::execute(
                &mut ExecutionContext::new(&mut doc.model),
                input,
            ) {
                CommandResult::Completed {
                    output,
                    warnings: w,
                } => {
                    objects += output.objects;
                    bound.push(table.clone());
                    warnings.extend(w);
                }
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => {
                    if let Some(Window::Gpkg(s)) = &mut self.exchange {
                        s.status = Some((
                            true,
                            if bound.is_empty() {
                                error.message
                            } else {
                                format!(
                                    "{} bağlandı; “{table}” bağlanamadı: {}",
                                    bound.join(", "),
                                    error.message
                                )
                            },
                        ));
                    }
                    return;
                }
                _ => return,
            }
        }
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
                "“{file}” bağlandı: {} tablo bağlı katman oldu, {objects} nesne. Değişiklikleri Kaynağa yaz ile dosyaya yazın.",
                bound.len()
            ),
        );
        for w in warnings {
            self.say(Level::Warn, w.message);
        }
    }

    fn gpkg_write(&mut self, path: PathBuf) {
        let Some(doc) = &self.document else {
            return;
        };
        let input = ExportGpkg {
            path: path.display().to_string(),
            layers: Vec::new(),
            overwrite: true,
        };
        match kentos_headless::file_commands::export_gpkg(&doc.model, &input) {
            Ok((output, warnings)) => {
                let rows: u32 = output.tables.iter().map(|t| t.rows).sum();
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    format!(
                        "“{name}” yazıldı. {} tablo, {rows} satır.",
                        output.tables.len()
                    ),
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            Err(error) => self.announce(
                Level::Error,
                format!("GeoPackage yazılamadı. {}", error.message),
            ),
        }
    }

    pub(super) fn gpkg_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let project = self.document.as_ref().map_or(0, |d| d.settings().srid);
        let mut list = Column::new().spacing(6);
        for (i, t) in s.tables.iter().enumerate() {
            let kind = match (&t.geometry, t.features) {
                (Some(g), true) => g.to_lowercase(),
                _ => "öznitelik tablosu".into(),
            };
            let label = format!("{} — {kind}, {} satır", t.name, t.rows);
            list = list.push(words::check(
                s.chosen[i],
                label,
                (t.rows > 0).then(|| event(Event::Toggle(i))),
            ));
        }
        let mut lines = Vec::new();
        let mismatch = s.srid.is_some_and(|f| f != project);
        match s.srid {
            None => lines.push(words::text_line(
                Line::Info,
                "Dosya koordinat sistemi bildirmiyor; koordinatlar projenin sisteminde sayılıp olduğu gibi alınır.",
            )),
            Some(f) if f == project => lines.push(words::text_line(
                Line::Ok,
                format!("Dosya projenin sisteminde (EPSG:{f}); koordinatlar olduğu gibi alınır."),
            )),
            Some(f) => lines.push(words::text_line(
                Line::Warn,
                format!(
                    "Dosya EPSG:{f} sisteminde, proje EPSG:{project}. Almak için koordinatlar projenin sistemine çevrilmeli."
                ),
            )),
        }
        let chosen = s.chosen.iter().filter(|c| **c).count();
        let rows: u64 = s
            .tables
            .iter()
            .zip(&s.chosen)
            .filter(|(_, c)| **c)
            .map(|(t, _)| t.rows)
            .sum();
        lines.push(words::text_line(
            if chosen == 0 { Line::Warn } else { Line::Info },
            if chosen == 0 {
                "Alınacak tablo seçilmedi.".to_owned()
            } else if s.bind {
                format!("{chosen} tablo, {rows} satır bağlanacak; her tablo kendi bağlı katmanı olur, sütunları alan.")
            } else {
                format!("{chosen} tablo, {rows} satır alınacak; her tablo bir katman, sütunları alan olur.")
            },
        ));
        let mut body = Column::new()
            .spacing(12)
            .push(words::file_line(&s.file, s.path.display().to_string()))
            .push(words::field("Tablolar", list, None));
        body = body.push(words::field(
            "Bağ",
            words::check(
                s.bind,
                "Bağlı katman olarak aç".to_owned(),
                Some(event(Event::Bind)),
            ),
            Some(if mismatch {
                "Katman projenin sisteminde, tablo kendi sisteminde kalır: Kaynağa yaz nesneleri tablonun sistemine geri çevirerek yazar.".to_owned()
            } else {
                "Katman tablonun çalışma kopyası olur: değişiklikler Kaynağa yaz ile dosyaya yazılır, dosyadaki değişiklikler Kaynaktan oku ile gelir.".to_owned()
            }),
        ));
        if mismatch {
            body = body.push(words::field(
                "Koordinat sistemi",
                words::check(
                    s.reproject,
                    format!("Projenin sistemine çevir (EPSG:{} → EPSG:{project})", s.srid.unwrap_or(0)),
                    Some(event(Event::Reproject)),
                ),
                Some("Projenin datum seçimleriyle; yaylı nesneler kesin çevrilemez, alınmaz ve söylenir.".to_owned()),
            ));
        }
        body = body.push(words::summary(lines));
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        let ready = chosen > 0 && (!mismatch || s.reproject);
        overlay::blocking(
            Dialog::new(if s.bind {
                "Kaynağa bağla"
            } else {
                "GeoPackage içe aktar"
            })
            .scroll(body)
            .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
            .action(words::primary(
                if s.bind { "Bağla" } else { "İçe aktar" },
                ready.then(|| event(Event::Run)),
            ))
            .width(640.0)
            .max_height(720.0),
        )
    }
}
