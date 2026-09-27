//! The flow's canvas on the desktop (DESIGN.md §7.16, docs/adr/0101): the
//! web's FlowView. The nodes stand where the core laid them out; a value
//! goes from a node's output (right) into another node's input (left), the
//! result on the right. The canvas draws and reports gestures; the builder
//! changes the text through the core.
//!
//! - drag from an output to an input: connect (what the input held goes apart);
//! - drag a connected input's pin away: take the connection off (onto
//!   another input: move it there); from an empty input to a node: connect it;
//! - a tree not connected to the result moves by its root's title;
//! - the wheel zooms, dragging the background pans; a palette entry carried
//!   out of the tree is let go here.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use iced::alignment::Vertical;
use iced::widget::canvas::{self, Frame, Geometry, LineDash, Path, Stroke, Text};
use iced::widget::text::{Alignment as Align, LineHeight, Shaping};
use iced::{Color, Font, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector, border, mouse};
use kentos_expression::editor::flow::{self as core, Flow, FlowNode, NodeKind, Type};
use kentos_ui::theme::{Mode, Tokens, typography};

use super::flow::{FlowEvent, View};
use super::highlight::Syntax;
use crate::app::Message;

/// A second press on the same node within this is a double click.
const DOUBLE: Duration = Duration::from_millis(450);
/// How far the pointer goes before a press becomes a drag (px).
const DRAG: f32 = 4.0;
/// How near a pin a press takes it (world px).
const PIN: f32 = 9.0;

fn flow_event(e: FlowEvent) -> Message {
    Message::Builder(super::Event::Flow(e))
}

fn publish(e: FlowEvent) -> canvas::Action<Message> {
    canvas::Action::publish(flow_event(e)).and_capture()
}

/// The canvas' view of the builder's flow.
pub(crate) struct FlowCanvas<'a> {
    pub flow: &'a Flow,
    pub view: View,
    pub selected: Option<&'a str>,
    pub values: &'a HashMap<String, String>,
    pub mode: Mode,
    /// A palette entry carried out of the tree.
    pub carrying: Option<&'a str>,
}

#[derive(Default)]
pub(crate) struct State {
    size: Option<Size>,
    gesture: Gesture,
    /// Where the button went down, and whether it has gone far since.
    press: Option<Point>,
    moved: bool,
    /// The pointer, relative to the canvas.
    cursor: Option<Point>,
    last_click: Option<(String, Instant)>,
}

#[derive(Default)]
enum Gesture {
    #[default]
    None,
    Pan {
        view: View,
    },
    /// From an output.
    Wire {
        from: String,
    },
    /// A connected input's edge taken.
    Pull {
        child: String,
        to: String,
        port: usize,
    },
    /// From an empty input, towards a node.
    Back {
        to: String,
        port: usize,
    },
    Move {
        tree: usize,
        origin: (f64, f64),
    },
}

/// What is under a point.
enum Hit {
    Out(String),
    Pin(String, usize),
    Port(String, usize),
    Plus(String),
    RemovePort(String, usize),
    Head(String),
    Node(String),
}

/// The tree a node is in (none for the result).
fn tree_of(id: &str) -> Option<usize> {
    (id != "r").then(|| id.split('.').next().and_then(|t| t.parse().ok()))?
}

fn is_root(id: &str) -> bool {
    id != "r" && !id.contains('.')
}

impl FlowCanvas<'_> {
    /// The view a gesture has moved to so far.
    fn current(&self, state: &State) -> View {
        match (&state.gesture, state.press, state.cursor) {
            (Gesture::Pan { view }, Some(press), Some(at)) if state.moved => View {
                x: view.x + at.x - press.x,
                y: view.y + at.y - press.y,
                k: view.k,
            },
            _ => self.view,
        }
    }

    fn world(v: View, p: Point) -> Point {
        Point::new((p.x - v.x) / v.k, (p.y - v.y) / v.k)
    }

    fn node(&self, id: &str) -> Option<&FlowNode> {
        self.flow.nodes.iter().find(|n| n.id == id)
    }

    /// Where a node stands now: its place, or its tree's being moved.
    fn at(&self, state: &State, n: &FlowNode) -> (f32, f32) {
        let (x, y) = (n.x as f32, n.y as f32);
        if let (Gesture::Move { tree, .. }, Some(press), Some(cursor)) =
            (&state.gesture, state.press, state.cursor)
            && state.moved
            && tree_of(&n.id) == Some(*tree)
        {
            let k = self.view.k;
            return (x + (cursor.x - press.x) / k, y + (cursor.y - press.y) / k);
        }
        (x, y)
    }

    fn hit(&self, p: Point) -> Option<Hit> {
        let f = self.flow;
        let (head, row) = (core::HEAD as f32, core::ROW as f32);
        for n in f.nodes.iter().rev() {
            let (x, y, w, h) = (n.x as f32, n.y as f32, n.w as f32, n.h as f32);
            if n.kind != NodeKind::Result && p.distance(Point::new(x + w, y + head / 2.0)) <= PIN {
                return Some(Hit::Out(n.id.clone()));
            }
            for (k, port) in n.ports.iter().enumerate() {
                if p.distance(Point::new(x, y + port.y as f32)) <= PIN {
                    return Some(Hit::Pin(n.id.clone(), k));
                }
            }
            if p.x < x || p.x > x + w || p.y < y || p.y > y + h {
                continue;
            }
            if n.grows && p.y <= y + head && p.x >= x + w - 24.0 {
                return Some(Hit::Plus(n.id.clone()));
            }
            for (k, port) in n.ports.iter().enumerate() {
                if (p.y - (y + port.y as f32)).abs() <= row / 2.0 {
                    if port.removable && p.x >= x + w - 22.0 {
                        return Some(Hit::RemovePort(n.id.clone(), k));
                    }
                    return Some(Hit::Port(n.id.clone(), k));
                }
            }
            return Some(if p.y <= y + head {
                Hit::Head(n.id.clone())
            } else {
                Hit::Node(n.id.clone())
            });
        }
        None
    }

    /// The input under a point: a port, or a node's first empty input.
    fn input_at(&self, p: Point) -> Option<(String, usize)> {
        match self.hit(p)? {
            Hit::Port(id, k) | Hit::Pin(id, k) | Hit::RemovePort(id, k) => Some((id, k)),
            Hit::Head(id) | Hit::Node(id) | Hit::Plus(id) | Hit::Out(id) => {
                let n = self.node(&id)?;
                let k = n.ports.iter().position(|port| port.from.is_none())?;
                Some((id, k))
            }
        }
    }
}

impl canvas::Program<Message> for FlowCanvas<'_> {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let size = bounds.size();
        if state.size != Some(size) {
            state.size = Some(size);
            return Some(canvas::Action::publish(flow_event(FlowEvent::Room(
                size.width,
                size.height,
            ))));
        }
        let canvas::Event::Mouse(event) = event else {
            return None;
        };
        match event {
            mouse::Event::CursorMoved { position } => {
                let p = *position - Vector::new(bounds.x, bounds.y);
                state.cursor = Some(p);
                if let Some(press) = state.press
                    && !state.moved
                    && p.distance(press) > DRAG
                {
                    state.moved = true;
                }
                let busy = !matches!(state.gesture, Gesture::None) || self.carrying.is_some();
                busy.then(canvas::Action::request_redraw)
            }
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let p = cursor.position_in(bounds)?;
                state.press = Some(p);
                state.moved = false;
                state.cursor = Some(p);
                let w = Self::world(self.view, p);
                match self.hit(w) {
                    Some(Hit::Out(id)) => state.gesture = Gesture::Wire { from: id },
                    Some(Hit::Pin(id, k)) => {
                        let from = self.node(&id).and_then(|n| n.ports[k].from.clone());
                        state.gesture = match from {
                            Some(child) => Gesture::Pull {
                                child,
                                to: id,
                                port: k,
                            },
                            None => Gesture::Back { to: id, port: k },
                        };
                    }
                    Some(Hit::Plus(id)) => {
                        state.press = None;
                        return Some(publish(FlowEvent::AddPort(id)));
                    }
                    Some(Hit::RemovePort(id, k)) => {
                        state.press = None;
                        return Some(publish(FlowEvent::RemovePort(id, k)));
                    }
                    Some(Hit::Head(id) | Hit::Node(id) | Hit::Port(id, _)) => {
                        let movable = is_root(&id) && tree_of(&id).is_some_and(|t| t > 0);
                        if movable
                            && let Some(n) = self.node(&id)
                            && w.y <= n.y as f32 + core::HEAD as f32
                            && let Some(tree) = tree_of(&id)
                        {
                            state.gesture = Gesture::Move {
                                tree,
                                origin: (n.x, n.y),
                            };
                        }
                        let now = Instant::now();
                        let twice = state
                            .last_click
                            .as_ref()
                            .is_some_and(|(k, at)| *k == id && now.duration_since(*at) < DOUBLE);
                        state.last_click = Some((id.clone(), now));
                        if twice {
                            state.last_click = None;
                            return Some(publish(FlowEvent::Open(id)));
                        }
                        if self.selected != Some(id.as_str()) {
                            return Some(publish(FlowEvent::Select(Some(id))));
                        }
                    }
                    None => state.gesture = Gesture::Pan { view: self.view },
                }
                Some(canvas::Action::request_redraw().and_capture())
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                let inside = cursor.position_in(bounds).is_some();
                let p = state.cursor.unwrap_or_default();
                if self.carrying.is_some() {
                    state.press = None;
                    if !inside {
                        return Some(canvas::Action::publish(flow_event(FlowEvent::Uncarry)));
                    }
                    let w = Self::world(self.view, p);
                    let to = self.input_at(w);
                    let at = (
                        f64::from(w.x) - core::NODE_W / 2.0,
                        f64::from(w.y) - core::HEAD / 2.0,
                    );
                    return Some(publish(FlowEvent::Drop { at, to }));
                }
                let gesture = std::mem::take(&mut state.gesture);
                let moved = std::mem::take(&mut state.moved);
                let press = state.press.take();
                let w = Self::world(self.view, p);
                let out = match gesture {
                    Gesture::Wire { from } if moved => self
                        .input_at(w)
                        .filter(|(to, _)| *to != from)
                        .map(|(to, port)| FlowEvent::Connect { from, to, port }),
                    Gesture::Pull { child, to, port } if moved => {
                        match self.hit(w).and_then(|h| match h {
                            Hit::Port(id, k) | Hit::Pin(id, k) | Hit::RemovePort(id, k) => {
                                Some((id, k))
                            }
                            _ => None,
                        }) {
                            Some((id, k)) if (id.as_str(), k) != (to.as_str(), port) => {
                                Some(FlowEvent::Connect {
                                    from: child,
                                    to: id,
                                    port: k,
                                })
                            }
                            Some(_) => None,
                            None => Some(FlowEvent::Disconnect { to, port }),
                        }
                    }
                    Gesture::Back { to, port } if moved => match self.hit(w) {
                        Some(Hit::Head(id) | Hit::Node(id) | Hit::Out(id) | Hit::Port(id, _))
                            if id != "r" && id != to =>
                        {
                            Some(FlowEvent::Connect { from: id, to, port })
                        }
                        _ => None,
                    },
                    Gesture::Move { tree, origin } if moved => press.map(|press| {
                        let k = f64::from(self.view.k);
                        FlowEvent::Move {
                            tree,
                            at: (
                                (origin.0 + f64::from(p.x - press.x) / k).round(),
                                (origin.1 + f64::from(p.y - press.y) / k).round(),
                            ),
                        }
                    }),
                    Gesture::Pan { view } => {
                        if moved && let Some(press) = press {
                            Some(FlowEvent::View(View {
                                x: view.x + p.x - press.x,
                                y: view.y + p.y - press.y,
                                k: view.k,
                            }))
                        } else if self.selected.is_some() {
                            Some(FlowEvent::Select(None))
                        } else {
                            None
                        }
                    }
                    _ => None,
                };
                Some(match out {
                    Some(e) => publish(e),
                    None => canvas::Action::request_redraw(),
                })
            }
            mouse::Event::WheelScrolled { delta } => {
                let p = cursor.position_in(bounds)?;
                let steps = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => *y,
                    mouse::ScrollDelta::Pixels { y, .. } => *y / 40.0,
                };
                let v = self.view;
                let k = (v.k * (steps * 0.12).exp()).clamp(0.3, 2.0);
                let (wx, wy) = ((p.x - v.x) / v.k, (p.y - v.y) / v.k);
                Some(publish(FlowEvent::View(View {
                    k,
                    x: p.x - wx * k,
                    y: p.y - wy * k,
                })))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        state: &State,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let t = Tokens::of(theme);
        let syntax = Syntax::of(self.mode);
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), t.field);
        let v = self.current(state);
        let paint = Paint {
            t: &t,
            syntax: &syntax,
            v,
            head: core::HEAD as f32,
            row: core::ROW as f32,
            value: core::VALUE as f32,
        };
        paint.grid(&mut frame, bounds.size());
        // The edge being pulled off is drawn as the wire, not in its place.
        let pulled = match &state.gesture {
            Gesture::Pull { to, port, .. } if state.moved => Some((to.as_str(), *port)),
            _ => None,
        };
        for n in &self.flow.nodes {
            let (nx, ny) = self.at(state, n);
            for (k, port) in n.ports.iter().enumerate() {
                let Some(child) = port.from.as_deref().and_then(|c| self.node(c)) else {
                    continue;
                };
                if pulled == Some((n.id.as_str(), k)) {
                    continue;
                }
                let (cx, cy) = self.at(state, child);
                let on = self.selected.is_some_and(|s| s == n.id || s == child.id);
                paint.edge(
                    &mut frame,
                    Point::new(cx + child.w as f32, cy + paint.head / 2.0),
                    Point::new(nx, ny + port.y as f32),
                    paint.type_color(child.ty),
                    on,
                );
            }
        }
        for n in &self.flow.nodes {
            let at = self.at(state, n);
            paint.node(
                &mut frame,
                n,
                at,
                self.selected == Some(n.id.as_str()),
                self.values.get(&n.id),
            );
        }
        // The wire that follows the pointer.
        if state.moved
            && let Some(p) = state.cursor
        {
            let w = FlowCanvas::world(v, p);
            let wire = match &state.gesture {
                Gesture::Wire { from } => self.node(from).map(|n| {
                    let (x, y) = self.at(state, n);
                    (Point::new(x + n.w as f32, y + paint.head / 2.0), w)
                }),
                Gesture::Pull { child, .. } => self.node(child).map(|n| {
                    let (x, y) = self.at(state, n);
                    (Point::new(x + n.w as f32, y + paint.head / 2.0), w)
                }),
                Gesture::Back { to, port } => self.node(to).map(|n| {
                    let (x, y) = self.at(state, n);
                    (w, Point::new(x, y + n.ports[*port].y as f32))
                }),
                _ => None,
            };
            if let Some((a, b)) = wire {
                paint.wire(&mut frame, a, b);
            }
        }
        // A palette entry being carried: where it would stand.
        if let (Some(key), Some(p)) = (self.carrying, cursor.position_in(bounds)) {
            let w = FlowCanvas::world(v, p);
            paint.ghost(&mut frame, key, w);
        }
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &State,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match state.gesture {
            Gesture::Pan { .. } if state.moved => return mouse::Interaction::Grabbing,
            Gesture::Move { .. } if state.moved => return mouse::Interaction::Grabbing,
            Gesture::Wire { .. } | Gesture::Pull { .. } | Gesture::Back { .. } => {
                return mouse::Interaction::Crosshair;
            }
            _ => {}
        }
        let Some(p) = cursor.position_in(bounds) else {
            return mouse::Interaction::None;
        };
        if self.carrying.is_some() {
            return mouse::Interaction::Copy;
        }
        match self.hit(FlowCanvas::world(self.view, p)) {
            Some(Hit::Out(_) | Hit::Pin(..)) => mouse::Interaction::Crosshair,
            Some(Hit::Plus(_) | Hit::RemovePort(..)) => mouse::Interaction::Pointer,
            Some(Hit::Head(id)) if is_root(&id) && tree_of(&id).is_some_and(|t| t > 0) => {
                mouse::Interaction::Grab
            }
            Some(_) => mouse::Interaction::Idle,
            None => mouse::Interaction::Grab,
        }
    }
}

/// What a frame is painted with.
struct Paint<'a> {
    t: &'a Tokens,
    syntax: &'a Syntax,
    v: View,
    head: f32,
    row: f32,
    value: f32,
}

impl Paint<'_> {
    fn to_screen(&self, p: Point) -> Point {
        Point::new(p.x * self.v.k + self.v.x, p.y * self.v.k + self.v.y)
    }

    fn type_color(&self, ty: Type) -> Color {
        match ty {
            Type::Number => self.syntax.literal,
            Type::Text => self.syntax.text,
            Type::Bool => self.syntax.keyword,
            Type::Any => self.syntax.operator,
        }
    }

    fn accent(&self, kind: NodeKind) -> Color {
        match kind {
            NodeKind::Result => self.t.accent,
            NodeKind::Field => self.syntax.field,
            NodeKind::Variable => self.syntax.variable,
            NodeKind::Number | NodeKind::Constant => self.syntax.literal,
            NodeKind::Text => self.syntax.text,
            NodeKind::Function => self.syntax.function,
            NodeKind::Operator => self.syntax.operator,
            NodeKind::Keyword => self.syntax.keyword,
        }
    }

    /// The dotted working surface.
    fn grid(&self, frame: &mut Frame, size: Size) {
        let step = 20.0 * self.v.k;
        if step < 6.0 {
            return;
        }
        let dot = self.t.border;
        let (ox, oy) = (self.v.x.rem_euclid(step), self.v.y.rem_euclid(step));
        let mut y = oy;
        while y < size.height {
            let mut x = ox;
            while x < size.width {
                frame.fill_rectangle(Point::new(x - 0.5, y - 0.5), Size::new(1.2, 1.2), dot);
                x += step;
            }
            y += step;
        }
    }

    fn curve(&self, a: Point, b: Point) -> Path {
        let (a, b) = (self.to_screen(a), self.to_screen(b));
        let dx = ((b.x - a.x).abs() / 2.0).max(36.0 * self.v.k);
        Path::new(|p| {
            p.move_to(a);
            p.bezier_curve_to(Point::new(a.x + dx, a.y), Point::new(b.x - dx, b.y), b);
        })
    }

    fn edge(&self, frame: &mut Frame, a: Point, b: Point, color: Color, on: bool) {
        let width = if on { 2.6 } else { 1.8 } * self.v.k.max(0.6);
        let color = if on { color } else { color.scale_alpha(0.85) };
        frame.stroke(
            &self.curve(a, b),
            Stroke::default().with_color(color).with_width(width),
        );
    }

    fn wire(&self, frame: &mut Frame, a: Point, b: Point) {
        frame.stroke(
            &self.curve(a, b),
            Stroke {
                line_dash: LineDash {
                    segments: &[6.0, 4.0],
                    offset: 0,
                },
                ..Stroke::default().with_color(self.t.accent).with_width(1.8)
            },
        );
    }

    fn text(
        &self,
        frame: &mut Frame,
        words: String,
        at: Point,
        size: f32,
        color: Color,
        font: Font,
    ) {
        frame.fill_text(Text {
            content: words,
            position: at,
            max_width: f32::INFINITY,
            color,
            size: Pixels(size * self.v.k),
            line_height: LineHeight::Relative(1.2),
            font,
            align_x: Align::Left,
            align_y: Vertical::Center,
            shaping: Shaping::Advanced,
        });
    }

    fn pill(&self, frame: &mut Frame, words: &str, right: Point, color: Color) {
        let k = self.v.k;
        let w = (typography::text_width(words, 10.0) + 10.0) * k;
        let h = 15.0 * k;
        let top_left = Point::new(right.x - w, right.y - h / 2.0);
        frame.fill(
            &Path::rounded_rectangle(top_left, Size::new(w, h), border::radius(h / 2.0)),
            self.t.border.scale_alpha(0.35),
        );
        self.text(
            frame,
            words.to_owned(),
            Point::new(top_left.x + 5.0 * k, right.y),
            10.0,
            color,
            typography::ui(),
        );
    }

    fn node(
        &self,
        frame: &mut Frame,
        n: &FlowNode,
        (x, y): (f32, f32),
        selected: bool,
        value: Option<&String>,
    ) {
        let k = self.v.k;
        let (w, h) = (n.w as f32, n.h as f32);
        let top_left = self.to_screen(Point::new(x, y));
        let size = Size::new(w * k, h * k);
        let radius = 7.0 * k;
        let body = Path::rounded_rectangle(top_left, size, border::radius(radius));
        let accent = self.accent(n.kind);
        let apart = n.id != "r" && tree_of(&n.id).is_some_and(|t| t > 0);
        // The shadow, the body, the accent along the top.
        frame.fill(
            &Path::rounded_rectangle(
                top_left + Vector::new(0.0, 1.5 * k),
                size,
                border::radius(radius),
            ),
            Color::BLACK.scale_alpha(0.16),
        );
        let fill = if n.kind == NodeKind::Result {
            self.t.header
        } else {
            self.t.surface
        };
        frame.fill(&body, fill);
        frame.fill(
            &Path::rounded_rectangle(
                top_left,
                Size::new(size.width, 3.0 * k),
                border::Radius {
                    top_left: radius,
                    top_right: radius,
                    bottom_right: 0.0,
                    bottom_left: 0.0,
                },
            ),
            accent,
        );
        let edge = if selected {
            self.t.accent
        } else if n.error.is_some() {
            self.t.danger
        } else if !n.warnings.is_empty() {
            self.t.warning
        } else {
            self.t.border
        };
        let segments: &[f32] = if apart && !selected { &[4.0, 3.0] } else { &[] };
        frame.stroke(
            &body,
            Stroke {
                line_dash: LineDash {
                    segments,
                    offset: 0,
                },
                ..Stroke::default()
                    .with_color(edge)
                    .with_width(if selected { 2.0 } else { 1.0 })
            },
        );
        // The title row.
        let head_mid = self.to_screen(Point::new(x, y + self.head / 2.0 + 1.0));
        let mut room = w - 20.0;
        if n.grows {
            room -= 18.0;
        }
        if n.error.is_some() || !n.warnings.is_empty() {
            room -= 16.0;
        }
        let (title_font, title_color, title_size) = match n.kind {
            NodeKind::Result => (typography::ui_strong(), self.t.text, 12.0),
            // A sign reads better in the regular face, a little larger: the bold `=` runs its bars together.
            NodeKind::Operator => (typography::mono(), accent, 13.5),
            _ => (typography::mono_strong(), accent, 12.0),
        };
        self.text(
            frame,
            clip(&n.title, room, title_size, n.kind != NodeKind::Result),
            Point::new(head_mid.x + 10.0 * k, head_mid.y),
            title_size,
            title_color,
            title_font,
        );
        let mut right = x + w - 8.0;
        if n.grows {
            self.text(
                frame,
                "+".into(),
                self.to_screen(Point::new(right - 8.0, y + self.head / 2.0 + 1.0)),
                14.0,
                self.t.muted,
                typography::ui_strong(),
            );
            right -= 18.0;
        }
        if n.error.is_some() || !n.warnings.is_empty() {
            let color = if n.error.is_some() {
                self.t.danger
            } else {
                self.t.warning
            };
            let c = self.to_screen(Point::new(right - 6.0, y + self.head / 2.0 + 1.0));
            frame.fill(&Path::circle(c, 6.5 * k), color);
            self.text(
                frame,
                "!".into(),
                Point::new(c.x - 1.8 * k, c.y),
                10.0,
                Color::WHITE,
                typography::ui_strong(),
            );
        }
        if !n.ports.is_empty() || value.is_some() || n.h > core::HEAD + 8.0 {
            let line_y = y + self.head;
            frame.stroke(
                &Path::line(
                    self.to_screen(Point::new(x, line_y)),
                    self.to_screen(Point::new(x + w, line_y)),
                ),
                Stroke::default().with_color(self.t.border).with_width(1.0),
            );
        }
        // The inputs.
        for p in &n.ports {
            let py = y + p.y as f32;
            let color = self.type_color(p.ty);
            let pin = self.to_screen(Point::new(x, py));
            let circle = Path::circle(pin, 5.0 * k);
            if p.from.is_some() {
                frame.fill(&circle, color);
            } else {
                frame.fill(&circle, self.t.field);
                let color = if p.optional {
                    color.scale_alpha(0.55)
                } else {
                    color
                };
                frame.stroke(
                    &circle,
                    Stroke::default().with_color(color).with_width(2.0 * k),
                );
            }
            let name_color = if p.from.is_some() {
                self.t.text
            } else {
                self.t.muted
            };
            self.text(
                frame,
                clip(&p.name, w - 60.0, 11.0, false),
                self.to_screen(Point::new(x + 12.0, py)),
                11.0,
                name_color,
                typography::ui(),
            );
            let row_right = self.to_screen(Point::new(x + w - 6.0, py));
            if p.removable {
                self.text(
                    frame,
                    "×".into(),
                    Point::new(row_right.x - 9.0 * k, row_right.y),
                    12.0,
                    self.t.muted,
                    typography::ui(),
                );
            } else if p.note.is_some() {
                self.text(
                    frame,
                    "ⓘ".into(),
                    Point::new(row_right.x - 11.0 * k, row_right.y),
                    11.0,
                    self.t.warning,
                    typography::ui(),
                );
            } else if p.ty != Type::Any && p.name != type_name(p.ty) {
                self.pill(frame, type_name(p.ty), row_right, color);
            }
        }
        // The value row (not for a constant: its title is its value).
        let room_for_value = h - self.head - n.ports.len() as f32 * self.row >= self.value;
        if room_for_value {
            let vy = y + h - self.value;
            frame.stroke(
                &Path::line(
                    self.to_screen(Point::new(x, vy)),
                    self.to_screen(Point::new(x + w, vy)),
                ),
                Stroke::default()
                    .with_color(self.t.border.scale_alpha(0.5))
                    .with_width(1.0),
            );
            let (words, color, font, size) = match (n.kind, value) {
                (NodeKind::Result, Some(v)) => {
                    (v.clone(), self.t.text, typography::mono_strong(), 12.0)
                }
                (NodeKind::Result, None) => {
                    ("—".to_owned(), self.t.muted, typography::mono(), 12.0)
                }
                (_, Some(v)) => (format!("= {v}"), self.t.muted, typography::mono(), 11.0),
                (_, None) => (String::new(), self.t.muted, typography::mono(), 11.0),
            };
            self.text(
                frame,
                clip(&words, w - 20.0, size, true),
                self.to_screen(Point::new(x + 10.0, vy + self.value / 2.0)),
                size,
                color,
                font,
            );
        }
        // The output.
        if n.kind != NodeKind::Result {
            let out = self.to_screen(Point::new(x + w, y + self.head / 2.0));
            let circle = Path::circle(out, 6.0 * k);
            frame.fill(&circle, self.type_color(n.ty));
            frame.stroke(
                &circle,
                Stroke::default().with_color(fill).with_width(1.5 * k),
            );
        }
    }

    /// Where a carried palette entry would stand.
    fn ghost(&self, frame: &mut Frame, key: &str, w: Point) {
        let k = self.v.k;
        let width = core::NODE_W as f32;
        let top_left = self.to_screen(Point::new(w.x - width / 2.0, w.y - self.head / 2.0));
        let size = Size::new(width * k, (self.head + 4.0) * k);
        let body = Path::rounded_rectangle(top_left, size, border::radius(7.0 * k));
        frame.fill(&body, self.t.surface.scale_alpha(0.8));
        frame.stroke(
            &body,
            Stroke {
                line_dash: LineDash {
                    segments: &[5.0, 3.0],
                    offset: 0,
                },
                ..Stroke::default().with_color(self.t.accent).with_width(1.5)
            },
        );
        let name = key.split_once(':').map_or(key, |(_, n)| n);
        let name = match key {
            "lit:number" => "Sayı",
            "lit:text" => "Metin",
            _ => name,
        };
        self.text(
            frame,
            clip(name, width - 20.0, 12.0, true),
            Point::new(top_left.x + 10.0 * k, top_left.y + size.height / 2.0),
            12.0,
            self.t.text,
            typography::mono_strong(),
        );
    }
}

fn type_name(ty: Type) -> &'static str {
    match ty {
        Type::Any => "değer",
        Type::Number => "sayı",
        Type::Text => "metin",
        Type::Bool => "koşul",
    }
}

/// Words cut to a width (world px at `size`), with “…” when they are.
fn clip(words: &str, width: f32, size: f32, mono: bool) -> String {
    let wide = |s: &str| {
        if mono {
            s.chars().count() as f32 * size * 0.6
        } else {
            typography::text_width(s, size)
        }
    };
    if wide(words) <= width {
        return words.to_owned();
    }
    let mut out: String = words.chars().collect();
    while !out.is_empty() && wide(&format!("{out}…")) > width {
        out.pop();
    }
    format!("{out}…")
}
