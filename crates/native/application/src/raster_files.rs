//! A raster's file read where it is, for the commands that read its pixels (Raster oturt's resampling, docs/adr/0518):
//! a linked GeoTIFF read by the ranges its reader asks for (its header, then each block a tile needs), a PNG or an
//! embedded raster whole; and a resampled raster written tile by tile to a file beside its name (`.part`, renamed when
//! whole) or into memory. A JPEG block (a JPEG-compressed TIFF, a JPEG) needs the host's decoder: refused here.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use kentos_formats::raster::source::{Put, Reader, Region, open_bytes};
use kentos_formats::raster::warp::Job;
use kentos_formats::raster::{ByteStore, Step};

/// The blocks a reader keeps at most, in bytes.
pub const READER_BUDGET: usize = 256 << 20;

/// The most a header (a TIFF's directories) may take before the file is taken for broken.
const HEADER_MOST: u64 = 64 << 20;

/// The largest PNG read whole.
const WHOLE_MOST: u64 = 512 << 20;

/// Where a raster's bytes are.
pub enum Bytes {
    File(File, u64),
    Memory(Vec<u8>),
}

impl Bytes {
    /// A file's bytes, opened.
    pub fn file(path: &Path) -> Result<Bytes, String> {
        let f = File::open(path).map_err(|e| format!("“{}” açılamadı: {e}.", path.display()))?;
        let size = f.metadata().map(|m| m.len()).unwrap_or(0);
        Ok(Bytes::File(f, size))
    }

    fn size(&self) -> u64 {
        match self {
            Bytes::File(_, n) => *n,
            Bytes::Memory(b) => b.len() as u64,
        }
    }

    fn read(&mut self, at: u64, len: u64) -> Result<Vec<u8>, String> {
        let end = at.checked_add(len).filter(|e| *e <= self.size()).ok_or("Dosya beklenenden kısa.")?;
        match self {
            Bytes::Memory(b) => Ok(b[at as usize..end as usize].to_vec()),
            Bytes::File(f, _) => {
                let mut out = vec![0; usize::try_from(len).map_err(|_| "Okunacak parça çok büyük.")?];
                f.seek(SeekFrom::Start(at)).map_err(|e| e.to_string())?;
                f.read_exact(&mut out).map_err(|e| e.to_string())?;
                Ok(out)
            }
        }
    }
}

/// A raster opened: its reader and its bytes, to read blocks from.
pub struct Opened {
    pub reader: Reader,
    bytes: Bytes,
}

impl std::fmt::Debug for Opened {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Opened").field("info", &self.reader.info).finish()
    }
}

/// A raster's reader from its bytes: a TIFF's header read by ranges; a PNG whole.
pub fn open(mut bytes: Bytes) -> Result<Opened, String> {
    if let Bytes::Memory(b) = &bytes {
        let reader = open_bytes(b, None, READER_BUDGET).map_err(|e| e.0)?;
        return Ok(Opened { reader, bytes });
    }
    let size = bytes.size();
    let head = bytes.read(0, size.min(8))?;
    if kentos_formats::raster::tiff::sniff(&head) {
        let mut store = ByteStore::new();
        let mut taken = 0u64;
        let tiff = loop {
            match kentos_formats::raster::tiff::parse(&store, size).map_err(|e| e.0)? {
                Step::Done(t) => break t,
                Step::Need(n) => {
                    taken += n.len;
                    if taken > HEADER_MOST {
                        return Err("TIFF'in başlığı çok büyük; dosya bozuk olabilir.".into());
                    }
                    store.put(n.offset, bytes.read(n.offset, n.len)?);
                }
            }
        };
        let reader = Reader::tiff(&tiff, None, READER_BUDGET).map_err(|e| e.0)?;
        return Ok(Opened { reader, bytes });
    }
    if size > WHOLE_MOST {
        return Err("Dosya GeoTIFF değil ve bütün okunamayacak kadar büyük.".into());
    }
    let whole = bytes.read(0, size)?;
    let reader = open_bytes(&whole, None, READER_BUDGET).map_err(|e| e.0)?;
    Ok(Opened { reader, bytes: Bytes::Memory(whole) })
}

impl Opened {
    /// Level 0's pixels in the region, its blocks read first.
    pub fn region(&mut self, x: i64, y: i64, w: u32, h: u32) -> Result<Option<Region>, String> {
        for need in self.reader.needs(0, x, y, w, h) {
            let block = self.bytes.read(need.offset, need.len)?;
            match self.reader.put_block(&need, &block).map_err(|e| e.0)? {
                Put::Done => {}
                Put::Jpeg(_) => {
                    return Err(
                        "Raster JPEG ile sıkıştırılmış; bu ortamda çözülemiyor. Masaüstünde oturtun.".into()
                    );
                }
            }
        }
        Ok(self.reader.region(0, x, y, w, h))
    }
}

/// Where a resampled raster goes.
pub enum Out {
    /// A file: written beside it as `.part`, renamed when whole.
    File(PathBuf),
    Memory,
}

/// A resampled raster: its file (written) or bytes, its bands and samples, and whether its last band is alpha.
#[derive(Debug)]
pub struct Written {
    pub path: Option<PathBuf>,
    pub bytes: Option<Vec<u8>>,
    pub bands: u32,
    pub sample: kentos_contracts::RasterSample,
    pub alpha: bool,
}

/// Runs `job` (its `header` first) on `source` into `out`; `stop` asked between tiles, `share` told after each.
pub fn resample(
    source: &mut Opened,
    mut job: Job,
    header: Vec<u8>,
    out: Out,
    stop: &dyn Fn() -> bool,
    share: &dyn Fn(u64, u64),
) -> Result<Written, String> {
    let part = match &out {
        Out::File(p) => Some(p.with_extension("tif.part")),
        Out::Memory => None,
    };
    let cleanup = |part: &Option<PathBuf>| {
        if let Some(p) = part {
            let _ = std::fs::remove_file(p);
        }
    };
    let mut file = match &part {
        Some(p) => Some(File::create(p).map_err(|e| format!("“{}” yazılamadı: {e}.", p.display()))?),
        None => None,
    };
    let mut memory: Vec<u8> = Vec::new();
    let mut put = |bytes: &[u8], file: &mut Option<File>| -> Result<(), String> {
        match file {
            Some(f) => f.write_all(bytes).map_err(|e| e.to_string()),
            None => {
                memory.extend_from_slice(bytes);
                Ok(())
            }
        }
    };
    let total = u64::from(job.grid.width.div_ceil(kentos_formats::raster::TILE))
        * u64::from(job.grid.height.div_ceil(kentos_formats::raster::TILE));
    let run = (|| -> Result<(), String> {
        put(&header, &mut file)?;
        let mut done = 0u64;
        while let Some(region) = job.region() {
            if stop() {
                return Err(STOPPED.into());
            }
            let samples = match region {
                Some((x, y, w, h)) => source.region(x, y, w, h)?,
                None => None,
            };
            let bytes = job.tile(samples.as_ref()).map_err(|e| e.0)?;
            put(&bytes, &mut file)?;
            done += 1;
            share(done, total);
        }
        Ok(())
    })();
    if let Err(e) = run {
        drop(file);
        cleanup(&part);
        return Err(e);
    }
    let (bands, sample, alpha) = (job.bands, job.sample, job.alpha);
    let (dirs, head) = match job.finish() {
        Ok(x) => x,
        Err(e) => {
            drop(file);
            cleanup(&part);
            return Err(e.0);
        }
    };
    if let Err(e) = put(&dirs, &mut file) {
        drop(file);
        cleanup(&part);
        return Err(e);
    }
    match (out, file) {
        (Out::File(target), Some(mut f)) => {
            let done = (|| -> std::io::Result<()> {
                f.seek(SeekFrom::Start(0))?;
                f.write_all(&head)?;
                f.sync_all()
            })();
            drop(f);
            if let Err(e) = done {
                cleanup(&part);
                return Err(format!("“{}” yazılamadı: {e}.", target.display()));
            }
            if let Some(p) = &part
                && let Err(e) = std::fs::rename(p, &target)
            {
                cleanup(&part);
                return Err(format!("“{}” yazılamadı: {e}.", target.display()));
            }
            Ok(Written { path: Some(target), bytes: None, bands, sample, alpha })
        }
        _ => {
            memory[..head.len()].copy_from_slice(&head);
            Ok(Written { path: None, bytes: Some(memory), bands, sample, alpha })
        }
    }
}

/// What a stopped resampling answers.
pub const STOPPED: &str = "durduruldu";
