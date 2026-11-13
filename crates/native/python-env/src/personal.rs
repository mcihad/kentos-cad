//! The user's personal environment (P04; PY-OPT-04…06; PY-UV-10): where the Python console and the user's own scripts
//! run and where the user adds packages such as NumPy. A small uv project of KentOS's ([`project`]: `pyproject.toml`,
//! `uv.lock`, `.venv`), pinned to KentOS's Python (`requires-python = "==3.12.*"`) and to this system
//! (`environments`): measured, a project open to every Python made uv lock two pandas at once (2.3.3 for 3.10, 3.1.1
//! for 3.11 and later) and the preview read both. Extensions have environments of their own (P05): nothing here reaches
//! them.
//!
//! - **Made offline:** an empty lock, an environment synced to it, KentOS's library from the installation's wheel
//!   ([`crate::sdk`]).
//! - **Previewed before anything changes:** the project is copied aside, uv adds (or removes) there, and the two locks'
//!   packages are compared: what comes, what goes, what changes version, which the user asked for and which come as
//!   their dependencies. Only when the user agrees does the copy replace the project; the environment is then synced to
//!   it exactly and KentOS's library put back after (an exact sync removes what the lock does not name). A failed sync
//!   puts the previous project back and syncs to it again, offline.
//! - **What is asked for is a package from the index**, never a path, an address or an option: a requirement starting
//!   with `-` would be read by uv as one of its own flags. `kentos` is refused: it is KentOS's library's name.
//! - **Room:** uv links packages from its cache into the environment, so removing the environment does not free the
//!   room its packages take; the cache ([`crate::places::cache`]) is shown and cleaned apart.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::python::{self, Cause, InstallError, Interpreter, Progress};
use crate::sdk::Wheel;
use crate::{places, uv};

/// How long resolving a change may take (it asks the index).
const RESOLVE_LIMIT: Duration = Duration::from_secs(10 * 60);

/// How long syncing may take: downloads.
pub const SYNC_LIMIT: Duration = Duration::from_secs(30 * 60);

/// How long an offline step may take.
const OFFLINE_LIMIT: Duration = Duration::from_secs(3 * 60);

/// The KentOS library version installed with the environment, written after installing it.
const SDK_MARK: &str = ".kentos-sdk";

/// The project's folder.
pub fn project(root: &Path) -> PathBuf {
    places::personal(root)
}

/// Where a change is previewed: beside the project, not in it (uv would take a project inside another for a member of
/// its workspace).
fn staging(root: &Path) -> PathBuf {
    root.join("kisisel-onizleme")
}

/// The environment's Python.
pub fn python(root: &Path) -> PathBuf {
    python_in(&project(root).join(".venv"), std::env::consts::OS)
}

/// A virtual environment's Python on a system: `Scripts\python.exe` on Windows, `bin/python` elsewhere.
pub fn python_in(venv: &Path, os: &str) -> PathBuf {
    if os == "windows" {
        venv.join("Scripts").join("python.exe")
    } else {
        venv.join("bin").join("python")
    }
}

/// The environment's Python when it is ready for the console: there, with KentOS's library put in.
pub fn ready(root: &Path) -> Option<PathBuf> {
    let python = python(root);
    (python.is_file() && project(root).join(SDK_MARK).is_file()).then_some(python)
}

/// A package of the environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Package {
    /// Normalised (PEP 503): lower case, `-` for runs of `-`, `_`, `.`.
    pub name: String,
    pub version: String,
    /// Asked for by the user; else a dependency of one that was.
    pub direct: bool,
}

/// The personal environment as it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Personal {
    pub python: PathBuf,
    /// The environment's files' bytes, as a file manager counts them (files linked from uv's cache counted in full).
    pub size: u64,
    /// The KentOS library's version installed; none when it is missing.
    pub sdk: Option<String>,
    /// Asked for first, then dependencies, each by name.
    pub packages: Vec<Package>,
}

/// What the user asks of the environment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Change {
    /// Requirements as typed (`numpy`, `pandas>=2`).
    Add(Vec<String>),
    /// Names.
    Remove(Vec<String>),
    /// Newer versions: packages asked for raised to the version given (`uv add "pandas>=3.1.1"`: re-adding a bare name
    /// keeps a pin the user wrote, measured), dependencies to the newest their requirements allow
    /// (`uv lock --upgrade-package`).
    Upgrade {
        raise: Vec<(String, String)>,
        others: Vec<String>,
    },
}

/// A package with a newer version in the index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outdated {
    pub name: String,
    pub version: String,
    pub latest: String,
}

/// What a change would do, as uv resolved it aside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Preview {
    pub change: Change,
    pub added: Vec<Package>,
    pub removed: Vec<Package>,
    /// Name, the version now, the version after.
    pub changed: Vec<(String, String, String)>,
    /// Names asked for after the change.
    pub direct: Vec<String>,
}

impl Preview {
    /// Nothing would change.
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

/// A package name as PEP 503 compares them.
pub fn normalize(name: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in name.trim().chars() {
        if matches!(c, '-' | '_' | '.') {
            dash = true;
        } else {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.extend(c.to_lowercase());
        }
    }
    out
}

/// A requirement the user typed, checked: a package of the index with an optional version (`numpy`, `pandas>=2,<3`,
/// `shapely[vectorized]==2.1`); never an option, a path or an address. Its name, normalised.
pub fn check(requirement: &str) -> Result<String, String> {
    let text = requirement.trim();
    if text.is_empty() {
        return Err("Paket adı yazın (ör. numpy).".to_owned());
    }
    if text.starts_with('-') {
        return Err(format!("“{text}” bir paket adı değil."));
    }
    if text.contains(['/', '\\', ':', '@', ';', '"', '\'', '`', '$']) {
        return Err(format!(
            "“{text}” bir paket adı değil: yalnız paket dizinindeki paketler kurulur (yol ya da adres değil)."
        ));
    }
    let allowed = |c: char| c.is_ascii_alphanumeric() || "._-[],<>=!~*+ ".contains(c);
    if !text.chars().all(allowed) {
        return Err(format!("“{text}” bir paket adı değil."));
    }
    let name: String = text
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        .collect();
    if name.is_empty() || !name.starts_with(|c: char| c.is_ascii_alphanumeric()) {
        return Err(format!("“{text}” bir paket adı değil."));
    }
    let name = normalize(&name);
    if name == "kentos" {
        return Err(
            "“kentos” KentOS'un kendi Python kitaplığının adı; başka bir yerden kurulamaz."
                .to_owned(),
        );
    }
    Ok(name)
}

/// The packages of `uv export --format requirements.txt` (`numpy==2.5.4 ; sys_platform == 'linux'`), by name.
pub fn exported(text: &str) -> BTreeMap<String, String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('-'))
        .filter_map(|line| {
            let pinned = line.split(';').next()?.trim();
            let (name, version) = pinned.split_once("==")?;
            Some((normalize(name), version.trim().to_owned()))
        })
        .collect()
}

/// The names in `[project] dependencies` of a `pyproject.toml` KentOS and uv write.
pub fn direct_names(pyproject: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_project = false;
    let mut in_list = false;
    for line in pyproject.lines() {
        let line = line.trim();
        if line.starts_with('[') && !in_list {
            in_project = line == "[project]";
            continue;
        }
        if in_project && !in_list && line.starts_with("dependencies") {
            let Some((_, rest)) = line.split_once('=') else {
                continue;
            };
            in_list = rest.trim_start().starts_with('[');
            names.extend(quoted_names(rest));
            if rest.contains(']') {
                in_list = false;
            }
            continue;
        }
        if in_list {
            names.extend(quoted_names(line));
            if line.contains(']') {
                in_list = false;
            }
        }
    }
    names
}

fn quoted_names(text: &str) -> Vec<String> {
    text.split('"')
        .skip(1)
        .step_by(2)
        .map(|requirement| {
            normalize(
                &requirement
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
                    .collect::<String>(),
            )
        })
        .filter(|name| !name.is_empty())
        .collect()
}

/// The project file KentOS writes: pinned to KentOS's Python's minor version and this system (`linux`, `windows`,
/// `macos`).
pub fn pyproject(python_version: &str, os: &str) -> String {
    let minor = python_version
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");
    let platform = match os {
        "windows" => "win32",
        "macos" => "darwin",
        _ => "linux",
    };
    format!(
        "# KentOS'un kişisel Python ortamı: Python konsolu ve kendi betikleriniz burada çalışır. Paketleri Uygulama\n\
         # ayarları › Python ve eklentiler'den ekleyip çıkarın; KentOS'un Python'una ve bu sisteme sabitlidir.\n\
         [project]\n\
         name = \"kentos-kisisel\"\n\
         version = \"0\"\n\
         requires-python = \"=={minor}.*\"\n\
         dependencies = []\n\
         \n\
         [tool.uv]\n\
         package = false\n\
         environments = [\"sys_platform == '{platform}'\"]\n"
    )
}

fn env_for(root: &Path) -> Vec<(&'static str, OsString)> {
    python::uv_env(root)
}

fn refs<'a>(env: &'a [(&'static str, OsString)]) -> Vec<(&'static str, &'a OsStr)> {
    env.iter().map(|(name, v)| (*name, v.as_os_str())).collect()
}

/// Runs uv in `dir` for KentOS's folders; its words when it fails.
fn uv_in(
    uv_path: &Path,
    root: &Path,
    dir: &Path,
    args: &[&OsStr],
    limit: Duration,
) -> Result<uv::Ran, String> {
    let env = env_for(root);
    let ran = uv::run(uv_path, args, Some(dir), &refs(&env), limit)?;
    if ran.status == Some(0) {
        Ok(ran)
    } else if ran.timed_out {
        Err("uv yanıt vermedi".to_owned())
    } else {
        Err(python::said(&ran.stderr).unwrap_or_else(|| ran.stderr.trim().to_owned()))
    }
}

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

/// The environment's packages as the project's lock pins them, and which were asked for.
fn locked(uv_path: &Path, root: &Path, dir: &Path) -> Result<Vec<Package>, String> {
    let args = os(&[
        "export",
        "--frozen",
        "--format",
        "requirements.txt",
        "--no-hashes",
        "--no-header",
        "--no-annotate",
        "--no-emit-project",
        "--no-config",
    ]);
    let ran = uv_in(uv_path, root, dir, &uv::os_args(&args), OFFLINE_LIMIT)?;
    let direct = std::fs::read_to_string(dir.join("pyproject.toml"))
        .map(|text| direct_names(&text))
        .unwrap_or_default();
    let mut packages: Vec<Package> = exported(&ran.stdout)
        .into_iter()
        .map(|(name, version)| Package {
            direct: direct.contains(&name),
            name,
            version,
        })
        .collect();
    packages.sort_by(|a, b| b.direct.cmp(&a.direct).then_with(|| a.name.cmp(&b.name)));
    Ok(packages)
}

/// The personal environment, when there is one.
pub fn read(uv_path: &Path, root: &Path) -> Result<Option<Personal>, String> {
    let dir = project(root);
    let python = python(root);
    if !python.is_file() || !dir.join("uv.lock").is_file() {
        return Ok(None);
    }
    Ok(Some(Personal {
        packages: locked(uv_path, root, &dir)?,
        size: python::folder_size(&dir.join(".venv")),
        sdk: std::fs::read_to_string(dir.join(SDK_MARK))
            .ok()
            .map(|v| v.trim().to_owned())
            .filter(|v| !v.is_empty()),
        python,
    }))
}

/// Puts KentOS's library into the environment from the installation's wheel: offline, without dependencies.
pub fn install_sdk(uv_path: &Path, root: &Path, sdk: &Wheel) -> Result<(), String> {
    let dir = project(root);
    let mut args = os(&[
        "pip",
        "install",
        "--no-deps",
        "--offline",
        "--no-config",
        "--python",
    ]);
    args.push(python(root).into_os_string());
    args.push(sdk.path.clone().into_os_string());
    uv_in(uv_path, root, &dir, &uv::os_args(&args), OFFLINE_LIMIT)
        .map_err(|why| format!("KentOS'un Python kitaplığı kurulamadı: {why}"))?;
    std::fs::write(dir.join(SDK_MARK), &sdk.version)
        .map_err(|e| format!("{} yazılamadı: {e}", dir.join(SDK_MARK).display()))
}

/// Makes the personal environment, offline: the project (kept when there is one: the user's packages), its lock, the
/// environment synced to it, KentOS's library.
pub fn create(
    uv_path: &Path,
    root: &Path,
    interpreter: &Interpreter,
    sdk: &Wheel,
) -> Result<(), String> {
    let dir = project(root);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{} açılamadı: {e}", dir.display()))?;
    let file = dir.join("pyproject.toml");
    if !file.is_file() {
        std::fs::write(&file, pyproject(&interpreter.version, std::env::consts::OS))
            .map_err(|e| format!("{} yazılamadı: {e}", file.display()))?;
    }
    let mut lock = os(&[
        "lock",
        "--offline",
        "--no-config",
        "--no-python-downloads",
        "--python",
    ]);
    lock.push(interpreter.path.clone().into_os_string());
    uv_in(uv_path, root, &dir, &uv::os_args(&lock), OFFLINE_LIMIT)?;
    let mut sync = os(&[
        "sync",
        "--locked",
        "--offline",
        "--no-config",
        "--no-python-downloads",
        "--python",
    ]);
    sync.push(interpreter.path.clone().into_os_string());
    uv_in(uv_path, root, &dir, &uv::os_args(&sync), OFFLINE_LIMIT)?;
    install_sdk(uv_path, root, sdk)
}

/// Removes the personal environment: the project, its lock and environment, a preview left aside. uv's cache stays
/// (it is shown and cleaned apart).
pub fn remove(root: &Path) -> Result<(), String> {
    for dir in [project(root), staging(root)] {
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!(
                    "{} silinemedi: {e}. Python konsolunu kapatıp yeniden deneyin.",
                    dir.display()
                ));
            }
        }
    }
    Ok(())
}

/// Why a preview failed, and what to do.
pub fn preview_words(said: &str, minor: &str) -> String {
    let lower = said.to_lowercase();
    if lower.contains("not found in the package registry") || lower.contains("no versions of") {
        format!("Paket dizininde bulunamadı. Adını denetleyin. uv: “{said}”")
    } else if lower.contains("no solution found") || lower.contains("unsatisfiable") {
        format!(
            "Bu paket KentOS'un Python'u ({minor}) ve bu sistemle kurulamıyor; başka bir sürüm deneyin. uv: “{said}”"
        )
    } else {
        match python::cause_of(said) {
            Cause::Network => format!(
                "Paket dizinine ulaşılamadı. Bağlantıyı ya da vekil sunucu ayarını (HTTPS_PROXY) denetleyin. uv: “{said}”"
            ),
            Cause::Certificate => format!(
                "Paket dizininin sertifikası bu bilgisayarda doğrulanamadı; kurumun sertifikası yüklü olmalı. uv: “{said}”"
            ),
            _ => format!("Önizlenemedi. uv: “{said}”"),
        }
    }
}

/// Resolves `change` aside, against the index, and says what it would do; the project is not touched.
pub fn preview(
    uv_path: &Path,
    root: &Path,
    interpreter: &Interpreter,
    change: &Change,
) -> Result<Preview, String> {
    let minor = interpreter
        .version
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");
    let dir = project(root);
    let aside = staging(root);
    let _ = std::fs::remove_dir_all(&aside);
    std::fs::create_dir_all(&aside).map_err(|e| format!("{} açılamadı: {e}", aside.display()))?;
    for file in ["pyproject.toml", "uv.lock"] {
        std::fs::copy(dir.join(file), aside.join(file))
            .map_err(|e| format!("{} kopyalanamadı: {e}", dir.join(file).display()))?;
    }
    let tail = |mut args: Vec<OsString>, sync_flag: bool| {
        if sync_flag {
            args.push(OsString::from("--no-sync"));
        }
        args.extend(os(&[
            "--no-config",
            "--no-python-downloads",
            "--system-certs",
            "--python",
        ]));
        args.push(interpreter.path.clone().into_os_string());
        args
    };
    let mut steps: Vec<Vec<OsString>> = Vec::new();
    match change {
        Change::Add(requirements) => {
            for requirement in requirements {
                check(requirement)?;
            }
            let mut args = os(&["add"]);
            args.extend(requirements.iter().map(|r| OsString::from(r.trim())));
            steps.push(tail(args, true));
        }
        Change::Remove(names) => {
            let mut args = os(&["remove"]);
            args.extend(names.iter().map(OsString::from));
            steps.push(tail(args, true));
        }
        Change::Upgrade { raise, others } => {
            if !raise.is_empty() {
                let mut args = os(&["add"]);
                for (name, version) in raise {
                    let requirement = format!("{name}>={version}");
                    check(&requirement)?;
                    args.push(OsString::from(requirement));
                }
                steps.push(tail(args, true));
            }
            if !others.is_empty() {
                let mut args = os(&["lock"]);
                for name in others {
                    check(name)?;
                    args.push(OsString::from("--upgrade-package"));
                    args.push(OsString::from(name));
                }
                steps.push(tail(args, false));
            }
        }
    }
    for args in &steps {
        uv_in(uv_path, root, &aside, &uv::os_args(args), RESOLVE_LIMIT)
            .map_err(|said| preview_words(&said, &minor))?;
    }
    let before = locked(uv_path, root, &dir)?;
    let after = locked(uv_path, root, &aside)?;
    let then: BTreeMap<&str, &Package> = before.iter().map(|p| (p.name.as_str(), p)).collect();
    let now: BTreeMap<&str, &Package> = after.iter().map(|p| (p.name.as_str(), p)).collect();
    Ok(Preview {
        change: change.clone(),
        added: after
            .iter()
            .filter(|p| !then.contains_key(p.name.as_str()))
            .cloned()
            .collect(),
        removed: before
            .iter()
            .filter(|p| !now.contains_key(p.name.as_str()))
            .cloned()
            .collect(),
        changed: after
            .iter()
            .filter_map(|p| {
                let old = then.get(p.name.as_str())?;
                (old.version != p.version)
                    .then(|| (p.name.clone(), old.version.clone(), p.version.clone()))
            })
            .collect(),
        direct: after
            .iter()
            .filter(|p| p.direct)
            .map(|p| p.name.clone())
            .collect(),
    })
}

/// How many bytes a downloaded byte of a wheel becomes once unpacked, in tenths (measured: numpy and pandas, 27.5 MiB
/// downloaded, 100.7 MB in uv's cache: × 3.66; 3.5 keeps the bar a little behind rather than ahead).
pub const UNPACKED_TENTHS: u64 = 35;

/// The bytes being unpacked into uv's cache right now: its `.tmp…` folders at the top, which become a package when it
/// is done (measured on uv 0.11.26).
pub fn under_way(cache: &Path) -> u64 {
    std::fs::read_dir(cache)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with(".tmp"))
                .map(|e| {
                    let path = e.path();
                    if path.is_dir() {
                        python::folder_size(&path)
                    } else {
                        e.metadata().map(|m| m.len()).unwrap_or(0)
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}

/// How far a sync is from uv's words: the sizes it said it downloads (`Downloading numpy (16.0MiB)`), those it said it
/// finished (` Downloaded numpy`), and installing once it says it prepared them; between, the bytes being unpacked
/// (`unpacking`), counted back to downloaded ones, move the bar, never past the whole. Small packages are not said.
pub fn read_sync_progress(said: &str, unpacking: u64) -> Progress {
    let mut sizes: BTreeMap<&str, u64> = BTreeMap::new();
    let mut done = 0;
    let mut installing = false;
    for line in said.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("Downloading ")
            && let (Some(open), Some(close)) = (rest.rfind('('), rest.rfind(')'))
            && let Some(size) = rest.get(open + 1..close).and_then(python::bytes)
        {
            sizes.insert(rest[..open].trim(), size);
        } else if let Some(name) = line.strip_prefix("Downloaded ") {
            done += sizes.get(name.trim()).copied().unwrap_or(0);
        } else if line.starts_with("Prepared ") || line.starts_with("Installed ") {
            installing = true;
        }
    }
    let total: u64 = sizes.values().sum();
    if installing {
        Progress::Unpacking
    } else if total > 0 {
        let moving = unpacking * 10 / UNPACKED_TENTHS;
        Progress::Downloading {
            // Not the whole until uv says so.
            done: (done + moving).min(total - total / 100),
            total: Some(total),
        }
    } else {
        Progress::Starting
    }
}

/// Makes the previewed change: the project aside replaces the project, the environment is synced to it exactly
/// (downloading what it lacks, said to `told`), KentOS's library is put back. Stopped as soon as `cancel` is set. On
/// failure the previous project comes back and the environment is synced to it again, offline.
pub fn apply(
    uv_path: &Path,
    root: &Path,
    interpreter: &Interpreter,
    sdk: &Wheel,
    cancel: &AtomicBool,
    told: &mut dyn FnMut(Progress),
) -> Result<(), InstallError> {
    let folder = |e: std::io::Error| InstallError::Failed {
        cause: Cause::Folder,
        said: e.to_string(),
    };
    let dir = project(root);
    let aside = staging(root);
    // The previous project, kept until the environment follows the new one.
    for file in ["pyproject.toml", "uv.lock"] {
        std::fs::copy(dir.join(file), dir.join(format!("{file}.onceki"))).map_err(folder)?;
    }
    for file in ["pyproject.toml", "uv.lock"] {
        std::fs::copy(aside.join(file), dir.join(file)).map_err(folder)?;
    }
    let synced = sync(uv_path, root, interpreter, false, cancel, told);
    let back = |dir: &Path| {
        for file in ["pyproject.toml", "uv.lock"] {
            let _ = std::fs::rename(dir.join(format!("{file}.onceki")), dir.join(file));
        }
    };
    match synced {
        Ok(()) => {
            for file in ["pyproject.toml", "uv.lock"] {
                let _ = std::fs::remove_file(dir.join(format!("{file}.onceki")));
            }
            let _ = std::fs::remove_dir_all(&aside);
            install_sdk(uv_path, root, sdk).map_err(|said| InstallError::Failed {
                cause: Cause::Other,
                said,
            })
        }
        Err(error) => {
            back(&dir);
            let idle = AtomicBool::new(false);
            let _ = sync(uv_path, root, interpreter, true, &idle, &mut |_| {});
            let _ = install_sdk(uv_path, root, sdk);
            Err(error)
        }
    }
}

/// Syncs the environment exactly to the project's lock; offline when asked (going back).
fn sync(
    uv_path: &Path,
    root: &Path,
    interpreter: &Interpreter,
    offline: bool,
    cancel: &AtomicBool,
    told: &mut dyn FnMut(Progress),
) -> Result<(), InstallError> {
    let dir = project(root);
    let env = env_for(root);
    let mut args = os(&[
        "sync",
        "--locked",
        "--no-config",
        "--no-python-downloads",
        "--system-certs",
    ]);
    if offline {
        args.push(OsString::from("--offline"));
    }
    args.push(OsString::from("--python"));
    args.push(interpreter.path.clone().into_os_string());
    let heard = Arc::new(Mutex::new(String::new()));
    let mut last = Progress::Starting;
    told(last);
    let cache = places::cache(root);
    let mut tick = || {
        let unpacking = under_way(&cache);
        let now = heard
            .lock()
            .map(|text| read_sync_progress(&text, unpacking))
            .unwrap_or(last);
        if now != last {
            last = now;
            told(now);
        }
    };
    let ran = uv::run_watched(
        uv_path,
        &uv::os_args(&args),
        Some(&dir),
        &refs(&env),
        SYNC_LIMIT,
        uv::Watch {
            cancel: Some(cancel),
            stderr: Some(heard.clone()),
            tick: Some(&mut tick),
        },
    )
    .map_err(InstallError::NotRun)?;
    // Once more at the end: a quick sync is over before its first tick.
    tick();
    if ran.cancelled {
        return Err(InstallError::Cancelled);
    }
    if ran.timed_out {
        return Err(InstallError::TimedOut);
    }
    if ran.status == Some(0) {
        return Ok(());
    }
    Err(InstallError::Failed {
        cause: python::cause_of(&ran.stderr),
        said: python::said(&ran.stderr).unwrap_or_default(),
    })
}

/// The environment's packages with a newer version in the index (`uv pip list --outdated`); KentOS's library is not
/// asked about: it is not the index's.
pub fn outdated(uv_path: &Path, root: &Path) -> Result<Vec<Outdated>, String> {
    #[derive(serde::Deserialize)]
    struct Row {
        name: String,
        version: String,
        latest_version: Option<String>,
    }
    let mut args = os(&[
        "pip",
        "list",
        "--outdated",
        "--exclude",
        "kentos",
        "--format",
        "json",
        "--no-config",
        "--system-certs",
        "--python",
    ]);
    args.push(python(root).into_os_string());
    let ran = uv_in(
        uv_path,
        root,
        &project(root),
        &uv::os_args(&args),
        RESOLVE_LIMIT,
    )
    .map_err(|said| preview_words(&said, ""))?;
    let rows: Vec<Row> = serde_json::from_str(ran.stdout.trim())
        .map_err(|e| format!("uv'nin güncelleme listesi okunamadı: {e}"))?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            Some(Outdated {
                name: normalize(&row.name),
                version: row.version,
                latest: row.latest_version?,
            })
        })
        .filter(|row| row.name != "kentos")
        .collect())
}

/// The environment's dependency tree as uv draws it (`uv tree`).
pub fn tree(uv_path: &Path, root: &Path) -> Result<String, String> {
    let args = os(&["tree", "--frozen", "--no-config"]);
    uv_in(
        uv_path,
        root,
        &project(root),
        &uv::os_args(&args),
        OFFLINE_LIMIT,
    )
    .map(|ran| ran.stdout)
}

/// uv's cache's bytes (KentOS's own, [`places::cache`]).
pub fn cache_size(root: &Path) -> u64 {
    python::folder_size(&places::cache(root))
}

/// Empties uv's cache of KentOS's. Environments keep working: their files are their own links. What they need again
/// is downloaded again.
pub fn clean_cache(uv_path: &Path, root: &Path) -> Result<(), String> {
    let args = os(&["cache", "clean", "--no-config"]);
    std::fs::create_dir_all(root).map_err(|e| format!("{} açılamadı: {e}", root.display()))?;
    uv_in(uv_path, root, root, &uv::os_args(&args), OFFLINE_LIMIT).map(|_| ())
}
