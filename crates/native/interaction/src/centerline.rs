//! Orta hat (docs/adr/0190; Netcad's Orta Hat Çiz; the web's
//! `CenterlineTool`, `apps/web/src/tools/centerlineTool.ts`): the axis
//! between two sides, a road's edges or a stream's banks. The first side and
//! the second are clicked; with Zincir a side is the clicked line's chain
//! (Birleştir's Zincir, docs/adr/0161 §3) joined into one path. The axis is
//! the core's (`ops::centerline`): exact between matched sides, else
//! sampled every Adım. It shows dashed; Enter, Uygula or a quick right click
//! writes it as a polyline through `cad.entities.create` (step “Orta hat”)
//! on the active layer and the tool waits for the next pair. Esc and Ctrl+Z
//! let the last side go; Esc with none leaves.

use kentos_contracts::{CreateOperation, Entity, EntityGeometry};
use kentos_domain::Slot;
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape, entity_outline};
use kentos_geometry_core::geom::bulge::bulge_path_outline;
use kentos_geometry_core::ops::centerline::{self as rules, Centerline as Axis};
use kentos_geometry_core::ops::join::{ChainObject, chain, join_entities};
use kentos_geometry_core::tools::point_calc::route_of;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::format::{Format, js_number};
use crate::log::Level;
use crate::points::{self, wire_all};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Memory, Pointer, Preview, Stroke, Tone, Tool};

/// The tool's id: its command is `tool.centerline`.
pub const ID: &str = "centerline";
pub const LABEL: &str = "Orta hat";
/// Adım at the start, metres.
pub const FIRST_STEP: f64 = 1.0;
/// Why a click gave no side.
pub const NO_OBJECT_HERE: &str =
    "Tıklanan yerde nesne yok; çizginin, yayın, dairenin, elipsin, eğrinin ya da alanın üzerine tıklayın.";
/// The second side is the first.
pub const SAME_SIDE: &str = "İkinci kenar birincisiyle aynı olamaz; öbür kenara tıklayın.";

/// A side: the objects it is made of and its path.
#[derive(Clone, Debug)]
struct Side {
    slots: Vec<Slot>,
    shape: Shape,
}

/// What the axis was worked out from: the options, the project's units
/// (for the prompt) and the sides' objects.
type Seen = (Memory, Format, Option<Vec<Slot>>, Option<Vec<Slot>>);

pub struct Centerline {
    first: Option<Side>,
    second: Option<Side>,
    asking: bool,
    plan: Option<Result<Axis, String>>,
    seen: Option<Seen>,
}

impl Default for Centerline {
    fn default() -> Self {
        Self::new()
    }
}

/// The line, arc or polyline `hit`'s chain among the visible ones, joined
/// into one path; the object alone when no chain goes on from it.
fn chained(hit: Slot, cx: &Context<'_>) -> Option<Side> {
    let doc = &*cx.doc;
    let seed = doc.get(hit)?;
    let alone = Side {
        slots: vec![hit],
        shape: shape(seed),
    };
    if !matches!(seed, Entity::Line(_) | Entity::Arc(_) | Entity::Polyline(_)) {
        return Some(alone);
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
            // Read, not written: a locked layer's line is a side too.
            objects.push(ChainObject {
                shape: item.shape.clone(),
                locked: false,
            });
        }
    }
    let seed_at = match slots.iter().position(|s| *s == hit) {
        Some(i) => i,
        None => return Some(alone),
    };
    let tolerance = cx.memory.join_tolerance.max(1e-9);
    let found = chain(&objects, seed_at, tolerance);
    if found.members.len() < 2 {
        return Some(alone);
    }
    let members: Vec<Slot> = found.members.iter().filter_map(|&i| slots.get(i).copied()).collect();
    // The core names each object by its `id`: the slot (Birleştir's `run_join`).
    let list: Vec<CoreEntity> = members
        .iter()
        .filter_map(|slot| {
            let e = doc.get(*slot)?;
            Some(CoreEntity {
                shape: shape(e),
                rest: vec![("id".to_owned(), Json::Num(f64::from(slot.0)))],
            })
        })
        .collect();
    match join_entities(&list, tolerance) {
        Ok(joined) if joined.groups.len() == 1 => Some(Side {
            slots: members,
            shape: joined.groups[0].geometry.shape.clone(),
        }),
        _ => Some(alone),
    }
}

impl Centerline {
    pub fn new() -> Self {
        Self {
            first: None,
            second: None,
            asking: false,
            plan: None,
            seen: None,
        }
    }

    /// The axis for the sides as the options are now; worked out again only
    /// when something it depends on changed.
    fn see(&mut self, cx: &mut Context<'_>) {
        let key = (
            *cx.memory,
            cx.format(),
            self.first.as_ref().map(|s| s.slots.clone()),
            self.second.as_ref().map(|s| s.slots.clone()),
        );
        if self.seen.as_ref() == Some(&key) {
            return;
        }
        self.seen = Some(key);
        let (Some(a), Some(b)) = (&self.first, &self.second) else {
            self.plan = None;
            return;
        };
        let plan = rules::centerline(&a.shape, &b.shape, cx.memory.centerline_step);
        if let Err(why) = &plan {
            cx.say(Level::Warn, why.clone());
        }
        self.plan = Some(plan);
    }

    /// The side under the click: the most specific object that has a route.
    fn pick(&self, at: Vec2, cx: &mut Context<'_>) -> Option<Side> {
        let hits = cx.spatial.hits(at, cx.pick_tolerance());
        if hits.is_empty() {
            cx.say(Level::Warn, NO_OBJECT_HERE);
            return None;
        }
        let slot = hits.into_iter().find(|slot| {
            cx.doc
                .get(*slot)
                .is_some_and(|e| route_of(&shape(e)).is_some())
        });
        let Some(slot) = slot else {
            cx.say(Level::Warn, rules::NO_ROUTE);
            return None;
        };
        if cx.memory.centerline_chain {
            chained(slot, cx)
        } else {
            let e = cx.doc.get(slot)?;
            Some(Side {
                slots: vec![slot],
                shape: shape(e),
            })
        }
    }

    /// Writes the axis and says so; the tool waits for the next pair.
    fn write(&mut self, cx: &mut Context<'_>) {
        let Some(Ok(axis)) = &self.plan else {
            return;
        };
        let geometry = EntityGeometry::Polyline {
            pts: wire_all(&axis.pts),
            bulges: axis
                .bulges
                .clone()
                .filter(|b| b.iter().any(|&x| x != 0.0)),
            zs: None,
            parts: None,
        };
        if points::write_objects(vec![geometry], Some(CreateOperation::Centerline), cx).is_none() {
            return;
        }
        let how = match axis.method {
            rules::Method::Matched => "kenar kenar",
            rules::Method::Sampled => "örneklenerek",
        };
        cx.say(
            Level::Success,
            format!("{LABEL}: {} köşeyle {how} yazıldı.", axis.pts.len()),
        );
        self.first = None;
        self.second = None;
    }
}

impl Tool for Centerline {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        if self.asking {
            return Prompt::new(LABEL, "örnekleme adımını yazın");
        }
        let step = match (&self.first, &self.second, &self.plan) {
            (None, _, _) => "birinci kenara tıklayın".to_owned(),
            (Some(_), None, _) => "ikinci kenara tıklayın".to_owned(),
            (_, _, Some(Ok(axis))) => format!(
                "{} köşe; Enter ile yazın ya da yeni bir kenar çiftine tıklayın",
                axis.pts.len()
            ),
            _ => "bu kenarlarla orta hat çizilemiyor; başka kenarlara tıklayın".to_owned(),
        };
        let (step_text, chain) = self.seen.as_ref().map_or_else(
            || (format!("{} m", js_number(FIRST_STEP)), true),
            |(m, format, ..)| (format.length(m.centerline_step), m.centerline_chain),
        );
        let prompt = Prompt::new(LABEL, step)
            .option_with("Adım", "B", step_text)
            .toggle("Zincir", "Z", chain);
        if matches!(self.plan, Some(Ok(_))) {
            prompt.option("Uygula", "Enter")
        } else {
            prompt
        }
    }

    fn point_count(&self) -> usize {
        usize::from(self.first.is_some()) + usize::from(self.second.is_some())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        false
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn pointer_move(&mut self, _p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.asking {
            return;
        }
        let Some(side) = self.pick(p.raw, cx) else {
            return;
        };
        match (&self.first, &self.second) {
            (Some(first), None) => {
                if side.slots.iter().any(|s| first.slots.contains(s)) {
                    cx.say(Level::Warn, SAME_SIDE);
                    return;
                }
                self.second = Some(side);
            }
            // A new pair starts from any click once the axis is shown.
            _ => {
                self.first = Some(side);
                self.second = None;
            }
        }
        self.see(cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let t = js_trim(text);
        if self.asking {
            match cx.typed_length(t) {
                Some(n) if n > 0.0 && n.is_finite() => {
                    cx.memory.centerline_step = n;
                    self.asking = false;
                }
                _ => cx.say(
                    Level::Warn,
                    format!("Adım sıfırdan büyük bir uzunluk olmalı; “{t}” yazıldı."),
                ),
            }
        } else {
            match upper_tr(t).as_str() {
                "B" => self.asking = true,
                "Z" => cx.memory.centerline_chain = !cx.memory.centerline_chain,
                _ => return false,
            }
        }
        self.see(cx);
        true
    }

    /// Enter: Adım's question ends; with an axis it is written; else the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if std::mem::take(&mut self.asking) {
            return Flow::Stay;
        }
        if matches!(self.plan, Some(Ok(_))) {
            self.write(cx);
            self.see(cx);
            return Flow::Stay;
        }
        Flow::Exit
    }

    /// Esc: Adım's question, then the second side, then the first go first.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if std::mem::take(&mut self.asking) {
            return true;
        }
        self.undo_step(cx)
    }

    /// Ctrl+Z: the last side goes; with none the drawing is undone.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        let gone = if self.second.is_some() {
            self.second = None;
            true
        } else {
            self.first.take().is_some()
        };
        if gone {
            self.see(cx);
        }
        gone
    }

    fn preview(&self, _format: &Format) -> Preview {
        let mut out = Preview::default();
        for side in [&self.first, &self.second].into_iter().flatten() {
            out.strokes
                .push(Stroke::solid(entity_outline(&side.shape, 72.0), false).tone(Tone::Snap));
        }
        if let Some(Ok(axis)) = &self.plan {
            let line = bulge_path_outline(&axis.pts, axis.bulges.as_deref(), false, 0.05);
            out.strokes.push(Stroke::dashed(line, false, [6.0, 4.0]).tone(Tone::Snap));
        }
        out
    }
}
