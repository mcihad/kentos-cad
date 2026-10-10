//! NetCDF classic files (docs/adr/0243 §2), as Unidata's “NetCDF Classic
//! and 64-bit Offset Format” and CDF-5 notes describe them: the header (the
//! magic `CDF` and its version, the record count, the dimensions, the global
//! attributes, the variables) and where each variable's values lie. Version
//! 1 has 32-bit offsets, 2 64-bit offsets, 5 64-bit counts and sizes and the
//! unsigned and 64-bit types. Everything is big-endian; names and values are
//! padded to four bytes; the record variables' slabs are interleaved record
//! after record (a single record variable without padding).
//!
//! The header is read from growing prefixes of the file ([`parse`] says how
//! many bytes it wants); no fault of a file panics the reader. NetCDF-4
//! (HDF5) and GRIB files are told apart and refused with a way out.

use crate::raster::{ByteStore, Need, RasterError, Step};

/// The most bytes a header may take.
pub const MAX_HEADER: u64 = 64 << 20;
/// The first prefix asked for.
const FIRST_READ: u64 = 64 << 10;
const MAX_DIMS: u64 = 1024;
const MAX_VARS: u64 = 8192;
const MAX_ATTRS: u64 = 4096;
const MAX_VAR_DIMS: u64 = 32;
const MAX_NAME: u64 = 256;
const MAX_ATTR_BYTES: u64 = 16 << 20;

const NC_DIMENSION: i32 = 10;
const NC_VARIABLE: i32 = 11;
const NC_ATTRIBUTE: i32 = 12;
/// A record count that says “as many as the file holds”.
const STREAMING: u64 = 0xFFFF_FFFF;

/// Why a NetCDF-4 file is not read, and the way out.
pub const HDF5_REFUSED: &str = "Bu NetCDF-4 (HDF5) dosyası; KentOS klasik NetCDF okur. `nccopy -k cdf5 girdi.nc çıktı.nc` ya da `cdo -f nc5 copy girdi.nc çıktı.nc` ile çevirin.";

/// What a file's first bytes say it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sniff {
    /// Classic NetCDF of version 1, 2 or 5.
    Classic(u8),
    /// NetCDF-4 (an HDF5 file).
    Hdf5,
    Grib,
    Other,
}

/// What a file's first bytes (at least eight) say it is.
pub fn sniff(head: &[u8]) -> Sniff {
    match head {
        [b'C', b'D', b'F', v @ (1 | 2 | 5), ..] => Sniff::Classic(*v),
        [0x89, b'H', b'D', b'F', b'\r', b'\n', 0x1A, b'\n', ..] => Sniff::Hdf5,
        [b'G', b'R', b'I', b'B', ..] => Sniff::Grib,
        _ => Sniff::Other,
    }
}

/// A value's type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NcType {
    Byte,
    Char,
    Short,
    Int,
    Float,
    Double,
    UByte,
    UShort,
    UInt,
    Int64,
    UInt64,
}

impl NcType {
    fn of(code: i32, version: u8) -> Option<NcType> {
        Some(match code {
            1 => NcType::Byte,
            2 => NcType::Char,
            3 => NcType::Short,
            4 => NcType::Int,
            5 => NcType::Float,
            6 => NcType::Double,
            7 if version == 5 => NcType::UByte,
            8 if version == 5 => NcType::UShort,
            9 if version == 5 => NcType::UInt,
            10 if version == 5 => NcType::Int64,
            11 if version == 5 => NcType::UInt64,
            _ => return None,
        })
    }

    /// The type's code in a header.
    pub fn code(self) -> i32 {
        match self {
            NcType::Byte => 1,
            NcType::Char => 2,
            NcType::Short => 3,
            NcType::Int => 4,
            NcType::Float => 5,
            NcType::Double => 6,
            NcType::UByte => 7,
            NcType::UShort => 8,
            NcType::UInt => 9,
            NcType::Int64 => 10,
            NcType::UInt64 => 11,
        }
    }

    /// Bytes a value takes.
    pub fn size(self) -> u64 {
        match self {
            NcType::Byte | NcType::Char | NcType::UByte => 1,
            NcType::Short | NcType::UShort => 2,
            NcType::Int | NcType::Float | NcType::UInt => 4,
            NcType::Double | NcType::Int64 | NcType::UInt64 => 8,
        }
    }

    /// The name the window says.
    pub fn name(self) -> &'static str {
        match self {
            NcType::Byte => "byte",
            NcType::Char => "char",
            NcType::Short => "short",
            NcType::Int => "int",
            NcType::Float => "float",
            NcType::Double => "double",
            NcType::UByte => "ubyte",
            NcType::UShort => "ushort",
            NcType::UInt => "uint",
            NcType::Int64 => "int64",
            NcType::UInt64 => "uint64",
        }
    }

    /// Whether values are whole numbers.
    pub fn integer(self) -> bool {
        !matches!(self, NcType::Float | NcType::Double | NcType::Char)
    }

    /// NetCDF's default fill value of the type (the library fills unwritten values with it).
    pub fn default_fill(self) -> f64 {
        match self {
            NcType::Byte => -127.0,
            NcType::Char => 0.0,
            NcType::Short => -32767.0,
            NcType::Int => -2_147_483_647.0,
            NcType::Float => f64::from(9.969_21e36_f32),
            NcType::Double => 9.969_209_968_386_869e36,
            NcType::UByte => 255.0,
            NcType::UShort => 65535.0,
            NcType::UInt => 4_294_967_295.0,
            NcType::Int64 => -9_223_372_036_854_775_806.0,
            NcType::UInt64 => 18_446_744_073_709_551_614.0,
        }
    }
}

/// An attribute's value.
#[derive(Clone, Debug, PartialEq)]
pub enum Attr {
    Text(String),
    Numbers { kind: NcType, values: Vec<f64> },
}

impl Attr {
    pub fn text(&self) -> Option<&str> {
        match self {
            Attr::Text(s) => Some(s.trim_end_matches('\0')),
            Attr::Numbers { .. } => None,
        }
    }

    /// Its numbers (none for text).
    pub fn numbers(&self) -> &[f64] {
        match self {
            Attr::Numbers { values, .. } => values,
            Attr::Text(_) => &[],
        }
    }

    /// Its first number.
    pub fn number(&self) -> Option<f64> {
        self.numbers().first().copied()
    }

    /// The numbers' type (none for text).
    pub fn kind(&self) -> Option<NcType> {
        match self {
            Attr::Numbers { kind, .. } => Some(*kind),
            Attr::Text(_) => None,
        }
    }
}

/// A dimension; `len` 0 the record (unlimited) dimension.
#[derive(Clone, Debug, PartialEq)]
pub struct Dim {
    pub name: String,
    pub len: u64,
}

/// A variable: its dimensions (ids), attributes, type and where its values begin.
#[derive(Clone, Debug, PartialEq)]
pub struct Var {
    pub name: String,
    pub dims: Vec<usize>,
    pub attrs: Vec<(String, Attr)>,
    pub kind: NcType,
    pub begin: u64,
}

impl Var {
    pub fn attr(&self, name: &str) -> Option<&Attr> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, a)| a)
    }

    /// A text attribute's text.
    pub fn text(&self, name: &str) -> Option<&str> {
        self.attr(name).and_then(Attr::text)
    }
}

/// A file's header.
#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub version: u8,
    pub numrecs: u64,
    pub dims: Vec<Dim>,
    pub attrs: Vec<(String, Attr)>,
    pub vars: Vec<Var>,
    /// The record dimension's id.
    pub record_dim: Option<usize>,
    /// Bytes a record takes (the record variables' slabs).
    pub record_size: u64,
    /// The file's size.
    pub size: u64,
}

impl Header {
    pub fn var(&self, name: &str) -> Option<&Var> {
        self.vars.iter().find(|v| v.name == name)
    }

    /// A global attribute.
    pub fn attr(&self, name: &str) -> Option<&Attr> {
        self.attrs.iter().find(|(k, _)| k == name).map(|(_, a)| a)
    }

    pub fn is_record(&self, v: &Var) -> bool {
        self.record_dim.is_some() && v.dims.first().copied() == self.record_dim
    }

    /// A dimension's length (the record dimension's: the records).
    pub fn dim_len(&self, d: usize) -> u64 {
        if Some(d) == self.record_dim {
            self.numrecs
        } else {
            self.dims.get(d).map_or(0, |x| x.len)
        }
    }

    /// The variable's shape.
    pub fn shape(&self, v: &Var) -> Vec<u64> {
        v.dims.iter().map(|&d| self.dim_len(d)).collect()
    }

    /// Values of a record's slab (of the whole variable when it is not a record one).
    pub fn slab_len(&self, v: &Var) -> u64 {
        let skip = usize::from(self.is_record(v));
        v.dims[skip..]
            .iter()
            .map(|&d| self.dims.get(d).map_or(0, |x| x.len))
            .product()
    }

    /// The bytes a run of `count` values from the value at `index` (one per
    /// dimension, row-major) takes, when they lie in one record's slab.
    pub fn run(&self, v: &Var, index: &[u64], count: u64) -> Option<Need> {
        if index.len() != v.dims.len() {
            return None;
        }
        let shape = self.shape(v);
        let record = self.is_record(v);
        let (first, rest) = if record {
            (index.first().copied().unwrap_or(0), 1)
        } else {
            (0, 0)
        };
        let mut flat: u64 = 0;
        for k in rest..index.len() {
            if index[k] >= shape[k] {
                return None;
            }
            flat = flat.checked_mul(shape[k])?.checked_add(index[k])?;
        }
        if flat.checked_add(count)? > self.slab_len(v) {
            return None;
        }
        if record && first >= self.numrecs {
            return None;
        }
        let at = v
            .begin
            .checked_add(first.checked_mul(self.record_size)?)?
            .checked_add(flat.checked_mul(v.kind.size())?)?;
        Some(Need {
            offset: at,
            len: count.checked_mul(v.kind.size())?,
        })
    }

    /// The runs holding every value of the variable, in order (a record variable's one a record).
    pub fn runs(&self, v: &Var) -> Vec<Need> {
        let len = self.slab_len(v) * v.kind.size();
        if self.is_record(v) {
            (0..self.numrecs)
                .map(|r| Need {
                    offset: v.begin + r * self.record_size,
                    len,
                })
                .collect()
        } else {
            vec![Need {
                offset: v.begin,
                len,
            }]
        }
    }
}

/// Values of `kind` from big-endian bytes, as floats (64-bit integers past 2^53 rounded).
pub fn decode(kind: NcType, bytes: &[u8], out: &mut Vec<f64>) {
    let n = (bytes.len() as u64 / kind.size()) as usize;
    out.reserve(n);
    match kind {
        NcType::Byte => out.extend(bytes.iter().map(|&b| f64::from(b as i8))),
        NcType::Char | NcType::UByte => out.extend(bytes.iter().map(|&b| f64::from(b))),
        NcType::Short => out.extend(
            bytes
                .chunks_exact(2)
                .map(|c| f64::from(i16::from_be_bytes([c[0], c[1]]))),
        ),
        NcType::UShort => out.extend(
            bytes
                .chunks_exact(2)
                .map(|c| f64::from(u16::from_be_bytes([c[0], c[1]]))),
        ),
        NcType::Int => out.extend(
            bytes
                .chunks_exact(4)
                .map(|c| f64::from(i32::from_be_bytes([c[0], c[1], c[2], c[3]]))),
        ),
        NcType::UInt => out.extend(
            bytes
                .chunks_exact(4)
                .map(|c| f64::from(u32::from_be_bytes([c[0], c[1], c[2], c[3]]))),
        ),
        NcType::Float => out.extend(
            bytes
                .chunks_exact(4)
                .map(|c| f64::from(f32::from_be_bytes([c[0], c[1], c[2], c[3]]))),
        ),
        NcType::Double => out.extend(
            bytes
                .chunks_exact(8)
                .map(|c| f64::from_be_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]])),
        ),
        NcType::Int64 => {
            out.extend(bytes.chunks_exact(8).map(|c| {
                i64::from_be_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f64
            }))
        }
        NcType::UInt64 => {
            out.extend(bytes.chunks_exact(8).map(|c| {
                u64::from_be_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]]) as f64
            }))
        }
    }
}

/// The header from the bytes handed over; or the prefix still needed.
pub fn parse(store: &ByteStore, size: u64) -> Result<Step<Header>, RasterError> {
    let have = store.prefix();
    match read_header(have, size) {
        Ok(h) => Ok(Step::Done(h)),
        Err(Fault::Short) => {
            let got = have.len() as u64;
            if got >= size {
                return Err(RasterError::new(
                    "NetCDF dosyası kesik: başlığı bitmeden dosya bitiyor.",
                ));
            }
            if got >= MAX_HEADER {
                return Err(RasterError::new(
                    "NetCDF dosyasının başlığı 64 MB'tan büyük; okunmuyor.",
                ));
            }
            let want = (got * 2).max(FIRST_READ).min(size).min(MAX_HEADER);
            Ok(Step::Need(Need {
                offset: 0,
                len: want,
            }))
        }
        Err(Fault::Bad(e)) => Err(RasterError::new(e)),
    }
}

/// A header read from a whole file's bytes (tests, small files).
pub fn parse_bytes(bytes: &[u8]) -> Result<Header, RasterError> {
    match read_header(bytes, bytes.len() as u64) {
        Ok(h) => Ok(h),
        Err(Fault::Short) => Err(RasterError::new(
            "NetCDF dosyası kesik: başlığı bitmeden dosya bitiyor.",
        )),
        Err(Fault::Bad(e)) => Err(RasterError::new(e)),
    }
}

enum Fault {
    /// More of the file is needed.
    Short,
    Bad(String),
}

fn bad<T>(why: impl Into<String>) -> Result<T, Fault> {
    Err(Fault::Bad(why.into()))
}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
    version: u8,
}

impl Cursor<'_> {
    fn take(&mut self, n: u64) -> Result<&[u8], Fault> {
        let n = usize::try_from(n)
            .map_err(|_| Fault::Bad("NetCDF başlığında çok büyük bir sayı var.".into()))?;
        let end = self.at.checked_add(n).ok_or(Fault::Short)?;
        let s = self.bytes.get(self.at..end).ok_or(Fault::Short)?;
        self.at = end;
        Ok(s)
    }

    fn i32(&mut self) -> Result<i32, Fault> {
        let b = self.take(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u32(&mut self) -> Result<u32, Fault> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> Result<u64, Fault> {
        let b = self.take(8)?;
        Ok(u64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    /// A count or length: 32 bits, 64 in CDF-5; negative refused.
    fn count(&mut self) -> Result<u64, Fault> {
        if self.version == 5 {
            let v = self.u64()?;
            if v > i64::MAX as u64 {
                return bad("NetCDF başlığında eksi bir sayı var.");
            }
            Ok(v)
        } else {
            let v = self.i32()?;
            u64::try_from(v).or_else(|_| bad("NetCDF başlığında eksi bir sayı var."))
        }
    }

    fn name(&mut self) -> Result<String, Fault> {
        let n = self.count()?;
        if n == 0 || n > MAX_NAME {
            return bad(format!(
                "NetCDF başlığında {n} baytlık bir ad var; ad 1–{MAX_NAME} bayt olur."
            ));
        }
        let raw = self.take(n)?.to_vec();
        self.take(pad(n) - n)?;
        String::from_utf8(raw).or_else(|_| bad("NetCDF başlığında UTF-8 olmayan bir ad var."))
    }

    fn attrs(&mut self) -> Result<Vec<(String, Attr)>, Fault> {
        let tag = self.i32()?;
        let n = self.count()?;
        if tag == 0 && n == 0 {
            return Ok(Vec::new());
        }
        if tag != NC_ATTRIBUTE {
            return bad("NetCDF başlığı bozuk: öznitelik listesi beklenirken başka bir şey var.");
        }
        if n > MAX_ATTRS {
            return bad(format!(
                "NetCDF başlığında {n} öznitelik var; en çok {MAX_ATTRS} okunur."
            ));
        }
        let mut out = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let name = self.name()?;
            let code = self.i32()?;
            let Some(kind) = NcType::of(code, self.version) else {
                return bad(format!(
                    "NetCDF başlığında bilinmeyen bir tür ({code}) var."
                ));
            };
            let m = self.count()?;
            let bytes = m.checked_mul(kind.size()).filter(|&b| b <= MAX_ATTR_BYTES);
            let Some(bytes) = bytes else {
                return bad(format!(
                    "NetCDF'in “{name}” özniteliği 16 MB'tan büyük; okunmuyor."
                ));
            };
            let raw = self.take(bytes)?.to_vec();
            self.take(pad(bytes) - bytes)?;
            let value = if kind == NcType::Char {
                Attr::Text(String::from_utf8_lossy(&raw).into_owned())
            } else {
                let mut values = Vec::new();
                decode(kind, &raw, &mut values);
                Attr::Numbers { kind, values }
            };
            out.push((name, value));
        }
        Ok(out)
    }
}

/// `n` rounded up to four.
pub fn pad(n: u64) -> u64 {
    n.div_ceil(4) * 4
}

fn read_header(bytes: &[u8], size: u64) -> Result<Header, Fault> {
    if bytes.len() < 4 {
        return if (bytes.len() as u64) < size.min(4) {
            Err(Fault::Short)
        } else {
            bad("Dosya NetCDF değil.")
        };
    }
    let version = match sniff(bytes) {
        Sniff::Classic(v) => v,
        Sniff::Hdf5 => return bad(HDF5_REFUSED),
        Sniff::Grib => {
            return bad(
                "Bu bir GRIB dosyası; KentOS klasik NetCDF okur. `cdo -f nc5 copy girdi.grib çıktı.nc` ile çevirin.",
            );
        }
        Sniff::Other => return bad("Dosya NetCDF değil."),
    };
    let mut c = Cursor {
        bytes,
        at: 4,
        version,
    };
    let numrecs = if version == 5 {
        let v = c.u64()?;
        if v == u64::MAX { STREAMING } else { v }
    } else {
        u64::from(c.u32()?)
    };
    // Dimensions.
    let tag = c.i32()?;
    let n = c.count()?;
    let mut dims = Vec::new();
    if !(tag == 0 && n == 0) {
        if tag != NC_DIMENSION {
            return bad("NetCDF başlığı bozuk: boyut listesi beklenirken başka bir şey var.");
        }
        if n > MAX_DIMS {
            return bad(format!(
                "NetCDF dosyasında {n} boyut var; en çok {MAX_DIMS} okunur."
            ));
        }
        for _ in 0..n {
            let name = c.name()?;
            let len = c.count()?;
            dims.push(Dim { name, len });
        }
    }
    let records: Vec<usize> = (0..dims.len()).filter(|&d| dims[d].len == 0).collect();
    if records.len() > 1 {
        return bad("NetCDF başlığı bozuk: birden çok kayıt boyutu var.");
    }
    let record_dim = records.first().copied();
    let attrs = c.attrs()?;
    // Variables.
    let tag = c.i32()?;
    let n = c.count()?;
    let mut vars = Vec::new();
    if !(tag == 0 && n == 0) {
        if tag != NC_VARIABLE {
            return bad("NetCDF başlığı bozuk: değişken listesi beklenirken başka bir şey var.");
        }
        if n > MAX_VARS {
            return bad(format!(
                "NetCDF dosyasında {n} değişken var; en çok {MAX_VARS} okunur."
            ));
        }
        for _ in 0..n {
            let name = c.name()?;
            let nd = c.count()?;
            if nd > MAX_VAR_DIMS {
                return bad(format!(
                    "NetCDF'in “{name}” değişkeninin {nd} boyutu var; en çok {MAX_VAR_DIMS} okunur."
                ));
            }
            let mut vd = Vec::with_capacity(nd as usize);
            for k in 0..nd {
                let id = c.count()?;
                let Some(id) = usize::try_from(id).ok().filter(|&i| i < dims.len()) else {
                    return bad(format!(
                        "NetCDF'in “{name}” değişkeni olmayan bir boyutu anıyor."
                    ));
                };
                if Some(id) == record_dim && k != 0 {
                    return bad(format!(
                        "NetCDF'in “{name}” değişkeninde kayıt boyutu ilk boyut değil."
                    ));
                }
                vd.push(id);
            }
            let vattrs = c.attrs()?;
            let code = c.i32()?;
            let Some(kind) = NcType::of(code, version) else {
                return bad(format!(
                    "NetCDF'in “{name}” değişkeninin türü ({code}) bilinmiyor."
                ));
            };
            // vsize is worked out below. Its 32-bit field is unsigned: a variable of
            // 2–4 GiB fills its top bit and one past 4 GiB has 2³² − 1 (Unidata's
            // format notes; netCDF-C writes them so), so it is skipped unread.
            if version == 5 {
                c.u64()?;
            } else {
                c.u32()?;
            }
            let begin = if version == 1 {
                u64::from(c.u32()?)
            } else {
                let b = c.u64()?;
                if b > i64::MAX as u64 {
                    return bad("NetCDF başlığında eksi bir ofset var.");
                }
                b
            };
            vars.push(Var {
                name,
                dims: vd,
                attrs: vattrs,
                kind,
                begin,
            });
        }
    }
    let mut header = Header {
        version,
        numrecs,
        dims,
        attrs,
        vars,
        record_dim,
        record_size: 0,
        size,
    };
    // A record's bytes: the record variables' slabs, each padded to four,
    // without padding when there is one record variable.
    let slab_bytes = |h: &Header, v: &Var| -> Option<u64> {
        let skip = usize::from(h.is_record(v));
        v.dims[skip..]
            .iter()
            .try_fold(1u64, |a, &d| a.checked_mul(h.dims[d].len))?
            .checked_mul(v.kind.size())
    };
    let mut record_vars = 0;
    let mut record_size = 0u64;
    for v in &header.vars {
        let Some(b) = slab_bytes(&header, v) else {
            return bad(format!(
                "NetCDF'in “{}” değişkeni okunamayacak kadar büyük.",
                v.name
            ));
        };
        if header.is_record(v) {
            record_vars += 1;
            record_size = record_size.saturating_add(pad(b));
        }
    }
    if record_vars == 1 {
        record_size = header
            .vars
            .iter()
            .find(|v| header.is_record(v))
            .and_then(|v| slab_bytes(&header, v))
            .unwrap_or(0);
    }
    header.record_size = record_size;
    if header.numrecs == STREAMING {
        let first = header
            .vars
            .iter()
            .filter(|v| header.is_record(v))
            .map(|v| v.begin)
            .min();
        header.numrecs = match first {
            Some(b) if record_size > 0 && size > b => (size - b) / record_size,
            _ => 0,
        };
    }
    for v in &header.vars {
        let Some(b) = slab_bytes(&header, v) else {
            return bad(format!(
                "NetCDF'in “{}” değişkeni okunamayacak kadar büyük.",
                v.name
            ));
        };
        let end = if header.is_record(v) {
            match header.numrecs {
                0 => Some(v.begin),
                r => (r - 1)
                    .checked_mul(header.record_size)
                    .and_then(|x| x.checked_add(v.begin))
                    .and_then(|x| x.checked_add(b)),
            }
        } else {
            v.begin.checked_add(b)
        };
        if end.is_none_or(|e| e > size) {
            return bad(format!(
                "NetCDF dosyası kesik: “{}” değişkeninin değerleri dosyanın dışına uzanıyor.",
                v.name
            ));
        }
    }
    Ok(header)
}
