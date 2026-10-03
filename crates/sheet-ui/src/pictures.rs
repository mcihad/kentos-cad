//! A sheet's pictures (logos, stamps) decoded for painting (design, open
//! question 6: Iced's image and SVG support): PNG through the `png` crate, JPEG
//! through `zune-jpeg` (both already the desktop's), into straight RGBA kept as
//! textures — the picture at its own size and halved again and again, each
//! halving the average of its four pixels, so a picture small on the paper is
//! drawn from a texture near its size and does not shimmer, and a magnified one
//! is smoothed (bilinear), not drawn as squares — and SVG pictures, which resvg
//! draws at the size they are shown. A picture that does not decode is drawn as
//! a missing one; the preflight says so (`asset_missing`), not this module.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::io::Cursor;

use iced::advanced::image::Handle as ImageHandle;
use iced::advanced::svg::Handle as SvgHandle;
use kentos_geometry_core::display::fixed;
use kentos_sheet::model::{ASSET_MAX_BYTES, AssetKind, AssetMeta};
use kentos_sheet::template::sha256_hex;

/// What a picture file chosen for a sheet is (design §3.4, the web's
/// `addPicture`): a PNG, JPEG or SVG no larger than the core allows,
/// measured, named by the SHA-256 of its bytes. Refused with the web's words.
pub fn asset_of(name: &str, bytes: &[u8]) -> Result<AssetMeta, String> {
    let kind = if bytes.starts_with(b"\x89PNG") {
        AssetKind::Png
    } else if bytes.starts_with(&[0xff, 0xd8]) {
        AssetKind::Jpeg
    } else if is_svg(name, bytes) {
        AssetKind::Svg
    } else {
        return Err(format!("“{name}” resim değil: PNG, JPEG ya da SVG seçin."));
    };
    let mb = |n: f64| n / 1_048_576.0;
    let size = u32::try_from(bytes.len())
        .ok()
        .filter(|n| *n <= ASSET_MAX_BYTES);
    let Some(size) = size else {
        return Err(format!(
            "“{name}” {} MB: bir resim en çok {} MB olabilir. Küçültüp yeniden seçin.",
            fixed(mb(bytes.len() as f64), 1),
            fixed(mb(f64::from(ASSET_MAX_BYTES)), 0)
        ));
    };
    let raster = decode(bytes)
        .ok_or_else(|| format!("“{name}” okunamadı: resim çözülemedi. Başka bir dosya deneyin."))?;
    Ok(AssetMeta {
        sha256: sha256_hex(bytes),
        kind,
        name: name.to_owned(),
        width: raster.width,
        height: raster.height,
        bytes: size,
        dpi: None,
    })
}

/// An SVG file: by its name, or by its text (an XML file whose first element is `svg`).
fn is_svg(name: &str, bytes: &[u8]) -> bool {
    if name.to_ascii_lowercase().ends_with(".svg") {
        return true;
    }
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]);
    let head = head.trim_start_matches('\u{feff}').trim_start();
    head.starts_with("<svg") || (head.starts_with("<?xml") && head.contains("<svg"))
}

/// A texture of a picture: its size and pixels.
#[derive(Clone, Debug, PartialEq)]
struct Level {
    width: u32,
    height: u32,
    handle: ImageHandle,
}

/// A decoded picture, ready to draw: its size, and its pixels as textures (its own size first,
/// then its halvings) or, for an SVG, the file resvg draws.
#[derive(Clone, Debug)]
pub struct Raster {
    pub width: u32,
    pub height: u32,
    levels: Vec<Level>,
    svg: Option<SvgHandle>,
    /// Its own size magnified by whole factors, made when first asked ([`Raster::texture_for`]);
    /// at most [`MAGNIFIED_KEPT`] of them.
    magnified: RefCell<BTreeMap<u32, ImageHandle>>,
}

impl PartialEq for Raster {
    fn eq(&self, other: &Raster) -> bool {
        (self.width, self.height, &self.levels, &self.svg)
            == (other.width, other.height, &other.levels, &other.svg)
    }
}

/// A texture magnified here is at most this a side.
const MAGNIFIED_MAX: u32 = 4096;
/// Magnified textures kept a picture (the stage and a card may show it at two sizes at once).
const MAGNIFIED_KEPT: usize = 4;

/// A picture is halved until its longer side is at most this.
const SMALLEST: u32 = 32;

impl Raster {
    /// A picture from its straight RGBA pixels, row by row: its texture and its halvings.
    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Option<Raster> {
        let n = (width as usize)
            .checked_mul(height as usize)?
            .checked_mul(4)?;
        if width == 0 || height == 0 || rgba.len() != n {
            return None;
        }
        let mut levels = Vec::new();
        let (mut w, mut h) = (width, height);
        let mut px = rgba;
        while w.max(h) > SMALLEST && (w > 1 || h > 1) {
            let (nw, nh, next) = halve(w, h, &px);
            levels.push(Level {
                width: w,
                height: h,
                handle: ImageHandle::from_rgba(w, h, px),
            });
            (w, h, px) = (nw, nh, next);
        }
        levels.push(Level {
            width: w,
            height: h,
            handle: ImageHandle::from_rgba(w, h, px),
        });
        Some(Raster {
            width,
            height,
            levels,
            svg: None,
            magnified: RefCell::default(),
        })
    }

    /// The pixels at the picture's own size (none for an SVG).
    pub fn rgba(&self) -> Option<&[u8]> {
        match &self.levels.first()?.handle {
            ImageHandle::Rgba { pixels, .. } => Some(pixels),
            _ => None,
        }
    }

    /// The sizes of its textures, its own first (none for an SVG).
    pub fn sizes(&self) -> Vec<(u32, u32)> {
        self.levels.iter().map(|l| (l.width, l.height)).collect()
    }

    /// The SVG resvg draws, for an SVG picture.
    pub(crate) fn svg(&self) -> Option<&SvgHandle> {
        self.svg.as_ref()
    }

    /// The texture of the picture shown `w` × `h` pixels from its own and its halvings: the
    /// smallest at least twice that (a screen may have two pixels to a point), its own size when
    /// shown larger than half of it.
    fn texture(&self, w: f32, h: f32) -> Option<&ImageHandle> {
        let want = |l: &Level| l.width as f32 >= 2.0 * w || l.height as f32 >= 2.0 * h;
        self.levels
            .iter()
            .rev()
            .find(|l| want(l))
            .or(self.levels.first())
            .map(|l| &l.handle)
    }

    /// The texture to draw the picture `w` × `h` pixels with. Shown smaller than itself: its own
    /// or a halving ([`Raster::texture`]). Magnified: itself magnified here by the whole factor
    /// that brings a texture pixel to at most a pixel drawn, smoothed (bilinear, as a GPU's own
    /// magnification), at most [`MAGNIFIED_MAX`] a side. The software renderer (a PNG export's)
    /// places a picture only to its texture's pixels; with these the screen and a PNG agree.
    pub(crate) fn texture_for(&self, w: f32, h: f32) -> Option<ImageHandle> {
        let base = self.levels.first()?;
        let scale = (w / base.width as f32).max(h / base.height as f32);
        let m = (scale.ceil() as u32).min((MAGNIFIED_MAX / base.width.max(base.height)).max(1));
        if scale <= 1.0 || m <= 1 {
            return self.texture(w, h).cloned();
        }
        if let Some(t) = self.magnified.borrow().get(&m) {
            return Some(t.clone());
        }
        let up = magnify(base.width, base.height, self.rgba()?, m);
        let t = ImageHandle::from_rgba(base.width * m, base.height * m, up);
        let mut kept = self.magnified.borrow_mut();
        while kept.len() >= MAGNIFIED_KEPT {
            // The one farthest from this factor goes.
            let far = kept.keys().copied().max_by_key(|k| k.abs_diff(m));
            if let Some(k) = far {
                kept.remove(&k);
            }
        }
        kept.insert(m, t.clone());
        Some(t)
    }
}

/// A picture magnified `m` times, bilinear (each new pixel from the four nearest, colours
/// weighed by their opacity), its edges held.
fn magnify(w: u32, h: u32, px: &[u8], m: u32) -> Vec<u8> {
    let (nw, nh) = ((w * m) as usize, (h * m) as usize);
    let mut out = Vec::with_capacity(nw * nh * 4);
    let at = |x: i64, y: i64| -> [f32; 4] {
        let (x, y) = (x.clamp(0, i64::from(w) - 1), y.clamp(0, i64::from(h) - 1));
        let i = ((y * i64::from(w) + x) * 4) as usize;
        let a = f32::from(px[i + 3]) / 255.0;
        [
            f32::from(px[i]) * a,
            f32::from(px[i + 1]) * a,
            f32::from(px[i + 2]) * a,
            a,
        ]
    };
    let k = m as f32;
    for oy in 0..nh {
        let sy = (oy as f32 + 0.5) / k - 0.5;
        let (y0, fy) = (sy.floor(), sy - sy.floor());
        for ox in 0..nw {
            let sx = (ox as f32 + 0.5) / k - 0.5;
            let (x0, fx) = (sx.floor(), sx - sx.floor());
            let (x0, y0i) = (x0 as i64, y0 as i64);
            let (p00, p10, p01, p11) = (
                at(x0, y0i),
                at(x0 + 1, y0i),
                at(x0, y0i + 1),
                at(x0 + 1, y0i + 1),
            );
            let mut c = [0f32; 4];
            for j in 0..4 {
                let top = p00[j] + (p10[j] - p00[j]) * fx;
                let bottom = p01[j] + (p11[j] - p01[j]) * fx;
                c[j] = top + (bottom - top) * fy;
            }
            let a = c[3];
            let channel = |v: f32| {
                if a <= 0.0 {
                    0
                } else {
                    (v / a).round().clamp(0.0, 255.0) as u8
                }
            };
            out.extend_from_slice(&[
                channel(c[0]),
                channel(c[1]),
                channel(c[2]),
                (a * 255.0).round() as u8,
            ]);
        }
    }
    out
}

/// A picture halved: each pixel the average of the (up to) four it covers, colours weighed by
/// their opacity so a transparent edge does not darken it.
fn halve(w: u32, h: u32, px: &[u8]) -> (u32, u32, Vec<u8>) {
    let (nw, nh) = (w.div_ceil(2).max(1), h.div_ceil(2).max(1));
    let mut out = Vec::with_capacity(nw as usize * nh as usize * 4);
    for y in 0..nh {
        for x in 0..nw {
            let (mut sum, mut a, mut n) = ([0u32; 3], 0u32, 0u32);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (sx, sy) = (x * 2 + dx, y * 2 + dy);
                if sx >= w || sy >= h {
                    continue;
                }
                let i = ((sy * w + sx) * 4) as usize;
                let alpha = u32::from(px[i + 3]);
                for (c, s) in sum.iter_mut().enumerate() {
                    *s += u32::from(px[i + c]) * alpha;
                }
                a += alpha;
                n += 1;
            }
            let rgb = |s: u32| (s + a / 2).checked_div(a).unwrap_or(0) as u8;
            out.extend_from_slice(&[
                rgb(sum[0]),
                rgb(sum[1]),
                rgb(sum[2]),
                ((a + n / 2) / n.max(1)) as u8,
            ]);
        }
    }
    (nw, nh, out)
}

/// An SVG file's picture: its size as the file gives it; resvg draws it at the size shown.
fn decode_svg(bytes: &[u8]) -> Option<Raster> {
    use resvg::usvg;
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(bytes, &options).ok()?;
    let size = tree.size();
    let (w, h) = (size.width().ceil(), size.height().ceil());
    if !(w >= 1.0 && h >= 1.0) {
        return None;
    }
    Some(Raster {
        width: w as u32,
        height: h as u32,
        levels: Vec::new(),
        svg: Some(SvgHandle::from_memory(bytes.to_vec())),
        magnified: RefCell::default(),
    })
}

/// The system's fonts for an SVG's text, read once (as Iced's SVG support reads them).
fn svg_fonts() -> std::sync::Arc<resvg::usvg::fontdb::Database> {
    static FONTS: std::sync::OnceLock<std::sync::Arc<resvg::usvg::fontdb::Database>> =
        std::sync::OnceLock::new();
    FONTS
        .get_or_init(|| {
            let mut db = resvg::usvg::fontdb::Database::new();
            db.load_system_fonts();
            std::sync::Arc::new(db)
        })
        .clone()
}

/// An SVG picture drawn as a PNG `width` × `height` pixels, transparent where it draws nothing:
/// a PDF's stand-in for it (`PdfAsset.raster`; the core draws no SVG), drawn by resvg as the
/// screen draws it. None for a file resvg cannot read.
pub fn svg_png(bytes: &[u8], width: u32, height: u32, dpi: u16) -> Option<Vec<u8>> {
    use resvg::{tiny_skia, usvg};
    let options = usvg::Options {
        fontdb: svg_fonts(),
        ..usvg::Options::default()
    };
    let tree = usvg::Tree::from_data(bytes, &options).ok()?;
    let size = tree.size();
    let mut pixmap = tiny_skia::Pixmap::new(width.max(1), height.max(1))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(
            pixmap.width() as f32 / size.width(),
            pixmap.height() as f32 / size.height(),
        ),
        &mut pixmap.as_mut(),
    );
    let rgba: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    let mut out = Vec::new();
    {
        let mut e = png::Encoder::new(&mut out, pixmap.width(), pixmap.height());
        e.set_color(png::ColorType::Rgba);
        e.set_depth(png::BitDepth::Eight);
        let ppm = (f64::from(dpi.max(1)) / 0.0254).round() as u32;
        e.set_pixel_dims(Some(png::PixelDimensions {
            xppu: ppm,
            yppu: ppm,
            unit: png::Unit::Meter,
        }));
        let mut w = e.write_header().ok()?;
        w.write_image_data(&rgba).ok()?;
    }
    Some(out)
}

/// A PNG, JPEG or SVG file's picture; none for anything else or a broken file.
pub fn decode(bytes: &[u8]) -> Option<Raster> {
    if bytes.starts_with(b"\x89PNG") {
        decode_png(bytes)
    } else if bytes.starts_with(&[0xff, 0xd8]) {
        decode_jpeg(bytes)
    } else if is_svg("", bytes) {
        decode_svg(bytes)
    } else {
        None
    }
}

fn decode_png(bytes: &[u8]) -> Option<Raster> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    let px = (w as usize) * (h as usize);
    let src = buf.get(..info.buffer_size())?;
    let rgba = match info.color_type {
        png::ColorType::Rgba => src.to_vec(),
        png::ColorType::Rgb => src
            .chunks_exact(3)
            .flat_map(|c| [c[0], c[1], c[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => src
            .chunks_exact(2)
            .flat_map(|c| [c[0], c[0], c[0], c[1]])
            .collect(),
        png::ColorType::Grayscale => src.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    if rgba.len() != px * 4 {
        return None;
    }
    Raster::from_rgba(w, h, rgba)
}

fn decode_jpeg(bytes: &[u8]) -> Option<Raster> {
    use zune_jpeg::JpegDecoder;
    use zune_jpeg::zune_core::bytestream::ZCursor;
    use zune_jpeg::zune_core::colorspace::ColorSpace;
    use zune_jpeg::zune_core::options::DecoderOptions;
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    // The decoder's own limits (16 384 pixels a side) keep a hostile file from asking for unbounded memory.
    decoder.decode_headers().ok()?;
    let (w, h) = decoder.dimensions()?;
    let mut rgba = vec![0; decoder.output_buffer_size()?];
    decoder.decode_into(&mut rgba).ok()?;
    let (width, height) = (u32::try_from(w).ok()?, u32::try_from(h).ok()?);
    if rgba.len() != w * h * 4 {
        return None;
    }
    Raster::from_rgba(width, height, rgba)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_of(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut e = png::Encoder::new(&mut out, w, h);
            e.set_color(png::ColorType::Rgba);
            e.set_depth(png::BitDepth::Eight);
            let mut wr = e.write_header().unwrap();
            wr.write_image_data(rgba).unwrap();
        }
        out
    }

    #[test]
    fn a_png_reads_into_a_texture_and_its_halvings() {
        // Left half red, right half transparent.
        let mut px = Vec::new();
        for _y in 0..4 {
            for x in 0..4 {
                px.extend(if x < 2 {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 255, 0]
                });
            }
        }
        let r = decode(&png_of(4, 4, &px)).unwrap();
        assert_eq!((r.width, r.height), (4, 4));
        assert_eq!(r.rgba(), Some(&px[..]));
        assert!(r.svg().is_none());
        assert!(decode(b"GIF89a").is_none());
        assert!(decode(b"\x89PNG broken").is_none());
    }

    /// Halved again and again, each pixel the average of the four it covers, the transparent
    /// ones not darkening it; the texture drawn is the smallest at least twice the size shown.
    #[test]
    fn a_large_picture_is_halved_and_drawn_from_the_texture_near_its_size() {
        let (w, h) = (200, 120);
        let mut px = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            for x in 0..w {
                px.extend(if (x + y) % 2 == 0 {
                    [0, 0, 0, 255]
                } else {
                    [255, 255, 255, 255]
                });
            }
        }
        let r = Raster::from_rgba(w as u32, h as u32, px).unwrap();
        assert_eq!(r.sizes(), [(200, 120), (100, 60), (50, 30), (25, 15)]);
        // The checker halved is mid grey.
        let (_, _, half) = halve(
            4,
            2,
            &[
                0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255,
                255, 0, 0, 0, 255, 255, 255, 255, 255, 0, 0, 0, 255,
            ],
        );
        assert_eq!(&half[..4], &[128, 128, 128, 255]);
        // Red beside transparent: red at half opacity, not dark red.
        let (_, _, edge) = halve(2, 1, &[255, 0, 0, 255, 0, 0, 255, 0]);
        assert_eq!(edge, [255, 0, 0, 128]);
        let size = |w: f32, h: f32| {
            r.texture_for(w, h)
                .and_then(|t| r.levels.iter().find(|l| l.handle == t))
                .map(|l| (l.width, l.height))
        };
        assert_eq!(size(20.0, 12.0), Some((50, 30)));
        assert_eq!(size(60.0, 36.0), Some((200, 120)));
        assert_eq!(size(5.0, 3.0), Some((25, 15)), "the smallest there is");
        // Magnified: its own size magnified by the whole factor (2.5 → 3), smoothed; kept.
        let big = r.texture_for(500.0, 300.0).expect("a texture");
        match &big {
            ImageHandle::Rgba { width, height, .. } => assert_eq!((*width, *height), (600, 360)),
            other => panic!("{other:?}"),
        }
        assert_eq!(r.texture_for(500.0, 300.0), Some(big));
        // At most four kept.
        for m in 3..12 {
            r.texture_for(200.0 * m as f32, 120.0 * m as f32);
        }
        assert_eq!(r.magnified.borrow().len(), MAGNIFIED_KEPT);
        // Bilinear: between a red and a blue pixel, purple; the edges held.
        let up = magnify(2, 1, &[200, 0, 0, 255, 0, 0, 200, 255], 4);
        assert_eq!(&up[..4], &[200, 0, 0, 255]);
        assert_eq!(&up[(3 * 4)..(4 * 4)], &[125, 0, 75, 255]);
        assert_eq!(up.len(), 8 * 4 * 4, "8 × 4");
        assert_eq!(&up[(7 * 4)..(8 * 4)], &[0, 0, 200, 255]);
    }

    /// An SVG drawn for a PDF: a PNG of the size asked, its red disc red, its corners clear.
    #[test]
    fn an_svg_is_drawn_as_a_png_for_a_pdf() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10"><circle cx="5" cy="5" r="4" fill="#c00000"/></svg>"##;
        let png = svg_png(svg, 40, 40, 150).expect("a PNG");
        let r = decode(&png).expect("it reads");
        assert_eq!((r.width, r.height), (40, 40));
        let px = r.rgba().expect("pixels");
        let at = |x: usize, y: usize| &px[(y * 40 + x) * 4..(y * 40 + x) * 4 + 4];
        assert_eq!(at(20, 20), &[192, 0, 0, 255]);
        assert_eq!(at(0, 0)[3], 0, "transparent where it draws nothing");
        assert!(svg_png(b"<svg", 4, 4, 150).is_none());
    }

    #[test]
    fn an_svg_reads_with_its_size_and_is_a_picture() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="120" height="40" viewBox="0 0 120 40"><circle cx="20" cy="20" r="18" fill="#c0392b"/></svg>"##;
        let r = decode(svg).expect("an SVG");
        assert_eq!((r.width, r.height, r.rgba()), (120, 40, None));
        assert!(r.svg().is_some());
        let meta = asset_of("logo.svg", svg).expect("a picture");
        assert_eq!(
            (meta.kind, meta.width, meta.height),
            (AssetKind::Svg, 120, 40)
        );
        // In millimetres it is measured as the file says (resvg's pixels, 96 to the inch).
        let mm = br#"<?xml version="1.0"?><svg xmlns="http://www.w3.org/2000/svg" width="25.4mm" height="12.7mm"/>"#;
        assert_eq!(decode(mm).map(|r| (r.width, r.height)), Some((96, 48)));
        assert!(asset_of("bozuk.svg", b"<svg").is_err());
        assert_eq!(
            asset_of("not.txt", b"merhaba").unwrap_err(),
            "“not.txt” resim değil: PNG, JPEG ya da SVG seçin."
        );
    }
}
