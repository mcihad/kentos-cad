//! Dış başvurular (docs/adr/0348 §5; `xref.attach`, `xref.manager`): another drawing file attached as a reference
//! through `cad.xrefs.edit`, and the drawing's references listed with their state through `cad.xrefs.list` —
//! Yeniden yükle, Boşalt, Yolu değiştir…, Kaldır, Bağla ▾. A changed or missing source is said when the drawing opens;
//! nothing is taken in without a command.

use std::path::{Path, PathBuf};

use iced::widget::{Column, Row, button, container, row, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_contracts::{
    CommandResult, CommandWarning, Vec2, XrefBindMode, XrefInfo, XrefKind, XrefState, XrefsEdit, XrefsEdited,
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
        let input = XrefsList { drawing: self.drawing_path() };
        match xrefs_list::plan(&ExecutionContext::new(&mut doc.model.clone()), &input) {
            CommandResult::Completed { output, .. } => Some(nested_under(output.xrefs)),
            _ => None,
        }
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
    fn xref_run(&mut self, operation: XrefsOperation) -> Option<XrefsEdited> {
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
        }
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

    /// What the drawing's references are when it opens: a changed or missing source said, never loaded silently.
    pub(crate) fn xrefs_opened(&mut self) {
        let Some(listed) = self.xrefs_listed() else { return };
        let changed: Vec<&str> = listed.iter().filter(|x| x.state == XrefState::Changed).map(|x| x.name.as_str()).collect();
        let missing: Vec<&str> = listed.iter().filter(|x| x.state == XrefState::Missing && x.parent.is_none()).map(|x| x.name.as_str()).collect();
        if !changed.is_empty() {
            self.warn(format!(
                "Dış başvuruların kaynağı değişti: {}. Çizim son yüklenen hâlini gösteriyor; yenisini almak için Ekle › Dış başvurular'da Yeniden yükle'ye basın.",
                changed.join(", ")
            ));
        }
        if !missing.is_empty() {
            self.warn(format!(
                "Dış başvuruların kaynağı bulunamadı: {}. Çizim son yüklenen hâlini gösteriyor; Ekle › Dış başvurular'da Yolu değiştir… ile dosyayı gösterin.",
                missing.join(", ")
            ));
        }
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
            let text = Column::new()
                .spacing(2)
                .push(label::body(x.name.clone()).wrapping(iced::widget::text::Wrapping::None))
                .push(label::caption(meta.join(" · ")))
                .push(label::caption(x.path.clone()).wrapping(iced::widget::text::Wrapping::None));
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
        let mut body = Column::new()
            .spacing(12)
            .push(words::summary(vec![line]))
            .push(iced::widget::scrollable(list).height(300.0))
            .push(actions);
        if self.xrefs.bind_menu && top {
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
