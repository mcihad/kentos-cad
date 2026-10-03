//! Türkiye's longitudes 25.5°–46.5° on a strip with the seven TUREF TM3
//! zones (the web's `ui/settings/zoneStrip.ts`, docs/adr/0165 §3): the
//! zone of the chosen system lit, the province's place marked with its
//! name. A click on a zone chooses it. Drawn in the web's units (its SVG's
//! view box), scaled to the strip's box.

use iced::alignment::Vertical;
use iced::widget::canvas::{self, Frame, Path, Stroke, Text};
use iced::widget::text::Alignment;
use iced::{Element, Fill, Pixels, Point, Rectangle, Renderer, Size, Theme, mouse};
use kentos_project::provinces::Province;
use kentos_ui::theme::{Tokens, typography};

use super::Event;
use crate::app::Message;

/// The zones' central meridians, west to east; their TUREF systems are 5253 onwards.
const MERIDIANS: [f32; 7] = [27.0, 30.0, 33.0, 36.0, 39.0, 42.0, 45.0];
const WEST: f32 = 25.5;
/// Units of the strip per degree, and its size.
const PER_DEG: f32 = 30.0;
const W: f32 = 21.0 * PER_DEG;
const H: f32 = 64.0;
/// The view box's left edge and width: room at both ends for the outer longitudes' labels.
const LEFT: f32 = -18.0;
const BOX_W: f32 = W + 36.0;

fn x(lon: f32) -> f32 {
    (lon - WEST) * PER_DEG
}

/// The TUREF zone of an SRID (5253–5259) or the ED50 one (2319–2325), as an index west to east.
pub(super) fn zone_index(srid: u32) -> Option<usize> {
    match srid {
        5253..=5259 => Some((srid - 5253) as usize),
        2319..=2325 => Some((srid - 2319) as usize),
        _ => None,
    }
}

pub(super) fn view<'a>(srid: u32, province: Option<&'static Province>) -> Element<'a, Message> {
    canvas::Canvas::new(Strip {
        chosen: zone_index(srid),
        province,
    })
    .width(Fill)
    .height(typography::from_default(70.0))
    .into()
}

struct Strip {
    chosen: Option<usize>,
    province: Option<&'static Province>,
}

/// The view box in the strip's box: its scale and where its origin lands.
struct Fit {
    scale: f32,
    left: f32,
    top: f32,
}

impl Fit {
    fn of(size: Size) -> Self {
        let scale = (size.width / BOX_W).min(size.height / H).max(0.01);
        Self {
            scale,
            left: (size.width - BOX_W * scale) / 2.0 - LEFT * scale,
            top: (size.height - H * scale) / 2.0,
        }
    }

    fn at(&self, u: f32, v: f32) -> Point {
        Point::new(self.left + u * self.scale, self.top + v * self.scale)
    }

    /// The zone under a point of the box.
    fn zone(&self, p: Point) -> Option<usize> {
        let (u, v) = (
            (p.x - self.left) / self.scale,
            (p.y - self.top) / self.scale,
        );
        if !(22.0..=46.0).contains(&v) {
            return None;
        }
        MERIDIANS
            .iter()
            .position(|cm| (x(cm - 1.5)..x(cm + 1.5)).contains(&u))
    }
}

impl canvas::Program<Message> for Strip {
    /// The zone under the pointer.
    type State = Option<usize>;

    fn update(
        &self,
        hovered: &mut Option<usize>,
        event: &iced::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let fit = Fit::of(bounds.size());
        let under = cursor.position_in(bounds).and_then(|p| fit.zone(p));
        match event {
            iced::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let i = under?;
                Some(
                    canvas::Action::publish(super::event(Event::System(5253 + i as u32)))
                        .and_capture(),
                )
            }
            iced::Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft)
                if under != *hovered =>
            {
                *hovered = under;
                Some(canvas::Action::request_redraw())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        hovered: &Option<usize>,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let fit = Fit::of(bounds.size());
        let s = fit.scale;
        let mut frame = Frame::new(renderer, bounds.size());
        let words = |frame: &mut Frame,
                     content: String,
                     at: Point,
                     size: f32,
                     strong: bool,
                     color,
                     align: Alignment| {
            frame.fill_text(Text {
                content,
                position: at,
                color,
                size: Pixels(size * s),
                font: if strong {
                    typography::ui_strong()
                } else {
                    typography::ui()
                },
                align_x: align,
                align_y: Vertical::Center,
                ..Text::default()
            });
        };
        for (i, cm) in MERIDIANS.iter().enumerate() {
            let chosen = self.chosen == Some(i);
            let zone = Path::rounded_rectangle(
                fit.at(x(cm - 1.5) + 1.0, 22.0),
                Size::new((3.0 * PER_DEG - 2.0) * s, 24.0 * s),
                (3.0 * s).into(),
            );
            frame.fill(&zone, if chosen { t.selection() } else { t.field });
            let edge = if chosen {
                t.accent
            } else if *hovered == Some(i) {
                t.faint
            } else {
                t.border_strong()
            };
            frame.stroke(
                &zone,
                Stroke::default()
                    .with_color(edge)
                    .with_width(if chosen { 1.5 } else { 1.0 }),
            );
            words(
                &mut frame,
                format!("TM{cm}"),
                fit.at(x(*cm), 34.0),
                11.0,
                chosen,
                if chosen { t.accent_hover } else { t.muted },
                Alignment::Center,
            );
        }
        // The zone boundaries' longitudes under the strip.
        for k in 0..=7 {
            let lon = WEST + 3.0 * k as f32;
            words(
                &mut frame,
                format!("{lon}°"),
                fit.at(x(lon), 56.5),
                9.5,
                false,
                t.faint,
                Alignment::Center,
            );
        }
        if let Some(p) = self.province {
            let px = x((p.lon as f32).clamp(WEST, WEST + 21.0));
            let align = if px < 60.0 {
                Alignment::Left
            } else if px > W - 60.0 {
                Alignment::Right
            } else {
                Alignment::Center
            };
            // The name and a pin above the strip, its point on the zone's edge: the zone's label stays clear.
            words(
                &mut frame,
                p.name.clone(),
                fit.at(px, 5.5),
                10.5,
                true,
                t.text,
                align,
            );
            frame.stroke(
                &Path::line(fit.at(px, 13.0), fit.at(px, 20.0)),
                Stroke::default().with_color(t.text).with_width(1.5),
            );
            let pin = Path::new(|b| {
                b.move_to(fit.at(px - 4.0, 17.0));
                b.line_to(fit.at(px + 4.0, 17.0));
                b.line_to(fit.at(px, 22.0));
                b.close();
            });
            frame.fill(&pin, t.text);
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _hovered: &Option<usize>,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        let fit = Fit::of(bounds.size());
        if cursor
            .position_in(bounds)
            .and_then(|p| fit.zone(p))
            .is_some()
        {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zone_is_found_under_the_pointer_and_from_its_system() {
        assert_eq!(
            [5253, 5256, 5259, 2319, 2325, 32636, 0].map(zone_index),
            [Some(0), Some(3), Some(6), Some(0), Some(6), None, None]
        );
        // The view box exactly: one unit a pixel, from -18.
        let fit = Fit::of(Size::new(BOX_W, H));
        assert_eq!((fit.scale, fit.left, fit.top), (1.0, 18.0, 0.0));
        // TM36's zone (34.5°–37.5°), in the middle of the band; above and below it, none.
        let tm36 = fit.at(x(36.0), 34.0);
        assert_eq!(fit.zone(tm36), Some(3));
        assert_eq!(fit.zone(fit.at(x(36.0), 10.0)), None);
        assert_eq!(fit.zone(fit.at(x(36.0), 56.0)), None);
        assert_eq!(fit.zone(fit.at(x(25.0), 34.0)), None);
    }
}
