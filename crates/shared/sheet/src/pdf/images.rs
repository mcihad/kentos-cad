//! A sheet's pictures for the PDF (design §9a): a JPEG goes in as it is
//! (`DCTDecode`), a PNG is read into its samples and written with Flate, its
//! transparency as a soft mask (`SMask`).
//!
//! The PNG reader is the core's own, on the `miniz_oxide` inflate the
//! formats already use (as the zipped Shapefile reader is): every PNG
//! (grey, RGB, palette, with alpha or `tRNS`, 1–16 bits, interlaced or not)
//! becomes 8-bit samples, the same on every target. The `png` crate is
//! native-only in this workspace; the tests hold this reader to it.

/// A picture ready for the PDF: its samples (or a JPEG's own bytes) and its mask.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Picture {
    pub width: u32,
    pub height: u32,
    pub kind: PictureKind,
    /// The colour samples, Flate-compressed (a PNG's) or the JPEG file.
    pub data: Vec<u8>,
    /// The alpha samples (8 bits), Flate-compressed; none: opaque.
    pub mask: Option<Vec<u8>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PictureKind {
    Gray,
    Rgb,
    /// A JPEG: its colour components (1, 3 or 4) and whether an Adobe CMYK is inverted.
    Jpeg {
        components: u8,
        inverted: bool,
    },
}

/// A picture's pixels at most: larger ones are refused (a few kilobytes of PNG can claim gigabytes).
const MAX_PIXELS: u64 = 120_000_000;
const MAX_SIDE: u32 = 16_384;

/// A PNG or JPEG file as a PDF picture; none for anything else or a broken file.
pub(super) fn picture(bytes: &[u8]) -> Option<Picture> {
    if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        let d = decode_png(bytes)?;
        let kind = if d.channels == 1 {
            PictureKind::Gray
        } else {
            PictureKind::Rgb
        };
        Some(Picture {
            width: d.width,
            height: d.height,
            kind,
            data: deflate(&d.color),
            mask: d.alpha.as_deref().map(deflate),
        })
    } else if bytes.starts_with(&[0xff, 0xd8]) {
        let (width, height, components, inverted) = jpeg_header(bytes)?;
        Some(Picture {
            width,
            height,
            kind: PictureKind::Jpeg {
                components,
                inverted,
            },
            data: bytes.to_vec(),
            mask: None,
        })
    } else {
        None
    }
}

/// Flate (zlib), the same bytes on every target.
pub(super) fn deflate(bytes: &[u8]) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec_zlib(bytes, 6)
}

// ── PNG ──────────────────────────────────────────────────────────────────

/// A PNG's samples, 8 bits each: grey or RGB, and alpha when any pixel is not opaque.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Decoded {
    pub width: u32,
    pub height: u32,
    /// 1 (grey) or 3 (RGB).
    pub channels: u8,
    pub color: Vec<u8>,
    pub alpha: Option<Vec<u8>>,
}

struct Header {
    width: u32,
    height: u32,
    depth: u8,
    color: u8,
    interlaced: bool,
}

impl Header {
    /// Samples a pixel has in the file.
    fn samples(&self) -> usize {
        match self.color {
            0 | 3 => 1,
            4 => 2,
            2 => 3,
            _ => 4,
        }
    }

    /// Bytes of a scanline of `w` pixels (without its filter byte).
    fn stride(&self, w: u32) -> usize {
        (w as usize * self.samples() * usize::from(self.depth)).div_ceil(8)
    }

    /// The filter's step: bytes of a whole pixel, at least one.
    fn step(&self) -> usize {
        (self.samples() * usize::from(self.depth))
            .div_ceil(8)
            .max(1)
    }
}

fn be32(b: &[u8]) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(..4)?.try_into().ok()?))
}

pub(super) fn decode_png(bytes: &[u8]) -> Option<Decoded> {
    let mut at = 8;
    let mut header: Option<Header> = None;
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut trns: Vec<u8> = Vec::new();
    let mut idat: Vec<u8> = Vec::new();
    // A file is read to its end mark, every chunk whole (its CRC's bytes there): a cut file is
    // refused rather than drawn from what came before the cut.
    let mut ended = false;
    while at + 8 <= bytes.len() {
        let len = be32(&bytes[at..])? as usize;
        let kind = bytes.get(at + 4..at + 8)?;
        let end = at.checked_add(8)?.checked_add(len)?;
        let data = bytes.get(at + 8..end)?;
        bytes.get(end..end.checked_add(4)?)?;
        match kind {
            b"IHDR" => {
                let h = Header {
                    width: be32(data)?,
                    height: be32(data.get(4..)?)?,
                    depth: *data.get(8)?,
                    color: *data.get(9)?,
                    interlaced: *data.get(12)? == 1,
                };
                let depth_ok = match h.color {
                    0 => matches!(h.depth, 1 | 2 | 4 | 8 | 16),
                    3 => matches!(h.depth, 1 | 2 | 4 | 8),
                    2 | 4 | 6 => matches!(h.depth, 8 | 16),
                    _ => false,
                };
                if !depth_ok
                    || h.width == 0
                    || h.height == 0
                    || h.width > MAX_SIDE
                    || h.height > MAX_SIDE
                    || u64::from(h.width) * u64::from(h.height) > MAX_PIXELS
                    || *data.get(10)? != 0
                    || *data.get(11)? != 0
                {
                    return None;
                }
                header = Some(h);
            }
            b"PLTE" => {
                palette = data.chunks_exact(3).map(|c| [c[0], c[1], c[2]]).collect();
            }
            b"tRNS" => trns = data.to_vec(),
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => {
                ended = true;
                break;
            }
            _ => {}
        }
        // Length, type, data and the CRC.
        at = at.checked_add(12)?.checked_add(len)?;
    }
    if !ended {
        return None;
    }
    let h = header?;
    if h.color == 3 && palette.is_empty() {
        return None;
    }
    let passes = passes(&h);
    let expected: usize = passes
        .iter()
        .map(|p| p.rows as usize * (1 + h.stride(p.cols)))
        .sum();
    let raw = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&idat, expected).ok()?;
    if raw.len() < expected {
        return None;
    }
    let (w, ht) = (h.width as usize, h.height as usize);
    let mut out = Out::new(&h, w * ht, &palette, &trns);
    let mut offset = 0;
    for p in &passes {
        let stride = h.stride(p.cols);
        let mut prev = vec![0u8; stride];
        for r in 0..p.rows as usize {
            let line = raw.get(offset..offset + 1 + stride)?;
            offset += 1 + stride;
            let mut cur = line[1..].to_vec();
            unfilter(line[0], &mut cur, &prev, h.step())?;
            for c in 0..p.cols as usize {
                let x = p.x0 as usize + c * p.dx as usize;
                let y = p.y0 as usize + r * p.dy as usize;
                out.put(y * w + x, &cur, c);
            }
            prev = cur;
        }
    }
    Some(out.finish(h.width, h.height))
}

/// One of Adam7's seven passes (or the whole picture): its first pixel, its steps and its size.
struct Pass {
    x0: u32,
    y0: u32,
    dx: u32,
    dy: u32,
    cols: u32,
    rows: u32,
}

fn passes(h: &Header) -> Vec<Pass> {
    let starts: &[(u32, u32, u32, u32)] = if h.interlaced {
        &[
            (0, 0, 8, 8),
            (4, 0, 8, 8),
            (0, 4, 4, 8),
            (2, 0, 4, 4),
            (0, 2, 2, 4),
            (1, 0, 2, 2),
            (0, 1, 1, 2),
        ]
    } else {
        &[(0, 0, 1, 1)]
    };
    starts
        .iter()
        .map(|&(x0, y0, dx, dy)| Pass {
            x0,
            y0,
            dx,
            dy,
            cols: h.width.saturating_sub(x0).div_ceil(dx),
            rows: h.height.saturating_sub(y0).div_ceil(dy),
        })
        .filter(|p| p.cols > 0 && p.rows > 0)
        .collect()
}

/// A scanline's filter undone (PNG §9): none, sub, up, average, Paeth.
fn unfilter(kind: u8, cur: &mut [u8], prev: &[u8], bpp: usize) -> Option<()> {
    match kind {
        0 => {}
        1 => {
            for i in bpp..cur.len() {
                cur[i] = cur[i].wrapping_add(cur[i - bpp]);
            }
        }
        2 => {
            for (c, p) in cur.iter_mut().zip(prev) {
                *c = c.wrapping_add(*p);
            }
        }
        3 => {
            for i in 0..cur.len() {
                let left = if i >= bpp { u16::from(cur[i - bpp]) } else { 0 };
                cur[i] = cur[i].wrapping_add(((left + u16::from(prev[i])) / 2) as u8);
            }
        }
        4 => {
            for i in 0..cur.len() {
                let a = if i >= bpp { i16::from(cur[i - bpp]) } else { 0 };
                let b = i16::from(prev[i]);
                let c = if i >= bpp {
                    i16::from(prev[i - bpp])
                } else {
                    0
                };
                let p = a + b - c;
                let (pa, pb, pc) = ((p - a).abs(), (p - b).abs(), (p - c).abs());
                let pred = if pa <= pb && pa <= pc {
                    a
                } else if pb <= pc {
                    b
                } else {
                    c
                };
                cur[i] = cur[i].wrapping_add(pred as u8);
            }
        }
        _ => return None,
    }
    Some(())
}

/// The picture being filled in, pixel by pixel.
struct Out<'a> {
    depth: u8,
    color_type: u8,
    samples: usize,
    palette: &'a [[u8; 3]],
    trns: &'a [u8],
    channels: u8,
    color: Vec<u8>,
    alpha: Vec<u8>,
    translucent: bool,
}

impl<'a> Out<'a> {
    fn new(h: &Header, pixels: usize, palette: &'a [[u8; 3]], trns: &'a [u8]) -> Self {
        let channels = if matches!(h.color, 0 | 4) { 1 } else { 3 };
        Out {
            depth: h.depth,
            color_type: h.color,
            samples: h.samples(),
            palette,
            trns,
            channels,
            color: vec![0; pixels * usize::from(channels)],
            alpha: vec![255; pixels],
            translucent: false,
        }
    }

    /// Sample `s` of pixel `c` of an unfiltered line, as the file stores it (1–16 bits).
    fn raw(&self, line: &[u8], c: usize, s: usize) -> u16 {
        let index = c * self.samples + s;
        match self.depth {
            16 => u16::from_be_bytes([line[index * 2], line[index * 2 + 1]]),
            8 => u16::from(line[index]),
            d => {
                let bits = index * usize::from(d);
                let byte = line[bits / 8];
                let shift = 8 - usize::from(d) - bits % 8;
                u16::from((byte >> shift) & ((1u8 << d) - 1))
            }
        }
    }

    /// A file value as 8 bits.
    fn eight(&self, v: u16) -> u8 {
        match self.depth {
            16 => (v >> 8) as u8,
            8 => v as u8,
            d => (u32::from(v) * 255 / ((1u32 << d) - 1)) as u8,
        }
    }

    fn put(&mut self, at: usize, line: &[u8], c: usize) {
        let ch = usize::from(self.channels);
        let mut alpha = 255u8;
        match self.color_type {
            3 => {
                let i = usize::from(self.raw(line, c, 0));
                let rgb = self.palette.get(i).copied().unwrap_or([0, 0, 0]);
                self.color[at * 3..at * 3 + 3].copy_from_slice(&rgb);
                alpha = self.trns.get(i).copied().unwrap_or(255);
            }
            0 | 4 => {
                let v = self.raw(line, c, 0);
                self.color[at] = self.eight(v);
                if self.color_type == 4 {
                    alpha = self.eight(self.raw(line, c, 1));
                } else if self.trns.len() >= 2
                    && u16::from_be_bytes([self.trns[0], self.trns[1]]) == v
                {
                    alpha = 0;
                }
            }
            _ => {
                let v = [
                    self.raw(line, c, 0),
                    self.raw(line, c, 1),
                    self.raw(line, c, 2),
                ];
                for (k, x) in v.iter().enumerate() {
                    self.color[at * ch + k] = self.eight(*x);
                }
                if self.color_type == 6 {
                    alpha = self.eight(self.raw(line, c, 3));
                } else if self.trns.len() >= 6 {
                    let t = [
                        u16::from_be_bytes([self.trns[0], self.trns[1]]),
                        u16::from_be_bytes([self.trns[2], self.trns[3]]),
                        u16::from_be_bytes([self.trns[4], self.trns[5]]),
                    ];
                    if t == v {
                        alpha = 0;
                    }
                }
            }
        }
        if alpha != 255 {
            self.translucent = true;
        }
        self.alpha[at] = alpha;
    }

    fn finish(self, width: u32, height: u32) -> Decoded {
        Decoded {
            width,
            height,
            channels: self.channels,
            color: self.color,
            alpha: self.translucent.then_some(self.alpha),
        }
    }
}

// ── JPEG ─────────────────────────────────────────────────────────────────

/// A JPEG's size, components and whether it is an Adobe CMYK (stored inverted), from its
/// markers; none for a broken file or a coding PDF does not read (arithmetic, lossless).
pub(super) fn jpeg_header(b: &[u8]) -> Option<(u32, u32, u8, bool)> {
    let mut at = 2;
    let mut adobe = false;
    loop {
        // Markers may be padded with fill bytes.
        while *b.get(at)? != 0xff {
            at += 1;
        }
        while *b.get(at)? == 0xff {
            at += 1;
        }
        let marker = *b.get(at)?;
        at += 1;
        if matches!(marker, 0xd0..=0xd9 | 0x01) {
            continue;
        }
        let len = usize::from(u16::from_be_bytes([*b.get(at)?, *b.get(at + 1)?]));
        if len < 2 {
            return None;
        }
        let seg = b.get(at + 2..at.checked_add(len)?)?;
        match marker {
            0xee if seg.starts_with(b"Adobe") => adobe = true,
            // Baseline, extended and progressive Huffman: what PDF's DCTDecode reads.
            0xc0..=0xc2 => {
                let height = u32::from(u16::from_be_bytes([*seg.get(1)?, *seg.get(2)?]));
                let width = u32::from(u16::from_be_bytes([*seg.get(3)?, *seg.get(4)?]));
                let components = *seg.get(5)?;
                if width == 0 || height == 0 || !matches!(components, 1 | 3 | 4) {
                    return None;
                }
                return Some((width, height, components, adobe && components == 4));
            }
            0xc3 | 0xc5..=0xc7 | 0xc9..=0xcb | 0xcd..=0xcf | 0xda => return None,
            _ => {}
        }
        at += len;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PNG written by the `png` crate (the reference in these tests).
    fn encode(
        size: (u32, u32),
        color: png::ColorType,
        depth: png::BitDepth,
        data: &[u8],
        extra: impl Fn(&mut png::Encoder<'_, &mut Vec<u8>>),
    ) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, size.0, size.1);
            e.set_color(color);
            e.set_depth(depth);
            extra(&mut e);
            let mut wr = e.write_header().expect("header");
            wr.write_image_data(data).expect("data");
        }
        out
    }

    /// What the `png` crate reads, as 8-bit grey or RGB, alpha and the channels.
    fn reference(bytes: &[u8]) -> (Vec<u8>, Vec<u8>, u8) {
        let mut d = png::Decoder::new(std::io::Cursor::new(bytes));
        d.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
        let mut r = d.read_info().expect("info");
        let mut buf = vec![0; r.output_buffer_size().expect("size")];
        let info = r.next_frame(&mut buf).expect("frame");
        let buf = &buf[..info.buffer_size()];
        match info.color_type {
            png::ColorType::Grayscale => (buf.to_vec(), vec![255; buf.len()], 1),
            png::ColorType::GrayscaleAlpha => (
                buf.chunks(2).map(|c| c[0]).collect(),
                buf.chunks(2).map(|c| c[1]).collect(),
                1,
            ),
            png::ColorType::Rgb => (buf.to_vec(), vec![255; buf.len() / 3], 3),
            png::ColorType::Rgba => (
                buf.chunks(4).flat_map(|c| [c[0], c[1], c[2]]).collect(),
                buf.chunks(4).map(|c| c[3]).collect(),
                3,
            ),
            other => panic!("{other:?}"),
        }
    }

    fn same_as_reference(bytes: &[u8]) {
        let d = decode_png(bytes).expect("ours reads it");
        let (color, alpha, channels) = reference(bytes);
        assert_eq!(d.channels, channels);
        assert_eq!(d.color, color);
        assert_eq!(d.alpha.unwrap_or_else(|| vec![255; alpha.len()]), alpha);
    }

    fn pattern(n: usize) -> Vec<u8> {
        (0..n)
            .map(|i| ((i * 37 + i / 7 * 11) % 251) as u8)
            .collect()
    }

    /// Every colour type and depth, with every filter, as the reference decoder reads it.
    #[test]
    fn a_png_is_read_as_the_png_crate_reads_it() {
        let (w, h) = (37u32, 23u32);
        let cases = [
            (
                png::ColorType::Grayscale,
                png::BitDepth::One,
                1usize,
                1usize,
            ),
            (png::ColorType::Grayscale, png::BitDepth::Four, 1, 4),
            (png::ColorType::Grayscale, png::BitDepth::Eight, 1, 8),
            (png::ColorType::Grayscale, png::BitDepth::Sixteen, 1, 16),
            (png::ColorType::GrayscaleAlpha, png::BitDepth::Eight, 2, 8),
            (png::ColorType::Rgb, png::BitDepth::Eight, 3, 8),
            (png::ColorType::Rgb, png::BitDepth::Sixteen, 3, 16),
            (png::ColorType::Rgba, png::BitDepth::Eight, 4, 8),
            (png::ColorType::Rgba, png::BitDepth::Sixteen, 4, 16),
        ];
        for (color, depth, samples, bits) in cases {
            let stride = (w as usize * samples * bits).div_ceil(8);
            let data = pattern(stride * h as usize);
            for filter in [
                png::Filter::NoFilter,
                png::Filter::Sub,
                png::Filter::Up,
                png::Filter::Avg,
                png::Filter::Paeth,
            ] {
                same_as_reference(&encode((w, h), color, depth, &data, |e| {
                    e.set_filter(filter)
                }));
            }
        }
        // A palette with transparency, 2 bits.
        let data = pattern((w as usize * 2).div_ceil(8) * h as usize);
        let bytes = encode(
            (w, h),
            png::ColorType::Indexed,
            png::BitDepth::Two,
            &data,
            |e| {
                e.set_palette(vec![255, 0, 0, 0, 255, 0, 0, 0, 255, 9, 9, 9]);
                e.set_trns(vec![255, 128, 0]);
            },
        );
        same_as_reference(&bytes);
        // tRNS on RGB: one colour clear.
        let mut data = pattern(w as usize * 3 * h as usize);
        data[..3].copy_from_slice(&[1, 2, 3]);
        let bytes = encode(
            (w, h),
            png::ColorType::Rgb,
            png::BitDepth::Eight,
            &data,
            |e| {
                e.set_trns(vec![0, 1, 0, 2, 0, 3]);
            },
        );
        let d = decode_png(&bytes).expect("reads");
        assert_eq!(d.alpha.as_ref().map(|a| a[0]), Some(0));
        same_as_reference(&bytes);
    }

    /// Adam7: the seven passes put back in place.
    #[test]
    fn an_interlaced_png_is_read_whole() {
        // An interlaced 13 × 9 RGB picture, written here: the png crate does not interlace.
        let (w, h) = (13u32, 9u32);
        let px = |x: u32, y: u32| [(x * 19) as u8, (y * 27) as u8, ((x + y) * 7) as u8];
        let hd = Header {
            width: w,
            height: h,
            depth: 8,
            color: 2,
            interlaced: true,
        };
        let mut raw = Vec::new();
        for p in passes(&hd) {
            for r in 0..p.rows {
                raw.push(0);
                for c in 0..p.cols {
                    raw.extend_from_slice(&px(p.x0 + c * p.dx, p.y0 + r * p.dy));
                }
            }
        }
        let mut file = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let chunk = |file: &mut Vec<u8>, kind: &[u8], data: &[u8]| {
            file.extend_from_slice(&(data.len() as u32).to_be_bytes());
            file.extend_from_slice(kind);
            file.extend_from_slice(data);
            file.extend_from_slice(&[0, 0, 0, 0]);
        };
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&w.to_be_bytes());
        ihdr.extend_from_slice(&h.to_be_bytes());
        ihdr.extend_from_slice(&[8, 2, 0, 0, 1]);
        chunk(&mut file, b"IHDR", &ihdr);
        chunk(&mut file, b"IDAT", &deflate(&raw));
        chunk(&mut file, b"IEND", &[]);
        let d = decode_png(&file).expect("reads");
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 3) as usize;
                assert_eq!(&d.color[i..i + 3], &px(x, y), "({x}, {y})");
            }
        }
        assert!(d.alpha.is_none(), "opaque: no mask");
    }

    /// A broken or hostile file is refused, never half read.
    #[test]
    fn a_broken_png_or_jpeg_is_refused() {
        let good = encode(
            (4, 4),
            png::ColorType::Rgb,
            png::BitDepth::Eight,
            &[7; 48],
            |_| {},
        );
        assert!(picture(&good).is_some());
        for cut in [9, 20, 33, good.len() - 13] {
            assert!(decode_png(&good[..cut]).is_none(), "cut at {cut}");
        }
        // A header claiming 100 000 × 100 000 pixels.
        let mut huge = good.clone();
        huge[16..20].copy_from_slice(&100_000u32.to_be_bytes());
        huge[20..24].copy_from_slice(&100_000u32.to_be_bytes());
        assert!(decode_png(&huge).is_none());
        assert!(picture(b"\xff\xd8\xff\xd9").is_none());
        assert!(picture(b"GIF89a").is_none());
    }

    /// A JPEG's size and components from its frame header; Adobe CMYK is marked inverted.
    #[test]
    fn a_jpeg_s_header_gives_its_size() {
        let mut j = vec![0xff, 0xd8];
        j.extend_from_slice(&[0xff, 0xee, 0, 14]);
        j.extend_from_slice(b"Adobe\0\x64\0\0\0\0\x02");
        j.extend_from_slice(&[0xff, 0xc2, 0, 20, 8, 0, 30, 0, 40, 4]);
        j.extend_from_slice(&[0; 12]);
        j.extend_from_slice(&[0xff, 0xda, 0, 2, 0xff, 0xd9]);
        assert_eq!(jpeg_header(&j), Some((40, 30, 4, true)));
        let p = picture(&j).expect("a picture");
        assert_eq!((p.width, p.height, p.data.len()), (40, 30, j.len()));
    }
}
