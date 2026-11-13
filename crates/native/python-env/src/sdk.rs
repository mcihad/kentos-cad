//! KentOS's Python library as the installation carries it (PY-UV-31): the `kentos` package, whose native module is built
//! from `crates/native/python` by maturin, as a wheel beside the program, with a description the packaging step writes
//! (`scripts/desktop/bundle_sdk.py`, [`DESCRIPTION`]): its file, version and SHA-256. It goes into the user's
//! environments from that file alone, offline and without dependencies (it has none); never from an index, so a
//! package named `kentos` from elsewhere cannot stand in for it.

use std::io::Read;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

/// The description's name, beside the wheel.
pub const DESCRIPTION: &str = "kentos-sdk.json";

const FORMAT: &str = "kentos.sdk-wheel";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Described {
    format: String,
    version: String,
    file: String,
    sha256: String,
}

/// Where this program's installation keeps the wheel ([`dirs_for`]); none when the program's path is not known.
pub fn dirs() -> Vec<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .map(|dir| dirs_for(std::env::consts::OS, &dir))
        .unwrap_or_default()
}

/// Where an installation keeps the wheel, for a program in `dir` on a system: `tools` beside it; in a macOS app
/// bundle also `Contents/Resources/python` (a wheel is a file the program reads, not code the bundle runs).
pub fn dirs_for(os: &str, dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![dir.join("tools")];
    if os == "macos"
        && dir.ends_with("Contents/MacOS")
        && let Some(contents) = dir.parent()
    {
        out.push(contents.join("Resources").join("python"));
    }
    out
}

/// The library's wheel, found and checked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wheel {
    pub path: PathBuf,
    pub version: String,
    /// Its SHA-256: an environment records which build it holds.
    pub sha256: String,
}

/// The installation's wheel for this program (`version`, its own): in the first of `dirs` with a description; the
/// file must be there, a plain name beside it, of that version and with that SHA-256.
pub fn find(dirs: &[PathBuf], version: &str) -> Result<Wheel, String> {
    let Some((dir, text)) = dirs.iter().find_map(|dir| {
        std::fs::read_to_string(dir.join(DESCRIPTION))
            .ok()
            .map(|text| (dir, text))
    }) else {
        return Err(
            "KentOS'un Python kitaplığı (kentos) bu kurulumda yok; KentOS'u yeniden kurun."
                .to_owned(),
        );
    };
    let described: Described =
        serde_json::from_str(&text).map_err(|e| format!("{DESCRIPTION} okunamadı: {e}"))?;
    if described.format != FORMAT {
        return Err(format!(
            "{DESCRIPTION} “{}” biçiminde; bu KentOS “{FORMAT}” okur.",
            described.format
        ));
    }
    if described.version != version {
        return Err(format!(
            "Kurulumdaki Python kitaplığı {} sürümünde, bu KentOS {version}; KentOS'u yeniden kurun.",
            described.version
        ));
    }
    if described.file.contains(['/', '\\']) || !described.file.ends_with(".whl") {
        return Err(format!(
            "{DESCRIPTION} bir tekerlek dosyası adı vermiyor: “{}”.",
            described.file
        ));
    }
    let path = dir.join(&described.file);
    let got = sha256_file(&path).map_err(|e| format!("{} okunamadı: {e}", path.display()))?;
    if !got.eq_ignore_ascii_case(&described.sha256) {
        return Err(format!(
            "{} bozuk (SHA-256 tutmuyor); KentOS'u yeniden kurun.",
            path.display()
        ));
    }
    Ok(Wheel {
        path,
        version: described.version,
        sha256: got.to_lowercase(),
    })
}

/// A file's SHA-256, lower-case hex.
pub fn sha256_file(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut digest = Sha256::new();
    let mut chunk = vec![0u8; 1 << 16];
    loop {
        let n = file.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        digest.update(&chunk[..n]);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}
