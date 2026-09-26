//! What the running tool shows over the drawing (docs/adr/0021, ADR 0018
//! “Önizleme”): its draft on a canvas layer above KentOS's wgpu drawing
//! area, the measurement tag below-right of the cursor and the value field
//! above-right of it, as widgets.
//!
//! The web draws the same things on its overlay canvas, apart from the
//! document's GPU layers (`PathTool.draw`, `drawTag`, `CursorInput`). The
//! draft changes with every pointer move and holds a few points, so it is
//! not a scene part: the wgpu scene is rebuilt only when the document changes
//! (ADR 0019), and the tag and the field need text, which the pipeline does
//! not draw yet (REN-12). World points become screen points through the
//! float64 camera; the float32 canvas gets only screen offsets.

use iced::widget::canvas::{self, LineDash, Path, Stroke};
use iced::widget::{Space, canvas as canvas_widget, column, container, pin, row, stack, text};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Theme, border, mouse};

use kentos_interaction::{MarkerShape, Preview, Tone, Vec2};
use kentos_render_wgpu::Camera;
use kentos_ui::theme::{Tokens, typography};

use crate::app::Message;
use crate::marks::Marks;

/// The value field's hint (the web's `.cursor-input__hint`).
const HINT: &str = "mesafe · Y,X · @dY,dX · @mesafe<açı";

/// The layer over the drawing area: the selection box and the snap marker
/// (marks.rs, docs/adr/0029), the draft, the tag and the value field.
/// Always present, empty when there is nothing to show, so the drawing
/// area keeps its place in the widget tree (and its gesture state).
pub fn layer<'a>(
    camera: &Camera,
    marks: Marks,
    preview: Option<Preview>,
    field: Option<(&'a str, Vec2)>,
) -> Element<'a, Message> {
    let screen = |p: Vec2| {
        let [x, y] = camera.world_to_screen(p);
        Point::new(x as f32, y as f32)
    };
    let mut layers: Vec<Element<'a, Message>> = Vec::new();
    // The snap colour of the drawing (`--canvas-snap`): a corner's reach is drawn in it.
    let snap = marks.colors.snap;
    if !marks.is_empty() {
        layers.push(canvas_widget(marks).width(Fill).height(Fill).into());
    }
    if let Some(preview) = preview {
        let tag = preview.tag.clone();
        let tag_tone = preview.tag_tone;
        layers.push(
            canvas_widget(Draft {
                preview,
                camera: *camera,
                snap,
            })
            .width(Fill)
            .height(Fill)
            .into(),
        );
        if let Some(tag) = tag {
            let at = screen(tag.at);
            layers.push(
                pin(measurement(tag.lines, tag_tone))
                    .x(at.x.round() + 16.0)
                    .y(at.y.round() + 16.0)
                    .width(Fill)
                    .height(Fill)
                    .into(),
            );
        }
    }
    if let Some((value, at)) = field {
        let at = screen(at);
        layers.push(
            pin(value_field(value))
                .x(at.x.round() + 18.0)
                .y(at.y.round() - 58.0)
                .width(Fill)
                .height(Fill)
                .into(),
        );
    }
    if layers.is_empty() {
        return Space::new().width(Fill).height(Fill).into();
    }
    stack(layers).width(Fill).height(Fill).into()
}

/// The measurement beside the cursor: length and bearing, or the arc's
/// radius and length, and the area once there are three corners; in the
/// danger colour where it names what goes (a vertex to remove).
fn measurement<'a>(lines: Vec<String>, tone: Tone) -> Element<'a, Message> {
    let color = move |t: &Tokens| match tone {
        Tone::Danger => t.danger,
        _ => t.accent,
    };
    let lines = column(lines.into_iter().map(|line| {
        text(line)
            .font(typography::ui_strong())
            .size(typography::caption())
            .style(move |theme: &Theme| text::Style {
                color: Some(color(&Tokens::of(theme))),
            })
            .into()
    }))
    .spacing(1);
    container(lines)
        .padding([3, 6])
        .style(move |theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: Some(t.popover.scale_alpha(0.92).into()),
                border: border::width(1).color(color(&t)),
                ..container::Style::default()
            }
        })
        .into()
}

/// The value field (the web's `CursorInput`): what is typed so far with a
/// caret, and what can be typed.
fn value_field<'a>(value: &'a str) -> Element<'a, Message> {
    let caret =
        container(Space::new().width(1).height(typography::body() + 2.0)).style(|theme: &Theme| {
            container::Style {
                background: Some(Tokens::of(theme).accent.into()),
                ..container::Style::default()
            }
        });
    let input = container(
        row![
            text(value)
                .font(typography::mono())
                .size(typography::body()),
            caret
        ]
        .spacing(1)
        .align_y(iced::Center),
    )
    .padding([0, 8])
    .width(typography::scaled(170.0))
    .height(typography::scaled(26.0))
    .align_y(iced::Center)
    .style(|theme: &Theme| {
        let t = Tokens::of(theme);
        container::Style {
            background: Some(t.field.into()),
            text_color: Some(t.text),
            border: border::rounded(3).width(1).color(t.accent),
            ..container::Style::default()
        }
    });
    let hint = text(HINT)
        .size(typography::caption() - 1.0)
        .style(|theme: &Theme| text::Style {
            color: Some(Tokens::of(theme).muted),
        });
    container(column![input, container(hint).padding([2, 2])].spacing(2))
        .padding(4)
        .style(|theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: Some(t.popover.into()),
                border: border::rounded(4).width(1).color(t.accent.scale_alpha(0.6)),
                shadow: iced::Shadow {
                    color: Color::BLACK.scale_alpha(0.3),
                    offset: iced::Vector::new(0.0, 2.0),
                    blur_radius: 8.0,
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// The draft's lines, in the accent colour (the web's `PathTool.draw`), and
/// the parts a tool gives another tone: what a trim takes away in the danger
/// colour, a corner's reach in the snap colour (docs/adr/0047).
struct Draft {
    preview: Preview,
    camera: Camera,
    snap: Color,
}

impl Draft {
    fn screen(&self, p: Vec2) -> Point {
        let [x, y] = self.camera.world_to_screen(p);
        Point::new(x as f32, y as f32)
    }

    fn path(&self, points: &[Vec2], closed: bool) -> Option<Path> {
        (points.len() >= 2).then(|| {
            Path::new(|b| {
                b.move_to(self.screen(points[0]));
                for p in &points[1..] {
                    b.line_to(self.screen(*p));
                }
                if closed {
                    b.close();
                }
            })
        })
    }
}

impl canvas::Program<Message> for Draft {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let tokens = Tokens::of(theme);
        let accent = tokens.accent;
        let tone = |t: Tone| match t {
            Tone::Accent => accent,
            Tone::Danger => tokens.danger,
            Tone::Snap => self.snap,
        };
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let dashed = |segments: &'static [f32]| Stroke {
            line_dash: LineDash {
                segments,
                offset: 0,
            },
            ..Stroke::default().with_color(accent).with_width(1.0)
        };
        // Filled areas under the lines (the web's `drawArea`): a corridor, a
        // donut; holes left out (even-odd), outlined solid (docs/adr/0057).
        for area in &self.preview.areas {
            let rings: Vec<Path> = area
                .rings
                .iter()
                .filter_map(|r| self.path(r, true))
                .collect();
            if rings.is_empty() {
                continue;
            }
            let shape = Path::new(|b| {
                for ring in &area.rings {
                    if let Some((first, rest)) = ring.split_first() {
                        b.move_to(self.screen(*first));
                        for p in rest {
                            b.line_to(self.screen(*p));
                        }
                        b.close();
                    }
                }
            });
            frame.fill(
                &shape,
                canvas::Fill {
                    style: canvas::Style::Solid(accent.scale_alpha(area.fill)),
                    rule: canvas::fill::Rule::EvenOdd,
                },
            );
            for ring in &rings {
                frame.stroke(
                    ring,
                    Stroke::default().with_color(accent).with_width(area.width),
                );
            }
        }
        // The closed shape once it has three corners: dashed, lightly filled.
        if let Some(ring) = self
            .preview
            .ring
            .as_deref()
            .and_then(|r| self.path(r, true))
        {
            frame.fill(&ring, accent.scale_alpha(0.08));
            frame.stroke(&ring, dashed(&[4.0, 4.0]));
        }
        if let Some(path) = self.path(&self.preview.path, false) {
            frame.stroke(&path, Stroke::default().with_color(accent).with_width(1.0));
        }
        for [a, b] in &self.preview.guides {
            if let Some(guide) = self.path(&[*a, *b], false) {
                frame.stroke(&guide, dashed(&[2.0, 3.0]));
            }
        }
        // The circle, the arc or the rectangle a tool would write, and its guides
        // (docs/adr/0032): as the web's `strokePath` draws them.
        for line in &self.preview.strokes {
            let Some(path) = self.path(&line.pts, line.closed) else {
                continue;
            };
            let dash = line.dash.unwrap_or_default();
            let stroke = Stroke {
                line_dash: LineDash {
                    segments: if line.dash.is_some() { &dash } else { &[] },
                    offset: 0,
                },
                ..Stroke::default()
                    .with_color(tone(line.tone))
                    .with_width(line.width)
            };
            frame.stroke(&path, stroke);
        }
        // A corner found under the cursor, a vertex to add or remove: 2 px marks
        // (docs/adr/0047); Böl's points to come and a perpendicular's right
        // angle: 1 px, where the point falls (docs/adr/0057).
        for m in &self.preview.markers {
            let at = self.screen(m.at);
            let (x, y) = (at.x.round() + 0.5, at.y.round() + 0.5);
            let mark = Path::new(|b| match m.shape {
                MarkerShape::Ring(r) => b.circle(Point::new(x, y), r),
                MarkerShape::Cross(h) => {
                    b.move_to(Point::new(x - h, y - h));
                    b.line_to(Point::new(x + h, y + h));
                    b.move_to(Point::new(x + h, y - h));
                    b.line_to(Point::new(x - h, y + h));
                }
                MarkerShape::Plus(h) => {
                    b.move_to(Point::new(x - h, y));
                    b.line_to(Point::new(x + h, y));
                    b.move_to(Point::new(x, y - h));
                    b.line_to(Point::new(x, y + h));
                }
                MarkerShape::Circle(r) => b.circle(at, r),
                MarkerShape::RightAngle { along, up } => {
                    // The web's `drawRightAngle`: 8 px along each side, on screen.
                    let unit = |to: Vec2| {
                        let s = self.screen(to);
                        let (dx, dy) = (s.x - at.x, s.y - at.y);
                        let l = dx.hypot(dy);
                        (l >= 1e-9).then(|| (dx / l, dy / l))
                    };
                    if let (Some(u), Some(v)) = (unit(along), unit(up)) {
                        let k = 8.0;
                        b.move_to(Point::new(at.x + u.0 * k, at.y + u.1 * k));
                        b.line_to(Point::new(at.x + (u.0 + v.0) * k, at.y + (u.1 + v.1) * k));
                        b.line_to(Point::new(at.x + v.0 * k, at.y + v.1 * k));
                    }
                }
            });
            let width = match m.shape {
                MarkerShape::Circle(_) | MarkerShape::RightAngle { .. } => 1.0,
                _ => 2.0,
            };
            frame.stroke(
                &mark,
                Stroke::default().with_color(tone(m.tone)).with_width(width),
            );
        }
        // Short texts beside points: a reference line's start “A” (docs/adr/0057).
        for label in &self.preview.labels {
            let at = self.screen(label.at);
            frame.fill_text(canvas::Text {
                content: label.text.clone(),
                position: Point::new(at.x + label.offset[0], at.y + label.offset[1]),
                color: tone(label.tone),
                size: iced::Pixels(typography::scaled(11.0)),
                font: typography::ui_strong(),
                align_y: iced::alignment::Vertical::Bottom,
                ..canvas::Text::default()
            });
        }
        // The objects picked for a tangent circle: a 9 px square where each was clicked.
        for p in &self.preview.squares {
            let at = self.screen(*p);
            let square = Path::rectangle(
                Point::new(at.x.round() - 4.5, at.y.round() - 4.5),
                iced::Size::new(9.0, 9.0),
            );
            frame.stroke(
                &square,
                Stroke::default().with_color(accent).with_width(1.0),
            );
        }
        // Points and texts among a modify tool's ghosts: a 7 px square each, solid (docs/adr/0037).
        for p in &self.preview.marks {
            let at = self.screen(*p);
            let square = Path::rectangle(
                Point::new(at.x.round() - 3.5, at.y.round() - 3.5),
                iced::Size::new(7.0, 7.0),
            );
            frame.stroke(
                &square,
                Stroke::default().with_color(accent).with_width(1.0),
            );
        }
        // A polar tracking ray across the area.
        if let Some(tracking) = self.preview.tracking {
            let o = self.screen(tracking.origin);
            let rad = tracking.angle.to_radians();
            let far = Point::new(
                o.x + (rad.cos() * 1e4) as f32,
                o.y - (rad.sin() * 1e4) as f32,
            );
            let ray = Path::line(o, far);
            frame.stroke(
                &ray,
                Stroke {
                    line_dash: LineDash {
                        segments: &[2.0, 4.0],
                        offset: 0,
                    },
                    ..Stroke::default()
                        .with_color(accent.scale_alpha(0.7))
                        .with_width(1.0)
                },
            );
        }
        vec![frame.into_geometry()]
    }
}
