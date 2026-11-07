//! The supplied KentOS wordmarks and application icon, embedded once and shared by all native surfaces.
//!
//! The wordmark and the mark are drawn from pictures made once at the sizes the apps draw them, at one and two device
//! pixels each (`scripts/assets/brand_sizes.py`: trimmed, filtered in linear light with Lanczos-3 over premultiplied
//! alpha); a size is chosen by the window's device pixels and drawn pixel for pixel. The large originals were drawn
//! small by the GPU, which samples bilinearly without mipmaps, and their edges came out jagged. The window icon still
//! takes the large original: the system scales it.
use crate::theme::Mode;
use iced::widget::canvas::{self, Frame, Geometry};
use iced::widget::image::Handle;
use iced::{Element, Rectangle, Renderer, Size, Theme, mouse};
use std::io::Cursor;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};

pub const APP_ICON_PNG: &[u8] = include_bytes!("../../../../assets/images/kentos_app_icon.png");

macro_rules! sized {
    ($name:literal; $($w:literal),+) => {
        [$(($w, include_bytes!(concat!("../../../../assets/images/brand/", $name, "_", stringify!($w), ".png")).as_slice())),+]
    };
}

/// The wordmarks' widths and the mark's sides, in device pixels; each list grows.
const LIGHT: [(u32, &[u8]); 8] = sized!("kentos_light_logo"; 100, 150, 160, 200, 260, 300, 320, 520);
const DARK: [(u32, &[u8]); 8] = sized!("kentos_dark_logo"; 100, 150, 160, 200, 260, 300, 320, 520);
const MARK: [(u32, &[u8]); 4] = sized!("kentos_app_icon"; 20, 28, 40, 56);

/// Device pixels per logical pixel of the window the brand is drawn in: two (a Retina screen) until the app says.
static DEVICE_SCALE: AtomicU32 = AtomicU32::new(0x4000_0000);

/// The window's device pixels per logical pixel (iced's scale factor at opening and at each `Rescaled`): the sizes
/// are chosen by it. A value that is not a positive number is ignored.
pub fn set_device_scale(scale: f32) {
    if scale.is_finite() && scale > 0.0 {
        DEVICE_SCALE.store(scale.to_bits(), Ordering::Relaxed);
    }
}

fn device_scale() -> f32 {
    f32::from_bits(DEVICE_SCALE.load(Ordering::Relaxed))
}

struct Picture {
    width: u32,
    height: u32,
    handle: Handle,
}

fn decode(bytes: &[u8]) -> Option<Picture> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    let source = &buffer[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => source.to_vec(),
        png::ColorType::Rgb => source.chunks_exact(3).flat_map(|p| [p[0], p[1], p[2], 255]).collect(),
        _ => return None,
    };
    let (width, height) = (info.width, info.height);
    Some(Picture { width, height, handle: Handle::from_rgba(width, height, rgba) })
}

/// The sizes of a picture: the mark's, or the wordmark's for the theme.
fn sizes(logo: bool, dark: bool) -> (&'static [(u32, &'static [u8])], &'static [OnceLock<Option<Picture>>]) {
    static LIGHT_PICTURES: [OnceLock<Option<Picture>>; 8] = [const { OnceLock::new() }; 8];
    static DARK_PICTURES: [OnceLock<Option<Picture>>; 8] = [const { OnceLock::new() }; 8];
    static MARK_PICTURES: [OnceLock<Option<Picture>>; 4] = [const { OnceLock::new() }; 4];
    match (logo, dark) {
        (false, _) => (&MARK, &MARK_PICTURES),
        (true, true) => (&DARK, &DARK_PICTURES),
        (true, false) => (&LIGHT, &LIGHT_PICTURES),
    }
}

/// The picture for `width` device pixels: the smallest made at least that wide (half a pixel spared), else the largest.
fn picture(logo: bool, dark: bool, width: f32) -> Option<&'static Picture> {
    let (files, cache) = sizes(logo, dark);
    let i = files.iter().position(|(w, _)| *w as f32 + 0.5 >= width).unwrap_or(files.len() - 1);
    cache[i].get_or_init(|| decode(files[i].1)).as_ref()
}

/// A square application mark: title bars, launchers and compact surfaces.
pub fn brand_mark<'a, M: 'a>(size: f32) -> Element<'a, M> {
    canvas::Canvas::new(Brand { logo: false }).width(size).height(size).into()
}

/// Full supplied wordmark, automatically following the actual light/dark theme.
pub fn brand_logo<'a, M: 'a>(width: f32) -> Element<'a, M> {
    canvas::Canvas::new(Brand { logo: true }).width(width).height(width / 4.).into()
}

/// Window/taskbar icon; macOS Dock/Finder additionally use the bundle's ICNS.
pub fn window_icon() -> Option<iced::window::Icon> {
    static ICON: OnceLock<Option<Picture>> = OnceLock::new();
    let p = ICON.get_or_init(|| decode(APP_ICON_PNG)).as_ref()?;
    let Handle::Rgba { pixels, .. } = &p.handle else {
        return None;
    };
    iced::window::icon::from_rgba(pixels.as_ref().to_vec(), p.width, p.height).ok()
}

struct Brand {
    logo: bool,
}

/// Where a picture goes in `bounds` (logical pixels) at `scale` device pixels each: fitted whole and centred, from
/// the picture made for the fitted width; when that picture is the fitted size to half a device pixel it is drawn at
/// its own size, pixel for pixel, its corner on a device pixel.
fn place(bounds: Size, scale: f32, aspect: f32, made: (u32, u32)) -> Rectangle {
    let fitted = if bounds.width / bounds.height <= aspect {
        Size::new(bounds.width, bounds.width / aspect)
    } else {
        Size::new(bounds.height * aspect, bounds.height)
    };
    let size = if (made.0 as f32 - fitted.width * scale).abs() <= 0.5 {
        Size::new(made.0 as f32 / scale, made.1 as f32 / scale)
    } else {
        fitted
    };
    let snap = |v: f32| (v * scale).round() / scale;
    Rectangle {
        x: snap((bounds.width - size.width) / 2.),
        y: snap((bounds.height - size.height) / 2.),
        width: size.width,
        height: size.height,
    }
}

impl<M> canvas::Program<M> for Brand {
    type State = ();
    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<Geometry> {
        let (dark, scale) = (Mode::of(theme).is_dark(), device_scale());
        // The aspect of the largest picture, the closest to the original's.
        let Some(largest) = picture(self.logo, dark, f32::INFINITY) else {
            return Vec::new();
        };
        let aspect = largest.width as f32 / largest.height as f32;
        let fitted = (bounds.width).min(bounds.height * aspect) * scale;
        let Some(p) = picture(self.logo, dark, fitted) else {
            return Vec::new();
        };
        let mut frame = Frame::new(renderer, bounds.size());
        frame.draw_image(
            place(bounds.size(), scale, aspect, (p.width, p.height)),
            canvas::Image::new(p.handle.clone()).snap(true),
        );
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_size_decodes_at_its_width_with_the_wordmark_s_aspect() {
        for (logo, dark) in [(true, false), (true, true), (false, false)] {
            let (files, _) = sizes(logo, dark);
            assert!(files.windows(2).all(|w| w[0].0 < w[1].0), "the sizes grow");
            for (w, bytes) in files {
                let p = decode(bytes).expect("a picture");
                assert_eq!(p.width, *w);
                let aspect = p.width as f32 / p.height as f32;
                if logo {
                    assert!((3.9..4.3).contains(&aspect), "{w}: {aspect}");
                } else {
                    assert_eq!(p.height, p.width);
                }
            }
        }
    }

    #[test]
    fn the_size_drawn_is_the_one_made_for_the_screen_s_pixels() {
        // The ribbon's 100-pixel wordmark on a Retina screen: the 200-pixel picture, at its own size.
        let p = picture(true, false, 200.0).expect("a picture");
        assert_eq!(p.width, 200);
        let r = place(Size::new(100.0, 25.0), 2.0, p.width as f32 / p.height as f32, (p.width, p.height));
        assert_eq!((r.width * 2.0, r.height * 2.0), (p.width as f32, p.height as f32));
        assert_eq!(((r.x * 2.0).fract(), (r.y * 2.0).fract()), (0.0, 0.0), "its corner on a device pixel");
        // On a screen of one device pixel each: the 100-pixel picture; a width between sizes takes the next above.
        assert_eq!(picture(true, false, 100.0).map(|p| p.width), Some(100));
        assert_eq!(picture(true, true, 210.0).map(|p| p.width), Some(260));
        assert_eq!(picture(false, false, 56.0).map(|p| p.width), Some(56));
        assert_eq!(picture(true, false, 4000.0).map(|p| p.width), Some(520), "past the largest: the largest");
    }

    #[test]
    fn the_device_scale_takes_only_a_positive_number() {
        let before = device_scale();
        set_device_scale(f32::NAN);
        set_device_scale(0.0);
        assert_eq!(device_scale(), before);
    }
}
