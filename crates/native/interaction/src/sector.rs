//! Daire dilimi (docs/adr/0140): a closed pie slice, on the `PointInputTool`
//! base like the other drawing tools:
//!
//! - the centre, clicked or typed;
//! - the start: a point (the radius is its distance from the centre and the
//!   start angle its direction), or a typed radius and then the start angle,
//!   typed or pointed;
//! - the end: a direction pointed at, or an angle typed. The arc is swept
//!   counter-clockwise from the start to the end.
//!
//! Typed angles are in the project's angle unit (grad unless the project
//! says degrees) and count counter-clockwise from east, as the arc tool's
//! do. Esc steps back (the start, then the centre); Enter or a quick right
//! click starts the slice over, or leaves when none is begun.
//!
//! The slice is one closed area, its ring the centre, the arc's start and end
//! with the arc as the middle edge, written through `cad.polygon.create`
//! with its bulges: one undo step, “Ekle”. The ring is the shared core's
//! (`construct::sector`).

use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::jsmath::{TAU, atan2};
use kentos_geometry_core::tools::construct::sector;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, plain_number};
use crate::prompt::Prompt;
use crate::tool::{Area, Context, Flow, Pointer, Preview, Stroke, Tag, Tone, Tool};

/// The tool's id: its command is `tool.sector`.
pub const ID: &str = "sector";
pub const LABEL: &str = "Daire dilimi";

/// How finely the arc is drawn, radians.
const STEP: f64 = 0.05;

/// What the tool waits for next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Centre,
    Start,
    StartAngle,
    End,
}

/// Daire dilimi.
#[derive(Clone, Debug, Default)]
pub struct Sector {
    d: Taken,
    /// The radius, metres: from the start point, or typed.
    radius: Option<f64>,
    /// The start angle, radians counter-clockwise from east.
    start: Option<f64>,
    /// The project's units, as of the last event.
    format: Format,
}

impl Sector {
    pub fn new() -> Self {
        Self::default()
    }

    fn stage(&self) -> Stage {
        match (self.d.last(), self.radius, self.start) {
            (None, ..) => Stage::Centre,
            (Some(_), _, Some(_)) => Stage::End,
            (Some(_), Some(_), None) => Stage::StartAngle,
            (Some(_), None, None) => Stage::Start,
        }
    }

    fn centre(&self) -> Option<Vec2> {
        self.d.pts.first().copied()
    }

    /// A point given, clicked or computed (the web's `accept`).
    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.format = cx.format();
        let Some(c) = self.centre() else {
            self.d.begin(p, cx);
            self.d.pts.push(p);
            return;
        };
        if dist(c, p) < points::SAME {
            cx.say(
                Level::Warn,
                "Nokta merkezle çakışıyor; merkezden farklı bir yer gösterin.",
            );
            return;
        }
        self.d.begin(p, cx);
        let direction = atan2(p.y - c.y, p.x - c.x);
        match self.stage() {
            Stage::Start => {
                self.radius = Some(dist(c, p));
                self.start = Some(direction);
            }
            Stage::StartAngle => self.start = Some(direction),
            Stage::End => self.write(direction, cx),
            Stage::Centre => {}
        }
    }

    /// The slice from the start to `end`, written; a sweep of nothing or of a
    /// whole turn is said and the tool waits for another end.
    fn write(&mut self, end: f64, cx: &mut Context<'_>) {
        let (Some(c), Some(r), Some(a0)) = (self.centre(), self.radius, self.start) else {
            return;
        };
        let Some(ring) = sector(c, r, a0, end) else {
            cx.say(
                Level::Warn,
                "Bitiş doğrultusu başlangıçla aynı; dilim oluşmuyor. Başka bir doğrultu ya da açı verin.",
            );
            return;
        };
        if points::write_area(&mut self.d, &ring.pts, ring.bulges, cx).is_some() {
            let f = cx.format();
            let sweep = (end - a0).rem_euclid(TAU);
            cx.say(
                Level::Success,
                format!(
                    "Daire dilimi eklendi: r = {}, açı {}",
                    f.length(r),
                    f.angle(sweep)
                ),
            );
        }
        self.d.pts.clear();
        self.radius = None;
        self.start = None;
    }

    /// Back one stage; false when nothing was begun.
    fn back(&mut self) -> bool {
        if self.start.is_some() || self.radius.is_some() {
            self.start = None;
            self.radius = None;
            return true;
        }
        if self.d.pts.is_empty() {
            return false;
        }
        self.d.pts.clear();
        true
    }

    /// The slice with the cursor as its end, drawn where it would be written.
    fn slice(&self, hover: Vec2) -> Preview {
        let (Some(c), Some(r), Some(a0)) = (self.centre(), self.radius, self.start) else {
            return Preview::default();
        };
        let end = atan2(hover.y - c.y, hover.x - c.x);
        let f = self.format;
        let tag = |lines: Vec<String>| Tag { at: hover, lines };
        let at = |a: f64| Vec2::new(c.x + r * a.cos(), c.y + r * a.sin());
        let Some(ring) = sector(c, r, a0, end) else {
            return Preview {
                strokes: vec![Stroke::solid(vec![c, at(a0)], false)],
                tag: Some(tag(vec!["Süpürme yok".to_owned()])),
                tag_tone: Tone::Danger,
                ..Preview::default()
            };
        };
        let outline = bulge_path_outline(&ring.pts, ring.bulges.as_deref(), true, STEP);
        Preview {
            areas: vec![Area {
                rings: vec![outline],
                fill: 0.2,
                width: 1.5,
                dash: None,
                fill_tone: Tone::Accent,
            }],
            tag: Some(tag(vec![
                format!("Açı {}", f.angle((end - a0).rem_euclid(TAU))),
                format!("Yarıçap {}", f.length(r)),
            ])),
            ..Preview::default()
        }
    }

    /// The radius and the start being chosen: the circle it makes, dashed,
    /// and the line from the centre.
    fn reach(&self, hover: Vec2) -> Preview {
        let Some(c) = self.centre() else {
            return Preview::default();
        };
        let f = self.format;
        let direction = atan2(hover.y - c.y, hover.x - c.x);
        let (r, end) = match self.radius {
            Some(r) => (
                r,
                Vec2::new(c.x + r * direction.cos(), c.y + r * direction.sin()),
            ),
            None => (dist(c, hover), hover),
        };
        let circle = Outline::of(&Shape::Circle { c, r }, Some([5.0, 3.0]), 1.0, Tone::Snap);
        Preview {
            strokes: circle
                .strokes
                .into_iter()
                .chain([Stroke::solid(vec![c, end], false)])
                .collect(),
            tag: Some(Tag {
                at: hover,
                lines: vec![
                    format!("Yarıçap {}", f.length(r)),
                    format!("Açı {}", f.angle(direction.rem_euclid(TAU))),
                ],
            }),
            ..Preview::default()
        }
    }
}

impl Tool for Sector {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let unit = self.format.angle_unit_name();
        match self.stage() {
            Stage::Centre => Prompt::new(LABEL, "dilimin merkezini belirtin"),
            Stage::Start => Prompt::new(LABEL, "başlangıç noktasını gösterin ya da yarıçapı yazın"),
            Stage::StartAngle => Prompt::new(
                LABEL,
                format!("başlangıç açısını yazın ({unit}, doğudan) ya da yönünü gösterin"),
            ),
            Stage::End => Prompt::new(
                LABEL,
                format!("bitiş doğrultusunu gösterin ya da açısını yazın ({unit})"),
            ),
        }
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.format = cx.format();
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.format = cx.format();
        self.d.hover = Some(self.d.constrain(p, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let point = self.d.constrain(p, cx);
        self.accept(point, cx);
    }

    /// A bare number is the radius, then the start angle, then the end angle;
    /// anything else is a point, as everywhere.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.format = cx.format();
        if let Some(n) = plain_number(text).filter(|_| self.stage() != Stage::Centre) {
            match self.stage() {
                Stage::Start => {
                    if n <= 0.0 {
                        cx.say(Level::Warn, "Yarıçap sıfırdan büyük olmalı.");
                    } else {
                        // Typed in the project's unit (docs/adr/0165 §2).
                        self.radius = Some(self.format.to_metres(n));
                    }
                }
                Stage::StartAngle => self.start = Some(self.format.angle_from_typed(n)),
                Stage::End => {
                    let end = self.format.angle_from_typed(n);
                    self.write(end, cx);
                }
                Stage::Centre => {}
            }
            return true;
        }
        let Some(p) = cx.typed_point(js_trim(text), self.d.last(), self.d.hover) else {
            return false;
        };
        self.accept(p, cx);
        true
    }

    /// With a slice begun it starts over; with none, the tool leaves.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        self.d.pts.clear();
        self.radius = None;
        self.start = None;
        Flow::Stay
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.back()
    }

    /// Ctrl+Z: the slice just written goes first, then the stage.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_last_made(cx) || self.back()
    }

    fn preview(&self, _format: &Format) -> Preview {
        let Some(hover) = self.d.hover else {
            return Preview::default();
        };
        let mut preview = match self.stage() {
            Stage::Centre => return Preview::default(),
            Stage::Start | Stage::StartAngle => self.reach(hover),
            Stage::End => self.slice(hover),
        };
        preview.tracking = self.d.tracking;
        preview
    }
}
