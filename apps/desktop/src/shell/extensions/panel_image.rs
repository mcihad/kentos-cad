//! A panel's pictures (docs/adr/8057 §40, §115): an `image` component shows the PNG or the JPEG its extension's code
//! gives (`ctx.panel.set(id, image=…)`): a chart drawn in Python, a preview, a record's photo. The code's bytes come as
//! base64 over the broker and are checked before anything is drawn.
//!
//! - **PNG and JPEG:** a chart and a drawing want PNG's lossless pixels, a photo comes as JPEG (KentOS's own decoder,
//!   zune-jpeg, docs/adr/0092). The size is read from the header before the picture is decoded, so one asking for
//!   more than [`MOST_SIDE`] pixels a side or [`MOST_PIXELS`] in all is refused unread (a hostile file cannot ask for
//!   unbounded memory).
//! - **Fitted, not stretched:** the picture keeps its proportions inside the component's height; its `alt` stands in
//!   its place until the code gives one.

use iced::widget::{column, container, image};
use iced::{ContentFit, Element, Fill, Length};
use kentos_ui::label;

use super::panels::OpenPanel;
use crate::app::Message;

/// The most pixels a side of a panel's picture.
pub(super) const MOST_SIDE: u32 = 4096;
/// The most pixels of a panel's picture in all (8 megapixels).
pub(super) const MOST_PIXELS: u64 = 8 * 1024 * 1024;
/// The most base64 text a picture comes as (about 6 MB of PNG).
const MOST_TEXT: usize = 8_000_000;
/// A picture's height when its component does not say, and its bounds (Kapı 1 says the same).
const HEIGHT: f64 = 200.0;

/// A picture the code gave (base64 PNG) as KentOS draws it, or why not in the code's words.
pub(super) fn decode(text: &str) -> Result<image::Handle, String> {
    if text.len() > MOST_TEXT {
        return Err("görsel çok büyük: en çok yaklaşık 6 MB PNG olur".to_owned());
    }
    let bytes = kentos_native_style::bitmap::base64(text)
        .ok_or_else(|| "image base64 kodlanmış PNG ya da JPEG baytları olmalı".to_owned())?;
    if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return jpeg(&bytes);
    }
    if bytes.len() < 24 || !bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Err("image bir PNG ya da JPEG olmalı (başka biçimler alınmaz)".to_owned());
    }
    // IHDR comes first: its width and height, read before a byte is decoded.
    let side =
        |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    within(side(16), side(20))?;
    match kentos_native_style::bitmap::png(&bytes) {
        Some(kentos_native_style::picture::Picture::Bitmap {
            width,
            height,
            rgba,
        }) => Ok(image::Handle::from_rgba(width, height, rgba)),
        _ => Err("PNG okunamadı: bozuk ya da desteklenmeyen bir PNG".to_owned()),
    }
}

/// A picture's size within the bounds, or why not.
fn within(width: u32, height: u32) -> Result<(), String> {
    if width == 0
        || height == 0
        || width > MOST_SIDE
        || height > MOST_SIDE
        || u64::from(width) * u64::from(height) > MOST_PIXELS
    {
        return Err(format!(
            "görsel {width} × {height} piksel: kenarı en çok {MOST_SIDE}, toplamı en çok 8 milyon piksel olur"
        ));
    }
    Ok(())
}

/// A JPEG (a photo): its size read from its frame header first, decoded only within the bounds.
fn jpeg(bytes: &[u8]) -> Result<image::Handle, String> {
    use zune_jpeg::JpegDecoder;
    use zune_jpeg::zune_core::bytestream::ZCursor;
    let broken = || "JPEG okunamadı: bozuk ya da desteklenmeyen bir JPEG".to_owned();
    let mut header = JpegDecoder::new(ZCursor::new(bytes));
    header.decode_headers().map_err(|_| broken())?;
    let (w, h) = header.dimensions().ok_or_else(broken)?;
    within(
        u32::try_from(w).unwrap_or(u32::MAX),
        u32::try_from(h).unwrap_or(u32::MAX),
    )?;
    match kentos_native_style::bitmap::jpeg(bytes) {
        Some(kentos_native_style::picture::Picture::Bitmap {
            width,
            height,
            rgba,
        }) => Ok(image::Handle::from_rgba(width, height, rgba)),
        _ => Err(broken()),
    }
}

/// An `image` component: its label, then its picture fitted in its height, or its `alt` until the code gives one.
pub(super) fn panel_image<'a>(
    p: &'a OpenPanel,
    id: &str,
    words: Option<&'a str>,
    alt: Option<&'a str>,
    height: Option<f64>,
) -> Element<'a, Message> {
    let tall = height.unwrap_or(HEIGHT).clamp(80.0, 600.0) as f32;
    let picture: Element<'a, Message> = match p.images.get(id) {
        // As wide as the panel, as tall as its proportions make it, at most its height: no band above or under it.
        Some(handle) => container(
            image(handle.clone())
                .content_fit(ContentFit::Contain)
                .width(Fill)
                .height(Length::Shrink),
        )
        .max_height(tall)
        .into(),
        None => container(label::muted(
            alt.unwrap_or("Görsel yok: eklentinin kodu verince burada görünür."),
        ))
        .width(Fill)
        .into(),
    };
    match words {
        Some(words) => column![label::caption(words.to_owned()), picture]
            .spacing(4)
            .into(),
        None => picture,
    }
}

/// A PNG of `width` × `height` grey pixels as base64, as an extension's code sends one (tests).
#[cfg(test)]
pub(super) fn test_png(width: u32, height: u32) -> String {
    let mut bytes = Vec::new();
    {
        let mut encoder = ::png::Encoder::new(&mut bytes, width, height);
        encoder.set_color(::png::ColorType::Grayscale);
        encoder.set_depth(::png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("a header");
        writer
            .write_image_data(&vec![128; (width * height) as usize])
            .expect("the pixels");
    }
    test_base64(&bytes)
}

/// Standard base64 (tests).
#[cfg(test)]
pub(super) fn test_base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ABC[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An 8 × 4 photo, as a camera writes it (made with Pillow, quality 70): 633 bytes.
    const SMALL_JPEG: &str = concat!(
        "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAoHBwgHBgoICAgLCgoLDhgQDg0NDh0VFhEYIx8lJCIfIiEmKzcvJik0KSEiMEEx",
        "NDk7Pj4+JS5ESUM8SDc9Pjv/2wBDAQoLCw4NDhwQEBw7KCIoOzs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7Ozs7",
        "Ozs7Ozs7Ozs7Ozs7Ozv/wAARCAAEAAgDASIAAhEBAxEB/8QAHwAAAQUBAQEBAQEAAAAAAAAAAAECAwQFBgcICQoL/8QAtRAA",
        "AgEDAwIEAwUFBAQAAAF9AQIDAAQRBRIhMUEGE1FhByJxFDKBkaEII0KxwRVS0fAkM2JyggkKFhcYGRolJicoKSo0NTY3ODk6",
        "Q0RFRkdISUpTVFVWV1hZWmNkZWZnaGlqc3R1dnd4eXqDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXG",
        "x8jJytLT1NXW19jZ2uHi4+Tl5ufo6erx8vP09fb3+Pn6/8QAHwEAAwEBAQEBAQEBAQAAAAAAAAECAwQFBgcICQoL/8QAtREA",
        "AgECBAQDBAcFBAQAAQJ3AAECAxEEBSExBhJBUQdhcRMiMoEIFEKRobHBCSMzUvAVYnLRChYkNOEl8RcYGRomJygpKjU2Nzg5",
        "OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6goOEhYaHiImKkpOUlZaXmJmaoqOkpaanqKmqsrO0tba3uLm6wsPE",
        "xcbHyMnK0tPU1dbX2Nna4uPk5ebn6Onq8vP09fb3+Pn6/9oADAMBAAIRAxEAPwDmaKKK8g/RD//Z",
    );

    /// A PNG the code gives is drawn; another format, text that is not base64, a picture past the bounds (read from
    /// its header, not decoded) and a broken PNG are refused with the code's words.
    #[test]
    fn a_png_is_drawn_and_anything_else_is_refused_before_it_is_decoded() {
        assert!(decode(&test_png(64, 32)).is_ok());
        assert!(
            decode(&test_base64(b"GIF89a not a png nor a jpeg, a gif's start"))
                .unwrap_err()
                .contains("PNG ya da JPEG olmalı")
        );
        assert!(decode("bu base64 değil!").unwrap_err().contains("base64"));
        let wide = decode(&test_png(5000, 1)).unwrap_err();
        assert!(wide.contains("5000 × 1"), "{wide}");
        // A header that asks for a huge picture over a few bytes: refused unread.
        let mut bytes = vec![
            0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
        ];
        bytes.extend(3000u32.to_be_bytes());
        bytes.extend(3000u32.to_be_bytes());
        bytes.extend([8, 0, 0, 0, 0]);
        assert!(
            decode(&test_base64(&bytes))
                .unwrap_err()
                .contains("3000 × 3000")
        );
        let mut broken = kentos_native_style::bitmap::base64(&test_png(8, 8)).unwrap();
        broken.truncate(40);
        assert!(
            decode(&test_base64(&broken))
                .unwrap_err()
                .contains("okunamadı")
        );
    }
    /// A photo (JPEG) is drawn; one asking for more than the bounds is refused from its frame header, unread; a
    /// broken one says so (docs/adr/8057 §115).
    #[test]
    fn a_jpeg_is_drawn_and_one_past_the_bounds_is_refused_unread() {
        assert!(decode(SMALL_JPEG).is_ok());
        let mut bytes = kentos_native_style::bitmap::base64(SMALL_JPEG).unwrap();
        let sof = bytes
            .windows(2)
            .position(|w| w == [0xFF, 0xC0])
            .expect("its frame header");
        // The frame header's height and width, after its length and precision.
        bytes[sof + 5..sof + 7].copy_from_slice(&2u16.to_be_bytes());
        bytes[sof + 7..sof + 9].copy_from_slice(&5000u16.to_be_bytes());
        let wide = decode(&test_base64(&bytes)).unwrap_err();
        assert!(wide.contains("5000 × 2"), "{wide}");
        let mut broken = kentos_native_style::bitmap::base64(SMALL_JPEG).unwrap();
        broken.truncate(30);
        assert!(
            decode(&test_base64(&broken))
                .unwrap_err()
                .contains("JPEG okunamadı")
        );
    }
}
