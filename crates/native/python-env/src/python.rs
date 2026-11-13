//! KentOS's own Python (PY-UV-03…07, PY-UV-16, PY-UV-23, PY-UV-24; the owner's decision, 13 November 2026): not part of
//! the installation and never inside KentOS's process. uv installs it into the user's KentOS folder
//! ([`places::interpreters`]) only when the user asks for it, having been told what is downloaded, from where and where
//! it goes; nothing is downloaded at run time.
//!
//! - **Nothing of the user's system changes:** `--no-bin` (no `python3.12` put in `~/.local/bin`), `--no-registry` (no
//!   Windows registry entry, PEP 514), `--no-config` (the user's own uv settings do not decide), and KentOS's own folders
//!   for uv's Pythons and cache (`UV_PYTHON_INSTALL_DIR`, `UV_CACHE_DIR`): the user's own uv Pythons and cache are not
//!   touched. Removing it is removing the folder.
//! - **Organisation networks:** the system's certificate store (`--system-certs`): a network that inspects TLS signs
//!   with a certificate the organisation put on its computers, which uv's built-in list does not know. The proxy
//!   (`HTTPS_PROXY`) is the system's, as the browser's.
//! - **Found** with `uv python list --only-installed --managed-python`, run in KentOS's folder: only KentOS's Pythons
//!   count; an active venv, a `.venv` nearby or the user's own uv Pythons do not. uv writes the path of one under that
//!   folder relative to it; it is read against the folder.
//! - **Fails clearly (PY-UV-07):** no internet, a certificate the computer does not trust, a full disk, a folder it may
//!   not write, a version this system has no build of, the time limit, the user's Durdur: each said in a sentence with
//!   what to do, and uv's own words beside it.
//! - **How far it is** (the owner's request, 13 November 2026: "hoş bir progress"): uv says the archive's size as it
//!   starts (`Downloading cpython-… (32.6MiB)`) and the archive grows as one file in a download folder of the install's
//!   own; the two give the share downloaded. Then it unpacks (`Extracting`), which has no measure.
//! - **Removed from the same place** (the owner's request: "Kaldırılabilmelidir"): uv uninstalls KentOS's Pythons and
//!   their folder goes with what uv leaves behind; the bytes freed are said.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Deserialize;

use crate::{compat, places, uv};

/// How long listing may take.
const LIST_LIMIT: Duration = Duration::from_secs(30);

/// How long installing may take: 35 MB over a slow line.
pub const INSTALL_LIMIT: Duration = Duration::from_secs(20 * 60);

/// How long removing may take.
const REMOVE_LIMIT: Duration = Duration::from_secs(5 * 60);

/// One of KentOS's installed Pythons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interpreter {
    /// `3.12.13`.
    pub version: String,
    /// The program: `…/yorumlayici/cpython-3.12-…/bin/python3.12`; on Windows `…\python.exe`.
    pub path: PathBuf,
}

/// The environment uv runs with for KentOS's Pythons: KentOS's folders for them and for its cache.
pub fn uv_env(root: &Path) -> Vec<(&'static str, OsString)> {
    vec![
        (
            "UV_PYTHON_INSTALL_DIR",
            places::interpreters(root).into_os_string(),
        ),
        ("UV_CACHE_DIR", places::cache(root).into_os_string()),
    ]
}

fn env_refs<'a>(env: &'a [(&'static str, OsString)]) -> Vec<(&'static str, &'a OsStr)> {
    env.iter().map(|(name, v)| (*name, v.as_os_str())).collect()
}

/// A row of `uv python list --output-format json`.
#[derive(Deserialize)]
struct Listed {
    version: String,
    path: Option<PathBuf>,
    #[serde(default)]
    implementation: String,
    #[serde(default)]
    variant: String,
}

/// KentOS's newest installed CPython in `range` (`>=3.10,<3.15`); none when there is none.
pub fn installed(uv_path: &Path, root: &Path, range: &str) -> Result<Option<Interpreter>, String> {
    std::fs::create_dir_all(root).map_err(|e| format!("{} açılamadı: {e}", root.display()))?;
    let env = uv_env(root);
    let args = [
        "python",
        "list",
        "--only-installed",
        "--managed-python",
        "--no-config",
        "--output-format",
        "json",
        range,
    ]
    .map(OsStr::new);
    let ran = uv::run(uv_path, &args, Some(root), &env_refs(&env), LIST_LIMIT)?;
    if ran.status != Some(0) {
        return Err(said(&ran.stderr).unwrap_or_else(|| {
            if ran.timed_out {
                "uv yanıt vermedi".to_owned()
            } else {
                "uv listeyi vermedi".to_owned()
            }
        }));
    }
    let listed: Vec<Listed> = serde_json::from_str(ran.stdout.trim())
        .map_err(|e| format!("uv'nin Python listesi okunamadı: {e}"))?;
    Ok(listed
        .into_iter()
        .filter(|l| {
            l.implementation.eq_ignore_ascii_case("cpython")
                && (l.variant.is_empty() || l.variant == "default")
        })
        // uv writes a path under the folder it runs in relative to it (measured, 0.11.26).
        .filter_map(|l| {
            Some(Interpreter {
                path: root.join(l.path?),
                version: l.version,
            })
        })
        .max_by_key(|i| compat::triple(&i.version)))
}

/// What kept a Python from being installed, as the program understands uv's words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Network,
    Certificate,
    Disk,
    Folder,
    /// No build of the version for this system.
    Unavailable,
    Other,
}

/// Why installing did not finish.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InstallError {
    /// The user stopped it.
    Cancelled,
    /// It took longer than [`INSTALL_LIMIT`].
    TimedOut,
    /// uv could not be started.
    NotRun(String),
    /// uv said why: what the program understood, and uv's own words.
    Failed { cause: Cause, said: String },
}

impl InstallError {
    /// What happened and what to do, as the window says it.
    pub fn words(&self, version: &str) -> String {
        let (sentence, said) = match self {
            InstallError::Cancelled => {
                return "Python kurulumu durduruldu; istediğinizde yeniden başlatabilirsiniz.".to_owned();
            }
            InstallError::TimedOut => {
                return format!(
                    "Python kurulumu {} dakikada bitmedi ve durduruldu: bağlantı çok yavaş olabilir. Yeniden deneyin.",
                    INSTALL_LIMIT.as_secs() / 60
                );
            }
            InstallError::NotRun(why) => return format!("uv çalıştırılamadı: {why}."),
            InstallError::Failed { cause, said } => (
                match cause {
                    Cause::Network => {
                        "Python indirilemedi: internete ulaşılamadı. Bağlantıyı ya da vekil sunucu ayarını \
                         (HTTPS_PROXY) denetleyip yeniden deneyin."
                            .to_owned()
                    }
                    Cause::Certificate => {
                        "Python indirilemedi: sunucunun sertifikası bu bilgisayarda doğrulanamadı. Kurum ağı \
                         bağlantıları denetliyorsa kurumun sertifikası bilgisayarda yüklü olmalı; BT'ye sorun."
                            .to_owned()
                    }
                    Cause::Disk => {
                        "Python kurulamadı: diskte yer yok. Yer açıp yeniden deneyin.".to_owned()
                    }
                    Cause::Folder => {
                        "Python kurulamadı: KentOS'un Python klasörüne yazılamadı. Klasörün izinlerini denetleyin."
                            .to_owned()
                    }
                    Cause::Unavailable => {
                        format!("Python {version} bu sistem için indirilemiyor.")
                    }
                    Cause::Other => "Python kurulamadı.".to_owned(),
                },
                said,
            ),
        };
        if said.is_empty() {
            sentence
        } else {
            format!("{sentence} uv: “{said}”")
        }
    }
}

/// How far installing is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    /// uv has not said what it downloads yet.
    Starting,
    /// Downloading: the bytes so far, of `total` once uv said it.
    Downloading { done: u64, total: Option<u64> },
    /// Unpacking and putting it in place.
    Unpacking,
}

impl Progress {
    /// The share done, 0..=1, when it can be told.
    pub fn share(&self) -> Option<f32> {
        match *self {
            Progress::Downloading {
                done,
                total: Some(total),
            } if total > 0 => Some((done as f64 / total as f64).min(1.0) as f32),
            _ => None,
        }
    }
}

/// How far installing is from what uv wrote so far (`Downloading cpython-… (32.6MiB)`, then `Downloaded` and
/// `Extracting`) and the bytes in its download folder (measured, 0.11.26: the archive grows there as one file).
pub fn read_progress(said: &str, downloaded: u64) -> Progress {
    let lines = || said.lines().map(str::trim);
    let total = lines().find_map(|line| {
        let rest = line.strip_prefix("Downloading ")?;
        let open = rest.rfind('(')?;
        let close = rest.rfind(')')?;
        bytes(rest.get(open + 1..close)?)
    });
    if lines().any(|line| {
        ["Downloaded ", "Extracting ", "Extracted "]
            .iter()
            .any(|word| line.starts_with(word))
    }) {
        Progress::Unpacking
    } else if total.is_some() || downloaded > 0 {
        Progress::Downloading {
            done: downloaded,
            total,
        }
    } else {
        Progress::Starting
    }
}

/// A size as uv writes it: `32.6MiB`, `512KiB`, `1.2GiB`, `900B`.
pub fn bytes(text: &str) -> Option<u64> {
    let text = text.trim();
    let split = text.find(|c: char| !(c.is_ascii_digit() || c == '.'))?;
    let (number, unit) = text.split_at(split);
    let number: f64 = number.parse().ok()?;
    let scale = match unit.trim() {
        "B" => 1.0,
        "KiB" => 1024.0,
        "MiB" => 1024.0 * 1024.0,
        "GiB" => 1024.0 * 1024.0 * 1024.0,
        "kB" | "KB" => 1e3,
        "MB" => 1e6,
        "GB" => 1e9,
        _ => return None,
    };
    Some((number * scale).round() as u64)
}

/// The bytes of the files under `dir`, links not followed (a minor version's link to its folder is not counted
/// twice; on Windows a junction is a link); 0 when it is not there.
pub fn folder_size(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .flatten()
        .map(|entry| match entry.file_type() {
            Ok(kind) if kind.is_dir() => folder_size(&entry.path()),
            Ok(kind) if kind.is_file() => entry.metadata().map(|m| m.len()).unwrap_or(0),
            _ => 0,
        })
        .sum()
}

/// Installs Python `version` (`3.12`) into KentOS's folder, saying how far it is to `told` as it changes (about every
/// [`uv::TICK`]); stopped as soon as `cancel` is set. The archive is downloaded into a folder of this install's own
/// ([`places::downloads`]) and removed with it, however it ends.
pub fn install(
    uv_path: &Path,
    root: &Path,
    version: &str,
    cancel: &AtomicBool,
    told: &mut dyn FnMut(Progress),
) -> Result<(), InstallError> {
    let folder = |e: std::io::Error| InstallError::Failed {
        cause: Cause::Folder,
        said: e.to_string(),
    };
    std::fs::create_dir_all(places::interpreters(root)).map_err(folder)?;
    // Two KentOS windows may install at once (uv's lock lets one wait): each its own download folder.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or_default();
    let downloads = places::downloads(root).join(format!("{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&downloads).map_err(folder)?;
    let mut env = uv_env(root);
    env.push(("UV_PYTHON_CACHE_DIR", downloads.clone().into_os_string()));
    let args = [
        "python",
        "install",
        version,
        "--no-bin",
        "--no-registry",
        "--no-config",
        "--no-progress",
        "--system-certs",
    ]
    .map(OsStr::new);
    let heard = Arc::new(Mutex::new(String::new()));
    let mut last = Progress::Starting;
    told(last);
    let mut tick = || {
        let now = heard
            .lock()
            .map(|text| read_progress(&text, folder_size(&downloads)))
            .unwrap_or(last);
        if now != last {
            last = now;
            told(now);
        }
    };
    let ran = uv::run_watched(
        uv_path,
        &args,
        Some(root),
        &env_refs(&env),
        INSTALL_LIMIT,
        uv::Watch {
            cancel: Some(cancel),
            stderr: Some(heard.clone()),
            tick: Some(&mut tick),
        },
    );
    let _ = std::fs::remove_dir_all(&downloads);
    // The downloads' folder too, when no other install is using it.
    let _ = std::fs::remove_dir(places::downloads(root));
    let ran = ran.map_err(InstallError::NotRun)?;
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
        cause: cause_of(&ran.stderr),
        said: said(&ran.stderr).unwrap_or_default(),
    })
}

/// Removes KentOS's Pythons, as the user asked: uv uninstalls them (when it is there), then their folder and what
/// downloads left go, so nothing of them stays. Says the bytes freed. Nothing else is touched: the user's own Pythons,
/// uv and its settings are not KentOS's.
pub fn remove(uv_path: Option<&Path>, root: &Path) -> Result<u64, String> {
    let folder = places::interpreters(root);
    let downloads = places::downloads(root);
    let freed = folder_size(&folder) + folder_size(&downloads);
    if let Some(uv_path) = uv_path
        && folder.is_dir()
    {
        let env = uv_env(root);
        let args = ["python", "uninstall", "--all", "--no-config"].map(OsStr::new);
        let ran = uv::run(uv_path, &args, Some(root), &env_refs(&env), REMOVE_LIMIT)?;
        if ran.status != Some(0) {
            return Err(removal_words(
                &said(&ran.stderr).unwrap_or_else(|| ran.stderr.trim().to_owned()),
            ));
        }
    }
    // uv leaves its lock and work folders behind.
    for dir in [folder, downloads] {
        match std::fs::remove_dir_all(&dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(removal_words(&e.to_string())),
        }
    }
    Ok(freed)
}

/// Why removing failed, and what to do: a Python still running holds its files on Windows.
fn removal_words(why: &str) -> String {
    let lower = why.to_lowercase();
    if [
        "os error 32",
        "used by another process",
        "text file busy",
        "resource busy",
    ]
    .iter()
    .any(|w| lower.contains(w))
    {
        format!(
            "Python kaldırılamadı: dosyaları kullanılıyor. Python konsolunu ve Python'lu eklentileri kapatıp yeniden \
             deneyin. ({why})"
        )
    } else {
        format!("Python kaldırılamadı: {why}")
    }
}

/// What uv's error words mean. The certificate first: uv says a certificate failure as a failed download too.
pub fn cause_of(stderr: &str) -> Cause {
    let text = stderr.to_lowercase();
    let any = |words: &[&str]| words.iter().any(|w| text.contains(w));
    if any(&["certificate", "unknownissuer", "self signed", "self-signed"]) {
        Cause::Certificate
    } else if any(&[
        "no space left",
        "os error 28",
        "os error 112",
        "not enough space",
        "disk quota",
    ]) {
        Cause::Disk
    } else if any(&[
        "permission denied",
        "access is denied",
        "os error 13",
        "os error 5)",
        "read-only file system",
    ]) {
        Cause::Folder
    } else if any(&["no download found", "no python found for request"]) {
        Cause::Unavailable
    } else if any(&[
        "dns error",
        "failed to lookup address",
        "connection refused",
        "connection reset",
        "tcp connect error",
        "timed out",
        "network is unreachable",
        "error sending request",
        "proxy",
    ]) {
        Cause::Network
    } else {
        Cause::Other
    }
}

/// uv's own words: its first `error:` line and the last cause under it, at most 300 characters.
pub fn said(stderr: &str) -> Option<String> {
    let lines: Vec<&str> = stderr.lines().map(str::trim).collect();
    let first = lines
        .iter()
        .find_map(|l| l.strip_prefix("error:"))
        .map(str::trim)?;
    let last = lines
        .iter()
        .rev()
        .find_map(|l| l.strip_prefix("Caused by:"))
        .map(str::trim);
    let text = match last {
        Some(cause) if cause != first => format!("{first} · {cause}"),
        _ => first.to_owned(),
    };
    Some(text.chars().take(300).collect())
}
