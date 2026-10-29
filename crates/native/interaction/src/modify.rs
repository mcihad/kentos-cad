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

use kentos_commands::{
    ArrayLayout, EntitiesArray, EntitiesArrayed, EntitiesTransform, EntitiesTransformed, Entity,
    Transform,
};
use kentos_domain::Slot;
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::jsmath::js_hypot;
use kentos_geometry_core::ops::transform::{TextPolicy, transform_shape_with};
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_native_application::geometry::{shape, with_shape};
use kentos_native_application::{ExecutionContext, array, transform};

use crate::Vec2;
use crate::format::Format;
use crate::js_trim;
use crate::outlines;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::select::SelectBox;
use crate::tool::{Context, Flow, Memory, Pointer, Preview, Stroke, Tag, Tool};

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
    /// A click on an object while picking, before it is selected: the
    /// stage's flow when it takes the click (Birleştir's Zincir), else
    /// `None` and the object is selected as usual.
    fn picked(&mut self, _hit: Slot, _cx: &mut Context<'_>) -> Option<Flow> {
        None
    }
    /// Where ortho, polar tracking, perpendicular snaps and typed `@` points
    /// are measured from (the web's `anchor`).
    fn anchor(&self) -> Option<Vec2>;
    /// Whether object snaps apply (the web's `snaps`): not for Hızlı ölçü,
    /// whose cursor's distance to the nearest edge places its dimensions.
    fn snaps(&self) -> bool {
        true
    }
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
    /// After [`Stages::begin`] kept the tool: pick again (Alan çıkar's
    /// second selection; Alan böl with no area to cut).
    fn repick(&self) -> bool {
        false
    }
    /// The picking prompt in place of the base's (Alan çıkar's two rounds);
    /// `n` is the selection's size.
    fn picking_prompt(&self, _n: usize) -> Option<Prompt> {
        None
    }
    /// What is drawn while picking (Alan çıkar: the areas to cut from).
    fn picking_preview(&self) -> Preview {
        Preview::default()
    }
    /// The stages' own drawing in place of the ghosts and the anchor's line
    /// (Alan böl: the cut line and the pieces it would leave).
    fn stage_preview(&self, _hover: Option<Vec2>, _format: &Format) -> Option<Preview> {
        None
    }
    /// The pointer in the stages, before it becomes a point (Alan böl picks
    /// its cutting line): whether the stage took it. A press that took it
    /// may end the tool.
    fn pointer(&mut self, _p: &Pointer, _down: bool, _cx: &mut Context<'_>) -> Option<Flow> {
        None
    }
    /// Esc in the stages: the stage steps back (Yol boyunca dizi lets go of
    /// its path) and answers true; false when it has nothing to give back
    /// and the tool leaves.
    fn back(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }
    /// The values an option offers in its menu in the stages (Hızlı ölçü's
    /// Stil, docs/adr/0183 §4); none by default.
    fn option_choices(&self, _key: &str) -> Vec<crate::tool::OptionChoice> {
        Vec::new()
    }
    /// One of `option_choices` chosen, as its key and value typed.
    fn choose_option(&mut self, _key: &str, _typed: &str, _cx: &mut Context<'_>) -> bool {
        false
    }
    /// Visible settings for an opt-in stage; existing modify tools have none.
    fn settings(&self, _format: &Format) -> Vec<crate::tool_settings::Setting> {
        Vec::new()
    }
    fn configure(&mut self, _key: &str, _value: &str, _cx: &mut Context<'_>) -> bool {
        false
    }
    /// Whether the stage asks for words (Hızlı ölçü's style name, docs/adr/0183 §4).
    fn takes_words(&self) -> bool {
        false
    }
    /// Lines and marks drawn with the ghosts, not in place of them (Yol
    /// boyunca dizi: the path and where the copies start).
    fn overlay(&self) -> Preview {
        Preview::default()
    }
    /// Whether the stages take a point computed elsewhere (the point
    /// calculator's) as a click. Kot ver takes numbers only.
    fn takes_points(&self) -> bool {
        true
    }
    /// The answer of a text field the stage asked for (Köşelere nokta's Ad
    /// and Kod): the typed text, or none (Esc).
    fn text_typed(&mut self, _text: Option<&str>, _cx: &mut Context<'_>) {}
    /// After a call in the stages: whether the tool goes back to picking its
    /// objects (Kot ver after a write, and on Esc). Asked once per call; the
    /// selection is the stage's to keep or to clear.
    fn take_repick(&mut self) -> bool {
        false
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
    /// Source-driven dimension membership, computed in a reversible document preview.
    followers: Preview,
    follower_scratch: Option<kentos_domain::Document>,
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
            followers: Preview::default(),
            follower_scratch: None,
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
        self.followers = Preview::default();
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
        // Existing copy/array previews stay unchanged. The domain owns all derived
        // membership; the same shared matrix kernel supplies only the source edits.
        if !cx.doc.dimension_sets().is_empty()
            && matches!(
                self.stages.id(),
                "move" | "rotate" | "scale" | "mirror" | "align"
            )
            && let [matrix] = affines.as_slice()
        {
            let changes: Vec<_> = cx
                .selection
                .ids()
                .iter()
                .take(MAX_GHOSTS)
                .filter_map(|&slot| {
                    let entity = cx.doc.get(slot)?;
                    if !matches!(entity, Entity::Wall(_) | Entity::Opening(_))
                        || cx.doc.layers().is_locked(&entity.base().layer_id)
                    {
                        return None;
                    }
                    with_shape(
                        entity,
                        transform_shape_with(&shape(entity), matrix, TextPolicy::Readable),
                    )
                    .map(|after| (slot, after))
                })
                .collect();
            let delta =
                cx.doc
                    .preview_followers_with(&mut self.follower_scratch, &changes, &[], &[]);
            for (_, entity) in delta.updated.iter().chain(&delta.added).take(MAX_GHOSTS) {
                if matches!(entity, Entity::Dimension(_)) {
                    let mut preview = crate::wall_dimensions::dimension_preview(
                        entity,
                        &cx.format(),
                        cx.doc.settings().drawing_font,
                    );
                    for stroke in &mut preview.strokes {
                        stroke.tone = crate::tool::Tone::Snap;
                    }
                    self.followers.strokes.extend(preview.strokes);
                    self.followers.texts.extend(preview.texts);
                }
            }
            for slot in delta.removed.iter().take(MAX_GHOSTS) {
                if let Some(entity) = cx.doc.get(*slot) {
                    self.followers.strokes.extend(
                        crate::edge::Outline::of(
                            &shape(entity),
                            Some([2., 3.]),
                            1.0,
                            crate::tool::Tone::Danger,
                        )
                        .strokes,
                    );
                }
            }
        }
    }

    fn after(&mut self, flow: Flow, cx: &Context<'_>) {
        if flow == Flow::Exit {
            self.done = true;
        }
        self.pick_again();
        self.refresh(cx);
    }

    /// Goes back to picking objects when a stage asked to.
    fn pick_again(&mut self) {
        if self.stages.take_repick() {
            self.picking = true;
            self.press = None;
            self.hover = None;
            self.tracking = None;
        }
    }
}

impl<S: Stages> Tool for Modify<S> {
    /// Etkin kat changed (docs/adr/0720 §4): the stages see it (Çizgiden duvar's Kat follows it).
    fn level_changed(&mut self, cx: &mut Context<'_>) {
        self.stages.see(cx);
    }

    fn snaps(&self) -> bool {
        self.stages.snaps()
    }

    /// A computed point in the stages, not while picking (the web's
    /// `SelectionFirstTool.acceptPoint`).
    fn accepts_points(&self) -> bool {
        self.stages.takes_points()
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.picking {
            return false;
        }
        let flow = self.stages.point(p, cx);
        self.after(flow, cx);
        true
    }

    fn id(&self) -> &'static str {
        self.stages.id()
    }

    fn label(&self) -> &'static str {
        self.stages.label()
    }

    fn prompt(&self) -> Prompt {
        if self.picking
            && let Some(prompt) = self.stages.picking_prompt(self.selected)
        {
            return prompt;
        }
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
        if !self.picking {
            if self.stages.begin(cx) == Flow::Exit {
                return Flow::Exit;
            }
            self.picking = self.stages.repick();
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
                    // The first corner stays on the drawing through a pan or a zoom (docs/adr/0277).
                    press.from = cx.view.to_screen(press.from_world);
                    press.to_world = p.raw;
                    let moved = js_hypot(p.screen[0] - press.from[0], p.screen[1] - press.from[1]);
                    if !press.dragging && moved > DRAG_THRESHOLD {
                        press.dragging = true;
                        cx.selection.set_hover(None);
                    }
                }
                None => {
                    // The selection filter's kinds (docs/adr/0187 §5).
                    let hit = crate::selectable::hover(cx, p.raw);
                    cx.selection.set_hover(hit);
                }
            }
            return;
        }
        let point = self.constrain(p, cx);
        self.hover = Some(point);
        let _ = self.stages.pointer(p, false, cx);
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
        if let Some(flow) = self.stages.pointer(p, true, cx) {
            self.after(flow, cx);
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
                from: cx.view.to_screen(press.from_world),
                to: p.screen,
                worlds: None,
            }
            .crossing();
            let ids = crate::selectable::in_box(cx, press.from_world, p.raw, crossing);
            cx.selection.add(ids);
        } else if let Some(hit) = crate::selectable::pick(cx, p.raw) {
            if let Some(flow) = self.stages.picked(hit, cx) {
                self.after(flow, cx);
                return;
            }
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
        let Some(p) = cx.typed_point(text, self.stages.anchor(), self.hover) else {
            return false;
        };
        let flow = self.stages.point(p, cx);
        self.after(flow, cx);
        true
    }

    /// The stages' menus, while picking too: only a chip the prompt shows
    /// asks for one (Çoklu tara's Desen shows while picking, docs/adr/0186 §4).
    fn option_choices(&self, key: &str) -> Vec<crate::tool::OptionChoice> {
        self.stages.option_choices(key)
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let taken = self.stages.choose_option(key, typed, cx);
        self.refresh(cx);
        taken
    }

    fn settings(&self, format: &Format) -> Vec<crate::tool_settings::Setting> {
        self.stages.settings(format)
    }
    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let taken = self.stages.configure(key, value, cx);
        self.refresh(cx);
        taken
    }

    fn takes_words(&self) -> bool {
        !self.picking && self.stages.takes_words()
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
        self.picking = self.stages.repick();
        self.refresh(cx);
        Flow::Stay
    }

    fn text_typed(&mut self, text: Option<&str>, cx: &mut Context<'_>) {
        if self.picking {
            return;
        }
        self.stages.text_typed(text, cx);
        self.refresh(cx);
    }

    /// Ctrl+Z undoes the drawing: the web's modify tools take no step back.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// Esc: a stage that can step back does; picking and the rest leave.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.picking || self.done || !self.stages.back(cx) {
            return false;
        }
        self.pick_again();
        self.refresh(cx);
        true
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn select_box(&self) -> Option<SelectBox> {
        let press = self.press.filter(|press| self.picking && press.dragging)?;
        Some(SelectBox {
            from: press.from,
            to: press.to,
            worlds: Some((press.from_world, press.to_world)),
        })
    }

    fn preview(&self, format: &Format) -> Preview {
        if self.picking {
            return self.stages.picking_preview();
        }
        if let Some(preview) = self.stages.stage_preview(self.hover, format) {
            return Preview {
                tracking: self.hover.and(self.tracking),
                ..preview
            };
        }
        let mut strokes = self.ghosts.clone();
        strokes.extend(self.followers.strokes.iter().cloned());
        let overlay = self.stages.overlay();
        strokes.extend(overlay.strokes);
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
            markers: overlay.markers,
            texts: self.followers.texts.clone(),
            tag,
            tracking: self.hover.and(self.tracking),
            ..Preview::default()
        }
    }
}

/// Ghost outlines as the geometry store gives them (`transform_outlines`,
/// `stretch_outlines`): dashed lines and point marks, as the web's
/// `strokePaths` draws the ghosts. Mimari denetim's fix preview draws its objects so too (docs/adr/0732 §5).
pub fn ghosts(paths: &[f64]) -> (Vec<Stroke>, Vec<Vec2>) {
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
        layer_id: None,
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
/// With `associative` the objects become one associative array of a block's
/// inserts (docs/adr/0300, 0302).
/// The array tools' İlişkili option (docs/adr/0302), its value.
pub(crate) fn associative_option(prompt: Prompt, m: &Memory) -> Prompt {
    prompt.option_with(
        "İlişkili",
        "İ",
        if m.array_associative {
            "evet"
        } else {
            "hayır"
        },
    )
}

/// İ turns the array tools' İlişkili on or off; whether `text` was it.
pub(crate) fn associative_typed(text: &str, cx: &mut Context<'_>) -> bool {
    if upper_tr(js_trim(text)) != "İ" {
        return false;
    }
    cx.memory.array_associative = !cx.memory.array_associative;
    true
}

pub(crate) fn array_selection(
    layout: ArrayLayout,
    associative: bool,
    cx: &mut Context<'_>,
) -> Option<usize> {
    let input = EntitiesArray {
        uids: selected_uids(cx),
        layout,
        associative,
        expected_revision: None,
    };
    let result = array::execute(&mut ExecutionContext::new(cx.doc), input);
    points::written(result, cx).map(|out: EntitiesArrayed| out.created.len())
}
