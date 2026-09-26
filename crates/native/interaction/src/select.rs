//! Seç: the web's `SelectTool` (`apps/web/src/tools/SelectTool.ts`), what the
//! pointer does while no command runs (docs/adr/0029):
//!
//! - a click picks the most specific object under it within the pick
//!   aperture (`drafting.pickAperture`): Shift turns it over in the
//!   selection, else it becomes the selection; a click on nothing clears
//!   the selection (Shift keeps it);
//! - a drag past 4 px draws a box: left to right a window (objects wholly
//!   inside), right to left a crossing (objects it touches); Shift adds them,
//!   else they become the selection;
//! - with no button down, the object under the pointer is hovered.
//!
//! Hidden layers' objects are never picked; locked layers' are, as on the
//! web (the erase tool leaves them in place). Ctrl does nothing here, as on
//! the web. Grips, the double click that edits text in place and the hold
//! that opens the selection menu are not on the desktop yet.

use kentos_geometry_core::jsmath::js_hypot;

use crate::Vec2;
use crate::tool::{Context, Pointer};

/// How far the pointer must move with the button down to draw a box, logical pixels.
const DRAG_THRESHOLD: f64 = 4.0;

/// Where a press or the pointer is: on the area and in the world (before snapping).
#[derive(Clone, Copy, Debug, PartialEq)]
struct At {
    screen: [f64; 2],
    world: Vec2,
}

/// The selection box being drawn, for the host to show: logical pixels on
/// the drawing area. A crossing box goes right to left.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SelectBox {
    pub from: [f64; 2],
    pub to: [f64; 2],
}

impl SelectBox {
    /// Right to left: every object it touches (the web draws it dashed, in the snap colour).
    pub fn crossing(&self) -> bool {
        self.to[0] < self.from[0]
    }
}

/// The select tool's state between pointer events.
#[derive(Clone, Debug, Default)]
pub struct Select {
    start: Option<At>,
    current: Option<At>,
    dragging: bool,
}

impl Select {
    pub fn new() -> Self {
        Self::default()
    }

    /// The left button went down: a click or a box starts here.
    pub fn pointer_down(&mut self, p: &Pointer) {
        let at = At {
            screen: p.screen,
            world: p.raw,
        };
        self.start = Some(at);
        self.current = Some(at);
        self.dragging = false;
    }

    /// The pointer moved: the box grows, or the object under it is hovered.
    pub fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(start) = self.start else {
            let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
            cx.selection.set_hover(hit);
            return;
        };
        self.current = Some(At {
            screen: p.screen,
            world: p.raw,
        });
        let moved = js_hypot(p.screen[0] - start.screen[0], p.screen[1] - start.screen[1]);
        if !self.dragging && moved > DRAG_THRESHOLD {
            self.dragging = true;
            cx.selection.set_hover(None);
        }
    }

    /// The left button came up: the box selects, or the click picks.
    pub fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(start) = self.start.take() else {
            return;
        };
        let current = self.current.take();
        let dragging = std::mem::take(&mut self.dragging);
        match current {
            Some(current) if dragging => {
                // The box as it was drawn decides: one rule for the look and the query.
                let crossing = SelectBox {
                    from: start.screen,
                    to: current.screen,
                }
                .crossing();
                let ids = cx.spatial.in_rect(start.world, current.world, crossing);
                if p.shift {
                    cx.selection.add(ids);
                } else {
                    cx.selection.set(ids);
                }
            }
            _ => match cx.spatial.pick(p.raw, cx.pick_tolerance()) {
                Some(hit) if p.shift => cx.selection.toggle(hit),
                Some(hit) => cx.selection.set([hit]),
                None if !p.shift => cx.selection.clear(),
                None => {}
            },
        }
    }

    /// The box being drawn, once the pointer has moved far enough.
    pub fn select_box(&self) -> Option<SelectBox> {
        match (self.dragging, self.start, self.current) {
            (true, Some(start), Some(current)) => Some(SelectBox {
                from: start.screen,
                to: current.screen,
            }),
            _ => None,
        }
    }

    /// Forgets a press in progress (a command started, a drawing opened).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
