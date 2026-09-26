//! Eğri: the web's `SplineTool` (`apps/web/src/tools/curveTools.ts`) on its
//! `PointInputTool` base, step for step (docs/adr/0057):
//!
//! - the points the smooth curve passes through, clicked or typed; Geri (G)
//!   takes the last one back, as Ctrl+Z does;
//! - Enter (or a quick right click) ends an open curve of two points or
//!   more; Kapat (K) a closed one of three or more;
//! - the preview is the curve through the points and the cursor, over the
//!   dashed chain of its points.
//!
//! Every curve is written through `cad.entities.create`, its own object and
//! undo step (“Ekle”), and says “Eğri eklendi: 4 nokta, 31.416 m”. The
//! curve and its length are the shared core's (centripetal Catmull-Rom).

use kentos_contracts::EntityGeometry;
use kentos_geometry_core::entity::entity_length;
use kentos_geometry_core::geom::spline::catmull_rom;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::point_text::{js_trim, point_from_text};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, wire_all};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tool};

/// The spline tool's id: its command is `tool.spline`.
pub const ID: &str = "spline";
pub const LABEL: &str = "Eğri";

/// Points a span of the previewed curve (the web's `catmullRom` default).
const PER_SPAN: f64 = 16.0;

/// The spline tool.
#[derive(Clone, Debug, Default)]
pub struct Spline {
    d: Taken,
}

impl Spline {
    pub fn new() -> Self {
        Self::default()
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        if self.d.last().is_none_or(|last| dist(last, p) > 1e-9) {
            self.d.pts.push(p);
        }
    }

    /// Geri (G) takes the last point back; Kapat (K) ends a closed curve.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "G" if !self.d.pts.is_empty() => {
                self.d.pts.pop();
                true
            }
            "K" if self.d.pts.len() >= 3 => {
                self.commit(true, cx);
                true
            }
            _ => false,
        }
    }

    /// Writes through `cad.entities.create` (docs/adr/0057): its own object
    /// and undo step, “Ekle”, then its length from the core; the draft
    /// starts over either way.
    fn commit(&mut self, closed: bool, cx: &mut Context<'_>) {
        let n = self.d.pts.len();
        let geometry = EntityGeometry::Spline {
            pts: wire_all(&self.d.pts),
            closed,
        };
        if let Some(out) = points::write_objects(vec![geometry], None, cx)
            && let Some(&id) = out.ids.first()
        {
            self.d.note(id, cx);
            let length = cx
                .doc
                .get(kentos_domain::Slot(id))
                .and_then(|e| entity_length(&shape(e)));
            if let Some(length) = length {
                let what = if closed { "Kapalı eğri" } else { "Eğri" };
                let line = format!("{what} eklendi: {n} nokta, {}", cx.format().length(length));
                cx.say(Level::Success, line);
            }
        }
        self.d.reset();
    }
}

impl Tool for Spline {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        match self.d.pts.len() {
            0 => Prompt::new(LABEL, "ilk noktayı belirtin"),
            1 => Prompt::new(LABEL, "sonraki noktayı belirtin"),
            _ => Prompt::new(LABEL, "sonraki noktayı belirtin")
                .option("Kapat", "K")
                .option("Geri", "G")
                .option("Bitir", "Enter"),
        }
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.d.hover = Some(self.d.constrain(p, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let p = self.d.constrain(p, cx);
        self.accept(p, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.option(&upper_tr(js_trim(text)), cx) {
            return true;
        }
        match point_from_text(text, self.d.last(), self.d.hover, |_| None) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }

    /// Enter: an open curve of two points or more is written; fewer start
    /// over; with none, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.d.pts.is_empty() {
            return Flow::Exit;
        }
        if self.d.pts.len() >= 2 {
            self.commit(false, cx);
        } else {
            self.d.reset();
        }
        Flow::Stay
    }

    /// Ctrl+Z (ADR 0018): Geri (G) first, the last point; then the curve
    /// just written, as an undo.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        if self.option("G", cx) {
            return true;
        }
        self.d.undo_last_made(cx)
    }

    fn preview(&self, _format: &Format) -> Preview {
        let chain: Vec<Vec2> = self.d.pts.iter().copied().chain(self.d.hover).collect();
        let mut strokes = Vec::new();
        if chain.len() >= 2 {
            strokes.push(Stroke::solid(catmull_rom(&chain, false, PER_SPAN), false));
        }
        strokes.push(Stroke::dashed(chain, false, [2.0, 4.0]));
        Preview {
            strokes,
            tracking: self.d.hover.and(self.d.tracking),
            ..Preview::default()
        }
    }
}
