//! WebP images of a GeoPackage's tiles (docs/adr/0627 §1): decoded, lossy or lossless, with or without alpha, and
//! written lossless (VP8L) by image-webp. An animated file is refused; the decoder is held to the image's own size.

use std::io::Cursor;

use image_webp::{ColorType, WebPDecoder, WebPEncoder};

use super::RasterError;

/// The largest side a WebP read here may have (a tile is 256 or 512; the format's own limit is 16 384).
pub const MOST_SIDE: u32 = 16_384;

/// A decoded WebP: `channels` 3 (RGB) or 4 (RGBA) bytes a pixel, row by row.
#[derive(Clone, Debug, PartialEq)]
pub struct Webp {
    pub width: u32,
    pub height: u32,
    pub channels: u32,
    pub pixels: Vec<u8>,
}

/// Whether the bytes begin a WebP (a RIFF of form `WEBP`).
pub fn sniff(head: &[u8]) -> bool {
    head.len() >= 12 && &head[0..4] == b"RIFF" && &head[8..12] == b"WEBP"
}

/// The WebP's pixels.
pub fn decode(bytes: &[u8]) -> Result<Webp, RasterError> {
    if !sniff(bytes) {
        return Err(RasterError::new("Dosya WebP değil."));
    }
    let mut d = WebPDecoder::new(Cursor::new(bytes))
        .map_err(|e| RasterError::new(format!("WebP okunamadı: {e}.")))?;
    if d.is_animated() {
        return Err(RasterError::new(
            "WebP canlandırma (birden çok kare) taşıyor; tek görüntülü WebP okunur.",
        ));
    }
    let (width, height) = d.dimensions();
    if width == 0 || height == 0 || width > MOST_SIDE || height > MOST_SIDE {
        return Err(RasterError::new(format!(
            "WebP {width} × {height} piksel; bir kenarı en çok {MOST_SIDE} piksel olur."
        )));
    }
    let channels = if d.has_alpha() { 4 } else { 3 };
    let n = width as usize * height as usize * channels as usize;
    d.set_memory_limit(n.saturating_mul(4).max(1 << 20));
    let mut pixels = vec![0u8; n];
    d.read_image(&mut pixels)
        .map_err(|e| RasterError::new(format!("WebP okunamadı: {e}.")))?;
    Ok(Webp {
        width,
        height,
        channels,
        pixels,
    })
}

/// A lossless WebP of 8-bit samples: `bands` 1 grey, 2 grey with alpha, 3 RGB, 4 RGBA, row by row. The same samples
/// make the same file.
pub fn encode_lossless(width: u32, height: u32, bands: u32, samples: &[u8]) -> Result<Vec<u8>, RasterError> {
    let color = match bands {
        1 => ColorType::L8,
        2 => ColorType::La8,
        3 => ColorType::Rgb8,
        4 => ColorType::Rgba8,
        _ => return Err(RasterError::new("WebP 1, 2, 3 ya da 4 bantlı yazılır.")),
    };
    if width == 0 || height == 0 || width > MOST_SIDE || height > MOST_SIDE {
        return Err(RasterError::new(format!(
            "WebP {width} × {height} piksel yazılamaz; bir kenarı 1 ile {MOST_SIDE} arasında olur."
        )));
    }
    if samples.len() != width as usize * height as usize * bands as usize {
        return Err(RasterError::new("WebP'nin örnekleri boyutuna uymuyor."));
    }
    let mut out = Vec::new();
    WebPEncoder::new(&mut out)
        .encode(samples, width, height, color)
        .map_err(|e| RasterError::new(format!("WebP yazılamadı: {e}.")))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lossless_webp_reads_back_the_same_pixels() {
        let rgba: Vec<u8> = (0..16 * 9 * 4).map(|i| (i * 13 % 256) as u8).collect();
        let w = decode(&encode_lossless(16, 9, 4, &rgba).unwrap()).unwrap();
        assert_eq!((w.width, w.height, w.channels), (16, 9, 4));
        assert_eq!(w.pixels, rgba);
        let rgb: Vec<u8> = (0..5 * 3 * 3).map(|i| (i * 7 % 256) as u8).collect();
        let w = decode(&encode_lossless(5, 3, 3, &rgb).unwrap()).unwrap();
        assert_eq!((w.channels, w.pixels), (3, rgb));
        // Grey comes back as colour: WebP holds RGB(A).
        let w = decode(&encode_lossless(2, 1, 1, &[10, 200]).unwrap()).unwrap();
        assert_eq!(w.pixels, vec![10, 10, 10, 200, 200, 200]);
    }

    #[test]
    fn the_same_pixels_make_the_same_file_and_bad_input_is_refused() {
        let px = vec![1u8, 2, 3, 4];
        assert_eq!(
            encode_lossless(2, 2, 1, &px).unwrap(),
            encode_lossless(2, 2, 1, &px).unwrap()
        );
        assert!(encode_lossless(2, 2, 1, &px[..3]).is_err());
        assert!(encode_lossless(2, 2, 5, &px).is_err());
        assert!(encode_lossless(0, 2, 1, &[]).is_err());
        assert!(decode(b"RIFF\0\0\0\0WEBPVP8 ").is_err());
        assert!(decode(b"not a webp at all").is_err());
    }
}
