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
//!   and data (docs/adr/0144).
//!
//! - Okunur yap (docs/adr/0145 §6): the texts that read upside down (turned
//!   more than 90° and at most 270°) turn half round about their box's
//!   middle, the box where it was and the alignment kept; a text that reads
//!   stays, and when none reads upside down that is said and nothing is
//!   written.
//!
//! Each writes one undo step through `cad.entities.edit`. The chains, the
//! pieces and the turn are the shared core's (`join_entities`,
//! `explode_entity`, `TextPlace::readable`).

use kentos_contracts::{EditOperation, Entity, EntityEdit, TextEntity};
use kentos_domain::Slot;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape, TextPlace, dimension_geom};
use kentos_geometry_core::geom::affine::Affine;
use kentos_geometry_core::geom::dimension::layout_dimension;
use kentos_geometry_core::ops::curve_cuts::Cut;
use kentos_geometry_core::ops::explode::explode_entity;
use kentos_geometry_core::ops::join::{ChainObject, chain, join_entities};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::blocks::core_entity;
use kentos_native_application::geometry::{drawing_font, edit_geometry, shape};

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::modify::{Modify, Stages};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Flow, Memory};

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
}

impl ObjectAction {
    pub fn join() -> Modify<Self> {
        Modify::with(Self {
            kind: Kind::Join,
            seen: None,
        })
    }

    pub fn explode() -> Modify<Self> {
        Modify::with(Self {
            kind: Kind::Explode,
            seen: None,
        })
    }

    pub fn readable() -> Modify<Self> {
        Modify::with(Self {
            kind: Kind::Readable,
            seen: None,
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
            Shape::Xline { .. } => "yardımcı çizgi",
            Shape::Ray { .. } => "ışın",
            Shape::Text { .. } => "yazı",
            Shape::Dimension { .. } => "ölçü",
            Shape::Hatch { .. } => "tarama",
            Shape::Insert { .. } => "blok",
            Shape::Leader { .. } => "kılavuz",
            Shape::Table { .. } => "tablo",
            Shape::Image { .. } => "resim",
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
    /// visible lines, arcs and polylines walked from `hit` by the core
    /// (`chain`), then joined as a selection is. Whether a chain was joined.
    fn join_chain(hit: Slot, cx: &mut Context<'_>) -> bool {
        let doc = &*cx.doc;
        let Some(seed) = doc.get(hit) else {
            return false;
        };
        if !matches!(seed, Entity::Line(_) | Entity::Arc(_) | Entity::Polyline(_)) {
            cx.say(
                Level::Warn,
                "Zincir bir çizgiden, yaydan ya da açık çoklu çizgiden başlar.",
            );
            return false;
        }
        if doc.layers().is_locked(&seed.base().layer_id) {
            cx.say(Level::Warn, "Kilitli katmandaki nesne birleştirilemez.");
            return false;
        }
        let mut slots = Vec::new();
        let mut objects = Vec::new();
        for item in cx.spatial.store().overlapping(&cx.view.visible(), None) {
            let Some(slot) = crate::spatial::slot(item.id) else {
                continue;
            };
            let Some(e) = doc.get(slot) else {
                continue;
            };
            if matches!(e, Entity::Line(_) | Entity::Arc(_) | Entity::Polyline(_)) {
                slots.push(slot);
                objects.push(ChainObject {
                    shape: item.shape.clone(),
                    locked: doc.layers().is_locked(&e.base().layer_id),
                });
            }
        }
        let seed_at = match slots.iter().position(|s| *s == hit) {
            Some(i) => i,
            None => {
                slots.push(hit);
                objects.push(ChainObject {
                    shape: shape(seed),
                    locked: false,
                });
                slots.len() - 1
            }
        };
        let found = chain(&objects, seed_at, cx.memory.join_tolerance.max(1e-9));
        if found.members.len() < 2 {
            cx.say(
                Level::Warn,
                "Bu nesneye ucu ucuna bağlanan nesne yok; zincir kurulamadı.",
            );
            return false;
        }
        // The clicked object keeps its place, its persistent id and its data (docs/adr/0161 §2).
        let members: Vec<Slot> = found
            .members
            .iter()
            .filter_map(|&i| slots.get(i).copied())
            .collect();
        Self::run_join(&members, Some(hit), cx);
        if found.locked {
            cx.say(Level::Warn, "Zincir kilitli katmandaki bir nesnede durdu.");
        }
        true
    }

    /// Birleştir (the web's `JoinTool.run`); `keep`, when one of a chain, is
    /// the object that chain becomes (Zincir's clicked one).
    fn run_join(targets: &[Slot], keep: Option<Slot>, cx: &mut Context<'_>) {
        let tolerance = cx.memory.join_tolerance.max(1e-9);
        // The core names each object by its `id`: the slot.
        let list: Vec<CoreEntity> = targets
            .iter()
            .filter_map(|slot| {
                let e = cx.doc.get(*slot)?;
                Some(CoreEntity {
                    shape: shape(e),
                    rest: vec![("id".to_owned(), Json::Num(f64::from(slot.0)))],
                })
            })
            .collect();
        let groups = match join_entities(&list, tolerance) {
            Ok(joined) => joined.groups,
            Err(error) => return cx.say(Level::Warn, error),
        };
        if groups.is_empty() {
            cx.say(
                Level::Warn,
                "Uçları birleşen çizgi, yay ya da açık çoklu çizgi bulunamadı. Toleransı artırmayı deneyin.",
            );
            return;
        }
        let slot_of = |id: &Json| match id {
            Json::Num(n) if *n >= 0.0 && n.fract() == 0.0 => Some(Slot(*n as u32)),
            _ => None,
        };
        // Each chain is its first object, joined (AutoCAD JOIN): its slot, persistent id,
        // layer, colour, attributes and label; the others go.
        let mut changes = Vec::new();
        let mut firsts = Vec::new();
        let mut joined = 0;
        for g in &groups {
            let slots: Vec<Slot> = g.sources.iter().filter_map(slot_of).collect();
            let first = keep
                .filter(|k| slots.contains(k))
                .or_else(|| slots.first().copied());
            let (Some(first), Some(geometry)) = (first, edge::geometry(&g.geometry.shape)) else {
                continue;
            };
            joined += slots.len();
            firsts.push(first);
            changes.push(EntityEdit::Replace {
                uid: edge::uid(cx.doc, first),
                geometry,
                keep_data: Some(true),
            });
            for slot in slots.iter().filter(|&&s| s != first) {
                changes.push(EntityEdit::Remove {
                    uid: edge::uid(cx.doc, *slot),
                });
            }
        }
        if edge::write(EditOperation::Join, changes, cx).is_none() {
            return;
        }
        cx.selection.set(firsts);
        let kinds: Vec<&str> = groups
            .iter()
            .map(|g| Self::kind_name(&g.geometry.shape))
            .collect();
        cx.say(
            Level::Success,
            format!("{joined} nesne birleştirildi: {}.", kinds.join(", ")),
        );
    }

    /// A dimension's measured value as the drawing shows it, in project
    /// units: what its exploded text says when it has none of its own.
    fn value_text(e: &Entity, f: &Format) -> String {
        let Entity::Dimension(d) = e else {
            return String::new();
        };
        if d.text.as_deref().is_some_and(|t| !t.is_empty()) {
            return String::new();
        }
        let s = shape(e);
        let Some(l) = dimension_geom(&s).and_then(|g| layout_dimension(&g)) else {
            return String::new();
        };
        f.dimension(l.prefix, l.unit, l.value)
    }

    /// Patlat (the web's `ExplodeTool.run`).
    fn run_explode(targets: &[Slot], cx: &mut Context<'_>) {
        let f = cx.format();
        let font = drawing_font(cx.doc.settings().drawing_font);
        let mut changes = Vec::new();
        let mut exploded = 0;
        let mut first_error: Option<String> = None;
        for slot in targets {
            let Some(e) = cx.doc.get(*slot) else { continue };
            // A block's insert opens into its definition's objects, one level
            // (docs/adr/0144): each keeps its own layer, colour, line weight
            // and data; what it lacks is the insert's.
            let cut = if let Entity::Insert(_) = e {
                cx.spatial.store().blocks().explode(&core_entity(e))
            } else {
                explode_entity(&shape(e), &Self::value_text(e, &f), font)
            };
            let insert = matches!(e, Entity::Insert(_));
            match cut {
                Cut::Error(error) => {
                    first_error.get_or_insert(error);
                }
                Cut::Pieces(pieces) => {
                    let uid = edge::uid(cx.doc, *slot);
                    exploded += 1;
                    changes.push(EntityEdit::Remove { uid: uid.clone() });
                    for piece in pieces {
                        let Some(geometry) = edge::geometry(&piece.shape) else {
                            continue;
                        };
                        changes.push(if insert {
                            block_piece(cx.doc, &uid, geometry, &piece)
                        } else {
                            EntityEdit::add(uid.clone(), geometry, None)
                        });
                    }
                }
            }
        }
        if exploded == 0 {
            if let Some(error) = first_error {
                cx.say(Level::Warn, error);
            }
            return;
        }
        let Some(out) = edge::write(EditOperation::Explode, changes, cx) else {
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
        let skipped = first_error
            .map(|e| format!(" Bazı nesneler atlandı: {e}"))
            .unwrap_or_default();
        cx.say(
            Level::Success,
            format!("{exploded} nesne patlatıldı: {n} parça.{skipped}"),
        );
    }
}

/// A block's object exploded from the insert `from` (docs/adr/0144): an
/// `add` with the object's own layer when the drawing has it as a layer (else
/// the insert's), its colour and line weight (the insert's when it has none:
/// the core gives them), its attributes and label.
fn block_piece(
    doc: &kentos_domain::Document,
    from: &str,
    geometry: kentos_contracts::EntityGeometry,
    piece: &CoreEntity,
) -> EntityEdit {
    let field = |name: &str| piece.rest.iter().find(|(k, _)| k == name).map(|(_, v)| v);
    let text = |name: &str| match field(name) {
        Some(Json::Str(s)) => Some(s.clone()),
        _ => None,
    };
    let layer_id = text("layerId").filter(|id| {
        doc.layers()
            .get(id)
            .is_some_and(|n| n.kind == kentos_contracts::LayerNodeType::Layer)
    });
    let line_weight = match field("lineWeight") {
        Some(Json::Num(w)) => Some(*w),
        _ => None,
    };
    let attrs = match field("attrs") {
        Some(Json::Obj(fields)) => Some(
            fields
                .iter()
                .filter_map(|(k, v)| match v {
                    Json::Str(s) => Some((k.clone(), s.clone())),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    };
    EntityEdit::Add {
        from: from.to_owned(),
        geometry,
        keep_data: None,
        layer_id,
        color: text("color"),
        line_weight,
        attrs,
        label: text("label"),
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
        if !targets.is_empty() {
            match self.kind {
                Kind::Join => Self::run_join(&targets, None, cx),
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
        Prompt::new(self.label(), "")
    }

    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn typed(&mut self, _text: &str, _cx: &mut Context<'_>) -> Option<Flow> {
        None
    }

    fn preview(&self, _hover: Vec2) -> Option<Affine> {
        None
    }
}
