//! Yeni pafta's extent as a picture (docs/adr/9187): the page, the main map's frame on it, and the drawing's box with
//! the chosen box (the drawing area's, the layer's) as the map would show them. A box past the frame is the part the
//! map cuts; one inside it shows the room left round it.

use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::{Color, Element, Point, Rectangle, Renderer, Size, Theme, mouse};
use kentos_ui::theme::{Tokens, typography};

use crate::message::Message;
use crate::new_extent::Picture;

/// The picture's size.
const WIDTH: f32 = 180.0;
const HEIGHT: f32 = 128.0;

struct Program(Picture);

impl canvas::Program<Message> for Program {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let p = &self.0;
        let mut frame = Frame::new(renderer, bounds.size());
        // The page fitted with a margin of 6 px, in the middle.
        let k = ((f64::from(bounds.width) - 12.0) / p.page[0])
            .min((f64::from(bounds.height) - 12.0) / p.page[1])
            .max(1e-6);
        let (ox, oy) = (
            (f64::from(bounds.width) - p.page[0] * k) / 2.0,
            (f64::from(bounds.height) - p.page[1] * k) / 2.0,
        );
        let rect = |b: [f64; 4]| {
            (
                Point::new((ox + b[0] * k) as f32, (oy + b[1] * k) as f32),
                Size::new((b[2] * k) as f32, (b[3] * k) as f32),
            )
        };
        // A box cut to the page: what falls past it is off the paper (none: wholly off it).
        let on_page = |b: [f64; 4]| -> Option<[f64; 4]> {
            let (l, t) = (b[0].max(0.0), b[1].max(0.0));
            let (r, d) = ((b[0] + b[2]).min(p.page[0]), (b[1] + b[3]).min(p.page[1]));
            (r > l && d > t).then_some([l, t, r - l, d - t])
        };
        let (page_at, page_size) = rect([0.0, 0.0, p.page[0], p.page[1]]);
        frame.fill_rectangle(page_at, page_size, Color::WHITE);
        frame.stroke(
            &Path::rectangle(page_at, page_size),
            Stroke::default()
                .with_color(t.border_strong())
                .with_width(1.0),
        );
        // The boxes on the page alone.
        if let Some(d) = p.drawing.and_then(on_page) {
            let (at, size) = rect(d);
            frame.fill_rectangle(
                at,
                size,
                Color {
                    a: 0.22,
                    ..t.accent
                },
            );
            frame.stroke(
                &Path::rectangle(at, size),
                Stroke::default().with_color(t.accent).with_width(1.0),
            );
        }
        if let Some(c) = p.chosen.and_then(on_page) {
            let (at, size) = rect(c);
            frame.stroke(
                &Path::rectangle(at, size),
                Stroke {
                    line_dash: canvas::LineDash {
                        segments: &[3.0, 2.0],
                        offset: 0,
                    },
                    ..Stroke::default().with_color(t.accent_hover).with_width(1.0)
                },
            );
        }
        // The map's frame over them.
        let (at, size) = rect(p.frame);
        frame.stroke(
            &Path::rectangle(at, size),
            Stroke::default().with_color(Color::BLACK).with_width(1.5),
        );
        vec![frame.into_geometry()]
    }
}

/// The picture of `p`.
pub(crate) fn picture<'a>(p: &Picture) -> Element<'a, Message> {
    canvas::Canvas::new(Program(p.clone()))
        .width(typography::from_default(WIDTH))
        .height(typography::from_default(HEIGHT))
        .into()
}
