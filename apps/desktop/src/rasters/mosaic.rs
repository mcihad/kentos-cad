//! Sanal mozaik (docs/adr/0517 §5; `raster.mosaic`): GeoTIFF files chosen, their order on the mosaic (the list's,
//! the finer pixel or the newer date on top), the mosaic's pixel and nodata and its VRT file; the window shows what
//! `cad.rasters.mosaic`'s plan says (the size, the pixel, each file's place, the warnings) as the choices change, and
//! Oluştur runs the command: the VRT written and, with “Katman olarak ekle”, the mosaic added as a raster on a new
//! layer, one undo step “Sanal mozaik”. Nothing is reckoned here: the plan and the write are the command's.

use std::path::{Path, PathBuf};

use iced::widget::{Column, button, container, row};
use iced::{Element, Fill, Length, Task};
use kentos_commands::{
    CommandResult, CommandWarning, MosaicOrder, MosaicResolution, RastersMosaic, RastersMosaicked,
};
use kentos_domain::NewLayer;
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, rasters_mosaic};
use kentos_ui::label;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::{Dialog, overlay};

use super::Event as Rasters;
use crate::app::{App, Dialog as Asking, Message, Picker};
use crate::exchange::words::{self, Kind as Line};

pub const TITLE: &str = "Sanal mozaik";

/// Which file lies on top, as the window names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    Given,
    Finest,
    Newest,
}

impl std::fmt::Display for Order {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Order::Given => "Listenin sırası",
            Order::Finest => "İnce piksel üstte",
            Order::Newest => "Yeni tarih üstte",
        })
    }
}

impl From<Order> for MosaicOrder {
    fn from(o: Order) -> MosaicOrder {
        match o {
            Order::Given => MosaicOrder::Given,
            Order::Finest => MosaicOrder::Finest,
            Order::Newest => MosaicOrder::Newest,
        }
    }
}

/// The mosaic's pixel, as the window names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pixel {
    Highest,
    Lowest,
    Average,
}

impl std::fmt::Display for Pixel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Pixel::Highest => "En ince",
            Pixel::Lowest => "En kaba",
            Pixel::Average => "Ortalama",
        })
    }
}

impl From<Pixel> for MosaicResolution {
    fn from(p: Pixel) -> MosaicResolution {
        match p {
            Pixel::Highest => MosaicResolution::Highest,
            Pixel::Lowest => MosaicResolution::Lowest,
            Pixel::Average => MosaicResolution::Average,
        }
    }
}

/// The plan's answer: the mosaic and its warnings, or why not.
type Planned = Result<(RastersMosaicked, Vec<CommandWarning>), String>;

#[derive(Debug, Clone)]
pub struct State {
    /// The files, top first (with the list's order).
    pub sources: Vec<PathBuf>,
    pub chosen: Option<usize>,
    pub order: Order,
    pub pixel: Pixel,
    /// A pixel size typed (empty: the files').
    pub size: String,
    /// A nodata typed (empty: the files').
    pub nodata: String,
    /// The VRT file; none: beside the first file.
    pub path: Option<PathBuf>,
    /// Chosen in the save dialog, which asked about an existing file.
    pub overwrite: bool,
    pub add: bool,
    pub layer: String,
    pub planned: Option<Planned>,
    status: Option<String>,
}

impl Default for State {
    fn default() -> State {
        State {
            sources: Vec::new(),
            chosen: None,
            order: Order::Given,
            pixel: Pixel::Highest,
            size: String::new(),
            nodata: String::new(),
            path: None,
            overwrite: false,
            add: true,
            layer: "Mozaik".to_owned(),
            planned: None,
            status: None,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    AddFiles,
    Picked(Option<Vec<PathBuf>>),
    Choose(usize),
    Remove,
    Up,
    Down,
    Order(Order),
    Pixel(Pixel),
    Size(String),
    Nodata(String),
    PickPath,
    PathPicked(Option<PathBuf>),
    Add,
    Layer(String),
    Run,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Rasters(Rasters::Mosaic(e))
}

fn name_of(p: &Path) -> String {
    p.file_name().map_or_else(
        || p.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

/// A number typed with a dot or a comma; none when empty, an error when not a number.
fn typed(text: &str, what: &str) -> Result<Option<f64>, String> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
        .map(Some)
        .ok_or_else(|| format!("{what} “{}” bir sayı değil.", text.trim()))
}

/// Where the VRT goes when none was chosen: beside the first file, `mozaik.vrt` (`mozaik-2.vrt` … when taken).
fn default_path(first: &Path) -> PathBuf {
    let folder = first.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut k = 1;
    loop {
        let name = if k == 1 {
            "mozaik.vrt".to_owned()
        } else {
            format!("mozaik-{k}.vrt")
        };
        let p = folder.join(name);
        if !p.exists() {
            return p;
        }
        k += 1;
    }
}

impl App {
    /// `raster.mosaic`: the window, and the file dialog over it.
    pub(crate) fn raster_mosaic_command(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        }
        self.rasters.mosaic = Some(State::default());
        self.dialog = Some(Asking::RasterMosaic);
        self.raster_mosaic_pick()
    }

    /// The window opened with these files, as Dosya ekle… would give them (the screenshots' and the tests').
    #[cfg(test)]
    pub(crate) fn raster_mosaic_with(&mut self, files: Vec<PathBuf>, order: Order) {
        let mut s = State {
            sources: files,
            order,
            ..State::default()
        };
        s.chosen = None;
        self.rasters.mosaic = Some(s);
        self.dialog = Some(Asking::RasterMosaic);
        self.raster_mosaic_replan();
    }

    fn raster_mosaic_pick(&mut self) -> Task<Message> {
        // A folder given to the trace player stands for its GeoTIFFs, in their names' order.
        if let Picker::File(path) = &self.picker {
            let files = if path.is_dir() {
                let mut f: Vec<PathBuf> = std::fs::read_dir(path)
                    .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).collect())
                    .unwrap_or_default();
                f.retain(|p| {
                    p.extension().is_some_and(|e| {
                        e.eq_ignore_ascii_case("tif") || e.eq_ignore_ascii_case("tiff")
                    })
                });
                f.sort();
                f
            } else {
                vec![path.clone()]
            };
            return Task::done(msg(Event::Picked(Some(files))));
        }
        Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title("Sanal mozaik: dosyalar")
                    .add_filter("GeoTIFF", &["tif", "tiff"])
                    .pick_files()
                    .await
                    .map(|fs| fs.into_iter().map(|f| f.path().to_path_buf()).collect())
            },
            |files| msg(Event::Picked(files)),
        )
    }

    fn raster_mosaic_path_pick(&mut self) -> Task<Message> {
        let start = self.rasters.mosaic.as_ref().and_then(|s| {
            s.path
                .clone()
                .or_else(|| s.sources.first().map(|f| default_path(f)))
        });
        if let Picker::File(path) = &self.picker
            && path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("vrt"))
        {
            return Task::done(msg(Event::PathPicked(Some(path.clone()))));
        }
        Task::perform(
            async move {
                let mut d = rfd::AsyncFileDialog::new()
                    .set_title("Sanal mozaik: VRT dosyası")
                    .add_filter("VRT", &["vrt"]);
                if let Some(p) = &start {
                    if let Some(folder) = p.parent() {
                        d = d.set_directory(folder);
                    }
                    d = d.set_file_name(name_of(p));
                }
                d.save_file().await.map(|f| f.path().to_path_buf())
            },
            |p| msg(Event::PathPicked(p)),
        )
    }

    /// The command's input from the window's choices; why not when a number typed is not one.
    fn raster_mosaic_input(
        &self,
        s: &State,
        layer: Option<String>,
    ) -> Result<RastersMosaic, String> {
        let first = s.sources.first().ok_or_else(|| {
            "Dosya seçilmedi: Dosya ekle… ile GeoTIFF dosyalarını seçin.".to_owned()
        })?;
        let path = s.path.clone().unwrap_or_else(|| default_path(first));
        Ok(RastersMosaic {
            sources: s.sources.iter().map(|p| p.display().to_string()).collect(),
            path: path.display().to_string(),
            order: s.order.into(),
            resolution: s.pixel.into(),
            pixel_size: typed(&s.size, "Piksel boyu")?,
            nodata: typed(&s.nodata, "Nodata")?,
            layer,
            overwrite: Some(s.overwrite),
            drawing: self
                .document
                .as_ref()
                .and_then(|d| d.path.as_ref())
                .map(|p| p.display().to_string()),
            expected_revision: None,
        })
    }

    /// The plan again, after a choice changed.
    fn raster_mosaic_replan(&mut self) {
        let Some(s) = &self.rasters.mosaic else {
            return;
        };
        let planned = if s.sources.is_empty() {
            None
        } else {
            Some(match (self.raster_mosaic_input(s, None), &self.document) {
                (Err(why), _) => Err(why),
                (Ok(_), None) => Err("Açık çizim yok.".into()),
                (Ok(input), Some(doc)) => match rasters_mosaic::plan(
                    &ExecutionContext::new(&mut doc.model.clone()),
                    &input,
                ) {
                    CommandResult::Completed { output, warnings } => Ok((output, warnings)),
                    CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                        Err(error.message)
                    }
                    other => Err(format!("{other:?}")),
                },
            })
        };
        if let Some(s) = &mut self.rasters.mosaic {
            s.planned = planned;
            s.status = None;
        }
    }

    pub(crate) fn raster_mosaic_event(&mut self, e: Event) -> Task<Message> {
        let Some(s) = &mut self.rasters.mosaic else {
            return Task::none();
        };
        match e {
            Event::AddFiles => return self.raster_mosaic_pick(),
            Event::Picked(None) => return Task::none(),
            Event::Picked(Some(files)) => {
                for f in files {
                    if !s.sources.contains(&f) {
                        s.sources.push(f);
                    }
                }
            }
            Event::Choose(k) => {
                s.chosen = Some(k);
                return Task::none();
            }
            Event::Remove => {
                if let Some(k) = s.chosen.filter(|&k| k < s.sources.len()) {
                    s.sources.remove(k);
                    s.chosen = (!s.sources.is_empty()).then(|| k.min(s.sources.len() - 1));
                }
            }
            Event::Up => {
                if let Some(k) = s.chosen.filter(|&k| k > 0 && k < s.sources.len()) {
                    s.sources.swap(k, k - 1);
                    s.chosen = Some(k - 1);
                    s.order = Order::Given;
                }
            }
            Event::Down => {
                if let Some(k) = s.chosen.filter(|&k| k + 1 < s.sources.len()) {
                    s.sources.swap(k, k + 1);
                    s.chosen = Some(k + 1);
                    s.order = Order::Given;
                }
            }
            Event::Order(o) => s.order = o,
            Event::Pixel(p) => {
                s.pixel = p;
                s.size.clear();
            }
            Event::Size(t) => s.size = t,
            Event::Nodata(t) => s.nodata = t,
            Event::PickPath => return self.raster_mosaic_path_pick(),
            Event::PathPicked(None) => return Task::none(),
            Event::PathPicked(Some(p)) => {
                let p = if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("vrt")) {
                    p
                } else {
                    p.with_extension("vrt")
                };
                s.overwrite = true;
                s.path = Some(p);
            }
            Event::Add => {
                s.add = !s.add;
                return Task::none();
            }
            Event::Layer(t) => {
                s.layer = t;
                return Task::none();
            }
            Event::Run => {
                self.raster_mosaic_run();
                return Task::none();
            }
            Event::Close => {
                self.rasters.mosaic = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.raster_mosaic_replan();
        Task::none()
    }

    fn raster_mosaic_status(&mut self, why: String) {
        if let Some(s) = &mut self.rasters.mosaic {
            s.status = Some(why);
        }
    }

    /// Oluştur: the command run; with “Katman olarak ekle” a new layer first, both one undo step.
    fn raster_mosaic_run(&mut self) {
        let Some(s) = self.rasters.mosaic.clone() else {
            return;
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        let model = &mut doc.model;
        let group = s.add.then(|| model.begin_group(TITLE));
        let layer = match &group {
            None => None,
            Some(_) => match model.add_layer(NewLayer::layer(s.layer.trim()), None, true) {
                Ok(id) => Some(id),
                Err(why) => {
                    if let Some(g) = group {
                        model.cancel_group(g);
                    }
                    self.raster_mosaic_status(why.0);
                    return;
                }
            },
        };
        let input = match self.raster_mosaic_input(&s, layer) {
            Ok(i) => i,
            Err(why) => {
                if let (Some(g), Some(doc)) = (group, &mut self.document) {
                    doc.model.cancel_group(g);
                }
                self.raster_mosaic_status(why);
                return;
            }
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        let model = &mut doc.model;
        match rasters_mosaic::execute(&mut ExecutionContext::new(model), input) {
            CommandResult::Completed { output, warnings } => {
                if let Some(g) = group {
                    model.end_group(g);
                }
                let slot = output
                    .uid
                    .as_deref()
                    .and_then(|u| kentos_domain::Uuid::parse_str(u).ok())
                    .and_then(|u| model.slot_of(u));
                self.rasters.mosaic = None;
                self.dialog = None;
                if let Some(slot) = slot {
                    self.zoom_to(&[slot]);
                }
                self.say(
                    Level::Success,
                    format!(
                        "Sanal mozaik yazıldı: {} ({} dosya, {} × {} piksel){}. Ctrl+Z katmanı geri alır, dosya kalır.",
                        output.path,
                        output.sources.len(),
                        output.width,
                        output.height,
                        if output.uid.is_some() { format!(", “{}” katmanına eklendi", s.layer.trim()) } else { String::new() }
                    ),
                );
                for w in warnings {
                    self.warn(w.message);
                }
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                if let Some(g) = group {
                    model.cancel_group(g);
                }
                self.raster_mosaic_status(error.message);
            }
            other => {
                if let Some(g) = group {
                    model.cancel_group(g);
                }
                self.raster_mosaic_status(format!("{other:?}"));
            }
        }
    }

    pub(crate) fn raster_mosaic_view(&self) -> Element<'_, Message> {
        let Some(s) = &self.rasters.mosaic else {
            return Column::new().into();
        };
        let format = self
            .document
            .as_ref()
            .map_or_else(kentos_interaction::Format::default, |d| {
                kentos_interaction::Format::of(d.settings())
            });
        let plan = s
            .planned
            .as_ref()
            .and_then(|p| p.as_ref().ok())
            .map(|(m, _)| m);
        let mut body = Column::new().spacing(12);
        // The files, top first: their place on the plan's mosaic.
        let rows: Vec<table::Row<'_, Message>> = s
            .sources
            .iter()
            .enumerate()
            .map(|(k, p)| {
                let used = plan.and_then(|m| {
                    let read = p.display().to_string();
                    m.sources.iter().find(|x| x.read == read)
                });
                let rank = used.map_or_else(|| "—".to_owned(), |u| u.rank.to_string());
                let size = used.map_or_else(String::new, |u| format!("{} × {}", u.width, u.height));
                let pixel = used.map_or_else(String::new, |u| {
                    format!(
                        "{} × {}",
                        format.length_bare(u.pixel[0]),
                        format.length_bare(u.pixel[1])
                    )
                });
                let date = used
                    .and_then(|u| u.date.clone())
                    .map_or_else(|| "—".to_owned(), |d| d.replace('T', " "));
                table::Row::new([
                    label::caption(rank).into(),
                    label::body(name_of(p)).into(),
                    label::caption(size).into(),
                    label::caption(pixel).into(),
                    label::caption(date).into(),
                ])
                .selected(s.chosen == Some(k))
                .on_press(msg(Event::Choose(k)))
            })
            .collect();
        let n = rows.len();
        let list = Table::new([
            table::Column::new("Sıra").width(40).align_right(),
            table::Column::new("Dosya").width(Length::Fill),
            table::Column::new("Boyut").width(84).align_right(),
            table::Column::new(format!("Piksel ({})", format.length_unit_label()))
                .width(92)
                .align_right(),
            table::Column::new("Tarih").width(132),
        ])
        .extend(rows)
        .empty("Dosya yok: Dosya ekle… ile GeoTIFF dosyalarını seçin.");
        let list = words::fitted(list, n);
        let small = |caption: &'static str, on: Option<Message>| {
            button(label::body(caption))
                .on_press_maybe(on)
                .padding(kentos_ui::theme::metrics::action())
                .style(kentos_ui::style::button::secondary)
        };
        let chosen = s.chosen.filter(|&k| k < s.sources.len());
        let tools = row![
            small("Dosya ekle…", Some(msg(Event::AddFiles))),
            small("Kaldır", chosen.map(|_| msg(Event::Remove))),
            small("Yukarı", chosen.filter(|&k| k > 0).map(|_| msg(Event::Up))),
            small(
                "Aşağı",
                chosen
                    .filter(|&k| k + 1 < s.sources.len())
                    .map(|_| msg(Event::Down))
            ),
        ]
        .spacing(6);
        body = body.push(words::field(
            "Dosyalar (Sıra 1 en üstte)",
            Column::new()
                .spacing(6)
                .push(container(list).width(Fill))
                .push(tools),
            None,
        ));
        let order = Segmented::new([Order::Given, Order::Finest, Order::Newest], s.order, |o| {
            msg(Event::Order(o))
        });
        body = body.push(words::field(
            "Örtüşen yerde üstteki",
            order,
            Some(match s.order {
                Order::Given => {
                    "Listenin ilk dosyası üstte; Yukarı ve Aşağı ile sıralayın.".to_owned()
                }
                Order::Finest => "Pikseli küçük olan üstte; eşitlerde listenin sırası.".to_owned(),
                Order::Newest => "TIFF tarihi yeni olan üstte; tarihsizler en altta.".to_owned(),
            }),
        ));
        let input = |placeholder: &'static str, value: &str, on: fn(String) -> Event| {
            kentos_ui::widget::text_field(placeholder, value).on_input(move |t| msg(on(t)))
        };
        let pixel = Segmented::new(
            [Pixel::Highest, Pixel::Lowest, Pixel::Average],
            s.pixel,
            |p| msg(Event::Pixel(p)),
        );
        body = body.push(
            row![
                container(words::field("Mozaiğin pikseli", pixel, None))
                    .width(Length::FillPortion(3)),
                container(words::field(
                    "Piksel boyu",
                    input("Dosyalardan", &s.size, Event::Size),
                    None
                ))
                .width(Length::FillPortion(1)),
                container(words::field(
                    "Nodata",
                    input("Dosyalardan", &s.nodata, Event::Nodata),
                    None
                ))
                .width(Length::FillPortion(1)),
            ]
            .spacing(16),
        );
        let path = s
            .path
            .clone()
            .or_else(|| s.sources.first().map(|f| default_path(f)))
            .map_or_else(|| "—".to_owned(), |p| p.display().to_string());
        body = body.push(words::field(
            "Mozaik dosyası (VRT)",
            row![
                container(label::body(path)).width(Fill),
                small("Kaydet…", (!s.sources.is_empty()).then(|| msg(Event::PickPath))),
            ]
            .spacing(8)
            .align_y(iced::Center),
            Some("Pikseller kopyalanmaz: GDAL'ın VRT dosyası dosyaları yerinde okur; GDAL ve QGIS de açar.".to_owned()),
        ));
        let add = words::check(s.add, "Katman olarak ekle", Some(msg(Event::Add)));
        let layer: Element<'_, Message> = if s.add {
            input("Katmanın adı", &s.layer, Event::Layer).into()
        } else {
            label::muted("Yalnız dosya yazılır.").into()
        };
        body = body.push(
            row![
                container(add).width(Length::FillPortion(1)),
                container(layer).width(Length::FillPortion(1))
            ]
            .spacing(16)
            .align_y(iced::Center),
        );
        // What the plan says.
        let mut lines = Vec::new();
        match &s.planned {
            None => lines.push(words::text_line(Line::Info, "Dosya seçilmedi: Dosya ekle… ile aynı sistemde, kuzeyi yukarıda GeoTIFF dosyalarını seçin.")),
            Some(Err(why)) => lines.push(words::text_line(Line::Error, why.clone())),
            Some(Ok((m, warnings))) => {
                let system = if m.srid == 0 { "sistemsiz".to_owned() } else { crate::crs::title_of(m.srid) };
                lines.push(words::text_line(
                    Line::Ok,
                    format!(
                        "{} dosya, {} × {} piksel, piksel {} × {} {}; {}; {} bant{}.",
                        m.sources.len(),
                        crate::crs::grouped(f64::from(m.width)),
                        crate::crs::grouped(f64::from(m.height)),
                        format.length_bare(m.affine[1]),
                        format.length_bare(-m.affine[5]),
                        format.length_unit_label(),
                        system,
                        m.bands,
                        m.nodata.map_or_else(String::new, |d| format!(", nodata {}", crate::crs::js_number(d)))
                    ),
                ));
                for w in warnings {
                    lines.push(words::text_line(Line::Warn, w.message.clone()));
                }
            }
        }
        if let Some(why) = &s.status {
            lines.push(words::text_line(Line::Error, why.clone()));
        }
        body = body.push(words::summary(lines));
        let can = matches!(&s.planned, Some(Ok(_))) && (!s.add || !s.layer.trim().is_empty());
        overlay::blocking(
            Dialog::new(TITLE)
                .scroll(body)
                .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
                .action(words::primary("Oluştur", can.then(|| msg(Event::Run))))
                .width(720.0)
                .max_height(800.0)
                .on_close(Some(msg(Event::Close))),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::{app_with_drawing, drive};

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/raster/v1/mosaic")
            .join(name)
    }

    fn send(app: &mut App, e: Event) {
        let task = app.update(Message::Rasters(Rasters::Mosaic(e)));
        drive(app, task);
    }

    /// A drawing in the files' system (TUREF / TM36).
    fn app() -> App {
        let mut app = app_with_drawing();
        let doc = app.document.as_mut().expect("a drawing");
        let mut settings = doc.settings().clone();
        settings.srid = 5256;
        doc.model.set_settings(settings);
        app
    }

    fn rasters(app: &App) -> Vec<kentos_commands::RasterEntity> {
        app.document
            .as_ref()
            .expect("a drawing")
            .model
            .entities()
            .filter_map(|e| match e {
                kentos_commands::Entity::Raster(r) => Some(r.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_window_plans_writes_and_adds_the_mosaic_as_one_step() {
        let mut app = app();
        app.picker = Picker::File(fixture("dem-a.tif"));
        let task = app.run("raster.mosaic");
        drive(&mut app, task);
        app.picker = Picker::File(fixture("dem-c.tif"));
        send(&mut app, Event::AddFiles);
        let s = app.rasters.mosaic.as_ref().expect("the window");
        assert_eq!(s.sources.len(), 2);
        let (m, warnings) = s.planned.clone().expect("a plan").expect("the plan");
        assert_eq!((m.width, m.height), (80, 60));
        assert_eq!(warnings[0].code, "mosaic_resampled");
        // Aşağı moves the first file under the second: dem-c (5 m) on top.
        send(&mut app, Event::Choose(0));
        send(&mut app, Event::Down);
        let s = app.rasters.mosaic.as_ref().expect("the window");
        let (m, _) = s.planned.clone().expect("a plan").expect("the plan");
        assert!(m.sources[0].read.ends_with("dem-c.tif"));
        let out = std::env::temp_dir().join(format!("kentos-mozaik-test-{}", std::process::id()));
        std::fs::create_dir_all(&out).unwrap();
        send(&mut app, Event::PathPicked(Some(out.join("dem"))));
        send(&mut app, Event::Run);
        assert!(app.rasters.mosaic.is_none(), "the window closes");
        let vrt = out.join("dem.vrt");
        assert!(vrt.is_file(), "the VRT is written, its extension given");
        let added = rasters(&app);
        assert_eq!(added.len(), 1);
        assert_eq!(
            added[0].raster.file.as_deref(),
            Some(vrt.display().to_string().as_str())
        );
        assert!(app.document.as_ref().unwrap().model.can_undo());
        // One step takes back the layer and the raster; the file stays.
        app.document.as_mut().unwrap().model.undo();
        assert!(rasters(&app).is_empty());
        assert!(vrt.is_file());
        let _ = std::fs::remove_dir_all(&out);
    }

    #[test]
    fn a_refused_mosaic_says_why_and_cannot_be_made() {
        let mut app = app();
        app.picker = Picker::File(fixture("dem-a.tif"));
        let task = app.run("raster.mosaic");
        drive(&mut app, task);
        app.picker = Picker::File(fixture("baska-sistem.tif"));
        send(&mut app, Event::AddFiles);
        let s = app.rasters.mosaic.as_ref().expect("the window");
        let why = s.planned.clone().expect("a plan").expect_err("refused");
        assert!(why.contains("EPSG:5255"), "{why}");
    }
}
