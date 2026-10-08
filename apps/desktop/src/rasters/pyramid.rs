//! A large raster's pyramid file (docs/adr/0204 §3): a raster over 4096
//! pixels on a side without overviews has its levels from the third on
//! made once, in one pass over level 0 (the formats core's `pyramid::Builder`),
//! and kept on the device as a tiled, Deflate GeoTIFF in
//! `$XDG_CACHE_HOME/kentos-cad/raster/<key>.tif` (else `~/.cache/…`); the key
//! is the SHA-256 of the file's path, size and change time. The pass runs on
//! a thread of its own, its progress for the bottom panel, Durdur stops it
//! (the raster's coarse levels are then made from level 0 as they are asked
//! for). A file made before is taken as it is.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use kentos_formats::raster::pyramid::Builder;
use kentos_formats::raster::{ByteStore, Step, TILE, tiff};

use super::tiles::{Opened, Origin, Service, Source, read_at};

/// A pass under way: its raster's name, how far it is, its stop.
#[derive(Clone, Debug)]
pub struct Progress {
    pub key: String,
    pub name: String,
    /// 0 to 1.
    pub done: f64,
    stop: Arc<AtomicBool>,
}

fn passes() -> &'static Mutex<HashMap<String, Progress>> {
    static PASSES: OnceLock<Mutex<HashMap<String, Progress>>> = OnceLock::new();
    PASSES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// The passes under way, for the bottom panel.
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

/// Durdur: the pass of `key` stops; its raster's coarse levels come from level 0.
pub fn stop(key: &str) {
    if let Some(p) = passes()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(key)
    {
        p.stop.store(true, Ordering::Relaxed);
    }
}

/// The pyramid files' folder a test gives.
#[cfg(test)]
pub(super) static TEST_FOLDER: Mutex<Option<PathBuf>> = Mutex::new(None);

/// The device's folder of pyramid files.
fn folder() -> Option<PathBuf> {
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
    Some(base.join("kentos-cad").join("raster"))
}

/// The pyramid file's name for a linked file: its path, size and change time hashed.
fn cache_path(path: &std::path::Path) -> Option<PathBuf> {
    let meta = std::fs::metadata(path).ok()?;
    let changed = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    let id = format!("{}|{}|{changed}", path.display(), meta.len());
    let hex = kentos_sheet::template::sha256_hex(id.as_bytes());
    Some(folder()?.join(format!("{hex}.tif")))
}

/// A pyramid file read and taken by the raster's reader; why not.
fn attach(opened: &Opened, path: &std::path::Path) -> Result<(), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let size = file.metadata().map_err(|e| e.to_string())?.len();
    let mut store = ByteStore::new();
    let t = loop {
        match tiff::parse(&store, size).map_err(|e| e.0)? {
            Step::Done(t) => break t,
            Step::Need(n) => store.put(n.offset, read_at(&file, n.offset, n.len)?),
        }
    };
    opened
        .reader
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .attach_pyramid(&t)
        .map_err(|e| e.0)?;
    *opened
        .pyramid
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = Some(file);
    Ok(())
}

/// Starts the raster's pyramid unless it is under way, made or stopped:
/// a file made before is taken at once, else a pass begins.
pub(super) fn start(service: &Service, key: &str, source: &Arc<Source>, opened: &Arc<Opened>) {
    let Origin::File(path) = &source.origin else {
        return;
    };
    if source.declined.load(Ordering::Relaxed) || source.building.swap(true, Ordering::Relaxed) {
        return;
    }
    let Some(target) = cache_path(path) else {
        return;
    };
    if target.is_file() && attach(opened, &target).is_ok() {
        source.building.store(false, Ordering::Relaxed);
        service.forget_tiles();
        return;
    }
    let name = path
        .file_name()
        .map_or_else(|| key.to_owned(), |n| n.to_string_lossy().into_owned());
    let stop = Arc::new(AtomicBool::new(false));
    {
        let mut running = passes().lock().unwrap_or_else(PoisonError::into_inner);
        // A pass for this file runs already (the raster taken away and given back, an undo
        // and a redo): this one waits for it, which hands it its levels when done.
        if running.contains_key(key) {
            return;
        }
        running.insert(
            key.to_owned(),
            Progress {
                key: key.to_owned(),
                name,
                done: 0.0,
                stop: stop.clone(),
            },
        );
    }
    let (key, source, opened) = (key.to_owned(), source.clone(), opened.clone());
    let _ = std::thread::Builder::new()
        .name("kentos-raster-pyramid".into())
        .spawn(move || {
            let made =
                build(&key, &opened, &target, &stop).is_ok() && !stop.load(Ordering::Relaxed);
            passes()
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .remove(&key);
            // The source this pass began for, and the one the scene registers now if it was given again.
            let service = super::tiles::service();
            let mut sources = vec![(source, Some(opened))];
            if let Some(now) = service.source(&key)
                && !Arc::ptr_eq(&now, &sources[0].0)
            {
                sources.push((now, None));
            }
            for (s, opened) in sources {
                if made {
                    let taken = opened.map_or_else(|| s.opened(), Ok);
                    if let Ok(o) = taken {
                        let _ = attach(&o, &target);
                    }
                } else {
                    s.declined.store(true, Ordering::Relaxed);
                }
                // Made or stopped, the raster draws now: its coarse levels from the file or from level 0.
                s.building.store(false, Ordering::Relaxed);
            }
            if made {
                service.forget_tiles();
            }
            service.wake();
        });
}

/// One pass over level 0 into `target` (a file beside it first, renamed when whole).
fn build(
    key: &str,
    opened: &Opened,
    target: &std::path::Path,
    stop: &AtomicBool,
) -> Result<(), String> {
    let (w, h, bands, sample, nodata, palette) = {
        let r = opened.reader.lock().unwrap_or_else(PoisonError::into_inner);
        (
            r.info.width,
            r.info.height,
            r.info.bands,
            r.info.sample,
            r.nodata,
            r.palette.is_some(),
        )
    };
    if let Some(dir) = target.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let part = target.with_extension("tif.part");
    let mut file = File::create(&part).map_err(|e| e.to_string())?;
    let (mut builder, header) =
        Builder::new(w, h, bands, sample, nodata, palette).map_err(|e| e.0)?;
    file.write_all(&header).map_err(|e| e.to_string())?;
    let mut y = 0;
    while y < h {
        if stop.load(Ordering::Relaxed) {
            drop(file);
            let _ = std::fs::remove_file(&part);
            return Err("durduruldu".into());
        }
        let n = TILE.min(h - y);
        opened.fill(0, 0, i64::from(y), w, n)?;
        let region = opened
            .reader
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .region(0, 0, i64::from(y), w, n)
            .ok_or_else(|| "Rasterin satırları okunamadı.".to_owned())?;
        let bytes = builder.push(&region).map_err(|e| e.0)?;
        file.write_all(&bytes).map_err(|e| e.to_string())?;
        y += n;
        if let Some(p) = passes()
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get_mut(key)
        {
            p.done = f64::from(y) / f64::from(h);
        }
        super::tiles::service().wake();
    }
    let (dirs, head) = builder.finish().map_err(|e| e.0)?;
    file.write_all(&dirs).map_err(|e| e.to_string())?;
    file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
    file.write_all(&head).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    std::fs::rename(&part, target).map_err(|e| e.to_string())
}
