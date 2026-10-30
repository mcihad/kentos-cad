//! Where the launcher keeps what it keeps (the XDG Base Directory
//! specification, as KentOS's desktop program does: docs/adr/0050, 0800), and
//! the one way it writes a file: whole, then renamed into place, so that a
//! crash leaves the old file or the new one, never half of either.

use std::io;
use std::path::{Path, PathBuf};

/// The user's home: `HOME`, else `USERPROFILE` (Windows).
pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

/// An XDG base folder: the variable when set (and absolute, as the
/// specification asks), else `fallback` under the home.
fn xdg(variable: &str, fallback: &str) -> Option<PathBuf> {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| home().map(|h| h.join(fallback)))
}

/// `$XDG_DATA_HOME`, else `~/.local/share`.
pub fn data_home() -> Option<PathBuf> {
    xdg("XDG_DATA_HOME", ".local/share")
}

/// `$XDG_CONFIG_HOME`, else `~/.config`.
pub fn config_home() -> Option<PathBuf> {
    xdg("XDG_CONFIG_HOME", ".config")
}

/// `$XDG_STATE_HOME`, else `~/.local/state`.
pub fn state_home() -> Option<PathBuf> {
    xdg("XDG_STATE_HOME", ".local/state")
}

/// The launcher's own settings and program places: `…/kentos-launcher`.
pub fn launcher_config() -> Option<PathBuf> {
    config_home().map(|c| c.join("kentos-launcher"))
}

/// Writes `text` whole next to `path`, then renames it into place; the
/// folder is made when missing.
pub fn write_whole(path: &Path, text: &str) -> io::Result<()> {
    let folder = path
        .parent()
        .ok_or_else(|| io::Error::other("dosyanın klasörü yok"))?;
    std::fs::create_dir_all(folder)?;
    let name = path
        .file_name()
        .ok_or_else(|| io::Error::other("dosyanın adı yok"))?;
    let mut temporary = name.to_os_string();
    temporary.push(".yaziliyor");
    let temporary = folder.join(temporary);
    std::fs::write(&temporary, text)?;
    std::fs::rename(&temporary, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temporary);
    })
}

#[cfg(test)]
pub(crate) fn scratch(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let dir = std::env::temp_dir().join(format!(
        "kentos-launcher-{name}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch folder");
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_written_whole_and_its_folder_made() {
        let dir = scratch("files");
        let path = dir.join("alt/klasor/ayarlar.json");
        write_whole(&path, "{}").expect("writes");
        assert_eq!(std::fs::read_to_string(&path).expect("reads"), "{}");
        write_whole(&path, "{\"a\": 1}").expect("writes over");
        assert_eq!(std::fs::read_to_string(&path).expect("reads"), "{\"a\": 1}");
        assert!(!dir.join("alt/klasor/ayarlar.json.yaziliyor").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
