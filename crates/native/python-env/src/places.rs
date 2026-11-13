//! Where the user's Python settings and environments are: the program's data folder's `python`
//! (`$XDG_DATA_HOME/kentos-cad/python`, else `~/.local/share/kentos-cad/python`; `~/Library/Application
//! Support/kentos-cad/python` on macOS). On Windows the **local** application data (`%LOCALAPPDATA%\kentos-cad\python`),
//! not the roaming one: environments run to gigabytes and are this computer's own, and an organisation's roaming
//! profiles would copy them at every sign-in. Per user: another account on the same computer has its own (PY-OPT-04).
//! `KENTOS_PYTHON_DIR` points elsewhere, so tests and real-window sessions never touch the user's own.

use std::path::PathBuf;

fn env(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
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
