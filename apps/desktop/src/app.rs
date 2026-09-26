//! The desktop shell's state and update (docs/adr/0017): the open drawing,
//! the ribbon, the docked panels, the command line, the dialogs, and the
//! tool session drawing on the drawing (docs/adr/0021).
//!
//! Every button, shortcut and typed name runs a web command id through
//! [`App::run`]; the ones the desktop does not run yet say so instead of
//! doing nothing (CLAUDE.md §4.5). Keys, clicks and typed values go through
//! `input.rs` by ADR 0018's rules.

use std::path::PathBuf;

use iced::widget::operation;
use iced::{Subscription, Task, Theme, event, keyboard, window};
use serde_json::Value;

use kentos_contracts::{ResolveReason, SettingConstraint};
use kentos_interaction::{Draft, Level, Session};
use kentos_ui::icon::Icon;
use kentos_ui::theme::{self, Accent, Mode};
use kentos_ui::widget::command_line::Entry;
use kentos_ui::widget::docking::{self, Docks, Side};

use crate::catalog::{Standing, catalog};
use crate::document::{self, Document};
use crate::input::{Field, release_keyboard};
use crate::keys::{self, KeyPress};
use crate::settings::Settings;
use crate::settings_view::{Edit, SettingsDraft, samples_label};
use crate::viewport::{self, Graphics, Viewport};

pub const COMMAND_INPUT: &str = "komut-satiri";

/// Width of the right dock at start (12 px body text; the showcase's).
const DOCK_WIDTH: f32 = 320.0;

/// The docked panels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    Layers,
    Properties,
}

impl Panel {
    pub fn title(self) -> &'static str {
        match self {
            Panel::Layers => "Katmanlar",
            Panel::Properties => "Özellikler",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Panel::Layers => Icon::Layers,
            Panel::Properties => Icon::Properties,
        }
    }

    fn layout() -> Docks<Panel> {
        let mut docks = Docks::new();
        docks.dock(Panel::Layers, Side::Right);
        docks.split(Panel::Properties, Side::Right);
        docks.set_size(Side::Right, DOCK_WIDTH);
        docks
    }
}

/// What waits for an answer in the middle of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialog {
    About,
    Shortcuts,
    /// The drawing has unsaved changes; asked before `then` throws them away.
    Unsaved(Then),
    /// Uygulama ayarları (`tools.options`); its draft is `App::settings_draft`.
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Then {
    Open,
    Close(window::Id),
}

/// Where the open and save dialogs are answered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Picker {
    /// The system's file dialog (`rfd`).
    #[default]
    Dialog,
    /// This file, without asking: the trace player's (the web runner's picker).
    File(PathBuf),
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Run a web command id (ribbon, shortcut, command line, dialog).
    Run(&'static str),
    RibbonTab(&'static str),
    CommandInput(String),
    CommandSubmitted,
    CommandRun(String),
    CommandHistoryToggled,
    /// Esc in the empty command line: the running command ends.
    CommandCancelled,
    /// The command line's text box took or let go of the keyboard.
    CommandFocus(bool),
    /// An option button of the running command's prompt (its key: `G`, `Enter`).
    PromptOption(&'static str),
    /// A key press no text box captured (keys.rs); routed by ADR 0018.
    Key(KeyPress),
    /// Shift, Ctrl, Alt or the logo key changed (Shift turns ortho over for a click).
    Modifiers(keyboard::Modifiers),
    Dock(docking::Event<Panel>),
    LayerSelected(String),
    LayerVisible(String),
    LayerLocked(String),
    LayerExpanded(String),
    /// A drawing read from disk, or `None` when the file dialog was cancelled.
    /// Boxed: a drawing is large and messages are moved often.
    Opened(Option<Result<Box<Document>, String>>),
    /// A save finished, or `None` when the dialog was cancelled.
    Saved(Option<Result<Written, String>>),
    CloseRequested(window::Id),
    DialogConfirmed,
    DialogClosed,
    /// The drawing area: its size, the pointer, pan, zoom and clicks.
    Viewport(viewport::Event),
    /// The settings window: a value in its draft, a preset, Kaydet, the file actions.
    Settings(Edit),
}

/// A finished save: which opened drawing, where, and the revision written.
#[derive(Debug, Clone)]
pub struct Written {
    pub session: u64,
    pub path: PathBuf,
    pub revision: u64,
}

pub struct App {
    pub document: Option<Document>,
    pub tab: &'static str,
    pub ribbon_collapsed: bool,
    pub mode: Mode,
    pub accent: Accent,
    pub docks: Docks<Panel>,
    pub selected_layer: Option<String>,
    pub history: Vec<Entry>,
    pub command_input: String,
    pub command_expanded: bool,
    pub dialog: Option<Dialog>,
    /// The drawing area's camera and scene cache.
    pub viewport: Viewport,
    /// The running tool and the last one started (kentos-interaction, docs/adr/0021).
    pub session: Session,
    /// The value field beside the cursor, while it is open (ADR 0018).
    pub field: Option<Field>,
    /// Drafting aids for new points: ortho, polar tracking, the snap aperture
    /// (the typed settings' `drafting.*`, applied by `apply_settings`).
    pub draft: Draft,
    /// Typed values open beside the cursor (`drafting.cursorInput`).
    pub cursor_input: bool,
    /// The typed settings (docs/adr/0023): kept in `ayarlar.json` when opened by `main`.
    pub settings: Settings,
    /// The settings window's draft while it is open.
    pub settings_draft: Option<SettingsDraft>,
    /// The sample-count failure already reported, so it is said once.
    reported_failure: Option<(u32, u32)>,
    /// Whether the command line's text box has the keyboard.
    pub line_focused: bool,
    pub modifiers: keyboard::Modifiers,
    /// The level of the newest message; the traces read it (ADR 0018).
    pub last_level: Option<Level>,
    pub picker: Picker,
}

impl App {
    /// The shell with settings in memory, opening `path` at once when given:
    /// what tests, snapshots and the trace player start from, never touching
    /// the user's files. `main` opens the real settings ([`App::start`]).
    pub fn boot(path: Option<PathBuf>) -> (Self, Task<Message>) {
        Self::start(path, Settings::memory())
    }

    /// The shell with these settings, opening `path` at once when given (`kentos-cad cizim.kcad`).
    pub fn start(path: Option<PathBuf>, settings: Settings) -> (Self, Task<Message>) {
        let mut app = Self {
            document: None,
            tab: catalog().tabs().nth(1).or(catalog().tabs().next()).map_or("home", |tab| tab.id),
            ribbon_collapsed: false,
            mode: Mode::Dark,
            accent: Accent::default(),
            docks: Panel::layout(),
            selected_layer: None,
            history: vec![Entry::Output(
                "KentOS CAD masaüstü hazır. Web'deki bütün komutlar şeritte; masaüstüne taşınmayanlar bunu söyler."
                    .to_owned(),
            )],
            command_input: String::new(),
            command_expanded: false,
            dialog: None,
            viewport: Viewport::new(),
            session: Session::new(),
            field: None,
            draft: Draft::default(),
            cursor_input: true,
            settings,
            settings_draft: None,
            reported_failure: None,
            line_focused: false,
            modifiers: keyboard::Modifiers::default(),
            last_level: None,
            picker: Picker::Dialog,
        };
        // The organisation's policy: no server sends one yet; a local file may stand in (docs/adr/0023).
        match Settings::policy_from_env() {
            Some(Ok(policy)) => {
                app.settings.set_policy(policy);
                app.output("Kurum politikası KENTOS_SETTINGS_POLICY dosyasından okundu (yerel deneme).");
            }
            Some(Err(error)) => app.warn(format!(
                "Kurum politikası okunamadı: {error}. Dosyayı denetleyin ya da KENTOS_SETTINGS_POLICY'yi kaldırın."
            )),
            None => {}
        }
        app.apply_settings();
        app.report_settings_open();
        let task = match path {
            Some(path) => Task::perform(
                async move { Some(Document::read(&path).map(Box::new)) },
                Message::Opened,
            ),
            None => Task::none(),
        };
        (app, task)
    }

    pub fn title(&self) -> String {
        match &self.document {
            Some(doc) => format!(
                "{}{} — KentOS CAD",
                doc.name(),
                if doc.dirty() { " •" } else { "" }
            ),
            None => "KentOS CAD".to_owned(),
        }
    }

    pub fn theme(&self) -> Theme {
        theme::theme(self.mode, self.accent)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            event::listen_with(keys::key_event),
            window::close_requests().map(Message::CloseRequested),
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        // What the drawing area's device can draw with, known after its first frame (AA-01).
        self.sync_device();
        match message {
            Message::Run(id) => return self.run(id),
            Message::RibbonTab(id) => self.tab = id,
            Message::CommandInput(text) => self.command_input = text,
            Message::CommandSubmitted => {
                let text = std::mem::take(&mut self.command_input);
                return self.submit_line(text.trim());
            }
            Message::CommandRun(name) => {
                self.command_input.clear();
                return self.run_typed(&name);
            }
            Message::CommandHistoryToggled => self.command_expanded = !self.command_expanded,
            Message::CommandCancelled => {
                self.line_focused = false;
                return self.run("tool.cancel");
            }
            Message::CommandFocus(focused) => {
                self.line_focused = focused;
                // The value field loses the keyboard to the command line (web: its blur).
                if focused {
                    self.field = None;
                }
            }
            Message::PromptOption(key) => return self.prompt_option(key),
            Message::Key(press) => return self.key(press),
            Message::Modifiers(modifiers) => self.modifiers = modifiers,
            Message::Dock(event) => self.docks.update(event),
            Message::LayerSelected(id) => {
                self.selected_layer = (self.selected_layer.as_deref() != Some(&id)).then_some(id);
            }
            // The layer tree's changes go through the document, as on the web: visibility
            // and lock are edits (unsaved) but not undo steps.
            Message::LayerVisible(id) => {
                if let Some(doc) = &mut self.document {
                    doc.model.toggle_layer_visible(&id);
                }
            }
            Message::LayerLocked(id) => {
                if let Some(doc) = &mut self.document {
                    doc.model.toggle_layer_locked(&id);
                }
            }
            Message::LayerExpanded(id) => {
                if let Some(doc) = &mut self.document
                    && let Some(expanded) = doc.find(&id).map(|n| !n.expanded)
                {
                    // Folding is kept in the file but is not an edit (web).
                    doc.model.set_layer_expanded(&id, expanded);
                }
            }
            Message::Opened(None) | Message::Saved(None) => {}
            Message::Opened(Some(Ok(doc))) => {
                self.output(format!(
                    "{} açıldı: {} nesne, {} katman.",
                    doc.name(),
                    doc.entity_count(),
                    doc.layer_count()
                ));
                // A draft belongs to the drawing it was drawn on.
                self.cancel();
                self.selected_layer = None;
                self.viewport.opened(&doc);
                self.document = Some(*doc);
            }
            Message::Opened(Some(Err(error))) | Message::Saved(Some(Err(error))) => {
                self.error(error)
            }
            Message::Saved(Some(Ok(written))) => match &mut self.document {
                Some(doc) if doc.session == written.session => {
                    doc.saved(written.path.clone(), written.revision);
                    let later = if doc.dirty() {
                        " Kayıt sürerken yapılan değişiklikler kaydedilmedi."
                    } else {
                        ""
                    };
                    self.output(format!("Kaydedildi: {}.{later}", written.path.display()));
                }
                // Another drawing was opened meanwhile: the file is written, but it is
                // not the open drawing's file, which keeps its path and its state.
                _ => self.output(format!("Kaydedildi: {}.", written.path.display())),
            },
            Message::CloseRequested(window) => {
                if self.document.as_ref().is_some_and(Document::dirty) {
                    self.dialog = Some(Dialog::Unsaved(Then::Close(window)));
                } else {
                    return window::close(window);
                }
            }
            Message::DialogConfirmed => {
                if let Some(Dialog::Unsaved(then)) = self.dialog.take() {
                    return match then {
                        Then::Open => self.open(),
                        Then::Close(window) => window::close(window),
                    };
                }
            }
            Message::DialogClosed => {
                self.dialog = None;
                self.settings_draft = None;
            }
            Message::Viewport(event) => return self.pointer(event),
            Message::Settings(edit) => return self.settings_edit(edit),
        }
        Task::none()
    }

    /// Puts the settings in use (docs/adr/0023): the tool session's drafting
    /// aids and value field, the theme; the drawing area reads `graphics()`
    /// each frame. Called after every change.
    pub(crate) fn apply_settings(&mut self) {
        let s = &self.settings;
        self.draft = Draft {
            ortho: s.bool("drafting.ortho"),
            polar: s
                .bool("drafting.polar")
                .then(|| s.number("drafting.polarIncrement")),
            snap_aperture: s.number("drafting.snapAperture"),
        };
        self.cursor_input = s.bool("drafting.cursorInput");
        self.mode = match s.effective("appearance.theme").as_str() {
            Some("light") => Mode::Light,
            _ => Mode::Dark,
        };
    }

    /// What the drawing area draws with: the effective sample count and pixel ratio.
    pub fn graphics(&self) -> Graphics {
        Graphics {
            samples: self.settings.number("graphics.msaa").max(1.0) as u32,
            hi_dpi: self.settings.bool("graphics.hiDpi"),
        }
    }

    /// Feeds the drawing area's device into the settings (TODOS.md AA-01,
    /// SET-03): the sample counts it takes, or the count it could not make
    /// targets for (AA-02); the effective value follows, the requested one stays.
    pub(crate) fn sync_device(&mut self) {
        let status = self.viewport.status();
        if status.supported.is_empty() {
            return;
        }
        let values = |counts: &mut dyn Iterator<Item = u32>| counts.map(Value::from).collect();
        let listed: Vec<String> = status.supported.iter().map(|&n| samples_label(n)).collect();
        let constraint = match &status.failure {
            Some(f) => SettingConstraint {
                allowed: values(
                    &mut status
                        .supported
                        .iter()
                        .copied()
                        .filter(|&c| c < f.requested),
                ),
                reason: ResolveReason::DeviceFailed,
                detail: format!("{}× hedefi kurulamadı ({}).", f.requested, f.error),
            },
            None => SettingConstraint {
                allowed: values(&mut status.supported.iter().copied()),
                reason: ResolveReason::DeviceUnsupported,
                detail: format!("Desteklenenler: {}.", listed.join(", ")),
            },
        };
        self.settings
            .set_constraint("graphics.msaa", Some(constraint));
        if let Some(f) = &status.failure
            && self.reported_failure != Some((f.requested, f.working))
        {
            self.reported_failure = Some((f.requested, f.working));
            self.warn(format!(
                "Kenar yumuşatma {}× bu aygıtta kurulamadı; son çalışan ayara ({}) dönüldü. Neden: {}",
                f.requested,
                samples_label(f.working),
                f.error
            ));
        }
    }

    /// What opening the settings did, in the command line: the one migration, a recovery.
    fn report_settings_open(&mut self) {
        let report = self.settings.report.clone();
        if let Some(m) = &report.migrated {
            self.output(format!(
                "Ayarlar {} dosyasından alındı ({} değer); o dosya olduğu gibi duruyor.",
                m.from, m.moved
            ));
        }
        if let Some(r) = &report.recovered {
            self.warn(format!(
                "Ayar dosyası okunamadı ya da geçersiz değer içeriyordu; eski metni {} olarak saklandı.",
                r.backup.display()
            ));
        }
    }

    /// A drafting aid of this session turned over (F8, F10): said as AutoCAD says it.
    fn toggle_session(&mut self, key: &'static str, name: &str) {
        let on = !self.settings.bool(key);
        let _ = self.settings.choose(&[(key, Value::Bool(on))]);
        self.apply_settings();
        self.output(format!("{name} {}", if on { "açık" } else { "kapalı" }));
    }

    /// The theme chosen from a command: a preference, kept.
    fn choose_theme(&mut self, mode: Mode) {
        let theme = if mode == Mode::Light { "light" } else { "dark" };
        let _ = self
            .settings
            .choose(&[("appearance.theme", Value::from(theme))]);
        self.apply_settings();
    }

    /// Runs a web command id: the desktop's handler, or a note that it is not here yet.
    pub fn run(&mut self, id: &'static str) -> Task<Message> {
        let Some(command) = catalog().get(id) else {
            self.error(format!("Komut bulunamadı: {id}"));
            return Task::none();
        };
        match command.standing {
            Standing::Ported => {}
            Standing::OnTheWeb => {
                self.output(format!(
                    "“{}” web'de var; masaüstüne henüz taşınmadı.",
                    command.title
                ));
                return Task::none();
            }
            Standing::Pending => {
                let why = command.pending_note.unwrap_or("Geliştirme aşamasında");
                self.output(format!("“{}”: {why}.", command.title));
                return Task::none();
            }
        }

        if let Some(tool) = id.strip_prefix("tool.")
            && Session::tools().contains(&tool)
        {
            return self.start_tool(tool);
        }
        match id {
            "file.open" => {
                if self.document.as_ref().is_some_and(Document::dirty) {
                    self.dialog = Some(Dialog::Unsaved(Then::Open));
                    return Task::none();
                }
                return self.open();
            }
            "file.save" => return self.save(false),
            "file.saveAs" => return self.save(true),
            "view.theme.dark" => self.choose_theme(Mode::Dark),
            "view.theme.light" => self.choose_theme(Mode::Light),
            "view.theme.toggle" => self.choose_theme(if self.mode == Mode::Light {
                Mode::Dark
            } else {
                Mode::Light
            }),
            "tools.options" => self.open_settings(),
            "draft.ortho" => self.toggle_session("drafting.ortho", "Orto"),
            "draft.polar" => self.toggle_session("drafting.polar", "Kutupsal izleme"),
            "view.ribbonCollapse" => self.ribbon_collapsed = !self.ribbon_collapsed,
            "view.zoomIn" => self.zoom_in(),
            "view.zoomOut" => self.zoom_out(),
            "view.zoomExtents" => self
                .viewport
                .update(viewport::Event::Extents, self.document.as_ref()),
            "commandline.focus" => return operation::focus(COMMAND_INPUT),
            "help.about" => self.dialog = Some(Dialog::About),
            "help.shortcuts" => self.dialog = Some(Dialog::Shortcuts),
            // An edit, not an undo step (web: LayerStore.showAll).
            "layer.showAll" => match &mut self.document {
                Some(doc) => doc.model.show_all_layers(),
                None => self.output("Açık çizim yok."),
            },
            "edit.undo" => self.undo(),
            "edit.redo" => self.step_history(false),
            "tool.confirm" => return self.confirm(),
            "tool.cancel" => self.cancel(),
            _ => self.error(format!(
                "{id}: masaüstü işleyicisi eksik (catalog::PORTED ile karşılaştırın)"
            )),
        }
        Task::none()
    }

    /// Undoes or redoes the drawing's last step, saying which (the web's
    /// “Geri alındı: Ekle”); with nothing to undo or redo, says that.
    pub(crate) fn step_history(&mut self, undo: bool) {
        let Some(doc) = &mut self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let step = if undo {
            doc.model.undo()
        } else {
            doc.model.redo()
        };
        self.output(match (step, undo) {
            (Some(label), true) => format!("Geri alındı: {label}"),
            (Some(label), false) => format!("Yinelendi: {label}"),
            (None, true) => "Geri alınacak değişiklik yok.".to_owned(),
            (None, false) => "Yinelenecek değişiklik yok.".to_owned(),
        });
    }

    /// Whether a ported command can run now: undo with a step to take, in
    /// the drawing or in the running command's draft, redo with one to take
    /// (web: `isEnabled`, `watch: [doc.canUndo, tools.prompt]`). Buttons of
    /// commands that cannot run are drawn dimmed.
    pub fn available(&self, id: &str) -> bool {
        let doc = self.document.as_ref().map(|doc| &doc.model);
        match id {
            "edit.undo" => {
                doc.is_some_and(kentos_domain::Document::can_undo)
                    || (self.session.is_running() && self.session.point_count() > 0)
            }
            "edit.redo" => doc.is_some_and(kentos_domain::Document::can_redo),
            _ => true,
        }
    }

    /// A name typed in the command line: an alias, the command's name or its
    /// title. A tool started from there gets the keyboard for the drawing.
    pub(crate) fn run_typed(&mut self, text: &str) -> Task<Message> {
        if text.is_empty() {
            return Task::none();
        }
        self.history.push(Entry::Input(text.to_owned()));
        let folded = fold(text);
        let found = catalog().commands().iter().find(|c| {
            c.aliases.iter().any(|a| fold(a) == folded) || fold(c.title) == folded || c.id == text
        });
        match found {
            Some(command) => {
                let task = self.run(command.id);
                if command.id.starts_with("tool.") && self.session.is_running() {
                    self.line_focused = false;
                    return Task::batch([task, release_keyboard()]);
                }
                task
            }
            None => {
                self.error(format!("Bilinmeyen komut: {text}. Komut adları için F1 ya da Yardım → Klavye kısayolları."));
                Task::none()
            }
        }
    }

    fn open(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            let path = path.clone();
            return Task::perform(
                async move { Some(Document::read(&path).map(Box::new)) },
                Message::Opened,
            );
        }
        Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Çizim aç")
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .pick_file()
                    .await?;
                Some(Document::read(file.path()).map(Box::new))
            },
            Message::Opened,
        )
    }

    /// Saves to the drawing's file, or asks where (always, with `choose`).
    /// What is written is the drawing as it is now, with its revision: a change
    /// made while the file is written stays unsaved.
    fn save(&mut self, choose: bool) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Kaydedilecek çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        };
        let snapshot = doc.model.to_snapshot();
        let revision = doc.model.revision();
        let session = doc.session;
        let known = doc.path.clone().filter(|_| !choose).or(match &self.picker {
            Picker::File(path) => Some(path.clone()),
            Picker::Dialog => None,
        });
        Task::perform(
            async move {
                let path = match known {
                    Some(path) => path,
                    None => {
                        let suggested = snapshot.name.trim_end_matches(".kcad").to_owned();
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Farklı kaydet")
                            .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                            .set_file_name(format!("{suggested}.kcad"))
                            .save_file()
                            .await?;
                        let path = file.path().to_path_buf();
                        if path.extension().is_some_and(|e| e == "kcad") {
                            path
                        } else {
                            path.with_extension("kcad")
                        }
                    }
                };
                Some(document::write(&snapshot, &path).map(|()| Written {
                    session,
                    path,
                    revision,
                }))
            },
            Message::Saved,
        )
    }

    /// A message in the command line, with the web's level: commands and typed
    /// values as input, information and success as output, warnings, errors.
    pub(crate) fn say(&mut self, level: Level, text: impl Into<String>) {
        let text = text.into();
        self.history.push(match level {
            Level::Command => Entry::Input(text),
            Level::Info | Level::Success => Entry::Output(text),
            Level::Warn => Entry::Warning(text),
            Level::Error => Entry::Error(text),
        });
        self.last_level = Some(level);
    }

    pub(crate) fn output(&mut self, text: impl Into<String>) {
        self.say(Level::Info, text);
    }

    pub(crate) fn warn(&mut self, text: impl Into<String>) {
        self.say(Level::Warn, text);
    }

    fn error(&mut self, text: impl Into<String>) {
        self.say(Level::Error, text);
    }
}

/// Turkish-aware case folding for command names: “çizgi”, “Cizgi” and “CIZGI” match.
fn fold(text: &str) -> String {
    text.trim()
        .chars()
        .map(|c| match c {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'I' | 'İ' | 'i' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::PORTED;

    #[test]
    fn every_ported_command_has_a_handler() {
        for id in PORTED {
            let (mut app, _) = App::boot(None);
            let _ = app.run(id);
            assert!(
                !app.history
                    .iter()
                    .any(|e| matches!(e, Entry::Error(text) if text.contains("işleyicisi eksik"))),
                "{id} is listed as ported but has no handler"
            );
        }
    }

    #[test]
    fn a_command_not_ported_says_so_and_changes_nothing() {
        let (mut app, _) = App::boot(None);
        let before = app.history.len();
        let _ = app.run("tool.circle");
        assert_eq!(app.history.len(), before + 1);
        assert!(
            matches!(app.history.last(), Some(Entry::Output(text)) if text.contains("masaüstüne henüz taşınmadı"))
        );
    }

    fn with_demo() -> App {
        let (mut app, _) = App::boot(None);
        let snapshot = kentos_contracts::DocumentSnapshotV1::from_json(include_str!(
            "../../../fixtures/document/v1/sample.json"
        ))
        .expect("the web's demo file reads");
        app.document = Some(Document::new(snapshot, None).expect("opens"));
        app
    }

    fn last_output(app: &App) -> &str {
        match app.history.last() {
            Some(Entry::Output(text)) => text,
            other => panic!("expected an output line, found {other:?}"),
        }
    }

    #[test]
    fn undo_and_redo_run_through_the_document_and_say_what_they_did() {
        let mut app = with_demo();
        assert!(!app.available("edit.undo") && !app.available("edit.redo"));
        let _ = app.run("edit.undo");
        assert_eq!(last_output(&app), "Geri alınacak değişiklik yok.");

        // An edit made on the document directly.
        let doc = app.document.as_mut().expect("open");
        let first = doc.model.entities().next().expect("an object").clone();
        let count = doc.entity_count();
        doc.model.add(first).expect("a slot");
        assert!(app.available("edit.undo"));

        let _ = app.run("edit.undo");
        assert_eq!(last_output(&app), "Geri alındı: Ekle");
        let doc = app.document.as_ref().expect("open");
        assert_eq!(doc.entity_count(), count);
        assert!(doc.dirty(), "an undo is a change to save");
        assert!(app.available("edit.redo") && !app.available("edit.undo"));

        let _ = app.run("edit.redo");
        assert_eq!(last_output(&app), "Yinelendi: Ekle");
        assert_eq!(
            app.document.as_ref().map(Document::entity_count),
            Some(count + 1)
        );
        let _ = app.run("edit.redo");
        assert_eq!(last_output(&app), "Yinelenecek değişiklik yok.");
    }

    #[test]
    fn a_save_that_finishes_after_another_drawing_was_opened_leaves_that_one_alone() {
        let mut app = with_demo();
        let first = app.document.as_ref().map(|doc| doc.session).expect("open");
        // The first drawing's save is still running when another is opened.
        let second = with_demo().document.expect("open");
        assert_ne!(second.session, first);
        let _ = app.update(Message::Opened(Some(Ok(Box::new(second)))));
        app.document.as_mut().expect("open").model.mark_unsaved();
        let _ = app.update(Message::Saved(Some(Ok(Written {
            session: first,
            path: PathBuf::from("ilk.kcad"),
            revision: 0,
        }))));
        let doc = app.document.as_ref().expect("open");
        assert_eq!(
            doc.path, None,
            "Ctrl+S must not write this drawing over the first one's file"
        );
        assert!(doc.dirty());
        assert_eq!(last_output(&app), "Kaydedildi: ilk.kcad.");
    }

    #[test]
    fn layer_changes_are_edits_but_not_undo_steps_and_folding_is_neither() {
        let mut app = with_demo();
        let group = app
            .document
            .as_ref()
            .and_then(|doc| doc.layers().iter().find(|n| !n.children.is_empty()))
            .map(|n| n.id.clone())
            .expect("the demo has a group");
        let _ = app.update(Message::LayerExpanded(group.clone()));
        let doc = app.document.as_ref().expect("open");
        assert!(doc.find(&group).is_some_and(|n| !n.expanded));
        assert!(!doc.dirty(), "folding is not an edit (web)");

        let _ = app.update(Message::LayerVisible(group.clone()));
        let _ = app.update(Message::LayerLocked(group));
        let _ = app.run("layer.showAll");
        let doc = app.document.as_ref().expect("open");
        assert!(doc.dirty());
        assert!(
            !app.available("edit.undo"),
            "visibility and lock are not undo steps (web)"
        );
    }

    #[test]
    fn the_web_keys_undo_and_redo() {
        use iced::Event;
        use keyboard::key::{Code, Physical};
        use keyboard::{Key, Location, Modifiers};
        let chord = |modifiers: Modifiers, letter: &str| {
            let message = keys::key_event(
                Event::Keyboard(keyboard::Event::KeyPressed {
                    key: Key::Character(letter.into()),
                    modified_key: Key::Character(letter.into()),
                    physical_key: Physical::Code(Code::KeyZ),
                    location: Location::Standard,
                    modifiers,
                    text: None,
                    repeat: false,
                }),
                event::Status::Ignored,
                window::Id::unique(),
            );
            let Some(Message::Key(press)) = message else {
                panic!("a key press for the router, not {message:?}");
            };
            keys::chord(&press).and_then(|chord| App::boot(None).0.shortcut(&chord))
        };
        assert_eq!(chord(Modifiers::CTRL, "z"), Some("edit.undo"));
        assert_eq!(chord(Modifiers::CTRL, "y"), Some("edit.redo"));
        assert_eq!(
            chord(Modifiers::CTRL | Modifiers::SHIFT, "Z"),
            Some("edit.redo")
        );
    }

    #[test]
    fn typed_names_find_commands_whatever_the_case_and_turkish_letters() {
        let (mut app, _) = App::boot(None);
        app.mode = Mode::Light;
        let _ = app.update(Message::CommandRun("temayı değiştir".into()));
        assert_eq!(app.mode, Mode::Dark, "the title works as a name");
        let _ = app.update(Message::CommandRun("OLMAYAN".into()));
        assert!(matches!(app.history.last(), Some(Entry::Error(_))));
    }

    #[test]
    fn a_tool_needs_an_open_drawing_and_says_so() {
        let (mut app, _) = App::boot(None);
        let _ = app.run("tool.polygon");
        assert!(!app.session.is_running());
        assert_eq!(
            last_output(&app),
            "Açık çizim yok. Önce bir çizim açın (Ctrl+O)."
        );
        let mut app = with_demo();
        let _ = app.run("tool.polygon");
        assert_eq!(app.session.tool_id(), "polygon");
        assert!(matches!(app.history.last(), Some(Entry::Input(name)) if name == "KA"));
        // Opening another drawing drops the command and its draft.
        let other = with_demo().document.expect("open");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(other)))));
        assert!(!app.session.is_running());
    }
}
