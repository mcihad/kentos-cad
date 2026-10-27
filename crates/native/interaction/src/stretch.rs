//! Esnet: the web's `StretchTool` (`apps/web/src/tools/editTools.ts`), step
//! for step (docs/adr/0047):
//!
//! - a window over the vertices to move: two corners clicked, or one dragged
//!   past 4 px. It takes the objects it touches that have a vertex in it
//!   (with a selection, only the selected ones); those on locked layers are
//!   left out with a warning, and with none left it says so and asks for the
//!   window again;
//! - a base point, then the target: clicked (object snaps; ortho and polar
//!   tracking from the base) or typed (`@dY,dX` from the base).
//!
//! The product command `cad.entities.stretch` does the work (docs/adr/0254
//! §2): the window's objects ([`stretch::objects_in`]), the objects as they
//! would be (its plan, the preview), the write (one undo step, “Esnet”); the
//! tool says “2 nesne esnetildi: ΔY 5.000  ΔX 0.000” and leaves. The
//! preview is the web's: the window being drawn as a crossing box, then the
//! window dashed in the snap colour, the objects as they would be (dashed),
//! the line from the base with its length.
//!
//! With Topolojik düzenleme on, the objects sharing a moved vertex go with
//! the stretched ones in the same step, selected or not, and are previewed
//! with them (docs/adr/0160 §3; the command's `topology`).
//!
//! The objects tied to them (associative dimensions and hatches, linked
//! texts) are previewed as they will follow, finely dotted in the snap
//! colour (the plan's `dependents`, docs/adr/0297 §3); the document's copy
//! the plan follows them on is kept while the drawing stays as it is.

use kentos_commands::{CommandResult, EntitiesStretch, StretchTopology, StretchWindow, codes};
use kentos_domain::Slot;
use kentos_geometry_core::geometry::{Bounds, dist};
use kentos_geometry_core::jsmath::{js_hypot, js_max, js_min};
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, stretch};

use crate::Vec2;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::modify::{GHOST_DASH, MAX_GHOSTS};
use crate::points::{self, wire};
use crate::prompt::Prompt;
use crate::select::SelectBox;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tag, TextGhost, Tone, Tool};

/// The stretch tool's id: its command is `tool.stretch`.
pub const ID: &str = "stretch";
pub const LABEL: &str = "Esnet";

/// How far a drag must go to be the window, logical pixels.
const DRAG_THRESHOLD: f64 = 4.0;
/// The window's dash and gap once drawn, logical pixels (the web's `[5, 4]`).
const WINDOW_DASH: [f32; 2] = [5.0, 4.0];
/// What the tool waits for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    #[default]
    FirstCorner,
    SecondCorner,
    Base,
    Target,
}

/// The stretch tool's state between events.
#[derive(Clone, Debug, Default)]
pub struct Stretch {
    stage: Stage,
    /// The window's first corner: in the world (before snapping) and on the area.
    first: Option<(Vec2, [f64; 2])>,
    window: Option<Bounds>,
    /// How many objects the window stretches.
    targets: usize,
    base: Option<Vec2>,
    /// The effective cursor: snapped and constrained once the window is drawn.
    hover: Option<Vec2>,
    hover_screen: Option<[f64; 2]>,
    tracking: Option<Tracking>,
    /// The objects as they would be: outlines and point marks.
    ghosts: Vec<Stroke>,
    marks: Vec<Vec2>,
    texts: Vec<TextGhost>,
    /// A copy of the drawing the plan follows the tied objects on, kept between moves (docs/adr/0297 §3).
    scratch: Option<kentos_domain::Document>,
    done: bool,
}

impl Stretch {
    pub fn new() -> Self {
        Self::default()
    }

    fn placing(&self) -> bool {
        matches!(self.stage, Stage::Base | Stage::Target)
    }

    /// The effective cursor: ortho and polar tracking from the base (the web's `constrain`).
    fn constrain(&mut self, p: &Pointer, cx: &Context<'_>) -> Vec2 {
        let from = if self.stage == Stage::Target {
            self.base
        } else {
            None
        };
        let (point, tracking) = points::constrain(from, p, cx);
        self.tracking = tracking;
        point
    }

    /// The command's input for the window and a displacement: the selection
    /// restricts it, Topolojik düzenleme's state goes with it.
    fn request(window: &Bounds, dx: f64, dy: f64, cx: &Context<'_>) -> EntitiesStretch {
        let doc = &*cx.doc;
        let uids = (!cx.selection.is_empty()).then(|| {
            cx.selection
                .ids()
                .iter()
                .filter_map(|s| doc.uid(*s).map(|u| u.to_string()))
                .collect()
        });
        EntitiesStretch {
            window: StretchWindow {
                min: wire(Vec2::new(window.min_x, window.min_y)),
                max: wire(Vec2::new(window.max_x, window.max_y)),
            },
            dx,
            dy,
            uids,
            topology: cx.draft.topology.then_some(StretchTopology {
                points: cx.draft.topology_points,
            }),
            expected_revision: None,
        }
    }

    /// Says the command's warnings, but not a selected object's lying outside
    /// the window: the selection may be wider than the window.
    fn say_warnings(warnings: Vec<kentos_commands::CommandWarning>, cx: &mut Context<'_>) {
        for w in warnings {
            if w.code != codes::OUTSIDE_WINDOW {
                cx.say(Level::Warn, w.message);
            }
        }
    }

    /// The window from the first corner to `w` takes its objects (the web's `closeWindow`).
    fn close_window(&mut self, w: Vec2, cx: &mut Context<'_>) {
        let Some((a, _)) = self.first else {
            return;
        };
        let r = Bounds {
            min_x: js_min(a.x, w.x),
            min_y: js_min(a.y, w.y),
            max_x: js_max(a.x, w.x),
            max_y: js_max(a.y, w.y),
        };
        let input = Self::request(&r, 0.0, 0.0, cx);
        match stretch::objects_in(cx.doc, Some(cx.spatial.store()), &input) {
            CommandResult::Completed { output, warnings } => {
                Self::say_warnings(warnings, cx);
                self.targets = output.len();
                self.window = Some(r);
                self.stage = Stage::Base;
            }
            other => {
                if let CommandResult::Failed { error } = other {
                    if error.code == codes::NOTHING_TO_STRETCH {
                        // The objects its refusal names were left out for their locked layer.
                        if !error.uids.is_empty() {
                            let line = format!(
                                "{} nesne kilitli katmanda olduğu için atlandı.",
                                error.uids.len()
                            );
                            cx.say(Level::Warn, line);
                        }
                        cx.say(
                            Level::Warn,
                            "Pencerede köşesi olan düzenlenebilir nesne yok; yeniden deneyin.",
                        );
                    } else {
                        cx.say(Level::Warn, error.message);
                    }
                }
                self.stage = Stage::FirstCorner;
                self.first = None;
            }
        }
    }

    /// A point clicked or typed: the base, then the target, which writes.
    fn point(&mut self, p: Vec2, cx: &mut Context<'_>) {
        if self.stage == Stage::Base {
            self.base = Some(p);
            self.stage = Stage::Target;
            return;
        }
        let (Stage::Target, Some(base), Some(window)) = (self.stage, self.base, self.window) else {
            return;
        };
        let (dx, dy) = (p.x - base.x, p.y - base.y);
        let input = Self::request(&window, dx, dy, cx);
        let result = {
            let mut ecx = ExecutionContext::with_store(cx.doc, cx.spatial.store());
            stretch::execute(&mut ecx, input)
        };
        let (result, warnings) = match result {
            CommandResult::Completed { output, warnings } => (
                CommandResult::Completed {
                    output,
                    warnings: Vec::new(),
                },
                warnings,
            ),
            other => (other, Vec::new()),
        };
        if let Some(out) = points::written(result, cx) {
            let f = cx.format();
            let line = format!(
                "{} nesne esnetildi: ΔY {}  ΔX {}",
                out.stretched.len(),
                f.length_bare(dx),
                f.length_bare(dy)
            );
            cx.say(Level::Success, line);
            if !out.dependents.is_empty() {
                cx.say(
                    Level::Info,
                    format!(
                        "{} bağlı nesne (ölçü, tarama ya da yazı) nesnesini izledi.",
                        out.dependents.len()
                    ),
                );
            }
            if !out.followed.is_empty() {
                cx.say(
                    Level::Info,
                    format!(
                        "Topolojik düzenleme: {} komşu nesne de değişti.",
                        out.followed.len()
                    ),
                );
            }
            Self::say_warnings(warnings, cx);
        }
        self.done = true;
        self.scratch = None;
    }

    /// The objects as the cursor would stretch them: the command's plan.
    fn refresh(&mut self, cx: &Context<'_>) {
        self.ghosts.clear();
        self.marks.clear();
        self.texts.clear();
        let Some(window) = self.window.filter(|_| !self.done) else {
            return;
        };
        let (dx, dy) = match (self.base, self.hover) {
            (Some(base), Some(hover)) => (hover.x - base.x, hover.y - base.y),
            _ => (0.0, 0.0),
        };
        if dx == 0.0 && dy == 0.0 {
            // Where they are: the window's objects as they stand.
            let input = Self::request(&window, 0.0, 0.0, cx);
            if let CommandResult::Completed { output, .. } =
                stretch::objects_in(cx.doc, Some(cx.spatial.store()), &input)
            {
                let doc = &*cx.doc;
                for uid in output.iter().take(MAX_GHOSTS) {
                    let Some(e) = kentos_domain::Uuid::parse_str(uid)
                        .ok()
                        .and_then(|u| doc.slot_of(u))
                        .and_then(|s: Slot| doc.get(s))
                    else {
                        continue;
                    };
                    let outline = Outline::of(&shape(e), Some(GHOST_DASH), 1.0, Tone::Accent);
                    self.ghosts.extend(outline.strokes);
                    self.marks.extend(outline.marks);
                }
            }
            return;
        }
        let input = Self::request(&window, dx, dy, cx);
        let CommandResult::Completed { output, .. } =
            stretch::plan_with(cx.doc, Some(cx.spatial.store()), &input, &mut self.scratch)
        else {
            return;
        };
        for e in output
            .stretched
            .iter()
            .chain(&output.followed)
            .take(MAX_GHOSTS)
        {
            let outline = Outline::of(&shape(e), Some(GHOST_DASH), 1.0, Tone::Accent);
            self.ghosts.extend(outline.strokes);
            self.marks.extend(outline.marks);
        }
        let followers = kentos_commands::DerivedEntitiesPlan {
            added: output.added,
            updated: output.dependents,
            removed: output.removed,
        };
        let mut preview = Preview::default();
        crate::wall_dimensions::append_followers(&mut preview, &followers, cx);
        self.ghosts.extend(preview.strokes);
        self.texts.extend(preview.texts);
    }
}

impl Tool for Stretch {
    /// A computed point for the base or the target (the web's `acceptPoint`).
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if !self.placing() {
            return false;
        }
        self.point(p, cx);
        self.refresh(cx);
        true
    }
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        match self.stage {
            Stage::FirstCorner => Prompt::new(
                LABEL,
                "taşınacak köşeleri içine alan pencerenin ilk köşesini belirtin",
            ),
            Stage::SecondCorner => Prompt::new(LABEL, "pencerenin karşı köşesini belirtin"),
            Stage::Base => Prompt::new(
                LABEL,
                format!("{} nesne için temel noktayı belirtin", self.targets),
            ),
            Stage::Target => Prompt::new(LABEL, "hedef noktayı belirtin ya da @dY,dX yazın"),
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    /// Object snaps apply to the base and the target, not the window's corners.
    fn snaps(&self) -> bool {
        self.placing()
    }

    fn snap_from(&self) -> Option<Vec2> {
        (self.stage == Stage::Target).then_some(self.base).flatten()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover_screen = Some(p.screen);
        self.hover = Some(if self.placing() {
            self.constrain(p, cx)
        } else {
            p.raw
        });
        self.refresh(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match self.stage {
            Stage::FirstCorner => {
                self.first = Some((p.raw, p.screen));
                self.stage = Stage::SecondCorner;
            }
            Stage::SecondCorner => self.close_window(p.raw, cx),
            Stage::Base | Stage::Target => {
                let point = self.constrain(p, cx);
                self.point(point, cx);
            }
        }
        self.refresh(cx);
    }

    /// Dragging the window works as two clicks.
    fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some((_, from)) = self.first.filter(|_| self.stage == Stage::SecondCorner) else {
            return;
        };
        if js_hypot(p.screen[0] - from[0], p.screen[1] - from[1]) > DRAG_THRESHOLD {
            self.close_window(p.raw, cx);
            self.refresh(cx);
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if !self.placing() {
            return false;
        }
        let from = if self.stage == Stage::Target {
            self.base
        } else {
            None
        };
        let Some(p) = cx.typed_point(text, from, self.hover) else {
            return false;
        };
        self.point(p, cx);
        self.refresh(cx);
        true
    }

    /// Enter, Space or a quick right click leaves, whatever the step (the web's `confirm`).
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// Ctrl+Z undoes the drawing: the web's stretch takes no step back.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn finished(&self) -> bool {
        self.done
    }

    /// The window being drawn, as a crossing box whichever way it goes.
    fn select_box(&self) -> Option<SelectBox> {
        let (_, from) = self.first.filter(|_| self.stage == Stage::SecondCorner)?;
        Some(SelectBox::touching(from, self.hover_screen?))
    }

    fn preview(&self, format: &Format) -> Preview {
        let Some(w) = self.window else {
            return Preview::default();
        };
        let corners = vec![
            Vec2::new(w.min_x, w.min_y),
            Vec2::new(w.max_x, w.min_y),
            Vec2::new(w.max_x, w.max_y),
            Vec2::new(w.min_x, w.max_y),
        ];
        let mut strokes = vec![Stroke::dashed(corners, true, WINDOW_DASH).tone(Tone::Snap)];
        strokes.extend(self.ghosts.iter().cloned());
        let mut preview = Preview {
            marks: self.marks.clone(),
            texts: self.texts.clone(),
            ..Preview::default()
        };
        if let (Some(base), Some(hover)) = (self.base, self.hover) {
            strokes.push(Stroke::solid(vec![base, hover], false));
            preview.tag = Some(Tag {
                at: hover,
                lines: vec![format.length(dist(base, hover))],
            });
            preview.tracking = self.tracking;
        }
        preview.strokes = strokes;
        preview
    }
}
