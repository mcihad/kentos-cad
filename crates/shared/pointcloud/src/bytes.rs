//! Little-endian numbers in and out of byte slices: reads past the end are
//! faults, never panics.

use crate::{PcError, Result};

/// A cursor over bytes that reads little-endian numbers.
#[derive(Clone, Copy, Debug)]
pub struct Read<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Read<'a> {
    pub fn new(bytes: &'a [u8]) -> Read<'a> {
        Read { bytes, at: 0 }
    }

    pub fn at(&self) -> usize {
        self.at
    }

    pub fn seek(&mut self, at: usize) {
        self.at = at;
    }

    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).filter(|&e| e <= self.bytes.len());
        match end {
            Some(e) => {
                let s = &self.bytes[self.at..e];
                self.at = e;
                Ok(s)
            }
            None => Err(PcError::new("Dosya beklenenden kısa.")),
        }
    }

    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    pub fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }

    pub fn u64(&mut self) -> Result<u64> {
        let b = self.take(8)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }

    pub fn i64(&mut self) -> Result<i64> {
        Ok(self.u64()? as i64)
    }

    pub fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_bits(self.u64()?))
    }

    /// A fixed-length text field: up to its first zero, invalid UTF-8 shown with replacements.
    pub fn text(&mut self, n: usize) -> Result<String> {
        let b = self.take(n)?;
        let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
        Ok(String::from_utf8_lossy(&b[..end]).trim_end().to_owned())
    }
}

#[inline]
pub fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

#[inline]
pub fn i16_at(b: &[u8], at: usize) -> i16 {
    i16::from_le_bytes([b[at], b[at + 1]])
}

#[inline]
pub fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

#[inline]
pub fn i32_at(b: &[u8], at: usize) -> i32 {
    i32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

#[inline]
pub fn f64_at(b: &[u8], at: usize) -> f64 {
    let mut a = [0u8; 8];
    a.copy_from_slice(&b[at..at + 8]);
    f64::from_le_bytes(a)
}

#[inline]
pub fn put_u16(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

#[inline]
pub fn put_i16(b: &mut [u8], at: usize, v: i16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

#[inline]
pub fn put_u32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

#[inline]
pub fn put_i32(b: &mut [u8], at: usize, v: i32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

#[inline]
pub fn put_u64(b: &mut [u8], at: usize, v: u64) {
    b[at..at + 8].copy_from_slice(&v.to_le_bytes());
}

#[inline]
pub fn put_i64(b: &mut [u8], at: usize, v: i64) {
    b[at..at + 8].copy_from_slice(&v.to_le_bytes());
}

#[inline]
pub fn put_f64(b: &mut [u8], at: usize, v: f64) {
    b[at..at + 8].copy_from_slice(&v.to_le_bytes());
}

/// Bytes written in order, little-endian.
#[derive(Clone, Debug, Default)]
pub struct Write {
    pub bytes: Vec<u8>,
}

impl Write {
    pub fn new() -> Write {
        Write::default()
    }

    pub fn u8(&mut self, v: u8) -> &mut Write {
        self.bytes.push(v);
        self
    }

    pub fn u16(&mut self, v: u16) -> &mut Write {
        self.bytes.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn u32(&mut self, v: u32) -> &mut Write {
        self.bytes.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn i32(&mut self, v: i32) -> &mut Write {
        self.bytes.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn u64(&mut self, v: u64) -> &mut Write {
        self.bytes.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn i64(&mut self, v: i64) -> &mut Write {
        self.bytes.extend_from_slice(&v.to_le_bytes());
        self
    }

    pub fn f64(&mut self, v: f64) -> &mut Write {
        self.bytes.extend_from_slice(&v.to_le_bytes());
        self
    }

    /// A fixed-length text field, cut or padded with zeros.
    pub fn text(&mut self, s: &str, n: usize) -> &mut Write {
        let b = s.as_bytes();
        let k = b.len().min(n);
        self.bytes.extend_from_slice(&b[..k]);
        self.bytes.extend(std::iter::repeat_n(0u8, n - k));
        self
    }

    pub fn raw(&mut self, b: &[u8]) -> &mut Write {
        self.bytes.extend_from_slice(b);
        self
    }
}
