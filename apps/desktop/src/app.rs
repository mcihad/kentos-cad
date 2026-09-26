//! The desktop shell's state and update (docs/adr/0017): the open drawing,
//! the ribbon, the docked panels, the command line, the dialogs, and the
//! tool session drawing on the drawing (docs/adr/0021).
//!
//! Every button, shortcut and typed name runs a web command id through
//! [`App::run`]; the ones the desktop does not run yet say so instead of
//! doing nothing (CLAUDE.md §4.5). Keys, clicks and typed values go through
//! `input.rs` by ADR 0018's rules.

use std::path::PathBuf;
use std::time::Instant;

use iced::widget::operation;
use iced::{Subscription, Task, Theme, event, keyboard, window};
use serde_json::Value;

use kentos_contracts::{ResolveReason, SettingConstraint};
use kentos_interaction::{
    Clipboard, Draft, Level, Memory, Selection, Session, SnapHit, Spatial, snap_kinds,
};
use kentos_ui::icon::Icon;
use kentos_ui::theme::{self, Accent, Mode};
use kentos_ui::widget::command_line::Entry;
use kentos_ui::widget::docking::{self, Docks, Side};

use crate::catalog::{Standing, catalog};
use crate::cloud::{self, CloudState};
use crate::document::Document;
use crate::input::{Field, release_keyboard};
use crate::keys::{self, KeyPress};
use crate::opening::{self, Purpose};
use crate::recovery::{self, Recovery};
use crate::saving;
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
    /// Unsaved work a crash left: the first of `App::recovery.offers` (recovery.rs).
    Recovery,
    /// “Buluta giriş” (cloud/account.rs); its fields are `App::cloud.sign_in`.
    SignIn,
    /// “Bulut projeleri” (cloud/catalog.rs).
    Catalog,
    /// “Buluta yükle” (cloud/upload.rs).
    Upload,
    /// A database project's save conflicts (cloud/follow.rs).
    Conflicts,
    /// A file project's revision refused: someone saved first (cloud/file.rs).
    FileConflict,
    /// A copy to remove from this device whose draft holds unsent work (cloud/catalog.rs).
    RemoveCopy,
    /// The open cloud project ended for this account (deleted, archived, access taken away).
    Ended,
    /// A file exchange window (exchange/): DXF or a coordinate list, in or out.
    Exchange,
    /// Yeni proje or Proje ayarları (project/).
    Project,
    /// Başlangıç (start.rs).
    Start,
}

/// Where the app goes once the drawing on screen is left (cloud/leaving.rs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Then {
    /// A local file (Aç).
    Open,
    Close(window::Id),
    /// Bulut oturumunu kapat.
    SignOut,
    /// A cloud project from the catalog.
    OpenCloud {
        tenant: kentos_domain::Uuid,
        project: kentos_domain::Uuid,
    },
    /// Buluta yükle.
    Upload,
    /// The drawing's own cloud project again, from the server.
    Reopen,
    /// The new project the Yeni proje window built (project/new.rs).
    NewProject,
    /// A recent file (start.rs); its path waits in `App::opening_recent`.
    OpenRecent,
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
    /// One of a tool's methods from its ribbon menu (Daire: 2 nokta): the
    /// tool starts, then takes the method's option as if typed (docs/adr/0032).
    RunMethod {
        id: &'static str,
        option: &'static str,
        label: &'static str,
    },
    RibbonTab(&'static str),
    CommandInput(String),
    CommandSubmitted,
    CommandRun(String),
    CommandHistoryToggled,
    /// A tab of the bottom panel chosen: the panel opens on it.
    BottomTab(crate::bottom::BottomTab),
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
    /// A drawing being opened in stages (opening.rs, docs/adr/0030).
    Opening(opening::Event),
    /// A drawing being saved off the UI thread (saving.rs).
    Saving(saving::Event),
    /// Recovery copies of unsaved work (recovery.rs).
    Recovery(recovery::Event),
    /// The cloud: signing in, the catalog, cloud projects (cloud/, docs/adr/0041).
    Cloud(Box<cloud::Event>),
    /// File exchange: DXF and coordinate lists, in and out (exchange/).
    Exchange(Box<crate::exchange::Event>),
    /// Yeni proje and Proje ayarları (project/).
    Project(Box<crate::project::Event>),
    /// The application menu (app_menu.rs).
    AppMenu(crate::app_menu::Event),
    /// Başlangıç (start.rs).
    Start(crate::start::Event),
    /// The interface's look from the Görünüm tab (appearance.rs).
    Appearance(crate::appearance::Event),
    /// The server's answer to `server.check` (view_commands.rs).
    ServerChecked(Result<kentos_contracts::Health, String>),
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
    /// The bottom panel's tab (bottom.rs), and the warnings seen on it.
    pub bottom_tab: crate::bottom::BottomTab,
    pub seen_warnings: usize,
    pub dialog: Option<Dialog>,
    /// The drawing area's camera and scene cache.
    pub viewport: Viewport,
    /// The running tool and the last one started (kentos-interaction, docs/adr/0021).
    pub session: Session,
    /// The geometry store kept in step with the open drawing: what a click
    /// picks, a box selects and a point snaps to (docs/adr/0029).
    pub spatial: Spatial,
    /// What the drawing tools remember between runs for as long as the app
    /// lives: the last circle radius, the rectangle's rotation and corners, the
    /// regular polygon's sides (the web's static tool fields, docs/adr/0032).
    pub memory: Memory,
    /// The selected objects and the hovered one (session state, not the drawing's).
    pub selection: Selection,
    /// What Kes and Panoya kopyala put aside for Yapıştır (session state: it
    /// outlives the drawing on screen; clipboard.rs, docs/adr/0056).
    pub clipboard: Clipboard,
    /// The object snap under the pointer while a tool snaps: its marker.
    pub snap: Option<SnapHit>,
    /// The drawing (its session) and generation the store and the selection last followed.
    followed: Option<(u64, u64)>,
    /// The value field beside the cursor, while it is open (ADR 0018).
    pub field: Option<Field>,
    /// Drafting aids for new points: ortho, polar tracking, the snap aperture
    /// (the typed settings' `drafting.*`, applied by `apply_settings`).
    pub draft: Draft,
    /// Typed values open beside the cursor (`drafting.cursorInput`).
    pub cursor_input: bool,
    /// The strip over the drawing while a command runs (`drafting.commandBar`, command_bar.rs).
    pub command_bar: bool,
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
    /// The open under way, if any (opening.rs).
    pub opening: Option<opening::Opening>,
    /// The save under way, if any (saving.rs).
    pub saving: Option<saving::Saving>,
    /// Where tests make the disk fail during a save (none in the app).
    pub save_faults: saving::Faults,
    /// Recovery copies of unsaved work; kept only when `main` opens their folder.
    pub recovery: Recovery,
    /// The cloud: the account, its windows, the open project's autosave (cloud/).
    pub cloud: CloudState,
    /// The open file exchange window (exchange/).
    pub exchange: Option<crate::exchange::Window>,
    /// The open project window (project/).
    pub project: Option<crate::project::Window>,
    /// The application menu, while it is open (app_menu.rs).
    pub app_menu: Option<crate::app_menu::State>,
    /// Drawings opened or saved lately (recent.rs); kept in a file when `main` opens its folder.
    pub recent: crate::recent::RecentFiles,
    /// The recent file to open once the drawing on screen is left (start.rs).
    pub opening_recent: Option<PathBuf>,
    /// The drawing area's background (Görünüm → Çizim zemini, appearance.rs).
    pub backdrop: crate::appearance::Backdrop,
    /// The interface's typefaces and text size as last applied (appearance.rs).
    pub typography: kentos_ui::theme::typography::Typography,
    /// The window fills the screen (`view.fullscreen`, view_commands.rs).
    pub fullscreen: bool,
    /// The side panels' layout while F4 hides them, put back as it was.
    pub(crate) hidden_docks: Option<Docks<Panel>>,
    /// A server check is on its way (`server.check` waits for it).
    pub server_checking: bool,
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
        Self::start_with(path, settings, Recovery::off())
    }

    /// `start`, keeping recovery copies in `recovery`: work a crash left is offered first.
    pub fn start_with(
        path: Option<PathBuf>,
        settings: Settings,
        recovery: Recovery,
    ) -> (Self, Task<Message>) {
        // The drawing's typefaces before the first frame (docs/adr/0055).
        crate::drawing_fonts::load();
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
            bottom_tab: crate::bottom::BottomTab::default(),
            seen_warnings: 0,
            dialog: None,
            viewport: Viewport::new(),
            session: Session::new(),
            spatial: Spatial::new(),
            memory: Memory::default(),
            selection: Selection::new(),
            clipboard: Clipboard::new(),
            snap: None,
            followed: None,
            field: None,
            draft: Draft::default(),
            cursor_input: true,
            command_bar: false,
            settings,
            settings_draft: None,
            reported_failure: None,
            line_focused: false,
            modifiers: keyboard::Modifiers::default(),
            last_level: None,
            picker: Picker::Dialog,
            opening: None,
            saving: None,
            save_faults: saving::Faults::NONE,
            recovery,
            cloud: CloudState::default(),
            exchange: None,
            project: None,
            app_menu: None,
            recent: crate::recent::RecentFiles::memory(),
            opening_recent: None,
            backdrop: crate::appearance::Backdrop::default(),
            typography: kentos_ui::theme::typography::current(),
            fullscreen: false,
            hidden_docks: None,
            server_checking: false,
        };
        if !app.recovery.offers.is_empty() {
            app.dialog = Some(Dialog::Recovery);
        }
        if let Some(why) = app.recovery.why_unavailable().map(str::to_owned) {
            app.warn(format!(
                "Kaydedilmemiş çalışmanın kurtarma kopyaları tutulamıyor ({why}); KentOS çökerse kaydedilmemiş değişiklikler kaybolur. Veri klasörünün yazılabilir olduğunu denetleyin, o zamana dek sık kaydedin."
            ));
        }
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
            Some(path) => app.start_opening(path, Purpose::File),
            None => Task::none(),
        };
        (app, task)
    }

    pub fn title(&self) -> String {
        match &self.document {
            // A cloud project says its workspace too (docs/adr/0041).
            Some(doc) => match doc.cloud_source() {
                Some(source) => format!(
                    "{}{} — {} — KentOS CAD",
                    doc.name(),
                    if doc.dirty() { " •" } else { "" },
                    source.workspace
                ),
                None => format!(
                    "{}{} — KentOS CAD",
                    doc.name(),
                    if doc.dirty() { " •" } else { "" }
                ),
            },
            None => "KentOS CAD".to_owned(),
        }
    }

    pub fn theme(&self) -> Theme {
        theme::theme(self.mode, self.accent)
    }

    pub fn subscription(&self) -> Subscription<Message> {
        // While the drawing has unsaved changes, the app looks every few seconds whether a
        // recovery copy is due; a database project's unsent work goes to its device draft instead.
        let unsaved = self.recovery.on()
            && self
                .document
                .as_ref()
                .is_some_and(|d| d.dirty() && !d.is_database());
        Subscription::batch([
            event::listen_with(keys::key_event),
            window::close_requests().map(Message::CloseRequested),
            if unsaved {
                Subscription::run(recovery::ticks)
            } else {
                Subscription::none()
            },
            // The cloud's timers: autosave, the draft, following, the catalog's search.
            if self.cloud.wants_ticks() {
                Subscription::run(cloud::ticks)
            } else {
                Subscription::none()
            },
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        let task = self.handle(message);
        self.follow_document();
        self.cloud_after(Instant::now());
        task
    }

    /// After every message: the geometry store takes the drawing's changes
    /// and the selection lets go of objects that are gone (an undo, a
    /// delete, another editor's deletion taken in from the cloud; the web's
    /// `selection.retain` on `changed`), only when the drawing changed
    /// (docs/adr/0029). It keys on the generation, which changes from outside
    /// move too; the revision is the saves' (docs/adr/0040). The snap marker
    /// belongs to a running tool.
    fn follow_document(&mut self) {
        if !self.session.is_running() {
            self.snap = None;
        }
        let Some(doc) = &self.document else {
            return;
        };
        let now = (doc.session, doc.model.generation());
        if self.followed == Some(now) {
            return;
        }
        self.followed = Some(now);
        self.spatial.sync(&doc.model);
        let model = &doc.model;
        self.selection.retain(|slot| model.get(slot).is_some());
    }

    fn handle(&mut self, message: Message) -> Task<Message> {
        // What the drawing area's device can draw with, known after its first frame (AA-01).
        self.sync_device();
        // While a drawing is being opened the app takes no command (opening.rs).
        if let Some(task) = self.while_opening(&message) {
            return task;
        }
        match message {
            Message::Run(id) => return self.run(id),
            Message::RunMethod { id, option, label } => {
                return self.run_method(id, option, label);
            }
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
            Message::BottomTab(tab) => self.show_bottom(tab),
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
                match doc.cloud_source() {
                    Some(source) => self.say(
                        Level::Success,
                        format!(
                            "“{}” bulut projesi açıldı ({}, {}): {} nesne.",
                            doc.name(),
                            source.workspace,
                            cloud::words::storage_title(source.storage()),
                            doc.entity_count()
                        ),
                    ),
                    None => self.output(format!(
                        "{} açıldı: {} nesne, {} katman.",
                        doc.name(),
                        doc.entity_count(),
                        doc.layer_count()
                    )),
                }
                if doc.legacy {
                    self.output(
                        "Dosya eski biçimde (KCAD v1). Kaydet, yeni biçimde (v2) yazmak için yer sorar; eski dosyanın üzerine kendiliğinden yazmaz.",
                    );
                }
                self.show_document(*doc);
                // A drawing opened from a file goes first in the recent files (a recovered one has none).
                self.remember_file();
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
                    self.recovery_saved();
                    self.remember_file();
                }
                // Another drawing was opened meanwhile: the file is written, but it is
                // not the open drawing's file, which keeps its path and its state.
                _ => self.output(format!("Kaydedildi: {}.", written.path.display())),
            },
            Message::CloseRequested(window) => {
                // A save stopped by the window closing would leave no file (the previous one
                // stays): it finishes first.
                if let Some(s) = &self.saving {
                    let name = s.name();
                    self.warn(format!(
                        "{name} kaydediliyor; kayıt bitince pencereyi yeniden kapatın."
                    ));
                } else {
                    // Unsent cloud work to its draft first, else the question (cloud/leaving.rs).
                    return self.leave(Then::Close(window));
                }
            }
            Message::DialogConfirmed => {
                if let Some(Dialog::Unsaved(then)) = self.dialog.take() {
                    // The unsaved changes are dropped on purpose: their recovery copy goes too.
                    self.recovery.discard();
                    self.cloud.leave_failure = None;
                    return self.proceed(then);
                }
            }
            Message::DialogClosed => self.close_dialog(),
            Message::Cloud(event) => return self.cloud_event(*event),
            Message::Exchange(event) => return self.exchange_event(*event),
            Message::Project(event) => return self.project_event(*event),
            Message::AppMenu(event) => return self.app_menu_event(event),
            Message::Start(event) => return self.start_event(event),
            Message::Appearance(event) => return self.appearance_event(event),
            Message::Viewport(event) => return self.pointer(event),
            Message::Settings(edit) => return self.settings_edit(edit),
            Message::Opening(event) => return self.opening_event(event),
            Message::Saving(event) => return self.saving_event(event),
            Message::Recovery(event) => return self.recovery_event(event),
            Message::ServerChecked(answer) => self.server_checked(answer),
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
            snap: s.bool("drafting.snap"),
            snap_kinds: snap_kinds(|key| s.bool(key)),
            pick_aperture: s.number("drafting.pickAperture"),
        };
        self.cursor_input = s.bool("drafting.cursorInput");
        self.command_bar = s.bool("drafting.commandBar");
        self.apply_appearance();
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
        let _ = self.appearance_event(crate::appearance::Event::Theme(mode));
    }

    /// Puts a drawing on screen: the one there before is left (its cloud
    /// project's copy written whole and let go), and so are the tool's draft,
    /// the selection and the geometry store, which belong to the drawing they
    /// were made on.
    pub(crate) fn show_document(&mut self, doc: Document) {
        self.close_cloud_project();
        self.cancel();
        self.selected_layer = None;
        self.viewport.opened(&doc);
        self.spatial.reload(&doc.model);
        self.selection = Selection::new();
        self.followed = Some((doc.session, doc.model.generation()));
        self.document = Some(doc);
    }

    /// Runs a web command id: the desktop's handler, or a note that it is not here yet.
    pub fn run(&mut self, id: &'static str) -> Task<Message> {
        // Whatever runs a command closes the application menu (the web's `executed`).
        self.app_menu = None;
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
        if id.starts_with("cloud.") {
            return self.cloud_command(id);
        }
        if crate::exchange::COMMANDS.contains(&id) {
            return self.exchange_command(id);
        }
        if crate::project::COMMANDS.contains(&id) {
            return self.project_command(id);
        }
        if crate::clipboard::COMMANDS.contains(&id) {
            return self.clipboard_command(id);
        }
        if crate::view_commands::COMMANDS.contains(&id) {
            return self.view_command(id);
        }
        match id {
            // The drawing on screen is left first: its unsent cloud work to its draft, or the question.
            "file.open" => return self.leave(Then::Open),
            "file.start" => self.open_start(),
            // A cloud project saves to the server (cloud/file.rs); Farklı kaydet writes a local file.
            "file.save"
                if self
                    .document
                    .as_ref()
                    .is_some_and(|d| d.cloud_source().is_some()) =>
            {
                return self.save_cloud();
            }
            "file.save" => return self.save(false),
            "file.saveAs" => return self.save(true),
            "view.theme.dark" => self.choose_theme(Mode::Dark),
            "view.theme.light" => self.choose_theme(Mode::Light),
            "view.theme.toggle" => self.choose_theme(self.mode.toggled()),
            id if id.starts_with("workspace.") => self.choose_mode(id),
            "tools.options" => self.open_settings(),
            "draft.ortho" => self.toggle_session("drafting.ortho", "Orto"),
            "draft.polar" => self.toggle_session("drafting.polar", "Kutupsal izleme"),
            "draft.snap" => self.toggle_session("drafting.snap", "Kenetleme"),
            // Selecting (docs/adr/0029): the pointer selects while no command runs.
            "tool.select" => self.leave_tool(),
            "edit.deselect" => self.selection.clear(),
            "edit.selectAll" => self.select_all(),
            "edit.invertSelection" => self.invert_selection(),
            "view.ribbonCollapse" => self.ribbon_collapsed = !self.ribbon_collapsed,
            // The bottom panel (bottom.rs): F2, and the coordinate list.
            "view.bottomPanel" => self.toggle_bottom(),
            "view.coords" => self.show_bottom(crate::bottom::BottomTab::Coords),
            "view.zoomIn" => self.zoom_in(),
            "view.zoomOut" => self.zoom_out(),
            "view.zoomExtents" => self
                .viewport
                .update(viewport::Event::Extents, self.document.as_ref()),
            "view.zoomSelection" => self.zoom_selection(),
            "commandline.focus" => return operation::focus(COMMAND_INPUT),
            "help.about" => self.dialog = Some(Dialog::About),
            "help.shortcuts" => self.dialog = Some(Dialog::Shortcuts),
            // An edit, not an undo step (web: LayerStore.showAll).
            "layer.showAll" => match &mut self.document {
                Some(doc) => doc.model.show_all_layers(),
                None => self.output("Açık çizim yok."),
            },
            // Edits, not undo steps (layering.rs).
            "layer.new" => self.new_layer(),
            "layer.newGroup" => self.new_group(),
            "edit.undo" => self.undo(),
            "edit.redo" => self.step_history(false),
            "tool.confirm" => return self.confirm(),
            // Esc with nothing to cancel leaves full screen (view_commands.rs).
            "tool.cancel" if self.fullscreen && !self.cancellable() => {
                return self.toggle_fullscreen();
            }
            "tool.cancel" => self.cancel(),
            "tool.repeat" => return self.repeat_last(),
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
    /// A command's on or off state where it has one (the ribbon and its menus show it).
    pub fn checked(&self, id: &str) -> Option<bool> {
        Some(match id {
            "draft.ortho" => self.draft.ortho,
            "draft.polar" => self.draft.polar.is_some(),
            "draft.snap" => self.draft.snap,
            "view.theme.dark" => self.mode == Mode::Dark,
            "view.theme.light" => self.mode == Mode::Light,
            "view.bottomPanel" => self.command_expanded,
            "view.rightPanel" => self.right_panel_shown(),
            "view.fullscreen" => self.fullscreen,
            id if id.starts_with("workspace.") => {
                crate::catalog::mode_command(self.work_mode()) == id
            }
            _ => return None,
        })
    }

    pub fn available(&self, id: &str) -> bool {
        let doc = self.document.as_ref().map(|doc| &doc.model);
        match id {
            "edit.undo" => {
                doc.is_some_and(kentos_domain::Document::can_undo)
                    || (self.session.is_running() && self.session.point_count() > 0)
            }
            "edit.redo" => doc.is_some_and(kentos_domain::Document::can_redo),
            "edit.deselect" | "view.zoomSelection" => !self.selection.is_empty(),
            id if crate::clipboard::COMMANDS.contains(&id) => self.clipboard_available(id),
            "server.check" => !self.server_checking,
            id if id.starts_with("cloud.") => self.cloud_available(id),
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

    /// Asks for a drawing and opens it in stages, off the UI thread (opening.rs).
    pub(crate) fn open(&mut self) -> Task<Message> {
        if let Picker::File(path) = &self.picker {
            let path = path.clone();
            return self.start_opening(path, Purpose::File);
        }
        Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Çizim aç")
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |path| Message::Opening(opening::Event::Picked(path)),
        )
    }

    /// Saves to the drawing's file as `.kcad` v2, or asks where (always, with
    /// `choose`, and for a drawing opened from a v1 file, which is never
    /// written over by itself; docs/adr/0025). The drawing of this moment is
    /// written, off the UI thread (saving.rs): a change made while the file is
    /// written stays unsaved.
    fn save(&mut self, choose: bool) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Kaydedilecek çizim yok. Önce bir çizim açın (Ctrl+O).");
            return Task::none();
        };
        let title = if doc.legacy && !choose {
            "Yeni biçimde kaydet (KCAD v2)"
        } else {
            "Farklı kaydet"
        };
        let known = doc
            .path
            .clone()
            .filter(|_| !choose && !doc.legacy)
            .or(match &self.picker {
                Picker::File(path) => Some(path.clone()),
                Picker::Dialog => None,
            });
        if let Some(path) = known {
            return self.start_saving(path);
        }
        let suggested = doc.name().trim_end_matches(".kcad").to_owned();
        Task::perform(
            async move {
                let file = rfd::AsyncFileDialog::new()
                    .set_title(title)
                    .add_filter("KentOS çizimi (.kcad)", &["kcad"])
                    .set_file_name(format!("{suggested}.kcad"))
                    .save_file()
                    .await?;
                let path = file.path().to_path_buf();
                Some(if path.extension().is_some_and(|e| e == "kcad") {
                    path
                } else {
                    path.with_extension("kcad")
                })
            },
            |path| Message::Saving(saving::Event::Picked(path)),
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

    pub(crate) fn error(&mut self, text: impl Into<String>) {
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
        let _ = app.run("tool.hatch");
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

    /// A tool's method from its ribbon menu starts the tool and gives it the
    /// method's option, as the web's `runEntry` does (docs/adr/0032).
    #[test]
    fn a_method_from_the_ribbon_starts_its_tool_with_its_option() {
        let mut app = with_demo();
        let _ = app.update(Message::RunMethod {
            id: "tool.circle",
            option: "TTT",
            label: "Teğet, teğet, teğet",
        });
        assert_eq!(app.session.tool_id(), "circle");
        assert_eq!(
            app.session.prompt().text(),
            "Daire: birinci teğet çizgi, yay ya da daireyi seçin"
        );
        let _ = app.update(Message::RunMethod {
            id: "tool.arc",
            option: "M",
            label: "Merkez, başlangıç, bitiş",
        });
        assert_eq!(app.session.tool_id(), "arc");
        assert_eq!(app.session.prompt().text(), "Yay: yayın merkezini belirtin");
        assert!(
            !app.history
                .iter()
                .any(|e| matches!(e, Entry::Warning(text) if text.contains("başlatılamadı"))),
            "both methods started"
        );

        // With no drawing open the tool does not start; that says why, once.
        let (mut closed, _) = App::boot(None);
        let before = closed.history.len();
        let _ = closed.update(Message::RunMethod {
            id: "tool.circle",
            option: "2N",
            label: "2 nokta",
        });
        assert!(!closed.session.is_running());
        assert_eq!(closed.history.len(), before + 1);
        assert_eq!(
            last_output(&closed),
            "Açık çizim yok. Önce bir çizim açın (Ctrl+O)."
        );
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

    /// Another editor's deletion comes in from outside (docs/adr/0040): the
    /// revision stays, so nothing is to save, but the store, the selection
    /// and the screen follow the generation and let the object go.
    #[test]
    fn changes_from_outside_reach_the_store_and_the_selection() {
        let (mut app, _) = App::boot(None);
        let demo = with_demo().document.expect("open");
        let _ = app.update(Message::Opened(Some(Ok(Box::new(demo)))));
        let doc = app.document.as_ref().expect("open");
        let slot = kentos_domain::Slot(doc.model.entities().next().expect("an object").base().id);
        let uid = doc.model.uid(slot).expect("a persistent id");
        let (revision, objects) = (doc.model.revision(), app.spatial.len());
        assert!(objects > 0, "the store follows the drawing");
        app.selection.set([slot]);
        let _ = app.update(Message::Modifiers(keyboard::Modifiers::default()));
        assert_eq!(app.selection.len(), 1);

        let doc = app.document.as_mut().expect("open");
        doc.model
            .apply_external(kentos_domain::External {
                remove: vec![uid],
                ..Default::default()
            })
            .expect("taken in");
        let _ = app.update(Message::Modifiers(keyboard::Modifiers::default()));
        assert!(app.selection.is_empty(), "the selection lets it go");
        assert_eq!(app.spatial.len(), objects - 1, "the store follows");
        let doc = app.document.as_ref().expect("open");
        assert_eq!(doc.model.revision(), revision);
        assert!(!doc.dirty(), "a change from outside is nothing to save");
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

    /// The line and polyline tools (docs/adr/0027) start from their web
    /// shortcuts (L, P) and from their names in the command line.
    #[test]
    fn the_line_and_polyline_tools_start_from_their_keys_and_names() {
        use keyboard::key::{Code, Physical};
        use keyboard::{Key, Modifiers};
        let letter = |c: &str, code: Code| {
            Message::Key(KeyPress {
                key: Key::Character(c.into()),
                physical: Physical::Code(code),
                modifiers: Modifiers::empty(),
                text: Some(c.to_owned()),
                repeat: false,
            })
        };
        let mut app = with_demo();
        let _ = app.update(letter("l", Code::KeyL));
        assert_eq!(app.session.tool_id(), "line");
        let _ = app.run("tool.cancel");
        let _ = app.update(letter("p", Code::KeyP));
        assert_eq!(app.session.tool_id(), "polyline");
        for (name, tool) in [
            ("çizgi", "line"),
            ("PL", "polyline"),
            ("coklucizgi", "polyline"),
        ] {
            let _ = app.run("tool.cancel");
            let _ = app.update(Message::CommandRun(name.into()));
            assert_eq!(app.session.tool_id(), tool, "{name}");
        }
    }

    /// While a command runs the command line suggests its options, never
    /// another command: a name typed there goes to the tool, as on the web
    /// (docs/adr/0027), and the draft stays.
    #[test]
    fn a_running_command_owns_what_is_typed() {
        use kentos_ui::widget::command_line::{Suggested, suggested};
        let mut app = with_demo();
        assert!(!app.line_commands().is_empty());
        let _ = app.run("tool.polygon");
        assert!(app.line_commands().is_empty());
        let _ = app.update(Message::Viewport(viewport::Event::Pressed(
            iced::Point::new(10.0, 10.0),
        )));
        assert_eq!(app.session.point_count(), 1);
        // After the first corner the prompt has options; they are still suggested.
        let offered = suggested(&app.line_commands(), app.line_prompt().as_ref(), "Ya");
        assert!(
            matches!(offered.as_slice(), [Suggested::Option { label, .. }] if label == "Yay"),
            "{offered:?}"
        );
        let _ = app.update(Message::CommandInput("KA".into()));
        let _ = app.update(Message::CommandSubmitted);
        assert_eq!(app.session.tool_id(), "polygon");
        assert_eq!(app.session.point_count(), 1, "the draft stays");
        assert!(
            matches!(app.history.last(), Some(Entry::Warning(text)) if text.contains("“KA” anlaşılamadı"))
        );
    }
}
