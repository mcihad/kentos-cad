//! Duvarı kaydır (docs/adr/0790 §1; Kaydır of docs/adr/0710, 0770): a wall's straight edge is clicked, then follows the
//! pointer square to itself; the preview is `cad.walls.edit`'s plan of `shift { edge, by }` (its effects card: the two
//! sides' clear distances, the rooms' areas, what follows), or the architectural bonds' refusal with the bonds marked
//! where they stand while the pointer moves (thick lines, ringed jambs, “× d1 — kaldırılınca geçer”). A click there or a
//! typed distance (to the edge's left, negative to its right) writes it in one undo step; the tool waits for the next
//! edge. Esc lets the edge go; Esc with none leaves.

use kentos_commands::{CommandResult, Entity, WallsEdit, WallsOperation};
use kentos_domain::Uuid;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, walls_edit};

use crate::bond_refusal::Refused;
use crate::edge::Outline;
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Stroke, Tag, Tone, Tool};
use crate::wall_distance::effect_lines;
use crate::{Format, Level, Vec2};

/// The tool's id: its command is `tool.wallShift`.
pub const ID: &str = "wallShift";
pub const LABEL: &str = "Duvarı kaydır";
/// A click where no wall's straight edge is.
pub const NO_WALL_HERE: &str =
    "Tıklanan yerde kaydırılacak duvar kenarı yok; kilitsiz bir duvarın düz kenarına tıklayın.";

/// The edge picked: its wall, the edge's index and two points, and its left (where a positive shift goes).
#[derive(Clone, Debug, PartialEq)]
struct Picked {
    uid: Uuid,
    edge: u32,
    a: Vec2,
    b: Vec2,
    left: Vec2,
}

/// What the shift where the pointer is would do: its distance, the walls as they would be, and the card.
#[derive(Clone, Debug, Default, PartialEq)]
struct Seen {
    at: Vec2,
    by: f64,
    walls: Vec<Entity>,
    lines: Vec<String>,
    refused: Option<Refused>,
    /// Refused for another reason: the card in the danger colour, nothing marked.
    wrong: bool,
}

#[derive(Clone, Debug, Default)]
pub struct WallShift {
    picked: Option<Picked>,
    seen: Option<Seen>,
}

impl WallShift {
    pub fn new() -> Self {
        Self::default()
    }
}

/// The nearest straight edge of the wall at `slot` to `p` (its index and points), within `reach`.
fn nearest_edge(e: &Entity, p: Vec2, reach: f64) -> Option<(u32, Vec2, Vec2)> {
    let Entity::Wall(w) = e else { return None };
    let pts = &w.wall.pts;
    let n = pts.len();
    let count = if w.wall.closed { n } else { n.saturating_sub(1) };
    let mut best: Option<(f64, u32, Vec2, Vec2)> = None;
    for k in 0..count {
        let bulge = w.wall.bulges.as_ref().and_then(|b| b.get(k)).copied().unwrap_or(0.0);
        if bulge != 0.0 {
            continue;
        }
        let (a, b) = (pts[k], pts[(k + 1) % n]);
        let (a, b) = (Vec2::new(a.x, a.y), Vec2::new(b.x, b.y));
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let l2 = dx * dx + dy * dy;
        if l2 <= 0.0 {
            continue;
        }
        let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0);
        let d = (p.x - (a.x + dx * t)).hypot(p.y - (a.y + dy * t));
        if best.is_none_or(|(bd, ..)| d < bd) {
            best = Some((d, k as u32, a, b));
        }
    }
    // The wall's body is the pick's reach: half its width beyond the path.
    let reach = reach + w.wall.width;
    best.filter(|(d, ..)| *d <= reach).map(|(_, k, a, b)| (k, a, b))
}

impl WallShift {
    /// The wall edge under the click, or why none is taken.
    fn pick(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let reach = cx.pick_tolerance();
        let found = cx
            .spatial
            .hits(p.raw, reach.max(0.3))
            .into_iter()
            .filter_map(|s| {
                let e = cx.doc.get(s)?;
                if cx.doc.layers().is_locked(&e.base().layer_id) {
                    return None;
                }
                nearest_edge(e, p.raw, reach).map(|edge| (s, edge))
            })
            .next();
        let Some((slot, (edge, a, b))) = found else {
            cx.say(Level::Warn, NO_WALL_HERE);
            return;
        };
        let Some(uid) = cx.doc.uid(slot) else { return };
        let l = (b.x - a.x).hypot(b.y - a.y);
        self.picked = Some(Picked {
            uid,
            edge,
            a,
            b,
            left: Vec2::new(-(b.y - a.y) / l, (b.x - a.x) / l),
        });
        cx.selection.set_hover(None);
    }

    fn request(&self, by: f64) -> Option<WallsEdit> {
        let k = self.picked.as_ref()?;
        Some(WallsEdit {
            uids: vec![k.uid.to_string()],
            expected_revision: None,
            operation: WallsOperation::Shift {
                edge: Some(k.edge),
                by,
            },
        })
    }

    /// The pointer's distance from the picked edge to its left.
    fn by_at(&self, p: Vec2) -> Option<f64> {
        let k = self.picked.as_ref()?;
        Some((p.x - k.a.x) * k.left.x + (p.y - k.a.y) * k.left.y)
    }

    /// The plan of the shift `by` (docs/adr/0770 §3), or its refusal (docs/adr/0790 §1).
    fn see(&mut self, at: Vec2, by: f64, cx: &mut Context<'_>) {
        let Some(input) = self.request(by) else { return };
        let f = cx.format();
        let result = walls_edit::plan(&ExecutionContext::new(cx.doc), &input);
        let mut seen = Seen {
            at,
            by,
            ..Seen::default()
        };
        if let Some(refused) = Refused::of_result(&result, cx) {
            seen.lines = refused.lines.clone();
            seen.refused = Some(refused);
        } else {
            match result {
                CommandResult::Completed { output, .. } => {
                    seen.lines = output.effects.as_ref().map_or_else(Vec::new, |e| {
                        effect_lines(e, output.joined.len(), None, &f)
                    });
                    seen.walls = output.walls.into_iter().chain(output.joined).collect();
                }
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => {
                    seen.lines = vec![error.message];
                    seen.wrong = true;
                }
                _ => {}
            }
        }
        seen.lines.insert(0, signed(f.length(by), by));
        self.seen = Some(seen);
    }

    /// Writes the shift `by`; the tool waits for the next edge. Whether it was written.
    fn write(&mut self, by: f64, cx: &mut Context<'_>) -> bool {
        let Some(input) = self.request(by) else {
            return false;
        };
        let result = walls_edit::execute(&mut ExecutionContext::new(cx.doc), input);
        let Some(done) = crate::points::written(result, cx) else {
            // A refusal's card over the drawing says it now (docs/adr/0790 §3); the next move plans again.
            self.seen = None;
            return false;
        };
        let shown = signed(cx.format().length(by), by);
        let joined = done.joined.len();
        cx.say(
            Level::Success,
            if joined > 0 {
                format!("{LABEL}: kenar {shown} kaydırıldı; bağlı {joined} duvar izledi.")
            } else {
                format!("{LABEL}: kenar {shown} kaydırıldı.")
            },
        );
        self.picked = None;
        self.seen = None;
        true
    }
}

/// A signed amount, its sign always shown.
fn signed(text: String, v: f64) -> String {
    if v > 0.0 { format!("+{text}") } else { text }
}

impl Tool for WallShift {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        match &self.picked {
            None => Prompt::new(LABEL, "kaydırılacak duvar kenarına tıklayın"),
            Some(_) => Prompt::new(
                LABEL,
                "uzaklığı yazın ya da yerine tıklayın (kenarın soluna artı)",
            ),
        }
    }

    fn point_count(&self) -> usize {
        usize::from(self.picked.is_some())
    }

    fn activate(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    /// Once the edge is picked the pointer snaps: the edge goes through the point.
    fn snaps(&self) -> bool {
        self.picked.is_some()
    }

    fn cursor(&self) -> Cursor {
        if self.picked.is_some() {
            Cursor::Cross
        } else {
            Cursor::Pick
        }
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match self.by_at(p.world) {
            Some(by) => self.see(p.world, by, cx),
            None => {
                let reach = cx.pick_tolerance();
                let hit = cx
                    .spatial
                    .hits(p.raw, reach.max(0.3))
                    .into_iter()
                    .find(|s| cx.doc.get(*s).is_some_and(|e| nearest_edge(e, p.raw, reach).is_some()));
                cx.selection.set_hover(hit);
            }
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match self.by_at(p.world) {
            None => self.pick(p, cx),
            Some(by) => {
                self.write(by, cx);
            }
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.picked.is_none() {
            return false;
        }
        match cx.typed_length(text) {
            Some(by) if by.is_finite() => {
                self.write(by, cx);
                true
            }
            _ => false,
        }
    }

    /// Enter leaves (a click or a typed distance writes).
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// Esc: the edge goes first.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.undo_step(cx)
    }

    /// Ctrl+Z: the edge goes; with none the drawing is undone.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.seen = None;
        self.picked.take().is_some()
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = Preview::default();
        let Some(k) = &self.picked else {
            return out;
        };
        // The edge as it is, then the walls as the shift leaves them, dashed.
        out.strokes.push(Stroke::solid(vec![k.a, k.b], false).width(2.0).tone(Tone::Snap));
        let Some(seen) = &self.seen else {
            return out;
        };
        for w in &seen.walls {
            out.strokes
                .extend(Outline::of(&shape(w), Some([6.0, 4.0]), 1.0, Tone::Accent).strokes);
        }
        if let Some(refused) = &seen.refused {
            // The card with the shift's distance on top.
            let card = Refused {
                lines: seen.lines.clone(),
                ..refused.clone()
            };
            return card.over(out, seen.at);
        }
        out.tag = Some(Tag {
            at: seen.at,
            lines: seen.lines.clone(),
        });
        if seen.wrong {
            out.tag_tone = Tone::Danger;
        }
        out
    }
}
