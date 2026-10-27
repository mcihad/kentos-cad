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
//! - with no button down, the object under the pointer is hovered;
//! - a press on a grip of a selected object moves that grip (docs/adr/0068):
//!   dragged, it goes where the button comes up; clicked, it is hot and the
//!   next click places it; a typed point places it too, Enter where the
//!   pointer is, Esc leaves it. Snaps, ortho and polar tracking apply from
//!   where the grip was. It writes through `cad.entities.edit`, operation
//!   `grip`, one undo step “Tutamaçla düzenle”: a layer locked while the grip
//!   waited is refused in the command's words, and an object gone meanwhile
//!   is said (the web's dd39864).
//!
//! Hidden layers' objects are never picked; locked layers' are, as on the
//! web (the erase tool leaves them in place), but their grips are not
//! taken. Ctrl does nothing here, as on the web.
//!
//! The selection filter (docs/adr/0187 §5) passes what it holds: the hover,
//! the click and the box. A click among several objects opens Sıradakini
//! seç's chip (§1): Shift+Boşluk or its list takes the next.
//!
//! With Topolojik düzenleme on (docs/adr/0160) a grip's edit puts the shared
//! corners and edges of the objects around it right too, in the same step
//! (`neighbours`): they are drawn dashed while the grip moves.

use kentos_commands::{EditOperation, EntitiesEdit, EntityEdit};
use kentos_domain::{Slot, Uuid};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::jsmath::js_hypot;
use kentos_geometry_core::ops::grips::mid_grip_segment;
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_native_application::geometry::{edit_geometry, shape};
use kentos_native_application::{ExecutionContext, edit};

use crate::Vec2;
use crate::edge::Outline;
use crate::elevation;
use crate::format::Format;
use crate::grip_edit::{GripMode, GripModes, HotGrip, Stage, Typed, stretched};
#[path = "select_opening.rs"]
mod opening;
#[path = "select_tag.rs"]
mod tag;
use crate::log::Level;
use crate::neighbours;
use crate::points;
use crate::prompt::Prompt;
use crate::tool::{Context, Pointer, Preview, Stroke, Tag, Tone};

/// How far the pointer must move with the button down to draw a box, logical pixels.
const DRAG_THRESHOLD: f64 = 4.0;
/// How near a grip the pointer must be to take it, logical pixels (the web's `gripAt`).
const GRIP_REACH: f64 = 6.0;
/// Past this many selected objects no grips are shown or taken (the web's limit).
pub const GRIP_LIMIT: usize = 150;

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
    /// The corners' world points, first and last (docs/adr/0277): the host draws them where the camera now shows them,
    /// so a pan or a zoom while the button is down keeps the box on the drawing.
    pub worlds: Option<(Vec2, Vec2)>,
}

impl SelectBox {
    /// Right to left: every object it touches (the web draws it dashed, in the snap colour).
    pub fn crossing(&self) -> bool {
        self.to[0] < self.from[0]
    }

    /// The box between two corners drawn as a crossing whichever way it
    /// went: Esnet's window takes the vertices it touches (the web draws it
    /// dashed in the snap colour, lightly filled).
    pub fn touching(a: [f64; 2], b: [f64; 2]) -> Self {
        Self {
            from: [a[0].max(b[0]), a[1]],
            to: [a[0].min(b[0]), b[1]],
            worlds: None,
        }
    }
}

/// A grip being moved (the web's `GripEdit`).
#[derive(Clone, Debug, PartialEq)]
struct GripEdit {
    slot: Slot,
    /// The object's persistent id: the command names it by that.
    uid: Option<Uuid>,
    index: usize,
    /// Where the grip was.
    origin: Vec2,
    /// Where the button went down on it.
    down: [f64; 2],
    /// Clicked without dragging: the next click places it.
    hot: bool,
    /// The elevation of the vertex the grip stands on, when it has one: its
    /// tag says it (docs/adr/0142). The vertex keeps it as it moves.
    z: Option<f64>,
    /// The other hot grips (Shift), moved with it in Esnet (docs/adr/0280).
    others: Vec<HotGrip>,
    /// The grip's mode, base point, reference and Kopya (docs/adr/0280).
    modes: GripModes,
}

/// The select tool's state between pointer events.
#[derive(Clone, Debug, Default)]
pub struct Select {
    start: Option<At>,
    current: Option<At>,
    dragging: bool,
    grip: Option<GripEdit>,
    /// Where the grip would go: the pointer, snapped, ortho and polar tracking applied.
    grip_point: Option<Vec2>,
    tracking: Option<Tracking>,
    /// The object as the grip would leave it, for the preview.
    moved: Option<Shape>,
    /// The neighbours as the grip would leave them (Topolojik düzenleme,
    /// docs/adr/0160), for the preview.
    follow: Vec<Shape>,
    /// The pointer rests on the grip of a vertex with something to say: its
    /// tag beside the pointer (docs/adr/0142, 0160).
    hover_grip: Option<GripTag>,
    /// Grips made hot with Shift, waiting for a click on one of them (docs/adr/0280).
    hot: Vec<HotGrip>,
    /// A dynamic block's parameter grip being moved (docs/adr/0319 §1).
    param: Option<crate::param_grip::ParamDrag>,
    /// A transform mode's ghosts of the selection, and their point marks.
    ghosts: Vec<Stroke>,
    marks: Vec<Vec2>,
    /// A transform mode's tag: the angle, the factor, the distance.
    mode_tag: Option<String>,
    /// The one selected opening's temporary dimensions (docs/adr/0701), and where the pointer last rested.
    temporary: crate::opening_distance::Overlay,
    pointer: Option<Vec2>,
}

/// What the tag of the grip the pointer rests on says: the vertex's
/// elevation (docs/adr/0142) and, with Topolojik düzenleme on, how many
/// objects share that corner (docs/adr/0160 §5).
#[derive(Clone, Copy, Debug, PartialEq)]
struct GripTag {
    /// Where the pointer is.
    at: Vec2,
    z: Option<f64>,
    shared: Option<usize>,
}

impl Select {
    pub fn new() -> Self {
        Self::default()
    }

    /// Grip `index` of `slot`, standing at `origin`, turns hot as if clicked: the next
    /// click places it (the grip menu's Esnet, docs/adr/0279).
    pub fn heat(&mut self, slot: Slot, index: usize, origin: Vec2, cx: &mut Context<'_>) {
        self.start = None;
        self.current = None;
        self.dragging = false;
        self.grip = Some(GripEdit {
            slot,
            uid: cx.doc.uid(slot),
            index,
            origin,
            down: cx.view.to_screen(origin),
            hot: true,
            z: grip_z(slot, index, cx),
            others: Vec::new(),
            modes: GripModes::with_texts(origin, cx.memory.mirror_text_mirrored),
        });
        self.grip_point = Some(origin);
        self.moved = None;
        cx.selection.set_hover(None);
    }

    /// The left button went down: a hot grip is placed, a grip is taken, or
    /// a click or a box starts here.
    pub fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover_grip = None;
        // A parameter's grip (docs/adr/0319 §1): a hot one is placed; one under the pointer is taken, a flip's turns.
        if self.param.as_ref().is_some_and(|g| g.hot) {
            if let Some(mut g) = self.param.take() {
                g.follow(p.world, cx);
                g.commit(cx);
            }
            return;
        }
        if self.grip.is_none()
            && !p.shift
            && let Some((slot, g)) = crate::param_grip::grip_at(p.screen, cx)
        {
            self.hot.clear();
            if g.kind == kentos_geometry_core::block_dynamic::GripKind::Flip {
                crate::param_grip::flip(slot, &g.name, cx);
            } else {
                self.param = Some(crate::param_grip::ParamDrag::start(slot, g, p.screen, cx));
                cx.selection.set_hover(None);
            }
            return;
        }
        if self.grip.as_ref().is_some_and(|g| g.hot) {
            let at = self.grip_point.unwrap_or(p.world);
            return self.commit(at, cx);
        }
        if let Some((slot, index, origin)) = grip_at(p.screen, cx) {
            // Shift: the grip joins the hot ones, or leaves them (docs/adr/0280).
            if p.shift {
                return self.toggle_hot(
                    HotGrip {
                        slot,
                        index,
                        origin,
                    },
                    cx,
                );
            }
            // A click on one of the hot grips starts the edit with the others; elsewhere they cool.
            let others: Vec<HotGrip> =
                if self.hot.iter().any(|h| h.slot == slot && h.index == index) {
                    self.hot
                        .iter()
                        .copied()
                        .filter(|h| !(h.slot == slot && h.index == index))
                        .collect()
                } else {
                    Vec::new()
                };
            self.hot.clear();
            self.grip = Some(GripEdit {
                slot,
                uid: cx.doc.uid(slot),
                index,
                origin,
                down: p.screen,
                hot: false,
                z: grip_z(slot, index, cx),
                others,
                modes: GripModes::with_texts(origin, cx.memory.mirror_text_mirrored),
            });
            self.grip_point = Some(origin);
            self.moved = None;
            cx.selection.set_hover(None);
            return;
        }
        self.hot.clear();
        let at = At {
            screen: p.screen,
            world: p.raw,
        };
        self.start = Some(at);
        self.current = Some(at);
        self.dragging = false;
    }

    /// The pointer moved: a grip follows it, the box grows, or the object
    /// under it is hovered.
    pub fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if let Some(g) = self.param.as_mut() {
            g.follow(p.world, cx);
            return;
        }
        if let Some(g) = &self.grip {
            let (point, tracking) = points::constrain(Some(g.modes.anchor()), p, cx);
            self.grip_point = Some(point);
            self.tracking = tracking;
            self.see_grip(point, cx);
            return;
        }
        let Some(start) = self.start else {
            // The selected opening's temporary dimensions on the face the pointer is by (docs/adr/0701).
            self.pointer = Some(p.raw);
            self.temporary.see(self.pointer, cx);
            // What a click would select: the selection filter's kinds (docs/adr/0187 §5).
            let hit = crate::selectable::hover(cx, p.raw);
            self.hover_grip = grip_tag_at(p, cx);
            // On a tagged grip the pointer is on the vertex, not on the object: it is not
            // hovered, and no rollover card opens over the tag (docs/adr/0142, 0160).
            cx.selection
                .set_hover(hit.filter(|_| self.hover_grip.is_none()));
            return;
        };
        self.hover_grip = None;
        // The box's first corner stays on the drawing through a pan or a zoom (docs/adr/0277).
        let start = At {
            screen: cx.view.to_screen(start.world),
            ..start
        };
        self.start = Some(start);
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

    /// The left button came up: a dragged grip is placed (a clicked one
    /// turns hot), the box selects, or the click picks.
    pub fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if let Some(g) = self.param.as_mut().filter(|g| !g.hot) {
            if js_hypot(p.screen[0] - g.down[0], p.screen[1] - g.down[1]) > DRAG_THRESHOLD {
                if let Some(mut g) = self.param.take() {
                    g.follow(p.world, cx);
                    g.commit(cx);
                }
            } else {
                g.hot = true;
            }
            return;
        }
        if let Some(g) = self.grip.as_mut().filter(|g| !g.hot) {
            if js_hypot(p.screen[0] - g.down[0], p.screen[1] - g.down[1]) > DRAG_THRESHOLD {
                let at = self.grip_point.unwrap_or(p.world);
                self.commit(at, cx);
            } else {
                g.hot = true;
            }
            return;
        }
        let Some(start) = self.start.take() else {
            return;
        };
        // The release is the last corner, where the view now shows it (docs/adr/0277).
        let current = self.current.take().map(|_| At {
            screen: p.screen,
            world: p.raw,
        });
        let dragging = std::mem::take(&mut self.dragging);
        match current {
            Some(current) if dragging => {
                // The box as it was drawn decides: one rule for the look and the query.
                let crossing = SelectBox {
                    from: cx.view.to_screen(start.world),
                    to: current.screen,
                    worlds: None,
                }
                .crossing();
                let (a, b) = (
                    crate::selectable::point(start.world),
                    crate::selectable::point(current.world),
                );
                let region =
                    match crate::selectable::twisted_ring(cx, start.world, current.world, crossing)
                    {
                        // A twisted view (docs/adr/0285 §2): the box drawn on the screen is a turned ring on the ground.
                        Some(ring) => ring,
                        None if crossing => kentos_commands::SelectRegion::Crossing { a, b },
                        None => kentos_commands::SelectRegion::Window { a, b },
                    };
                let ids = crate::selectable::query(cx, region);
                if p.shift {
                    cx.selection.add(ids);
                } else {
                    cx.selection.set(ids);
                }
            }
            _ => {
                // Every object the click could mean, the most specific first (docs/adr/0187 §1):
                // the first is selected; among several, Sıradakini seç's chip opens.
                let candidates = crate::selectable::candidates(cx, p.raw);
                // Ctrl: the sub-object the click falls on, each candidate's (docs/adr/0276).
                if p.ctrl && !p.shift {
                    pick_sub(p.raw, candidates, cx);
                    return;
                }
                match candidates.first().copied() {
                    Some(hit) if p.shift => {
                        cx.selection.toggle(hit);
                        if cx.selection.contains(hit) {
                            cx.selection.start_cycle(p.raw, candidates);
                        }
                    }
                    Some(hit) => {
                        cx.selection.set([hit]);
                        cx.selection.start_cycle(p.raw, candidates);
                    }
                    None if !p.shift => cx.selection.clear(),
                    None => {}
                }
            }
        }
    }

    /// The box being drawn, once the pointer has moved far enough.
    pub fn select_box(&self) -> Option<SelectBox> {
        match (self.dragging, self.start, self.current) {
            (true, Some(start), Some(current)) => Some(SelectBox {
                from: start.screen,
                to: current.screen,
                worlds: Some((start.world, current.world)),
            }),
            _ => None,
        }
    }

    /// Forgets a press in progress (a command started, a drawing opened).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The temporary dimensions brought up to date for the selection (docs/adr/0701).
    pub fn see_temporary(&mut self, cx: &Context<'_>) {
        if self.grip.is_none() && self.param.is_none() {
            self.temporary.see(self.pointer, cx);
        }
    }

    /// Where the temporary dimensions' numbers stand (docs/adr/0701).
    pub fn temporary_numbers(&self) -> Vec<Vec2> {
        self.temporary.sides.iter().map(|s| s.text_at).collect()
    }

    /// The text field of the temporary dimension whose number is under the pointer, to run (docs/adr/0701).
    pub fn temporary_at(&self, screen: [f64; 2], cx: &Context<'_>) -> Option<crate::opening_distance::OpeningDistance> {
        if self.grip.is_some() || self.param.is_some() || self.start.is_some() {
            return None;
        }
        self.temporary.at(screen, cx)
    }

    /// Whether a grip is being moved: the select tool then takes points,
    /// Enter and Esc, and snaps (the web's `snaps`).
    pub fn grip_active(&self) -> bool {
        self.grip.is_some() || self.param.is_some()
    }

    /// The grip being moved, drawn larger (the web's `activeGrip`).
    pub fn active_grip(&self) -> Option<(Slot, usize)> {
        self.grip.as_ref().map(|g| (g.slot, g.index))
    }

    /// Where perpendicular and tangent snaps are taken from: where the grip was.
    pub fn snap_from(&self) -> Option<Vec2> {
        self.grip.as_ref().map(|g| g.origin)
    }

    pub fn prompt(&self) -> Prompt {
        if let Some(g) = &self.param {
            return g.prompt();
        }
        match &self.grip {
            Some(g) => g.modes.prompt(),
            None => Prompt::idle(),
        }
    }

    /// A grip mode's letter or value (docs/adr/0280), else a typed point:
    /// the grip goes there, relative to the base point (`@dY,dX`).
    pub fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        // A parameter's grip takes its value, or a point (docs/adr/0319 §1).
        if let Some(mut g) = self.param.take() {
            if g.typed(text, cx) {
                g.commit(cx);
                return true;
            }
            self.param = Some(g);
            return false;
        }
        let Some(g) = self.grip.as_mut() else {
            return false;
        };
        let typed = g.modes.typed(text, &cx.format());
        cx.memory.mirror_text_mirrored = g.modes.text_mirrored;
        match typed {
            Some(Typed::Said(line)) => {
                g.hot = true;
                cx.say(Level::Info, line);
                return true;
            }
            Some(Typed::Write(t)) => {
                self.write_transform(t, cx);
                return true;
            }
            None => {}
        }
        let Some(p) = cx.typed_point(text, Some(g.modes.anchor()), self.grip_point) else {
            return false;
        };
        self.commit(p, cx);
        true
    }

    /// A point computed by the point calculator: the grip goes there (the
    /// web's `SelectTool.acceptPoint`). False when no grip moves.
    pub fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if let Some(mut g) = self.param.take() {
            g.follow(p, cx);
            g.commit(cx);
            return true;
        }
        if self.grip.is_none() {
            return false;
        }
        self.commit(p, cx);
        true
    }

    /// Enter, Space or a quick right click: the grip goes where the pointer
    /// is. False when no grip moves (the last command repeats then).
    pub fn confirm(&mut self, cx: &mut Context<'_>) -> bool {
        if let Some(g) = self.param.take() {
            g.commit(cx);
            return true;
        }
        let Some(g) = &self.grip else {
            return false;
        };
        let at = self.grip_point.unwrap_or(g.origin);
        self.commit(at, cx);
        true
    }

    /// Esc: the grip stays where it was. False when no grip moves.
    pub fn cancel(&mut self) -> bool {
        let moving = self.grip.is_some() || self.param.is_some() || !self.hot.is_empty();
        self.param = None;
        self.end_grip();
        self.hot.clear();
        moving
    }

    fn end_grip(&mut self) {
        self.grip = None;
        self.grip_point = None;
        self.tracking = None;
        self.moved = None;
        self.follow.clear();
        self.ghosts.clear();
        self.marks.clear();
        self.mode_tag = None;
    }

    /// The grip's point at `at`: a base point or a reference's point, or the
    /// mode's placement. Esnet writes the objects with their hot grips moved
    /// by the same distance through `cad.entities.edit`, operation `grip` (the
    /// step “Tutamaçla düzenle”): each object by its persistent id, its whole
    /// new geometry explicit (TODOS.md CMD-07), with Kopya as a new object.
    /// The other modes write the selection's transform. An object gone
    /// meanwhile, a shape it cannot take and the command's refusal are said;
    /// nothing changes then (the web's `commitGrip`). With Kopya the grip
    /// stays hot for the next placement.
    fn commit(&mut self, at: Vec2, cx: &mut Context<'_>) {
        let Some(g) = self.grip.as_mut() else {
            return;
        };
        if g.modes.take_point(at) {
            g.hot = true;
            return;
        }
        if g.modes.mode != GripMode::Stretch {
            let unit = cx.typed_length("1").unwrap_or(1.0);
            match g.modes.transform(at, g.origin, unit) {
                Some(t) => self.write_transform(t, cx),
                None => g.hot = true,
            }
            return;
        }
        let Some(g) = self.grip.take() else {
            return;
        };
        let (dx, dy) = (at.x - g.modes.base.x, at.y - g.modes.base.y);
        let copy = g.modes.copy;
        let keep = copy.then(|| g.clone());
        self.end_grip();
        if g.uid.and_then(|uid| cx.doc.slot_of(uid)).is_none() {
            cx.say(
                Level::Warn,
                "Tutamacın nesnesi artık çizimde yok (silinmiş ya da geri alınmış); tutamaç bırakıldı.",
            );
            return;
        }
        if js_hypot(dx, dy) <= 1e-9 {
            self.grip = keep.map(|g| GripEdit { hot: true, ..g });
            return;
        }
        let grips: Vec<HotGrip> = std::iter::once(HotGrip {
            slot: g.slot,
            index: g.index,
            origin: g.origin,
        })
        .chain(g.others.iter().copied())
        .collect();
        if self.commit_tag_grip(&grips, dx, dy, copy, cx)
            || self.commit_opening_grip(&grips, dx, dy, copy, cx) {
            if let Some(mut g) = keep {
                g.hot = true;
                g.modes.placed += 1;
                self.grip = Some(g);
            }
            return;
        }
        let moved = stretched(cx.doc, &grips, dx, dy);
        if moved.is_empty() {
            cx.say(
                Level::Warn,
                "Bu konum geçersiz bir şekil oluşturuyor; tutamaç yerinde bırakıldı.",
            );
            return;
        }
        let mut changes = Vec::with_capacity(moved.len());
        for (slot, _, after) in &moved {
            let (Some(uid), Some(geometry)) = (cx.doc.uid(*slot), edit_geometry(shape(after)))
            else {
                continue;
            };
            changes.push(if copy {
                EntityEdit::add(uid.to_string(), geometry, Some(true))
            } else {
                EntityEdit::Update {
                    uid: uid.to_string(),
                    geometry,
                }
            });
        }
        // The neighbours sharing what moved go in the same step (docs/adr/0160 §6); a copy has none.
        let edits: Vec<(Slot, Shape, Shape)> = moved
            .iter()
            .map(|(slot, before, after)| (*slot, shape(before), shape(after)))
            .collect();
        let follow = if copy {
            None
        } else {
            neighbours::neighbours_of(cx, &edits)
        };
        changes.extend(follow.iter().flat_map(|n| n.changes.iter().cloned()));
        let input = EntitiesEdit {
            operation: EditOperation::Grip,
            changes,
            expected_revision: None,
        };
        let result = edit::execute(&mut ExecutionContext::new(cx.doc), input);
        if points::written(result, cx).is_some() {
            neighbours::say(follow.as_ref(), cx);
            if copy {
                cx.say(
                    Level::Success,
                    format!("{} nesne kopyalandı (Esnet).", moved.len()),
                );
            }
        }
        if let Some(mut g) = keep {
            g.hot = true;
            g.modes.placed += 1;
            self.grip = Some(g);
        }
    }

    /// A transform mode's placement: the selection through `cad.entities.transform`, in place or as copies; with
    /// Kopya the grip stays hot, else the edit ends.
    fn write_transform(&mut self, t: kentos_commands::Transform, cx: &mut Context<'_>) {
        let Some(g) = self.grip.as_mut() else {
            return;
        };
        let copy = g.modes.copy;
        if let Some(n) = crate::modify::transform_selection(t.clone(), copy, cx) {
            let line = g.modes.said(&t, n);
            cx.say(Level::Success, line);
        }
        if copy {
            if let Some(g) = self.grip.as_mut() {
                g.hot = true;
                g.modes.placed += 1;
            }
        } else {
            self.end_grip();
        }
    }

    /// What the grip would do at `point`: Esnet's objects and their neighbours, a transform's ghosts and tag.
    fn see_grip(&mut self, point: Vec2, cx: &Context<'_>) {
        let Some(g) = &self.grip else {
            return;
        };
        self.moved = None;
        self.follow.clear();
        self.ghosts.clear();
        self.marks.clear();
        self.mode_tag = None;
        if g.modes.stage != Stage::Point {
            return;
        }
        if g.modes.mode == GripMode::Stretch {
            let (dx, dy) = (point.x - g.modes.base.x, point.y - g.modes.base.y);
            let grips: Vec<HotGrip> = std::iter::once(HotGrip {
                slot: g.slot,
                index: g.index,
                origin: g.origin,
            })
            .chain(g.others.iter().copied())
            .collect();
            // A mixed edit cannot be represented as one tag placement;
            // do not preview a partial move which commit will refuse.
            if tag::mixed(&grips, cx) {
                return;
            }
            if let Some(planned) = opening::preview(&grips, dx, dy, g.modes.copy, cx) {
                self.moved = planned.into_iter().next();
                return;
            }
            let moved = stretched(cx.doc, &grips, dx, dy);
            let edits: Vec<(Slot, Shape, Shape)> = moved
                .iter()
                .map(|(slot, before, after)| (*slot, shape(before), shape(after)))
                .collect();
            if !g.modes.copy {
                self.follow = neighbours::neighbours_of(cx, &edits)
                    .map(|n| n.shapes)
                    .unwrap_or_default();
            }
            // The first object as `moved`, the others with the neighbours: all alike, dashed.
            let mut shapes = edits.into_iter().map(|(_, _, after)| after);
            self.moved = shapes.next();
            self.follow.extend(shapes);
            return;
        }
        let unit = cx.typed_length("1").unwrap_or(1.0);
        let Some(affine) = g.modes.affine(point, g.origin, unit) else {
            return;
        };
        let ids: Vec<f64> = cx.selection.ids().iter().map(|s| f64::from(s.0)).collect();
        let paths =
            cx.spatial
                .store()
                .transform_outlines(&ids, &[affine], crate::modify::MAX_GHOSTS);
        (self.ghosts, self.marks) = crate::modify::ghosts(&paths);
        self.mode_tag = g.modes.tag(point, g.origin, unit, &cx.format());
    }

    /// Shift and a click on a grip: it joins the hot grips, or leaves them; the log says how many there are.
    fn toggle_hot(&mut self, grip: HotGrip, cx: &mut Context<'_>) {
        match self
            .hot
            .iter()
            .position(|h| h.slot == grip.slot && h.index == grip.index)
        {
            Some(i) => {
                self.hot.remove(i);
            }
            None => self.hot.push(grip),
        }
        let line = match self.hot.len() {
            0 => "Sıcak tutamaç kalmadı.".to_owned(),
            n => format!(
                "{n} tutamaç sıcak; düzenlemek için birine tıklayın (Shift ile ekleyip çıkarın)."
            ),
        };
        cx.say(Level::Info, line);
    }

    /// The hot grips: the one being moved, the others moved with it, those made hot with Shift.
    pub fn hot_grips(&self) -> Vec<(Slot, usize)> {
        let mut out: Vec<(Slot, usize)> = self.hot.iter().map(|h| (h.slot, h.index)).collect();
        if let Some(g) = &self.grip {
            out.push((g.slot, g.index));
            out.extend(g.others.iter().map(|h| (h.slot, h.index)));
        }
        out
    }

    /// While a grip moves: the object as it would be, dashed; a dashed line
    /// from where the grip was; its distance beside the pointer (the web's `draw`).
    pub fn preview(&self, format: &Format) -> Option<Preview> {
        if let Some(g) = &self.param {
            return Some(g.preview(format));
        }
        let (Some(g), Some(at)) = (self.grip.as_ref(), self.grip_point) else {
            // Resting on the grip of a vertex: its elevation and its shared corner (docs/adr/0142, 0160).
            let Some(tag) = self.hover_grip else {
                // The selected opening's temporary dimensions (docs/adr/0701).
                return (!self.temporary.sides.is_empty() && self.start.is_none())
                    .then(|| self.temporary.preview.clone());
            };
            let mut lines: Vec<String> = tag
                .z
                .map(|z| elevation_line(z, format))
                .into_iter()
                .collect();
            lines.extend(tag.shared.map(|n| format!("{n} nesnenin köşesi")));
            return Some(Preview {
                tag: Some(Tag { at: tag.at, lines }),
                ..Preview::default()
            });
        };
        let mut strokes = Vec::new();
        // The objects and the neighbours that follow them (docs/adr/0160 §5), alike.
        for shape in self.moved.iter().chain(&self.follow) {
            strokes.extend(Outline::of(shape, Some([4.0, 3.0]), 1.5, Tone::Accent).strokes);
        }
        // A transform mode's ghosts of the selection (docs/adr/0280).
        strokes.extend(self.ghosts.iter().cloned());
        let from = g.modes.anchor();
        strokes.push(Stroke::dashed(vec![from, at], false, [2.0, 3.0]));
        let lines = if g.modes.mode == GripMode::Stretch && g.modes.stage == Stage::Point {
            let mut lines = vec![format.length(dist(from, at))];
            lines.extend(g.z.map(|z| elevation_line(z, format)));
            lines
        } else {
            self.mode_tag.iter().cloned().collect()
        };
        Some(Preview {
            strokes,
            marks: self.marks.clone(),
            tag: (!lines.is_empty()).then_some(Tag { at, lines }),
            tracking: self.tracking,
            ..Preview::default()
        })
    }
}

/// The grip under the pointer of a selected object not on a locked layer,
/// the nearest within reach (the web's `gripAt`): the object, the grip's
/// index and where it is.
pub(crate) fn grip_at(screen: [f64; 2], cx: &Context<'_>) -> Option<(Slot, usize, Vec2)> {
    let ids = cx.selection.ids();
    if ids.len() > GRIP_LIMIT {
        return None;
    }
    let doc = &*cx.doc;
    let editable: Vec<Slot> = ids
        .iter()
        .copied()
        .filter(|&s| {
            doc.get(s)
                .is_some_and(|e| !doc.layers().is_locked(&e.base().layer_id))
        })
        .collect();
    let mut found = None;
    let mut best = GRIP_REACH;
    for set in cx.spatial.grips(&editable) {
        for (index, &p) in set.points.iter().enumerate() {
            if !set.shown(index, cx.view) {
                continue;
            }
            let s = cx.view.to_screen(p);
            let d = js_hypot(s[0] - screen[0], s[1] - screen[1]);
            if d <= best {
                best = d;
                found = Some((set.slot, index, p));
            }
        }
    }
    found
}

/// The tag of the grip of a vertex the pointer rests on: the vertex's
/// elevation (docs/adr/0142) and, with Topolojik düzenleme on, how many
/// objects share that corner (docs/adr/0160 §5); none when it says neither.
/// An edge's middle has none. Grips are looked for only while a selected
/// object has an elevation anywhere or the mode is on: a drawing without
/// either pays nothing for the tag on every pointer move.
fn grip_tag_at(p: &Pointer, cx: &Context<'_>) -> Option<GripTag> {
    let doc = &*cx.doc;
    let ids = cx.selection.ids();
    // A selection past the grips' limit has none to rest on, and is not looked through.
    if ids.len() > GRIP_LIMIT
        || (!cx.draft.topology
            && !ids
                .iter()
                .any(|slot| doc.get(*slot).is_some_and(elevation::has_any)))
    {
        return None;
    }
    let (slot, index, at) = grip_at(p.screen, cx)?;
    let e = doc.get(slot)?;
    if mid_grip_segment(&shape(e), index).is_some() {
        return None;
    }
    let z = grip_z(slot, index, cx);
    let shared = neighbours::corner_count(cx, slot, at);
    (z.is_some() || shared.is_some()).then_some(GripTag {
        at: p.world,
        z,
        shared,
    })
}

/// The elevation of the vertex the grip `index` of the object at `slot`
/// stands on, when it has one (docs/adr/0142).
fn grip_z(slot: Slot, index: usize, cx: &Context<'_>) -> Option<f64> {
    cx.doc
        .get(slot)
        .and_then(|e| elevation::grip_elevation(e, index))
}

/// What the grip's tag says of a vertex's elevation: `Kot 104.000 m`.
fn elevation_line(z: f64, format: &Format) -> String {
    format!("Kot {}", format.length(z))
}

/// A Ctrl click (docs/adr/0276): the objects it could mean, each with the sub-object it falls on
/// (`cad.entities.pick`'s rule); the first is selected and said with its id and place.
fn pick_sub(at: Vec2, candidates: Vec<Slot>, cx: &mut Context<'_>) {
    let tol = cx.pick_tolerance();
    let picks: Vec<(Slot, Option<crate::SubObject>)> = candidates
        .iter()
        .map(|s| {
            let sub = cx.doc.get(*s).and_then(|e| {
                kentos_native_application::entities_pick::sub_object(cx.doc, e, at, tol)
            });
            let sub = sub.map(|(path, p)| crate::SubObject {
                path,
                place: kentos_domain::dimension_ties::contract_place(p),
            });
            (*s, sub)
        })
        .collect();
    cx.selection.pick_sub(at, picks);
    if let Some(line) = sub_line(cx.doc, cx.selection) {
        cx.say(crate::log::Level::Info, line);
    }
}

/// The selected sub-object as the log says it: “Alt nesne: Kapalı alan 0192f5a0 · 3. köşe”.
pub fn sub_line(doc: &kentos_domain::Document, selection: &crate::Selection) -> Option<String> {
    let [slot] = selection.ids() else {
        return None;
    };
    let e = doc.get(*slot)?;
    let uid = doc.uid(*slot)?.to_string();
    let place = match selection.sub() {
        Some((_, sub)) => {
            let words = crate::dimension_reassociate::place_words(&sub.place);
            if sub.path.is_empty() {
                words
            } else {
                format!("{words} (blokta)")
            }
        }
        None => "bütün nesne".to_owned(),
    };
    Some(format!(
        "Alt nesne: {} {} · {place}",
        crate::data_search::kind_name(e),
        uid.get(..8).unwrap_or(&uid)
    ))
}
