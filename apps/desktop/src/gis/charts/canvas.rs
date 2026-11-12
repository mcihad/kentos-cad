//! The Grafik tab's chart on screen (docs/adr/0227 §5, §6; the web's
//! `ChartPanel` canvas): two canvases, one over the other.
//!
//! - [`Chart`] lays the chart out in its room and paints it: the shapes as
//!   one picture in the screen's pixels (paint.rs), the texts over it in the
//!   interface's typeface; kept until the chart, its marks, the room or the
//!   theme change.
//! - [`Overlay`] follows the pointer: the hovered element outlined in the
//!   first ink and its tooltip (values first, names after), Saçılım's
//!   dragged box; a press publishes the element under it, a dragged box its
//!   corners.

use std::cell::Cell;

use iced::advanced::image::{FilterMethod, Handle};
use iced::alignment::Vertical;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::widget::text::{Alignment as Align, LineHeight, Shaping};
use iced::{Color, Pixels, Point, Rectangle, Renderer, Size, Theme, mouse};
use kentos_geometry_core::ops::charts::Work;
use kentos_geometry_core::ops::charts::layout::{
    Align as TextAlign, Baseline, Frame as Room, Layout, Prim, Role,
};
use kentos_ui::theme::{Tokens, typography};

use super::paint::{Step, circle, picture, round_rect, wedge};
use super::palette::color;
use super::{ChartsPanel, Event, Source};
use crate::app::Message;

/// A drag shorter than this (px) is a press.
const DRAG: f32 = 3.0;

fn publish(e: Event) -> canvas::Action<Message> {
    canvas::Action::publish(Message::Charts(e)).and_capture()
}

/// The chart's picture as last painted, and what it was painted for.
#[derive(Default)]
pub(crate) struct Painted {
    cache: canvas::Cache,
    /// The chart's stamp, the theme's colours and the pixel ratio.
    key: Cell<Option<(u64, u64, u32)>>,
}

/// The room's text: the interface's caption size and typeface (the web's `--fs-xs`, `--font-ui`).
pub(crate) fn room(size: Size) -> Room {
    Room {
        width: size.width,
        height: size.height,
        font: typography::caption(),
        family: typography::current().family.name().to_owned(),
    }
}

/// What the theme's colours of the chart come to, to know when to paint again.
fn theme_key(t: &Tokens) -> u64 {
    let c = |c: Color| {
        u64::from(
            c.into_rgba8()
                .iter()
                .fold(0u32, |a, b| a << 8 | u32::from(*b)),
        )
    };
    c(t.surface) ^ c(t.accent).rotate_left(17) ^ c(t.text).rotate_left(34) ^ u64::from(t.is_dark)
}

/// The work of the source shown, borrowed for `f`.
fn with_work<R>(panel: &ChartsPanel, f: impl FnOnce(&mut Work) -> R) -> Option<R> {
    match panel.source {
        Source::Table if panel.table.is_some() => {
            panel.table_data.borrow_mut().as_mut().map(|(_, w)| f(w))
        }
        _ => panel
            .layer_data
            .borrow_mut()
            .as_mut()
            .map(|(_, d)| f(&mut d.work)),
    }
}

/// The chart, laid out in its room and painted.
pub(super) struct Chart<'a> {
    pub panel: &'a ChartsPanel,
    pub stamp: u64,
    /// The screen's device pixels per chart pixel.
    pub scale: f32,
}

impl canvas::Program<Message> for Chart<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let t = Tokens::of(theme);
        let painted = &self.panel.painted;
        self.panel.room.set(Some(bounds));
        let key = (self.stamp, theme_key(&t), self.scale.to_bits());
        if painted.key.get() != Some(key) {
            painted.cache.clear();
            painted.key.set(Some(key));
        }
        let geometry = painted.cache.draw(renderer, bounds.size(), |frame| {
            let size = bounds.size();
            if size.width < 10.0 || size.height < 10.0 {
                return;
            }
            let laid = with_work(self.panel, |w| w.lay_out(&room(size)).cloned()).flatten();
            let Some(layout) = laid else {
                return;
            };
            let k = self.scale.max(0.5);
            let (pw, ph) = (
                (size.width * k).ceil() as u32,
                (size.height * k).ceil() as u32,
            );
            if let Some(pixels) = picture(&layout, &t, pw, ph, k) {
                frame.draw_image(
                    Rectangle::new(Point::ORIGIN, Size::new(pw as f32 / k, ph as f32 / k)),
                    canvas::Image::new(Handle::from_rgba(pw, ph, pixels))
                        .filter_method(FilterMethod::Linear)
                        .snap(true),
                );
            }
            texts(frame, &layout, &t);
        });
        vec![geometry]
    }
}

/// The layout's texts in the interface's typeface.
fn texts(frame: &mut Frame, layout: &Layout, t: &Tokens) {
    for p in &layout.prims {
        let Prim::Text {
            x,
            y,
            text,
            size,
            weight,
            align,
            baseline,
            color: role,
        } = p
        else {
            continue;
        };
        frame.fill_text(Text {
            content: text.clone(),
            position: Point::new(*x, *y),
            color: color(t, *role),
            size: Pixels(*size),
            font: if *weight >= 600 {
                typography::ui_strong()
            } else {
                typography::ui()
            },
            align_x: match align {
                TextAlign::Start => Align::Left,
                TextAlign::Middle => Align::Center,
                TextAlign::End => Align::Right,
            },
            align_y: match baseline {
                Baseline::Top => Vertical::Top,
                Baseline::Middle => Vertical::Center,
                Baseline::Bottom => Vertical::Bottom,
            },
            line_height: LineHeight::Relative(1.0),
            shaping: Shaping::Advanced,
            ..Text::default()
        });
    }
}

/// The steps as a canvas path.
fn path_of(steps: &[Step]) -> Path {
    Path::new(|b| {
        for s in steps {
            match *s {
                Step::Move(x, y) => b.move_to(Point::new(x, y)),
                Step::Line(x, y) => b.line_to(Point::new(x, y)),
                Step::Cubic(a, c, d, e, f, g) => {
                    b.bezier_curve_to(Point::new(a, c), Point::new(d, e), Point::new(f, g));
                }
                Step::Close => b.close(),
            }
        }
    })
}

/// The pointer over the chart.
pub(super) struct Overlay<'a> {
    pub panel: &'a ChartsPanel,
    /// Saçılım: a drag draws a box.
    pub scatter: bool,
    /// A layer's chart: a press selects (a table's rows are no objects).
    pub selects: bool,
}

#[derive(Default)]
pub(super) struct OverlayState {
    hovered: Option<u32>,
    at: Option<Point>,
    press: Option<Point>,
    dragged: bool,
}

impl Overlay<'_> {
    fn hit(&self, p: Point) -> Option<u32> {
        with_work(self.panel, |w| w.hit(p.x, p.y)).flatten()
    }
}

impl canvas::Program<Message> for Overlay<'_> {
    type State = OverlayState;

    fn update(
        &self,
        state: &mut OverlayState,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let canvas::Event::Mouse(event) = event else {
            return None;
        };
        match event {
            mouse::Event::CursorMoved { .. } => {
                let Some(p) = cursor.position_in(bounds) else {
                    if state.hovered.take().is_some() || state.at.take().is_some() {
                        return Some(canvas::Action::request_redraw());
                    }
                    return None;
                };
                if let Some(press) = state.press {
                    if self.scatter && p.distance(press) >= DRAG {
                        state.dragged = true;
                    }
                    state.at = Some(p);
                    return Some(canvas::Action::request_redraw());
                }
                let hit = self.hit(p);
                let moved = state.at != Some(p);
                state.at = Some(p);
                if hit != state.hovered || (hit.is_some() && moved) {
                    state.hovered = hit;
                    return Some(canvas::Action::request_redraw());
                }
                None
            }
            mouse::Event::CursorLeft => {
                state.hovered = None;
                state.at = None;
                Some(canvas::Action::request_redraw())
            }
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let p = cursor.position_in(bounds)?;
                state.press = Some(p);
                state.dragged = false;
                Some(canvas::Action::capture())
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                let press = state.press.take()?;
                let p = cursor.position_in(bounds).unwrap_or(press);
                let dragged = std::mem::take(&mut state.dragged);
                if !self.selects {
                    return Some(canvas::Action::request_redraw());
                }
                if dragged {
                    return Some(publish(Event::Within([press.x, press.y, p.x, p.y])));
                }
                Some(publish(Event::Press(self.hit(p))))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &OverlayState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let t = Tokens::of(theme);
        let mut frame = Frame::new(renderer, bounds.size());
        // Saçılım's box.
        if let (Some(press), Some(p), true) = (state.press, state.at, state.dragged) {
            let top_left = Point::new(press.x.min(p.x), press.y.min(p.y));
            let size = Size::new((p.x - press.x).abs(), (p.y - press.y).abs());
            frame.fill_rectangle(top_left, size, t.selection());
            frame.stroke(
                &Path::rectangle(top_left, size),
                Stroke::default()
                    .with_color(t.accent_line())
                    .with_width(1.0),
            );
            return vec![frame.into_geometry()];
        }
        let Some(e) = state.hovered else {
            return vec![frame.into_geometry()];
        };
        let tip = with_work(self.panel, |w| {
            if let Some(layout) = w.layout() {
                outline(&mut frame, layout, e, &t);
            }
            w.tip(e as usize)
        })
        .unwrap_or_default();
        if let Some(at) = state.at {
            tooltip(&mut frame, &tip, at, bounds.size(), &t);
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &OverlayState,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if cursor.position_in(bounds).is_none() {
            return mouse::Interaction::default();
        }
        if state.dragged {
            mouse::Interaction::Crosshair
        } else if state.hovered.is_some() && self.selects {
            mouse::Interaction::Pointer
        } else {
            mouse::Interaction::default()
        }
    }
}

/// The hovered element outlined in the first ink: its bars, slices and
/// markers, or its point (the web's `outline`).
fn outline(frame: &mut Frame, layout: &Layout, e: u32, t: &Tokens) {
    let ink = Stroke::default()
        .with_color(color(t, Role::Ink1))
        .with_width(1.5);
    let e = e as i32;
    let mut found = false;
    for p in &layout.prims {
        let steps = match *p {
            Prim::Rect {
                x,
                y,
                w,
                h,
                r_top,
                r_bottom,
                elem,
                ..
            } if elem == e => round_rect(x, y, w, h, r_top, r_bottom),
            Prim::Wedge {
                cx,
                cy,
                r0,
                r1,
                a0,
                a1,
                elem,
                ..
            } if elem == e => wedge(cx, cy, r0, r1, a0, a1),
            Prim::Circle { x, y, r, elem, .. } if elem == e => circle(x, y, r + 2.0),
            _ => continue,
        };
        frame.stroke(&path_of(&steps), ink);
        found = true;
    }
    if found {
        return;
    }
    // A scatter's point: the bulk points carry their elements in order.
    let Some(k) = layout.point_elems.iter().position(|&p| p as i32 == e) else {
        return;
    };
    let r = layout
        .prims
        .iter()
        .find_map(|p| match *p {
            Prim::Points {
                start, count, r, ..
            } if (start..start + count).contains(&(k as u32)) => Some(r),
            _ => None,
        })
        .unwrap_or(4.0);
    if let (Some(&x), Some(&y)) = (layout.points.get(k * 2), layout.points.get(k * 2 + 1)) {
        frame.stroke(&Path::circle(Point::new(x, y), r + 3.0), ink);
    }
}

/// The tooltip beside the pointer, inside the chart: each line its value
/// first in the first ink (the first line's semibold), its name after in the second.
fn tooltip(frame: &mut Frame, lines: &[(String, String)], at: Point, room: Size, t: &Tokens) {
    if lines.is_empty() {
        return;
    }
    let size = typography::caption();
    let line = (size * 1.45).round();
    let (pad_x, pad_y, gap) = (8.0, 6.0, 8.0);
    let widths: Vec<(f32, f32)> = lines
        .iter()
        .enumerate()
        .map(|(i, (label, value))| {
            (
                typography::measured_width(value, size, i == 0),
                typography::measured_width(label, size, false),
            )
        })
        .collect();
    let inner = widths.iter().map(|(v, l)| v + gap + l).fold(0.0, f32::max);
    let w = (inner + 2.0 * pad_x).min(280.0_f32.max(room.width * 0.6));
    let h = line * lines.len() as f32 + 2.0 * pad_y;
    let left = if at.x + 14.0 + w > room.width {
        at.x - 14.0 - w
    } else {
        at.x + 14.0
    };
    let top = (at.y - h / 2.0).min(room.height - h - 4.0).max(4.0);
    let origin = Point::new(left.max(4.0), top);
    frame.fill(
        &Path::rounded_rectangle(origin, Size::new(w, h), 6.0.into()),
        t.popover,
    );
    frame.stroke(
        &Path::rounded_rectangle(origin, Size::new(w, h), 6.0.into()),
        Stroke::default()
            .with_color(t.border_strong())
            .with_width(1.0),
    );
    for (i, ((label, value), (vw, _))) in lines.iter().zip(&widths).enumerate() {
        let y = origin.y + pad_y + line * i as f32 + line / 2.0;
        let x = origin.x + pad_x;
        let text = |content: &str, x: f32, color: Color, strong: bool| Text {
            content: content.to_owned(),
            position: Point::new(x, y),
            color,
            size: Pixels(size),
            font: if strong {
                typography::ui_strong()
            } else {
                typography::ui()
            },
            align_y: Vertical::Center,
            shaping: Shaping::Advanced,
            ..Text::default()
        };
        frame.fill_text(text(value, x, t.text, i == 0));
        frame.fill_text(text(label, x + vw + gap, t.muted, false));
    }
}

/// The chart's last room in the window (a saved picture takes its size).
pub(crate) fn last_room(panel: &ChartsPanel) -> Option<Rectangle> {
    panel.room.get()
}
