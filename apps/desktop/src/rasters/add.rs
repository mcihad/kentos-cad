//! Raster ekle (docs/adr/0204 §1, §8; the web's `ui/raster/RasterAddDialog.ts`):
//! a GeoTIFF, TIFF, PNG or JPEG chosen, its header read off the interface's
//! thread (a TIFF's directories only; a PNG or JPEG decoded whole) with the
//! world file beside it; the window shows its size, bands, samples, how it
//! is stored, its levels, what places it, its system and nodata, and what
//! the rule of the systems says (`kentos_formats::raster::place`): the
//! project's system, the user's yes for an unknown one, a refusal for
//! another, the middle of the view for an unplaced raster. Ekle writes a
//! new layer named after the file and the raster on it as one undo step
//! (Raster ekle), linked (its path) or embedded (its bytes in the project's
//! library, at most 32 MB), and shows it.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::{Column, column, container, row, text_input};
use iced::{Element, Fill, Task};
use kentos_contracts::{
    CommandResult, CreateOperation, EntitiesCreate, EntityGeometry, NewObject, RasterFields,
    RasterStyle,
};
use kentos_domain::NewLayer;
use kentos_formats::raster::place::{self, Rule};
use kentos_formats::raster::source::RasterInfo;
use kentos_interaction::{Format, Level};
use kentos_native_application::{ExecutionContext, create};
use kentos_ui::label;
use kentos_ui::widget::pairs::Pairs;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Dialog, overlay};

use super::Event as Rasters;
use crate::app::{App, Dialog as Asking, Message, Picker};
use crate::exchange::words::{self, Kind as Line};

pub const TITLE: &str = "Raster ekle";

static READS: AtomicU64 = AtomicU64::new(0);

/// What the file's header said.
#[derive(Debug, Clone)]
pub struct Read {
    pub info: RasterInfo,
    pub style: RasterStyle,
    /// The world file that placed it, by its name.
    pub world: Option<String>,
    pub bytes: u64,
}

/// How the raster is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    Linked,
    Embedded,
}

impl std::fmt::Display for Keep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Keep::Linked => "Bağlı",
            Keep::Embedded => "Göm",
        })
    }
}

#[derive(Debug, Clone)]
pub struct State {
    pub path: PathBuf,
    pub name: String,
    reading: u64,
    pub read: Option<Result<Read, String>>,
    /// The user said an unknown system is the project's.
    pub confirmed: bool,
    pub keep: Keep,
    pub layer: String,
    status: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The file to read, or none (the dialog cancelled).
    Picked(Option<PathBuf>),
    /// Boxed: what the header said is the largest event.
    Read(u64, Box<Result<Read, String>>),
    Confirm,
    Keep(Keep),
    Layer(String),
    Another,
    Run,
    Close,
}

fn msg(e: Event) -> Message {
    Message::Rasters(Rasters::Add(e))
}

/// The world file beside `path`, as GDAL looks for it: its name and text.
fn world_beside(path: &Path) -> Option<(String, String)> {
    let ext = path.extension()?.to_string_lossy().into_owned();
    for w in kentos_formats::raster::world::extensions(&ext) {
        for candidate in [
            path.with_extension(&w),
            path.with_extension(w.to_uppercase()),
        ] {
            if let Ok(text) = std::fs::read_to_string(&candidate) {
                let name = candidate
                    .file_name()
                    .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                return Some((name, text));
            }
        }
    }
    None
}

/// The header read: what the window shows, its look, the world file that placed it.
pub fn inspect(path: &Path) -> Result<Read, String> {
    let bytes = std::fs::metadata(path)
        .map_err(|e| format!("“{}” okunamadı: {e}.", path.display()))?
        .len();
    let opened = super::tiles::open(&super::tiles::Origin::File(path.to_path_buf()))?;
    let reader = opened
        .reader
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut info = reader.info.clone();
    let mut world = None;
    if info.affine.is_none()
        && let Some((name, text)) = world_beside(path)
    {
        let affine = kentos_formats::raster::world::read(&text)
            .map_err(|e| format!("Dünya dosyası “{name}” okunamadı: {}.", e.0))?;
        info.affine = Some(affine);
        info.placed_by = "world".to_owned();
        world = Some(name);
    }
    let style = kentos_formats::raster::style::default_style(
        info.bands,
        info.sample,
        reader.palette.is_some(),
    );
    Ok(Read {
        info,
        style,
        world,
        bytes,
    })
}

/// The file's size as the window says it.
fn size_words(bytes: u64) -> String {
    if bytes >= 1 << 20 {
        format!(
            "{} MB",
            kentos_interaction::fixed(bytes as f64 / f64::from(1 << 20), 1).replace('.', ",")
        )
    } else {
        format!("{} KB", (bytes / 1024).max(1))
    }
}

/// How a raster's samples are named.
fn colour_words(info: &RasterInfo) -> String {
    let colour = match info.color.as_str() {
        "rgb" => "RGB",
        "palette" => "paletli",
        "ycbcr" => "YCbCr (JPEG)",
        _ => "gri",
    };
    let alpha = if info.alpha { ", alfa bandıyla" } else { "" };
    format!(
        "{} bant, {}, {colour}{alpha}",
        info.bands,
        info.sample.label()
    )
}

/// How a TIFF is stored.
fn storage_words(info: &RasterInfo) -> String {
    if info.compression.is_empty() {
        return "Tek parça resim".to_owned();
    }
    let blocks = if info.tiled { "karolu" } else { "şeritli" };
    let big = if info.big { ", BigTIFF" } else { "" };
    format!("{}, {blocks}{big}", info.compression)
}

impl App {
    /// `raster.add`: the file asked for, then the window.
    pub(crate) fn raster_add_command(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        }
        self.raster_add_pick()
    }

    fn raster_add_pick(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(msg(Event::Picked(Some(path.clone()))));
        }
        Task::perform(
            async {
                rfd::AsyncFileDialog::new()
                    .set_title(TITLE)
                    .add_filter(
                        "Raster (GeoTIFF, TIFF, PNG, JPEG)",
                        &["tif", "tiff", "png", "jpg", "jpeg"],
                    )
                    .pick_file()
                    .await
                    .map(|f| f.path().to_path_buf())
            },
            |path| msg(Event::Picked(path)),
        )
    }

    pub(crate) fn raster_add_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Picked(None) => return Task::none(),
            Event::Picked(Some(path)) => {
                let id = READS.fetch_add(1, Ordering::Relaxed) + 1;
                let name = path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned(),
                );
                let layer = words::base_name(&name);
                self.rasters.add = Some(State {
                    path: path.clone(),
                    name,
                    reading: id,
                    read: None,
                    confirmed: false,
                    keep: Keep::Linked,
                    layer,
                    status: None,
                });
                self.dialog = Some(Asking::RasterAdd);
                return super::off_thread(move || msg(Event::Read(id, Box::new(inspect(&path)))));
            }
            Event::Read(id, read) => {
                let read = *read;
                if let Some(s) = &mut self.rasters.add
                    && s.reading == id
                {
                    if read
                        .as_ref()
                        .is_ok_and(|r| r.bytes > super::MOST_EMBEDDED as u64)
                    {
                        s.keep = Keep::Linked;
                    }
                    s.read = Some(read);
                }
            }
            Event::Confirm => {
                if let Some(s) = &mut self.rasters.add {
                    s.confirmed = !s.confirmed;
                }
            }
            Event::Keep(k) => {
                if let Some(s) = &mut self.rasters.add {
                    s.keep = k;
                }
            }
            Event::Layer(t) => {
                if let Some(s) = &mut self.rasters.add {
                    s.layer = t;
                }
            }
            Event::Another => return self.raster_add_pick(),
            Event::Run => self.raster_add_run(),
            Event::Close => {
                self.rasters.add = None;
                if self.dialog == Some(Asking::RasterAdd) {
                    self.dialog = None;
                }
            }
        }
        Task::none()
    }

    /// The project's system: its SRID and how a sentence names it.
    fn raster_project(&self) -> (u32, String) {
        let Some(doc) = &self.document else {
            return (0, String::new());
        };
        let srid = doc.settings().srid;
        let name = kentos_project::systems::own(doc.settings())
            .map_or_else(|| "Yerel (koordinat sistemi yok)".to_owned(), |n| n.title);
        (srid, name)
    }

    /// Where the raster goes, when it goes.
    fn raster_add_placement(&self, s: &State) -> Option<([f64; 6], u32)> {
        let Some(Ok(r)) = &s.read else {
            return None;
        };
        let b = self.viewport.camera.visible_bounds();
        place::placement(
            &r.info,
            self.raster_project().0,
            s.confirmed,
            [b.min_x, b.min_y, b.max_x, b.max_y],
        )
    }

    fn raster_add_can(&self, s: &State) -> bool {
        self.raster_add_placement(s).is_some() && !s.layer.trim().is_empty()
    }

    /// Ekle: the new layer and the raster on it, one undo step; embedded first into the library.
    fn raster_add_run(&mut self) {
        let Some(s) = self.rasters.add.clone() else {
            return;
        };
        let (Some((affine, srid)), Some(Ok(read))) =
            (self.raster_add_placement(&s), s.read.clone())
        else {
            return;
        };
        let (mut asset, mut file) = (None, Some(s.path.to_string_lossy().into_owned()));
        let mut library = None;
        if s.keep == Keep::Embedded {
            let bytes = match std::fs::read(&s.path) {
                Ok(b) => b,
                Err(e) => {
                    self.raster_add_status(format!("“{}” okunamadı: {e}.", s.name));
                    return;
                }
            };
            match super::library_item(&s.name, &bytes, read.info.width, read.info.height) {
                Ok((id, item)) => {
                    asset = Some(id.clone());
                    file = None;
                    library = Some((id, item));
                }
                Err(why) => {
                    self.raster_add_status(why);
                    return;
                }
            }
        }
        let fields = RasterFields {
            affine,
            width: read.info.width,
            height: read.info.height,
            bands: read.info.bands,
            sample: read.info.sample,
            asset,
            file,
            srid,
            style: read.style.clone(),
            opacity: None,
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        let model = &mut doc.model;
        if let Some((id, item)) = library {
            super::keep_in_library(model, &id, item);
        }
        let group = model.begin_group(TITLE);
        let layer = match model.add_layer(NewLayer::layer(s.layer.trim()), None, true) {
            Ok(id) => id,
            Err(why) => {
                model.cancel_group(group);
                self.raster_add_status(why.0);
                return;
            }
        };
        let input = EntitiesCreate {
            layer_id: layer,
            objects: vec![NewObject {
                geometry: EntityGeometry::Raster(fields),
                color: None,
                line_weight: None,
                attrs: None,
                label: None,
                label_of: None,
                label_scale: None,
                symbol: None,
            }],
            operation: Some(CreateOperation::Raster),
            expected_revision: None,
        };
        let result = create::execute(&mut ExecutionContext::new(model), input);
        match result {
            CommandResult::Completed { output, warnings } => {
                model.end_group(group);
                let slots: Vec<kentos_domain::Slot> =
                    output.ids.iter().map(|&i| kentos_domain::Slot(i)).collect();
                self.rasters.add = None;
                self.dialog = None;
                self.zoom_to(&slots);
                let how = if s.keep == Keep::Embedded {
                    "gömülü"
                } else {
                    "bağlı"
                };
                self.say(
                    Level::Success,
                    format!(
                        "“{}” {how} raster olarak “{}” katmanına eklendi. Ctrl+Z geri alır.",
                        s.name,
                        s.layer.trim()
                    ),
                );
                for w in warnings {
                    self.warn(w.message);
                }
                if read.info.affine.is_none() {
                    self.say(
                        Level::Info,
                        format!("“{}” oturtulmamış: Raster oturt ile kontrol noktalarından yerine oturtun.", s.name),
                    );
                }
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                model.cancel_group(group);
                self.raster_add_status(error.message);
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => {
                model.cancel_group(group);
                self.raster_add_status("Raster yazılamadı.".to_owned());
            }
        }
    }

    fn raster_add_status(&mut self, why: String) {
        if let Some(s) = &mut self.rasters.add {
            s.status = Some(why);
        }
    }

    pub(crate) fn raster_add_view(&self) -> Element<'_, Message> {
        let Some(s) = &self.rasters.add else {
            return iced::widget::text("").into();
        };
        let meta = match &s.read {
            None => "okunuyor…".to_owned(),
            Some(Err(_)) => "okunamadı".to_owned(),
            Some(Ok(r)) => format!(
                "{} × {} piksel, {}",
                crate::crs::grouped(f64::from(r.info.width)),
                crate::crs::grouped(f64::from(r.info.height)),
                size_words(r.bytes)
            ),
        };
        let mut body = Column::new()
            .spacing(12)
            .push(words::file_line(&s.name, meta));
        match &s.read {
            None => {
                body = body.push(words::summary(vec![words::text_line(
                    Line::Info,
                    "Dosyanın başlığı okunuyor…",
                )]))
            }
            Some(Err(why)) => {
                body = body.push(words::summary(vec![words::text_line(
                    Line::Error,
                    why.clone(),
                )]))
            }
            Some(Ok(r)) => {
                body = body.push(self.raster_add_facts(r));
                body = body.push(self.raster_add_rule(s, r));
                body = body.push(self.raster_add_options(s, r));
            }
        }
        if let Some(why) = &s.status {
            body = body.push(words::text_line(Line::Error, why.clone()));
        }
        overlay::blocking(
            Dialog::new(TITLE)
                .scroll(body)
                .action(words::ghost("Başka dosya…", Some(msg(Event::Another))))
                .action(words::secondary("Vazgeç", Some(msg(Event::Close))))
                .action(words::primary(
                    "Ekle",
                    self.raster_add_can(s).then(|| msg(Event::Run)),
                ))
                .width(640.0)
                .max_height(760.0),
        )
    }

    fn raster_add_facts<'a>(&self, r: &'a Read) -> Element<'a, Message> {
        let i = &r.info;
        let format = self
            .document
            .as_ref()
            .map_or_else(Format::default, |d| Format::of(d.settings()));
        let levels = if i.needs_pyramid {
            format!(
                "{} kat; dosyada önizleme yok: ilk açılışta önizleme piramidi bir kez hazırlanır",
                i.levels
            )
        } else if i.overviews > 0 {
            format!("{} kat; {} önizleme dosyada", i.levels, i.overviews)
        } else {
            format!("{} kat", i.levels)
        };
        let placed = match (i.placed_by.as_str(), &r.world) {
            ("geotiff", _) => "GeoTIFF etiketleri".to_owned(),
            ("world", Some(w)) => format!("Dünya dosyası ({w})"),
            ("world", None) => "Dünya dosyası".to_owned(),
            _ => "Yok (oturtulmamış)".to_owned(),
        };
        let pixel = match i.affine {
            Some([_, a, b, _, c, d]) => format!(
                "{} × {} {}",
                format.length_bare(kentos_geometry_core::jsmath::js_hypot(a, c)),
                format.length_bare(kentos_geometry_core::jsmath::js_hypot(b, d)),
                format.length_unit_label()
            ),
            None => "—".to_owned(),
        };
        let system = match i.epsg {
            Some(srid) => crate::crs::title_of(srid),
            None if i.affine.is_some() => "Dosya söylemiyor".to_owned(),
            None => "—".to_owned(),
        };
        let nodata = i
            .nodata
            .map_or_else(|| "Yok".to_owned(), crate::crs::js_number);
        let pairs = Pairs::new()
            .push(label::caption("Bantlar"), label::body(colour_words(i)))
            .push(label::caption("Saklama"), label::body(storage_words(i)))
            .push(label::caption("Katlar"), label::body(levels))
            .push(label::caption("Konum"), label::body(placed))
            .push(label::caption("Piksel boyu"), label::body(pixel))
            .push(label::caption("Sistem"), label::body(system))
            .push(label::caption("Nodata"), label::body(nodata));
        container(pairs)
            .padding([8, 10])
            .width(Fill)
            .style(kentos_ui::style::container::bordered)
            .into()
    }

    fn raster_add_rule<'a>(&self, s: &'a State, r: &'a Read) -> Element<'a, Message> {
        let (srid, project) = self.raster_project();
        let line = match place::rule(&r.info, srid) {
            Rule::Same { .. } => words::text_line(
                Line::Ok,
                format!("Rasterin sistemi projeninkiyle aynı: {project}. Dosyanın dediği yere eklenir."),
            ),
            Rule::Other { srid: other } => words::text_line(
                Line::Error,
                format!(
                    "Rasterin sistemi ({}) projeninkinden ({project}) başka; raster eklenmez. KentOS rasteri yeniden izdüşürmez: rasteri projenin sistemine çevirip yeniden deneyin.",
                    crate::crs::title_of(other)
                ),
            ),
            Rule::Unknown => words::line(
                Line::Warn,
                column![
                    label::body("Dosya koordinat sistemini söylemiyor. Raster ancak projenin sisteminde olduğu söylenirse eklenir."),
                    words::check(
                        s.confirmed,
                        format!("Raster projenin sisteminde ({project})"),
                        Some(msg(Event::Confirm)),
                    ),
                ]
                .spacing(6),
            ),
            Rule::Unplaced => words::text_line(
                Line::Info,
                "Rasterin konumu yok: görünümün ortasına, pikseli görünümün kısa kenarının binde biri olarak oturtulmamış eklenir; Raster oturt ile kontrol noktalarından yerine oturtulur.",
            ),
        };
        words::summary(vec![line])
    }

    fn raster_add_options<'a>(&self, s: &'a State, r: &'a Read) -> Element<'a, Message> {
        let large = r.bytes > super::MOST_EMBEDDED as u64;
        let keep = Segmented::new_with(
            [Keep::Linked, Keep::Embedded],
            s.keep,
            |k| msg(Event::Keep(k)),
            move |k| k == Keep::Linked || !large,
        );
        let hint = if large {
            format!(
                "Dosya {}; gömülü raster en çok {} MB olabilir, bağlı kalır.",
                size_words(r.bytes),
                super::MOST_EMBEDDED >> 20
            )
        } else if s.keep == Keep::Embedded {
            "Dosyanın baytları projenin kitaplığına alınır; çizim dosyayla birlikte taşınır."
                .to_owned()
        } else {
            "Çizim dosyanın yolunu tutar; dosya yerinden oynarsa raster bulunamaz.".to_owned()
        };
        let name = kentos_ui::widget::focus_ring(
            text_input("Katmanın adı", &s.layer)
                .on_input(|t| msg(Event::Layer(t)))
                .on_submit(msg(Event::Run))
                .padding([5, 8])
                .size(kentos_ui::theme::typography::body())
                .style(kentos_ui::style::field::input),
        );
        row![
            container(words::field("Kaynak", keep, Some(hint))).width(iced::Length::FillPortion(3)),
            container(words::field(
                "Yeni katman",
                name,
                Some("Raster kendi katmanında durur.".to_owned())
            ))
            .width(iced::Length::FillPortion(2)),
        ]
        .spacing(16)
        .into()
    }
}
