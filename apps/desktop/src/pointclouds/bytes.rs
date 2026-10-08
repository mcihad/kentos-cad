//! Where a cloud's bytes are read from (docs/adr/0207 §1): a file, an address
//! by HTTP ranges, or the project's library. An address's bytes are read in
//! 64 KB blocks, neighbours in one request, and kept within 64 MB, the least
//! recently used let go first.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

/// A block of an address's bytes.
const BLOCK: u64 = 64 * 1024;
/// Bytes an address's blocks may take.
const KEEP: u64 = 64 * 1024 * 1024;

/// Where a file of a cloud is.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Origin {
    /// A linked file.
    File(PathBuf),
    /// An address, read by ranges.
    Url(String),
    /// An embedded file's bytes, by the library's id.
    Asset(String),
}

/// An address opened: its size and version, and its blocks read so far.
#[derive(Debug)]
pub struct Remote {
    pub url: String,
    pub size: u64,
    pub version: Option<String>,
    blocks: Mutex<Held>,
}

/// The blocks read so far, each by its number with the tick it was last used at, and the clock.
#[derive(Debug, Default)]
struct Held {
    blocks: HashMap<u64, (Arc<Vec<u8>>, u64)>,
    tick: u64,
}

impl Remote {
    pub fn open(url: &str) -> Result<Remote, String> {
        let p = kentos_cloud::range::probe(url)?;
        Ok(Remote {
            url: url.to_owned(),
            size: p.size,
            version: p.version,
            blocks: Mutex::new(Held::default()),
        })
    }

    /// `len` bytes from `offset`.
    pub fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        if len == 0 {
            return Ok(Vec::new());
        }
        let end = offset
            .checked_add(len)
            .filter(|&e| e <= self.size)
            .ok_or_else(|| format!("“{}”: istenen bayt dosyanın dışında.", self.url))?;
        let (first, last) = (offset / BLOCK, (end - 1) / BLOCK);
        // A large read is not kept: it is one request of its own.
        if last - first >= 64 {
            return kentos_cloud::range::read(&self.url, offset, len, self.version.as_deref());
        }
        let mut missing = Vec::new();
        {
            let held = self.blocks.lock().unwrap_or_else(PoisonError::into_inner);
            for b in first..=last {
                if !held.blocks.contains_key(&b) {
                    missing.push(b);
                }
            }
        }
        // Neighbouring missing blocks in one request.
        let mut i = 0;
        while i < missing.len() {
            let mut j = i;
            while j + 1 < missing.len() && missing[j + 1] == missing[j] + 1 {
                j += 1;
            }
            let from = missing[i] * BLOCK;
            let to = ((missing[j] + 1) * BLOCK).min(self.size);
            let bytes =
                kentos_cloud::range::read(&self.url, from, to - from, self.version.as_deref())?;
            let mut held = self.blocks.lock().unwrap_or_else(PoisonError::into_inner);
            let tick = held.tick;
            for (k, b) in (missing[i]..=missing[j]).enumerate() {
                let a = k * BLOCK as usize;
                let e = (a + BLOCK as usize).min(bytes.len());
                held.blocks
                    .insert(b, (Arc::new(bytes[a..e].to_vec()), tick));
            }
            i = j + 1;
        }
        let mut held = self.blocks.lock().unwrap_or_else(PoisonError::into_inner);
        held.tick += 1;
        let tick = held.tick;
        let mut out = Vec::with_capacity(len as usize);
        for b in first..=last {
            let Some((block, used)) = held.blocks.get_mut(&b) else {
                return Err(format!("“{}” okunamadı.", self.url));
            };
            *used = tick;
            let start = b * BLOCK;
            let a = offset.max(start) - start;
            let e = end.min(start + block.len() as u64) - start;
            out.extend_from_slice(&block[a as usize..e as usize]);
        }
        // Let the least recently used go while too much is kept.
        let kept = held.blocks.len() as u64 * BLOCK;
        if kept > KEEP {
            let mut ages: Vec<(u64, u64)> =
                held.blocks.iter().map(|(k, (_, t))| (*t, *k)).collect();
            ages.sort_unstable();
            let drop = ((kept - KEEP) / BLOCK) as usize + 1;
            for (_, k) in ages.into_iter().take(drop) {
                held.blocks.remove(&k);
            }
        }
        Ok(out)
    }
}

/// A file of a cloud opened for reading.
#[derive(Debug)]
pub enum Bytes {
    File(File, u64),
    Remote(Remote),
    Memory(Arc<Vec<u8>>),
}

impl Bytes {
    pub fn size(&self) -> u64 {
        match self {
            Bytes::File(_, n) => *n,
            Bytes::Remote(r) => r.size,
            Bytes::Memory(b) => b.len() as u64,
        }
    }

    /// `len` bytes from `offset`.
    pub fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        match self {
            Bytes::File(f, size) => {
                if offset.checked_add(len).is_none_or(|e| e > *size) {
                    return Err("Nokta bulutu dosyası beklenenden kısa.".into());
                }
                crate::rasters::tiles::read_at(f, offset, len)
            }
            Bytes::Remote(r) => r.read(offset, len),
            Bytes::Memory(b) => {
                let a = usize::try_from(offset).map_err(|_| "Bayt dosyanın dışında.".to_owned())?;
                let e = a.saturating_add(len as usize);
                b.get(a..e)
                    .map(<[u8]>::to_vec)
                    .ok_or_else(|| "Bayt dosyanın dışında.".into())
            }
        }
    }

    /// A file on the disk.
    pub fn file(path: &Path) -> Result<Bytes, String> {
        let f = File::open(path).map_err(|e| format!("“{}” açılamadı: {e}.", path.display()))?;
        let n = f.metadata().map_err(|e| e.to_string())?.len();
        Ok(Bytes::File(f, n))
    }
}
