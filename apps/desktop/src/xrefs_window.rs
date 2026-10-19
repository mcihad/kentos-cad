//! Dış başvurular (docs/adr/0348 §5; `xref.attach`, `xref.manager`): another drawing file attached as a reference
//! through `cad.xrefs.edit`, and the drawing's references listed with their state through `cad.xrefs.list` —
//! Yeniden yükle, Boşalt, Yolu değiştir…, Kaldır, Bağla ▾. A changed or missing source is said when the drawing opens;
//! nothing is taken in without a command.

use std::path::{Path, PathBuf};

use iced::widget::{Column, Row, button, container, row, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{
    AssetKind, AssetState, AssetStatus, AssetVia, AssetsCheck, CommandResult, CommandWarning, Vec2, XrefBindMode, XrefInfo, XrefKind, XrefState, XrefsEdit, XrefsEdited,
    XrefsList, XrefsOperation,
};
use kentos_interaction::pick::PickPoint;
use kentos_native_application::{ExecutionContext, xrefs_edit, xrefs_list};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::widget::{Dialog as Frame, NumberInput, Segmented, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message, Picker};
use crate::exchange::words::{self, Kind};
use kentos_interaction::Level;
use crate::traces::Control;

/// The windows' titles, which a trace names them by.
pub const TITLE: &str = "Dış başvurular";
pub const ATTACH_TITLE: &str = "Dış başvuru ekle";
const CLOSE: &str = "Kapat";
const RELOAD: &str = "Yeniden yükle";
const UNLOAD: &str = "Boşalt";
const REPATH: &str = "Yolu değiştir…";
const DETACH: &str = "Kaldır";
const BIND: &str = "Bağla";
const BIND_INSERT: &str = "Bağla, katmanları birleştir";
const ADD: &str = "Ekle…";
const ADD_PATH: &str = "Klasör ekle…";
const IN_PLACE: &str = "Yerinde ekle";
const ON_SCREEN: &str = "Noktayı seç";
const CANCEL: &str = "Vazgeç";

/// Ekle or Bindir, as the segmented choice shows them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Attach,
    Overlay,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Attach => "Ekle",
            Self::Overlay => "Bindir",
        })
    }
}

const MODES: [Mode; 2] = [Mode::Attach, Mode::Overlay];

/// The window's parts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Section {
    #[default]
    References,
    Paths,
    Others,
}

impl std::fmt::Display for Section {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::References => "Dış başvurular",
            Self::Paths => "Arama yolları",
            Self::Others => "Diğer bağlı dosyalar",
        })
    }
}

const SECTIONS: [Section; 3] = [Section::References, Section::Paths, Section::Others];

/// Dış başvuru ekle's choices, before the point.
#[derive(Clone, Debug)]
pub struct Attach {
    pub path: PathBuf,
    pub name: String,
    pub mode: Mode,
    pub scale: f64,
    /// Degrees, counterclockwise.
    pub rotation: f64,
}

#[derive(Clone, Debug, Default)]
pub struct State {
    /// The list, as `cad.xrefs.list` gave it when the window opened or after a command.
    pub listed: Option<Vec<XrefInfo>>,
    /// The row chosen.
    pub chosen: Option<usize>,
    /// Bağla ▾ open.
    pub bind_menu: bool,
    /// Dış başvuru ekle's window, or its choices while the point is picked on the drawing.
    pub attach: Option<Attach>,
    pub picking: bool,
    /// The project's search paths (docs/adr/0349 §1).
    pub paths: Vec<String>,
    /// The drawing's other linked files: pictures, rasters, point clouds, tables, attachments, sources.
    pub others: Vec<AssetStatus>,
    pub section: Section,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Purpose {
    Attach,
    Repath,
}

#[derive(Clone, Debug)]
pub enum Event {
    Picked(Purpose, Option<PathBuf>),
    Name(String),
    Mode(Mode),
    Scale(f64),
    Rotation(f64),
    InPlace,
    OnScreen,
    CancelAttach,
    Choose(usize),
    Reload,
    Unload,
    Repath,
    Detach,
    BindMenu,
    Bind(XrefBindMode),
    Add,
    Section(Section),
    AddPath,
    PathPicked(Option<PathBuf>),
    RemovePath(usize),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Xrefs(e)
}

fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned())
}

fn state_words(s: XrefState) -> (&'static str, Kind) {
    match s {
        XrefState::Loaded => ("Yüklü", Kind::Ok),
        XrefState::Unloaded => ("Boşaltıldı", Kind::Info),
        XrefState::Missing => ("Bulunamadı", Kind::Error),
        XrefState::Changed => ("Değişti", Kind::Warn),
    }
}

impl App {
    /// The open drawing's file, for the references' relative paths and the cycle check.
    fn drawing_path(&self) -> Option<String> {
        self.document.as_ref()?.path.as_ref().map(|p| p.display().to_string())
    }

    fn xrefs_listed(&self) -> Option<Vec<XrefInfo>> {
        let doc = self.document.as_ref()?;
        let input = XrefsList { drawing: self.drawing_path(), affected: false };
        match xrefs_list::plan(&ExecutionContext::new(&mut doc.model.clone()), &input) {
            CommandResult::Completed { output, .. } => Some(nested_under(output.xrefs)),
            _ => None,
        }
    }

    /// The other linked files (Diğer bağlı dosyalar) and the search paths, as the drawing has them now.
    fn xrefs_others(&mut self) {
        let Some(doc) = self.document.as_ref() else { return };
        let drawing = self.drawing_path();
        // Pictures, tables and attachments with their SHA-256 where one is kept; a bound layer's source (a large file) only
        // looked for.
        let near = AssetsCheck {
            drawing: drawing.clone(),
            kinds: Some(vec![AssetKind::Image, AssetKind::Raster, AssetKind::PointCloud, AssetKind::Table, AssetKind::Attachment]),
            digests: Some(true),
        };
        let sources = AssetsCheck { drawing, kinds: Some(vec![AssetKind::Source]), digests: Some(false) };
        let mut others = kentos_native_application::assets_check::check_in(&doc.model, &near).assets;
        others.extend(kentos_native_application::assets_check::check_in(&doc.model, &sources).assets);
        self.xrefs.others = others;
        self.xrefs.paths = doc.model.settings().xref_paths.clone();
    }

    /// Dış başvurular (`xref.manager`).
    pub(crate) fn open_xrefs(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.xrefs.listed = self.xrefs_listed();
        self.xrefs.chosen = self.xrefs.listed.as_ref().filter(|l| !l.is_empty()).map(|_| 0);
        self.xrefs.bind_menu = false;
        self.xrefs_others();
        self.dialog = Some(Dialog::Xrefs);
    }

    /// Dış başvuru ekle… (`xref.attach`): the source drawing asked for, then its window.
    pub(crate) fn start_xref_attach(&mut self) -> Task<Message> {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return Task::none();
        }
        self.pick_xref_file(Purpose::Attach)
    }

    fn pick_xref_file(&mut self, purpose: Purpose) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            return Task::done(msg(Event::Picked(purpose, Some(path.clone()))));
        }
        let title = match purpose {
            Purpose::Attach => ATTACH_TITLE,
            Purpose::Repath => "Dış başvurunun yeni yolu",
        };
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title(title)
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            move |path| msg(Event::Picked(purpose, path)),
        )
    }

    /// Runs `operation` as one step; the answer said, the list read again.
    pub(crate) fn xref_run(&mut self, operation: XrefsOperation) -> Option<XrefsEdited> {
        let locked = self.tree_locked();
        let drawing = self.drawing_path();
        let doc = self.document.as_mut()?;
        let input = XrefsEdit { operation, drawing, expected_revision: None };
        // A project whose layer tree is not ours to change takes no new layer.
        if let Some(locked) = locked
            && let CommandResult::Completed { output, .. } = xrefs_edit::plan(&ExecutionContext::new(&mut doc.model), &input)
            && !output.layers.is_empty()
        {
            self.warn(format!("{TITLE}: {locked}"));
            return None;
        }
        let (output, warnings) = match xrefs_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, warnings } => (output, warnings),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                self.warn(format!("{TITLE}: {}", error.message));
                return None;
            }
            _ => return None,
        };
        self.say_warnings(&warnings);
        if self.dialog == Some(Dialog::Xrefs) {
            self.xrefs.listed = self.xrefs_listed();
            let n = self.xrefs.listed.as_ref().map_or(0, Vec::len);
            self.xrefs.chosen = self.xrefs.chosen.filter(|k| *k < n).or((n > 0).then_some(0));
            self.xrefs_others();
        }
        // A command answered: the files as they are now are the ones seen.
        self.xref_watch_seen();
        Some(output)
    }

    fn say_warnings(&mut self, warnings: &[CommandWarning]) {
        for w in warnings {
            self.warn(format!("{TITLE}: {}", w.message));
        }
    }

    fn attach_now(&mut self, at: Option<Vec2>) {
        let Some(a) = self.xrefs.attach.take() else { return };
        let layer = self.document.as_ref().map(|d| d.model.layers().active().to_owned()).unwrap_or_default();
        let operation = XrefsOperation::Attach {
            path: a.path.display().to_string(),
            mode: match a.mode {
                Mode::Attach => XrefKind::Attach,
                Mode::Overlay => XrefKind::Overlay,
            },
            name: Some(a.name.clone()).filter(|n| !n.trim().is_empty()),
            at,
            scale: Some(a.scale),
            rotation: Some(a.rotation),
            layer_id: layer,
        };
        if let Some(done) = self.xref_run(operation) {
            let mut parts = vec![format!("{} nesne", done.objects)];
            if !done.layers.is_empty() {
                parts.push(format!("{} yeni katman", done.layers.len()));
            }
            if !done.nested.is_empty() {
                parts.push(format!("{} iç blok ya da iç içe dış başvuru", done.nested.len()));
            }
            self.say(
                Level::Success,
                format!(
                    "“{}” dış başvuru olarak eklendi: “{}”, {}; katmanları “Dış başvuru: {}” grubunda (tek adımda geri alınır).",
                    file_name(&a.path),
                    done.name,
                    parts.join(", "),
                    done.name
                ),
            );
        }
    }

    /// The point Dış başvuru ekle asked for (input.rs); false when it asked none.
    pub(crate) fn xrefs_picked(&mut self, p: Option<kentos_interaction::Vec2>) -> bool {
        if !std::mem::take(&mut self.xrefs.picking) {
            return false;
        }
        match p {
            Some(p) => self.attach_now(Some(Vec2 { x: p.x, y: p.y })),
            None => {
                // Esc: the window comes back with its choices.
                self.dialog = Some(Dialog::XrefAttach);
            }
        }
        true
    }

    /// The search paths written (a setting): the list and its states read again.
    fn xrefs_paths(&mut self, paths: Vec<String>) {
        if let Some(done) = self.xref_run(XrefsOperation::Paths { paths }) {
            self.say(
                Level::Success,
                if done.paths.is_empty() {
                    "Dış başvuruların arama yolu yok.".to_owned()
                } else {
                    format!("Dış başvuruların arama yolları: {}.", done.paths.join("; "))
                },
            );
        }
        self.open_xrefs();
    }

    fn chosen_name(&self) -> Option<String> {
        let listed = self.xrefs.listed.as_ref()?;
        listed.get(self.xrefs.chosen?).map(|x| x.name.clone())
    }

    pub(crate) fn xrefs_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Picked(_, None) => {}
            Event::Picked(Purpose::Attach, Some(path)) => {
                let name = path.file_stem().map_or_else(String::new, |s| s.to_string_lossy().into_owned());
                self.xrefs.attach = Some(Attach { path, name, mode: Mode::Attach, scale: 1.0, rotation: 0.0 });
                self.dialog = Some(Dialog::XrefAttach);
            }
            Event::Picked(Purpose::Repath, Some(path)) => {
                if let Some(name) = self.chosen_name() {
                    let path = path.display().to_string();
                    if let Some(done) = self.xref_run(XrefsOperation::Repath { name, path: path.clone() }) {
                        self.say(Level::Success, format!("“{}” artık {} dosyasından: {} nesne.", done.name, file_name(Path::new(&path)), done.objects));
                    }
                }
            }
            Event::Name(t) => {
                if let Some(a) = self.xrefs.attach.as_mut() {
                    a.name = t;
                }
            }
            Event::Mode(m) => {
                if let Some(a) = self.xrefs.attach.as_mut() {
                    a.mode = m;
                }
            }
            Event::Scale(v) => {
                if let Some(a) = self.xrefs.attach.as_mut() {
                    a.scale = v;
                }
            }
            Event::Rotation(v) => {
                if let Some(a) = self.xrefs.attach.as_mut() {
                    a.rotation = v;
                }
            }
            Event::InPlace => {
                self.dialog = None;
                self.attach_now(None);
            }
            Event::OnScreen => {
                self.dialog = None;
                self.xrefs.picking = true;
                self.say(Level::Command, ATTACH_TITLE.to_owned());
                self.field = None;
                self.snap = None;
                self.session.run(Box::new(PickPoint::new("", "Dış başvurunun yeri (kaynağın 0,0 noktası)")));
                self.with_tool(|s, cx| s.activate(cx));
            }
            Event::CancelAttach => {
                self.xrefs.attach = None;
                self.dialog = None;
            }
            Event::Choose(k) => {
                self.xrefs.chosen = Some(k);
                self.xrefs.bind_menu = false;
            }
            Event::Reload => {
                if let Some(name) = self.chosen_name()
                    && let Some(done) = self.xref_run(XrefsOperation::Reload { name })
                {
                    self.say(Level::Success, format!("“{}” yeniden yüklendi: {} nesne.", done.name, done.objects));
                }
            }
            Event::Unload => {
                if let Some(name) = self.chosen_name()
                    && let Some(done) = self.xref_run(XrefsOperation::Unload { name })
                {
                    self.say(Level::Success, format!("“{}” boşaltıldı; yerleştirmeleri kaldı, Yeniden yükle içeriğini getirir.", done.name));
                }
            }
            Event::Repath => return self.pick_xref_file(Purpose::Repath),
            Event::Detach => {
                if let Some(name) = self.chosen_name()
                    && let Some(done) = self.xref_run(XrefsOperation::Detach { name })
                {
                    self.say(Level::Success, format!("“{}” kaldırıldı: {} yerleştirmesiyle (tek adımda geri alınır).", done.name, done.removed));
                }
            }
            Event::BindMenu => self.xrefs.bind_menu = !self.xrefs.bind_menu,
            Event::Bind(mode) => {
                self.xrefs.bind_menu = false;
                if let Some(name) = self.chosen_name()
                    && let Some(done) = self.xref_run(XrefsOperation::Bind { name, mode })
                {
                    let merged = if done.merged.is_empty() {
                        String::new()
                    } else {
                        format!("; birleşen katmanlar: {}", done.merged.join(", "))
                    };
                    self.say(Level::Success, format!("“{}” sıradan blok oldu{merged}.", done.name));
                }
            }
            Event::Add => {
                self.dialog = None;
                return self.start_xref_attach();
            }
            Event::AddPath => {
                if let Picker::File(path) = &self.picker {
                    let folder = path.parent().map(Path::to_path_buf);
                    return Task::done(msg(Event::PathPicked(folder)));
                }
                return Task::perform(
                    async {
                        let folder = rfd::AsyncFileDialog::new().set_title("Arama yolu").pick_folder().await?;
                        Some(folder.path().to_path_buf())
                    },
                    |folder| msg(Event::PathPicked(folder)),
                );
            }
            Event::Section(s) => self.xrefs.section = s,
            Event::PathPicked(None) => {}
            Event::PathPicked(Some(folder)) => {
                let mut paths = self.xrefs.paths.clone();
                paths.push(folder.display().to_string());
                self.xrefs_paths(paths);
            }
            Event::RemovePath(k) => {
                let mut paths = self.xrefs.paths.clone();
                if k < paths.len() {
                    paths.remove(k);
                    self.xrefs_paths(paths);
                }
            }
            Event::Close => {
                self.xrefs = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn xrefs_view(&self) -> Element<'_, Message> {
        let listed = self.xrefs.listed.as_deref().unwrap_or_default();
        let chosen = self.xrefs.chosen.and_then(|k| listed.get(k));
        let counts = |s: XrefState| listed.iter().filter(|x| x.state == s).count();
        let line = if listed.is_empty() {
            words::text_line(Kind::Info, "Bu çizimde dış başvuru yok. Ekle… ile başka bir çizimi kaynağına bağlı ekleyin.")
        } else if counts(XrefState::Changed) + counts(XrefState::Missing) > 0 {
            words::text_line(
                Kind::Warn,
                format!(
                    "{} dış başvuru; {} değişti, {} bulunamadı. Çizim onların son yüklenen hâlini gösterir.",
                    listed.len(),
                    counts(XrefState::Changed),
                    counts(XrefState::Missing)
                ),
            )
        } else {
            words::text_line(Kind::Ok, format!("{} dış başvuru, hepsi kaynağıyla aynı.", listed.len()))
        };
        let mut list = Column::new().spacing(4);
        for (k, x) in listed.iter().enumerate() {
            let (state, kind) = state_words(x.state);
            let mut meta = vec![
                match x.kind {
                    XrefKind::Attach => "Ekle".to_owned(),
                    XrefKind::Overlay => "Bindir".to_owned(),
                },
                format!("{} nesne", x.objects),
            ];
            match &x.parent {
                Some(p) => meta.push(format!("“{p}” getirdi")),
                None => meta.push(format!("{} yerleştirme", x.inserts)),
            }
            let place = match x.via {
                AssetVia::SearchPath => format!("{} · arama yolundan bulundu", x.path),
                AssetVia::Relative => format!("{} · göreli yolundan bulundu", x.path),
                _ if !x.tried.is_empty() => format!("Denenen: {}", x.tried.join("; ")),
                _ => x.path.clone(),
            };
            let text = Column::new()
                .spacing(2)
                .push(label::body(x.name.clone()).wrapping(iced::widget::text::Wrapping::None))
                .push(label::caption(meta.join(" · ")))
                .push(label::caption(place).wrapping(iced::widget::text::Wrapping::None));
            let glyph = match x.state {
                XrefState::Loaded => "xrefAttach",
                XrefState::Unloaded => "xrefManager",
                XrefState::Missing | XrefState::Changed => "warning",
            };
            let face = row![
                container(icon(crate::icons::from_web(Some(glyph))).size(18.0)).padding(iced::Padding { left: if x.parent.is_some() { 18.0 } else { 0.0 }, ..Default::default() }),
                container(text).width(Fill).clip(true),
                state_chip(kind, state),
            ]
            .spacing(10)
            .align_y(Center);
            list = list.push(
                button(face)
                    .on_press(msg(Event::Choose(k)))
                    .padding([6, 10])
                    .width(Fill)
                    .style(style::button::row(self.xrefs.chosen == Some(k))),
            );
        }
        let top = chosen.is_some_and(|x| x.parent.is_none());
        let on = |e: Event| top.then(|| msg(e));
        let mut actions = Row::new()
            .spacing(8)
            .push(words::secondary(RELOAD, on(Event::Reload)))
            .push(words::secondary(UNLOAD, on(Event::Unload)))
            .push(words::secondary(REPATH, on(Event::Repath)))
            .push(words::secondary(DETACH, on(Event::Detach)))
            .push(words::secondary(if self.xrefs.bind_menu { "Bağla ▴" } else { "Bağla ▾" }, on(Event::BindMenu)));
        if chosen.is_some_and(|x| x.parent.is_some()) {
            actions = actions.push(label::caption("İç içe olanı onu getiren değiştirir."));
        }
        let sections = Segmented::new(SECTIONS, self.xrefs.section, |s| msg(Event::Section(s)));
        let mut body = Column::new().spacing(12).push(sections);
        match self.xrefs.section {
            Section::References => {
                body = body
                    .push(words::summary(vec![line]))
                    .push(iced::widget::scrollable(list).height(300.0))
                    .push(actions);
            }
            Section::Paths => body = body.push(self.xref_paths_view()),
            Section::Others => body = body.push(self.xref_others_view()),
        }
        if self.xrefs.bind_menu && top && self.xrefs.section == Section::References {
            body = body.push(
                container(
                    Column::new()
                        .spacing(2)
                        .push(menu_item(BIND, "Sıradan blok olur; katmanları “Dış başvuru” grubunda kalır.", Event::Bind(XrefBindMode::Bind)))
                        .push(menu_item(BIND_INSERT, "Katmanları bu çizimin aynı yoldaki katmanlarıyla birleşir.", Event::Bind(XrefBindMode::Insert))),
                )
                .padding(4)
                .style(style::container::field_box),
            );
        }
        overlay::modal(
            Frame::new(TITLE)
                .push(body)
                .aside(words::ghost(ADD, Some(msg(Event::Add))))
                .action(words::primary(CLOSE, Some(msg(Event::Close))))
                .width(720.0),
            msg(Event::Close),
        )
    }

    /// Arama yolları: the project's folders in order, each removable, and Klasör ekle….
    fn xref_paths_view(&self) -> Element<'_, Message> {
        let line = words::text_line(
            Kind::Info,
            "Kaynak kayıtlı yolunda ve açık çizimin klasöründeki göreli yolunda yoksa bu klasörlerde, sırasıyla aranır: önce göreli yoluyla, sonra dosyanın adıyla. Göreli klasör projenin dosyasının klasöründendir. Arama yolları projenin ayarıdır.",
        );
        let mut list = Column::new().spacing(4);
        if self.xrefs.paths.is_empty() {
            list = list.push(label::caption("Arama yolu yok."));
        }
        for (k, p) in self.xrefs.paths.iter().enumerate() {
            list = list.push(
                container(
                    row![
                        icon(Icon::Folder).size(16.0).tone(Tone::Muted),
                        container(label::body(p.clone()).wrapping(iced::widget::text::Wrapping::None)).width(Fill).clip(true),
                        button(icon(Icon::Close).size(12.0).tone(Tone::Muted))
                            .on_press(msg(Event::RemovePath(k)))
                            .padding(4)
                            .style(style::button::ghost),
                    ]
                    .spacing(10)
                    .align_y(Center),
                )
                .padding([4, 10])
                .style(style::container::field_box)
                .width(Fill),
            );
        }
        Column::new()
            .spacing(12)
            .push(words::summary(vec![line]))
            .push(iced::widget::scrollable(list).height(260.0))
            .push(Row::new().push(words::secondary(ADD_PATH, Some(msg(Event::AddPath)))))
            .into()
    }

    /// Diğer bağlı dosyalar: the drawing's pictures, rasters, point clouds, tables, attachments and sources, read-only.
    fn xref_others_view(&self) -> Element<'_, Message> {
        let others = &self.xrefs.others;
        let bad = others.iter().filter(|a| matches!(a.state, AssetState::Missing | AssetState::Changed)).count();
        let line = if others.is_empty() {
            words::text_line(Kind::Info, "Çizimin başka bağlı dosyası yok: resimler ve rasterler gömülü ya da hiç yok.")
        } else if bad > 0 {
            words::text_line(Kind::Warn, format!("{} bağlı dosyanın {bad} tanesi bulunamadı ya da değişti.", others.len()))
        } else {
            words::text_line(Kind::Ok, format!("{} bağlı dosya, hepsi yerinde.", others.len()))
        };
        let mut list = Column::new().spacing(4);
        for a in others {
            let what = match a.kind {
                AssetKind::Image => "Resim",
                AssetKind::Raster => "Raster",
                AssetKind::PointCloud => "Nokta bulutu",
                AssetKind::Table => "Tablo",
                AssetKind::Attachment => "Ek",
                AssetKind::Source => "Bağlı katman",
                AssetKind::Xref => "Dış başvuru",
            };
            let (state, kind) = match a.state {
                AssetState::Ok => ("Yerinde", Kind::Ok),
                AssetState::Changed => ("Değişti", Kind::Warn),
                AssetState::Missing => ("Bulunamadı", Kind::Error),
                AssetState::Unloaded => ("Boşaltıldı", Kind::Info),
                AssetState::Remote => ("Adres", Kind::Info),
                AssetState::Database => ("Veritabanı", Kind::Info),
            };
            let place = match (&a.found, a.via) {
                (Some(f), AssetVia::Relative) => format!("{f} · göreli yolundan"),
                (Some(f), _) => f.clone(),
                (None, _) if !a.tried.is_empty() => format!("Denenen: {}", a.tried.join("; ")),
                (None, _) => a.path.clone(),
            };
            let text = Column::new()
                .spacing(2)
                .push(label::body(format!("{what} · {}", a.name)).wrapping(iced::widget::text::Wrapping::None))
                .push(label::caption(place).wrapping(iced::widget::text::Wrapping::None));
            list = list.push(
                container(row![container(text).width(Fill).clip(true), state_chip(kind, state)].spacing(10).align_y(Center))
                    .padding([6, 10])
                    .width(Fill),
            );
        }
        Column::new().spacing(12).push(words::summary(vec![line])).push(iced::widget::scrollable(list).height(300.0)).into()
    }

    pub(crate) fn xref_attach_view(&self) -> Element<'_, Message> {
        let Some(a) = &self.xrefs.attach else {
            return iced::widget::text("").into();
        };
        let line = words::text_line(
            Kind::Info,
            format!(
                "“{}” bu çizime kaynağına bağlı bir blok olarak eklenir; katmanları “Dış başvuru: ad” grubunda gelir. Kaynak değişince Dış başvurular söyler, Yeniden yükle alır.",
                file_name(&a.path)
            ),
        );
        let name = focus_ring(
            text_input("Dosyanın adı", &a.name)
                .on_input(|t| msg(Event::Name(t)))
                .padding([5, 8])
                .style(style::field::input),
        );
        let numbers = row![
            words::field("Ölçek", NumberInput::new(a.scale, |v| msg(Event::Scale(v))).range(0.000_001..=1_000_000.0).step(0.5).decimals(3).point().width(120), None),
            words::field("Dönüş (°)", NumberInput::new(a.rotation, |v| msg(Event::Rotation(v))).step(15.0).decimals(2).point().width(120), None),
        ]
        .spacing(16);
        let body = Column::new()
            .spacing(12)
            .push(words::summary(vec![line]))
            .push(words::file_line(&file_name(&a.path), a.path.parent().map_or_else(String::new, |p| p.display().to_string())))
            .push(words::field("Ad", name, None))
            .push(words::field(
                "Tür",
                Segmented::new(MODES, a.mode, |m| msg(Event::Mode(m))),
                Some(match a.mode {
                    Mode::Attach => "Bu çizimi dış başvuru olarak alan çizimde de görünür.".to_owned(),
                    Mode::Overlay => "Yalnız bu çizimde görünür; bu çizimi alan çizime gelmez.".to_owned(),
                }),
            ))
            .push(numbers);
        overlay::modal(
            Frame::new(ATTACH_TITLE)
                .push(body)
                .aside(words::ghost(CANCEL, Some(msg(Event::CancelAttach))))
                .action(words::secondary(ON_SCREEN, Some(msg(Event::OnScreen))))
                .action(words::primary(IN_PLACE, Some(msg(Event::InPlace))))
                .width(560.0),
            msg(Event::CancelAttach),
        )
    }

    /// The windows' buttons by their words (a trace's `dialog` step).
    pub(crate) fn xrefs_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(RELOAD) => Some(msg(Event::Reload)),
            Control::Press(UNLOAD) => Some(msg(Event::Unload)),
            Control::Press(DETACH) => Some(msg(Event::Detach)),
            Control::Press(BIND) => Some(msg(Event::Bind(XrefBindMode::Bind))),
            Control::Press(BIND_INSERT) => Some(msg(Event::Bind(XrefBindMode::Insert))),
            Control::Press(IN_PLACE) => Some(msg(Event::InPlace)),
            Control::Press(ON_SCREEN) => Some(msg(Event::OnScreen)),
            Control::Press(CANCEL) => Some(msg(Event::CancelAttach)),
            Control::Press(ADD_PATH) => Some(msg(Event::AddPath)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The references each followed by those it brought (and theirs), as the list shows them.
fn nested_under(all: Vec<XrefInfo>) -> Vec<XrefInfo> {
    fn take(name: Option<&str>, all: &mut Vec<Option<XrefInfo>>, out: &mut Vec<XrefInfo>) {
        let here: Vec<usize> = (0..all.len())
            .filter(|k| all[*k].as_ref().is_some_and(|x| x.parent.as_deref() == name))
            .collect();
        for k in here {
            if let Some(x) = all[k].take() {
                let n = x.name.clone();
                out.push(x);
                take(Some(&n), all, out);
            }
        }
    }
    let mut slots: Vec<Option<XrefInfo>> = all.into_iter().map(Some).collect();
    let mut out = Vec::with_capacity(slots.len());
    take(None, &mut slots, &mut out);
    // One whose parent is not listed stays, at the end.
    out.extend(slots.into_iter().flatten());
    out
}

/// A reference's state: its mark and word, at the row's end.
fn state_chip<'a>(kind: Kind, words: &'a str) -> Element<'a, Message> {
    let (glyph, tone) = match kind {
        Kind::Ok => (Icon::Success, Tone::Success),
        Kind::Warn => (Icon::Warning, Tone::Warning),
        Kind::Info => (Icon::Info, Tone::Muted),
        Kind::Error => (Icon::Error, Tone::Danger),
    };
    row![icon(glyph).size(14.0).tone(tone), label::caption(words)].spacing(4).align_y(Center).into()
}

fn menu_item<'a>(title: &'a str, about: &'a str, e: Event) -> Element<'a, Message> {
    button(Column::new().spacing(2).push(label::body(title)).push(label::caption(about)))
        .on_press(msg(e))
        .padding([6, 10])
        .width(Fill)
        .style(style::button::subtle)
        .into()
}
