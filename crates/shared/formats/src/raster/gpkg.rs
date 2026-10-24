//! A GeoPackage's tile tables as rasters (docs/adr/0627 §1–§3; plan/gis.md `CBS-03`): what a table holds (pictures,
//! or a gridded coverage's values, OGC 17-066r1), a tile's bytes as the reader's samples, the reader's levels over the
//! table's zooms, the window a raster linked to a table shows, and a coverage's float tile written. The file itself
//! (SQLite) is the host's: this core never reads it.
//!
//! A picture tile is a PNG of 8 bits, a JPEG (the host's decoder) or a WebP, any of them in one table (GDAL's AUTO
//! writes the opaque tiles as JPEG and the others as PNG); it is read as red, green, blue and alpha bytes, a grey tile
//! its grey in all three colours, a tile without alpha opaque. An integer coverage's tile is a 16-bit grey PNG, a
//! float coverage's a TIFF of one 32-bit float band; a value is (stored × the tile's scale + its offset) × the
//! coverage's scale + its offset, and a stored value equal to the null is no value.

use kentos_contracts::RasterSample;

use super::source::open_bytes;
use super::{RasterError, Samples, png, webp};

/// A zoom of a tile table, as the reader plans by it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Zoom {
    pub zoom: i64,
    pub matrix_width: i64,
    pub matrix_height: i64,
    pub tile_width: u32,
    pub tile_height: u32,
    /// A pixel's width and height in the system's units.
    pub pixel: [f64; 2],
}

/// What a gridded coverage's tiles' own scales and offsets allow (GDAL's checks): every tile's scale 1 and offset 0,
/// and every tile's scale 1 and offset 0 or 1 (GDAL's 16-bit coverage whose nodata is −32 768).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileScales {
    pub plain: bool,
    pub plain_or_one: bool,
}

/// A gridded coverage as the reader reads it: as GDAL 3.12's GPKG driver does (docs/adr/0627 §2), its band's type,
/// values and nodata, so that a table reads alike in KentOS, GDAL and QGIS.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Coverage {
    /// `integer` (16-bit PNG tiles); else `float` (32-bit float TIFF tiles).
    pub integer: bool,
    pub scale: f64,
    pub offset: f64,
    /// The null: a stored value of an integer coverage (a whole number 0 … 65 535; none for another, as GDAL ignores
    /// it), a value of a float one.
    pub null: Option<f64>,
    /// The smallest value that means something: an integer coverage read as floats has its values rounded to whole
    /// numbers when it is 1 (1 when the row gives none), as GDAL does.
    pub precision: f64,
    /// The band read: U16 for an integer coverage whose values are its stored ones (scale 1, offset 0, every tile's
    /// scale 1 and offset 0), I16 for one offset by −32 768 (every tile's offset 0, or 0 or 1 with the null 65 535)
    /// or by −32 767 with the null 65 535, F32 otherwise.
    pub sample: RasterSample,
}

impl Coverage {
    /// A coverage from its ancillary row and what its tiles' rows allow.
    pub fn new(
        integer: bool,
        scale: f64,
        offset: f64,
        null: Option<f64>,
        precision: Option<f64>,
        tiles: TileScales,
    ) -> Coverage {
        let null = if integer {
            null.filter(|n| (0.0..=65535.0).contains(n) && n.fract() == 0.0)
        } else {
            null
        };
        let high_null = null == Some(65535.0);
        // GDAL reads as signed what it writes for signed data: offset −32 768 (the tiles plain, or 65 535's tiles
        // at offset 1 too), or −32 767 with 65 535 as the null.
        let signed = (offset == -32768.0
            && (if high_null {
                tiles.plain_or_one
            } else {
                tiles.plain
            }))
            || (offset == -32767.0 && high_null && tiles.plain);
        let sample = if !integer || scale != 1.0 {
            RasterSample::F32
        } else if offset == 0.0 && tiles.plain {
            RasterSample::U16
        } else if signed {
            RasterSample::I16
        } else {
            RasterSample::F32
        };
        Coverage {
            integer,
            scale,
            offset,
            null,
            precision: precision.unwrap_or(1.0),
            sample,
        }
    }

    /// The band's nodata, GDAL's: an integer coverage's null as it is (U16; I16 when not above 32 767, else
    /// −32 768) or the lowest float (F32), a float coverage's null as a 32-bit float.
    pub fn nodata(&self) -> Option<f64> {
        let n = self.null?;
        Some(match (self.integer, self.sample) {
            (true, RasterSample::U16) => n,
            (true, RasterSample::I16) if n > 32767.0 => -32768.0,
            (true, RasterSample::I16) => n,
            (true, _) => f64::from(f32::MIN),
            (false, _) => f64::from(n as f32),
        })
    }

    /// A stored value of an integer coverage's tile (`scale` and `offset` the tile's own) as its band holds it.
    pub fn value(&self, stored: u16, scale: f64, offset: f64) -> f64 {
        if self.null == Some(f64::from(stored)) {
            return self.nodata().unwrap_or(0.0);
        }
        let v = (f64::from(stored) * scale + offset) * self.scale + self.offset;
        match self.sample {
            RasterSample::U16 => f64::from(stored),
            RasterSample::I16 => (v + 0.5).floor().clamp(-32768.0, 32767.0),
            _ if self.precision == 1.0 => f64::from(((v + 0.5).floor()) as f32),
            _ => f64::from(v as f32),
        }
    }
}

/// What a tile table holds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Content {
    /// Pictures: red, green, blue and alpha bytes.
    Image,
    /// A gridded coverage's values: one band.
    Coverage(Coverage),
}

impl Content {
    pub fn bands(&self) -> u32 {
        match self {
            Content::Image => 4,
            Content::Coverage(_) => 1,
        }
    }

    pub fn sample(&self) -> RasterSample {
        match self {
            Content::Image => RasterSample::U8,
            Content::Coverage(c) => c.sample,
        }
    }

    pub fn nodata(&self) -> Option<f64> {
        match self {
            Content::Image => None,
            Content::Coverage(c) => c.nodata(),
        }
    }
}

/// A tile's samples, or a JPEG picture for the host's decoder ([`from_jpeg`]).
#[derive(Clone, Debug, PartialEq)]
pub enum Decoded {
    Samples(Samples),
    Jpeg(Vec<u8>),
}

/// What a tile's first bytes say it is: `png`, `jpeg`, `webp`, `tiff` or `other`.
pub fn format(data: &[u8]) -> &'static str {
    if png::sniff(data) {
        "png"
    } else if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "jpeg"
    } else if webp::sniff(data) {
        "webp"
    } else if data.starts_with(b"II*\0") || data.starts_with(b"MM\0*") {
        "tiff"
    } else {
        "other"
    }
}

/// Why a tile is not read before its pixels are: its format is one the table's kind does not hold (`unsupported`),
/// or its header does not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TileProblem {
    pub unsupported: bool,
    pub why: String,
}

/// A tile's header: its format, and its size when the header says it here (a JPEG's is the host's decoder's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub format: &'static str,
    pub size: Option<(u32, u32)>,
}

/// A tile's format and size from its first bytes, checked against the table's kind (docs/adr/0627 §1): pictures are
/// 8-bit PNG, JPEG or WebP; an integer coverage's tiles 16-bit grey PNG; a float coverage's TIFF.
pub fn header(content: &Content, data: &[u8]) -> Result<Header, TileProblem> {
    let unsupported = |why: &str| TileProblem {
        unsupported: true,
        why: why.to_owned(),
    };
    let unreadable = |why: String| TileProblem {
        unsupported: false,
        why,
    };
    let f = format(data);
    let png_head = || -> Result<(u32, u32, u8, u8), TileProblem> {
        if data.len() < 26 || &data[12..16] != b"IHDR" {
            return Err(unreadable("PNG'nin başlığı (IHDR) okunmuyor".into()));
        }
        let w = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let h = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);
        Ok((w, h, data[24], data[25]))
    };
    match (content, f) {
        (_, "other") => Err(unreadable("PNG, JPEG, WebP ya da TIFF değil".into())),
        (Content::Image, "png") => {
            let (w, h, depth, _) = png_head()?;
            if depth > 8 {
                return Err(unsupported(
                    "8 bitten fazla bir PNG; resim karoları 8 bitlik PNG, JPEG ya da WebP olur",
                ));
            }
            Ok(Header {
                format: f,
                size: Some((w, h)),
            })
        }
        (Content::Image, "jpeg") => Ok(Header {
            format: f,
            size: None,
        }),
        (Content::Image, "webp") => {
            let d = image_webp::WebPDecoder::new(std::io::Cursor::new(data))
                .map_err(|_| unreadable("WebP'nin başlığı okunmuyor".into()))?;
            Ok(Header {
                format: f,
                size: Some(d.dimensions()),
            })
        }
        (Content::Image, _) => Err(unsupported(
            "TIFF; TIFF karo yalnız yükseklik (kapsam) tablosunda olur",
        )),
        (Content::Coverage(c), "png") if c.integer => {
            let (w, h, depth, color) = png_head()?;
            if depth != 16 || color != 0 {
                return Err(unsupported(
                    "tam sayılı kapsamın karosu 16 bitlik gri PNG olur",
                ));
            }
            Ok(Header {
                format: f,
                size: Some((w, h)),
            })
        }
        (Content::Coverage(c), _) if c.integer => Err(unsupported(
            "tam sayılı kapsamın karosu 16 bitlik gri PNG olur",
        )),
        (Content::Coverage(_), "tiff") => {
            let mut store = super::ByteStore::new();
            store.put(0, data.to_vec());
            match super::tiff::parse(&store, data.len() as u64) {
                Ok(super::Step::Done(t)) => match t.ifds.first() {
                    Some(d) => Ok(Header {
                        format: f,
                        size: Some((
                            u32::try_from(d.width).unwrap_or(u32::MAX),
                            u32::try_from(d.height).unwrap_or(u32::MAX),
                        )),
                    }),
                    None => Err(unreadable("TIFF'in görüntüsü yok".into())),
                },
                Ok(super::Step::Need(_)) | Err(_) => {
                    Err(unreadable("TIFF'in başlığı okunmuyor".into()))
                }
            }
        }
        (Content::Coverage(_), _) => Err(unsupported(
            "kayan noktalı kapsamın karosu 32 bitlik TIFF olur",
        )),
    }
}

fn wrong_size(w: u32, h: u32, width: u32, height: u32) -> RasterError {
    RasterError::new(format!(
        "karo {w} × {h} piksel, matrisin karo boyu {width} × {height}."
    ))
}

/// Interleaved 8-bit pixels of `channels` (1 grey, 2 grey and alpha, 3 RGB, 4 RGBA) as RGBA.
fn rgba(pixels: &[u8], channels: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(pixels.len() / channels.max(1) * 4);
    match channels {
        1 => pixels
            .iter()
            .for_each(|&g| out.extend_from_slice(&[g, g, g, 255])),
        2 => pixels
            .chunks_exact(2)
            .for_each(|c| out.extend_from_slice(&[c[0], c[0], c[0], c[1]])),
        3 => pixels
            .chunks_exact(3)
            .for_each(|c| out.extend_from_slice(&[c[0], c[1], c[2], 255])),
        _ => out.extend_from_slice(pixels),
    }
    out
}

/// A TIFF tile's one band of 32-bit floats: its width, height and values.
pub fn float_values(data: &[u8]) -> Result<(u32, u32, Vec<f32>), RasterError> {
    let mut r = open_bytes(data, None, 64 << 20)?;
    let (w, h) = (r.info.width, r.info.height);
    if r.info.bands != 1 || r.info.sample != RasterSample::F32 {
        return Err(RasterError::new(
            "karo tek bantlı 32 bitlik kayan noktalı TIFF değil.",
        ));
    }
    for need in r.needs(0, 0, 0, w, h) {
        let (a, b) = (need.offset as usize, (need.offset + need.len) as usize);
        let Some(bytes) = data.get(a..b) else {
            return Err(RasterError::new("TIFF karosu eksik."));
        };
        if !matches!(r.put_block(&need, bytes)?, super::source::Put::Done) {
            return Err(RasterError::new("TIFF karosu JPEG sıkıştırmalı; okunmaz."));
        }
    }
    match r.region(0, 0, 0, w, h).map(|reg| reg.samples) {
        Some(Samples::F32(v)) => Ok((w, h, v)),
        _ => Err(RasterError::new("TIFF karosu okunamadı.")),
    }
}

/// A tile's bytes as `width` × `height` samples of the table's content; `scale` and `offset` are a coverage tile's
/// own (1 and 0 otherwise).
pub fn decode(
    content: &Content,
    data: &[u8],
    width: u32,
    height: u32,
    scale: f64,
    offset: f64,
) -> Result<Decoded, RasterError> {
    let n = width as usize * height as usize;
    match content {
        Content::Image => match format(data) {
            "png" => {
                let p = png::decode(data)?;
                if (p.width, p.height) != (width, height) {
                    return Err(wrong_size(p.width, p.height, width, height));
                }
                match p.samples {
                    Samples::U8(v) if (1..=4).contains(&p.bands) => {
                        Ok(Decoded::Samples(Samples::U8(rgba(&v, p.bands as usize))))
                    }
                    _ => Err(RasterError::new(
                        "karo 8 bitten fazla bir PNG; resim karoları 8 bitlik PNG, JPEG ya da WebP olur.",
                    )),
                }
            }
            "webp" => {
                let w = webp::decode(data)?;
                if (w.width, w.height) != (width, height) {
                    return Err(wrong_size(w.width, w.height, width, height));
                }
                Ok(Decoded::Samples(Samples::U8(rgba(
                    &w.pixels,
                    w.channels as usize,
                ))))
            }
            "jpeg" => Ok(Decoded::Jpeg(data.to_vec())),
            "tiff" => Err(RasterError::new(
                "karo TIFF; TIFF karo yalnız yükseklik (kapsam) tablosunda okunur.",
            )),
            _ => Err(RasterError::new("karo PNG, JPEG ya da WebP değil.")),
        },
        Content::Coverage(c) if c.integer => {
            if format(data) != "png" {
                return Err(RasterError::new(
                    "tam sayılı kapsamın karosu PNG değil; 16 bitlik gri PNG olmalı.",
                ));
            }
            let p = png::decode(data)?;
            if (p.width, p.height) != (width, height) {
                return Err(wrong_size(p.width, p.height, width, height));
            }
            let Samples::U16(stored) = p.samples else {
                return Err(RasterError::new(
                    "tam sayılı kapsamın karosu 16 bitlik gri PNG değil.",
                ));
            };
            if p.bands != 1 || stored.len() != n {
                return Err(RasterError::new(
                    "tam sayılı kapsamın karosu tek bantlı değil.",
                ));
            }
            Ok(Decoded::Samples(match c.sample {
                RasterSample::U16 => Samples::U16(stored),
                RasterSample::I16 => Samples::I16(
                    stored
                        .iter()
                        .map(|&v| c.value(v, scale, offset) as i16)
                        .collect(),
                ),
                _ => Samples::F32(
                    stored
                        .iter()
                        .map(|&v| c.value(v, scale, offset) as f32)
                        .collect(),
                ),
            }))
        }
        Content::Coverage(_) => {
            if format(data) != "tiff" {
                return Err(RasterError::new(
                    "kayan noktalı kapsamın karosu TIFF değil.",
                ));
            }
            let (w, h, v) = float_values(data)?;
            if (w, h) != (width, height) {
                return Err(wrong_size(w, h, width, height));
            }
            // GDAL reads a float coverage's values as its tiles hold them: the scales and offsets, which are 1 and 0
            // for a float coverage, are not applied.
            Ok(Decoded::Samples(Samples::F32(v)))
        }
    }
}

/// A JPEG picture's pixels from the host's decoder (`components` 1 grey, 3 colour, 4 a browser's RGBA) as RGBA.
pub fn from_jpeg(
    pixels: &[u8],
    components: u32,
    width: u32,
    height: u32,
) -> Result<Samples, RasterError> {
    let n = width as usize * height as usize;
    if !matches!(components, 1 | 3 | 4) || pixels.len() != n * components as usize {
        return Err(RasterError::new(format!(
            "JPEG karosu {width} × {height} piksel değil ya da bileşenleri okunmuyor."
        )));
    }
    let mut v = rgba(pixels, components as usize);
    if components == 4 {
        // A JPEG is opaque.
        v.chunks_exact_mut(4).for_each(|p| p[3] = 255);
    }
    Ok(Samples::U8(v))
}

/// A tile the table does not hold: transparent, or the coverage's nodata (0 without one).
pub fn missing(content: &Content, width: u32, height: u32) -> Samples {
    let n = width as usize * height as usize * content.bands() as usize;
    Samples::filled(content.sample(), n, content.nodata().unwrap_or(0.0))
}

/// A coverage's float tile: a little-endian TIFF of one 32-bit float band in one strip, LZW-coded (what GDAL writes
/// and its validator holds a gridded coverage's float tile to). The same values make the same file.
pub fn float_tiff(width: u32, height: u32, values: &[f32]) -> Result<Vec<u8>, RasterError> {
    if width == 0 || height == 0 || values.len() != width as usize * height as usize {
        return Err(RasterError::new(
            "TIFF karosunun değerleri boyutuna uymuyor.",
        ));
    }
    let raw: Vec<u8> = values.iter().flat_map(|v| v.to_le_bytes()).collect();
    let coded = super::codec::lzw_encode(&raw);
    let entries: [(u16, u16, u32); 11] = [
        (256, 4, width),
        (257, 4, height),
        (258, 3, 32),
        (259, 3, 5),
        (262, 3, 1),
        (273, 4, 0),
        (277, 3, 1),
        (278, 4, height),
        (279, 4, coded.len() as u32),
        (284, 3, 1),
        (339, 3, 3),
    ];
    let data_at = 8 + 2 + entries.len() as u32 * 12 + 4;
    let mut out = Vec::with_capacity(data_at as usize + coded.len());
    out.extend_from_slice(b"II*\0");
    out.extend_from_slice(&8u32.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for (tag, kind, value) in entries {
        out.extend_from_slice(&tag.to_le_bytes());
        out.extend_from_slice(&kind.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes());
        let value = if tag == 273 { data_at } else { value };
        if kind == 3 {
            out.extend_from_slice(&(value as u16).to_le_bytes());
            out.extend_from_slice(&[0, 0]);
        } else {
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&coded);
    Ok(out)
}

/// A tile block's bytes as a host hands them to the reader (docs/adr/0627 §3): the tile's scale and offset (two
/// little-endian 64-bit floats; a coverage's tile ancillary row, 1 and 0 otherwise), then the tile's own bytes; none
/// after the sixteen when the table holds no such tile.
pub fn frame(scale: f64, offset: f64, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(16 + data.len());
    out.extend_from_slice(&scale.to_le_bytes());
    out.extend_from_slice(&offset.to_le_bytes());
    out.extend_from_slice(data);
    out
}

/// A framed tile block's scale, offset and tile bytes ([`frame`]).
pub fn unframe(bytes: &[u8]) -> Result<(f64, f64, &[u8]), RasterError> {
    if bytes.len() < 16 {
        return Err(RasterError::new(
            "Karo bloğu eksik: ölçeği ve ötelemesi yok.",
        ));
    }
    let f = |a: usize| {
        let mut b = [0u8; 8];
        b.copy_from_slice(&bytes[a..a + 8]);
        f64::from_le_bytes(b)
    };
    Ok((f(0), f(8), &bytes[16..]))
}

/// A tile block's place: its zoom, column and row in the table, and its width and height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TileAt {
    pub zoom: i64,
    pub column: i64,
    pub row: i64,
    pub width: u32,
    pub height: u32,
}

/// What a host needs to read and decode a tile table's blocks off the reader (docs/adr/0627 §3): each level's tiles
/// and the table's content.
#[derive(Clone, Debug, PartialEq)]
pub struct TileBlocks {
    pub content: Content,
    /// Each level of the reader: its tiles' level and the level's size; none for a level worked out here.
    pub levels: Vec<Option<(TileLevel, u32, u32)>>,
}

impl TileBlocks {
    /// A tile block's place in its table; none for another block.
    pub fn at(&self, need: &super::source::BlockNeed) -> Option<TileAt> {
        if need.file != super::source::TILES_FILE || need.member != 0 {
            return None;
        }
        let (t, w, h) = (*self.levels.get(usize::from(need.ifd))?)?;
        let (c0, r0, across, _) = t.span(w, h);
        let i = i64::from(need.index);
        Some(TileAt {
            zoom: t.zoom.zoom,
            column: c0 + i % across,
            row: r0 + i / across,
            width: t.zoom.tile_width,
            height: t.zoom.tile_height,
        })
    }

    /// A tile block's framed bytes ([`frame`]) decoded: its samples (the table's missing tile when it holds none), or
    /// a JPEG for the host's decoder.
    pub fn decode(
        &self,
        need: &super::source::BlockNeed,
        bytes: &[u8],
    ) -> Result<Decoded, RasterError> {
        let at = self
            .at(need)
            .ok_or_else(|| RasterError::new("Bu raster'in böyle bir karosu yok."))?;
        let (scale, offset, data) = unframe(bytes)?;
        if data.is_empty() {
            return Ok(Decoded::Samples(missing(
                &self.content,
                at.width,
                at.height,
            )));
        }
        if let Err(p) = header(&self.content, data) {
            return Err(RasterError::new(format!(
                "{}/{}/{} karosu okunmadı: {}",
                at.zoom, at.column, at.row, p.why
            )));
        }
        decode(&self.content, data, at.width, at.height, scale, offset).map_err(|e| {
            RasterError::new(format!(
                "{}/{}/{} karosu okunamadı: {}",
                at.zoom, at.column, at.row, e.0
            ))
        })
    }
}

/// A level read from a table's zoom: the level's pixel (0, 0) is the zoom's matrix pixel (`x`, `y`), counted from the
/// tile matrix set's top-left corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TileLevel {
    pub zoom: Zoom,
    pub x: i64,
    pub y: i64,
}

impl TileLevel {
    /// The first tile column and row a level of `width` × `height` pixels covers, and how many across and down.
    pub fn span(&self, width: u32, height: u32) -> (i64, i64, i64, i64) {
        let (tw, th) = (
            i64::from(self.zoom.tile_width),
            i64::from(self.zoom.tile_height),
        );
        let c0 = self.x.div_euclid(tw);
        let r0 = self.y.div_euclid(th);
        let c1 = (self.x + i64::from(width.max(1)) - 1).div_euclid(tw);
        let r1 = (self.y + i64::from(height.max(1)) - 1).div_euclid(th);
        (c0, r0, c1 - c0 + 1, r1 - r0 + 1)
    }
}

/// The zoom whose pixel is `f` times zoom `p`'s (relative difference within 1e-9 on both axes).
fn zoom_at(zooms: &[Zoom], p: [f64; 2], f: f64) -> Option<Zoom> {
    let near = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs();
    zooms
        .iter()
        .find(|z| near(z.pixel[0], p[0] * f) && near(z.pixel[1], p[1] * f))
        .copied()
}

/// The reader's levels over a table's zooms (docs/adr/0627 §3): level 0 is zoom `zoom0` from its pixel (`x`, `y`), a
/// coarser level `k` is the zoom whose pixel is 2ᵏ times zoom0's when `x` and `y` are whole multiples of 2ᵏ (its
/// pixels are then whole pixels of the level), otherwise none (worked out from the level below). One entry per level
/// of a `width` × `height` raster (the geometry core's halving to one tile).
pub fn levels(
    zooms: &[Zoom],
    zoom0: i64,
    x: i64,
    y: i64,
    width: u32,
    height: u32,
) -> Vec<Option<TileLevel>> {
    let sizes = super::source::level_sizes(width, height);
    let Some(z0) = zooms.iter().find(|z| z.zoom == zoom0).copied() else {
        return vec![None; sizes.len()];
    };
    (0..sizes.len())
        .map(|k| {
            if k == 0 {
                return Some(TileLevel { zoom: z0, x, y });
            }
            let f = (1i64 << k.min(40)) as f64;
            let step = 1i64 << k.min(40);
            if x.rem_euclid(step) != 0 || y.rem_euclid(step) != 0 {
                return None;
            }
            zoom_at(zooms, z0.pixel, f).map(|zoom| TileLevel {
                zoom,
                x: x / step,
                y: y / step,
            })
        })
        .collect()
}

/// The window a raster linked to a table shows (docs/adr/0627 §3): the data's whole pixels at zoom `zoom0` (from
/// pixel `x`, `y`, `width` × `height`), its first pixel moved up and left to a whole multiple of 2ᴷ, where K is the
/// coarsest zoom it can draw from (each zoom's pixel twice the one before, at most down to the level of one tile), its
/// width and height grown as much. A file GDAL wrote (its grid starting at the data) moves nothing.
pub fn linked_window(
    zooms: &[Zoom],
    zoom0: i64,
    x: i64,
    y: i64,
    width: u32,
    height: u32,
) -> (i64, i64, u32, u32) {
    let Some(z0) = zooms.iter().find(|z| z.zoom == zoom0).copied() else {
        return (x, y, width, height);
    };
    let n = super::source::level_sizes(width, height).len();
    let mut k = 0usize;
    while k + 1 < n && zoom_at(zooms, z0.pixel, (1i64 << (k + 1).min(40)) as f64).is_some() {
        k += 1;
    }
    let step = 1i64 << k.min(40);
    let (ax, ay) = (x - x.rem_euclid(step), y - y.rem_euclid(step));
    let grow = |v: u32, by: i64| u32::try_from(i64::from(v) + by).unwrap_or(u32::MAX);
    (ax, ay, grow(width, x - ax), grow(height, y - ay))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zoom(z: i64, side: i64, px: f64) -> Zoom {
        Zoom {
            zoom: z,
            matrix_width: side,
            matrix_height: side,
            tile_width: 256,
            tile_height: 256,
            pixel: [px, px],
        }
    }

    #[test]
    fn a_float_tile_reads_back_the_same_values() {
        let v: Vec<f32> = (0..256 * 256).map(|i| (i as f32) * 0.25 - 1000.5).collect();
        let t = float_tiff(256, 256, &v).unwrap();
        assert_eq!(format(&t), "tiff");
        let (w, h, back) = float_values(&t).unwrap();
        assert_eq!((w, h), (256, 256));
        assert_eq!(back, v);
        assert_eq!(float_tiff(256, 256, &v).unwrap(), t);
        assert!(float_tiff(3, 2, &v[..5]).is_err());
    }

    #[test]
    fn an_integer_coverage_reads_its_stored_values_scaled_and_its_null_as_nodata() {
        let stored: Vec<u16> = vec![0, 100, 65535, 40000];
        let p = png::encode16(2, 2, &stored, 6).unwrap();
        // Plain: the stored values themselves, its null as nodata.
        let plain = TileScales {
            plain: true,
            plain_or_one: true,
        };
        let c = Coverage::new(true, 1.0, 0.0, Some(65535.0), None, plain);
        assert_eq!(c.sample, RasterSample::U16);
        assert_eq!(c.nodata(), Some(65535.0));
        let Decoded::Samples(s) = decode(&Content::Coverage(c), &p, 2, 2, 1.0, 0.0).unwrap() else {
            panic!()
        };
        assert_eq!(s, Samples::U16(stored.clone()));
        // Offset −32768: the signed values; the null 65535 is −32768.
        let c = Coverage::new(true, 1.0, -32768.0, Some(65535.0), None, plain);
        assert_eq!(c.sample, RasterSample::I16);
        let Decoded::Samples(s) = decode(&Content::Coverage(c), &p, 2, 2, 1.0, 0.0).unwrap() else {
            panic!()
        };
        assert_eq!(s, Samples::I16(vec![-32768, -32668, -32768, 7232]));
        // Scaled tiles: floats, the null the lowest float; the precision 1 (also when not given) rounds the values
        // to whole numbers, another leaves them.
        let scaled = TileScales {
            plain: false,
            plain_or_one: false,
        };
        let c = Coverage::new(true, 2.0, 10.0, Some(65535.0), None, scaled);
        assert_eq!(c.sample, RasterSample::F32);
        let Decoded::Samples(s) = decode(&Content::Coverage(c), &p, 2, 2, 0.5, 100.3).unwrap()
        else {
            panic!()
        };
        assert_eq!(s, Samples::F32(vec![211.0, 311.0, f32::MIN, 40211.0]));
        let c = Coverage::new(true, 2.0, 10.0, Some(65535.0), Some(0.5), scaled);
        let Decoded::Samples(s) = decode(&Content::Coverage(c), &p, 2, 2, 0.5, 100.3).unwrap()
        else {
            panic!()
        };
        assert_eq!(s, Samples::F32(vec![210.6, 310.6, f32::MIN, 40210.6]));
        // GDAL's 16-bit coverage whose nodata is −32 768: tiles offset by 1 read as I16, the null −32 768.
        let one = TileScales {
            plain: false,
            plain_or_one: true,
        };
        let c = Coverage::new(true, 1.0, -32768.0, Some(65535.0), None, one);
        assert_eq!(c.sample, RasterSample::I16);
        let Decoded::Samples(s) = decode(&Content::Coverage(c), &p, 2, 2, 1.0, 1.0).unwrap() else {
            panic!()
        };
        assert_eq!(s, Samples::I16(vec![-32767, -32667, -32768, 7233]));
        assert_eq!(
            Coverage::new(true, 1.0, -32768.0, Some(100.0), None, one).sample,
            RasterSample::F32
        );
        assert_eq!(
            Coverage::new(true, 1.0, -32767.0, Some(65535.0), None, plain).sample,
            RasterSample::I16
        );
        // An I16 null not above 32 767 is the band's nodata as it is (GDAL's), a float null a 32-bit float, an
        // integer null that is no stored value none.
        assert_eq!(
            Coverage::new(true, 1.0, -32768.0, Some(100.0), None, plain).nodata(),
            Some(100.0)
        );
        assert_eq!(
            Coverage::new(false, 1.0, 0.0, Some(0.1), None, plain).nodata(),
            Some(f64::from(0.1f32))
        );
        assert_eq!(
            Coverage::new(true, 1.0, 0.0, Some(-5.0), None, plain).nodata(),
            None
        );
        assert_eq!(
            Coverage::new(true, 1.0, 0.0, Some(70000.0), None, plain).null,
            None
        );
    }

    #[test]
    fn pictures_read_as_rgba_whatever_their_format() {
        let grey = png::encode(2, 1, 1, &[10, 200], 6).unwrap();
        let Decoded::Samples(s) = decode(&Content::Image, &grey, 2, 1, 1.0, 0.0).unwrap() else {
            panic!()
        };
        assert_eq!(s, Samples::U8(vec![10, 10, 10, 255, 200, 200, 200, 255]));
        let w = webp::encode_lossless(2, 1, 4, &[1, 2, 3, 4, 5, 6, 7, 8]).unwrap();
        let Decoded::Samples(s) = decode(&Content::Image, &w, 2, 1, 1.0, 0.0).unwrap() else {
            panic!()
        };
        assert_eq!(s, Samples::U8(vec![1, 2, 3, 4, 5, 6, 7, 8]));
        assert!(matches!(
            decode(&Content::Image, &[0xFF, 0xD8, 0xFF, 0xE0], 2, 1, 1.0, 0.0),
            Ok(Decoded::Jpeg(_))
        ));
        assert_eq!(
            from_jpeg(&[9, 8, 7, 6, 5, 4], 3, 2, 1).unwrap(),
            Samples::U8(vec![9, 8, 7, 255, 6, 5, 4, 255])
        );
        assert!(decode(&Content::Image, &grey, 3, 1, 1.0, 0.0).is_err());
        assert!(decode(&Content::Image, b"GIF89a....", 2, 1, 1.0, 0.0).is_err());
    }

    #[test]
    fn levels_take_the_zooms_whose_pixels_are_whole_pixels_of_a_level() {
        let zooms = [
            zoom(0, 1, 8.0),
            zoom(1, 2, 4.0),
            zoom(2, 4, 2.0),
            zoom(3, 8, 1.0),
        ];
        // 2000 × 1000 pixels from (0, 0): levels 0…3 (2000, 1000, 500, 250), every one a zoom.
        let l = levels(&zooms, 3, 0, 0, 2000, 1000);
        assert_eq!(
            l.iter()
                .map(|t| t.map(|t| (t.zoom.zoom, t.x, t.y)))
                .collect::<Vec<_>>(),
            vec![
                Some((3, 0, 0)),
                Some((2, 0, 0)),
                Some((1, 0, 0)),
                Some((0, 0, 0))
            ]
        );
        // From (6, 4): level 1 is zoom 2 from (3, 2); level 2 would start at (1.5, 1): worked out.
        let l = levels(&zooms, 3, 6, 4, 2000, 1000);
        assert_eq!(l[1].map(|t| (t.zoom.zoom, t.x, t.y)), Some((2, 3, 2)));
        assert_eq!(l[2], None);
        assert_eq!(l[3], None);
        // The linked window moves (6, 4) to (0, 0) and grows: zooms down to level 3 are there.
        assert_eq!(
            linked_window(&zooms, 3, 6, 4, 2000, 1000),
            (0, 0, 2006, 1004)
        );
        // A zoom whose pixel is not twice the finer one's is no level.
        let odd = [zoom(0, 1, 3.0), zoom(1, 2, 1.0)];
        assert_eq!(levels(&odd, 1, 0, 0, 600, 600)[1], None);
        assert_eq!(linked_window(&odd, 1, 5, 5, 600, 600), (5, 5, 600, 600));
    }

    #[test]
    fn a_reader_reads_a_window_of_tiles_and_a_coarser_zoom_as_its_level() {
        use super::super::source::{Put, Reader};
        let small = |z: i64, side: i64, px: f64| Zoom {
            zoom: z,
            matrix_width: side,
            matrix_height: side,
            tile_width: 4,
            tile_height: 4,
            pixel: [px, px],
        };
        let zooms = [small(0, 1, 2.0), small(1, 2, 1.0)];
        // A matrix pixel's value: zoom 1 col·100 + row, zoom 0 7000 + col·100 + row.
        let tile = |z: i64, c: i64, r: i64| -> Vec<u8> {
            let v: Vec<f32> = (0..16)
                .map(|i| {
                    let (x, y) = (c * 4 + i % 4, r * 4 + i / 4);
                    (if z == 0 { 7000 } else { 0 } + x * 100 + y) as f32
                })
                .collect();
            float_tiff(4, 4, &v).unwrap()
        };
        let content = Content::Coverage(Coverage::new(
            false,
            1.0,
            0.0,
            None,
            None,
            TileScales {
                plain: true,
                plain_or_one: true,
            },
        ));
        // 300 × 300 so that there are two levels; the window from (2, 2) of zoom 1.
        let lv = levels(&zooms, 1, 2, 2, 300, 300);
        assert_eq!(lv[1].map(|t| (t.zoom.zoom, t.x, t.y)), Some((0, 1, 1)));
        let mut r = Reader::tiles(
            content,
            300,
            300,
            &lv,
            [0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
            None,
            false,
            1 << 20,
        )
        .unwrap();
        for level in 0..2 {
            for need in r.needs(level, 0, 0, 5, 4) {
                let (z, c, row, tw, th) = r.tile_at(&need).unwrap();
                let decoded = if c < 2 && row < 2 && (z == 1 || (c < 1 && row < 1)) {
                    decode(&content, &tile(z, c, row), tw, th, 1.0, 0.0).unwrap()
                } else {
                    Decoded::Samples(missing(&content, tw, th))
                };
                assert_eq!(r.keep_tile(&need, decoded), Put::Done);
            }
        }
        let reg = r.region(0, 0, 0, 5, 4).unwrap();
        assert_eq!(reg.at(0, 0, 0), 202.0);
        assert_eq!(reg.at(4, 3, 0), 605.0);
        // Level 1 is zoom 0 from its pixel (1, 1).
        let reg = r.region(1, 0, 0, 2, 2).unwrap();
        assert_eq!(reg.at(0, 0, 0), 7101.0);
        assert_eq!(reg.at(1, 1, 0), 7202.0);
        // Past the matrix (zoom 1 is 8 pixels a side): missing, 0.
        let needs = r.needs(0, 6, 6, 2, 2);
        for need in needs {
            let (_, _, _, tw, th) = r.tile_at(&need).unwrap();
            r.keep_tile(&need, Decoded::Samples(missing(&content, tw, th)));
        }
        assert_eq!(r.region(0, 6, 6, 1, 1).unwrap().at(0, 0, 0), 0.0);
    }

    #[test]
    fn a_level_spans_its_tiles() {
        let t = TileLevel {
            zoom: zoom(3, 8, 1.0),
            x: 300,
            y: 10,
        };
        // Pixels 300…1299 across: tiles 1…5; 10…509 down: tiles 0…1.
        assert_eq!(t.span(1000, 500), (1, 0, 5, 2));
    }
}
