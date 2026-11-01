//! The supplied KentOS wordmarks and application icon, embedded once and shared by all native surfaces.
use crate::theme::Mode;
use iced::widget::canvas::{self, Frame, Geometry};
use iced::widget::image::Handle;
use iced::{Element, Rectangle, Renderer, Size, Theme, mouse};
use std::{io::Cursor, sync::OnceLock};

pub const APP_ICON_PNG: &[u8] = include_bytes!("../../../../assets/images/kentos_app_icon.png");
const LIGHT: &[u8] = include_bytes!("../../../../assets/images/kentos_light_logo.png");
const DARK: &[u8] = include_bytes!("../../../../assets/images/kentos_dark_logo.png");

struct Picture {
    width: u32,
    height: u32,
    handle: Handle,
}
fn decode(bytes: &[u8], trim: bool) -> Option<Picture> {
    let mut decoder = png::Decoder::new(Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    let source = &buffer[..info.buffer_size()];
    let rgba: Vec<u8> = match info.color_type {
        png::ColorType::Rgba => source.to_vec(),
        png::ColorType::Rgb => source
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        _ => return None,
    };
    let (mut width, mut height) = (info.width, info.height);
    // Fit the visible wordmark, not the large transparent export margins.
    // The source PNG remains unchanged; near-transparent export noise (< 4%) does not set UI spacing.
    let rgba = if trim {
        let (mut x0, mut y0, mut x1, mut y1) = (width, height, 0, 0);
        for (i, p) in rgba.chunks_exact(4).enumerate() {
            if p[3] > 8 {
                let (x, y) = (i as u32 % width, i as u32 / width);
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x + 1);
                y1 = y1.max(y + 1);
            }
        }
        if x0 >= x1 || y0 >= y1 {
            return None;
        }
        let mut out = Vec::with_capacity(((x1 - x0) * (y1 - y0) * 4) as usize);
        for y in y0..y1 {
            out.extend_from_slice(
                &rgba[((y * width + x0) * 4) as usize..((y * width + x1) * 4) as usize],
            );
        }
        width = x1 - x0;
        height = y1 - y0;
        out
    } else {
        rgba
    };
    let handle = Handle::from_rgba(width, height, rgba);
    Some(Picture {
        width,
        height,
        handle,
    })
}
fn picture(logo: bool, dark: bool) -> Option<&'static Picture> {
    static ICON: OnceLock<Option<Picture>> = OnceLock::new();
    static LIGHT_IMAGE: OnceLock<Option<Picture>> = OnceLock::new();
    static DARK_IMAGE: OnceLock<Option<Picture>> = OnceLock::new();
    let (cache, bytes) = if !logo {
        (&ICON, APP_ICON_PNG)
    } else if dark {
        (&DARK_IMAGE, DARK)
    } else {
        (&LIGHT_IMAGE, LIGHT)
    };
    cache.get_or_init(|| decode(bytes, logo)).as_ref()
}
/// A square application mark: title bars, launchers and compact surfaces.
pub fn brand_mark<'a, M: 'a>(size: f32) -> Element<'a, M> {
    canvas::Canvas::new(Brand { logo: false })
        .width(size)
        .height(size)
        .into()
}
/// Full supplied wordmark, automatically following the actual light/dark theme.
pub fn brand_logo<'a, M: 'a>(width: f32) -> Element<'a, M> {
    canvas::Canvas::new(Brand { logo: true })
        .width(width)
        .height(width / 4.)
        .into()
}
/// Window/taskbar icon; macOS Dock/Finder additionally use the bundle's ICNS.
pub fn window_icon() -> Option<iced::window::Icon> {
    let p = picture(false, false)?;
    let Handle::Rgba { pixels, .. } = &p.handle else {
        return None;
    };
    iced::window::icon::from_rgba(pixels.as_ref().to_vec(), p.width, p.height).ok()
}
struct Brand {
    logo: bool,
}
impl<M> canvas::Program<M> for Brand {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<Geometry> {
        let Some(p) = picture(self.logo, Mode::of(theme).is_dark()) else {
            return Vec::new();
        };
        let k = (bounds.width / p.width as f32).min(bounds.height / p.height as f32);
        let size = Size::new(p.width as f32 * k, p.height as f32 * k);
        let mut frame = Frame::new(renderer, bounds.size());
        frame.draw_image(
            Rectangle {
                x: (bounds.width - size.width) / 2.,
                y: (bounds.height - size.height) / 2.,
                width: size.width,
                height: size.height,
            },
            canvas::Image::new(p.handle.clone()),
        );
        vec![frame.into_geometry()]
    }
}
