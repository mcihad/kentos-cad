//! A cloud's index made once (docs/adr/0207 §4): a LAS, LAZ or text cloud
//! read in one pass (a LAZ's chunks decompressed on several threads at once),
//! its records put in bins on the disk, the bins built on several threads
//! (the core's `index::build_bin`) and the COPC written to the device's cache,
//! `$XDG_CACHE_HOME/kentos-cad/pointcloud/<key>.copc.laz` (else `~/.cache/…`).
//! The key is the SHA-256 of the file's path, size and change time (an
//! address's own, its size and version; an embedded file's id). One pass per
//! key; its progress shows in the panel at the drawing's lower right, Durdur
//! stops it (the cloud's plan alone shows). The cache keeps at most 16 GB,
//! the indexes opened least recently let go first.

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use kentos_processing::cloud_files::{self, Reading};

use super::bytes::{Bytes, Origin};
use super::service::Member;

/// The most the cache keeps.
const CACHE_MOST: u64 = 16 * 1024 * 1024 * 1024;

/// A pass under way: its file's name, how far, its stop.
#[derive(Clone, Debug)]
pub struct Progress {
    pub key: String,
    pub name: String,
    pub done: f64,
    stop: Arc<AtomicBool>,
    members: Vec<Arc<Member>>,
}

fn passes() -> &'static Mutex<HashMap<String, Progress>> {
    static PASSES: OnceLock<Mutex<HashMap<String, Progress>>> = OnceLock::new();
    PASSES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The passes under way, for the panel.
pub fn progress() -> Vec<Progress> {
    let mut out: Vec<Progress> = passes()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .values()
        .cloned()
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Durdur: the pass of `key` stops.
pub fn stop(key: &str) {
    if let Some(p) = passes()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(key)
    {
        p.stop.store(true, Ordering::Relaxed);
    }
}

/// The cache folder a test gives.
#[cfg(test)]
pub(crate) static TEST_FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);

/// The device's folder of indexes.
pub fn folder() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(f) = TEST_FOLDER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
    {
        return Some(f);
    }
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join("kentos-cad").join("pointcloud"))
}

/// A file's index key: its path, size and change time hashed (an address: itself, its size and version).
pub fn key_of(m: &Member) -> Result<String, String> {
    let text = match &m.origin {
        Origin::File(p) => {
            let meta =
                std::fs::metadata(p).map_err(|e| format!("“{}” okunamadı: {e}.", p.display()))?;
            let changed = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_nanos());
            format!("file|{}|{}|{changed}", p.display(), meta.len())
        }
        Origin::Url(u) => {
            let r = super::bytes::Remote::open(u)?;
            format!("url|{u}|{}|{}", r.size, r.version.unwrap_or_default())
        }
        Origin::Asset(id) => format!("asset|{id}"),
    };
    Ok(kentos_sheet::template::sha256_hex(text.as_bytes()))
}

/// The index made before, if any (its time touched: the cache lets the oldest go).
pub fn cached(m: &Member) -> Result<Option<PathBuf>, String> {
    let Some(dir) = folder() else {
        return Err("Önbellek klasörü bulunamadı (HOME yok).".into());
    };
    let path = dir.join(format!("{}.copc.laz", key_of(m)?));
    if path.is_file() {
        if let Ok(f) = File::options().append(true).open(&path) {
            let _ = f.set_modified(std::time::SystemTime::now());
        }
        return Ok(Some(path));
    }
    Ok(None)
}

/// Starts the pass of `m`'s key, or waits on the one under way.
pub fn start(m: Arc<Member>) {
    let key = match key_of(&m) {
        Ok(k) => k,
        Err(e) => {
            super::service::indexed(&m, Err(e));
            return;
        }
    };
    let mut all = passes().lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(p) = all.get_mut(&key) {
        p.members.push(m);
        return;
    }
    let stop = Arc::new(AtomicBool::new(false));
    all.insert(
        key.clone(),
        Progress {
            key: key.clone(),
            name: m.name.clone(),
            done: 0.0,
            stop: stop.clone(),
            members: vec![m.clone()],
        },
    );
    drop(all);
    let _ = std::thread::Builder::new()
        .name("kentos-bulut-dizin".into())
        .spawn(move || {
            let result = run(&m, &key, &stop);
            let members = passes()
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&key)
                .map(|p| p.members)
                .unwrap_or_default();
            for member in members {
                super::service::indexed(&member, result.clone());
            }
        });
}

fn set_done(key: &str, done: f64) {
    let mut all = passes().lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(p) = all.get_mut(key) {
        p.done = done.clamp(0.0, 1.0);
        for m in &p.members {
            m.set(super::service::State::Indexing(p.done));
        }
    }
    drop(all);
    super::service::service().wake();
}

/// The bins' folder of a pass.
fn bins_dir(dir: &std::path::Path, key: &str) -> PathBuf {
    dir.join(format!("{key}.bins"))
}

/// Opens the source for reading.
fn reading(m: &Member) -> Result<Reading<'static>, String> {
    cloud_files::reading(Box::new(m.bytes()?), m.format)
}

fn run(m: &Member, key: &str, stop: &AtomicBool) -> Result<PathBuf, String> {
    let dir = folder().ok_or("Önbellek klasörü bulunamadı (HOME yok).")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("“{}” açılamadı: {e}.", dir.display()))?;
    let target = dir.join(format!("{key}.copc.laz"));
    let part = dir.join(format!("{key}.copc.laz.part"));
    let bins = bins_dir(&dir, key);
    let cleanup = || {
        let _ = std::fs::remove_dir_all(&bins);
        let _ = std::fs::remove_file(&part);
    };
    let reading = match reading(m) {
        Ok(r) => r,
        Err(e) => {
            cleanup();
            return Err(e);
        }
    };
    let mut tick = |done: f64| {
        set_done(key, done);
        stop.load(Ordering::Relaxed)
    };
    let result = cloud_files::build_index(reading, &mut tick, &bins, &part);
    match result {
        Ok(()) => {
            let _ = std::fs::remove_dir_all(&bins);
            std::fs::rename(&part, &target)
                .map_err(|e| format!("“{}” yazılamadı: {e}.", target.display()))?;
            trim(&dir, &target);
            Ok(target)
        }
        Err(e) => {
            cleanup();
            Err(e)
        }
    }
}

/// A COPC of the LAS or LAZ at `from` written at `to` (İşlemler's COPC
/// results, docs/adr/0207 §7): the shared index (docs/adr/0572), its bins
/// beside `to`; `tick` takes the share and says whether to stop.
pub fn copc_file(
    from: &std::path::Path,
    to: &std::path::Path,
    tick: &mut dyn FnMut(f64) -> bool,
) -> Result<(), String> {
    cloud_files::copc_file(Box::new(Bytes::file(from)?), to, tick)
}

/// Lets the indexes opened least recently go while the cache holds more than it may.
fn trim(dir: &std::path::Path, keep: &std::path::Path) {
    let Ok(read) = std::fs::read_dir(dir) else {
        return;
    };
    let mut files: Vec<(std::time::SystemTime, u64, PathBuf)> = read
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let meta = e.metadata().ok()?;
            (p.extension().is_some_and(|x| x == "laz") && meta.is_file()).then(|| {
                (
                    meta.modified().ok().unwrap_or(std::time::UNIX_EPOCH),
                    meta.len(),
                    p,
                )
            })
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    files.sort();
    for (_, size, p) in files {
        if total <= CACHE_MOST {
            break;
        }
        if p != keep && std::fs::remove_file(&p).is_ok() {
            total = total.saturating_sub(size);
        }
    }
}
