//! A label as its style draws it, for the Etiketler window (docs/adr/9186): its example text in the middle of a strip
//! of the drawing area's ground, with the map's own look (`placed_look`: colour, halo, background, shadow and letters in
//! the project's typeface), so the window shows what the map will. A point layer's label is drawn with its symbol
//! (the second round's T25): around it (top right, clear of its edge) or on it, as the placement says.

use super::placed::{backdrop_at, placed_look, text_at};
use super::*;

/// The largest size the strip draws a label at: a larger one is drawn at it.
const LARGEST: f32 = 40.0;
/// The strip's height.
const HEIGHT: f32 = 64.0;

/// A point symbol's radius in the strip.
const SYMBOL: f32 = 4.5;

/// The strip's picture: the text, its style, the canvas's colours and typeface, and a point layer's symbol colour.
struct Example {
    text: String,
    style: kentos_commands::LabelStyle,
    colors: Colors,
    font: DrawingFont,
    point: Option<iced::Color>,
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
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            color(self.colors.palette.background),
        );
        let c = placed_look(&self.colors, self.font, &self.style, 0, Slot(0), false);
        let size = (self.style.size as f32).clamp(SMALLEST, LARGEST);
        let width = measure(&self.text, c.font).width * size / REFERENCE;
        let mut at = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        if let Some(symbol) = self.point {
            // On the symbol when the placement says so; else around it, top right, clear of its edge.
            let on = self.style.point == Some(kentos_commands::PointLabelMode::Center)
                || (self.style.point.is_none()
                    && self.style.placement == kentos_commands::LabelPlacement::Center);
            let centre = if on {
                at
            } else {
                Point::new(
                    bounds.width / 2.0 - width / 2.0 - SYMBOL,
                    bounds.height / 2.0 + size * 0.35,
                )
            };
            frame.fill(&canvas::Path::circle(centre, SYMBOL), symbol);
            if !on {
                at = Point::new(
                    centre.x + SYMBOL + 3.0 + width / 2.0,
                    centre.y - SYMBOL - size * 0.35,
                );
            }
        }
        backdrop_at(
            &mut frame,
            &c,
            at,
            0.0,
            width,
            size * 1.2,
            false,
            self.colors.accent(),
        );
        text_at(&mut frame, &c, &self.text, at, 0.0, size);
        vec![frame.into_geometry()]
    }
}

/// `text` labelled with `style` on a strip of the canvas's colours (`colors`) in the project's typeface; `point`, a
/// point layer's symbol colour: the symbol drawn with it where the label goes.
pub fn example<'a>(
    text: String,
    style: kentos_commands::LabelStyle,
    colors: Colors,
    font: DrawingFont,
    point: Option<iced::Color>,
) -> Element<'a, Message> {
    canvas::Canvas::new(Example {
        text,
        style,
        colors,
        font,
        point,
    })
    .width(Fill)
    .height(kentos_ui::theme::typography::from_default(HEIGHT))
    .into()
}
