//! The launcher's state and what changes it (docs/adr/0800): the pages, the
//! apps read from their manifests and what each runs, the processes started,
//! the recent drawings, the notices, the add dialog and the settings. The
//! view is in `view/`; nothing here draws.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use iced::futures::channel::mpsc;
use iced::futures::{SinkExt, Stream};
use iced::keyboard::{self, key::Named};
use iced::{Event, Subscription, Task, Theme, event};
use kentos_launch::Module;
use kentos_launch::recent::RecentFiles;
use kentos_ui::theme::typography::{self, Typography};
use kentos_ui::theme::{Accent, Mode};
use kentos_ui::widget::Severity;

use crate::cards::{self, Status};
use crate::draft::{self, Draft, Kind};
use crate::glyphs::Glyph;
use crate::locations::Locations;
use crate::manifest::{Launch, Manifest};
use crate::notices::Notices;
use crate::pictures::Pictures;
use crate::process::Processes;
use crate::recent::{self, Row};
use crate::registry::{Folders, Registry};
use crate::resolve::{self, Finder, Found, Spec};
use crate::settings::{Settings, ThemeChoice};
use crate::tones::Tone;

/// The search field's id (Ctrl+F).
pub const SEARCH: &str = "uygulama-ara";

/// The launcher's pages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Apps,
    Recent,
    Updates,
    Downloads,
    Settings,
}

/// What is open over the page.
#[derive(Debug, Clone, PartialEq)]
pub enum Dialog {
    /// Uygulama ekle or Düzenle.
    Edit(Box<Draft>),
    /// Kaldır's question.
    Remove { id: String, name: String },
}

/// A change to the add dialog's draft.
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    Name(String),
    Note(String),
    Kind(Kind),
    Program(String),
    Args(String),
    Url(String),
    Tone(Tone),
    Glyph(Glyph),
}

#[derive(Debug, Clone)]
pub enum Message {
    Page(Page),
    Search(String),
    /// Enter in the search: the selected card, else the first that starts.
    SearchSubmit,
    FocusSearch,
    FileSearch(String),
    Hover(Option<String>),
    Select(String),
    Launch(String),
    /// Dosya aç…: a file of the app's kinds, then the app with it.
    OpenWith(String),
    FileChosen(String, Option<PathBuf>),
    /// Konumu seç…: the program's place.
    Locate(String),
    Located(String, Option<PathBuf>),
    ForgetPlace(String),
    OpenRecent(usize),
    /// Once a second while anything runs: who ended.
    Tick,
    ToggleNotices,
    CloseNotices,
    ClearNotices,
    DismissNotice(u64),
    AddApp,
    EditApp(String),
    RemoveApp(String),
    RemoveConfirmed(String),
    Draft(Field),
    PickProgram,
    ProgramPicked(Option<PathBuf>),
    SaveDraft,
    CloseDialog,
    Theme(ThemeChoice),
    TextSize(u8),
    MinimizeOnLaunch(bool),
    SystemTheme(iced::theme::Mode),
    OpenFolder(PathBuf),
    ShowManifest(String),
    Reload,
    Escape,
    /// Arrow keys over the cards: (columns, rows).
    Move(i32, i32),
}

/// The launcher.
pub struct Launcher {
    pub page: Page,
    pub registry: Registry,
    pub folders: Folders,
    pub locations: Locations,
    pub settings: Settings,
    settings_path: Option<PathBuf>,
    /// The system's theme is dark (followed while the theme is Sistem).
    pub system_dark: bool,
    finder: Box<dyn Finder>,
    /// What each app runs, by id.
    pub found: HashMap<String, Found>,
    pub processes: Processes,
    pub recent: Vec<Row>,
    recent_folder: Option<PathBuf>,
    /// Now, milliseconds since the epoch: what “5 dakika önce” is said from.
    pub now: u64,
    pub query: String,
    pub file_query: String,
    pub hovered: Option<String>,
    pub selected: Option<String>,
    pub notices: Notices,
    pub notices_open: bool,
    pub dialog: Option<Dialog>,
    pub pictures: Pictures,
    /// The grid's columns as last laid out (the arrow keys move by them).
    pub columns: std::cell::Cell<usize>,
}

/// Milliseconds since the epoch.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
}

impl Launcher {
    /// The launcher with the user's files: settings, manifests, places, recent drawings.
    pub fn boot() -> (Self, Task<Message>) {
        let settings_path = Settings::default_path();
        let settings = settings_path
            .as_deref()
            .map(Settings::load)
            .unwrap_or_default();
        typography::set(Typography {
            size: settings.text_size,
            ..Typography::DEFAULT
        });
        let mut launcher = Self::new(
            Folders::system(),
            Locations::default_path().map_or_else(Locations::memory, |p| Locations::open(&p)),
            Box::new(resolve::System),
            Processes::system(),
            RecentFiles::default_folder(),
        );
        launcher.settings = settings;
        launcher.settings_path = settings_path;
        launcher.reload();
        let theme = iced::system::theme().map(Message::SystemTheme);
        (launcher, theme)
    }

    /// A launcher over the given places (the snapshot and tests give their own).
    pub fn new(
        folders: Folders,
        locations: Locations,
        finder: Box<dyn Finder>,
        processes: Processes,
        recent_folder: Option<PathBuf>,
    ) -> Self {
        Self {
            page: Page::Apps,
            registry: Registry::default(),
            folders,
            locations,
            settings: Settings::default(),
            settings_path: None,
            system_dark: true,
            finder,
            found: HashMap::new(),
            processes,
            recent: Vec::new(),
            recent_folder,
            now: now_ms(),
            query: String::new(),
            file_query: String::new(),
            hovered: None,
            selected: None,
            notices: Notices::default(),
            notices_open: false,
            dialog: None,
            pictures: Pictures::default(),
            columns: std::cell::Cell::new(3),
        }
    }

    /// Reads the manifests and the recent drawings again; says what could not be read.
    pub fn reload(&mut self) {
        self.registry = Registry::load(&self.folders);
        self.after_registry();
        self.read_recent();
    }

    /// Uses a registry given (the snapshot's bundled apps).
    pub fn set_registry(&mut self, registry: Registry) {
        self.registry = registry;
        self.after_registry();
    }

    fn after_registry(&mut self) {
        self.now = now_ms();
        for problem in self.registry.problems.clone() {
            let (severity, text) = if problem.left_out {
                (
                    Severity::Warning,
                    format!("Uygulama yok sayıldı: {}", problem.reason),
                )
            } else {
                (Severity::Info, problem.reason.clone())
            };
            self.notices
                .push(severity, text, Some(problem.file), self.now);
        }
        self.pictures.forget_files();
        for shot in crate::pictures::Shot::ALL {
            let _ = self.pictures.shot(shot);
        }
        let mut unreadable = Vec::new();
        for app in &self.registry.apps {
            let files = [
                match &app.picture {
                    Some(crate::manifest::Picture::File(path)) => Some(path.clone()),
                    _ => None,
                },
                match &app.icon {
                    crate::manifest::Icon::File(path)
                        if path
                            .extension()
                            .is_some_and(|e| e.eq_ignore_ascii_case("png")) =>
                    {
                        Some(path.clone())
                    }
                    _ => None,
                },
            ];
            for path in files.into_iter().flatten() {
                if let Err(why) = self.pictures.file(&path) {
                    unreadable.push((why, app.origin.file_name()));
                }
            }
        }
        for (why, file) in unreadable {
            self.notices.push(
                Severity::Info,
                format!("Resim gösterilemedi: {why}."),
                file,
                self.now,
            );
        }
        self.resolve_all();
        if self
            .selected
            .as_ref()
            .is_some_and(|id| self.registry.get(id).is_none())
        {
            self.selected = None;
        }
    }

    fn resolve_all(&mut self) {
        self.found = self
            .registry
            .apps
            .iter()
            .map(|app| {
                let place = self.locations.get(&app.id);
                (
                    app.id.clone(),
                    resolve::resolve(app, place, self.finder.as_ref()),
                )
            })
            .collect();
    }

    fn read_recent(&mut self) {
        self.now = now_ms();
        let list = self
            .recent_folder
            .as_deref()
            .map(RecentFiles::open)
            .unwrap_or_default();
        self.recent = recent::rows(
            list.list(),
            self.now,
            Path::exists,
            crate::files::home().as_deref(),
        );
    }

    /// The recent drawings given (the snapshot's).
    pub fn set_recent(&mut self, rows: Vec<Row>) {
        self.recent = rows;
    }

    /// The theme shown.
    pub fn mode(&self) -> Mode {
        match self.settings.theme {
            ThemeChoice::Light => Mode::Light,
            ThemeChoice::Dark => Mode::Dark,
            ThemeChoice::System if self.system_dark => Mode::Dark,
            ThemeChoice::System => Mode::Light,
        }
    }

    pub fn theme(&self) -> Theme {
        kentos_ui::theme::theme(self.mode(), Accent::default())
    }

    /// An app's state.
    pub fn status(&self, app: &Manifest) -> Status {
        let found = self.found.get(&app.id).unwrap_or(&Found::Soon);
        cards::status(found, self.processes.count(&app.id))
    }

    /// The apps the search lets through, in the cards' order.
    pub fn visible(&self) -> Vec<&Manifest> {
        self.registry
            .apps
            .iter()
            .filter(|a| cards::matches(a, &self.query))
            .collect()
    }

    /// The card drawn as the one to start: the one under the pointer, else the selected one.
    pub fn hot(&self) -> Option<&str> {
        self.hovered.as_deref().or(self.selected.as_deref())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![event::listen_with(key)];
        if !self.processes.is_empty() {
            subscriptions.push(Subscription::run(ticks));
        }
        if self.settings.theme == ThemeChoice::System {
            subscriptions.push(iced::system::theme_changes().map(Message::SystemTheme));
        }
        Subscription::batch(subscriptions)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Page(page) => {
                self.page = page;
                self.notices_open = false;
                if page == Page::Recent {
                    self.read_recent();
                }
            }
            Message::Search(query) => self.query = query,
            Message::SearchSubmit => {
                let target = self
                    .selected
                    .clone()
                    .filter(|id| self.visible().iter().any(|a| &a.id == id))
                    .or_else(|| {
                        self.visible()
                            .into_iter()
                            .find(|a| self.status(a).starts())
                            .map(|a| a.id.clone())
                    });
                if let Some(id) = target {
                    return self.launch(&id, None);
                }
            }
            Message::FocusSearch => {
                if self.page == Page::Apps || self.page == Page::Recent {
                    return iced::widget::operation::focus(iced::widget::Id::new(SEARCH));
                }
            }
            Message::FileSearch(query) => self.file_query = query,
            Message::Hover(id) => self.hovered = id,
            Message::Select(id) => self.selected = Some(id),
            Message::Launch(id) => {
                self.selected = Some(id.clone());
                return self.launch(&id, None);
            }
            Message::OpenWith(id) => return self.pick_file(id),
            Message::FileChosen(id, Some(path)) => {
                let refused = self.registry.get(&id).and_then(|app| {
                    (!resolve::opens(app, &path)).then(|| {
                        format!(
                            "{} “{}” dosyasını açmaz; açtıkları: {}.",
                            app.name,
                            path.file_name()
                                .map_or_else(String::new, |n| n.to_string_lossy().into_owned()),
                            app.files.join(", ")
                        )
                    })
                });
                if let Some(sentence) = refused {
                    self.say(Severity::Warning, sentence, None);
                    return Task::none();
                }
                return self.launch(&id, Some(&path));
            }
            Message::FileChosen(_, None)
            | Message::Located(_, None)
            | Message::ProgramPicked(None) => {}
            Message::Locate(id) => return self.pick_program(id),
            Message::Located(id, Some(path)) => {
                if let Err(e) = self.locations.set(&id, path) {
                    self.say(
                        Severity::Error,
                        format!("Konum kaydedilemedi ({e}). Ayar klasörünün iznini denetleyin."),
                        None,
                    );
                }
                self.resolve_all();
            }
            Message::ForgetPlace(id) => {
                if let Err(e) = self.locations.clear(&id) {
                    self.say(Severity::Error, format!("Konum silinemedi ({e})."), None);
                }
                self.resolve_all();
            }
            Message::OpenRecent(index) => return self.open_recent(index),
            Message::Tick => {
                self.now = now_ms();
                for ended in self.processes.poll() {
                    self.say(Severity::Error, ended.sentence, None);
                }
            }
            Message::ToggleNotices => {
                self.notices_open = !self.notices_open;
                if self.notices_open {
                    self.now = now_ms();
                    self.notices.read_all();
                }
            }
            Message::CloseNotices => self.notices_open = false,
            Message::ClearNotices => self.notices.clear(),
            Message::DismissNotice(id) => self.notices.dismiss(id),
            Message::AddApp => {
                self.notices_open = false;
                self.dialog = Some(Dialog::Edit(Box::new(Draft::new())));
            }
            Message::EditApp(id) => {
                if let Some(app) = self.registry.get(&id) {
                    self.dialog = Some(Dialog::Edit(Box::new(Draft::of(app))));
                }
            }
            Message::RemoveApp(id) => {
                if let Some(app) = self.registry.get(&id) {
                    self.dialog = Some(Dialog::Remove {
                        id,
                        name: app.name.clone(),
                    });
                }
            }
            Message::RemoveConfirmed(id) => {
                self.dialog = None;
                if let Some(app) = self.registry.get(&id).cloned() {
                    match draft::remove(&app) {
                        Ok(()) => {
                            let _ = self.locations.clear(&id);
                            self.reload();
                        }
                        Err(why) => self.say(Severity::Error, why, app.origin.file_name()),
                    }
                }
            }
            Message::Draft(field) => {
                if let Some(Dialog::Edit(draft)) = &mut self.dialog {
                    edit(draft, field);
                }
            }
            Message::PickProgram => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Programı seçin")
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    Message::ProgramPicked,
                );
            }
            Message::ProgramPicked(Some(path)) => {
                if let Some(Dialog::Edit(draft)) = &mut self.dialog {
                    draft.program = path.to_string_lossy().into_owned();
                    if draft.name.trim().is_empty() {
                        draft.name = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().into_owned())
                            .unwrap_or_default();
                    }
                }
            }
            Message::SaveDraft => return self.save_draft(),
            Message::CloseDialog => self.dialog = None,
            Message::Theme(choice) => {
                self.settings.theme = choice;
                self.save_settings();
                if choice == ThemeChoice::System {
                    return iced::system::theme().map(Message::SystemTheme);
                }
            }
            Message::TextSize(size) => {
                self.settings.text_size = f32::from(size);
                typography::set(Typography {
                    size: self.settings.text_size,
                    ..Typography::DEFAULT
                });
                self.save_settings();
            }
            Message::MinimizeOnLaunch(on) => {
                self.settings.minimize_on_launch = on;
                self.save_settings();
            }
            Message::SystemTheme(mode) => self.system_dark = mode != iced::theme::Mode::Light,
            Message::OpenFolder(folder) => {
                if let Err(e) = std::fs::create_dir_all(&folder) {
                    self.say(
                        Severity::Error,
                        format!("Klasör açılamadı ({e})."),
                        Some(folder.display().to_string()),
                    );
                } else if let Err(why) = self.processes.open("klasor", "Klasör", folder.as_os_str())
                {
                    self.say(Severity::Error, why, None);
                }
            }
            Message::ShowManifest(id) => {
                let folder = self
                    .registry
                    .get(&id)
                    .and_then(|a| a.origin.base().map(Path::to_path_buf));
                if let Some(folder) = folder {
                    return self.update(Message::OpenFolder(folder));
                }
            }
            Message::Reload => self.reload(),
            Message::Escape => {
                if self.dialog.is_some() {
                    self.dialog = None;
                } else if self.notices_open {
                    self.notices_open = false;
                } else if !self.query.is_empty() && self.page == Page::Apps {
                    self.query.clear();
                }
            }
            Message::Move(dx, dy) => {
                if self.dialog.is_none() && self.page == Page::Apps {
                    self.move_selection(dx, dy);
                }
            }
        }
        Task::none()
    }

    /// Says something in the notices.
    pub fn say(&mut self, severity: Severity, text: String, detail: Option<String>) {
        self.now = now_ms();
        self.notices.push(severity, text, detail, self.now);
    }

    fn launch(&mut self, id: &str, file: Option<&Path>) -> Task<Message> {
        let Some(app) = self.registry.get(id) else {
            return Task::none();
        };
        let name = app.name.clone();
        let result = match self.found.get(id) {
            Some(Found::Program(spec)) => {
                let spec = spec.with_file(file);
                self.processes.start(id, &name, &spec)
            }
            Some(Found::Url(url)) => {
                let url = url.clone();
                self.processes.open(id, &name, std::ffi::OsStr::new(&url))
            }
            Some(Found::Missing(why)) => Err(format!("{name} başlatılamadı: {why}")),
            Some(Found::Soon) | None => return Task::none(),
        };
        match result {
            Ok(()) if self.settings.minimize_on_launch => {
                iced::window::latest().and_then(|window| iced::window::minimize(window, true))
            }
            Ok(()) => Task::none(),
            Err(sentence) => {
                self.say(Severity::Error, sentence, None);
                Task::none()
            }
        }
    }

    fn pick_file(&self, id: String) -> Task<Message> {
        let Some(app) = self.registry.get(&id) else {
            return Task::none();
        };
        let title = format!("{} ile açılacak dosya", app.name);
        let files = app.files.clone();
        let name = app.name.clone();
        Task::perform(
            async move {
                let mut dialog = rfd::AsyncFileDialog::new().set_title(title);
                if !files.is_empty() {
                    dialog = dialog.add_filter(format!("{name} dosyaları"), &files);
                }
                dialog.pick_file().await.map(|f| f.path().to_path_buf())
            },
            move |path| Message::FileChosen(id.clone(), path),
        )
    }

    fn pick_program(&self, id: String) -> Task<Message> {
        let Some(app) = self.registry.get(&id) else {
            return Task::none();
        };
        let title = format!("{} programının yeri", app.name);
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title(title)
                    .pick_file()
                    .await
                    .map(|f| f.path().to_path_buf())
            },
            move |path| Message::Located(id.clone(), path),
        )
    }

    /// The program a recent drawing opens in: its module's own program (docs/adr/0802), else `kentos-cad` alone
    /// for a drawing kept before modules.
    pub fn recent_spec(&self, row: &Row) -> Result<Spec, String> {
        if let Some(module) = row.module {
            let app = self
                .registry
                .apps
                .iter()
                .find(|a| a.launch == Some(Launch::Kentos(module)));
            if let Some(app) = app {
                return match self.found.get(&app.id) {
                    Some(Found::Program(spec)) => Ok(spec.with_file(Some(&row.path))),
                    Some(Found::Missing(why)) => Err(format!("{} başlatılamadı: {why}", app.name)),
                    _ => Err(format!("{} başlatılamaz.", app.name)),
                };
            }
        }
        let cad = self.finder.find(resolve::KENTOS_CAD).ok_or_else(|| {
            "kentos-cad bulunamadı (launcher'ın yanında ve PATH'te arandı). KentOS'u kurun."
                .to_owned()
        })?;
        Ok(Spec {
            program: cad,
            args: kentos_launch::args::command_line(row.module, Some(&row.path)),
            ..Spec::default()
        })
    }

    fn open_recent(&mut self, index: usize) -> Task<Message> {
        let Some(row) = self.recent.get(index).cloned() else {
            return Task::none();
        };
        if row.missing {
            return Task::none();
        }
        let id = match row.module {
            Some(m) => format!("kentos-{}", module_word(m)),
            None => resolve::KENTOS_CAD.to_owned(),
        };
        let result = self
            .recent_spec(&row)
            .and_then(|spec| self.processes.start(&id, &row.name, &spec));
        match result {
            Ok(()) if self.settings.minimize_on_launch => {
                iced::window::latest().and_then(|window| iced::window::minimize(window, true))
            }
            Ok(()) => Task::none(),
            Err(sentence) => {
                self.say(
                    Severity::Error,
                    sentence,
                    Some(row.path.display().to_string()),
                );
                Task::none()
            }
        }
    }

    fn save_draft(&mut self) -> Task<Message> {
        let Some(Dialog::Edit(draft)) = &mut self.dialog else {
            return Task::none();
        };
        draft.tried = true;
        let Some(folder) = self.folders.user.clone() else {
            draft.refusal = Some(
                "Kullanıcının uygulama klasörü bilinmiyor (HOME ya da XDG_DATA_HOME yok)."
                    .to_owned(),
            );
            return Task::none();
        };
        let registry = &self.registry;
        let Some(app) = draft.manifest(&folder, |id| registry.get(id).is_some()) else {
            return Task::none();
        };
        match draft::write(&app) {
            Ok(_) => {
                let id = app.id.clone();
                self.dialog = None;
                self.reload();
                self.selected = Some(id);
            }
            Err(why) => draft.refusal = Some(why),
        }
        Task::none()
    }

    fn save_settings(&mut self) {
        if let Some(path) = &self.settings_path
            && let Err(e) = self.settings.save(path)
        {
            let file = path.display().to_string();
            self.say(
                Severity::Warning,
                format!("Ayarlar kaydedilemedi ({e}); bu oturumda geçerli."),
                Some(file),
            );
        }
    }

    fn move_selection(&mut self, dx: i32, dy: i32) {
        let ids: Vec<String> = self.visible().iter().map(|a| a.id.clone()).collect();
        if ids.is_empty() {
            return;
        }
        let columns = self.columns.get().max(1) as i32;
        let at = self
            .selected
            .as_ref()
            .and_then(|id| ids.iter().position(|i| i == id));
        let next = match at {
            None => 0,
            Some(at) => (at as i32 + dx + dy * columns).clamp(0, ids.len() as i32 - 1) as usize,
        };
        self.selected = Some(ids[next].clone());
        self.hovered = None;
    }
}

/// A module's word in an id (`kentos-mimari`, `kentos-gis`).
fn module_word(module: Module) -> &'static str {
    match module {
        Module::Architecture => "mimari",
        Module::Map => "harita",
        Module::Gis => "gis",
    }
}

fn edit(draft: &mut Draft, field: Field) {
    draft.refusal = None;
    match field {
        Field::Name(v) => draft.name = v,
        Field::Note(v) => draft.note = v,
        Field::Kind(v) => draft.kind = v,
        Field::Program(v) => draft.program = v,
        Field::Args(v) => draft.args = v,
        Field::Url(v) => draft.url = v,
        Field::Tone(v) => draft.tone = v,
        Field::Glyph(v) => {
            draft.glyph = v;
            draft.icon_file = None;
        }
    }
}

/// The keys the launcher answers when no field takes them.
fn key(event: Event, status: event::Status, _window: iced::window::Id) -> Option<Message> {
    let Event::Keyboard(keyboard::Event::KeyPressed { key, modifiers, .. }) = event else {
        return None;
    };
    match key.as_ref() {
        keyboard::Key::Named(Named::Escape) => Some(Message::Escape),
        keyboard::Key::Character("f") if modifiers.command() => Some(Message::FocusSearch),
        _ if status == event::Status::Captured => None,
        keyboard::Key::Named(Named::ArrowLeft) => Some(Message::Move(-1, 0)),
        keyboard::Key::Named(Named::ArrowRight) => Some(Message::Move(1, 0)),
        keyboard::Key::Named(Named::ArrowUp) => Some(Message::Move(0, -1)),
        keyboard::Key::Named(Named::ArrowDown) => Some(Message::Move(0, 1)),
        keyboard::Key::Named(Named::Enter) => Some(Message::SearchSubmit),
        _ => None,
    }
}

/// A tick a second, from a thread of its own, while the subscription lives.
fn ticks() -> impl Stream<Item = Message> {
    let (mut out, ticks) = mpsc::channel(1);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_secs(1));
            if iced::futures::executor::block_on(out.send(Message::Tick)).is_err() {
                break;
            }
        }
    });
    ticks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolve::tests::Fixed;

    fn launcher(finder: Fixed) -> Launcher {
        let mut launcher = Launcher::new(
            Folders::default(),
            Locations::memory(),
            Box::new(finder),
            Processes::quiet(),
            None,
        );
        launcher.set_registry(Registry::builtin());
        launcher
    }

    fn kentos(names: &[&'static str]) -> Fixed {
        Fixed(
            names
                .iter()
                .map(|n| (*n, PathBuf::from(format!("/opt/kentos/{n}"))))
                .collect(),
        )
    }

    #[test]
    fn states_follow_what_is_found_and_the_summary_counts_them() {
        let launcher = launcher(kentos(&["kentos-mimari", "kentos-harita", "kentos-gis"]));
        let states: Vec<Status> = launcher
            .registry
            .apps
            .iter()
            .map(|a| launcher.status(a))
            .collect();
        assert_eq!(
            states,
            [
                Status::Installed,
                Status::Installed,
                Status::Installed,
                Status::Soon,
                Status::Soon,
                Status::Soon
            ]
        );
        assert_eq!(
            cards::Counts::of(states).parts(),
            ["6 uygulama", "3 kurulu", "3 yakında"]
        );
        let none = super::tests::launcher(Fixed(HashMap::new()));
        assert_eq!(none.status(&none.registry.apps[0]), Status::Missing);
        // The one KentOS program alone makes every module installed (docs/adr/0802 §1): kentos-cad as each.
        let cad = super::tests::launcher(kentos(&["kentos-cad"]));
        for i in 0..3 {
            assert_eq!(cad.status(&cad.registry.apps[i]), Status::Installed);
        }
    }

    #[test]
    fn a_recent_drawing_opens_in_its_module() {
        let launcher = launcher(kentos(&["kentos-cad", "kentos-mimari", "kentos-gis"]));
        let row = |module: Option<Module>| Row {
            name: "a.kcad".into(),
            folder: "~".into(),
            path: PathBuf::from("/ev/a.kcad"),
            ago: "az önce".into(),
            info: String::new(),
            module,
            missing: false,
        };
        let os =
            |args: &[&str]| -> Vec<std::ffi::OsString> { args.iter().map(Into::into).collect() };
        let mimari = launcher
            .recent_spec(&row(Some(Module::Architecture)))
            .expect("opens");
        assert_eq!(mimari.program, PathBuf::from("/opt/kentos/kentos-mimari"));
        assert_eq!(mimari.args, os(&["/ev/a.kcad"]));
        // No kentos-harita beside it: the one KentOS program as KentOS Harita (docs/adr/0802 §1).
        let harita = launcher
            .recent_spec(&row(Some(Module::Map)))
            .expect("kentos-cad opens it as KentOS Harita");
        assert_eq!(harita.program, PathBuf::from("/opt/kentos/kentos-cad"));
        assert_eq!(harita.args, os(&["--modul", "harita", "/ev/a.kcad"]));
        let gis = launcher
            .recent_spec(&row(Some(Module::Gis)))
            .expect("opens");
        assert_eq!(gis.program, PathBuf::from("/opt/kentos/kentos-gis"));
        assert_eq!(
            gis.args,
            os(&["/ev/a.kcad"]),
            "its own program: the file alone"
        );
        let plain = launcher.recent_spec(&row(None)).expect("opens");
        assert_eq!(plain.args, os(&["/ev/a.kcad"]));
        let nothing = super::tests::launcher(Fixed(HashMap::new()));
        assert!(
            nothing
                .recent_spec(&row(None))
                .expect_err("no program")
                .contains("kentos-cad bulunamadı")
        );
    }

    #[test]
    fn the_arrows_move_the_selection_by_the_grid() {
        let mut launcher = launcher(kentos(&["kentos-cad"]));
        launcher.columns.set(3);
        let _ = launcher.update(Message::Move(1, 0));
        assert_eq!(
            launcher.selected.as_deref(),
            Some("kentos-mimari"),
            "the first at first"
        );
        let _ = launcher.update(Message::Move(0, 1));
        assert_eq!(
            launcher.selected.as_deref(),
            Some("kentos-yol"),
            "down a row of three"
        );
        let _ = launcher.update(Message::Move(1, 0));
        assert_eq!(
            launcher.selected.as_deref(),
            Some("kentos-3d"),
            "along the row"
        );
        let _ = launcher.update(Message::Move(0, -1));
        assert_eq!(
            launcher.selected.as_deref(),
            Some("kentos-harita"),
            "up a row"
        );
        let _ = launcher.update(Message::Move(5, 5));
        assert_eq!(
            launcher.selected.as_deref(),
            Some("kentos-afet"),
            "the last at most"
        );
        let _ = launcher.update(Message::Move(-5, -5));
        assert_eq!(
            launcher.selected.as_deref(),
            Some("kentos-mimari"),
            "the first at least"
        );
        let _ = launcher.update(Message::Search("gis".into()));
        let _ = launcher.update(Message::Move(-1, 0));
        assert_eq!(
            launcher.selected.as_deref(),
            Some("kentos-gis"),
            "among the cards shown"
        );
        let _ = launcher.update(Message::Escape);
        assert!(launcher.query.is_empty(), "Esc clears the search");
    }

    #[test]
    fn a_missing_program_is_said_when_started() {
        let mut launcher = launcher(Fixed(HashMap::new()));
        let _ = launcher.update(Message::Launch("kentos-harita".into()));
        let said: Vec<String> = launcher
            .notices
            .newest_first()
            .map(|n| n.text.clone())
            .collect();
        assert_eq!(said.len(), 1);
        assert!(
            said[0].starts_with("KentOS Harita başlatılamadı: KentOS Harita bulunamadı"),
            "{}",
            said[0]
        );
        for soon in ["kentos-yol", "kentos-3d", "kentos-afet"] {
            let _ = launcher.update(Message::Launch(soon.into()));
        }
        assert_eq!(
            launcher.notices.len(),
            1,
            "a soon app starts nothing and says nothing"
        );
    }
}
