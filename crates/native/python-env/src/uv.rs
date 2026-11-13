//! uv found and run (PY-UV-01, PY-UV-02, PY-UV-08, PY-UV-22, PY-UV-24; PY-OPT-03).
//!
//! - **Found:** the path the user chose, alone when there is one; else `PATH`, then the folders uv's installers use that
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
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::compat::{self, Compat};

/// Bytes kept of a stream's output.
pub const MAX_OUTPUT: usize = 256 * 1024;

/// How long `uv --version` may take.
const VERSION_LIMIT: Duration = Duration::from_secs(10);

/// What looking for uv found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UvStatus {
    /// Found and new enough.
    Found { path: PathBuf, version: String },
    /// Found, older than the manifest asks.
    TooOld {
        path: PathBuf,
        version: String,
        min: String,
    },
    /// A file is there but does not run, or does not say its version.
    NotRunnable { path: PathBuf, why: String },
    /// Not where it was looked for.
    NotFound { looked: Vec<PathBuf> },
    /// This computer's platform is not one the manifest lists.
    Unsupported { platform: String },
}

impl UvStatus {
    /// Ready to use.
    pub fn ok(&self) -> bool {
        matches!(self, UvStatus::Found { .. })
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
            UvStatus::Found { path, version } => {
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
            UvStatus::NotFound { looked } => format!(
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
    path_var: Option<&OsStr>,
    home: Option<&Path>,
    local_app_data: Option<&Path>,
) -> Vec<PathBuf> {
    candidates_for(std::env::consts::OS, chosen, path_var, home, local_app_data)
}

/// Where uv is looked for on a system (`linux`, `windows`, `macos`), in order: `chosen` alone when given; else every
/// folder of `path_var`, then the user folders uv's installers use. `home` is `%USERPROFILE%` on Windows.
pub fn candidates_for(
    os: &str,
    chosen: Option<&Path>,
    path_var: Option<&OsStr>,
    home: Option<&Path>,
    local_app_data: Option<&Path>,
) -> Vec<PathBuf> {
    if let Some(chosen) = chosen {
        return vec![chosen.to_path_buf()];
    }
    let name = exe(os);
    let mut out: Vec<PathBuf> = path_var
        .map(|p| std::env::split_paths(p).map(|dir| dir.join(name)).collect())
        .unwrap_or_default();
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

/// Looks for uv on this computer: the chosen path, `PATH`, the user folders.
pub fn detect(chosen: Option<&Path>) -> UvStatus {
    let path_var = std::env::var_os("PATH");
    let home =
        std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    let local = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    detect_in(
        compat::compat(),
        &compat::this_platform(),
        &candidates(
            chosen,
            path_var.as_deref(),
            home.as_deref(),
            local.as_deref(),
        ),
    )
}

/// The first of `looked` that is a file, asked its version and checked against `compat`.
pub fn detect_in(compat: &Compat, platform: &compat::Platform, looked: &[PathBuf]) -> UvStatus {
    if !compat.supports(platform) {
        return UvStatus::Unsupported {
            platform: format!("{} {}", platform.os, platform.arch),
        };
    }
    let Some(path) = looked.iter().find(|p| p.is_file()) else {
        return UvStatus::NotFound {
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
fn drain(mut stream: impl Read + Send + 'static) -> std::sync::mpsc::Receiver<(Vec<u8>, bool)> {
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
                }
            }
        }
        let _ = sent.send((kept, cut));
    });
    got
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
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let out = child.stdout.take().map(drain);
    let err = child.stderr.take().map(drain);
    let started = Instant::now();
    let mut timed_out = false;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) if started.elapsed() >= limit => {
                let _ = child.kill();
                let _ = child.wait();
                timed_out = true;
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
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
        cut: cut_out || cut_err,
    })
}

/// `args` as the operating system's strings, for [`run`].
pub fn os_args(args: &[OsString]) -> Vec<&OsStr> {
    args.iter().map(OsString::as_os_str).collect()
}
