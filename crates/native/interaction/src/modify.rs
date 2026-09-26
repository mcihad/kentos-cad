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
//!   `cad.entities.transform` (the arrays: `cad.entities.array`, docs/adr/0047):
//!   the selection's persistent ids explicit in the input (TODOS.md CMD-07),
//!   in place or as copies, one undo step named after the tool. The
//!   command's refusal or warning is the tool's message.
//!
//! The preview is the web's: the selection's outlines where the transforms
//! would put them, dashed (at most 400 objects, the geometry store draws
//! them: `transform_outlines`), the line from the anchor to the cursor, the
//! tag beside the cursor. None of the geometry is computed here.

use kentos_contracts::{
    ArrayLayout, EntitiesArray, EntitiesArrayed, EntitiesTransform, EntitiesTransformed, Transform,
};
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::jsmath::js_hypot;
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_geometry_core::tools::point_text::point_from_text;
use kentos_native_application::{ExecutionContext, array, transform};

use crate::Vec2;
use crate::format::Format;
use crate::outlines;
use crate::points;
use crate::prompt::Prompt;
use crate::select::SelectBox;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tag, Tool};

/// Ghosts drawn at most (the web's `MAX_GHOSTS`): one more is drawn, then the rest are left out.
pub(crate) const MAX_GHOSTS: usize = 400;
/// How far the pointer must move with the button down to draw a box, logical pixels.
const DRAG_THRESHOLD: f64 = 4.0;
/// A ghost's dash and gap, logical pixels (the web's `[4, 3]`).
pub(crate) const GHOST_DASH: [f32; 2] = [4.0, 3.0];

/// One modify tool's own part: its stages after the selection is confirmed.
pub trait Stages {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    /// The selection is confirmed: the stages start over (the web's `begin`).
    /// A tool that acts at once on the selection (Birleştir, Patlat) does so
    /// here and answers [`Flow::Exit`].
    fn begin(&mut self, cx: &mut Context<'_>) -> Flow;
    /// Sees what the session remembers and the project's units at every
    /// event, for a prompt that shows them while picking.
    fn see(&mut self, _cx: &Context<'_>) {}
    /// The picking prompt, with what the tool adds to it (the web's `pickHint`).
    fn picking_hint(&self, prompt: Prompt) -> Prompt {
        prompt
    }
    /// Typed text while picking (Birleştir's tolerance): whether it was taken.
    fn picking_input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }
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
    /// Whether text the stage did not read is read as a point (the web's
    /// `super.input`): not by Dizi, which reads counts and spacings only.
    fn typed_points(&self) -> bool {
        true
    }
    /// A confirm (Enter, Space, a quick right click) in the stages: the web's
    /// modify tools leave; the arrays go on to their next stage or write.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }
    /// The transform to preview with the cursor at `hover` (the web's `previewTransforms`).
    fn preview(&self, hover: Vec2) -> Option<Affine>;
    /// Every transform to preview, one ghost of the selection each (the web's
    /// `previewTransforms`): the arrays' copies show without a cursor too.
    fn previews(&self, hover: Option<Vec2>) -> Vec<Affine> {
        hover
            .and_then(|hover| self.preview(hover))
            .into_iter()
            .collect()
    }
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

    /// The ghosts where the transforms would put the selection, from the
    /// geometry store (the web draws them every frame from `ghosts`).
    fn refresh(&mut self, cx: &Context<'_>) {
        self.stages.see(cx);
        self.ghosts.clear();
        self.marks.clear();
        self.selected = cx.selection.len();
        if self.picking || self.done {
            return;
        }
        let affines = self.stages.previews(self.hover);
        if affines.is_empty() {
            return;
        }
        let ids: Vec<f64> = cx
            .selection
            .ids()
            .iter()
            .map(|slot| f64::from(slot.0))
            .collect();
        let paths = cx
            .spatial
            .store()
            .transform_outlines(&ids, &affines, MAX_GHOSTS);
        (self.ghosts, self.marks) = ghosts(&paths);
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
            self.stages.picking_hint(Prompt::new(
                self.stages.label(),
                format!(
                    "nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın ({} seçili)",
                    self.selected
                ),
            ))
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
        if !self.picking && self.stages.begin(cx) == Flow::Exit {
            return Flow::Exit;
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
            let taken = self.stages.picking_input(text, cx);
            self.refresh(cx);
            return taken;
        }
        if let Some(flow) = self.stages.typed(text, cx) {
            self.after(flow, cx);
            return true;
        }
        if !self.stages.typed_points() {
            return false;
        }
        let Some(p) = point_from_text(text, self.stages.anchor(), self.hover, |_| None) else {
            return false;
        };
        let flow = self.stages.point(p, cx);
        self.after(flow, cx);
        true
    }

    /// Picking with something selected: on to the stages; picking with
    /// nothing, the tool leaves. In the stages, the stage decides (the web's
    /// modify tools leave, the arrays go on).
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if !self.picking {
            let flow = self.stages.confirm(cx);
            self.after(flow, cx);
            return flow;
        }
        if cx.selection.is_empty() {
            return Flow::Exit;
        }
        self.picking = false;
        self.press = None;
        cx.selection.set_hover(None);
        if self.stages.begin(cx) == Flow::Exit {
            return Flow::Exit;
        }
        self.refresh(cx);
        Flow::Stay
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

/// Ghost outlines as the geometry store gives them (`transform_outlines`,
/// `stretch_outlines`): dashed lines and point marks, as the web's
/// `strokePaths` draws the ghosts.
pub(crate) fn ghosts(paths: &[f64]) -> (Vec<Stroke>, Vec<Vec2>) {
    outlines::read(paths, |pts, closed| Stroke::dashed(pts, closed, GHOST_DASH))
}

/// The selected objects' persistent ids, as the commands name them.
fn selected_uids(cx: &Context<'_>) -> Vec<String> {
    cx.selection
        .ids()
        .iter()
        .filter_map(|slot| cx.doc.uid(*slot))
        .map(|uid| uid.to_string())
        .collect()
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
    let input = EntitiesTransform {
        uids: selected_uids(cx),
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

/// Writes copies of the selection laid out by `layout` through the product
/// command `cad.entities.array` (docs/adr/0047): the selected objects'
/// persistent ids, one undo step named after the tool. The command's refusal
/// or warning is said as the tool's; how many copies were made, or `None`.
pub(crate) fn array_selection(layout: ArrayLayout, cx: &mut Context<'_>) -> Option<usize> {
    let input = EntitiesArray {
        uids: selected_uids(cx),
        layout,
        expected_revision: None,
    };
    let result = array::execute(&mut ExecutionContext::new(cx.doc), input);
    points::written(result, cx).map(|out: EntitiesArrayed| out.created.len())
}
