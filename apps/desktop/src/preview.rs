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

use kentos_interaction::{Preview, Vec2};
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
    if !marks.is_empty() {
        layers.push(canvas_widget(marks).width(Fill).height(Fill).into());
    }
    if let Some(preview) = preview {
        let tag = preview.tag.clone();
        layers.push(
            canvas_widget(Draft {
                preview,
                camera: *camera,
            })
            .width(Fill)
            .height(Fill)
            .into(),
        );
        if let Some(tag) = tag {
            let at = screen(tag.at);
            layers.push(
                pin(measurement(tag.lines))
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
/// radius and length, and the area once there are three corners.
fn measurement<'a>(lines: Vec<String>) -> Element<'a, Message> {
    let lines = column(lines.into_iter().map(|line| {
        text(line)
            .font(typography::ui_strong())
            .size(typography::caption())
            .style(|theme: &Theme| text::Style {
                color: Some(Tokens::of(theme).accent),
            })
            .into()
    }))
    .spacing(1);
    container(lines)
        .padding([3, 6])
        .style(|theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: Some(t.popover.scale_alpha(0.92).into()),
                border: border::width(1).color(t.accent),
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

/// The draft's lines, in the accent colour (the web's `PathTool.draw`).
struct Draft {
    preview: Preview,
    camera: Camera,
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
        let accent = Tokens::of(theme).accent;
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let dashed = |segments: &'static [f32]| Stroke {
            line_dash: LineDash {
                segments,
                offset: 0,
            },
            ..Stroke::default().with_color(accent).with_width(1.0)
        };
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
