//! PNG decoded for a raster (docs/adr/0204 §1): every colour type and bit
//! depth, a palette and its transparency, and Adam7 interlacing. 8-bit
//! colour or transparency comes out as RGB or RGBA (a palette expanded);
//! 8-bit grey as one band; 16-bit grey as one band of 16 bits (a height
//! model), its tRNS value its nodata; 16-bit colour as three or four bands of
//! 16 bits. Grey below 8 bits is scaled to 8.

use kentos_contracts::RasterSample;

use super::{RasterError, Samples};

/// The most pixels a PNG raster may have (docs/adr/0204 §1).
pub const MAX_PIXELS: u64 = 64_000_000;

/// A decoded PNG.
#[derive(Clone, Debug, PartialEq)]
pub struct Png {
    pub width: u32,
    pub height: u32,
    pub bands: u32,
    pub samples: Samples,
    /// A 16-bit grey file's transparent value.
    pub nodata: Option<f64>,
}

/// Whether the bytes begin a PNG.
pub fn sniff(head: &[u8]) -> bool {
    head.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])
}

/// Its size, from the header alone (for the window before decoding).
pub fn size(bytes: &[u8]) -> Option<(u32, u32)> {
    if !sniff(bytes) || bytes.get(12..16)? != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes(bytes.get(16..20)?.try_into().ok()?);
    let h = u32::from_be_bytes(bytes.get(20..24)?.try_into().ok()?);
    Some((w, h))
}

fn be32(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

/// Adam7's passes: start column, start row, column step, row step.
const ADAM7: [(u32, u32, u32, u32); 7] = [
    (0, 0, 8, 8),
    (4, 0, 8, 8),
    (0, 4, 4, 8),
    (2, 0, 4, 4),
    (0, 2, 2, 4),
    (1, 0, 2, 2),
    (0, 1, 1, 2),
];

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let (ia, ib, ic) = (i32::from(a), i32::from(b), i32::from(c));
    let p = ia + ib - ic;
    let (pa, pb, pc) = ((p - ia).abs(), (p - ib).abs(), (p - ic).abs());
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// One pass's scanlines unfiltered in place (`bpp` bytes a pixel, at least 1).
fn unfilter(
    data: &mut [u8],
    rows: usize,
    stride: usize,
    bpp: usize,
) -> Result<Vec<u8>, RasterError> {
    let mut out = vec![0u8; rows * stride];
    for r in 0..rows {
        let at = r * (stride + 1);
        let Some(line) = data.get(at..at + stride + 1) else {
            return Err(RasterError::new("PNG kesik: görüntü verisi eksik."));
        };
        let kind = line[0];
        let (done, rest) = out.split_at_mut(r * stride);
        let prev: &[u8] = if r > 0 {
            &done[(r - 1) * stride..]
        } else {
            &[]
        };
        let cur = &mut rest[..stride];
        for i in 0..stride {
            let x = line[1 + i];
            let a = if i >= bpp { cur[i - bpp] } else { 0 };
            let b = prev.get(i).copied().unwrap_or(0);
            let c = if i >= bpp {
                prev.get(i - bpp).copied().unwrap_or(0)
            } else {
                0
            };
            cur[i] = match kind {
                0 => x,
                1 => x.wrapping_add(a),
                2 => x.wrapping_add(b),
                3 => x.wrapping_add(((u16::from(a) + u16::from(b)) / 2) as u8),
                4 => x.wrapping_add(paeth(a, b, c)),
                _ => {
                    return Err(RasterError::new(format!(
                        "PNG bozuk: bilinmeyen satır süzgeci {kind}."
                    )));
                }
            };
        }
    }
    Ok(out)
}

/// The PNG's pixels, or why it is not read.
pub fn decode(bytes: &[u8]) -> Result<Png, RasterError> {
    if !sniff(bytes) {
        return Err(RasterError::new("Dosya PNG değil."));
    }
    let mut at = 8usize;
    let mut header: Option<(u32, u32, u8, u8, u8)> = None;
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut trns: Vec<u8> = Vec::new();
    let mut idat: Vec<u8> = Vec::new();
    while let Some(len) = bytes.get(at..at + 4).map(be32) {
        let len = len as usize;
        let kind = bytes
            .get(at + 4..at + 8)
            .ok_or_else(|| RasterError::new("PNG kesik."))?;
        let data = bytes
            .get(at + 8..at + 8 + len)
            .ok_or_else(|| RasterError::new("PNG kesik: bir bölümün verisi eksik."))?;
        match kind {
            b"IHDR" if data.len() >= 13 => {
                header = Some((
                    be32(&data[0..4]),
                    be32(&data[4..8]),
                    data[8],
                    data[9],
                    data[12],
                ));
            }
            b"PLTE" => palette = data.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect(),
            b"tRNS" => trns = data.to_vec(),
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
        at += 12 + len;
    }
    let (width, height, depth, color, interlace) =
        header.ok_or_else(|| RasterError::new("PNG bozuk: başlık (IHDR) yok."))?;
    if width == 0 || height == 0 || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(RasterError::new(format!(
            "PNG {width} × {height} piksel; en çok {} milyon piksellik PNG okunur. Büyük rasteri GeoTIFF'e çevirin.",
            MAX_PIXELS / 1_000_000
        )));
    }
    let channels: u32 = match color {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => {
            return Err(RasterError::new(format!(
                "PNG'nin renk türü ({color}) bilinmiyor."
            )));
        }
    };
    let ok_depth = match color {
        0 => matches!(depth, 1 | 2 | 4 | 8 | 16),
        3 => matches!(depth, 1 | 2 | 4 | 8),
        _ => matches!(depth, 8 | 16),
    };
    if !ok_depth {
        return Err(RasterError::new(format!(
            "PNG'nin bit derinliği ({depth}) renk türüne uymuyor."
        )));
    }
    let bits = u32::from(depth) * channels;
    let bpp = bits.div_ceil(8).max(1) as usize;
    let passes: Vec<(u32, u32, u32, u32)> = if interlace == 1 {
        ADAM7.to_vec()
    } else {
        vec![(0, 0, 1, 1)]
    };
    let pass_size = |&(x0, y0, dx, dy): &(u32, u32, u32, u32)| {
        let w = if width > x0 {
            (width - x0).div_ceil(dx)
        } else {
            0
        };
        let h = if height > y0 {
            (height - y0).div_ceil(dy)
        } else {
            0
        };
        (w, h)
    };
    let raw_len: usize = passes
        .iter()
        .map(|p| {
            let (w, h) = pass_size(p);
            if w == 0 || h == 0 {
                0
            } else {
                h as usize * (1 + (w as usize * bits as usize).div_ceil(8))
            }
        })
        .sum();
    let mut raw = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&idat, raw_len.max(1))
        .map_err(|_| RasterError::new("PNG bozuk: görüntü verisi açılamadı."))?;
    // Every sample of the image, channel by channel, at its own depth (≤ 16 bits).
    let n = width as usize * height as usize;
    let mut values = vec![0u16; n * channels as usize];
    let mut offset = 0usize;
    for p in &passes {
        let (w, h) = pass_size(p);
        if w == 0 || h == 0 {
            continue;
        }
        let stride = (w as usize * bits as usize).div_ceil(8);
        let len = h as usize * (stride + 1);
        let Some(chunk) = raw.get_mut(offset..offset + len) else {
            return Err(RasterError::new("PNG kesik: görüntü verisi eksik."));
        };
        let lines = unfilter(chunk, h as usize, stride, bpp)?;
        offset += len;
        let (x0, y0, dx, dy) = *p;
        for r in 0..h as usize {
            let line = &lines[r * stride..(r + 1) * stride];
            for c in 0..w as usize {
                let (x, y) = (x0 as usize + c * dx as usize, y0 as usize + r * dy as usize);
                for ch in 0..channels as usize {
                    let s = c * channels as usize + ch;
                    let v = match depth {
                        16 => u16::from_be_bytes([line[2 * s], line[2 * s + 1]]),
                        8 => u16::from(line[s]),
                        d => {
                            let bit = s * usize::from(d);
                            let byte = line[bit / 8];
                            let shift = 8 - usize::from(d) - (bit % 8);
                            u16::from((byte >> shift) & ((1u8 << d) - 1))
                        }
                    };
                    values[(y * width as usize + x) * channels as usize + ch] = v;
                }
            }
        }
    }
    let png = |bands: u32, samples: Samples, nodata: Option<f64>| Png {
        width,
        height,
        bands,
        samples,
        nodata,
    };
    match (color, depth) {
        (3, _) => {
            // A palette expanded; its transparency an alpha band.
            let alpha = !trns.is_empty();
            let bands = if alpha { 4 } else { 3 };
            let mut out = Vec::with_capacity(n * bands);
            for &i in &values {
                let i = usize::from(i);
                let c = palette.get(i).copied().unwrap_or([0, 0, 0]);
                out.extend_from_slice(&c);
                if alpha {
                    out.push(trns.get(i).copied().unwrap_or(255));
                }
            }
            Ok(png(bands as u32, Samples::U8(out), None))
        }
        (0, 16) => {
            let nodata =
                (trns.len() >= 2).then(|| f64::from(u16::from_be_bytes([trns[0], trns[1]])));
            Ok(png(1, Samples::U16(values), nodata))
        }
        (0, d) => {
            let max = (1u16 << d) - 1;
            let scale = |v: u16| ((u32::from(v) * 255 + u32::from(max) / 2) / u32::from(max)) as u8;
            if trns.len() >= 2 {
                let key = u16::from_be_bytes([trns[0], trns[1]]);
                let mut out = Vec::with_capacity(n * 4);
                for &v in &values {
                    let g = scale(v);
                    out.extend_from_slice(&[g, g, g, if v == key { 0 } else { 255 }]);
                }
                Ok(png(4, Samples::U8(out), None))
            } else {
                Ok(png(
                    1,
                    Samples::U8(values.iter().map(|&v| scale(v)).collect()),
                    None,
                ))
            }
        }
        (4, 8) => {
            let mut out = Vec::with_capacity(n * 4);
            for px in values.chunks_exact(2) {
                let g = px[0] as u8;
                out.extend_from_slice(&[g, g, g, px[1] as u8]);
            }
            Ok(png(4, Samples::U8(out), None))
        }
        (4, _) => {
            // 16-bit grey with alpha: the grey band; a transparent pixel nodata (0).
            let out: Vec<u16> = values
                .chunks_exact(2)
                .map(|px| if px[1] == 0 { 0 } else { px[0] })
                .collect();
            Ok(png(1, Samples::U16(out), Some(0.0)))
        }
        (2, 8) if trns.len() >= 6 => {
            let key = [
                u16::from_be_bytes([trns[0], trns[1]]),
                u16::from_be_bytes([trns[2], trns[3]]),
                u16::from_be_bytes([trns[4], trns[5]]),
            ];
            let mut out = Vec::with_capacity(n * 4);
            for px in values.chunks_exact(3) {
                out.extend_from_slice(&[px[0] as u8, px[1] as u8, px[2] as u8]);
                out.push(if px == key { 0 } else { 255 });
            }
            Ok(png(4, Samples::U8(out), None))
        }
        (2 | 6, 8) => Ok(png(
            channels,
            Samples::U8(values.iter().map(|&v| v as u8).collect()),
            None,
        )),
        (2 | 6, _) => Ok(png(channels, Samples::U16(values), None)),
        _ => Err(RasterError::new("PNG'nin renk türü okunmuyor.")),
    }
}

/// The sample type a decoded PNG has.
pub fn sample_of(p: &Png) -> RasterSample {
    p.samples.kind()
}

/// The CRC-32 of a PNG chunk (the type and the data).
fn crc32(parts: &[&[u8]]) -> u32 {
    let mut table = [0u32; 256];
    for (n, slot) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *slot = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for part in parts {
        for &b in *part {
            crc = table[((crc ^ u32::from(b)) & 0xFF) as usize] ^ (crc >> 8);
        }
    }
    crc ^ 0xFFFF_FFFF
}

fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc32(&[kind, data]).to_be_bytes());
}

/// A PNG of 8-bit samples (docs/adr/0556): `bands` 1 is grey, 2 grey with alpha, 3 RGB, 4 RGBA; `samples` are the
/// pixels row by row, bands interleaved. Rows are written without a filter and Deflate-coded by `level` (1 fast … 9
/// small); the file is the same for the same samples.
pub fn encode(
    width: u32,
    height: u32,
    bands: u32,
    samples: &[u8],
    level: u8,
) -> Result<Vec<u8>, RasterError> {
    let color_type = match bands {
        1 => 0u8,
        2 => 4,
        3 => 2,
        4 => 6,
        _ => return Err(RasterError::new("PNG 1, 2, 3 ya da 4 bantlı yazılır.")),
    };
    let stride = width as usize * bands as usize;
    if width == 0 || height == 0 || samples.len() != stride * height as usize {
        return Err(RasterError::new("PNG'nin örnekleri boyutuna uymuyor."));
    }
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for row in samples.chunks_exact(stride) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, color_type, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &super::write::code(&raw, level));
    chunk(&mut out, b"IEND", &[]);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoded_pngs_decode_to_the_same_pixels() {
        let rgba: Vec<u8> = (0..4 * 3 * 4).map(|i| (i * 7 % 256) as u8).collect();
        let p = decode(&encode(4, 3, 4, &rgba, 6).unwrap()).unwrap();
        assert_eq!((p.width, p.height, p.bands), (4, 3, 4));
        assert_eq!(p.samples, Samples::U8(rgba));
        let rgb: Vec<u8> = (0..5 * 2 * 3).map(|i| (i * 11 % 256) as u8).collect();
        let p = decode(&encode(5, 2, 3, &rgb, 1).unwrap()).unwrap();
        assert_eq!((p.bands, p.samples), (3, Samples::U8(rgb)));
        let grey: Vec<u8> = (0..6).map(|i| i * 40).collect();
        let p = decode(&encode(3, 2, 1, &grey, 9).unwrap()).unwrap();
        assert_eq!((p.bands, p.samples), (1, Samples::U8(grey)));
        // Grey with alpha reads back as RGBA (the decoder's rule).
        let la = vec![10, 255, 20, 0];
        let p = decode(&encode(2, 1, 2, &la, 6).unwrap()).unwrap();
        assert_eq!(p.samples, Samples::U8(vec![10, 10, 10, 255, 20, 20, 20, 0]));
    }

    #[test]
    fn the_same_pixels_make_the_same_file_and_bad_input_is_refused() {
        let px = vec![1u8, 2, 3, 4];
        assert_eq!(encode(2, 2, 1, &px, 6).unwrap(), encode(2, 2, 1, &px, 6).unwrap());
        assert!(encode(2, 2, 1, &px[..3], 6).is_err());
        assert!(encode(2, 2, 5, &px, 6).is_err());
        assert!(encode(0, 2, 1, &[], 6).is_err());
    }

    #[test]
    fn the_crc_of_an_empty_iend_is_the_known_one() {
        assert_eq!(crc32(&[b"IEND"]), 0xAE42_6082);
    }
}
