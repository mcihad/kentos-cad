//! The rasters' tiles on the desktop (docs/adr/0204 §3, §5, §11): what the
//! raster pass asks the host for ([`ImageSource::raster_tile`]) made off the
//! interface's thread. The drawing's rasters are registered by their key
//! (`file:<path>` or `asset:<id>`) as the scene is built; a tile asked for
//! goes on the queue, nearest the view's centre first (the pass asks so);
//! workers (one less than the cores, one to four) read and decode its blocks
//! and colour it; a tile not asked for again by the next frame is dropped
//! (the frames' generation). A reader's lock is held only to list a tile's
//! blocks, keep them and take its region: reading, decoding and colouring
//! run beside the other workers. Finished tiles are kept, the least recently
//! used let go first, within a budget; each finished batch asks the window
//! for a frame ([`ready`]).
//!
//! A large raster without overviews gets its pyramid file first ([`super::pyramid`]):
//! until it is made, its coarse levels are not made from level 0.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};

use iced::futures::{SinkExt, Stream};
use kentos_contracts::{RasterRender, RasterSample, RasterStretch, RasterStyle};
use kentos_formats::raster::source::{BlockNeed, PYRAMID_FIRST, Put, Reader, decode_block};
use kentos_formats::raster::stats::Stats;
use kentos_formats::raster::{ByteStore, Step, tiff};

use crate::pointclouds::bytes::{Bytes, Remote};

/// Bytes a raster's decoded blocks may take.
const READER_BUDGET: usize = 96 * 1024 * 1024;
/// Bytes the finished tiles may take (about 240 tiles).
const TILE_BUDGET: usize = 64 * 1024 * 1024;
/// The most bytes a file's header may take (directories, offsets).
const HEADER_MOST: u64 = 64 * 1024 * 1024;
/// A whole PNG or JPEG is read and decoded up to this size.
const IMAGE_MOST: u64 = 512 * 1024 * 1024;

/// Where a raster's bytes are.
#[derive(Clone, Debug)]
pub enum Origin {
    /// A linked file.
    File(PathBuf),
    /// An embedded raster's bytes from the project's library.
    Bytes(Arc<Vec<u8>>),
    /// A GeoTIFF (a COG) at an address, read by HTTP ranges (docs/adr/0207 §1).
    Url(String),
}

impl PartialEq for Origin {
    fn eq(&self, other: &Origin) -> bool {
        match (self, other) {
            (Origin::File(a), Origin::File(b)) => a == b,
            (Origin::Bytes(a), Origin::Bytes(b)) => Arc::ptr_eq(a, b) || a == b,
            (Origin::Url(a), Origin::Url(b)) => a == b,
            _ => false,
        }
    }
}

/// A tile's name: its paint hashed, its level and place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct TileKey {
    paint: u64,
    level: u32,
    tx: u32,
    ty: u32,
}

/// The paint's hash: its raster, look and pixel (as the pass's atlas names it).
fn paint_hash(raster: &str, look: &str, affine: &[f64; 6]) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    raster.hash(&mut h);
    look.hash(&mut h);
    for v in [affine[1], affine[2], affine[4], affine[5]] {
        v.to_bits().hash(&mut h);
    }
    h.finish()
}

/// A tile asked for.
#[derive(Clone, Debug)]
struct Want {
    raster: String,
    look: String,
    affine: [f64; 6],
    /// The frame it was last asked in.
    generation: u64,
}

#[derive(Default)]
struct Queue {
    order: VecDeque<TileKey>,
    wanted: HashMap<TileKey, Want>,
    busy: HashSet<TileKey>,
}

/// The finished tiles, the least recently used let go first.
#[derive(Default)]
struct TileCache {
    tiles: HashMap<TileKey, (Arc<Vec<u8>>, u64)>,
    tick: u64,
    bytes: usize,
}

impl TileCache {
    fn get(&mut self, key: &TileKey) -> Option<Arc<Vec<u8>>> {
        self.tick += 1;
        let tick = self.tick;
        self.tiles.get_mut(key).map(|(t, at)| {
            *at = tick;
            t.clone()
        })
    }

    fn put(&mut self, key: TileKey, tile: Arc<Vec<u8>>) {
        self.tick += 1;
        self.bytes += tile.len();
        if let Some((old, _)) = self.tiles.insert(key, (tile, self.tick)) {
            self.bytes -= old.len();
        }
        while self.bytes > TILE_BUDGET {
            let Some((&oldest, _)) = self.tiles.iter().min_by_key(|(_, (_, at))| *at) else {
                break;
            };
            if let Some((t, _)) = self.tiles.remove(&oldest) {
                self.bytes -= t.len();
            }
        }
    }

    fn drop_paints(&mut self, keep: impl Fn(u64) -> bool) {
        let gone: Vec<TileKey> = self
            .tiles
            .keys()
            .filter(|k| !keep(k.paint))
            .copied()
            .collect();
        for k in gone {
            if let Some((t, _)) = self.tiles.remove(&k) {
                self.bytes -= t.len();
            }
        }
    }
}

/// A raster opened: its reader, the file blocks are read from, its statistics.
pub(super) struct Opened {
    pub(super) reader: Mutex<Reader>,
    /// Level 0's bytes (a TIFF read in pieces: a file, an address or the
    /// library's), or none (a whole image decoded at once).
    source: Option<Bytes>,
    /// The file's size in bytes.
    pub(super) size: u64,
    /// What names its pyramid file: a linked file's path, size and change
    /// time, an address with its size and version; none when embedded.
    pub(super) identity: Option<String>,
    /// The pyramid file once made (blocks of file 1).
    pub(super) pyramid: Mutex<Option<File>>,
    stats: Mutex<Option<Stats>>,
}

impl Opened {
    /// A run of the raster's bytes: its file's, its pyramid's or its own.
    fn read(&self, need: &BlockNeed) -> Result<Vec<u8>, String> {
        if need.len > 64 * 1024 * 1024 {
            return Err("Rasterin bir bloğu 64 MB'tan büyük.".into());
        }
        if need.file == 1 {
            let pyramid = self.pyramid.lock().unwrap_or_else(PoisonError::into_inner);
            return match pyramid.as_ref() {
                Some(f) => read_at(f, need.offset, need.len),
                None => Err("Önizleme piramidi henüz yok.".into()),
            };
        }
        match &self.source {
            Some(b) => b.read(need.offset, need.len),
            None => Err("Rasterin dosyası açık değil.".into()),
        }
    }

    /// Every block a region wants read, decoded and kept; false when one cannot be.
    pub(super) fn fill(&self, level: usize, x: i64, y: i64, w: u32, h: u32) -> Result<(), String> {
        // Twice at most: a block let go by another worker between keeping and use is read again.
        for _ in 0..3 {
            let needs = self
                .reader
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .needs(level, x, y, w, h);
            if needs.is_empty() {
                return Ok(());
            }
            for need in &needs {
                let layout = self
                    .reader
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .layout_for(need)
                    .ok_or_else(|| "Rasterin böyle bir bloğu yok.".to_owned())?;
                let bytes = self.read(need)?;
                let decoded = decode_block(&layout, need.index, &bytes).map_err(|e| e.0)?;
                let put = self
                    .reader
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .keep(need, decoded);
                if let Put::Jpeg(stream) = put {
                    let (pixels, comps) = jpeg_pixels(&stream)?;
                    self.reader
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .put_pixels(need, pixels, comps)
                        .map_err(|e| e.0)?;
                }
            }
        }
        Ok(())
    }

    /// The statistics a look stretches by (docs/adr/0204 §4), worked out once
    /// from the finest level no wider than 1024 pixels.
    fn stats(&self) -> Result<Stats, String> {
        if let Some(s) = self
            .stats
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
        {
            return Ok(s);
        }
        let (level, w, h) = {
            let r = self.reader.lock().unwrap_or_else(PoisonError::into_inner);
            let level = r.stats_level();
            let lv = &r.levels[level];
            (level, lv.width, lv.height)
        };
        self.fill(level, 0, 0, w, h)?;
        let stats = self
            .reader
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .stats()
            .ok_or_else(|| "Rasterin istatistikleri okunamadı.".to_owned())?;
        *self.stats.lock().unwrap_or_else(PoisonError::into_inner) = Some(stats.clone());
        Ok(stats)
    }
}

/// `len` bytes of `file` from `offset`, without moving a shared cursor.
pub(crate) fn read_at(file: &File, offset: u64, len: u64) -> Result<Vec<u8>, String> {
    let mut out = vec![0u8; usize::try_from(len).map_err(|_| "Blok çok büyük.".to_owned())?];
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.read_exact_at(&mut out, offset)
            .map_err(|e| format!("Raster dosyası okunamadı: {e}."))?;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileExt;
        let mut done = 0;
        while done < out.len() {
            let n = file
                .seek_read(&mut out[done..], offset + done as u64)
                .map_err(|e| format!("Raster dosyası okunamadı: {e}."))?;
            if n == 0 {
                return Err("Raster dosyası beklenenden kısa.".into());
            }
            done += n;
        }
    }
    Ok(out)
}

/// A JPEG block's or image's pixels (zune-jpeg) and their components.
pub(crate) fn jpeg_pixels(stream: &[u8]) -> Result<(Vec<u8>, u32), String> {
    use zune_jpeg::JpegDecoder;
    use zune_jpeg::zune_core::bytestream::ZCursor;
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    use zune_jpeg::zune_core::options::DecoderOptions;
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut d = JpegDecoder::new_with_options(ZCursor::new(stream), options);
    let pixels = d.decode().map_err(|e| format!("JPEG çözülemedi: {e}."))?;
    Ok((pixels, 3))
}

/// A linked file's identity for its pyramid: its path, size and change time.
fn file_identity(path: &std::path::Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    let changed = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    Some(format!("{}|{}|{changed}", path.display(), meta.len()))
}

/// A raster opened from where its bytes are; why not when it cannot be.
pub(super) fn open(origin: &Origin) -> Result<Opened, String> {
    let read = open_reader(origin, READER_BUDGET)?;
    Ok(Opened {
        reader: Mutex::new(read.reader),
        source: read.source,
        size: read.size,
        identity: read.identity,
        pyramid: Mutex::new(None),
        stats: Mutex::new(None),
    })
}

/// A raster's reader, its header read, keeping at most `budget` bytes of blocks.
pub(crate) struct ReaderOpen {
    pub(crate) reader: Reader,
    /// Level 0's bytes (a TIFF read in pieces), or none (a whole image decoded at once).
    pub(crate) source: Option<Bytes>,
    /// The file's size in bytes.
    pub(crate) size: u64,
    /// What names its pyramid file (a TIFF's only).
    pub(crate) identity: Option<String>,
}

/// A raster's reader from where its bytes are (the scene's, an analysis's
/// own with its own budget, docs/adr/0231 §2); why not when it cannot be.
pub(crate) fn open_reader(origin: &Origin, budget: usize) -> Result<ReaderOpen, String> {
    let (source, identity) = match origin {
        Origin::File(path) => (Bytes::file(path)?, file_identity(path)),
        Origin::Bytes(b) => (Bytes::Memory(b.clone()), None),
        Origin::Url(u) => {
            let r = Remote::open(u.trim())?;
            let id = format!(
                "url|{}|{}|{}",
                r.url,
                r.size,
                r.version.clone().unwrap_or_default()
            );
            (Bytes::Remote(r), Some(id))
        }
    };
    let size = source.size();
    let head = source.read(0, size.min(16))?;
    let remote = matches!(origin, Origin::Url(_));
    if remote && !tiff::sniff(&head) {
        return Err(
            "Adresten yalnız GeoTIFF (COG) eklenir; PNG ve JPEG'i indirip dosya olarak ekleyin."
                .into(),
        );
    }
    let whole = |limit: u64| -> Result<Vec<u8>, String> {
        if size > limit {
            return Err(format!(
                "Raster {} MB; bu biçimde en çok {} MB okunur.",
                size >> 20,
                limit >> 20
            ));
        }
        source.read(0, size)
    };
    let reader = if tiff::sniff(&head) {
        let mut store = ByteStore::new();
        let mut taken = 0u64;
        let t = loop {
            match tiff::parse(&store, size).map_err(|e| e.0)? {
                Step::Done(t) => break t,
                Step::Need(n) => {
                    taken += n.len;
                    if taken > HEADER_MOST {
                        return Err("TIFF'in başlığı çok büyük.".into());
                    }
                    store.put(n.offset, source.read(n.offset, n.len)?);
                }
            }
        };
        Reader::tiff(&t, None, budget).map_err(|e| e.0)?
    } else if kentos_formats::raster::png::sniff(&head) {
        let p = kentos_formats::raster::png::decode(&whole(IMAGE_MOST)?).map_err(|e| e.0)?;
        Reader::image(
            p.width, p.height, p.bands, p.samples, p.nodata, None, budget,
        )
        .map_err(|e| e.0)?
    } else if head.starts_with(&[0xFF, 0xD8]) {
        let data = whole(IMAGE_MOST)?;
        let (w, h) = jpeg_size(&data)?;
        let (pixels, comps) = jpeg_pixels(&data)?;
        Reader::image(
            w,
            h,
            comps,
            kentos_formats::raster::Samples::U8(pixels),
            None,
            None,
            budget,
        )
        .map_err(|e| e.0)?
    } else {
        return Err("Dosya GeoTIFF, TIFF, PNG ya da JPEG değil.".into());
    };
    let tiled = tiff::sniff(&head);
    Ok(ReaderOpen {
        reader,
        source: tiled.then_some(source),
        size,
        identity: identity.filter(|_| tiled),
    })
}

/// A JPEG's size from its header (zune-jpeg).
pub(super) fn jpeg_size(data: &[u8]) -> Result<(u32, u32), String> {
    use zune_jpeg::JpegDecoder;
    use zune_jpeg::zune_core::bytestream::ZCursor;
    let mut d = JpegDecoder::new(ZCursor::new(data));
    d.decode_headers()
        .map_err(|e| format!("JPEG okunamadı: {e}."))?;
    let (w, h) = d
        .dimensions()
        .ok_or_else(|| "JPEG'in boyu yok.".to_owned())?;
    Ok((w as u32, h as u32))
}

/// A raster the scene draws: where it is and, once a worker opened it, the opened one.
pub(super) struct Source {
    pub(super) origin: Origin,
    pub(super) opened: Mutex<Option<Result<Arc<Opened>, String>>>,
    /// Its pyramid file is being made: its coarse levels wait.
    pub(super) building: AtomicBool,
    /// Its pyramid's pass was stopped: its coarse levels come from level 0.
    pub(super) declined: AtomicBool,
}

impl Source {
    /// The opened raster, opening it the first time; why not.
    pub(super) fn opened(&self) -> Result<Arc<Opened>, String> {
        let mut o = self.opened.lock().unwrap_or_else(PoisonError::into_inner);
        if o.is_none() {
            *o = Some(open(&self.origin).map(Arc::new));
        }
        o.clone().unwrap_or_else(|| Err("Raster açılamadı.".into()))
    }

    fn failed(&self) -> bool {
        matches!(
            &*self.opened.lock().unwrap_or_else(PoisonError::into_inner),
            Some(Err(_))
        )
    }
}

/// Whether a look stretches by the raster's statistics.
fn wants_stats(style: &RasterStyle, sample: RasterSample) -> bool {
    match style.render {
        RasterRender::Hillshade | RasterRender::Palette => false,
        _ => match style.stretch {
            RasterStretch::Manual => false,
            RasterStretch::MinMax | RasterStretch::Percent => true,
            RasterStretch::None => sample != RasterSample::U8,
        },
    }
}

/// The service's shared state.
pub(super) struct Shared {
    pub(super) sources: Mutex<HashMap<String, Arc<Source>>>,
    /// The rasters each drawing thread's scene shows: the interface's one
    /// thread in the app; in the tests each test's own, so that one test's
    /// scene never lets go of another's rasters.
    shown: Mutex<HashMap<std::thread::ThreadId, HashSet<String>>>,
    looks: Mutex<HashMap<String, Arc<RasterStyle>>>,
    queue: Mutex<Queue>,
    work: Condvar,
    tiles: Mutex<TileCache>,
    generation: AtomicU64,
    /// A tile was made since the window last heard.
    fresh: Mutex<bool>,
    told: Condvar,
}

/// The rasters' tiles of the desktop: one for the app.
pub struct Service {
    pub(super) shared: Arc<Shared>,
}

/// Whether the drawing has drawn a raster (the window then listens for tiles).
pub fn in_use() -> bool {
    USED.load(Ordering::Relaxed)
}

static USED: AtomicBool = AtomicBool::new(false);

/// The app's service, its workers started the first time.
pub fn service() -> &'static Service {
    static SERVICE: OnceLock<Service> = OnceLock::new();
    SERVICE.get_or_init(|| {
        let shared = Arc::new(Shared {
            sources: Mutex::new(HashMap::new()),
            shown: Mutex::new(HashMap::new()),
            looks: Mutex::new(HashMap::new()),
            queue: Mutex::new(Queue::default()),
            work: Condvar::new(),
            tiles: Mutex::new(TileCache::default()),
            generation: AtomicU64::new(0),
            fresh: Mutex::new(false),
            told: Condvar::new(),
        });
        let workers = std::thread::available_parallelism()
            .map_or(1, |n| n.get().saturating_sub(1))
            .clamp(1, 4);
        for i in 0..workers {
            let s = shared.clone();
            let _ = std::thread::Builder::new()
                .name(format!("kentos-raster-{i}"))
                .spawn(move || worker(&s));
        }
        Service { shared }
    })
}

impl Service {
    /// The scene draws the raster `key` from `origin`: registered, or its
    /// tiles let go when it now comes from elsewhere.
    pub fn register(&self, key: &str, origin: impl FnOnce() -> Option<Origin>) {
        USED.store(true, Ordering::Relaxed);
        // This thread shows it until its scene says otherwise (`keep_only`).
        {
            let mut shown = self
                .shared
                .shown
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            let mine = shown.entry(std::thread::current().id()).or_default();
            if !mine.contains(key) {
                mine.insert(key.to_owned());
            }
        }
        let mut sources = self
            .shared
            .sources
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if sources.contains_key(key) {
            return;
        }
        if let Some(origin) = origin() {
            sources.insert(
                key.to_owned(),
                Arc::new(Source {
                    origin,
                    opened: Mutex::new(None),
                    building: AtomicBool::new(false),
                    declined: AtomicBool::new(false),
                }),
            );
        }
    }

    /// Lets go of every raster no drawing thread's scene shows (another drawing opened, a raster
    /// removed): `keep` is this thread's.
    pub fn keep_only(&self, keep: &HashSet<String>) {
        let mut shown = self
            .shared
            .shown
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let thread = std::thread::current().id();
        if shown.get(&thread) != Some(keep) {
            shown.insert(thread, keep.clone());
        }
        let mut sources = self
            .shared
            .sources
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        let before = sources.len();
        sources.retain(|k, _| shown.values().any(|s| s.contains(k)));
        drop(shown);
        if sources.len() != before {
            let alive: HashSet<String> = sources.keys().cloned().collect();
            drop(sources);
            // Their requests go; their tiles age out of the cache.
            let mut q = self
                .shared
                .queue
                .lock()
                .unwrap_or_else(PoisonError::into_inner);
            q.wanted.retain(|_, w| alive.contains(&w.raster));
            let Queue { order, wanted, .. } = &mut *q;
            order.retain(|k| wanted.contains_key(k));
        }
    }

    /// Lets every tile go (a raster's file changed on the disk, a style's look).
    pub fn forget_tiles(&self) {
        self.shared
            .tiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .drop_paints(|_| false);
    }

    /// The source the scene registered, if any.
    pub(super) fn source(&self, key: &str) -> Option<Arc<Source>> {
        self.shared
            .sources
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
            .cloned()
    }

    /// Waits until no tile asked for is waiting or being made, at most
    /// `limit`; whether tiles were made meanwhile (tests and pictures).
    #[cfg(test)]
    pub fn settle(&self, limit: std::time::Duration) -> bool {
        let start = std::time::Instant::now();
        let made_before = self
            .shared
            .tiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .tick;
        loop {
            let idle = {
                let q = self
                    .shared
                    .queue
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner);
                q.order.is_empty() && q.busy.is_empty()
            };
            let building = self
                .shared
                .sources
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .values()
                .any(|s| s.building.load(Ordering::Relaxed));
            if (idle && !building) || start.elapsed() > limit {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        self.shared
            .tiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .tick
            != made_before
    }

    /// Asks the window for a frame (a pyramid's progress, a pass done).
    pub fn wake(&self) {
        *self
            .shared
            .fresh
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = true;
        self.shared.told.notify_one();
    }

    /// A frame begins: what is not asked for again by its end may be dropped.
    pub fn frame(&self) {
        self.shared.generation.fetch_add(1, Ordering::Relaxed);
    }

    /// A tile when made; otherwise it is asked for.
    pub fn tile(
        &self,
        raster: &str,
        look: &str,
        affine: &[f64; 6],
        level: u32,
        tx: u32,
        ty: u32,
    ) -> Option<Arc<Vec<u8>>> {
        let key = TileKey {
            paint: paint_hash(raster, look, affine),
            level,
            tx,
            ty,
        };
        if let Some(t) = self
            .shared
            .tiles
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&key)
        {
            return Some(t);
        }
        let source = self.source(raster)?;
        if source.failed() {
            return None;
        }
        // A large raster's coarse levels wait for its pyramid file.
        if level as usize >= PYRAMID_FIRST && source.building.load(Ordering::Relaxed) {
            return None;
        }
        let generation = self.shared.generation.load(Ordering::Relaxed);
        let mut q = self
            .shared
            .queue
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        match q.wanted.get_mut(&key) {
            Some(w) => w.generation = generation,
            None => {
                q.wanted.insert(
                    key,
                    Want {
                        raster: raster.to_owned(),
                        look: look.to_owned(),
                        affine: *affine,
                        generation,
                    },
                );
                q.order.push_back(key);
                self.shared.work.notify_one();
            }
        }
        None
    }

    /// The parsed look of a style's JSON.
    fn look(&self, text: &str) -> Option<Arc<RasterStyle>> {
        let mut looks = self
            .shared
            .looks
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if let Some(l) = looks.get(text) {
            return Some(l.clone());
        }
        let parsed = Arc::new(RasterStyle::from_json_text(text)?);
        looks.insert(text.to_owned(), parsed.clone());
        Some(parsed)
    }
}

/// A worker: takes the next tile asked for in this frame or the last, makes it.
fn worker(shared: &Arc<Shared>) {
    let service = Service {
        shared: shared.clone(),
    };
    loop {
        let (key, want) = {
            let mut q = shared.queue.lock().unwrap_or_else(PoisonError::into_inner);
            loop {
                let now = shared.generation.load(Ordering::Relaxed);
                let Some(key) = q.order.pop_front() else {
                    q = shared.work.wait(q).unwrap_or_else(PoisonError::into_inner);
                    continue;
                };
                let Some(want) = q.wanted.get(&key).cloned() else {
                    continue;
                };
                // Out of view since: dropped (docs/adr/0204 §11).
                if want.generation + 1 < now {
                    q.wanted.remove(&key);
                    continue;
                }
                if !q.busy.insert(key) {
                    continue;
                }
                break (key, want);
            }
        };
        let made = make(&service, &want, key);
        {
            let mut q = shared.queue.lock().unwrap_or_else(PoisonError::into_inner);
            q.busy.remove(&key);
            q.wanted.remove(&key);
        }
        if let Some(tile) = made {
            shared
                .tiles
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .put(key, Arc::new(tile));
            *shared.fresh.lock().unwrap_or_else(PoisonError::into_inner) = true;
            shared.told.notify_one();
        }
    }
}

/// A tile's colours; none when its raster cannot be read or it must wait.
fn make(service: &Service, want: &Want, key: TileKey) -> Option<Vec<u8>> {
    let source = service.source(&want.raster)?;
    let opened = source.opened().ok()?;
    let style = service.look(&want.look)?;
    let level = key.level as usize;
    // A large raster without overviews: its pyramid first (docs/adr/0204 §3).
    let (needs_pyramid, sample) = {
        let r = opened.reader.lock().unwrap_or_else(PoisonError::into_inner);
        (r.info.needs_pyramid, r.info.sample)
    };
    if needs_pyramid && !source.declined.load(Ordering::Relaxed) {
        super::pyramid::start(service, &want.raster, &source, &opened);
        if level >= PYRAMID_FIRST && source.building.load(Ordering::Relaxed) {
            return None;
        }
    }
    let stats = if wants_stats(&style, sample) {
        Some(opened.stats().ok()?)
    } else {
        None
    };
    let (x, y, w, h) = Reader::tile_region(key.tx, key.ty);
    opened.fill(level, x, y, w, h).ok()?;
    let parts = opened
        .reader
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .tile_parts(level, key.tx, key.ty, &want.affine)?;
    let mut out = Vec::new();
    parts.render(&style, stats.as_ref(), &mut out);
    Some(out)
}

/// The bands' statistics of raster `key` (Raster stili), worked out once and kept.
pub fn stats_of(key: &str, origin: Origin) -> Result<Stats, String> {
    let s = service();
    s.register(key, || Some(origin));
    let source = s
        .source(key)
        .ok_or_else(|| "Raster bulunamadı.".to_owned())?;
    source.opened()?.stats()
}

/// The bands' values at pixel (`i`, `j`) of raster `key` (Koordinat oku); none outside or unread.
pub fn values_at(key: &str, origin: Origin, i: i64, j: i64) -> Option<Vec<f64>> {
    let s = service();
    s.register(key, || Some(origin));
    let opened = s.source(key)?.opened().ok()?;
    opened.fill(0, i, j, 1, 1).ok()?;
    opened
        .reader
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .sample(i, j)
}

/// The window's message each time tiles were made (at most one a frame, ~60 a second).
pub fn ready() -> impl Stream<Item = crate::app::Message> {
    let (mut out, stream) = iced::futures::channel::mpsc::channel(1);
    let shared = service().shared.clone();
    std::thread::spawn(move || {
        loop {
            {
                let mut fresh = shared.fresh.lock().unwrap_or_else(PoisonError::into_inner);
                while !*fresh {
                    fresh = shared
                        .told
                        .wait(fresh)
                        .unwrap_or_else(PoisonError::into_inner);
                }
                *fresh = false;
            }
            if iced::futures::executor::block_on(out.send(crate::app::Message::RastersReady))
                .is_err()
            {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });
    stream
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/interaction/v1/rasters")
            .join(name)
    }

    /// A tile asked for is made by the workers and kept: the photograph's
    /// first tile is opaque colour, the elevation model's lit ramp too.
    #[test]
    fn tiles_asked_for_are_made() {
        let s = service();
        for (key, file, look, affine) in [
            (
                "file:test/orto.tif",
                "orto.tif",
                r#"{"render":"rgb","bands":[1,2,3]}"#,
                [0.0, 0.6, 0.0, 0.0, 0.0, -0.6],
            ),
            (
                "file:test/dem.tif",
                "dem.tif",
                r#"{"render":"rampShade","bands":[1],"stretch":"minMax","ramp":"Arazi"}"#,
                [0.0, 1.6, 0.0, 0.0, 0.0, -1.6],
            ),
        ] {
            let mut got = None;
            for _ in 0..2000 {
                // Another test's drawing may let it go (`keep_only`): given again, as a scene gives it.
                s.register(key, || Some(Origin::File(fixture(file))));
                s.frame();
                if let Some(t) = s.tile(key, look, &affine, 0, 1, 1) {
                    got = Some(t);
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            let tile = got.unwrap_or_else(|| panic!("{key}: the tile was not made"));
            assert_eq!(tile.len(), 258 * 258 * 4);
            let opaque = tile.chunks_exact(4).filter(|p| p[3] == 255).count();
            assert!(opaque > 258 * 258 * 9 / 10, "{key}: {opaque} opaque pixels");
            let lit = tile
                .chunks_exact(4)
                .filter(|p| p[0] > 0 || p[1] > 0 || p[2] > 0)
                .count();
            assert!(lit > 258 * 258 * 9 / 10, "{key}: {lit} coloured pixels");
        }
    }
}
