//! Raster picture decoders shared by the atlas and headless paper output (CAD-SHEET-06).
use crate::picture::Picture;
use std::io::Cursor;
/// `data:image/png;base64,…` → the file's bytes.
pub fn data_url(url: &str) -> Option<(&str, Vec<u8>)> {
    let rest = url.strip_prefix("data:")?;
    let (head, body) = rest.split_once(',')?;
    let mime = head.split(';').next().unwrap_or("");
    if !head.ends_with(";base64") {
        return None;
    }
    base64(body).map(|b| (mime, b))
}

/// Standard base64, whitespace ignored; None when it is not.
pub fn base64(text: &str) -> Option<Vec<u8>> {
    let value = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => u32::from(c - b'A'),
            b'a'..=b'z' => u32::from(c - b'a') + 26,
            b'0'..=b'9' => u32::from(c - b'0') + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        })
    };
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        if c.is_ascii_whitespace() {
            continue;
        }
        if c == b'=' {
            break;
        }
        acc = (acc << 6) | value(c)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(out)
}

/// A JPEG's pixels as opaque RGBA (zune-jpeg; docs/adr/0092). The decoder's
/// own limits (16 384 pixels a side) keep a hostile file from asking for
/// unbounded memory; a broken file is no picture.
pub fn jpeg(bytes: &[u8]) -> Option<Picture> {
    use zune_jpeg::JpegDecoder;
    use zune_jpeg::zune_core::bytestream::ZCursor;
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    use zune_jpeg::zune_core::options::DecoderOptions;
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    decoder.decode_headers().ok()?;
    let (w, h) = decoder.dimensions()?;
    let mut rgba = vec![0; decoder.output_buffer_size()?];
    decoder.decode_into(&mut rgba).ok()?;
    let (width, height) = (u32::try_from(w).ok()?, u32::try_from(h).ok()?);
    (rgba.len() == w * h * 4).then_some(Picture::Bitmap {
        width,
        height,
        rgba,
    })
}

/// A PNG's pixels as straight-alpha RGBA.
pub fn png(bytes: &[u8]) -> Option<Picture> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let px = (w as usize) * (h as usize);
    let data = &buf[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => data
            .chunks_exact(3)
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => data
            .chunks_exact(2)
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Grayscale => data.iter().flat_map(|g| [*g, *g, *g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    (rgba.len() == px * 4).then_some(Picture::Bitmap {
        width: w,
        height: h,
        rgba,
    })
}
