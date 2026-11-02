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
type Kept = Option<((u64, u64, u64), Arc<ConstraintMarks>)>;

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
        let key = (doc.session, doc.model.revision(), doc.model.generation());
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

/// A mark laid out inside the drawable viewport; its full label remains in the cached glyph.
struct Placed<'a> {
    index: usize,
    rect: Rectangle,
    label: Option<std::borrow::Cow<'a, str>>,
}

/// Keep a shared-anchor bar together at a viewport edge and wrap it when necessary. The viewport size excludes
/// dock panels, so a label never disappears underneath one. Independent nearby bars use the nearest free row.
fn boxes<'a>(marks: &'a ConstraintMarks, camera: &Camera, size: Size) -> Vec<Placed<'a>> {
    use std::collections::BTreeMap;
    const GAP: f32 = 2.0;
    const PAD: f32 = 4.0;
    const HEIGHT: f32 = 18.0;
    let room = size.width - 2.0 * PAD;
    if room < 18.0 || size.height < HEIGHT + 2.0 * PAD {
        return Vec::new();
    }
    // The key keeps the existing four-pixel shared-anchor grouping. Group insertion order preserves set order.
    let mut keys = BTreeMap::new();
    let mut groups: Vec<(Point, Vec<Placed<'a>>)> = Vec::new();
    for (index, g) in marks.glyphs.iter().enumerate() {
        let [x, y] = camera.world_to_screen(g.at);
        let (x, y) = (x as f32, y as f32);
        if !x.is_finite()
            || !y.is_finite()
            || x < -40.0
            || y < -40.0
            || x > size.width + 40.0
            || y > size.height + 40.0
        {
            continue;
        }
        let key = ((x / 4.0).round() as i64, (y / 4.0).round() as i64);
        let group = *keys.entry(key).or_insert_with(|| {
            groups.push((Point::new(x + 8.0, y - 26.0), Vec::new()));
            groups.len() - 1
        });
        let label = g
            .label
            .as_ref()
            .filter(|_| room >= 38.0)
            .map(|l| typography::elide(l, 11.0, room - 26.0));
        let width = label
            .as_ref()
            .map_or(18.0, |l| 26.0 + typography::measured_width(l, 11.0, false))
            .min(room);
        groups[group].1.push(Placed {
            index,
            rect: Rectangle::new(Point::ORIGIN, Size::new(width, HEIGHT)),
            label,
        });
    }
    let mut out = Vec::new();
    // Index the occupied vertical strips: a long label and a nearby circle/line bar must not paint over each
    // other. The same final rectangles are used by hover below. Do not compare every glyph in every frame.
    let mut occupied: BTreeMap<i32, Vec<Rectangle>> = BTreeMap::new();
    let strip = HEIGHT + GAP;
    for (anchor, mut bar) in groups {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut width: f32 = 0.0;
        for mark in &mut bar {
            if x > 0.0 && x + mark.rect.width > room {
                x = 0.0;
                y += HEIGHT + GAP;
            }
            mark.rect.x = x;
            mark.rect.y = y;
            width = width.max(x + mark.rect.width);
            x += mark.rect.width + GAP;
        }
        let height = y + HEIGHT;
        let left = anchor.x.clamp(PAD, (size.width - PAD - width).max(PAD));
        let max_top = (size.height - PAD - height).max(PAD);
        let preferred = anchor.y.clamp(PAD, max_top).floor();
        let candidate = |top: f32, mark: &Placed<'_>| {
            Rectangle::new(
                Point::new((mark.rect.x + left).floor(), (mark.rect.y + top).floor()),
                mark.rect.size(),
            )
        };
        let free = |top: f32| {
            bar.iter().all(|mark| {
                let r = candidate(top, mark);
                r.y + HEIGHT <= size.height - PAD
                    && occupied
                        .range(
                            (r.y / strip).floor() as i32..=((r.y + HEIGHT) / strip).floor() as i32,
                        )
                        .all(|(_, row)| row.iter().all(|other| !r.intersects(other)))
            })
        };
        // Stable set order and closest vertical displacement keep bars near their source. At a truly saturated
        // viewport the complete set remains in Parametreler; overlapping/unhoverable marks are not emitted.
        let Some(top) = (0..=(size.height / strip).ceil() as usize)
            .flat_map(|row| {
                let dy = row as f32 * strip;
                [preferred - dy, preferred + dy]
            })
            .filter(|top| *top >= PAD && *top <= max_top)
            .find(|top| free(*top))
        else {
            continue;
        };
        for mut mark in bar {
            mark.rect.x = (mark.rect.x + left).floor();
            mark.rect.y = (mark.rect.y + top).floor();
            // A viewport cannot contain an arbitrary number of rows. The parameter window always retains the set.
            if mark.rect.y + HEIGHT <= size.height - PAD {
                for row in (mark.rect.y / strip).floor() as i32
                    ..=((mark.rect.y + HEIGHT) / strip).floor() as i32
                {
                    occupied.entry(row).or_default().push(mark.rect);
                }
                out.push(mark);
            }
        }
    }
    out
}

fn tip_rect(mark: Rectangle, width: f32, size: Size) -> Rectangle {
    let width = width.min((size.width - 8.0).max(0.0));
    Rectangle::new(
        Point::new(
            mark.x.clamp(4.0, (size.width - width - 4.0).max(4.0)),
            (if mark.y >= 26.0 {
                mark.y - 22.0
            } else {
                mark.y + mark.height + 4.0
            })
            .clamp(4.0, (size.height - 22.0).max(4.0)),
        ),
        Size::new(width, 18.0),
    )
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
            .find(|m| m.rect.contains(p))
            .map(|m| m.index)
    });
    if let Some(i) = over {
        let g = &marks.glyphs[i];
        if let Some(parts) = marks.bounds.get(g.index) {
            crate::marks::draw_bonds(frame, parts, camera, colors, tokens.accent);
        }
    }
    for placed in &laid {
        let r = &placed.rect;
        let g = &marks.glyphs[placed.index];
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
        if let Some(label) = &placed.label {
            frame.fill_text(Text {
                content: label.to_string(),
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
        && let Some(placed) = laid.iter().find(|m| m.index == i)
    {
        let g = &marks.glyphs[i];
        let mut words = g.title.to_owned();
        if g.reference {
            words.push_str(" · başvuru");
        }
        if g.unmet {
            words.push_str(" · tutmuyor");
        }
        if placed.label.as_deref() != g.label.as_deref()
            && let Some(label) = &g.label
        {
            words.push_str(" · ");
            words.push_str(label);
        }
        let words = typography::elide(&words, 11.0, (frame.width() - 20.0).max(0.0));
        let width = typography::measured_width(&words, 11.0, true) + 12.0;
        let r = tip_rect(placed.rect, width, frame.size());
        let at = r.position();
        let tip = Path::rounded_rectangle(at, r.size(), 4.0.into());
        frame.fill(&tip, tokens.text);
        frame.fill_text(Text {
            content: words.into_owned(),
            position: Point::new(at.x + 6.0, at.y + 9.0),
            color: tokens.surface,
            size: Pixels(11.0),
            font: typography::ui_strong(),
            align_y: iced::alignment::Vertical::Center,
            ..Text::default()
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::ConstraintId;
    use kentos_interaction::Vec2;

    fn marks(at: Vec2, labels: &[Option<&str>]) -> ConstraintMarks {
        ConstraintMarks {
            glyphs: labels
                .iter()
                .enumerate()
                .map(|(i, label)| Glyph {
                    id: ConstraintId([i as u8; 16]),
                    index: i,
                    icon: "constraintDistance",
                    title: "Uzaklık",
                    at,
                    label: label.map(str::to_owned),
                    unmet: false,
                    reference: false,
                })
                .collect(),
            bounds: Vec::new(),
        }
    }
    fn camera(w: f64, h: f64) -> Camera {
        Camera {
            width: w,
            height: h,
            ..Default::default()
        }
    }
    fn inside(r: Rectangle, size: Size) {
        assert!(r.x >= 4.0 && r.y >= 4.0, "{r:?}");
        assert!(
            r.x + r.width <= size.width - 4.0 && r.y + r.height <= size.height - 4.0,
            "{r:?}"
        );
    }
    #[test]
    fn opening_another_drawing_at_the_same_revision_never_reuses_its_marks() {
        use crate::app::Message;
        let (mut app, _) = App::boot(None);
        crate::constraint_scenes::drawn(&mut app);
        crate::constraint_scenes::rectangle(&mut app);
        let first = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let open = |app: &mut App, snapshot| {
            let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
            let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        };
        open(&mut app, first.clone());
        let old = app.constraint_marks().unwrap();
        let stamp = app
            .document
            .as_ref()
            .map(|d| (d.model.revision(), d.model.generation()))
            .unwrap();
        assert!(Arc::ptr_eq(&old, &app.constraint_marks().unwrap()));
        let mut next = first;
        next.constraints.parameters[0].name = "yeni_boy".into();
        for c in &mut next.constraints.constraints {
            if let kentos_commands::ConstraintRule::Distance { parameter, .. } = &mut c.rule {
                *parameter = "yeni_boy".into();
            }
        }
        // Both drawings open at revision/generation zero; their distinct session is the cache boundary.
        open(&mut app, next);
        let new = app.constraint_marks().unwrap();
        assert_eq!(
            app.document
                .as_ref()
                .map(|d| (d.model.revision(), d.model.generation()))
                .unwrap(),
            stamp
        );
        assert!(!Arc::ptr_eq(&old, &new));
        assert!(new.glyphs.iter().any(|g| {
            g.label
                .as_ref()
                .is_some_and(|l| l.starts_with("yeni_boy = "))
        }));
        assert!(!old.glyphs.iter().any(|g| {
            g.label
                .as_ref()
                .is_some_and(|l| l.starts_with("yeni_boy = "))
        }));
    }
    #[test]
    fn shared_anchor_marks_fit_all_four_viewport_edges_without_overlapping_each_other() {
        let _font = crate::appearance::tests::TYPOGRAPHY.lock();
        let size = Size::new(320.0, 220.0);
        let c = camera(320.0, 220.0);
        for at in [[1., 1.], [319., 1.], [1., 219.], [319., 219.]] {
            let world = c.screen_to_world(at[0], at[1]);
            let m = marks(world, &[None, Some("kiriş = 4.472135955 m"), None]);
            let placed = boxes(&m, &c, size);
            assert_eq!(placed.len(), 3);
            for (i, p) in placed.iter().enumerate() {
                inside(p.rect, size);
                assert!(
                    placed
                        .iter()
                        .skip(i + 1)
                        .all(|other| !p.rect.intersects(&other.rect))
                );
                assert!(
                    p.rect.contains(p.rect.center()),
                    "the same rectangle is the hover target"
                );
                inside(tip_rect(p.rect, 180.0, size), size);
            }
        }
    }
    #[test]
    fn long_shared_anchor_bars_wrap_and_long_names_elide_without_changing_the_full_label() {
        let _font = crate::appearance::tests::TYPOGRAPHY.lock();
        let size = Size::new(180.0, 160.0);
        let c = camera(180.0, 160.0);
        let long = "çok_uzun_parametre_adı_ile_kısıt_değeri = 0.000000002 mm";
        let m = marks(
            c.screen_to_world(179.0, 1.0),
            &[Some(long), None, Some("boy = 12 cm"), None],
        );
        let placed = boxes(&m, &c, size);
        assert_eq!(placed.len(), 4);
        assert!(placed[1].rect.y > placed[0].rect.y);
        assert!(placed[0].label.as_deref().unwrap().ends_with('…'));
        assert_eq!(m.glyphs[0].label.as_deref(), Some(long));
        for (i, p) in placed.iter().enumerate() {
            inside(p.rect, size);
            assert!(
                placed
                    .iter()
                    .skip(i + 1)
                    .all(|o| !p.rect.intersects(&o.rect))
            );
        }
        assert!(boxes(&m, &c, Size::new(10., 10.)).is_empty());
    }
    #[test]
    fn technical_plate_nearby_anchors_keep_every_mark_readable_and_hoverable() {
        let _font = crate::appearance::tests::TYPOGRAPHY.lock();
        let snapshot = kentos_kcad::decode(include_bytes!(
            "../../../fixtures/interaction/v1/constraint-technical-plate.kcad"
        ))
        .unwrap();
        let model = kentos_domain::Document::from_snapshot_v2(snapshot).unwrap();
        let marks = ConstraintMarks {
            glyphs: glyphs(&model),
            bounds: Vec::new(),
        };
        // Match the actual acceptance window, then pan/zoom and a smaller drawable area.
        for (size, scale) in [
            (Size::new(1128., 592.), 37.6),
            (Size::new(800., 500.), 26.),
            (Size::new(480., 400.), 16.),
        ] {
            let mut c = camera(size.width as f64, size.height as f64);
            c.center = kentos_interaction::Vec2::new(487013., 4420004.);
            c.scale = scale;
            let laid = boxes(&marks, &c, size);
            assert_eq!(
                laid.len(),
                marks.glyphs.len(),
                "all plate marks must remain available"
            );
            for (i, p) in laid.iter().enumerate() {
                inside(p.rect, size);
                assert!(
                    laid.iter().skip(i + 1).all(|o| !p.rect.intersects(&o.rect)),
                    "independent anchors overlap: {:?}",
                    p.rect
                );
                assert_eq!(
                    laid.iter()
                        .filter(|m| m.rect.contains(p.rect.center()))
                        .count(),
                    1,
                    "hover names exactly one mark"
                );
            }
            let again = boxes(&marks, &c, size);
            assert!(
                laid.iter()
                    .zip(&again)
                    .all(|(a, b)| a.index == b.index && a.rect == b.rect),
                "stationary camera must be stable"
            );
        }
    }
}
