//! Kenar düzenle (docs/adr/0288): the grip menu's Yay… and Kenar genişliği… on a path's edge, AutoCAD PEDIT's
//! segment edits and the multifunctional grip's Convert to Arc.
//!
//! - Yay…: the arc through a point clicked or typed (`through`), or of a radius typed (`radius`; its side the
//!   pointer's, right of the edge's travel positive); the preview is `cad.entities.vertex`'s plan for the pointer.
//! - Kenar genişliği…: the start width typed, then the end width (Enter: the same); a polyline's edge.
//!
//! Both write through `cad.entities.vertex` (`arc` with its point or radius, `width`), one undo step named by the
//! action; the command's refusal is said and the tool waits for another value; written, it ends. Nothing is
//! computed here but the side of the pointer.

use kentos_contracts::{CommandResult, EntitiesVertex};
use kentos_geometry_core::entity::entity_outline;
use kentos_native_application::{ExecutionContext, entities_vertex};

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, wire};
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tool};

pub const ID: &str = "segmentEdit";
pub const LABEL: &str = "Kenar düzenle";

/// What the tool asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    /// A point the arc passes through, or its radius.
    Arc,
    /// The start and end widths.
    Width,
}

/// Kenar düzenle on one edge.
#[derive(Clone, Debug)]
pub struct SegmentEdit {
    input: EntitiesVertex,
    ask: Ask,
    /// The edge's ends, as the object's travel goes.
    a: Vec2,
    b: Vec2,
    hover: Option<Vec2>,
    /// The start width typed, the end waited for.
    start: Option<f64>,
    /// The object as the plan would write it, for the pointer's arc.
    ghost: Vec<Vec2>,
    done: bool,
}

impl SegmentEdit {
    /// The tool on the edge from `a` to `b` of `input`'s object, `input` naming the edge (its action `arc` or `width`).
    pub fn new(input: EntitiesVertex, ask: Ask, a: Vec2, b: Vec2) -> Self {
        Self {
            input,
            ask,
            a,
            b,
            hover: None,
            start: None,
            ghost: Vec::new(),
            done: false,
        }
    }

    /// Which side of the edge's travel `p` is on: right (a positive bulge's side) is 1, left −1.
    fn side(&self, p: Vec2) -> f64 {
        let cross =
            (self.b.x - self.a.x) * (p.y - self.a.y) - (self.b.y - self.a.y) * (p.x - self.a.x);
        if cross > 0.0 { -1.0 } else { 1.0 }
    }

    fn with_through(&self, p: Vec2) -> EntitiesVertex {
        EntitiesVertex {
            through: Some(wire(p)),
            radius: None,
            ..self.input.clone()
        }
    }

    /// Writes `input` through `cad.entities.vertex`; written, the tool ends.
    fn write(&mut self, input: EntitiesVertex, cx: &mut Context<'_>) {
        let step = entities_vertex::label(input.action);
        let mut ecx = ExecutionContext::with_store(cx.doc, cx.spatial.store());
        let result = entities_vertex::execute(&mut ecx, input);
        if let Some(output) = points::written(result, cx) {
            if output.changed.is_empty() {
                cx.say(
                    Level::Info,
                    format!("{step}: kenar zaten böyle; değişmedi."),
                );
            } else {
                cx.say(Level::Success, format!("{step}: tamam."));
            }
            self.done = true;
        }
    }

    /// The object as the plan writes it for an arc through `p`: its outline (none when refused).
    fn plan_ghost(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let input = self.with_through(p);
        let ecx = ExecutionContext::with_store(cx.doc, cx.spatial.store());
        self.ghost = match entities_vertex::plan(&ecx, &input) {
            CommandResult::Completed { output, .. } => output
                .changed
                .first()
                .map(|e| entity_outline(&crate::spatial::record(e).3, 96.0))
                .unwrap_or_default(),
            _ => Vec::new(),
        };
    }
}

impl Tool for SegmentEdit {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        match (self.ask, self.start) {
            (Ask::Arc, _) => Prompt::new(
                LABEL,
                "yayın geçeceği noktayı gösterin ya da yarıçapı yazın (imlecin yanına bükülür)",
            ),
            (Ask::Width, None) => Prompt::new(
                LABEL,
                "kenarın başlangıç genişliğini yazın (0: genişliksiz)",
            ),
            (Ask::Width, Some(_)) => Prompt::new(
                LABEL,
                "kenarın bitiş genişliğini yazın (Enter: başlangıçtaki)",
            ),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.world);
        if self.ask == Ask::Arc {
            self.plan_ghost(p.world, cx);
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.ask == Ask::Arc {
            let input = self.with_through(p.world);
            self.write(input, cx);
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let value = crate::points::plain_value(text, cx);
        match (self.ask, value) {
            (Ask::Arc, Some((_, metres))) => {
                // The typed radius bows to the pointer's side.
                let side = self.hover.map_or(1.0, |h| self.side(h));
                let input = EntitiesVertex {
                    through: None,
                    radius: Some(metres.abs() * side),
                    ..self.input.clone()
                };
                self.write(input, cx);
                true
            }
            (Ask::Arc, None) => match cx.typed_point(text, None, self.hover) {
                Some(p) => {
                    let input = self.with_through(p);
                    self.write(input, cx);
                    true
                }
                None => false,
            },
            (Ask::Width, Some((_, metres))) => {
                match self.start {
                    None => self.start = Some(metres),
                    Some(start) => {
                        let input = EntitiesVertex {
                            widths: Some([start, metres]),
                            ..self.input.clone()
                        };
                        self.write(input, cx);
                    }
                }
                true
            }
            (Ask::Width, None) => false,
        }
    }

    /// Enter: the end width the start's; with nothing typed, the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        match (self.ask, self.start) {
            (Ask::Width, Some(start)) => {
                let input = EntitiesVertex {
                    widths: Some([start, start]),
                    ..self.input.clone()
                };
                self.write(input, cx);
                if self.done { Flow::Exit } else { Flow::Stay }
            }
            _ => Flow::Exit,
        }
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.start.take().is_some() {
            return true;
        }
        false
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut preview = Preview::default();
        preview
            .strokes
            .push(Stroke::dashed(vec![self.a, self.b], false, [3.0, 3.0]));
        if self.ask == Ask::Arc && !self.ghost.is_empty() {
            preview
                .strokes
                .push(Stroke::solid(self.ghost.clone(), false).width(1.5));
        }
        preview
    }
}
