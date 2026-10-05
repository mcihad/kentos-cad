//! Delik ekle, Deliği sil and Deliği doldur (docs/adr/0173 §5): the web's
//! `holeTools.ts`, step for step.
//!
//! - Delik ekle is the path tool's ring (`path::Shape::Hole`, drawn as
//!   Kapalı alan's): its first point finds the area ([`target`]: the selected
//!   one when it is around the point, else the smallest around it), the
//!   finished ring is cut from it ([`add`]).
//! - Deliği sil and Deliği doldur ([`HoleClick`]) highlight the hole under
//!   the pointer; a click removes it, or writes a new area of its ring (the
//!   hole stays: the new area fills it) on the area's layer, in its colour and
//!   line weight, without its attributes and label.
//!
//! Every write is one `cad.entities.edit` step named after the tool; the
//! command carries elevations by where the vertices lie (docs/adr/0142). The
//! geometry is the shared core's (`ops::holes`); none is computed here.

use kentos_contracts::{EditOperation, Entity, EntityEdit};
use kentos_domain::Slot;
use kentos_geometry_core::entity::{Shape, area_parts, polygon_ring};
use kentos_geometry_core::geom::arrangement::Ring;
use kentos_geometry_core::ops::holes::{HoleRefusal, hole_add, hole_remove, hole_ring};

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::Prompt;
use crate::spatial::measures;
use crate::tool::{self, Context, Flow, Pointer, Preview, Tag, Tone, Tool};

/// Delik ekle: `tool.holeAdd` (the path tool's `Shape::Hole`).
pub const ADD_ID: &str = "holeAdd";
pub const ADD_LABEL: &str = "Delik ekle";
/// Deliği sil: `tool.holeRemove`.
pub const REMOVE_ID: &str = "holeRemove";
pub const REMOVE_LABEL: &str = "Deliği sil";
/// Deliği doldur: `tool.holeFill`.
pub const FILL_ID: &str = "holeFill";
pub const FILL_LABEL: &str = "Deliği doldur";

/// Delik ekle's first point is in no area.
pub const NO_TARGET: &str =
    "Deliğin ekleneceği alanı seçin ya da halkaya bir alanın içinden başlayın.";
/// A click of Deliği sil or Deliği doldur outside every hole.
pub const NOT_IN_HOLE: &str = "Bir deliğin içine tıklayın.";

/// An area on an unlocked layer.
fn open_area(slot: Slot, cx: &Context<'_>) -> bool {
    cx.doc
        .get(slot)
        .is_some_and(|e| matches!(e, Entity::Polygon(_)) && edge::unlocked(e, cx.doc))
}

/// The area Delik ekle's ring starting at `p` goes into: the one selected,
/// when it alone is and `p` is in it or in one of its holes; else the
/// smallest area around `p`, then the area with the smallest hole around
/// it (a ring started in a hole widens it). None on locked layers.
pub fn target(p: Vec2, cx: &Context<'_>) -> Option<Slot> {
    let around: Vec<Slot> = cx
        .spatial
        .containing(p)
        .into_iter()
        .map(|(slot, _)| slot)
        .chain(cx.spatial.holes_at(p).into_iter().map(|(slot, _)| slot))
        .filter(|&slot| open_area(slot, cx))
        .collect();
    match cx.selection.ids() {
        [one] if around.contains(one) => Some(*one),
        _ => around.first().copied(),
    }
}

/// The rings of the area at `slot` as the preview fills it: each part's outer
/// ring and its holes (Delik ekle shows the area its ring goes into).
pub fn rings_of(slot: Slot, cx: &Context<'_>) -> Vec<Vec<Vec2>> {
    let Some(e) = cx.doc.get(slot) else {
        return Vec::new();
    };
    let mut rings = Vec::new();
    for part in area_parts(&edge::core(e).shape).iter() {
        if let Shape::Polygon {
            pts, bulges, holes, ..
        } = part
        {
            rings.push(polygon_ring(pts, bulges.as_deref()));
            for h in holes.iter().flatten() {
                rings.push(polygon_ring(&h.pts, h.bulges.as_deref()));
            }
        }
    }
    rings
}

/// The refusal in the tools' words.
fn refusal_text(why: &HoleRefusal) -> &'static str {
    match why {
        HoleRefusal::NotArea => "Delik yalnız kapalı alana eklenir.",
        HoleRefusal::Degenerate => "Delik en az üç köşeli, alanı olan bir halka olmalı.",
        HoleRefusal::Outside => {
            "Delik alanın içinde kalmalı; sınırı aşan bölümü çıkarmak için Alan çıkar'ı kullanın."
        }
        HoleRefusal::Splits => {
            "Delik alanı parçalara ayırırdı; alanı bölmek için Alan böl'ü kullanın."
        }
        HoleRefusal::NotInHole => NOT_IN_HOLE,
    }
}

/// How many holes an area has, in all its parts.
fn hole_count(shape: &Shape) -> usize {
    area_parts(shape)
        .iter()
        .map(|part| match part {
            Shape::Polygon { holes: Some(h), .. } => h.len(),
            _ => 0,
        })
        .sum()
}

/// The area's net size now, as the log says it.
fn area_text(slot: Slot, cx: &Context<'_>) -> String {
    let area = cx.doc.get(slot).and_then(|e| measures(e).0).unwrap_or(0.0);
    cx.format().area(area)
}

/// Delik ekle: the ring cut from the area at `slot`, one undo step. Whether
/// it was written; the refusal said otherwise.
pub fn add(slot: Slot, ring: Ring, cx: &mut Context<'_>) -> bool {
    let Some(e) = cx.doc.get(slot) else {
        return false;
    };
    let before = edge::core(e).shape;
    let after = match hole_add(&before, &ring) {
        Ok(shape) => shape,
        Err(why) => {
            cx.say(Level::Warn, refusal_text(&why));
            return false;
        }
    };
    let merged = hole_count(&after) <= hole_count(&before);
    let Some(geometry) = edge::geometry(&after) else {
        return false;
    };
    let uid = edge::uid(cx.doc, slot);
    if edge::write(
        EditOperation::HoleAdd,
        vec![EntityEdit::Update { uid, geometry }],
        cx,
    )
    .is_none()
    {
        return false;
    }
    let size = area_text(slot, cx);
    let text = if merged {
        format!("Delik eklendi, var olan delikle birleşti; alan {size} oldu.")
    } else {
        format!("Delik eklendi; alan {size} oldu.")
    };
    cx.say(Level::Success, text);
    true
}

/// The hole under a point: its area and its ring.
#[derive(Clone, Debug, PartialEq)]
struct Found {
    slot: Slot,
    at: Vec2,
    ring: Ring,
}

/// The smallest hole around `p` of an area on an unlocked layer.
fn found(p: Vec2, cx: &Context<'_>) -> Option<Found> {
    cx.spatial
        .holes_at(p)
        .into_iter()
        .filter(|&(slot, _)| open_area(slot, cx))
        .find_map(|(slot, _)| {
            let ring = hole_ring(&edge::core(cx.doc.get(slot)?).shape, p).ok()?;
            Some(Found { slot, at: p, ring })
        })
}

/// Deliği sil or Deliği doldur: a click in a hole removes it, or fills it
/// with a new area.
#[derive(Clone, Debug)]
pub struct HoleClick {
    fill: bool,
    hover: Option<Found>,
}

impl HoleClick {
    /// Deliği sil (`tool.holeRemove`).
    pub fn remove() -> Self {
        Self {
            fill: false,
            hover: None,
        }
    }

    /// Deliği doldur (`tool.holeFill`).
    pub fn fill() -> Self {
        Self {
            fill: true,
            hover: None,
        }
    }

    fn remove_at(&self, hole: &Found, cx: &mut Context<'_>) {
        let Some(e) = cx.doc.get(hole.slot) else {
            return;
        };
        let after = match hole_remove(&edge::core(e).shape, hole.at) {
            Ok(shape) => shape,
            Err(why) => return cx.say(Level::Warn, refusal_text(&why)),
        };
        let Some(geometry) = edge::geometry(&after) else {
            return;
        };
        let uid = edge::uid(cx.doc, hole.slot);
        if edge::write(
            EditOperation::HoleRemove,
            vec![EntityEdit::Update { uid, geometry }],
            cx,
        )
        .is_some()
        {
            let text = format!("Delik silindi; alan {} oldu.", area_text(hole.slot, cx));
            cx.say(Level::Success, text);
        }
    }

    fn fill_at(&self, hole: &Found, cx: &mut Context<'_>) {
        let shape = Shape::Polygon {
            pts: hole.ring.pts.clone(),
            bulges: hole.ring.bulges.clone(),
            holes: None,
            parts: None,
        };
        let Some(geometry) = edge::geometry(&shape) else {
            return;
        };
        let from = edge::uid(cx.doc, hole.slot);
        let Some(out) = edge::write(
            EditOperation::HoleFill,
            vec![EntityEdit::add(from, geometry, None)],
            cx,
        ) else {
            return;
        };
        let made: Vec<Slot> = out
            .created
            .iter()
            .filter_map(|uid| kentos_domain::Uuid::parse_str(uid).ok())
            .filter_map(|uid| cx.doc.slot_of(uid))
            .collect();
        let size = made.first().map_or_else(String::new, |&s| area_text(s, cx));
        cx.selection.set(made);
        cx.say(
            Level::Success,
            format!("Delik dolduruldu: yeni alan {size}."),
        );
    }
}

impl Tool for HoleClick {
    fn id(&self) -> &'static str {
        if self.fill { FILL_ID } else { REMOVE_ID }
    }

    fn label(&self) -> &'static str {
        if self.fill { FILL_LABEL } else { REMOVE_LABEL }
    }

    fn prompt(&self) -> Prompt {
        let what = if self.fill {
            "doldurulacak deliğin içine tıklayın"
        } else {
            "silinecek deliğin içine tıklayın"
        };
        Prompt::new(self.label(), what)
    }

    fn point_count(&self) -> usize {
        0
    }

    fn snaps(&self) -> bool {
        false
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.hover = found(p.raw, cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(hole) = found(p.raw, cx) else {
            cx.say(Level::Warn, NOT_IN_HOLE);
            return;
        };
        if self.fill {
            self.fill_at(&hole, cx);
        } else {
            self.remove_at(&hole, cx);
        }
        // What the click changed is under the pointer now.
        self.hover = found(p.raw, cx);
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// Enter, Space or a quick right click leave the tool.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    /// The tool takes nothing back itself: Ctrl+Z is the drawing's.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// The hole under the pointer filled lightly, its area by it.
    fn preview(&self, format: &Format) -> Preview {
        let Some(hole) = &self.hover else {
            return Preview::default();
        };
        let size = kentos_geometry_core::geom::region::ring_area(&hole.ring).abs();
        Preview {
            areas: vec![tool::Area {
                rings: vec![polygon_ring(&hole.ring.pts, hole.ring.bulges.as_deref())],
                fill: 0.16,
                width: 2.0,
                dash: None,
                fill_tone: Tone::Accent,
            }],
            tag: Some(Tag {
                at: hole.at,
                lines: vec![format!("Delik {}", format.area(size))],
            }),
            ..Preview::default()
        }
    }
}
