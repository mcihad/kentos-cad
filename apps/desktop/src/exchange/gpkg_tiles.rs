//! GeoPackage karolarını oku and Rasteri GeoPackage karosu olarak yaz (docs/adr/0556, plan/gis.md CBS-03's raster
//! profile). In: the file's tile tables are listed with their zoom levels, pixels and tile counts; a table, a zoom and
//! (asked) the view's area are chosen and the window shows what `cad.rasters.gpkgread`'s plan says (the size, the
//! pixel, the system, the tiles missing); Raster olarak ekle… asks where the GeoTIFF goes and runs the command, the
//! raster on a new layer as one undo step “GeoPackage karoları”. Out: a raster of the drawing, a table name and
//! whether the table goes into a GeoPackage that is there; the window shows `cad.rasters.gpkgwrite`'s plan (the
//! zooms, the tiles) and Yaz… asks for the file and writes through the command. Nothing is reckoned here: the plans
//! and the writes are the commands', so a script and the window do the same.

use std::path::{Path, PathBuf};

use iced::widget::{Column, container, row, text_input};
use iced::{Element, Fill, Length, Task};
use kentos_commands::{
    CommandResult, CommandWarning, Entity, GpkgTileSet, GpkgTilesMode, RastersGpkgRead,
    RastersGpkgReadDone, RastersGpkgWrite, RastersGpkgWritten,
};
use kentos_domain::{NewLayer, Slot};
use kentos_interaction::{Format, Level};
use kentos_native_application::ExecutionContext;
use kentos_native_application::gpkg_tiles::{read, tile_tables, write};
use kentos_ui::label;
use kentos_ui::widget::radio::RadioGroup;
use kentos_ui::widget::{Dialog, overlay};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

pub const READ_TITLE: &str = "GeoPackage karolarını oku";
pub const WRITE_TITLE: &str = "Rasteri GeoPackage karosu olarak yaz";
/// The undo step of a raster added from tiles (the command's own step name).
const STEP: &str = "GeoPackage karoları";

#[derive(Debug)]
pub struct ReadState {
    pub path: PathBuf,
    pub file: String,
    pub tables: Vec<GpkgTileSet>,
    /// The chosen table, by its place in `tables`, and its zoom level.
    pub table: usize,
    pub zoom: u32,
    /// Only the view's area.
    pub area: bool,
    /// The raster goes on a new layer of this name.
    pub add: bool,
    pub layer: String,
    pub plan: Result<(RastersGpkgReadDone, Vec<CommandWarning>), String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RasterChoice {
    pub uid: String,
    pub label: String,
}

#[derive(Debug)]
pub struct WriteState {
    pub rasters: Vec<RasterChoice>,
    pub chosen: usize,
    pub table: String,
    /// The table goes into a GeoPackage that is there.
    pub add: bool,
    pub plan: Result<(RastersGpkgWritten, Vec<CommandWarning>), String>,
    pub status: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The GeoPackage to read; none when the dialog was cancelled.
    Opened(Option<PathBuf>),
    Table(usize),
    Zoom(u32),
    Area,
    Add,
    Layer(String),
    /// Raster olarak ekle…: the GeoTIFF's path asked for.
    Save,
    Saved(Option<PathBuf>),
    /// The raster to write, and its table's name.
    Raster(usize),
    TableName(String),
    /// Yaz…: the GeoPackage's path asked for.
    Pick,
    Picked(Option<PathBuf>),
    /// Written, or why not.
    Done(Box<Result<(RastersGpkgWritten, Vec<CommandWarning>), String>>),
}

fn event(e: Event) -> Message {
    message(Exchange::GpkgTiles(e))
}

fn name_of(p: &Path) -> String {
    p.file_name().map_or_else(
        || p.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// A table's name from a file's: its stem with what SQLite takes as a name (`tiles` when nothing is left).
fn table_from(stem: &str) -> String {
    let mut name: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.is_empty() || name.starts_with(|c: char| c.is_ascii_digit()) {
        name.insert(0, '_');
    }
    if name == "_" { "tiles".into() } else { name }
}

impl App {
    fn tiles_drawing_file(&self) -> Option<String> {
        self.document
            .as_ref()
            .and_then(|d| d.path.as_ref())
            .map(|p| p.display().to_string())
    }

    // ── Oku ──────────────────────────────────────────────────────────────────────────────────────────────────

    pub(super) fn gpkg_tiles_read_pick(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(event(Event::Opened(Some(path.clone()))));
        }
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title(READ_TITLE)
                    .add_filter("GeoPackage", &["gpkg"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |p| event(Event::Opened(p)),
        )
    }

    /// The command's input from the window's choices; without an output it only says what would be read.
    fn gpkg_tiles_read_input(
        &self,
        s: &ReadState,
        output: Option<&Path>,
        layer: Option<String>,
    ) -> RastersGpkgRead {
        let table = s.tables.get(s.table);
        let area = s.area.then(|| {
            let b = self.viewport.camera.visible_bounds();
            [b.min_x, b.min_y, b.max_x, b.max_y]
        });
        RastersGpkgRead {
            path: s.path.display().to_string(),
            table: table.map(|t| t.table.clone()),
            zoom: Some(s.zoom),
            area,
            output: output.map(|p| p.display().to_string()),
            layer,
            overwrite: Some(true),
            drawing: self.tiles_drawing_file(),
            expected_revision: None,
        }
    }

    fn gpkg_tiles_read_replan(&mut self) {
        let Some(Window::GpkgTilesRead(s)) = &self.exchange else {
            return;
        };
        let input = self.gpkg_tiles_read_input(s, None, None);
        let plan = match &self.document {
            Some(doc) => match read::plan(&ExecutionContext::new(&mut doc.model.clone()), &input) {
                CommandResult::Completed { output, warnings } => Ok((output, warnings)),
                CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                    Err(error.message)
                }
                _ => Err("GeoPackage okunamadı.".into()),
            },
            None => Err("Açık çizim yok.".into()),
        };
        if let Some(Window::GpkgTilesRead(s)) = &mut self.exchange {
            s.plan = plan;
            s.status = None;
        }
    }

    /// Raster olarak ekle: the command run; with a new layer first, both one undo step.
    fn gpkg_tiles_read_run(&mut self, output: PathBuf) {
        let Some(Window::GpkgTilesRead(s)) = &self.exchange else {
            return;
        };
        let (add, layer_name) = (s.add, s.layer.trim().to_owned());
        let Some(Window::GpkgTilesRead(s)) = self.exchange.take() else {
            return;
        };
        let s = *s;
        let input_for =
            |layer: Option<String>, app: &App| app.gpkg_tiles_read_input(&s, Some(&output), layer);
        let Some(doc) = &mut self.document else {
            self.exchange = Some(Window::GpkgTilesRead(Box::new(s)));
            return;
        };
        let model = &mut doc.model;
        let group = add.then(|| model.begin_group(STEP));
        let layer = match &group {
            None => None,
            Some(_) => match model.add_layer(NewLayer::layer(&layer_name), None, true) {
                Ok(id) => Some(id),
                Err(why) => {
                    if let Some(g) = group {
                        model.cancel_group(g);
                    }
                    self.exchange = Some(Window::GpkgTilesRead(Box::new(ReadState {
                        status: Some(why.0),
                        ..s
                    })));
                    return;
                }
            },
        };
        let input = input_for(layer, self);
        let Some(doc) = &mut self.document else {
            return;
        };
        let model = &mut doc.model;
        match read::execute(&mut ExecutionContext::new(model), input) {
            CommandResult::Completed {
                output: done,
                warnings,
            } => {
                if let Some(g) = group {
                    model.end_group(g);
                }
                let slot = done
                    .uid
                    .as_deref()
                    .and_then(|u| kentos_domain::Uuid::parse_str(u).ok())
                    .and_then(|u| model.slot_of(u));
                self.close_exchange();
                if let Some(slot) = slot {
                    self.zoom_to(&[slot]);
                }
                self.announce(
                    if warnings.is_empty() { Level::Success } else { Level::Warn },
                    format!(
                        "{} karo okundu: {} × {} piksel, {}{}. Ctrl+Z katmanı geri alır, dosya kalır.",
                        done.tiles_read,
                        done.width,
                        done.height,
                        done.path.as_deref().map_or_else(String::new, |p| p.to_owned()),
                        if done.uid.is_some() { format!(", “{layer_name}” katmanına eklendi") } else { String::new() }
                    ),
                );
                for w in warnings {
                    self.say(Level::Warn, w.message);
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                if let Some(g) = group {
                    model.cancel_group(g);
                }
                self.exchange = Some(Window::GpkgTilesRead(Box::new(ReadState {
                    status: Some(error.message),
                    ..s
                })));
            }
            other => {
                if let Some(g) = group {
                    model.cancel_group(g);
                }
                self.exchange = Some(Window::GpkgTilesRead(Box::new(ReadState {
                    status: Some(format!("{other:?}")),
                    ..s
                })));
            }
        }
    }

    // ── Yaz ──────────────────────────────────────────────────────────────────────────────────────────────────

    pub(super) fn gpkg_tiles_write_open(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let rasters: Vec<RasterChoice> = doc
            .model
            .entities()
            .filter_map(|e| match e {
                Entity::Raster(r) => {
                    let uid = doc.model.uid(Slot(r.base.id))?.to_string();
                    let name = r
                        .raster
                        .file
                        .as_deref()
                        .map(|f| name_of(Path::new(f)))
                        .or_else(|| r.raster.asset.clone())
                        .or_else(|| r.raster.url.clone())
                        .unwrap_or_else(|| "raster".to_owned());
                    Some(RasterChoice {
                        uid,
                        label: format!(
                            "{name} — {} × {} piksel, {} bant",
                            r.raster.width, r.raster.height, r.raster.bands
                        ),
                    })
                }
                _ => None,
            })
            .collect();
        if rasters.is_empty() {
            self.output("Çizimde raster yok: önce Raster ekle ile bir raster ekleyin.");
            return Task::none();
        }
        let table = rasters
            .first()
            .map(|r| table_from(r.label.split(['.', ' ']).next().unwrap_or("tiles")))
            .unwrap_or_else(|| "tiles".into());
        self.open_window(Window::GpkgTilesWrite(Box::new(WriteState {
            rasters,
            chosen: 0,
            table,
            add: false,
            plan: Err(String::new()),
            status: None,
        })));
        self.gpkg_tiles_write_replan();
        Task::none()
    }

    fn gpkg_tiles_write_input(
        &self,
        s: &WriteState,
        path: &Path,
        add: bool,
        overwrite: bool,
    ) -> Option<RastersGpkgWrite> {
        Some(RastersGpkgWrite {
            uid: s.rasters.get(s.chosen)?.uid.clone(),
            path: path.display().to_string(),
            table: Some(s.table.trim().to_owned()).filter(|t| !t.is_empty()),
            mode: if add {
                GpkgTilesMode::Add
            } else {
                GpkgTilesMode::New
            },
            identifier: None,
            overwrite: Some(overwrite),
            drawing: self.tiles_drawing_file(),
        })
    }

    fn gpkg_tiles_write_replan(&mut self) {
        let Some(Window::GpkgTilesWrite(s)) = &self.exchange else {
            return;
        };
        // The plan asks a file that is not there: what the pyramid would be does not depend on the place.
        let probe =
            std::env::temp_dir().join(format!("kentos-karo-plan-{}.gpkg", std::process::id()));
        let plan = match (
            &self.document,
            self.gpkg_tiles_write_input(s, &probe, false, false),
        ) {
            (Some(doc), Some(input)) => {
                match write::plan(&ExecutionContext::new(&mut doc.model.clone()), &input) {
                    CommandResult::Completed { output, warnings } => Ok((output, warnings)),
                    CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                        Err(error.message)
                    }
                    _ => Err("Raster karoya çevrilemedi.".into()),
                }
            }
            _ => Err("Açık çizim yok.".into()),
        };
        if let Some(Window::GpkgTilesWrite(s)) = &mut self.exchange {
            s.plan = plan;
            s.status = None;
        }
    }

    fn gpkg_tiles_write_pick(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(event(Event::Picked(Some(path.clone()))));
        }
        let Some(Window::GpkgTilesWrite(s)) = &self.exchange else {
            return Task::none();
        };
        let add = s.add;
        let suggested = s.table.trim().to_owned() + ".gpkg";
        Task::perform(
            async move {
                if add {
                    let file = rfd::AsyncFileDialog::new()
                        .set_title("Tablonun ekleneceği GeoPackage")
                        .add_filter("GeoPackage", &["gpkg"])
                        .pick_file()
                        .await?;
                    return Some(file.path().to_path_buf());
                }
                let file = rfd::AsyncFileDialog::new()
                    .set_title(WRITE_TITLE)
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
            |p| event(Event::Picked(p)),
        )
    }

    pub(super) fn gpkg_tiles_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Opened(None) | Event::Saved(None) | Event::Picked(None) => {}
            Event::Opened(Some(path)) => match tile_tables(&path) {
                Ok(tables) if tables.is_empty() => self.say(
                    Level::Error,
                    format!("{} dosyasında karo (raster) tablosu yok.", path.display()),
                ),
                Ok(tables) => {
                    let zoom = tables[0].zooms.last().map_or(0, |z| z.zoom);
                    let file = name_of(&path);
                    let layer = table_from(&tables[0].table);
                    self.open_window(Window::GpkgTilesRead(Box::new(ReadState {
                        path,
                        file,
                        tables,
                        table: 0,
                        zoom,
                        area: false,
                        add: true,
                        layer,
                        plan: Err(String::new()),
                        status: None,
                    })));
                    self.gpkg_tiles_read_replan();
                }
                Err(why) => self.say(Level::Error, format!("{} açılamadı: {why}", path.display())),
            },
            Event::Table(i) => {
                if let Some(Window::GpkgTilesRead(s)) = &mut self.exchange
                    && let Some(t) = s.tables.get(i)
                {
                    s.table = i;
                    s.zoom = t.zooms.last().map_or(0, |z| z.zoom);
                    s.layer = table_from(&t.table);
                }
                self.gpkg_tiles_read_replan();
            }
            Event::Zoom(z) => {
                if let Some(Window::GpkgTilesRead(s)) = &mut self.exchange {
                    s.zoom = z;
                }
                self.gpkg_tiles_read_replan();
            }
            Event::Area => {
                if let Some(Window::GpkgTilesRead(s)) = &mut self.exchange {
                    s.area = !s.area;
                }
                self.gpkg_tiles_read_replan();
            }
            Event::Add => match &mut self.exchange {
                Some(Window::GpkgTilesRead(s)) => s.add = !s.add,
                Some(Window::GpkgTilesWrite(s)) => s.add = !s.add,
                _ => {}
            },
            Event::Layer(t) => {
                if let Some(Window::GpkgTilesRead(s)) = &mut self.exchange {
                    s.layer = t;
                }
            }
            Event::Save => {
                if let Picker::File(path) = &self.picker {
                    return Task::done(event(Event::Saved(Some(path.clone()))));
                }
                let Some(Window::GpkgTilesRead(s)) = &self.exchange else {
                    return Task::none();
                };
                let table = s
                    .tables
                    .get(s.table)
                    .map_or_else(String::new, |t| t.table.clone());
                let suggested = format!(
                    "{}-{}-z{}.tif",
                    s.file.trim_end_matches(".gpkg"),
                    table,
                    s.zoom
                );
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Rasterin GeoTIFF'i")
                            .add_filter("GeoTIFF", &["tif", "tiff"])
                            .set_file_name(suggested)
                            .save_file()
                            .await?;
                        let mut path = file.path().to_path_buf();
                        if path.extension().is_none() {
                            path.set_extension("tif");
                        }
                        Some(path)
                    },
                    |p| event(Event::Saved(p)),
                );
            }
            Event::Saved(Some(path)) => self.gpkg_tiles_read_run(path),
            Event::Raster(i) => {
                if let Some(Window::GpkgTilesWrite(s)) = &mut self.exchange
                    && i < s.rasters.len()
                {
                    s.chosen = i;
                }
                self.gpkg_tiles_write_replan();
            }
            Event::TableName(t) => {
                if let Some(Window::GpkgTilesWrite(s)) = &mut self.exchange {
                    s.table = t;
                }
                self.gpkg_tiles_write_replan();
            }
            Event::Pick => return self.gpkg_tiles_write_pick(),
            Event::Picked(Some(path)) => {
                let Some(Window::GpkgTilesWrite(s)) = &self.exchange else {
                    return Task::none();
                };
                // The dialog asked before writing over a file; adding goes into the one chosen.
                let add = s.add;
                let Some(input) = self.gpkg_tiles_write_input(s, &path, add, !add) else {
                    return Task::none();
                };
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let mut model = doc.model.clone();
                if let Some(Window::GpkgTilesWrite(s)) = &mut self.exchange {
                    s.status = Some((false, format!("{} yazılıyor…", path.display())));
                }
                return off_thread(
                    move || match write::execute(&mut ExecutionContext::new(&mut model), input) {
                        CommandResult::Completed { output, warnings } => Ok((output, warnings)),
                        CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                            Err(error.message)
                        }
                        other => Err(format!("{other:?}")),
                    },
                    |r| Exchange::GpkgTiles(Event::Done(Box::new(r))),
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
                            "{} yazıldı: “{}” tablosu, {} zoom düzeyi, {} karo.",
                            out.path,
                            out.table,
                            out.zooms.len(),
                            out.tiles
                        ),
                    );
                    for w in warnings {
                        self.say(Level::Warn, w.message);
                    }
                }
                Err(why) => {
                    if let Some(Window::GpkgTilesWrite(s)) = &mut self.exchange {
                        s.status = Some((true, format!("GeoPackage yazılamadı. {why}")));
                    }
                }
            },
        }
        Task::none()
    }

    // ── Pencereler ───────────────────────────────────────────────────────────────────────────────────────────

    pub(super) fn gpkg_tiles_read_view<'a>(&'a self, s: &'a ReadState) -> Element<'a, Message> {
        let format = self
            .document
            .as_ref()
            .map_or_else(Format::default, |d| Format::of(d.settings()));
        let project = self.document.as_ref().map_or(0, |d| d.settings().srid);
        let mut tables = RadioGroup::new(s.table, |i| event(Event::Table(i)));
        for (i, t) in s.tables.iter().enumerate() {
            let system = t
                .srid
                .map_or_else(|| "sistemsiz".to_owned(), crate::crs::title_of);
            tables = tables.option(
                i,
                t.table.clone(),
                format!("{system}, {} düzey", t.zooms.len()),
            );
        }
        let mut body = Column::new()
            .spacing(12)
            .push(words::file_line(&s.file, s.path.display().to_string()))
            .push(words::field("Karo tabloları", tables, None));
        if let Some(t) = s.tables.get(s.table) {
            let mut zooms = RadioGroup::new(s.zoom, |z| event(Event::Zoom(z)));
            for z in t.zooms.iter().rev() {
                let formats = if z.formats.is_empty() {
                    "karo yok".to_owned()
                } else {
                    z.formats.join(", ")
                };
                zooms = zooms.option(
                    z.zoom,
                    format!("Düzey {}", z.zoom),
                    format!(
                        "piksel {} {}, {} × {} karo ızgarası, {} karo ({formats})",
                        format.length_bare(z.pixel[0]),
                        format.length_unit_label(),
                        z.matrix_width,
                        z.matrix_height,
                        z.tiles
                    ),
                );
            }
            body = body.push(words::field(
                "Zoom düzeyi",
                zooms,
                Some("Raster seçilen düzeyin karolarından yan yana konarak kurulur; en ince düzey en çok ayrıntıyı verir.".to_owned()),
            ));
        }
        body = body.push(words::field(
            "Alan",
            words::check(
                s.area,
                "Yalnız görünen alan".to_owned(),
                Some(event(Event::Area)),
            ),
            Some("Seçilmezse tablonun veri kutusu okunur; en çok 25 milyon piksel.".to_owned()),
        ));
        let input = |placeholder: &'static str, value: &str, on: fn(String) -> Event| {
            kentos_ui::widget::focus_ring(
                text_input(placeholder, value)
                    .on_input(move |t| event(on(t)))
                    .padding([5, 8])
                    .size(kentos_ui::theme::typography::body())
                    .style(kentos_ui::style::field::input),
            )
        };
        let add = words::check(s.add, "Katman olarak ekle", Some(event(Event::Add)));
        let layer: Element<'_, Message> = if s.add {
            input("Katmanın adı", &s.layer, Event::Layer).into()
        } else {
            label::muted("Yalnız GeoTIFF yazılır.").into()
        };
        body = body.push(
            row![
                container(add).width(Length::FillPortion(1)),
                container(layer).width(Length::FillPortion(1))
            ]
            .spacing(16)
            .align_y(iced::Center),
        );
        let mut lines = Vec::new();
        let ready = match &s.plan {
            Ok((out, warnings)) => {
                let system = if out.srid == 0 {
                    "sistemsiz".to_owned()
                } else {
                    crate::crs::title_of(out.srid)
                };
                lines.push(words::text_line(
                    Line::Ok,
                    format!(
                        "{} × {} piksel, piksel {} × {} {}; {system}; {} karo okunur, {} karo yok.",
                        crate::crs::grouped(f64::from(out.width)),
                        crate::crs::grouped(f64::from(out.height)),
                        format.length_bare(out.affine[1]),
                        format.length_bare(-out.affine[5]),
                        format.length_unit_label(),
                        out.tiles_read,
                        out.tiles_missing
                    ),
                ));
                if out.srid != 0 && s.add && out.srid != project {
                    lines.push(words::text_line(
                        Line::Warn,
                        format!(
                            "Tablonun sistemi EPSG:{}, projeninki {}; raster çevrilmeden eklenmez. “Katman olarak ekle”yi kapatıp yalnız GeoTIFF yazabilirsiniz.",
                            out.srid,
                            if project == 0 { "yerel".to_owned() } else { format!("EPSG:{project}") }
                        ),
                    ));
                }
                for w in warnings {
                    lines.push(words::text_line(Line::Warn, w.message.clone()));
                }
                !(out.srid != 0 && s.add && out.srid != project)
            }
            Err(why) => {
                if !why.is_empty() {
                    lines.push(words::text_line(Line::Warn, why.clone()));
                }
                false
            }
        };
        if let Some(why) = &s.status {
            lines.push(words::text_line(Line::Error, why.clone()));
        }
        body = body.push(words::summary(lines));
        let ready = ready && (!s.add || !s.layer.trim().is_empty());
        overlay::blocking(
            Dialog::new(READ_TITLE)
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    "Raster olarak ekle…",
                    ready.then(|| event(Event::Save)),
                ))
                .width(680.0)
                .max_height(780.0),
        )
    }

    pub(super) fn gpkg_tiles_write_view<'a>(&'a self, s: &'a WriteState) -> Element<'a, Message> {
        let format = self
            .document
            .as_ref()
            .map_or_else(Format::default, |d| Format::of(d.settings()));
        let mut rasters = RadioGroup::new(s.chosen, |i| event(Event::Raster(i)));
        for (i, r) in s.rasters.iter().enumerate() {
            rasters = rasters.option(i, r.label.clone(), "");
        }
        let input = kentos_ui::widget::focus_ring(
            text_input("tiles", &s.table)
                .on_input(|t| event(Event::TableName(t)))
                .padding([5, 8])
                .size(kentos_ui::theme::typography::body())
                .style(kentos_ui::style::field::input),
        );
        let mut body = Column::new()
            .spacing(12)
            .push(words::field("Raster", rasters, None))
            .push(words::field(
                "Karo tablosunun adı",
                container(input).width(Fill),
                Some("Harf ya da alt çizgiyle başlar; harf, rakam ve alt çizgi içerir.".to_owned()),
            ))
            .push(words::field(
                "Dosya",
                words::check(
                    s.add,
                    "Var olan GeoPackage'a ekle".to_owned(),
                    Some(message(Exchange::GpkgTiles(Event::Add))),
                ),
                Some(if s.add {
                    "Tablo seçilen GeoPackage'a tek işlemde eklenir; aynı adlı tablo varsa reddedilir.".to_owned()
                } else {
                    "Yeni bir GeoPackage yazılır; var olan dosyanın üzerine yazmayı kaydetme penceresi sorar.".to_owned()
                }),
            ));
        let mut lines = Vec::new();
        let ready = match &s.plan {
            Ok((out, warnings)) => {
                let finest = out.zooms.last();
                lines.push(words::text_line(
                    Line::Ok,
                    format!(
                        "{} zoom düzeyi, {} karo (256 × 256 PNG); en ince düzeyin pikseli {} {}; karolar {}.",
                        out.zooms.len(),
                        out.tiles,
                        finest.map_or_else(String::new, |z| format.length_bare(z.pixel[0])),
                        format.length_unit_label(),
                        match out.bands {
                            1 => "gri",
                            2 => "gri ve alfa",
                            _ => "renkli ve alfalı",
                        }
                    ),
                ));
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
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        overlay::blocking(
            Dialog::new(WRITE_TITLE)
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary(
                    if s.add { "Ekle…" } else { "Yaz…" },
                    ready.then(|| event(Event::Pick)),
                ))
                .width(680.0)
                .max_height(720.0),
        )
    }
}
