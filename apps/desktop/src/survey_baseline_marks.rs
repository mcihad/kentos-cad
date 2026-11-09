//! A GNSS baseline drawn on the map from GNSS arşivi's row (plan/map-cad.md MAP-03, docs/adr/3320): pressing a
//! baseline's row (`bz-N`) selects its two ends' accepted points, as before, and leaves them joined by a dashed line
//! with an arrowhead at its end and the baseline's id and length (the archive's, the vector's ellipsoidal length) at
//! its middle, until İşareti kaldır (`data.unmark`), another baseline's row or another drawing. The ends are the
//! accepted points' objects as the drawing holds them now: a point moved after, the line follows it; a point gone,
//! nothing is drawn. Nothing is written.

use std::sync::Arc;

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::{Pixels, Point, Vector};
use kentos_commands::Entity;
use kentos_domain::Uuid;
use kentos_render_wgpu::Camera;
use kentos_ui::theme::{Tokens, typography};

use crate::app::App;
use crate::viewport::MarkColors;

/// What the row keeps: the drawing's session, the two ends' objects by their lasting ids and the words.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    pub session: u64,
    pub from: String,
    pub to: String,
    pub words: String,
}

/// The baseline on the drawing: its two ends and its words.
#[derive(Clone, Debug, PartialEq)]
pub struct BaselineMark {
    pub from: [f64; 2],
    pub to: [f64; 2],
    pub words: String,
}

impl App {
    /// The baseline for the drawing as it is: none when no row was pressed, for another drawing, or when an end's
    /// object is no longer a point of the drawing.
    pub(crate) fn baseline_marks(&self) -> Option<Arc<BaselineMark>> {
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
        Some(Arc::new(BaselineMark { from: at(&shown.from)?, to: at(&shown.to)?, words: shown.words.clone() }))
    }
}

/// The baseline: a dashed line from its start to its end in the accent, an arrowhead at the end, its words at the
/// middle on a halo, set off the line to its left.
pub(crate) fn draw(frame: &mut canvas::Frame, m: &BaselineMark, camera: &Camera, colors: &MarkColors, t: &Tokens) {
    let screen = |p: [f64; 2]| {
        let s = camera.world_to_screen(kentos_interaction::Vec2::new(p[0], p[1]));
        Point::new(s[0] as f32, s[1] as f32)
    };
    let (a, b) = (screen(m.from), screen(m.to));
    let color = t.accent;
    let line = Path::line(a, b);
    frame.stroke(&line, Stroke::default().with_color(colors.halo).with_width(5.0));
    let dashed = Stroke::default().with_color(color).with_width(2.0);
    frame.stroke(&line, Stroke { line_dash: canvas::LineDash { segments: &[8.0, 5.0], offset: 0 }, ..dashed });
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
