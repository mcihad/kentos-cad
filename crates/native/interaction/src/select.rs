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

use kentos_contracts::{EditOperation, EntitiesEdit, EntityEdit};
use kentos_domain::{Slot, Uuid};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::jsmath::js_hypot;
use kentos_geometry_core::ops::grips::{mid_grip_segment, move_grip};
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_native_application::geometry::{edit_geometry, shape, with_shape};
use kentos_native_application::{ExecutionContext, edit};

use crate::Vec2;
use crate::edge::Outline;
use crate::elevation;
use crate::format::Format;
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

    /// The left button went down: a hot grip is placed, a grip is taken, or
    /// a click or a box starts here.
    pub fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover_grip = None;
        if self.grip.as_ref().is_some_and(|g| g.hot) {
            let at = self.grip_point.unwrap_or(p.world);
            return self.commit(at, cx);
        }
        if let Some((slot, index, origin)) = grip_at(p.screen, cx) {
            self.grip = Some(GripEdit {
                slot,
                uid: cx.doc.uid(slot),
                index,
                origin,
                down: p.screen,
                hot: false,
                z: grip_z(slot, index, cx),
            });
            self.grip_point = Some(origin);
            self.moved = None;
            cx.selection.set_hover(None);
            return;
        }
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
        if let Some(g) = &self.grip {
            let (point, tracking) = points::constrain(Some(g.origin), p, cx);
            self.grip_point = Some(point);
            self.tracking = tracking;
            let before = cx.doc.get(g.slot).map(shape);
            self.moved = before
                .as_ref()
                .and_then(|b| move_grip(&CoreEntity::new(b.clone()), g.index, point))
                .map(|e| e.shape);
            self.follow = match (&before, &self.moved) {
                (Some(before), Some(moved)) => neighbours::neighbours(cx, g.slot, before, moved)
                    .map(|n| n.shapes)
                    .unwrap_or_default(),
                _ => Vec::new(),
            };
            return;
        }
        let Some(start) = self.start else {
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
                let ids = crate::selectable::ids(cx, ids);
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
            }),
            _ => None,
        }
    }

    /// Forgets a press in progress (a command started, a drawing opened).
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Whether a grip is being moved: the select tool then takes points,
    /// Enter and Esc, and snaps (the web's `snaps`).
    pub fn grip_active(&self) -> bool {
        self.grip.is_some()
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
        if self.grip.is_some() {
            Prompt::new("Tutamaç", "yeni konumu belirtin ya da koordinat yazın (Esc: vazgeç)")
        } else {
            Prompt::idle()
        }
    }

    /// A typed point places the grip, relative to where it was (`@dY,dX`).
    pub fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let Some(g) = &self.grip else {
            return false;
        };
        let Some(p) = cx.typed_point(text, Some(g.origin), self.grip_point) else {
            return false;
        };
        self.commit(p, cx);
        true
    }

    /// A point computed by the point calculator: the grip goes there (the
    /// web's `SelectTool.acceptPoint`). False when no grip moves.
    pub fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        if self.grip.is_none() {
            return false;
        }
        self.commit(p, cx);
        true
    }

    /// Enter, Space or a quick right click: the grip goes where the pointer
    /// is. False when no grip moves (the last command repeats then).
    pub fn confirm(&mut self, cx: &mut Context<'_>) -> bool {
        let Some(g) = &self.grip else {
            return false;
        };
        let at = self.grip_point.unwrap_or(g.origin);
        self.commit(at, cx);
        true
    }

    /// Esc: the grip stays where it was. False when no grip moves.
    pub fn cancel(&mut self) -> bool {
        let moving = self.grip.is_some();
        self.end_grip();
        moving
    }

    fn end_grip(&mut self) {
        self.grip = None;
        self.grip_point = None;
        self.tracking = None;
        self.moved = None;
        self.follow.clear();
    }

    /// The object with its grip at `at`, written through `cad.entities.edit`,
    /// operation `grip` (the step “Tutamaçla düzenle”): the object by its
    /// persistent id, its whole new geometry explicit (TODOS.md CMD-07). An
    /// object gone meanwhile, a shape it cannot take and the command's
    /// refusal are said; nothing changes then (the web's `commitGrip`).
    fn commit(&mut self, at: Vec2, cx: &mut Context<'_>) {
        let Some(g) = self.grip.take() else {
            return;
        };
        self.end_grip();
        let found = g
            .uid
            .and_then(|uid| Some((uid, cx.doc.slot_of(uid)?)))
            .and_then(|(uid, slot)| Some((uid, slot, cx.doc.get(slot)?.clone())));
        let Some((uid, slot, e)) = found else {
            cx.say(
                Level::Warn,
                "Tutamacın nesnesi artık çizimde yok (silinmiş ya da geri alınmış); tutamaç bırakıldı.",
            );
            return;
        };
        let moved = move_grip(&CoreEntity::new(shape(&e)), g.index, at)
            .and_then(|m| with_shape(&e, m.shape));
        match moved {
            None => cx.say(
                Level::Warn,
                "Bu konum geçersiz bir şekil oluşturuyor; tutamaç yerinde bırakıldı.",
            ),
            Some(m) if dist(g.origin, at) > 1e-9 => {
                let Some(geometry) = edit_geometry(shape(&m)) else {
                    return;
                };
                // The neighbours sharing what moved go in the same step (docs/adr/0160 §6).
                let follow = neighbours::neighbours(cx, slot, &shape(&e), &shape(&m));
                let mut changes = vec![EntityEdit::Update {
                    uid: uid.to_string(),
                    geometry,
                }];
                changes.extend(follow.iter().flat_map(|n| n.changes.iter().cloned()));
                let input = EntitiesEdit {
                    operation: EditOperation::Grip,
                    changes,
                    expected_revision: None,
                };
                let result = edit::execute(&mut ExecutionContext::new(cx.doc), input);
                if points::written(result, cx).is_some() {
                    neighbours::say(follow.as_ref(), cx);
                }
            }
            Some(_) => {}
        }
    }

    /// While a grip moves: the object as it would be, dashed; a dashed line
    /// from where the grip was; its distance beside the pointer (the web's `draw`).
    pub fn preview(&self, format: &Format) -> Option<Preview> {
        let (Some(g), Some(at)) = (self.grip.as_ref(), self.grip_point) else {
            // Resting on the grip of a vertex: its elevation and its shared corner (docs/adr/0142, 0160).
            let tag = self.hover_grip?;
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
        // The object and the neighbours that follow it (docs/adr/0160 §5), alike.
        for shape in self.moved.iter().chain(&self.follow) {
            strokes.extend(Outline::of(shape, Some([4.0, 3.0]), 1.5, Tone::Accent).strokes);
        }
        strokes.push(Stroke::dashed(vec![g.origin, at], false, [2.0, 3.0]));
        let mut lines = vec![format.length(dist(g.origin, at))];
        lines.extend(g.z.map(|z| elevation_line(z, format)));
        Some(Preview {
            strokes,
            tag: Some(Tag { at, lines }),
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
