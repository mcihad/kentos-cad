//! Device drafts on disk (docs/adr/0040), the desktop's counterpart of the
//! web's IndexedDB store (`kentos.cloud/drafts`): one JSON file per server,
//! account and project, in a folder the desktop chooses (its data folder).
//!
//! - **Durable when saved:** a draft is written to a temporary file, flushed
//!   to the disk, then renamed over the old one (and the folder flushed), so
//!   a crash leaves the old draft or the new one, never half of one.
//! - **Never lost:** a draft that cannot be read, or that belongs to another
//!   account, is not removed or overwritten: it is kept aside as it was
//!   (`<name>#unreadable-<ms>.json`, as the web keeps it) and the caller is
//!   told, before a new draft takes its place.
//! - It holds drawing data only: no session, password or address beyond the
//!   server's name in its folder (TODOS.md SYNC-13).

use std::fs;
use std::future::Future;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::failure::ApiFailure;
use crate::runtime::run;
use crate::sync::{DRAFT_VERSION, Draft};

/// Where the drafts of this device are kept.
#[derive(Clone, Debug)]
pub struct DraftStore {
    dir: PathBuf,
}

/// A draft's place: its server, account and project.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DraftKey {
    path: PathBuf,
    user: String,
}

/// What was found at a draft's place.
#[derive(Debug, PartialEq)]
pub enum Loaded {
    None,
    Found(Box<Draft>),
    /// It could not be read, or it is another account's: kept aside as it was, at `kept`.
    KeptAside {
        kept: PathBuf,
        reason: String,
    },
}

/// A name part safe in a file name: letters, digits, `.`, `-` and `_`; anything else `_`.
fn part(text: &str) -> String {
    let safe: String = text
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if safe.is_empty() || safe.chars().all(|c| c == '.') {
        "_".to_owned()
    } else {
        safe
    }
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis())
}

impl DraftStore {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// The place of one account's draft of one project on one server (its address as `Cloud::server` gives it).
    pub fn key(&self, server: &str, user: &str, tenant: Uuid, project: Uuid) -> DraftKey {
        let host = server
            .split_once("://")
            .map_or(server, |(_, rest)| rest)
            .trim_end_matches('/');
        DraftKey {
            path: self
                .dir
                .join(part(host))
                .join(part(user))
                .join(format!("{tenant}_{project}.json")),
            user: user.to_owned(),
        }
    }

    /// Writes the draft over the old one, durably (see the module comment).
    pub fn save(&self, key: &DraftKey, draft: &Draft) -> io::Result<()> {
        let folder = key
            .path
            .parent()
            .ok_or_else(|| io::Error::other("taslak klasörü yok"))?;
        fs::create_dir_all(folder)?;
        let bytes = serde_json::to_vec(draft).map_err(io::Error::other)?;
        let tmp = key
            .path
            .with_extension(format!("json.tmp-{}", std::process::id()));
        let written = (|| {
            let mut file = fs::File::create(&tmp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            fs::rename(&tmp, &key.path)?;
            // The rename itself survives a crash once the folder is flushed.
            fs::File::open(folder)?.sync_all()
        })();
        if written.is_err() {
            let _ = fs::remove_file(&tmp);
        }
        written
    }

    /// Removes the draft (everything in it reached the server).
    pub fn remove(&self, key: &DraftKey) -> io::Result<()> {
        match fs::remove_file(&key.path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }

    /// Reads the draft at `key`; one that cannot be read, or is another
    /// account's, is kept aside as it was and reported, never removed.
    pub fn load(&self, key: &DraftKey) -> io::Result<Loaded> {
        let bytes = match fs::read(&key.path) {
            Ok(b) => b,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Loaded::None),
            Err(e) => return Err(e),
        };
        let reason = match serde_json::from_slice::<Draft>(&bytes) {
            Ok(d) if d.version != DRAFT_VERSION => {
                format!(
                    "taslak biçimi {} bu sürümün okuduğu {DRAFT_VERSION} değil",
                    d.version
                )
            }
            Ok(d) if d.user_id != key.user => "taslak başka bir hesabın".to_owned(),
            Ok(d) => return Ok(Loaded::Found(Box::new(d))),
            Err(e) => format!("taslak okunamadı ({e})"),
        };
        let kept = aside(&key.path);
        fs::rename(&key.path, &kept)?;
        Ok(Loaded::KeptAside { kept, reason })
    }

    /// `save` off the async threads (a large draft takes a moment), from any executor.
    pub fn save_later(
        &self,
        key: DraftKey,
        draft: Draft,
    ) -> impl Future<Output = Result<(), ApiFailure>> + Send + 'static {
        let store = self.clone();
        run(async move {
            tokio::task::spawn_blocking(move || store.save(&key, &draft))
                .await
                .map_err(|e| ApiFailure::local(format!("Taslak yazılamadı: {e}")))?
                .map_err(|e| {
                    ApiFailure::local(format!(
                        "Değişiklikler bu cihaza yedeklenemedi ({e}); programı kapatmayın."
                    ))
                })
        })
    }
}

/// Where an unreadable draft is kept: beside it, with the time.
fn aside(path: &Path) -> PathBuf {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    path.with_file_name(format!("{stem}#unreadable-{}.json", now_ms()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn temp() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("kentos-drafts-{}", Uuid::now_v7()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn draft(user: &str) -> Draft {
        Draft {
            version: DRAFT_VERSION,
            user_id: user.into(),
            changes: BTreeMap::new(),
            meta: None,
            inflight: None,
            updated: 1,
        }
    }

    #[test]
    fn a_draft_is_kept_per_server_account_and_project() {
        let dir = temp();
        let store = DraftStore::new(&dir);
        let (t, p) = (Uuid::now_v7(), Uuid::now_v7());
        let key = store.key("https://kentos.kurum.gov.tr/cad/", "ayse-1", t, p);
        let other = store.key("http://127.0.0.1:8787", "ayse-1", t, p);
        assert_ne!(key, other);
        assert!(key.path.starts_with(dir.join("kentos.kurum.gov.tr_cad")));
        assert_eq!(store.load(&key).unwrap(), Loaded::None);
        store.save(&key, &draft("ayse-1")).unwrap();
        assert_eq!(
            store.load(&key).unwrap(),
            Loaded::Found(Box::new(draft("ayse-1")))
        );
        assert_eq!(store.load(&other).unwrap(), Loaded::None);
        // Nothing half-written is left beside it.
        let names: Vec<_> = fs::read_dir(key.path.parent().unwrap())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names.len(), 1);
        store.remove(&key).unwrap();
        store.remove(&key).unwrap();
        assert_eq!(store.load(&key).unwrap(), Loaded::None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn an_unreadable_or_foreign_draft_is_kept_aside_not_lost() {
        let dir = temp();
        let store = DraftStore::new(&dir);
        let key = store.key(
            "http://127.0.0.1:8787",
            "ayse-1",
            Uuid::now_v7(),
            Uuid::now_v7(),
        );
        fs::create_dir_all(key.path.parent().unwrap()).unwrap();
        fs::write(&key.path, b"{ bozuk").unwrap();
        let Loaded::KeptAside { kept, reason } = store.load(&key).unwrap() else {
            panic!("an unreadable draft was not kept aside");
        };
        assert!(reason.contains("okunamadı"), "{reason}");
        assert_eq!(fs::read(&kept).unwrap(), b"{ bozuk");
        assert!(kept.to_string_lossy().contains("#unreadable-"));
        // Another account's draft under this account's name: kept aside too.
        store.save(&key, &draft("baska")).unwrap();
        let Loaded::KeptAside { reason, .. } = store.load(&key).unwrap() else {
            panic!("another account's draft was put back");
        };
        assert!(reason.contains("başka bir hesabın"), "{reason}");
        assert_eq!(store.load(&key).unwrap(), Loaded::None);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn names_from_the_outside_stay_inside_the_folder() {
        assert_eq!(part("../../etc"), ".._.._etc");
        assert_eq!(part(".."), "_");
        assert_eq!(part(""), "_");
        assert_eq!(part("Ayşe"), "ay_e");
    }
}
