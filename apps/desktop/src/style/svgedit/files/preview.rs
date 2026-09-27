//! A drawing's picture in a window (the web's `<img>` of the drawing's SVG
//! in the import and export windows): its shapes fitted into the box on
//! the paper (or on a checkerboard for a transparent PNG), painted as the
//! canvas paints them.

use iced::mouse::Cursor;
use iced::widget::canvas::{self, Frame, Geometry, Path};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};
use kentos_svg_core::shape::Obj;
use kentos_ui::theme::Tokens;

use super::super::paint::{View, paint_shape};
use super::super::state::Options;
use crate::app::Message;

pub struct Preview {
    pub shapes: Vec<Obj>,
    /// The canvas's box shown: x, y, width, height in drawing units.
    pub view: [f64; 4],
    pub options: Options,
    /// None: a checkerboard (a transparent picture).
    pub paper: Option<Color>,
}

impl canvas::Program<Message> for Preview {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let pad = 12.0;
        let [x, y, w, h] = self.view;
        let (bw, bh) = (f64::from(bounds.width) - 2.0 * pad, f64::from(bounds.height) - 2.0 * pad);
        let k = if w > 0.0 && h > 0.0 {
            (bw / w).min(bh / h).max(0.0)
        } else {
            1.0
        };
        let ox = pad + (bw - w * k) / 2.0 - x * k;
        let oy = pad + (bh - h * k) / 2.0 - y * k;
        let view = View { zoom: k, ox, oy };
        let a = view.point([x, y]);
        let b = view.point([x + w, y + h]);
        let paper = Path::rectangle(a, Size::new(b.x - a.x, b.y - a.y));
        match self.paper {
            Some(c) => frame.fill(&paper, c),
            None => {
                // A checkerboard of 8 px squares under a transparent picture.
                let light = Color::from_rgb8(0xf2, 0xf2, 0xf2);
                let dark = Color::from_rgb8(0xd9, 0xd9, 0xd9);
                frame.fill(&paper, light);
                let step = 8.0;
                let mut yy = a.y;
                let mut row = 0;
                while yy < b.y {
                    let mut xx = a.x + if row % 2 == 0 { 0.0 } else { step };
                    while xx < b.x {
                        let cw = step.min(b.x - xx);
                        let ch = step.min(b.y - yy);
                        frame.fill_rectangle(Point::new(xx, yy), Size::new(cw, ch), dark);
                        xx += 2.0 * step;
                    }
                    yy += step;
                    row += 1;
                }
            }
        }
        for s in self.shapes.iter().filter(|s| !s.is("hidden")) {
            paint_shape(&mut frame, s, &view, &self.options, 1.0);
        }
        // The picture is the canvas's box: what lies outside it is not in the
        // file's picture, so the margins are laid over it in the box's colour.
        let back = Tokens::of(theme).surface;
        let (fw, fh) = (bounds.width, bounds.height);
        for (p, s) in [
            (Point::ORIGIN, Size::new(fw, a.y)),
            (Point::new(0.0, b.y), Size::new(fw, fh - b.y)),
            (Point::new(0.0, a.y), Size::new(a.x, b.y - a.y)),
            (Point::new(b.x, a.y), Size::new(fw - b.x, b.y - a.y)),
        ] {
            if s.width > 0.0 && s.height > 0.0 {
                frame.fill_rectangle(p, s, back);
            }
        }
        vec![frame.into_geometry()]
    }
}
