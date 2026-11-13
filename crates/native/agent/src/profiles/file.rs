//! The agents' file (docs/adr/8056 §4): `yapay-zeka-ajanlari.json` beside the desktop's settings, readable by the user
//! alone (0600) and written whole through a file beside it, as the device's database connections are. The kept keys sit
//! in it for the agents whose key is kept, and only for them. A file that does not read is set aside, never written
//! over; one that cannot be read at all (its permissions) stops every write.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::{AgentProfile, KeySource, Secret, State};

const FILE: &str = "yapay-zeka-ajanlari.json";
const FORMAT: &str = "kentos.ai-agents";
const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct StoreFile {
    format: String,
    version: u32,
    #[serde(default)]
    agents: Vec<AgentProfile>,
    /// The kept keys by agent id: only agents whose key is `Stored`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    keys: BTreeMap<String, String>,
}

/// The device's agents' file: `$XDG_CONFIG_HOME/kentos-cad/yapay-zeka-ajanlari.json`, else under `~/.config`.
pub fn device_file() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join("kentos-cad").join(FILE))
}

/// The state of the file at `path`: none there is an empty list; one that does not read is set aside.
pub(super) fn read(path: &Path) -> State {
    let bytes = match std::fs::read(path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return State::default(),
        Err(e) => {
            return State {
                unreadable: Some(e.to_string()),
                ..State::default()
            };
        }
    };
    match serde_json::from_slice::<StoreFile>(&bytes) {
        Ok(f) if f.format == FORMAT && f.version == VERSION => {
            let mut s = State::default();
            for p in f.agents {
                if !p.id.is_empty() && !s.profiles.iter().any(|q| q.id == p.id) {
                    s.profiles.push(p);
                }
            }
            // Exactly one default: the first marked one, else the first agent.
            let first = s.profiles.iter().position(|p| p.default).unwrap_or(0);
            for (i, p) in s.profiles.iter_mut().enumerate() {
                p.default = i == first;
            }
            for (id, key) in f.keys {
                let kept = s
                    .profiles
                    .iter()
                    .any(|p| p.id == id && p.key == KeySource::Stored);
                if kept && !key.trim().is_empty() {
                    s.stored.insert(id, Secret(key.trim().to_owned()));
                }
            }
            s
        }
        _ => {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("yapay-zeka-ajanlari");
            let aside = path.with_file_name(format!("{stem}-okunamadi-{stamp}.json"));
            let moved = std::fs::rename(path, &aside).is_ok();
            State {
                set_aside: moved.then_some(aside),
                // Not moved: it would be written over, so nothing is written.
                unreadable: (!moved).then(|| "dosya okunamadı ve kenara alınamadı".to_owned()),
                ..State::default()
            }
        }
    }
}

/// Writes the agents and their kept keys to `path`.
pub(super) fn write(path: &Path, s: &State) -> Result<(), String> {
    if let Some(why) = &s.unreadable {
        return Err(format!(
            "Yapay zekâ ajanları kaydedilmedi: {} okunamıyor ({why}). Dosyanın izinlerini denetleyin.",
            path.display()
        ));
    }
    let keys = s
        .profiles
        .iter()
        .filter(|p| p.key == KeySource::Stored)
        .filter_map(|p| s.stored.get(&p.id).map(|k| (p.id.clone(), k.0.clone())))
        .collect();
    let file = StoreFile {
        format: FORMAT.into(),
        version: VERSION,
        agents: s.profiles.clone(),
        keys,
    };
    let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    write_private(path, text.as_bytes()).map_err(|e| {
        format!(
            "Yapay zekâ ajanları kaydedilemedi ({}): {e}. Klasörün yazılabilir olduğunu denetleyin.",
            path.display()
        )
    })
}

/// Writes `bytes` to `path` readable by the user alone, through a file beside it.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let temp = path.with_extension("json.yeni");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    options.mode(0o600);
    let mut f = options.open(&temp)?;
    f.write_all(bytes)?;
    f.sync_all()?;
    drop(f);
    #[cfg(unix)]
    std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600))?;
    std::fs::rename(&temp, path)
}
