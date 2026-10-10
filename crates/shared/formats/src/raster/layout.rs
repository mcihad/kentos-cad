//! How a TIFF directory's blocks lie and how one block's bytes become
//! samples (docs/adr/0204 §1): the sample type of its bits and format, the
//! checks that refuse what is not read (with why), and the block decoded:
//! LZW, Deflate or PackBits undone, the predictor undone, white-is-zero grey
//! turned round; a JPEG block handed back as a stream for the host.

use kentos_contracts::RasterSample;

use super::tiff::Ifd;
use super::{RasterError, Samples, codec};

/// How a TIFF directory's blocks lie.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub file: u8,
    pub ifd: u16,
    pub width: u32,
    pub height: u32,
    pub block_w: u32,
    pub block_h: u32,
    pub across: u32,
    pub down: u32,
    /// Bands a block holds: all (chunky) or one (planar).
    pub per_block: u32,
    pub planar: bool,
    pub bands: u32,
    pub sample: RasterSample,
    pub compression: u64,
    pub predictor: u64,
    pub white_is_zero: bool,
    pub little: bool,
    pub offsets: Vec<u64>,
    pub counts: Vec<u64>,
    pub jpeg_tables: Option<Vec<u8>>,
    /// A NetCDF variable's blocks (docs/adr/0243 §5): their stored type,
    /// unpacking and whether their rows run south to north.
    pub cube: Option<CubeDecode>,
}

/// How a NetCDF block's bytes become samples: big-endian values of the
/// stored type, unpacked (packed values, fills and the invalid made
/// nothing), the rows turned round when the file runs them south to north.
#[derive(Clone, Debug, PartialEq)]
pub struct CubeDecode {
    pub unpack: crate::multidim::cf::Unpack,
    pub flip: bool,
}

impl Layout {
    /// The block at column `bx`, row `by` of band `band` (planar) as an index.
    pub fn index(&self, bx: u32, by: u32, band: u32) -> u32 {
        let per_plane = self.across * self.down;
        (if self.planar { band * per_plane } else { 0 }) + by * self.across + bx
    }

    /// The rows block row `by` holds (a last strip is shorter).
    fn rows(&self, by: u32) -> u32 {
        self.block_h.min(self.height - by * self.block_h)
    }

    /// The bytes its decoded block `index` takes.
    fn expected(&self, index: u32) -> usize {
        if let Some(c) = &self.cube {
            let rows = self.rows(index % self.down.max(1));
            return self.block_w as usize * rows as usize * c.unpack.raw.size() as usize;
        }
        let per_plane = self.across * self.down;
        let by = (index % per_plane) / self.across;
        let rows = if self.across == 1 && self.block_w == self.width {
            self.rows(by)
        } else {
            self.block_h
        };
        self.block_w as usize * rows as usize * self.per_block as usize * self.sample.bytes()
    }
}

/// The sample type of a directory's bits and format.
fn sample_of(bits: u64, format: u64) -> Option<RasterSample> {
    Some(match (format, bits) {
        (1, 8) => RasterSample::U8,
        (1, 16) => RasterSample::U16,
        (1, 32) => RasterSample::U32,
        (2, 8) => RasterSample::I8,
        (2, 16) => RasterSample::I16,
        (2, 32) => RasterSample::I32,
        (3, 32) => RasterSample::F32,
        (3, 64) => RasterSample::F64,
        _ => return None,
    })
}

/// A compression's name, as the window says it; none when not read.
pub fn compression_name(c: u64) -> Option<&'static str> {
    Some(match c {
        1 => "Sıkıştırmasız",
        5 => "LZW",
        7 => "JPEG",
        8 | 32946 => "Deflate",
        32773 => "PackBits",
        _ => return None,
    })
}

/// A directory's layout, or why it is not read.
pub fn layout(file: u8, ifd_index: usize, ifd: &Ifd, little: bool) -> Result<Layout, RasterError> {
    let bands = u32::try_from(ifd.samples).unwrap_or(0);
    if !(1..=255).contains(&bands) {
        return Err(RasterError::new(format!(
            "TIFF'in bant sayısı ({}) okunmuyor.",
            ifd.samples
        )));
    }
    let bits = ifd.bits.first().copied().unwrap_or(1);
    if ifd.bits.iter().any(|&b| b != bits) {
        return Err(RasterError::new(
            "TIFF'in bantları farklı bit derinliklerinde; okunmuyor.",
        ));
    }
    let format = ifd.sample_format.first().copied().unwrap_or(1);
    let sample = sample_of(bits, format).ok_or_else(|| {
        RasterError::new(format!(
            "TIFF'in örnekleri {bits} bit{}: 8, 16, 32 bit tam sayı ve 32, 64 bit kayan nokta okunur.",
            if format == 3 { " kayan nokta" } else { "" }
        ))
    })?;
    if compression_name(ifd.compression).is_none() {
        return Err(RasterError::new(format!(
            "TIFF'in sıkıştırması ({}) okunmuyor: sıkıştırmasız, LZW, Deflate, PackBits ve JPEG okunur.",
            ifd.compression
        )));
    }
    let photometric = ifd.photometric.unwrap_or(1);
    if !matches!(photometric, 0..=3 | 6) {
        return Err(RasterError::new(format!(
            "TIFF'in renk düzeni ({photometric}) okunmuyor: gri, RGB, palet ve JPEG'le YCbCr okunur."
        )));
    }
    if photometric == 6 && ifd.compression != 7 {
        return Err(RasterError::new(
            "YCbCr renkli TIFF yalnız JPEG sıkıştırmasıyla okunur.",
        ));
    }
    if ifd.compression == 7 && sample != RasterSample::U8 {
        return Err(RasterError::new(
            "JPEG sıkıştırmalı TIFF yalnız 8 bitse okunur.",
        ));
    }
    if !matches!(ifd.predictor, 1..=3) {
        return Err(RasterError::new(format!(
            "TIFF'in öngörücüsü ({}) okunmuyor.",
            ifd.predictor
        )));
    }
    if ifd.predictor == 3 && !sample.float() || ifd.predictor == 2 && sample.float() {
        return Err(RasterError::new("TIFF'in öngörücüsü örnek türüne uymuyor."));
    }
    let (width, height) = (
        u32::try_from(ifd.width).unwrap_or(0),
        u32::try_from(ifd.height).unwrap_or(0),
    );
    if width == 0
        || height == 0
        || width > kentos_contracts::MAX_RASTER_SIDE
        || height > kentos_contracts::MAX_RASTER_SIDE
    {
        return Err(RasterError::new(format!(
            "TIFF'in boyu ({} × {}) okunmuyor.",
            ifd.width, ifd.height
        )));
    }
    let planar = ifd.planar == 2 && bands > 1;
    let (block_w, block_h) = match (ifd.tile_width, ifd.tile_length) {
        (Some(w), Some(h)) if w > 0 && h > 0 => {
            (u32::try_from(w).unwrap_or(0), u32::try_from(h).unwrap_or(0))
        }
        _ => {
            let rps = ifd
                .rows_per_strip
                .unwrap_or(ifd.height)
                .clamp(1, ifd.height);
            (width, u32::try_from(rps).unwrap_or(height))
        }
    };
    if block_w == 0 || block_h == 0 || u64::from(block_w) * u64::from(block_h) > 64 * 1024 * 1024 {
        return Err(RasterError::new("TIFF'in blok boyu okunmuyor."));
    }
    let across = width.div_ceil(block_w);
    let down = height.div_ceil(block_h);
    let blocks = u64::from(across) * u64::from(down) * if planar { u64::from(bands) } else { 1 };
    if ifd.offsets.len() as u64 != blocks || ifd.counts.len() as u64 != blocks {
        return Err(RasterError::new(format!(
            "TIFF bozuk: {blocks} blok olmalı, dizinde {} ofset ve {} boy var.",
            ifd.offsets.len(),
            ifd.counts.len()
        )));
    }
    let _ = ifd_index;
    Ok(Layout {
        file,
        ifd: u16::try_from(ifd_index).unwrap_or(u16::MAX),
        width,
        height,
        block_w,
        block_h,
        across,
        down,
        per_block: if planar { 1 } else { bands },
        planar,
        bands,
        sample,
        compression: ifd.compression,
        predictor: ifd.predictor,
        white_is_zero: photometric == 0,
        little,
        offsets: ifd.offsets.clone(),
        counts: ifd.counts.clone(),
        jpeg_tables: ifd.jpeg_tables.clone(),
        cube: None,
    })
}

/// A block's bytes made samples, or its JPEG stream.
pub fn decode_block(l: &Layout, index: u32, bytes: &[u8]) -> Result<Put2, RasterError> {
    let expected = l.expected(index);
    if let Some(c) = &l.cube {
        return Ok(Put2::Samples(decode_cube(l, c, index, bytes, expected)));
    }
    let mut buf = match l.compression {
        1 => {
            let mut v = bytes
                .get(..expected.min(bytes.len()))
                .unwrap_or(&[])
                .to_vec();
            v.resize(expected, 0);
            v
        }
        5 => codec::lzw(bytes, expected)?,
        8 | 32946 => codec::inflate(bytes, expected)?,
        32773 => codec::packbits(bytes, expected),
        7 => {
            return Ok(Put2::Jpeg(codec::jpeg_stream(
                l.jpeg_tables.as_deref(),
                bytes,
            )));
        }
        c => {
            return Err(RasterError::new(format!(
                "TIFF'in sıkıştırması ({c}) okunmuyor."
            )));
        }
    };
    buf.resize(expected, 0);
    let row = l.block_w as usize * l.per_block as usize;
    let mut little = l.little;
    match l.predictor {
        2 => codec::unpredict_horizontal(
            &mut buf,
            row,
            l.per_block as usize,
            l.sample.bytes(),
            l.little,
        ),
        3 => {
            codec::unpredict_float(&mut buf, row, l.per_block as usize, l.sample.bytes());
            little = false;
        }
        _ => {}
    }
    let mut samples = Samples::from_bytes(l.sample, &buf, little);
    if l.white_is_zero {
        match &mut samples {
            Samples::U8(v) => v.iter_mut().for_each(|x| *x = 255 - *x),
            Samples::U16(v) => v.iter_mut().for_each(|x| *x = 65535 - *x),
            _ => {}
        }
    }
    Ok(Put2::Samples(samples))
}

/// A NetCDF block's samples (`expected` stored bytes; short bytes read as nothing).
fn decode_cube(l: &Layout, c: &CubeDecode, index: u32, bytes: &[u8], expected: usize) -> Samples {
    let mut raw = Vec::new();
    crate::multidim::netcdf::decode(
        c.unpack.raw,
        bytes.get(..expected.min(bytes.len())).unwrap_or(&[]),
        &mut raw,
    );
    let want = expected / c.unpack.raw.size() as usize;
    raw.resize(want, f64::NAN);
    let w = l.block_w as usize;
    let rows = want / w.max(1);
    let mut out = Samples::filled(l.sample, want, 0.0);
    let _ = index;
    for r in 0..rows {
        let from = if c.flip { rows - 1 - r } else { r };
        for i in 0..w {
            out.set(r * w + i, c.unpack.value(raw[from * w + i]));
        }
    }
    out
}

/// A decoded block: samples, or a JPEG stream still to decode.
#[derive(Clone, Debug, PartialEq)]
pub enum Put2 {
    Samples(Samples),
    Jpeg(Vec<u8>),
}
