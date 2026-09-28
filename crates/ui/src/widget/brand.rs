//! KentOS logosu: lacivert karo üstünde K (DESIGN.md §2; web'in
//! `brandButton.ts` `brandMark`'ı, aynı 24 birimlik kutuda). K'nin gövdesi ve
//! kolları bir ölçme noktasında birleşir. Markanın kendi rengindedir, seçilen
//! vurguyu izlemez; altında hafif bir gölge vardır.
//!
//! ```ignore
//! row![brand_mark(20), label::heading("KentOS")]
//! ```

use iced::widget::canvas::{self, Frame, Geometry, LineCap, LineJoin, Path, Stroke, gradient};
use iced::{Color, Element, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::theme::{Mode, brand};

/// Verilen boyda (piksel) logo.
pub fn brand_mark<'a, Message: 'a>(size: f32) -> Element<'a, Message> {
    canvas::Canvas::new(Mark).width(size).height(size).into()
}

struct Mark;

impl<Message> canvas::Program<Message> for Mark {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let b = brand(Mode::of(theme));
        let mut frame = Frame::new(renderer, bounds.size());
        // The web's 24-unit box, scaled to the size asked for.
        let k = bounds.width.min(bounds.height) / 24.0;
        let at = |x: f32, y: f32| Point::new(x * k, y * k);
        let square = |from: f32, side: f32, radius: f32| {
            Path::rounded_rectangle(
                at(from, from),
                Size::new(side * k, side * k),
                (radius * k).into(),
            )
        };

        // The shadow under the tile (the web's drop-shadow 0 1px 1.5px).
        frame.with_save(|frame| {
            frame.translate(Vector::new(0.0, k.max(0.5)));
            frame.fill(&square(1.0, 22.0, 6.0), Color::BLACK.scale_alpha(0.22));
        });
        // The tile, light navy to navy across, and its thin light edge.
        frame.fill(
            &square(1.0, 22.0, 6.0),
            gradient::Linear::new(at(1.0, 1.0), at(23.0, 23.0))
                .add_stop(0.0, b.hi)
                .add_stop(1.0, b.tile),
        );
        frame.stroke(
            &square(1.5, 21.0, 5.5),
            Stroke::default()
                .with_color(Color::WHITE.scale_alpha(0.28))
                .with_width(k.max(0.5)),
        );
        // The K: its stem, its arms, and the survey point where they meet.
        let ink = Stroke {
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default().with_color(b.ink).with_width(2.3 * k)
        };
        frame.stroke(&Path::line(at(8.0, 6.2), at(8.0, 17.8)), ink);
        frame.stroke(
            &Path::new(|p| {
                p.move_to(at(16.6, 6.4));
                p.line_to(at(10.3, 12.0));
                p.line_to(at(16.6, 17.6));
            }),
            ink,
        );
        frame.fill(&Path::circle(at(10.3, 12.0), 2.1 * k), b.ink);
        frame.fill(&Path::circle(at(10.3, 12.0), 0.8 * k), b.hi);

        vec![frame.into_geometry()]
    }
}
