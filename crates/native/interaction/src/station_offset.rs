//! Dik ayak ölç (`tool.stationOffset`, docs/adr/0141): Netcad's Prizma. The
//! reference line is given by two points, its start A and its end B (snaps
//! apply); every point clicked after them is read against it:
//!
//! - **dik ayak** (abscissa): how far its foot is from A towards B, negative
//!   behind A;
//! - **dik boy** (ordinate): how far it is from the line, square to it, the
//!   right of A→B positive and the left negative.
//!
//! These are Nokta hesapla's yan nokta values, the shared core's `side_offsets`.
//! Beside the cursor: `Ayak 12.400 m` and `Boy +3.100 m` (the sign of the
//! boy always, `+` for the right, a true minus (U+2212) for the left, none
//! for zero; a negative ayak is the formatter's ordinary “-”); each click says
//! `Dik ayak 12.400 m, dik boy 3.100 m (sağda)` in the log. Nothing is written
//! to the drawing.
//!
//! Başka hat (H) picks a new line; Enter or a quick right click ends. Esc steps
//! back: from the points to A (a new line), from B to A, from A it leaves.

use kentos_geometry_core::geom::survey::{side_offsets, side_point};
use kentos_geometry_core::geometry::{bearing_grad, dist};
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, SAME};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Flow, Label, Marker, MarkerShape, Pointer, Preview, Stroke, Tag, Tone, Tool,
};

/// The tool's id: its command is `tool.stationOffset`.
pub const ID: &str = "stationOffset";
pub const LABEL: &str = "Dik ayak ölç";

/// How far the line's extension runs either way, in view diagonals.
const REACH: f64 = 2.0;
/// A boy this small (metres) is on the line.
const ON_THE_LINE: f64 = 0.0005;
/// The extension's and the perpendicular's dash and gap, logical pixels.
const DASH: [f32; 2] = [4.0, 4.0];
/// How many read points stay marked on the drawing: the latest ones.
const MARKED: usize = 60;

/// Dik ayak ölç.
#[derive(Clone, Debug, Default)]
pub struct StationOffset {
    a: Option<Vec2>,
    b: Option<Vec2>,
    /// The last point given, for typed relative points and snaps.
    last: Option<Vec2>,
    /// The points read against the line, ringed.
    read: Vec<Vec2>,
    /// The pointer's point as the next one would be taken.
    hover: Option<Vec2>,
    tracking: Option<Tracking>,
    reach: f64,
}

impl StationOffset {
    pub fn new() -> Self {
        Self::default()
    }

    /// The point at the pointer as the step takes it: the end B is
    /// constrained by ortho and polar tracking from A; the others are as
    /// snapped.
    fn point_at(&mut self, p: &Pointer, cx: &Context<'_>) -> Vec2 {
        match (self.a, self.b) {
            (Some(a), None) => {
                let (point, tracking) = points::constrain(Some(a), p, cx);
                self.tracking = tracking;
                point
            }
            _ => {
                self.tracking = None;
                p.world
            }
        }
    }

    /// A point given: A, then B, then points to read.
    fn take(&mut self, p: Vec2, cx: &mut Context<'_>) {
        match (self.a, self.b) {
            (None, _) => {
                points::echo(p, cx);
                self.a = Some(p);
            }
            (Some(a), None) => {
                if dist(a, p) <= SAME {
                    cx.say(
                        Level::Warn,
                        "B noktası A ile çakışıyor; hattın sonu için başka bir nokta gösterin.",
                    );
                    return;
                }
                points::echo(p, cx);
                self.b = Some(p);
            }
            (Some(a), Some(b)) => self.read(a, b, p, cx),
        }
        self.last = Some(p);
    }

    /// `p` against the line A→B, said in the log.
    fn read(&mut self, a: Vec2, b: Vec2, p: Vec2, cx: &mut Context<'_>) {
        let Some(o) = side_offsets(a, b, p) else {
            return;
        };
        let side = if o.ordinat.abs() < ON_THE_LINE {
            "hat üzerinde"
        } else if o.ordinat > 0.0 {
            "sağda"
        } else {
            "solda"
        };
        let f = cx.format();
        let text = format!(
            "Dik ayak {}, dik boy {} ({side})",
            f.length(foot_of(o.absis)),
            f.length(o.ordinat.abs())
        );
        cx.say(Level::Info, text);
        self.read.push(p);
        if self.read.len() > MARKED {
            self.read.remove(0);
        }
    }

    /// A new line: A is asked again.
    fn new_line(&mut self) {
        self.a = None;
        self.b = None;
        self.read.clear();
    }

    /// One step back; false when there is none (the tool is at A).
    fn step_back(&mut self) -> bool {
        match (self.a, self.b) {
            (None, _) => false,
            (Some(_), None) => {
                self.a = None;
                true
            }
            (Some(_), Some(_)) => {
                self.new_line();
                true
            }
        }
    }
}

/// An ayak as shown: never “-0.000”.
fn foot_of(absis: f64) -> f64 {
    if absis.abs() < ON_THE_LINE {
        0.0
    } else {
        absis
    }
}

/// A boy for the tag: the sign always, `+` for the right and a true minus
/// (U+2212) for the left, none for zero.
fn signed(format: &Format, boy: f64) -> String {
    if boy.abs() < ON_THE_LINE {
        format.length(0.0)
    } else if boy > 0.0 {
        format!("+{}", format.length(boy))
    } else {
        format!("\u{2212}{}", format.length(-boy))
    }
}

impl Tool for StationOffset {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        match (self.a, self.b) {
            (None, _) => Prompt::new(LABEL, "hattın başına tıklayın (A)"),
            (Some(_), None) => Prompt::new(LABEL, "hattın sonuna tıklayın (B)"),
            _ => Prompt::new(LABEL, "ölçülecek noktaya tıklayın")
                .option("Başka hat", "H")
                .option("Bitir", "Enter"),
        }
    }

    fn point_count(&self) -> usize {
        usize::from(self.a.is_some()) + usize::from(self.b.is_some())
    }

    /// Perpendicular and tangent snaps are taken from the last point given.
    fn snap_from(&self) -> Option<Vec2> {
        self.last
    }

    /// A point computed by the point calculator, as if clicked.
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.take(p, cx);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.reach = points::reach(cx.view, REACH);
        self.hover = Some(self.point_at(p, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.reach = points::reach(cx.view, REACH);
        let point = self.point_at(p, cx);
        self.take(point, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if upper_tr(js_trim(text)) == "H" && self.b.is_some() {
            self.new_line();
            return true;
        }
        match cx.typed_point(text, self.last, self.hover) {
            Some(p) => {
                self.take(p, cx);
                true
            }
            None => false,
        }
    }

    /// Enter or a quick right click ends.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// Esc steps back: to A from the points and from B; from A it leaves.
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        self.step_back()
    }

    /// Ctrl+Z steps back as Esc does; at A the drawing's undo is the user's.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.step_back()
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut preview = Preview::default();
        let (Some(a), hover) = (self.a, self.hover) else {
            return preview;
        };
        let label = |at: Vec2, text: &str| Label {
            at,
            text: text.to_owned(),
            offset: [6.0, -6.0],
            tone: Tone::Snap,
        };
        let ring = |at: Vec2, radius: f32, tone: Tone| Marker {
            at,
            shape: MarkerShape::Ring(radius),
            tone,
        };
        preview.markers.push(ring(a, 4.0, Tone::Snap));
        preview.labels.push(label(a, "A"));
        // B is still to give: the line as far as the cursor, its length and semt.
        let Some(b) = self.b else {
            if let Some(h) = hover.filter(|h| dist(a, *h) > SAME) {
                preview.strokes.push(Stroke::solid(vec![a, h], false));
                preview.tag = Some(Tag {
                    at: h,
                    lines: vec![
                        format.length(dist(a, h)),
                        format!("Semt {}", format.bearing(bearing_grad(a, h))),
                    ],
                });
                preview.tracking = self.tracking;
            }
            return preview;
        };
        let l = dist(a, b);
        let (ux, uy) = ((b.x - a.x) / l, (b.y - a.y) / l);
        let along = Vec2::new(ux, uy);
        let far = self.reach;
        // The line, and its extension across the view: the foot may lie beyond either end.
        preview.strokes.push(
            Stroke::dashed(
                vec![
                    Vec2::new(a.x - ux * far, a.y - uy * far),
                    Vec2::new(b.x + ux * far, b.y + uy * far),
                ],
                false,
                DASH,
            )
            .tone(Tone::Snap),
        );
        preview
            .strokes
            .push(Stroke::solid(vec![a, b], false).width(1.5));
        preview.markers.push(ring(b, 4.0, Tone::Snap));
        preview.labels.push(label(b, "B"));
        // One point against the line: its foot, the dashed perpendicular to it and the
        // square corner between them.
        let reading = |p: Vec2, tone: Tone, preview: &mut Preview| {
            let o = side_offsets(a, b, p)?;
            let foot = side_point(a, b, o.absis, 0.0)?;
            if o.ordinat.abs() >= ON_THE_LINE && dist(foot, p) > SAME {
                preview.strokes.push(
                    Stroke::dashed(vec![foot, p], false, DASH)
                        .width(1.5)
                        .tone(tone),
                );
                preview.markers.push(Marker {
                    at: foot,
                    shape: MarkerShape::RightAngle {
                        along: Vec2::new(foot.x + along.x, foot.y + along.y),
                        up: p,
                    },
                    tone,
                });
            }
            preview.markers.push(ring(foot, 3.0, tone));
            Some(o)
        };
        for &p in &self.read {
            reading(p, Tone::Snap, &mut preview);
            preview.markers.push(ring(p, 4.0, Tone::Snap));
        }
        let Some(p) = hover else {
            return preview;
        };
        if let Some(o) = reading(p, Tone::Accent, &mut preview) {
            preview.tag = Some(Tag {
                at: p,
                lines: vec![
                    format!("Ayak {}", format.length(foot_of(o.absis))),
                    format!("Boy {}", signed(format, o.ordinat)),
                ],
            });
        }
        preview
    }
}
