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
//!   drops the point and the tool waits for the next;
//! - what it places is said before the click (docs/adr/0726 §3): beside the
//!   cursor the footprint's real width × depth in the drawing's unit, the
//!   block's unit, the turn and the mirror; a mark at the base point; the
//!   settings pane has the block, its size and unit, a dynamic block's sizes
//!   and views (its lookups, visibilities, flips and lengths; Varyant (V)
//!   steps the first list), the factors, the turn, Aynala and Duvara hizala.
//!   The values go into `cad.blocks.insert`'s `parameters`; the ghost draws
//!   the variant.

use std::collections::BTreeMap;

use kentos_commands::blocks::{name_key, turn_of};
use kentos_commands::{BlockId, BlockParameter, BlockUnits, BlocksInsert, ParamValue};
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
use crate::tool::{
    Context, Flow, Marker, MarkerShape, Memory, OptionChoice, Pointer, Preview, Stroke, Tag, Tone, Tool,
    ViewChange,
};
use crate::tool_settings::Setting;

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
    /// A dynamic block's values given in the tool, by parameter name, and the block they are for.
    values: BTreeMap<String, ParamValue>,
    values_of: Option<BlockId>,
    /// What the tool knows of the chosen block, as of the last call.
    chosen: Chosen,
}

/// The keys of a dynamic block's parameters in the settings pane, in their order (at most eight).
const PARAM_KEYS: [&str; 8] = ["p0", "p1", "p2", "p3", "p4", "p5", "p6", "p7"];

/// A dynamic block's parameter the tool offers before placing (docs/adr/0726 §4).
#[derive(Clone, Debug, PartialEq)]
enum Offer {
    /// A lookup's rows or a visibility's states, by label; the one chosen.
    Choice {
        name: String,
        labels: Vec<String>,
        chosen: Option<String>,
    },
    /// A flip.
    Flag { name: String, on: bool },
    /// A linear parameter's length, in the definition's numbers.
    Length { name: String, value: f64 },
}

impl Offer {
    fn name(&self) -> &str {
        match self {
            Offer::Choice { name, .. } | Offer::Flag { name, .. } | Offer::Length { name, .. } => name,
        }
    }
}

/// What the tool knows of the chosen block, as of the last call (`see`).
#[derive(Clone, Debug, Default, PartialEq)]
struct Chosen {
    block: Option<BlockId>,
    /// Its unit in words (docs/adr/0315 §4).
    units: String,
    /// A unitless block's numbers are the drawing's units: a length is typed as a plain number.
    unitless: bool,
    /// The factor its inserts take for its unit.
    unit_factor: f64,
    /// The definition's box with the values, about its base point; none when it draws nothing.
    bbox: Option<[f64; 4]>,
    /// The values resolved as the command resolves them, for the ghost.
    resolved: Option<BTreeMap<String, ParamValue>>,
    /// The drawing's blocks by name, for the Blok row.
    blocks: Vec<(BlockId, String)>,
    offers: Vec<Offer>,
    /// Why the values given are refused, said in the pane.
    problem: Option<String>,
    /// The revision and values this was measured at.
    measured: Option<(u64, BTreeMap<String, ParamValue>)>,
}

/// A block's unit in words (docs/adr/0726 §3): Blok ekle's tag and pane, Öznitelikler and Bloklar › Kitaplık.
pub fn unit_words(units: Option<BlockUnits>) -> &'static str {
    match units {
        Some(BlockUnits::Mm) => "milimetre",
        Some(BlockUnits::Cm) => "santimetre",
        Some(BlockUnits::M) => "metre",
        Some(BlockUnits::In) => "inç",
        Some(BlockUnits::Ft) => "fit",
        Some(BlockUnits::Unitless) => "birimsiz (çizimin birimiyle büyür)",
        None => "belirtilmemiş (metre sayılır)",
    }
}

/// What the tool knows of `block` with `values` in `doc`.
fn chosen_of(
    doc: &kentos_domain::Document,
    block: Option<BlockId>,
    values: &BTreeMap<String, ParamValue>,
) -> Chosen {
    let blocks = doc.blocks().iter().map(|b| (b.id, b.name.clone())).collect();
    let measured = Some((doc.revision(), values.clone()));
    let Some(def) = block.and_then(|id| doc.block(id)) else {
        return Chosen {
            blocks,
            measured,
            unit_factor: 1.0,
            ..Chosen::default()
        };
    };
    let (resolved, problem) = match &def.dynamic {
        Some(d) if !values.is_empty() => {
            match kentos_native_application::block_params::resolve(d, &BTreeMap::new(), values) {
                Ok(r) => (Some(r), None),
                Err((_, _, words)) => (None, Some(words)),
            }
        }
        _ => (None, None),
    };
    let bbox = kentos_native_application::blocks::definition_box(doc, def.id, resolved.as_ref());
    // A value as the click places it: resolved (a lookup's row sets its lengths), else as given.
    let given = |name: &str| resolved.as_ref().and_then(|r| r.get(name)).or_else(|| values.get(name));
    let offers = def
        .dynamic
        .iter()
        .flat_map(|d| d.parameters.iter())
        .filter_map(|p| match p {
            BlockParameter::Lookup { name, rows } => Some(Offer::Choice {
                name: name.clone(),
                labels: rows.iter().map(|r| r.label.clone()).collect(),
                chosen: match given(name) {
                    Some(ParamValue::Text(t)) => Some(t.clone()),
                    _ => None,
                },
            }),
            BlockParameter::Visibility { name, states } => Some(Offer::Choice {
                name: name.clone(),
                labels: states.iter().map(|s| s.name.clone()).collect(),
                chosen: match given(name) {
                    Some(ParamValue::Text(t)) => Some(t.clone()),
                    _ => states.first().map(|s| s.name.clone()),
                },
            }),
            BlockParameter::Flip { name, .. } => Some(Offer::Flag {
                name: name.clone(),
                on: matches!(given(name), Some(ParamValue::Flag(true))),
            }),
            BlockParameter::Linear { name, .. } => Some(Offer::Length {
                name: name.clone(),
                value: match (given(name), p.default_value()) {
                    (Some(ParamValue::Number(v)), _) => *v,
                    (None, Some(ParamValue::Number(v))) => v,
                    _ => 0.0,
                },
            }),
            _ => None,
        })
        .take(PARAM_KEYS.len())
        .collect();
    Chosen {
        block: Some(def.id),
        units: unit_words(def.units).to_owned(),
        unitless: def.units == Some(BlockUnits::Unitless),
        unit_factor: blocks_insert::unit_factor(doc, def),
        bbox,
        resolved,
        blocks,
        offers,
        problem,
        measured,
    }
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
        let block = cx.memory.block_insert;
        let name = block
            .and_then(|id| cx.doc.block(id))
            .map_or_else(String::new, |b| b.name.clone());
        self.seen = Some((*cx.memory, name));
        let revision = cx.doc.revision();
        if self.walls.is_none_or(|(r, _)| r != revision) {
            let any = cx.doc.entities().any(|e| matches!(e, Entity::Wall(_)));
            self.walls = Some((revision, any));
        }
        // Another block's values are not this one's.
        if self.values_of != block {
            self.values.clear();
            self.values_of = block;
        }
        let stale = self.chosen.block != block
            || self
                .chosen
                .measured
                .as_ref()
                .is_none_or(|(r, v)| *r != revision || *v != self.values);
        if stale {
            self.chosen = chosen_of(cx.doc, block, &self.values);
        }
    }

    /// The footprint the block places with the tool's factors, metres: its box's width and depth.
    fn footprint(&self) -> Option<(f64, f64)> {
        let (m, _) = self.seen.as_ref()?;
        let b = self.chosen.bbox?;
        let unit = self.chosen.unit_factor;
        let x = (m.block_scale * unit).abs();
        let y = (m.block_y_scale.unwrap_or(m.block_scale) * unit).abs();
        Some(((b[2] - b[0]) * x, (b[3] - b[1]) * y))
    }

    /// The first list of the block's sizes or views stepped to its next label (Varyant).
    fn next_variant(&mut self) -> bool {
        let Some(Offer::Choice { name, labels, chosen }) = self
            .chosen
            .offers
            .iter()
            .find(|o| matches!(o, Offer::Choice { .. }))
            .cloned()
        else {
            return false;
        };
        if labels.is_empty() {
            return false;
        }
        let at = chosen
            .as_ref()
            .and_then(|c| labels.iter().position(|l| l == c))
            .map_or(0, |i| (i + 1) % labels.len());
        self.values
            .insert(name, ParamValue::Text(labels[at].clone()));
        true
    }

    /// A value given for the offer at `index`, in the pane's words: a label, Açık/Kapalı's yes/no, a length.
    fn give(&mut self, index: usize, value: &str, cx: &mut Context<'_>) -> bool {
        let Some(offer) = self.chosen.offers.get(index).cloned() else {
            return false;
        };
        let given = match &offer {
            Offer::Choice { labels, .. } => {
                let Some(label) = labels.iter().find(|l| name_key(l) == name_key(value.trim())) else {
                    cx.say(Level::Warn, format!("“{}” {} için bir seçenek değil.", value.trim(), offer.name()));
                    return false;
                };
                ParamValue::Text(label.clone())
            }
            Offer::Flag { .. } => ParamValue::Flag(value == "yes"),
            Offer::Length { .. } => {
                let read = if self.chosen.unitless {
                    points::plain_number(value)
                } else {
                    cx.typed_length(value)
                };
                match read.filter(|v| v.is_finite()) {
                    Some(v) => ParamValue::Number(v),
                    None => {
                        cx.say(Level::Warn, format!("{} için bir uzunluk yazın.", offer.name()));
                        return false;
                    }
                }
            }
        };
        self.values.insert(offer.name().to_owned(), given);
        true
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
            "V" => return self.next_variant(),
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
    fn insert_input(
        &self,
        block: BlockId,
        p: Vec2,
        turn: f64,
        attrs: Option<std::collections::BTreeMap<String, String>>,
        cx: &Context<'_>,
    ) -> BlocksInsert {
        let m = &*cx.memory;
        BlocksInsert {
            // A dynamic block's sizes and views chosen in the tool (docs/adr/0726 §4).
            parameters: (!self.values.is_empty() && self.values_of == Some(block))
                .then(|| self.values.clone()),
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
        let input = self.insert_input(block, p, turn, attrs, cx);
        let result = blocks_insert::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            self.d.note(out.id, cx);
        }
    }

    /// The block as it would be placed at the cursor; nothing while a value is asked for.
    fn place_ghost(&mut self, cx: &Context<'_>) {
        // The chosen block and its values as of now: the ghost is the variant the click places.
        self.see(cx);
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
        let params = self
            .chosen
            .resolved
            .as_ref()
            .and_then(kentos_native_application::blocks::core_params);
        let paths = cx.spatial.store().insert_outlines_with(
            &block.to_text(),
            h,
            (x, y),
            turn_of(self.turn(cx)),
            m.block_mirror,
            params,
        );
        (self.ghost, self.marks) = ghosts(&paths);
    }

    /// What the tag beside the cursor says (docs/adr/0726 §3): the footprint, the unit, the turn and the mirror.
    fn tag_lines(&self, f: &crate::format::Format) -> Vec<String> {
        let Some((m, name)) = &self.seen else {
            return Vec::new();
        };
        let mut lines = Vec::new();
        match self.footprint() {
            Some((w, d)) => lines.push(format!(
                "{name}: {} × {} {}",
                f.length_bare(w),
                f.length_bare(d),
                f.length_unit_label()
            )),
            None => lines.push(format!("{name}: çizecek nesnesi yok")),
        }
        let mut more = vec![format!("birimi {}", self.chosen.units)];
        match self.fit {
            Some((_, t)) => more.push(format!("dönüş {}° duvara göre", fixed(t, 1))),
            None if m.block_rotation != 0.0 => more.push(format!("dönüş {}°", fixed(m.block_rotation, 1))),
            None => {}
        }
        if m.block_mirror {
            more.push("aynalı".to_owned());
        }
        lines.push(more.join(" · "));
        lines
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
        // A dynamic block's first list of sizes or views (docs/adr/0726 §4).
        if let Some(Offer::Choice { chosen, .. }) = self
            .chosen
            .offers
            .iter()
            .find(|o| matches!(o, Offer::Choice { .. }))
        {
            prompt = prompt.option_with("Varyant", "V", chosen.clone().unwrap_or_else(|| "varsayılan".into()));
        }
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

    fn preview(&self, format: &crate::format::Format) -> Preview {
        let shown = self.ask.is_none() && !self.ghost.is_empty();
        let at = self.d.hover.filter(|_| shown);
        Preview {
            strokes: self.ghost.clone(),
            marks: self.marks.clone(),
            tracking: self.d.tracking,
            // Its base point, and what it places, before the click (docs/adr/0726 §3).
            markers: at
                .map(|at| Marker {
                    at,
                    shape: MarkerShape::Plus(6.0),
                    tone: Tone::Snap,
                })
                .into_iter()
                .collect(),
            tag: at.map(|at| Tag {
                at,
                lines: self.tag_lines(format),
            }),
            ..Preview::default()
        }
    }

    /// The pane's rows (docs/adr/0726 §4): the block, its size and unit, a dynamic block's sizes and views, the
    /// factors, the turn, Aynala and Duvara hizala.
    fn settings(&self, format: &crate::format::Format) -> Vec<Setting> {
        let Some((m, _)) = &self.seen else {
            return Vec::new();
        };
        let c = &self.chosen;
        let mut rows = vec![Setting::choice(
            "block",
            "Blok",
            c.blocks
                .iter()
                .map(|(id, name)| OptionChoice {
                    label: name.clone(),
                    typed: name.clone(),
                    icon: None,
                    preview: None,
                    checked: Some(*id) == c.block,
                    command: None,
                })
                .collect(),
        )];
        if let Some((w, d)) = self.footprint() {
            rows.push(
                Setting::text(
                    "size",
                    "En × boy",
                    format!(
                        "{} × {} {}",
                        format.length_bare(w),
                        format.length_bare(d),
                        format.length_unit_label()
                    ),
                )
                .read_only(),
            );
        }
        rows.push(Setting::text("units", "Bloğun birimi", c.units.clone()).read_only());
        for (offer, key) in c.offers.iter().zip(PARAM_KEYS) {
            rows.push(match offer {
                Offer::Choice { name, labels, chosen } => {
                    let mut row = Setting::choice(
                        key,
                        name.clone(),
                        labels
                            .iter()
                            .map(|l| OptionChoice {
                                label: l.clone(),
                                typed: l.clone(),
                                icon: None,
                                preview: None,
                                checked: chosen.as_ref() == Some(l),
                                command: None,
                            })
                            .collect(),
                    );
                    // No row chosen: the block as it is drawn.
                    if row.value.is_empty() {
                        row.value = "Varsayılan".into();
                    }
                    row
                }
                Offer::Flag { name, on } => Setting::toggle(key, name.clone(), *on),
                Offer::Length { name, value } => Setting::text(
                    key,
                    name.clone(),
                    if c.unitless {
                        fixed(*value, 3)
                    } else {
                        format.length(*value)
                    },
                ),
            });
        }
        if let Some(why) = &c.problem {
            rows.push(Setting::notice("problem", "Değer", why.clone()));
        }
        rows.push(Setting::text("scale", "Ölçek", fixed(m.block_scale, 4)));
        rows.push(Setting::text(
            "yscale",
            "Y ölçeği",
            fixed(m.block_y_scale.unwrap_or(m.block_scale), 4),
        ));
        rows.push(Setting::text("turn", "Dönüş", format!("{}°", fixed(m.block_rotation, 4))));
        rows.push(Setting::toggle("mirror", "Aynala", m.block_mirror));
        if self.walls.is_some_and(|(_, any)| any) {
            rows.push(Setting::toggle("wall", "Duvara hizala", m.block_to_wall));
        }
        rows
    }

    fn configure(&mut self, key: &str, value: &str, cx: &mut Context<'_>) -> bool {
        let number = || points::plain_number(value.trim().trim_end_matches('°'));
        let done = match key {
            "block" => match cx
                .doc
                .blocks()
                .iter()
                .find(|b| name_key(&b.name) == name_key(value.trim()))
            {
                Some(b) => {
                    cx.memory.block_insert = Some(b.id);
                    true
                }
                None => {
                    cx.say(Level::Warn, format!("Çizimde “{}” bloğu yok.", value.trim()));
                    false
                }
            },
            "scale" | "yscale" => match number() {
                Some(n) if n > 0.0 && n.is_finite() => {
                    self.value(if key == "scale" { Ask::Scale } else { Ask::YScale }, n, cx);
                    true
                }
                _ => {
                    cx.say(Level::Warn, "Ölçek sıfırdan büyük olmalı.");
                    false
                }
            },
            "turn" => match number().filter(|n| n.is_finite()) {
                Some(n) => {
                    cx.memory.block_rotation = n;
                    true
                }
                None => {
                    cx.say(Level::Warn, "Dönüşü derece olarak yazın.");
                    false
                }
            },
            "mirror" => {
                cx.memory.block_mirror = value == "yes";
                true
            }
            "wall" => {
                cx.memory.block_to_wall = value == "yes";
                true
            }
            _ => match PARAM_KEYS.iter().position(|k| *k == key) {
                Some(i) => self.give(i, value, cx),
                None => false,
            },
        };
        self.see(cx);
        self.place_ghost(cx);
        done
    }
}
