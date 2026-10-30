//! A captured print page on the same canvas image path as placed paper images.
use crate::Message;
use iced::{
    Element, Length, Point, Rectangle, Size, Theme, mouse,
    widget::canvas::{self, Frame},
};
#[derive(Debug)]
struct Tile {
    handle: iced::advanced::image::Handle,
    rect: Rectangle,
}
#[derive(Debug)]
pub(crate) struct Page {
    size: Size,
    tiles: Vec<Tile>,
}
impl Page {
    /// Iced uploads textures >=2 MB asynchronously. Bounded tiles also make the first
    /// headless frame complete, and avoid GPU texture-side limits for large paper.
    pub(crate) fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        let mut tiles = Vec::new();
        if rgba.len() == width as usize * height as usize * 4 {
            for y in (0..height).step_by(512) {
                for x in (0..width).step_by(512) {
                    let w = (width - x).min(512);
                    let h = (height - y).min(512);
                    let mut pixels = Vec::with_capacity((w * h * 4) as usize);
                    for row in y..y + h {
                        let start = ((row * width + x) * 4) as usize;
                        pixels.extend_from_slice(&rgba[start..start + (w * 4) as usize]);
                    }
                    tiles.push(Tile {
                        handle: iced::advanced::image::Handle::from_rgba(w, h, pixels),
                        rect: Rectangle::new(
                            Point::new(x as f32, y as f32),
                            Size::new(w as f32, h as f32),
                        ),
                    });
                }
            }
        }
        Self {
            size: Size::new(width as f32, height as f32),
            tiles,
        }
    }
}
impl canvas::Program<Message> for &Page {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &iced::Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let k = (bounds.width / self.size.width).min(bounds.height / self.size.height);
        let size = Size::new(self.size.width * k, self.size.height * k);
        let rect = Rectangle::new(
            Point::new(
                (bounds.width - size.width) / 2.,
                (bounds.height - size.height) / 2.,
            ),
            size,
        );
        for tile in &self.tiles {
            let r = tile.rect;
            frame.draw_image(
                Rectangle::new(
                    Point::new(rect.x + r.x * k, rect.y + r.y * k),
                    Size::new(r.width * k, r.height * k),
                ),
                canvas::Image::new(tile.handle.clone()),
            );
        }
        vec![frame.into_geometry()]
    }
}
pub(crate) fn view(page: &Page) -> Element<'_, Message> {
    canvas::Canvas::new(page)
        .width(Length::Fill)
        .height(Length::Fixed(340.))
        .into()
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_captured_page_is_visible_on_the_canvas() {
        let page = super::Page::new(794, 1123, vec![255; 794 * 1123 * 4]);
        let mut snapshot = kentos_ui::snapshot::Snapshot::new(iced::Size::new(360., 360.)).unwrap();
        let shot = snapshot.render(super::view(&page), &iced::Theme::Dark);
        let white = shot
            .rgba
            .chunks_exact(4)
            .filter(|p| p[0] > 248 && p[1] > 248 && p[2] > 248)
            .count();
        assert!(white > 70_000, "{white}");
    }
    #[test]
    fn a_captured_page_survives_dialog_layout() {
        let page = super::Page::new(794, 1123, vec![255; 794 * 1123 * 4]);
        let mut snapshot =
            kentos_ui::snapshot::Snapshot::new(iced::Size::new(1100., 720.)).unwrap();
        let dialog = kentos_ui::widget::Dialog::new("Baskı önizlemesi")
            .push(
                iced::widget::column![kentos_ui::label::caption("Kontrol"), super::view(&page)]
                    .spacing(8),
            )
            .width(700.);
        let shot = snapshot.render(
            dialog.modal(crate::Message::CloseDialog),
            &iced::Theme::Dark,
        );
        let white = shot
            .rgba
            .chunks_exact(4)
            .filter(|p| p[0] > 248 && p[1] > 248 && p[2] > 248)
            .count();
        assert!(white > 70_000, "{white}");
    }
}
