//! Reading an HDF5 file's metadata (docs/adr/0645 §2): ranges through the host's [`Fetch`] in 64 KB pages kept for
//! the opening, and a cursor over a structure's bytes with the file's sizes of offsets and lengths.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use super::super::chunked::Fetch;

const PAGE: u64 = 64 << 10;
/// Pages kept while a file opens.
const PAGES_MOST: usize = 4096;

/// An HDF5 file being opened.
pub struct H5 {
    fetch: Arc<dyn Fetch>,
    pub size: u64,
    /// Bytes of an offset (an address) and of a length.
    pub o: usize,
    pub l: usize,
    /// Where addresses count from.
    pub base: u64,
    pages: RefCell<HashMap<u64, Arc<Vec<u8>>>>,
}

impl H5 {
    pub fn new(fetch: Arc<dyn Fetch>, size: u64) -> H5 {
        H5 {
            fetch,
            size,
            o: 8,
            l: 8,
            base: 0,
            pages: RefCell::new(HashMap::new()),
        }
    }

    pub fn fetch(&self) -> Arc<dyn Fetch> {
        self.fetch.clone()
    }

    fn page(&self, p: u64) -> Result<Arc<Vec<u8>>, String> {
        if let Some(b) = self.pages.borrow().get(&p) {
            return Ok(b.clone());
        }
        let start = p * PAGE;
        if start >= self.size {
            return Err("HDF5 dosyası beklenenden kısa.".into());
        }
        let b = Arc::new(self.fetch.range(start, PAGE.min(self.size - start))?);
        let mut pages = self.pages.borrow_mut();
        if pages.len() >= PAGES_MOST {
            pages.clear();
        }
        pages.insert(p, b.clone());
        Ok(b)
    }

    /// `n` bytes at the absolute place `at`.
    pub fn read(&self, at: u64, n: usize) -> Result<Vec<u8>, String> {
        let end = at
            .checked_add(n as u64)
            .filter(|&e| e <= self.size)
            .ok_or("HDF5 dosyasında bir adres dosyanın dışını gösteriyor.")?;
        if n as u64 > 4 * PAGE {
            return self.fetch.range(at, n as u64);
        }
        let mut out = Vec::with_capacity(n);
        let mut pos = at;
        while pos < end {
            let p = pos / PAGE;
            let page = self.page(p)?;
            let from = (pos - p * PAGE) as usize;
            let to = ((end - p * PAGE) as usize).min(page.len());
            if from >= to {
                return Err("HDF5 dosyası beklenenden kısa.".into());
            }
            out.extend_from_slice(&page[from..to]);
            pos = p * PAGE + to as u64;
        }
        Ok(out)
    }

    /// A cursor over `n` bytes at the address `addr` (relative to the base).
    pub fn at(&self, addr: u64, n: usize) -> Result<Cur, String> {
        let abs = self
            .base
            .checked_add(addr)
            .ok_or("HDF5 dosyasında bozuk bir adres var.")?;
        let n = n.min(self.size.saturating_sub(abs) as usize);
        Ok(Cur::new(self.read(abs, n)?, self.o, self.l))
    }

    /// The address `addr` as a place in the file.
    pub fn abs(&self, addr: u64) -> u64 {
        self.base.saturating_add(addr)
    }
}

/// A cursor over a structure's bytes.
#[derive(Clone, Debug)]
pub struct Cur {
    pub b: Vec<u8>,
    pub at: usize,
    o: usize,
    l: usize,
}

fn short() -> String {
    "HDF5 dosyasında bir yapı eksik ya da bozuk.".into()
}

impl Cur {
    pub fn new(b: Vec<u8>, o: usize, l: usize) -> Cur {
        Cur { b, at: 0, o, l }
    }

    pub fn sizes(&self) -> (usize, usize) {
        (self.o, self.l)
    }

    pub fn left(&self) -> usize {
        self.b.len().saturating_sub(self.at)
    }

    pub fn bytes(&mut self, n: usize) -> Result<&[u8], String> {
        let end = self.at.checked_add(n).ok_or_else(short)?;
        let s = self.b.get(self.at..end).ok_or_else(short)?;
        self.at = end;
        Ok(s)
    }

    pub fn skip(&mut self, n: usize) -> Result<(), String> {
        self.bytes(n).map(|_| ())
    }

    pub fn u8(&mut self) -> Result<u8, String> {
        Ok(self.bytes(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16, String> {
        let b = self.bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u32(&mut self) -> Result<u32, String> {
        let b = self.bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn u64(&mut self) -> Result<u64, String> {
        self.uvar(8)
    }

    /// An unsigned number of `n` bytes, little-endian.
    pub fn uvar(&mut self, n: usize) -> Result<u64, String> {
        let b = self.bytes(n)?;
        let mut v = 0u64;
        for (i, &x) in b.iter().enumerate().take(8) {
            v |= u64::from(x) << (8 * i);
        }
        Ok(v)
    }

    /// An address; none when undefined (all bits set).
    pub fn addr(&mut self) -> Result<Option<u64>, String> {
        let n = self.o;
        let v = self.uvar(n)?;
        let undefined = if n >= 8 {
            u64::MAX
        } else {
            (1u64 << (8 * n)) - 1
        };
        Ok((v != undefined).then_some(v))
    }

    /// A length.
    pub fn length(&mut self) -> Result<u64, String> {
        let n = self.l;
        self.uvar(n)
    }

    /// Checks a four-byte signature.
    pub fn sig(&mut self, s: &[u8; 4], what: &str) -> Result<(), String> {
        if self.bytes(4)? != s {
            return Err(format!(
                "HDF5 dosyasında {what} beklenen yerde değil; dosya bozuk olabilir."
            ));
        }
        Ok(())
    }

    /// The rest of the bytes from here.
    pub fn rest(&self) -> &[u8] {
        self.b.get(self.at..).unwrap_or(&[])
    }
}

/// Bytes needed for a number up to `n` (HDF5's `H5VM_limit_enc_size`).
pub fn limit_enc_size(n: u64) -> usize {
    (log2(n) / 8 + 1) as usize
}

/// ⌊log₂ n⌋ (0 for 0).
pub fn log2(n: u64) -> u32 {
    if n == 0 { 0 } else { 63 - n.leading_zeros() }
}
