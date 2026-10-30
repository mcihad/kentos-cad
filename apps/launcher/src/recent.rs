//! Son dosyalar (docs/adr/0800 §5): the drawings KentOS's programs opened or
//! saved lately, as they keep them (`kentos_launch::recent`, read only here;
//! the programs write it). Each row says the module it was worked in when
//! the list knows it, where the file is and when; a file no longer there is
//! shown dimmed and cannot be opened.

use std::path::{Path, PathBuf};

use kentos_launch::Module;
use kentos_launch::recent::Recent;

/// A row of the page.
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub name: String,
    /// The folder it is in, as the row writes it (the home as `~`).
    pub folder: String,
    pub path: PathBuf,
    /// “az önce”, “5 dakika önce” … from `now`.
    pub ago: String,
    /// What the program wrote about it: objects and coordinate system.
    pub info: String,
    pub module: Option<Module>,
    /// The file is no longer where the list says.
    pub missing: bool,
}

/// The rows of `list`, newest first as the list is, `now` in milliseconds
/// since the epoch; `exists` says whether a file is still there.
pub fn rows(
    list: &[Recent],
    now: u64,
    exists: impl Fn(&Path) -> bool,
    home: Option<&Path>,
) -> Vec<Row> {
    list.iter()
        .map(|r| Row {
            name: r.name.clone(),
            folder: r
                .path
                .parent()
                .map(|p| shown_folder(p, home))
                .unwrap_or_default(),
            path: r.path.clone(),
            ago: ago(r.at, now),
            info: r.info.clone(),
            module: r.module,
            missing: !exists(&r.path),
        })
        .collect()
}

/// A folder as a row writes it: under the home with `~`.
fn shown_folder(folder: &Path, home: Option<&Path>) -> String {
    match home.and_then(|h| folder.strip_prefix(h).ok()) {
        Some(rest) if rest.as_os_str().is_empty() => "~".to_owned(),
        Some(rest) => format!("~/{}", rest.display()),
        None => folder.display().to_string(),
    }
}

/// A time said from `now` (milliseconds since the epoch): “az önce”,
/// “5 dakika önce”, “3 saat önce”, “dün”, “4 gün önce”, “2 ay önce”,
/// “1 yıl önce”. Days, not dates: the launcher does not guess the time zone.
pub fn ago(at: u64, now: u64) -> String {
    let seconds = now.saturating_sub(at) / 1000;
    match seconds {
        0..60 => "az önce".to_owned(),
        60..3_600 => format!("{} dakika önce", seconds / 60),
        3_600..86_400 => format!("{} saat önce", seconds / 3_600),
        86_400..172_800 => "dün".to_owned(),
        172_800..2_592_000 => format!("{} gün önce", seconds / 86_400),
        2_592_000..31_536_000 => format!("{} ay önce", seconds / 2_592_000),
        _ => format!("{} yıl önce", seconds / 31_536_000),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: u64 = 1_793_000_000_000;

    fn recent(path: &str, minutes_ago: u64, module: Option<Module>) -> Recent {
        Recent {
            path: PathBuf::from(path),
            name: Path::new(path)
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            at: NOW - minutes_ago * 60_000,
            info: "334 nesne · Yerel".to_owned(),
            module,
        }
    }

    #[test]
    fn rows_say_where_when_and_whether_the_file_is_there() {
        let list = [
            recent(
                "/ev/cihad/projeler/konut.kcad",
                0,
                Some(Module::Architecture),
            ),
            recent("/ev/cihad/imar.kcad", 90, Some(Module::Map)),
            recent("/mnt/arsiv/eski.dxf", 60 * 24 * 40, None),
        ];
        let rows = rows(
            &list,
            NOW,
            |p| !p.starts_with("/mnt"),
            Some(Path::new("/ev/cihad")),
        );
        assert_eq!(rows.len(), 3);
        assert_eq!(
            (
                rows[0].folder.as_str(),
                rows[0].ago.as_str(),
                rows[0].missing
            ),
            ("~/projeler", "az önce", false)
        );
        assert_eq!(
            (rows[1].folder.as_str(), rows[1].ago.as_str()),
            ("~", "1 saat önce")
        );
        assert_eq!(rows[1].module, Some(Module::Map));
        assert_eq!(
            (
                rows[2].folder.as_str(),
                rows[2].ago.as_str(),
                rows[2].missing
            ),
            ("/mnt/arsiv", "1 ay önce", true)
        );
        assert_eq!(rows[2].module, None);
    }

    #[test]
    fn times_are_said_from_now() {
        let at = |seconds: u64| ago(NOW - seconds * 1000, NOW);
        assert_eq!(at(59), "az önce");
        assert_eq!(at(5 * 60), "5 dakika önce");
        assert_eq!(at(3 * 3600 + 59), "3 saat önce");
        assert_eq!(at(86_400 + 10), "dün");
        assert_eq!(at(4 * 86_400), "4 gün önce");
        assert_eq!(at(65 * 86_400), "2 ay önce");
        assert_eq!(at(800 * 86_400), "2 yıl önce");
        assert_eq!(
            ago(NOW + 5_000, NOW),
            "az önce",
            "a clock that ran back is now"
        );
    }
}
