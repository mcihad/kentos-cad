//! The SVG editor's drawing surface (the web's `SvgCanvas.render` and its
//! overlay): the canvas on its paper with the grid, the tracing reference,
//! the shapes (with the tile preview's copies around them and an array's
//! ghosts), and on top, in screen space, the guides, the draft, the box
//! being dragged, the selection's box and handles or the node tool's
//! markers, the measure, a panel's point, the snap marker and the rulers.
//! The paper and shapes are drawn once per change of the drawing or the
//! view (Iced's canvas cache); the rest follows the pointer.
//!
//! The canvas reports the pointer in stage pixels, with the keys held, and
//! its size; the editor does the rest (`pointer.rs`).

use iced::keyboard::{self, key::Named};
use iced::mouse::{self, Cursor};
use iced::widget::canvas::{self, Frame, Geometry, LineDash, Path, Stroke};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme};
use kentos_svg_core::arrange::{mirror_matrix, polar_array, rect_array};
use kentos_svg_core::model::{shapes_box, transform_shape};
use kentos_svg_core::path::Matrix;
use kentos_ui::theme::{Mode, Tokens};

use super::ToolId;
use super::draw_tool::draw_draft;
use super::hit::Hit;
use super::measure::draw_measure;
use super::node_tool::{NodeColors, draw_nodes};
use super::paint::{View, paint_shape};
use super::pointer::Op;
use super::rulers::{RulerColors, draw_guides, draw_rulers};
use super::snap::draw_snap;
use super::state::{ArrayKind, At, Options, SvgEditor, Tab};
use super::{Event, ev};
use crate::app::Message;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Middle,
    Right,
}

/// What the canvas tells the editor.
#[derive(Clone, Debug)]
pub enum Input {
    Down {
        s: [f64; 2],
        button: Button,
        shift: bool,
        space: bool,
    },
    Move {
        s: [f64; 2],
        shift: bool,
        alt: bool,
    },
    Up {
        alt: bool,
    },
    Wheel {
        s: [f64; 2],
        up: bool,
    },
    Resized(f64, f64),
    Left,
}

/// The canvas's colours: the theme's tokens and the web's canvas colours.
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub field: Color,
    pub panel: Color,
    pub panel_head: Color,
    pub line: Color,
    pub line_strong: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub danger: Color,
    /// `--canvas-snap`.
    pub snap: Color,
    /// `--c-info`: guides and a panel's point.
    pub info: Color,
}

impl Palette {
    pub fn of(theme: &Theme) -> Palette {
        let t = Tokens::of(theme);
        let dark = Mode::of(theme).is_dark();
        Palette {
            field: t.field,
            panel: t.surface,
            panel_head: t.header,
            line: t.border,
            line_strong: t.muted,
            text: t.text,
            muted: t.muted,
            accent: t.accent,
            danger: t.danger,
            snap: if dark {
                Color::from_rgb8(0x6f, 0xd0, 0x8c)
            } else {
                Color::from_rgb8(0x1a, 0x9a, 0x48)
            },
            info: if dark {
                Color::from_rgb8(0x6d, 0xb3, 0xf2)
            } else {
                Color::from_rgb8(0x1f, 0x6f, 0xc4)
            },
        }
    }
}

/// The canvas widget's program; `ink` and `paper` are the preview's colours
/// as the theme gives them (the symbol's colour follows the theme until one
/// is picked, the paper unless the drawing keeps its own).
pub struct Stage<'a> {
    pub ed: &'a SvgEditor,
    pub ink: String,
    pub paper: String,
}

/// What the cached picture was drawn with, beyond the drawing: the theme's and the preview's colours.
fn stamp(pal: &Palette, o: &Options) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for c in [pal.field, pal.line, pal.line_strong, pal.panel] {
        c.into_rgba8().hash(&mut h);
    }
    o.ink.hash(&mut h);
    o.second.hash(&mut h);
    o.paper.hash(&mut h);
    h.finish()
}

/// What the canvas remembers between events.
#[derive(Debug, Default)]
pub struct StageState {
    size: Option<Size>,
    modifiers: keyboard::Modifiers,
    space: bool,
    pressed: Option<Button>,
}

fn publish(input: Input) -> canvas::Action<Message> {
    canvas::Action::publish(ev(Event::Stage(input))).and_capture()
}

fn at(p: Point) -> [f64; 2] {
    [f64::from(p.x), f64::from(p.y)]
}

impl canvas::Program<Message> for Stage<'_> {
    type State = StageState;

    fn update(
        &self,
        state: &mut StageState,
        event: &canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> Option<canvas::Action<Message>> {
        if state.size != Some(bounds.size()) {
            state.size = Some(bounds.size());
            return Some(canvas::Action::publish(ev(Event::Stage(Input::Resized(
                f64::from(bounds.width),
                f64::from(bounds.height),
            )))));
        }
        // The pointer relative to the stage, also outside it while a button is held.
        let local = cursor
            .position()
            .map(|p| Point::new(p.x - bounds.x, p.y - bounds.y));
        let inside = cursor.is_over(bounds);
        match event {
            canvas::Event::Keyboard(keyboard::Event::ModifiersChanged(m)) => {
                state.modifiers = *m;
                None
            }
            canvas::Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(Named::Space),
                ..
            }) if inside => {
                state.space = true;
                None
            }
            canvas::Event::Keyboard(keyboard::Event::KeyReleased {
                key: keyboard::Key::Named(Named::Space),
                ..
            }) => {
                state.space = false;
                None
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(b)) if inside => {
                let button = match b {
                    mouse::Button::Left => Button::Left,
                    mouse::Button::Middle => Button::Middle,
                    mouse::Button::Right => Button::Right,
                    _ => return None,
                };
                state.pressed = Some(button);
                let s = at(local?);
                Some(publish(Input::Down {
                    s,
                    button,
                    shift: state.modifiers.shift(),
                    space: state.space,
                }))
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if !inside && state.pressed.is_none() {
                    return None;
                }
                Some(canvas::Action::publish(ev(Event::Stage(Input::Move {
                    s: at(local?),
                    shift: state.modifiers.shift(),
                    alt: state.modifiers.alt(),
                }))))
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(b)) => {
                let button = match b {
                    mouse::Button::Left => Button::Left,
                    mouse::Button::Middle => Button::Middle,
                    mouse::Button::Right => Button::Right,
                    _ => return None,
                };
                if state.pressed != Some(button) {
                    return None;
                }
                state.pressed = None;
                Some(publish(Input::Up {
                    alt: state.modifiers.alt(),
                }))
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) if inside => {
                let dy = match delta {
                    mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => {
                        *y
                    }
                };
                if dy == 0.0 {
                    return None;
                }
                Some(publish(Input::Wheel {
                    s: at(local?),
                    up: dy > 0.0,
                }))
            }
            canvas::Event::Mouse(mouse::Event::CursorLeft) if state.pressed.is_none() => {
                Some(canvas::Action::publish(ev(Event::Stage(Input::Left))))
            }
            _ => None,
        }
    }

    fn draw(
        &self,
        _state: &StageState,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let ed = self.ed;
        let pal = Palette::of(theme);
        let mut o = ed.options.clone();
        o.ink.clone_from(&self.ink);
        o.paper.clone_from(&self.paper);
        let key = (ed.revision, stamp(&pal, &o));
        if ed.drawn.get() != key {
            ed.shapes_cache.clear();
            ed.drawn.set(key);
        }
        let size = bounds.size();
        let base = ed
            .shapes_cache
            .draw(renderer, size, |frame| draw_base(frame, ed, &o, &pal));
        let mut over = Frame::new(renderer, size);
        draw_overlay(&mut over, ed, &o, &pal, size);
        vec![base, over.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        state: &StageState,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> mouse::Interaction {
        let ed = self.ed;
        if matches!(ed.op, Some(Op::Pan { .. } | Op::Reference { .. })) {
            return mouse::Interaction::Grabbing;
        }
        let Some(p) = cursor.position_in(bounds) else {
            return mouse::Interaction::default();
        };
        if state.space {
            return mouse::Interaction::Grab;
        }
        match ed.hit_at(at(p)) {
            Hit::Ruler(super::hit::Axis::H) => mouse::Interaction::ResizingVertically,
            Hit::Ruler(super::hit::Axis::V) => mouse::Interaction::ResizingHorizontally,
            Hit::Handle(1 | 5) => mouse::Interaction::ResizingVertically,
            Hit::Handle(3 | 7) => mouse::Interaction::ResizingHorizontally,
            Hit::Handle(0 | 4) => mouse::Interaction::ResizingDiagonallyDown,
            Hit::Handle(_) => mouse::Interaction::ResizingDiagonallyUp,
            Hit::Rot => mouse::Interaction::Grab,
            Hit::Node(..) | Hit::Guide(_) => mouse::Interaction::Move,
            Hit::Shape(_) if ed.tool == ToolId::Select => mouse::Interaction::Pointer,
            _ => mouse::Interaction::Crosshair,
        }
    }
}

fn view_of(ed: &SvgEditor) -> View {
    View {
        zoom: ed.camera.zoom,
        ox: ed.camera.ox,
        oy: ed.camera.oy,
    }
}

/// The copies an array or mirror would make (the Dizi tab with its preview on).
pub fn array_matrices(ed: &SvgEditor) -> Option<Vec<Matrix>> {
    let sel = ed.chosen();
    let b = shapes_box(&sel).ok().flatten()?;
    let a = &ed.ui.array;
    let centre = |at: At, point: Option<[f64; 2]>| match at {
        At::Box => Some([(b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0]),
        At::Canvas => Some([ed.doc.width / 2.0, ed.doc.height / 2.0]),
        At::Point => point,
    };
    match a.kind {
        ArrayKind::Rect => Some(rect_array(
            &b,
            a.rect.rows,
            a.rect.cols,
            a.rect.dx,
            a.rect.dy,
            a.rect.gap,
        )),
        ArrayKind::Polar => {
            let c = centre(a.polar.at, a.polar.point)?;
            Some(polar_array(
                &b,
                a.polar.count,
                a.polar.angle,
                c,
                a.polar.rotate,
                a.polar.ccw,
            ))
        }
        ArrayKind::Mirror => {
            let c = centre(a.mirror.at, a.mirror.point)?;
            Some(vec![mirror_matrix(a.mirror.axis, c, a.mirror.deg)])
        }
    }
}

/// The grid, the shapes, the tile preview, the ghosts and the canvas's frame.
fn draw_base(frame: &mut Frame, ed: &SvgEditor, o: &Options, pal: &Palette) {
    let view = view_of(ed);
    let doc = &ed.doc;
    // The field and the paper are the backdrop's, under the reference (view.rs).
    let a = view.point([0.0, 0.0]);
    let b = view.point([doc.width, doc.height]);
    let paper = Path::rectangle(a, Size::new(b.x - a.x, b.y - a.y));
    if o.grid > 0.0 && o.grid * view.zoom >= 5.0 {
        let grid = Path::new(|p| {
            let mut x = 0.0;
            while x <= doc.width + 1e-9 {
                p.move_to(view.point([x, 0.0]));
                p.line_to(view.point([x, doc.height]));
                x += o.grid;
            }
            let mut y = 0.0;
            while y <= doc.height + 1e-9 {
                p.move_to(view.point([0.0, y]));
                p.line_to(view.point([doc.width, y]));
                y += o.grid;
            }
        });
        frame.stroke(
            &grid,
            Stroke::default()
                .with_color(Color {
                    a: pal.line.a * 0.55,
                    ..pal.line
                })
                .with_width(1.0),
        );
    }
    if o.tile {
        // The drawing repeated around itself, as a pattern fill tiles it.
        for dx in [-1.0, 0.0, 1.0] {
            for dy in [-1.0, 0.0, 1.0] {
                if dx == 0.0 && dy == 0.0 {
                    continue;
                }
                let v = view.shifted(dx * doc.width, dy * doc.height);
                for s in doc.shapes.iter().filter(|s| !s.is("hidden")) {
                    paint_shape(frame, s, &v, o, 0.45);
                }
            }
        }
    }
    for s in doc.shapes.iter().filter(|s| !s.is("hidden")) {
        paint_shape(frame, s, &view, o, 1.0);
    }
    // The copies an array or mirror would make (at most 400 shapes).
    if ed.ui.tab == Tab::Array
        && ed.ui.array.preview
        && let Some(ms) = array_matrices(ed)
    {
        let src: Vec<_> = ed
            .chosen()
            .into_iter()
            .filter(|s| !s.is("hidden"))
            .collect();
        let mut count = 0;
        'all: for m in &ms {
            for s in &src {
                count += 1;
                if count > 401 {
                    break 'all;
                }
                if let Ok(Some(g)) = transform_shape(s, m) {
                    paint_shape(frame, &g, &view, o, 0.32);
                }
            }
        }
    }
    frame.stroke(
        &paper,
        Stroke::default()
            .with_color(pal.line_strong)
            .with_width(1.0),
    );
}

/// What follows the pointer, in screen space.
fn draw_overlay(frame: &mut Frame, ed: &SvgEditor, o: &Options, pal: &Palette, size: Size) {
    let view = view_of(ed);
    let rc = RulerColors {
        panel_head: pal.panel_head,
        line: pal.line,
        text: pal.muted,
        accent: pal.accent,
        info: pal.info,
        danger: pal.danger,
    };
    draw_guides(frame, ed, size, &rc);
    draw_draft(frame, ed, &view, pal.accent, pal.panel);
    if let Some(Op::Draw { p0, p1 }) = &ed.op
        && let Some(s) = ed.shape_from_drag(*p0, *p1, false)
    {
        paint_shape(frame, &s, &view, o, 0.6);
    }
    if let Some(Op::Marquee { p0, p1, .. }) = &ed.op {
        let (a, b) = (view.point(*p0), view.point(*p1));
        let r = Path::rectangle(
            Point::new(a.x.min(b.x), a.y.min(b.y)),
            Size::new((a.x - b.x).abs(), (a.y - b.y).abs()),
        );
        frame.fill(
            &r,
            Color {
                a: 0.12,
                ..pal.accent
            },
        );
        frame.stroke(&r, Stroke::default().with_color(pal.accent).with_width(1.0));
    }
    if ed.node_edit.is_some() && ed.node_shape().is_some() {
        draw_nodes(
            frame,
            ed,
            &view,
            &NodeColors {
                accent: pal.accent,
                panel: pal.panel,
                text: pal.text,
            },
        );
    } else if let Some((a, c, locked)) = ed.handle_box() {
        let dashed = Stroke {
            line_dash: LineDash {
                segments: &[4.0, 3.0],
                offset: 0,
            },
            ..Stroke::default().with_color(pal.accent).with_width(1.0)
        };
        let (x0, y0, x1, y1) = (a[0] as f32, a[1] as f32, c[0] as f32, c[1] as f32);
        frame.stroke(
            &Path::rectangle(Point::new(x0, y0), Size::new(x1 - x0, y1 - y0)),
            dashed,
        );
        if !locked {
            let edge = Stroke::default().with_color(pal.accent).with_width(1.2);
            for h in SvgEditor::handles(a, c) {
                let r = Path::rectangle(
                    Point::new(h[0] as f32 - 4.0, h[1] as f32 - 4.0),
                    Size::new(8.0, 8.0),
                );
                frame.fill(&r, pal.panel);
                frame.stroke(&r, edge);
            }
            let mx = (x0 + x1) / 2.0;
            frame.stroke(
                &Path::line(Point::new(mx, y0), Point::new(mx, y0 - 22.0)),
                dashed,
            );
            let knob = Path::circle(Point::new(mx, y0 - 26.0), 5.0);
            frame.fill(&knob, pal.panel);
            frame.stroke(&knob, edge);
        }
    }
    if ed.tool == ToolId::Measure {
        draw_measure(frame, ed, &view, pal.accent, pal.muted, pal.panel);
    }
    if let Some(m) = ed.marker {
        let p = view.point(m);
        let pin = Path::new(|b| {
            b.move_to(Point::new(p.x - 9.0, p.y));
            b.line_to(Point::new(p.x + 9.0, p.y));
            b.move_to(Point::new(p.x, p.y - 9.0));
            b.line_to(Point::new(p.x, p.y + 9.0));
            b.circle(p, 4.0);
        });
        frame.stroke(&pin, Stroke::default().with_color(pal.info).with_width(1.6));
    }
    if ed.op.is_some()
        || ed.draw.drafting()
        || ed.picking.is_some()
        || ed.tool == ToolId::Measure
        || ed.nodes.busy()
        || ed.rulers.busy()
    {
        draw_snap(frame, ed, pal.snap, pal.panel);
    }
    draw_rulers(frame, ed, size, &rc);
}
