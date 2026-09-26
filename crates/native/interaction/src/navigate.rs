//! Kaydır and Pencere yakınlaştır: the web's `PanTool` and `ZoomWindowTool`
//! (`apps/web/src/tools/SelectTool.ts`), step for step (docs/adr/0056):
//!
//! - Kaydır: a drag with the left button moves the view with the pointer
//!   (the middle button does it with any tool). It stays until Esc and is
//!   not remembered for repeat: Enter repeats the command before it;
//! - Pencere yakınlaştır: two clicks, or a drag past 4 px, give the corners
//!   of a box, and the view shows it as large as it fits, with no margin;
//!   then it leaves. A box with no width or no height changes nothing.
//!
//! Neither snaps, takes a confirm (Enter repeats the last command) or
//! changes the drawing; the host's camera takes what they ask for
//! ([`ViewChange`]).

use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::js_hypot;

use crate::Vec2;
use crate::format::Format;
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Stroke, Tool, ViewChange};

/// Kaydır's id: its command is `tool.pan`.
pub const PAN_ID: &str = "pan";
pub const PAN_LABEL: &str = "Kaydır";
/// Pencere yakınlaştır's id: its command is `tool.zoomWindow`.
pub const ZOOM_WINDOW_ID: &str = "zoomWindow";
pub const ZOOM_WINDOW_LABEL: &str = "Pencere yakınlaştır";

/// How far the pointer must move with the button down to drag a box, logical
/// pixels (the web's `DRAG_THRESHOLD`).
const DRAG_THRESHOLD: f64 = 4.0;
/// The box's dash and gap, logical pixels (the web's `[5, 4]`).
const BOX_DASH: [f32; 2] = [5.0, 4.0];
/// A side this short is no box (the web's `1e-6`).
const LEAST_SIDE: f64 = 1e-6;

/// Kaydır.
#[derive(Clone, Debug, Default)]
pub struct Pan {
    /// Where the pointer was at the drag's last event.
    last: Option<[f64; 2]>,
}

impl Pan {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Tool for Pan {
    fn id(&self) -> &'static str {
        PAN_ID
    }

    fn label(&self) -> &'static str {
        PAN_LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(PAN_LABEL, "sürükleyerek görünümü taşıyın. Çıkmak için Esc")
    }

    fn point_count(&self) -> usize {
        0
    }

    fn snaps(&self) -> bool {
        false
    }

    /// While the button is down, the view follows the pointer.
    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(last) = self.last else {
            return;
        };
        cx.view_changes.push(ViewChange::Pan {
            dx: p.screen[0] - last[0],
            dy: p.screen[1] - last[1],
        });
        self.last = Some(p.screen);
    }

    fn pointer_down(&mut self, p: &Pointer, _cx: &mut Context<'_>) {
        self.last = Some(p.screen);
    }

    fn pointer_up(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {
        self.last = None;
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// Never asked: the tool takes no confirm ([`Tool::confirms`]).
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn confirms(&self) -> bool {
        false
    }

    fn cursor(&self) -> Cursor {
        Cursor::Grab
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview::default()
    }
}

/// Pencere yakınlaştır.
#[derive(Clone, Debug, Default)]
pub struct ZoomWindow {
    /// The first corner: the world point under the pointer (unsnapped) and
    /// where the pointer was on the area.
    first: Option<(Vec2, [f64; 2])>,
    /// The pointer's world point, for the box.
    hover: Option<Vec2>,
    /// The view was asked for; the tool leaves.
    done: bool,
}

impl ZoomWindow {
    pub fn new() -> Self {
        Self::default()
    }

    /// The opposite corner: the view shows the box, when it has a width and a height.
    fn finish(&mut self, corner: Vec2, cx: &mut Context<'_>) {
        if let Some((a, _)) = self.first
            && (corner.x - a.x).abs() > LEAST_SIDE
            && (corner.y - a.y).abs() > LEAST_SIDE
        {
            cx.view_changes.push(ViewChange::Fit {
                bounds: Bounds {
                    min_x: a.x.min(corner.x),
                    min_y: a.y.min(corner.y),
                    max_x: a.x.max(corner.x),
                    max_y: a.y.max(corner.y),
                },
                padding: 0.0,
            });
        }
        self.done = true;
    }
}

impl Tool for ZoomWindow {
    fn id(&self) -> &'static str {
        ZOOM_WINDOW_ID
    }

    fn label(&self) -> &'static str {
        ZOOM_WINDOW_LABEL
    }

    fn prompt(&self) -> Prompt {
        let step = if self.first.is_none() {
            "ilk köşeyi belirtin"
        } else {
            "karşı köşeyi belirtin"
        };
        Prompt::new(ZOOM_WINDOW_LABEL, step)
    }

    /// The web's tool counts no points: Ctrl+Z undoes the drawing.
    fn point_count(&self) -> usize {
        0
    }

    fn snaps(&self) -> bool {
        false
    }

    fn pointer_move(&mut self, p: &Pointer, _cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
    }

    /// The first click gives the first corner, the second the opposite one.
    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match self.first {
            None => self.first = Some((p.raw, p.screen)),
            Some(_) => self.finish(p.raw, cx),
        }
    }

    /// Let go past the drag threshold: the box is dragged.
    fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if let Some((_, from)) = self.first
            && !self.done
            && js_hypot(p.screen[0] - from[0], p.screen[1] - from[1]) > DRAG_THRESHOLD
        {
            self.finish(p.raw, cx);
        }
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// Never asked: the tool takes no confirm ([`Tool::confirms`]).
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn confirms(&self) -> bool {
        false
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// The box from the first corner to the pointer, dashed in the accent colour.
    fn preview(&self, _format: &Format) -> Preview {
        let (Some((a, _)), Some(b)) = (self.first, self.hover) else {
            return Preview::default();
        };
        let ring = vec![a, Vec2::new(b.x, a.y), b, Vec2::new(a.x, b.y)];
        Preview {
            strokes: vec![Stroke::dashed(ring, true, BOX_DASH)],
            ..Preview::default()
        }
    }

    fn finished(&self) -> bool {
        self.done
    }
}
