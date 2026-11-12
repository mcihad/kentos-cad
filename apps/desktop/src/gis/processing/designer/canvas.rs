//! Model tasarımcısı's diagram (the web's ModelCanvas): the boxes and
//! their connections in one world, moved and scaled together. The canvas
//! only draws and reports gestures; the designer changes the draft.
//!
//! - a press on a box selects it, dragging it moves it on the 10 px grid;
//! - dragging from a box's port draws a wire; let go over a step, the menu
//!   asks which of its inputs it feeds;
//! - dragging the background pans; a press without moving selects the model;
//! - the wheel zooms about the pointer; a double click on a box goes to its
//!   settings, on the background shows everything;
//! - a tool carried from the parts is let go here.

use std::time::{Duration, Instant};

use iced::alignment::Vertical;
use iced::widget::canvas::{self, Frame, Geometry, LineDash, Path, Stroke, Text};
use iced::widget::text::{Alignment as Align, LineHeight, Shaping};
use iced::{Color, Pixels, Point, Rectangle, Renderer, Size, Theme, Vector, border, mouse};
use kentos_processing::Registry;
use kentos_processing::designer::{self as plan, Pt, StepMeta, View, canvas as c};
use kentos_processing::model::{Model, step_name};
use kentos_processing::model_edit::{INPUT_TYPES, NodeRef};
use kentos_ui::icon::Icon;
use kentos_ui::theme::{Tokens, typography};

use super::Event;
use crate::app::Message;

/// A second press on the same place within this is a double click.
const DOUBLE: Duration = Duration::from_millis(450);

fn publish(e: Event) -> canvas::Action<Message> {
    canvas::Action::publish(Message::ModelDesigner(e)).and_capture()
}

/// The diagram as the canvas sees it.
pub(super) struct Diagram<'a> {
    pub model: &'a Model,
    pub view: View,
    pub floor: f64,
    pub selected: Option<&'a NodeRef>,
    /// Each step's first problem.
    pub problems: Vec<(&'a str, &'a str)>,
    pub registry: &'a Registry,
    /// A tool carried from the parts: its name and icon.
    pub carrying: Option<(String, Option<String>)>,
    /// The carried tool's id.
    pub carried: Option<&'a str>,
    /// The size the designer last heard of.
    pub known: Option<Size>,
}

#[derive(Default)]
pub(super) struct State {
    gesture: Gesture,
    press: Option<Point>,
    moved: bool,
    cursor: Option<Point>,
    last_click: Option<(Option<NodeRef>, Instant)>,
}

#[derive(Default)]
enum Gesture {
    #[default]
    None,
    Pan {
        view: View,
    },
    Move {
        node: NodeRef,
        origin: (f64, f64),
    },
    Wire {
        from: NodeRef,
    },
}

/// What is under a point of the world.
enum Hit {
    Port(NodeRef),
    Box(NodeRef),
}

fn world(v: View, p: Point) -> Pt {
    Pt::new((f64::from(p.x) - v.x) / v.k, (f64::from(p.y) - v.y) / v.k)
}

fn screen(v: View, p: Pt) -> Point {
    Point::new((p.x * v.k + v.x) as f32, (p.y * v.k + v.y) as f32)
}

impl Diagram<'_> {
    /// The view a pan in progress has moved to.
    fn current(&self, state: &State) -> View {
        match (&state.gesture, state.press, state.cursor) {
            (Gesture::Pan { view }, Some(press), Some(at)) if state.moved => View {
                x: view.x + f64::from(at.x - press.x),
                y: view.y + f64::from(at.y - press.y),
                k: view.k,
            },
            _ => self.view,
        }
    }

    fn input_at(&self, name: &str) -> Pt {
        plan::input_at(self.model, name)
    }

    fn box_of(&self, node: &NodeRef) -> Option<(Pt, f64, f64)> {
        match node {
            NodeRef::Input(name) => Some((self.input_at(name), c::INPUT_W, c::INPUT_H)),
            NodeRef::Step(id) => self
                .model
                .steps
                .iter()
                .find(|s| &s.id == id)
                .map(|s| (plan::step_at(s), c::STEP_W, c::STEP_H)),
        }
    }

    /// Where a box's port is (world); a step whose tool gives nothing has none.
    fn port_of(&self, node: &NodeRef) -> Option<Pt> {
        match node {
            NodeRef::Input(name) => Some(plan::input_port(self.input_at(name))),
            NodeRef::Step(id) => {
                let s = self.model.steps.iter().find(|s| &s.id == id)?;
                let outputs = self
                    .registry
                    .tool(&s.tool)
                    .is_some_and(|t| !t.outputs.is_empty());
                outputs.then(|| plan::step_port(plan::step_at(s)))
            }
        }
    }

    /// The boxes, top first (steps are drawn over inputs).
    fn nodes(&self) -> Vec<NodeRef> {
        let mut out: Vec<NodeRef> = self
            .model
            .steps
            .iter()
            .rev()
            .map(|s| NodeRef::Step(s.id.clone()))
            .collect();
        out.extend(
            self.model
                .inputs
                .iter()
                .rev()
                .map(|i| NodeRef::Input(i.name().to_owned())),
        );
        out
    }

    fn hit(&self, w: Pt, k: f64) -> Option<Hit> {
        // The port answers within its ring (8 screen px).
        let reach = 8.0 / k;
        for node in self.nodes() {
            if let Some(port) = self.port_of(&node) {
                // The ring's centre stands 1 px right of the box's edge (the web's -7 px, 12 px wide).
                let centre = Pt::new(port.x + 1.0, port.y);
                if (w.x - centre.x).hypot(w.y - centre.y) <= reach {
                    return Some(Hit::Port(node));
                }
            }
        }
        for node in self.nodes() {
            if let Some((at, width, height)) = self.box_of(&node)
                && w.x >= at.x
                && w.x <= at.x + width
                && w.y >= at.y
                && w.y <= at.y + height
            {
                return Some(Hit::Box(node));
            }
        }
        None
    }

    /// The step under a point, not `except`.
    fn step_under(&self, w: Pt, except: &NodeRef) -> Option<String> {
        self.model.steps.iter().rev().find_map(|s| {
            let at = plan::step_at(s);
            let inside =
                w.x >= at.x && w.x <= at.x + c::STEP_W && w.y >= at.y && w.y <= at.y + c::STEP_H;
            (inside && *except != NodeRef::Step(s.id.clone())).then(|| s.id.clone())
        })
    }
}

impl canvas::Program<Message> for Diagram<'_> {
    type State = State;

    fn update(
        &self,
        state: &mut State,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Option<canvas::Action<Message>> {
        let size = bounds.size();
        if self.known != Some(size) {
            return Some(canvas::Action::publish(Message::ModelDesigner(
                Event::Resized(size),
            )));
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
                    && f64::from(p.distance(press)) >= c::DRAG
                {
                    state.moved = true;
                }
                if let (Gesture::Move { node, origin }, Some(press), true) =
                    (&state.gesture, state.press, state.moved)
                {
                    let at = (
                        plan::snap(origin.0 + f64::from(p.x - press.x) / self.view.k),
                        plan::snap(origin.1 + f64::from(p.y - press.y) / self.view.k),
                    );
                    return Some(publish(Event::Move {
                        node: node.clone(),
                        at,
                        done: false,
                    }));
                }
                let busy = !matches!(state.gesture, Gesture::None) || self.carried.is_some();
                busy.then(canvas::Action::request_redraw)
            }
            mouse::Event::ButtonPressed(mouse::Button::Left) => {
                let p = cursor.position_in(bounds)?;
                // The zoom buttons are their own widgets over the corner.
                state.press = Some(p);
                state.moved = false;
                state.cursor = Some(p);
                let w = world(self.view, p);
                let hit = self.hit(w, self.view.k);
                let now = Instant::now();
                let target = match &hit {
                    Some(Hit::Box(n) | Hit::Port(n)) => Some(n.clone()),
                    None => None,
                };
                let twice = state
                    .last_click
                    .as_ref()
                    .is_some_and(|(t, at)| *t == target && now.duration_since(*at) < DOUBLE);
                state.last_click = Some((target.clone(), now));
                if twice {
                    state.last_click = None;
                    state.press = None;
                    state.gesture = Gesture::None;
                    return Some(publish(match target {
                        Some(node) => Event::Open(node),
                        None => Event::Fit,
                    }));
                }
                match hit {
                    Some(Hit::Port(node)) => {
                        state.gesture = Gesture::Wire { from: node };
                        Some(canvas::Action::request_redraw().and_capture())
                    }
                    Some(Hit::Box(node)) => {
                        let origin = self
                            .box_of(&node)
                            .map_or((0.0, 0.0), |(at, _, _)| (at.x, at.y));
                        let select = self.selected != Some(&node);
                        state.gesture = Gesture::Move {
                            node: node.clone(),
                            origin,
                        };
                        if select {
                            Some(publish(Event::Select(Some(node))))
                        } else {
                            Some(canvas::Action::request_redraw().and_capture())
                        }
                    }
                    None => {
                        state.gesture = Gesture::Pan { view: self.view };
                        Some(canvas::Action::request_redraw().and_capture())
                    }
                }
            }
            mouse::Event::ButtonReleased(mouse::Button::Left) => {
                let inside = cursor.position_in(bounds);
                if let Some(tool) = self.carried {
                    state.press = None;
                    state.gesture = Gesture::None;
                    return Some(match inside {
                        Some(p) => {
                            let w = world(self.view, p);
                            publish(Event::AddTool {
                                tool: tool.to_owned(),
                                at: Some((w.x, w.y)),
                            })
                        }
                        None => canvas::Action::publish(Message::ModelDesigner(Event::Uncarry)),
                    });
                }
                let gesture = std::mem::take(&mut state.gesture);
                let moved = state.moved;
                let view = self.current(state);
                state.press = None;
                state.moved = false;
                match gesture {
                    Gesture::None => None,
                    Gesture::Pan { .. } if moved => Some(publish(Event::View(view))),
                    Gesture::Pan { .. } => Some(publish(Event::Select(None))),
                    Gesture::Move { node, origin } if moved => {
                        // Where the live moves left it: one undo step now.
                        let at = self.box_of(&node).map_or(origin, |(at, _, _)| (at.x, at.y));
                        Some(publish(Event::Move {
                            node,
                            at,
                            done: true,
                        }))
                    }
                    Gesture::Move { .. } => Some(canvas::Action::request_redraw()),
                    Gesture::Wire { from } => {
                        let p = state.cursor?;
                        let w = world(self.view, p);
                        match (moved, self.step_under(w, &from)) {
                            (true, Some(to)) => Some(publish(Event::Wire { from, to, at: p })),
                            _ => Some(canvas::Action::request_redraw()),
                        }
                    }
                }
            }
            mouse::Event::WheelScrolled { delta } => {
                let p = cursor.position_in(bounds)?;
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => -f64::from(*y) * 100.0,
                    mouse::ScrollDelta::Pixels { y, .. } => -f64::from(*y),
                };
                let f = (-dy * c::WHEEL).exp();
                let view = plan::zoom_at(
                    self.view,
                    Pt::new(f64::from(p.x), f64::from(p.y)),
                    f,
                    self.floor,
                );
                Some(publish(Event::View(view)))
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
        let v = self.current(state);
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), t.field);
        grid(&mut frame, v, bounds.size(), t.border);
        let k = v.k as f32;
        let faint = t.muted.scale_alpha(0.75);

        // The curves, then every label over them, at its target step.
        let labels = plan::edge_labels(self.model, &|id| self.registry.tool(id));
        for label in &labels {
            let (Some(a), Some(target)) = (
                self.port_of(&label.from).or_else(|| match &label.from {
                    NodeRef::Step(id) => self
                        .model
                        .steps
                        .iter()
                        .find(|s| &s.id == id)
                        .map(|s| plan::step_port(plan::step_at(s))),
                    NodeRef::Input(_) => None,
                }),
                self.model.steps.iter().find(|s| s.id == label.to),
            ) else {
                continue;
            };
            let b = plan::step_entry(plan::step_at(target));
            let on = self.selected == Some(&label.from)
                || self.selected == Some(&NodeRef::Step(label.to.clone()));
            let (color, width) = if on { (t.accent, 2.0) } else { (faint, 1.6) };
            frame.stroke(
                &curve(v, a, b),
                Stroke::default()
                    .with_color(color)
                    .with_width(width * k.max(0.5)),
            );
        }
        for label in &labels {
            let at = screen(v, label.at);
            halo_text(&mut frame, &label.text, at, 11.0 * k, faint, t.field);
        }

        // A wire being drawn, and the step it would go to.
        let mut drop: Option<String> = None;
        if let (Gesture::Wire { from }, Some(p)) = (&state.gesture, state.cursor)
            && state.moved
            && let Some(a) = self.port_of(from)
        {
            let w = world(v, p);
            frame.stroke(
                &curve(v, a, w),
                Stroke {
                    line_dash: LineDash {
                        segments: &[6.0 * k, 4.0 * k],
                        offset: 0,
                    },
                    ..Stroke::default()
                        .with_color(t.accent)
                        .with_width(1.6 * k.max(0.5))
                },
            );
            drop = self.step_under(w, from);
        }

        let hovered = cursor
            .position_in(bounds)
            .and_then(|p| match self.hit(world(v, p), v.k) {
                Some(Hit::Box(n) | Hit::Port(n)) => Some(n),
                None => None,
            });
        for input in &self.model.inputs {
            let node = NodeRef::Input(input.name().to_owned());
            let kind = INPUT_TYPES
                .iter()
                .find(|x| x.type_name == input.type_name());
            let meta = plan::texts::canvas::input_meta(
                kind.map_or(input.type_name(), |x| x.label),
                input.0.get("optional").and_then(serde_json::Value::as_bool) == Some(true),
            );
            let look = BoxLook {
                at: self.input_at(input.name()),
                w: c::INPUT_W,
                h: c::INPUT_H,
                icon: crate::icons::from_web(Some(kind.map_or("processing", |x| x.icon))),
                name: input.label().to_owned(),
                meta: Meta::Text(meta),
                input: true,
                selected: self.selected == Some(&node),
                hovered: hovered.as_ref() == Some(&node),
                problem: false,
                drop: false,
                port: true,
                entry: false,
            };
            look.draw(&mut frame, v, &t);
        }
        for step in &self.model.steps {
            let node = NodeRef::Step(step.id.clone());
            let tool = self.registry.tool(&step.tool);
            let problem = self
                .problems
                .iter()
                .find(|(s, _)| *s == step.id)
                .map(|(_, p)| *p);
            let look = BoxLook {
                at: plan::step_at(step),
                w: c::STEP_W,
                h: c::STEP_H,
                icon: crate::icons::from_web(Some(
                    tool.as_ref()
                        .and_then(|t| t.icon.as_deref())
                        .unwrap_or("processing"),
                )),
                name: step_name(step, &|id| self.registry.tool(id)),
                meta: match plan::step_meta(step, tool.as_ref(), problem) {
                    StepMeta::Warn(w) => Meta::Warn(w),
                    StepMeta::Text(t) => Meta::Text(t),
                },
                input: false,
                selected: self.selected == Some(&node),
                hovered: hovered.as_ref() == Some(&node),
                problem: problem.is_some(),
                drop: drop.as_deref() == Some(step.id.as_str()),
                port: tool.is_some_and(|t| !t.outputs.is_empty()),
                entry: true,
            };
            look.draw(&mut frame, v, &t);
        }

        // A tool carried from the parts: where it would stand.
        if let (Some((name, icon)), Some(p)) = (&self.carrying, cursor.position_in(bounds)) {
            ghost(&mut frame, p, name, icon.as_deref(), &t);
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
            Gesture::Wire { .. } => return mouse::Interaction::Crosshair,
            _ => {}
        }
        let Some(p) = cursor.position_in(bounds) else {
            return mouse::Interaction::None;
        };
        if self.carried.is_some() {
            return mouse::Interaction::Copy;
        }
        match self.hit(world(self.view, p), self.view.k) {
            Some(Hit::Port(_)) => mouse::Interaction::Crosshair,
            Some(Hit::Box(_)) => mouse::Interaction::Idle,
            None => mouse::Interaction::Grab,
        }
    }
}

/// The dot grid (twice the snap grid), moving with the view.
fn grid(frame: &mut Frame, v: View, size: Size, color: Color) {
    let step = (c::GRID * 2.0 * v.k) as f32;
    if step < 4.0 {
        return;
    }
    let ox = (v.x as f32).rem_euclid(step);
    let oy = (v.y as f32).rem_euclid(step);
    let dots = Path::new(|b| {
        let mut y = oy;
        while y < size.height {
            let mut x = ox;
            while x < size.width {
                b.circle(Point::new(x, y), 1.0);
                x += step;
            }
            y += step;
        }
    });
    frame.fill(&dots, color);
}

/// An edge from a port to a step's entry (the plan's `curve`), on the screen.
fn curve(v: View, a: Pt, b: Pt) -> Path {
    let (c1, c2) = plan::curve(a, b);
    let (a, c1, c2, b) = (screen(v, a), screen(v, c1), screen(v, c2), screen(v, b));
    Path::new(|p| {
        p.move_to(a);
        p.bezier_curve_to(c1, c2, b);
    })
}

/// A label with a halo of the diagram's ground (the web's 4 px stroke), its
/// right end at `at` on the baseline.
fn halo_text(frame: &mut Frame, content: &str, at: Point, size: f32, color: Color, ground: Color) {
    let text = |p: Point, color: Color| Text {
        content: content.to_owned(),
        position: p,
        color,
        size: Pixels(size),
        font: typography::ui(),
        align_x: Align::Right,
        align_y: Vertical::Bottom,
        line_height: LineHeight::Relative(1.0),
        shaping: Shaping::Advanced,
        ..Text::default()
    };
    for (dx, dy) in [
        (-1.5, 0.0),
        (1.5, 0.0),
        (0.0, -1.5),
        (0.0, 1.5),
        (-1.0, -1.0),
        (1.0, -1.0),
        (-1.0, 1.0),
        (1.0, 1.0),
    ] {
        frame.fill_text(text(Point::new(at.x + dx, at.y + dy), ground));
    }
    frame.fill_text(text(at, color));
}

/// A box's second line.
enum Meta {
    Text(String),
    Warn(String),
}

/// How a box is drawn (the web's `.mnode`).
struct BoxLook {
    at: Pt,
    w: f64,
    h: f64,
    icon: Icon,
    name: String,
    meta: Meta,
    input: bool,
    selected: bool,
    hovered: bool,
    problem: bool,
    drop: bool,
    port: bool,
    entry: bool,
}

impl BoxLook {
    fn draw(&self, frame: &mut Frame, v: View, t: &Tokens) {
        let k = v.k as f32;
        let top_left = screen(v, self.at);
        let size = Size::new(self.w as f32 * k, self.h as f32 * k);
        let radius = 6.0 * k;
        let rect = Path::rounded_rectangle(top_left, size, border::Radius::from(radius));
        if self.drop {
            let ring = Path::rounded_rectangle(
                Point::new(top_left.x - 3.0, top_left.y - 3.0),
                Size::new(size.width + 6.0, size.height + 6.0),
                border::Radius::from(radius + 3.0),
            );
            frame.fill(&ring, t.accent.scale_alpha(0.6));
        }
        frame.fill(&rect, if self.input { t.header } else { t.surface });
        let (edge, dashed) = if self.selected {
            (t.accent, false)
        } else if self.problem {
            (t.warning, true)
        } else if self.hovered {
            (t.muted.scale_alpha(0.75), false)
        } else {
            (t.border, false)
        };
        let stroke = Stroke::default()
            .with_color(edge)
            .with_width(if self.selected { 2.0 } else { 1.0 });
        let stroke = if dashed {
            Stroke {
                line_dash: LineDash {
                    segments: &[4.0, 3.0],
                    offset: 0,
                },
                ..stroke
            }
        } else {
            stroke
        };
        frame.stroke(&rect, stroke);
        if self.input {
            // The blue edge of what is asked for.
            let bar = Path::rounded_rectangle(
                top_left,
                Size::new(3.0 * k.max(0.7), size.height),
                border::Radius {
                    top_left: radius,
                    bottom_left: radius,
                    top_right: 0.0,
                    bottom_right: 0.0,
                },
            );
            frame.fill(&bar, t.info);
        }
        // The icon and the two lines, as the web lays them: 12 px in, 10 px apart.
        let icon_size = 16.0 * k;
        let icon_at = Point::new(
            top_left.x + 12.0 * k,
            top_left.y + (size.height - icon_size) / 2.0,
        );
        let icon_color = if self.selected {
            t.accent_hover
        } else if self.input {
            t.info
        } else {
            t.muted
        };
        kentos_ui::icon::draw(
            frame,
            self.icon,
            icon_color,
            Rectangle::new(icon_at, Size::new(icon_size, icon_size)),
        );
        let text_x = icon_at.x + icon_size + 10.0 * k;
        let room = (top_left.x + size.width - 16.0 * k - text_x).max(0.0);
        let name_size = typography::body() * k;
        let meta_size = typography::caption() * k;
        let middle = top_left.y + size.height / 2.0;
        let name = elide_to(&self.name, name_size, room, true);
        frame.fill_text(Text {
            content: name,
            position: Point::new(text_x, middle - 1.0 * k),
            color: t.text,
            size: Pixels(name_size),
            font: typography::ui_strong(),
            align_y: Vertical::Bottom,
            shaping: Shaping::Advanced,
            ..Text::default()
        });
        let (meta, meta_color, warn) = match &self.meta {
            Meta::Text(s) => (s.as_str(), t.muted.scale_alpha(0.75), false),
            Meta::Warn(s) => (s.as_str(), t.warning, true),
        };
        let mut meta_x = text_x;
        if warn {
            let s = 12.0 * k;
            kentos_ui::icon::draw(
                frame,
                Icon::Warning,
                t.warning,
                Rectangle::new(Point::new(meta_x, middle + 2.0 * k), Size::new(s, s)),
            );
            meta_x += s + 4.0 * k;
        }
        let meta = elide_to(meta, meta_size, (room - (meta_x - text_x)).max(0.0), false);
        frame.fill_text(Text {
            content: meta,
            position: Point::new(meta_x, middle + 1.0 * k),
            color: meta_color,
            size: Pixels(meta_size),
            font: typography::ui(),
            align_y: Vertical::Top,
            shaping: Shaping::Advanced,
            ..Text::default()
        });
        if self.port {
            // The port: a 12 px ring on the right edge (the web's -7 px).
            let centre = Point::new(top_left.x + size.width + 1.0 * k, middle);
            let ring = Path::circle(centre, 6.0 * k);
            frame.fill(&ring, t.surface);
            frame.stroke(
                &ring,
                Stroke::default()
                    .with_color(t.muted.scale_alpha(0.75))
                    .with_width(2.0 * k.max(0.5)),
            );
        }
        if self.entry {
            let dot = Path::circle(Point::new(top_left.x - 0.5 * k, middle), 3.5 * k);
            frame.fill(&dot, t.muted.scale_alpha(0.75));
        }
    }
}

/// A text cut with “…” to `room` px (the web's `text-overflow: ellipsis`).
fn elide_to(text: &str, size: f32, room: f32, strong: bool) -> String {
    let width = if strong {
        typography::strong_width(text, size)
    } else {
        typography::text_width(text, size)
    };
    if width <= room {
        return text.to_owned();
    }
    typography::elide(text, size, room).into_owned()
}

/// The carried tool beside the pointer (the web's ghost: 10 px right, 8 px down).
fn ghost(frame: &mut Frame, p: Point, name: &str, icon: Option<&str>, t: &Tokens) {
    let size = typography::body();
    let width = 10.0 + 15.0 + 6.0 + typography::text_width(name, size) + 10.0;
    let height = size + 12.0;
    let at = Point::new(p.x + 10.0, p.y + 8.0);
    let body = Path::rounded_rectangle(at, Size::new(width, height), border::Radius::from(6.0));
    frame.fill(&body, t.popover);
    frame.stroke(
        &body,
        Stroke::default()
            .with_color(t.accent.scale_alpha(0.6))
            .with_width(1.0),
    );
    kentos_ui::icon::draw(
        frame,
        crate::icons::from_web(Some(icon.unwrap_or("processing"))),
        t.text,
        Rectangle::new(
            Point::new(at.x + 10.0, at.y + (height - 15.0) / 2.0),
            Size::new(15.0, 15.0),
        ),
    );
    frame.fill_text(Text {
        content: name.to_owned(),
        position: Point::new(at.x + 31.0, at.y + height / 2.0),
        color: t.text,
        size: Pixels(size),
        font: typography::ui(),
        align_y: Vertical::Center,
        shaping: Shaping::Advanced,
        ..Text::default()
    });
}
