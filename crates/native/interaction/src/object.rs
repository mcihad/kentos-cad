//! Birleştir and Patlat: the web's `JoinTool` and `ExplodeTool` on their
//! `SelectionActionTool` base (`apps/web/src/tools/editTools.ts`), on the
//! desktop's selection-first base ([`crate::modify`]), step for step
//! (docs/adr/0047):
//!
//! - with objects selected they act at once and the tool leaves; without,
//!   the objects are picked first (click, window) and a confirm acts;
//! - objects on a locked layer are left out, with the web's warning;
//! - Birleştir: lines, arcs and open polylines meeting end to end within the
//!   end gap tolerance become one polyline each (a chain that closes, a
//!   closed area), in the place and with the persistent id and the data of
//!   its first object; the others go. A number typed while picking is the
//!   tolerance, kept for as long as the app lives. Zincir (Z, kept too;
//!   docs/adr/0161 §2): a click on a line, an arc or an open polyline joins
//!   it with the objects joined end to end with it among the visible ones,
//!   up to a free end, a junction, a locked object or the start again, and
//!   the tool leaves;
//! - Patlat: a polyline or a closed area comes apart into lines and arcs, a
//!   spline into a polyline, a dimension into lines and its text, a
//!   patterned hatch into lines; the pieces are new objects from it (its
//!   layer and colour) and become the selection. A block's insert opens into
//!   its definition's objects, one level, each with its own layer, colour
//!   and data (docs/adr/0144); an item of an associative array explodes its
//!   whole array, which is gone (docs/adr/0302). With a curve among them (a
//!   circle, an arc, an ellipse, a spline, a NURBS curve) the tool stays: the
//!   polylines to come and their vertices are shown, Sayı (S), Aralık (A) and
//!   Sapma (P) choose how dense they are, a number typed the chosen one's
//!   value, Enter writes (docs/adr/0308).
//!
//! - Okunur yap (docs/adr/0145 §6): the texts that read upside down (turned
//!   more than 90° and at most 270°) turn half round about their box's
//!   middle, the box where it was and the alignment kept; a text that reads
//!   stays, and when none reads upside down that is said and nothing is
//!   written.
//!
//! Birleştir writes through its product command `cad.entities.join`
//! (docs/adr/0254 §4), the chains found there by the shared core
//! (`join_entities`, `chain`); Patlat through its own, `cad.entities.explode`
//! (docs/adr/0302), the pieces the core's (`explode_entity`); Okunur yap
//! writes one undo step through `cad.entities.edit`, the turn the core's
//! (`TextPlace::readable`).

use kentos_contracts::{
    CommandResult, EditOperation, EntitiesEditPlan, EntitiesExplode, EntitiesJoin, Entity,
    EntityEdit, ExplodeDensity, TextEntity, TrimBoundaries,
};
use kentos_domain::Slot;
use kentos_geometry_core::entity::{Shape, TextPlace};
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::{drawing_font, edit_geometry, shape};
use kentos_native_application::{ExecutionContext, entities_explode, entities_join};

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::points::{self, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Marker, MarkerShape, Memory, Preview, Tone};

/// The join tool's id: its command is `tool.join`.
pub const JOIN_ID: &str = "join";
/// The explode tool's id: its command is `tool.explode`.
pub const EXPLODE_ID: &str = "explode";
/// Okunur yap's id: its command is `tool.readable`.
pub const READABLE_ID: &str = "readable";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Join,
    Explode,
    Readable,
}

/// The join or the explode tool's part after the selection.
#[derive(Clone, Debug)]
pub struct ObjectAction {
    kind: Kind,
    /// What the session remembered and the project's units, as of the last event.
    seen: Option<(Memory, Format)>,
    /// Patlat with curves among its objects: those objects, and what the command would write (docs/adr/0308).
    curves: Vec<Slot>,
    plan: Option<EntitiesEditPlan>,
    drawn: Preview,
}

/// Whether Patlat makes `e` a polyline as dense as asked (docs/adr/0308).
fn is_curve(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Circle(_)
            | Entity::Arc(_)
            | Entity::Ellipse(_)
            | Entity::Spline(_)
            | Entity::Nurbs(_)
    )
}

/// The most vertex rings Patlat's preview draws.
const MAX_RINGS: usize = 20_000;

/// Patlat's density as the session keeps it.
fn density(m: &Memory) -> ExplodeDensity {
    match m.explode_way {
        1 => ExplodeDensity::Count {
            segments: m.explode_count,
        },
        2 => ExplodeDensity::Step {
            length: m.explode_step,
        },
        _ => ExplodeDensity::Deviation {
            tolerance: m.explode_deviation,
        },
    }
}

impl ObjectAction {
    fn of(kind: Kind) -> Modify<Self> {
        Modify::with(Self {
            kind,
            seen: None,
            curves: Vec::new(),
            plan: None,
            drawn: Preview::default(),
        })
    }

    pub fn join() -> Modify<Self> {
        Self::of(Kind::Join)
    }

    pub fn explode() -> Modify<Self> {
        Self::of(Kind::Explode)
    }

    pub fn readable() -> Modify<Self> {
        Self::of(Kind::Readable)
    }

    /// Patlat's input for `targets` at the session's density.
    fn explode_input(targets: &[Slot], cx: &Context<'_>) -> EntitiesExplode {
        EntitiesExplode {
            uids: targets
                .iter()
                .filter_map(|s| cx.doc.uid(*s))
                .map(|u| u.to_string())
                .collect(),
            density: Some(density(cx.memory)),
            expected_revision: None,
        }
    }

    /// The command's plan at the session's density, drawn: the polylines to come and their vertices.
    fn replan(&mut self, cx: &mut Context<'_>) {
        self.see(cx);
        self.drawn = Preview::default();
        self.plan = None;
        let input = Self::explode_input(&self.curves, cx);
        match entities_explode::plan_in(cx.doc, Some(cx.spatial.store()), &input) {
            CommandResult::Completed { output, .. } => {
                let mut rings = 0;
                for e in &output.created {
                    let s = shape(e);
                    self.drawn
                        .strokes
                        .extend(edge::Outline::of(&s, None, 2.0, Tone::Accent).strokes);
                    if let Entity::Polyline(p) = e {
                        for v in &p.pts {
                            if rings < MAX_RINGS {
                                self.drawn.markers.push(Marker {
                                    at: Vec2::new(v.x, v.y),
                                    shape: MarkerShape::Ring(3.0),
                                    tone: Tone::Accent,
                                });
                                rings += 1;
                            }
                        }
                    }
                }
                self.plan = Some(output);
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                cx.say(Level::Warn, error.message);
            }
            _ => {}
        }
    }

    /// How many vertices the polylines to come have.
    fn vertices(&self) -> usize {
        self.plan.as_ref().map_or(0, |p| {
            p.created
                .iter()
                .map(|e| match e {
                    Entity::Polyline(p) => p.pts.len(),
                    _ => 0,
                })
                .sum()
        })
    }

    /// Okunur yap (the web's `ReadableTool.run`, docs/adr/0145 §6): the texts
    /// that read upside down, turned half round about their box's middle in
    /// the drawing's typeface, in one step “Okunur yap”.
    fn run_readable(targets: &[Slot], cx: &mut Context<'_>) {
        let texts: Vec<(Slot, TextEntity)> = targets
            .iter()
            .filter_map(|slot| match cx.doc.get(*slot) {
                Some(Entity::Text(t)) => Some((*slot, t.clone())),
                _ => None,
            })
            .collect();
        if texts.is_empty() {
            cx.say(
                Level::Warn,
                "Seçimde yazı yok. Okunur yap yazı nesnelerini çevirir; yazıları seçip yeniden deneyin.",
            );
            return;
        }
        let font = drawing_font(cx.doc.settings().drawing_font);
        let changes: Vec<EntityEdit> = texts
            .iter()
            .filter_map(|(slot, t)| {
                // A multi-line text turns about its box's middle too (docs/adr/0182); one along a
                // curve turns its curve the other way, its alignment's shares swapped (docs/adr/0196 §3).
                let s = shape(&Entity::Text(t.clone()));
                let place = TextPlace::of(&s)?;
                let turned = if t.path.is_some() {
                    let placed = place.along(font)?.readable()?;
                    TextEntity {
                        p: kentos_contracts::Vec2 {
                            x: placed.p.x,
                            y: placed.p.y,
                        },
                        rotation: placed.rotation,
                        align: placed
                            .align
                            .and_then(|a| kentos_contracts::TextAlign::from_name(a.name())),
                        path: Some(kentos_native_application::geometry::contract_curve(
                            placed.curve,
                        )),
                        ..t.clone()
                    }
                } else {
                    let (p, rotation) = place.readable(font)?;
                    TextEntity {
                        p: kentos_contracts::Vec2 { x: p.x, y: p.y },
                        rotation,
                        ..t.clone()
                    }
                };
                Some(EntityEdit::Update {
                    uid: edge::uid(cx.doc, *slot),
                    geometry: edit_geometry(shape(&Entity::Text(turned)))?,
                })
            })
            .collect();
        if changes.is_empty() {
            cx.say(
                Level::Info,
                format!(
                    "Ters okunan yazı yok: {} yazının hepsi okunuyor.",
                    texts.len()
                ),
            );
            return;
        }
        let turned = changes.len();
        if edge::write(EditOperation::Readable, changes, cx).is_none() {
            return;
        }
        cx.say(Level::Success, format!("{turned} yazı okunur yapıldı."));
    }

    /// An object's kind as the web names it in messages, lower case.
    fn kind_name(s: &Shape) -> &'static str {
        match s {
            Shape::Point { .. } => "nokta",
            Shape::Line { .. } => "çizgi",
            Shape::Polyline { .. } => "çoklu çizgi",
            Shape::Polygon { .. } => "kapalı alan",
            Shape::Circle { .. } => "daire",
            Shape::Arc { .. } => "yay",
            Shape::Ellipse { .. } => "elips",
            Shape::Spline { .. } => "eğri",
            Shape::Nurbs { .. } => "NURBS eğrisi",
            Shape::Wall { .. } => "duvar",
            Shape::Opening { .. } => "açıklık",
            Shape::Xline { .. } => "yardımcı çizgi",
            Shape::Ray { .. } => "ışın",
            Shape::Text { .. } => "yazı",
            Shape::Dimension { .. } => "ölçü",
            Shape::Hatch { .. } => "tarama",
            Shape::Insert { .. } => "blok",
            Shape::Leader { .. } => "kılavuz",
            Shape::Table { .. } => "tablo",
            Shape::Image { .. } => "resim",
            Shape::Raster { .. } => "raster",
            Shape::PointCloud { .. } => "nokta bulutu",
            Shape::Record {} => "kayıt",
        }
    }

    /// The selected objects not on a locked layer, saying how many were left out (the web's `begin`).
    pub(crate) fn targets(cx: &mut Context<'_>) -> Vec<Slot> {
        let all: Vec<Slot> = cx
            .selection
            .ids()
            .iter()
            .copied()
            .filter(|slot| cx.doc.get(*slot).is_some())
            .collect();
        let doc = &*cx.doc;
        let editable: Vec<Slot> = all
            .iter()
            .copied()
            .filter(|slot| doc.get(*slot).is_some_and(|e| edge::unlocked(e, doc)))
            .collect();
        if editable.len() < all.len() {
            cx.say(
                Level::Warn,
                format!(
                    "{} nesne kilitli katmanda olduğu için atlandı.",
                    all.len() - editable.len()
                ),
            );
        }
        editable
    }

    /// Zincir (docs/adr/0161 §2, the web's `JoinTool.joinChain`): the
    /// visible lines, arcs and polylines walked from `hit` and joined, by the
    /// command `cad.entities.join` with its chain over the view. Whether a
    /// chain was joined.
    fn join_chain(hit: Slot, cx: &mut Context<'_>) -> bool {
        let Some(uid) = cx.doc.uid(hit) else {
            return false;
        };
        let view = cx.view.visible();
        let input = EntitiesJoin {
            uids: vec![uid.to_string()],
            tolerance: Some(cx.memory.join_tolerance.max(1e-9)),
            chain: Some(TrimBoundaries::Area {
                min: wire(Vec2::new(view.min_x, view.min_y)),
                max: wire(Vec2::new(view.max_x, view.max_y)),
            }),
            expected_revision: None,
        };
        Self::write_join(input, cx)
    }

    /// Birleştir (the web's `JoinTool.run`): the command `cad.entities.join`
    /// with the selection's objects.
    fn run_join(targets: &[Slot], cx: &mut Context<'_>) {
        let doc = &*cx.doc;
        let uids: Vec<String> = targets
            .iter()
            .filter_map(|s| doc.uid(*s).map(|u| u.to_string()))
            .collect();
        let input = EntitiesJoin {
            uids,
            tolerance: Some(cx.memory.join_tolerance.max(1e-9)),
            chain: None,
            expected_revision: None,
        };
        Self::write_join(input, cx);
    }

    /// Writes a join, selects what the chains became and says it, the
    /// command's warnings after (a chain that stopped at a locked object).
    fn write_join(input: EntitiesJoin, cx: &mut Context<'_>) -> bool {
        let result = entities_join::execute(
            &mut ExecutionContext::with_store(cx.doc, cx.spatial.store()),
            input,
        );
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
        let Some(out) = points::written(result, cx) else {
            return false;
        };
        let doc = &*cx.doc;
        let firsts: Vec<Slot> = out
            .changed
            .iter()
            .filter_map(|u| {
                kentos_domain::Uuid::parse_str(u)
                    .ok()
                    .and_then(|u| doc.slot_of(u))
            })
            .collect();
        let kinds: Vec<&str> = firsts
            .iter()
            .filter_map(|s| doc.get(*s))
            .map(|e| Self::kind_name(&shape(e)))
            .collect();
        let joined = out.changed.len() + out.removed.len();
        cx.selection.set(firsts);
        cx.say(
            Level::Success,
            format!("{joined} nesne birleştirildi: {}.", kinds.join(", ")),
        );
        for w in warnings {
            cx.say(Level::Warn, w.message);
        }
        true
    }

    /// Patlat (the web's `ExplodeTool.run`) through `cad.entities.explode` (docs/adr/0302): the pieces are the
    /// command's; an item of an associative array explodes its whole array.
    fn run_explode(targets: &[Slot], cx: &mut Context<'_>) {
        let input = Self::explode_input(targets, cx);
        if input.uids.is_empty() {
            return;
        }
        let result = entities_explode::execute(
            &mut ExecutionContext::with_store(cx.doc, cx.spatial.store()),
            input,
        );
        let Some(out) = points::written(result, cx) else {
            return;
        };
        let created: Vec<Slot> = out
            .created
            .iter()
            .filter_map(|uid| kentos_domain::Uuid::parse_str(uid).ok())
            .filter_map(|uid| cx.doc.slot_of(uid))
            .collect();
        let n = created.len();
        cx.selection.set(created);
        cx.say(
            Level::Success,
            format!("{} nesne patlatıldı: {n} parça.", out.removed.len()),
        );
    }
}

impl Stages for ObjectAction {
    fn id(&self) -> &'static str {
        match self.kind {
            Kind::Join => JOIN_ID,
            Kind::Explode => EXPLODE_ID,
            Kind::Readable => READABLE_ID,
        }
    }

    fn label(&self) -> &'static str {
        match self.kind {
            Kind::Join => "Birleştir",
            Kind::Explode => "Patlat",
            Kind::Readable => "Okunur yap",
        }
    }

    /// Acts on the selection at once, and the tool leaves.
    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        let targets = Self::targets(cx);
        // Curves among Patlat's objects: their density is shown and chosen first (docs/adr/0308).
        if self.kind == Kind::Explode
            && targets.iter().any(|s| cx.doc.get(*s).is_some_and(is_curve))
        {
            self.curves = targets;
            self.replan(cx);
            return Flow::Stay;
        }
        if !targets.is_empty() {
            match self.kind {
                Kind::Join => Self::run_join(&targets, cx),
                Kind::Explode => Self::run_explode(&targets, cx),
                Kind::Readable => Self::run_readable(&targets, cx),
            }
        }
        Flow::Exit
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    /// Birleştir says its end gap tolerance and how to change it, and Zincir.
    fn picking_hint(&self, prompt: Prompt) -> Prompt {
        if self.kind != Kind::Join {
            return prompt;
        }
        let (m, f) = self.seen.unwrap_or_default();
        prompt
            .note(format!(
                "uç boşluğu toleransı {}",
                f.length(m.join_tolerance)
            ))
            .then()
            .note("değiştirmek için sayı yazın")
            .then()
            .toggle("Zincir", "Z", m.join_chain)
    }

    /// With Zincir on, Birleştir asks for one object of the chain.
    fn picking_prompt(&self, _n: usize) -> Option<Prompt> {
        let (m, _) = self.seen.unwrap_or_default();
        (self.kind == Kind::Join && m.join_chain).then(|| {
            self.picking_hint(Prompt::new(self.label(), "zincirin bir nesnesine tıklayın"))
        })
    }

    /// A number not below zero typed while picking is Birleştir's
    /// tolerance; Z turns Zincir on and off.
    fn picking_input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.kind != Kind::Join {
            return false;
        }
        if upper_tr(js_trim(text)) == "Z" {
            cx.memory.join_chain = !cx.memory.join_chain;
            self.see(cx);
            return true;
        }
        // Typed in the project's unit (docs/adr/0165 §2).
        match cx.typed_length(text) {
            Some(n) if n >= 0.0 => {
                cx.memory.join_tolerance = n;
                true
            }
            _ => false,
        }
    }

    /// Zincir: the clicked object's chain is joined, and the tool leaves;
    /// when no chain can be joined from it (said), another object may be clicked.
    fn picked(&mut self, hit: Slot, cx: &mut Context<'_>) -> Option<Flow> {
        if self.kind != Kind::Join || !cx.memory.join_chain {
            return None;
        }
        Some(if Self::join_chain(hit, cx) {
            Flow::Exit
        } else {
            Flow::Stay
        })
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        if self.curves.is_empty() {
            return Prompt::new(self.label(), "");
        }
        let (m, f) = self.seen.unwrap_or_default();
        let step = match self.plan.as_ref() {
            Some(p) => format!(
                "{} nesne, {} köşe; sıklığı seçin ya da Enter ile patlatın",
                p.removed.len(),
                self.vertices()
            ),
            None => "bu sıklıkta patlatılamaz; sıklığı değiştirin".to_owned(),
        };
        let mark = |way: u8, text: String| {
            if m.explode_way == way {
                format!("{text} ●")
            } else {
                text
            }
        };
        Prompt::new(self.label(), step)
            .then()
            .option("Patlat", "Enter")
            .option_with("Sayı", "S", mark(1, m.explode_count.to_string()))
            .option_with("Aralık", "A", mark(2, f.length(m.explode_step)))
            .option_with("Sapma", "P", mark(0, f.length(m.explode_deviation)))
            .then()
            .note("seçilenin değeri için sayı yazın")
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        if self.curves.is_empty() {
            Flow::Exit
        } else {
            Flow::Stay
        }
    }

    /// Patlat's density: S, A and P choose the way, a number its value (a count, or a length in the project's unit).
    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        if self.curves.is_empty() {
            return None;
        }
        let m = &mut *cx.memory;
        match upper_tr(js_trim(text)).as_str() {
            "S" => m.explode_way = 1,
            "A" => m.explode_way = 2,
            "P" => m.explode_way = 0,
            _ if m.explode_way == 1 => match js_trim(text).parse::<u32>() {
                Ok(n) if (1..=kentos_contracts::MAX_EXPLODE_VERTICES).contains(&n) => {
                    m.explode_count = n;
                }
                _ => {
                    cx.say(
                        Level::Warn,
                        "Bölüm sayısı 1 ile 100 000 arasında bir tam sayı olmalı.",
                    );
                    return Some(Flow::Stay);
                }
            },
            _ => match cx.typed_length(text) {
                Some(l) if l > 0.0 => {
                    let m = &mut *cx.memory;
                    if m.explode_way == 2 {
                        m.explode_step = l;
                    } else {
                        m.explode_deviation = l;
                    }
                }
                _ => {
                    cx.say(Level::Warn, "Sıfırdan büyük bir uzunluk yazın.");
                    return Some(Flow::Stay);
                }
            },
        }
        self.replan(cx);
        Some(Flow::Stay)
    }

    fn typed_points(&self) -> bool {
        self.curves.is_empty()
    }

    /// Enter writes Patlat at the chosen density.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if !self.curves.is_empty() {
            let targets = std::mem::take(&mut self.curves);
            Self::run_explode(&targets, cx);
        }
        Flow::Exit
    }

    fn stage_preview(&self, _hover: Option<Vec2>, _format: &Format) -> Option<Preview> {
        (!self.curves.is_empty()).then(|| self.drawn.clone())
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }
}
