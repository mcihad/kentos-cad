//! The connections' secrets on this device (docs/adr/0208 §12): an API key,
//! a user's password, a token, a client's secret. They are never written
//! into a drawing (the project names a connection, its origin and how it
//! proves itself); they live in `$XDG_CONFIG_HOME/kentos-cad/baglantilar.json`
//! (else `~/.config/kentos-cad/`), readable by the user alone (0600), each
//! under its connection's origin and id, so two projects naming the same
//! connection use the one secret. A file that does not read is set aside as
//! `baglantilar-okunamadi-<time>.json`, never written over. The web keeps
//! the same in its own storage (`kentos.connections.v1`).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock, PoisonError};

use kentos_contracts::ConnectionSecret;
use serde::{Deserialize, Serialize};

const FILE: &str = "baglantilar.json";
const FORMAT: &str = "kentos.connection-secrets";

/// One secret under its connection's origin.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub origin: String,
    pub secret: ConnectionSecret,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct SecretsFile {
    format: String,
    version: u32,
    entries: Vec<Entry>,
}

/// The device's secrets, read once and written on each change.
pub struct Secrets {
    folder: Option<PathBuf>,
    entries: Mutex<Vec<Entry>>,
    /// Bumped on each change: a service that failed for want of a secret tries again.
    version: AtomicU64,
}

fn folder() -> Option<PathBuf> {
    // The tests' app keeps its secrets in memory, as its settings: never the user's file.
    if cfg!(test) {
        return None;
    }
    crate::settings::Settings::config_dir()
}

/// The app's secrets.
pub fn secrets() -> &'static Secrets {
    static SECRETS: OnceLock<Secrets> = OnceLock::new();
    SECRETS.get_or_init(|| Secrets::open(folder()))
}

impl Secrets {
    /// The secrets kept in `folder` (none: in memory only).
    pub fn open(folder: Option<PathBuf>) -> Secrets {
        let entries = folder.as_deref().map(read).unwrap_or_default();
        Secrets {
            folder,
            entries: Mutex::new(entries),
            version: AtomicU64::new(0),
        }
    }

    /// The secret of connection `id` at `origin`.
    pub fn get(&self, origin: &str, id: &str) -> Option<ConnectionSecret> {
        self.entries
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .iter()
            .find(|e| e.origin == origin && e.secret.id == id)
            .map(|e| e.secret.clone())
    }

    /// Keeps `secret` for its connection at `origin`, replacing the one there was.
    pub fn put(&self, origin: &str, secret: ConnectionSecret) -> Result<(), String> {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        entries.retain(|e| !(e.origin == origin && e.secret.id == secret.id));
        entries.push(Entry {
            origin: origin.to_owned(),
            secret,
        });
        self.version.fetch_add(1, Ordering::Relaxed);
        self.write(&entries)
    }

    /// Forgets the secret of connection `id` at `origin`.
    pub fn remove(&self, origin: &str, id: &str) -> Result<(), String> {
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        let before = entries.len();
        entries.retain(|e| !(e.origin == origin && e.secret.id == id));
        if entries.len() == before {
            return Ok(());
        }
        self.version.fetch_add(1, Ordering::Relaxed);
        self.write(&entries)
    }

    /// Changes so far.
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::Relaxed)
    }

    fn write(&self, entries: &[Entry]) -> Result<(), String> {
        let Some(folder) = &self.folder else {
            return Ok(());
        };
        let file = SecretsFile {
            format: FORMAT.into(),
            version: 1,
            entries: entries.to_vec(),
        };
        let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
        write_private(&folder.join(FILE), text.as_bytes()).map_err(|e| {
            format!(
                "Bağlantı bilgileri kaydedilemedi ({}): {e}",
                folder.display()
            )
        })
    }
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

/// The entries of the file in `folder`; one that does not read is set aside.
fn read(folder: &Path) -> Vec<Entry> {
    let path = folder.join(FILE);
    let Ok(bytes) = std::fs::read(&path) else {
        return Vec::new();
    };
    match serde_json::from_slice::<SecretsFile>(&bytes) {
        Ok(f) if f.format == FORMAT && f.version == 1 => f.entries,
        _ => {
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let _ = std::fs::rename(
                &path,
                folder.join(format!("baglantilar-okunamadi-{stamp}.json")),
            );
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn secret(id: &str, value: &str) -> ConnectionSecret {
        ConnectionSecret {
            id: id.into(),
            values: vec![value.into()],
            user: None,
            password: None,
            token: None,
            client_id: None,
            client_secret: None,
        }
    }

    #[test]
    fn kept_by_origin_and_id_readable_by_the_user_alone() {
        let dir = std::env::temp_dir().join(format!("kentos-secrets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let s = Secrets::open(Some(dir.clone()));
        s.put("https://atlas.harita.gov.tr", secret("hgm", "anahtar-1"))
            .unwrap();
        s.put("https://atlas.harita.gov.tr", secret("hgm", "anahtar-2"))
            .unwrap();
        s.put("https://other.example.com", secret("hgm", "baska"))
            .unwrap();
        assert_eq!(
            s.get("https://atlas.harita.gov.tr", "hgm").unwrap().values,
            vec!["anahtar-2"]
        );
        assert_eq!(s.version(), 3);
        // Read again from the file.
        let again = Secrets::open(Some(dir.clone()));
        assert_eq!(
            again
                .get("https://other.example.com", "hgm")
                .unwrap()
                .values,
            vec!["baska"]
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join(FILE))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        again.remove("https://other.example.com", "hgm").unwrap();
        assert!(
            Secrets::open(Some(dir.clone()))
                .get("https://other.example.com", "hgm")
                .is_none()
        );
        // A file that does not read is set aside, not written over.
        std::fs::write(dir.join(FILE), b"{bozuk").unwrap();
        let fresh = Secrets::open(Some(dir.clone()));
        assert!(fresh.get("https://atlas.harita.gov.tr", "hgm").is_none());
        let aside = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with("baglantilar-okunamadi-")
            });
        assert!(aside);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
