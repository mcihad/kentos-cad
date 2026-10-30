//! GeoPackage içe ve dışa aktar (docs/adr/0502, plan/gis.md CBS-03): the window shows a file's feature and attribute
//! tables with their rows and kinds and the file's coordinate system against the project's, and takes the chosen ones
//! with `file.import.gpkg`; a file in another system goes in only with Projenin sistemine çevir ticked (no silent
//! move). Dışa aktar asks where and writes the layers with `file.export.gpkg`. Both write through the commands, so a
//! script and the window do the same. Bağlı katman olarak aç (docs/adr/0503) binds each chosen table to a new layer
//! with `cad.sources.edit` instead: the layer is the table's working copy (bound.rs); a file in another system is not
//! bound, a bound layer is never moved.

use std::path::PathBuf;

use iced::widget::{Column, text_input};
use iced::{Element, Fill, Task};
use kentos_commands::{CallOp, CommandResult, ExportGpkg, GpkgImported, MergeReport};
use kentos_domain::Slot;
use kentos_geopackage::{DataType, GeoPackage};
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;
use kentos_ui::widget::{Dialog, focus_ring, overlay};
use serde_json::json;

use super::binding::{Ask, Started, Table as ToBind};
use super::merge_form::{self, MergeEvent, MergeForm};
use super::server_work;
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
    /// With `bind`: only the rows in the view's area (docs/adr/0510).
    pub area: bool,
    /// With `bind`: the field that names a row whatever the table's row ids (docs/adr/0533 §1, 0636 §7); empty, none.
    pub key: String,
    /// The file's relations and its tables' default QGIS styles, taken with the tables (docs/adr/0627 §6, §7).
    pub relations: usize,
    pub styles: usize,
    pub status: Option<(bool, String)>,
    /// Var olan katmanla birleştir (docs/adr/0628 §8), and while it is on the plan's merges or why it would not go in.
    pub merge: MergeForm,
    pub merged: Option<Result<Vec<MergeReport>, String>>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The file picked; whether the window opens with Bağlı katman olarak aç ticked.
    Picked(Option<PathBuf>, bool),
    /// Kaynaklar's table (docs/adr/0599 §2): the file, the table alone chosen (none: as Picked), bound or copied.
    Table(PathBuf, Option<String>, bool),
    Toggle(usize),
    Reproject,
    Bind,
    Area,
    /// Anahtar alanı typed.
    Key(String),
    Merge(MergeEvent),
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
    // The relations' mapping tables and QGIS's styles come in with the layers, not as layers (docs/adr/0627 §6, §7).
    let beside = gp.side_tables().map_err(|e| e.0)?;
    for l in gp.layers().map_err(|e| e.0)? {
        if !matches!(l.data_type, DataType::Features | DataType::Attributes)
            || beside.contains(&l.name)
        {
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
    let relations = gp.relations().map_err(|e| e.0)?.len();
    let styles = gp.default_styles().map_err(|e| e.0)?.len();
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
        area: false,
        key: String::new(),
        relations,
        styles,
        status: None,
        merge: MergeForm::new(path, true),
        merged: None,
    })
}

/// The command's input for the window as it stands; `files` gives the rows refused their file (not for a plan).
fn import_input(s: &State, files: bool) -> serde_json::Value {
    let tables: Vec<&str> = s
        .tables
        .iter()
        .zip(&s.chosen)
        .filter(|(_, c)| **c)
        .map(|(t, _)| t.name.as_str())
        .collect();
    let mut input = json!({ "path": s.path.display().to_string(), "tables": tables, "reproject": s.reproject });
    if let Some(m) = s.merge.input() {
        input["merge"] = serde_json::to_value(m).unwrap_or_default();
    }
    let layers = s.merge.layers_map();
    if !layers.is_empty() {
        input["layers"] = json!(layers);
    }
    if files && let Some(r) = s.merge.rejects() {
        input["rejects"] = json!(r);
    }
    input
}

/// The relations and QGIS styles a GeoPackage carried, as the message says them (docs/adr/0627 §6, §7).
fn profile_words(relations: usize, styles: u32) -> String {
    let mut parts = Vec::new();
    if relations > 0 {
        parts.push(format!("{relations} ilişki"));
    }
    if styles > 0 {
        parts.push(format!("{styles} QGIS stili"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(", {}", parts.join(", "))
    }
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
            Event::Table(path, table, bind) => match describe(&path) {
                Ok(mut state) => {
                    if let Some(table) = &table {
                        state.chosen = state.tables.iter().map(|t| &t.name == table).collect();
                    }
                    self.open_window(Window::Gpkg(State { bind, ..state }));
                }
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
            Event::Area => {
                if let Some(Window::Gpkg(s)) = &mut self.exchange {
                    s.area = !s.area;
                }
            }
            Event::Key(k) => {
                if let Some(Window::Gpkg(s)) = &mut self.exchange {
                    s.key = k;
                }
            }
            Event::Merge(e) => {
                if let Some(Window::Gpkg(s)) = &mut self.exchange {
                    s.merge.apply(e);
                }
            }
            Event::Run => {
                self.gpkg_run();
                return Task::none();
            }
        }
        self.gpkg_merge_replan();
        Task::none()
    }

    /// While Var olan katmanla birleştir is on: the import's plan (made on a copy, no file written), its target layer
    /// and keys, and its merges or why it would not go in.
    fn gpkg_merge_replan(&mut self) {
        let (Some(Window::Gpkg(s)), Some(doc)) = (&mut self.exchange, &mut self.document) else {
            return;
        };
        if !s.merge.on || s.bind {
            s.merged = None;
            return;
        }
        // The tables chosen go onto the layer chosen, whatever a plan said before.
        s.merge.offer(&doc.model);
        let chosen: Vec<String> = s
            .tables
            .iter()
            .zip(&s.chosen)
            .filter(|(_, c)| **c)
            .map(|(t, _)| t.name.clone())
            .collect();
        s.merge.learn_tables(chosen);
        let answer = kentos_native_application::registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            "file.import.gpkg",
            CallOp::Plan,
            import_input(s, false),
        );
        let result: Option<CommandResult<GpkgImported>> =
            answer.ok().and_then(|v| serde_json::from_value(v).ok());
        s.merged = Some(match result {
            Some(CommandResult::Completed { output, .. }) => {
                let made: Vec<&str> = output.layers.iter().map(|l| l.id.as_str()).collect();
                s.merge.learn(&doc.model, output.tables.iter().map(|t| t.layer.as_str()), &made);
                Ok(output.merged)
            }
            Some(CommandResult::Failed { error } | CommandResult::Conflict { error }) => Err(error.message),
            _ => Err("GeoPackage okunamadı.".into()),
        });
    }

    /// What “Sunucuya içe aktar” asks of the server for the window as it stands (docs/adr/0561 §4): the chosen tables,
    /// whether the coordinates move into the project's system.
    pub(crate) fn gpkg_server_work(s: &State) -> server_work::Work {
        let tables: Vec<&str> = s
            .tables
            .iter()
            .zip(&s.chosen)
            .filter(|(_, c)| **c)
            .map(|(t, _)| t.name.as_str())
            .collect();
        let mut lines = vec![format!("Tablolar: {}.", tables.join(", "))];
        if s.reproject {
            lines.push("Başka sistemdeyse projenin sistemine çevrilir.".to_owned());
        }
        server_work::Work::import(
            kentos_commands::ImportFormat::Gpkg,
            s.file.clone(),
            server_work::Source::Path(s.path.clone()),
            {
                // The rows refused stay on the server's side: no file of this computer is written.
                let mut input = import_input(s, false);
                if let Some(o) = input.as_object_mut() {
                    o.remove("path");
                }
                input
            },
            lines,
        )
    }

    fn gpkg_run(&mut self) {
        let Some(Window::Gpkg(s)) = &self.exchange else {
            return;
        };
        if s.bind {
            return self.gpkg_bind();
        }
        let input = import_input(s, true);
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
                if !slots.is_empty() {
                    self.zoom_to(&slots);
                }
                let records: u32 = output.tables.iter().map(|t| t.records).sum();
                if !output.merged.is_empty() {
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
                    return;
                }
                self.announce(
                    if warnings.is_empty() {
                        Level::Success
                    } else {
                        Level::Warn
                    },
                    format!(
                        "“{}” alındı. {} tablodan {} nesne{}{}; Ctrl+Z geri alır.",
                        output.file,
                        output.tables.len(),
                        output.objects,
                        if records > 0 {
                            format!(" ({records} geometrisiz kayıt)")
                        } else {
                            String::new()
                        },
                        profile_words(output.relations.len(), output.styles)
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

    /// Bağlı katman olarak aç: each chosen table onto a new layer bound to it, one `cad.sources.edit` each, at once when
    /// small, else on threads of their own while the window closes (exchange/binding.rs, docs/adr/0757); a refusal at
    /// once is said in the window.
    fn gpkg_bind(&mut self) {
        let Some(Window::Gpkg(s)) = &self.exchange else {
            return;
        };
        // The view's area, for the tables with geometry (docs/adr/0510): a table without one is bound whole.
        let area = s.area.then(|| {
            let b = self.viewport.camera.visible_bounds();
            [b.min_x, b.min_y, b.max_x, b.max_y]
        });
        let tables: Vec<ToBind> = s
            .tables
            .iter()
            .zip(&s.chosen)
            .filter(|(_, c)| **c)
            .map(|(t, _)| ToBind {
                name: t.name.clone(),
                geometry: t.geometry.is_some(),
                rows: Some(t.rows),
            })
            .collect();
        let ask = Ask {
            source: s.path.display().to_string(),
            shown: s.file.clone(),
            tables,
            reproject: s.reproject,
            area,
            key: Some(s.key.trim().to_owned()).filter(|k| !k.is_empty()),
            file: true,
            then: "Değişiklikleri Kaynağa yaz ile dosyaya yazın.".to_owned(),
        };
        match self.start_binding(ask) {
            Started::Done | Started::Going => self.close_exchange(),
            Started::Failed(why) => {
                if let Some(Window::Gpkg(s)) = &mut self.exchange {
                    s.status = Some((true, why));
                }
            }
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
            // GeoPackage dışa aktar has no window to ask Zaman sürgüsünün zamanıyla in (docs/adr/0646 §2).
            time: None,
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
                        "“{name}” yazıldı. {} tablo, {rows} satır{}.",
                        output.tables.len(),
                        profile_words(output.relations.len(), output.styles)
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
            } else if s.bind && s.area {
                format!("{chosen} tablo bağlanacak, yalnız görünen alandaki satırlarıyla (tablolarda {rows} satır var); her tablo kendi bağlı katmanı olur.")
            } else if s.bind {
                format!("{chosen} tablo, {rows} satır bağlanacak; her tablo kendi bağlı katmanı olur, sütunları alan.")
            } else {
                let mut side = Vec::new();
                if s.relations > 0 {
                    side.push(format!("{} ilişkisi", s.relations));
                }
                if s.styles > 0 {
                    side.push(format!("{} QGIS stili", s.styles));
                }
                let side = if side.is_empty() {
                    String::new()
                } else {
                    format!(" Dosyanın {} de gelir.", side.join(" ve "))
                };
                format!("{chosen} tablo, {rows} satır alınacak; her tablo bir katman, sütunları alan olur.{side}")
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
        if s.bind {
            let b = self.viewport.camera.visible_bounds();
            let (w, h) = (b.max_x - b.min_x, b.max_y - b.min_y);
            let format = self
                .document
                .as_ref()
                .map(|d| kentos_project::format::Format::of(d.model.settings()));
            let size = format.map_or_else(
                || format!("{w:.0} × {h:.0} m"),
                |f| format!("{} × {}", f.length_bare(w), f.length(h)),
            );
            body = body.push(words::field(
                "Alan",
                words::check(
                    s.area,
                    "Yalnız görünen alanın satırları".to_owned(),
                    Some(event(Event::Area)),
                ),
                Some(format!(
                    "Görünen alan {size}. Yalnız kutusu bu alana değen satırlar alınır; eşitleme alanın dışındaki satırlara dokunmaz, dışarı taşınan nesne kendi satırına yazılır. Geometrisiz tablo bütün bağlanır."
                )),
            ));
        }
        if s.bind {
            body = body.push(words::field(
                "Anahtar alanı",
                focus_ring(
                    text_input("Yok", &s.key)
                        .on_input(|t| event(Event::Key(t)))
                        .padding([5, 8])
                        .width(Fill)
                        .size(kentos_ui::theme::typography::body())
                        .style(kentos_ui::style::field::input),
                ),
                Some("Tablo başka bir programla yeniden yazılınca satırın kimliği değişebilir: etiket iğneleri o zaman satırı bu alanın değeriyle bulur. Her satırda dolu ve tekil olmalı; boş bırakılırsa iğneler satırın kimliğiyle ve geometrisiyle izlenir.".to_owned()),
            ));
        }
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
        if !s.bind {
            body = body.push(s.merge.view(|e| event(Event::Merge(e))));
            match &s.merged {
                Some(Ok(merged)) => lines.extend(merge_form::report_lines(merged, |id| self.layer_path(id))),
                Some(Err(why)) => lines.push(words::text_line(Line::Warn, why.clone())),
                None => {}
            }
        }
        body = body.push(words::summary(lines));
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        let ready = chosen > 0 && (!mismatch || s.reproject);
        let mut dialog = Dialog::new(if s.bind {
            "Kaynağa bağla"
        } else {
            "GeoPackage içe aktar"
        })
        .scroll(body);
        // A bound layer is the table's working copy on this computer: the server's job takes the rows in as objects.
        if !s.bind
            && chosen > 0
            && let Some(button) = self.server_work_button(Self::gpkg_server_work(s))
        {
            dialog = dialog.action(button);
        }
        overlay::blocking(
            dialog
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
