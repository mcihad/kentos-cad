//! Açı ölç (docs/adr/0140): the angle at a vertex between two arms, on the
//! `PointInputTool` base: the vertex, the first arm's point, the second
//! arm's point, each clicked (snapping) or typed.
//!
//! The angle is drawn live as an arc between the arms with its value beside
//! it, and the tag beside the cursor says the angle and its explement in the
//! project's angle unit (grad unless the project says degrees). The third
//! point sends the result to the message log and the tool asks for the next
//! vertex; nothing is written to the drawing. Esc steps back a point; Enter
//! or a quick right click starts over, or leaves when none is given.
//!
//! The smaller of the two angles is “the angle”, the larger its explement
//! (dış açı). The values are the shared core's (`construct::angle_at`).

use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::jsmath::{PI, TAU, atan2, cos, sin};
use kentos_geometry_core::tools::construct::angle_at;
use kentos_geometry_core::tools::point_text::point_from_text;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, constrain};
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Label, Pointer, Preview, Stroke, Tag, Tone, Tool};

/// The tool's id: its command is `tool.measureAngle`.
pub const ID: &str = "measureAngle";
pub const LABEL: &str = "Açı ölç";

/// How wide the arc is drawn, logical pixels, and how finely, radians.
const ARC_PX: f64 = 46.0;
const STEP: f64 = 0.05;

/// Açı ölç.
#[derive(Clone, Debug, Default)]
pub struct MeasureAngle {
    d: Taken,
    /// The project's units and the arc's radius in metres, as of the last event.
    format: Format,
    arc: f64,
}

impl MeasureAngle {
    pub fn new() -> Self {
        Self {
            arc: 1.0,
            ..Self::default()
        }
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.format = cx.format();
        self.arc = cx.view.world_length(ARC_PX);
    }

    /// A point given, clicked or computed.
    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.see(cx);
        if let Some(&vertex) = self.d.pts.first()
            && dist(vertex, p) < points::SAME
        {
            cx.say(
                Level::Warn,
                "Kol noktası tepe noktasıyla çakışıyor; tepeden farklı bir yer gösterin.",
            );
            return;
        }
        self.d.begin(p, cx);
        self.d.pts.push(p);
        if let [v, p1, p2] = self.d.pts[..] {
            match angle_at(v, p1, p2) {
                Some(a) => {
                    let f = self.format;
                    cx.say(
                        Level::Success,
                        format!("Açı: {}, dış açı: {}", f.angle(a.inner), f.angle(a.outer)),
                    );
                }
                None => cx.say(
                    Level::Warn,
                    "Kollardan biri sıfır uzunlukta; açı ölçülemedi.",
                ),
            }
            self.d.pts.clear();
        }
    }

    /// The arc between the arms with the angle by it, and the lines of the arms.
    fn drawn(&self, hover: Vec2) -> Preview {
        let f = self.format;
        let Some(&v) = self.d.pts.first() else {
            return Preview::default();
        };
        let mut strokes = vec![Stroke::solid(vec![v, hover], false)];
        let Some(&p1) = self.d.pts.get(1) else {
            return Preview {
                strokes,
                ..Preview::default()
            };
        };
        // The first arm is given, the second follows the cursor.
        strokes = vec![
            Stroke::solid(vec![v, p1], false),
            Stroke::solid(vec![v, hover], false),
        ];
        let Some(a) = angle_at(v, p1, hover) else {
            return Preview {
                strokes,
                ..Preview::default()
            };
        };
        // The arc of the smaller angle, from the arm it starts at, counter-clockwise.
        let (a1, a2) = (
            atan2(p1.y - v.y, p1.x - v.x),
            atan2(hover.y - v.y, hover.x - v.x),
        );
        let (from, span) = if a.sweep <= PI {
            (a1, a.sweep)
        } else {
            (a2, TAU - a.sweep)
        };
        let radius = self
            .arc
            .min(0.6 * dist(v, p1).min(dist(v, hover)))
            .max(1e-9);
        let steps = ((span / STEP).ceil() as usize).max(2);
        let at = |t: f64, r: f64| Vec2::new(v.x + r * cos(t), v.y + r * sin(t));
        let arc: Vec<Vec2> = (0..=steps)
            .map(|k| at(from + span * k as f64 / steps as f64, radius))
            .collect();
        // The arc and its number stand out from the arms (often the parcel's own edges): the
        // snap colour, and the number goes to the side of the wedge that leaves it room.
        strokes.push(Stroke::solid(arc, false).width(2.0).tone(Tone::Snap));
        let middle = from + span / 2.0;
        let text = f.angle(a.inner);
        let width = 7.0 * text.chars().count() as f32 + 6.0;
        let (side, up) = (cos(middle), sin(middle));
        let offset = [
            if side < -0.2 {
                -width
            } else if side > 0.2 {
                6.0
            } else {
                -width / 2.0
            },
            if up > 0.3 {
                -18.0
            } else if up < -0.3 {
                4.0
            } else {
                -8.0
            },
        ];
        Preview {
            strokes,
            labels: vec![Label {
                at: at(middle, radius * 1.2),
                text,
                offset,
                tone: Tone::Snap,
            }],
            tag: Some(Tag {
                at: hover,
                lines: vec![
                    format!("Açı {}", f.angle(a.inner)),
                    format!("Dış açı {}", f.angle(a.outer)),
                ],
            }),
            ..Preview::default()
        }
    }
}

impl Tool for MeasureAngle {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let step = match self.d.pts.len() {
            0 => "açının tepe noktasını belirtin",
            1 => "birinci kolun bir noktasını belirtin",
            _ => "ikinci kolun bir noktasını belirtin",
        };
        Prompt::new(LABEL, step)
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.pts.first().copied()
    }

    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        true
    }

    /// Ortho and polar tracking run from the vertex: both arms are directions from it.
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        let (point, tracking) = constrain(self.d.pts.first().copied(), p, cx);
        self.d.hover = Some(point);
        self.d.tracking = tracking;
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let (point, _) = constrain(self.d.pts.first().copied(), p, cx);
        self.accept(point, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let Some(p) = point_from_text(text, self.d.pts.first().copied(), self.d.hover, |d| {
            cx.track_along(d)
        }) else {
            return false;
        };
        self.accept(p, cx);
        true
    }

    /// With points given it starts over; with none, the tool leaves.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        self.d.pts.clear();
        Flow::Stay
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.d.pts.pop().is_some()
    }

    /// It writes nothing, so Ctrl+Z takes a point back, else undoes the drawing.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.d.pts.pop().is_some()
    }

    fn preview(&self, _format: &Format) -> Preview {
        let Some(hover) = self.d.hover else {
            return Preview::default();
        };
        Preview {
            tracking: self.d.tracking,
            ..self.drawn(hover)
        }
    }
}
