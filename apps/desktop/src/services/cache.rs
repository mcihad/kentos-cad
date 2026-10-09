//! Map services' answers kept on this device (docs/adr/0208 §7): tiles,
//! capabilities, styles, sprites, by HTTP's caching rules
//! (`kentos_services::cache`): what a server says not to keep is not kept;
//! what is fresh is used without asking; what is stale is asked for again
//! with its validators (`If-None-Match`, `If-Modified-Since`), and while the
//! network is gone a stale tile still shows unless the server said it must
//! not. The files are under `$XDG_CACHE_HOME/kentos-cad/servis/` (else
//! `~/.cache/…`), each answer's bytes beside a small JSON of what it was
//! (its address without its secrets, when it came, how long it stays
//! fresh); a key is the address's hash, the address in the JSON tells a
//! collision. The least recently read go first when the folder outgrows its
//! budget.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use kentos_services::cache::{Policy, policy};
use serde::{Deserialize, Serialize};

/// Bytes the folder may take before the least recently read answers go.
pub const BUDGET: u64 = 2 * 1024 * 1024 * 1024;

/// What an answer was.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    /// The address without its secrets: the key's own text.
    pub url: String,
    /// When it arrived (or was last said to be the same), ms since the epoch.
    pub stored_ms: u64,
    pub fresh_ms: u64,
    pub must_revalidate: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    pub status: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
}

impl Meta {
    /// Whether it is fresh at `now_ms`.
    pub fn fresh(&self, now_ms: u64) -> bool {
        now_ms < self.stored_ms.saturating_add(self.fresh_ms)
    }

    /// The headers that ask whether it is still the one.
    pub fn validators(&self) -> Vec<(String, String)> {
        kentos_services::cache::revalidation(&Policy {
            store: true,
            fresh_ms: self.fresh_ms,
            must_revalidate: self.must_revalidate,
            etag: self.etag.clone(),
            last_modified: self.last_modified.clone(),
        })
    }
}

/// A kept answer.
#[derive(Clone, Debug)]
pub struct Kept {
    pub meta: Meta,
    pub body: Vec<u8>,
}

/// Now, ms since the epoch.
pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

/// The folder: `$XDG_CACHE_HOME/kentos-cad/servis`, else `~/.cache/kentos-cad/servis`.
pub fn folder() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("kentos-cad").join("servis"))
}

/// FNV-1a, 64 bits: a key stable across runs and builds.
fn hash(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.as_bytes() {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// An answer's two files under `root`.
fn paths(root: &Path, url: &str) -> (PathBuf, PathBuf) {
    let key = format!("{:016x}", hash(url));
    let dir = root.join(&key[..2]);
    (
        dir.join(format!("{key}.bin")),
        dir.join(format!("{key}.json")),
    )
}

/// The cache in a folder.
#[derive(Clone, Debug)]
pub struct Cache {
    root: Option<PathBuf>,
}

impl Cache {
    /// The device's cache.
    pub fn device() -> Cache {
        Cache { root: folder() }
    }

    /// A cache in `root` (the tests').
    #[cfg(test)]
    pub fn at(root: PathBuf) -> Cache {
        Cache { root: Some(root) }
    }

    /// The answer kept for `url` (its address without secrets); read marks it used.
    pub fn get(&self, url: &str) -> Option<Kept> {
        let root = self.root.as_ref()?;
        let (bin, json) = paths(root, url);
        let meta: Meta = serde_json::from_slice(&std::fs::read(&json).ok()?).ok()?;
        if meta.url != url {
            return None;
        }
        let body = std::fs::read(&bin).ok()?;
        // Used now: the trim lets go of the least recently read first.
        if let Ok(f) = std::fs::File::options().write(true).open(&bin) {
            let _ = f.set_modified(SystemTime::now());
        }
        Some(Kept { meta, body })
    }

    /// Keeps an answer to `url` when its headers allow it; none written when they do not.
    pub fn put(
        &self,
        url: &str,
        status: u16,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Option<Meta> {
        let root = self.root.as_ref()?;
        let p = policy(headers);
        if !p.store {
            return None;
        }
        let media = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            .map(|(_, v)| v.clone());
        let meta = Meta {
            url: url.to_owned(),
            stored_ms: now_ms(),
            fresh_ms: p.fresh_ms,
            must_revalidate: p.must_revalidate,
            etag: p.etag,
            last_modified: p.last_modified,
            status,
            media,
        };
        let (bin, json) = paths(root, url);
        let dir = bin.parent()?;
        std::fs::create_dir_all(dir).ok()?;
        // The bytes first, then the description that names them.
        write_atomic(&bin, body).ok()?;
        write_atomic(&json, &serde_json::to_vec(&meta).ok()?).ok()?;
        Some(meta)
    }

    /// The server said a kept answer is still the one (304): fresh again by its new headers.
    pub fn renew(&self, kept: &Meta, headers: &[(String, String)]) -> Option<Meta> {
        let root = self.root.as_ref()?;
        let p = policy(headers);
        let meta = Meta {
            stored_ms: now_ms(),
            fresh_ms: p.fresh_ms,
            must_revalidate: p.must_revalidate,
            etag: p.etag.or_else(|| kept.etag.clone()),
            last_modified: p.last_modified.or_else(|| kept.last_modified.clone()),
            ..kept.clone()
        };
        let (_, json) = paths(root, &kept.url);
        write_atomic(&json, &serde_json::to_vec(&meta).ok()?).ok()?;
        Some(meta)
    }

    /// Forgets every answer whose address starts with `prefix` (a service's
    /// “Önbelleği temizle”); how many went.
    pub fn forget(&self, prefix: &str) -> usize {
        let Some(root) = &self.root else {
            return 0;
        };
        let mut gone = 0;
        for json in files(root, "json") {
            let Ok(bytes) = std::fs::read(&json) else {
                continue;
            };
            let Ok(meta) = serde_json::from_slice::<Meta>(&bytes) else {
                continue;
            };
            if meta.url.starts_with(prefix) {
                let _ = std::fs::remove_file(json.with_extension("bin"));
                let _ = std::fs::remove_file(&json);
                gone += 1;
            }
        }
        gone
    }

    /// Lets the least recently read answers go until the folder takes at
    /// most four fifths of `budget`; the bytes it takes after.
    pub fn trim(&self, budget: u64) -> u64 {
        let Some(root) = &self.root else {
            return 0;
        };
        let mut all: Vec<(SystemTime, u64, PathBuf)> = files(root, "bin")
            .into_iter()
            .filter_map(|p| {
                let m = std::fs::metadata(&p).ok()?;
                Some((m.modified().unwrap_or(UNIX_EPOCH), m.len(), p))
            })
            .collect();
        let mut total: u64 = all.iter().map(|(_, n, _)| n).sum();
        if total <= budget {
            return total;
        }
        all.sort_by_key(|(at, _, _)| *at);
        let goal = budget / 5 * 4;
        for (_, n, p) in all {
            if total <= goal {
                break;
            }
            let _ = std::fs::remove_file(p.with_extension("json"));
            if std::fs::remove_file(&p).is_ok() {
                total = total.saturating_sub(n);
            }
        }
        total
    }
}

/// Every file with `extension` two levels under `root`.
fn files(root: &Path, extension: &str) -> Vec<PathBuf> {
    let Ok(dirs) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for d in dirs.filter_map(Result::ok) {
        let Ok(inner) = std::fs::read_dir(d.path()) else {
            continue;
        };
        out.extend(
            inner
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == extension)),
        );
    }
    out
}

/// Writes through a file beside the target, so a reader sees the old or the new, whole.
fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let temp = path.with_extension(format!(
        "{}.{}",
        path.extension().and_then(|x| x.to_str()).unwrap_or(""),
        std::process::id()
    ));
    std::fs::write(&temp, bytes)?;
    std::fs::rename(&temp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    #[test]
    fn kept_by_the_rules_renewed_forgotten_and_trimmed() {
        let dir = std::env::temp_dir().join(format!("kentos-servis-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let c = Cache::at(dir.clone());
        let url = "https://tile.openstreetmap.org/15/19458/12412.png";
        let m = c
            .put(
                url,
                200,
                &headers(&[
                    ("Cache-Control", "max-age=604800"),
                    ("ETag", "\"a1\""),
                    ("Content-Type", "image/png"),
                ]),
                b"png",
            )
            .expect("kept");
        assert!(m.fresh(now_ms()));
        assert_eq!(m.media.as_deref(), Some("image/png"));
        let k = c.get(url).expect("read");
        assert_eq!(k.body, b"png");
        assert_eq!(k.meta.validators(), headers(&[("If-None-Match", "\"a1\"")]));
        // Not kept when the server says so.
        assert!(
            c.put(
                "https://x/no",
                200,
                &headers(&[("Cache-Control", "no-store")]),
                b"x"
            )
            .is_none()
        );
        assert!(c.get("https://x/no").is_none());
        // A stale one renewed by a 304 keeps its validator.
        let stale = c
            .put(
                "https://x/stale",
                200,
                &headers(&[("Cache-Control", "no-cache"), ("ETag", "\"b\"")]),
                b"s",
            )
            .unwrap();
        assert!(!stale.fresh(now_ms() + 1));
        let renewed = c
            .renew(&stale, &headers(&[("Cache-Control", "max-age=60")]))
            .unwrap();
        assert!(renewed.fresh(now_ms()));
        assert_eq!(renewed.etag.as_deref(), Some("\"b\""));
        // A service's own answers forgotten by their address.
        assert_eq!(c.forget("https://x/"), 1);
        assert!(c.get("https://x/stale").is_none());
        assert!(c.get(url).is_some());
        // Trimmed to four fifths of the budget, the least recently read first.
        for i in 0..10 {
            c.put(
                &format!("https://t/{i}"),
                200,
                &headers(&[("Cache-Control", "max-age=60")]),
                &[0u8; 1000],
            )
            .unwrap();
        }
        let left = c.trim(5000);
        assert!(left <= 4000, "{left}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
