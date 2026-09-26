//! Çizgi: the web's `LineTool` (`apps/web/src/tools/drawTools.ts`) on its
//! `PointInputTool` base, step for step (docs/adr/0027):
//!
//! - points come from clicks (ortho and polar tracking applied) and from
//!   typed text (the shared grammar, `point_text`); a second point on the
//!   same place adds nothing;
//! - every point after the first writes one line from the last point, its
//!   own object and its own undo step, through the product command
//!   `cad.line.create`; a refused line (a locked layer) adds no point;
//! - Geri (G) takes the chain's last line back: as an undo when the drawing
//!   has not changed since it was written, so a later Ctrl+Z cannot bring it
//!   back, else by removing it; Kapat (K), with three points or more, draws
//!   a line back to the first point and ends the chain;
//! - a confirm ends the chain (saying how many lines it wrote) and the tool
//!   stays for the next one; with no point it leaves;
//! - Ctrl+Z takes back the newest step (ADR 0018): the chain's last line,
//!   else the draft's first point, else the drawing's undo.
//!
//! The web's messages are kept word for word. Every calculation is the
//! shared core's (`kentos-geometry-core`); none is written here.

use kentos_contracts::LineCreate;
use kentos_domain::Slot;
use kentos_geometry_core::geometry::{bearing_grad, dist};
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_geometry_core::tools::point_text::{js_trim, point_from_text};
use kentos_native_application::{ExecutionContext, line};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, SAME, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Pointer, Preview, Tag, Tool};

/// The line tool's id: its command is `tool.line`.
pub const ID: &str = "line";
pub const LABEL: &str = "Çizgi";
/// Points before Kapat (K) is offered: a closed chain needs a triangle.
const CLOSE_MIN: usize = 3;

/// The line tool: a chain of lines.
#[derive(Clone, Debug, Default)]
pub struct Line {
    pts: Vec<Vec2>,
    /// The effective cursor from the last pointer move.
    hover: Option<Vec2>,
    tracking: Option<Tracking>,
    /// Lines of this chain, newest last, so G can take the last one back
    /// (AutoCAD LINE → Undo).
    created: Vec<Slot>,
    /// Objects written for the chain being drawn, newest last (the web's `made`).
    made: Vec<Slot>,
    /// The drawing's revision right after the newest of them was written or taken back.
    made_at: Option<u64>,
}

impl Line {
    pub fn new() -> Self {
        Self::default()
    }

    fn last(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        points::echo(p, cx);
        // A first point starts a new chain: what was written before is the drawing's to undo.
        if self.pts.is_empty() {
            self.made.clear();
        }
        self.on_point(p, cx);
    }

    fn on_point(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let last = self.last();
        if last.is_some_and(|last| dist(last, p) <= SAME) {
            return;
        }
        if let Some(last) = last {
            let Some(slot) = self.create(last, p, cx) else {
                return;
            };
            self.created.push(slot);
        }
        self.pts.push(p);
    }

    /// Writes one segment through the product command `cad.line.create`
    /// (docs/adr/0027): its own object and undo step. What the web's tool
    /// knows implicitly is explicit in its input (CMD-07): the active layer;
    /// the desktop has no current colour, so the layer's colour applies. The
    /// new line's slot, or `None` with the command's message when it refused.
    fn create(&mut self, a: Vec2, b: Vec2, cx: &mut Context<'_>) -> Option<Slot> {
        let input = LineCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            a: wire(a),
            b: wire(b),
            color: None,
            attrs: None,
            expected_revision: None,
        };
        let result = line::execute(&mut ExecutionContext::new(cx.doc), input);
        let slot = Slot(points::written(result, cx)?.id);
        self.made.push(slot);
        self.made_at = Some(cx.doc.revision());
        Some(slot)
    }

    /// Takes back the newest object this chain wrote, as an undo, when the
    /// drawing has not changed since: a later Ctrl+Z on the drawing then
    /// cannot bring it back. False otherwise (the web's `undoLastMade`).
    fn undo_last_made(&mut self, cx: &mut Context<'_>) -> bool {
        if self.made.is_empty() || self.made_at != Some(cx.doc.revision()) {
            return false;
        }
        self.made.pop();
        cx.doc.undo();
        self.made_at = Some(cx.doc.revision());
        true
    }

    /// Option letters: G (Geri) while the chain has a line, K (Kapat) with
    /// three points or more. False when the key is neither now.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        if key == "G"
            && let Some(slot) = self.created.pop()
        {
            // Taken back as an undo when nothing changed since, so a later Ctrl+Z cannot bring the line back.
            if !self.undo_last_made(cx) {
                cx.doc.remove(&[slot]);
            }
            self.pts.pop();
            return true;
        }
        if key != "K" || self.pts.len() < CLOSE_MIN {
            return false;
        }
        self.on_point(self.pts[0], cx);
        self.finish(cx);
        true
    }

    /// Ends the chain, saying how many lines it wrote, and starts over.
    fn finish(&mut self, cx: &mut Context<'_>) {
        if !self.created.is_empty() {
            cx.say(
                Level::Success,
                format!("{} çizgi eklendi.", self.created.len()),
            );
        }
        self.created.clear();
        self.reset();
    }

    fn reset(&mut self) {
        self.pts.clear();
        self.made.clear();
    }
}

impl Tool for Line {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let n = self.pts.len();
        if n == 0 {
            return Prompt::new(LABEL, "ilk noktayı belirtin");
        }
        let mut prompt = Prompt::new(LABEL, "sonraki noktayı belirtin");
        if !self.created.is_empty() {
            prompt = prompt.option("Geri", "G");
        }
        if n >= CLOSE_MIN {
            prompt = prompt.option("Kapat", "K");
        }
        prompt.option("Bitir", "Enter")
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let (point, tracking) = points::constrain(self.last(), p, cx);
        self.tracking = tracking;
        self.hover = Some(point);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let (point, tracking) = points::constrain(self.last(), p, cx);
        self.tracking = tracking;
        self.accept(point, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.option(&upper_tr(js_trim(text)), cx) {
            return true;
        }
        // Object tracking has no line on the desktop yet: a bare number follows the cursor.
        match point_from_text(text, self.last(), self.hover, |_| None) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.pts.is_empty() {
            return Flow::Exit;
        }
        self.finish(cx);
        Flow::Stay
    }

    /// Ctrl+Z while the tool runs (ADR 0018): newest first, its own Geri (G),
    /// then a line written for the chain (as an undo), then the draft's point.
    /// The web's line tool does not step back a point at a time: with no line
    /// left, the chain starts over (`stepsFromPoints` is false).
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if !self.pts.is_empty() && self.option("G", cx) {
            return true;
        }
        if self.undo_last_made(cx) {
            return true;
        }
        if self.pts.is_empty() {
            return false;
        }
        self.reset();
        true
    }

    fn preview(&self, format: &Format) -> Preview {
        let path = self.pts.iter().copied().chain(self.hover).collect();
        let tag = match (self.last(), self.hover) {
            (Some(last), Some(hover)) => Some(Tag {
                at: hover,
                lines: vec![
                    format.length(dist(last, hover)),
                    format!("Semt {}", format.bearing(bearing_grad(last, hover))),
                ],
            }),
            _ => None,
        };
        Preview {
            path,
            ring: None,
            guides: Vec::new(),
            tracking: tag.as_ref().and(self.tracking),
            tag,
        }
    }
}
