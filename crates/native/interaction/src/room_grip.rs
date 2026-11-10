//! Odayı büyüt (docs/adr/8023; ADV-11 "Odayı büyütme"): a selected room shows a grip on each of its straight edges
//! that stands on a wall's face, a pair of arrowheads square to the edge at its middle. Dragged, or clicked and placed
//! with the next click, the wall behind the edge follows the pointer square to itself: Duvarı kaydır's plan
//! (`cad.walls.edit`, `shift`; docs/adr/0790 §1) with its bonds, what is joined to the wall following and the rooms'
//! boundaries found again from the walls. The card says the room's inner size across that edge, its area and the other
//! rooms' (Floorplanner's distance to the opposite wall, Chief Architect's room sizes). A typed number is the room's
//! new inner size across; with + or − before it, by how much it grows or shrinks. One undo step.
//!
//! A room whose boundary was drawn (no seed) does not follow its walls: it has no such grips, nor does an edge on a
//! locked wall, an arc or a room separator.

use kentos_commands::{CommandResult, Entity, WallsEdit, WallsOperation};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::geom::wall::left_face;
use kentos_geometry_core::jsmath::js_hypot;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, wall_distances, walls_edit};

use crate::Vec2;
use crate::bond_refusal::Refused;
use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::Prompt;
use crate::spatial::Spatial;
use crate::tool::{Context, Preview, Stroke, Tag, Tone};

pub const LABEL: &str = "Odayı büyüt";
/// How near a grip the pointer reaches it, pixels (the other grips').
const REACH: f64 = 6.0;
/// A room's edge on a wall's face: within a tenth of a millimetre (its boundary was found on the faces).
const ON_FACE: f64 = 1e-4;
/// Two directions this near are parallel: the sine of their angle.
const PARALLEL: f64 = 1e-6;

/// A room's edge grip: the room, the wall edge behind it, where the grip stands (the edge's middle), the edge's ends
/// and the direction away from the room (the arrowheads').
#[derive(Clone, Debug, PartialEq)]
pub struct RoomGrip {
    pub space: Slot,
    pub wall: Slot,
    pub wall_uid: Uuid,
    pub edge: u32,
    pub at: Vec2,
    pub a: Vec2,
    pub b: Vec2,
    pub outward: Vec2,
    /// The room is on the wall edge's left: the edge moves out of the room to its right.
    room_left: bool,
}

fn v(p: kentos_commands::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

/// The selected rooms' edge grips, on unlocked layers; none past the grips' limit.
pub fn grips(doc: &Document, spatial: &Spatial, ids: &[Slot]) -> Vec<RoomGrip> {
    if ids.len() > crate::select::GRIP_LIMIT {
        return Vec::new();
    }
    let mut out = Vec::new();
    for &slot in ids {
        let Some(Entity::Space(room)) = doc.get(slot) else {
            continue;
        };
        if room.space.seed.is_none() || doc.layers().is_locked(&room.base.layer_id) {
            continue;
        }
        for part in &room.space.parts {
            let pts: Vec<Vec2> = part.outer.pts.iter().map(|p| v(*p)).collect();
            let n = pts.len();
            if n < 3 {
                continue;
            }
            // Its inside is on the left of its edges when it runs counter-clockwise.
            let twice: f64 = (0..n)
                .map(|i| pts[i].x * pts[(i + 1) % n].y - pts[(i + 1) % n].x * pts[i].y)
                .sum();
            for k in 0..n {
                let bulge = part
                    .outer
                    .bulges
                    .as_ref()
                    .and_then(|b| b.get(k))
                    .copied()
                    .unwrap_or(0.0);
                if bulge != 0.0 {
                    continue;
                }
                let (a, b) = (pts[k], pts[(k + 1) % n]);
                let l = js_hypot(b.x - a.x, b.y - a.y);
                if l <= 1e-6 {
                    continue;
                }
                let u = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
                let outward = if twice > 0.0 {
                    Vec2::new(u.y, -u.x)
                } else {
                    Vec2::new(-u.y, u.x)
                };
                let at = Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
                if let Some((wall, wall_uid, edge, room_left)) =
                    facing(doc, spatial, &room.space.arch.level, at, u, outward)
                {
                    out.push(RoomGrip {
                        space: slot,
                        wall,
                        wall_uid,
                        edge,
                        at,
                        a,
                        b,
                        outward,
                        room_left,
                    });
                }
            }
        }
    }
    out
}

/// The wall edge whose face the room's edge through `at` (along `u`, the room's outside toward `outward`) stands on:
/// a straight edge of an unlocked wall on the room's level, parallel to it, the face toward the room on it.
fn facing(
    doc: &Document,
    spatial: &Spatial,
    level: &Option<String>,
    at: Vec2,
    u: Vec2,
    outward: Vec2,
) -> Option<(Slot, Uuid, u32, bool)> {
    for slot in spatial.hits(at, 0.05) {
        let Some(e @ Entity::Wall(w)) = doc.get(slot) else {
            continue;
        };
        if &w.wall.arch.level != level || doc.layers().is_locked(&w.base.layer_id) {
            continue;
        }
        let kentos_geometry_core::entity::Shape::Wall {
            pts,
            bulges,
            wall: spec,
        } = shape(e)
        else {
            continue;
        };
        let n = pts.len();
        let count = if spec.closed { n } else { n.saturating_sub(1) };
        for k in 0..count {
            if bulges
                .as_ref()
                .and_then(|b| b.get(k))
                .copied()
                .unwrap_or(0.0)
                != 0.0
            {
                continue;
            }
            let (a, b) = (pts[k], pts[(k + 1) % n]);
            let l = js_hypot(b.x - a.x, b.y - a.y);
            if l <= 0.0 {
                continue;
            }
            let along = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
            if (u.x * along.y - u.y * along.x).abs() > PARALLEL {
                continue;
            }
            let left = Vec2::new(-along.y, along.x);
            let room_left = dot(outward, left) < 0.0;
            let off = Vec2::new(at.x - a.x, at.y - a.y);
            let face = if room_left {
                left_face(&spec)
            } else {
                left_face(&spec) - spec.width
            };
            let t = dot(off, along);
            if (dot(off, left) - face).abs() <= ON_FACE && t >= -spec.width && t <= l + spec.width {
                return Some((slot, doc.uid(slot)?, k as u32, room_left));
            }
        }
    }
    None
}

/// The room edge grip under the pointer, the nearest within reach.
pub(crate) fn grip_at(screen: [f64; 2], cx: &Context<'_>) -> Option<RoomGrip> {
    let mut found = None;
    let mut best = REACH;
    for g in grips(cx.doc, cx.spatial, cx.selection.ids()) {
        let s = cx.view.to_screen(g.at);
        let d = js_hypot(s[0] - screen[0], s[1] - screen[1]);
        if d <= best {
            best = d;
            found = Some(g);
        }
    }
    found
}

/// The room's inner size across the grip's edge now: the clear distance from the face to the wall facing it.
fn across(g: &RoomGrip, doc: &Document) -> Option<f64> {
    let gaps = wall_distances::gaps_of(doc, g.wall, Some(g.edge)).ok()?;
    if g.room_left { gaps.left } else { gaps.right }.map(|gap| gap.distance)
}

/// What the pointer resting on a grip is told.
pub(crate) fn hover_lines(g: &RoomGrip, cx: &Context<'_>) -> Vec<String> {
    let mut lines = vec![format!("{LABEL}: kenarı sürükleyin")];
    if let Some(d) = across(g, cx.doc) {
        lines.push(format!("İç ölçü {}", cx.format().length(d)));
    }
    lines
}

/// A room's name for a line: “Mahal 1 Salon”, “Mahal”.
fn room_name(doc: &Document, slot: Slot) -> String {
    let Some(Entity::Space(room)) = doc.get(slot) else {
        return "Mahal".to_owned();
    };
    let words: Vec<&str> = [room.space.number.as_deref(), room.space.name.as_deref()]
        .into_iter()
        .flatten()
        .filter(|t| !t.is_empty())
        .collect();
    if words.is_empty() {
        "Mahal".to_owned()
    } else {
        format!("Mahal {}", words.join(" "))
    }
}

fn room_area(doc: &Document, slot: Slot) -> Option<f64> {
    match doc.arch_value(doc.get(slot)?, "area", None) {
        Some(kentos_domain::arch_values::ArchValue::Area(a)) => Some(a),
        _ => None,
    }
}

/// A signed amount, its sign always shown.
fn signed(text: String, v: f64) -> String {
    if v > 0.0 { format!("+{text}") } else { text }
}

/// A room edge grip being moved.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RoomDrag {
    pub grip: RoomGrip,
    space_uid: Option<Uuid>,
    /// Where the button went down on it.
    pub down: [f64; 2],
    /// Clicked without dragging: the next click places it.
    pub hot: bool,
    /// The room's inner size across the edge before, when a wall faces it there.
    across: Option<f64>,
    /// Where the pointer is, and how far out of the room the edge goes there (negative: into it).
    at: Vec2,
    out: f64,
    /// The walls as the move leaves them, the card, the bonds' refusal, or another refusal (the card in danger).
    walls: Vec<Entity>,
    lines: Vec<String>,
    refused: Option<Refused>,
    wrong: bool,
}

impl RoomDrag {
    pub fn start(grip: RoomGrip, down: [f64; 2], cx: &Context<'_>) -> Self {
        RoomDrag {
            space_uid: cx.doc.uid(grip.space),
            across: across(&grip, cx.doc),
            at: grip.at,
            grip,
            down,
            hot: false,
            out: 0.0,
            walls: Vec::new(),
            lines: Vec::new(),
            refused: None,
            wrong: false,
        }
    }

    /// The shift of the wall edge that moves the room's edge `out` (to its left positive).
    fn request(&self, out: f64) -> WallsEdit {
        WallsEdit {
            uids: vec![self.grip.wall_uid.to_string()],
            expected_revision: None,
            operation: WallsOperation::Shift {
                edge: Some(self.grip.edge),
                by: if self.grip.room_left { -out } else { out },
            },
        }
    }

    /// The grip at `at`: the edge goes out of the room by the pointer's distance square to it, planned.
    pub fn follow(&mut self, at: Vec2, cx: &mut Context<'_>) {
        self.at = at;
        let g = &self.grip;
        self.out = dot(Vec2::new(at.x - g.at.x, at.y - g.at.y), g.outward);
        self.see(cx);
    }

    /// The plan of the move (docs/adr/0770 §3) and its card, or its refusal (docs/adr/0790 §1).
    fn see(&mut self, cx: &mut Context<'_>) {
        let f = cx.format();
        let result = walls_edit::plan(&ExecutionContext::new(cx.doc), &self.request(self.out));
        self.walls.clear();
        self.refused = None;
        self.wrong = false;
        self.lines = Vec::new();
        if let Some(refused) = Refused::of_result(&result, cx) {
            self.lines = refused.lines.clone();
            self.lines.insert(0, self.size_line(None, &f));
            self.refused = Some(refused);
            return;
        }
        match result {
            CommandResult::Completed { output, .. } => {
                let joined = output.joined.len();
                if let Some(e) = &output.effects {
                    self.lines = self.card(e, joined, &f);
                }
                self.walls = output.walls.into_iter().chain(output.joined).collect();
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.lines = vec![self.size_line(None, &f), error.message];
                self.wrong = true;
            }
            _ => {}
        }
    }

    /// The room's inner size across, before and after; the move when no wall faces it across.
    fn size_line(&self, after: Option<f64>, f: &Format) -> String {
        match self.across {
            Some(before) => format!(
                "İç ölçü: {} → {}",
                f.length(before),
                f.length(after.unwrap_or(before + self.out))
            ),
            None => format!("Kaydırma: {}", signed(f.length(self.out), self.out)),
        }
    }

    /// The card: the room's inner size, its area, then the other side's distance, the other rooms and what follows.
    fn card(&self, e: &kentos_commands::WallEffects, joined: usize, f: &Format) -> Vec<String> {
        let (inner, other) = if self.grip.room_left {
            (&e.left, &e.right)
        } else {
            (&e.right, &e.left)
        };
        let mut lines = vec![self.size_line(inner.as_ref().map(|g| g.after), f)];
        let mine = self.space_uid.map(|u| u.to_string());
        let (first, rest): (Vec<_>, Vec<_>) = e
            .spaces
            .iter()
            .partition(|s| mine.as_deref() == Some(s.uid.as_str()));
        lines.extend(
            first
                .into_iter()
                .chain(rest)
                .map(|s| crate::wall_distance::space_line(s, f)),
        );
        if let Some(g) = other {
            lines.push(format!(
                "Öbür yan mesafe: {} → {}",
                f.length(g.before),
                f.length(g.after)
            ));
        }
        lines.extend(crate::wall_distance::count_lines(e, joined));
        lines
    }

    /// A value typed for the grip: the room's new inner size across, or with + or − by how much it grows or shrinks
    /// (with no wall across, the number is how far the edge goes out). The distance out it gives; none for another
    /// text.
    pub fn typed_out(&self, text: &str, cx: &Context<'_>) -> Option<f64> {
        let t = text.trim();
        let (sign, rest) = match t.chars().next() {
            Some('+') => (Some(1.0), &t[1..]),
            Some(c @ ('-' | '−')) => (Some(-1.0), &t[c.len_utf8()..]),
            _ => (None, t),
        };
        let value = cx.typed_length(rest.trim()).filter(|x| x.is_finite())?;
        Some(match (sign, self.across) {
            (Some(s), _) => s * value,
            (None, Some(now)) => value - now,
            (None, None) => value,
        })
    }

    /// Writes the move `out` (the pointer's when none is given): one undo step; the room's area before and after said.
    pub fn commit(mut self, out: Option<f64>, cx: &mut Context<'_>) {
        if let Some(out) = out {
            self.out = out;
        }
        let same = |uid: Option<Uuid>, slot: Slot, cx: &Context<'_>| {
            uid.is_some_and(|u| cx.doc.slot_of(u) == Some(slot))
        };
        if !same(Some(self.grip.wall_uid), self.grip.wall, cx)
            || !same(self.space_uid, self.grip.space, cx)
        {
            cx.say(
                Level::Warn,
                "Tutamacın odası ya da duvarı artık çizimde yok (silinmiş ya da geri alınmış); tutamaç bırakıldı.",
            );
            return;
        }
        if self.out.abs() < 1e-9 {
            return;
        }
        let name = room_name(cx.doc, self.grip.space);
        let before = room_area(cx.doc, self.grip.space);
        let result =
            walls_edit::execute(&mut ExecutionContext::new(cx.doc), self.request(self.out));
        if crate::points::written(result, cx).is_none() {
            return;
        }
        let f = cx.format();
        let moved = signed(f.length(self.out), self.out);
        let text = match (before, room_area(cx.doc, self.grip.space)) {
            (Some(b), Some(a)) => format!(
                "Oda {}: {name} {} → {}; kenarı {moved} kaydı.",
                if a >= b {
                    "büyütüldü"
                } else {
                    "küçültüldü"
                },
                f.area(b),
                f.area(a)
            ),
            _ => format!("Odanın kenarı {moved} kaydı."),
        };
        cx.say(Level::Success, text);
    }

    /// The command line's words while the grip moves.
    pub fn prompt(&self) -> Prompt {
        Prompt::new(
            LABEL,
            "kenarı yerine tıklayın ya da yeni iç ölçüyü yazın (+ ya da − ile ne kadar büyüyeceğini)",
        )
    }

    /// The edge as it is and where it goes, the room's inner size across it, the walls as the move leaves them
    /// (dashed), and the card.
    pub fn preview(&self, _format: &Format) -> Preview {
        let g = &self.grip;
        let off = Vec2::new(g.outward.x * self.out, g.outward.y * self.out);
        let moved = |p: Vec2| Vec2::new(p.x + off.x, p.y + off.y);
        let mut out = Preview::default();
        out.strokes.push(
            Stroke::solid(vec![g.a, g.b], false)
                .width(2.0)
                .tone(Tone::Snap),
        );
        out.strokes.push(
            Stroke::solid(vec![moved(g.a), moved(g.b)], false)
                .width(2.0)
                .tone(Tone::Accent),
        );
        if let Some(before) = self.across {
            let from = moved(g.at);
            let width = before + self.out;
            let to = Vec2::new(from.x - g.outward.x * width, from.y - g.outward.y * width);
            out.strokes
                .push(Stroke::dashed(vec![from, to], false, [4.0, 3.0]).tone(Tone::Accent));
        }
        for w in &self.walls {
            out.strokes
                .extend(Outline::of(&shape(w), Some([6.0, 4.0]), 1.0, Tone::Accent).strokes);
        }
        if let Some(refused) = &self.refused {
            let card = Refused {
                lines: self.lines.clone(),
                ..refused.clone()
            };
            return card.over(out, self.at);
        }
        out.tag = Some(Tag {
            at: self.at,
            lines: self.lines.clone(),
        });
        if self.wrong {
            out.tag_tone = Tone::Danger;
        }
        out
    }
}
