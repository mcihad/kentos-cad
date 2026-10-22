//! Konumlu fotoğraflar (docs/adr/0225 §3; the web's `ui/io/PhotoImportDialog.ts`):
//! JPEG photos' EXIF read off the UI thread by the shared reader
//! (`kentos_formats::exif`), each place moved from WGS 84 into the project's
//! system the way Koordinat dönüştür moves it (the rules are
//! `kentos_interaction::photos`); the photos show as they will be written,
//! those without a place or out of the system's reach with why. Each photo a
//! point with its attributes and itself as its attachment: embedded in the
//! project's library (Gömülü), or linked to its file, relative to the
//! drawing's folder when it is under it. One undo step “Konumlu
//! fotoğraflar”, onto a new layer or one of the drawing's. A project without
//! a system takes none (CLAUDE.md §5).

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use iced::widget::{Column, column, container, row, text_input};
use iced::{Element, Fill, Length, Task};
use kentos_commands::{Bounds, LineError, PhotoExif};
use kentos_geometry_core::crs::format_dd;
use kentos_interaction::Level;
use kentos_interaction::photos::{self, Photo, Placed, Plan};
use kentos_project::systems::{self, Named};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::table::{Column as TableColumn, Row as TableRow, Table};
use kentos_ui::widget::{Dialog, overlay};
use serde_json::json;

use super::apply::{self, ImportPlan, LayerTarget, layer_named};
use super::coord_import::point_layer_style;
use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message, off_thread};
use crate::app::{App, Message, Picker};

/// The window's title and the undo step's name.
pub const TITLE: &str = "Konumlu fotoğraflar";

/// Rows of the preview table.
const PREVIEW_ROWS: usize = 12;

/// Problems shown in the summary; the rest are counted.
const SHOWN_PROBLEMS: usize = 5;

static READS: AtomicU64 = AtomicU64::new(0);

/// A photo chosen: its file's name, where it is and its bytes.
#[derive(Debug, Clone)]
pub struct PhotoFile {
    pub name: String,
    pub path: PathBuf,
    pub bytes: Arc<[u8]>,
}

/// A library image's id, format and size in pixels.
#[derive(Debug, Clone)]
struct Image {
    id: String,
    format: String,
    width: u64,
    height: u64,
}

/// A photo as the window read it: what its EXIF says, and its image when it
/// is one the drawing shows (else the EXIF's place is dropped with why: the
/// web's rule).
#[derive(Debug, Clone)]
pub struct Read {
    exif: PhotoExif,
    image: Option<Image>,
}

#[derive(Debug, Clone)]
pub struct State {
    files: Vec<PhotoFile>,
    read: Option<Arc<Vec<Read>>>,
    reading_id: u64,
    /// Gömülü: the photos into the project's library; else linked to their files.
    embed: bool,
    /// The target layer's id; `None`: a new layer named `name`.
    target: Option<String>,
    name: String,
    status: Option<(bool, String)>,
    plan: Option<Arc<Plan>>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The photos chosen; none when the dialog was cancelled.
    Picked(Option<Vec<PhotoFile>>),
    Read {
        id: u64,
        read: Arc<Vec<Read>>,
    },
    Embed,
    Target(Option<String>),
    Name(String),
    Another,
    Run,
}

fn event(e: Event) -> Message {
    message(Exchange::PhotoImport(e))
}

/// One photo read: its EXIF and whether its image is one the drawing shows.
fn read_one(file: &PhotoFile) -> Read {
    let mut exif = kentos_formats::exif::read(&file.bytes);
    let image = match crate::pictures::library_item(&file.name, &file.bytes) {
        Ok((id, item)) => Some(Image {
            id,
            format: item["format"].as_str().unwrap_or("jpeg").to_owned(),
            width: item["width"].as_u64().unwrap_or(0),
            height: item["height"].as_u64().unwrap_or(0),
        }),
        Err(why) => {
            if exif.latitude.is_some() {
                exif.latitude = None;
                exif.longitude = None;
                exif.problem = Some(why);
            }
            None
        }
    };
    Read { exif, image }
}

/// Reads the photos off the UI thread.
fn read_all(id: u64, files: Vec<PhotoFile>) -> Task<Message> {
    off_thread(
        move || files.iter().map(read_one).collect::<Vec<_>>(),
        move |read| {
            Exchange::PhotoImport(Event::Read {
                id,
                read: Arc::new(read),
            })
        },
    )
}

/// Megabytes as the web writes them (`toLocaleString('tr-TR', { maximumFractionDigits: 1 })`):
/// “0”, “1,5”, “1.234,5”.
fn megabytes(bytes: usize) -> String {
    let tenths = (bytes as f64 / (1024.0 * 1024.0) * 10.0).round() as u64;
    let (whole, tenth) = (tenths / 10, tenths % 10);
    let digits = whole.to_string();
    let mut grouped = String::new();
    for (i, d) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            grouped.push('.');
        }
        grouped.push(d);
    }
    if tenth == 0 {
        grouped
    } else {
        format!("{grouped},{tenth}")
    }
}

/// The extent of the placed photos.
fn bounds(placed: &[Placed]) -> Bounds {
    let mut b = Bounds {
        min_x: f64::INFINITY,
        min_y: f64::INFINITY,
        max_x: f64::NEG_INFINITY,
        max_y: f64::NEG_INFINITY,
    };
    for p in placed {
        b.min_x = b.min_x.min(p.p.x);
        b.min_y = b.min_y.min(p.p.y);
        b.max_x = b.max_x.max(p.p.x);
        b.max_y = b.max_y.max(p.p.y);
    }
    b
}

impl App {
    /// Konumlu fotoğraflar (`file.import.photos`): asks for the photos, straight from the command.
    pub(super) fn pick_photos(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            let path = path.clone();
            return Task::perform(
                async move {
                    let bytes = std::fs::read(&path).ok()?;
                    let name = path.file_name()?.to_string_lossy().into_owned();
                    Some(vec![PhotoFile {
                        name,
                        path,
                        bytes: bytes.into(),
                    }])
                },
                |files| event(Event::Picked(files)),
            );
        }
        Task::perform(
            async {
                let picked = rfd::AsyncFileDialog::new()
                    .set_title(TITLE)
                    .add_filter("Fotoğraf (JPEG)", &["jpg", "jpeg"])
                    .pick_files()
                    .await?;
                let mut files = Vec::with_capacity(picked.len());
                for file in picked {
                    let name = file.file_name();
                    let path = file.path().to_path_buf();
                    let bytes = file.read().await;
                    files.push(PhotoFile {
                        name,
                        path,
                        bytes: bytes.into(),
                    });
                }
                Some(files)
            },
            |files| event(Event::Picked(files)),
        )
    }

    /// The window over `files`, read off the thread.
    pub(crate) fn photo_import_files(&mut self, files: Vec<PhotoFile>) -> Task<Message> {
        if files.is_empty() {
            return Task::none();
        }
        let (target, embed) = match &self.exchange {
            Some(Window::PhotoImport(s)) => (s.target.clone(), s.embed),
            _ => (None, false),
        };
        let id = READS.fetch_add(1, Ordering::Relaxed) + 1;
        let task = read_all(id, files.clone());
        self.open_window(Window::PhotoImport(Box::new(State {
            files,
            read: None,
            reading_id: id,
            embed,
            target,
            name: "Fotoğraflar".to_owned(),
            status: None,
            plan: None,
        })));
        task
    }

    pub(super) fn photo_import_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Picked(None) => return Task::none(),
            Event::Picked(Some(files)) => return self.photo_import_files(files),
            Event::Another => return self.pick_photos(),
            Event::Run => {
                self.photo_import_run();
                return Task::none();
            }
            _ => {}
        }
        let Some(Window::PhotoImport(s)) = &mut self.exchange else {
            return Task::none();
        };
        let mut again = true;
        match e {
            Event::Read { id, read } => {
                if id != s.reading_id {
                    return Task::none();
                }
                s.read = Some(read);
            }
            Event::Embed => s.embed = !s.embed,
            Event::Target(t) => {
                s.target = t;
                again = false;
            }
            Event::Name(name) => {
                s.name = name;
                again = false;
            }
            Event::Picked(_) | Event::Another | Event::Run => {}
        }
        if again {
            self.photo_place();
        }
        Task::none()
    }

    /// The project's own system, when it has one (docs/adr/0168 §1).
    fn photo_system(&self) -> Option<Named> {
        systems::own(self.document.as_ref()?.settings())
    }

    /// A photo's link: relative to the drawing's folder when it is under it,
    /// else its whole path (docs/adr/0225 §1).
    fn photo_link(&self, path: &std::path::Path) -> String {
        let folder = self
            .document
            .as_ref()
            .and_then(|d| d.path.as_deref())
            .and_then(std::path::Path::parent);
        folder.and_then(|f| path.strip_prefix(f).ok()).map_or_else(
            || path.to_string_lossy().into_owned(),
            |r| r.to_string_lossy().replace('\\', "/"),
        )
    }

    /// The photos placed again with the current choices.
    fn photo_place(&mut self) {
        let system = self.photo_system();
        let choices = self
            .document
            .as_ref()
            .map(|d| systems::choices(d.settings()))
            .unwrap_or_default();
        let Some(Window::PhotoImport(s)) = &self.exchange else {
            return;
        };
        let plan = match (&s.read, system) {
            (
                Some(read),
                Some(Named {
                    name,
                    system: Some(to),
                    ..
                }),
            ) => {
                let photos: Vec<Photo> = s
                    .files
                    .iter()
                    .zip(read.iter())
                    .map(|(f, r)| Photo {
                        name: f.name.clone(),
                        exif: r.exif.clone(),
                        asset: if s.embed {
                            r.image.as_ref().map(|i| i.id.clone())
                        } else {
                            None
                        },
                        file: (!s.embed).then(|| self.photo_link(&f.path)),
                    })
                    .collect();
                Some(Arc::new(photos::place(&photos, &to, &name, &choices)))
            }
            _ => None,
        };
        if let Some(Window::PhotoImport(s)) = &mut self.exchange {
            s.plan = plan;
        }
    }

    /// Where the points go: an existing layer, or a new one (merged into a
    /// layer of the same name if there is one).
    fn photo_target(&self, s: &State) -> Option<LayerTarget> {
        if let Some(id) = &s.target {
            return Some(LayerTarget::Existing(id.clone()));
        }
        let name = s.name.trim();
        if name.is_empty() {
            return None;
        }
        let doc = self.document.as_ref()?;
        Some(match layer_named(&doc.model, name) {
            Some(same) => LayerTarget::Existing(same.id.clone()),
            None => LayerTarget::New {
                frozen: None,
                plot: None,
                name: name.to_owned(),
                style: Box::new(point_layer_style()),
                visible: true,
                locked: false,
                fields: Vec::new(),
            },
        })
    }

    /// Why the import cannot run now, if a reason must be said (a locked layer).
    fn photo_locked(&self, s: &State) -> Option<String> {
        let Some(LayerTarget::Existing(id)) = self.photo_target(s) else {
            return None;
        };
        let doc = self.document.as_ref()?;
        doc.model.layers().is_locked(&id).then(|| {
            format!(
                "“{}” katmanı kilitli; kilidini Katmanlar panelinden açın ya da başka bir katman seçin.",
                self.layer_name(&id)
            )
        })
    }

    fn photo_can_import(&self, s: &State) -> bool {
        s.read.is_some()
            && s.plan.as_ref().is_some_and(|p| !p.placed.is_empty())
            && self.photo_target(s).is_some()
            && self.photo_locked(s).is_none()
    }

    fn photo_import_run(&mut self) {
        let Some(Window::PhotoImport(s)) = &self.exchange else {
            return;
        };
        if !self.photo_can_import(s) {
            return;
        }
        let (Some(target), Some(plan), Some(read)) =
            (self.photo_target(s), s.plan.clone(), s.read.clone())
        else {
            return;
        };
        // The placed photos' images in the project's library first (an edit, no undo step:
        // docs/adr/0092), each once, under Ekler.
        let mut images = Vec::new();
        if s.embed {
            for p in &plan.placed {
                let k = p.index as usize - 1;
                let (
                    Some(f),
                    Some(Read {
                        image: Some(im), ..
                    }),
                ) = (s.files.get(k), read.get(k))
                else {
                    continue;
                };
                let item = json!({
                    "kind": "asset",
                    "id": im.id,
                    "name": photos::stem(&f.name),
                    "path": [crate::attachments::PHOTO_PATH],
                    "format": im.format,
                    "data": format!("data:image/{};base64,{}", im.format, crate::style::manager::files::base64(&f.bytes)),
                    "width": im.width,
                    "height": im.height,
                });
                images.push((im.id.clone(), item));
            }
        }
        let skipped = plan.skipped.len();
        let layer_name = match &target {
            LayerTarget::New { name, .. } => name.clone(),
            LayerTarget::Existing(id) => self.layer_name(id),
        };
        let import = ImportPlan {
            source_srid: None,
            label: TITLE.to_owned(),
            layers: vec![(String::new(), target)],
            group: None,
        };
        let Some(doc) = &mut self.document else {
            return;
        };
        for (id, item) in images {
            crate::pictures::keep_in_library(&mut doc.model, &id, item);
        }
        match apply::apply_import(
            &mut doc.model,
            photos::entities(&plan.placed),
            Vec::new(),
            &import,
        ) {
            Err(error) => {
                if let Some(Window::PhotoImport(s)) = &mut self.exchange {
                    s.status = Some((true, error));
                }
            }
            Ok(applied) => {
                self.zoom_to(&applied.slots);
                self.say(
                    Level::Success,
                    format!(
                        "{} konumlu fotoğraf “{layer_name}” katmanına nokta olarak alındı{}. Tek adımda geri alınabilir.",
                        plan.placed.len(),
                        if applied.created.is_empty() { "" } else { " (yeni katman)" }
                    ),
                );
                if skipped > 0 {
                    self.warn(format!(
                        "{skipped} fotoğraf alınmadı (konumu yok ya da projenin sistemine çevrilemiyor); nedenleri pencerede yazılıydı."
                    ));
                }
                self.close_exchange();
            }
        }
    }

    pub(super) fn photo_import_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let bytes: usize = s.files.iter().map(|f| f.bytes.len()).sum();
        let what = if s.files.len() == 1 {
            s.files[0].name.clone()
        } else {
            format!("{} fotoğraf", s.files.len())
        };
        let meta = format!(
            "{} MB{}",
            megabytes(bytes),
            if s.read.is_none() {
                ", okunuyor…"
            } else {
                ""
            }
        );
        let mut body = Column::new()
            .spacing(12)
            .push(words::file_line(&what, meta))
            .push(self.photo_table(s))
            .push(self.photo_summary(s))
            .push(self.photo_layer(s));
        let status = self
            .photo_locked(s)
            .map(|why| (true, why))
            .or_else(|| s.status.clone());
        if let Some((error, text)) = status {
            body = body.push(words::text_line(
                if error { Line::Error } else { Line::Info },
                text,
            ));
        }
        let can = self.photo_can_import(s);
        overlay::blocking(
            Dialog::new(TITLE)
                .scroll(body)
                .action(words::ghost(
                    "Başka fotoğraflar…",
                    Some(event(Event::Another)),
                ))
                .action(words::secondary("Vazgeç", Some(message(Exchange::Close))))
                .action(words::primary("İçe aktar", can.then(|| event(Event::Run))))
                .width(960.0)
                .max_height(820.0),
        )
    }

    fn photo_table<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(read) = &s.read else {
            return words::empty("Fotoğraflar okunuyor…");
        };
        let Some(plan) = &s.plan else {
            return words::empty("Önizleme yok: konumlar projenin sistemine çevrilemiyor.");
        };
        let format = self.number_format();
        let geographic = self.photo_system().is_some_and(|n| n.geographic());
        let (first, second) = if geographic {
            ("Enlem".to_owned(), "Boylam".to_owned())
        } else {
            (format.axes_text("Y (sağa)"), format.axes_text("X (yukarı)"))
        };
        let place = |p: &Placed| -> (String, String) {
            if geographic {
                (format_dd(p.p.y, true, 9), format_dd(p.p.x, false, 9))
            } else {
                (format.coord(p.p.x), format.coord(p.p.y))
            }
        };
        let rows = s
            .files
            .iter()
            .zip(read.iter())
            .take(PREVIEW_ROWS)
            .enumerate()
            .map(|(i, (f, r))| {
                let line = u32::try_from(i + 1).unwrap_or(u32::MAX);
                let placed = plan.placed.iter().find(|p| p.index == line);
                let (a, b) = placed.map_or_else(|| ("—".to_owned(), "—".to_owned()), place);
                let state: Element<'a, Message> = match placed {
                    Some(_) => label::caption("Alınacak").into(),
                    None => label::caption(
                        r.exif
                            .problem
                            .clone()
                            .unwrap_or_else(|| "Alınmayacak".to_owned()),
                    )
                    .style(style::text::warning)
                    .into(),
                };
                TableRow::new([
                    label::caption(line.to_string()).into(),
                    label::body(f.name.clone()).into(),
                    label::caption(r.exif.taken.clone().unwrap_or_default()).into(),
                    label::mono(a).into(),
                    label::mono(b).into(),
                    label::mono(
                        placed
                            .and_then(|p| p.attrs.get("Yön").cloned())
                            .unwrap_or_default(),
                    )
                    .into(),
                    state,
                ])
            });
        Table::new([
            TableColumn::new("Sıra").width(40).align_right(),
            TableColumn::new("Ad").width(150),
            TableColumn::new("Tarih").width(178),
            TableColumn::new(first).width(104).align_right(),
            TableColumn::new(second).width(112).align_right(),
            TableColumn::new("Yön").width(64).align_right(),
            TableColumn::new("Durum").width(Length::Fill),
        ])
        .extend(rows)
        .into()
    }

    fn photo_summary<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        if s.read.is_none() {
            return words::summary(vec![words::text_line(Line::Info, "Fotoğraflar okunuyor…")]);
        }
        let mut lines = Vec::new();
        match (self.photo_system(), &s.plan) {
            (None, _) => lines.push(words::text_line(
                Line::Error,
                "Projenin koordinat sistemi yok: fotoğrafların konumu WGS 84'tedir ve yerel sisteme çevrilemez. Proje ayarlarından bir koordinat sistemi seçin ya da özel sistem tanımlayın.",
            )),
            (Some(Named { name, system: None, .. }), _) => lines.push(words::text_line(
                Line::Error,
                format!("“{name}” sisteminin tanımı dönüşümlerde kullanılamıyor; fotoğrafların konumu ona çevrilemez."),
            )),
            (Some(named), Some(plan)) => {
                let n = plan.placed.len();
                lines.push(if n > 0 {
                    words::text_line(
                        Line::Ok,
                        format!(
                            "{n} fotoğraf nokta olarak alınacak; her biri kendi fotoğrafını ek olarak taşır ({}).",
                            if s.embed {
                                "projenin kitaplığına gömülür"
                            } else {
                                "dosyasına bağlı; çizimin klasöründekiler göreli yolla"
                            }
                        ),
                    )
                } else {
                    words::text_line(Line::Warn, "Konumu olan fotoğraf yok.")
                });
                if let Some(way) = plan.placed.first().and_then(|p| p.attrs.get("Dönüşüm")) {
                    let kind = if way.ends_with("resmî dönüşüm değil") {
                        Line::Warn
                    } else {
                        Line::Info
                    };
                    lines.push(words::text_line(kind, format!("Dönüşüm: {way}.")));
                }
                if n > 0 {
                    lines.push(words::text_line(
                        Line::Info,
                        "Yükseklik deniz düzeyine göredir ve dayanağı EXIF'te yazmaz: kot olarak değil, Yükseklik özniteliği olarak yazılır.",
                    ));
                }
                if !plan.skipped.is_empty() {
                    lines.push(problems(
                        format!("{} fotoğraf alınmayacak:", plan.skipped.len()),
                        &plan.skipped,
                    ));
                }
                if n > 0 && !named.geographic() {
                    lines.push(words::text_line(
                        Line::Info,
                        words::extent_text(&self.number_format(), &bounds(&plan.placed)),
                    ));
                }
            }
            (Some(_), None) => {}
        }
        words::summary(lines)
    }

    fn photo_layer<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(doc) = &self.document else {
            return label::caption("").into();
        };
        let layers = doc.model.layers();
        let leaves: Vec<String> = layers
            .leaves()
            .iter()
            .filter(|l| l.service.is_none())
            .map(|l| l.id.clone())
            .collect();
        let mut choices = vec![Choice::new("Yeni katman")];
        choices.extend(leaves.iter().map(|id| {
            let mark = if layers.is_locked(id) {
                " (kilitli)"
            } else if !layers.is_visible(id) {
                " (gizli)"
            } else {
                ""
            };
            Choice::new(format!("{}{mark}", self.layer_path(id)))
        }));
        let selected = match &s.target {
            None => Some(0),
            Some(id) => leaves.iter().position(|l| l == id).map(|i| i + 1),
        };
        let pick = Select::new(choices, selected, move |i| {
            event(Event::Target(
                i.checked_sub(1).and_then(|k| leaves.get(k).cloned()),
            ))
        });
        let hidden = s
            .target
            .as_ref()
            .is_some_and(|id| !layers.is_visible(id))
            .then(|| "Katman gizli: noktalar alınır ama görünmez.".to_owned());
        let mut parts = row![
            container(words::field("Hedef katman", pick, hidden)).width(Length::FillPortion(3))
        ]
        .spacing(16);
        if s.target.is_none() {
            let name = text_input("Yeni katmanın adı", &s.name)
                .on_input(|t| event(Event::Name(t)))
                .on_submit(event(Event::Run))
                .padding([5, 8])
                .size(kentos_ui::theme::typography::body())
                .style(style::field::input);
            parts = parts.push(
                container(words::field(
                    "Yeni katmanın adı",
                    kentos_ui::widget::focus_ring(name),
                    Some("Bu adda bir katman varsa noktalar ona eklenir.".to_owned()),
                ))
                .width(Length::FillPortion(2)),
            );
        }
        column![
            parts,
            words::check(
                s.embed,
                "Gömülü: fotoğraflar projenin kitaplığına gömülür",
                Some(event(Event::Embed))
            ),
        ]
        .spacing(10)
        .width(Fill)
        .into()
    }
}

/// A warning line with the first problems and how many more.
fn problems<'a>(title: String, items: &[LineError]) -> Element<'a, Message> {
    let shown: Vec<Element<'a, Message>> = items
        .iter()
        .take(SHOWN_PROBLEMS)
        .map(|e| label::caption(format!("• {}", e.message)).into())
        .collect();
    let mut list = Column::with_children(shown).spacing(2);
    if items.len() > SHOWN_PROBLEMS {
        list = list.push(label::caption(format!(
            "• … ve {} fotoğraf daha.",
            items.len() - SHOWN_PROBLEMS
        )));
    }
    words::line(Line::Warn, column![label::body(title), list].spacing(4))
}

#[cfg(test)]
impl State {
    /// The points as they will be written, with the current choices.
    pub(crate) fn plan(&self) -> Option<&Plan> {
        self.plan.as_deref()
    }
}

#[cfg(test)]
impl App {
    /// Konumlu fotoğraflar over `files`, read now (the pictures' scenes and the tests run no tasks).
    pub(crate) fn photo_import_now(&mut self, files: Vec<PhotoFile>) {
        let _ = self.photo_import_files(files.clone());
        let Some(Window::PhotoImport(s)) = &self.exchange else {
            return;
        };
        let id = s.reading_id;
        let read = files.iter().map(read_one).collect();
        let _ = self.photo_import_event(Event::Read {
            id,
            read: Arc::new(read),
        });
    }

    /// The photo import's state, while its window is open.
    pub(crate) fn photo_import_state(&self) -> Option<&State> {
        match &self.exchange {
            Some(Window::PhotoImport(s)) => Some(s),
            _ => None,
        }
    }
}
