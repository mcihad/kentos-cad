//! Çitle seç (`tool.selectFence`, docs/adr/0141): a fence is drawn and every
//! visible object it crosses is selected.
//!
//! - Click the fence's points (snaps, ortho and polar tracking apply from the
//!   last); Geri (G) drops the last one. A right click or Enter ends.
//! - The fence crosses an object when it meets an edge, a text's body, or
//!   passes within the pick tolerance of a point (the store's `in_fence`). It
//!   needs two points at least: with one, Enter says so and the tool waits.
//! - What it crosses replaces the selection, or joins it when Shift is held
//!   (at the last pointer event, or as Enter is pressed: a right click with
//!   Shift opens the snap menu instead), and the tool goes back to Seç: `Çit 3
//!   nesneyi kesti; seçildi.` When it crosses nothing the selection is left as
//!   it was, and the tool leaves all the same: `Çit hiçbir nesneyi kesmedi.`
//! - Esc drops the fence drawn so far; with none it leaves.
//!
//! Only visible objects are selected; objects on a locked layer are, as in a
//! window selection.

use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, SAME};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Marker, MarkerShape, Pointer, Preview, Stroke, Tone, Tool};

/// The tool's id: its command is `tool.selectFence`.
pub const ID: &str = "selectFence";
pub const LABEL: &str = "Çitle seç";

/// The fence's dash and gap, logical pixels.
const DASH: [f32; 2] = [7.0, 4.0];

/// Çitle seç.
#[derive(Clone, Debug, Default)]
pub struct SelectFence {
    pts: Vec<Vec2>,
    /// The next point as the pointer would give it (ortho and polar tracking applied).
    hover: Option<Vec2>,
    tracking: Option<Tracking>,
    /// Shift was held at the last pointer event: ending the fence then adds to the selection.
    shift: bool,
}

impl SelectFence {
    pub fn new() -> Self {
        Self::default()
    }

    fn last(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    /// A point of the fence, unless it is on the last one.
    fn add(&mut self, p: Vec2) {
        if self.last().is_none_or(|q| dist(q, p) > SAME) {
            self.pts.push(p);
        }
    }

    /// The fence is done: what it crosses is selected, and the tool leaves.
    fn finish(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.pts.is_empty() {
            return Flow::Exit;
        }
        if self.pts.len() < 2 {
            cx.say(
                Level::Warn,
                "Çit için en az iki nokta gerekir; ikinci noktayı gösterin.",
            );
            return Flow::Stay;
        }
        let fence = std::mem::take(&mut self.pts);
        let hits = cx.spatial.in_fence(&fence, cx.pick_tolerance());
        let hits = crate::selectable::ids(cx, hits);
        let n = hits.len();
        if n == 0 {
            cx.say(Level::Warn, "Çit hiçbir nesneyi kesmedi.");
        } else {
            cx.selection.take(hits, cx.shift || self.shift);
            cx.say(Level::Info, format!("Çit {n} nesneyi kesti; seçildi."));
        }
        Flow::Exit
    }
}

impl Tool for SelectFence {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.pts.is_empty() {
            return Prompt::new(LABEL, "çitin ilk noktasına tıklayın");
        }
        Prompt::new(LABEL, "sonraki noktaya tıklayın")
            .option("Geri", "G")
            .option("Bitir", "Enter")
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    /// Perpendicular and tangent snaps are taken from the last point of the fence.
    fn snap_from(&self) -> Option<Vec2> {
        self.last()
    }

    /// A point computed by the point calculator, as if clicked.
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, _cx: &mut Context<'_>) -> bool {
        self.add(p);
        true
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.shift = p.shift;
        let (point, tracking) = points::constrain(self.last(), p, cx);
        self.hover = Some(point);
        self.tracking = tracking;
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.shift = p.shift;
        let (point, tracking) = points::constrain(self.last(), p, cx);
        self.tracking = tracking;
        self.add(point);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if upper_tr(js_trim(text)) == "G" {
            return self.pts.pop().is_some();
        }
        match cx.typed_point(text, self.last(), self.hover) {
            Some(p) => {
                self.add(p);
                true
            }
            None => false,
        }
    }

    /// Enter or a quick right click: the fence is done.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.finish(cx)
    }

    /// Esc drops the fence drawn so far; with none it leaves.
    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.pts.is_empty() {
            return false;
        }
        self.pts.clear();
        true
    }

    /// Ctrl+Z takes the last point back.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.pts.pop().is_some()
    }

    /// The fence as a dashed path to the cursor.
    fn preview(&self, _format: &Format) -> Preview {
        let mut line = self.pts.clone();
        line.extend(self.hover.filter(|_| !self.pts.is_empty()));
        let strokes = if line.len() >= 2 {
            vec![Stroke::dashed(line, false, DASH).width(1.5)]
        } else {
            Vec::new()
        };
        Preview {
            strokes,
            markers: self
                .pts
                .iter()
                .map(|&at| Marker {
                    at,
                    shape: MarkerShape::Ring(3.0),
                    tone: Tone::Accent,
                })
                .collect(),
            tracking: self.tracking.filter(|_| !self.pts.is_empty()),
            ..Preview::default()
        }
    }
}
