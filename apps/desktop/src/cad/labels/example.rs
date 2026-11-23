//! A label as its style draws it, for the Etiketler window (docs/adr/9186): its example text in the middle of a strip
//! of the drawing area's ground, with the map's own look (`placed_look`: colour, halo, background, shadow and letters in
//! the project's typeface), so the window shows what the map will.

use super::placed::{backdrop_at, placed_look, text_at};
use super::*;

/// The largest size the strip draws a label at: a larger one is drawn at it.
const LARGEST: f32 = 40.0;
/// The strip's height.
const HEIGHT: f32 = 64.0;

/// The strip's picture: the text, its style and the canvas's colours and typeface.
struct Example {
    text: String,
    style: kentos_commands::LabelStyle,
    colors: Colors,
    font: DrawingFont,
}

impl canvas::Program<Message> for Example {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), color(self.colors.palette.background));
        let c = placed_look(&self.colors, self.font, &self.style, 0, Slot(0), false);
        let size = (self.style.size as f32).clamp(SMALLEST, LARGEST);
        let width = measure(&self.text, c.font).width * size / REFERENCE;
        let at = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        backdrop_at(&mut frame, &c, at, 0.0, width, size * 1.2, false, self.colors.accent());
        text_at(&mut frame, &c, &self.text, at, 0.0, size);
        vec![frame.into_geometry()]
    }
}

/// `text` labelled with `style` on a strip of the canvas's colours (`colors`) in the project's typeface.
pub fn example<'a>(
    text: String,
    style: kentos_commands::LabelStyle,
    colors: Colors,
    font: DrawingFont,
) -> Element<'a, Message> {
    canvas::Canvas::new(Example { text, style, colors, font })
        .width(Fill)
        .height(kentos_ui::theme::typography::from_default(HEIGHT))
        .into()
}
