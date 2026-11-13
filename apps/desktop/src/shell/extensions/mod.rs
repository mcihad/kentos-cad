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

mod ribbon;
mod view;

use std::path::{Path, PathBuf};

use iced::Task;
use kentos_extension_host::{Choice, Grant, Options, Registry, Review, Source, SourceKind, State};
use kentos_extensions::catalog::{self, Catalog, CatalogEntry};
use kentos_extensions::manifest::Capability;
use kentos_extensions::version::Version;
use kentos_extensions::{Host, Limits};

use crate::app::{App, Dialog, Message};

/// The flag file that makes the next start one without extensions; removed by that start.
const SAFE_NEXT: &str = "eklentisiz-baslat";
/// The catalog folder opened last, opened again at the next start (an organisation's share stays the same).
const CATALOG_FOLDER: &str = "katalog-klasoru";

/// The window's parts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    Installed,
    Catalog,
}

impl std::fmt::Display for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Part::Installed => "Kurulu",
            Part::Catalog => "Katalog",
        })
    }
}

/// A catalog folder opened in the window.
pub(crate) struct CatalogFolder {
    pub(crate) folder: PathBuf,
    pub(crate) catalog: Catalog,
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
}

impl Extensions {
    /// The registry under `root`, with the organisation's policy; started unless `safe_mode`. The notes say what the
    /// start did, for the message log.
    pub(crate) fn open(root: &Path, safe_mode: bool) -> (Extensions, Vec<(bool, String)>) {
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
                for (id, result) in registry.startup() {
                    if let Err(problem) = result {
                        notes.push((
                            true,
                            format!("“{id}” eklentisi etkinleştirilemedi: {}", problem.message),
                        ));
                    }
                }
                let active = registry
                    .extensions()
                    .iter()
                    .filter(|e| e.state == State::Active)
                    .count();
                if safe_mode && !registry.extensions().is_empty() {
                    notes.push((
                        false,
                        "Eklentisiz başlatıldı: hiçbir eklenti etkinleştirilmedi. Eklentiler penceresinden tek tek \
                         açabilir ya da yeniden başlatabilirsiniz."
                            .to_owned(),
                    ));
                } else if active > 0 {
                    notes.push((false, format!("{active} eklenti etkinleştirildi.")));
                }
                extensions.registry = Some(registry);
                extensions.missing = None;
            }
            Err(problem) => {
                notes.push((
                    true,
                    format!("Eklenti kayıt defteri açılamadı: {}", problem.message),
                ));
                extensions.missing = Some(problem.message);
            }
        }
        (extensions, notes)
    }

    /// The device's registry: the user's folder; a start without extensions when asked by the command line, the
    /// environment or the window's one-shot flag (which this start removes).
    pub(crate) fn device() -> (Extensions, Vec<(bool, String)>) {
        let Some(root) = kentos_extension_host::places::user_root() else {
            let why = "Bu bilgisayarda kullanıcı veri klasörü yok (XDG_DATA_HOME ya da HOME); eklentiler kurulamaz.";
            return (
                Extensions {
                    missing: Some(why.to_owned()),
                    ..Extensions::default()
                },
                vec![(true, why.to_owned())],
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

    /// Paket yükle (`extensions.install`): the manager, then the file dialog.
    pub(crate) fn install_extension(&mut self) -> Task<Message> {
        self.open_extensions();
        self.extensions_event(Event::PickPackage)
    }

    fn extensions_say(&mut self, error: bool, words: String) {
        self.extensions.note = Some((error, words));
    }

    /// An extension's name, as the user knows it; its id when it is not installed.
    fn extension_name(&self, id: &str) -> String {
        self.extensions
            .registry
            .as_ref()
            .and_then(|r| r.manifest(id))
            .map_or_else(|| id.to_owned(), |m| m.name.clone())
    }

    /// Something the extensions add changed: the ribbon, Kaynaklar and the agents' lists follow.
    pub(crate) fn extensions_changed(&mut self) {
        self.extension_tabs_changed();
    }

    pub(crate) fn extensions_event(&mut self, event: Event) -> Task<Message> {
        let name = match &event {
            Event::Enable(id)
            | Event::Disable(id)
            | Event::Remove(id)
            | Event::Retry(id)
            | Event::Switch(id, _)
            | Event::ResetData(id) => self.extension_name(id),
            _ => String::new(),
        };
        match event {
            Event::Part(part) => {
                self.extensions.part = part;
                self.extensions.chosen = match part {
                    Part::Installed => self
                        .extensions
                        .registry
                        .as_ref()
                        .and_then(|r| r.extensions().first().map(|e| e.id.clone())),
                    Part::Catalog => self
                        .extensions
                        .catalog
                        .as_ref()
                        .and_then(|c| c.catalog.latest(true).first().map(|e| e.file.clone())),
                };
            }
            Event::Choose(id) => self.extensions.chosen = Some(id),
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
        }
        Task::none()
    }

    /// An extension's data source on the map, through Kaynaklar's own service window (its address, kind and first
    /// layer filled in); a GeoPackage in the package opens as a file does.
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
            return Task::none();
        };
        let kind = match source.kind {
            DataSourceKind::Wms => Some(ServiceKind::Wms),
            DataSourceKind::Wmts => Some(ServiceKind::Wmts),
            DataSourceKind::Xyz => Some(ServiceKind::Xyz),
            DataSourceKind::VectorTiles => Some(ServiceKind::Vector),
            DataSourceKind::Wfs | DataSourceKind::OgcapiFeatures | DataSourceKind::Geopackage => {
                None
            }
        };
        match (kind, &source.url, &source.file) {
            (Some(kind), Some(url), _) => {
                self.dialog = None;
                self.open_saved_service(
                    kind,
                    url.clone(),
                    None,
                    source.layers.first().cloned(),
                    None,
                )
            }
            (None, _, Some(file)) => {
                let path = registry
                    .package_dir(ext)
                    .map(|dir| file.split('/').fold(dir, |d, part| d.join(part)));
                match path {
                    // A copy, not a link: the package's folder goes when the extension is removed.
                    Some(path) => {
                        self.dialog = None;
                        self.update(crate::exchange::open_gpkg(
                            path,
                            source.layers.first().cloned(),
                            false,
                        ))
                    }
                    None => Task::none(),
                }
            }
            _ => {
                self.output(format!(
                    "“{}” ({}) bu sürümde Kaynaklar'dan açılmıyor; adresi: {}.",
                    source.title,
                    match source.kind {
                        DataSourceKind::Wfs => "WFS",
                        _ => "OGC API Features",
                    },
                    source.url.clone().unwrap_or_default()
                ));
                Task::none()
            }
        }
    }

    /// An extension's form into the project, as Formu içe aktar takes a `.kform` file: onto the chosen form's layer,
    /// else the active layer.
    fn open_extension_form(&mut self, ext: &str, local: &str) {
        let Some(registry) = self.extensions.registry.as_ref() else {
            return;
        };
        let Some(definition) = registry
            .manifest(ext)
            .and_then(|m| m.entrypoints.forms.iter().find(|f| f.id == local))
            .map(|f| f.definition.clone())
        else {
            return;
        };
        match registry
            .package_file(ext, &definition)
            .map(String::from_utf8)
        {
            Some(Ok(text)) => {
                let name = definition
                    .rsplit('/')
                    .next()
                    .unwrap_or(&definition)
                    .to_owned();
                self.dialog = None;
                self.form_imported(Some((name, text)));
            }
            _ => self.warn(format!(
                "“{ext}” eklentisinin formu ({definition}) okunamadı; eklentiyi yeniden kurun."
            )),
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
                if shown {
                    let count = catalog.latest(true).len();
                    self.extensions_say(false, format!("“{}”: {count} eklenti.", catalog.name));
                    self.extensions.chosen = catalog.latest(true).first().map(|e| e.file.clone());
                    self.extensions.part = Part::Catalog;
                    if let Some(root) = &self.extensions.root {
                        let _ =
                            std::fs::write(root.join(CATALOG_FOLDER), folder.display().to_string());
                    }
                }
                self.extensions.catalog = Some(CatalogFolder {
                    folder: folder.to_path_buf(),
                    catalog,
                });
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
