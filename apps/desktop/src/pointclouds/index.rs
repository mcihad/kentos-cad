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

use std::collections::{BTreeMap, HashMap};
use std::fs::{File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use kentos_contracts::CloudFormat;
use kentos_pointcloud::copc::Key;
use kentos_pointcloud::index::{BinOut, Builder, Cube, Distribute, Plan};
use kentos_pointcloud::record::{Layout, standard_len, wide_format, widen};
use kentos_pointcloud::source::{Cloud, Opening};
use kentos_pointcloud::{Step, text};

use super::bytes::{Bytes, Origin};
use super::service::Member;

/// The most the cache keeps.
const CACHE_MOST: u64 = 16 * 1024 * 1024 * 1024;
/// A text cloud is read in pieces of this size.
const PIECE: u64 = 4 * 1024 * 1024;

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

fn bin_path(dir: &std::path::Path, k: Key) -> PathBuf {
    dir.join(format!("{}-{}-{}-{}.bin", k.d, k.x, k.y, k.z))
}

fn append(dir: &std::path::Path, k: Key, bytes: &[u8]) -> Result<(), String> {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(bin_path(dir, k))
        .map_err(|e| format!("Dizinin kutusu yazılamadı: {e}."))?;
    f.write_all(bytes)
        .map_err(|e| format!("Dizinin kutusu yazılamadı: {e}."))
}

/// The records of a source, widened, handed over in its order (a LAS file's header boxed: it is large).
enum Reading {
    Las(Box<Cloud>, Bytes),
    Text(text::Plan, Bytes),
}

/// Opens the source for reading.
fn reading(m: &Member) -> Result<Reading, String> {
    let bytes = m.bytes()?;
    if m.format == CloudFormat::Xyz {
        let mut scan = text::Scan::new();
        let size = bytes.size();
        let mut at = 0;
        while at < size {
            let n = PIECE.min(size - at);
            scan.feed(&bytes.read(at, n)?).map_err(|e| e.0)?;
            at += n;
        }
        let plan = scan.finish().map_err(|e| e.0)?;
        return Ok(Reading::Text(plan, bytes));
    }
    let mut o = Opening::new(bytes.size());
    let cloud = loop {
        match o.step().map_err(|e| e.0)? {
            Step::Done(c) => break c,
            Step::Need(n) => o.put(n.offset, bytes.read(n.offset, n.len)?),
        }
    };
    Ok(Reading::Las(Box::new(cloud), bytes))
}

/// The system's WKT for the index: the file's, else the registry's for its EPSG code.
fn wkt_of(cloud: &Cloud) -> Option<String> {
    if let Some(w) = &cloud.crs.wkt {
        return Some(w.clone());
    }
    let epsg = cloud.crs.epsg?;
    let system = kentos_project::crs::system(epsg)?;
    kentos_geometry_core::crs::text::write_wkt(&system.name, &system.transform_system()?, None)
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
    let result = build(reading, &mut tick, &bins, &part);
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

/// The plan of a source.
fn plan_of(reading: &Reading) -> (Plan, Option<(Layout, Layout)>) {
    match reading {
        Reading::Las(c, _) => {
            let from = c.layout;
            let fmt = wide_format(from.format);
            let to = Layout::new(fmt, standard_len(fmt) + from.extra());
            let mut plan = Plan::new(
                c.bounds(),
                c.head.count,
                c.head.scale,
                c.head.offset,
                fmt,
                from.extra() as u16,
            );
            plan.wkt = wkt_of(c);
            plan.extra_vlr = c
                .vlrs
                .iter()
                .chain(&c.evlrs)
                .find(|v| v.is("LASF_Spec", 4))
                .cloned();
            plan.gps_standard = c.head.gps_standard();
            plan.source_id = c.head.source_id;
            (plan, Some((from, to)))
        }
        Reading::Text(p, _) => (
            Plan::new(p.bounds, p.count, p.scale, p.offset, p.columns.format(), 0),
            None,
        ),
    }
}

/// The index of `reading` written at `part`, its bins in `bins`; `tick`
/// takes the share done (0–1) and says whether to stop (an empty error then).
fn build(
    reading: Reading,
    tick: &mut dyn FnMut(f64) -> bool,
    bins: &std::path::Path,
    part: &std::path::Path,
) -> Result<(), String> {
    let (mut plan, layouts) = plan_of(&reading);
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get().saturating_sub(1))
        .clamp(1, 4);
    let mut attempt = 0;
    let summary = loop {
        attempt += 1;
        let _ = std::fs::remove_dir_all(bins);
        std::fs::create_dir_all(bins).map_err(|e| format!("Dizinin kutuları açılamadı: {e}."))?;
        let mut dist = Distribute::new(plan.clone());
        match &reading {
            Reading::Las(cloud, bytes) => {
                let runs = cloud.runs();
                let total = runs.len().max(1);
                let (from, to) = layouts.ok_or("Kaydın düzeni yok.")?;
                for (b, batch) in runs.chunks(threads * 2).enumerate() {
                    if tick(0.5 * (b * threads * 2).min(total) as f64 / total as f64) {
                        return Err(String::new());
                    }
                    let read: Vec<Result<Vec<u8>, String>> = std::thread::scope(|s| {
                        let handles: Vec<_> = batch
                            .iter()
                            .map(|run| {
                                s.spawn(move || -> Result<Vec<u8>, String> {
                                    let raw = bytes.read(run.need.offset, run.need.len)?;
                                    let mut recs = Vec::new();
                                    cloud.records(run, &raw, &mut recs).map_err(|e| e.0)?;
                                    let mut wide = vec![0u8; to.len * run.count as usize];
                                    for (i, r) in recs.chunks_exact(from.len).enumerate() {
                                        widen(
                                            &from,
                                            r,
                                            &to,
                                            &mut wide[i * to.len..(i + 1) * to.len],
                                        );
                                    }
                                    Ok(wide)
                                })
                            })
                            .collect();
                        handles
                            .into_iter()
                            .map(|h| h.join().unwrap_or_else(|_| Err("Parça çözülemedi.".into())))
                            .collect()
                    });
                    for (run, wide) in batch.iter().zip(read) {
                        for (k, chunk) in dist.add(&wide?, run.first) {
                            append(bins, k, &chunk)?;
                        }
                    }
                }
            }
            Reading::Text(p, bytes) => {
                let mut parse = text::Parse::new(p.clone());
                let size = bytes.size().max(1);
                let mut at = 0;
                let mut first = 0u64;
                let mut out = Vec::new();
                while at < bytes.size() {
                    if tick(0.5 * at as f64 / size as f64) {
                        return Err(String::new());
                    }
                    let n = PIECE.min(bytes.size() - at);
                    out.clear();
                    parse.feed(&bytes.read(at, n)?, &mut out).map_err(|e| e.0)?;
                    at += n;
                    for (k, chunk) in dist.add(&out, first) {
                        append(bins, k, &chunk)?;
                    }
                    first += (out.len() / parse.record_len()) as u64;
                }
                out.clear();
                parse.finish(&mut out).map_err(|e| e.0)?;
                for (k, chunk) in dist.add(&out, first) {
                    append(bins, k, &chunk)?;
                }
            }
        }
        let (rest, summary) = dist.finish();
        for (k, chunk) in rest {
            append(bins, k, &chunk)?;
        }
        if summary.outside && attempt == 1 {
            // The header's bounds were wrong: once more round the points' own (not again).
            plan.cube = Cube::of(summary.bounds);
            continue;
        }
        break summary;
    };
    let mut builder = Builder::new(plan.clone(), summary.clone()).map_err(|e| e.0)?;
    let laz = builder.laz().clone();
    let mut out =
        File::create(part).map_err(|e| format!("“{}” yazılamadı: {e}.", part.display()))?;
    out.write_all(&builder.head().map_err(|e| e.0)?)
        .map_err(|e| e.to_string())?;
    let mut queue: Vec<Key> = summary.bins.keys().copied().collect();
    let total = queue.len().max(1);
    let mut built = 0usize;
    while !queue.is_empty() {
        if tick(0.5 + 0.5 * built as f64 / total as f64) {
            return Err(String::new());
        }
        let take: Vec<Key> = queue.drain(..threads.min(queue.len())).collect();
        let results: Vec<Result<BinOut, String>> = std::thread::scope(|s| {
            let handles: Vec<_> = take
                .iter()
                .map(|&k| {
                    let (plan, laz) = (&plan, &laz);
                    s.spawn(move || -> Result<BinOut, String> {
                        let bytes = std::fs::read(bin_path(bins, k))
                            .map_err(|e| format!("Dizinin kutusu okunamadı: {e}."))?;
                        kentos_pointcloud::index::build_bin(plan, laz, k, &bytes).map_err(|e| e.0)
                    })
                })
                .collect();
            handles
                .into_iter()
                .map(|h| h.join().unwrap_or_else(|_| Err("Kutu kurulamadı.".into())))
                .collect()
        });
        let mut front: Vec<Key> = Vec::new();
        for (k, r) in take.iter().zip(results) {
            match r? {
                BinOut::Built { chunks, up } => {
                    for c in &chunks {
                        out.write_all(&c.bytes).map_err(|e| e.to_string())?;
                    }
                    builder.accept(*k, &chunks, up);
                    let _ = std::fs::remove_file(bin_path(bins, *k));
                    built += 1;
                }
                BinOut::Split(parts) => {
                    let _ = std::fs::remove_file(bin_path(bins, *k));
                    let mut sub: BTreeMap<Key, Vec<u8>> = BTreeMap::new();
                    for (sk, b) in parts {
                        sub.entry(sk).or_default().extend(b);
                    }
                    for (sk, b) in sub {
                        append(bins, sk, &b)?;
                        front.push(sk);
                    }
                }
            }
        }
        // A split bin's parts come next, in their order.
        for (i, k) in front.into_iter().enumerate() {
            queue.insert(i, k);
        }
    }
    let fin = builder.finish().map_err(|e| e.0)?;
    for c in &fin.chunks {
        out.write_all(&c.bytes).map_err(|e| e.to_string())?;
    }
    out.write_all(&fin.tail).map_err(|e| e.to_string())?;
    for (at, b) in &fin.patches {
        out.seek(SeekFrom::Start(*at)).map_err(|e| e.to_string())?;
        out.write_all(b).map_err(|e| e.to_string())?;
    }
    out.sync_all().map_err(|e| e.to_string())?;
    Ok(())
}

/// A COPC of the LAS or LAZ at `from` written at `to` (İşlemler's COPC
/// results, docs/adr/0207 §7): the same index, its bins beside `to`;
/// `tick` takes the share and says whether to stop (an empty error then).
pub fn copc_file(
    from: &std::path::Path,
    to: &std::path::Path,
    tick: &mut dyn FnMut(f64) -> bool,
) -> Result<(), String> {
    let bytes = Bytes::file(from)?;
    let mut o = Opening::new(bytes.size());
    let cloud = loop {
        match o.step().map_err(|e| e.0)? {
            Step::Done(c) => break c,
            Step::Need(n) => o.put(n.offset, bytes.read(n.offset, n.len)?),
        }
    };
    let name = to
        .file_name()
        .map_or_else(|| "bulut".to_owned(), |n| n.to_string_lossy().into_owned());
    let bins = to.with_file_name(format!("{name}.kutular"));
    let part = to.with_file_name(format!("{name}.yaziliyor"));
    let result = build(Reading::Las(Box::new(cloud), bytes), tick, &bins, &part);
    let _ = std::fs::remove_dir_all(&bins);
    match result {
        Ok(()) => {
            std::fs::rename(&part, to).map_err(|e| format!("“{}” yazılamadı: {e}.", to.display()))
        }
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
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
