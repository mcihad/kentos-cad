//! IFC modeli (plan/ifc-uyum.md IFC-12, docs/adr/8061 §4–§5, docs/adr/8073 §5): the application's adapter of the
//! model viewer (crates/model-view-ui). It opens an IFC file in a window of its own, always beside the plan
//! (shell/windows/, docs/adr/8060 §10, 8122), reads it on a thread of its own with the reading
//! face (`kentos_ifc::read`): the file's summary and objects, their bodies and the openings' volumes, handed to the
//! viewer as its plain input; the chosen object's details are read from the file when it is chosen. A read still
//! running gives way to a new one; İptal and the window's close stop it, and what it would have given never shows.
//!
//! The viewer and the plan choose together when KentOS wrote the file from the open drawing: an element's GlobalId is
//! its persistent id (`kentos_ifc::step::guid`), so a choice in either finds the other's object. A file another program
//! wrote chooses nothing in the plan. The viewer never changes the drawing. Its look is the component's; this file
//! gives it the drawing area's sky, the numbers' format and the quality the 3D views are drawn at.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use iced::futures::channel::mpsc;
use iced::widget::space;
use iced::{Element, Size, Task, Theme};
use kentos_domain::Uuid;
use kentos_ifc::read::{self as ifc, Limits, Model};
use kentos_interaction::Level;
use kentos_model_view::Input;
use kentos_model_view_ui::{Host, Out, Stage, Viewer};
use kentos_ui::widget::Dialog as Frame;

use crate::app::{App, Dialog, Message};
use crate::traces::Control;
use crate::windows::{OwnWindow, Surface};

/// The window's title, which a trace names it by.
pub(crate) const TITLE: &str = "IFC modeli";
const SURFACE: Surface = Surface::IfcViewer;

/// The open viewer: its file, the read under way (its number and its stop), the file read, the plan's selection it
/// last took and the Denetim findings it last painted.
pub struct IfcWindow {
    pub viewer: Viewer,
    pub path: PathBuf,
    id: u64,
    stop: Arc<AtomicBool>,
    file: Option<(Arc<Vec<u8>>, Arc<Model>)>,
    taken: u64,
    found: Option<u64>,
    /// The objects asked for when the file was opened (a review's finding, docs/adr/8104): chosen and painted once it
    /// is read, by their GlobalIds.
    want: Vec<String>,
}

impl std::fmt::Debug for IfcWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IfcWindow")
            .field("path", &self.path)
            .field("viewer", &self.viewer)
            .finish_non_exhaustive()
    }
}

/// A file read: its bytes, the model, the viewer's input and what the status bar says of it.
#[derive(Debug)]
pub struct Read {
    pub(crate) bytes: Arc<Vec<u8>>,
    pub(crate) model: Arc<Model>,
    pub(crate) input: Input,
}

#[derive(Debug, Clone)]
pub enum Event {
    Viewer(kentos_model_view_ui::Message),
    /// The file IFC modeli görüntüle… asked for; none when the dialog was closed.
    Picked(Option<PathBuf>),
    /// A file opened in the viewer (IFC dışa aktar's Görüntüle).
    Open(PathBuf),
    Progress {
        id: u64,
        phase: &'static str,
    },
    Done {
        id: u64,
        result: Result<Arc<Read>, String>,
    },
}

fn msg(event: Event) -> Message {
    Message::IfcViewer(event)
}

static NEXT: AtomicU64 = AtomicU64::new(1);

/// The phases the status names while the file is read.
const READING: &str = "Dosya okunuyor";
const MODEL: &str = "Nesneler ve ilişkiler okunuyor";
const BODIES: &str = "Gövdeler üçgenleniyor";

/// Reads the file: the model, its bodies and its openings' volumes, the viewer's input; none when it was stopped.
fn read(
    path: &Path,
    stop: &AtomicBool,
    tell: &mut dyn FnMut(&'static str),
) -> Option<Result<Read, String>> {
    tell(READING);
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) => {
            return Some(Err(format!(
                "{} okunamadı: {e}. Dosyanın yerini ve okuma iznini denetleyin.",
                path.display()
            )));
        }
    };
    let name = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    read_bytes(&name, bytes, stop, tell)
}

/// Reads a file's `bytes` named `name` (one on disk, or one written in memory: 3B görünüm's drawing, docs/adr/8119):
/// the model, its bodies and its openings' volumes, the viewer's input; none when it was stopped.
pub(crate) fn read_bytes(
    name: &str,
    bytes: Vec<u8>,
    stop: &AtomicBool,
    tell: &mut dyn FnMut(&'static str),
) -> Option<Result<Read, String>> {
    let cancel = || stop.load(Ordering::Relaxed);
    let stopped = |e: &ifc::ReadError| matches!(e, ifc::ReadError::Cancelled);
    tell(MODEL);
    let model = match ifc::read(&bytes, &Limits::default(), &cancel) {
        Ok(m) => m,
        Err(e) if stopped(&e) => return None,
        Err(e) => return Some(Err(e.to_string())),
    };
    tell(BODIES);
    let made = model.bodies(&bytes, &cancel).and_then(|mut b| {
        let openings = model.opening_bodies(&bytes, &cancel)?;
        b.bodies.extend(openings.bodies);
        b.not_shown.extend(openings.not_shown);
        Ok(b)
    });
    let bodies = match made {
        Ok(b) => b,
        Err(e) if stopped(&e) => return None,
        Err(e) => return Some(Err(e.to_string())),
    };
    let input = ifc::view::input(name, &model, &bodies);
    Some(Ok(Read {
        bytes: Arc::new(bytes),
        model: Arc::new(model),
        input,
    }))
}

impl App {
    /// IFC modeli görüntüle…: asks for the file, then opens it.
    pub(crate) fn pick_ifc_model(&mut self) -> Task<Message> {
        Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("IFC modeli görüntüle")
                    .add_filter("IFC (.ifc)", &["ifc"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |path| msg(Event::Picked(path)),
        )
    }

    /// Opens `path` in the viewer, a read still running giving way; the file is read on a thread of its own.
    pub(crate) fn open_ifc_model(&mut self, path: PathBuf) -> Task<Message> {
        if let Some(w) = self.ifc_viewer.take() {
            w.stop.store(true, Ordering::Relaxed);
        }
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let stop = Arc::new(AtomicBool::new(false));
        let title = path.file_name().map_or_else(
            || path.display().to_string(),
            |n| n.to_string_lossy().into_owned(),
        );
        self.ifc_viewer = Some(Box::new(IfcWindow {
            viewer: Viewer::new(title),
            path: path.clone(),
            id,
            stop: stop.clone(),
            file: None,
            taken: self.selection.version(),
            found: None,
            want: Vec::new(),
        }));
        self.show_surface(SURFACE);
        iced_runtime::task::blocking(move |mut out: mpsc::Sender<Message>| {
            let mut tell = |phase: &'static str| {
                let _ = out.try_send(msg(Event::Progress { id, phase }));
            };
            let result = read(&path, &stop, &mut tell);
            // Stopped: what it would have given is never shown.
            let Some(result) = result.filter(|_| !stop.load(Ordering::Relaxed)) else {
                return;
            };
            let done = msg(Event::Done {
                id,
                result: result.map(Arc::new),
            });
            let _ = iced::futures::executor::block_on(iced::futures::SinkExt::send(&mut out, done));
        })
    }

    /// The model at `path` opened with `keys` (GlobalIds) chosen and painted once it is read: a review's finding shown on
    /// its objects (docs/adr/8104). A file already open is not read again.
    pub fn open_ifc_model_showing(&mut self, path: PathBuf, keys: Vec<String>) -> Task<Message> {
        if let Some(w) = self
            .ifc_viewer
            .as_mut()
            .filter(|w| w.path == path && w.file.is_some())
        {
            w.want = keys;
            Self::show_wanted(w);
            self.show_surface(SURFACE);
            return Task::none();
        }
        let task = self.open_ifc_model(path);
        if let Some(w) = self.ifc_viewer.as_mut() {
            w.want = keys;
        }
        task
    }

    /// The objects asked for chosen (the first) and painted, the chosen one's details read.
    fn show_wanted(w: &mut IfcWindow) {
        if w.want.is_empty() {
            return;
        }
        w.viewer.set_found(w.want.iter().map(String::as_str));
        if w.viewer.choose_key(&w.want[0])
            && let Some(object) = w.viewer.view().and_then(|v| v.selected())
            && let Some((bytes, model)) = &w.file
            && let Ok(d) = model.details(bytes, object)
        {
            w.viewer.set_details(object, d);
        }
    }

    /// The viewer closes: its read stops, its model and scene go (the GPU's copy with its window), and the main
    /// window's dialog when it was over the drawing.
    pub fn ifc_viewer_gone(&mut self) {
        if let Some(w) = self.ifc_viewer.take() {
            w.stop.store(true, Ordering::Relaxed);
        }
        self.surface_away(SURFACE);
    }

    pub(crate) fn ifc_viewer_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Picked(Some(path)) | Event::Open(path) => return self.open_ifc_model(path),
            Event::Picked(None) => {}
            Event::Progress { id, phase } => {
                if let Some(w) = self.ifc_viewer.as_mut().filter(|w| w.id == id) {
                    w.viewer.progress(phase, 0, 0);
                }
            }
            Event::Done { id, result } => {
                let Some(w) = self.ifc_viewer.as_mut().filter(|w| w.id == id) else {
                    return Task::none();
                };
                match result {
                    Ok(read) => {
                        let read = Arc::try_unwrap(read).unwrap_or_else(|r| Read {
                            bytes: r.bytes.clone(),
                            model: r.model.clone(),
                            input: r.input.clone(),
                        });
                        let s = &read.model.summary;
                        let mut facts = vec![(s.schema.clone(), schema_tip(&read.model))];
                        let findings = read.model.findings.len();
                        if findings > 0 {
                            facts.push((
                                format!("{findings} okuma bulgusu"),
                                "Dosyada okumayı durdurmayan bozukluklar var (çözülemeyen kayıt, olmayan başvuru, yinelenen GlobalId); ayrıntısı İncele'dedir.".to_owned(),
                            ));
                        }
                        w.viewer.load(read.input, facts);
                        w.file = Some((read.bytes, read.model));
                        if w.want.is_empty() {
                            self.follow_ifc_viewer();
                        } else if let Some(w) = self.ifc_viewer.as_mut() {
                            Self::show_wanted(w);
                        }
                    }
                    Err(why) => w.viewer.failed(why),
                }
            }
            Event::Viewer(m) => {
                let Some(w) = self.ifc_viewer.as_mut() else {
                    return Task::none();
                };
                match w.viewer.update(m) {
                    Some(Out::Chosen(chosen)) => self.ifc_viewer_chosen(chosen),
                    Some(Out::Cancel) => {
                        self.ifc_viewer_gone();
                        self.say(Level::Info, "IFC modelinin okunması durduruldu.");
                    }
                    Some(Out::Close) => self.ifc_viewer_gone(),
                    None => {}
                }
            }
        }
        Task::none()
    }

    /// An object chosen in the viewer: its details read from the file, and the plan's object chosen when the file is
    /// the drawing's (a choice of nothing, or of what the drawing has no object for, leaves the plan as it is).
    fn ifc_viewer_chosen(&mut self, chosen: Option<(usize, String)>) {
        let Some(w) = self.ifc_viewer.as_mut() else {
            return;
        };
        let Some((object, key)) = chosen else { return };
        if let Some((bytes, model)) = &w.file {
            match model.details(bytes, object) {
                Ok(d) => w.viewer.set_details(object, d),
                Err(e) => self.say(Level::Warn, format!("Nesnenin özellikleri okunamadı: {e}")),
            }
        }
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let Some(slot) = kentos_ifc::step::unguid(&key)
            .map(Uuid::from_u128)
            .and_then(|u| doc.model.slot_of(u))
        else {
            return;
        };
        self.selection.set(vec![slot]);
        if let Some(w) = self.ifc_viewer.as_mut() {
            // Taken now: following the plan does not choose it again.
            w.taken = self.selection.version();
        }
    }

    /// The viewer follows the plan's selection (shell/app/update.rs, after every message): the first object chosen in
    /// the plan is chosen in the viewer when the file holds it. The objects of Denetim's findings (the rows chosen,
    /// else all it lists: what 3B görünüm paints) are painted as found when the file is the drawing's (its GlobalIds
    /// the drawing's ids, docs/adr/8074 §2).
    pub fn follow_ifc_viewer(&mut self) {
        self.follow_ifc_viewer_found();
        let version = self.selection.version();
        let Some(w) = self
            .ifc_viewer
            .as_mut()
            .filter(|w| w.taken != version && w.viewer.stage == Stage::Shown)
        else {
            return;
        };
        w.taken = version;
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let Some(uuid) = self.selection.ids().iter().find_map(|s| doc.model.uid(*s)) else {
            return;
        };
        let key = kentos_ifc::step::guid(uuid.as_u128());
        if w.viewer.choose_key(&key)
            && let Some(object) = w.viewer.view().and_then(|v| v.selected())
            && let Some((bytes, model)) = &w.file
            && let Ok(d) = model.details(bytes, object)
        {
            w.viewer.set_details(object, d);
        }
    }

    /// Denetim's findings painted, when they changed since the viewer last took them.
    fn follow_ifc_viewer_found(&mut self) {
        let key = self.building3d_found_key();
        // A review's finding shown keeps its objects painted (docs/adr/8104).
        if self.ifc_viewer.as_ref().is_none_or(|w| {
            w.found == Some(key) || w.viewer.stage != Stage::Shown || !w.want.is_empty()
        }) {
            return;
        }
        let keys: Vec<String> = self
            .building3d_found()
            .iter()
            .filter_map(|u| Uuid::parse_str(u).ok())
            .map(|u| kentos_ifc::step::guid(u.as_u128()))
            .collect();
        if let Some(w) = self.ifc_viewer.as_mut() {
            w.found = Some(key);
            w.viewer.set_found(keys.iter().map(String::as_str));
        }
    }

    /// The viewer, as its host shows it.
    fn ifc_viewer_layout(&self) -> Element<'_, Message> {
        let Some(w) = &self.ifc_viewer else {
            return space().into();
        };
        w.viewer
            .layout(self.viewer_host(|m| msg(Event::Viewer(m)), Vec::new()))
    }

    /// What a viewer's host gives it (IFC modeli's, 3B görünüm's building, docs/adr/8119): the drawing area's sky, the
    /// project's coordinates, and a measured value with the project's decimals by its unit, in the file's unit (not
    /// converted); `end` at the strip's end.
    pub(crate) fn viewer_host<'a>(
        &'a self,
        to: fn(kentos_model_view_ui::Message) -> Message,
        end: Vec<Element<'a, Message>>,
    ) -> Host<'a, Message> {
        let f = self.format();
        let (length, area) = self.document.as_ref().map_or((3, 2), |d| {
            let s = d.model.settings();
            (s.length_decimals as usize, s.area_decimals as usize)
        });
        Host {
            to,
            end,
            sky: crate::scene3d_window::sky(self.canvas()),
            quality: self.scene3d_quality(),
            coordinate: Box::new(move |v| f.coord(v)),
            measure: Box::new(move |v, unit| {
                match kentos_model_view_ui::look::digits(unit, length, area) {
                    Some(d) => kentos_interaction::fixed(v, d),
                    None => kentos_model_view_ui::look::plain(v),
                }
            }),
            east: f.east_label(),
            north: f.north_label(),
        }
    }

    /// Over the drawing: a window like the others, the viewer framed in it.
    pub(crate) fn ifc_viewer_view(&self) -> Element<'_, Message> {
        if self.ifc_viewer.is_none() {
            return space().into();
        }
        let height =
            (self.window_size.height - kentos_ui::theme::typography::scaled(200.0)).max(420.0);
        let viewer = iced::widget::container(self.ifc_viewer_layout()).height(height);
        crate::windows::placed(
            Frame::new(TITLE).fill(viewer).width(1280.0),
            SURFACE,
            false,
            msg(Event::Viewer(kentos_model_view_ui::Message::Close)),
        )
    }

    /// Beside the plan: the whole of its own window.
    pub(crate) fn ifc_viewer_window_view(&self) -> Element<'_, Message> {
        iced::widget::container(self.ifc_viewer_layout())
            .style(|t: &Theme| iced::widget::container::Style {
                background: Some(iced::Background::Color(
                    kentos_ui::theme::Tokens::of(t).surface,
                )),
                ..Default::default()
            })
            .into()
    }

    pub(crate) fn ifc_viewer_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = self.ifc_viewer.as_ref() else {
            return Err(format!("{TITLE} penceresi açık değil"));
        };
        viewer_control(&w.viewer, control, TITLE).map(|m| Some(msg(Event::Viewer(m))))
    }
}

/// A trace's control of a viewer in the window titled `title` (IFC modeli's, 3B görünüm's building): the viewer's
/// message it sends; a tree row by its words chooses its object.
pub(crate) fn viewer_control(
    viewer: &Viewer,
    control: Control<'_>,
    title: &str,
) -> Result<kentos_model_view_ui::Message, String> {
    use kentos_model_view::Cut;
    use kentos_model_view_ui::{Message as V, Panel};
    Ok(match control {
        Control::Press("Kapat") => V::Close,
        Control::Press("İptal") => V::Cancel,
        Control::Press("Sığdır") => V::Fit,
        Control::Press("Seçime yaklaş") => V::Focus,
        Control::Press("Gizle") => V::Hide,
        Control::Press("Yalnız bırak") => V::Isolate,
        Control::Press("Hepsini göster") => V::ShowAll,
        Control::Press("Mahaller") => V::Spaces,
        Control::Press("Açıklıklar") => V::Openings,
        Control::Press("Zonlar") => V::Zones,
        Control::Press("Saydam") => V::SeeThrough,
        Control::Press("Kuzeye bak") => V::North,
        Control::Press("Lejant") => V::Legend,
        Control::Press("Yatay kesit") => V::Cut(Cut::Level),
        Control::Press("Kuzey–güney düşey kesiti") => V::Cut(Cut::NorthSouth),
        Control::Press("Doğu–batı düşey kesiti") => V::Cut(Cut::EastWest),
        Control::Press("Seçimin çevresinde kutu") => V::Boxed,
        Control::Press("Kesiti kaldır") => V::Uncut,
        Control::Press("Öbür yanı") => V::Flip,
        Control::Press("Ölç") => V::Measure,
        Control::Press("Uzaklık") => V::Tool(kentos_model_view_ui::Tool::Distance),
        Control::Press("Açı") => V::Tool(kentos_model_view_ui::Tool::Angle),
        Control::Press("Alan") => V::Tool(kentos_model_view_ui::Tool::Area),
        Control::Press("Vazgeç") => V::MeasureCancel,
        Control::Press("Temizle") => V::MeasureClear,
        Control::Press("Ağacı daralt" | "Yapı ağacını aç") => {
            V::Pane(kentos_model_view_ui::Side::Tree)
        }
        Control::Press("Özellikleri daralt" | "Özellikleri aç") => {
            V::Pane(kentos_model_view_ui::Side::Details)
        }
        Control::Press("Yapı") => V::Panel(Panel::Structure),
        Control::Press("Sınıf") => V::Panel(Panel::Class),
        Control::Press("Gösterilemeyen") => V::Panel(Panel::NotShown),
        Control::Press(name) if kentos_model_view::VIEWS.iter().any(|v| v.0 == name) => V::View(
            kentos_model_view::VIEWS
                .iter()
                .position(|v| v.0 == name)
                .unwrap_or(0),
        ),
        Control::Press(name) => {
            // A tree row by its words: the object chosen.
            match viewer
                .view()
                .and_then(|v| v.input().objects.iter().position(|o| o.label == name))
            {
                Some(o) => V::Choose(Some(o)),
                None => return Err(format!("“{title}” penceresinde {name} yok")),
            }
        }
        other => return Err(format!("“{title}” penceresinde {other} yok")),
    })
}

#[cfg(any(test, feature = "testing"))]
impl App {
    /// The read under way done here and now, as its thread would give it (a test runs no task).
    pub fn ifc_viewer_read_now(&mut self) {
        let Some(w) = self.ifc_viewer.as_ref() else {
            return;
        };
        let (id, path, stop) = (w.id, w.path.clone(), w.stop.clone());
        if let Some(result) = read(&path, &stop, &mut |_| {}) {
            let _ = self.ifc_viewer_event(Event::Done {
                id,
                result: result.map(Arc::new),
            });
        }
    }

    /// The read under way's number: a test gives a late result with it.
    pub fn ifc_viewer_id(&self) -> Option<u64> {
        self.ifc_viewer.as_ref().map(|w| w.id)
    }

    /// A late result for read `id`, as its thread would give it.
    pub fn ifc_viewer_late(&mut self, id: u64, path: &Path) {
        let stop = AtomicBool::new(false);
        if let Some(result) = read(path, &stop, &mut |_| {}) {
            let _ = self.ifc_viewer_event(Event::Done {
                id,
                result: result.map(Arc::new),
            });
        }
    }

    /// What a picture of IFC modeli shows: its own window while it is open beside the plan, else the main window.
    pub fn ifc_viewer_shown(&self) -> Element<'_, Message> {
        match self.windows.of(SURFACE) {
            Some(w) if self.surface_wanted(SURFACE) => self.window_view(w.id),
            _ => self.view(),
        }
    }
}

/// What the schema readout's tip says: the schema, the program that wrote the file, its units.
pub(crate) fn schema_tip(m: &Model) -> String {
    let s = &m.summary;
    let by = s
        .header
        .originating_system
        .as_deref()
        .or(s.header.preprocessor.as_deref())
        .unwrap_or("bilinmiyor");
    let length = s.units.length.as_ref().map_or_else(
        || "yazılmamış".to_owned(),
        |u| format!("{}{}", u.prefix.as_deref().unwrap_or(""), u.name),
    );
    format!(
        "Şema {} ({} kayıt). Yazan program: {by}. Uzunluk birimi: {length}. Koordinatlar dosyadaki gibidir; görüntüleyici dönüştürmez.",
        s.schema, s.records
    )
}

/// IFC modeli as a window of its own (shell/windows/surface.rs): it opens beside the plan until the user brings it over
/// the drawing; Esc and its close button close it.
pub(crate) struct IfcView;

impl OwnWindow for IfcView {
    fn kept_as(&self) -> &'static str {
        "ifc-modeli"
    }

    fn sizes(&self) -> (Size, Size) {
        (Size::new(1400.0, 860.0), Size::new(900.0, 560.0))
    }

    /// A model is looked at beside the plan, often on another screen.
    fn opens_apart(&self) -> bool {
        true
    }

    /// Never over the drawing (docs/adr/8122, the owner's choice of 18 November).
    fn always_apart(&self) -> bool {
        true
    }

    fn what(&self) -> &'static str {
        "IFC modelini"
    }

    fn apart_tip(&self) -> &'static str {
        "IFC modelini kendi penceresinde açar: pencereyi başka bir ekrana taşıyıp çizerken modeli izleyebilirsiniz. Sonraki açılışlar da ayrı pencerede olur."
    }

    fn return_tip(&self) -> &'static str {
        "IFC modelini yeniden ana pencerede, çizimin üstünde açar. Sonraki açılışlar da orada olur."
    }

    fn dialog(&self) -> Dialog {
        Dialog::IfcViewer
    }

    fn is_open(&self, app: &App) -> bool {
        app.ifc_viewer.is_some()
    }

    fn title(&self, app: &App) -> String {
        app.ifc_viewer.as_ref().map_or_else(
            || TITLE.to_owned(),
            |w| format!("{} — {TITLE}", w.viewer.title),
        )
    }

    fn view<'a>(&self, app: &'a App) -> Element<'a, Message> {
        app.ifc_viewer_window_view()
    }

    fn close(&self, app: &mut App) -> Task<Message> {
        app.ifc_viewer_gone();
        Task::none()
    }

    fn trace_title(&self, _app: &App) -> Option<String> {
        Some(TITLE.to_owned())
    }

    fn trace_control(
        &self,
        app: &App,
        control: Control<'_>,
    ) -> Option<Result<Option<Message>, String>> {
        Some(app.ifc_viewer_control(control))
    }
}
