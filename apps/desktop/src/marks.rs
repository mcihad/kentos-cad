//! Marks over the drawing that follow the pointer (docs/adr/0029), drawn on
//! Iced's canvas like the draft (preview.rs), as the web draws them on its
//! overlay canvas (`viewport/overlay.ts` `drawSnap`, `tools/preview.ts`
//! `drawSelectionBox`):
//!
//! - the object snap marker: its kind's glyph in the snap colour
//!   (`--canvas-snap`), 1.5 px, and its Turkish name above-right of it with
//!   a halo of the area's colour, so it never meets the measurement below-right;
//! - the selection box: left to right a window (blue, solid), right to left
//!   a crossing (the snap colour, dashed 5/4), each filled at 10 %.
//!
//! They change with every pointer move and hold one glyph or one box, so
//! they are not scene parts; the selection's own highlight is (viewport.rs).

use iced::widget::canvas::{self, LineDash, Path, Stroke, Text};
use iced::{Pixels, Point, Rectangle, Renderer, Size, Theme, Vector, mouse};

use kentos_interaction::{SelectBox, SnapHit, SnapKind};
use kentos_render_wgpu::Camera;
use kentos_ui::theme::typography;

use crate::viewport::MarkColors;

/// A snap kind as the web names it beside the marker (`SNAP_LABEL`).
pub fn snap_label(kind: SnapKind) -> &'static str {
    match kind {
        SnapKind::Endpoint => "Uç nokta",
        SnapKind::Midpoint => "Orta nokta",
        SnapKind::Center => "Merkez",
        SnapKind::Node => "Nokta",
        SnapKind::Quadrant => "Çeyrek",
        SnapKind::Intersection => "Kesişim",
        SnapKind::Perpendicular => "Dik",
        SnapKind::Tangent => "Teğet",
        SnapKind::Nearest => "En yakın",
    }
}

/// A snap kind as the traces write it (the web's `SnapKind` names).
pub fn snap_name(kind: SnapKind) -> &'static str {
    match kind {
        SnapKind::Endpoint => "endpoint",
        SnapKind::Midpoint => "midpoint",
        SnapKind::Center => "center",
        SnapKind::Node => "node",
        SnapKind::Quadrant => "quadrant",
        SnapKind::Intersection => "intersection",
        SnapKind::Perpendicular => "perpendicular",
        SnapKind::Tangent => "tangent",
        SnapKind::Nearest => "nearest",
    }
}

/// The marks' canvas program: the snap marker and the selection box, when there are.
pub struct Marks {
    pub camera: Camera,
    pub snap: Option<SnapHit>,
    pub select: Option<SelectBox>,
    pub colors: MarkColors,
}

impl Marks {
    pub fn is_empty(&self) -> bool {
        self.snap.is_none() && self.select.is_none()
    }
}

impl<Message> canvas::Program<Message> for Marks {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        if let Some(b) = self.select {
            select_box(&mut frame, b, &self.colors);
        }
        if let Some(hit) = self.snap {
            let [x, y] = self.camera.world_to_screen(hit.point);
            // On the pixel's middle, as the web strokes it.
            let at = Point::new(x.round() as f32 + 0.5, y.round() as f32 + 0.5);
            snap_marker(&mut frame, hit.kind, at, &self.colors);
        }
        vec![frame.into_geometry()]
    }
}

/// The web's `drawSelectionBox`.
fn select_box(frame: &mut canvas::Frame, b: SelectBox, colors: &MarkColors) {
    let crossing = b.crossing();
    let color = if crossing { colors.snap } else { colors.window };
    let (x0, y0) = (b.from[0].min(b.to[0]) as f32, b.from[1].min(b.to[1]) as f32);
    let size = Size::new(
        (b.to[0] - b.from[0]).abs() as f32,
        (b.to[1] - b.from[1]).abs() as f32,
    );
    frame.fill_rectangle(Point::new(x0, y0), size, color.scale_alpha(0.1));
    let outline = Path::rectangle(Point::new(x0 + 0.5, y0 + 0.5), size);
    let segments: &[f32] = if crossing { &[5.0, 4.0] } else { &[] };
    frame.stroke(
        &outline,
        Stroke {
            line_dash: LineDash {
                segments,
                offset: 0,
            },
            ..Stroke::default().with_color(color).with_width(1.0)
        },
    );
}

/// The web's `drawSnap`: the kind's glyph and its name above-right.
fn snap_marker(frame: &mut canvas::Frame, kind: SnapKind, at: Point, colors: &MarkColors) {
    let (x, y) = (at.x, at.y);
    let p = |dx: f32, dy: f32| Point::new(x + dx, y + dy);
    let glyph = Path::new(|b| match kind {
        SnapKind::Midpoint => {
            b.move_to(p(0.0, -6.0));
            b.line_to(p(6.0, 5.0));
            b.line_to(p(-6.0, 5.0));
            b.close();
        }
        SnapKind::Center | SnapKind::Node => b.circle(at, 5.5),
        SnapKind::Quadrant => {
            b.move_to(p(0.0, -6.0));
            b.line_to(p(6.0, 0.0));
            b.line_to(p(0.0, 6.0));
            b.line_to(p(-6.0, 0.0));
            b.close();
        }
        SnapKind::Intersection => {
            b.move_to(p(-5.0, -5.0));
            b.line_to(p(5.0, 5.0));
            b.move_to(p(5.0, -5.0));
            b.line_to(p(-5.0, 5.0));
        }
        SnapKind::Perpendicular => {
            b.move_to(p(-6.0, -6.0));
            b.line_to(p(-6.0, 5.0));
            b.line_to(p(6.0, 5.0));
            b.move_to(p(-6.0, 0.0));
            b.line_to(p(0.0, 0.0));
            b.line_to(p(0.0, 5.0));
        }
        SnapKind::Tangent => {
            b.circle(p(0.0, 1.0), 4.5);
            b.move_to(p(-6.5, -4.5));
            b.line_to(p(6.5, -4.5));
        }
        SnapKind::Nearest => {
            b.move_to(p(-5.0, -5.0));
            b.line_to(p(5.0, -5.0));
            b.line_to(p(-5.0, 5.0));
            b.line_to(p(5.0, 5.0));
            b.close();
        }
        SnapKind::Endpoint => b.rectangle(p(-5.0, -5.0), Size::new(10.0, 10.0)),
    });
    frame.stroke(
        &glyph,
        Stroke::default().with_color(colors.snap).with_width(1.5),
    );
    // Above-right; bottom of the text at y − 7, with the area's colour as a halo.
    let label = Text {
        content: snap_label(kind).to_owned(),
        position: p(9.0, -7.0),
        color: colors.halo,
        size: Pixels(typography::scaled(10.5)),
        font: typography::ui_strong(),
        align_y: iced::alignment::Vertical::Bottom,
        ..Text::default()
    };
    for (dx, dy) in [
        (-1.0, 0.0),
        (1.0, 0.0),
        (0.0, -1.0),
        (0.0, 1.0),
        (-1.0, -1.0),
        (1.0, 1.0),
        (-1.0, 1.0),
        (1.0, -1.0),
    ] {
        frame.fill_text(Text {
            position: label.position + Vector::new(dx, dy),
            ..label.clone()
        });
    }
    frame.fill_text(Text {
        color: colors.snap,
        ..label
    });
}
