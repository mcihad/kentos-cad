//! TIFF and BigTIFF directories (docs/adr/0204 §1): the header, every image
//! file directory (IFD) of the chain and the tags a raster needs, read from
//! whatever bytes the host has handed over; a byte not yet handed over is a
//! [`Need`]. Unknown tags are skipped without reading their values.
//!
//! Limits (docs/adr/0009): at most `MAX_IFDS` directories, `MAX_ENTRIES`
//! entries each, `MAX_VALUE_BYTES` for one tag's values; a directory seen
//! twice ends the chain (no loop).

use super::{ByteStore, Need, RasterError, Step};

pub const MAX_IFDS: usize = 64;
pub const MAX_ENTRIES: u64 = 4096;
pub const MAX_VALUE_BYTES: u64 = 64 * 1024 * 1024;
/// How much is asked for at a time when a directory is needed.
const WINDOW: u64 = 64 * 1024;

// Tags.
pub const NEW_SUBFILE_TYPE: u16 = 254;
pub const IMAGE_WIDTH: u16 = 256;
pub const IMAGE_LENGTH: u16 = 257;
pub const BITS_PER_SAMPLE: u16 = 258;
pub const COMPRESSION: u16 = 259;
pub const PHOTOMETRIC: u16 = 262;
pub const STRIP_OFFSETS: u16 = 273;
pub const SAMPLES_PER_PIXEL: u16 = 277;
pub const ROWS_PER_STRIP: u16 = 278;
pub const STRIP_BYTE_COUNTS: u16 = 279;
pub const PLANAR_CONFIGURATION: u16 = 284;
pub const PREDICTOR: u16 = 317;
pub const COLOR_MAP: u16 = 320;
pub const TILE_WIDTH: u16 = 322;
pub const TILE_LENGTH: u16 = 323;
pub const TILE_OFFSETS: u16 = 324;
pub const TILE_BYTE_COUNTS: u16 = 325;
pub const EXTRA_SAMPLES: u16 = 338;
pub const SAMPLE_FORMAT: u16 = 339;
pub const JPEG_TABLES: u16 = 347;
pub const YCBCR_SUBSAMPLING: u16 = 530;
pub const MODEL_PIXEL_SCALE: u16 = 33550;
pub const MODEL_TIEPOINT: u16 = 33922;
pub const MODEL_TRANSFORMATION: u16 = 34264;
pub const GEO_KEY_DIRECTORY: u16 = 34735;
pub const GEO_DOUBLE_PARAMS: u16 = 34736;
pub const GEO_ASCII_PARAMS: u16 = 34737;
pub const GDAL_NODATA: u16 = 42113;

/// The tags read; the rest are skipped.
const WANTED: [u16; 28] = [
    NEW_SUBFILE_TYPE,
    IMAGE_WIDTH,
    IMAGE_LENGTH,
    BITS_PER_SAMPLE,
    COMPRESSION,
    PHOTOMETRIC,
    STRIP_OFFSETS,
    SAMPLES_PER_PIXEL,
    ROWS_PER_STRIP,
    STRIP_BYTE_COUNTS,
    PLANAR_CONFIGURATION,
    PREDICTOR,
    COLOR_MAP,
    TILE_WIDTH,
    TILE_LENGTH,
    TILE_OFFSETS,
    TILE_BYTE_COUNTS,
    EXTRA_SAMPLES,
    SAMPLE_FORMAT,
    JPEG_TABLES,
    YCBCR_SUBSAMPLING,
    MODEL_PIXEL_SCALE,
    MODEL_TIEPOINT,
    MODEL_TRANSFORMATION,
    GEO_KEY_DIRECTORY,
    GEO_DOUBLE_PARAMS,
    GEO_ASCII_PARAMS,
    GDAL_NODATA,
];

/// One directory's tags.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Ifd {
    pub subfile: u64,
    pub width: u64,
    pub height: u64,
    pub bits: Vec<u64>,
    pub compression: u64,
    pub photometric: Option<u64>,
    pub samples: u64,
    pub rows_per_strip: Option<u64>,
    pub planar: u64,
    pub predictor: u64,
    pub color_map: Vec<u64>,
    pub tile_width: Option<u64>,
    pub tile_length: Option<u64>,
    pub offsets: Vec<u64>,
    pub counts: Vec<u64>,
    pub extra: Vec<u64>,
    pub sample_format: Vec<u64>,
    pub jpeg_tables: Option<Vec<u8>>,
    pub ycbcr_subsampling: Vec<u64>,
    pub pixel_scale: Vec<f64>,
    pub tiepoints: Vec<f64>,
    pub transformation: Vec<f64>,
    pub geo_keys: Vec<u64>,
    pub geo_doubles: Vec<f64>,
    pub geo_ascii: String,
    pub gdal_nodata: Option<String>,
}

/// A file's header and directories.
#[derive(Clone, Debug, PartialEq)]
pub struct Tiff {
    pub little: bool,
    pub big: bool,
    pub ifds: Vec<Ifd>,
}

/// Whether the bytes begin a TIFF (classic or BigTIFF).
pub fn sniff(head: &[u8]) -> bool {
    matches!(
        head.get(..4),
        Some([0x49, 0x49, 42 | 43, 0]) | Some([0x4D, 0x4D, 0, 42 | 43])
    )
}

struct Bytes<'a> {
    store: &'a ByteStore,
    size: u64,
    little: bool,
}

/// What reading a run of bytes gave: them, or the bytes to ask for.
enum Got<'a> {
    Bytes(&'a [u8]),
    Need(Need),
}

impl<'a> Bytes<'a> {
    fn need(&self, offset: u64, len: u64) -> Need {
        let start = offset.min(self.size);
        let want = len.max(WINDOW);
        Need {
            offset: start,
            len: want
                .min(self.size.saturating_sub(start))
                .max(len.min(self.size.saturating_sub(start))),
        }
    }

    fn read(&self, offset: u64, len: u64) -> Result<Got<'a>, RasterError> {
        if offset.checked_add(len).is_none_or(|end| end > self.size) {
            return Err(RasterError::new(format!(
                "TIFF dosyası kesik: {offset}. bayttan {len} bayt okunacaktı ama dosya {} bayt.",
                self.size
            )));
        }
        match self.store.get(offset, len) {
            Some(b) => Ok(Got::Bytes(b)),
            None => Ok(Got::Need(self.need(offset, len))),
        }
    }

    fn u16(&self, b: &[u8]) -> u64 {
        let a = [b[0], b[1]];
        u64::from(if self.little {
            u16::from_le_bytes(a)
        } else {
            u16::from_be_bytes(a)
        })
    }

    fn u32(&self, b: &[u8]) -> u64 {
        let a = [b[0], b[1], b[2], b[3]];
        u64::from(if self.little {
            u32::from_le_bytes(a)
        } else {
            u32::from_be_bytes(a)
        })
    }

    fn u64(&self, b: &[u8]) -> u64 {
        let mut a = [0u8; 8];
        a.copy_from_slice(&b[..8]);
        if self.little {
            u64::from_le_bytes(a)
        } else {
            u64::from_be_bytes(a)
        }
    }
}

macro_rules! got {
    ($e:expr) => {
        match $e? {
            Got::Bytes(b) => b,
            Got::Need(n) => return Ok(Step::Need(n)),
        }
    };
}

/// A value's size in bytes by its field type; none for an unknown type.
fn type_size(kind: u64) -> Option<u64> {
    Some(match kind {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 | 13 => 4,
        5 | 10 | 12 | 16 | 17 | 18 => 8,
        _ => return None,
    })
}

/// A tag's values, as unsigned integers or floats or bytes.
enum Values {
    Ints(Vec<u64>),
    Floats(Vec<f64>),
    Bytes(Vec<u8>),
}

fn decode(b: &Bytes<'_>, kind: u64, count: u64, raw: &[u8]) -> Values {
    let n = usize::try_from(count).unwrap_or(0);
    match kind {
        1 | 6 => Values::Ints(raw.iter().take(n).map(|&x| u64::from(x)).collect()),
        2 | 7 => Values::Bytes(raw.iter().take(n).copied().collect()),
        3 | 8 => Values::Ints(raw.chunks_exact(2).take(n).map(|c| b.u16(c)).collect()),
        4 | 9 | 13 => Values::Ints(raw.chunks_exact(4).take(n).map(|c| b.u32(c)).collect()),
        16..=18 => Values::Ints(raw.chunks_exact(8).take(n).map(|c| b.u64(c)).collect()),
        5 | 10 => Values::Floats(
            raw.chunks_exact(8)
                .take(n)
                .map(|c| {
                    let (num, den) = (b.u32(&c[..4]), b.u32(&c[4..]));
                    if kind == 10 {
                        f64::from(num as u32 as i32) / f64::from(den as u32 as i32)
                    } else {
                        num as f64 / den as f64
                    }
                })
                .collect(),
        ),
        11 => Values::Floats(
            raw.chunks_exact(4)
                .take(n)
                .map(|c| f64::from(f32::from_bits(b.u32(c) as u32)))
                .collect(),
        ),
        12 => Values::Floats(
            raw.chunks_exact(8)
                .take(n)
                .map(|c| f64::from_bits(b.u64(c)))
                .collect(),
        ),
        _ => Values::Ints(Vec::new()),
    }
}

fn ints(v: Values) -> Vec<u64> {
    match v {
        Values::Ints(i) => i,
        Values::Floats(f) => f
            .into_iter()
            .map(|x| {
                if x.is_finite() && x >= 0.0 {
                    x as u64
                } else {
                    0
                }
            })
            .collect(),
        Values::Bytes(b) => b.into_iter().map(u64::from).collect(),
    }
}

fn floats(v: Values) -> Vec<f64> {
    match v {
        Values::Ints(i) => i.into_iter().map(|x| x as f64).collect(),
        Values::Floats(f) => f,
        Values::Bytes(b) => b.into_iter().map(f64::from).collect(),
    }
}

fn text(v: Values) -> String {
    match v {
        Values::Bytes(b) => {
            let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
            String::from_utf8_lossy(&b[..end]).into_owned()
        }
        _ => String::new(),
    }
}

fn bytes(v: Values) -> Vec<u8> {
    match v {
        Values::Bytes(b) => b,
        Values::Ints(i) => i.into_iter().map(|x| x as u8).collect(),
        Values::Floats(_) => Vec::new(),
    }
}

/// The file's header and directories from the bytes `store` holds, or the
/// bytes still needed; `size` is the file's length.
pub fn parse(store: &ByteStore, size: u64) -> Result<Step<Tiff>, RasterError> {
    let probe = Bytes {
        store,
        size,
        little: true,
    };
    if size < 8 {
        return Err(RasterError::new("Dosya TIFF olamayacak kadar kısa."));
    }
    let head = got!(probe.read(0, size.min(16)));
    let little = match head.get(..2) {
        Some([0x49, 0x49]) => true,
        Some([0x4D, 0x4D]) => false,
        _ => {
            return Err(RasterError::new(
                "Dosya TIFF değil: başı II ya da MM ile başlamıyor.",
            ));
        }
    };
    let b = Bytes {
        store,
        size,
        little,
    };
    let magic = b.u16(&head[2..4]);
    let big = match magic {
        42 => false,
        43 => {
            if head.len() < 16 || b.u16(&head[4..6]) != 8 {
                return Err(RasterError::new(
                    "BigTIFF başlığı bozuk: ofset boyu 8 değil.",
                ));
            }
            true
        }
        _ => {
            return Err(RasterError::new(
                "Dosya TIFF değil: başlıktaki sayı 42 ya da 43 değil.",
            ));
        }
    };
    let mut next = if big {
        b.u64(&head[8..16])
    } else {
        b.u32(&head[4..8])
    };
    let mut ifds = Vec::new();
    let mut seen = Vec::new();
    while next != 0 {
        if seen.contains(&next) {
            break;
        }
        if ifds.len() >= MAX_IFDS {
            return Err(RasterError::new(format!(
                "TIFF dosyasında {MAX_IFDS}'ten çok görüntü dizini var; okunmuyor."
            )));
        }
        seen.push(next);
        let count_size = if big { 8 } else { 2 };
        let raw = got!(b.read(next, count_size));
        let count = if big { b.u64(raw) } else { b.u16(raw) };
        if count == 0 || count > MAX_ENTRIES {
            return Err(RasterError::new(format!(
                "TIFF görüntü dizininin etiket sayısı {count}; 1 ile {MAX_ENTRIES} arasında olmalı."
            )));
        }
        let entry = if big { 20 } else { 12 };
        let start = next + count_size;
        let body = got!(b.read(start, count * entry + if big { 8 } else { 4 }));
        let mut ifd = Ifd {
            compression: 1,
            samples: 1,
            planar: 1,
            predictor: 1,
            ..Ifd::default()
        };
        for k in 0..count as usize {
            let e = &body[k * entry as usize..(k + 1) * entry as usize];
            let tag = b.u16(&e[..2]) as u16;
            if !WANTED.contains(&tag) {
                continue;
            }
            let kind = b.u16(&e[2..4]);
            let n = if big {
                b.u64(&e[4..12])
            } else {
                b.u32(&e[4..8])
            };
            let Some(size_of) = type_size(kind) else {
                continue;
            };
            let total = n
                .checked_mul(size_of)
                .filter(|&t| t <= MAX_VALUE_BYTES)
                .ok_or_else(|| {
                    RasterError::new(format!(
                        "TIFF etiketi {tag}'in değerleri çok büyük ({n} değer)."
                    ))
                })?;
            let inline = if big { 8 } else { 4 };
            let raw: &[u8] = if total <= inline {
                &e[if big { 12 } else { 8 }..][..total as usize]
            } else {
                let at = if big {
                    b.u64(&e[12..20])
                } else {
                    b.u32(&e[8..12])
                };
                got!(b.read(at, total))
            };
            let v = decode(&b, kind, n, raw);
            let first = |v: Values| ints(v).first().copied().unwrap_or(0);
            match tag {
                NEW_SUBFILE_TYPE => ifd.subfile = first(v),
                IMAGE_WIDTH => ifd.width = first(v),
                IMAGE_LENGTH => ifd.height = first(v),
                BITS_PER_SAMPLE => ifd.bits = ints(v),
                COMPRESSION => ifd.compression = first(v),
                PHOTOMETRIC => ifd.photometric = Some(first(v)),
                STRIP_OFFSETS | TILE_OFFSETS => ifd.offsets = ints(v),
                SAMPLES_PER_PIXEL => ifd.samples = first(v),
                ROWS_PER_STRIP => ifd.rows_per_strip = Some(first(v)),
                STRIP_BYTE_COUNTS | TILE_BYTE_COUNTS => ifd.counts = ints(v),
                PLANAR_CONFIGURATION => ifd.planar = first(v),
                PREDICTOR => ifd.predictor = first(v),
                COLOR_MAP => ifd.color_map = ints(v),
                TILE_WIDTH => ifd.tile_width = Some(first(v)),
                TILE_LENGTH => ifd.tile_length = Some(first(v)),
                EXTRA_SAMPLES => ifd.extra = ints(v),
                SAMPLE_FORMAT => ifd.sample_format = ints(v),
                JPEG_TABLES => ifd.jpeg_tables = Some(bytes(v)),
                YCBCR_SUBSAMPLING => ifd.ycbcr_subsampling = ints(v),
                MODEL_PIXEL_SCALE => ifd.pixel_scale = floats(v),
                MODEL_TIEPOINT => ifd.tiepoints = floats(v),
                MODEL_TRANSFORMATION => ifd.transformation = floats(v),
                GEO_KEY_DIRECTORY => ifd.geo_keys = ints(v),
                GEO_DOUBLE_PARAMS => ifd.geo_doubles = floats(v),
                GEO_ASCII_PARAMS => ifd.geo_ascii = text(v),
                GDAL_NODATA => ifd.gdal_nodata = Some(text(v)),
                _ => {}
            }
        }
        let tail = &body[(count * entry) as usize..];
        next = if big { b.u64(tail) } else { b.u32(tail) };
        ifds.push(ifd);
    }
    if ifds.is_empty() {
        return Err(RasterError::new("TIFF dosyasında görüntü dizini yok."));
    }
    Ok(Step::Done(Tiff { little, big, ifds }))
}
