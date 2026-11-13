//! uv found and run (PY-UV-01, PY-UV-02, PY-UV-08, PY-UV-22, PY-UV-24; PY-OPT-03).
//!
//! - **Found:** the path the user chose, alone when there is one; else **the uv KentOS's installation carries** (the
//!   owner's decision, 13 November 2026: `tools/uv[.exe]` beside the program; in a macOS app bundle `Contents/MacOS/uv`,
//!   flat beside the program, where Apple asks helper tools to be: `Resources` is not for code, and code in nested
//!   folders breaks signing); else `PATH`, then the folders uv's installers use that
//!   a desktop session's `PATH` often lacks (a program started from the menu does not read the shell's profile, and a
//!   running Windows program does not see a `PATH` an installer changed after it started): `~/.local/bin` (the
//!   standalone installer, on Windows `%USERPROFILE%\.local\bin`), `~/.cargo/bin`; on Windows also WinGet's
//!   `%LOCALAPPDATA%\Microsoft\WinGet\Links` and Scoop's `%USERPROFILE%\scoop\shims`; on macOS `/opt/homebrew/bin`
//!   and `/usr/local/bin`. Its version is asked (`uv --version`) and checked against the manifest. uv is never replaced.
//! - **On Windows** uv starts without a console window (`CREATE_NO_WINDOW`): started from a windowed program, a console
//!   program would flash a black window at every call.
//! - **Run:** with its arguments (argv), never through a shell; at most a time limit long, its output cut at
//!   [`MAX_OUTPUT`] bytes a stream; with none of the environment's uv or Python settings (`UV_*`, `VIRTUAL_ENV`,
//!   `PYTHONPATH`, `PYTHONHOME`, `CONDA_PREFIX`), so an active venv, a `.python-version` or the user's own uv settings
//!   never decide where it goes: callers pass `--no-config` and explicit paths.

use std::ffi::{OsStr, OsString};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::compat::{self, Compat};

/// Bytes kept of a stream's output.
pub const MAX_OUTPUT: usize = 256 * 1024;

/// How long `uv --version` may take.
const VERSION_LIMIT: Duration = Duration::from_secs(10);

/// What looking for uv found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UvStatus {
    /// Found and new enough; `bundled` when it is the one KentOS's installation carries.
    Found {
        path: PathBuf,
        version: String,
        bundled: bool,
    },
    /// Found, older than the manifest asks.
    TooOld {
        path: PathBuf,
        version: String,
        min: String,
    },
    /// A file is there but does not run, or does not say its version.
    NotRunnable { path: PathBuf, why: String },
    /// Not where it was looked for; `own` when the installation's own folder was among them (not when the user's
    /// chosen path was looked at alone).
    NotFound { looked: Vec<PathBuf>, own: bool },
    /// This computer's platform is not one the manifest lists.
    Unsupported { platform: String },
}

impl UvStatus {
    /// Ready to use.
    pub fn ok(&self) -> bool {
        matches!(self, UvStatus::Found { .. })
    }

    /// Whether uv's official ways in help: missing or too old; not when only the user's own path was wrong, for then
    /// the way out is that path (left empty, KentOS's own uv is used).
    pub fn offers_install(&self) -> bool {
        match self {
            UvStatus::TooOld { .. } => true,
            UvStatus::NotFound { looked, own } => *own || looked.len() != 1,
            _ => false,
        }
    }

    /// uv's path, when one was found.
    pub fn path(&self) -> Option<&Path> {
        match self {
            UvStatus::Found { path, .. }
            | UvStatus::TooOld { path, .. }
            | UvStatus::NotRunnable { path, .. } => Some(path),
            _ => None,
        }
    }

    /// What it means and what to do, as the window says it.
    pub fn words(&self) -> String {
        match self {
            UvStatus::Found {
                path,
                version,
                bundled: true,
            } => format!("KentOS'la gelen uv {version} hazır ({}).", path.display()),
            UvStatus::Found { path, version, .. } => {
                format!("uv {version} bulundu: {}.", path.display())
            }
            UvStatus::TooOld { path, version, min } => format!(
                "uv {version} çok eski ({}); en az {min} gerekiyor. uv'yi kendi yoluyla güncelleyin (`uv self update`) \
                 ve yeniden algılayın.",
                path.display()
            ),
            UvStatus::NotRunnable { path, why } => format!(
                "{} çalıştırılamıyor: {why}. Doğru dosya mı, çalıştırma izni var mı? Yolu düzeltip yeniden algılayın.",
                path.display()
            ),
            // The user's own path, looked at alone: KentOS's own uv was not looked for.
            UvStatus::NotFound { looked, own: false } if looked.len() == 1 => format!(
                "uv verilen yolda yok: {}. Yolu düzeltin ya da silip Uygula'ya basın: boş bırakılınca KentOS önce \
                 kendi uv'sini kullanır.",
                looked[0].display()
            ),
            UvStatus::NotFound { looked, own: true } => format!(
                "uv bulunamadı ({} yere bakıldı, KentOS'un kendi klasörü dahil). KentOS'la gelen uv silinmiş olabilir: \
                 KentOS'u yeniden kurun ya da aşağıdaki resmî komutu kendiniz çalıştırıp yeniden algılayın; uv'nin \
                 yolunu elle de verebilirsiniz.",
                looked.len()
            ),
            UvStatus::NotFound { looked, .. } => format!(
                "uv bulunamadı ({} yere bakıldı). Kurmak isterseniz aşağıdaki resmî komutu kendiniz çalıştırın, sonra \
                 yeniden algılayın; ya da uv'nin yolunu elle verin.",
                looked.len()
            ),
            UvStatus::Unsupported { platform } => format!(
                "Bu sistem ({platform}) KentOS'un Python'unun desteklediklerinden değil; Python eklentileri burada \
                 çalışmaz."
            ),
        }
    }
}

/// The program's file name on a system.
fn exe(os: &str) -> &'static str {
    if os == "windows" { "uv.exe" } else { "uv" }
}

/// Where uv is looked for on this computer's system: see [`candidates_for`].
pub fn candidates(
    chosen: Option<&Path>,
    bundled: &[PathBuf],
    path_var: Option<&OsStr>,
    home: Option<&Path>,
    local_app_data: Option<&Path>,
) -> Vec<PathBuf> {
    candidates_for(
        std::env::consts::OS,
        chosen,
        bundled,
        path_var,
        home,
        local_app_data,
    )
}

/// The folders this program's installation keeps its own uv in ([`bundled_dirs_for`]); none when the program's path is
/// not known.
pub fn bundled_dirs() -> Vec<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .map(|dir| bundled_dirs_for(std::env::consts::OS, &dir))
        .unwrap_or_default()
}

/// The folders an installation keeps its own uv in, for a program in `dir` on a system: `tools` beside it; in a macOS
/// app bundle also the program's own folder (`Contents/MacOS`). Nowhere else: a program in `/usr/bin` does not make
/// `/usr/bin/uv` KentOS's.
pub fn bundled_dirs_for(os: &str, dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![dir.join("tools")];
    if os == "macos" && dir.ends_with("Contents/MacOS") {
        out.push(dir.to_path_buf());
    }
    out
}

/// Where uv is looked for on a system (`linux`, `windows`, `macos`), in order: `chosen` alone when given; else the
/// installation's own folders (`bundled`), every folder of `path_var`, then the user folders uv's installers use.
/// `home` is `%USERPROFILE%` on Windows.
pub fn candidates_for(
    os: &str,
    chosen: Option<&Path>,
    bundled: &[PathBuf],
    path_var: Option<&OsStr>,
    home: Option<&Path>,
    local_app_data: Option<&Path>,
) -> Vec<PathBuf> {
    if let Some(chosen) = chosen {
        return vec![chosen.to_path_buf()];
    }
    let name = exe(os);
    let mut out: Vec<PathBuf> = bundled.iter().map(|dir| dir.join(name)).collect();
    out.extend(
        path_var
            .map(|p| {
                std::env::split_paths(p)
                    .map(|dir| dir.join(name))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default(),
    );
    if let Some(home) = home {
        out.push(home.join(".local").join("bin").join(name));
        out.push(home.join(".cargo").join("bin").join(name));
    }
    if os == "windows" {
        if let Some(local) = local_app_data {
            out.push(
                local
                    .join("Microsoft")
                    .join("WinGet")
                    .join("Links")
                    .join(name),
            );
        }
        if let Some(home) = home {
            out.push(home.join("scoop").join("shims").join(name));
        }
    }
    if os == "macos" {
        out.push(PathBuf::from("/opt/homebrew/bin").join(name));
        out.push(PathBuf::from("/usr/local/bin").join(name));
    }
    let mut seen = std::collections::BTreeSet::new();
    out.retain(|p| seen.insert(p.clone()));
    out
}

/// Looks for uv on this computer: the chosen path; else the installation's own, `PATH`, the user folders.
pub fn detect(chosen: Option<&Path>) -> UvStatus {
    let path_var = std::env::var_os("PATH");
    let home =
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let bundled = bundled_dirs();
    detect_in(
        compat::compat(),
        &compat::this_platform(),
        &bundled,
        &candidates(
            chosen,
            &bundled,
            path_var.as_deref(),
            home.as_deref(),
            local.as_deref(),
        ),
    )
}

/// The first of `looked` that is a file, asked its version and checked against `compat`; found in one of `bundled`, it
/// is the installation's own.
pub fn detect_in(
    compat: &Compat,
    platform: &compat::Platform,
    bundled: &[PathBuf],
    looked: &[PathBuf],
) -> UvStatus {
    if !compat.supports(platform) {
        return UvStatus::Unsupported {
            platform: format!("{} {}", platform.os, platform.arch),
        };
    }
    let Some(path) = looked.iter().find(|p| p.is_file()) else {
        return UvStatus::NotFound {
            own: looked.iter().any(|p| {
                p.parent()
                    .is_some_and(|dir| bundled.iter().any(|b| b == dir))
            }),
            looked: looked.to_vec(),
        };
    };
    let ran = match run(path, &[OsStr::new("--version")], None, &[], VERSION_LIMIT) {
        Ok(ran) => ran,
        Err(why) => {
            return UvStatus::NotRunnable {
                path: path.clone(),
                why,
            };
        }
    };
    let version = ran
        .stdout
        .split_whitespace()
        .nth(1)
        .filter(|_| ran.stdout.starts_with("uv ") && ran.status == Some(0))
        .and_then(|v| compat::triple(v).map(|_| v.to_owned()));
    let Some(version) = version else {
        let said = ran
            .stdout
            .trim()
            .lines()
            .next()
            .unwrap_or_default()
            .to_owned();
        return UvStatus::NotRunnable {
            path: path.clone(),
            why: if ran.timed_out {
                "sürümünü sorunca yanıt vermedi".to_owned()
            } else {
                format!("sürümünü söylemedi (“{said}”)")
            },
        };
    };
    if compat.uv_new_enough(&version) {
        UvStatus::Found {
            bundled: path
                .parent()
                .is_some_and(|dir| bundled.iter().any(|b| b == dir)),
            path: path.clone(),
            version,
        }
    } else {
        UvStatus::TooOld {
            path: path.clone(),
            version,
            min: compat.uv.min.clone(),
        }
    }
}

/// A finished run.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ran {
    /// The exit code; none when it was ended (the time limit) or a signal ended it.
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    /// It ran out of time and was ended.
    pub timed_out: bool,
    /// It was stopped when asked ([`Watch::cancel`]).
    pub cancelled: bool,
    /// Its output was cut at [`MAX_OUTPUT`].
    pub cut: bool,
}

/// The environment variables that would decide for uv or Python instead of the caller.
pub(crate) fn steers(name: &OsStr) -> bool {
    let name = name.to_string_lossy();
    name.starts_with("UV_")
        || matches!(
            name.as_ref(),
            "VIRTUAL_ENV" | "PYTHONPATH" | "PYTHONHOME" | "CONDA_PREFIX" | "PYTHONSTARTUP"
        )
}

/// How long a stream is waited for once its program ended: a program uv started and that outlived it may hold the pipe
/// open, and what was read so far is enough.
const DRAINED: Duration = Duration::from_secs(2);

/// A stream read up to [`MAX_OUTPUT`] bytes, the rest read and dropped so the program never blocks on a full pipe; what
/// was kept comes over the channel when the stream ends.
fn drain(
    mut stream: impl Read + Send + 'static,
    live: Option<Arc<Mutex<String>>>,
) -> std::sync::mpsc::Receiver<(Vec<u8>, bool)> {
    let (sent, got) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut kept = Vec::new();
        let mut cut = false;
        let mut chunk = [0u8; 8192];
        loop {
            match stream.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let room = MAX_OUTPUT.saturating_sub(kept.len());
                    kept.extend_from_slice(&chunk[..n.min(room)]);
                    cut |= n > room;
                    // As it comes, for a caller that follows the run (a download's size, uv's phases).
                    if let Some(live) = &live
                        && let Ok(mut text) = live.lock()
                        && text.len() < MAX_OUTPUT
                    {
                        text.push_str(&String::from_utf8_lossy(&chunk[..n]));
                    }
                }
            }
        }
        let _ = sent.send((kept, cut));
    });
    got
}

/// How often [`Watch::tick`] is called while a program runs.
pub const TICK: Duration = Duration::from_millis(150);

/// How a run is followed while it lasts.
#[derive(Default)]
pub struct Watch<'a> {
    /// Set, the program is ended (the user's Durdur).
    pub cancel: Option<&'a AtomicBool>,
    /// Its error stream as it is written, up to [`MAX_OUTPUT`] bytes: uv says there what it downloads and how big.
    pub stderr: Option<Arc<Mutex<String>>>,
    /// Called about every [`TICK`] while it runs.
    pub tick: Option<&'a mut dyn FnMut()>,
}

/// Runs `program` with `args` in `dir` (none: as it is), `extra_env` added to an environment without the variables
/// that steer uv or Python; at most `limit` long, then ended.
pub fn run(
    program: &Path,
    args: &[&OsStr],
    dir: Option<&Path>,
    extra_env: &[(&str, &OsStr)],
    limit: Duration,
) -> Result<Ran, String> {
    run_watched(program, args, dir, extra_env, limit, Watch::default())
}

/// [`run`], followed while it lasts: ended as soon as `watch.cancel` is set, its error stream readable as it comes,
/// `watch.tick` called every [`TICK`].
pub fn run_watched(
    program: &Path,
    args: &[&OsStr],
    dir: Option<&Path>,
    extra_env: &[(&str, &OsStr)],
    limit: Duration,
    mut watch: Watch,
) -> Result<Ran, String> {
    let cancel = watch.cancel;
    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (name, _) in std::env::vars_os() {
        if steers(&name) {
            command.env_remove(name);
        }
    }
    for (name, value) in extra_env {
        command.env(name, value);
    }
    if let Some(dir) = dir {
        command.current_dir(dir);
    }
    // Started from a windowed program, a console program would flash a black window at every call.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let mut child = spawn(&mut command)?;
    let out = child.stdout.take().map(|s| drain(s, None));
    let err = child.stderr.take().map(|s| drain(s, watch.stderr.clone()));
    let mut ticked = Instant::now();
    let started = Instant::now();
    let mut timed_out = false;
    let mut cancelled = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) => {
                let _ = child.kill();
                let _ = child.wait();
                cancelled = true;
                break None;
            }
            Ok(None) if started.elapsed() >= limit => {
                let _ = child.kill();
                let _ = child.wait();
                timed_out = true;
                break None;
            }
            Ok(None) => {
                if let Some(tick) = watch.tick.as_mut()
                    && ticked.elapsed() >= TICK
                {
                    ticked = Instant::now();
                    tick();
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(e.to_string()),
        }
    };
    let take = |stream: Option<std::sync::mpsc::Receiver<(Vec<u8>, bool)>>| {
        stream
            .and_then(|s| s.recv_timeout(DRAINED).ok())
            .unwrap_or_default()
    };
    let (stdout, cut_out) = take(out);
    let (stderr, cut_err) = take(err);
    Ok(Ran {
        status,
        stdout: String::from_utf8_lossy(&stdout).into_owned(),
        stderr: String::from_utf8_lossy(&stderr).into_owned(),
        timed_out,
        cancelled,
        cut: cut_out || cut_err,
    })
}

/// How many times a busy program is tried, [`BUSY_WAIT`] apart.
const BUSY_TRIES: u32 = 8;
const BUSY_WAIT: Duration = Duration::from_millis(25);

/// Starts `command`, tried again briefly while its program is "busy" (`ETXTBSY`, os error 26 on Linux and macOS): a
/// program written a moment ago is still open for writing in a process another thread has just started, whose inherited
/// descriptor closes only at its own exec. Measured on this crate's tests, which write their fake uv and start them at
/// once from eight threads: 8 failures in 25 runs before, none after.
fn spawn(command: &mut Command) -> Result<Child, String> {
    let mut tries = 1;
    loop {
        match command.spawn() {
            Err(e) if cfg!(unix) && e.raw_os_error() == Some(26) && tries < BUSY_TRIES => {
                tries += 1;
                std::thread::sleep(BUSY_WAIT);
            }
            started => return started.map_err(|e| e.to_string()),
        }
    }
}

/// `args` as the operating system's strings, for [`run`].
pub fn os_args(args: &[OsString]) -> Vec<&OsStr> {
    args.iter().map(OsString::as_os_str).collect()
}
