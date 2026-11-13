//! An extension version's own environment (P05, P07; PY-UV-10…12, 15…19, 22…24, 28, 30, 31).
//!
//! - **Its own, never shared:** an environment is keyed by the extension, its version, its lock's SHA-256, KentOS's
//!   Python, this system and the optional features chosen ([`Key`]); a short hash of the key names its folder
//!   (`<python>/ortamlar/<hash>`, short for Windows' path length). Two extensions asking for different versions of a
//!   package each get theirs (P05); the cache of downloads is shared, a venv never is (PY-UV-12).
//! - **From the package's lock alone:** `uv sync --locked --no-dev --no-install-project`: what the author locked, not a
//!   new resolution; development dependencies are not installed (PY-UV-17, 19). The project's two files are copied into
//!   the environment's folder: the installed package stays read-only.
//! - **Previewed first:** `uv sync --dry-run` says what it would install, on this computer, before anything is
//!   downloaded (PY-UV-14).
//! - **Made, checked, then marked ready:** after the sync, KentOS's library from the installation's wheel (offline, the
//!   lock may not name a package `kentos`: PY-UV-31), `uv pip check`, and an import of the library in the environment's
//!   own Python. Only then is the record [`RECORD`] written; a folder without it is a half-made environment, never used
//!   and removed when made again. The environment in use is never changed in place: another key, another folder
//!   (PY-UV-18).
//! - **Ready works offline (P07):** being ready is files on disk ([`ready`]); running uses the environment's own Python,
//!   and nothing syncs or downloads then. Measured: syncing a ready environment again removes KentOS's library, as an
//!   exact sync removes what the lock does not name, so nothing does.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::personal;
use crate::python::{self, Cause, InstallError, Interpreter, Progress};
use crate::sdk::Wheel;
use crate::{places, uv};

/// The record a ready environment's folder holds, written last.
pub const RECORD: &str = "ortam.json";

const FORMAT: &str = "kentos.extension-env";

/// How long a check may take.
const CHECK_LIMIT: Duration = Duration::from_secs(2 * 60);

/// What an environment is for: the same key, the same environment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Key {
    pub id: String,
    pub version: String,
    /// The lock's SHA-256.
    pub lock: String,
    /// KentOS's Python's version (`3.12.13`).
    pub python: String,
    /// This system (`linux-x86_64`).
    pub platform: String,
    /// The optional features chosen (the extras of the project), sorted.
    pub extras: Vec<String>,
}

impl Key {
    /// The folder's name: 16 hex digits of the key's SHA-256.
    pub fn hash(&self) -> String {
        let text = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            self.id,
            self.version,
            self.lock,
            self.python,
            self.platform,
            self.extras.join(",")
        );
        Sha256::digest(text.as_bytes())
            .iter()
            .take(8)
            .map(|b| format!("{b:02x}"))
            .collect()
    }
}

/// A ready environment, as its record says.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Ready {
    pub format: String,
    pub key: Key,
    /// Its Python.
    pub python: PathBuf,
    /// What it holds besides KentOS's library.
    pub packages: Vec<Listed>,
    /// The SHA-256 of KentOS's library's wheel it holds: another KentOS's is put in before a run ([`refresh_library`]).
    #[serde(default)]
    pub sdk: String,
    /// Seconds since 1970.
    pub made_at: u64,
}

/// A package of an environment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Listed {
    pub name: String,
    pub version: String,
}

/// Where the extensions' environments are.
pub fn folder(root: &Path) -> PathBuf {
    root.join("ortamlar")
}

/// An environment's folder.
pub fn place(root: &Path, key: &Key) -> PathBuf {
    folder(root).join(key.hash())
}

/// This system as keys name it (`linux-x86_64`).
pub fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// The SHA-256 of a package's lock.
pub fn lock_digest(package: &Path, lock_file: &str) -> Result<String, String> {
    let lock = package.join(lock_file);
    crate::sdk::sha256_file(&lock).map_err(|e| format!("{} okunamadı: {e}", lock.display()))
}

/// The key of an installed extension version's environment: its lock (`lock_file`, in the package) read for its
/// SHA-256; the extras chosen are sorted and kept once.
pub fn key_for(
    package: &Path,
    lock_file: &str,
    id: &str,
    version: &str,
    interpreter: &Interpreter,
    extras: &[String],
) -> Result<Key, String> {
    let sum = lock_digest(package, lock_file)?;
    let extras: BTreeSet<String> = extras.iter().map(|e| e.trim().to_owned()).collect();
    Ok(Key {
        id: id.to_owned(),
        version: version.to_owned(),
        lock: sum,
        python: interpreter.version.clone(),
        platform: platform(),
        extras: extras.into_iter().filter(|e| !e.is_empty()).collect(),
    })
}

/// The environment for `key` when it is ready: its record there and of this key, its Python there. Files only, no uv:
/// it answers offline.
pub fn ready(root: &Path, key: &Key) -> Option<Ready> {
    let dir = place(root, key);
    let text = std::fs::read_to_string(dir.join(RECORD)).ok()?;
    let record: Ready = serde_json::from_str(&text).ok()?;
    (record.format == FORMAT && &record.key == key && record.python.is_file()).then_some(record)
}

/// The ready environments' records, read from their folders.
fn records(root: &Path) -> Vec<Ready> {
    std::fs::read_dir(folder(root))
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|e| std::fs::read_to_string(e.path().join(RECORD)).ok())
                .filter_map(|text| serde_json::from_str::<Ready>(&text).ok())
                .filter(|r| r.format == FORMAT)
                .collect()
        })
        .unwrap_or_default()
}

/// A ready environment of an extension version on this Python and system, whatever features it was made with: the
/// newest, so its features are the ones the user chose last. Files only: it answers offline.
pub fn find(root: &Path, id: &str, version: &str, lock: &str, python: &str) -> Option<Ready> {
    let here = platform();
    records(root)
        .into_iter()
        .filter(|r| {
            r.key.id == id
                && r.key.version == version
                && r.key.lock == lock
                && r.key.python == python
                && r.key.platform == here
                && r.python.is_file()
        })
        .max_by_key(|r| r.made_at)
}

/// Removes an extension's environments but `keep`'s (none: all): made again with other features, or the extension
/// removed. Says how many went.
pub fn clean_for(root: &Path, id: &str, keep: Option<&Key>) -> usize {
    let kept = keep.map(Key::hash);
    records(root)
        .into_iter()
        .filter(|r| r.key.id == id && Some(r.key.hash()) != kept)
        .filter(|r| std::fs::remove_dir_all(place(root, &r.key)).is_ok())
        .count()
}

/// The project's two files, copied into `dir` as uv names them.
fn copy_project(
    package: &Path,
    project_file: &str,
    lock_file: &str,
    dir: &Path,
) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{} açılamadı: {e}", dir.display()))?;
    for (from, to) in [(project_file, "pyproject.toml"), (lock_file, "uv.lock")] {
        let source = package.join(from);
        std::fs::copy(&source, dir.join(to))
            .map_err(|e| format!("{} kopyalanamadı: {e}", source.display()))?;
    }
    Ok(())
}

/// Whether the lock names a package `kentos`: KentOS's library comes from the installation, never from an index.
pub fn shadows_library(lock: &str) -> bool {
    lock.lines().any(|line| {
        line.trim()
            .strip_prefix("name = ")
            .is_some_and(|name| personal::normalize(name.trim_matches('"')) == "kentos")
    })
}

const SHADOWS: &str = "Eklentinin kilidi “kentos” adlı bir paket istiyor; bu ad KentOS'un kendi kitaplığının, başka yerden kurulamaz.";

fn sync_args(interpreter: &Interpreter, key: &Key, dry: bool) -> Vec<OsString> {
    let mut args: Vec<OsString> = [
        "sync",
        "--locked",
        "--no-dev",
        "--no-install-project",
        "--no-config",
        "--no-python-downloads",
        "--system-certs",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    if dry {
        args.push(OsString::from("--dry-run"));
    }
    for extra in &key.extras {
        args.push(OsString::from("--extra"));
        args.push(OsString::from(extra));
    }
    args.push(OsString::from("--python"));
    args.push(interpreter.path.clone().into_os_string());
    args
}

/// The environment uv works with for one: KentOS's folders, and the environment's own venv.
fn env_for(root: &Path, venv: &Path) -> Vec<(&'static str, OsString)> {
    let mut env = python::uv_env(root);
    env.push(("UV_PROJECT_ENVIRONMENT", venv.as_os_str().to_owned()));
    env
}

fn refs<'a>(env: &'a [(&'static str, OsString)]) -> Vec<(&'static str, &'a OsStr)> {
    env.iter().map(|(name, v)| (*name, v.as_os_str())).collect()
}

/// The packages uv says it would install (` + numpy==2.5.4`).
pub fn would_install(said: &str) -> Vec<Listed> {
    said.lines()
        .filter_map(|line| line.trim().strip_prefix("+ "))
        .filter_map(|pinned| {
            let (name, version) = pinned.split_once("==")?;
            let version = version.split_whitespace().next()?;
            Some(Listed {
                name: personal::normalize(name),
                version: version.to_owned(),
            })
        })
        .collect()
}

/// What preparing `key`'s environment would install on this computer, read from the package's lock by uv without
/// downloading or writing anything (PY-UV-14).
pub fn preview(
    uv_path: &Path,
    root: &Path,
    interpreter: &Interpreter,
    package: &Path,
    files: (&str, &str),
    key: &Key,
) -> Result<Vec<Listed>, String> {
    let aside = folder(root).join(format!(".onizleme-{}", key.hash()));
    let _ = std::fs::remove_dir_all(&aside);
    copy_project(package, files.0, files.1, &aside)?;
    let lock = std::fs::read_to_string(aside.join("uv.lock")).unwrap_or_default();
    if shadows_library(&lock) {
        let _ = std::fs::remove_dir_all(&aside);
        return Err(SHADOWS.to_owned());
    }
    let env = env_for(root, &aside.join(".venv"));
    let args = sync_args(interpreter, key, true);
    let ran = uv::run(
        uv_path,
        &uv::os_args(&args),
        Some(&aside),
        &refs(&env),
        CHECK_LIMIT,
    );
    let _ = std::fs::remove_dir_all(&aside);
    let ran = ran?;
    if ran.status != Some(0) {
        return Err(lock_words(
            &python::said(&ran.stderr).unwrap_or_else(|| ran.stderr.trim().to_owned()),
        ));
    }
    Ok(would_install(&ran.stderr))
}

/// Why uv could not use the package's lock, and what it means for the user.
fn lock_words(said: &str) -> String {
    let lower = said.to_lowercase();
    if lower.contains("lockfile")
        && (lower.contains("out of date") || lower.contains("needs to be updated"))
    {
        format!(
            "Eklentinin kilidi projesiyle uyuşmuyor; yayımcısı “uv lock” çalıştırıp yeniden paketlemeli. uv: “{said}”"
        )
    } else if lower.contains("requires-python") || lower.contains("python version") {
        format!("Eklenti KentOS'un Python'uyla çalışmıyor. uv: “{said}”")
    } else {
        match python::cause_of(said) {
            Cause::Network => format!(
                "Paket dizinine ulaşılamadı. Bağlantıyı ya da vekil sunucu ayarını (HTTPS_PROXY) denetleyin. uv: “{said}”"
            ),
            _ => format!("Eklentinin ortamı hazırlanamadı. uv: “{said}”"),
        }
    }
}

/// Prepares `key`'s environment from the package's lock: synced (downloads said to `told`), KentOS's library put in,
/// checked, and only then marked ready. Stopped as soon as `cancel` is set; whatever ends it before the record, the
/// half-made folder goes. A ready one is used as it is.
#[allow(clippy::too_many_arguments)]
pub fn prepare(
    uv_path: &Path,
    root: &Path,
    interpreter: &Interpreter,
    sdk: &Wheel,
    package: &Path,
    files: (&str, &str),
    key: &Key,
    cancel: &AtomicBool,
    told: &mut dyn FnMut(Progress),
) -> Result<Ready, InstallError> {
    if let Some(made) = ready(root, key) {
        return Ok(made);
    }
    let dir = place(root, key);
    let made = make(
        uv_path,
        root,
        interpreter,
        sdk,
        package,
        files,
        key,
        &dir,
        cancel,
        told,
    );
    if made.is_err() {
        let _ = std::fs::remove_dir_all(&dir);
    }
    made
}

#[allow(clippy::too_many_arguments)]
fn make(
    uv_path: &Path,
    root: &Path,
    interpreter: &Interpreter,
    sdk: &Wheel,
    package: &Path,
    files: (&str, &str),
    key: &Key,
    dir: &Path,
    cancel: &AtomicBool,
    told: &mut dyn FnMut(Progress),
) -> Result<Ready, InstallError> {
    let other = |said: String| InstallError::Failed {
        cause: Cause::Other,
        said,
    };
    // A folder without its record is a half-made one: made again from nothing.
    let _ = std::fs::remove_dir_all(dir);
    let project = dir.join("proje");
    copy_project(package, files.0, files.1, &project).map_err(|said| InstallError::Failed {
        cause: Cause::Folder,
        said,
    })?;
    let lock = std::fs::read_to_string(project.join("uv.lock")).unwrap_or_default();
    if shadows_library(&lock) {
        return Err(other(SHADOWS.to_owned()));
    }
    let venv = dir.join(".venv");
    let env = env_for(root, &venv);
    let args = sync_args(interpreter, key, false);
    let heard = Arc::new(Mutex::new(String::new()));
    let cache = places::cache(root);
    let mut last = Progress::Starting;
    told(last);
    let mut tick = || {
        let unpacking = personal::under_way(&cache);
        let now = heard
            .lock()
            .map(|text| personal::read_sync_progress(&text, unpacking))
            .unwrap_or(last);
        if now != last {
            last = now;
            told(now);
        }
    };
    let ran = uv::run_watched(
        uv_path,
        &uv::os_args(&args),
        Some(&project),
        &refs(&env),
        personal::SYNC_LIMIT,
        uv::Watch {
            cancel: Some(cancel),
            stderr: Some(heard.clone()),
            tick: Some(&mut tick),
        },
    )
    .map_err(InstallError::NotRun)?;
    tick();
    if ran.cancelled {
        return Err(InstallError::Cancelled);
    }
    if ran.timed_out {
        return Err(InstallError::TimedOut);
    }
    if ran.status != Some(0) {
        let said = python::said(&ran.stderr).unwrap_or_default();
        return Err(match python::cause_of(&ran.stderr) {
            Cause::Other => other(lock_words(&said)),
            cause => InstallError::Failed { cause, said },
        });
    }
    let python = personal::python_in(&venv, std::env::consts::OS);
    // KentOS's library, from the installation's wheel, offline.
    let mut library: Vec<OsString> = [
        "pip",
        "install",
        "--no-deps",
        "--offline",
        "--no-config",
        "--python",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    library.push(python.clone().into_os_string());
    library.push(sdk.path.clone().into_os_string());
    check_ran(uv::run(
        uv_path,
        &uv::os_args(&library),
        Some(dir),
        &refs(&env),
        CHECK_LIMIT,
    ))
    .map_err(|why| other(format!("KentOS'un Python kitaplığı kurulamadı: {why}")))?;
    // Consistent, and the library imports in the environment's own Python.
    let mut check: Vec<OsString> = ["pip", "check", "--no-config", "--python"]
        .iter()
        .map(OsString::from)
        .collect();
    check.push(python.clone().into_os_string());
    check_ran(uv::run(
        uv_path,
        &uv::os_args(&check),
        Some(dir),
        &refs(&env),
        CHECK_LIMIT,
    ))
    .map_err(|why| other(format!("Ortamdaki paketler birbiriyle uyuşmuyor: {why}")))?;
    let smoke = ["-I", "-c", "import kentos"].map(OsStr::new);
    check_ran(uv::run(&python, &smoke, Some(dir), &[], CHECK_LIMIT)).map_err(|why| {
        other(format!(
            "Ortamın Python'u KentOS kitaplığını açamadı: {why}"
        ))
    })?;
    let packages = listed(uv_path, dir, &python, &env).map_err(other)?;
    let record = Ready {
        format: FORMAT.to_owned(),
        key: key.clone(),
        python,
        packages,
        sdk: sdk.sha256.clone(),
        made_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default(),
    };
    // The record last: before it, the folder is a half-made environment.
    let text = serde_json::to_string_pretty(&record).unwrap_or_default();
    let part = dir.join(format!("{RECORD}.yazilan"));
    std::fs::write(&part, text)
        .and_then(|()| std::fs::rename(&part, dir.join(RECORD)))
        .map_err(|e| InstallError::Failed {
            cause: Cause::Folder,
            said: e.to_string(),
        })?;
    Ok(record)
}

fn check_ran(ran: Result<uv::Ran, String>) -> Result<uv::Ran, String> {
    let ran = ran?;
    if ran.status == Some(0) {
        Ok(ran)
    } else if ran.timed_out {
        Err("yanıt vermedi".to_owned())
    } else {
        let said = python::said(&ran.stderr)
            .or_else(|| {
                ran.stderr
                    .lines()
                    .rev()
                    .find(|l| !l.trim().is_empty())
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| ran.stdout.trim().to_owned());
        Err(said)
    }
}

/// What an environment holds besides KentOS's library (`uv pip list`).
fn listed(
    uv_path: &Path,
    dir: &Path,
    python: &Path,
    env: &[(&'static str, OsString)],
) -> Result<Vec<Listed>, String> {
    let mut args: Vec<OsString> = [
        "pip",
        "list",
        "--format",
        "json",
        "--exclude",
        "kentos",
        "--no-config",
        "--python",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    args.push(python.as_os_str().to_owned());
    let ran = check_ran(uv::run(
        uv_path,
        &uv::os_args(&args),
        Some(dir),
        &refs(env),
        CHECK_LIMIT,
    ))?;
    let rows: Vec<Listed> = serde_json::from_str(ran.stdout.trim())
        .map_err(|e| format!("uv'nin paket listesi okunamadı: {e}"))?;
    Ok(rows
        .into_iter()
        .map(|row| Listed {
            name: personal::normalize(&row.name),
            version: row.version,
        })
        .collect())
}

/// Puts this KentOS's library into a ready environment when it holds another build (KentOS updated since it was
/// made): offline, from the installation's wheel, its record updated. The environment's packages are not touched, so
/// a ready environment keeps working without the internet after an update (P07, PY-UV-31).
pub fn refresh_library(
    uv_path: &Path,
    root: &Path,
    ready: &Ready,
    sdk: &Wheel,
) -> Result<Ready, String> {
    if ready.sdk == sdk.sha256 {
        return Ok(ready.clone());
    }
    let dir = place(root, &ready.key);
    let env = env_for(root, &dir.join(".venv"));
    let mut args: Vec<OsString> = [
        "pip",
        "install",
        "--no-deps",
        "--offline",
        "--no-config",
        "--reinstall-package",
        "kentos",
        "--python",
    ]
    .iter()
    .map(OsString::from)
    .collect();
    args.push(ready.python.clone().into_os_string());
    args.push(sdk.path.clone().into_os_string());
    check_ran(uv::run(
        uv_path,
        &uv::os_args(&args),
        Some(&dir),
        &refs(&env),
        CHECK_LIMIT,
    ))
    .map_err(|why| format!("KentOS'un Python kitaplığı ortama konamadı: {why}"))?;
    let fresh = Ready {
        sdk: sdk.sha256.clone(),
        ..ready.clone()
    };
    let text = serde_json::to_string_pretty(&fresh).unwrap_or_default();
    let part = dir.join(format!("{RECORD}.yazilan"));
    std::fs::write(&part, text)
        .and_then(|()| std::fs::rename(&part, dir.join(RECORD)))
        .map_err(|e| format!("{} yazılamadı: {e}", dir.join(RECORD).display()))?;
    Ok(fresh)
}

/// Removes `key`'s environment.
pub fn remove(root: &Path, key: &Key) -> Result<(), String> {
    let dir = place(root, key);
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!(
            "{} silinemedi: {e}. Eklentinin işlemi sürüyorsa bitmesini bekleyin.",
            dir.display()
        )),
    }
}

/// The environments' folders no key in `keep` names: an extension version removed, a lock changed, KentOS's Python
/// changed. Only folders named as KentOS names them (16 hex digits) count, and previews left aside (PY-UV-30).
pub fn unused(root: &Path, keep: &[Key]) -> Vec<PathBuf> {
    let kept: BTreeSet<String> = keep.iter().map(Key::hash).collect();
    let Ok(entries) = std::fs::read_dir(folder(root)) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let ours = name.len() == 16 && name.chars().all(|c| c.is_ascii_hexdigit())
                || name.starts_with(".onizleme-");
            ours && !kept.contains(&name)
        })
        .map(|e| e.path())
        .collect();
    out.sort();
    out
}

/// Removes the environments no key in `keep` names; says how many went.
pub fn clean(root: &Path, keep: &[Key]) -> usize {
    unused(root, keep)
        .into_iter()
        .filter(|dir| std::fs::remove_dir_all(dir).is_ok())
        .count()
}
