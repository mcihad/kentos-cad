//! Kısıtlar, the constraint editor (design §3.2, §11; the web's
//! `ConstraintEditor`): which distances an item keeps when the paper
//! changes, as a square drawing (the box, the item in it, a pin on each of
//! its four sides and its two middle lines) and three lists (Yatay, Düşey,
//! Göre). A click on a pin takes that side; Shift+click keeps both sides
//! (the item stretches). With several items chosen, a pin only some of them
//! have is drawn faint and a list whose values differ says “—”.

use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::widget::{canvas as canvas_widget, column, row};
use iced::{Color, Element, Event, Point, Rectangle, Renderer, Size, Theme, keyboard, mouse};
use kentos_sheet::model::{ConstraintBox, Constraints, HConstraint, VConstraint};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::Tokens;
use kentos_ui::widget::Menu;
use kentos_ui::widget::property_grid::choice;

use crate::message::Message;

/// The drawing's side, pixels.
const SIDE: f32 = 96.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pin {
    Left,
    Right,
    Top,
    Bottom,
    CenterH,
    CenterV,
}

/// Whether every, some or none of the items keep a pin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Held {
    All,
    Some,
    None,
}

fn h_pins(h: HConstraint) -> &'static [Pin] {
    match h {
        HConstraint::Left => &[Pin::Left],
        HConstraint::Right => &[Pin::Right],
        HConstraint::LeftRight => &[Pin::Left, Pin::Right],
        HConstraint::Center => &[Pin::CenterH],
        HConstraint::Scale => &[],
    }
}

fn v_pins(v: VConstraint) -> &'static [Pin] {
    match v {
        VConstraint::Top => &[Pin::Top],
        VConstraint::Bottom => &[Pin::Bottom],
        VConstraint::TopBottom => &[Pin::Top, Pin::Bottom],
        VConstraint::Center => &[Pin::CenterV],
        VConstraint::Scale => &[],
    }
}

/// What a click on a pin makes of the axis (the web's `clickPin`): the pin alone, or with Shift both sides.
fn click(pin: Pin, extend: bool) -> Message {
    match (pin, extend) {
        (Pin::Left | Pin::Right, true) => Message::ConstraintH(HConstraint::LeftRight),
        (Pin::Left, false) => Message::ConstraintH(HConstraint::Left),
        (Pin::Right, false) => Message::ConstraintH(HConstraint::Right),
        (Pin::CenterH, _) => Message::ConstraintH(HConstraint::Center),
        (Pin::Top | Pin::Bottom, true) => Message::ConstraintV(VConstraint::TopBottom),
        (Pin::Top, false) => Message::ConstraintV(VConstraint::Top),
        (Pin::Bottom, false) => Message::ConstraintV(VConstraint::Bottom),
        (Pin::CenterV, _) => Message::ConstraintV(VConstraint::Center),
    }
}

/// The pins' places in the square: each a line to hit and to draw.
fn pins(b: Rectangle) -> [(Pin, Point, Point); 6] {
    let item = item_rect(b);
    let (cx, cy) = (item.center_x(), item.center_y());
    [
        (Pin::Left, Point::new(b.x + 4.0, cy), Point::new(item.x, cy)),
        (
            Pin::Right,
            Point::new(item.x + item.width, cy),
            Point::new(b.x + b.width - 4.0, cy),
        ),
        (Pin::Top, Point::new(cx, b.y + 4.0), Point::new(cx, item.y)),
        (
            Pin::Bottom,
            Point::new(cx, item.y + item.height),
            Point::new(cx, b.y + b.height - 4.0),
        ),
        (
            Pin::CenterH,
            Point::new(cx, item.y + 6.0),
            Point::new(cx, item.y + item.height - 6.0),
        ),
        (
            Pin::CenterV,
            Point::new(item.x + 6.0, cy),
            Point::new(item.x + item.width - 6.0, cy),
        ),
    ]
}

fn item_rect(b: Rectangle) -> Rectangle {
    let m = b.width * 0.3;
    Rectangle::new(
        Point::new(b.x + m, b.y + m),
        Size::new(b.width - 2.0 * m, b.height - 2.0 * m),
    )
}

fn dist(p: Point, a: Point, b: Point) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ((a.x + t * dx - p.x).powi(2) + (a.y + t * dy - p.y).powi(2)).sqrt()
}

struct Drawing<'a> {
    constraints: &'a [Constraints],
    enabled: bool,
}

#[derive(Default)]
struct DrawingState {
    shift: bool,
    hovered: Option<Pin>,
}

impl Drawing<'_> {
    fn held(&self, pin: Pin) -> Held {
        let n = self
            .constraints
            .iter()
            .filter(|c| h_pins(c.h).contains(&pin) || v_pins(c.v).contains(&pin))
            .count();
        match n {
            0 => Held::None,
            n if n == self.constraints.len() => Held::All,
            _ => Held::Some,
        }
    }

    fn pin_at(&self, b: Rectangle, p: Point) -> Option<Pin> {
        pins(b)
            .into_iter()
            .map(|(pin, a, c)| (pin, dist(p, a, c)))
            .filter(|(_, d)| *d < 6.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(pin, _)| pin)
    }
}

impl canvas::Program<Message> for Drawing<'_> {
    type State = DrawingState;

    fn update(
        &self,
        state: &mut DrawingState,
        event: &Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let local = Rectangle::new(Point::ORIGIN, bounds.size());
        match event {
            Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => {
                state.shift = m.shift();
                None
            }
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                let now = cursor
                    .position_in(bounds)
                    .and_then(|p| self.pin_at(local, p));
                (now != state.hovered).then(|| {
                    state.hovered = now;
                    canvas::Action::request_redraw()
                })
            }
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) if self.enabled => {
                let p = cursor.position_in(bounds)?;
                let pin = self.pin_at(local, p)?;
                Some(canvas::Action::publish(click(pin, state.shift)).and_capture())
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &DrawingState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut f = Frame::new(renderer, bounds.size());
        let b = Rectangle::new(Point::ORIGIN, bounds.size());
        // The box: the margins, the page or the group.
        f.stroke(
            &Path::rectangle(
                Point::new(0.5, 0.5),
                Size::new(b.width - 1.0, b.height - 1.0),
            ),
            Stroke {
                line_dash: canvas::LineDash {
                    segments: &[3.0, 2.0],
                    offset: 0,
                },
                ..Stroke::default().with_color(t.border).with_width(1.0)
            },
        );
        let item = item_rect(b);
        f.fill_rectangle(item.position(), item.size(), Color { a: 0.12, ..t.text });
        f.stroke(
            &Path::rectangle(item.position(), item.size()),
            Stroke::default().with_color(t.muted).with_width(1.0),
        );
        for (pin, a, c) in pins(b) {
            let held = self.held(pin);
            let hovered = state.hovered == Some(pin) && self.enabled;
            let color = match held {
                Held::All => t.accent,
                Held::Some => Color { a: 0.5, ..t.accent },
                Held::None if hovered => t.text,
                Held::None => Color { a: 0.35, ..t.muted },
            };
            let width = if held == Held::None { 1.5 } else { 2.5 };
            let dash: &[f32] = if matches!(pin, Pin::CenterH | Pin::CenterV) && held == Held::None {
                &[2.0, 2.0]
            } else {
                &[]
            };
            f.stroke(
                &Path::line(a, c),
                Stroke {
                    line_dash: canvas::LineDash {
                        segments: dash,
                        offset: 0,
                    },
                    ..Stroke::default().with_color(color).with_width(width)
                },
            );
        }
        vec![f.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &DrawingState,
        _: Rectangle,
        _: mouse::Cursor,
    ) -> mouse::Interaction {
        if state.hovered.is_some() && self.enabled {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

fn h_text(h: HConstraint) -> &'static str {
    match h {
        HConstraint::Left => "Sol",
        HConstraint::Right => "Sağ",
        HConstraint::LeftRight => "Sol ve sağ (genişler)",
        HConstraint::Center => "Orta",
        HConstraint::Scale => "Orantılı",
    }
}

fn v_text(v: VConstraint) -> &'static str {
    match v {
        VConstraint::Top => "Üst",
        VConstraint::Bottom => "Alt",
        VConstraint::TopBottom => "Üst ve alt (uzar)",
        VConstraint::Center => "Orta",
        VConstraint::Scale => "Orantılı",
    }
}

fn box_text(b: ConstraintBox) -> &'static str {
    match b {
        ConstraintBox::Margins => "Kenar boşlukları",
        ConstraintBox::Page => "Sayfa",
        ConstraintBox::Group => "Grup",
    }
}

/// The value every item has, or none when they differ.
fn common<T: PartialEq + Copy>(cs: &[Constraints], of: impl Fn(&Constraints) -> T) -> Option<T> {
    let first = of(cs.first()?);
    cs.iter().all(|c| of(c) == first).then_some(first)
}

/// The editor for the chosen items' constraints; `in_group` offers the group as the box.
pub fn editor<'a>(
    constraints: &'a [Constraints],
    in_group: bool,
    enabled: bool,
) -> Element<'a, Message> {
    let hs = [
        HConstraint::Left,
        HConstraint::Right,
        HConstraint::LeftRight,
        HConstraint::Center,
        HConstraint::Scale,
    ];
    let vs = [
        VConstraint::Top,
        VConstraint::Bottom,
        VConstraint::TopBottom,
        VConstraint::Center,
        VConstraint::Scale,
    ];
    let h = common(constraints, |c| c.h);
    let v = common(constraints, |c| c.v);
    let b = common(constraints, |c| c.relative_to);
    let list = move |title: &'static str,
                     value: &'static str,
                     menu: Box<dyn Fn() -> Menu<Message> + 'a>| {
        column![
            label::caption(title).style(style::text::muted),
            choice(value, None, menu)
        ]
        .spacing(2)
    };
    let h_menu = move || {
        hs.iter().fold(Menu::new(), |m, x| {
            m.radio(h_text(*x), Some(*x) == h, Message::ConstraintH(*x))
        })
    };
    let v_menu = move || {
        vs.iter().fold(Menu::new(), |m, x| {
            m.radio(v_text(*x), Some(*x) == v, Message::ConstraintV(*x))
        })
    };
    let boxes: Vec<ConstraintBox> = if in_group {
        vec![
            ConstraintBox::Margins,
            ConstraintBox::Page,
            ConstraintBox::Group,
        ]
    } else {
        vec![ConstraintBox::Margins, ConstraintBox::Page]
    };
    let b_menu = move || {
        boxes.iter().fold(Menu::new(), |m, x| {
            m.radio(box_text(*x), Some(*x) == b, Message::ConstraintBox(*x))
        })
    };
    row![
        canvas_widget(Drawing {
            constraints,
            enabled
        })
        .width(SIDE)
        .height(SIDE),
        column![
            list("Yatay", h.map_or("—", h_text), Box::new(h_menu)),
            list("Düşey", v.map_or("—", v_text), Box::new(v_menu)),
            list("Göre", b.map_or("—", box_text), Box::new(b_menu)),
        ]
        .spacing(6)
        .width(iced::Fill),
    ]
    .spacing(12)
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pin_takes_its_side_and_shift_keeps_both() {
        assert!(matches!(
            click(Pin::Left, false),
            Message::ConstraintH(HConstraint::Left)
        ));
        assert!(matches!(
            click(Pin::Right, true),
            Message::ConstraintH(HConstraint::LeftRight)
        ));
        assert!(matches!(
            click(Pin::Bottom, false),
            Message::ConstraintV(VConstraint::Bottom)
        ));
        assert!(matches!(
            click(Pin::CenterV, true),
            Message::ConstraintV(VConstraint::Center)
        ));
        let d = Drawing {
            constraints: &[
                Constraints {
                    h: HConstraint::LeftRight,
                    ..Constraints::default()
                },
                Constraints {
                    h: HConstraint::Left,
                    ..Constraints::default()
                },
            ],
            enabled: true,
        };
        assert_eq!(d.held(Pin::Left), Held::All);
        assert_eq!(d.held(Pin::Right), Held::Some);
        assert_eq!(d.held(Pin::CenterH), Held::None);
        let b = Rectangle::new(Point::ORIGIN, Size::new(SIDE, SIDE));
        assert_eq!(d.pin_at(b, Point::new(10.0, SIDE / 2.0)), Some(Pin::Left));
        assert_eq!(
            d.pin_at(b, Point::new(SIDE / 2.0, SIDE - 8.0)),
            Some(Pin::Bottom)
        );
    }
}
