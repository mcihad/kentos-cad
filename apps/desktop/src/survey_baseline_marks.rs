//! GNSS baselines drawn on the map from GNSS arşivi (plan/map-cad.md MAP-03, docs/adr/3320, 3720): pressing a
//! baseline's row (`bz-N`) selects its two ends' accepted points, as before, and leaves them joined by a dashed line
//! with an arrowhead at its end and the baseline's id and length (the archive's, the vector's ellipsoidal length) at
//! its middle; Ağı göster (docs/adr/3720) draws every baseline whose ends have accepted points at once, the network's
//! view, a baseline left out in the danger colour, dotted. They stay until İşareti kaldır (`data.unmark`), another
//! row or another drawing. The ends are the accepted points' objects as the drawing holds them now: a point moved
//! after, the line follows it; a point gone, that line is not drawn. Nothing is written.

use std::sync::Arc;

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::{Pixels, Point, Vector};
use kentos_commands::Entity;
use kentos_domain::Uuid;
use kentos_render_wgpu::Camera;
use kentos_ui::theme::{Tokens, typography};

use crate::app::App;
use crate::viewport::MarkColors;

/// A baseline as a row or the network keeps it: the two ends' objects by their lasting ids, the words and whether it
/// is left out (docs/adr/3720).
#[derive(Clone, Debug, PartialEq)]
pub struct ShownLine {
    pub from: String,
    pub to: String,
    pub words: String,
    pub excluded: bool,
}

/// What was asked to be drawn: the drawing's session and the baselines.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    pub session: u64,
    pub lines: Vec<ShownLine>,
}

/// A baseline on the drawing: its two ends, its words and whether it is left out.
#[derive(Clone, Debug, PartialEq)]
pub struct BaselineMark {
    pub from: [f64; 2],
    pub to: [f64; 2],
    pub words: String,
    pub excluded: bool,
}

/// The baselines on the drawing (one from a row, the network from Ağı göster).
#[derive(Clone, Debug, PartialEq)]
pub struct BaselineMarks {
    pub lines: Vec<BaselineMark>,
}

impl App {
    /// The baselines for the drawing as it is: none when nothing was asked, for another drawing, or when no line has
    /// both ends' objects still points of the drawing (a line whose end is gone is left out).
    pub(crate) fn baseline_marks(&self) -> Option<Arc<BaselineMarks>> {
        let shown = self.baseline_shown.as_ref()?;
        let doc = self.document.as_ref()?;
        if doc.session != shown.session {
            return None;
        }
        let at = |uid: &str| -> Option<[f64; 2]> {
            let slot = doc.model.slot_of(Uuid::parse_str(uid).ok()?)?;
            match doc.model.get(slot)? {
                Entity::Point(p) => Some([p.p.x, p.p.y]),
                _ => None,
            }
        };
        let lines: Vec<BaselineMark> = shown
            .lines
            .iter()
            .filter_map(|l| {
                Some(BaselineMark { from: at(&l.from)?, to: at(&l.to)?, words: l.words.clone(), excluded: l.excluded })
            })
            .collect();
        (!lines.is_empty()).then(|| Arc::new(BaselineMarks { lines }))
    }
}

/// The baselines, each by [`draw_one`].
pub(crate) fn draw(frame: &mut canvas::Frame, m: &BaselineMarks, camera: &Camera, colors: &MarkColors, t: &Tokens) {
    for line in &m.lines {
        draw_one(frame, line, camera, colors, t);
    }
}

/// A baseline: a dashed line from its start to its end in the accent (left out, docs/adr/3720: dotted in the danger
/// colour), an arrowhead at the end, its words at the middle on a halo, set off the line to its left.
fn draw_one(frame: &mut canvas::Frame, m: &BaselineMark, camera: &Camera, colors: &MarkColors, t: &Tokens) {
    let screen = |p: [f64; 2]| {
        let s = camera.world_to_screen(kentos_interaction::Vec2::new(p[0], p[1]));
        Point::new(s[0] as f32, s[1] as f32)
    };
    let (a, b) = (screen(m.from), screen(m.to));
    let color = if m.excluded { t.danger } else { t.accent };
    let segments: &[f32] = if m.excluded { &[3.0, 4.0] } else { &[8.0, 5.0] };
    let line = Path::line(a, b);
    frame.stroke(&line, Stroke::default().with_color(colors.halo).with_width(5.0));
    let dashed = Stroke::default().with_color(color).with_width(2.0);
    frame.stroke(&line, Stroke { line_dash: canvas::LineDash { segments, offset: 0 }, ..dashed });
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let length = (dx * dx + dy * dy).sqrt();
    if length < 1.0 {
        return;
    }
    let (ux, uy) = (dx / length, dy / length);
    // The arrowhead: 12 px back along the line, 5 px to each side.
    let back = Point::new(b.x - ux * 12.0, b.y - uy * 12.0);
    let head = Path::new(|p| {
        p.move_to(b);
        p.line_to(Point::new(back.x - uy * 5.0, back.y + ux * 5.0));
        p.line_to(Point::new(back.x + uy * 5.0, back.y - ux * 5.0));
        p.close();
    });
    frame.fill(&head, color);
    for end in [a, b] {
        frame.stroke(&Path::circle(end, 5.0), Stroke::default().with_color(color).with_width(2.0));
    }
    let middle = Point::new((a.x + b.x) / 2.0 + uy * 10.0, (a.y + b.y) / 2.0 - ux * 10.0);
    let text = Text {
        content: m.words.clone(),
        position: middle,
        color: colors.halo,
        size: Pixels(typography::scaled(11.0)),
        font: typography::ui_strong(),
        align_y: iced::alignment::Vertical::Center,
        ..Text::default()
    };
    for (ox, oy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
        frame.fill_text(Text { position: text.position + Vector::new(ox, oy), ..text.clone() });
    }
    frame.fill_text(Text { color, ..text });
}
