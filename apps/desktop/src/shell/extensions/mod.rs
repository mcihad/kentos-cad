//! Eklentiler (docs/adr/8057 §3; plan/eklenti.md ADIM 18): the user's extension registry in the window.
//!
//! The manager lists the installed extensions with their state, trust and source, and the packages of a catalog
//! folder (an organisation's share or an offline disk, `kentos kek index`). A package is reviewed before it is
//! installed: its description, change notes, dependencies, the capabilities it asks for with their reasons and what
//! changed since the installed version; the user grants what it may do. Installing, enabling, disabling, going back and
//! removing are the registry's (kentos-extension-host); this window only asks and says what happened.
//!
//! The registry is the user's (`<veri>/kentos-cad/eklentiler`); `KENTOS_EXTENSIONS_DIR` points elsewhere, so tests and
//! real-window sessions never touch the user's own. A start without extensions (`--eklentisiz`, `KENTOS_EKLENTISIZ`, or
//! "bir sonraki açılışta eklentisiz başlat" from this window) activates nothing.

mod bisect;
mod browse;
pub(crate) mod environment;
mod problems;
mod ribbon;
pub(crate) mod runner;
mod search;
pub(crate) mod tools;
mod view;

pub(crate) use search::ExtHit;

use std::path::{Path, PathBuf};

use iced::Task;
use kentos_extension_host::{
    Choice, Grant, Options, Record, Registry, Review, Source, SourceKind, State,
};
use kentos_extensions::catalog::{self, Catalog, CatalogEntry};
use kentos_extensions::manifest::Capability;
use kentos_extensions::version::Version;
use kentos_extensions::{Host, Limits};
use kentos_interaction::Level;

use crate::app::{App, Dialog, Message};

/// The flag file that makes the next start one without extensions; removed by that start.
const SAFE_NEXT: &str = "eklentisiz-baslat";
/// The catalog folder opened last, opened again at the next start (an organisation's share stays the same).
const CATALOG_FOLDER: &str = "katalog-klasoru";

pub(crate) use browse::Part;

/// A catalog folder opened in the window.
pub(crate) struct CatalogFolder {
    pub(crate) folder: PathBuf,
    pub(crate) catalog: Catalog,
    /// Its entries new since its previous opening (`id@version`, browse.rs).
    pub(crate) fresh: std::collections::BTreeSet<String>,
}

/// A package being reviewed before it is installed.
pub(crate) struct Pending {
    pub(crate) bytes: Vec<u8>,
    pub(crate) source: Source,
    /// The file's name, as the window shows it.
    pub(crate) file: String,
    pub(crate) review: Review,
    pub(crate) changelog: Option<String>,
    /// The capabilities the user grants, of those the package asks for.
    pub(crate) granted: Vec<Capability>,
    /// The Python contributions are wanted.
    pub(crate) python: bool,
    /// The package has Python contributions at all.
    pub(crate) has_python: bool,
    pub(crate) replace_source: bool,
}

/// The extensions' state in the window.
pub(crate) struct Extensions {
    /// None while the program runs without one: a test, a computer without a data folder, a registry that did not
    /// open (`missing` says why).
    pub(crate) registry: Option<Registry>,
    pub(crate) missing: Option<String>,
    /// Where the registry is.
    pub(crate) root: Option<PathBuf>,
    /// This start activated nothing.
    pub(crate) safe_mode: bool,
    pub(crate) part: Part,
    /// The extension (id) or the catalog entry (file) the details show.
    pub(crate) chosen: Option<String>,
    pub(crate) catalog: Option<CatalogFolder>,
    pub(crate) review: Option<Pending>,
    /// The last action's outcome: an error or not, and its words.
    pub(crate) note: Option<(bool, String)>,
    /// The extensions' ribbon tabs, rebuilt from the registry when it changes (ribbon.rs).
    pub(crate) tabs: Vec<ribbon::ExtTab>,
    /// The extensions' right-click menu items, rebuilt with the tabs.
    pub(crate) menus: Vec<ribbon::ExtMenuItem>,
    /// The chosen extension's whole log is shown, not its newest lines.
    pub(crate) full_log: bool,
    /// The extensions with a problem standing or kept from running, as the status bar's cell says them; worked out
    /// when the registry or a log changes, not at every frame (problems.rs).
    pub(crate) troubles: Vec<problems::Trouble>,
    /// The button Komut ara revealed, outlined for a moment (search.rs).
    pub(crate) flash: Option<ExtHit>,
    /// Sorunlu eklentiyi bul's window over the manager while it is open (bisect.rs).
    pub(crate) bisect: Option<bisect::BisectStep>,
    /// Each installed extension's own icon (its SVG in the package), read when the registry changes, not at every
    /// frame; the puzzle where none.
    pub(crate) icons: std::collections::BTreeMap<String, kentos_ui::icon::Icon>,
    /// The word the lists are filtered by (browse.rs).
    pub(crate) query: String,
    /// The program the lists are filtered by; none: every program.
    pub(crate) program: Option<kentos_extensions::manifest::Module>,
    /// Each extension's Python environment, by id, once looked for (environment.rs).
    pub(crate) envs: std::collections::BTreeMap<String, environment::EnvState>,
    /// The extensions' processes, by id: one an extension version, kept between its runs (runner.rs).
    pub(crate) runners: std::collections::BTreeMap<String, runner::Runner>,
    /// The runs' and the processes' numbers.
    pub(crate) run_counter: u64,
    pub(crate) runner_generation: u64,
    /// The last run's result: the extension, its title, what it returned.
    pub(crate) result: Option<(String, String, serde_json::Value)>,
    /// Their İşlemler tools: listed from the registry, run in the extension versions' processes (tools.rs).
    pub(crate) tools: tools::ExtTools,
}

impl Default for Extensions {
    fn default() -> Extensions {
        Extensions {
            registry: None,
            missing: Some("Bu oturumda eklenti kayıt defteri açılmadı.".to_owned()),
            root: None,
            safe_mode: false,
            part: Part::Installed,
            chosen: None,
            catalog: None,
            review: None,
            note: None,
            tabs: Vec::new(),
            menus: Vec::new(),
            full_log: false,
            troubles: Vec::new(),
            flash: None,
            bisect: None,
            icons: std::collections::BTreeMap::new(),
            query: String::new(),
            program: None,
            envs: std::collections::BTreeMap::new(),
            runners: std::collections::BTreeMap::new(),
            run_counter: 0,
            runner_generation: 0,
            result: None,
            tools: tools::ExtTools::default(),
        }
    }
}

/// This KentOS as a host for packages.
pub(crate) fn host() -> Host {
    Host::this(Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_else(|| Version::new(0, 1, 0)))
}

fn msg(event: Event) -> Message {
    Message::Extensions(event)
}

#[derive(Clone, Debug)]
pub(crate) enum Event {
    Part(Part),
    Choose(String),
    /// An extension's Python environment (environment.rs).
    Env(environment::EnvEvent),
    /// An extension's contribution run in its process: extension, kind (`command`), local id (runner.rs).
    Run(String, String, String),
    /// Its environment found (or why not): extension, kind, local id, what starting it needs.
    Launch(String, String, String, Result<runner::Launch, String>),
    /// Durdur on an extension's run.
    StopRun(String),
    /// A stopped run not answered in time: its process is ended.
    RunOverdue(String, u64),
    /// What an extension's process said: extension, its process's generation, what.
    Runner(String, u64, crate::python::host::Said),
    Enable(String),
    Disable(String),
    Remove(String),
    Retry(String),
    /// Go to another installed version: the extension and the version.
    Switch(String, String),
    ResetData(String),
    PickPackage,
    /// The chosen `.kek`: its name and bytes; none when the user cancelled.
    Picked(Option<(String, Vec<u8>)>),
    PickCatalog,
    PickedCatalog(Option<PathBuf>),
    /// Review a catalog entry, by its file.
    Get(String),
    Grant(Capability, bool),
    Python(bool),
    ReplaceSource(bool),
    Install,
    CancelReview,
    SafeNext(bool),
    Close,
    /// An extension's data source, from its tab or Kaynaklar: (extension, local id).
    OpenSource(String, String),
    /// An extension's form, from its tab: (extension, local id).
    OpenForm(String, String),
    /// The chosen extension's report onto the clipboard, for its publisher or an agent.
    CopyReport(String),
    /// Its whole log, or its newest lines.
    FullLog(bool),
    /// The status bar's cell: the manager on the first extension in trouble.
    ShowTrouble,
    /// The manager on this extension, its log in view (a failure's notice's Ayrıntı).
    Show(String),
    /// A revealed button's outline goes: its tab, panel and place.
    FlashEnd(&'static str, usize, usize),
    /// The lists' word, and their program filter by its place in browse.rs's `PROGRAMS`.
    Query(String),
    ProgramFilter(usize),
    /// Sorunlu eklentiyi bul (bisect.rs): its window, its start, a round's answer (the problem still seen), leaving,
    /// closing the window (a bisection going on goes on), disabling the one found.
    BisectOpen,
    BisectStart,
    BisectAnswer(bool),
    BisectStop,
    BisectClose,
    BisectDisable(String),
}

impl Extensions {
    /// The registry under `root`, with the organisation's policy; started unless `safe_mode`. The notes say what the
    /// start did, for the message log.
    pub(crate) fn open(root: &Path, safe_mode: bool) -> (Extensions, Vec<(Level, String)>) {
        let policy_file = kentos_extension_host::places::policy_path();
        let options = Options {
            safe_mode,
            policy: kentos_extension_host::places::load_policy(&policy_file),
            ..Options::default()
        };
        let mut notes = Vec::new();
        let mut extensions = Extensions {
            root: Some(root.to_path_buf()),
            safe_mode,
            ..Extensions::default()
        };
        match Registry::open(root, host(), options) {
            Ok(mut registry) => {
                let started = registry.startup();
                let name_of = |registry: &Registry, id: &str| {
                    registry
                        .manifest(id)
                        .map_or_else(|| id.to_owned(), |m| m.name.clone())
                };
                for (id, result) in &started {
                    if let Err(problem) = result {
                        let name = name_of(&registry, id);
                        notes.push((
                            Level::Error,
                            format!(
                                "{name} eklentisi çalıştırılmadı: {} Ayrıntı ve rapor: Eklentiler › {name}.",
                                problem.message
                            ),
                        ));
                    }
                }
                // Enabled but kept from starting by a state of before (failed, blocked, incompatible, waiting): said at
                // every start until the user decides, one line for all.
                let idle: Vec<String> = registry
                    .extensions()
                    .iter()
                    .filter(|e| {
                        e.enabled
                            && !safe_mode
                            && !matches!(
                                e.state,
                                State::Active | State::Ready | State::Disabled | State::Held
                            )
                            && !started.iter().any(|(id, r)| *id == e.id && r.is_err())
                    })
                    .map(|e| format!("{} ({})", e.name, e.state_label))
                    .collect();
                if !idle.is_empty() {
                    notes.push((
                        Level::Warn,
                        format!(
                            "Çalışmayan eklenti: {}. Nedenleri ve yapılacaklar Eklentiler penceresinde.",
                            idle.join(", ")
                        ),
                    ));
                }
                // Sorunlu eklentiyi bul goes on after a restart: its question again (bisect.rs).
                if let Some(b) = registry.bisection() {
                    notes.push((
                        Level::Warn,
                        format!(
                            "Sorunlu eklentiyi bulma sürüyor (tur {}): bazı eklentiler bu turda kapalı. Soru Eklentiler \
                             penceresinde.",
                            b.rounds() + 1
                        ),
                    ));
                    extensions.bisect = Some(bisect::BisectStep::Asking);
                }
                let active = registry
                    .extensions()
                    .iter()
                    .filter(|e| e.state == State::Active)
                    .count();
                if safe_mode && !registry.extensions().is_empty() {
                    notes.push((
                        Level::Info,
                        "Eklentisiz başlatıldı: hiçbir eklenti etkinleştirilmedi. Eklentiler penceresinden tek tek \
                         açabilir ya da yeniden başlatabilirsiniz."
                            .to_owned(),
                    ));
                } else if active > 0 {
                    notes.push((Level::Info, format!("{active} eklenti etkinleştirildi.")));
                }
                extensions.registry = Some(registry);
                extensions.missing = None;
            }
            Err(problem) => {
                notes.push((
                    Level::Error,
                    format!(
                        "Eklenti kayıt defteri açılamadı; bu açılışta hiçbir eklenti çalışmıyor: {}",
                        problem.message
                    ),
                ));
                extensions.missing = Some(problem.message);
            }
        }
        (extensions, notes)
    }

    /// The device's registry: the user's folder; a start without extensions when asked by the command line, the
    /// environment or the window's one-shot flag (which this start removes).
    pub(crate) fn device() -> (Extensions, Vec<(Level, String)>) {
        let Some(root) = kentos_extension_host::places::user_root() else {
            let why = "Bu bilgisayarda kullanıcı veri klasörü yok (XDG_DATA_HOME ya da HOME); eklentiler kurulamaz.";
            return (
                Extensions {
                    missing: Some(why.to_owned()),
                    ..Extensions::default()
                },
                vec![(Level::Warn, why.to_owned())],
            );
        };
        let flag = root.join(SAFE_NEXT);
        let asked = flag.exists();
        if asked {
            let _ = std::fs::remove_file(&flag);
        }
        let safe = asked
            || std::env::args().any(|a| a == "--eklentisiz")
            || std::env::var_os("KENTOS_EKLENTISIZ").is_some_and(|v| !v.is_empty());
        Extensions::open(&root, safe)
    }

    /// The catalog folder to open at start: `KENTOS_EXTENSIONS_CATALOG` (an organisation's set-up, a real-window
    /// session), else the one opened last.
    pub(crate) fn startup_catalog(&self) -> Option<PathBuf> {
        if let Some(dir) = std::env::var_os("KENTOS_EXTENSIONS_CATALOG").filter(|v| !v.is_empty()) {
            return Some(PathBuf::from(dir));
        }
        let text = std::fs::read_to_string(self.root.as_ref()?.join(CATALOG_FOLDER)).ok()?;
        let folder = PathBuf::from(text.trim());
        folder.is_dir().then_some(folder)
    }

    /// Whether the next start is asked to be one without extensions.
    pub(crate) fn safe_next(&self) -> bool {
        self.root
            .as_ref()
            .is_some_and(|r| r.join(SAFE_NEXT).exists())
    }
}

impl App {
    /// Eklentiler (`extensions.manage`): the manager window.
    pub(crate) fn open_extensions(&mut self) {
        let x = &mut self.extensions;
        if x.chosen.is_none() {
            x.chosen = x
                .registry
                .as_ref()
                .and_then(|r| r.extensions().first().map(|e| e.id.clone()));
        }
        self.dialog = Some(Dialog::Extensions);
    }

    /// A `.kek` the program was started with, a double click on it (PY-KEK-15): its review over the manager. Opening a
    /// package never installs or runs it; Kur does, after the review.
    pub(crate) fn review_kek_file(&mut self, path: &Path) {
        self.open_extensions();
        match std::fs::read(path) {
            Ok(bytes) => {
                let source = Source {
                    kind: SourceKind::File,
                    location: path.display().to_string(),
                };
                self.review_package(path.display().to_string(), bytes, source);
            }
            Err(e) => self.extensions_say(true, format!("{} okunamadı: {e}.", path.display())),
        }
    }

    /// Paket yükle (`extensions.install`): the manager, then the file dialog.
    pub(crate) fn install_extension(&mut self) -> Task<Message> {
        self.open_extensions();
        self.extensions_event(Event::PickPackage)
    }

    fn extensions_say(&mut self, error: bool, words: String) {
        self.extensions.note = Some((error, words));
    }

    /// The details show the first row of what the list shows now (an extension's id, or a catalog entry's file).
    fn choose_first(&mut self) {
        let x = &self.extensions;
        let part = x.part;
        self.extensions.chosen = if part.is_catalog() {
            x.entries_of(part).first().map(|e| e.file.clone())
        } else {
            x.installed_of(part).first().map(|e| e.id.clone())
        };
    }

    /// An extension's name, as the user knows it; its id when it is not installed.
    pub(crate) fn extension_name(&self, id: &str) -> String {
        self.extensions
            .registry
            .as_ref()
            .and_then(|r| r.manifest(id))
            .map_or_else(|| id.to_owned(), |m| m.name.clone())
    }

    /// Something the extensions add changed: the ribbon, Kaynaklar, the agents' lists and the status bar's cell
    /// follow.
    pub(crate) fn extensions_changed(&mut self) {
        self.extension_tabs_changed();
        self.ext_tools_changed();
        self.extensions.troubles = problems::troubles(&self.extensions);
        self.extensions.icons = self
            .extensions
            .registry
            .as_ref()
            .map(|r| {
                r.extensions()
                    .into_iter()
                    .filter_map(|e| {
                        let path = r.manifest(&e.id)?.icon.clone()?;
                        let fallback = crate::icons::from_web(Some("extensions"));
                        Some((e.id.clone(), ribbon::icon_of(r, &e.id, &path, fallback)))
                    })
                    .collect()
            })
            .unwrap_or_default();
    }

    pub(crate) fn extensions_event(&mut self, event: Event) -> Task<Message> {
        let name = match &event {
            Event::Enable(id)
            | Event::Disable(id)
            | Event::Remove(id)
            | Event::Retry(id)
            | Event::Switch(id, _)
            | Event::ResetData(id)
            | Event::CopyReport(id) => self.extension_name(id),
            _ => String::new(),
        };
        match event {
            Event::Part(part) => {
                self.extensions.part = part;
                self.choose_first();
            }
            Event::Query(text) => {
                self.extensions.query = text;
                self.choose_first();
            }
            Event::ProgramFilter(i) => {
                self.extensions.program = browse::PROGRAMS.get(i).and_then(|(m, _)| *m);
                self.choose_first();
            }
            Event::Choose(id) => {
                self.extensions.chosen = Some(id.clone());
                self.extensions.full_log = false;
                // Its Python environment, when it has one, looked for while its details are shown.
                return self.env_look(&id);
            }
            Event::Env(event) => return self.env_event(event),
            Event::Run(ext, kind, local) => return self.ext_run(&ext, &kind, &local),
            Event::Launch(ext, kind, local, launch) => {
                return self.ext_launch(&ext, &kind, &local, launch);
            }
            Event::StopRun(ext) => return self.ext_stop(&ext),
            Event::RunOverdue(ext, id) => self.ext_overdue(&ext, id),
            Event::Runner(ext, generation, said) => return self.ext_said(&ext, generation, said),
            Event::Enable(id) => {
                let result = self.extensions.registry.as_mut().map(|r| r.enable(&id));
                match result {
                    Some(Ok(Some(activation))) => {
                        let left = activation.skipped.len();
                        self.extensions_say(
                            false,
                            if left == 0 {
                                format!("{name} etkinleştirildi.")
                            } else {
                                format!("{name} etkinleştirildi; {left} katkısı verilmeyen izin ya da Python yüzünden dışarıda.")
                            },
                        );
                    }
                    Some(Ok(None)) => self.extensions_say(
                        false,
                        "Etkinleştirildi; bu açılış eklentisiz olduğu için bir sonraki açılışta çalışır.".to_owned(),
                    ),
                    Some(Err(problem)) => self.extensions_say(true, problem.message),
                    None => {}
                }
                self.extensions_changed();
            }
            Event::Disable(id) => {
                let result = self.extensions.registry.as_mut().map(|r| r.disable(&id));
                match result {
                    Some(Ok(stopped)) if stopped.is_empty() => {
                        self.extensions_say(false, format!("{name} devre dışı bırakıldı."));
                    }
                    Some(Ok(stopped)) => self.extensions_say(
                        false,
                        format!(
                            "{name} devre dışı bırakıldı; ona bağlı olanlar da durdu: {}.",
                            stopped.join(", ")
                        ),
                    ),
                    Some(Err(problem)) => self.extensions_say(true, problem.message),
                    None => {}
                }
                self.extensions_changed();
            }
            Event::Remove(id) => {
                let result = self
                    .extensions
                    .registry
                    .as_mut()
                    .map(|r| r.remove(&id, true));
                match result {
                    Some(Ok(_)) => {
                        self.extensions_say(
                            false,
                            format!("{name} kaldırıldı. Verisi duruyor; aynı eklenti yeniden kurulunca kullanılır."),
                        );
                        // Its Python environments go with it: they are of no other extension (PY-UV-30).
                        if let Some(root) = self.python_setup.root.as_deref() {
                            kentos_python_env::extension::clean_for(root, &id, None);
                        }
                        self.extensions.envs.remove(&id);
                        self.extensions.chosen = self
                            .extensions
                            .registry
                            .as_ref()
                            .and_then(|r| r.extensions().first().map(|e| e.id.clone()));
                    }
                    Some(Err(problem)) => self.extensions_say(true, problem.message),
                    None => {}
                }
                self.extensions_changed();
            }
            Event::Retry(id) => {
                let result = self.extensions.registry.as_mut().map(|r| r.retry(&id));
                match result {
                    Some(Ok(_)) => {
                        self.extensions_say(false, format!("{name} yeniden başlatıldı."))
                    }
                    Some(Err(problem)) => self.extensions_say(true, problem.message),
                    None => {}
                }
                self.extensions_changed();
            }
            Event::Switch(id, version) => {
                let result = self
                    .extensions
                    .registry
                    .as_mut()
                    .map(|r| r.switch_version(&id, &version));
                match result {
                    Some(Ok(_)) => {
                        self.extensions_say(false, format!("{name} {version} sürümüne geçti."))
                    }
                    Some(Err(problem)) => self.extensions_say(true, problem.message),
                    None => {}
                }
                self.extensions_changed();
            }
            Event::ResetData(id) => {
                let result = self.extensions.registry.as_mut().map(|r| r.reset_data(&id));
                match result {
                    Some(Ok(())) => self
                        .extensions_say(false, format!("{name} eklentisinin verisi sıfırlandı.")),
                    Some(Err(problem)) => self.extensions_say(true, problem.message),
                    None => {}
                }
            }
            Event::PickPackage => {
                return Task::perform(
                    async {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Eklenti paketi yükle")
                            .add_filter("KentOS eklenti paketi (.kek)", &["kek"])
                            .pick_file()
                            .await?;
                        let bytes = file.read().await;
                        Some((file.path().display().to_string(), bytes))
                    },
                    |picked| msg(Event::Picked(picked)),
                );
            }
            Event::Picked(None) | Event::PickedCatalog(None) => {}
            Event::Picked(Some((path, bytes))) => {
                let source = Source {
                    kind: SourceKind::File,
                    location: path.clone(),
                };
                self.review_package(path, bytes, source);
            }
            Event::PickCatalog => {
                return Task::perform(
                    async {
                        let folder = rfd::AsyncFileDialog::new()
                            .set_title("Eklenti kataloğu klasörü")
                            .pick_folder()
                            .await?;
                        Some(folder.path().to_path_buf())
                    },
                    |picked| msg(Event::PickedCatalog(picked)),
                );
            }
            Event::PickedCatalog(Some(folder)) => self.open_extension_catalog(&folder, true),
            Event::Get(file) => {
                let Some(found) = self.extensions.catalog.as_ref() else {
                    return Task::none();
                };
                let Some(entry) = found
                    .catalog
                    .packages
                    .iter()
                    .find(|e| e.file == file)
                    .cloned()
                else {
                    return Task::none();
                };
                let path = found.folder.join(&entry.file);
                let folder = found.folder.display().to_string();
                match std::fs::read(&path) {
                    Ok(bytes) => {
                        if let Err(problem) = entry.verify(&bytes, &Limits::default()) {
                            // Said in the window and kept in the extension's log: a share whose file changed after
                            // its index was written is the organisation's to mend.
                            if let Some(registry) = self.extensions.registry.as_mut() {
                                registry.write_log(
                                    Record::warning(
                                        problem.code,
                                        format!(
                                            "{} {} katalogdan kurulmadı: {}",
                                            entry.name, entry.version, problem.message
                                        ),
                                    )
                                    .of(&entry.id, None)
                                    .part("katalog")
                                    .detail(path.display().to_string()),
                                );
                            }
                            self.extensions_say(true, problem.message);
                            return Task::none();
                        }
                        let source = Source {
                            kind: SourceKind::Organization,
                            location: folder,
                        };
                        self.review_package(entry.file.clone(), bytes, source);
                        if let Some(pending) = self.extensions.review.as_mut()
                            && pending.changelog.is_none()
                        {
                            pending.changelog = entry.changelog;
                        }
                    }
                    Err(e) => {
                        self.extensions_say(true, format!("{} okunamadı: {e}.", path.display()))
                    }
                }
            }
            Event::Grant(capability, on) => {
                if let Some(p) = self.extensions.review.as_mut() {
                    p.granted.retain(|c| *c != capability);
                    if on {
                        p.granted.push(capability);
                    }
                }
            }
            Event::Python(on) => {
                if let Some(p) = self.extensions.review.as_mut() {
                    p.python = on;
                }
            }
            Event::ReplaceSource(on) => {
                if let Some(p) = self.extensions.review.as_mut() {
                    p.replace_source = on;
                }
            }
            Event::Install => self.install_reviewed(),
            Event::CancelReview => self.extensions.review = None,
            Event::SafeNext(on) => {
                if let Some(root) = self.extensions.root.clone() {
                    let flag = root.join(SAFE_NEXT);
                    let done = if on {
                        std::fs::create_dir_all(&root).and_then(|()| std::fs::write(&flag, b"1"))
                    } else {
                        std::fs::remove_file(&flag).or_else(|e| {
                            if e.kind() == std::io::ErrorKind::NotFound {
                                Ok(())
                            } else {
                                Err(e)
                            }
                        })
                    };
                    match done {
                        Ok(()) if on => self.extensions_say(
                            false,
                            "Bir sonraki açılış eklentisiz olacak; ondan sonrakiler yine eklentilerle.".to_owned(),
                        ),
                        Ok(()) => self.extensions_say(false, "Bir sonraki açılış eklentilerle olacak.".to_owned()),
                        Err(e) => self.extensions_say(true, format!("{} yazılamadı: {e}.", flag.display())),
                    }
                }
            }
            Event::Close => {
                self.extensions.review = None;
                if self.dialog == Some(Dialog::Extensions) {
                    self.dialog = None;
                }
            }
            Event::OpenSource(ext, local) => return self.open_extension_source(&ext, &local),
            Event::OpenForm(ext, local) => self.open_extension_form(&ext, &local),
            Event::CopyReport(id) => {
                if let Some(report) = self.extension_report(&id) {
                    self.extensions_say(
                        false,
                        format!(
                            "{name} raporu panoya kopyalandı: yayımcısına ya da bir ajana yapıştırabilirsiniz."
                        ),
                    );
                    return iced::clipboard::write(report);
                }
            }
            Event::FullLog(on) => self.extensions.full_log = on,
            Event::BisectOpen => self.open_bisection(),
            Event::BisectStart => self.bisect_start(),
            Event::BisectAnswer(seen) => self.bisect_answer(seen),
            Event::BisectStop => self.bisect_stop(),
            Event::BisectClose => {
                self.extensions.bisect = None;
                if self
                    .extensions
                    .registry
                    .as_ref()
                    .is_some_and(|r| r.bisection().is_some())
                {
                    self.extensions_say(
                        false,
                        "Sorunlu eklentiyi bulma sürüyor; “Soruya dön” ile cevaplayın ya da bırakın.".to_owned(),
                    );
                }
            }
            Event::BisectDisable(id) => {
                self.extensions.bisect = None;
                return self.extensions_event(Event::Disable(id));
            }
            Event::FlashEnd(tab, panel, item) => {
                self.extension_flash_end(ExtHit { tab, panel, item });
            }
            Event::ShowTrouble => {
                let first = self.extensions.troubles.first().map(|t| t.id.clone());
                self.open_extensions();
                self.extensions.part = Part::Installed;
                // Chosen as a click on its row chooses it: its Python environment looked for, not left “aranıyor”.
                if let Some(first) = first {
                    return self.extensions_event(Event::Choose(first));
                }
            }
            Event::Show(ext) => {
                self.open_extensions();
                self.extensions.part = Part::Installed;
                return self.extensions_event(Event::Choose(ext));
            }
        }
        Task::none()
    }

    /// An extension's data source on the map, through Kaynaklar's own service window (its address, kind and first
    /// layer filled in, the window knowing whose it is); a GeoPackage in the package opens as a file does. What fails is
    /// said with the extension's name and kept in its log (problems.rs).
    fn open_extension_source(&mut self, ext: &str, local: &str) -> Task<Message> {
        use kentos_commands::ServiceKind;
        use kentos_extensions::manifest::DataSourceKind;
        let Some(registry) = self.extensions.registry.as_ref() else {
            return Task::none();
        };
        let Some(source) = registry
            .manifest(ext)
            .and_then(|m| m.contributions.data_sources.iter().find(|s| s.id == local))
            .cloned()
        else {
            self.extension_failed(
                ext,
                &format!("veri kaynağı “{local}”"),
                "source_missing",
                "Bu veri kaynağı eklentinin kurulu sürümünde yok. Eklentiyi güncelleyin ya da yayımcısına bildirin.",
                None,
            );
            return Task::none();
        };
        let package_dir = registry.package_dir(ext);
        let part = format!("veri kaynağı “{}”", source.title);
        let service = match source.kind {
            DataSourceKind::Wms => Some(ServiceKind::Wms),
            DataSourceKind::Wmts => Some(ServiceKind::Wmts),
            DataSourceKind::Xyz => Some(ServiceKind::Xyz),
            DataSourceKind::VectorTiles => Some(ServiceKind::Vector),
            DataSourceKind::Wfs | DataSourceKind::OgcapiFeatures | DataSourceKind::Geopackage => {
                None
            }
        };
        let incomplete = |app: &mut App| {
            app.extension_failed(
                ext,
                &part,
                "source_incomplete",
                "Veri kaynağının adresi ya da dosyası yazılmamış; açılamadı. Yayımcısına bildirin.",
                None,
            );
            Task::none()
        };
        match (source.kind, service) {
            (_, Some(kind)) => {
                let Some(url) = source.url.clone() else {
                    return incomplete(self);
                };
                self.dialog = None;
                let task =
                    self.open_saved_service(kind, url, None, source.layers.first().cloned(), None);
                // Whose it is: a failure to connect or to add is the extension's to say (services/window.rs).
                if let Some(w) = self.service_window.as_mut() {
                    w.origin = Some((ext.to_owned(), source.title.clone()));
                }
                task
            }
            (DataSourceKind::Geopackage, None) => {
                let Some(file) = source.file.clone() else {
                    return incomplete(self);
                };
                let path = package_dir.map(|dir| file.split('/').fold(dir, |d, part| d.join(part)));
                match path {
                    // A copy, not a link: the package's folder goes when the extension is removed.
                    Some(path) if path.is_file() => {
                        self.dialog = None;
                        self.update(crate::exchange::open_gpkg(
                            path,
                            source.layers.first().cloned(),
                            false,
                        ))
                    }
                    _ => {
                        self.extension_failed(
                            ext,
                            &part,
                            "source_file_missing",
                            &format!(
                                "Paketteki dosyası ({file}) yok. Eklentiyi yeniden kurun ya da yayımcısına bildirin."
                            ),
                            None,
                        );
                        Task::none()
                    }
                }
            }
            (kind, None) => {
                // Not the extension's failure: this KentOS does not open the kind yet.
                let words = format!(
                    "{} eklentisinin “{}” veri kaynağı ({}) bu sürümde Kaynaklar'dan açılmıyor; adresi: {}.",
                    self.extension_name(ext),
                    source.title,
                    match kind {
                        DataSourceKind::Wfs => "WFS",
                        _ => "OGC API Features",
                    },
                    source.url.clone().unwrap_or_default()
                );
                if let Some(registry) = self.extensions.registry.as_mut() {
                    registry.write_log(
                        Record::info("source_not_supported", words.clone())
                            .of(ext, None)
                            .part(part),
                    );
                }
                self.warn(words);
                Task::none()
            }
        }
    }

    /// An extension's form into the project, as Formu içe aktar takes a `.kform` file: onto the chosen form's layer,
    /// else the active layer. A form that is missing or not a form package is said with the extension's name and kept
    /// in its log; the import's own refusal is said by the import.
    fn open_extension_form(&mut self, ext: &str, local: &str) {
        let Some(registry) = self.extensions.registry.as_ref() else {
            return;
        };
        let Some(definition) = registry
            .manifest(ext)
            .and_then(|m| m.entrypoints.forms.iter().find(|f| f.id == local))
            .map(|f| f.definition.clone())
        else {
            self.extension_failed(
                ext,
                &format!("form “{local}”"),
                "form_missing",
                "Bu form eklentinin kurulu sürümünde yok. Eklentiyi güncelleyin ya da yayımcısına bildirin.",
                None,
            );
            return;
        };
        let part = format!("form “{local}”");
        let text = match registry.package_file(ext, &definition) {
            None => {
                self.extension_failed(
                    ext,
                    &part,
                    "form_file_missing",
                    &format!(
                        "Form dosyası ({definition}) pakette yok. Eklentiyi yeniden kurun ya da yayımcısına bildirin."
                    ),
                    None,
                );
                return;
            }
            Some(bytes) => {
                match String::from_utf8(bytes) {
                    Ok(text) => text,
                    Err(e) => {
                        self.extension_failed(
                            ext,
                            &part,
                            "form_unreadable",
                            &format!("Form dosyası ({definition}) UTF-8 metin değil. Yayımcısına bildirin."),
                            Some(&e.to_string()),
                        );
                        return;
                    }
                }
            }
        };
        if let Err(e) = serde_json::from_str::<kentos_commands::FormPackage>(&text) {
            self.extension_failed(
                ext,
                &part,
                "form_invalid",
                &format!("Form dosyası ({definition}) bir KentOS form paketi değil. Yayımcısına bildirin."),
                Some(&format!("{definition}: {e}")),
            );
            return;
        }
        if self.document.is_none() {
            self.output("Formu eklemek için önce bir çizim açın (Ctrl+O).");
            return;
        }
        let name = definition
            .rsplit('/')
            .next()
            .unwrap_or(&definition)
            .to_owned();
        self.dialog = None;
        if self.form_imported(Some((name, text))) {
            self.extension_worked(ext, &part, "form_imported", "Form projeye alındı.");
        }
    }

    /// Opens a catalog folder's `index.json`; `shown`: the user chose it here, so the window turns to it and the folder
    /// is kept for the next start.
    pub(crate) fn open_extension_catalog(&mut self, folder: &Path, shown: bool) {
        let file = folder.join(catalog::FILE);
        let read = std::fs::read_to_string(&file)
            .map_err(|e| format!("{} okunamadı ({e}). Klasörde “kentos kek index” ile yazılmış bir dizin olmalı.", file.display()))
            .and_then(|text| Catalog::read(&text).map_err(|p| p.message));
        match read {
            Ok(catalog) => {
                // What came since its previous opening (browse.rs): Yeni.
                let fresh = browse::fresh_since_last(self.extensions.root.as_deref(), &catalog);
                let count = catalog.latest(true).len();
                let name = catalog.name.clone();
                self.extensions.catalog = Some(CatalogFolder {
                    folder: folder.to_path_buf(),
                    catalog,
                    fresh,
                });
                if shown {
                    self.extensions_say(false, format!("“{name}”: {count} eklenti."));
                    // Updates first when there are, else what can be installed.
                    self.extensions.part = if self.extensions.count(Part::Updates) > 0 {
                        Part::Updates
                    } else {
                        Part::Available
                    };
                    self.choose_first();
                    if let Some(root) = &self.extensions.root {
                        let _ =
                            std::fs::write(root.join(CATALOG_FOLDER), folder.display().to_string());
                    }
                }
            }
            Err(why) if shown => self.extensions_say(true, why),
            Err(why) => self.warn(format!("Eklenti kataloğu açılamadı: {why}")),
        }
    }

    /// Reviews a package's bytes before installing: nothing is installed until the user says Kur.
    fn review_package(&mut self, file: String, bytes: Vec<u8>, source: Source) {
        let Some(registry) = self.extensions.registry.as_ref() else {
            let why = self.extensions.missing.clone().unwrap_or_default();
            self.extensions_say(true, why);
            return;
        };
        let review = registry.review(&bytes, &source);
        let opened = kentos_extensions::open(&bytes, &Limits::default());
        let manifest = review.inspection.manifest.as_ref();
        let changelog = manifest
            .and_then(|m| m.changelog_file.as_ref())
            .and_then(|p| opened.files.get(p))
            .and_then(|b| String::from_utf8(b.clone()).ok());
        let granted = manifest
            .map(|m| m.capabilities().into_iter().collect())
            .unwrap_or_default();
        let has_python = manifest.is_some_and(|m| m.python.is_some());
        self.extensions.review = Some(Pending {
            bytes,
            source,
            file,
            review,
            changelog,
            granted,
            python: has_python,
            has_python,
            replace_source: false,
        });
    }

    /// Kur: the reviewed package with the user's choices.
    fn install_reviewed(&mut self) {
        let Some(pending) = self.extensions.review.as_ref() else {
            return;
        };
        let Some(registry) = self.extensions.registry.as_mut() else {
            return;
        };
        let choice = Choice {
            grant: Grant {
                capabilities: pending.granted.clone(),
                python: pending.python && pending.has_python,
            },
            enable: true,
            replace_source: pending.replace_source,
        };
        let name = pending
            .review
            .inspection
            .manifest
            .as_ref()
            .map(|m| m.name.clone())
            .unwrap_or_default();
        match registry.install(&pending.bytes, &pending.source, &choice) {
            Ok(installed) => {
                let words = match (
                    &installed.previous,
                    installed.already,
                    &installed.activation,
                ) {
                    (_, true, None) => format!("{name} {} zaten kurulu.", installed.version),
                    (Some(old), false, _) if *old != installed.version => {
                        format!("{name} {old} → {} güncellendi.", installed.version)
                    }
                    _ => format!("{name} {} kuruldu.", installed.version),
                };
                let extra = match &installed.activation {
                    Some(Err(problem)) => format!(" Etkinleştirilmedi: {}", problem.message),
                    _ => String::new(),
                };
                let error = matches!(installed.activation, Some(Err(_)));
                self.extensions.chosen = Some(installed.id.clone());
                self.extensions.part = Part::Installed;
                self.extensions.review = None;
                self.extensions_say(error, format!("{words}{extra}"));
                self.extensions_changed();
            }
            Err(problems) => {
                let first = problems.first().map(|p| p.to_string()).unwrap_or_default();
                let more = problems.len().saturating_sub(1);
                self.extensions_say(
                    true,
                    if more == 0 {
                        first
                    } else {
                        format!("{first} (ve {more} sorun daha)")
                    },
                );
            }
        }
    }

    /// A catalog entry's install state: the installed version, when it is installed.
    pub(crate) fn installed_version(&self, id: &str) -> Option<String> {
        self.extensions
            .registry
            .as_ref()?
            .extensions()
            .into_iter()
            .find(|e| e.id == id)
            .map(|e| e.version)
    }
}

/// A data source's kind in a word, as Kaynaklar shows it.
pub(crate) fn kind_word(kind: kentos_extensions::manifest::DataSourceKind) -> &'static str {
    use kentos_extensions::manifest::DataSourceKind as K;
    match kind {
        K::Wms => "WMS",
        K::Wmts => "WMTS",
        K::Xyz => "XYZ",
        K::Wfs => "WFS",
        K::OgcapiFeatures => "OGC API Features",
        K::VectorTiles => "Vektör karo",
        K::Geopackage => "GeoPackage",
    }
}

impl App {
    /// The data sources of the running extensions this program shows (extension, its name, the source), for
    /// Kaynaklar's Eklentilerden.
    pub(crate) fn extension_sources(
        &self,
    ) -> Vec<(String, String, kentos_extensions::manifest::DataSource)> {
        let Some(registry) = self.extensions.registry.as_ref() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for c in registry.contributions() {
            if c.kind != kentos_extension_host::ContributionKind::DataSource {
                continue;
            }
            let ext = &c.lease.extension;
            let Some(manifest) = registry.manifest(ext) else {
                continue;
            };
            if !manifest
                .modules
                .iter()
                .any(|m| self.extension_module_here(*m))
            {
                continue;
            }
            let local =
                c.id.strip_prefix(&format!("ext.{ext}."))
                    .unwrap_or_default();
            if let Some(source) = manifest
                .contributions
                .data_sources
                .iter()
                .find(|s| s.id == local)
            {
                out.push((ext.clone(), manifest.name.clone(), source.clone()));
            }
        }
        out
    }
}

/// Whether a catalog entry is newer than what is installed.
pub(crate) fn is_update(entry: &CatalogEntry, installed: Option<&str>) -> bool {
    installed.is_some_and(|v| Version::parse(&entry.version) > Version::parse(v))
}

#[cfg(test)]
mod tests;
