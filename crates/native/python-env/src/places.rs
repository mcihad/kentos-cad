//! Where the user's Python settings and environments are: the program's data folder's `python`
//! (`$XDG_DATA_HOME/kentos-cad/python`, else `~/.local/share/kentos-cad/python`; `~/Library/Application
//! Support/kentos-cad/python` on macOS). On Windows the **local** application data (`%LOCALAPPDATA%\kentos-cad\python`),
//! not the roaming one: environments run to gigabytes and are this computer's own, and an organisation's roaming
//! profiles would copy them at every sign-in. Per user: another account on the same computer has its own (PY-OPT-04).
//! `KENTOS_PYTHON_DIR` points elsewhere, so tests and real-window sessions never touch the user's own.

use std::path::{Path, PathBuf};

fn env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// KentOS's own Pythons, installed by uv when the user asks (`UV_PYTHON_INSTALL_DIR`). Short names: on Windows a
/// path is long enough already.
pub fn interpreters(root: &Path) -> PathBuf {
    root.join("yorumlayici")
}

/// uv's cache for KentOS's work (`UV_CACHE_DIR`), apart from the user's own uv cache.
pub fn cache(root: &Path) -> PathBuf {
    root.join("onbellek")
}

/// Where Python archives are downloaded to while installing (`UV_PYTHON_CACHE_DIR`), each install in a folder of its
/// own, removed when it ends: the archive is not kept (35 MB), and a download growing there is what the progress bar
/// measures.
pub fn downloads(root: &Path) -> PathBuf {
    root.join("indirme")
}

/// The personal environment's project (`pyproject.toml`, `uv.lock`, `.venv`): crate::personal.
pub fn personal(root: &Path) -> PathBuf {
    root.join("kisisel")
}

/// The user's Python folder; none when this computer gives no data folder.
pub fn root() -> Option<PathBuf> {
    if let Some(dir) = env("KENTOS_PYTHON_DIR") {
        return Some(dir);
    }
    let base = if cfg!(target_os = "windows") {
        env("LOCALAPPDATA").or_else(|| env("APPDATA"))
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library").join("Application Support"))
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local").join("share")))
    };
    base.map(|b| b.join("kentos-cad").join("python"))
}
