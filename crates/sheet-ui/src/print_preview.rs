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
/// The view over the page: 1 fits it whole; the offset moves the fitted page, in screen pixels.
#[derive(Debug, Default)]
pub(crate) struct Look {
    zoom: f32,
    offset: iced::Vector,
    grab: Option<Point>,
    last_click: Option<std::time::Instant>,
}
impl Look {
    fn zoom(&self) -> f32 {
        if self.zoom > 0. { self.zoom } else { 1. }
    }
}
/// The most the preview enlarges the fitted page.
const MOST: f32 = 32.;
impl Page {
    /// Where the page lies in `bounds` under `look`.
    fn placed(&self, bounds: Size, look: &Look) -> (Rectangle, f32) {
        let fit = (bounds.width / self.size.width).min(bounds.height / self.size.height);
        let k = fit * look.zoom();
        let size = Size::new(self.size.width * k, self.size.height * k);
        let at = Point::new(
            (bounds.width - size.width) / 2. + look.offset.x,
            (bounds.height - size.height) / 2. + look.offset.y,
        );
        (Rectangle::new(at, size), k)
    }
}
impl canvas::Program<Message> for &Page {
    type State = Look;
    fn update(
        &self,
        look: &mut Look,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        use iced::Event::Mouse;
        match event {
            Mouse(mouse::Event::WheelScrolled { delta }) => {
                let at = cursor.position_in(bounds)?;
                let lines = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.,
                };
                let before = look.zoom();
                let after = (before * 1.25_f32.powf(lines)).clamp(1., MOST);
                // The point under the pointer stays under it.
                let centre = Point::new(bounds.width / 2., bounds.height / 2.);
                let r = after / before;
                look.offset = iced::Vector::new(
                    (at.x - centre.x) - (at.x - centre.x - look.offset.x) * r,
                    (at.y - centre.y) - (at.y - centre.y - look.offset.y) * r,
                );
                look.zoom = after;
                if after <= 1. {
                    look.offset = iced::Vector::ZERO;
                }
                Some(canvas::Action::request_redraw().and_capture())
            }
            Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let at = cursor.position_in(bounds)?;
                let now = std::time::Instant::now();
                if look.last_click.is_some_and(|t| now.duration_since(t).as_millis() < 400) {
                    *look = Look::default();
                    return Some(canvas::Action::request_redraw().and_capture());
                }
                look.last_click = Some(now);
                look.grab = Some(at);
                Some(canvas::Action::capture())
            }
            Mouse(mouse::Event::CursorMoved { .. }) => {
                let from = look.grab?;
                let at = cursor.position_in(bounds).unwrap_or(from);
                look.offset = look.offset + (at - from);
                look.grab = Some(at);
                Some(canvas::Action::request_redraw().and_capture())
            }
            Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                look.grab.take().map(|_| canvas::Action::capture())
            }
            _ => None,
        }
    }
    fn draw(
        &self,
        look: &Look,
        renderer: &iced::Renderer,
        _: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (rect, k) = self.placed(bounds.size(), look);
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
    fn mouse_interaction(&self, look: &Look, bounds: Rectangle, cursor: mouse::Cursor) -> mouse::Interaction {
        if look.grab.is_some() {
            mouse::Interaction::Grabbing
        } else if cursor.is_over(bounds) {
            mouse::Interaction::Grab
        } else {
            mouse::Interaction::default()
        }
    }
}
pub(crate) fn view(page: &Page) -> Element<'_, Message> {
    canvas::Canvas::new(page)
        .width(Length::Fill)
        .height(Length::Fixed(540.))
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
