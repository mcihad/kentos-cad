//! Where the styled atlas gets its pictures (docs/adr/0090): library SVG
//! drawings read into vector pictures (`svg.rs`), PNG and JPEG images decoded, and
//! texts as glyph outlines from Iced's text system, in the faces the web's
//! font stacks name (Arial and Times are Liberation Sans and Serif on Linux,
//! as the browser's fontconfig aliases them; Arimo, which ships with KentOS,
//! when a face is missing). Everything is kept by key: a picture is read once.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex, PoisonError};

use iced::Font;
use iced::advanced::graphics::text::{self as gtext, Paragraph, cosmic_text};
use iced::advanced::text::{self, Paragraph as _};
use iced::alignment::Vertical;
use iced::font::{Family, Style, Weight};
use iced::{Pixels, Size};
use kentos_native_style::batches::AtlasImage;
use kentos_render_wgpu::styled::picture::{ImageSource, Picture, Segment, TextOutline};

/// Letters are outlined at this size and scaled.
const REFERENCE: f32 = 100.0;

type TextKey = (String, Option<String>, u16, bool);

/// The atlas's pictures and glyphs, read on first use and kept; the
/// drawing's picture objects' pixels as the app hands them (docs/adr/0192 §3).
#[derive(Default)]
pub struct Images {
    pictures: Mutex<HashMap<String, Option<Arc<Picture>>>>,
    bitmaps: Mutex<HashMap<String, Option<Arc<kentos_render_wgpu::styled::Bitmap>>>>,
    texts: Mutex<HashMap<TextKey, Option<Arc<TextOutline>>>>,
    faces: Mutex<HashMap<String, &'static str>>,
}

impl std::fmt::Debug for Images {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Images").finish_non_exhaustive()
    }
}

/// The family a CSS font stack draws with: the first name the text system
/// has, Linux's metric twins for Arial and Times, Arimo when none is there.
fn face_of(stack: &str, has: &dyn Fn(&str) -> bool) -> &'static str {
    const KNOWN: [&str; 8] = [
        "Liberation Sans",
        "Liberation Sans Narrow",
        "Liberation Serif",
        "Arimo",
        "Barlow",
        "IBM Plex Mono",
        "Tinos",
        "DejaVu Serif",
    ];
    for name in stack.split(',') {
        let name = name.trim().trim_matches(|c| c == '"' || c == '\'');
        let wanted: &[&str] = match name {
            "Arial" | "Helvetica" | "Liberation Sans" => &["Liberation Sans", "Arimo"],
            "Arimo" => &["Arimo"],
            "Arial Narrow" | "Liberation Sans Narrow" => &["Liberation Sans Narrow"],
            "Times New Roman" | "Times" | "Liberation Serif" => &["Liberation Serif"],
            "Tinos" => &["Tinos"],
            "serif" => &["Liberation Serif", "DejaVu Serif"],
            "Barlow" => &["Barlow"],
            "IBM Plex Mono" | "monospace" => &["IBM Plex Mono"],
            "sans-serif" => &["Arimo"],
            _ => &[],
        };
        if let Some(found) = wanted.iter().find(|w| has(w)) {
            return KNOWN
                .iter()
                .find(|k| *k == found)
                .copied()
                .unwrap_or("Arimo");
        }
    }
    "Arimo"
}

fn weight_of(weight: f64) -> Weight {
    match weight as u32 {
        0..=149 => Weight::Thin,
        150..=249 => Weight::ExtraLight,
        250..=349 => Weight::Light,
        350..=449 => Weight::Normal,
        450..=549 => Weight::Medium,
        550..=649 => Weight::Semibold,
        650..=749 => Weight::Bold,
        750..=849 => Weight::ExtraBold,
        _ => Weight::Black,
    }
}

/// A text's glyph outlines in a face, the baseline at y = 0.
fn outline(content: &str, font: Font) -> Option<TextOutline> {
    let paragraph = Paragraph::with_text(text::Text {
        content,
        bounds: Size::INFINITE,
        size: Pixels(REFERENCE),
        line_height: text::LineHeight::Relative(1.0),
        font,
        align_x: text::Alignment::Left,
        align_y: Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
    });
    let buffer = paragraph.buffer();
    let mut cache = cosmic_text::SwashCache::new();
    let mut system = gtext::font_system()
        .write()
        .unwrap_or_else(PoisonError::into_inner);
    let mut glyphs = Vec::new();
    let mut advance: f32 = 0.0;
    // One line: the first run is the text.
    if let Some(run) = buffer.layout_runs().next() {
        advance = run.line_w;
        for glyph in run.glyphs {
            let physical = glyph.physical((0.0, 0.0), 1.0);
            let (x, y) = (glyph.x + glyph.x_offset, glyph.y_offset);
            let Some(commands) = cache.get_outline_commands(system.raw(), physical.cache_key)
            else {
                continue;
            };
            // Font units are y up with the origin on the baseline; the picture is y down.
            let p = |qx: f32, qy: f32| -> [f32; 2] { [qx + x, -qy + y] };
            let segments = commands
                .iter()
                .map(|c| match c {
                    cosmic_text::Command::MoveTo(a) => Segment::Move(p(a.x, a.y)),
                    cosmic_text::Command::LineTo(a) => Segment::Line(p(a.x, a.y)),
                    cosmic_text::Command::QuadTo(c, a) => Segment::Quad(p(c.x, c.y), p(a.x, a.y)),
                    cosmic_text::Command::CurveTo(c1, c2, a) => {
                        Segment::Cubic(p(c1.x, c1.y), p(c2.x, c2.y), p(a.x, a.y))
                    }
                    cosmic_text::Command::Close => Segment::Close,
                })
                .collect();
            glyphs.push(segments);
        }
    }
    Some(TextOutline {
        size: REFERENCE,
        advance,
        glyphs,
    })
}

/// `data:image/png;base64,…` → the file's bytes.
pub(crate) fn data_url(url: &str) -> Option<(&str, Vec<u8>)> {
    let rest = url.strip_prefix("data:")?;
    let (head, body) = rest.split_once(',')?;
    let mime = head.split(';').next().unwrap_or("");
    if !head.ends_with(";base64") {
        return None;
    }
    base64(body).map(|b| (mime, b))
}

/// Standard base64, whitespace ignored; None when it is not.
fn base64(text: &str) -> Option<Vec<u8>> {
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

impl Images {
    pub fn new() -> Images {
        Images::default()
    }

    /// Whether the picture `key` was handed in (with pixels or none).
    pub fn has_bitmap(&self, key: &str) -> bool {
        self.bitmaps
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .contains_key(key)
    }

    /// Hands in a picture's pixels, or none when it cannot be read.
    pub fn put_bitmap(&self, key: String, bitmap: Option<Arc<kentos_render_wgpu::styled::Bitmap>>) {
        self.bitmaps
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, bitmap);
    }

    fn family(&self, stack: Option<&str>) -> &'static str {
        let stack = stack.unwrap_or("Barlow, sans-serif");
        let mut faces = self.faces.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(f) = faces.get(stack) {
            return f;
        }
        let face = {
            let mut system = gtext::font_system()
                .write()
                .unwrap_or_else(PoisonError::into_inner);
            let db = system.raw().db();
            face_of(stack, &|name: &str| {
                db.faces()
                    .any(|f| f.families.iter().any(|(n, _)| n == name))
            })
        };
        faces.insert(stack.to_owned(), face);
        face
    }

    /// The picture of an SVG text (the atlas's and the previews'), kept by `key`.
    pub fn svg(&self, key: &str, svg: &str) -> Option<Arc<Picture>> {
        if let Some(p) = self
            .pictures
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
        {
            return p.clone();
        }
        let glyphs = |t: &str, f: Option<&str>, w: f64, i: bool| self.text(t, f, w, i);
        let read = super::svg::picture(svg, &glyphs).map(Arc::new);
        self.pictures
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key.to_owned(), read.clone());
        read
    }

    fn raster(&self, key: &str, url: &str) -> Option<Arc<Picture>> {
        if let Some(p) = self
            .pictures
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
        {
            return p.clone();
        }
        let read = data_url(url)
            .and_then(|(mime, bytes)| match mime {
                "image/png" => png(&bytes),
                "image/jpeg" => jpeg(&bytes),
                _ => None,
            })
            .map(Arc::new);
        self.pictures
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key.to_owned(), read.clone());
        read
    }
}

impl ImageSource for Images {
    fn bitmap(&self, key: &str) -> Option<Arc<kentos_render_wgpu::styled::Bitmap>> {
        self.bitmaps
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(key)
            .cloned()
            .flatten()
    }

    fn picture(&self, image: &AtlasImage) -> Option<Arc<Picture>> {
        match image {
            AtlasImage::Svg { key, svg, .. } => self.svg(key, svg),
            AtlasImage::Raster { key, url, .. } => self.raster(key, url),
            _ => None,
        }
    }

    fn text(
        &self,
        content: &str,
        font: Option<&str>,
        weight: f64,
        italic: bool,
    ) -> Option<Arc<TextOutline>> {
        let key: TextKey = (
            content.to_owned(),
            font.map(str::to_owned),
            weight as u16,
            italic,
        );
        if let Some(t) = self
            .texts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&key)
        {
            return t.clone();
        }
        let family = self.family(font);
        let face = Font {
            family: Family::Name(family),
            weight: weight_of(weight),
            style: if italic { Style::Italic } else { Style::Normal },
            ..Font::DEFAULT
        };
        let read = outline(content, face).map(Arc::new);
        self.texts
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, read.clone());
        read
    }
}

/// A 16 × 8 JPEG for the tests: the left half #E15759, the right #4E79A7
/// (quality 95, no chroma subsampling; written with Pillow).
#[cfg(test)]
pub(crate) const TEST_JPEG: &str = "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAIBAQEBAQIBAQECAgICAgQDAgICAgUEBAMEBgUGBgYFBgYGBwkIBgcJBwYGCAsI\
CQoKCgoKBggLDAsKDAkKCgr/2wBDAQICAgICAgUDAwUKBwYHCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoKCgoK\
CgoKCgoKCgoKCgoKCgr/wAARCAAIABADAREAAhEBAxEB/8QAFQABAQAAAAAAAAAAAAAAAAAAAAb/xAAUEAEAAAAAAAAAAAAA\
AAAAAAAA/8QAFgEBAQEAAAAAAAAAAAAAAAAACQcI/8QAFBEBAAAAAAAAAAAAAAAAAAAAAP/aAAwDAQACEQMRAD8AJORRBtoC\
Hf/Z";

/// [`TEST_JPEG`]'s bytes.
#[cfg(test)]
pub(crate) fn test_jpeg() -> Vec<u8> {
    base64(TEST_JPEG).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stacks_find_the_metric_twins() {
        let linux = |n: &str| {
            matches!(
                n,
                "Liberation Sans" | "Liberation Serif" | "Arimo" | "Barlow"
            )
        };
        assert_eq!(
            face_of(
                "Arial, \"Liberation Sans\", Arimo, Helvetica, sans-serif",
                &linux
            ),
            "Liberation Sans"
        );
        assert_eq!(
            face_of(
                "\"Times New Roman\", \"Liberation Serif\", Tinos, Times, serif",
                &linux
            ),
            "Liberation Serif"
        );
        let bare = |n: &str| n == "Arimo";
        assert_eq!(
            face_of(
                "Arial, \"Liberation Sans\", Arimo, Helvetica, sans-serif",
                &bare
            ),
            "Arimo"
        );
        assert_eq!(face_of("\"Times New Roman\", Times, serif", &bare), "Arimo");
    }

    #[test]
    fn reads_a_jpeg() {
        let bytes = test_jpeg();
        let Some(Picture::Bitmap {
            width,
            height,
            rgba,
        }) = jpeg(&bytes)
        else {
            panic!("a bitmap");
        };
        assert_eq!((width, height), (16, 8));
        let px = |x: usize, y: usize| &rgba[(y * 16 + x) * 4..][..4];
        // Lossy, so near the colours; opaque.
        let near = |p: &[u8], c: [u8; 3]| {
            p[..3].iter().zip(c).all(|(a, b)| a.abs_diff(b) <= 12) && p[3] == 255
        };
        assert!(near(px(2, 4), [0xE1, 0x57, 0x59]), "{:?}", px(2, 4));
        assert!(near(px(13, 4), [0x4E, 0x79, 0xA7]), "{:?}", px(13, 4));
        // The atlas reads it from the library's data address.
        let url = format!("data:image/jpeg;base64,{TEST_JPEG}");
        assert!(Images::new().raster("foto", &url).is_some());
        // A cut file is no picture, and no panic.
        assert!(jpeg(&bytes[..60]).is_none());
        assert!(jpeg(b"\xFF\xD8").is_none());
    }

    #[test]
    fn reads_base64() {
        assert_eq!(base64("aGVsbG8=").as_deref(), Some(&b"hello"[..]));
        assert_eq!(
            data_url("data:image/png;base64,aGk=").map(|(m, b)| (m.to_owned(), b)),
            Some(("image/png".to_owned(), b"hi".to_vec()))
        );
        assert!(base64("*").is_none());
    }
}
