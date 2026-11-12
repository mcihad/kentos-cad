//! Konum (docs/adr/0433): a form item showing the record's place: its shape on a small map, north up,
//! its centre's coordinates and its area or length. It writes nothing.

use iced::widget::canvas::{self, Canvas, Frame, Geometry, Path, Stroke};
use iced::widget::{column, container, row};
use iced::{Element, Fill, Point, Rectangle, Renderer, Theme, mouse};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::{entity_area, entity_length, entity_outline, is_closed_outline};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::{label, style};

use crate::app::{App, Message};

/// The record's outline, fitted into the canvas.
struct Outline {
    points: Vec<Vec2>,
    closed: bool,
}

impl<M> canvas::Program<M> for Outline {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let t = Tokens::of(theme);
        let (mut lo, mut hi) = (
            Vec2::new(f64::INFINITY, f64::INFINITY),
            Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        );
        for p in &self.points {
            lo = Vec2::new(lo.x.min(p.x), lo.y.min(p.y));
            hi = Vec2::new(hi.x.max(p.x), hi.y.max(p.y));
        }
        if !lo.x.is_finite() {
            return vec![frame.into_geometry()];
        }
        let (w, h) = (f64::from(bounds.width), f64::from(bounds.height));
        let pad = 14.0;
        let span = (hi.x - lo.x).max(hi.y - lo.y);
        let s = if span > 0.0 {
            ((w - 2.0 * pad) / (hi.x - lo.x).max(1e-9))
                .min((h - 2.0 * pad) / (hi.y - lo.y).max(1e-9))
        } else {
            1.0
        };
        let c = Vec2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0);
        let at = |p: &Vec2| {
            Point::new(
                (w / 2.0 + (p.x - c.x) * s) as f32,
                (h / 2.0 - (p.y - c.y) * s) as f32,
            )
        };
        // A quiet grid under the shape: the map's ground.
        let grid = Stroke::default().with_color(t.border).with_width(1.0);
        for k in 1..4 {
            let x = bounds.width * k as f32 / 4.0;
            let y = bounds.height * k as f32 / 4.0;
            frame.stroke(
                &Path::line(Point::new(x, 0.0), Point::new(x, bounds.height)),
                grid,
            );
            frame.stroke(
                &Path::line(Point::new(0.0, y), Point::new(bounds.width, y)),
                grid,
            );
        }
        let path = Path::new(|b| {
            for (k, p) in self.points.iter().enumerate() {
                if k == 0 {
                    b.move_to(at(p));
                } else {
                    b.line_to(at(p));
                }
            }
            if self.closed {
                b.close();
            }
        });
        if self.closed {
            frame.fill(
                &path,
                iced::Color {
                    a: 0.22,
                    ..t.accent
                },
            );
        }
        let ink = Stroke::default()
            .with_color(t.accent)
            .with_width(2.0)
            .with_line_join(canvas::LineJoin::Round)
            .with_line_cap(canvas::LineCap::Round);
        frame.stroke(&path, ink);
        if self.points.len() == 1 {
            frame.fill(&Path::circle(at(&self.points[0]), 4.0), t.accent);
        }
        // North up: a small K at the top right.
        frame.fill_text(canvas::Text {
            content: "K ↑".into(),
            position: Point::new(bounds.width - 28.0, 6.0),
            color: t.muted,
            size: iced::Pixels(typography::caption()),
            ..canvas::Text::default()
        });
        vec![frame.into_geometry()]
    }
}

/// The centre of `points`: a closed ring's area centroid, else the middle of its box.
fn centre(points: &[Vec2], closed: bool) -> Option<Vec2> {
    if points.is_empty() {
        return None;
    }
    if closed && points.len() >= 3 {
        let (mut a, mut cx, mut cy) = (0.0, 0.0, 0.0);
        let o = points[0];
        for i in 0..points.len() {
            let p = Vec2::new(points[i].x - o.x, points[i].y - o.y);
            let q = points[(i + 1) % points.len()];
            let q = Vec2::new(q.x - o.x, q.y - o.y);
            let cross = p.x * q.y - q.x * p.y;
            a += cross;
            cx += (p.x + q.x) * cross;
            cy += (p.y + q.y) * cross;
        }
        if a.abs() > 1e-12 {
            return Some(Vec2::new(o.x + cx / (3.0 * a), o.y + cy / (3.0 * a)));
        }
    }
    let (lo, hi) = points.iter().fold(
        (
            Vec2::new(f64::INFINITY, f64::INFINITY),
            Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
        ),
        |(lo, hi), p| {
            (
                Vec2::new(lo.x.min(p.x), lo.y.min(p.y)),
                Vec2::new(hi.x.max(p.x), hi.y.max(p.y)),
            )
        },
    );
    Some(Vec2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0))
}

impl App {
    /// The Konum item for the record `uid` (none: a new record or the designer's preview).
    pub(crate) fn location_box<'a>(
        &'a self,
        uid: &str,
        title: Option<&str>,
    ) -> Element<'a, Message> {
        let words = title.unwrap_or("Konum").to_owned();
        let entity = self.document.as_ref().and_then(|d| {
            let slot = d.model.slot_of(kentos_domain::Uuid::parse_str(uid).ok()?)?;
            d.model.get(slot).cloned()
        });
        let Some(entity) = entity else {
            return container(
                column![
                    label::strong(words),
                    label::caption("Kaydın şekli kaydedildikten sonra burada görünür.")
                        .style(muted),
                ]
                .spacing(6),
            )
            .padding(10)
            .width(Fill)
            .style(style::container::field_box)
            .into();
        };
        let shape = kentos_native_application::geometry::shape(&entity);
        let points = entity_outline(&shape, 48.0);
        let closed = is_closed_outline(&shape);
        let format = self.format();
        let mut facts = column![label::strong(words)].spacing(6);
        if let Some(c) = centre(&points, closed) {
            facts = facts.push(fact("Merkez", format.point(c)));
        }
        if closed {
            if let Some(a) = entity_area(&shape) {
                facts = facts.push(fact("Alan", format.area(a)));
            }
            if let Some(l) = entity_length(&shape) {
                facts = facts.push(fact("Çevre", format.length(l)));
            }
        } else if let Some(l) = entity_length(&shape).filter(|l| *l > 0.0) {
            facts = facts.push(fact("Uzunluk", format.length(l)));
        }
        let map = Canvas::new(Outline { points, closed })
            .width(typography::scaled(220.0))
            .height(typography::scaled(140.0));
        container(row![container(map).style(style::container::field_box), facts].spacing(14))
            .padding(10)
            .width(Fill)
            .style(style::container::field_box)
            .into()
    }
}

fn fact<'a>(name: &'static str, value: String) -> Element<'a, Message> {
    row![
        container(label::caption(name).style(muted)).width(typography::scaled(64.0)),
        label::body(value),
    ]
    .spacing(8)
    .into()
}

fn muted(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rings_centre_is_its_area_centroid() {
        let square = [
            Vec2::new(10.0, 0.0),
            Vec2::new(14.0, 0.0),
            Vec2::new(14.0, 4.0),
            Vec2::new(10.0, 4.0),
        ];
        let c = centre(&square, true).unwrap();
        assert!((c.x - 12.0).abs() < 1e-12 && (c.y - 2.0).abs() < 1e-12);
        let line = [Vec2::new(0.0, 0.0), Vec2::new(6.0, 2.0)];
        assert_eq!(centre(&line, false), Some(Vec2::new(3.0, 1.0)));
    }
}
