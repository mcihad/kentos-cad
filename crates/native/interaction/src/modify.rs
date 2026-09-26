//! What the modify tools share: the web's `SelectionFirstTool`
//! (`apps/web/src/tools/modifyTools.ts`) without its DOM side, step for step
//! (docs/adr/0037):
//!
//! - started with a selection, the tool asks for its points at once;
//!   started without one, it picks first: a click turns the object under it
//!   over in the selection, a drag past 4 px adds what its box holds (left
//!   to right a window, right to left a crossing), until a confirm (Enter,
//!   Space, a quick right click) with something selected;
//! - then each tool's stages ([`Stages`]): its points, clicked (object snaps,
//!   ortho and polar tracking from its anchor) or typed, and its options;
//! - what it does is written through the product command
//!   `cad.entities.transform`: the selection's persistent ids explicit in the
//!   input (TODOS.md CMD-07), in place or as copies, one undo step named
//!   after the tool. The command's refusal or warning is the tool's message.
//!
//! The preview is the web's: the selection's outlines where the transform
//! would put them, dashed (at most 400 objects, the geometry store draws
//! them: `transform_outlines`), the line from the anchor to the cursor, the
//! tag beside the cursor. None of the geometry is computed here.

use kentos_contracts::{EntitiesTransform, EntitiesTransformed, Transform};
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::jsmath::js_hypot;
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_geometry_core::tools::point_text::point_from_text;
use kentos_native_application::{ExecutionContext, transform};

use crate::Vec2;
use crate::format::Format;
use crate::points;
use crate::prompt::Prompt;
use crate::select::SelectBox;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tag, Tool};

/// Ghosts drawn at most (the web's `MAX_GHOSTS`): one more is drawn, then the rest are left out.
const MAX_GHOSTS: usize = 400;
/// How far the pointer must move with the button down to draw a box, logical pixels.
const DRAG_THRESHOLD: f64 = 4.0;
/// A ghost's dash and gap, logical pixels (the web's `[4, 3]`).
const GHOST_DASH: [f32; 2] = [4.0, 3.0];

/// One modify tool's own part: its stages after the selection is confirmed.
pub trait Stages {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    /// The selection is confirmed: the stages start over (the web's `begin`).
    fn begin(&mut self);
    /// Where ortho, polar tracking, perpendicular snaps and typed `@` points
    /// are measured from (the web's `anchor`).
    fn anchor(&self) -> Option<Vec2>;
    /// The stage's prompt; `n` is the selection's size.
    fn prompt(&self, n: usize) -> Prompt;
    /// A point, clicked or typed. [`Flow::Exit`] once the tool is done.
    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) -> Flow;
    /// Typed text the stage reads before a point (an option, a number):
    /// what became of it, or `None` to read it as a point.
    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow>;
    /// The transform to preview with the cursor at `hover` (the web's `previewTransforms`).
    fn preview(&self, hover: Vec2) -> Option<Affine>;
    /// What the tag beside the cursor says (the web's `previewTag`).
    fn tag(&self, _hover: Vec2, _format: &Format) -> Vec<String> {
        Vec::new()
    }
}

/// A press on the drawing while picking: where it began and where the pointer is.
#[derive(Clone, Copy, Debug)]
struct Press {
    from: [f64; 2],
    from_world: Vec2,
    to: [f64; 2],
    to_world: Vec2,
    dragging: bool,
}

/// A modify tool: picking, then its stages.
pub struct Modify<S: Stages> {
    stages: S,
    picking: bool,
    press: Option<Press>,
    /// The effective cursor in the stages (the web's `hover`).
    hover: Option<Vec2>,
    tracking: Option<Tracking>,
    /// The selection's size when it was last looked at, for the prompt.
    selected: usize,
    /// Where the transform would put the selection: outlines and point marks.
    ghosts: Vec<Stroke>,
    marks: Vec<Vec2>,
    /// Written and left (the web's `ctx.tools.exit()` after a transform).
    done: bool,
}

impl<S: Stages> Modify<S> {
    pub(crate) fn with(stages: S) -> Self {
        Self {
            stages,
            picking: true,
            press: None,
            hover: None,
            tracking: None,
            selected: 0,
            ghosts: Vec::new(),
            marks: Vec::new(),
            done: false,
        }
    }

    /// The effective cursor from the anchor (ortho, polar tracking; a snapped point exact).
    fn constrain(&mut self, p: &Pointer, cx: &Context<'_>) -> Vec2 {
        let (point, tracking) = points::constrain(self.stages.anchor(), p, cx);
        self.tracking = tracking;
        point
    }

    /// The ghosts where the transform would put the selection, from the
    /// geometry store (the web draws them every frame from `ghosts`).
    fn refresh(&mut self, cx: &Context<'_>) {
        self.ghosts.clear();
        self.marks.clear();
        self.selected = cx.selection.len();
        if self.picking || self.done {
            return;
        }
        let Some(m) = self.hover.and_then(|hover| self.stages.preview(hover)) else {
            return;
        };
        let ids: Vec<f64> = cx
            .selection
            .ids()
            .iter()
            .map(|slot| f64::from(slot.0))
            .collect();
        let paths = cx
            .spatial
            .store()
            .transform_outlines(&ids, &[m], MAX_GHOSTS);
        // `flags, n, x0, y0, …` per path: 0 open, 1 closed, 2 a marker (points and text).
        let mut at = 0;
        while at + 1 < paths.len() {
            let flags = paths[at];
            let n = paths[at + 1] as usize;
            let pts: Vec<Vec2> = (0..n)
                .filter_map(|k| {
                    let (x, y) = (*paths.get(at + 2 + 2 * k)?, *paths.get(at + 3 + 2 * k)?);
                    Some(Vec2::new(x, y))
                })
                .collect();
            at += 2 + 2 * n;
            if flags == 2.0 {
                self.marks.extend(pts.first());
            } else if pts.len() >= 2 {
                self.ghosts
                    .push(Stroke::dashed(pts, flags == 1.0, GHOST_DASH));
            }
        }
    }

    fn after(&mut self, flow: Flow, cx: &Context<'_>) {
        if flow == Flow::Exit {
            self.done = true;
        }
        self.refresh(cx);
    }
}

impl<S: Stages> Tool for Modify<S> {
    fn id(&self) -> &'static str {
        self.stages.id()
    }

    fn label(&self) -> &'static str {
        self.stages.label()
    }

    fn prompt(&self) -> Prompt {
        if self.picking {
            Prompt::new(
                self.stages.label(),
                format!(
                    "nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın ({} seçili)",
                    self.selected
                ),
            )
        } else {
            self.stages.prompt(self.selected)
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    /// With a selection the stages start at once; without one, picking first.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.picking = cx.selection.is_empty();
        if !self.picking {
            self.stages.begin();
        }
        self.refresh(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        if self.picking {
            None
        } else {
            self.stages.anchor()
        }
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.picking {
            match &mut self.press {
                Some(press) => {
                    press.to = p.screen;
                    press.to_world = p.raw;
                    let moved = js_hypot(p.screen[0] - press.from[0], p.screen[1] - press.from[1]);
                    if !press.dragging && moved > DRAG_THRESHOLD {
                        press.dragging = true;
                        cx.selection.set_hover(None);
                    }
                }
                None => {
                    let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
                    cx.selection.set_hover(hit);
                }
            }
            return;
        }
        let point = self.constrain(p, cx);
        self.hover = Some(point);
        self.refresh(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.picking {
            // A click turns one object over; a drag draws a box (decided on release).
            self.press = Some(Press {
                from: p.screen,
                from_world: p.raw,
                to: p.screen,
                to_world: p.raw,
                dragging: false,
            });
            return;
        }
        let point = self.constrain(p, cx);
        let flow = self.stages.point(point, cx);
        self.after(flow, cx);
    }

    fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(press) = self.press.take().filter(|_| self.picking) else {
            return;
        };
        if press.dragging {
            // The box as it was drawn decides: one rule for the look and the query.
            let crossing = SelectBox {
                from: press.from,
                to: press.to,
            }
            .crossing();
            let ids = cx
                .spatial
                .in_rect(press.from_world, press.to_world, crossing);
            cx.selection.add(ids);
        } else if let Some(hit) = cx.spatial.pick(p.raw, cx.pick_tolerance()) {
            cx.selection.toggle(hit);
        }
        self.refresh(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.picking {
            return false;
        }
        if let Some(flow) = self.stages.typed(text, cx) {
            self.after(flow, cx);
            return true;
        }
        let Some(p) = point_from_text(text, self.stages.anchor(), self.hover, |_| None) else {
            return false;
        };
        let flow = self.stages.point(p, cx);
        self.after(flow, cx);
        true
    }

    /// Picking with something selected: on to the stages. Otherwise the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.picking && !cx.selection.is_empty() {
            self.picking = false;
            self.press = None;
            cx.selection.set_hover(None);
            self.stages.begin();
            self.refresh(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    /// Ctrl+Z undoes the drawing: the web's modify tools take no step back.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn select_box(&self) -> Option<SelectBox> {
        let press = self.press.filter(|press| self.picking && press.dragging)?;
        Some(SelectBox {
            from: press.from,
            to: press.to,
        })
    }

    fn preview(&self, format: &Format) -> Preview {
        if self.picking {
            return Preview::default();
        }
        let mut strokes = self.ghosts.clone();
        if let (Some(a), Some(hover)) = (self.stages.anchor(), self.hover) {
            strokes.push(Stroke::solid(vec![a, hover], false));
        }
        let tag = self.hover.and_then(|hover| {
            let lines = self.stages.tag(hover, format);
            (!lines.is_empty()).then_some(Tag { at: hover, lines })
        });
        Preview {
            strokes,
            marks: self.marks.clone(),
            tag,
            tracking: self.hover.and(self.tracking),
            ..Preview::default()
        }
    }
}

/// Writes a transform of the selection through the product command
/// `cad.entities.transform` (docs/adr/0037): the selected objects'
/// persistent ids, in place or as copies. The command's refusal or warning
/// is said as the tool's; how many objects were written, or `None`.
pub(crate) fn transform_selection(
    transform: Transform,
    copy: bool,
    cx: &mut Context<'_>,
) -> Option<usize> {
    let uids = cx
        .selection
        .ids()
        .iter()
        .filter_map(|slot| cx.doc.uid(*slot))
        .map(|uid| uid.to_string())
        .collect();
    let input = EntitiesTransform {
        uids,
        transform,
        copy: copy.then_some(true),
        expected_revision: None,
    };
    let result = transform::execute(&mut ExecutionContext::new(cx.doc), input);
    points::written(result, cx).map(|out: EntitiesTransformed| {
        if copy {
            out.created.len()
        } else {
            out.changed.len()
        }
    })
}
