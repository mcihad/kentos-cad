//! What the exchange files (IFC, LandXML) share across modules (docs/adr/8054 §11): a file written whole and read
//! back before it takes its name, a UTC stamp and its reading, and a command's refusal with its path. Road design
//! and architecture both write such files; the core keeps these so neither needs the other.

use std::path::{Path, PathBuf};

use crate::checks::{Stop, error};

/// The text written at `path` whole or not at all: beside it, read back whole, then moved over it.
pub(crate) fn write_file(path: &Path, text: &str) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".kentos-yaziliyor");
    let tmp = PathBuf::from(tmp);
    let done = (|| -> Result<(), String> {
        std::fs::write(&tmp, text.as_bytes()).map_err(|e| format!("{} yazılamadı: {e}", tmp.display()))?;
        let back = std::fs::read_to_string(&tmp).map_err(|e| format!("{} geri okunamadı: {e}", tmp.display()))?;
        if back != text {
            return Err("yazılan dosya geri okununca aynı çıkmadı".into());
        }
        std::fs::rename(&tmp, path).map_err(|e| format!("{} yerine konamadı: {e}", path.display()))
    })();
    if done.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    done
}

/// The `$`-less stamp, `YYYY-MM-DDThh:mm:ss`, as the file's date and time; none where it does not read.
pub(crate) fn stamp_of(text: &str) -> Option<(String, String)> {
    let (d, t) = text.split_once('T')?;
    let ok_date = d.len() == 10 && d.as_bytes()[4] == b'-' && d.as_bytes()[7] == b'-';
    let ok_time = t.len() == 8 && t.as_bytes()[2] == b':' && t.as_bytes()[5] == b':';
    let digits = |s: &str| s.chars().filter(|c| *c != '-' && *c != ':').all(|c| c.is_ascii_digit());
    (ok_date && ok_time && digits(d) && digits(t)).then(|| (d.to_owned(), t.to_owned()))
}

/// Now, UTC, as a stamp.
pub(crate) fn now() -> (String, String) {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64;
    let (y, m, d) = kentos_geometry_core::time::civil_from_days(secs.div_euclid(86_400));
    let s = secs.rem_euclid(86_400);
    (format!("{y:04}-{m:02}-{d:02}"), format!("{:02}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60))
}

pub(crate) fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, (!path.is_empty()).then(|| path.to_owned())))
}
