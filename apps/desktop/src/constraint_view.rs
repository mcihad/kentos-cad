//! The constraints over the drawing (docs/adr/0338 §1; AutoCAD's constraint bars): each constraint's mark by each part
//! it names, its kind's icon in a small box beside the part, a dimension's name and value after it, a constraint the
//! drawing does not meet in the danger colour; the pointer on a mark shows the parts it names, in the accent colour,
//! and its name. Kısıtları göster (`constraint.show`) hides them and shows them again. The marks are worked out once
//! per change of the drawing (`kentos_interaction::constraint_marks`), not in every frame.

use std::cell::RefCell;
use std::sync::Arc;

use iced::widget::canvas::{self, Path, Stroke, Text};
use iced::{Pixels, Point, Rectangle, Size};
use kentos_interaction::constraint_marks::{Glyph, bound, glyphs};
use kentos_interaction::dimension_bonds::Bond;
use kentos_render_wgpu::Camera;
use kentos_ui::theme::{Tokens, typography};

use crate::app::App;
use crate::viewport::MarkColors;

/// The marks and, by constraint, the parts each names.
#[derive(Debug, Default)]
pub(crate) struct ConstraintMarks {
    pub glyphs: Vec<Glyph>,
    pub bounds: Vec<Vec<Bond>>,
}

/// The marks kept for the drawing's revision and generation.
type Kept = Option<((u64, u64), Arc<ConstraintMarks>)>;

/// Whether the marks show, and the marks of the drawing as it was last asked.
#[derive(Debug, Default)]
pub(crate) struct ConstraintView {
    hidden: bool,
    /// The pointer over the drawing, as the drawing area last said.
    pub(crate) pointer: Option<Point>,
    kept: RefCell<Kept>,
}

impl ConstraintView {
    /// Kısıtları göster: hides the marks, or shows them again.
    pub(crate) fn toggle(&mut self) {
        self.hidden = !self.hidden;
    }

    pub(crate) fn shown(&self) -> bool {
        !self.hidden
    }
}

impl App {
    /// The constraints' marks for the drawing as it is; none when hidden or none.
    pub(crate) fn constraint_marks(&self) -> Option<Arc<ConstraintMarks>> {
        let doc = self.document.as_ref()?;
        if !self.constraint_view.shown() || doc.model.constraints().constraints.is_empty() {
            return None;
        }
        let key = (doc.model.revision(), doc.model.generation());
        if let Some((k, marks)) = self.constraint_view.kept.borrow().as_ref()
            && *k == key
        {
            return Some(marks.clone());
        }
        let model = &doc.model;
        let marks = Arc::new(ConstraintMarks {
            glyphs: glyphs(model),
            bounds: (0..model.constraints().constraints.len())
                .map(|i| bound(model, i))
                .collect(),
        });
        *self.constraint_view.kept.borrow_mut() = Some((key, marks.clone()));
        Some(marks)
    }
}

/// A mark's box on the screen: by its part, after the marks before it at the same place.
fn boxes(marks: &ConstraintMarks, camera: &Camera, size: Size) -> Vec<(usize, Rectangle)> {
    let mut taken: Vec<((i64, i64), f32)> = Vec::new();
    let mut out = Vec::new();
    for (i, g) in marks.glyphs.iter().enumerate() {
        let [x, y] = camera.world_to_screen(g.at);
        let (x, y) = (x as f32, y as f32);
        if x < -40.0 || y < -40.0 || x > size.width + 40.0 || y > size.height + 40.0 {
            continue;
        }
        let key = ((x / 4.0).round() as i64, (y / 4.0).round() as i64);
        let width = 18.0
            + g.label
                .as_ref()
                .map_or(0.0, |l| 6.4 * l.chars().count() as f32 + 6.0);
        let shift = match taken.iter_mut().find(|(k, _)| *k == key) {
            Some((_, used)) => {
                let s = *used;
                *used += width + 2.0;
                s
            }
            None => {
                taken.push((key, width + 2.0));
                0.0
            }
        };
        out.push((
            i,
            Rectangle::new(
                Point::new((x + 8.0 + shift).round(), (y - 26.0).round()),
                Size::new(width, 18.0),
            ),
        ));
    }
    out
}

/// The marks over the drawing, and the parts of the one under the pointer.
pub(crate) fn draw(
    frame: &mut canvas::Frame,
    marks: &ConstraintMarks,
    camera: &Camera,
    colors: &MarkColors,
    tokens: &Tokens,
    pointer: Option<Point>,
) {
    let laid = boxes(marks, camera, frame.size());
    let over = pointer.and_then(|p| {
        laid.iter()
            .rev()
            .find(|(_, r)| r.contains(p))
            .map(|(i, _)| *i)
    });
    if let Some(i) = over {
        let g = &marks.glyphs[i];
        if let Some(parts) = marks.bounds.get(g.index) {
            crate::marks::draw_bonds(frame, parts, camera, colors, tokens.accent);
        }
    }
    for (i, r) in &laid {
        let g = &marks.glyphs[*i];
        let hot = over.is_some_and(|o| marks.glyphs[o].index == g.index);
        let edge = if g.unmet {
            tokens.danger
        } else if hot {
            tokens.accent
        } else {
            tokens.border
        };
        let face = Path::rounded_rectangle(r.position(), r.size(), 4.0.into());
        frame.fill(&face, tokens.surface);
        frame.stroke(
            &face,
            Stroke::default()
                .with_color(edge)
                .with_width(if hot { 1.5 } else { 1.0 }),
        );
        let ink = if g.reference {
            tokens.muted
        } else if g.unmet {
            tokens.danger
        } else {
            tokens.text
        };
        kentos_ui::icon::draw(
            frame,
            crate::icons::from_web(Some(g.icon)),
            ink,
            Rectangle::new(Point::new(r.x + 2.0, r.y + 2.0), Size::new(14.0, 14.0)),
        );
        if let Some(label) = &g.label {
            frame.fill_text(Text {
                content: label.clone(),
                position: Point::new(r.x + 20.0, r.y + 9.0),
                color: ink,
                size: Pixels(11.0),
                font: typography::ui(),
                align_y: iced::alignment::Vertical::Center,
                ..Text::default()
            });
        }
    }
    // The name of the one under the pointer, above its box.
    if let Some(i) = over
        && let Some((_, r)) = laid.iter().find(|(k, _)| *k == i)
    {
        let g = &marks.glyphs[i];
        let mut words = g.title.to_owned();
        if g.reference {
            words.push_str(" · başvuru");
        }
        if g.unmet {
            words.push_str(" · tutmuyor");
        }
        let width = 7.0 * words.chars().count() as f32 + 12.0;
        let at = Point::new(r.x, r.y - 22.0);
        let tip = Path::rounded_rectangle(at, Size::new(width, 18.0), 4.0.into());
        frame.fill(&tip, tokens.text);
        frame.fill_text(Text {
            content: words,
            position: Point::new(at.x + 6.0, at.y + 9.0),
            color: tokens.surface,
            size: Pixels(11.0),
            font: typography::ui_strong(),
            align_y: iced::alignment::Vertical::Center,
            ..Text::default()
        });
    }
}
