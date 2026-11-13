//! The registry's files under the user's extension folder (docs/adr/8057 §2):
//!
//! ```text
//! <kök>/registry.json              kentos.extension-registry v1: what is installed, enabled, failed, granted
//! <kök>/revocations.json           the last revocation list applied, so it holds offline too
//! <kök>/packages/<id>/<sürüm>/     an installed version's files, read-only
//! <kök>/staging/<id>@<sürüm>.<n>/  a version being prepared; never read as installed
//! <kök>/data/<id>/                 the extension's own settings and data: kept when it is removed
//! <kök>/.lock                      held while a process changes the registry
//! ```
//!
//! `registry.json` is written whole to a temporary file and renamed over the old one, so a crash leaves the old or the
//! new registry, never half of one. Every change takes the lock, reads the file again and writes it back: the desktop,
//! the headless host and MCP may share one user's registry.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use kentos_extensions::Problem;
use kentos_extensions::manifest::Capability;
use serde::{Deserialize, Serialize};

pub const FORMAT: &str = "kentos.extension-registry";
pub const SCHEMA_VERSION: u32 = 1;

/// Where a package came from: the same id from two sources is the user's choice, never silent (D2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    /// A `.kek` file the user chose.
    File,
    /// An author's working folder, packed on install and on every reload.
    DeveloperFolder,
    /// The organisation's catalog or offline folder.
    Organization,
    /// The public catalog.
    Public,
}

impl SourceKind {
    pub fn words(self) -> &'static str {
        match self {
            SourceKind::File => "dosya",
            SourceKind::DeveloperFolder => "geliştirici klasörü",
            SourceKind::Organization => "kurum kataloğu",
            SourceKind::Public => "genel katalog",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Source {
    pub kind: SourceKind,
    /// The file, the folder or the catalog's address, as the user saw it.
    pub location: String,
}

/// The update channel: a preview is a pre-release version (`1.2.0-beta.1`), offered only to who asked (D9, D15).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Channel {
    #[default]
    Stable,
    Preview,
}

/// A version on disk.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstalledVersion {
    pub version: String,
    /// The content digest the inspector gave; the files are checked against it before they run.
    pub digest: String,
    /// Seconds since 1970.
    pub installed_at: u64,
}

/// What the user decided at install: the capabilities granted and whether the Python contributions are wanted.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Grant {
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    /// The Python contributions (commands, tools, audits …); without them no environment is prepared (PY-KEK-17).
    #[serde(default)]
    pub python: bool,
}

/// Why an extension is stopped though the user did not stop it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", deny_unknown_fields)]
pub enum Failure {
    /// It crashed again and again (the watchdog); it is not tried at start-up until the user retries (D6).
    Crashed { reason: String },
    /// Its files are not what was installed.
    Integrity { reason: String },
}

/// A line of an extension's log: what happened, in words.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Event {
    pub at: u64,
    pub message: String,
}

/// How an activation went (D6, ECO-QA-14): shown in the manager; a threshold only after measuring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Metrics {
    pub activation_micros: u64,
    #[serde(default)]
    pub memory_bytes: u64,
    #[serde(default)]
    pub processes: u32,
}

/// An extension in the registry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub source: Source,
    /// The versions on disk: the one in use and the one before it (to go back to).
    pub installed: Vec<InstalledVersion>,
    /// The version in use.
    pub active: String,
    /// The user wants it running.
    pub enabled: bool,
    #[serde(default)]
    pub grant: Grant,
    #[serde(default)]
    pub channel: Channel,
    /// Kept on this version: no update offered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<Failure>,
    /// Crashes in a row since it last ran well.
    #[serde(default)]
    pub crashes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metrics: Option<Metrics>,
    #[serde(default)]
    pub log: Vec<Event>,
}

/// Lines kept in an extension's log.
pub const LOG_LINES: usize = 50;

impl Entry {
    pub fn note(&mut self, message: impl Into<String>) {
        self.log.push(Event {
            at: now(),
            message: message.into(),
        });
        if self.log.len() > LOG_LINES {
            let extra = self.log.len() - LOG_LINES;
            self.log.drain(..extra);
        }
    }

    pub fn version(&self, version: &str) -> Option<&InstalledVersion> {
        self.installed.iter().find(|v| v.version == version)
    }
}

/// `registry.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegistryFile {
    pub format: String,
    pub schema_version: u32,
    #[serde(default)]
    pub extensions: BTreeMap<String, Entry>,
}

impl Default for RegistryFile {
    fn default() -> RegistryFile {
        RegistryFile {
            format: FORMAT.to_owned(),
            schema_version: SCHEMA_VERSION,
            extensions: BTreeMap::new(),
        }
    }
}

/// Seconds since 1970.
pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

pub fn problem(code: &'static str, message: impl Into<String>) -> Problem {
    Problem::new(code, message)
}

/// A file system error said for the user: what could not be done, where, and why.
pub fn io(what: &str, path: &Path, e: &std::io::Error) -> Problem {
    problem("registry_io", format!("{what}: {} ({e}).", path.display()))
}

/// The folders under the root.
#[derive(Clone, Debug)]
pub struct Layout {
    pub root: PathBuf,
}

impl Layout {
    pub fn registry(&self) -> PathBuf {
        self.root.join("registry.json")
    }

    pub fn revocations(&self) -> PathBuf {
        self.root.join("revocations.json")
    }

    pub fn package(&self, id: &str, version: &str) -> PathBuf {
        self.root.join("packages").join(id).join(version)
    }

    pub fn packages_of(&self, id: &str) -> PathBuf {
        self.root.join("packages").join(id)
    }

    pub fn staging(&self) -> PathBuf {
        self.root.join("staging")
    }

    pub fn data(&self, id: &str) -> PathBuf {
        self.root.join("data").join(id)
    }

    fn lock(&self) -> PathBuf {
        self.root.join(".lock")
    }

    /// Reads `registry.json`; none written yet is an empty registry.
    pub fn read(&self) -> Result<RegistryFile, Problem> {
        let path = self.registry();
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(RegistryFile::default());
            }
            Err(e) => return Err(io("Eklenti kaydı okunamadı", &path, &e)),
        };
        let file: RegistryFile = serde_json::from_str(&text).map_err(|e| {
            problem(
                "registry_corrupt",
                format!(
                    "Eklenti kaydı ({}) okunamadı: {e}. Dosyayı yedekleyip silin; kurulu paketler yeniden kurulabilir.",
                    path.display()
                ),
            )
        })?;
        if file.format != FORMAT || file.schema_version > SCHEMA_VERSION {
            return Err(problem(
                "registry_newer",
                format!(
                    "Eklenti kaydı “{}” sürüm {}; bu KentOS en çok {SCHEMA_VERSION}'i okur. KentOS'u güncelleyin.",
                    file.format, file.schema_version
                ),
            ));
        }
        Ok(file)
    }

    /// Writes `registry.json` whole: a temporary file, then a rename over the old one.
    pub fn write(&self, file: &RegistryFile) -> Result<(), Problem> {
        let text = serde_json::to_string_pretty(file).unwrap_or_default();
        write_atomic(&self.registry(), text.as_bytes())
    }

    /// Holds the lock while `work` runs: another process changing the registry waits up to `wait`. A lock older than
    /// two minutes is a crashed process's and is taken over.
    pub fn locked<T>(
        &self,
        wait: Duration,
        work: impl FnOnce() -> Result<T, Problem>,
    ) -> Result<T, Problem> {
        let path = self.lock();
        fs::create_dir_all(&self.root)
            .map_err(|e| io("Eklenti klasörü oluşturulamadı", &self.root, &e))?;
        let started = SystemTime::now();
        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut f) => {
                    let _ = writeln!(f, "{}", std::process::id());
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    let stale = fs::metadata(&path)
                        .and_then(|m| m.modified())
                        .ok()
                        .and_then(|t| t.elapsed().ok())
                        .is_some_and(|age| age > Duration::from_secs(120));
                    if stale {
                        let _ = fs::remove_file(&path);
                        continue;
                    }
                    if started.elapsed().unwrap_or_default() > wait {
                        return Err(problem(
                            "registry_busy",
                            "Eklenti kaydını başka bir KentOS penceresi ya da işlemi değiştiriyor; biraz sonra yeniden \
                             deneyin.",
                        ));
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
                Err(e) => return Err(io("Eklenti kaydı kilitlenemedi", &path, &e)),
            }
        }
        let result = work();
        let _ = fs::remove_file(&path);
        result
    }
}

/// Writes a file whole: a temporary file beside it, flushed, then renamed over it.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), Problem> {
    let parent = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| io("Klasör oluşturulamadı", parent, &e))?;
    let temp = parent.join(format!(
        ".{}.{}.yazılıyor",
        path.file_name().and_then(|n| n.to_str()).unwrap_or("dosya"),
        std::process::id()
    ));
    let mut f = fs::File::create(&temp).map_err(|e| io("Dosya yazılamadı", &temp, &e))?;
    f.write_all(bytes)
        .and_then(|()| f.sync_all())
        .map_err(|e| io("Dosya yazılamadı", &temp, &e))?;
    drop(f);
    fs::rename(&temp, path).map_err(|e| {
        let _ = fs::remove_file(&temp);
        io("Dosya yerine konamadı", path, &e)
    })
}

/// Writes a package's files under `dir` and makes them read-only.
pub fn write_files(dir: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<(), Problem> {
    for (path, data) in files {
        // The inspector allowed only plain relative ASCII paths; joined part by part all the same.
        let target = path
            .split('/')
            .fold(dir.to_path_buf(), |p, part| p.join(part));
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| io("Klasör oluşturulamadı", parent, &e))?;
        }
        fs::write(&target, data).map_err(|e| io("Paket dosyası yazılamadı", &target, &e))?;
        let mut permissions = fs::metadata(&target)
            .map_err(|e| io("Dosya okunamadı", &target, &e))?
            .permissions();
        permissions.set_readonly(true);
        let _ = fs::set_permissions(&target, permissions);
    }
    Ok(())
}

/// A folder's files by their paths inside it (`/` between folders).
pub fn read_files(dir: &Path) -> Result<BTreeMap<String, Vec<u8>>, Problem> {
    let mut files = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let entries = fs::read_dir(&at).map_err(|e| io("Klasör okunamadı", &at, &e))?;
        for entry in entries {
            let entry = entry.map_err(|e| io("Klasör okunamadı", &at, &e))?;
            let path = entry.path();
            let kind = entry
                .file_type()
                .map_err(|e| io("Dosya okunamadı", &path, &e))?;
            if kind.is_symlink() {
                return Err(problem(
                    "registry_link",
                    format!(
                        "{} sembolik bağ; kurulu bir pakette bağ olmaz.",
                        path.display()
                    ),
                ));
            }
            if kind.is_dir() {
                stack.push(path);
            } else {
                let relative = path
                    .strip_prefix(dir)
                    .ok()
                    .and_then(Path::to_str)
                    .map(|p| p.replace('\\', "/"))
                    .ok_or_else(|| {
                        problem("registry_io", format!("{}: adı okunamadı.", path.display()))
                    })?;
                let data = fs::read(&path).map_err(|e| io("Dosya okunamadı", &path, &e))?;
                files.insert(relative, data);
            }
        }
    }
    Ok(files)
}

/// Removes a folder and everything in it, read-only files too (Windows refuses to delete those).
pub fn remove_tree(dir: &Path) -> Result<(), Problem> {
    if !dir.exists() {
        return Ok(());
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        if let Ok(entries) = fs::read_dir(&at) {
            for entry in entries.flatten() {
                let path = entry.path();
                if entry.file_type().is_ok_and(|t| t.is_dir()) {
                    stack.push(path);
                } else if let Ok(meta) = fs::symlink_metadata(&path) {
                    let mut permissions = meta.permissions();
                    #[allow(clippy::permissions_set_readonly_false)]
                    permissions.set_readonly(false);
                    let _ = fs::set_permissions(&path, permissions);
                }
            }
        }
    }
    fs::remove_dir_all(dir).map_err(|e| io("Klasör silinemedi", dir, &e))
}
