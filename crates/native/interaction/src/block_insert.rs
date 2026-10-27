//! Blok ekle: the web's `BlockInsertTool` (`apps/web/src/tools/blockTools.ts`)
//! on its `PointInputTool` base, step for step (docs/adr/0144 §6):
//!
//! - without a block in the drawing the tool says so and leaves;
//! - every click (or typed point) places the block there, an insert through
//!   `cad.blocks.insert` (docs/adr/0315 §4) on the active layer in the current
//!   colour: its own object and undo step (“Ekle”); the tool waits for the next;
//!   a unitless block takes the drawing's unit, a block drawn in another unit is
//!   said;
//! - Blok (B) takes the drawing's next block; Ölçek (Ö, or O), Y ölçeği (Y;
//!   docs/adr/0315 §3) and Dönüş (D, degrees counter-clockwise) ask for a
//!   typed value; Aynala (A) mirrors in the block's x axis. Ölçek sets both
//!   factors. The block, factors, turn and mirror stay for as long as the app
//!   lives ([`Memory`]);
//! - Duvara hizala (H, docs/adr/0702): near a wall's face (40 px) the block's
//!   point stands on the face and its x runs along it, its front (+y) toward
//!   the pointer's side, the turn given instead of Dönüş's;
//! - the ghost is the block as it would be placed at the cursor, from the
//!   geometry store (`insert_outlines`);
//! - a block with attribute definitions (§7) asks their values first: the
//!   point waits while the host's window asks ([`ViewChange::AttributeValues`]);
//!   Yerleştir writes the insert with them ([`Tool::values_given`]), Vazgeç
//!   drops the point and the tool waits for the next.

use kentos_commands::blocks::turn_of;
use kentos_commands::{BlockId, BlocksInsert};
use kentos_native_application::{ExecutionContext, blocks_insert};
use kentos_geometry_core::tools::point_text::js_trim;

use crate::Vec2;
use crate::format::fixed;
use kentos_commands::Entity;
use kentos_domain::Slot;
use crate::log::Level;
use crate::modify::ghosts;
use crate::points::{self, Taken, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Memory, Pointer, Preview, Stroke, Tool, ViewChange};

/// The tool's id: its command is `tool.blockInsert`.
pub const ID: &str = "blockInsert";
pub const LABEL: &str = "Blok ekle";
const NO_BLOCK: &str = "Çizimde blok yok; önce Blok oluştur ile bir blok tanımlayın.";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Scale,
    YScale,
    Rotation,
}

/// The Blok ekle tool.
#[derive(Clone, Debug, Default)]
pub struct BlockInsert {
    d: Taken,
    ask: Option<Ask>,
    /// What the session remembered, and the block's name, as of the last call.
    seen: Option<(Memory, String)>,
    /// The ghost at the cursor, as of the last pointer move.
    ghost: Vec<Stroke>,
    marks: Vec<Vec2>,
    /// The point waiting for the block's attribute values (§7), with its turn (degrees).
    pending: Option<(Vec2, f64)>,
    /// Duvara hizala's place at the pointer: the point on the face and the turn (degrees).
    fit: Option<(Vec2, f64)>,
    /// Whether the drawing has walls, as of its revision: Duvara hizala is offered only then.
    walls: Option<(u64, bool)>,
}

/// How near a wall's face Duvara hizala takes the block, logical pixels.
const WALL_REACH_PX: f64 = 40.0;

/// The wall face nearest `p` within `reach`: the point on it and the turn (degrees, counter-clockwise) that lays a
/// block's x along the face, its +y away from the wall toward `p` (docs/adr/0702). Straight face segments only.
pub fn wall_fit(p: Vec2, reach: f64, cx: &Context<'_>) -> Option<(Vec2, f64)> {
    let mut best: Option<(f64, Vec2, f64)> = None;
    for item in cx.spatial.store().near(p, reach) {
        let Some(e) = cx.doc.get(Slot(item.id as u32)) else {
            continue;
        };
        if !matches!(e, Entity::Wall(_)) {
            continue;
        }
        let kentos_geometry_core::entity::Shape::Wall { pts, bulges, wall } =
            kentos_native_application::geometry::shape(e)
        else {
            continue;
        };
        let Ok(lines) = kentos_geometry_core::geom::wall::lines(&pts, bulges.as_deref(), &wall) else {
            continue;
        };
        for l in [lines.first(), lines.last()].into_iter().flatten() {
            let n = l.pts.len();
            let count = if wall.closed { n } else { n.saturating_sub(1) };
            for k in 0..count {
                if l.bulges.get(k).is_some_and(|b| *b != 0.0) {
                    continue;
                }
                let (a, b) = (l.pts[k], l.pts[(k + 1) % n]);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let len2 = dx * dx + dy * dy;
                if len2 <= 0.0 {
                    continue;
                }
                let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0);
                let q = Vec2::new(a.x + dx * t, a.y + dy * t);
                let d = (p.x - q.x).hypot(p.y - q.y);
                if d > reach || best.is_some_and(|(bd, ..)| d >= bd) {
                    continue;
                }
                // The front toward the pointer: the face's left when the pointer is on it, else the way back.
                let left = (p.x - q.x) * -dy + (p.y - q.y) * dx >= 0.0;
                let turn = if left { dy.atan2(dx) } else { (-dy).atan2(-dx) };
                best = Some((d, q, turn.to_degrees()));
            }
        }
    }
    best.map(|(_, q, turn)| (q, turn))
}

impl BlockInsert {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        let name = cx
            .memory
            .block_insert
            .and_then(|id| cx.doc.block(id))
            .map_or_else(String::new, |b| b.name.clone());
        self.seen = Some((*cx.memory, name));
        let revision = cx.doc.revision();
        if self.walls.is_none_or(|(r, _)| r != revision) {
            let any = cx.doc.entities().any(|e| matches!(e, Entity::Wall(_)));
            self.walls = Some((revision, any));
        }
    }

    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        match key {
            "B" => {
                // The drawing's next block, round to the first.
                let blocks = cx.doc.blocks();
                let at = blocks
                    .iter()
                    .position(|b| Some(b.id) == cx.memory.block_insert);
                let next = at.map_or(0, |i| (i + 1) % blocks.len().max(1));
                cx.memory.block_insert = blocks.get(next).map(|b| b.id);
            }
            "Ö" | "O" => self.ask = Some(Ask::Scale),
            "Y" => self.ask = Some(Ask::YScale),
            "D" => self.ask = Some(Ask::Rotation),
            "A" => cx.memory.block_mirror = !cx.memory.block_mirror,
            "H" => cx.memory.block_to_wall = !cx.memory.block_to_wall,
            _ => return false,
        }
        true
    }

    /// A typed value while one is asked for: a scale above zero, a turn in degrees.
    fn value(&mut self, ask: Ask, n: f64, cx: &mut Context<'_>) {
        match ask {
            Ask::Scale | Ask::YScale if n <= 0.0 => {
                cx.say(Level::Warn, "Ölçek sıfırdan büyük olmalı.")
            }
            Ask::Scale => {
                cx.memory.block_scale = n;
                cx.memory.block_y_scale = None;
                self.ask = None;
            }
            Ask::YScale => {
                cx.memory.block_y_scale = (n != cx.memory.block_scale).then_some(n);
                self.ask = None;
            }
            Ask::Rotation => {
                cx.memory.block_rotation = n;
                self.ask = None;
            }
        }
    }

    /// `cad.blocks.insert`'s input: everything explicit (CMD-07), the active layer and the current colour.
    fn input(
        block: BlockId,
        p: Vec2,
        turn: f64,
        attrs: Option<std::collections::BTreeMap<String, String>>,
        cx: &Context<'_>,
    ) -> BlocksInsert {
        let m = &*cx.memory;
        BlocksInsert {
            parameters: Default::default(),
            block: Some(block),
            name: None,
            at: wire(p),
            scale: Some(m.block_scale),
            y_scale: m.block_y_scale,
            rotation: Some(turn_of(turn)),
            mirror: m.block_mirror.then_some(true),
            layer_id: cx.doc.layers().active().to_owned(),
            color: cx.draft.color_text(),
            attributes: attrs,
            // An object template's data, label and symbol (docs/adr/0176 §3).
            attrs: cx.template_attrs(None),
            label: cx.template_label(),
            symbol: cx.template_symbol(),
            expected_revision: None,
        }
    }

    /// The turn the block goes in at the pointer: Duvara hizala's near a wall, else Dönüş's.
    fn turn(&self, cx: &Context<'_>) -> f64 {
        self.fit.map_or(cx.memory.block_rotation, |(_, t)| t)
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        let (None, None, Some(block)) = (self.ask, self.pending, cx.memory.block_insert) else {
            return;
        };
        let turn = self.turn(cx);
        // A block with attributes to ask asks their values first (§7): not a constant, a preset or a field one
        // (docs/adr/0317 §1).
        if cx.doc.block(block).is_some_and(|b| {
            b.attributes
                .iter()
                .any(|a| !a.options.constant && !a.options.preset && a.options.field.is_none())
        }) {
            self.pending = Some((p, turn));
            cx.view_changes.push(ViewChange::AttributeValues(block));
            return;
        }
        self.write(block, p, turn, None, cx);
    }

    fn write(
        &mut self,
        block: BlockId,
        p: Vec2,
        turn: f64,
        attrs: Option<std::collections::BTreeMap<String, String>>,
        cx: &mut Context<'_>,
    ) {
        let input = Self::input(block, p, turn, attrs, cx);
        let result = blocks_insert::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            self.d.note(out.id, cx);
        }
    }

    /// The block as it would be placed at the cursor; nothing while a value is asked for.
    fn place_ghost(&mut self, cx: &Context<'_>) {
        self.ghost.clear();
        self.marks.clear();
        let (Some(h), None, Some(block)) = (self.d.hover, self.ask, cx.memory.block_insert) else {
            return;
        };
        let m = *cx.memory;
        // The factors as the command writes them: a unitless block's times the drawing's unit.
        let unit = cx
            .doc
            .block(block)
            .map_or(1.0, |b| blocks_insert::unit_factor(cx.doc, b));
        let x = m.block_scale * unit;
        let y = m.block_y_scale.unwrap_or(m.block_scale) * unit;
        let paths = cx.spatial.store().insert_outlines_xy(
            &block.to_text(),
            h,
            (x, y),
            turn_of(self.turn(cx)),
            m.block_mirror,
        );
        (self.ghost, self.marks) = ghosts(&paths);
    }
}

impl Tool for BlockInsert {
    /// A point computed by the point calculator, as if clicked (the web's `acceptPoint`).
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        self.see(cx);
        true
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let (m, name) = self
            .seen
            .clone()
            .unwrap_or_else(|| (Memory::default(), String::new()));
        let step = match self.ask {
            Some(Ask::Scale) => "ölçeği yazın (x ve y birlikte)".to_owned(),
            Some(Ask::YScale) => "y ölçeğini yazın".to_owned(),
            Some(Ask::Rotation) => "dönüş açısını derece olarak yazın".to_owned(),
            None => format!("“{name}” için yerleştirme noktasına tıklayın"),
        };
        let mut prompt = Prompt::new(LABEL, step).option("Blok", "B");
        // Duvara hizala where there are walls, next to the block: it decides the turn (docs/adr/0702).
        if self.walls.is_some_and(|(_, any)| any) {
            prompt = prompt.option_with(
                "Duvara hizala",
                "H",
                if m.block_to_wall { "açık" } else { "kapalı" },
            );
        }
        let turn = match self.fit {
            Some((_, t)) => format!("{}° duvara göre", fixed(t, 4)),
            None => format!("{}°", fixed(m.block_rotation, 4)),
        };
        prompt
            .option_with("Ölçek", "Ö", fixed(m.block_scale, 4))
            .option_with(
                "Y ölçeği",
                "Y",
                fixed(m.block_y_scale.unwrap_or(m.block_scale), 4),
            )
            .option_with("Dönüş", "D", turn)
            .option_with(
                "Aynala",
                "A",
                if m.block_mirror { "açık" } else { "kapalı" },
            )
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    /// Without a block in the drawing there is nothing to place: it says so and leaves.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        let blocks = cx.doc.blocks();
        let Some(first) = blocks.first().map(|b| b.id) else {
            cx.say(Level::Warn, NO_BLOCK);
            return Flow::Exit;
        };
        if !blocks.iter().any(|b| Some(b.id) == cx.memory.block_insert) {
            cx.memory.block_insert = Some(first);
        }
        self.see(cx);
        Flow::Stay
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.fit = cx
            .memory
            .block_to_wall
            .then(|| wall_fit(p.raw, cx.view.world_length(WALL_REACH_PX), cx))
            .flatten();
        self.d.hover = Some(match self.fit {
            Some((q, _)) => q,
            None => self.d.constrain(p, cx),
        });
        self.place_ghost(cx);
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.fit = cx
            .memory
            .block_to_wall
            .then(|| wall_fit(p.raw, cx.view.world_length(WALL_REACH_PX), cx))
            .flatten();
        let p = match self.fit {
            Some((q, _)) => q,
            None => self.d.constrain(p, cx),
        };
        self.accept(p, cx);
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let done = if self.option(&upper_tr(js_trim(text)), cx) {
            true
        } else if let (Some(ask), Some(n)) = (self.ask, points::plain_number(text)) {
            self.value(ask, n, cx);
            true
        } else {
            match cx.typed_point(text, self.d.last(), self.d.hover) {
                Some(p) => {
                    self.accept(p, cx);
                    true
                }
                None => false,
            }
        };
        self.place_ghost(cx);
        self.see(cx);
        done
    }

    /// It never holds a point: a confirm leaves (the web's `PointInputTool.confirm`).
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        self.d.undo_step(false, cx)
    }

    /// The values window's answer: Yerleştir writes the insert at the
    /// waiting point with them (an empty map: none), Vazgeç drops the point.
    fn values_given(
        &mut self,
        values: Option<&std::collections::BTreeMap<String, String>>,
        cx: &mut Context<'_>,
    ) {
        let (Some((p, turn)), Some(values), Some(block)) =
            (self.pending.take(), values, cx.memory.block_insert)
        else {
            return;
        };
        let attrs = (!values.is_empty()).then(|| values.clone());
        self.write(block, p, turn, attrs, cx);
        self.see(cx);
    }

    fn preview(&self, _format: &crate::format::Format) -> Preview {
        Preview {
            strokes: self.ghost.clone(),
            marks: self.marks.clone(),
            tracking: self.d.tracking,
            ..Preview::default()
        }
    }
}
