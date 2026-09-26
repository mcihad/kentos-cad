//! Nokta: the web's `PointTool` without an elevation (`askZ: false`,
//! `apps/web/src/tools/drawTools.ts`) on its `PointInputTool` base, step for
//! step (docs/adr/0032):
//!
//! - every point, clicked (snaps, ortho and polar tracking applied) or typed
//!   (the shared grammar), writes one point object through `cad.point.create`:
//!   its own object and its own undo step; the point's echo is its message;
//! - the tool never holds a point, so a confirm (Enter, Space, a quick right
//!   click) leaves it;
//! - Ctrl+Z takes the newest point back as an undo while the drawing has not
//!   changed since; with nothing of its own it undoes the drawing.
//!
//! Nothing is drawn while it runs but the cursor and the snap marker, as on
//! the web.

use kentos_contracts::PointCreate;
use kentos_geometry_core::tools::point_text::point_from_text;
use kentos_native_application::{ExecutionContext, point};

use crate::Vec2;
use crate::format::Format;
use crate::points::{self, Taken, wire};
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Tool};

/// The point tool's id: its command is `tool.point`.
pub const ID: &str = "point";
pub const LABEL: &str = "Nokta";

/// The point tool: one point object per point given.
#[derive(Clone, Debug, Default)]
pub struct Point {
    d: Taken,
}

impl Point {
    pub fn new() -> Self {
        Self::default()
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        self.write(p, cx);
    }

    /// Writes one point through the product command `cad.point.create`
    /// (docs/adr/0032): the active layer explicit in its input (CMD-07); the
    /// desktop has no current colour, so the layer's applies.
    fn write(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let input = PointCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            p: wire(p),
            z: None,
            label: None,
            color: None,
            attrs: None,
            expected_revision: None,
        };
        let result = point::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(written) = points::written(result, cx) {
            self.d.note(written.id, cx);
        }
    }
}

impl Tool for Point {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "nokta konumunu belirtin")
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
        match point_from_text(text, self.d.last(), self.d.hover, |_| None) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }

    /// It never holds a point: a confirm leaves (the web's `PointInputTool.confirm`).
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(false, cx)
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview::default()
    }
}
