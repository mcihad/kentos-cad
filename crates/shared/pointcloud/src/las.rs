//! LAS 1.0–1.4's header, VLRs and EVLRs (ASPRS LAS 1.4 R15, docs/adr/0207 §2).
//!
//! A header is read from its first 375 bytes (fewer when the file is
//! shorter); the VLRs lie between the header and the point data, the EVLRs
//! (1.4) from `evlr_start` to the end. Every number is checked against the
//! file's size and the bounds of §2 before anything is read from it.

use crate::bytes::{Read, Write};
use crate::record;
use crate::{PcError, Result};

/// The most VLRs and EVLRs a file may have.
pub const MAX_VLRS: u32 = 10_000;
/// The most bytes one VLR's or EVLR's payload may take.
pub const MAX_VLR_BYTES: u64 = 64 * 1024 * 1024;
/// The bytes of a 1.4 header (the most a reader looks at).
pub const HEADER_1_4: usize = 375;
/// The bytes of a VLR's and an EVLR's own header.
pub const VLR_HEAD: usize = 54;
pub const EVLR_HEAD: usize = 60;

/// Global encoding's bits.
pub const ENCODING_GPS_STANDARD: u16 = 1;
pub const ENCODING_WKT: u16 = 1 << 4;

/// A LAS file's header.
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub major: u8,
    pub minor: u8,
    pub source_id: u16,
    pub global_encoding: u16,
    pub guid: [u8; 16],
    pub system: String,
    pub software: String,
    pub day: u16,
    pub year: u16,
    pub header_size: u16,
    pub data_offset: u32,
    pub vlr_count: u32,
    /// The point data record format, 0–10 (LAZ's compression bits off).
    pub format: u8,
    /// Whether the format byte says the points are LAZ-compressed.
    pub compressed: bool,
    pub record_len: u16,
    /// The points (1.4's 64-bit count when given, else the legacy one).
    pub count: u64,
    /// Points by return 1 to 15 (legacy 1 to 5 before 1.4).
    pub by_return: [u64; 15],
    pub scale: [f64; 3],
    pub offset: [f64; 3],
    pub min: [f64; 3],
    pub max: [f64; 3],
    pub waveform_start: u64,
    pub evlr_start: u64,
    pub evlr_count: u32,
}

/// A variable length record (or an extended one).
#[derive(Clone, Debug, PartialEq)]
pub struct Vlr {
    pub user: String,
    pub record: u16,
    pub description: String,
    pub data: Vec<u8>,
    pub extended: bool,
}

impl Vlr {
    pub fn new(user: &str, record: u16, description: &str, data: Vec<u8>) -> Vlr {
        Vlr {
            user: user.to_owned(),
            record,
            description: description.to_owned(),
            data,
            extended: false,
        }
    }

    pub fn is(&self, user: &str, record: u16) -> bool {
        self.user == user && self.record == record
    }

    /// Its bytes as a VLR (54-byte head) or, when extended, an EVLR (60).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Write::new();
        w.u16(0).text(&self.user, 16).u16(self.record);
        if self.extended {
            w.u64(self.data.len() as u64);
        } else {
            w.u16(self.data.len().min(usize::from(u16::MAX)) as u16);
        }
        w.text(&self.description, 32).raw(&self.data);
        w.bytes
    }
}

impl Header {
    /// Whether the points carry GPS time as adjusted standard time.
    pub fn gps_standard(&self) -> bool {
        self.global_encoding & ENCODING_GPS_STANDARD != 0
    }

    /// The legacy point count field's value for these points (LAS 1.4 R15 §2.4).
    pub fn legacy_count(format: u8, count: u64) -> u32 {
        if format < 6 && count <= u64::from(u32::MAX) {
            count as u32
        } else {
            0
        }
    }

    /// The header's bytes for its version (227, 235 or 375 bytes and its own `header_size`).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Write::new();
        w.raw(b"LASF").u16(self.source_id).u16(self.global_encoding);
        w.raw(&self.guid);
        w.u8(self.major).u8(self.minor);
        w.text(&self.system, 32).text(&self.software, 32);
        w.u16(self.day).u16(self.year).u16(self.header_size);
        w.u32(self.data_offset).u32(self.vlr_count);
        w.u8(self.format | if self.compressed { 0x80 } else { 0 });
        w.u16(self.record_len);
        w.u32(Header::legacy_count(self.format, self.count));
        for r in 0..5 {
            w.u32(Header::legacy_count(self.format, self.by_return[r]));
        }
        for v in self.scale.iter().chain(&self.offset) {
            w.f64(*v);
        }
        for k in 0..3 {
            w.f64(self.max[k]).f64(self.min[k]);
        }
        if self.minor >= 3 {
            w.u64(self.waveform_start);
        }
        if self.minor >= 4 {
            w.u64(self.evlr_start).u32(self.evlr_count).u64(self.count);
            for r in self.by_return {
                w.u64(r);
            }
        }
        let size = usize::from(self.header_size);
        if w.bytes.len() < size {
            w.bytes.resize(size, 0);
        }
        w.bytes
    }
}

/// Whether bytes start a LAS file.
pub fn sniff(head: &[u8]) -> bool {
    head.starts_with(b"LASF")
}

/// A header from the file's first bytes (375, or the whole file when shorter), checked against `size`.
pub fn header(bytes: &[u8], size: u64) -> Result<Header> {
    if !sniff(bytes) {
        return Err(PcError::new(
            "Dosya bir LAS ya da LAZ değil: “LASF” ile başlamıyor.",
        ));
    }
    if bytes.len() < 227 {
        return Err(PcError::new("LAS başlığı eksik: dosya 227 bayttan kısa."));
    }
    let mut r = Read::new(bytes);
    r.seek(4);
    let source_id = r.u16()?;
    let global_encoding = r.u16()?;
    let mut guid = [0u8; 16];
    guid.copy_from_slice(r.take(16)?);
    let major = r.u8()?;
    let minor = r.u8()?;
    if major != 1 || minor > 4 {
        return Err(PcError::new(format!(
            "LAS {major}.{minor} okunmuyor; 1.0 ile 1.4 arası okunur."
        )));
    }
    let system = r.text(32)?;
    let software = r.text(32)?;
    let day = r.u16()?;
    let year = r.u16()?;
    let header_size = r.u16()?;
    let data_offset = r.u32()?;
    let vlr_count = r.u32()?;
    let format_byte = r.u8()?;
    let record_len = r.u16()?;
    let legacy_count = r.u32()?;
    let mut by_return = [0u64; 15];
    for b in by_return.iter_mut().take(5) {
        *b = u64::from(r.u32()?);
    }
    let mut scale = [0.0; 3];
    for v in &mut scale {
        *v = r.f64()?;
    }
    let mut offset = [0.0; 3];
    for v in &mut offset {
        *v = r.f64()?;
    }
    let (mut min, mut max) = ([0.0; 3], [0.0; 3]);
    for k in 0..3 {
        max[k] = r.f64()?;
        min[k] = r.f64()?;
    }
    let min_size = match minor {
        0..=2 => 227,
        3 => 235,
        _ => 375,
    };
    if usize::from(header_size) < min_size {
        return Err(PcError::new(format!(
            "LAS 1.{minor} başlığı en az {min_size} bayt olmalı; dosya {header_size} diyor."
        )));
    }
    let mut waveform_start = 0;
    let (mut evlr_start, mut evlr_count, mut count) = (0u64, 0u32, u64::from(legacy_count));
    if minor >= 3 {
        if bytes.len() < 235 {
            return Err(PcError::new("LAS başlığı eksik."));
        }
        waveform_start = r.u64()?;
    }
    if minor >= 4 {
        if bytes.len() < HEADER_1_4 {
            return Err(PcError::new(
                "LAS 1.4 başlığı eksik: dosya 375 bayttan kısa.",
            ));
        }
        evlr_start = r.u64()?;
        evlr_count = r.u32()?;
        let wide = r.u64()?;
        let mut wide_returns = [0u64; 15];
        for b in &mut wide_returns {
            *b = r.u64()?;
        }
        if wide > 0 || legacy_count == 0 {
            count = wide;
            by_return = wide_returns;
        }
    }
    // LAZ marks the format byte's high bit (older writers the next one too).
    let compressed = format_byte & 0x80 != 0;
    let format = format_byte & 0x3F;
    if format > 10 {
        return Err(PcError::new(format!(
            "Nokta kaydı biçimi {format} tanınmıyor; 0 ile 10 arası okunur."
        )));
    }
    let need = record::standard_len(format);
    if usize::from(record_len) < need || record_len > record::MAX_RECORD {
        return Err(PcError::new(format!(
            "Nokta kaydı {record_len} bayt; {format}. biçim en az {need}, en çok {} bayt olabilir.",
            record::MAX_RECORD
        )));
    }
    if scale.iter().any(|s| !(s.is_finite() && *s != 0.0)) || offset.iter().any(|o| !o.is_finite())
    {
        return Err(PcError::new(
            "Ölçek çarpanları sıfırdan farklı, ötelemeler sonlu sayılar olmalı.",
        ));
    }
    if vlr_count > MAX_VLRS || evlr_count > MAX_VLRS {
        return Err(PcError::new(format!(
            "Dosyada en çok {MAX_VLRS} VLR ve EVLR olabilir."
        )));
    }
    if u64::from(data_offset) < u64::from(header_size) || u64::from(data_offset) > size {
        return Err(PcError::new(
            "Nokta verisinin yeri başlıkla ya da dosyanın boyuyla uyuşmuyor.",
        ));
    }
    if !compressed {
        let end = u64::from(data_offset).checked_add(count.saturating_mul(u64::from(record_len)));
        if end.is_none_or(|e| e > size) {
            return Err(PcError::new(format!(
                "Dosya kısa: {count} nokta için yeri yok (kesik ya da bozuk LAS)."
            )));
        }
    }
    if evlr_count > 0 && (evlr_start < u64::from(data_offset) || evlr_start > size) {
        return Err(PcError::new("EVLR'lerin yeri dosyanın dışında."));
    }
    Ok(Header {
        major,
        minor,
        source_id,
        global_encoding,
        guid,
        system,
        software,
        day,
        year,
        header_size,
        data_offset,
        vlr_count,
        format,
        compressed,
        record_len,
        count,
        by_return,
        scale,
        offset,
        min,
        max,
        waveform_start,
        evlr_start,
        evlr_count,
    })
}

/// The VLRs from the bytes between the header and the point data (`bytes` starts at `header_size`).
pub fn vlrs(head: &Header, bytes: &[u8]) -> Result<Vec<Vlr>> {
    let mut r = Read::new(bytes);
    let mut out = Vec::with_capacity(head.vlr_count as usize);
    for i in 0..head.vlr_count {
        if bytes.len() - r.at() < VLR_HEAD {
            return Err(PcError::new(format!(
                "{}. VLR başlıkla nokta verisi arasına sığmıyor.",
                i + 1
            )));
        }
        r.u16()?;
        let user = r.text(16)?;
        let record = r.u16()?;
        let len = r.u16()?;
        let description = r.text(32)?;
        let data = r
            .take(usize::from(len))
            .map_err(|_| PcError::new(format!("{}. VLR ({user}) kısa.", i + 1)))?
            .to_vec();
        out.push(Vlr {
            user,
            record,
            description,
            data,
            extended: false,
        });
    }
    Ok(out)
}

/// The EVLRs from the file's bytes from `evlr_start` to its end.
pub fn evlrs(head: &Header, bytes: &[u8]) -> Result<Vec<Vlr>> {
    let mut r = Read::new(bytes);
    let mut out = Vec::new();
    for i in 0..head.evlr_count {
        if bytes.len() - r.at() < EVLR_HEAD {
            return Err(PcError::new(format!("{}. EVLR kısa.", i + 1)));
        }
        r.u16()?;
        let user = r.text(16)?;
        let record = r.u16()?;
        let len = r.u64()?;
        let description = r.text(32)?;
        if len > MAX_VLR_BYTES {
            return Err(PcError::new(format!(
                "{}. EVLR ({user}) {} MB; en çok {} MB okunur.",
                i + 1,
                len >> 20,
                MAX_VLR_BYTES >> 20
            )));
        }
        let data = r
            .take(len as usize)
            .map_err(|_| PcError::new(format!("{}. EVLR ({user}) dosyaya sığmıyor.", i + 1)))?
            .to_vec();
        out.push(Vlr {
            user,
            record,
            description,
            data,
            extended: true,
        });
    }
    Ok(out)
}
