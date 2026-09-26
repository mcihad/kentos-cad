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
//! The shared core stretches each object (`stretch_entity`: the vertices in
//! the window move, the rest stay; an arc keeps its bow). Written through
//! the product command `cad.entities.edit` (updates, “stretch”), one undo
//! step, “Esnet”; says “2 nesne esnetildi: ΔY 5.000  ΔX 0.000” and leaves.
//! The preview is the web's: the window being drawn as a crossing box, then
//! the window dashed in the snap colour, the objects as they would be
//! (dashed, the geometry store's `stretch_outlines`), the line from the base
//! with its length.

use kentos_contracts::{EditOperation, EntityEdit};
use kentos_domain::Slot;
use kentos_geometry_core::geometry::{Bounds, dist};
use kentos_geometry_core::jsmath::{js_hypot, js_max, js_min};
use kentos_geometry_core::ops::stretch::stretch_entity;
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_geometry_core::tools::point_text::point_from_text;

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::modify::{MAX_GHOSTS, ghosts};
use crate::points;
use crate::prompt::Prompt;
use crate::select::SelectBox;
use crate::tool::{Context, Flow, Pointer, Preview, Stroke, Tag, Tone, Tool};

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
    /// The objects the window stretches.
    targets: Vec<Slot>,
    base: Option<Vec2>,
    /// The effective cursor: snapped and constrained once the window is drawn.
    hover: Option<Vec2>,
    hover_screen: Option<[f64; 2]>,
    tracking: Option<Tracking>,
    /// The objects as they would be: outlines and point marks.
    ghosts: Vec<Stroke>,
    marks: Vec<Vec2>,
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
        let restrict = !cx.selection.is_empty();
        let doc = &*cx.doc;
        let all: Vec<Slot> = cx
            .spatial
            .in_rect(a, w, true)
            .into_iter()
            .filter(|slot| !restrict || cx.selection.contains(*slot))
            .filter(|slot| {
                doc.get(*slot)
                    .is_some_and(|e| stretch_entity(&edge::core(e), &r, 0.0, 0.0).is_some())
            })
            .collect();
        let targets: Vec<Slot> = all
            .iter()
            .copied()
            .filter(|slot| doc.get(*slot).is_some_and(|e| edge::unlocked(e, doc)))
            .collect();
        if targets.len() < all.len() {
            let line = format!(
                "{} nesne kilitli katmanda olduğu için atlandı.",
                all.len() - targets.len()
            );
            cx.say(Level::Warn, line);
        }
        if targets.is_empty() {
            cx.say(
                Level::Warn,
                "Pencerede köşesi olan düzenlenebilir nesne yok; yeniden deneyin.",
            );
            self.stage = Stage::FirstCorner;
            self.first = None;
            return;
        }
        self.targets = targets;
        self.window = Some(r);
        self.stage = Stage::Base;
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
        // Each object's new geometry from the core, written as one edit: slot,
        // persistent id and every other field kept.
        let changes: Vec<EntityEdit> = self
            .targets
            .iter()
            .filter_map(|slot| {
                let e = cx.doc.get(*slot)?;
                let g = stretch_entity(&edge::core(e), &window, dx, dy)?;
                Some(EntityEdit::Update {
                    uid: edge::uid(cx.doc, *slot),
                    geometry: edge::geometry(&g.shape)?,
                })
            })
            .collect();
        let written = if changes.is_empty() {
            Some(0)
        } else {
            edge::write(EditOperation::Stretch, changes, cx).map(|out| out.changed.len())
        };
        if let Some(n) = written {
            let f = cx.format();
            let line = format!(
                "{n} nesne esnetildi: ΔY {}  ΔX {}",
                f.length_bare(dx),
                f.length_bare(dy)
            );
            cx.say(Level::Success, line);
        }
        self.done = true;
    }

    /// The objects as the cursor would stretch them, from the geometry store.
    fn refresh(&mut self, cx: &Context<'_>) {
        self.ghosts.clear();
        self.marks.clear();
        let Some(window) = self.window.filter(|_| !self.done) else {
            return;
        };
        let (dx, dy) = match (self.base, self.hover) {
            (Some(base), Some(hover)) => (hover.x - base.x, hover.y - base.y),
            _ => (0.0, 0.0),
        };
        let ids: Vec<f64> = self
            .targets
            .iter()
            .take(MAX_GHOSTS)
            .map(|slot| f64::from(slot.0))
            .collect();
        let paths = cx.spatial.store().stretch_outlines(&ids, &window, dx, dy);
        (self.ghosts, self.marks) = ghosts(&paths);
    }
}

impl Tool for Stretch {
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
                format!("{} nesne için temel noktayı belirtin", self.targets.len()),
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
        let Some(p) = point_from_text(text, from, self.hover, |_| None) else {
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
